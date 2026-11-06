#!/usr/bin/env python3
"""`road.profiles.draw` with KGM's slope and platform rows over the frame (plan/road.md Y83, docs/adr/0989), without
KentOS code.

The corridor's crosses and each side's cut or fill are road_corridor_command_cases.py's (its `make`, the sections by
road_section_command_cases.py's reference); the drawing's frame, columns and object count road_profile_command_cases.py's
`drawing`. The rows: a side's fill slope where the side is in fill, its cut slope where in cut, "-" elsewhere; the
platform's width (the components' widths); the pavement's thickness; each value's run from the middle between two
crosses whose values differ. Slopes horizontal over vertical in small whole numbers (KGM's "3/2").

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with shapely \\
        python3 scripts/fixtures/road_profile_command_cases.py   # writes road.profiles.draw.json with these cases
"""

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402

ROWS = ["SOL DOL.ŞEVİ", "SOL YAR.ŞEVİ", "PLATFORM GEN.", "ÜST YAPI KAL.", "SAĞ DOL.ŞEVİ", "SAĞ YAR.ŞEVİ"]
# KGM's 0.45 m of pavement (the 2024 sample sheets' ÜST YAPI KAL.).
PAVEMENT = {"layers": [{"name": "Aşınma", "thickness": 0.05}, {"name": "Binder", "thickness": 0.06},
                       {"name": "Temel", "thickness": 0.34}], "edge": {"kind": "vertical"}}


def slope_text(h):
    for d in range(1, 11):
        n = h * d
        if abs(n - round(n)) < 1e-9 and round(n) >= 1:
            return f"{round(n)}/{d}"
    return f"{h:.2f}/1"


def rows_of(c, templates, s0, s1):
    _, _, _, regs = ccmd.make(c, templates=templates)
    at = []
    for ri, reg in enumerate(regs):
        own = next(x for x in templates if x["id"] == c["regions"][ri]["template"])
        pav = own.get("pavement")
        pv = f"{sum(lay['thickness'] for lay in pav['layers']):.2f} m." if pav else "-"
        for cr in reg["crosses"]:
            sec, s = cr["section"], cr["station"]
            if sec is None:
                continue
            tid, tl = ccmd.template_at(c, ri, s, templates)
            t = next(x for x in tl if x["id"] == tid)
            left, right = t["left"], t.get("right") or t["left"]

            def width(side):
                return sum(cm["width"] for cm in side.get("components", []))

            def fill(kind, side):
                return slope_text(side["fill"]) if kind == "fill" else "-"

            def cut(kind, side):
                return slope_text(side["cut"]) if kind == "cut" else "-"
            lk, rk = sec["left"]["kind"], sec["right"]["kind"]
            at.append((s, [fill(lk, left), cut(lk, left), f"{width(left) + width(right):.2f} m.", pv,
                           fill(rk, right), cut(rk, right)]))
    at.sort(key=lambda x: x[0])
    first = max([i for i, (s, _) in enumerate(at) if s <= s0 + 1e-9], default=0)
    shown = []
    for s, v in at[first:]:
        if s >= s1 + 1e-9:
            break
        shown.append((s, v))
    rows = []
    for k in range(6):
        cells = []
        for i, (s, v) in enumerate(shown):
            frm = max(s0 if i == 0 else 0.5 * (shown[i - 1][0] + s), s0)
            if not cells:
                cells.append({"from": s0, "to": s1, "text": v[k]})
            elif cells[-1]["text"] == v[k]:
                pass
            elif frm <= cells[-1]["from"] + 1e-9:
                cells[-1]["text"] = v[k]
            else:
                cells[-1]["to"] = frm
                cells.append({"from": frm, "to": s1, "text": v[k]})
        if shown and shown[0][0] > s0 + 1e-9:
            cells[0]["from"] = shown[0][0]
            cells.insert(0, {"from": s0, "to": shown[0][0], "text": ""})
        if shown and shown[-1][0] < s1 - 1e-9:
            cells[-1]["to"] = shown[-1][0]
            cells.append({"from": shown[-1][0], "to": s1, "text": ""})
        rows.append(cells)
    return rows


def rows_count(rows):
    """The rows' objects: the names' column's two edges; per row its line, its name, each cell's words (an empty cell
    off the corridor none) and a line between two cells."""
    return 2 + sum(2 + sum(1 for c in cells if c["text"]) + len(cells) - 1 for cells in rows)


def cases():
    base = ccmd.setup()
    first = len(base["entities"]) + 1
    road = base["settings"]["road"]
    paved = copy.deepcopy(base)
    for t in paved["settings"]["road"]["templates"]:
        if t["id"] == "tk-1":
            t["pavement"] = copy.deepcopy(PAVEMENT)
    out = []

    def ok(name, input_, setup_=None):
        b = setup_ or base
        templates = b["settings"]["road"]["templates"]
        c = next(x for x in b["settings"]["road"]["corridors"]
                 if input_["corridor"] in (x["id"], x["name"]))
        d = next(x for x in b["settings"]["road"]["profiles"] if x["id"] == input_["profile"])
        drawn, n, _ = pcmd.drawing(d, input_["at"], bands="kgm")
        s0, s1 = d["pvis"][0]["station"], d["pvis"][-1]["station"]
        rows = rows_of(c, templates, s0, s1)
        n += rows_count(rows)
        result = {"status": "completed", "output": {**drawn, "uids": "$any", "revision": "$current"}, "warnings": []}
        expect = {**pcmd.STEPPED, "road": b["settings"]["road"], "ids": list(range(1, first + n))}
        case = {"name": name, "steps": [{"op": "execute", "input": input_, "result": result, "expect": expect,
                                         "within": 1e-6}],
                "rows": rows}
        if setup_ is not None:
            case["setup"] = setup_
        out.append(case)

    def no(name, input_, code, message, path):
        out.append({"name": name, "steps": [{"op": "execute", "input": input_,
                    "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                    "expect": {**pcmd.UNCHANGED, "road": road}}]})

    at = {"profile": "b-1", "layerId": "0", "at": [0.0, -200.0], "bands": "kgm"}
    ok("Koridorun şev ve platform satırları: Tip 1 ve Tip 2, yarma ve dolgu", {**at, "corridor": "c-1"})
    ok("Üstyapılı tip kesit: ÜST YAPI KAL. 0.45 m.", {**at, "corridor": "Ana koridor"}, setup_=paved)
    no("KentOS bantlarıyla: ret", {**at, "bands": "kentos", "corridor": "c-1"}, "invalid_label",
       "Şev ve platform satırları yalnız KGM bantlarıyla (bands: kgm) çizilir.", "corridor")
    no("Olmayan koridor: ret", {**at, "corridor": "yok"}, "unknown_corridor",
       "“yok” kimlikli ya da adlı koridor projede yok.", "corridor")
    no("Başka aksın koridoru: ret", {**at, "corridor": "c-2"}, "invalid_label",
       "“Yan koridor” koridoru boykesitin aksında değil.", "corridor")
    return base, out


BANDS = ROOT / "fixtures/road/v1/corridor-bands.json"


def main():
    """The command cases go into road.profiles.draw.json (road_profile_command_cases.py writes it); this writes each
    drawing case's rows, which KentOS's own rows are held to (crates/native/application/src/road_corridor_bands.rs)."""
    base, cs = cases()
    data = {"note": "ADR 0989. scripts/fixtures/road_corridor_band_cases.py yazar; KentOS kodu yok.", "setup": base,
            "cases": [{"name": c["name"], "setup": c.get("setup"), "corridor": c["steps"][0]["input"]["corridor"],
                       "profile": c["steps"][0]["input"]["profile"], "rows": c["rows"]} for c in cs if "rows" in c]}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not BANDS.exists() or BANDS.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{BANDS.name} güncel değil: python3 {Path(__file__).name}")
        print(f"{BANDS.name}: güncel")
        return
    BANDS.write_text(text, encoding="utf-8")
    print(f"{BANDS.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
