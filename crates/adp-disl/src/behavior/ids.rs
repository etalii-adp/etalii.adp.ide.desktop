//! Where the id of an element a `create` makes comes from (DISL section 11.5.1; the standalone host's `IIdSource`
//! and `DislIds`): the specification's `persistence.ids` strategy for a runtime, or a fixed list for a test or for a
//! host that minted the id already, so that a redone add keeps it.

use std::cell::Cell;

use crate::json;
use crate::loading::specification::DislSpecification;

/// The source of the ids new elements get.
pub trait IdSource {
    /// The id of the next element of `type_name`.
    fn next(&self, type_name: &str) -> Result<String, String>;
}

/// A source that gives its ids in order, and refuses a create past the last.
pub struct FixedIds {
    ids: Vec<String>,
    next: Cell<usize>,
}

impl IdSource for FixedIds {
    fn next(&self, type_name: &str) -> Result<String, String> {
        let index = self.next.get();
        match self.ids.get(index) {
            Some(id) => {
                self.next.set(index + 1);
                Ok(id.clone())
            }
            None => Err(format!("No id is left for a new {type_name}.")),
        }
    }
}

/// A source that gives `ids` in order.
pub fn fixed(ids: &[&str]) -> FixedIds {
    FixedIds {
        ids: ids.iter().map(|i| (*i).to_owned()).collect(),
        next: Cell::new(0),
    }
}

/// The strategies a runtime can mint without the model (section 11.5.1): `uuid-v4` and `uuid-v7` in an encoding, and
/// `nanoid` of an alphabet and a length, each after the rule's `prefix`.
pub struct Uuids {
    strategy: Minted,
    prefix: Prefix,
}

enum Minted {
    Uuid { v7: bool, encoding: String },
    Nanoid { alphabet: Vec<char>, length: usize },
}

/// A rule's `prefix`: one for every type, or one per type name.
enum Prefix {
    None,
    All(String),
    ByType(Vec<(String, String)>),
}

impl Prefix {
    fn of(&self, type_name: &str) -> &str {
        match self {
            Prefix::None => "",
            Prefix::All(prefix) => prefix,
            Prefix::ByType(prefixes) => prefixes
                .iter()
                .find(|(t, _)| t == type_name)
                .map_or("", |(_, prefix)| prefix.as_str()),
        }
    }
}

impl IdSource for Uuids {
    fn next(&self, type_name: &str) -> Result<String, String> {
        let id = match &self.strategy {
            Minted::Uuid { v7, encoding } => {
                let uuid = if *v7 {
                    uuid::Uuid::now_v7()
                } else {
                    uuid::Uuid::new_v4()
                };
                match encoding.as_str() {
                    "base36" => base36(uuid.as_u128()),
                    "base64url" => base64url(uuid.as_bytes()),
                    _ => uuid.hyphenated().to_string(),
                }
            }
            Minted::Nanoid { alphabet, length } => nanoid(alphabet, *length),
        };
        Ok(format!("{}{id}", self.prefix.of(type_name)))
    }
}

/// nanoid's own alphabet of 64 characters (DISL 0.12, section 11.5).
const NANOID: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
/// The alphabet `base36`: the digits and the lower-case letters.
const BASE36: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

/// `length` characters of `alphabet`, each drawn uniformly from a cryptographically secure source (section 11.5): the
/// random bytes of version 4 uuids, a byte that would favour the first characters of the alphabet drawn again.
fn nanoid(alphabet: &[char], length: usize) -> String {
    let usable = 256 - 256 % alphabet.len();
    let mut id = String::with_capacity(length);
    let mut drawn = 0;
    while drawn < length {
        let uuid = uuid::Uuid::new_v4();
        // Bytes 6 and 8 hold the version and the variant; the others are random.
        for (_, byte) in uuid.as_bytes().iter().enumerate().filter(|(i, _)| *i != 6 && *i != 8) {
            if usize::from(*byte) < usable && drawn < length {
                id.push(alphabet[usize::from(*byte) % alphabet.len()]);
                drawn += 1;
            }
        }
    }
    id
}

/// The specification's own strategy (section 11.5.1), for the strategies a runtime can mint without the model:
/// `uuid-v4` and `uuid-v7`, in `hex`, `base64url` or `base36`, and `nanoid` of `nanoid`'s alphabet or `base36`, 21
/// characters unless `length` says otherwise; each after the rule's `prefix`, for every type or per type.
pub fn of(specification: &DislSpecification) -> Result<Uuids, String> {
    let ids = json::member(json::member(specification.root(), "persistence"), "ids");
    let strategy = json::string_of(ids, "strategy").unwrap_or("uuid-v7");
    let prefix = match json::get(ids, "prefix") {
        Some(serde_json::Value::String(prefix)) => Prefix::All(prefix.clone()),
        Some(serde_json::Value::Object(prefixes)) => Prefix::ByType(
            prefixes
                .iter()
                .filter_map(|(t, p)| p.as_str().map(|p| (t.clone(), p.to_owned())))
                .collect(),
        ),
        _ => Prefix::None,
    };
    let strategy = match strategy {
        "uuid-v4" | "uuid-v7" => Minted::Uuid {
            v7: strategy == "uuid-v7",
            encoding: json::string_of(ids, "encoding").unwrap_or("hex").to_owned(),
        },
        "nanoid" => Minted::Nanoid {
            alphabet: match json::string_of(ids, "alphabet").unwrap_or("nanoid") {
                "base36" => BASE36,
                _ => NANOID,
            }
            .chars()
            .collect(),
            length: json::get(ids, "length")
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| *n > 0)
                .unwrap_or(21),
        },
        _ => {
            return Err(format!(
                "This runtime cannot mint ids of strategy '{strategy}' without the model; give the interpreter an id source."
            ));
        }
    };
    Ok(Uuids { strategy, prefix })
}

/// ShortGuids: version 4 GUIDs as 25 characters of base 36, the ids the standalone host's modules minted for what
/// they created (`ShortGuid.NewShortGuid()`), for a module that mints its own.
pub fn short_guids() -> Uuids {
    Uuids {
        strategy: Minted::Uuid {
            v7: false,
            encoding: "base36".to_owned(),
        },
        prefix: Prefix::None,
    }
}

/// The 128-bit value as an unsigned integer in lowercase base 36, left-padded with 0 to 25 characters.
fn base36(mut value: u128) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut text = [b'0'; 25];
    for slot in text.iter_mut().rev() {
        *slot = ALPHABET[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8_lossy(&text).into_owned()
}

/// Base64url without padding.
fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        let symbols = chunk.len() + 1;
        for i in 0..symbols {
            text.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    text
}
