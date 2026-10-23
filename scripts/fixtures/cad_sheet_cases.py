"""Independent CAD paper and mixed-unit block regressions (ADR 0589).

DXF group streams and expected physical coordinates are written from integers
in micrometres and Fraction unit definitions, without importing KentOS code.
"""
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/formats/v1/cad-sheet"
UNITS = {"mm": (4, F(1, 1000)), "cm": (5, F(1, 100)), "m": (6, F(1)),
         "in": (1, F(127, 5000)), "ft": (2, F(381, 1250))}


def pairs(*items):
    return "".join(f"{k}\n{float(v):.17g}\n" if isinstance(v, F) else f"{k}\n{v}\n" for k, v in items)


def section(name, body):
    return pairs((0, "SECTION"), (2, name)) + body + pairs((0, "ENDSEC"))


def line(a, b, extra=()):
    return pairs((0, "LINE"), (8, "0"), *extra, (10, a[0]), (20, a[1]), (11, b[0]), (21, b[1]))


def insert(name, at, sx, sy):
    return pairs((0, "INSERT"), (8, "0"), (2, name), (10, at[0]), (20, at[1]),
                 (41, sx), (42, sy), (43, 1), (50, 0))


def block(name, base, body):
    return pairs((0, "BLOCK"), (2, name), (70, 0), (10, base[0]), (20, base[1])) + body + pairs((0, "ENDBLK"))


def build():
    files = {}
    units = []
    combos = [(a, b, None) for a in UNITS for b in UNITS]
    combos += [("mm", "mm", "cm"), ("mm", "in", "cm"), ("m", "ft", "mm")]
    for index, (drawing, child, chosen) in enumerate(combos):
        dc, ds = UNITS[drawing]
        bc, bs = UNITS[child]
        parent = list(UNITS)[(index + 1) % len(UNITS)]
        pc, ps = UNITS[parent]
        selected = UNITS[chosen][1] if chosen else ds
        multiplier = selected / ds
        # Nonzero base and asymmetric scaling expose errors hidden by an origin-only square.
        base = [F(1, 4) / bs, F(1, 2) / bs]
        end = [F(9, 4) / bs, F(31, 20) / bs]
        header = section("HEADER", pairs((9, "$ACADVER"), (1, "AC1032"), (9, "$INSUNITS"), (70, dc)))
        tables = section("TABLES", pairs((0, "TABLE"), (2, "BLOCK_RECORD"), (70, 2),
            (0, "BLOCK_RECORD"), (2, "CHILD"), (70, bc),
            (0, "BLOCK_RECORD"), (2, "PARENT"), (70, pc), (0, "ENDTAB")))
        definitions = section("BLOCKS", block("CHILD", base, line(base, end)) + block("PARENT", [0, 0],
            insert("CHILD", [F(3) / ps, F(4) / ps], bs / ps, 2 * bs / ps)))
        entities = section("ENTITIES", insert("CHILD", [F(10) / ds, F(20) / ds], bs / ds, 2 * bs / ds)
            + insert("PARENT", [F(10) / ds, F(20) / ds], ps / ds, ps / ds))
        name = f"units-{index:02}.dxf"
        files[name] = header + tables + definitions + entities + pairs((0, "EOF"))
        units.append({"file": name, "drawing": drawing, "child": child, "parent": parent,
            "fileUnit": chosen, "multiplier": float(multiplier),
            "childBase": [.25, .5], "childEnd": [2.25, 1.55],
            "direct": [[float(10 * multiplier), float(20 * multiplier)],
                       [float(12 * multiplier), float(F(221, 10) * multiplier)]],
            "nested": [[float(13 * multiplier), float(24 * multiplier)],
                       [float(15 * multiplier), float(F(261, 10) * multiplier)]]})
    # Points are µm on paper. These cover zero-span, short, reversed and
    # narrow closed paths without changing a drawing's geometry to fit its handles.
    shapes = [
        ("line", [[10000, 20000], [30000, 20000]]),
        ("line", [[30000, 20000], [10000, 20000]]),
        ("line", [[10000, 20000], [10000, 40000]]),
        ("line", [[10000, 40000], [10000, 20000]]),
        ("line", [[0, 0], [500, 0]]), ("line", [[0, 0], [0, 500]]),
        ("line", [[0, 0], [500, 500]]), ("line", [[-1000, -1000], [-501, -999]]),
        ("line", [[10000, 20000], [10001, 40000]]),
        ("line", [[10000, 20000], [30000, 20001]]),
        ("polyline", [[0, 0], [500, 0], [999, 1]]),
        ("polyline", [[0, 0], [0, 500], [1, 999]]),
        ("polyline", [[0, 0], [1000, 0], [2000, 0]]),
        ("polyline", [[0, 2000], [0, 1000], [0, 0]]),
        ("polygon", [[0, 0], [20000, 0], [20000, 100], [0, 100]]),
        ("polygon", [[0, 0], [100, 0], [100, 20000], [0, 20000]]),
        ("polygon", [[10000, 10000], [10500, 10000], [10500, 10500], [10000, 10500]]),
        ("polygon", [[-1000, -1000], [-1, -1000], [-1, -500]]),
        ("polyline", [[0, 0], [999, 999], [1, 998]]),
        ("line", [[1000, 1000], [2000, 2000]]),
    ]
    paper = []
    body = line([0, 0], [1000, 0])
    for index, (kind, points) in enumerate(shapes):
        sid = f"paper-{index:02}"
        paper.append({"id": sid, "kind": kind, "pointsUm": points})
        extra = ((67, 1), (410, "Paper"))
        if kind == "line":
            body += line([F(x, 1000) for x in points[0]], [F(x, 1000) for x in points[1]], extra)
        else:
            body += pairs((0, "LWPOLYLINE"), (8, "0"), *extra, (90, len(points)), (70, int(kind == "polygon")))
            for x, y in points:
                body += pairs((10, F(x, 1000)), (20, F(y, 1000)))
    files["thin-paper.dxf"] = section("HEADER", pairs((9, "$ACADVER"), (1, "AC1032"), (9, "$INSUNITS"), (70, 4))) + section("ENTITIES", body) + section("OBJECTS", pairs(
        (0, "LAYOUT"), (100, "AcDbPlotSettings"), (44, 420), (45, 297), (72, 1),
        (100, "AcDbLayout"), (1, "Paper"), (71, 1))) + pairs((0, "EOF"))
    files["cases.json"] = json.dumps({"format": "kentos.cad-sheet-cases", "version": 1,
        "note": "Independent physical coordinates; scripts/fixtures/cad_sheet_cases.py, ADR 0589.",
        "units": units, "paper": paper}, ensure_ascii=False, indent=2) + "\n"
    return files


if __name__ == "__main__":
    files = build()
    if "--check" in sys.argv:
        bad = [name for name, content in files.items() if not (DIR / name).exists() or (DIR / name).read_text() != content]
        if bad:
            raise SystemExit(f"CAD sheet fixtures differ: {bad}")
    else:
        DIR.mkdir(parents=True, exist_ok=True)
        for name, content in files.items():
            (DIR / name).write_text(content)
    print(f"CAD-SHEET: {len(files)} files, 28 unit cases, 20 paper cases")
