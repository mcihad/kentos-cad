#!/usr/bin/env python3
"""The facades' test building (docs/adr/8043): the section building (scripts/fixtures/section_building.py) with four
elevation marks 3.5 m outside its walls, each looking at its facade: 1 looking south at the north facade (y 9), 2
looking north at the south one (y 0), 3 looking west at the east one (x 12), 4 looking east at the west one (x 0).

Writes fixtures/arch/v1/facade-building.kcad with the independent reference writer, without KentOS code; `--check`
compares and writes nothing."""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as ref  # noqa: E402
import section_building as sb  # noqa: E402

OUT = sb.ROOT / "fixtures/arch/v1/facade-building.kcad"

# (number, bubble, looking: degrees from east, counter-clockwise).
for number, at, angle in (("1", (6.0, 12.5), 270.0), ("2", (6.0, -3.5), 90.0), ("3", (15.5, 4.5), 180.0),
                          ("4", (-3.5, 4.5), 0.0)):
    sb.add({"kind": "viewmark", "layerId": "isaret", "form": "elevation", "points": [sb.P(*at)], "angle": angle,
            "mark": number, "words": {"top": number, "bottom": "—"}})
sb.d["name"] = "Görünüş binası"


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    data = ref.container(ref.document(sb.d))
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != data:
            print(f"{OUT.relative_to(sb.ROOT)} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(sb.ROOT)}: {len(sb.ents)} nesne, başvuruyla aynı.")
        return 0
    OUT.write_bytes(data)
    print(f"{OUT.relative_to(sb.ROOT)}: {len(sb.ents)} nesne yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
