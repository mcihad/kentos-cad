"""Writes fixtures/formats/v1/dxf-units: small DXF files and what reading them gives (docs/adr/0257).

The files are written here by hand from the DXF reference (HEADER's $INSUNITS, BLOCK_RECORD's group 70), and the
expectations from the ADR's rule, without KentOS code:

- `fileUnit` given: the values are in it;
- else in a local project the declared unit, any AutoCAD defines (DXF reference, $INSUNITS); in a project with a
  coordinate system metres, another declared unit said (`unit_ignored`);
- a file that declares none, or a code AutoCAD does not define: the numbers are not scaled, the project's unit
  (metres with a system) is taken and said (`unit_assumed`);
- a block whose record names another unit than the file's is said (`block_units_differ`), AutoCAD's own blocks
  (`*Model_Space`, `*Paper_Space`, a dimension's `*D…`) aside; its objects are in the file's unit, its inserts' scale
  holds the ratio;
- a length in a unit is turned into metres with the unit's exact ratio: ×1000 ÷ 1 000 000 is not used, one whole
  factor or $INSUNITS's fraction (an inch ×254 ÷10000, a US survey foot ×1200 ÷3937), one float64 multiplication then
  one division, as the reader turns a typed value into metres.

    python3 scripts/fixtures/dxf_units_cases.py           # writes the files
    python3 scripts/fixtures/dxf_units_cases.py --check   # compares them with the ones on disk
"""
import json
import sys
from decimal import Decimal, getcontext
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/formats/v1/dxf-units"

getcontext().prec = 60
# 648 000 / π astronomical units in metres, the nearest double (π to 50 digits).
PARSEC = float(Decimal(648_000) / Decimal("3.14159265358979323846264338327950288419716939937510")
               * Decimal(149_597_870_700))
# The units a user may choose (`fileUnit`) and their $INSUNITS codes.
CODES = {"in": 1, "ft": 2, "mi": 3, "mm": 4, "cm": 5, "m": 6, "km": 7, "yd": 10, "dm": 14, "usFt": 21}
# Every unit the DXF reference defines for $INSUNITS (0 is unitless), by code: the metres it is, as a whole-number
# fraction (mul, div) from its definition (an inch 25.4 mm, a foot 12 inches, a mile 5280 feet, a yard 3 feet, a US
# survey foot 1200/3937 m, its inch, yard and mile likewise; an astronomical unit 149 597 870 700 m, a light year
# 9 460 730 472 580 800 m, a parsec 648 000/π astronomical units as the nearest double), and its Turkish name.
AUTOCAD = {
    1: ((254.0, 10_000.0), "inç"), 2: ((3048.0, 10_000.0), "fit"), 3: ((1_609_344.0, 1000.0), "mil"),
    4: ((1.0, 1000.0), "milimetre"), 5: ((1.0, 100.0), "santimetre"), 6: ((1.0, 1.0), "metre"),
    7: ((1000.0, 1.0), "kilometre"), 8: ((254.0, 1e10), "mikroinç"), 9: ((254.0, 1e7), "binde bir inç"),
    10: ((9144.0, 10_000.0), "yarda"), 11: ((1.0, 1e10), "angstrom"), 12: ((1.0, 1e9), "nanometre"),
    13: ((1.0, 1e6), "mikrometre"), 14: ((1.0, 10.0), "desimetre"), 15: ((10.0, 1.0), "dekametre"),
    16: ((100.0, 1.0), "hektometre"), 17: ((1e9, 1.0), "gigametre"), 18: ((149_597_870_700.0, 1.0), "astronomik birim"),
    19: ((9_460_730_472_580_800.0, 1.0), "ışık yılı"), 20: ((PARSEC, 1.0), "parsek"),
    21: ((1200.0, 3937.0), "ABD ölçme fiti"), 22: ((100.0, 3937.0), "ABD ölçme inci"),
    23: ((3600.0, 3937.0), "ABD ölçme yardası"), 24: ((6_336_000.0, 3937.0), "ABD ölçme mili"),
}
OF_CODE = {v: k for k, v in CODES.items()}
PROJECT_UNITS = {"mm": "mm", "cm": "cm", "m": "m"}


def metres(v: float, code: int) -> float:
    """`v` in the unit of $INSUNITS `code` as metres: one multiplication, then one division."""
    (mul, div), _ = AUTOCAD[code]
    if mul == 1.0:
        return v / div
    if div == 1.0:
        return v * mul
    return v * mul / div


def factor(code: int) -> float:
    (mul, div), _ = AUTOCAD[code]
    return mul / div


def pairs(*groups) -> str:
    out = []
    for code, value in groups:
        out.append(f"{code:>3}")
        out.append(str(value))
    return "\n".join(out) + "\n"


def dxf(insunits: int | None, blocks: list[tuple[str, int | None, str]], entities: str) -> str:
    """A DXF R2000 file: its header, its block records (name, unit) and blocks (name, unit, objects), its entities."""
    head = [(0, "SECTION"), (2, "HEADER"), (9, "$ACADVER"), (1, "AC1015")]
    if insunits is not None:
        head += [(9, "$INSUNITS"), (70, insunits)]
    head += [(0, "ENDSEC")]
    tables = [(0, "SECTION"), (2, "TABLES"), (0, "TABLE"), (2, "LAYER"),
              (0, "LAYER"), (2, "0"), (70, 0), (62, 7), (6, "CONTINUOUS"),
              (0, "LAYER"), (2, "DUVAR"), (70, 0), (62, 1), (6, "CONTINUOUS"),
              (0, "ENDTAB"), (0, "TABLE"), (2, "BLOCK_RECORD")]
    for i, (name, unit, _) in enumerate(blocks):
        tables += [(0, "BLOCK_RECORD"), (5, f"{0x100 + i:X}"), (100, "AcDbSymbolTableRecord"),
                   (100, "AcDbBlockTableRecord"), (2, name)]
        if unit is not None:
            tables += [(70, unit)]
    tables += [(0, "ENDTAB"), (0, "ENDSEC")]
    text = pairs(*head) + pairs(*tables)
    text += pairs((0, "SECTION"), (2, "BLOCKS"))
    for name, _, objects in blocks:
        text += pairs((0, "BLOCK"), (8, "0"), (2, name), (70, 0), (10, 0.0), (20, 0.0), (30, 0.0), (3, name))
        text += objects
        text += pairs((0, "ENDBLK"), (8, "0"))
    text += pairs((0, "ENDSEC"), (0, "SECTION"), (2, "ENTITIES"))
    text += entities
    text += pairs((0, "ENDSEC"), (0, "EOF"))
    return text


def line(x1, y1, x2, y2, layer="DUVAR"):
    return pairs((0, "LINE"), (8, layer), (10, x1), (20, y1), (30, 0.0), (11, x2), (21, y2), (31, 0.0))


def circle(x, y, r, layer="DUVAR"):
    return pairs((0, "CIRCLE"), (8, layer), (10, x), (20, y), (30, 0.0), (40, r))


def text(x, y, h, words, layer="DUVAR"):
    return pairs((0, "TEXT"), (8, layer), (10, x), (20, y), (30, 0.0), (40, h), (1, words))


def mtext(x, y, h, width, words, layer="DUVAR"):
    return pairs((0, "MTEXT"), (8, layer), (100, "AcDbEntity"), (100, "AcDbMText"), (10, x), (20, y), (30, 0.0),
                 (40, h), (41, width), (71, 1), (1, words))


def insert(name, x, y, scale, layer="DUVAR"):
    return pairs((0, "INSERT"), (8, layer), (2, name), (10, x), (20, y), (30, 0.0), (41, scale), (42, scale),
                 (43, scale), (50, 0.0))


# A room in millimetres: a wall, a column, a label, a note box and a door.
ROOM_MM = (line(0.0, 0.0, 4000.0, 0.0) + circle(1000.0, 500.0, 250.0) + text(100.0, 3000.0, 250.0, "Salon")
           + mtext(100.0, 2500.0, 200.0, 3000.0, "Not kutusu") + insert("KAPI", 1000.0, 0.0, 1.0))
DOOR_MM = line(0.0, 0.0, 900.0, 0.0, "0")

# A plot in metres with a panel block drawn in millimetres, inserted at 1:1000 (AutoCAD's insertion scaling).
PLOT_M = line(0.0, 0.0, 12.5, 0.0) + insert("PANO", 5.0, 5.0, 0.001)
PANEL_MM = line(0.0, 0.0, 900.0, 0.0, "0")

FILES = {
    "mm.dxf": (4, [("KAPI", 4, DOOR_MM)], ROOM_MM),
    "unitless.dxf": (0, [("KAPI", None, DOOR_MM)], ROOM_MM),
    "no-header.dxf": (None, [("KAPI", None, DOOR_MM)], ROOM_MM),
    "unknown-code.dxf": (99, [("KAPI", None, DOOR_MM)], ROOM_MM),
    "inches.dxf": (1, [], line(0.0, 0.0, 100.0, 0.0) + circle(10.0, 10.0, 5.0)),
    "us-feet.dxf": (21, [], line(0.0, 0.0, 3937.0, 0.0)),
    "block-units.dxf": (6, [("*Model_Space", 4, ""), ("PANO", 4, PANEL_MM)], PLOT_M),
    "micrometres.dxf": (13, [], line(0.0, 0.0, 2_500_000.0, 0.0)),
}


def room(unit: int):
    """The room's measures in metres when its numbers are in the unit of $INSUNITS code `unit`."""
    k = lambda v: metres(v, unit)
    return {
        "line": [[k(0.0), k(0.0)], [k(4000.0), k(0.0)]],
        "circle": {"c": [k(1000.0), k(500.0)], "r": k(250.0)},
        "text": {"p": [k(100.0), k(3000.0)], "height": k(250.0)},
        "mtext": {"p": [k(100.0), k(2500.0)], "height": k(200.0), "boxWidth": k(3000.0)},
        "insert": {"p": [k(1000.0), k(0.0)], "scale": 1.0},
        "block": {"KAPI": [[k(0.0), k(0.0)], [k(900.0), k(0.0)]]},
    }


def decision(declared: int | None, used: int, source: str, assumed: bool):
    """The reader's decision: the declared code (none for 0 or no header), the code used and its name, why, the
    metres per unit, whether it was taken without being declared or chosen."""
    out = {"used": used, "name": AUTOCAD[used][1], "source": source, "factor": factor(used), "assumed": assumed}
    if declared not in (None, 0):
        out["declared"] = declared
    return out


def expect(file: str, project: str, file_unit: str | None):
    """What reading `file` gives in a project (`mm`, `cm`, `m` local; `crs` with a system), `fileUnit` given or not."""
    declared = FILES[file][0]
    known = declared if declared in AUTOCAD else None
    warnings = []
    if file_unit is not None:
        used, source, assumed = CODES[file_unit], "chosen", False
    elif project == "crs":
        used, source, assumed = 6, "metres", known is None
    elif known is not None:
        used, source, assumed = known, "declared", False
    else:
        used, source, assumed = CODES[PROJECT_UNITS[project]], "project", True
    if assumed:
        warnings.append("unit_assumed")
    if source == "metres" and known is not None and known != 6:
        warnings.append("unit_ignored")
    blocks = FILES[file][1]
    differ = [{"name": n, "code": u} for n, u, _ in blocks
              if u not in (None, 0) and u != (declared or 0) and not n.startswith("*")]
    if differ:
        warnings.append("block_units_differ")
    unit = decision(declared, used, source, assumed)
    if differ:
        unit["blocks"] = differ
    out = {"unit": unit, "warnings": warnings}
    if file in ("mm.dxf", "unitless.dxf", "no-header.dxf", "unknown-code.dxf"):
        out["room"] = room(used)
    elif file == "inches.dxf":
        out["line"] = [[0.0, 0.0], [metres(100.0, used), 0.0]]
        out["circle"] = {"c": [metres(10.0, used), metres(10.0, used)], "r": metres(5.0, used)}
    elif file == "us-feet.dxf":
        out["line"] = [[0.0, 0.0], [metres(3937.0, used), 0.0]]
    elif file == "block-units.dxf":
        out["line"] = [[0.0, 0.0], [metres(12.5, used), 0.0]]
        out["insert"] = {"p": [metres(5.0, used), metres(5.0, used)], "scale": 0.001}
        out["block"] = {"PANO": [[metres(0.0, used), 0.0], [metres(900.0, used), 0.0]]}
    elif file == "micrometres.dxf":
        out["line"] = [[0.0, 0.0], [metres(2_500_000.0, used), 0.0]]
    return out


CASES = [
    ("mm.dxf", "mm", None), ("mm.dxf", "m", None), ("mm.dxf", "crs", None), ("mm.dxf", "crs", "mm"),
    ("mm.dxf", "m", "cm"),
    ("unitless.dxf", "mm", None), ("unitless.dxf", "m", None), ("unitless.dxf", "crs", None), ("unitless.dxf", "mm", "cm"),
    ("no-header.dxf", "cm", None), ("no-header.dxf", "crs", "mm"),
    ("unknown-code.dxf", "mm", None), ("unknown-code.dxf", "crs", None),
    ("inches.dxf", "m", None), ("inches.dxf", "mm", None), ("inches.dxf", "crs", None), ("inches.dxf", "crs", "in"),
    ("us-feet.dxf", "m", None), ("us-feet.dxf", "crs", "usFt"), ("us-feet.dxf", "m", "ft"),
    ("block-units.dxf", "m", None), ("block-units.dxf", "crs", None),
    ("micrometres.dxf", "m", None), ("micrometres.dxf", "crs", None),
]


def build():
    files = {name: dxf(*spec) for name, spec in FILES.items()}
    cases = [{"file": f, "project": p, **({"fileUnit": u} if u else {}), "expect": expect(f, p, u)} for f, p, u in CASES]
    doc = {
        "format": "kentos.dxf-units",
        "version": 1,
        "note": "Written by scripts/fixtures/dxf_units_cases.py from docs/adr/0257 and the DXF reference; do not edit by hand.",
        "names": {k: AUTOCAD[CODES[k]][1] for k in CODES},
        "cases": cases,
    }
    return files, json.dumps(doc, ensure_ascii=False, indent=2) + "\n"


def main() -> int:
    files, cases = build()
    want = {**{name: text_ for name, text_ in files.items()}, "cases.json": cases}
    if "--check" in sys.argv[1:]:
        bad = [n for n, t in want.items() if not (DIR / n).exists() or (DIR / n).read_text(encoding="utf-8") != t]
        if bad:
            print(f"fixtures/formats/v1/dxf-units: yeniden üretilenle aynı değil: {', '.join(bad)} (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"dxf-units güncel: {len(files)} dosya, {len(CASES)} durum.")
        return 0
    DIR.mkdir(parents=True, exist_ok=True)
    for name, t in want.items():
        (DIR / name).write_text(t, encoding="utf-8")
    print(f"yazıldı: fixtures/formats/v1/dxf-units ({len(files)} dosya, {len(CASES)} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
