//! New ids by DISL's and DESL's `nanoid` strategy (spec 089, FR-001 and FR-003): random characters of an alphabet,
//! every character equally likely.

/// The alphabet `base36`: the digits and the lower-case letters.
pub const BASE36: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

/// A new id of `length` characters of `alphabet`, each drawn uniformly: a random byte that would favour the first
/// characters of the alphabet is drawn again.
pub fn nanoid(alphabet: &str, length: usize) -> String {
    let characters: Vec<char> = alphabet.chars().collect();
    assert!(
        !characters.is_empty() && characters.len() <= 256,
        "an alphabet of 1 to 256 characters"
    );
    let usable = 256 - 256 % characters.len();
    let mut id = String::with_capacity(length);
    let mut bytes = [0u8; 32];
    while id.chars().count() < length {
        getrandom::fill(&mut bytes).expect("the operating system's randomness");
        for byte in bytes {
            if (byte as usize) < usable && id.chars().count() < length {
                id.push(characters[byte as usize % characters.len()]);
            }
        }
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_has_its_length_and_only_its_alphabet() {
        for _ in 0..100 {
            let id = nanoid(BASE36, 8);
            assert_eq!(8, id.len());
            assert!(id.chars().all(|c| BASE36.contains(c)), "{id}");
        }
    }

    #[test]
    fn two_ids_differ() {
        assert_ne!(nanoid(BASE36, 8), nanoid(BASE36, 8));
    }

    #[test]
    fn every_character_is_drawn() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            seen.extend(nanoid(BASE36, 8).chars());
        }
        assert_eq!(36, seen.len());
    }
}
