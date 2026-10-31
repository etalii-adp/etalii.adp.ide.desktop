//! Asks the platform to open a file with the person's own program for it (etalii.adp spec 074, FR-005, FR-024, R4). The host
//! embeds no browser and needs none: it starts the platform's opener and does not wait for what the opener starts.

use std::io;
use std::path::Path;
use std::process::Command;

/// The program that opens a file on the platform `os` (as `std::env::consts::OS` names it).
fn opener(os: &str) -> &'static str {
    match os {
        "windows" => "explorer",
        "macos" => "open",
        _ => "xdg-open",
    }
}

/// The command that opens `path` on the platform `os`.
fn command_for(os: &str, path: &Path) -> Command {
    let mut command = Command::new(opener(os));
    command.arg(path);
    command
}

/// Starts the platform's opener on `path`; an error when it is missing or will not start.
pub fn open(path: &Path) -> io::Result<()> {
    let mut child = command_for(std::env::consts::OS, path).spawn()?;
    // Collected on the side, so that an opener that ends leaves nothing behind.
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_platform_has_its_own_opener_given_the_whole_path_as_one_argument() {
        let path = Path::new("/a folder/mental-health {2026-10-31 14.05}.html");
        for (os, program) in [
            ("windows", "explorer"),
            ("macos", "open"),
            ("linux", "xdg-open"),
            ("freebsd", "xdg-open"),
        ] {
            let command = command_for(os, path);
            assert_eq!(command.get_program(), program);
            assert_eq!(command.get_args().collect::<Vec<_>>(), [path.as_os_str()]);
        }
    }
}
