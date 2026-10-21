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

/// The uuid strategies a runtime can mint without the model, in an encoding.
pub struct Uuids {
    v7: bool,
    encoding: String,
}

impl IdSource for Uuids {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        let uuid = if self.v7 {
            uuid::Uuid::now_v7()
        } else {
            uuid::Uuid::new_v4()
        };
        Ok(match self.encoding.as_str() {
            "base36" => base36(uuid.as_u128()),
            "base64url" => base64url(uuid.as_bytes()),
            _ => uuid.hyphenated().to_string(),
        })
    }
}

/// The specification's own strategy (section 11.5.1), for the strategies a runtime can mint without the model:
/// `uuid-v4` and `uuid-v7`, in `hex`, `base64url` or `base36`.
pub fn of(specification: &DislSpecification) -> Result<Uuids, String> {
    let ids = json::member(json::member(specification.root(), "persistence"), "ids");
    let strategy = json::string_of(ids, "strategy").unwrap_or("uuid-v7");
    let encoding = json::string_of(ids, "encoding").unwrap_or("hex").to_owned();
    match strategy {
        "uuid-v4" => Ok(Uuids { v7: false, encoding }),
        "uuid-v7" => Ok(Uuids { v7: true, encoding }),
        _ => Err(format!(
            "This runtime cannot mint ids of strategy '{strategy}' without the model; give the interpreter an id source."
        )),
    }
}

/// ShortGuids: version 4 GUIDs as 25 characters of base 36, the ids the standalone host's modules minted for what
/// they created (`ShortGuid.NewShortGuid()`), for a module that mints its own.
pub fn short_guids() -> Uuids {
    Uuids {
        v7: false,
        encoding: "base36".to_owned(),
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
