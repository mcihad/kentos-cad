#!/usr/bin/env python3
"""ADR 0724 §5's shared cases: Metraj's stair rows (`cad.spaces.quantities`), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.spaces.quantities.stairs.json; `--check` compares and writes nothing. The drawing holds
seven stairs: a straight one with railings both sides on Zemin kat bound to its height, an L on 1. kat with a short
landing and a left railing, a U whose two turns share one landing, a curved one round the left with its centre post and
both railings, one on a locked and one on a hidden layer, and one clockwise curved on no level; and a line.

A stair's row, as the ADR gives it: its width, risers, riser height and tread; its height (its risers × their height);
its walking line's length, its landings in; its landings (its turns', those less than a tread apart one); its band's
plan area (a straight one's: half its width each side, mitred at its turns, the ring's shoelace; a curved one's: the
annulus between its two arcs, radius × width × its turn); its railings' length (a straight one's: the band's sides
5 cm inside; a curved one's: the arcs 5 cm inside, (radius ∓ (width/2 − 0.05)) × its turn, the left inner turning
left); a curved one's radius and its tread at its inner side (tread × (radius − width/2) / radius); whether it is
bound to its level's height. The level, phase and layer filters are the spaces' (ADR 0342 §4); a usage filter leaves
the stairs out. The totals add the stairs' area, railings and risers, said only when not zero.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_cases as ac  # noqa: E402  (a bulged line's arc)
import ramp_geometry as rg  # noqa: E402  (a walking line's stations, band and railings)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.spaces.quantities.stairs.json"
COMMAND = "cad.spaces.quantities"
STRAIGHT_RULE = ("alan: yürüme çizgisi boyunca, dönüşlerde gönyeli şeridin plan alanı; uzunluk: yürüme çizgisi, sahanlıklar "
                 "dahil; korkuluk: kenarlardan 5 cm içerideki çizgiler")
CURVED_RULE = ("alan: iki eş merkezli yayın arası (yarıçap × genişlik × dönüş açısı); uzunluk: yürüme çizgisinin yayı; "
               "korkuluk: kenarlardan 5 cm içerideki yaylar; iç basamak: basamak × (yarıçap − genişlik/2) ÷ yarıçap")
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.06, "height": 3.0}]
INSET = 0.05


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, visible=True, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def stair(id, layer_id, pts, width, risers, riser, tread, **extra):
    return {"kind": "stair", "id": id, "layerId": layer_id, "attrs": {}, "pts": [P(*p) for p in pts], "width": width,
            "risers": risers, "riserHeight": riser, "tread": tread, **extra}


SEED = [
    stair(1, "merdiven", [(0, 0), (4.76, 0)], 1.2, 18, 3.06 / 18, 0.28, railings="both", heightFromLevel=True,
          arch={"level": "zemin", "phase": "new"}),
    stair(2, "merdiven", [(0, 10), (2.8, 10), (2.8, 13)], 1.0, 18, 3.0 / 18, 0.28, landing=0.8, railings="left",
          arch={"level": "kat-1"}),
    stair(3, "merdiven", [(10, 0), (13, 0), (13, 1.2), (10, 1.2)], 1.1, 20, 0.15, 0.3, cut=8,
          arch={"level": "zemin", "phase": "existing"}),
    stair(4, "merdiven", [(21.5, 0), (18.5, 0)], 1.0, 16, 0.18, 0.28, bulges=[1.0, 0.0], post=0.3, railings="both",
          arch={"level": "kat-1"}),
    stair(5, "kilitli", [(0, 20), (3, 20)], 1.2, 12, 0.17, 0.27, arch={"level": "kat-1"}),
    stair(6, "gizli", [(0, 25), (2.8, 25)], 0.9, 10, 0.18, 0.3),
    stair(7, "merdiven", [(31.2, 0), (30, 1.2)], 1.0, 8, 0.17, 0.25, bulges=[-math.tan(math.pi / 8), 0.0],
          railings="right"),
    {"kind": "line", "id": 8, "layerId": "cizim", "attrs": {}, "a": P(0, 50), "b": P(5, 50)},
]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Merdivenler",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("merdiven", "Merdiven"), layer("kilitli", "Kilitli katman", locked=True),
               layer("gizli", "Gizli katman", visible=False), layer("cizim", "Çizim")],
    "activeLayer": "merdiven",
    "styles": {"items": [], "categories": []},
    "entities": SEED,
}
IDS = [e["id"] for e in SEED]


class Drawing:
    def __init__(self):
        self.objs = {e["id"]: copy.deepcopy(e) for e in SEED}
        self.levels = copy.deepcopy(LEVELS)
        self.next = max(IDS) + 1

    def ids(self):
        return sorted(self.objs)


def stair_landings(pts, land, tread):
    """A straight stair's landings: at each turn `land` long within its line, those less than a tread apart one."""
    at = rg.stations(pts)
    total = at[-1]
    out = []
    for k in range(1, len(pts) - 1):
        a, b = max(at[k] - land / 2.0, 0.0), min(at[k] + land / 2.0, total)
        if out and a - out[-1][1] < tread - 1e-9:
            out[-1] = (out[-1][0], max(out[-1][1], b))
        else:
            out.append((a, b))
    return out


def row(e):
    """A stair's row, as ADR 0724 §5 gives it."""
    pts = [rg.xy(p) for p in e["pts"]]
    width = e["width"]
    out = {"uid": f"$uidOf:{e['id']}"}
    if (e.get("arch") or {}).get("level") is not None:
        out["level"] = e["arch"]["level"]
    out["width"] = width
    out["risers"] = e["risers"]
    out["riserHeight"] = e["riserHeight"]
    out["tread"] = e["tread"]
    out["height"] = e["risers"] * e["riserHeight"]
    if e.get("bulges"):
        _, r, _, sweep = ac.arc_of(pts[0], pts[1], e["bulges"][0])
        turn = -1.0 if sweep < 0 else 1.0
        off = max(width / 2.0 - INSET, 0.0)
        out["length"] = abs(sweep) * r
        out["area"] = abs(sweep) * r * width
        sides = {"left": [1.0], "right": [-1.0], "both": [1.0, -1.0]}.get(e.get("railings"), [])
        rail = sum(abs(sweep) * (r - turn * k * off) for k in sides)
        if rail != 0.0:
            out["railingLength"] = rail
        out["radius"] = r
        out["innerTread"] = e["tread"] * (r - width / 2.0) / r
        rule = CURVED_RULE
    else:
        out["length"] = rg.stations(pts)[-1]
        count = len(stair_landings(pts, e.get("landing", width), e["tread"]))
        if count:
            out["landings"] = count
        out["area"] = rg.band_area(pts, width)
        rail = rg.railing_length(pts, width, e.get("railings"))
        if rail != 0.0:
            out["railingLength"] = rail
        rule = STRAIGHT_RULE
    if e.get("heightFromLevel"):
        out["heightFromLevel"] = True
    out["rule"] = rule
    return out


def answer(d, inp):
    """The command's answer for `inp` on `d`: a refusal, or its output."""
    known = [lv["id"] for lv in d.levels]
    for k, lv in enumerate(inp.get("levels") or []):
        if lv not in known:
            return {"status": "failed", "error": {"code": "level_not_found", "message": f"Projede “{lv}” kimlikli kat yok. Var olan bir katın kimliğini verin.",
                                                  "path": f"levels[{k}]"}}
    layers = [ly["id"] for ly in SETUP["layers"]]
    for k, ly in enumerate(inp.get("layers") or []):
        if ly not in layers:
            return {"status": "failed", "error": {"code": "layer_not_found", "message": f"“{ly}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                                                  "path": f"layers[{k}]"}}
    rows = []
    for i in d.ids():
        e = d.objs[i]
        if e["kind"] != "stair" or inp.get("usages") is not None:
            continue
        arch = e.get("arch") or {}
        if inp.get("layers") is not None and e["layerId"] not in inp["layers"]:
            continue
        if inp.get("levels") is not None and arch.get("level") not in inp["levels"]:
            continue
        if inp.get("phases") is not None and arch.get("phase") not in inp["phases"]:
            continue
        rows.append(row(e))
    out = {"spaces": [], "walls": [], "openings": []}
    if rows:
        out["stairs"] = rows
    totals = {"spaceArea": 0.0, "wallLength": 0.0, "wallNetArea": 0.0, "openings": 0, "openingArea": 0.0}
    area = sum(r["area"] for r in rows)
    rails = sum(r.get("railingLength", 0.0) for r in rows)
    risers = sum(r["risers"] for r in rows)
    if area != 0.0:
        totals["stairArea"] = area
    if rails != 0.0:
        totals["stairRailingLength"] = rails
    if risers:
        totals["stairRisers"] = risers
    out["totals"] = totals
    return {"status": "completed", "output": out, "warnings": []}


def query(d, inp, op="execute"):
    s = {"op": op, "command": COMMAND, "input": inp}
    result = answer(d, inp)
    if op == "validate" and result["status"] == "completed":
        result = {"status": "completed", "output": None, "warnings": []}
    s["result"] = result
    s["within"] = 1e-9
    s["expect"] = {"ids": d.ids(), "revision": "same"}
    return s


def edited(uid, operation):
    return {"op": "execute", "command": "cad.arch.edit",
            "input": {"uids": [f"$uidOf:{uid}"], "operation": {"kind": "set", **operation}}, "result": "$any"}


def cases():
    out = []

    def add(name, steps):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "steps": steps})

    for name, inp in [
        ("bütün merdivenler: düz, L, U, döner, kilitli, gizli ve saat yönünde döner", {}),
        ("zemin kat süzgeci: kata bağlı düz ve U", {"levels": ["zemin"]}),
        ("1. kat süzgeci: L, merkez direkli döner ve kilitli", {"levels": ["kat-1"]}),
        ("iki kat birden: katsız merdivenler dışarıda", {"levels": ["zemin", "kat-1"]}),
        ("evre süzgeci: yeni", {"phases": ["new"]}),
        ("evre süzgeci: var olan", {"phases": ["existing"]}),
        ("katman süzgeci: merdiven katmanı", {"layers": ["merdiven"]}),
        ("katman süzgeci: kilitli katmanın merdiveni de ölçülür", {"layers": ["kilitli"]}),
        ("katman süzgeci: gizli katmanın katsız merdiveni", {"layers": ["gizli"]}),
        ("kullanım süzgeci merdivenleri almaz", {"usages": ["Islak"]}),
        ("kat ve evre birlikte: var olan U", {"levels": ["zemin"], "phases": ["existing"]}),
        ("katsız çizginin katmanı: hiçbir merdiven yok", {"layers": ["cizim"]}),
    ]:
        add(name, [query(Drawing(), inp)])
    add("plan aynı cevabı verir, çizim değişmez", [query(Drawing(), {}, op="plan")])
    add("doğrula bir şey yazmaz", [query(Drawing(), {"levels": ["zemin"]}, op="validate")])
    add("olmayan kat reddedilir", [query(Drawing(), {"levels": ["yok"]})])
    add("olmayan katman reddedilir", [query(Drawing(), {"layers": ["yok"]})])

    # Zemin kat's height raised (a project setting): the bound stair's riser height and height follow.
    d = Drawing()
    raised = [{**LEVELS[0], "height": 3.4}, LEVELS[1]]
    d.levels = raised
    d.objs[1]["riserHeight"] = 3.4 / 18
    add("kat yüksekliği değişince bağlı merdivenin rıhtı ve yüksekliği izler",
        [{"op": "execute", "command": "cad.settings.set", "input": {"arch": {"levels": raised}}, "result": "$any"},
         query(d, {"levels": ["zemin"]})])

    # Railings both sides on the U: its railings in the totals.
    d = Drawing()
    d.objs[3]["railings"] = "both"
    add("U merdivene iki yan korkuluk: korkuluk toplamda",
        [edited(3, {"railings": "both"}), query(d, {"levels": ["zemin"]})])

    # The L's landing made its width: still one landing, the same line.
    d = Drawing()
    del d.objs[2]["landing"]
    add("L merdivenin sahanlığı kol genişliği olur", [edited(2, {"landing": None}), query(d, {"levels": ["kat-1"]})])

    # The curved stair widened: its area, inner tread and railings change.
    d = Drawing()
    d.objs[4]["width"] = 1.2
    add("döner merdiven genişleyince alanı, iç basamağı ve korkuluğu değişir",
        [edited(4, {"width": 1.2}), query(d, {"levels": ["kat-1"]})])

    # Deleted, then undone.
    d = Drawing()
    del d.objs[1]
    gone = query(d, {"levels": ["zemin"]})
    add("silinen merdiven metrajdan çıkar, geri alınınca döner",
        [{"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:1"]}, "result": "$any"}, gone,
         {"op": "undo", "returns": "Sil"}, query(Drawing(), {"levels": ["zemin"]})])
    return out


def document():
    note = ("ADR 0724 §5. Beklenenler scripts/fixtures/stair_quantity_cases.py'nin ADR'den yazılmış kurallarından "
            "(ramp_geometry.py, arch_cases.py), KentOS kodu olmadan. Kurulumdaki çizimde 7 merdiven ve bir çizgi var; yeni "
            "nesne 9'dan başlar.")
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Metraj: merdivenlerin satırları (rıht, yükseklik, yürüme çizgisi, sahanlık, plan alanı, korkuluk, döner "
                     "merdivenin yarıçapı ve iç basamağı, kat bağı); süzgeçler; retler",
            "note": note, "setup": SETUP, "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document()
    assert len(doc["cases"]) >= 20, len(doc["cases"])
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND} (merdivenler): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
