"""Generate the parity baseline from the standalone host at the reference commit (etalii.adp spec 030, tasks T011 and T012).

    python tools/generate-parity-baseline.py <path to an etalii.adp.ide.standalone clone> [<commit>]

Reads the standalone host's tool catalogue, docs/tools.md, at the commit (default: the reference commit 8b6ccfc) with
`git show`, so the clone's checkout is not touched, and writes:

- crates/adp-core/catalogue/tools.json: every tool type the standalone host lists, in its order, with its origin, display
  name, kind, group and state (FR-021, research R3); those without a module have the states identified, specified
  and to-do; a tool type the standalone host added after the reference commit is taken from docs/tools.md at its own
  commit (LATER) and placed where that listing puts it;
- parity/inventory.md: one row per capability of the standalone host's workspace and per tool type it has a module
  for, in the format of etalii.adp spec 030 contracts/parity.md (FR-010).

Rows of parity/inventory.md that already exist keep their Check and State columns, and the sections after the table
("What is left") are kept as they are, so the file can be regenerated.
"""

import html
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REFERENCE = "8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e"
CATALOGUE = ROOT / "crates" / "adp-core" / "catalogue" / "tools.json"
INVENTORY = ROOT / "parity" / "inventory.md"

STATES = {
    "Identified": "identified",
    "Specified": "specified",
    "To-do": "to-do",
    "Work-in-progress": "work-in-progress",
    "Prototype": "prototype",
    "Implemented": "implemented",
}
WITHOUT_MODULE = {"identified", "specified", "to-do"}

# Tool types added to the standalone host after the reference commit, each with the commit its row is taken from: a
# designer recorded at a later commit of its own (etalii.adp spec 036, research R3 and R6). They join the catalogue
# only; the inventory stays the reference commit's.
LATER = {"etalii/hierarchical-requirement": "a7fdf039c204535e9fc3a14f456e6f5a36a685ce"}

ROW = re.compile(
    r'<tr><td style="white-space: nowrap;">[^&<]*&nbsp;(?P<state>[^<]+)</td>'
    r'<td style="white-space: nowrap;"><code>(?P<origin>[^<]+)</code></td>'
    r"<td>(?P<name>[^<]+)</td><td>(?P<kind>Diagram|Designer|Editor)</td>"
)
GROUP = re.compile(r"<h3>(?P<group>[^<]+)</h3>")

# The workspace's capabilities at the reference commit, in the words of spec 030's user stories, each with the
# standalone host's specification it comes from (spec-workflow names) and the increment of the plan that delivers it.
CAPABILITIES = [
    ("Project list: add, validate, remove and open projects, kept across restarts", "login-project-selection, project-root-folder-explorer", 2),
    ("Explorer tree following the disk live", "project-root-folder-explorer, view-delta-adoption", 2),
    ("Registrations nested under the file or folder they register, several per subject", "adp-file-nesting, folder-add-registration", 2),
    ("Explorer actions: add, add folder, rename (F2), delete with confirmation", "add-diagram-action, create-diagram-file, inline-rename", 2),
    ("Explorer actions: open as text, open with", "modular-text-editors", 2),
    ("Explorer actions: validate, validate all", "errors-and-warnings-panel", 2),
    ("Keyboard navigation of the explorer and its context menu (Shift+F10)", "reusable-context-menu", 2),
    ("Tabs: open or focus, unsaved state, save with Ctrl+S, close", "diagram-workspace-tabs", 2),
    ("Resizable panes: explorer and search, tabs, errors and warnings, toolbox and properties", "adp-diagram-ide", 2),
    ("Ribbon: Save, New, Undo, Redo, contextual groups, Zoom In, Zoom Out, Fit to View", "adp-diagram-ide, fit-to-view-extent", 2),
    ("One selection per window; actions as data to ribbon, context menu and keyboard", "context-service, centralized-selection", 2),
    ("Prompts: choices, file dialogs, text input, confirmations, notices", "context-service", 2),
    ("Undo and redo of every change, composite changes as one step, a barrier at a change from outside", "diagram-undo-redo, multi-select", 2),
    ("Errors and warnings panel: validated, cached per project, revalidated at start", "errors-and-warnings-panel", 2),
    ("Search panel (a placeholder at the reference commit)", "adp-diagram-ide", 2),
    ("Light and dark themes following the system, with the contrast the theme tests require", "client-centralization", 2),
    ("FBL families YAML, JSON, XML and Lines; plugin bodies; folder subjects; routing", "fbl-implementation", 3),
    ("FBL splices keeping every other byte, the byte order mark and the majority line ending", "fbl-implementation, fbl-written-text", 3),
    ("The registration file (.adp): origin, body, layout, unknown blocks kept", "folder-add-registration, adp-file-nesting", 3),
    ("DISL runtime: findings, toolbox, operations, constraints, notation; later versions refused", "backend-centralization", 3),
    ("DESL and DED runtime", "knowledge-designer", 3),
    ("CEL expressions", "backend-centralization", 3),
    ("Canvas: pan with right-drag and scroll bars, zoom, fit to view", "canvas-scrollbars, canvas-single-scrollbar, fit-to-view-extent", 4),
    ("Canvas gestures: drag, move, resize, connect, Escape to cancel", "drag-and-drop-centralization, selection-after-drag", 4),
    ("Box selection and Ctrl selection", "multi-select", 4),
    ("Toolbox dragged onto the canvas; a connection dropped on empty space creates its target", "drag-and-drop-centralization", 4),
    ("Tab and Enter add a child or a sibling", "adp-diagram-ide", 4),
    ("Inline label rename on the canvas", "inline-rename", 4),
    ("Layout modes: Manual, Automatic Horizontal, Vertical, Cloud; collapse and expand", "layout-modes", 4),
    ("Property grid: selection chain, groups, read-only rows with their reason, five editor kinds", "property-grid", 4),
    ("Property grid on several elements, one undo for all", "multi-select", 4),
    ("Changes on disk reconciled with unsaved edits in open views", "view-delta-adoption", 4),
    ("Add dialog: tool types grouped by vendor, with names and states", "add-diagram-action", 4),
    ("Windowed tables of 10,000 rows", "knowledge-designer", 5),
    ("Keyboard reach and accessibility names of every interactive element", "client-centralization", 6),
]


def show(clone: pathlib.Path, commit: str, path: str) -> str:
    return subprocess.run(
        ["git", "-C", str(clone), "show", f"{commit}:{path}"], check=True, capture_output=True, text=True, encoding="utf-8"
    ).stdout


def tool_types(tools_md: str) -> list[dict]:
    rows, group = [], ""
    for line in tools_md.splitlines():
        heading = GROUP.search(line)
        if heading:
            group = html.unescape(heading.group("group")).strip()
        match = ROW.search(line)
        if match:
            rows.append(
                {
                    "origin": match.group("origin"),
                    "displayName": html.unescape(match.group("name")).strip(),
                    "kind": match.group("kind").lower(),
                    "group": group,
                    "state": STATES[match.group("state").strip()],
                }
            )
    return rows


def with_later(clone: pathlib.Path, types: list[dict]) -> list[dict]:
    """Return the reference commit's tool types with every LATER one inserted after the row it follows at its commit."""
    result = list(types)
    for origin, commit in LATER.items():
        later = tool_types(show(clone, commit, "docs/tools.md"))
        index = next(i for i, t in enumerate(later) if t["origin"] == origin)
        before = later[index - 1]["origin"] if index > 0 else None
        at = next((i + 1 for i, t in enumerate(result) if t["origin"] == before), 0)
        result.insert(at, later[index])
    return result


def existing_rows() -> dict[str, tuple[str, str]]:
    """Return {capability: (check, state)} of the current inventory, to keep what was filled in."""
    if not INVENTORY.exists():
        return {}
    kept = {}
    for line in INVENTORY.read_text(encoding="utf-8").splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) == 6 and cells[0].startswith("P-"):
            kept[cells[1]] = (cells[4], cells[5])
    return kept


def trailing_sections() -> list[str]:
    """Return the lines of the current inventory from its first section after the table on, to keep them."""
    if not INVENTORY.exists():
        return []
    lines = INVENTORY.read_text(encoding="utf-8").splitlines()
    start = next((i for i, line in enumerate(lines) if line.startswith("## ")), len(lines))
    return lines[start:]


def main() -> None:
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    clone, commit = pathlib.Path(sys.argv[1]), sys.argv[2] if len(sys.argv) == 3 else REFERENCE
    types = tool_types(show(clone, commit, "docs/tools.md"))
    if not types:
        sys.exit("No tool types found in docs/tools.md; has its table changed?")

    identity = [t for t in types if t["state"] in WITHOUT_MODULE]
    CATALOGUE.parent.mkdir(parents=True, exist_ok=True)
    CATALOGUE.write_text(
        json.dumps({"source": {"repository": "etalii-adp/etalii.adp.ide.standalone", "path": "docs/tools.md", "commit": commit,
                               "later": LATER},
                    "toolTypes": with_later(clone, types)}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    kept = existing_rows()
    rows = [(c, s, i) for c, s, i in CAPABILITIES]
    for t in types:
        if t["state"] in WITHOUT_MODULE:
            continue
        increment = 5 if t["kind"] == "designer" else 2 if t["kind"] == "editor" else 4
        rows.append((f"{t['displayName']} (`{t['origin']}`, {t['state']})", f"docs/tools.md, {t['group']}", increment))

    lines = [
        "# Parity inventory",
        "",
        f"Every capability and tool type of the standalone host at [`{commit[:7]}`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/{commit}), "
        "in the format of etalii.adp spec 030 [contracts/parity.md](https://github.com/etalii-adp/etalii.adp/blob/develop/specs/030-desktop-native-rust/contracts/parity.md). "
        "Generated by `tools/generate-parity-baseline.py`; regenerating keeps the Check and State columns and the sections after the table. "
        f"The {len(identity)} tool types the standalone host lists without a module are in `crates/adp-core/catalogue/tools.json` with every other tool type, not here.",
        "",
        "| Id | Capability | Standalone source | Increment | Check | State |",
        "|---|---|---|---|---|---|",
    ]
    for number, (capability, source, increment) in enumerate(rows, start=1):
        check, state = kept.get(capability, ("", "planned"))
        lines.append(f"| P-{number:03d} | {capability} | {source} | {increment} | {check} | {state} |")
    trailing = trailing_sections()
    if trailing:
        lines += [""] + trailing
    INVENTORY.parent.mkdir(parents=True, exist_ok=True)
    INVENTORY.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"{len(rows)} inventory rows, {len(identity)} identity-only tool types, from standalone {commit[:7]}.")


if __name__ == "__main__":
    main()
