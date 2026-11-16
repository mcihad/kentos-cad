#!/usr/bin/env python3
"""A large IFC file for the reading engine's time and memory (docs/adr/8061 §7 "Süre ve bellek"): the section building
(fixtures/arch/v1/ifc/kesit-binasi.ifc) copied N times into one file, each copy's record numbers moved by a million
and its GlobalIds made its own, so nothing is said twice. Synthetic: every copy keeps its project, so the file has N
projects side by side. Written under .run/ (not kept in Git):

    python3 scripts/fixtures/ifc_large_case.py 700    # → .run/ifc-buyuk/kesit-binasi-x700.ifc, about 110 MB
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"


def tag(k):
    out = ""
    for _ in range(4):
        out = ALPHABET[k % 64] + out
        k //= 64
    return out


def main():
    copies = int(sys.argv[1]) if len(sys.argv) > 1 else 700
    text = (ROOT / "fixtures/arch/v1/ifc/kesit-binasi.ifc").read_text(encoding="ascii")
    head, rest = text.split("DATA;\n", 1)
    data = rest.rsplit("ENDSEC;", 1)[0]
    out = ROOT / ".run" / "ifc-buyuk" / f"kesit-binasi-x{copies}.ifc"
    out.parent.mkdir(parents=True, exist_ok=True)
    gid = re.compile(r"=(IFC[A-Z0-9_]+)\('([0-9A-Za-z_$]{22})'")
    ref = re.compile(r"#(\d+)")
    with out.open("w", encoding="ascii") as f:
        f.write(head + "DATA;\n")
        for k in range(copies):
            shift = k * 1_000_000
            part = ref.sub(lambda m: f"#{int(m.group(1)) + shift}", data)
            # The copy's tag in the GlobalId's characters 3–6: KentOS's GlobalIds count up in their last ones.
            part = gid.sub(lambda m: f"={m.group(1)}('{m.group(2)[:2]}{tag(k)}{m.group(2)[6:]}'", part)
            f.write(part)
        f.write("ENDSEC;\nEND-ISO-10303-21;\n")
    print(out.relative_to(ROOT), out.stat().st_size // 1_000_000, "MB")


if __name__ == "__main__":
    main()
