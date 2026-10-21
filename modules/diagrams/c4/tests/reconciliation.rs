//! ADP's rules against the verdicts Structurizr's own inspector recorded for the corpus (etalii.adp spec 030, task
//! T061), ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Reconciliation.Tests.cs`
//! and its helper `C4Verdict.cs` at `8b6ccfc`. Only the mirrored rules are compared, in ADP's vocabulary, in both
//! directions; every fixture has a verdict naming the CLI that recorded it, and a verdict that cannot be read fails
//! rather than being skipped.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_c4::mirror::{adp_rule_mirroring, mirrored};
use adp_diagram_c4::{parser, rules};

const REGENERATION_HINT: &str = "Regenerate the verdicts with the Structurizr CLI (`structurizr inspect -workspace <fixture>`) and see C4InteropTests.EveryVerdict_IsStillWhatStructurizrSays.";

/// The corpus every fixture test runs over (`C4Lines.Tests.cs`, `Corpus`).
const CORPUS: [&str; 12] = [
    "minimal.dsl",
    "comments-everywhere.dsl",
    "crlf-line-endings.dsl",
    "lf-line-endings.dsl",
    "no-trailing-newline.dsl",
    "unusual-indentation.dsl",
    "unmodelled-constructs.dsl",
    "deployment-nested.dsl",
    "dynamic-interactions.dsl",
    "big-bank-plc.dsl",
    "aws-deployment.dsl",
    "crowded-component-view.dsl",
];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// One line of a recorded verdict: `LEVEL | rule | message`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    level: String,
    rule_id: String,
    message: String,
}

/// What Structurizr's inspector said about one fixture, and which CLI said it.
#[derive(Debug, Clone)]
struct Verdict {
    cli_version: String,
    findings: Vec<Finding>,
}

impl Verdict {
    fn path_for(fixture: &str) -> PathBuf {
        let stem = Path::new(fixture).file_stem().unwrap().to_string_lossy().into_owned();
        fixtures().join("verdicts").join(format!("{stem}.inspect.txt"))
    }

    fn exists(fixture: &str) -> bool {
        Verdict::path_for(fixture).is_file()
    }

    fn read(fixture: &str) -> Result<Verdict, String> {
        let path = Verdict::path_for(fixture);
        if !path.is_file() {
            return Err(format!(
                "No recorded Structurizr verdict for '{fixture}'. Every fixture in the corpus needs one. {REGENERATION_HINT}"
            ));
        }
        Verdict::read_file(&path)
    }

    fn read_file(path: &Path) -> Result<Verdict, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let lines: Vec<&str> = text.lines().collect();
        let header = lines.first().copied().unwrap_or_default();
        let prefix = "# structurizr-cli ";
        let Some(rest) = header.strip_prefix(prefix) else {
            return Err(format!(
                "'{}' does not start with the `{prefix}<version>, recorded <date>` header, so there is no way to tell which Structurizr produced it. {REGENERATION_HINT}",
                path.display()
            ));
        };
        let cli_version = rest.split(',').next().unwrap_or_default().trim().to_owned();
        let mut findings = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() < 3 {
                return Err(format!(
                    "'{}' line {} is not `LEVEL | rule | message`: '{line}'. {REGENERATION_HINT}",
                    path.display(),
                    index + 1
                ));
            }
            let rule_id = parts[1].trim();
            if rule_id.is_empty() {
                return Err(format!(
                    "'{}' line {} names no rule: '{line}'. {REGENERATION_HINT}",
                    path.display(),
                    index + 1
                ));
            }
            findings.push(Finding {
                level: parts[0].trim().to_owned(),
                rule_id: rule_id.to_owned(),
                message: parts[2].trim().to_owned(),
            });
        }
        Ok(Verdict { cli_version, findings })
    }

    fn rule_ids(&self) -> BTreeSet<String> {
        self.findings.iter().map(|f| f.rule_id.clone()).collect()
    }
}

#[test]
fn adp_and_structurizr_agree_on_every_mirrored_rule() {
    for fixture in CORPUS {
        let verdict = Verdict::read(fixture).unwrap();
        let text = fs::read_to_string(fixtures().join(fixture)).unwrap();
        let adp: BTreeSet<String> = rules::validate(&parser::parse_text(&text))
            .into_iter()
            .map(|p| p.rule_id)
            .filter(|rule| mirrored(rule).is_some())
            .collect();
        let structurizr: BTreeSet<String> = verdict
            .rule_ids()
            .iter()
            .filter_map(|rule| adp_rule_mirroring(rule))
            .map(str::to_owned)
            .collect();
        let silent: Vec<&String> = structurizr.difference(&adp).collect();
        let alone: Vec<&String> = adp.difference(&structurizr).collect();
        assert!(
            silent.is_empty() && alone.is_empty(),
            "'{fixture}' (verdict recorded by structurizr-cli {}): Structurizr reports {silent:?} where ADP is silent, and ADP reports {alone:?} where Structurizr is. {REGENERATION_HINT}",
            verdict.cli_version
        );
    }
}

#[test]
fn every_fixture_has_a_recorded_verdict() {
    for fixture in CORPUS {
        assert!(
            Verdict::exists(fixture),
            "'{fixture}' is in the corpus but has no recorded Structurizr verdict. {REGENERATION_HINT}"
        );
    }
}

#[test]
fn every_verdict_names_the_cli_that_produced_it() {
    for fixture in CORPUS {
        assert!(!Verdict::read(fixture).unwrap().cli_version.is_empty(), "{fixture}");
    }
}

#[test]
fn the_worked_example_is_where_this_spec_started() {
    let verdict = Verdict::read("big-bank-plc.dsl").unwrap();
    assert_eq!(verdict.findings.len(), 26);
    assert!(verdict.rule_ids().contains("model.deploymentnode.description"));
    assert!(verdict.rule_ids().contains("model.relationship.technology"));
    let first = &verdict.findings[0];
    assert_eq!(first.level, "ERROR");
    assert_eq!(first.rule_id, "workspace.scope");
    assert!(first.message.starts_with("This workspace has no defined scope."));
}

#[test]
fn a_corrupted_verdict_fails_rather_than_being_skipped() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("corrupted.inspect.txt");
    fs::write(&path, "this file has no header and no pipes").unwrap();
    let problem = Verdict::read_file(&path).unwrap_err();
    assert!(problem.contains("header"));
    assert!(problem.contains("structurizr inspect"));
}

#[test]
fn a_missing_verdict_fails_naming_the_file() {
    let problem = Verdict::read("no-such-fixture.dsl").unwrap_err();
    assert!(problem.contains("no-such-fixture"));
    assert!(problem.contains("structurizr inspect"));
}
