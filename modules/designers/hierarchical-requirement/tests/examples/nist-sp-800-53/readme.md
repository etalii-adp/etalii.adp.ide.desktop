# NIST SP 800-53 security and privacy controls

A large requirement document: the controls of [NIST SP 800-53 Revision 5](https://doi.org/10.6028/NIST.SP.800-53r5), *Security and Privacy Controls for Information Systems and Organizations*, with the baselines of [NIST SP 800-53B](https://doi.org/10.6028/NIST.SP.800-53B). It is about 150 times the size of the web shop beside it, and was chosen for that: a public requirements set with a real hierarchy, thousands of entries and references between them, under a licence that lets it be copied.

| File | What it is |
|---|---|
| `nist-sp-800-53.md` | The requirement document: 20 families, 300 controls and 714 control enhancements as 1,034 items three levels deep, and 1,582 requirements |
| `nist-sp-800-53.adp` | Its registration: the four baseline tags and their colours, every family collapsed, and three stored filters |
| `generate.py` | The script that wrote both from NIST's files |
| `LICENSE.md` | The licence of the repository the data comes from, verbatim |

## Where it comes from

It was written by `generate.py` from five files of NIST's [OSCAL content](https://github.com/usnistgov/oscal-content), folder `nist.gov/SP800-53/rev5/json/`, at commit `f0de6d8d8c059d374f15e0ae07508d82e370fc1d` (`main`, fetched 2026-10-24). They are the catalog `NIST_SP-800-53_rev5_catalog.json` (release 5.2.0) and the `LOW`, `MODERATE`, `HIGH` and `PRIVACY` baseline profiles. To write it again, download those five files into one folder and run `python -I generate.py <that folder>`. The legend is taken from `../web-shop/web-shop.md`, so both documents tell a writer the same.

**The licence** is the repository's own statement in its `LICENSE.md`, copied here: the content is a work of the United States government, in the public domain within the United States, and NIST waives copyright worldwide through the CC0 1.0 Universal public domain dedication. Neither licence is share-alike. The statement was read from the repository at the commit above, not from a page about it.

## How the controls became a requirement document

- **Items.** Each family is an item at the top, such as *Access Control*. Each control is an item in its family, such as *AC-2 Account Management*, and each of its enhancements is an item in the control, such as *AC-2(1) Automated System Account Management*. Withdrawn controls and enhancements are left out.
- **Requirements.** Each lettered part of a control's statement is one requirement, its title the part's text, such as *(a) Define and document the types of accounts allowed and specifically prohibited for use within the system;*. A statement without parts is one requirement. The numbered sub-parts are the requirement's description, one line each, labelled like *(d)(1)*, because a description holds no list.
- **Parameters** are written as NIST writes them in the publication: *[Assignment: organization-defined frequency]* and *[Selection: remove; disable]*.
- **Tags.** A requirement carries `#low`, `#moderate`, `#high` and `#privacy` for the SP 800-53B baselines that select its control or enhancement. One in no baseline carries none.
- **References.** A control's or enhancement's related controls are its description: *Related controls: [ITEM-0016], [ITEM-0061]*. A related control that is withdrawn is left out.
- **The registration** collapses every family, so the page opens on twenty boxes. Its three filters are *High baseline*, *Privacy baseline* and *Access control*.

The document reads without a finding. `NistControlsTests` holds it to that and to the counts above. It also holds that the page opens it and shows another program's change within SC-003's two seconds, and that renaming one item writes only that item's line.

## What this example does not demonstrate

- **No categories and no states.** SP 800-53 classifies its controls by family and baseline, not as functional or non-functional, and says nothing of a control's progress. No requirement carries `category:` or `state:`, so the registration lists none and the pickers offer the starting ones.
- **No assessments.** The `Assessments` section is absent. The web shop shows every kind.
- **Nothing a person or an agent wrote.** Every entry was generated, so the document has no `ITEM-NEW` or `REQ-NEW`, no other list marker than `-`, and nothing wrong. The web shop and the fixtures of `hierarchical-requirement-faults` show those.
- **None of SP 800-53's discussion, assessment procedures or references to other publications.** Only the statements, the related controls and the baselines were taken.
