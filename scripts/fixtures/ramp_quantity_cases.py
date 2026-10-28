#!/usr/bin/env python3
"""ADR 0723 §3's shared cases: Metraj's ramp rows (`cad.spaces.quantities`), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.spaces.quantities.ramps.json; `--check` compares and writes nothing. The drawing holds
eight ramps: a straight one with railings both sides, an L bound to 1. kat 15 cm below it, a U going down, one on a
locked and one on a hidden layer, one whose landing takes its whole line, one with a straight corner (as En uzun kol
adds) and a flat one; and a line.

A ramp's row, as the ADR gives it (`ramp_geometry.py`): its centre line's length; its sloped runs' (the line less its
landings, those that touch made one); its landings; its band's plan area (half its width each side, mitred at the
inner points; the ring's shoelace); its slope as drawn (the rise's size over the sloped length, none when no run is
left); its start (its base, else 0) and its end (start plus rise); its offset over its level when bound; its railings'
lines' length (the band's sides 5 cm inside). The level, phase and layer filters are the spaces' (ADR 0342 §4); a
usage filter leaves the ramps out. The totals add the ramps' area and railings, said only when not zero.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_command_cases as acc  # noqa: E402  (En uzun kol's line: its float reference)
import ramp_geometry as rg  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.spaces.quantities.ramps.json"
COMMAND = "cad.spaces.quantities"
RULE = ("alan: orta çizgi boyunca, dönüşlerde gönyeli şeridin plan alanı; eğimli uzunluk: orta çizgi eksi sahanlıklar; "
        "eğim: kot farkı ÷ eğimli uzunluk; korkuluk: kenarlardan 5 cm içerideki çizgiler")
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, visible=True, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def ramp(id, layer_id, pts, width, rise, **extra):
    return {"kind": "ramp", "id": id, "layerId": layer_id, "attrs": {}, "pts": [P(*p) for p in pts], "width": width,
            "rise": rise, **extra}


SEED = [
    ramp(1, "rampa", [(0, 0), (12, 0)], 1.5, 0.6, slope=5.0, railings="both", arch={"level": "zemin", "phase": "new"}),
    ramp(2, "rampa", [(0, 10), (6, 10), (6, 16)], 1.2, 0.5, base=3.0 + -0.15, baseOffset=-0.15, landing=1.5,
         railings="left", arch={"level": "kat-1"}),
    ramp(3, "rampa", [(20, 0), (26, 0), (26, 3), (20, 3)], 1.4, -0.6, base=0.6, landing=1.5,
         arch={"level": "zemin", "phase": "existing"}),
    ramp(4, "kilitli", [(0, 20), (8, 20)], 1.2, 0.4, base=3.2, arch={"level": "kat-1"}),
    ramp(5, "gizli", [(0, 25), (6, 25)], 1.5, 0.3),
    ramp(6, "rampa", [(40, 0), (41, 0), (41, 1)], 1.0, 0.2, landing=5.0, arch={"level": "zemin"}),
    ramp(7, "rampa", [(0, 30), (10.75, 30), (22, 30)], 1.5, 1.0, landing=1.5, arch={"level": "zemin"}),
    ramp(8, "rampa", [(0, 40), (5, 40)], 1.2, 0.0, arch={"level": "kat-1", "phase": "existing"}),
    {"kind": "line", "id": 9, "layerId": "cizim", "attrs": {}, "a": P(0, 50), "b": P(5, 50)},
]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Rampalar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("rampa", "Rampa"), layer("kilitli", "Kilitli katman", locked=True),
               layer("gizli", "Gizli katman", visible=False), layer("cizim", "Çizim")],
    "activeLayer": "rampa",
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


def row(e):
    """A ramp's row, as ADR 0723 §3 gives it."""
    pts = [rg.xy(p) for p in e["pts"]]
    land = e.get("landing", e["width"])
    run = rg.sloped_length(pts, land)
    base = e.get("base", 0.0)
    out = {"uid": f"$uidOf:{e['id']}"}
    if (e.get("arch") or {}).get("level") is not None:
        out["level"] = e["arch"]["level"]
    out["width"] = e["width"]
    out["rise"] = e["rise"]
    if run > 0.0:
        out["slope"] = abs(e["rise"]) / run * 100.0
    out["run"] = run
    out["length"] = rg.stations(pts)[-1]
    count = len(rg.landings(pts, land))
    if count:
        out["landings"] = count
    out["area"] = rg.band_area(pts, e["width"])
    out["base"] = base
    out["top"] = base + e["rise"]
    if e.get("baseOffset") is not None:
        out["baseOffset"] = e["baseOffset"]
    rail = rg.railing_length(pts, e["width"], e.get("railings"))
    if rail != 0.0:
        out["railingLength"] = rail
    out["rule"] = RULE
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
        if e["kind"] != "ramp" or inp.get("usages") is not None:
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
        out["ramps"] = rows
    totals = {"spaceArea": 0.0, "wallLength": 0.0, "wallNetArea": 0.0, "openings": 0, "openingArea": 0.0}
    area = sum(r["area"] for r in rows)
    rails = sum(r.get("railingLength", 0.0) for r in rows)
    if area != 0.0:
        totals["rampArea"] = area
    if rails != 0.0:
        totals["rampRailingLength"] = rails
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


def cases():
    out = []

    def add(name, steps):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "steps": steps})

    for name, inp in [
        ("bütün rampalar: düz, L, U, kilitli, gizli, kolsuz, düz köşeli ve düz", {}),
        ("zemin kat süzgeci", {"levels": ["zemin"]}),
        ("1. kat süzgeci: kata bağlı ve kendi kotlu rampa", {"levels": ["kat-1"]}),
        ("iki kat birden: katsız rampa dışarıda", {"levels": ["zemin", "kat-1"]}),
        ("evre süzgeci: yeni", {"phases": ["new"]}),
        ("evre süzgeci: var olan", {"phases": ["existing"]}),
        ("katman süzgeci: rampa katmanı", {"layers": ["rampa"]}),
        ("katman süzgeci: kilitli katmanın rampası da ölçülür", {"layers": ["kilitli"]}),
        ("katman süzgeci: gizli katmanın katsız rampası", {"layers": ["gizli"]}),
        ("kullanım süzgeci rampaları almaz", {"usages": ["Islak"]}),
        ("kat ve evre birlikte: inen U rampa", {"levels": ["zemin"], "phases": ["existing"]}),
        ("katman ve evre: var olan rampalar", {"layers": ["rampa"], "phases": ["existing"]}),
        ("katsız çizginin katmanı: hiçbir rampa yok", {"layers": ["cizim"]}),
    ]:
        add(name, [query(Drawing(), inp)])
    add("plan aynı cevabı verir, çizim değişmez", [query(Drawing(), {}, op="plan")])
    add("doğrula bir şey yazmaz", [query(Drawing(), {"levels": ["zemin"]}, op="validate")])
    add("olmayan kat reddedilir", [query(Drawing(), {"levels": ["yok"]})])
    add("olmayan katman reddedilir", [query(Drawing(), {"layers": ["yok"]})])

    # The level raised (a project setting): the bound ramp's start and end follow; the one with its own base stays.
    d = Drawing()
    raised = [LEVELS[0], {**LEVELS[1], "elevation": 3.5}]
    d.levels = raised
    d.objs[2]["base"] = 3.5 + d.objs[2]["baseOffset"]
    add("kat yükselince bağlı rampanın başlangıcı ve bitişi izler",
        [{"op": "execute", "command": "cad.settings.set", "input": {"arch": {"levels": raised}}, "result": "$any"},
         query(d, {"levels": ["kat-1"]})])

    # Railings both sides on the U ramp.
    d = Drawing()
    d.objs[3]["railings"] = "both"
    add("U rampaya iki yan korkuluk: korkuluk toplamda",
        [{"op": "execute", "command": "cad.arch.edit",
          "input": {"uids": ["$uidOf:3"], "operation": {"kind": "set", "railings": "both"}}, "result": "$any"},
         query(d, {"levels": ["zemin"]})])

    # A longer landing on the L: its sloped length and slope change.
    d = Drawing()
    d.objs[2]["landing"] = 2.0
    add("L rampanın sahanlığı uzayınca eğimli uzunluğu kısalır, eğimi artar",
        [{"op": "execute", "command": "cad.arch.edit",
          "input": {"uids": ["$uidOf:2"], "operation": {"kind": "set", "landing": 2.0}}, "result": "$any"},
         query(d, {"levels": ["kat-1"]})])

    # A ramp written with En uzun kol: its landings counted (docs/adr/0723 §1).
    d = Drawing()
    new = {"pts": [P(0, 60), P(30, 60)], "width": 1.5, "rise": 1.2, "slope": 5.0, "landing": 1.5, "maxRun": 9.0, "fit": True}
    line = acc.float_ramp_landings([(0.0, 60.0), (30.0, 60.0)], 1.2, 5.0, 1.5, 9.0, True)
    made = ramp(d.next, "rampa", line, 1.5, 1.2, slope=5.0, landing=1.5)
    d.objs[d.next] = made
    d.next += 1
    add("en uzun kolla yazılan rampanın ara sahanlıkları sayılır",
        [{"op": "execute", "command": "cad.ramps.create", "input": {"layerId": "rampa", "ramps": [new]}, "result": "$any"},
         query(d, {"layers": ["rampa"]})])

    # Deleted, then undone.
    d = Drawing()
    del d.objs[1]
    gone = query(d, {"levels": ["zemin"]})
    add("silinen rampa metrajdan çıkar, geri alınınca döner",
        [{"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:1"]}, "result": "$any"}, gone,
         {"op": "undo", "returns": "Sil"}, query(Drawing(), {"levels": ["zemin"]})])
    return out


def document():
    note = ("ADR 0723 §3. Beklenenler scripts/fixtures/ramp_quantity_cases.py'nin ADR'den yazılmış kurallarından "
            "(ramp_geometry.py), KentOS kodu olmadan. Kurulumdaki çizimde 8 rampa ve bir çizgi var; yeni nesne 10'dan başlar.")
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Metraj: rampaların satırları (eğimli uzunluk, sahanlık, plan alanı, eğim, kotlar, korkuluk); süzgeçler; retler",
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
        print(f"{COMMAND} (rampalar): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
