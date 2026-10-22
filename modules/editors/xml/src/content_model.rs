//! A DTD element's content model, as libxml2 writes it in its message (`(title , ingredient+ , step+)`), read into a
//! position automaton, so that a content error can be said as the standalone host's validator says it: the first
//! child that does not fit and what was expected there, or what is still missing at the end.

/// Where an element's content first leaves its model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Departure {
    /// The child at this index (of the items given) is not allowed there; the names that were.
    Child(usize, Vec<String>),
    /// Text at this index is not allowed there; the names that were.
    Text(usize, Vec<String>),
    /// The content ended early; the names that could come next.
    Incomplete(Vec<String>),
}

/// One item of an element's content: a child element by name, or text that is not only white space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Element(String),
    Text,
}

/// A parsed content model.
pub struct Model {
    /// The element name of each position.
    names: Vec<String>,
    first: Vec<usize>,
    last: Vec<usize>,
    follow: Vec<Vec<usize>>,
    nullable: bool,
    mixed: bool,
}

enum Particle {
    Name(String),
    Text,
    Sequence(Vec<Particle>),
    Choice(Vec<Particle>),
    Optional(Box<Particle>),
    Any(Box<Particle>),
    Some(Box<Particle>),
}

impl Model {
    /// Reads a model such as `(a , (b | c)+ , d?)` or `(#PCDATA | a)*`; `None` for one it cannot read.
    pub fn parse(text: &str) -> Option<Self> {
        let tokens = tokens(text);
        let mut at = 0;
        let particle = particle(&tokens, &mut at)?;
        if at != tokens.len() {
            return None;
        }
        let mut model = Self {
            names: Vec::new(),
            first: Vec::new(),
            last: Vec::new(),
            follow: Vec::new(),
            nullable: false,
            mixed: text.contains("#PCDATA"),
        };
        let (nullable, first, last) = model.build(&particle);
        model.nullable = nullable;
        model.first = first;
        model.last = last;
        Some(model)
    }

    /// Where `items` first leave the model, or `None` when they fit it.
    pub fn departure(&self, items: &[Item]) -> Option<Departure> {
        let mut state: Option<Vec<usize>> = None;
        for (index, item) in items.iter().enumerate() {
            let candidates = self.candidates(state.as_deref());
            match item {
                Item::Text if self.mixed => {}
                Item::Text => return Some(Departure::Text(index, self.names_of(&candidates))),
                Item::Element(name) => {
                    let next: Vec<usize> = candidates.iter().copied().filter(|p| &self.names[*p] == name).collect();
                    if next.is_empty() {
                        return Some(Departure::Child(index, self.names_of(&candidates)));
                    }
                    state = Some(next);
                }
            }
        }
        let accepting = match &state {
            None => self.nullable,
            Some(positions) => positions.iter().any(|p| self.last.contains(p)),
        };
        (!accepting).then(|| Departure::Incomplete(self.names_of(&self.candidates(state.as_deref()))))
    }

    fn candidates(&self, state: Option<&[usize]>) -> Vec<usize> {
        let mut candidates = match state {
            None => self.first.clone(),
            Some(positions) => positions.iter().flat_map(|p| self.follow[*p].iter().copied()).collect(),
        };
        candidates.sort_unstable();
        candidates.dedup();
        candidates
    }

    fn names_of(&self, positions: &[usize]) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for position in positions {
            if !names.contains(&self.names[*position]) {
                names.push(self.names[*position].clone());
            }
        }
        names
    }

    /// Glushkov's construction: whether it may be empty, and its first and last positions.
    fn build(&mut self, particle: &Particle) -> (bool, Vec<usize>, Vec<usize>) {
        match particle {
            Particle::Name(name) => {
                let position = self.names.len();
                self.names.push(name.clone());
                self.follow.push(Vec::new());
                (false, vec![position], vec![position])
            }
            Particle::Text => (true, Vec::new(), Vec::new()),
            Particle::Sequence(items) => {
                let (mut nullable, mut first, mut last): (bool, Vec<usize>, Vec<usize>) =
                    (true, Vec::new(), Vec::new());
                for item in items {
                    let (n, f, l) = self.build(item);
                    for p in &last {
                        self.follow[*p].extend(f.iter().copied());
                    }
                    if nullable {
                        first.extend(f.iter().copied());
                    }
                    last = if n { [last, l].concat() } else { l };
                    nullable &= n;
                }
                (nullable, first, last)
            }
            Particle::Choice(items) => {
                let (mut nullable, mut first, mut last) = (false, Vec::new(), Vec::new());
                for item in items {
                    let (n, f, l) = self.build(item);
                    nullable |= n;
                    first.extend(f);
                    last.extend(l);
                }
                (nullable, first, last)
            }
            Particle::Optional(inner) => {
                let (_, f, l) = self.build(inner);
                (true, f, l)
            }
            Particle::Any(inner) | Particle::Some(inner) => {
                let (n, f, l) = self.build(inner);
                for p in &l {
                    self.follow[*p].extend(f.iter().copied());
                }
                (n || matches!(particle, Particle::Any(_)), f, l)
            }
        }
    }
}

fn tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut name = String::new();
    for c in text.chars() {
        if "(),|?*+".contains(c) || c.is_whitespace() {
            if !name.is_empty() {
                tokens.push(std::mem::take(&mut name));
            }
            if !c.is_whitespace() {
                tokens.push(c.to_string());
            }
        } else {
            name.push(c);
        }
    }
    if !name.is_empty() {
        tokens.push(name);
    }
    tokens
}

fn particle(tokens: &[String], at: &mut usize) -> Option<Particle> {
    let token = tokens.get(*at)?;
    *at += 1;
    let base = if token == "(" {
        let mut items = vec![particle(tokens, at)?];
        let mut separator = None;
        loop {
            let next = tokens.get(*at)?;
            *at += 1;
            match next.as_str() {
                ")" => break,
                "," | "|" if separator.as_ref().is_none_or(|s| s == next) => {
                    separator = Some(next.clone());
                    items.push(particle(tokens, at)?);
                }
                _ => return None,
            }
        }
        if separator.as_deref() == Some("|") {
            Particle::Choice(items)
        } else {
            Particle::Sequence(items)
        }
    } else if token == "#PCDATA" {
        Particle::Text
    } else if "(),|?*+".contains(token.as_str()) {
        return None;
    } else {
        Particle::Name(token.clone())
    };
    Some(match tokens.get(*at).map(String::as_str) {
        Some("?") => {
            *at += 1;
            Particle::Optional(Box::new(base))
        }
        Some("*") => {
            *at += 1;
            Particle::Any(Box::new(base))
        }
        Some("+") => {
            *at += 1;
            Particle::Some(Box::new(base))
        }
        _ => base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elements(names: &[&str]) -> Vec<Item> {
        names.iter().map(|n| Item::Element((*n).to_owned())).collect()
    }

    #[test]
    fn the_first_child_out_of_place_is_found_with_what_was_expected() {
        let model = Model::parse("(title , ingredient+ , step+)").unwrap();
        assert_eq!(model.departure(&elements(&["title", "ingredient", "step"])), None);
        assert_eq!(
            model.departure(&elements(&["ingredient", "step"])),
            Some(Departure::Child(0, vec!["title".into()]))
        );
        assert_eq!(
            model.departure(&elements(&["title", "ingredient"])),
            Some(Departure::Incomplete(vec!["ingredient".into(), "step".into()]))
        );
    }

    #[test]
    fn choices_repeats_and_mixed_content_are_read() {
        let model = Model::parse("(a , (b | c)+ , d?)").unwrap();
        assert_eq!(
            model.departure(&elements(&["a"])),
            Some(Departure::Incomplete(vec!["b".into(), "c".into()]))
        );
        assert_eq!(
            model.departure(&elements(&["a", "b", "d", "q"])),
            Some(Departure::Child(3, vec![]))
        );
        let mixed = Model::parse("(#PCDATA | a)*").unwrap();
        assert_eq!(mixed.departure(&[Item::Text, Item::Element("a".into())]), None);
        assert_eq!(
            mixed.departure(&elements(&["b"])),
            Some(Departure::Child(0, vec!["a".into()]))
        );
        let only = Model::parse("(a)").unwrap();
        assert_eq!(
            only.departure(&[Item::Text, Item::Element("a".into())]),
            Some(Departure::Text(0, vec!["a".into()]))
        );
    }
}
