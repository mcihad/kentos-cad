#!/usr/bin/env python3
"""The kitchen run's shared cases (“Mutfak dizisi”, docs/adr/0706, 0727), from the ADRs' rules, without KentOS code:
`cad.casework.run`, `cad.casework.edit` and `cad.casework.query`, and the drawing's following of a run's walls.

Writes fixtures/commands/v1/cad.casework.{run,edit,query}.json; `--check` compares and writes nothing. The rules are
`casework_model.py`'s (its module text gives them); this file builds the drawings, the steps and what they leave:
- a run's objects are written in the plan's order to the slots after the drawing's last (slots are never given twice);
- a run laid anew keeps its objects by their part and number (a module's, an upper's, a leg's filler, a corner's),
  updated in place; what it no longer has is removed, what it newly has added in the plan's order;
- the drawing keeps each run as a record: its recipe (the input without an expected revision), each leg's wall face
  tie or null, its objects with their parts, why it broke;
- a run follows its walls: a tied wall changed lays it on the path its faces give; a wall removed breaks it
  (`wallGone`); its every object moved together moves it; another command's change of one of its objects is refused
  (`casework_owned`), its every object removed removes it.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import casework_model as M  # noqa: E402
from casework_model import P, fixed, num, refusal  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
LIBRARY = json.loads((ROOT / "catalog/v1/blocks/kentos-temel.kblok").read_text())["blocks"]


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def setup(blocks=None, walls=False, unit=None):
    d = {
        "format": "kentos.document", "version": 1, "name": "Mutfak",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad"},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("mutfak", "Mutfak"), layer("duvar", "Duvar"), layer("kilitli", "Kilitli katman", locked=True),
                   layer("gizli", "Gizli katman", visible=False),
                   layer("plan", "Plan", kind="group", children=[layer("dolap", "Dolap")])],
        "activeLayer": "mutfak",
        "styles": {"items": [], "categories": []},
        "entities": [{"kind": "line", "id": 1, "layerId": "mutfak", "attrs": {}, "a": P(-1, -1), "b": P(6, -1)}],
    }
    if unit:
        d["settings"]["drawingUnit"] = unit
    if walls:
        # The room's inner faces: west x = 0, north y = 3.0, east x = 3.6 (each wall 0.20 thick, its path the middle).
        for i, (a, b) in enumerate([((-0.1, 3.1), (3.7, 3.1)), ((-0.1, -0.1), (-0.1, 3.1)), ((3.7, 3.1), (3.7, -0.1))]):
            d["entities"].append({"kind": "wall", "id": 2 + i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)],
                                  "width": 0.2, "justify": "center"})
    if blocks:
        d["blocks"] = blocks
    return d


def layers_of(d):
    out = {}

    def walk(nodes, locked=False, visible=True):
        for n in nodes:
            lk, vis = locked or n["locked"], visible and n["visible"]
            out[n["id"]] = {"type": n["type"], "name": n["name"], "locked": lk, "visible": vis}
            walk(n.get("children", []), lk, vis)
    walk(d["layers"])
    return out


def with_non_finite(inp, non_finite):
    inp = copy.deepcopy(inp)
    for path, word in non_finite.items():
        value = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}[word]
        node = inp
        parts = path.replace("]", "").replace("[", ".").split(".")
        for key in parts[:-1]:
            node = node[int(key)] if key.isdigit() else node[key]
        last = parts[-1]
        node[int(last) if last.isdigit() else last] = value
    return inp


class Drawing:
    """A drawing as the cases' steps leave it: its objects by slot, its blocks, its walls and openings, its runs."""

    def __init__(self, d):
        self.setup = d
        self.layers = layers_of(d)
        self.unit = d["settings"].get("drawingUnit", "m")
        self.objects = {e["id"]: copy.deepcopy(e) for e in d["entities"]}
        self.next = max(self.objects) + 1
        self.blocks = list(d.get("blocks", []))
        self.openings = []
        self.runs = []  # {"recipe", "ties", "members": [(part, index, slot)], "broken"}

    def ids(self):
        return sorted(self.objects)

    def walls(self):
        out = []
        for slot in self.ids():
            e = self.objects[slot]
            if e["kind"] == "wall":
                w = {k: e[k] for k in ("pts", "width") if k in e}
                w["justify"] = e.get("justify", "center")
                w["offset"] = e.get("offset", 0.0)
                w["closed"] = e.get("closed", False)
                w["uid"] = f"$uidOf:{slot}"
                out.append(w)
        return out

    def view(self):
        return {"blocks": self.blocks, "layers": self.layers, "walls": self.walls(), "openings": self.openings,
                "unit": self.unit}

    def record(self, run):
        r = {"recipe": run["recipe"], "ties": run["ties"],
             "members": [{"uid": f"$uidOf:{slot}", "part": part, **({"index": index} if index is not None else {})}
                         for part, index, slot in run["members"]]}
        if run.get("broken"):
            r["broken"] = run["broken"]
        return r

    def records(self):
        return [self.record(r) for r in self.runs]


def recipe_of(inp):
    return {k: v for k, v in inp.items() if k != "expectedRevision"}


def tied(ties):
    return [None if t is None else {"wall": t["wall"], "edge": t["edge"], "face": t["face"]} for t in ties]


def entity_at(e, slot):
    return {**e, "id": slot}


def built_output(b, slots):
    out = {"run": "$any", "uids": ["$any"] * len(slots), "ids": slots, "blocks": [x["name"] for x in b.added],
           "modules": b.modules, "legs": b.legs}
    if b.fridge is not None:
        out["fridge"] = b.fridge
    out["filler"] = b.filler
    out["revision"] = "$current"
    return out


def plan_output(b, dr):
    ids_of = {x["name"]: x["id"] for x in b.added}

    def own(e):
        name = e.get("block", "")[len("$blockOf:"):] if e.get("block", "").startswith("$blockOf:") else None
        return {**e, "id": 0, **({"block": ids_of[name]} if name in ids_of else {})}
    out = {"entities": [own(e) for _, _, e in b.members], "blocks": [x["name"] for x in b.added],
           "modules": b.modules, "legs": b.legs}
    if b.fridge is not None:
        out["fridge"] = b.fridge
    out["filler"] = b.filler
    out["revision"] = "$current"
    return out


def lay(dr, run_index, recipe, b):
    """Writes laid run `b` over run `run_index` (None: a new run): its objects kept by part, the rest removed or
    added; the slots of its objects in plan order and the slots it newly takes."""
    old = {} if run_index is None else {(p, i): s for p, i, s in dr.runs[run_index]["members"]}
    keep = {(p, i) for p, i, _ in b.members}
    for key, slot in old.items():
        if key not in keep:
            del dr.objects[slot]
    members, slots, added = [], [], []
    for part, index, e in b.members:
        slot = old.get((part, index))
        if slot is None:
            slot = dr.next
            dr.next += 1
            added.append(slot)
        was = dr.objects.get(slot)
        e = entity_at(e, slot)
        if was is not None:
            # What the run does not own stays the object's own (docs/adr/0727 §5).
            for k in ("label", "symbol", "labelPins", "attachments"):
                if k in was:
                    e[k] = was[k]
            extra = {k: v for k, v in was.get("attrs", {}).items() if k not in ("Modül", "Tür", "Açılma")}
            e["attrs"] = {**extra, **e["attrs"]} if extra else e["attrs"]
        dr.objects[slot] = e
        members.append((part, index, slot))
        slots.append(slot)
    for blk in b.added:
        dr.blocks.append(blk)
    run = {"recipe": recipe, "ties": tied(b.ties), "members": members, "broken": None}
    if run_index is None:
        dr.runs.append(run)
    else:
        dr.runs[run_index] = run
    return slots, added


def written_expect(dr, before_ids, extra=None):
    exp = {"ids": dr.ids(), "entities": {str(s): dr.objects[s] for s in dr.ids() if s not in before_ids or True},
           "casework": dr.records(), "canUndo": True, "dirty": True, "revision": "changed"}
    if extra:
        exp.update(extra)
    return exp


def run_step(dr, op, inp, non_finite=None, stale=False, command=None):
    """`cad.casework.run` on the drawing: its answer and what the drawing holds after."""
    ask = with_non_finite(inp, non_finite or {})
    if stale:
        ask["expectedRevision"] = "$stale"
    b, refused = M.run(dr.view(), ask, LIBRARY)
    s = {"op": op, **({"command": command} if command else {}), "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if refused is not None:
        s["result"] = refused
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": b.warnings}
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    if op == "plan":
        s["result"] = {"status": "completed", "output": plan_output(b, dr), "warnings": b.warnings}
        s["within"] = 1e-9
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    had_blocks = bool(dr.blocks)
    added_blocks = list(b.added)
    slots, _ = lay(dr, None, recipe_of(inp), b)
    s["result"] = {"status": "completed", "output": built_output(b, slots), "warnings": b.warnings}
    s["within"] = 1e-9
    s["expect"] = {"ids": dr.ids(), "entities": {str(x): dr.objects[x] for x in slots}, "casework": dr.records(),
                   "canUndo": True, "dirty": True, "revision": "changed"}
    if added_blocks and not had_blocks:
        s["expect"]["blocks"] = [{k: v for k, v in x.items() if k != "id"} for x in added_blocks]
    return s


def kitchen(**kw):
    inp = {"layerId": "mutfak", "from": P(0, 0), "to": P(3.2, 0), "sink": 2, "cooktop": 4}
    inp.update(kw)
    for k in [k for k, v in inp.items() if v is None]:
        del inp[k]
    return inp


def ell(**kw):
    """The room's L: along the north wall from x = 2.4 to its corner, then down the west wall to y = 0.6."""
    inp = {"layerId": "mutfak", "from": P(2.4, 3.0), "through": [P(0, 3.0)], "to": P(0, 0.6), "sink": 2,
           "cooktop": 5}
    inp.update(kw)
    for k in [k for k, v in inp.items() if v is None]:
        del inp[k]
    return inp


def you(**kw):
    """The room's U: up the west wall from y = 1.2, along the north wall, down the east wall to y = 1.2."""
    inp = {"layerId": "mutfak", "from": P(0, 1.2), "through": [P(0, 3.0), P(3.6, 3.0)], "to": P(3.6, 1.2),
           "side": "right", "sink": 4, "cooktop": 7}
    inp.update(kw)
    for k in [k for k, v in inp.items() if v is None]:
        del inp[k]
    return inp


def own_cabinet(width_rule=True, units="m"):
    """The drawing's own “Alt dolap”: a plain 0.60 square, with or without its width parameter."""
    side = 0.6 if units != "unitless" else 600.0
    b = {"id": "0192f5a1-7050-7000-8000-000000000001", "name": "Alt dolap", "base": P(0, 0), "units": units,
         "entities": [{"kind": "polyline", "id": 1, "layerId": "", "attrs": {},
                       "pts": [P(0, 0), P(side, 0), P(side, side), P(0, side)], "closed": True}]}
    if width_rule:
        if units == "unitless":
            rule = {"min": 300.0, "max": 1500.0, "step": 150.0}
        else:
            rule = {"min": 0.3, "max": 1.5, "step": 0.3}
        b["dynamic"] = {"parameters": [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(side, 0),
                                        "rule": rule}],
                        "actions": [{"kind": "stretch", "parameter": "Genişlik", "pieces": [1],
                                     "min": P(side - 0.1 * (1000 if units == "unitless" else 1), -0.1),
                                     "max": P(side + 0.1 * (1000 if units == "unitless" else 1), side + 0.1)}]}
    return b


def own_sink():
    """The drawing's own “Evye”: a 1.00 × 0.50 bowl."""
    return {"id": "0192f5a1-7050-7000-8000-000000000002", "name": "Evye", "base": P(0, 0), "units": "m",
            "entities": [{"kind": "polyline", "id": 1, "layerId": "", "attrs": {},
                          "pts": [P(0, 0), P(1.0, 0), P(1.0, 0.5), P(0, 0.5)], "closed": True}]}


def opening_step(dr, host_slot, at, number, kind=None, sill=None):
    """A door or window made on a wall by `cad.openings.create` before the run (its model's data kept)."""
    form = "window" if kind == "window" else "door"
    width = 1.2 if form == "window" else 0.9
    o = {"host": f"$uidOf:{host_slot}", "at": at, "number": number}
    if form == "window":
        o["form"] = "window"
        o["width"] = width
    if sill is not None:
        o["sill"] = sill
    slot = dr.next
    dr.next += 1
    sill_of = sill if sill is not None else (0.9 if form == "window" else None)
    dr.openings.append({"host": f"$uidOf:{host_slot}", "at": at, "width": width, "anchor": "center", "form": form,
                        "sill": sill_of, "number": number})
    dr.objects[slot] = {"kind": "opening", "id": slot}  # its fields the document's
    return {"op": "execute", "command": "cad.openings.create",
            "input": {"layerId": "duvar", "openings": [o]},
            "result": {"status": "completed", "output": {"uids": ["$any"], "ids": [slot], "revision": "$current"},
                       "warnings": []},
            "within": 1e-9}


# ── cad.casework.run ─────────────────────────────────────────────────────────

def run_cases():
    out = []

    def add(name, d, make):
        dr = Drawing(d)
        steps = make(dr)
        out.append({"name": name, **({"setup": d} if d is not BASE_SETUP else {}), "steps": steps})

    base = kitchen()
    add("Evye 2., ocak 4. modülde: doğrula yazmaz", BASE_SETUP, lambda dr: [run_step(dr, "validate", base)])
    add("Plan yazmadan dolapları, tezgâhı, kesimleri ve dolguyu gösterir", BASE_SETUP,
        lambda dr: [run_step(dr, "plan", base)])

    def undo(dr):
        made = run_step(dr, "execute", base)
        return [made, {"op": "undo", "returns": "Mutfak dizisi",
                       "expect": {"ids": [1], "casework": [], "canUndo": False, "canRedo": True, "revision": "changed"}}]
    add("Yazılır, kitaplığın blokları ve dizinin kaydı aynı adımda; tek geri al", BASE_SETUP, undo)
    add("Gizli katmana uyarıyla yazılır", BASE_SETUP, lambda dr: [run_step(dr, "execute", kitchen(layerId="gizli"))])
    for name, inp in [
        ("sağ yanda dizi: dolaplar aynalı, tezgâh ve kesimler öbür yanda", kitchen(side="right")),
        ("tam dolan çizgi: dolgu yok, delik yok", kitchen(to=P(2.4, 0), sink=None, cooktop=None)),
        ("45 cm modüller: ocak modülü 60 cm, 5 cm dolgu", kitchen(module=0.45, to=P(2.0, 0), sink=None, cooktop=2)),
        ("üst dolaplar modüllerin üstünde", kitchen(upper=True)),
        ("düşey duvar boyunca (90°)", kitchen(to=P(0, 3.2))),
        ("ters yönde çizilen dizi (180°)", kitchen(**{"from": P(3.2, 0)}, to=P(0, 0))),
        ("0,65 m derin tezgâh", kitchen(depth=0.65)),
        ("evyesiz, yalnız ocak", kitchen(sink=None, cooktop=3)),
        ("eğik (3-4-5) duvar boyunca", kitchen(to=P(2.4, 3.2))),
        ("renk ve çizgi kalınlığı nesnelere", kitchen(color="#E5484D", lineWeight=0.35)),
        ("çizgi tam evye modülü: tek modül", kitchen(to=P(0.9, 0), sink=1, cooktop=None)),
        ("bulaşık makinesi evyenin yanında", kitchen(dishwasher=3)),
        ("fırın ocağın altında", kitchen(oven=True)),
        ("buzdolabı başta: tezgâh ondan sonra", kitchen(fridge="start", to=P(3.8, 0))),
        ("buzdolabı sonda", kitchen(fridge="end", to=P(3.8, 0))),
        ("modüllerin kendi enleri: 45 cm tek kapı, 75 cm çift kapı",
         kitchen(perModule=[{"module": 1, "width": 0.45}, {"module": 3, "width": 0.75}])),
        ("modülün kapısı sağdan açılır", kitchen(perModule=[{"module": 1, "hand": "right"}])),
        ("bütün cihazlar ve üst dolaplarla", kitchen(to=P(4.4, 0), dishwasher=3, oven=True, fridge="end",
                                                     upper=True)),
    ]:
        add(name, BASE_SETUP, lambda dr, inp=inp: [run_step(dr, "execute", inp)])
    room = setup(walls=True)
    for name, inp in [
        ("L mutfak kör köşeyle: kollar kuzey ve batı duvarlarının yüzüne bağlanır", ell()),
        ("L mutfak açık köşeyle", ell(corner="open")),
        ("L mutfak buzdolabı sonda, bulaşık makinesi evyenin yanında", ell(fridge="end", dishwasher=3)),
        ("L mutfak üst dolaplarla ve 0,65 m tezgâhla", ell(upper=True, depth=0.65)),
        ("U mutfak: üç kol, iki kör köşe, sağ yanda", you()),
        ("U mutfak açık köşelerle, fırın ve buzdolabıyla", you(corner="open", oven=True, fridge="start")),
        ("duvarsız L: kollar bağlanmaz", ell(**{"from": P(2.4, 5.0)}, through=[P(0, 5.0)], to=P(0, 3.4))),
        ("kör köşedeki modülde bulaşık makinesi uyarılır (casework_blind_corner)", ell(dishwasher=3)),
        ("kör köşedeki 0,90 m evye modülü uyarılır", ell(sink=3, cooktop=5)),
        ("kör köşedeki fırınlı ocak uyarılır", ell(sink=1, cooktop=3, oven=True)),
        ("açık köşede kör modül yok: aynı bulaşık makinesi uyarılmaz", ell(corner="open", dishwasher=3)),
        ("U mutfakta kör köşedeki bulaşık makinesi ve köşeye taşan ocak uyarılır", you(dishwasher=3)),
    ]:
        add(name, room, lambda dr, inp=inp: [run_step(dr, "execute", inp)])

    def door(dr):
        return [opening_step(dr, 2, 1.6, "K1"), run_step(dr, "execute", ell())]
    add("kapının önüne gelen kol uyarılır (casework_clash)", room, door)

    def low_window(dr):
        return [opening_step(dr, 2, 1.6, "P1", kind="window", sill=0.6), run_step(dr, "execute", ell())]
    add("denizliği tezgâhtan alçak pencere uyarılır", room, low_window)

    def window(dr):
        return [opening_step(dr, 2, 1.6, "P2", kind="window"), run_step(dr, "execute", ell())]
    add("denizliği 0,90 m olan pencere uyarı vermez", room, window)
    own = setup([own_cabinet()])
    add("çizimin kendi Alt dolap bloğu kullanılır; kitaplıktan yalnız evye ve ocak gelir", own,
        lambda dr: [run_step(dr, "execute", kitchen(cooktop=3))])
    add("çizimin bloğunun kuralına uymayan modül reddedilir (0,30 adımlı)", own,
        lambda dr: [run_step(dr, "execute", kitchen(module=0.45))])
    add("çizimin kendi Alt dolap bloğu evyesiz kullanılır", own,
        lambda dr: [run_step(dr, "execute", kitchen(sink=None, cooktop=None, to=P(1.8, 0)))])
    bare = setup([own_cabinet(False)])
    add("Genişlik parametresi olmayan Alt dolap bloğu reddedilir", bare, lambda dr: [run_step(dr, "execute", kitchen())])
    unitless = setup([own_cabinet(units="unitless")], unit="mm")
    add("mm çizimde birimsiz dolap bloğu: ölçek ve Genişlik bloğun sayılarıyla", unitless,
        lambda dr: [run_step(dr, "execute", kitchen(sink=None, cooktop=None, to=P(2.0, 0),
                                                    perModule=[{"module": 1, "width": 0.9}]))])
    sink_own = setup([own_sink()])
    add("çizimin 1,00 m'lik evyesi 90 cm modüle sığmaz", sink_own, lambda dr: [run_step(dr, "execute", kitchen())])
    add("çizimin 1,00 m'lik evyesi 120 cm modüle sığar", sink_own,
        lambda dr: [run_step(dr, "execute", kitchen(perModule=[{"module": 2, "width": 1.2}], to=P(3.6, 0)))])
    for name, inp, kw in [
        ("başı ile sonu aynı çizgi reddedilir", kitchen(to=P(0, 0)), {}),
        ("kitaplığın enlerinden olmayan modül reddedilir", kitchen(module=0.5), {}),
        ("çok geniş modül reddedilir", kitchen(module=1.5), {}),
        ("ince tezgâh reddedilir", kitchen(depth=0.2), {}),
        ("evye için sığ tezgâh reddedilir (0,55 m ister)", kitchen(depth=0.5), {}),
        ("bir modüle yetmeyen çizgi reddedilir", kitchen(to=P(0.5, 0), sink=None, cooktop=None), {}),
        ("evye modülüne yetmeyen çizgi reddedilir", kitchen(to=P(0.8, 0), sink=1, cooktop=None), {}),
        ("buzdolabından sonra kalan bir modüle yetmez (negatif kalan)",
         kitchen(to=P(1.0, 0), sink=None, cooktop=None, fridge="start"), {}),
        ("dizinin dışındaki evye reddedilir", kitchen(sink=6), {}),
        ("dizinin dışındaki ocak reddedilir", kitchen(cooktop=5), {}),
        ("dizinin dışındaki bulaşık makinesi reddedilir", kitchen(dishwasher=9), {}),
        ("evye ve ocak aynı modülde reddedilir", kitchen(cooktop=2), {}),
        ("bulaşık makinesi evyenin modülünde reddedilir", kitchen(dishwasher=2), {}),
        ("ocaksız fırın reddedilir", kitchen(cooktop=None, oven=True), {}),
        ("bulaşık makinesinin eni 0,60'tan başka olamaz",
         kitchen(dishwasher=3, perModule=[{"module": 3, "width": 0.75}]), {}),
        ("cihazın açılması verilmez", kitchen(dishwasher=3, perModule=[{"module": 3, "hand": "left"}]), {}),
        ("çift kapılı modülün açılması verilmez", kitchen(perModule=[{"module": 2, "hand": "right"}]), {}),
        ("aynı modülün ayarı iki kez verilmez",
         kitchen(perModule=[{"module": 1, "width": 0.45}, {"module": 1, "hand": "right"}]), {}),
        ("modül numarası 0 olmaz", kitchen(perModule=[{"module": 0, "width": 0.45}]), {}),
        ("dizinin dışındaki modülün ayarı reddedilir", kitchen(perModule=[{"module": 8, "width": 0.45}]), {}),
        ("kuralına uymayan modül eni reddedilir", kitchen(perModule=[{"module": 1, "width": 0.5}]), {}),
        ("sonsuz başlangıç noktası reddedilir", kitchen(), {"non_finite": {"from.x": "NaN"}}),
        ("sonsuz tezgâh derinliği reddedilir", kitchen(depth=0.6), {"non_finite": {"depth": "Infinity"}}),
        ("sonsuz modül eni reddedilir", kitchen(perModule=[{"module": 1, "width": 0.45}]),
         {"non_finite": {"perModule[0].width": "NaN"}}),
        ("olmayan katman reddedilir", kitchen(layerId="yok"), {}),
        ("katman grubu reddedilir", kitchen(layerId="plan"), {}),
        ("kilitli katman reddedilir", kitchen(layerId="kilitli"), {}),
        ("geçersiz çizgi kalınlığı reddedilir", kitchen(lineWeight=120.0), {}),
        ("geçersiz beklenen sürüm reddedilir", kitchen(expectedRevision="07"), {}),
    ]:
        add(name, BASE_SETUP, lambda dr, inp=inp, kw=kw: [run_step(dr, "execute", inp, **kw)])
    for name, inp, kw in [
        ("üç köşeli dizi reddedilir", ell(through=[P(0, 3.0), P(0, 1.0), P(2.0, 1.0)], to=P(2.0, 2.0)), {}),
        ("dik olmayan köşe reddedilir", ell(to=P(1.0, 1.0)), {}),
        ("dolapların öbür yanına dönen köşe reddedilir", ell(side="right"), {}),
        ("dar köşe: ikinci kol köşe dolabından sonra bir modüle yetmez", ell(to=P(0, 2.0)), {}),
        ("kolun başı ile sonu aynı nokta reddedilir", ell(through=[P(2.4, 3.0)]), {}),
        ("sonsuz köşe reddedilir", ell(), {"non_finite": {"through[0].y": "Infinity"}}),
    ]:
        add(name, room, lambda dr, inp=inp, kw=kw: [run_step(dr, "execute", inp, **kw)])

    def stale(dr):
        made = run_step(dr, "execute", base)
        st = run_step(Drawing(BASE_SETUP), "execute", {**base, "expectedRevision": "$prepared"}, stale=True)
        st["expect"]["ids"] = made["expect"]["ids"]
        return [{"op": "captureRevision", "as": "prepared"}, made, st]
    add("sürüm değişince yazılmaz (çakışma)", BASE_SETUP, stale)
    return out


BASE_SETUP = setup()


# ── cad.casework.edit, and the walls' following ───────────────────────────────

EDIT_STEP = "Mutfak dizisini düzenle"
RELEASE_STEP = "Mutfak dizisini bırak"
OWNED = ("Mutfak dizisinin nesnesi diziden ayrı değiştirilemez ya da silinemez; diziyi düzenleyin (Öznitelikler › "
         "Mutfak dizisi), bırakın ya da dizinin tamamını silin.")


def prefixed(refused, prefix, whole=None):
    out = copy.deepcopy(refused)
    if whole is not None:
        out["error"]["path"] = whole
    elif "path" in out["error"]:
        out["error"]["path"] = prefix + out["error"]["path"]
    return out


def edit_step(dr, k, edit, op="execute", stale=False, non_finite=None):
    """`cad.casework.edit` on run `k` (named by its first object): its answer and what the drawing holds after."""
    run = dr.runs[k]
    inp = {"run": f"$uidOf:{run['members'][0][2]}", "edit": edit}
    s = {"op": op, "command": "cad.casework.edit", "input": copy.deepcopy(inp)}
    if stale:
        s["input"]["expectedRevision"] = "$prepared"
    if non_finite:
        s["nonFinite"] = non_finite
        edit = with_non_finite({"edit": edit}, non_finite)["edit"]
    kind = edit["kind"]
    if kind == "release":
        if op != "execute":
            s["result"] = ({"status": "completed", "output": None, "warnings": []} if op == "validate" else
                           {"status": "completed", "output": {"entities": [], "blocks": [], "modules": [], "legs": [],
                                                              "filler": 0.0, "revision": "$current"}, "warnings": []})
            s["expect"] = {"ids": dr.ids(), "revision": "same"}
            return s
        slots = [slot for _, _, slot in run["members"]]
        del dr.runs[k]
        s["result"] = {"status": "completed", "output": {
            "run": "$any", "uids": ["$any"] * len(slots), "ids": slots, "blocks": [], "modules": [], "legs": [],
            "filler": 0.0, "released": True, "revision": "$current"}, "warnings": []}
        s["expect"] = {"ids": dr.ids(), "casework": dr.records(), "canUndo": True, "revision": "changed"}
        return s
    if kind == "module":
        recipe = M.edited_module(run["recipe"], edit["module"], edit.get("width"), edit.get("hand"), edit.get("holds"))
        prefix, whole = "", "edit"
    else:
        recipe = recipe_of(edit["recipe"])
        prefix, whole = "edit.recipe.", None
    b, refused = M.run(dr.view(), recipe, LIBRARY)
    if refused is not None:
        s["result"] = prefixed(refused, prefix, whole)
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    if stale:
        out = refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                      "çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", "conflict")
        out["error"]["revision"] = "$current"
        s["result"] = out
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    warnings = [{**w, "path": "edit" if kind == "module" else prefix + w["path"]} for w in b.warnings]
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": warnings}
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    if op == "plan":
        s["result"] = {"status": "completed", "output": plan_output(b, dr), "warnings": warnings}
        s["within"] = 1e-9
        s["expect"] = {"ids": dr.ids(), "revision": "same"}
        return s
    slots, _ = lay(dr, k, recipe, b)
    s["result"] = {"status": "completed", "output": built_output(b, slots), "warnings": warnings}
    s["within"] = 1e-9
    s["expect"] = {"ids": dr.ids(), "entities": {str(x): dr.objects[x] for x in slots}, "casework": dr.records(),
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def moved_wall(dr, slot, dx, dy):
    """The wall in `slot` moved by `cad.entities.transform`; each run tied to it laid on the path its faces give."""
    before = copy.deepcopy(dr.objects[slot])
    after = copy.deepcopy(before)
    after["pts"] = [P(p["x"] + dx, p["y"] + dy) for p in before["pts"]]
    dr.objects[slot] = after
    uid = f"$uidOf:{slot}"
    wall_of = lambda e: {**{k: e[k] for k in ("pts", "width")}, "justify": e.get("justify", "center"),  # noqa: E731
                         "offset": e.get("offset", 0.0), "closed": e.get("closed", False), "uid": uid}
    changed_runs = []
    for k, run in enumerate(dr.runs):
        if not any(t is not None and t["wall"] == uid for t in run["ties"]):
            continue
        path = M.followed_path(run["recipe"], run["ties"], {uid: (wall_of(before), wall_of(after))})
        recipe = M.with_path(run["recipe"], path)
        b, refused = M.run(dr.view(), recipe, [])
        assert refused is None, refused
        slots, _ = lay(dr, k, recipe, b)
        changed_runs.extend(slots)
    s = {"op": "execute", "command": "cad.entities.transform",
         "input": {"uids": [uid], "transform": {"kind": "move", "dx": dx, "dy": dy}},
         "result": {"status": "completed", "output": {"changed": [uid], "created": [], "locked": [],
                                                      "revision": "$current"}, "warnings": []},
         "within": 1e-9}
    s["expect"] = {"ids": dr.ids(), "entities": {str(slot): after, **{str(x): dr.objects[x] for x in changed_runs}},
                   "casework": dr.records(), "canUndo": True, "revision": "changed"}
    return s


def moved_run(dr, k, dx, dy):
    """Every object of run `k` moved together: the run moves with them, its walls found anew."""
    run = dr.runs[k]
    slots = [slot for _, _, slot in run["members"]]
    counter = next(slot for part, _, slot in run["members"] if part == "counter")
    before = dr.objects[counter]["pts"]
    after = [P(p["x"] + dx, p["y"] + dy) for p in before]
    path = M.moved_path(run["recipe"], before, after)
    recipe = M.with_path(run["recipe"], path)
    b, refused = M.run(dr.view(), recipe, [])
    assert refused is None, refused
    laid, _ = lay(dr, k, recipe, b)
    uids = [f"$uidOf:{x}" for x in slots]
    s = {"op": "execute", "command": "cad.entities.transform",
         "input": {"uids": uids, "transform": {"kind": "move", "dx": dx, "dy": dy}},
         "result": {"status": "completed", "output": {"changed": uids, "created": [], "locked": [],
                                                      "revision": "$current"}, "warnings": []},
         "within": 1e-9}
    s["expect"] = {"ids": dr.ids(), "entities": {str(x): dr.objects[x] for x in laid}, "casework": dr.records(),
                   "canUndo": True, "revision": "changed"}
    return s


def owned_step(dr, transform_uids=None, delete_slot=None):
    """Another command's change of a run's object: refused, nothing written."""
    if delete_slot is not None:
        s = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [f"$uidOf:{delete_slot}"]}}
        uids = [f"$uidOf:{delete_slot}"]
    else:
        s = {"op": "execute", "command": "cad.entities.transform",
             "input": {"uids": transform_uids, "transform": {"kind": "move", "dx": 0.1, "dy": 0}}}
        uids = transform_uids
    s["result"] = {"status": "failed", "error": {
        "code": "casework_owned", "message": f"Yazma çizimi tutarsız bırakırdı; hiçbir şey yazılmadı: {OWNED}",
        "uids": uids}}
    s["expect"] = {"ids": dr.ids(), "casework": dr.records(), "revision": "same"}
    return s


def edit_cases():
    out = []
    room = setup(walls=True)

    def add(name, make, d=room):
        dr = Drawing(d)
        steps = make(dr)
        out.append({"name": name, **({"setup": d} if d is not room else {}), "steps": steps})

    def with_run(inp, then):
        def make(dr):
            steps = [run_step(dr, "execute", inp, command="cad.casework.run")]
            return steps + then(dr)
        return make

    for name, inp, edit in [
        ("bir modülün eni değişir, sonrakiler kayar, nesneler yerinde güncellenir", ell(),
         {"kind": "module", "module": 1, "width": 0.75}),
        ("bir modülün kapısı sağdan açılır", ell(), {"kind": "module", "module": 4, "hand": "right"}),
        ("bir modül bulaşık makinesi olur", ell(), {"kind": "module", "module": 3, "holds": "dishwasher"}),
        ("evye başka modüle taşınır", ell(), {"kind": "module", "module": 4, "holds": "sink"}),
        ("ocağın modülü alt dolap olur: ocak ve fırın kalkar", ell(oven=True),
         {"kind": "module", "module": 5, "holds": "base"}),
        ("modülün kendi değerleri kalkar", ell(perModule=[{"module": 1, "width": 0.45}]),
         {"kind": "module", "module": 1}),
        ("tarif değişir: derinlik ve üst dolaplar (yeni nesneler eklenir)", ell(),
         {"kind": "recipe", "recipe": ell(depth=0.65, upper=True)}),
        ("tarif değişir: açık köşe (köşe dolgusu yeniden çizilir)", ell(),
         {"kind": "recipe", "recipe": ell(corner="open")}),
        ("tarif değişir: buzdolabı sonda, ocak dördüncü modülde (bir modül eksilir)", ell(),
         {"kind": "recipe", "recipe": ell(fridge="end", cooktop=4)}),
        ("tarifte ocağın modülü dizinin dışında kalır: reddedilir", ell(),
         {"kind": "recipe", "recipe": ell(fridge="end")}),
        ("U mutfakta tek modülün eni", you(), {"kind": "module", "module": 2, "width": 0.45}),
        ("kuralına uymayan modül eni reddedilir (yol edit)", ell(), {"kind": "module", "module": 1, "width": 0.5}),
        ("çift kapılı modüle açılma verilmez", ell(), {"kind": "module", "module": 2, "hand": "left"}),
        ("modül genişler, kolun dolgusu biter", ell(), {"kind": "module", "module": 1, "width": 0.9}),
        ("genişleyen modül sonraki kola geçer, ocağın modülü dizinin dışında kalır: reddedilir", ell(),
         {"kind": "module", "module": 3, "width": 1.2}),
        ("tarifin geçersiz derinliği reddedilir (yol edit.recipe.depth)", ell(),
         {"kind": "recipe", "recipe": ell(depth=0.2)}),
        ("tarifin dik olmayan köşesi reddedilir", ell(), {"kind": "recipe", "recipe": ell(to=P(1.0, 1.0))}),
    ]:
        add(name, with_run(inp, lambda dr, edit=edit: [edit_step(dr, 0, edit)]))
    for name, recipe in [
        ("tarifin olmayan katmanı reddedilir (yol edit.recipe.layerId)", ell(layerId="yok")),
        ("tarifin katman grubu reddedilir", ell(layerId="plan")),
        ("tarifin kilitli katmanı reddedilir", ell(layerId="kilitli")),
        ("tarif gizli katmana taşınır: uyarıyla", ell(layerId="gizli")),
    ]:
        add(name, with_run(ell(), lambda dr, recipe=recipe: [edit_step(dr, 0, {"kind": "recipe", "recipe": recipe})]))

    def clash(dr):
        return [opening_step(dr, 2, 1.6, "K1"), run_step(dr, "execute", ell(), command="cad.casework.run"),
                edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.45})]
    add("kapının önündeki dizinin modülü düzenlenir: çakışma uyarısı edit'te", lambda dr: clash(dr))

    def bad_revision(dr):
        run = dr.runs[0]
        return [{"op": "execute", "command": "cad.casework.edit",
                 "input": {"run": f"$uidOf:{run['members'][0][2]}",
                           "edit": {"kind": "module", "module": 1, "width": 0.75}, "expectedRevision": "07"},
                 "result": refusal("invalid_revision", "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi "
                                   "bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                                   "expectedRevision"),
                 "expect": {"ids": dr.ids(), "revision": "same"}}]
    add("geçersiz beklenen sürüm reddedilir", with_run(ell(), bad_revision))
    add("sonsuz modül eni reddedilir (yol edit)", with_run(ell(), lambda dr: [
        edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.75}, non_finite={"edit.width": "NaN"})]))
    add("tarifin sonsuz derinliği reddedilir (yol edit.recipe.depth)", with_run(ell(), lambda dr: [
        edit_step(dr, 0, {"kind": "recipe", "recipe": ell(depth=0.6)}, non_finite={"edit.recipe.depth": "Infinity"})]))

    add("düzenleme plan ve doğrulama yazmaz", with_run(ell(), lambda dr: [
        edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.75}, op="validate"),
        edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.75}, op="plan")]))

    def undo_edit(dr):
        before = {"ids": dr.ids(), "casework": dr.records()}
        e = edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.75})
        return [e, {"op": "undo", "returns": EDIT_STEP, "expect": {**before, "revision": "changed"}}]
    add("düzenleme tek adımda geri alınır", with_run(ell(), undo_edit))

    add("dizi bırakılır: nesneler kalır, kaydı gider", with_run(ell(), lambda dr: [
        edit_step(dr, 0, {"kind": "release"})]))

    def not_found(dr):
        return [{"op": "execute", "command": "cad.casework.edit",
                 "input": {"run": "yok", "edit": {"kind": "release"}},
                 "result": refusal("casework_not_found", "“yok” çizimdeki bir mutfak dizisinin ya da bir "
                                   "nesnesinin kimliği değil. Dizinin kimliğini ya da bir nesnesinin kalıcı kimliğini "
                                   "verin.", "run"),
                 "expect": {"ids": dr.ids(), "revision": "same"}}]
    add("dizinin ya da nesnesinin kimliği olmayan yazı bulunmaz (casework_not_found)", with_run(ell(), not_found))

    def stale(dr):
        return [{"op": "captureRevision", "as": "prepared"},
                run_step(dr, "execute", kitchen(**{"from": P(10, 0)}, to=P(13.2, 0)), command="cad.casework.run"),
                edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.75}, stale=True)]
    add("sürüm değişince düzenleme yazılmaz", with_run(ell(), stale))

    # The walls' following (docs/adr/0727 §5).
    add("kuzey duvarı taşınınca L dizi izler: köşe yüzlerin kesişimi, kollar boylarını korur",
        with_run(ell(), lambda dr: [moved_wall(dr, 2, 0.0, 0.5)]))
    add("batı duvarı taşınınca köşe ve ikinci kol izler", with_run(ell(), lambda dr: [moved_wall(dr, 3, -0.3, 0.0)]))
    add("U dizinin doğu duvarı taşınınca orta kol uzar", with_run(you(), lambda dr: [moved_wall(dr, 4, 0.6, 0.0)]))
    add("düz dizi duvarıyla birlikte yana kayar",
        with_run(kitchen(**{"from": P(0.2, 3.0)}, to=P(3.4, 3.0), side="right", layerId="mutfak"),
                 lambda dr: [moved_wall(dr, 2, 0.5, 0.0)]))

    def wall_gone(dr):
        steps = [{"op": "captureUid", "as": "duvar", "id": 2}]
        del dr.objects[2]
        for run in dr.runs:
            run["broken"] = "wallGone"
        rec = json.loads(json.dumps(dr.records()).replace('"$uidOf:2"', '"$uid:duvar"'))
        steps.append({"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uid:duvar"]},
                      "result": {"status": "completed", "output": {"removed": ["$uid:duvar"], "locked": [],
                                                                   "revision": "$current"}, "warnings": []},
                      "expect": {"ids": dr.ids(), "casework": rec, "canUndo": True, "revision": "changed"}})
        return steps
    add("duvar silinince dizi kırılır (wallGone), nesneler yerinde kalır", with_run(ell(), wall_gone))

    def moved_together(dr):
        return [moved_run(dr, 0, 0.0, -2.0)]
    add("dizinin bütün nesneleri birlikte taşınınca dizi de taşınır, duvar bağı yeniden bulunur",
        with_run(ell(), moved_together))

    def one_moved(dr):
        slot = dr.runs[0]["members"][0][2]
        return [owned_step(dr, transform_uids=[f"$uidOf:{slot}"])]
    add("tek dolabı başka komutla taşımak reddedilir (casework_owned)", with_run(ell(), one_moved))

    def one_deleted(dr):
        slot = dr.runs[0]["members"][-1][2]
        return [owned_step(dr, delete_slot=slot)]
    add("tezgâhı tek başına silmek reddedilir (casework_owned)", with_run(ell(), one_deleted))

    def all_deleted(dr):
        run = dr.runs[0]
        slots = [slot for _, _, slot in run["members"]]
        steps = [{"op": "captureUid", "as": f"u{x}", "id": x} for x in slots]
        for x in slots:
            del dr.objects[x]
        dr.runs.clear()
        uids = [f"$uid:u{x}" for x in slots]
        steps.append({"op": "execute", "command": "cad.entities.delete", "input": {"uids": uids},
                      "result": {"status": "completed", "output": {"removed": uids, "locked": [],
                                                                   "revision": "$current"}, "warnings": []},
                      "expect": {"ids": dr.ids(), "casework": [], "canUndo": True, "revision": "changed"}})
        return steps
    add("dizinin bütün nesneleri silinince kaydı da gider", with_run(ell(), all_deleted))
    return out


# ── cad.casework.query ─────────────────────────────────────────────────────────

def info_of(dr, k, run_id="$any"):
    """What `cad.casework.query` says of run `k`: its recipe laid in the drawing as it is now (no library: its blocks
    are the drawing's), its objects, what keeps it."""
    run = dr.runs[k]
    b, refused = M.run(dr.view(), run["recipe"], [])
    assert refused is None, refused
    out = {"id": run_id, "recipe": run["recipe"], "legs": b.legs, "modules": b.modules}
    if b.fridge is not None:
        out["fridge"] = b.fridge
    out["filler"] = b.filler
    out["members"] = dr.record(run)["members"]
    out["missing"] = []
    if run.get("broken"):
        out["broken"] = run["broken"]
    return out


def query_step(dr, key=None, runs=None, op="execute", ids=None):
    """`cad.casework.query` (all runs, or the one `key` names): the runs `runs` (indexes; all by default)."""
    s = {"op": op, "command": "cad.casework.query", "input": {} if key is None else {"run": key}}
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
    else:
        which = range(len(dr.runs)) if runs is None else runs
        s["result"] = {"status": "completed", "output": {
            "runs": [info_of(dr, k, (ids or {}).get(k, "$any")) for k in which], "revision": "$current"},
            "warnings": []}
        s["within"] = 1e-9
    s["expect"] = {"ids": dr.ids(), "revision": "same"}
    return s


def query_cases():
    out = []
    room = setup(walls=True)

    def add(name, make, d=room):
        dr = Drawing(d)
        out.append({"name": name, **({"setup": d} if d is not room else {}), "steps": make(dr)})

    def made(dr, inp):
        return run_step(dr, "execute", inp, command="cad.casework.run")

    def capture(k, name="dizi"):
        return {"op": "captureCasework", "as": name, "index": k}

    add("dizisiz çizimde boş liste", lambda dr: [query_step(dr)])
    add("doğrulama dizi yokken de tamamlanır", lambda dr: [query_step(dr, op="validate")])
    add("düz dizinin modül tablosu", lambda dr: [made(dr, kitchen()), query_step(dr)], BASE_SETUP)
    add("plan yazmadan aynı cevabı verir", lambda dr: [made(dr, kitchen()), query_step(dr, op="plan")], BASE_SETUP)
    add("L dizi kollarıyla, köşesiyle ve duvar bağlarıyla", lambda dr: [made(dr, ell()), query_step(dr)])
    add("U dizi açık köşelerle", lambda dr: [made(dr, you(corner="open")), query_step(dr)])
    add("dizinin kimliğiyle sorulur",
        lambda dr: [made(dr, ell()), capture(0), query_step(dr, "$uid:dizi", [0], ids={0: "$uid:dizi"})])

    def by_member(dr):
        steps = [made(dr, ell()), made(dr, kitchen(**{"from": P(10, 0)}, to=P(13.2, 0)))]
        member = dr.runs[1]["members"][3][2]
        return steps + [query_step(dr, f"$uidOf:{member}", [1])]
    add("bir nesnesinin kimliğiyle o dizi sorulur", by_member)
    add("iki dizi çizim sırasıyla", lambda dr: [made(dr, ell()), made(dr, kitchen(**{"from": P(10, 0)},
                                                                                     to=P(13.2, 0))),
                                                 query_step(dr)])
    add("buzdolabı ve cihazlarla", lambda dr: [made(dr, kitchen(to=P(4.4, 0), dishwasher=3, oven=True,
                                                                 fridge="end")), query_step(dr)], BASE_SETUP)
    add("modüllerin kendi enleri ve açılmaları",
        lambda dr: [made(dr, kitchen(perModule=[{"module": 1, "width": 0.45, "hand": "right"},
                                                {"module": 3, "width": 0.75}])), query_step(dr)], BASE_SETUP)
    add("modül düzenlendikten sonra yeni tablo",
        lambda dr: [made(dr, ell()), edit_step(dr, 0, {"kind": "module", "module": 1, "width": 0.75}),
                    query_step(dr)])
    add("tarif düzenlendikten sonra üst dolaplarıyla",
        lambda dr: [made(dr, ell()), edit_step(dr, 0, {"kind": "recipe", "recipe": ell(upper=True)}),
                    query_step(dr)])
    add("bırakılan dizi listede yok",
        lambda dr: [made(dr, ell()), edit_step(dr, 0, {"kind": "release"}), query_step(dr)])

    def undone(dr):
        steps = [made(dr, kitchen())]
        dr.runs.clear()
        for slot in [s for s in dr.ids() if s != 1]:
            del dr.objects[slot]
        dr.blocks.clear()
        return steps + [{"op": "undo", "returns": "Mutfak dizisi"}, query_step(dr)]
    add("geri alınan dizi listede yok", undone, BASE_SETUP)
    add("duvarı taşınan dizinin yeni kolları",
        lambda dr: [made(dr, ell()), moved_wall(dr, 2, 0.0, 0.5), query_step(dr)])

    def broken(dr):
        steps = [made(dr, ell())]
        steps.append({"op": "captureUid", "as": "duvar", "id": 2})
        del dr.objects[2]
        for run in dr.runs:
            run["broken"] = "wallGone"
        steps.append({"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uid:duvar"]},
                      "result": {"status": "completed", "output": {"removed": ["$uid:duvar"], "locked": [],
                                                                   "revision": "$current"}, "warnings": []}})
        q = query_step(dr)
        q["result"] = json.loads(json.dumps(q["result"]).replace('"$uidOf:2"', '"$uid:duvar"'))
        return steps + [q]
    add("duvarı silinen dizi kırık (wallGone) görünür, kolları duvarsız", broken)
    add("bütün nesneleriyle taşınan dizi yeni yerinde",
        lambda dr: [made(dr, ell()), moved_run(dr, 0, 0.0, -2.0), query_step(dr)])

    def gone(dr):
        steps = [made(dr, kitchen())]
        run = dr.runs[0]
        slots = [slot for _, _, slot in run["members"]]
        steps += [{"op": "captureUid", "as": f"u{x}", "id": x} for x in slots]
        for x in slots:
            del dr.objects[x]
        dr.runs.clear()
        uids = [f"$uid:u{x}" for x in slots]
        steps.append({"op": "execute", "command": "cad.entities.delete", "input": {"uids": uids},
                      "result": {"status": "completed", "output": {"removed": uids, "locked": [],
                                                                   "revision": "$current"}, "warnings": []}})
        return steps + [query_step(dr)]
    add("bütün nesneleri silinen dizi listede yok", gone, BASE_SETUP)
    add("mm çizimde birimsiz dolap bloğuyla",
        lambda dr: [made(dr, kitchen(sink=None, cooktop=None, to=P(2.0, 0), perModule=[{"module": 1, "width": 0.9}])),
                    query_step(dr)], setup([own_cabinet(units="unitless")], unit="mm"))

    def unknown(dr):
        return [made(dr, kitchen()),
                {"op": "execute", "command": "cad.casework.query", "input": {"run": "yok"},
                 "result": refusal("casework_not_found", "“yok” çizimdeki bir mutfak dizisinin ya da bir "
                                   "nesnesinin kimliği değil. Dizinin kimliğini ya da bir nesnesinin kalıcı kimliğini "
                                   "verin.", "run"),
                 "expect": {"ids": dr.ids(), "revision": "same"}}]
    add("dizinin ya da nesnesinin kimliği olmayan yazı bulunmaz", unknown, BASE_SETUP)

    def random_id(dr):
        key = "0192f5a1-7050-7000-8000-00000000ffff"
        return [made(dr, kitchen()),
                {"op": "validate", "command": "cad.casework.query", "input": {"run": key},
                 "result": refusal("casework_not_found", f"“{key}” çizimdeki bir mutfak dizisinin ya da bir "
                                   "nesnesinin kimliği değil. Dizinin kimliğini ya da bir nesnesinin kalıcı kimliğini "
                                   "verin.", "run"),
                 "expect": {"ids": dr.ids(), "revision": "same"}}]
    add("olmayan dizi kimliği doğrulamada da bulunmaz", random_id, BASE_SETUP)
    return out


# ── writing ──────────────────────────────────────────────────────────────────

NOTE = ("Ortak durumlar scripts/fixtures/casework_cases.py'den (kurallar casework_model.py, docs/adr/0706 ve 0727; "
        "KentOS kodu olmadan). Kurulumda yer yoksa çizim: bir çizgi (1) ve dört katman, odada kuzey (2), batı (3) ve "
        "doğu (4) duvarları; iç yüzler y = 3,0, x = 0, x = 3,6.")


def documents():
    return {
        "cad.casework.run": {
            "format": "kentos.command-cases", "version": 1, "command": "cad.casework.run", "commandVersion": 1,
            "title": "Mutfak dizisi: düz, L ve U dizi, köşeler, cihazlar, tezgâh kesimleri, dizinin kaydı",
            "note": NOTE, "setup": BASE_SETUP, "cases": run_cases()},
        "cad.casework.edit": {
            "format": "kentos.command-cases", "version": 1, "command": "cad.casework.edit", "commandVersion": 1,
            "title": "Mutfak dizisini düzenle: modül, tarif, bırakma; duvarları ve nesneleriyle izleme",
            "note": NOTE, "setup": setup(walls=True), "cases": edit_cases()},
        "cad.casework.query": {
            "format": "kentos.command-cases", "version": 1, "command": "cad.casework.query", "commandVersion": 1,
            "title": "Mutfak dizilerini sorgula: modül tablosu, kollar, nesneler, kırık diziler",
            "note": NOTE, "setup": setup(walls=True), "cases": query_cases()},
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="compare with the files, write nothing")
    args = ap.parse_args()
    bad = False
    for command, doc in documents().items():
        path = OUT / f"{command}.json"
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)}: güncel değil", file=sys.stderr)
                bad = True
        else:
            path.write_text(text)
            print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
