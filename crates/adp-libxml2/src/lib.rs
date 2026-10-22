//! libxml2, statically linked, for the XML editor's schema checks (etalii.adp spec 030, FR-020, task T032; plan
//! "Complexity Tracking": the one C library of the desktop host, because no complete pure-Rust XSD validator exists).
//!
//! `libxml2/` holds libxml2 2.15.4's library sources as LaminarDB's `laminardb-libxml2-ffi` 0.33.0 vendors them
//! (MIT; from `https://download.gnome.org/sources/libxml2/2.15/libxml2-2.15.4.tar.xz`, with LaminarDB's one change: an
//! included or imported schema is read through the caller's resource loader too), cut down to the files the build
//! compiles, with one change of ADP's, marked "ADP" in `xmlschemas.c`: a facet's error gives the type whose facet
//! failed as its third string, so a finding can name the datatype as the standalone host's does. `build.rs` compiles them with `cc`, so no CMake and no system libxml2 is needed on any platform.
//!
//! This crate is the only place the desktop host calls C, which is why it alone allows `unsafe` (its `Cargo.toml`).
//! Its calls are safe: every file libxml2 asks for goes through [`Files`], by a project path, so nothing outside the
//! project and nothing on the network is read. A document is read under the address [`address_of`] gives its project
//! path; a reference that leaves the project, names a web address or any other scheme is refused, and the refusal
//! is recorded with its reason.

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::Once;

/// Reads a file of the project by its path segments, or `None` when it cannot be read.
pub type Files<'a> = &'a dyn Fn(&[String]) -> Option<String>;

/// The scheme and root of the addresses documents are read under; the root folder's name stands for the project's.
const ROOT: &str = "adp-project:///@/";

/// Where a finding came from in libxml2 (its `xmlErrorDomain`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// Reading the text: well-formedness, entities, the DTD's own syntax.
    Parser,
    /// Validity against a DTD.
    Dtd,
    /// Compiling an XSD.
    SchemaParser,
    /// Validity against an XSD.
    SchemaValidity,
    /// Reading a file.
    Io,
    /// Anything else, by its number.
    Other(i32),
}

/// One error or warning libxml2 reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub domain: Domain,
    /// libxml2's `xmlParserErrors` code.
    pub code: i32,
    /// An error, else a warning.
    pub error: bool,
    /// libxml2's sentence, trimmed.
    pub message: String,
    /// The project path of the file it is about, when that is a file of the project.
    pub file: Option<Vec<String>>,
    /// 1-based, 0 when unknown.
    pub line: u32,
    /// 1-based, 0 when unknown.
    pub column: u32,
    /// The node it is about, as libxml2's `xmlGetNodePath` writes it (`/library/book[2]`, `/a/b/@c`).
    pub node: Option<String>,
    pub str1: Option<String>,
    pub str2: Option<String>,
    pub str3: Option<String>,
    pub int1: i32,
}

/// A reference libxml2 asked for and was not given, and why, in words said after the reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The address as libxml2 asked for it.
    pub address: String,
    pub reason: String,
}

/// What a check found, and which files it read and was refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub issues: Vec<Issue>,
    pub refused: Vec<Refusal>,
    /// The project paths read, in the order they were first read.
    pub read: Vec<Vec<String>>,
}

/// What checking a document against XSDs gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XsdCheck {
    /// The schemas compiled; the document's issues against them.
    Checked(Outcome),
    /// The schemas did not compile; the first error of the outcome is what stopped them.
    DoesNotCompile(Outcome),
}

/// The address a file of the project is read under.
pub fn address_of(path: &[String]) -> String {
    let mut address = ROOT.to_owned();
    for (index, segment) in path.iter().enumerate() {
        if index > 0 {
            address.push('/');
        }
        for byte in segment.bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&byte) {
                address.push(char::from(byte));
            } else {
                address.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    address
}

/// The project path an address names, or the reason it names none.
pub fn path_of(address: &str) -> Result<Vec<String>, String> {
    let Some(rest) = address.strip_prefix(ROOT) else {
        let lower = address.to_ascii_lowercase();
        return Err(if lower.starts_with("http:") || lower.starts_with("https:") {
            "is a web address; ADP reads schemas from the project".into()
        } else if lower.starts_with("adp-project:") {
            "leaves the project".into()
        } else {
            "is not a file of the project".into()
        });
    };
    let mut path = Vec::new();
    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => return Err("leaves the project".into()),
            other => path.push(decoded(other).ok_or_else(|| "is not a file of the project".to_owned())?),
        }
    }
    if path.is_empty() {
        return Err("names a folder, not a file".into());
    }
    Ok(path)
}

fn decoded(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = segment.get(at + 1..at + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Checks `text`, the document at `document` (its project path) as typed, against the DTD its `DOCTYPE` names and
/// its internal subset. The DTD is read through `files`; the entities it declares are expanded as a reader would.
pub fn check_against_dtd(text: &str, document: &[String], files: Files) -> Outcome {
    init();
    let session = Session::new(files, Vec::new());
    let options = XML_PARSE_NOENT | XML_PARSE_DTDLOAD | XML_PARSE_DTDVALID | XML_PARSE_NONET | XML_PARSE_BIG_LINES;
    if let Some(doc) = session.parse(text, &address_of(document), options) {
        // SAFETY: `doc` came from `xmlCtxtReadMemory` and is freed once.
        unsafe { xmlFreeDoc(doc) };
    }
    session.into_outcome()
}

/// Compiles the XSDs `roots` names, each by its namespace ("" for none) and project path, and checks `text`, the
/// document at `document` as typed, against them all.
pub fn check_against_xsd(text: &str, document: &[String], roots: &[(String, Vec<String>)], files: Files) -> XsdCheck {
    init();
    let (address, wrapper) = match roots {
        [(_, path)] => (address_of(path), None),
        _ => {
            // Several roots: one schema that includes those without a namespace and imports the others.
            let mut wrapper = String::from("<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n");
            for (namespace, path) in roots {
                let location = escaped(&address_of(path));
                if namespace.is_empty() {
                    wrapper.push_str(&format!("  <xs:include schemaLocation=\"{location}\"/>\n"));
                } else {
                    wrapper.push_str(&format!(
                        "  <xs:import namespace=\"{}\" schemaLocation=\"{location}\"/>\n",
                        escaped(namespace)
                    ));
                }
            }
            wrapper.push_str("</xs:schema>\n");
            ("adp-project:///roots.xsd".to_owned(), Some(wrapper))
        }
    };
    let overrides = wrapper.map(|w| vec![(address.clone(), w)]).unwrap_or_default();
    let session = Session::new(files, overrides);
    let Some(schema) = session.compile(&address) else {
        return XsdCheck::DoesNotCompile(session.into_outcome());
    };
    let options = XML_PARSE_NOENT | XML_PARSE_DTDLOAD | XML_PARSE_NONET | XML_PARSE_BIG_LINES;
    if let Some(doc) = session.parse(text, &address_of(document), options) {
        // Only what the schema finds about the document is wanted: well-formedness is said elsewhere.
        session.issues.borrow_mut().clear();
        // SAFETY: `schema` is a compiled schema and `doc` a parsed document; each is freed once, the context first.
        unsafe {
            let validator = xmlSchemaNewValidCtxt(schema);
            if !validator.is_null() {
                xmlSchemaSetValidStructuredErrors(validator, Some(on_error), session.as_data());
                xmlSchemaValidateDoc(validator, doc);
                xmlSchemaFreeValidCtxt(validator);
            }
            xmlFreeDoc(doc);
        }
    }
    // SAFETY: freed once, after its last use.
    unsafe { xmlSchemaFree(schema) };
    XsdCheck::Checked(session.into_outcome())
}

/// Compiles `text`, the XSD at `path` as typed, reading what it includes and imports through `files`; `None` when it
/// compiles, else what stopped it.
pub fn compile_xsd(text: &str, path: &[String], files: Files) -> Option<Outcome> {
    init();
    let address = address_of(path);
    let session = Session::new(files, vec![(address.clone(), text.to_owned())]);
    match session.compile(&address) {
        Some(schema) => {
            // SAFETY: freed once.
            unsafe { xmlSchemaFree(schema) };
            None
        }
        None => Some(session.into_outcome()),
    }
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

fn init() {
    static ONCE: Once = Once::new();
    // SAFETY: libxml2's documented one-time initialisation.
    ONCE.call_once(|| unsafe { xmlInitParser() });
}

/// What one check shares with libxml2's callbacks.
struct Session<'a> {
    files: Files<'a>,
    /// Texts served instead of the files at these addresses: a schema as typed, the wrapper of several roots.
    overrides: Vec<(String, String)>,
    issues: RefCell<Vec<Issue>>,
    refused: RefCell<Vec<Refusal>>,
    read: RefCell<Vec<Vec<String>>>,
}

impl<'a> Session<'a> {
    fn new(files: Files<'a>, overrides: Vec<(String, String)>) -> Self {
        Self {
            files,
            overrides,
            issues: RefCell::new(Vec::new()),
            refused: RefCell::new(Vec::new()),
            read: RefCell::new(Vec::new()),
        }
    }

    fn as_data(&self) -> *mut c_void {
        std::ptr::from_ref(self).cast_mut().cast()
    }

    fn into_outcome(self) -> Outcome {
        Outcome {
            issues: self.issues.into_inner(),
            refused: self.refused.into_inner(),
            read: self.read.into_inner(),
        }
    }

    /// Parses `text` under `address`; `None` when libxml2 gave no document.
    fn parse(&self, text: &str, address: &str, options: c_int) -> Option<*mut c_void> {
        let length = c_int::try_from(text.len()).ok()?;
        let url = CString::new(address).ok()?;
        // SAFETY: the context is freed once; `text` and `url` outlive the call; the callbacks get `self`, which
        // outlives the context.
        unsafe {
            let context = xmlNewParserCtxt();
            if context.is_null() {
                return None;
            }
            xmlCtxtSetErrorHandler(context, Some(on_error), self.as_data());
            xmlCtxtSetResourceLoader(context, Some(load), self.as_data());
            let doc = xmlCtxtReadMemory(
                context,
                text.as_ptr().cast(),
                length,
                url.as_ptr(),
                c"UTF-8".as_ptr(),
                options,
            );
            xmlFreeParserCtxt(context);
            (!doc.is_null()).then_some(doc)
        }
    }

    /// Compiles the schema at `address`; `None` when it does not compile.
    fn compile(&self, address: &str) -> Option<*mut c_void> {
        let url = CString::new(address).ok()?;
        // SAFETY: the parser context and a schema that is not returned are freed once; `self` outlives the context.
        unsafe {
            let context = xmlSchemaNewParserCtxt(url.as_ptr());
            if context.is_null() {
                return None;
            }
            xmlSchemaSetParserStructuredErrors(context, Some(on_error), self.as_data());
            xmlSchemaSetResourceLoader(context, Some(load), self.as_data());
            let schema = xmlSchemaParse(context);
            xmlSchemaFreeParserCtxt(context);
            let errors = self.issues.borrow().iter().any(|i| i.error);
            if !schema.is_null() && !errors {
                return Some(schema);
            }
            if !schema.is_null() {
                xmlSchemaFree(schema);
            }
        }
        if !self.issues.borrow().iter().any(|i| i.error) {
            self.issues.borrow_mut().push(Issue {
                domain: Domain::SchemaParser,
                code: 0,
                error: true,
                message: "The schema could not be read.".into(),
                file: path_of(address).ok(),
                line: 0,
                column: 0,
                node: None,
                str1: None,
                str2: None,
                str3: None,
                int1: 0,
            });
        }
        None
    }

    /// The text at `address`, or why there is none.
    fn text_at(&self, address: &str) -> Result<String, String> {
        if let Some((_, text)) = self.overrides.iter().find(|(a, _)| a == address) {
            return Ok(text.clone());
        }
        let path = path_of(address)?;
        let text = (self.files)(&path).ok_or_else(|| "is not in the project".to_owned())?;
        let mut read = self.read.borrow_mut();
        if !read.contains(&path) {
            read.push(path);
        }
        Ok(text)
    }
}

const XML_PARSE_NOENT: c_int = 1 << 1;
const XML_PARSE_DTDLOAD: c_int = 1 << 2;
const XML_PARSE_DTDVALID: c_int = 1 << 4;
const XML_PARSE_NONET: c_int = 1 << 11;
const XML_PARSE_BIG_LINES: c_int = 1 << 22;
const XML_IO_ENOENT: c_int = 1524;
const XML_ERR_NO_MEMORY: c_int = 2;

/// libxml2's resource loader: every file it asks for, by address.
extern "C" fn load(
    data: *mut c_void,
    url: *const c_char,
    _public_id: *const c_char,
    _kind: c_int,
    _flags: c_int,
    out: *mut *mut c_void,
) -> c_int {
    if data.is_null() || url.is_null() || out.is_null() {
        return XML_IO_ENOENT;
    }
    // SAFETY: `data` is the `Session` the call was given, alive for the call; `url` is a C string.
    let session = unsafe { &*data.cast::<Session>() };
    // SAFETY: as above.
    let address = unsafe { CStr::from_ptr(url) }.to_string_lossy().into_owned();
    let text = session
        .text_at(&address)
        .and_then(|t| CString::new(t).map_err(|_| "is not a text file".to_owned()));
    match text {
        Ok(text) => {
            // SAFETY: libxml2 copies the string (no static flag) and gives the caller the input it returns.
            let input = unsafe { xmlNewInputFromString(url, text.as_ptr(), 0) };
            if input.is_null() {
                return XML_ERR_NO_MEMORY;
            }
            // SAFETY: `out` is libxml2's result pointer.
            unsafe { *out = input };
            0
        }
        Err(reason) => {
            if let Ok(mut refused) = session.refused.try_borrow_mut() {
                refused.push(Refusal { address, reason });
            }
            XML_IO_ENOENT
        }
    }
}

/// libxml2's structured error handler: every error and warning, with the node it is about.
extern "C" fn on_error(data: *mut c_void, error: *const XmlError) {
    if data.is_null() || error.is_null() {
        return;
    }
    // SAFETY: `data` is the `Session` the call was given; `error` is libxml2's, valid for the call.
    let session = unsafe { &*data.cast::<Session>() };
    // SAFETY: as above.
    let error = unsafe { &*error };
    let text = |p: *const c_char| {
        // SAFETY: libxml2's strings are C strings or null.
        (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().trim().to_owned())
    };
    let node = if error.node.is_null() {
        None
    } else {
        // SAFETY: the node is alive during the callback; its path is freed with libxml2's own deallocator.
        unsafe {
            let path = xmlGetNodePath(error.node);
            let owned = text(path);
            if !path.is_null()
                && let Some(free) = xmlFree
            {
                free(path.cast());
            }
            owned
        }
    };
    let domain = match error.domain {
        1 | 3 => Domain::Parser,
        4 | 23 => Domain::Dtd,
        8 => Domain::Io,
        16 => Domain::SchemaParser,
        17 => Domain::SchemaValidity,
        other => Domain::Other(other),
    };
    let issue = Issue {
        domain,
        code: error.code,
        error: error.level >= 2,
        message: text(error.message).unwrap_or_default(),
        file: text(error.file).and_then(|f| path_of(&f).ok()),
        line: u32::try_from(error.line).unwrap_or(0),
        column: u32::try_from(error.int2).unwrap_or(0),
        node,
        str1: text(error.str1),
        str2: text(error.str2),
        str3: text(error.str3),
        int1: error.int1,
    };
    if let Ok(mut issues) = session.issues.try_borrow_mut() {
        issues.push(issue);
    }
}

/// libxml2's `xmlError`.
#[repr(C)]
struct XmlError {
    domain: c_int,
    code: c_int,
    message: *const c_char,
    level: c_int,
    file: *const c_char,
    line: c_int,
    str1: *const c_char,
    str2: *const c_char,
    str3: *const c_char,
    int1: c_int,
    int2: c_int,
    ctxt: *mut c_void,
    node: *mut c_void,
}

type ErrorHandler = Option<extern "C" fn(*mut c_void, *const XmlError)>;
type Loader = Option<extern "C" fn(*mut c_void, *const c_char, *const c_char, c_int, c_int, *mut *mut c_void) -> c_int>;

unsafe extern "C" {
    fn xmlInitParser();
    static xmlFree: Option<unsafe extern "C" fn(*mut c_void)>;
    fn xmlGetNodePath(node: *const c_void) -> *mut c_char;
    fn xmlNewParserCtxt() -> *mut c_void;
    fn xmlFreeParserCtxt(context: *mut c_void);
    fn xmlCtxtSetErrorHandler(context: *mut c_void, handler: ErrorHandler, data: *mut c_void);
    fn xmlCtxtSetResourceLoader(context: *mut c_void, loader: Loader, data: *mut c_void);
    fn xmlCtxtReadMemory(
        context: *mut c_void,
        buffer: *const c_char,
        size: c_int,
        url: *const c_char,
        encoding: *const c_char,
        options: c_int,
    ) -> *mut c_void;
    fn xmlFreeDoc(doc: *mut c_void);
    fn xmlNewInputFromString(url: *const c_char, text: *const c_char, flags: c_int) -> *mut c_void;
    fn xmlSchemaNewParserCtxt(url: *const c_char) -> *mut c_void;
    fn xmlSchemaSetParserStructuredErrors(context: *mut c_void, handler: ErrorHandler, data: *mut c_void);
    fn xmlSchemaSetResourceLoader(context: *mut c_void, loader: Loader, data: *mut c_void) -> c_int;
    fn xmlSchemaParse(context: *mut c_void) -> *mut c_void;
    fn xmlSchemaFreeParserCtxt(context: *mut c_void);
    fn xmlSchemaFree(schema: *mut c_void);
    fn xmlSchemaNewValidCtxt(schema: *mut c_void) -> *mut c_void;
    fn xmlSchemaSetValidStructuredErrors(context: *mut c_void, handler: ErrorHandler, data: *mut c_void);
    fn xmlSchemaValidateDoc(context: *mut c_void, doc: *mut c_void) -> c_int;
    fn xmlSchemaFreeValidCtxt(context: *mut c_void);
}
