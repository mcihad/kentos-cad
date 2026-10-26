"""Local GUI evidence and a small, historical Git-backed image manifest.

No drawing/model tests run here. ``metadata`` never verifies image bytes;
``verify`` fails when an image is missing or changed. ``restore`` is explicit,
checks the historical blob and never replaces a different existing file.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile

FORMAT = "kentos.gui-evidence.manifest"
IMAGE_SUFFIXES = {".png", ".jpg", ".jpeg", ".webp", ".gif"}
DEFAULT_MANIFEST = "docs/baseline/gui-evidence-manifest.json"


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.PIPE)


def image_path(name: str) -> bool:
    return name.startswith("docs/baseline/") and PurePosixPath(name).suffix.lower() in IMAGE_SUFFIXES


def target(root: Path, name: str) -> Path:
    root = root.resolve()
    path = PurePosixPath(name)
    if (not image_path(name) or path.is_absolute() or str(path) != name
            or ".." in path.parts or "\\" in name or "\0" in name):
        raise ValueError(f"Unsafe baseline image path: {name!r}")
    out = root.joinpath(*path.parts)
    current = out
    while current != root:
        if current.is_symlink():
            raise ValueError(f"Image path contains a symlink: {name}")
        current = current.parent
    if not out.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Image path leaves the repository: {name}")
    return out


def tree(root: Path, commit: str) -> dict[str, str]:
    result = {}
    for row in git(root, "ls-tree", "-r", "-z", commit, "--", "docs/baseline").split(b"\0"):
        if not row:
            continue
        metadata, name = row.split(b"\t", 1)
        mode, kind, oid = metadata.decode("ascii").split()
        name = name.decode("utf-8")
        if image_path(name):
            if kind != "blob" or mode not in {"100644", "100755"}:
                raise ValueError(f"Historical image is not a regular file: {name}")
            result[name] = oid
    return result


class GitBlobs:
    """Stream one historical blob at a time; do not buffer the whole archive."""

    def __init__(self, root: Path):
        self.process = subprocess.Popen(
            ["git", "-C", str(root), "cat-file", "--batch"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )

    def __enter__(self):
        return self

    def read(self, oid: str) -> bytes:
        self.process.stdin.write((oid + "\n").encode("ascii"))
        self.process.stdin.flush()
        header = self.process.stdout.readline().decode("ascii").strip().split()
        if len(header) != 3 or header[0] != oid or header[1] != "blob":
            raise ValueError(f"Historical Git blob unavailable: {oid}")
        size = int(header[2])
        data = self.process.stdout.read(size)
        if len(data) != size or self.process.stdout.read(1) != b"\n":
            raise ValueError(f"Incomplete historical Git blob: {oid}")
        return data

    def __exit__(self, kind, value, traceback):
        if kind is not None:
            self.process.kill()
        self.process.stdin.close()
        self.process.wait(timeout=10)
        self.process.stdout.close()
        self.process.stderr.close()


def git_blob_hash(data: bytes, oid: str) -> str:
    algorithm = "sha1" if len(oid) == 40 else "sha256"
    return hashlib.new(algorithm, f"blob {len(data)}\0".encode() + data).hexdigest()


def matches(data: bytes, row: dict) -> bool:
    return (len(data) == row["bytes"]
            and hashlib.sha256(data).hexdigest() == row["sha256"]
            and git_blob_hash(data, row["gitBlob"]) == row["gitBlob"])


def record(root: Path, commit: str) -> dict:
    commit = git(root, "rev-parse", "--verify", commit + "^{commit}").decode().strip()
    rows = []
    with GitBlobs(root) as blobs:
        for name, oid in sorted(tree(root, commit).items()):
            target(root, name)
            data = blobs.read(oid)
            if git_blob_hash(data, oid) != oid:
                raise ValueError(f"Git blob hash mismatch: {name}")
            rows.append(dict(path=name, sha256=hashlib.sha256(data).hexdigest(),
                             bytes=len(data), gitBlob=oid))
    return dict(format=FORMAT, version=1, sourceCommit=commit,
                imageCount=len(rows), totalBytes=sum(r["bytes"] for r in rows), images=rows)


def load(root: Path, path: Path) -> dict:
    data = json.loads(path.read_text())
    if data.get("format") != FORMAT or data.get("version") != 1:
        raise ValueError("Unsupported GUI evidence manifest")
    if not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", data["sourceCommit"]):
        raise ValueError("Invalid historical source commit")
    names = set()
    for row in data["images"]:
        target(root, row["path"])
        if row["path"] in names:
            raise ValueError(f"Duplicate image path: {row['path']}")
        names.add(row["path"])
        if (not re.fullmatch(r"[0-9a-f]{64}", row["sha256"])
                or not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", row["gitBlob"])
                or type(row["bytes"]) is not int or row["bytes"] < 0):
            raise ValueError(f"Invalid image metadata: {row['path']}")
    if data["imageCount"] != len(names) or data["totalBytes"] != sum(r["bytes"] for r in data["images"]):
        raise ValueError("Image manifest totals disagree")
    return data


def inspect(root: Path, manifest: dict, *, verify: bool) -> dict:
    missing, changed = [], []
    verified = 0
    for row in manifest["images"]:
        path = target(root, row["path"])
        if not path.exists():
            missing.append(row["path"])
        elif verify:
            if path.is_file() and matches(path.read_bytes(), row):
                verified += 1
            else:
                changed.append(row["path"])
    return dict(mode="verify" if verify else "metadata-only", imageCount=manifest["imageCount"],
                verifiedImages=verified, missingImages=len(missing), changedImages=len(changed),
                allImageBytesVerified=verify and not missing and not changed,
                missing=missing, changed=changed)


def restore(root: Path, manifest: dict, names: list[str]) -> dict:
    wanted = set(names)
    rows = [r for r in manifest["images"] if not wanted or r["path"] in wanted]
    if wanted - {r["path"] for r in rows}:
        raise ValueError("Restore requested paths outside the manifest")
    history = tree(root, manifest["sourceCommit"])
    for row in rows:
        if history.get(row["path"]) != row["gitBlob"]:
            raise ValueError(f"Historical commit/blob mismatch: {row['path']}")
    restored, present, conflicts = [], [], []
    with GitBlobs(root) as blobs:
        for row in rows:
            path = target(root, row["path"])
            if path.exists():
                if path.is_file() and matches(path.read_bytes(), row):
                    present.append(row["path"])
                else:
                    conflicts.append(row["path"])
                continue
            data = blobs.read(row["gitBlob"])
            if not matches(data, row):
                raise ValueError(f"Historical image bytes disagree: {row['path']}")
            path.parent.mkdir(parents=True, exist_ok=True)
            temporary = None
            try:
                with tempfile.NamedTemporaryFile(prefix=".gui-restore-", dir=path.parent, delete=False) as out:
                    temporary = Path(out.name)
                    out.write(data)
                    out.flush()
                    os.fsync(out.fileno())
                # A hard link publishes complete bytes atomically and fails if any file now exists.
                try:
                    os.link(temporary, path)
                    restored.append(row["path"])
                except FileExistsError:
                    if path.is_file() and matches(path.read_bytes(), row):
                        present.append(row["path"])
                    else:
                        conflicts.append(row["path"])
            finally:
                if temporary is not None:
                    temporary.unlink()
    return dict(mode="restore", restoredImages=len(restored), alreadyPresent=len(present),
                conflicts=conflicts, overwrittenImages=0)


def tracked(root: Path) -> dict:
    paths = [os.fsdecode(p) for p in git(root, "ls-files", "-z").split(b"\0") if p]
    images = [p for p in paths if image_path(p)]
    return dict(mode="check-tracked", trackedBaselineImages=len(images), images=images)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--manifest", type=Path, default=Path(DEFAULT_MANIFEST))
    commands = parser.add_subparsers(dest="command", required=True)
    rec = commands.add_parser("record", help="Record historical image metadata without copying images")
    rec.add_argument("--source-commit", default="HEAD")
    commands.add_parser("metadata", help="Check metadata and report availability; verify no image bytes")
    commands.add_parser("verify", help="Require every local image and verify its bytes")
    res = commands.add_parser("restore", help="Explicitly restore historical images; never overwrite a different file")
    res.add_argument("paths", nargs="*")
    commands.add_parser("check-tracked", help="Reject only tracked docs/baseline images, not product/fixture assets")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    path = args.manifest if args.manifest.is_absolute() else root / args.manifest
    try:
        if args.command == "record":
            data = record(root, args.source_commit)
            text = json.dumps(data, indent=2, ensure_ascii=False) + "\n"
            path.parent.mkdir(parents=True, exist_ok=True)
            try:
                with path.open("x") as out:
                    out.write(text)
            except FileExistsError:
                if path.read_text() != text:
                    raise ValueError("Existing manifest differs; choose another manifest path")
            result = dict(mode="record", manifest=str(path), imageCount=data["imageCount"],
                          totalBytes=data["totalBytes"], sourceCommit=data["sourceCommit"])
        elif args.command == "check-tracked":
            result = tracked(root)
        else:
            manifest = load(root, path)
            result = restore(root, manifest, args.paths) if args.command == "restore" else inspect(
                root, manifest, verify=args.command == "verify")
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return int(bool(result.get("conflicts") or result.get("trackedBaselineImages")
                        or args.command == "verify" and not result["allImageBytesVerified"]))
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        print(json.dumps(dict(mode=args.command, error=str(error)), ensure_ascii=False))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
