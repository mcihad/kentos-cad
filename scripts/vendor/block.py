#!/usr/bin/env python3
"""Apple's blocks for Rust, block 0.1.6 with its extern static inhabited: writes crates/vendor/block from the crates.io
release, its archive checked against the SHA-256 Cargo locked.

wgpu-hal 27's Metal backend (under iced 0.14) uses `block` through the `metal` crate. The release declares the
Objective-C class of a stack block as `enum Class { }` and an extern static of it; a static of an uninhabited type is a
future error (rust-lang/rust#74840) and every build printed cargo's future-incompatibility note for it. The copy here
declares `Class` as an opaque zero-sized `#[repr(C)]` struct instead, the form Rust's FFI guide gives for an opaque
foreign type; the static's address, the only use, is the same. Nothing else changes; the release's tests (which need
an `objc_test_utils` crate it does not ship) are left out. The workspace takes the copy through `[patch.crates-io]`.

  python3 scripts/vendor/block.py           write the copy
  python3 scripts/vendor/block.py --check   fail when the copy is not what the release and the rule give
"""

import argparse
import hashlib
import io
import sys
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates" / "vendor" / "block"
NAME, VERSION = "block", "0.1.6"
SHA256 = "0d8c1fef690941d3e7788d328517591fecc684c084084702d6ff1641e993699a"
URL = f"https://static.crates.io/crates/{NAME}/{NAME}-{VERSION}.crate"

BEFORE = "enum Class { }\n"
AFTER = """/// KentOS: the Objective-C class, opaque. A zero-sized `#[repr(C)]` struct rather than an empty enum, so the extern
/// static below is not of an uninhabited type (rust-lang/rust#74840); only its address is ever taken.
#[repr(C)]
struct Class {
    _private: [u8; 0],
}
"""

CARGO = f"""# KentOS: the crates.io release of block {VERSION} (MIT) with one change, written by scripts/vendor/block.py
# (KENTOS.md). Not a workspace member; the workspace takes it through [patch.crates-io].
[package]
name = "{NAME}"
version = "{VERSION}"
authors = ["Steven Sheldon"]
description = "Rust interface for Apple's C language extension of blocks."
keywords = ["blocks", "osx", "ios", "objective-c"]
readme = "README.md"
repository = "http://github.com/SSheldon/rust-block"
documentation = "http://ssheldon.github.io/rust-objc/block/"
license = "MIT"

# The release's `extern` blocks name no ABI (the C ABI, as written in 2017): a crates.io dependency's lints are capped,
# a path dependency's are not; the one lint it raises stays quiet as it did.
[lints.rust]
missing_abi = "allow"
"""

NOTE = f"""# block {VERSION}, its extern static inhabited

The crates.io release of [block](https://github.com/SSheldon/rust-block) {VERSION} (MIT, Steven Sheldon), Apple's C
blocks for Rust, which wgpu-hal's Metal backend uses through the `metal` crate (iced 0.14, wgpu 27). One change in
`src/lib.rs`: `enum Class {{ }}` becomes an opaque zero-sized `#[repr(C)]` struct. The release's extern static
`_NSConcreteStackBlock: Class` was a static of an uninhabited type, a future error (rust-lang/rust#74840) that cargo
reported at every build; only its address is taken, which is the same. The release's tests are left out (they need an
`objc_test_utils` crate the release does not ship). Its `extern` blocks name no ABI (the C ABI, written so in 2017):
as a path dependency its lints are not capped as a crates.io one's are, so the manifest allows that one lint.

Written by `python3 scripts/vendor/block.py` from the release's archive (its SHA-256 is the one Cargo locked);
`--check` fails when this folder is not what the release and the rule give. Do not edit the files by hand. When iced
and wgpu move to `objc2`'s blocks, take the copy out and the patch with it.
"""


def archive():
    """The release's archive: Cargo's download cache when it has it, crates.io otherwise; its SHA-256 checked."""
    for cached in sorted((Path.home() / ".cargo" / "registry" / "cache").glob(f"*/{NAME}-{VERSION}.crate")):
        data = cached.read_bytes()
        if hashlib.sha256(data).hexdigest() == SHA256:
            return data
    with urllib.request.urlopen(URL) as answer:
        data = answer.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise SystemExit(f"{URL}: SHA-256 Cargo'nun kilitlediği değil")
    return data


def files():
    """The copy's files: name → text."""
    out = {"Cargo.toml": CARGO, "KENTOS.md": NOTE}
    with tarfile.open(fileobj=io.BytesIO(archive()), mode="r:gz") as tar:
        for member in tar.getmembers():
            path = member.name.split("/", 1)[1] if "/" in member.name else member.name
            if not member.isfile() or path == "Cargo.toml":
                continue
            text = tar.extractfile(member).read().decode("utf-8")
            if path == "src/lib.rs":
                if text.count(BEFORE) != 1:
                    raise SystemExit("src/lib.rs'de `enum Class { }` bir kez yok: değişikliğin yerini elle bulun")
                text = text.replace(BEFORE, AFTER)
            out[path] = text
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the copy is not what the release and the rule give")
    args = ap.parse_args()
    want = files()
    have = {str(p.relative_to(OUT)): p.read_text(encoding="utf-8") for p in OUT.rglob("*") if p.is_file()} if OUT.exists() else {}
    if args.check:
        if have != want:
            wrong = sorted(set(have) ^ set(want) | {k for k in want if k in have and have[k] != want[k]})
            print(f"{OUT.relative_to(ROOT)} sürümden ve kuraldan çıkan değil ({', '.join(wrong)}); yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} sürümden ve kuraldan çıkanla aynı.")
        return 0
    for name in set(have) - set(want):
        (OUT / name).unlink()
    for name, text in want.items():
        path = OUT / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(want)} dosya.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
