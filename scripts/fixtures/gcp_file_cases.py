#!/usr/bin/env python3
"""Control points to and from a file (docs/adr/0518 §4): QGIS's georeferencer's `.points`, read here with Python's
`csv` module, no KentOS code.

    python3 scripts/fixtures/gcp_file_cases.py           # writes fixtures/raster/v1/gcp/cases.json
    python3 scripts/fixtures/gcp_file_cases.py --check   # writes nothing; compares

- `qgis.points` as QGIS 3 writes it: a `#CRS:` line, the header `mapX,mapY,sourceX,sourceY,enable,dX,dY,residual`, the
  source's y the negative row; `enable` 0 a point left out;
- `kentos.points` as KentOS writes it: two columns more, `role` (control, check, off) and `name`;
- the written text of a set of points (`write`): the header, then per point the target, the pixel with its row negated,
  enable (1 for a control point), the residual's parts and length, the role and the name; numbers in Python's `repr`
  (the shortest text that reads back), 0 for a zero.
"""
import csv
import io
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "raster" / "v1" / "gcp"
OUT = DIR / "cases.json"


def read(path):
    text = path.read_text(encoding="utf-8").lstrip("﻿")
    crs, rows = None, []
    for line in text.splitlines():
        if line.startswith("#CRS:"):
            crs = line[len("#CRS:"):].strip() or None
        elif line.strip() and not line.startswith("#"):
            rows.append(line)
    reader = csv.DictReader(io.StringIO("\n".join(rows)))
    points = []
    for row in reader:
        row = {k.lower(): (v or "").strip() for k, v in row.items()}
        role = row.get("role", "")
        enabled = float(row["enable"]) != 0.0 if row.get("enable") else True
        p = {}
        if row.get("name"):
            p["name"] = row["name"]
        p["pixel"] = [float(row["sourcex"]), -float(row["sourcey"]) + 0.0]
        p["target"] = [float(row["mapx"]), float(row["mapy"])]
        if role == "check":
            p["check"] = True
        elif role == "off" or (role == "" and not enabled):
            p["off"] = True
        points.append(p)
    return {"points": points, "crs": crs}


def num(v):
    return "0" if v == 0 else repr(float(v)).removesuffix(".0") if float(v).is_integer() else repr(float(v))


def write(points, residuals, crs=None):
    # The targets' system as QGIS writes it, a `#CRS:` line first, the definition on one line (docs/adr/0594 §2).
    out = [f"#CRS: {' '.join(crs.split())}"] if crs else []
    out.append("mapX,mapY,sourceX,sourceY,enable,dX,dY,residual,role,name")
    for p, r in zip(points, residuals):
        dx, dy = r if r is not None else (0.0, 0.0)
        role = "check" if p.get("check") else "off" if p.get("off") else "control"
        control = int(not p.get("check") and not p.get("off"))
        name = "".join(ch for ch in p.get("name", "") if ch != "," and ch.isprintable())
        out.append(",".join([num(p["target"][0]), num(p["target"][1]), num(p["pixel"][0]), num(-p["pixel"][1]),
                             str(control), num(dx), num(dy), num(math.sqrt(dx * dx + dy * dy)), role, name]))
    return "\n".join(out) + "\n"


WRITE_POINTS = [
    {"name": "K1", "pixel": [80.5, 60.5], "target": [487240.25, 4420771.4]},
    {"name": "K,2", "pixel": [1940.5, 0.0], "target": [488170.25, 4420808.6]},
    {"pixel": [1000.5, 750.5], "target": [487722.75, 4420445.0], "check": True},
    {"pixel": [1500.25, 300.75], "target": [487955.0, 4420680.0], "off": True},
]
WRITE_RESIDUALS = [[0.004, -0.002], None, [0.0126, -0.0031], [0.5, 0.25]]
# A definition over lines, as a provider gives it: written on one line.
WRITE_CRS = 'PROJCS["ED50 / TM 30",\n  GEOGCS["ED50",DATUM["European_Datum_1950",SPHEROID["International 1924",6378388,297]]],\n  PROJECTION["Transverse_Mercator"],PARAMETER["central_meridian",30],UNIT["metre",1]]'


def build():
    return {
        "format": "kentos.gcp-file-cases",
        "version": 1,
        "reads": [{"file": name, **read(DIR / name)} for name in ("qgis.points", "kentos.points")],
        "writes": [{"points": WRITE_POINTS, "residuals": WRITE_RESIDUALS, "text": write(WRITE_POINTS, WRITE_RESIDUALS)},
                   {"points": WRITE_POINTS, "residuals": WRITE_RESIDUALS, "crs": WRITE_CRS,
                    "text": write(WRITE_POINTS, WRITE_RESIDUALS, WRITE_CRS)}],
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{OUT.relative_to(ROOT)} güncel.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")


main()
