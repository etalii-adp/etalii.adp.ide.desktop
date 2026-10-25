//! The generated document SC-005 is measured on (the standalone host's `LargeDocument.Tests.cs` at a7fdf03; etalii.adp
//! spec 036, T056): the module's tests hold the bounds, and the shell's `benches/hierarchy.rs` prints the times.

use std::fmt::Write;

/// Ten top items of ten levels each, every item holding five requirements with a description, tags, a category and a
/// state: 100 items and 500 requirements, in CRLF.
pub fn large_document() -> String {
    let mut text = String::from(
        "<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->\r\n<!-- last: ITEM-0100 REQ-0500 -->\r\n# A large subject\r\n\r\n## Requirements\r\n\r\n",
    );
    let mut requirement = 0;
    for item in 1..=100 {
        let indent = " ".repeat(((item - 1) % 10) * 2);
        let _ = write!(
            text,
            "{indent}- ITEM-{item:04} Part {item} of the subject #part{}\r\n{indent}  What part {item} covers, in a sentence.\r\n",
            item % 7
        );
        for _ in 0..5 {
            requirement += 1;
            let _ = write!(
                text,
                "{indent}  - REQ-{requirement:04} Requirement {requirement} holds. #topic{} category:functional state:proposed\r\n{indent}    Measured as [REQ-{:04}] is.\r\n",
                requirement % 11,
                (requirement - 1).max(1)
            );
        }
    }
    text + "\r\n## Assessments\r\n\r\n- [ ] Question REQ-0250: Is this one needed?\r\n"
}
