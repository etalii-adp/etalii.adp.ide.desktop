//! The desktop host's window (spec 030). Until task T021 starts the shell, it states what it is and exits.

fn main() {
    println!("{}", banner());
}

fn banner() -> String {
    format!(
        "ADP desktop host {}: no window yet (etalii.adp spec 030, task T021).",
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::banner;

    #[test]
    fn banner_names_the_host_and_its_version() {
        let text = banner();
        assert!(text.starts_with("ADP desktop host "));
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }
}
