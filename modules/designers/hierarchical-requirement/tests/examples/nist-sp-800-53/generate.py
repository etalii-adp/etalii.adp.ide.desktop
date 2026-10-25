"""Writes nist-sp-800-53.md and nist-sp-800-53.adp from NIST's OSCAL catalog of SP 800-53 Revision 5 and its four
SP 800-53B baseline profiles, as readme.md says.

Usage: python -I generate.py <folder holding the five OSCAL JSON files> [<output folder, by default this one>]

The legend is taken from the web shop example beside this folder, so both documents tell a writer the same.
"""
import json
import re
import sys
from pathlib import Path

here = Path(__file__).resolve().parent
source = Path(sys.argv[1])
target = Path(sys.argv[2]) if len(sys.argv) > 2 else here
catalog = json.loads((source / "NIST_SP-800-53_rev5_catalog.json").read_text(encoding="utf-8"))["catalog"]

BASELINES = ["low", "moderate", "high", "privacy"]


def baseline(name):
    profile = json.loads((source / f"NIST_SP-800-53_rev5_{name.upper()}-baseline_profile.json").read_text(encoding="utf-8"))["profile"]
    ids = set()
    for imp in profile["imports"]:
        for include in imp.get("include-controls", []):
            ids.update(include.get("with-ids", []))
    return ids


members = {name: baseline(name) for name in BASELINES}


def prop(node, name):
    """A property's plain value: of several with one name, the one without a class (NIST's own label, AC-1 rather than AC-01)."""
    found = [p for p in node.get("props", []) if p["name"] == name]
    plain = [p for p in found if "class" not in p]
    return (plain or found or [{"value": None}])[0]["value"]


def withdrawn(control):
    return prop(control, "status") == "withdrawn"


params = {}


def collect(control):
    for param in control.get("params", []):
        params[param["id"]] = param
    for child in control.get("controls", []):
        collect(child)


for group in catalog["groups"]:
    for control in group["controls"]:
        collect(control)


def render_param(pid):
    param = params[pid]
    if "select" in param:
        select = param["select"]
        how = select.get("how-many")
        head = "Selection (one or more)" if how == "one-or-more" else "Selection"
        choices = "; ".join(fill(choice) for choice in select.get("choice", []))
        return f"[{head}: {choices}]"
    what = fill(param.get("label", pid))
    return f"[Assignment: {what if what.startswith('organization-defined') else 'organization-defined ' + what}]"


def fill(prose):
    return re.sub(r"\{\{\s*insert:\s*param,\s*([^\s}]+)\s*\}\}", lambda m: render_param(m.group(1)), prose)


def text(prose):
    # One line of text: no line breaks, and nothing a reader takes for a tag at the end of a title.
    return " ".join(fill(prose).split())


def label(part):
    value = prop(part, "label") or ""
    return value.rstrip(".").strip("()")


def sub_lines(part, prefix):
    """The nested parts of a statement part, as description lines labelled (d)(1)(a)."""
    lines = []
    for child in part.get("parts", []):
        if child["name"] != "item":
            continue
        mark = f"{prefix}({label(child)})"
        lines.append(f"{mark} {text(child.get('prose', ''))}")
        lines.extend(sub_lines(child, mark))
    return lines


items = 0
requirements = 0
item_of = {}


def next_item():
    global items
    items += 1
    return f"ITEM-{items:04d}"


def next_requirement():
    global requirements
    requirements += 1
    return f"REQ-{requirements:04d}"


def display(control):
    return prop(control, "label") or control["id"].upper()


# Number every item first, so a related control can be referred to wherever it stands.
plan = []
for group in catalog["groups"]:
    family = next_item()
    controls = []
    for control in group["controls"]:
        if withdrawn(control):
            continue
        number = next_item()
        item_of[control["id"]] = number
        enhancements = []
        for enhancement in control.get("controls", []):
            if withdrawn(enhancement):
                continue
            enhancement_number = next_item()
            item_of[enhancement["id"]] = enhancement_number
            enhancements.append((enhancement_number, enhancement))
        controls.append((number, control, enhancements))
    plan.append((family, group, controls))
items_total = items


def tags(control):
    return "".join(f" #{name}" for name in BASELINES if control["id"] in members[name])


def related(control):
    numbers = [item_of[link["href"][1:]] for link in control.get("links", []) if link.get("rel") == "related" and link["href"][1:] in item_of]
    return ", ".join(f"[{number}]" for number in numbers)


def entry_lines(number, control, indent):
    pad = " " * indent
    lines = [f"{pad}- {number} {display(control)} {control['title']}"]
    refs = related(control)
    if refs:
        lines.append(f"{pad}  Related controls: {refs}.")
    statement = next((part for part in control.get("parts", []) if part["name"] == "statement"), None)
    parts = [part for part in (statement or {}).get("parts", []) if part["name"] == "item"]
    if statement is not None and not parts:
        parts = [statement]
    for part in parts:
        title = text(part.get("prose", ""))
        mark = label(part)
        heading = f"({mark}) " if mark and part is not statement else ""
        lines.append(f"{pad}  - {next_requirement()} {heading}{title}{tags(control)}")
        for line in sub_lines(part, f"({mark})" if mark else ""):
            lines.append(f"{pad}    {line}")
    return lines


body = []
for family, group, controls in plan:
    body.append(f"- {family} {group['title']}")
    body.append(f"  The {group['title'].lower()} family ({group['id'].upper()}) of NIST SP 800-53 Revision 5.")
    for number, control, enhancements in controls:
        body.extend(entry_lines(number, control, 2))
        for enhancement_number, enhancement in enhancements:
            body.extend(entry_lines(enhancement_number, enhancement, 4))

web_shop = (here.parent / "web-shop" / "web-shop.md").read_text(encoding="utf-8").replace("\r\n", "\n")
legend = web_shop[web_shop.index("## How to read this file"):web_shop.index("## Requirements")].rstrip("\n").split("\n")
version = catalog["metadata"].get("version")
head = [
    "<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->",
    f"<!-- last: ITEM-{items:04d} REQ-{requirements:04d} -->",
    "# NIST SP 800-53 Revision 5 security and privacy controls",
    "",
    *legend,
    "",
    "## Requirements",
    "",
    f"The controls of NIST SP 800-53 Revision 5 (release {version}), one item per family, control and control enhancement, and one requirement per part of a control's statement. A requirement is tagged with the baselines of NIST SP 800-53B that select its control.",
    "",
]
document = "\r\n".join(head + body) + "\r\n"
target.mkdir(parents=True, exist_ok=True)
(target / "nist-sp-800-53.md").write_bytes(document.encode("utf-8"))

registration = [
    "etalii/hierarchical-requirement",
    "body: nist-sp-800-53.md",
    "tags:",
    "  low: green",
    "  moderate: yellow",
    "  high: red",
    "  privacy: purple",
    "collapsed:",
    *[f"  {family}: yes" for family, _, _ in plan],
    "filters:",
    "  High baseline: #high sort:number",
    "  Privacy baseline: #privacy sort:number",
    "  Access control: item:access* sort:number",
]
(target / "nist-sp-800-53.adp").write_bytes(("\r\n".join(registration) + "\r\n").encode("utf-8"))
print(f"items {items} requirements {requirements} families {len(plan)} bytes {len(document.encode('utf-8'))}")
