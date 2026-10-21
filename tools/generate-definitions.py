"""Generate the standalone host's tool-type definitions for the add dialog (etalii.adp spec 030, task T068).

    python tools/generate-definitions.py <path to an etalii.adp.ide.standalone clone> [<commit>]

Reads every diagram module's `DiagramDefinition`s from the standalone host's sources at the commit (default: the
reference commit 8b6ccfc) with `git show`, so the clone's checkout is not touched, and writes
crates/adp-core/catalogue/definitions.json: for each definition, in the order of its module folder and then its file,
its origin, title, description, icon, extension, whether the extension is shared, its alternate extension, its subject
and the test that says on which bodies its add on a file is suggested. That is what the standalone host's add dialog
(`AddDiagramContextActionProvider`, `DiagramOptionTree`) shows and what it writes.

The `SuggestsBody` tests are C# lambdas; the generator does not translate code, so it names the four it knows by
origin, and fails on a fifth.
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REFERENCE = "8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e"
OUTPUT = ROOT / "crates" / "adp-core" / "catalogue" / "definitions.json"

# The standalone host's `SuggestsBody` tests at the reference commit, as data: a body is suggested when it contains
# one of `contains`, or has a Markdown heading whose text is one of `heading` (ignoring case).
SUGGESTS = {
    "w3c/owl": {"contains": ["owl:Ontology", "http://www.w3.org/2002/07/owl#Ontology"]},
    "w3c/skos": {"contains": ["skos:ConceptScheme", "http://www.w3.org/2004/02/skos/core#ConceptScheme"]},
    "w3c/shacl": {"contains": ["sh:NodeShape", "sh:property", "http://www.w3.org/ns/shacl#NodeShape"]},
    "etalii/agent-behavior-modelling": {"heading": ["behavior", "behaviour"]},
}

STRING = r'"(?:[^"\\]|\\.)*"'
CONSTANT = re.compile(r"const string (\w+)\s*=\s*(" + STRING + r")\s*;")
ORIGIN_CONSTANT = re.compile(
    r"ToolOrigin (\w+)\s*(?:\{\s*get;\s*\})?\s*=\s*new(?: ToolOrigin)?\(\s*(" + STRING + r")\s*,\s*(" + STRING
    + r")\s*(?:,\s*(" + STRING + r")\s*)?\)"
)


def git(clone, *args):
    return subprocess.run(["git", "-C", str(clone), *args], check=True, capture_output=True, text=True,
                          encoding="utf-8").stdout


def unquote(literal):
    return json.loads(literal)


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return "\n".join(re.sub(r"(^|\s)//.*$", "", line) for line in text.splitlines())


def arguments(text, start):
    """The top-level arguments of the call whose `(` is at `start`, and the index after its `)`."""
    depth, current, parts, i, quoted = 0, [], [], start, False
    while i < len(text):
        c = text[i]
        if quoted:
            current.append(c)
            if c == "\\":
                current.append(text[i + 1])
                i += 1
            elif c == '"':
                quoted = False
        elif c == '"':
            quoted = True
            current.append(c)
        elif c in "([{":
            depth += 1
            if depth > 1:
                current.append(c)
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                parts.append("".join(current).strip())
                return parts, i + 1
            current.append(c)
        elif c == "," and depth == 1:
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(c)
        i += 1
    raise ValueError("unbalanced call")


def string_value(expression, constants):
    """A string expression: literals and constants joined with `+`."""
    value = ""
    for part in re.split(r"\s*\+\s*(?=(?:[^\"]*\"[^\"]*\")*[^\"]*$)", expression.strip()):
        part = part.strip()
        if part.startswith('"'):
            value += unquote(part)
        elif part in constants:
            value += constants[part]
        else:
            raise ValueError(f"cannot read the string {expression!r}")
    return value


def origin_value(expression, origins, constants):
    expression = expression.strip()
    if expression.startswith("new ToolOrigin("):
        parts, _ = arguments(expression, expression.index("("))
        return "/".join(string_value(p, constants) for p in parts)
    name = expression.rsplit(".", 1)[-1]
    if name in origins:
        return origins[name]
    raise ValueError(f"cannot read the origin {expression!r}")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    clone = pathlib.Path(sys.argv[1])
    commit = sys.argv[2] if len(sys.argv) > 2 else REFERENCE
    files = [f for f in git(clone, "ls-tree", "-r", "--name-only", commit, "src/diagrams").splitlines()
             if "/backend/" in f and f.endswith(".cs") and ".Tests/" not in f]
    modules = {}
    for f in files:
        modules.setdefault(f.split("/")[2], []).append(f)

    definitions = []
    for module in sorted(modules):
        texts = {f: strip_comments(git(clone, "show", f"{commit}:{f}")) for f in sorted(modules[module])}
        constants, origins = {}, {}
        for text in texts.values():
            for name, literal in CONSTANT.findall(text):
                constants[name] = unquote(literal)
            for name, *parts in ORIGIN_CONSTANT.findall(text):
                origins[name] = "/".join(unquote(p) for p in parts if p)
        for f, text in texts.items():
            for match in re.finditer(r"(?:DiagramDefinition \w+ \{ get; \} = |\[\s*|,\s*)new\(", text):
                start = match.end() - 1
                parts, _ = arguments(text, start)
                if len(parts) < 2:
                    continue
                try:
                    origin = origin_value(parts[0], origins, constants)
                except ValueError:
                    continue
                positional = [p for p in parts[1:] if not re.match(r"^\w+\s*:", p)]
                named = dict(re.match(r"^(\w+)\s*:\s*(.*)$", p, re.S).groups() for p in parts[1:]
                             if re.match(r"^\w+\s*:", p))
                definition = {
                    "origin": origin,
                    "title": string_value(positional[0], constants),
                    "description": string_value(positional[1], constants) if len(positional) > 1 else "",
                    "icon": string_value(named["Icon"], constants) if "Icon" in named else "",
                    "extension": string_value(named["Extension"], constants) if "Extension" in named else "",
                    "sharedExtension": named.get("SharedExtension", "false").strip() == "true",
                    "alternateExtension": string_value(named["AlternateExtension"], constants)
                    if "AlternateExtension" in named else "",
                    "subject": "folder" if named.get("Subject", "").strip().endswith("Folder") else "document",
                }
                if "SuggestsBody" in named:
                    if origin not in SUGGESTS:
                        raise ValueError(f"{origin} has a SuggestsBody test the generator does not know")
                    definition["suggestsBody"] = SUGGESTS[origin]
                if any(d["origin"] == origin for d in definitions):
                    continue
                definitions.append(definition)

    OUTPUT.write_text(json.dumps({
        "source": {"repository": "etalii-adp/etalii.adp.ide.standalone", "path": "src/diagrams/*/backend",
                   "commit": git(clone, "rev-parse", commit).strip()},
        "definitions": definitions,
    }, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    print(f"{len(definitions)} definitions written to {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
