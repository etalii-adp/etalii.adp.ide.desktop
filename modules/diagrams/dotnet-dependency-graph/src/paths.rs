//! Paths as the standalone host's readers spelled and compared them (etalii.adp spec 030, task T082):
//! `Path.GetFullPath` resolving `.` and `..` without touching the disk, `Path.GetRelativePath` forward-slashed, the
//! ordinal comparison that ignores case where the platform's file system does (`StringComparer.OrdinalIgnoreCase` for a
//! path), and the `*` and `?` of `Directory.EnumerateFiles`' search pattern.

use std::cmp::Ordering;
use std::path::{Component, Path, PathBuf};

/// Whether the platform's file system ignores case, as .NET takes it: Windows and macOS do.
pub fn platform_ignores_case() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

/// `path` made absolute, with `.` and `..` resolved without touching the file system (`Path.GetFullPath`).
pub fn normalise(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The key a path is compared by: its text, case folded where the platform ignores case.
pub fn key(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if platform_ignores_case() {
        text.to_lowercase()
    } else {
        text
    }
}

/// Whether two paths name one file.
pub fn same(a: &Path, b: &Path) -> bool {
    key(a) == key(b)
}

/// `path` relative to `folder`, both absolute, with `/` between its parts on every platform (`Path.GetRelativePath`,
/// then `Replace('\\', '/')`).
pub fn relative(folder: &Path, path: &Path) -> String {
    let from: Vec<String> = parts(folder);
    let to: Vec<String> = parts(path);
    let equal = |a: &String, b: &String| {
        if platform_ignores_case() {
            a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    };
    let common = from.iter().zip(&to).take_while(|(a, b)| equal(a, b)).count();
    let mut out: Vec<String> = std::iter::repeat_n("..".to_owned(), from.len() - common).collect();
    out.extend(to[common..].iter().cloned());
    if out.is_empty() { ".".to_owned() } else { out.join("/") }
}

fn parts(path: &Path) -> Vec<String> {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// `text` with every `\` and `/` turned into the platform's separator, as the readers normalised a path a file wrote.
pub fn with_platform_separators(text: &str) -> String {
    text.replace(['\\', '/'], std::path::MAIN_SEPARATOR_STR)
}

/// Two texts compared as `StringComparer.OrdinalIgnoreCase` compares them: character by character, each taken to its
/// upper case.
pub fn compare_ignoring_case(a: &str, b: &str) -> Ordering {
    let upper = |c: char| {
        let mut upper = c.to_uppercase();
        match (upper.next(), upper.next()) {
            (Some(u), None) => u,
            _ => c,
        }
    };
    a.chars().map(upper).cmp(b.chars().map(upper))
}

/// Whether two texts are equal ignoring case (`StringComparison.OrdinalIgnoreCase`).
pub fn equal_ignoring_case(a: &str, b: &str) -> bool {
    compare_ignoring_case(a, b) == Ordering::Equal
}

/// Whether the file or folder name `name` matches the search pattern `pattern`: `*` any run of characters, `?` any one,
/// case ignored where the platform ignores it.
pub fn matches(pattern: &str, name: &str) -> bool {
    let fold = |text: &str| -> Vec<char> {
        if platform_ignores_case() {
            text.to_lowercase().chars().collect()
        } else {
            text.chars().collect()
        }
    };
    let (pattern, name) = (fold(pattern), fold(name));
    let (mut p, mut n) = (0, 0);
    let (mut star, mut resume) = (None, 0);
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            resume = n;
            p += 1;
        } else if let Some(s) = star {
            p = s + 1;
            resume += 1;
            n = resume;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_matches_as_a_search_pattern_does() {
        assert!(matches("*.csproj", "App.csproj"));
        assert!(!matches("*.csproj", "App.csproj.user"));
        assert!(matches("mod?", "modA"));
        assert!(matches("*", "anything"));
        assert!(!matches("a*b", "ac"));
    }

    #[test]
    fn a_relative_path_walks_up_where_it_has_to() {
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/b/c/d.csproj")), "c/d.csproj");
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/x/d.csproj")), "../x/d.csproj");
    }

    #[test]
    fn texts_are_ordered_ignoring_case() {
        assert_eq!(compare_ignoring_case("serilog", "System"), Ordering::Less);
        assert_eq!(
            compare_ignoring_case("Pipeline.Core.Tests", "pipeline.storage"),
            Ordering::Less
        );
        assert!(equal_ignoring_case("Serilog", "SERILOG"));
    }
}
