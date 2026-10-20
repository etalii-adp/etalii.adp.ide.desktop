"""Extract VS Code's default light and dark themes (etalii.adp spec 030, research R9, contracts/vscode-theme.md, task T018).

    python tools/extract-vscode-theme.py <VS Code release tag>

Reads, from microsoft/vscode at the tag, the themes `ThemeSettingDefaults` names as the defaults
(src/vs/workbench/services/themes/common/workbenchThemeService.ts), resolves each theme's chain of `include`s
(a child's colours win; token colours run parent first), and writes crates/adp-theme/vscode/dark.json and light.json,
SOURCE.md and VS Code's licence. The sizes in sizes.md are kept by hand, each with its source line at the same tag.
"""

import json
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "crates" / "adp-theme" / "vscode"
RAW = "https://raw.githubusercontent.com/microsoft/vscode/{tag}/{path}"
THEMES = "extensions/theme-defaults"
SERVICE = "src/vs/workbench/services/themes/common/workbenchThemeService.ts"


def fetch(tag: str, path: str) -> str:
    with urllib.request.urlopen(RAW.format(tag=tag, path=path), timeout=60) as response:
        return response.read().decode("utf-8")


def jsonc(text: str) -> dict:
    """Parse VS Code's JSON with comments and trailing commas, leaving `//` inside strings alone."""
    out, i, in_string = [], 0, False
    while i < len(text):
        c = text[i]
        if in_string:
            out.append(c)
            if c == "\\":
                out.append(text[i + 1])
                i += 1
            elif c == '"':
                in_string = False
        elif c == '"':
            in_string = True
            out.append(c)
        elif text.startswith("//", i):
            while i < len(text) and text[i] != "\n":
                i += 1
            continue
        elif text.startswith("/*", i):
            i = text.index("*/", i) + 2
            continue
        else:
            out.append(c)
        i += 1
    return json.loads(re.sub(r",(\s*[}\]])", r"\1", "".join(out)))


def resolve(tag: str, path: str) -> tuple[dict, list, list[str]]:
    theme = jsonc(fetch(tag, f"{THEMES}/themes/{path}"))
    colors, tokens, chain = {}, [], []
    if "include" in theme:
        colors, tokens, chain = resolve(tag, theme["include"].removeprefix("./"))
    colors.update(theme.get("colors", {}))
    tokens.extend(theme.get("tokenColors", []))
    return colors, tokens, chain + [path]


def defaults(tag: str) -> dict[str, str]:
    """{'dark': label, 'light': label} of the default themes, from ThemeSettingDefaults."""
    service = fetch(tag, SERVICE)
    found = dict(re.findall(r"COLOR_THEME_(DARK|LIGHT) = '([^']+)'", service))
    return {"dark": found["DARK"], "light": found["LIGHT"]}


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    tag = sys.argv[1]
    labels = defaults(tag)
    package = jsonc(fetch(tag, f"{THEMES}/package.json"))
    by_id = {t.get("id", t["label"]): t["path"].removeprefix("./themes/") for t in package["contributes"]["themes"]}
    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    for kind, label in labels.items():
        colors, tokens, chain = resolve(tag, by_id[label])
        document = {"name": label, "kind": kind, "vscode": tag, "files": chain, "colors": dict(sorted(colors.items())), "tokenColors": tokens}
        (OUT / f"{kind}.json").write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
        rows.append(f"| {kind} | {label} | {' → '.join(f'`{p}`' for p in reversed(chain))} | {len(colors)} | {len(tokens)} |")
    (OUT / "LICENSE-vscode.txt").write_text(fetch(tag, "LICENSE.txt"), encoding="utf-8", newline="\n")
    (OUT / "SOURCE.md").write_text(
        "# Source of the theme values\n\n"
        f"Extracted by `tools/extract-vscode-theme.py {tag}` from [microsoft/vscode at `{tag}`](https://github.com/microsoft/vscode/tree/{tag}/{THEMES}/themes), "
        f"MIT licence ([LICENSE-vscode.txt](LICENSE-vscode.txt)). The default themes are the ones `ThemeSettingDefaults` names in `{SERVICE}`; "
        "each theme's `include` chain is resolved, the child's colours winning.\n\n"
        "| Kind | Theme | Files, child first | Colours | Token colours |\n|---|---|---|---|---|\n" + "\n".join(rows) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"VS Code {tag}: {', '.join(f'{k} = {v}' for k, v in labels.items())}.")


if __name__ == "__main__":
    main()
