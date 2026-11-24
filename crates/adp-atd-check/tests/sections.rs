//! A designer's sections surface (etalii.adp spec 086, T018; ATD §9.4, DESL §7.8): record and list parts with DESL's
//! `SectionPart` settings, and the settings only a list part has refused on a record part.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/trip.atd");
    let files = Overlay {
        open: BTreeMap::from([(path.clone(), text.to_owned())]),
        under: &FsFiles,
    };
    check(&files, &path)
        .findings
        .iter()
        .map(|f| format!("{} {}:{} {}", f.code, f.line, f.column, f.message))
        .collect()
}

const TRIP: &str = r#"atd 0.1
designer example/trip-log version "0.1.0" {
  label "Trip log"
  model {
    element Trip {
      attribute title: string required
    }
    element Stop {
      attribute at: datetime
      attribute place: string required
    }
  }
  surface sections {
    record trip "Trip" Trip { fields: [{ attribute: title, label: "Title" }] }
    list stops "Stops" Stop {
      fields: [{ attribute: at, label: "Time" }, { attribute: place, label: "Place" }]
      order: (has(self.at) ? string(self.at) : '~')
    }
  }
}
"#;

#[test]
fn record_and_list_parts_check() {
    assert_eq!(findings(TRIP), Vec::<String>::new());
}

#[test]
fn a_list_parts_setting_is_refused_on_a_record() {
    let text = TRIP.replace(
        "record trip \"Trip\" Trip { fields",
        "record trip \"Trip\" Trip { order: (1), fields",
    );
    assert_eq!(
        findings(&text),
        [
            "ATD5001 14:31 `order` is a list part's setting; a record part shows the one element of its type (DESL §7.8.1)."
        ]
    );
}

#[test]
fn a_setting_section_part_does_not_have_is_refused() {
    let text = TRIP.replace("order: (has", "sortBy: (has");
    let found = findings(&text);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("ATD5001 17:7 `sortBy` is not a setting")),
        "{found:?}"
    );
}

#[test]
fn a_sections_surface_is_a_designers() {
    let text = TRIP.replace("designer example/trip-log", "diagram example/trip-log");
    let found = findings(&text);
    assert!(found.iter().any(|f| f.starts_with("ATD3002")), "{found:?}");
}
