//! What was read from a body's bytes, kept so it is not read again (etalii.adp spec 030, SC-006); ported from the
//! standalone host's `EtAlii.Adp.Diagram.Modules/TextMemo.cs` and `RecentBodies.cs` at `8b6ccfc`.
//!
//! A [`TextMemo`] keeps the last reading with the bytes it was read from, compared by identity: a body's bytes are
//! never changed in place, so the same allocation is the same text and an edit, which makes new bytes, makes the memo
//! stale by itself. [`RecentBodies`] keeps the readings of the last few texts by what they hold: reading is a pure
//! function of the bytes, so a host asked for the same text in turn - by its document, its validator, its menus and a
//! command - shares the reading rather than reading it once more. The standalone host kept forks of open bodies
//! there; a reading here is shared behind an `Rc`, which nothing edits.

use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// The last value read from a body's bytes.
pub struct TextMemo<T> {
    last: Option<(Rc<[u8]>, T)>,
}

impl<T> Default for TextMemo<T> {
    fn default() -> Self {
        TextMemo { last: None }
    }
}

impl<T: Clone> TextMemo<T> {
    /// What `read` gives for `bytes`, read only when these bytes have not been read before.
    pub fn of(&mut self, bytes: &Rc<[u8]>, read: impl FnOnce() -> T) -> T {
        match &self.last {
            Some((last, value)) if Rc::ptr_eq(last, bytes) => value.clone(),
            _ => {
                let value = read();
                self.last = Some((bytes.clone(), value.clone()));
                value
            }
        }
    }
}

/// The readings of the last few texts read or written, by their bytes.
pub struct RecentBodies<T> {
    entries: VecDeque<(u64, Rc<[u8]>, Rc<T>)>,
}

impl<T> Default for RecentBodies<T> {
    fn default() -> Self {
        RecentBodies {
            entries: VecDeque::new(),
        }
    }
}

impl<T> RecentBodies<T> {
    /// How many texts are kept.
    pub const CAPACITY: usize = 8;

    /// The reading remembered for `bytes`, now the most recent; none when none is.
    pub fn find(&mut self, bytes: &[u8]) -> Option<Rc<T>> {
        let hash = hash(bytes);
        let at = self
            .entries
            .iter()
            .position(|(h, b, _)| *h == hash && b.as_ref() == bytes)?;
        let entry = self.entries.remove(at)?;
        let found = entry.2.clone();
        self.entries.push_front(entry);
        Some(found)
    }

    /// Remembers `reading` of `bytes` as the most recent, in place of one with the same bytes, forgetting the oldest
    /// beyond the few kept.
    pub fn remember(&mut self, bytes: Rc<[u8]>, reading: Rc<T>) {
        let hash = hash(&bytes);
        self.entries.retain(|(h, b, _)| !(*h == hash && *b == bytes));
        self.entries.push_front((hash, bytes, reading));
        self.entries.truncate(Self::CAPACITY);
    }
}

fn hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn the_same_bytes_are_read_once() {
        let mut memo = TextMemo::default();
        let bytes: Rc<[u8]> = Rc::from(&b"a"[..]);
        let reads = Cell::new(0);
        let mut read = || {
            reads.set(reads.get() + 1);
            format!("read {}", reads.get())
        };
        let first = memo.of(&bytes, &mut read);
        let second = memo.of(&bytes, &mut read);
        assert_eq!((first.as_str(), second.as_str(), reads.get()), ("read 1", "read 1", 1));
    }

    #[test]
    fn new_bytes_are_read_again_even_when_they_hold_the_same_text() {
        let mut memo = TextMemo::default();
        let reads = Cell::new(0);
        let mut read = || {
            reads.set(reads.get() + 1);
            format!("read {}", reads.get())
        };
        let before = memo.of(&Rc::from(&b"a"[..]), &mut read);
        let after = memo.of(&Rc::from(&b"a"[..]), &mut read);
        assert_eq!((before.as_str(), after.as_str()), ("read 1", "read 2"));
    }

    #[test]
    fn a_remembered_reading_is_found_by_its_text_in_other_bytes() {
        let mut recent = RecentBodies::default();
        let reading = Rc::new("items: []");
        recent.remember(Rc::from(&b"items: []\n"[..]), reading.clone());
        let found = recent.find(b"items: []\n").expect("found");
        assert!(Rc::ptr_eq(&found, &reading));
        assert!(recent.find(b"items: [] \n").is_none());
    }

    #[test]
    fn the_oldest_is_forgotten_beyond_the_last_eight() {
        let mut recent = RecentBodies::default();
        let texts: Vec<String> = (0..9).map(|i| format!("# {i}\nitems: []\n")).collect();
        for text in &texts {
            recent.remember(Rc::from(text.as_bytes()), Rc::new(text.clone()));
        }
        assert!(recent.find(texts[0].as_bytes()).is_none());
        for text in &texts[1..] {
            assert!(recent.find(text.as_bytes()).is_some(), "{text}");
        }
    }
}
