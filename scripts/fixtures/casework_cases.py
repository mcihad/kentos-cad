#!/usr/bin/env python3
"""`cad.casework.run` (“Mutfak dizisi”, docs/adr/0706)'s shared cases, from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.casework.run.json; `--check` compares and writes nothing. The rules, as the ADR gives
them:

- the line from→to is where the cabinets' backs stand; its frame: the unit direction along it and the unit normal
  into the run's side (left: the direction turned a quarter counter-clockwise; right: clockwise);
- modules of `module` width (0.60 unless given) from `from`; with a sink one of them, the `sink`-th, 0.90 wide; their
  count the most the line holds (a hair, 1e-9, of slack); the rest of the line a filler at its end (none under 1 mm);
- each module a base cabinet (KentOS's “Alt dolap”, its width the parameter “Genişlik”, not written when 0.60, its
  default), turned along the line, mirrored on the right side; the sink (“Evye”, 0.80 wide) 5 cm in from its module's
  start and the line, the cooktop (“Ocak”, centred on its base point) at its module's middle 4 cm from the line; wall
  cabinets (“Üst dolap”) over the modules when asked; the filler a closed polyline, 0.60 deep; the countertop an area
  over the whole line, `depth` deep (0.60 unless given), counter-clockwise;
- a block the drawing names is used as it is, else KentOS's library's is brought in the same step;
- refusals in order: not_finite (from, to, module, depth), invalid_casework (no length, depth outside 0.30–1.20, the
  cabinet without its width parameter, the module or the sink module not one of the cabinet's widths, no module
  fits, the sink's then the cooktop's module outside the run, both in one, the wall cabinet's widths), then the line
  weight, the revision and the layer; warnings: layer_hidden, casework_filler.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.casework.run.json"
COMMAND = "cad.casework.run"
LIBRARY = json.loads((ROOT / "catalog/v1/blocks/kentos-temel.kblok").read_text())["blocks"]
SLACK = 1e-9
SINK_WIDTH = 0.9
SINK_INSET = 0.05
COOKTOP_INSET = 0.04


def num(x):
    """A number as Rust's `{}` writes an f64."""
    x = float(x)
    if x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def fixed(v, d):
    t = f"{abs(v):.{d}f}"
    return ("-" + t) if v < 0 and float(t) != 0 else t


def P(x, y):
    return {"x": float(x), "y": float(y)}


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def setup(blocks=None):
    d = {
        "format": "kentos.document", "version": 1, "name": "Mutfak",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad"},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("mutfak", "Mutfak"), layer("kilitli", "Kilitli katman", locked=True),
                   layer("gizli", "Gizli katman", visible=False),
                   layer("plan", "Plan", kind="group", children=[layer("dolap", "Dolap")])],
        "activeLayer": "mutfak",
        "styles": {"items": [], "categories": []},
        "entities": [{"kind": "line", "id": 1, "layerId": "mutfak", "attrs": {}, "a": P(-1, 0), "b": P(6, 0)}],
    }
    if blocks:
        d["blocks"] = blocks
    return d


def library(name):
    return next(b for b in LIBRARY if b["name"] == name)


def js_hypot(a, b):
    """`Math.hypot(a, b)` as the core computes it: scaled and compensated."""
    xs = [abs(a), abs(b)]
    m = max(xs)
    if m == 0.0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in xs:
        q = x / m
        summand = q * q - comp
        pre = total + summand
        comp = (pre - total) - summand
        total = pre
    return math.sqrt(total) * m


def width_param(block):
    for p in (block.get("dynamic") or {}).get("parameters", []):
        if p.get("kind") == "linear" and p.get("name") == "Genişlik":
            return p
    return None


def slack(x):
    return 1e-9 * max(abs(x), 1.0)


def rust_round(k):
    """f64::round: half away from zero."""
    return math.floor(k + 0.5) if k >= 0 else -math.floor(-k + 0.5)


def number_problem(rule, v):
    """The block parameter's rule (docs/adr/0318 §1): its least, its most, its step from the least."""
    lo, hi, step = rule.get("min"), rule.get("max"), rule.get("step")
    if lo is not None and v < lo - slack(lo):
        return f"en küçük değer {num(lo)}"
    if hi is not None and v > hi + slack(hi):
        return f"en büyük değer {num(hi)}"
    if step is not None:
        start = lo if lo is not None else 0.0
        k = (v - start) / step
        if abs(k - rust_round(k)) * step > slack(v):
            return f"{num(start)} değerinden {num(step)} adımlarla"
    return None


def default_width(p):
    return js_hypot(p["to"]["x"] - p["from"]["x"], p["to"]["y"] - p["from"]["y"])


def width_values(p, w):
    d = default_width(p)
    return {} if abs(d - w) <= slack(d) else {"Genişlik": w}


class Frame:
    def __init__(self, a, b, left):
        dx, dy = b["x"] - a["x"], b["y"] - a["y"]
        self.length = js_hypot(dx, dy)
        self.a = a
        self.u = (dx / self.length, dy / self.length) if self.length > SLACK else (0.0, 0.0)
        self.n = (-self.u[1], self.u[0]) if left else (self.u[1], -self.u[0])

    def at(self, s, t):
        return {"x": self.a["x"] + self.u[0] * s + self.n[0] * t, "y": self.a["y"] + self.u[1] * s + self.n[1] * t}

    def rotation(self):
        return math.atan2(self.u[1], self.u[0])


def count_of(length, module, sink):
    if sink is not None:
        return 0 if length + SLACK < sink else math.floor((length - sink) / module + SLACK) + 1
    return math.floor(length / module + SLACK)


def base_of(inp):
    b = {"layerId": inp["layerId"], "attrs": {}}
    if inp.get("color") is not None:
        b["color"] = inp["color"]
    if inp.get("lineWeight") is not None:
        b["lineWeight"] = inp["lineWeight"]
    return b


def insert(base, name, p, frame, mirror, params):
    e = {"kind": "insert", **base, "block": f"$blockOf:{name}", "p": p, "scale": 1.0, "rotation": frame.rotation()}
    if mirror:
        e["mirror"] = True
    if params:
        e["parameters"] = params
    return e


def band(frame, a, b, depth, left):
    pts = [frame.at(a, 0.0), frame.at(b, 0.0), frame.at(b, depth), frame.at(a, depth)]
    return pts if left else list(reversed(pts))


def run(doc_blocks, layers, inp):
    """The command's answer: (result, entities written in order, blocks brought) or (refusal, None, None)."""
    invalid = lambda words, path: (refusal("invalid_casework", words, path), None, None)  # noqa: E731
    for key, whose in (("from", "Dizinin başının"), ("to", "Dizinin sonunun")):
        for axis, name in (("x", "doğu (Y)"), ("y", "kuzey (X)")):
            if not math.isfinite(inp[key][axis]):
                return refusal("not_finite", f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", f"{key}.{axis}"), None, None
    for key, what in (("module", "Modül eni"), ("depth", "Tezgâh derinliği")):
        if inp.get(key) is not None and not math.isfinite(inp[key]):
            return refusal("not_finite", f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", key), None, None
    left = inp.get("side", "left") == "left"
    frame = Frame(inp["from"], inp["to"], left)
    if not frame.length > SLACK:
        return invalid("Dizinin başı ile sonu aynı nokta; dolapların sırtının dayandığı çizgiyi verin.", "to")
    module = inp.get("module", 0.6)
    depth = inp.get("depth", 0.6)
    if not (0.3 - 1e-9 <= depth <= 1.2 + 1e-9):
        return invalid(f"Tezgâh derinliği {num(depth)} m; 0,30 ile 1,20 m arasında olmalı.", "depth")
    added = []

    def definition(name):
        have = next((b for b in doc_blocks if b["name"] == name), None)
        if have is not None:
            return have
        b = library(name)
        if b not in added:
            added.append(b)
        return b

    cabinet = definition("Alt dolap")
    width = width_param(cabinet)
    if width is None:
        return invalid("“Alt dolap” bloğunun “Genişlik” parametresi yok; dizi modülün enini bu parametreyle verir.", "module")
    why = number_problem(width.get("rule", {}), module)
    if why:
        return invalid(f"Modül eni {num(module)} m; “Alt dolap” bloğunun eni için {why}.", "module")
    sink_w = SINK_WIDTH if inp.get("sink") is not None else None
    if sink_w is not None:
        why = number_problem(width.get("rule", {}), sink_w)
        if why:
            return invalid(f"Evye modülü {num(sink_w)} m; “Alt dolap” bloğunun eni için {why}.", "sink")
    count = count_of(frame.length, module, sink_w)
    if count == 0:
        return invalid(f"Dizinin çizgisi {fixed(frame.length, 3)} m; bir modüle ({fixed(sink_w if sink_w else module, 2)} m) yetmiyor. Çizgiyi uzatın ya da modülü daraltın.", "to")
    for key, whose in (("sink", "Evye"), ("cooktop", "Ocak")):
        k = inp.get(key)
        if k is not None and not 1 <= k <= count:
            return invalid(f"{whose} {k}. modülde; dizi {count} modül (1 ile {count} arası).", key)
    if inp.get("sink") is not None and inp.get("sink") == inp.get("cooktop"):
        return invalid("Evye ve ocak aynı modülde olamaz; ocağı başka bir modüle koyun.", "cooktop")
    sink_block = definition("Evye") if inp.get("sink") is not None else None
    cooktop_block = definition("Ocak") if inp.get("cooktop") is not None else None
    upper = None
    if inp.get("upper") is True:
        ub = definition("Üst dolap")
        up = width_param(ub)
        if up is None:
            return invalid("“Üst dolap” bloğunun “Genişlik” parametresi yok; dizi modülün enini bu parametreyle verir.", "upper")
        for w in [module] + ([sink_w] if sink_w else []):
            why = number_problem(up.get("rule", {}), w)
            if why:
                return invalid(f"Üst dolap {num(w)} m; “Üst dolap” bloğunun eni için {why}.", "upper")
        upper = up
    lw = inp.get("lineWeight")
    if lw is not None and not (0 <= lw <= 100):
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None, None
    rev = inp.get("expectedRevision")
    if rev is not None and not rev.startswith("$") and not (rev.isdigit() and (rev == "0" or not rev.startswith("0"))):
        return refusal("invalid_revision", f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), None, None
    if rev == "$stale":
        out = refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", "conflict")
        out["error"]["revision"] = "$current"
        return out, None, None
    warnings = []
    lid = inp["layerId"]
    node = layers.get(lid)
    if node is None:
        return refusal("layer_not_found", f"“{lid}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), None, None
    if node["type"] != "layer":
        return refusal("not_a_layer", f"“{node['name']}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"), None, None
    if node["locked"]:
        return refusal("layer_locked", f"“{node['name']}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"), None, None
    if not node["visible"]:
        warnings.append({"code": "layer_hidden", "message": f"“{node['name']}” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"})

    base = base_of(inp)
    sink_at, cooktop_at = inp.get("sink"), inp.get("cooktop")
    spans, at = [], 0.0
    for i in range(1, count + 1):
        w = sink_w if sink_at == i else module
        spans.append((at, w))
        at += w
    entities, modules = [], []
    for i, (start, w) in enumerate(spans, 1):
        entities.append(insert(base, "Alt dolap", frame.at(start, 0.0), frame, not left, width_values(width, w)))
        kind = "sink" if sink_at == i else "cooktop" if cooktop_at == i else "base"
        modules.append({"kind": kind, "start": start, "width": w})
    end = spans[-1][0] + spans[-1][1]
    filler = frame.length - end
    if sink_block is not None:
        start, _ = spans[sink_at - 1]
        entities.append(insert(base, "Evye", frame.at(start + SINK_INSET, SINK_INSET), frame, not left, {}))
    if cooktop_block is not None:
        start, w = spans[cooktop_at - 1]
        entities.append(insert(base, "Ocak", frame.at(start + w / 2.0, COOKTOP_INSET), frame, not left, {}))
    if upper is not None:
        for start, w in spans:
            entities.append(insert(base, "Üst dolap", frame.at(start, 0.0), frame, not left, width_values(upper, w)))
    if filler > 1e-3:
        entities.append({"kind": "polyline", **base, "pts": band(frame, end, frame.length, 0.6, left), "closed": True})
        warnings.append({"code": "casework_filler", "message": f"{count} modülden sonra dizinin sonunda {fixed(filler, 3)} m dolgu kalıyor.", "path": "to"})
    entities.append({"kind": "polygon", **base, "pts": band(frame, 0.0, frame.length, depth, left)})
    out = {"blocks": [b["name"] for b in added], "modules": modules, "filler": filler if filler > 1e-3 else 0.0}
    return {"status": "completed", "warnings": warnings, "out": out}, entities, added


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
        parts = path.split(".")
        for key in parts[:-1]:
            node = node[key]
        node[parts[-1]] = value
    return inp


def step(op, inp, d, non_finite=None, stale=False):
    """One step on the setup `d`: its answer and what the drawing holds after."""
    ask = with_non_finite(inp, non_finite or {})
    if stale:
        ask["expectedRevision"] = "$stale"
    result, entities, added = run(d.get("blocks", []), layers_of(d), ask)
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    ids = [e["id"] for e in d["entities"]]
    if entities is None:
        s["result"] = result
        s["expect"] = {"ids": ids, "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
        s["expect"] = {"ids": ids, "revision": "same"}
        return s
    out = result["out"]
    if op == "plan":
        # Nothing is written: a block the plan brings is named by the library's own id, not yet the drawing's.
        ids_of = {b["name"]: b["id"] for b in added}

        def own(e):
            name = e.get("block", "")[len("$blockOf:"):] if e.get("block", "").startswith("$blockOf:") else None
            return {**e, "id": 0, **({"block": ids_of[name]} if name in ids_of else {})}
        planned = [own(e) for e in entities]
        s["result"] = {"status": "completed", "output": {"entities": planned, **out, "revision": "$current"},
                       "warnings": result["warnings"]}
        s["within"] = 1e-9
        s["expect"] = {"ids": ids, "revision": "same"}
        return s
    first = max(ids) + 1
    new = list(range(first, first + len(entities)))
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(entities), "ids": new, **out,
                                                    "revision": "$current"}, "warnings": result["warnings"]}
    s["within"] = 1e-9
    s["expect"] = {"ids": ids + new, "entities": {str(i): {**e, "id": i} for i, e in zip(new, entities)},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    if added and not d.get("blocks"):
        s["expect"]["blocks"] = [{k: v for k, v in b.items() if k != "id"} for b in added]
    return s


def kitchen(**kw):
    inp = {"layerId": "mutfak", "from": P(0, 0), "to": P(3.2, 0), "sink": 2, "cooktop": 4}
    inp.update(kw)
    for k in [k for k, v in inp.items() if v is None]:
        del inp[k]
    return inp


def own_cabinet(width_rule=True):
    """The drawing's own “Alt dolap”: a plain 0.60 square, with or without its width parameter."""
    b = {"id": "0192f5a1-7050-7000-8000-000000000001", "name": "Alt dolap", "base": P(0, 0), "units": "m",
         "entities": [{"kind": "polyline", "id": 1, "layerId": "", "attrs": {},
                       "pts": [P(0, 0), P(0.6, 0), P(0.6, 0.6), P(0, 0.6)], "closed": True}]}
    if width_rule:
        b["dynamic"] = {"parameters": [{"kind": "linear", "name": "Genişlik", "from": P(0, 0), "to": P(0.6, 0),
                                        "rule": {"min": 0.3, "max": 1.5, "step": 0.3}}],
                        "actions": [{"kind": "stretch", "parameter": "Genişlik", "pieces": [1],
                                     "min": P(0.5, -0.1), "max": P(0.7, 0.7)}]}
    return b


def cases():
    out = []
    add = lambda name, steps, d=None: out.append({"name": name, **({"setup": d} if d else {}), "steps": steps})  # noqa: E731
    d = setup()
    base = kitchen()
    add("Evye 2., ocak 4. modülde: doğrula yazmaz", [step("validate", base, d)])
    add("Plan yazmadan dolapları, tezgâhı ve dolguyu gösterir", [step("plan", base, d)])
    add("Yazılır, kitaplığın blokları aynı adımda gelir; tek geri al", [
        step("execute", base, d),
        {"op": "undo", "returns": "Mutfak dizisi", "expect": {"ids": [1], "canUndo": False, "canRedo": True, "revision": "changed"}}])
    add("Gizli katmana uyarıyla yazılır", [step("execute", kitchen(layerId="gizli"), d)])
    for name, inp in [
        ("sağ yanda dizi: dolaplar aynalı, tezgâh öbür yanda", kitchen(side="right")),
        ("tam dolan çizgi: dolgu yok", kitchen(to=P(2.4, 0), sink=None, cooktop=None)),
        ("45 cm modüller ve 20 cm dolgu", kitchen(module=0.45, to=P(2.0, 0), sink=None, cooktop=2)),
        ("üst dolaplar modüllerin üstünde", kitchen(upper=True)),
        ("düşey duvar boyunca (90°)", kitchen(from_=None, to=P(0, 3.2), **{"from": P(0, 0)})),
        ("ters yönde çizilen dizi (180°)", kitchen(**{"from": P(3.2, 0)}, to=P(0, 0))),
        ("0,65 m derin tezgâh", kitchen(depth=0.65)),
        ("evyesiz, yalnız ocak", kitchen(sink=None, cooktop=3)),
        ("eğik (3-4-5) duvar boyunca", kitchen(to=P(2.4, 3.2))),
        ("renk ve çizgi kalınlığı nesnelere", kitchen(color="#E5484D", lineWeight=0.35)),
        ("çizgi tam evye modülü: tek modül", kitchen(to=P(0.9, 0), sink=1, cooktop=None)),
    ]:
        inp = {k: v for k, v in inp.items() if k != "from_"}
        add(name, [step("execute", inp, d)])
    own = setup([own_cabinet()])
    add("çizimin kendi Alt dolap bloğu kullanılır; kitaplıktan yalnız evye ve ocak gelir", [step("execute", kitchen(), own)], own)
    add("çizimin bloğunun kuralına uymayan modül reddedilir (0,30 adımlı)", [step("execute", kitchen(module=0.45), own)], own)
    add("çizimin kendi Alt dolap bloğu evyesiz kullanılır", [step("execute", kitchen(sink=None, cooktop=None, to=P(1.8, 0)), own)], own)
    bare = setup([own_cabinet(False)])
    add("Genişlik parametresi olmayan Alt dolap bloğu reddedilir", [step("execute", kitchen(), bare)], bare)
    for name, inp, kw in [
        ("başı ile sonu aynı çizgi reddedilir", kitchen(to=P(0, 0)), {}),
        ("kitaplığın enlerinden olmayan modül reddedilir", kitchen(module=0.5), {}),
        ("çok geniş modül reddedilir", kitchen(module=1.5), {}),
        ("ince tezgâh reddedilir", kitchen(depth=0.2), {}),
        ("bir modüle yetmeyen çizgi reddedilir", kitchen(to=P(0.5, 0), sink=None, cooktop=None), {}),
        ("evye modülüne yetmeyen çizgi reddedilir", kitchen(to=P(0.8, 0), sink=1, cooktop=None), {}),
        ("dizinin dışındaki evye reddedilir", kitchen(sink=6), {}),
        ("dizinin dışındaki ocak reddedilir", kitchen(cooktop=5), {}),
        ("evye ve ocak aynı modülde reddedilir", kitchen(cooktop=2), {}),
        ("sonsuz başlangıç noktası reddedilir", kitchen(), {"non_finite": {"from.x": "NaN"}}),
        ("sonsuz tezgâh derinliği reddedilir", kitchen(depth=0.6), {"non_finite": {"depth": "Infinity"}}),
        ("olmayan katman reddedilir", kitchen(layerId="yok"), {}),
        ("katman grubu reddedilir", kitchen(layerId="plan"), {}),
        ("kilitli katman reddedilir", kitchen(layerId="kilitli"), {}),
        ("geçersiz çizgi kalınlığı reddedilir", kitchen(lineWeight=120.0), {}),
        ("geçersiz beklenen sürüm reddedilir", kitchen(expectedRevision="07"), {}),
    ]:
        add(name, [step("execute", inp, d, **kw)])
    made = step("execute", base, d)
    stale = step("execute", {**base, "expectedRevision": "$prepared"}, d, stale=True)
    stale["expect"]["ids"] = made["expect"]["ids"]
    add("sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return out


def document():
    note = ("ADR 0706 §1. Beklenenler scripts/fixtures/casework_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
            "olmadan; kitaplığın blokları catalog/v1/blocks/kentos-temel.kblok'tan. Kurulumdaki çizimde 1 nesne var; yeni "
            "nesne 2'den başlar.")
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Mutfak dizisi: modüller, evye ve ocak, dolgu, tezgâh, üst dolaplar, kitaplığın blokları; retler",
            "note": note, "setup": setup(), "cases": cases()}


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
        print(f"{COMMAND}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
