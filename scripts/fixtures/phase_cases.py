#!/usr/bin/env python3
"""The phase view's table and looks (docs/adr/0346 §2), from the ADR, without KentOS code.

Writes fixtures/arch/v1/phases.json; `--check` compares and writes nothing.

- The table: in Tümü every phase as it is; Mevcut durum: existing and demolish as they are, new and temporary hidden;
  Yıkım planı: existing as it is, demolish dashed, new and temporary hidden; Yeni durum: existing faded, demolish
  hidden, new as it is, temporary dashed. An object without a phase is as it is in every view.
- A look on an object: hidden, it is not drawn; dashed, its own line type `dashed` and no line pattern of its own;
  faded, its own transparency 0.6, or its own when that is more.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arch/v1/phases.json"
VIEWS = ["all", "existing", "demolition", "proposed"]
PHASES = [None, "existing", "demolish", "new", "temporary"]
TABLE = {
    "all": {"existing": "shown", "demolish": "shown", "new": "shown", "temporary": "shown"},
    "existing": {"existing": "shown", "demolish": "shown", "new": "hidden", "temporary": "hidden"},
    "demolition": {"existing": "shown", "demolish": "dashed", "new": "hidden", "temporary": "hidden"},
    "proposed": {"existing": "faded", "demolish": "hidden", "new": "shown", "temporary": "dashed"},
}


def look(view, phase):
    return "shown" if phase is None else TABLE[view][phase]


def wall(phase, **base):
    e = {"kind": "wall", "id": 1, "layerId": "duvar", "attrs": {}, **base,
         "pts": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}], "width": 0.2, "justify": "center"}
    if phase:
        e["arch"] = {"phase": phase}
    return e


def drawn(e, lk):
    if lk == "hidden":
        return None
    out = json.loads(json.dumps(e))
    if lk == "dashed":
        out["lineType"] = "dashed"
        out.pop("linePattern", None)
    if lk == "faded":
        out["transparency"] = max(out.get("transparency", 0.0), 0.6)
    return out


def build():
    table = [{"view": v, "phase": p, "look": look(v, p)} for v in VIEWS for p in PHASES]
    objects = []
    for name, e, view in [
        ("yıkılacak duvar Yıkım planında kesikli", wall("demolish"), "demolition"),
        ("yıkılacak duvarın kendi deseni kesikliye bırakır", wall("demolish", linePattern="Sınır"), "demolition"),
        ("mevcut duvar Yeni durumda soluk", wall("existing"), "proposed"),
        ("kendi saydamlığı daha çok olan soluk kalır", wall("existing", transparency=0.8), "proposed"),
        ("az saydam mevcut duvar soluklaşır", wall("existing", transparency=0.2), "proposed"),
        ("yeni duvar Mevcut durumda gizli", wall("new"), "existing"),
        ("geçici duvar Yeni durumda kesikli", wall("temporary"), "proposed"),
        ("evresiz duvar her görünümde olduğu gibi", wall(None), "demolition"),
        ("yeni duvar Tümü'nde olduğu gibi", wall("new"), "all"),
    ]:
        lk = look(view, e.get("arch", {}).get("phase"))
        objects.append({"name": name, "view": view, "entity": e, "look": lk, "drawn": drawn(e, lk)})
    return {"format": "kentos.phase-cases", "version": 1,
            "note": "ADR 0346 §2. scripts/fixtures/phase_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan.",
            "table": table, "objects": objects}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print("evreler: tablo ve görünüşler başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
