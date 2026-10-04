#!/usr/bin/env python3
"""GeographicLib's Rust port with libm's functions (docs/adr/0171): writes crates/shared/geographiclib-rs from the
crates.io release of geographiclib-rs 0.2.7, its archive checked against the SHA-256 Cargo locked.

The port calls Rust's own float methods (`x.sin()`, `y.atan2(x)` …): on a Linux machine those are glibc's, in the
browser's WASM the `libm` port's, and the two part in the last bits; a geodesic's length and a polygon's area then
part far more (the area is a sum of each edge's area to the equator: 6·10⁻⁷ of a small area between the desktop and the
web). The geometry core keeps every target's answers equal bit for bit with `libm` (docs/adr/0008), so the copy here
calls `libm` the same way on every target: each transcendental method call becomes a method of the crate's own `Lm`
trait (`x.lm_sin()`, `y.lm_atan2(x)`), which calls `libm`. Nothing else changes; the binary `geodsolve` is left out.

  python3 scripts/vendor/geographiclib.py           write the copy
  python3 scripts/vendor/geographiclib.py --check   fail when the copy is not what the release and the rule give
"""

import argparse
import hashlib
import io
import re
import sys
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates" / "shared" / "geographiclib-rs"
NAME, VERSION = "geographiclib-rs", "0.2.7"
SHA256 = "c5a7f08910fd98737a6eda7568e7c5e645093e073328eeef49758cfe8b0489c7"
URL = f"https://static.crates.io/crates/{NAME}/{NAME}-{VERSION}.crate"

# The float methods whose results the platforms' maths libraries may round differently, and libm's names for them.
# sqrt is IEEE's correctly rounded operation everywhere; abs, copysign, to_degrees and the like are exact arithmetic.
METHODS = {"sin": "sin", "cos": "cos", "sin_cos": "sincos", "atan2": "atan2", "hypot": "hypot", "cbrt": "cbrt",
           "atanh": "atanh", "atan": "atan"}
# Every transcendental method a later release might start calling: none may be left behind.
TRANSCENDENTAL = ["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "asinh", "acosh",
                  "atanh", "exp", "exp2", "exp_m1", "ln", "log", "log2", "log10", "ln_1p", "powf", "cbrt", "hypot",
                  "sin_cos"]

LM = '''//! KentOS: the float methods whose results the platforms' maths libraries
//! may round differently, through `libm` on every target, so that a native
//! build and the browser's WASM answer alike bit for bit (docs/adr/0171;
//! written by scripts/vendor/geographiclib.py).

pub(crate) trait Lm: Sized {
    fn lm_sin(self) -> Self;
    fn lm_cos(self) -> Self;
    fn lm_sin_cos(self) -> (Self, Self);
    fn lm_atan2(self, x: Self) -> Self;
    fn lm_hypot(self, y: Self) -> Self;
    fn lm_cbrt(self) -> Self;
    fn lm_atanh(self) -> Self;
    fn lm_atan(self) -> Self;
}

impl Lm for f64 {
    fn lm_sin(self) -> f64 {
        libm::sin(self)
    }
    fn lm_cos(self) -> f64 {
        libm::cos(self)
    }
    fn lm_sin_cos(self) -> (f64, f64) {
        libm::sincos(self)
    }
    fn lm_atan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
    fn lm_hypot(self, y: f64) -> f64 {
        libm::hypot(self, y)
    }
    fn lm_cbrt(self) -> f64 {
        libm::cbrt(self)
    }
    fn lm_atanh(self) -> f64 {
        libm::atanh(self)
    }
    fn lm_atan(self) -> f64 {
        libm::atan(self)
    }
}
'''

CARGO = f'''# geographiclib-rs {VERSION} from crates.io with libm's functions on every target
# (docs/adr/0171; written by scripts/vendor/geographiclib.py, do not edit).
# Not a workspace member: the release's own code, linted by its authors.
[package]
name = "{NAME}"
version = "{VERSION}"
edition = "2018"
rust-version = "1.70.0"
description = "A port of geographiclib in Rust (KentOS: libm's functions on every target)."
license = "MIT"
repository = "https://github.com/georust/geographiclib-rs"
publish = false

[features]
default = ["accurate"]
test_full = []
test_short = []

[lib]
name = "geographiclib_rs"
path = "src/lib.rs"

[dependencies]
accurate = {{ version = "0.3", optional = true, default-features = false }}
libm = "0.2.8"
'''

NOTE = f'''# geographiclib-rs {VERSION}, libm on every target

The crates.io release of [geographiclib-rs](https://github.com/georust/geographiclib-rs) {VERSION} (MIT, `LICENSE`),
GeographicLib's geodesics and polygon areas (Karney 2013), for the plane, ellipsoid and ground values of
docs/adr/0171. One change: each transcendental float method call (`sin`, `cos`, `sin_cos`, `atan2`, `hypot`, `cbrt`,
`atanh`, `atan`) goes through `libm` (`src/lm.rs`), so that the desktop and the browser's WASM give the same bits
(docs/adr/0008); the release calls Rust's own methods, glibc's on the desktop. The binary `geodsolve` is left out.

Written by `python3 scripts/vendor/geographiclib.py` from the release's archive (its SHA-256 is Cargo's); `--check`
fails when this folder is not what the release and the rule give. Do not edit the files by hand.

Measured on Karney's GeodTest (500 000 geodesics with exact answers; 4 October 2026): the inverse problem's largest
length error is 11 nm here and 7.5 nm in the release (GeographicLib states 15 nm), 2.8 nm in both for geodesics under
10 km; the direct problem's 9.5 nm and 8.7 nm in latitude, 12.6 nm in longitude in both. Four of the release's unit
tests pin glibc's last bit with `assert_eq!` and part by one ulp here; the rest pass.
'''


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


def patch(text):
    """The source with each listed method call through `Lm`; `use crate::lm::Lm;` where one was changed."""
    changed = text
    for method in METHODS:
        changed = re.sub(rf"\.{method}\(", f".lm_{method}(", changed)
    left = [m for m in TRANSCENDENTAL if re.search(rf"\.{m}\(", changed)]
    if left:
        raise SystemExit(f"libm'e çevrilmemiş yöntemler: {', '.join(left)}; METHODS'a ekleyin")
    if changed == text:
        return text
    # After the module's inner doc comments and attributes: its first line that is neither.
    lines = changed.split("\n")
    at = 0
    while at < len(lines) and (lines[at].startswith("//!") or lines[at].startswith("#![") or not lines[at].strip()):
        at += 1
    lines.insert(at, "use crate::lm::Lm;")
    return "\n".join(lines)


def files():
    """The copy's files: name → text."""
    out = {"Cargo.toml": CARGO, "KENTOS.md": NOTE, "src/lm.rs": LM}
    with tarfile.open(fileobj=io.BytesIO(archive()), mode="r:gz") as tar:
        for member in tar.getmembers():
            path = member.name.split("/", 1)[1] if "/" in member.name else member.name
            if not member.isfile() or path.startswith("src/bin/"):
                continue
            if path in ("LICENSE", "README.md", "CHANGES.md") or (path.startswith("src/") and path.endswith(".rs")):
                text = tar.extractfile(member).read().decode("utf-8")
                if path == "src/lib.rs":
                    # The trait's module, beside the release's own.
                    text = text.replace("\nmod geomath;", "\nmod geomath;\nmod lm;", 1)
                    if "\nmod lm;" not in text:
                        raise SystemExit("src/lib.rs'de `mod geomath;` yok: modül yerini elle bulun")
                out[path] = patch(text) if path.startswith("src/") else text
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
    calls = sum(len(re.findall(r"\.lm_\w+\(", t)) for n, t in want.items() if n.startswith("src/") and n != "src/lm.rs")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(want)} dosya, {calls} çağrı libm'den.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
