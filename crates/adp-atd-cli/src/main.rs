//! `atd`, the command line of ATD (etalii.adp spec 035, contracts/checker.md): `check`, `fmt`, `tree` and `tables`. Exit codes: 0 when there is nothing to report, 1 when there is, 2 on a usage or an I/O error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adp_atd_check::{Finding, FsFiles, check, findings_json};
use serde_json::Value;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/modules.rs"));
}

const USAGE: &str = "usage: atd [--spec-dir <dir>] <command> …
  check [--json] <path>…                     check every description whose files are among the paths
  check --expect <findings.json> <file>      compare a file's findings with the expected ones
  check --corpus <dir>                       compare every <name>.atd of a folder with <name>.findings.json
  fmt [--check] <path>…                      write files in the canonical layout
  tree <file>                                print a file's abstract tree
  tables [--grammar <file.lark>]             print the normalized parse tables
  --version                                  print this tool's version and the versions of ATD it reads";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut spec_dir: Option<PathBuf> = None;
    if let Some(i) = args.iter().position(|a| a == "--spec-dir") {
        if i + 1 >= args.len() {
            return usage("--spec-dir needs a folder");
        }
        spec_dir = Some(PathBuf::from(args.remove(i + 1)));
        args.remove(i);
    }
    if args.iter().any(|a| a == "--version") {
        println!(
            "atd {} (ATD {})",
            env!("CARGO_PKG_VERSION"),
            adp_atd_syntax::VERSIONS.join(", ")
        );
        return ExitCode::SUCCESS;
    }
    let Some(command) = args.first().cloned() else {
        return usage("name a command");
    };
    let rest = &args[1..];
    match command.as_str() {
        "check" => check_command(rest),
        "tree" => tree_command(rest),
        "tables" => tables_command(rest, spec_dir.as_deref()),
        "fmt" => fmt_command(rest),
        other => usage(&format!("`{other}` is not a command")),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("atd: {problem}\n{USAGE}");
    ExitCode::from(2)
}

/// The `.atd` files of the paths: files as given, folders searched.
fn atd_files(paths: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for path in paths {
        let path = PathBuf::from(path);
        if path.is_dir() {
            collect(&path, &mut out);
        } else if path.is_file() {
            out.push(path);
        } else {
            return Err(format!("{} is not there", path.display()));
        }
    }
    Ok(out)
}

fn collect(folder: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|e| e.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "atd") {
            out.push(path);
        }
    }
}

fn check_command(args: &[String]) -> ExitCode {
    let json = args.iter().any(|a| a == "--json");
    if let Some(i) = args.iter().position(|a| a == "--expect") {
        let (Some(expected), Some(file)) = (args.get(i + 1), args.iter().skip(i + 2).find(|a| !a.starts_with("--")))
        else {
            return usage("--expect needs a findings file and a description");
        };
        return match expect(Path::new(expected), Path::new(file)) {
            Ok(differences) if differences.is_empty() => ExitCode::SUCCESS,
            Ok(differences) => {
                for line in differences {
                    println!("{line}");
                }
                ExitCode::from(1)
            }
            Err(problem) => usage(&problem),
        };
    }
    if let Some(i) = args.iter().position(|a| a == "--corpus") {
        let Some(folder) = args.get(i + 1) else {
            return usage("--corpus needs a folder");
        };
        return corpus(Path::new(folder));
    }
    let paths: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    if paths.is_empty() {
        return usage("check needs a path");
    }
    let files = match atd_files(&paths) {
        Ok(files) => files,
        Err(problem) => return usage(&problem),
    };
    let findings = check_all(&files);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&findings_json(&findings)).unwrap_or_default()
        );
    } else {
        for finding in &findings {
            println!("{}", finding.to_line());
        }
    }
    if findings.iter().any(|f| f.severity == adp_atd_check::Severity::Error) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// The findings of every description the files belong to, each description once.
fn check_all(files: &[PathBuf]) -> Vec<Finding> {
    let mut done: Vec<PathBuf> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    for file in files {
        if done.contains(file) {
            continue;
        }
        let checked = check(&FsFiles, file);
        for unit in &checked.description.files {
            done.push(unit.path.clone());
        }
        for finding in checked.findings {
            if !findings.contains(&finding) {
                findings.push(finding);
            }
        }
    }
    findings.sort_by(|a, b| (&a.file, a.line, a.column, a.code).cmp(&(&b.file, b.line, b.column, b.code)));
    findings
}

type Key = (String, String, String, u64, u64);

fn basename(file: &str) -> String {
    file.rsplit(['/', '\\']).next().unwrap_or(file).to_owned()
}

/// The differences between a file's findings and the expected ones, by code, severity, file, line and column.
fn expect(expected: &Path, file: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(expected).map_err(|e| format!("{}: {e}", expected.display()))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", expected.display()))?;
    let mut wanted: Vec<Key> = value["findings"]
        .as_array()
        .ok_or(format!("{} has no findings", expected.display()))?
        .iter()
        .map(|f| {
            (
                f["code"].as_str().unwrap_or("").to_owned(),
                f["severity"].as_str().unwrap_or("").to_owned(),
                basename(f["file"].as_str().unwrap_or("")),
                f["line"].as_u64().unwrap_or(0),
                f["column"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    if !file.is_file() {
        return Err(format!("{} is not there", file.display()));
    }
    let checked = check(&FsFiles, file);
    let mut found: Vec<Key> = checked
        .findings
        .iter()
        .map(|f| {
            (
                f.code.to_owned(),
                f.severity.as_str().to_owned(),
                basename(&f.file),
                f.line as u64,
                f.column as u64,
            )
        })
        .collect();
    wanted.sort();
    found.sort();
    let mut differences = Vec::new();
    for missing in wanted.iter().filter(|w| !found.contains(w)) {
        differences.push(format!(
            "{}: missing {} {} at {}:{}:{}",
            file.display(),
            missing.1,
            missing.0,
            missing.2,
            missing.3,
            missing.4
        ));
    }
    for extra in found.iter().filter(|f| !wanted.contains(f)) {
        let message = checked
            .findings
            .iter()
            .find(|f| f.code == extra.0 && f.line as u64 == extra.3 && f.column as u64 == extra.4)
            .map(|f| f.message.clone())
            .unwrap_or_default();
        differences.push(format!(
            "{}: unexpected {} {} at {}:{}:{}: {message}",
            file.display(),
            extra.1,
            extra.0,
            extra.2,
            extra.3,
            extra.4
        ));
    }
    Ok(differences)
}

fn corpus(folder: &Path) -> ExitCode {
    let mut files = Vec::new();
    collect(folder, &mut files);
    let mut failed = 0;
    let mut checked = 0;
    for file in &files {
        let expected = file.with_extension("findings.json");
        if !expected.is_file() {
            continue;
        }
        checked += 1;
        match expect(&expected, file) {
            Ok(differences) if differences.is_empty() => println!("ok   {}", file.display()),
            Ok(differences) => {
                failed += 1;
                println!("FAIL {}", file.display());
                for line in differences {
                    println!("  {line}");
                }
            }
            Err(problem) => {
                failed += 1;
                println!("FAIL {}: {problem}", file.display());
            }
        }
    }
    println!("{checked} refused description(s), {failed} with other findings than expected.");
    if failed == 0 && checked > 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn tree_command(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else {
        return usage("tree needs a file");
    };
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) => return usage(&format!("{file}: {e}")),
    };
    let parse = adp_atd_syntax::parse(&text);
    let name = Path::new(file)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    match adp_atd_syntax::lower(&parse, &name) {
        Ok(tree) => {
            println!("{}", serde_json::to_string_pretty(&tree).unwrap_or_default());
            if parse.errors.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("{file}: {}", error.message);
            ExitCode::from(1)
        }
    }
}

fn tables_command(args: &[String], spec_dir: Option<&Path>) -> ExitCode {
    let built = if let Some(i) = args.iter().position(|a| a == "--grammar") {
        let Some(file) = args.get(i + 1) else {
            return usage("--grammar needs a file");
        };
        match std::fs::read_to_string(file) {
            Ok(text) => adp_atd_grammar::build_single(&text, "start"),
            Err(e) => return usage(&format!("{file}: {e}")),
        }
    } else if let Some(dir) = spec_dir {
        adp_atd_grammar::build_dir(&dir.join("grammar"), "atd", "description")
    } else {
        let load = |module: &str| {
            embedded::MODULES
                .iter()
                .find(|(m, _)| *m == module)
                .map(|(_, text)| (*text).to_owned())
        };
        adp_atd_grammar::build_with("atd", "description", &load)
    };
    match built {
        Ok(built) => {
            print!(
                "{}",
                adp_atd_grammar::tables::to_json(&built.grammar, &built.tables, "specifications/atd/grammar/atd.lark")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn fmt_command(args: &[String]) -> ExitCode {
    let only_check = args.iter().any(|a| a == "--check");
    let paths: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    let files = match atd_files(&paths) {
        Ok(files) => files,
        Err(problem) => return usage(&problem),
    };
    let mut changed = 0;
    let mut refused = 0;
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            return usage(&format!("{} cannot be read", file.display()));
        };
        match adp_atd_format::format(&text) {
            Err(problem) => {
                refused += 1;
                println!("{}: not formatted: {problem}", file.display());
            }
            Ok(formatted) if formatted != text => {
                changed += 1;
                println!("{}", file.display());
                if !only_check && std::fs::write(&file, formatted).is_err() {
                    return usage(&format!("{} cannot be written", file.display()));
                }
            }
            Ok(_) => {}
        }
    }
    if refused > 0 || (only_check && changed > 0) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
