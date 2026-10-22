//! Builds the libxml2 sources in `libxml2/` as a static library with the C compiler `cc` finds, without CMake or any
//! system package, so the same build runs on Windows, macOS and Linux. Only the parser, the tree, DTD validation,
//! regular expressions, patterns and XML Schema 1.0 are compiled; HTTP, FTP, catalogues, XInclude, XPath, HTML,
//! RELAX NG, Schematron, iconv, ICU, zlib and modules are not, so nothing it does can reach the network or load code.

use std::env;
use std::fs;
use std::path::PathBuf;

const SOURCES: &[&str] = &[
    "buf.c",
    "chvalid.c",
    "dict.c",
    "encoding.c",
    "entities.c",
    "error.c",
    "globals.c",
    "hash.c",
    "list.c",
    "parser.c",
    "parserInternals.c",
    "pattern.c",
    "SAX2.c",
    "threads.c",
    "tree.c",
    "uri.c",
    "valid.c",
    "xmlIO.c",
    "xmlmemory.c",
    "xmlregexp.c",
    "xmlschemas.c",
    "xmlschemastypes.c",
    "xmlstring.c",
];

/// The features of `xmlversion.h`: on, or off.
const FEATURES: &[(&str, bool)] = &[
    ("WITH_THREADS", true),
    ("WITH_THREAD_ALLOC", false),
    ("WITH_OUTPUT", false),
    ("WITH_PUSH", false),
    ("WITH_READER", false),
    ("WITH_PATTERN", true),
    ("WITH_WRITER", false),
    ("WITH_SAX1", false),
    ("WITH_HTTP", false),
    ("WITH_VALID", true),
    ("WITH_HTML", false),
    ("WITH_C14N", false),
    ("WITH_CATALOG", false),
    ("WITH_XPATH", false),
    ("WITH_XPTR", false),
    ("WITH_XINCLUDE", false),
    ("WITH_ICONV", false),
    ("WITH_ICU", false),
    ("WITH_ISO8859X", false),
    ("WITH_DEBUG", false),
    ("WITH_REGEXPS", true),
    ("WITH_RELAXNG", false),
    ("WITH_SCHEMAS", true),
    ("WITH_SCHEMATRON", false),
    ("WITH_MODULES", false),
    ("WITH_ZLIB", false),
];

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source = manifest.join("libxml2");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    println!("cargo:rerun-if-changed=libxml2");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");

    // xmlversion.h, from its template, as libxml2's own CMake build writes it.
    let version = fs::read_to_string(source.join("VERSION")).expect("libxml2/VERSION");
    let version = version.trim();
    let parts: Vec<u32> = version
        .split('.')
        .map(|p| p.parse().expect("a version number"))
        .collect();
    let number = parts[0] * 10000 + parts[1] * 100 + parts[2];
    let mut header = fs::read_to_string(source.join("include/libxml/xmlversion.h.in")).expect("xmlversion.h.in");
    header = header
        .replace("@VERSION@", version)
        .replace("@LIBXML_VERSION_NUMBER@", &number.to_string())
        .replace("@LIBXML_VERSION_EXTRA@", "")
        .replace("@MODULE_EXTENSION@", "");
    for (feature, on) in FEATURES {
        header = header.replace(&format!("@{feature}@"), if *on { "1" } else { "0" });
    }
    assert!(!header.contains("#if @"), "a feature of xmlversion.h is not set");
    let generated = out.join("include");
    fs::create_dir_all(generated.join("libxml")).expect("the include folder");
    fs::write(generated.join("libxml/xmlversion.h"), header).expect("xmlversion.h");

    // config.h: what CMake would have found on the platform.
    let unix = target_os != "windows";
    let thread_local = if target_env == "msvc" {
        "__declspec(thread)"
    } else {
        "_Thread_local"
    };
    let config = format!(
        "#define HAVE_DECL_GETENTROPY {}\n#define HAVE_DECL_GLOB 0\n#define HAVE_DECL_MMAP 0\n#define HAVE_STDINT_H 1\n#define XML_THREAD_LOCAL {thread_local}\n",
        u8::from(unix && matches!(target_os.as_str(), "macos" | "linux"))
    );
    fs::write(generated.join("config.h"), config).expect("config.h");

    let mut build = cc::Build::new();
    build
        .include(&generated)
        .include(&source)
        .include(source.join("include"))
        .warnings(false)
        .opt_level(2)
        .define("LIBXML_STATIC", None);
    if target_os == "macos" && env::var_os("MACOSX_DEPLOYMENT_TARGET").is_none() {
        // The oldest macOS Rust links for, so the objects agree with the program they are linked into.
        build.flag("-mmacosx-version-min=11.0");
    }
    if target_env == "msvc" {
        build.define("_CRT_SECURE_NO_WARNINGS", None);
    } else {
        build.flag_if_supported("-std=gnu11");
    }
    for file in SOURCES {
        build.file(source.join(file));
    }
    build.compile("xml2");

    match target_os.as_str() {
        "windows" => println!("cargo:rustc-link-lib=bcrypt"),
        "macos" | "ios" => {}
        _ => {
            println!("cargo:rustc-link-lib=m");
            println!("cargo:rustc-link-lib=pthread");
        }
    }
}
