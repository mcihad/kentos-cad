#!/usr/bin/env python3
"""The platform's math library and the port's number helpers only become rarer (docs/adr/9403 §4).

A computed value is the same on every platform and is compared with its independent reference bit for bit only when
it comes from the numeric base (`kentos-num`) or, until its functions move there, from `libm`. Two things stand in
the way, and both are counted here; a count never grows:

1. Calls of the std float methods that go to the platform's C library (`x.sin()`, `x.atan2(y)`, `x.hypot(y)`…) or
   round differently from one build to another (`powi`, `mul_add`), in the crates that compute. The interface's own
   crates (the kit, the windows' drawing, the renderers) work in pixels and float32 and are left out; so are tests.
   What remains of `hypot` is float32 screen arithmetic in the desktop crates.
2. What is left of the helpers the geometry core was ported from TypeScript with (`kentos_num::port`: `js_cmp`,
   `truthy`, `or`, `stable_sort`, `js_hypot_n`): each waits for a written rule of its own, and no new use is
   written. So do the text helpers that still read and write numbers as JavaScript does (`js_trim`, `js_number`,
   `js_text`…; plan/bakim.md BK-50). The others have their rules in the numeric base already; the module `jsmath`
   is gone.

    python3 scripts/dev/platform_math.py           # the check (make guard)
    python3 scripts/dev/platform_math.py --write   # lowers the baseline to today's counts; writes nothing when one grew
    python3 scripts/dev/platform_math.py --list hypot   # where a count comes from
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_suffix(".baseline.json")

# Float methods that are the platform's, or not the same bits on every build.
METHODS = (
    "sin", "cos", "tan", "sin_cos", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh",
    "exp", "exp2", "exp_m1", "ln", "ln_1p", "log10", "log2", "powf", "powi", "cbrt", "hypot", "mul_add",
)
METHOD = re.compile(r"\.\s*(" + "|".join(METHODS) + r")\(")
# The port's leftover helpers by their names; `or` only as a free call (`x.or(…)` is an Option's own method).
PORT = re.compile(r"\b(js_[a-z0-9_]+|jsmath|truthy|stable_sort)\b|(?<![.\w])or\(")

# Not computing crates: the interface works in pixels; the vendored code is its authors'; the base holds the rule.
LEFT_OUT = (
    "crates/vendor/", "crates/shared/geographiclib-rs/", "crates/shared/num/", "crates/ui/", "crates/sheet-ui/",
    "crates/model-view-ui/", "crates/render/", "crates/imza/", "apps/ui-showcase/", "apps/launcher/", "apps/imza-",
)


def is_test(path: str) -> bool:
    name = path.rsplit("/", 1)[-1]
    return "/tests/" in path or "/benches/" in path or name == "tests.rs" or name.endswith("_tests.rs")


def sources() -> list[str]:
    out = subprocess.run(["git", "ls-files", "--", "crates", "apps"], cwd=ROOT, capture_output=True, text=True,
                         check=True).stdout
    return [f for f in out.splitlines() if f.endswith(".rs") and not f.startswith(LEFT_OUT) and not is_test(f)]


def code(line: str) -> str:
    """The line without its comment: a comment may name a method it does not call."""
    return line.split("//", 1)[0]


def counts(listing: str | None = None) -> dict[str, int]:
    found: dict[str, int] = {m: 0 for m in METHODS}
    found["js_"] = 0
    for f in sources():
        try:
            text = (ROOT / f).read_text(encoding="utf-8")
        except (UnicodeDecodeError, FileNotFoundError):
            continue
        in_tests = False
        for n, line in enumerate(text.splitlines(), 1):
            # A file's own test module, at its end, is test code.
            if line.startswith("mod tests") or line.startswith("mod ") and line.rstrip().endswith("_tests {"):
                in_tests = True
            if in_tests:
                continue
            body = code(line)
            for m in METHOD.findall(body):
                found[m] += 1
                if listing == m:
                    print(f"{f}:{n}: {line.strip()[:150]}")
            hits = len(PORT.findall(body))
            if hits:
                found["js_"] += hits
                if listing == "js_":
                    print(f"{f}:{n}: {line.strip()[:150]}")
    return found


def main() -> int:
    args = sys.argv[1:]
    if args[:1] == ["--list"]:
        total = counts(args[1])
        print(f"{args[1]}: {total[args[1]]}")
        return 0
    now = counts()
    base = json.loads(BASELINE.read_text(encoding="utf-8")) if BASELINE.exists() else {}
    grown = [k for k, n in now.items() if base and n > base.get(k, 0)]
    if "--write" in args:
        if grown:
            for k in grown:
                print(f"“{k}” {base.get(k, 0)} → {now[k]} arttı; taban yazılmadı.")
            return 1
        BASELINE.write_text(json.dumps(now, indent=1) + "\n", encoding="utf-8")
        print(f"Taban yazıldı: {sum(now.values())} çağrı.")
        return 0
    red = False
    for k, n in now.items():
        allowed = base.get(k, 0)
        if n > allowed:
            red = True
            what = "taşınan JavaScript yardımcısı" if k == "js_" else f"platformun `{k}` yöntemi"
            print(f"{what}: {allowed} → {n} arttı. Hesap sayısal temelden (kentos-num) ya da libm'den yapılır; "
                  f"yeri için: python3 scripts/dev/platform_math.py --list {k} (ADR 9403).")
        elif n < allowed:
            red = True
            print(f"“{k}” {allowed} → {n} azaldı; tabanı düşürün (--write).")
    if red:
        return 1
    rest = {k: n for k, n in now.items() if n}
    print(f"Platform matematiği ve taşınan yardımcılar tabanda: {rest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
