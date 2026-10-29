#!/usr/bin/env python3
"""Tablodan nokta from every coordinate system its Koordinat sistemi offers (docs/adr/0223 §5, docs/adr/0741).

    python3 scripts/fixtures/table_points_systems_cases.py           # writes the table and the cases
    python3 scripts/fixtures/table_points_systems_cases.py --check   # writes nothing; compares

Writes fixtures/processing/v1/nokta-sistemler.csv and table-points-systems.json. The drawing is
table-points.kcad (WGS 84 / UTM 36N). For each system of the registry but the local one, two places near its
own central meridian are put into that system by PROJ (from TUREF latitude and longitude, along the EPSG path
crs_transform_cases.py names), written with fixed decimals into the system's own two columns, and moved into the
drawing's system by PROJ along the same named path; KentOS is not used. The rows of the other systems are empty
in a system's columns, so every run also says why it skipped them (“Y_<kod>” boş). A geographic system's cells
are degrees.

The tolerance is a millimetre: the datum paths are the same EPSG operations, but PROJ and the core compose them
in their own order (crs_sweep.py's known difference is a tenth of a millimetre).
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import crs_transform_cases as crs  # noqa: E402
import table_points_cases as tp  # noqa: E402

ROOT = HERE.parents[1]
PROC = ROOT / "fixtures/processing/v1"
FILE = "nokta-sistemler.csv"
TOL = 1e-3

SYSTEMS = [s for s in json.loads(crs.REGISTRY.read_text(encoding="utf-8"))["systems"] if s["kind"] != "local"]
LAYER_ID = tp.new_id(tp.LAYER, set())
PROJECT = next(s for s in SYSTEMS if s["srid"] == 32636)


def places(entry):
    """Two TUREF (lat, lon) places near the system's own central meridian; Ankara and Sinop for the others."""
    cm = entry.get("centralMeridian")
    if cm is None or entry["kind"] == "geographic" or entry.get("projection") == "Pseudo-Mercator":
        return [(39.9334, 32.8597), (42.0231, 35.1531)]
    return [(39.5, cm + 0.8), (41.2, cm - 1.3)]


def cell(entry, v):
    return f"{v:.9f}" if entry["kind"] == "geographic" else f"{v:.4f}"


def table():
    """The header and rows: Ad, then each system's Y and X; a row fills only its own system's two cells."""
    header = ["Ad"] + [h for s in SYSTEMS for h in (f"Y_{s['srid']}", f"X_{s['srid']}")]
    rows = []
    for s in SYSTEMS:
        for k, (lat, lon) in enumerate(places(s), 1):
            x, y = crs.into(s, lat, lon)
            row = {"Ad": f"S{s['srid']}-{k}", f"Y_{s['srid']}": cell(s, x), f"X_{s['srid']}": cell(s, y)}
            rows.append([row.get(h, "") for h in header])
    return header, rows


def text(header, rows):
    return "".join(";".join(r) + "\n" for r in [header] + rows)


def expect(entry, header, rows):
    """The run from `entry`'s columns: its own rows give points, every other row is skipped as empty."""
    ycol, xcol = f"Y_{entry['srid']}", f"X_{entry['srid']}"
    iy, ix = header.index(ycol), header.index(xcol)
    t, _, _ = crs.transformer(entry, PROJECT)
    added, report = [], []
    for i, r in enumerate(rows):
        if not r[iy]:
            report.append([str(i + 2), f"“{ycol}” boş"])
            continue
        x, y = t.transform(float(r[iy]), float(r[ix]))
        attrs = {h: r[k] for k, h in enumerate(header) if r[k].strip()}
        attrs["Ad"] = r[0]
        added.append({"kind": "point", "layerId": LAYER_ID, "attrs": attrs, "p": {"x": x, "y": y}})
    return {
        "status": "ok",
        "summary": f"{len(rows)} satırdan {len(added)} nokta yazıldı.",
        "undo": tp.UNDO,
        "layers": [{"id": LAYER_ID, "name": tp.LAYER, "style": {**tp.style(tp.COLOR), "fill": f"{tp.COLOR}26"}}],
        "added": added,
        "log": [{"level": "warn", "text": f"{len(report)} satır atlandı; nedenleri tabloda."}],
        "outputs": {"count": len(added), "table": {"columns": ["Satır", "Neden"], "rows": report}},
    }


def cases(header, rows):
    out = []
    for s in SYSTEMS:
        values = {"sourceKind": "file", "file": FILE, "east": f"Y_{s['srid']}", "north": f"X_{s['srid']}", "name": "Ad",
                  "system": str(s["srid"])}
        out.append({
            "id": f"system-{s['srid']}",
            "title": f"Tablodan nokta: {s['name']} koordinatları UTM 36N'ye; diğer sistemlerin satırları boş diye atlanır",
            "document": "table-points.kcad",
            "run": {"tool": tp.TOOL},
            "values": values,
            "expect": expect(s, header, rows),
        })
    return out


def outputs():
    header, rows = table()
    doc = {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "documents": {}, "files": {FILE: FILE},
           "cases": cases(header, rows)}
    return [(PROC / "table-points-systems.json", tp.text_of(doc).encode("utf-8")), (PROC / FILE, text(header, rows).encode("utf-8"))]


def main():
    outs = outputs()
    if "--check" in sys.argv[1:]:
        bad = [p for p, data in outs if not p.exists() or p.read_bytes() != data]
        for p in bad:
            print(f"{p.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print("Tablodan nokta'nın sistem durumları tutarlı: fixtures/processing/v1/table-points-systems.json")
        return 0
    for p, data in outs:
        p.write_bytes(data)
    print(f"written: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
