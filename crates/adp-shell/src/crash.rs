//! What a failed start leaves behind. The program has no console on Windows (`windows_subsystem = "windows"`), so a
//! panic or a window that cannot be made would otherwise close it without a word: the reason is appended to
//! `crash.log` in the state folder, or in the system's temporary folder when there is none, and shown in a message box
//! that names that file.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The name of the file a failed start is written to.
pub const LOG_NAME: &str = "crash.log";

/// Where a failed start is written: in the state folder when there is one, else in the system's temporary folder.
pub fn log_file(state: Option<&Path>) -> PathBuf {
    match state {
        Some(folder) => folder.join(LOG_NAME),
        None => std::env::temp_dir().join("adp-desktop").join(LOG_NAME),
    }
}

/// The report of one failure: when, which version, and what went wrong.
pub fn report_text(reason: &str) -> String {
    let since_1970 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!(
        "--- adp-desktop {} on {}, {since_1970} seconds since 1970\n{reason}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
    )
}

/// Appends the report of `reason` to `file`, creating its folder; whether it could be written.
pub fn write_report(file: &Path, reason: &str) -> bool {
    if let Some(folder) = file.parent()
        && std::fs::create_dir_all(folder).is_err()
    {
        return false;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .and_then(|mut f| f.write_all(report_text(reason).as_bytes()))
        .is_ok()
}

/// The text of the message box: the reason, and where it was written.
pub fn message(reason: &str, file: &Path, written: bool) -> String {
    if written {
        format!(
            "ADP could not continue.\n\n{reason}\n\nThe details are in {}.",
            file.display()
        )
    } else {
        format!("ADP could not continue.\n\n{reason}")
    }
}

/// Writes the report and tells the person, in a message box, unless `quiet` (the start-up probe and the tests).
pub fn report(file: &Path, reason: &str, quiet: bool) {
    let written = write_report(file, reason);
    eprintln!("{}", message(reason, file, written));
    if !quiet {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("ADP")
            .set_description(message(reason, file, written))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

/// Reports every panic as [`report`] does, with its place and a backtrace, before the default hook runs. The message
/// box is shown only for a panic of the main thread, the one the platform lets open a window.
pub fn install_panic_hook(file: PathBuf, quiet: bool) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let reason = panic_reason(info);
        let on_main = std::thread::current().name() == Some("main");
        report(&file, &reason, quiet || !on_main);
        default(info);
    }));
}

/// A panic as the log keeps it: its message, its place, the thread and a backtrace.
fn panic_reason(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic without a message".to_owned());
    let place = info
        .location()
        .map(|l| format!(" at {}:{}", l.file(), l.line()))
        .unwrap_or_default();
    let thread = std::thread::current().name().unwrap_or("unnamed").to_owned();
    format!(
        "The program stopped on thread '{thread}'{place}: {payload}\n\n{}",
        std::backtrace::Backtrace::force_capture()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_appended_in_a_folder_made_for_it() {
        let folder = tempfile::tempdir().unwrap();
        let file = log_file(Some(&folder.path().join("not yet made")));
        assert!(write_report(&file, "first"));
        assert!(write_report(&file, "second"));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("first\n") && text.contains("second\n"), "{text}");
        assert_eq!(text.matches("--- adp-desktop").count(), 2);
    }

    #[test]
    fn without_a_state_folder_the_report_goes_to_the_temporary_folder() {
        assert!(log_file(None).starts_with(std::env::temp_dir()));
    }

    #[test]
    fn the_message_names_the_file_only_when_it_was_written() {
        let file = Path::new("state").join(LOG_NAME);
        assert!(message("no adapter", &file, true).contains(&file.display().to_string()));
        assert!(!message("no adapter", &file, false).contains(LOG_NAME));
    }

    #[test]
    fn a_panic_is_written_with_its_place() {
        let folder = tempfile::tempdir().unwrap();
        let file = log_file(Some(folder.path()));
        let previous = std::panic::take_hook();
        install_panic_hook(file.clone(), true);
        let outcome = std::thread::Builder::new()
            .name("probe".into())
            .spawn(|| panic!("the probe panics"))
            .unwrap()
            .join();
        std::panic::set_hook(previous);
        assert!(outcome.is_err());
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            text.contains("thread 'probe'") && text.contains("the probe panics"),
            "{text}"
        );
        assert!(text.contains("crash.rs"), "{text}");
    }
}
