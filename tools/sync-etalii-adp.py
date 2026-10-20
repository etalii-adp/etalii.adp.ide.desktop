"""Vendor etalii.adp's specifications/ and definitions/ at one commit (etalii.adp spec 030, research R3, task T010).

    python tools/sync-etalii-adp.py <commit>   rewrite vendor/etalii.adp/ and vendor/etalii.adp.lock
    python tools/sync-etalii-adp.py --check    fail when vendor/etalii.adp/ differs from the locked commit

The files are taken from GitHub's archive of the commit, so no clone of etalii.adp is needed.
"""

import io
import pathlib
import re
import shutil
import sys
import tarfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
VENDOR = ROOT / "vendor" / "etalii.adp"
LOCK = ROOT / "vendor" / "etalii.adp.lock"
FOLDERS = ("specifications", "definitions")
ARCHIVE = "https://codeload.github.com/etalii-adp/etalii.adp/tar.gz/{commit}"


def fetch(commit: str) -> dict[str, bytes]:
    """Return {relative path: bytes} of the vendored folders at the commit."""
    with urllib.request.urlopen(ARCHIVE.format(commit=commit), timeout=120) as response:
        data = response.read()
    files: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive.getmembers():
            if not member.isfile():
                continue
            parts = member.name.split("/", 1)
            if len(parts) < 2 or not parts[1].startswith(tuple(f + "/" for f in FOLDERS)):
                continue
            extracted = archive.extractfile(member)
            assert extracted is not None
            files[parts[1]] = extracted.read()
    if not files:
        sys.exit(f"Nothing found under {', '.join(FOLDERS)} at etalii.adp {commit}.")
    return files


def vendored() -> dict[str, bytes]:
    return {p.relative_to(VENDOR).as_posix(): p.read_bytes() for p in VENDOR.rglob("*") if p.is_file()}


def locked_commit() -> str:
    match = re.search(r"^commit: ([0-9a-f]{40})$", LOCK.read_text(encoding="utf-8"), re.MULTILINE)
    if not match:
        sys.exit(f"{LOCK.relative_to(ROOT)} names no 40-character commit.")
    return match.group(1)


def sync(commit: str) -> None:
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        sys.exit("Give the full 40-character etalii.adp commit.")
    files = fetch(commit)
    shutil.rmtree(VENDOR, ignore_errors=True)
    for name, content in files.items():
        target = VENDOR / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
    LOCK.write_text(
        "# The etalii.adp commit that vendor/etalii.adp/ copies (specifications/ and definitions/).\n"
        "# Change it only with tools/sync-etalii-adp.py.\n"
        f"commit: {commit}\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"Vendored {len(files)} files of etalii.adp {commit}.")


def check() -> None:
    commit = locked_commit()
    expected, actual = fetch(commit), vendored()
    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))
    changed = sorted(n for n in set(expected) & set(actual) if expected[n] != actual[n])
    for label, names in (("missing", missing), ("not in etalii.adp", extra), ("changed", changed)):
        for name in names:
            print(f"{label}: vendor/etalii.adp/{name}")
    if missing or extra or changed:
        sys.exit(f"vendor/etalii.adp/ differs from etalii.adp {commit}; run tools/sync-etalii-adp.py {commit}.")
    print(f"vendor/etalii.adp/ is etalii.adp {commit}: {len(expected)} files.")


if __name__ == "__main__":
    if sys.argv[1:] == ["--check"]:
        check()
    elif len(sys.argv) == 2:
        sync(sys.argv[1])
    else:
        sys.exit(__doc__)
