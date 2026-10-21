"""Vendor the standalone host's bindings that the vendored specifications name (etalii.adp spec 030, task T048).

    python tools/vendor-standalone-bindings.py <path to an etalii.adp.ide.standalone clone> [<commit>]

Several specifications under vendor/etalii.adp/definitions/ name their FBL binding by its address in the standalone
host's repository (`https://raw.githubusercontent.com/etalii-adp/etalii.adp.ide.standalone/develop/<path>#<name>`),
because the standalone host bundles those bindings with its modules. This script copies each such file, at the
reference commit (default 8b6ccfc) and read with `git show` so the clone's checkout is not touched, to
vendor/etalii.adp.ide.standalone/<path>, where the catalogue resolves the address. A named file the commit does not
have is reported and skipped: the standalone host reads that tool type with its module's own reader.
"""

import json
import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFINITIONS = ROOT / "vendor" / "etalii.adp" / "definitions"
VENDOR = ROOT / "vendor" / "etalii.adp.ide.standalone"
REFERENCE = "8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e"
ADDRESS = re.compile(r"https://raw\.githubusercontent\.com/etalii-adp/etalii\.adp\.ide\.standalone/develop/([^\"#]+)")


def main() -> None:
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    clone, commit = pathlib.Path(sys.argv[1]), sys.argv[2] if len(sys.argv) == 3 else REFERENCE
    paths = sorted(
        {
            match
            for specification in sorted(DEFINITIONS.glob("*/*.dis")) + sorted(DEFINITIONS.glob("*/*.des"))
            for match in ADDRESS.findall(specification.read_text(encoding="utf-8"))
        }
    )
    if VENDOR.exists():
        shutil.rmtree(VENDOR)
    copied = []
    for path in paths:
        shown = subprocess.run(["git", "-C", str(clone), "show", f"{commit}:{path}"], capture_output=True)
        if shown.returncode != 0:
            print(f"Not at {commit[:7]}, skipped: {path}")
            continue
        target = VENDOR / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(shown.stdout)
        copied.append(path)
    (VENDOR / "provenance.json").write_text(
        json.dumps({"repository": "etalii-adp/etalii.adp.ide.standalone", "commit": commit, "files": copied}, indent=2)
        + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"{len(copied)} of {len(paths)} bindings vendored from standalone {commit[:7]}.")


if __name__ == "__main__":
    main()
