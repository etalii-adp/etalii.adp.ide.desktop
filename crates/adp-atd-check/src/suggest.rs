//! The closest names a finding offers (ATD §15.5): names of the namespace in scope within an edit distance of 2 or
//! sharing a prefix of 3, at most five, nearest first.

pub fn closest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut scored: Vec<(usize, &str)> = Vec::new();
    for candidate in candidates {
        if candidate == name {
            continue;
        }
        let distance = levenshtein(&name.to_lowercase(), &candidate.to_lowercase());
        let prefix = name.chars().zip(candidate.chars()).take_while(|(a, b)| a == b).count();
        if distance <= 2 || prefix >= 3 {
            scored.push((distance, candidate));
        }
    }
    scored.sort();
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(5).map(|(_, c)| c.to_owned()).collect()
}

/// The edit distance with transpositions of adjacent characters counted as one edit.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// "Did you mean `a` or `b`?" for a message, or nothing.
pub fn did_you_mean(suggestions: &[String]) -> String {
    match suggestions {
        [] => String::new(),
        [one] => format!(" Did you mean `{one}`?"),
        [rest @ .., last] => {
            let quoted: Vec<String> = rest.iter().map(|s| format!("`{s}`")).collect();
            format!(" Did you mean {} or `{last}`?", quoted.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_names_and_shared_prefixes_are_offered() {
        assert_eq!(closest("titel", ["title", "tier", "name"]), vec!["title", "tier"]);
        assert_eq!(
            closest("layout.radialTree", ["layout.twoSidedTree", "reader.turtle"]),
            vec!["layout.twoSidedTree"]
        );
        assert!(closest("zzz", ["title"]).is_empty());
    }
}
