#!/usr/bin/env python3
"""What was removed stays removed (docs/adr/9402).

The web app (apps/web) and the WASM bindings (crates/wasm) left the repository on 28 November 2026. Two things are
held here:

1. Neither folder exists, tracked or not. A merge from an older branch, or a tool that still writes to an old path
   (generated TypeScript, a WASM package), would bring files back without anyone asking.
2. The lines of source that still speak of them only become fewer. Comments written when the web app was the first
   client name its files (`apps/web/src/...`) and say “native and WASM”; they are cleaned in the files a change
   touches, and no new one is written. The counts are in removed_paths.baseline.json.

    python3 scripts/dev/removed_paths.py           # the check (make guard)
    python3 scripts/dev/removed_paths.py --write   # lowers the baseline to today's counts; writes nothing when one grew
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_suffix(".baseline.json")

# Folders that must not exist.
GONE = ["apps/web", "crates/wasm"]

# What is counted, in tracked source outside the documents (docs/, plan/, *.md keep the history as it was written).
MENTIONS = {
    "apps/web": re.compile(r"apps/web\b"),
    "wasm": re.compile(r"wasm", re.IGNORECASE),
}
SOURCE = (".rs", ".py", ".toml", ".mjs", ".js", ".ts", ".sh", ".wgsl", ".yaml", ".yml", ".css")
# This guard and its baseline name what they guard.
OWN = {"scripts/dev/removed_paths.py", "scripts/dev/removed_paths.baseline.json"}


def tracked() -> list[str]:
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout
    return [
        f
        for f in out.splitlines()
        if f.endswith(SOURCE)
        and not f.startswith(("docs/", "plan/", "crates/vendor/", "node_modules/"))
        and f != "Cargo.lock"
        and f not in OWN
    ]


def counts() -> dict[str, int]:
    found = dict.fromkeys(MENTIONS, 0)
    for f in tracked():
        try:
            text = (ROOT / f).read_text(encoding="utf-8")
        except (UnicodeDecodeError, FileNotFoundError):
            continue
        for line in text.splitlines():
            for name, pattern in MENTIONS.items():
                if pattern.search(line):
                    found[name] += 1
    return found


def main() -> int:
    back = [p for p in GONE if (ROOT / p).exists()]
    for p in back:
        print(f"{p} geri gelmiş: bu klasör 28 Kasım 2026'da kaldırıldı (ADR 9402). İçindekini silin ya da yeni yerine taşıyın.")
    now = counts()
    base = json.loads(BASELINE.read_text(encoding="utf-8")) if BASELINE.exists() else {}
    if "--write" in sys.argv[1:]:
        # The baseline only goes down: with a count grown nothing is written (the first run writes what it finds).
        grown = [name for name, n in now.items() if base and n > base.get(name, 0)]
        if grown:
            for name in grown:
                print(f"“{name}” anan kaynak satırı {base.get(name, 0)} → {now[name]} arttı; taban yazılmadı.")
            return 1
        BASELINE.write_text(json.dumps(now, indent=1) + "\n", encoding="utf-8")
        print(f"Taban yazıldı: {now}")
        return 1 if back else 0
    grown = False
    for name, n in now.items():
        allowed = base.get(name, 0)
        if n > allowed:
            grown = True
            print(f"“{name}” anan kaynak satırı {allowed} → {n} arttı: kaldırılan web uygulamasına ve WASM'a yeni atıf yazılmaz (ADR 9402).")
        elif n < allowed:
            grown = True
            print(f"“{name}” anan kaynak satırı {allowed} → {n} azaldı; tabanı düşürün (--write).")
    if back or grown:
        return 1
    print(f"Kaldırılanlar yerinde değil; atıflar tabanda: {now}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
