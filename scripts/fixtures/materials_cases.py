#!/usr/bin/env python3
"""Building materials and their bindings (docs/adr/0785), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.materials.edit.json; `--check` compares and writes nothing. The rules, as the ADR gives
them:

- a material is an id (lower-case letters, digits and “-”, 1 to 64), a name (not blank, no edge spaces or control
  characters, at most 128 letters), a category (other when not given), a way of counting (volume when not given), a
  cut hatch and a cut pen; the ids and the names (case aside) are each once in the project;
- a component bound to a material (`materialId`) has the material's name as its `material`, its hatch as its `hatch` and
  its cut pen as its `edge`; a slab bound has the name as its `material`;
- a material's change rewrites every wall style's bound component, every wall's (a wall of a style whose components were
  the style's takes the style's new ones), every slab's; walls and slabs on locked layers keep their older values and
  are reported (`material_locked`);
- Define makes one (its id given, or from its name, then `-2`, `-3`…), Update changes one (by its id, else its name),
  Remove takes one away unless something is bound to it (`material_in_use`, with the bound walls' and slabs' ids),
  Assign binds wall style components, wall components and slabs (none: leaves them, their text and look kept), Adopt
  makes a material of every unbound text (styles, then walls, then slabs; a text another component of which has another
  look is left) and binds;
- every write is one undo step “Malzeme”.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.materials.edit.json"
STEP = "Malzeme"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def comp(name, material=None, mid=None, thickness=None, hatch=None, edge=None, detail="standard"):
    c = {"name": name}
    if material is not None:
        c["material"] = material
    if mid is not None:
        c["materialId"] = mid
    if thickness is not None:
        c["thickness"] = thickness
    if edge is not None:
        c["edge"] = edge
    if hatch is not None:
        c["hatch"] = hatch
    c["detail"] = detail
    return c


BRICK = {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0}
INS = {"pattern": "ANSI37", "scale": 0.5, "angle": 0.0}
CONCRETE = {"pattern": "AR-CONC", "scale": 1.0, "angle": 0.0}
STYLE_COMPS = [
    comp("İç sıva", "İç sıva", thickness=0.02, detail="detailed"),
    comp("Tuğla", "Tuğla", "tugla", hatch=BRICK, edge={"lineWeight": 0.5}),
    comp("Isı yalıtımı", "Isı yalıtımı", thickness=0.05, hatch=INS),
    comp("Dış sıva", "Dış sıva", thickness=0.03, detail="detailed"),
]
OWN_COMPS = [
    comp("İç sıva", "İç sıva", thickness=0.02, hatch={"pattern": "SOLID", "scale": 1.0, "angle": 0.0}, detail="detailed"),
    comp("Tuğla", "Tuğla", "tugla", hatch=BRICK, edge={"lineWeight": 0.5}),
    comp("Dış sıva", "Dış sıva", thickness=0.03, detail="detailed"),
]
MATERIALS = [
    {"id": "tugla", "name": "Tuğla", "category": "structure", "hatch": BRICK, "cut": {"lineWeight": 0.5}},
    {"id": "betonarme", "name": "Betonarme", "category": "structure", "hatch": CONCRETE},
]
UIDS = [f"0192f5a1-7850-7000-8000-{i:012x}" for i in range(1, 9)]


def wall(id, y, layer_id, style=True, comps=None, x0=0.0, x1=6.0, width=0.25):
    w = {"kind": "wall", "id": id, "layerId": layer_id, "attrs": {}, "pts": [P(x0, y), P(x1, y)], "width": width,
         "justify": "center"}
    if style:
        w["style"] = "dis-duvar"
    w["components"] = copy.deepcopy(comps if comps is not None else STYLE_COMPS)
    w["height"] = 3.0
    return w


def slab(id, layer_id, x0, material, mid=None):
    s = {"kind": "slab", "id": id, "layerId": layer_id, "attrs": {},
         "parts": [{"outer": {"pts": [P(x0, 0), P(x0 + 4, 0), P(x0 + 4, 3), P(x0, 3)]}}], "thickness": 0.15,
         "material": material}
    if mid is not None:
        s["materialId"] = mid
    return s


def setup():
    return {
        "format": "kentos.document", "version": 2, "name": "Malzemeler", "uids": UIDS,
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "wallStyles": [{"id": "dis-duvar", "name": "Dış duvar", "components": copy.deepcopy(STYLE_COMPS),
                                     "width": 0.25}],
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0}],
                              "materials": copy.deepcopy(MATERIALS)}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kilitli", "Kilitli", locked=True), layer("doseme", "Döşeme")],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "entities": [
            wall(1, 0, "duvar"),
            wall(2, 4, "duvar"),
            wall(3, 0, "duvar", style=False, comps=OWN_COMPS, x0=10, x1=14, width=0.2),
            wall(4, 8, "kilitli"),
            slab(5, "doseme", 20, "Betonarme", "betonarme"),
            slab(6, "doseme", 30, "Betonarme"),
            slab(7, "kilitli", 40, "Betonarme", "betonarme"),
            {"kind": "line", "id": 8, "layerId": "duvar", "attrs": {}, "a": P(0, 20), "b": P(5, 20)},
        ],
    }


# ── The model ───────────────────────────────────────────────────────────────────────────────────────────────────


class Refused(Exception):
    def __init__(self, code, message, path, uids=None):
        self.result = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
        if uids:
            self.result["error"]["uids"] = uids


TURKISH = {"ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ı": "i", "I": "i", "İ": "i", "i": "i", "ö": "o", "Ö": "o",
           "ş": "s", "Ş": "s", "ü": "u", "Ü": "u"}


def id_of(name):
    out = ""
    for ch in name:
        if ch in TURKISH:
            c = TURKISH[ch]
        elif ch.isascii() and ch.isalnum():
            c = ch.lower()
        else:
            c = "-"
        if c == "-" and (not out or out.endswith("-")):
            continue
        out += c
    out = out[:60].rstrip("-")
    return out or "malzeme"


def id_ok(i):
    return 1 <= len(i) <= 64 and all(c.islower() and c.isascii() or c.isdigit() or c == "-" for c in i)


def name_problem(name):
    if not name.strip() or name.strip() != name or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in name) or len(name) > 128:
        return f"“{name}” malzeme adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 128 harf."
    return None


def find(table, key, path):
    for i, m in enumerate(table):
        if m["id"] == key:
            return i
    for i, m in enumerate(table):
        if m["name"].lower() == key.lower():
            return i
    raise Refused("material_not_found",
                  f"Projede “{key}” kimlikli ya da adlı malzeme yok. Malzemeler penceresindeki bir malzemeyi verin.", path)


def free_id(table, name):
    base = id_of(name)
    i, n = base, 2
    while any(m["id"] == i for m in table):
        i, n = f"{base}-{n}", n + 1
    return i


def taken(table, id, name, but=None):
    for k, m in enumerate(table):
        if k == but:
            continue
        if m["id"] == id:
            return f"Projede “{id}” kimlikli bir malzeme (“{m['name']}”) zaten var."
        if m["name"].lower() == name.lower():
            return f"Projede “{m['name']}” adında bir malzeme zaten var; başka bir ad verin."
    return None


def changed(m, change):
    m = copy.deepcopy(m)
    for k in ("name", "category", "quantity"):
        if k in change:
            m[k] = change[k]
    for k in ("hatch", "cut"):
        if k in change:
            if change[k] is None:
                m.pop(k, None)
            else:
                m[k] = change[k]
    if m.get("category") == "other":
        m.pop("category")
    if m.get("quantity") == "volume":
        m.pop("quantity")
    return m


def bind(m, c):
    """The component bound to `m`: its material, id, hatch and edge the material's; the rest its own (the key order
    as the contract writes a component)."""
    out = {"name": c["name"], "material": m["name"], "materialId": m["id"]}
    if "thickness" in c:
        out["thickness"] = c["thickness"]
    if "cut" in m:
        out["edge"] = m["cut"]
    if "hatch" in m:
        out["hatch"] = m["hatch"]
    out["detail"] = c["detail"]
    for k in ("priority", "wrap"):
        if k in c:
            out[k] = c[k]
    return out


def leave(c):
    return {k: v for k, v in c.items() if k != "materialId"}


def rebound(comps, table):
    out = []
    for c in comps:
        m = next((m for m in table if m["id"] == c.get("materialId")), None)
        out.append(bind(m, c) if m else c)
    return out


def norm(c):
    """A component as a comparable value (key order aside)."""
    return json.dumps(c, sort_keys=True, ensure_ascii=False)


def same_comps(a, b):
    return [norm(c) for c in a] == [norm(c) for c in b]


def locked_layers(doc):
    return {l["id"] for l in doc["layers"] if l["locked"]}


def uid(doc, id):
    return doc["uids"][id - 1]


def follow(doc, styles, table, assigned):
    """Every style, wall and slab brought to `table`: the styles after, the updates (id → entity), the report."""
    report = {"styles": [], "walls": [], "slabs": [], "locked": [], "left": []}
    before = doc["settings"].get("wallStyles", [])
    after = [dict(s, components=rebound(s["components"], table)) for s in styles]
    for s in after:
        old = next((b for b in before if b["id"] == s["id"]), None)
        if old is None or norm(old) != norm(s):
            report["styles"].append(s["id"])
    updates = {}
    locked = locked_layers(doc)
    for orig in doc["entities"]:
        e = assigned.get(orig["id"], orig)
        if e["kind"] == "wall":
            nxt = copy.deepcopy(e)
            st = next((s for s in after if s["id"] == e.get("style")), None)
            if st is not None and any(b["id"] == st["id"] and same_comps(b["components"], e.get("components", []))
                                      for b in before):
                nxt["components"] = copy.deepcopy(st["components"])
            else:
                nxt["components"] = rebound(e.get("components", []), table)
        elif e["kind"] == "slab":
            nxt = copy.deepcopy(e)
            m = next((m for m in table if m["id"] == e.get("materialId")), None)
            if m is not None:
                nxt["material"] = m["name"]
        else:
            continue
        if norm(nxt) == norm(orig):
            continue
        if orig["layerId"] in locked:
            report["locked"].append(uid(doc, orig["id"]))
            continue
        report["walls" if nxt["kind"] == "wall" else "slabs"].append(uid(doc, orig["id"]))
        updates[orig["id"]] = nxt
    return after, updates, report


def target_entity(doc, assigned, u, path):
    if not (len(u) == 36 and all(c in "0123456789abcdef-" for c in u) and u.count("-") == 4):
        raise Refused("invalid_uid", f"“{u}” bir kalıcı kimlik (UUID) değil; duvarın ya da döşemenin kimliğini verin.", path)
    if u not in doc["uids"]:
        raise Refused("entity_not_found", f"Çizimde “{u}” kimlikli nesne yok.", path)
    id = doc["uids"].index(u) + 1
    e = next(e for e in doc["entities"] if e["id"] == id)
    assigned.setdefault(id, copy.deepcopy(e))
    return assigned[id]


def check_locked(doc, e, path):
    if e["layerId"] in locked_layers(doc):
        name = next(l["name"] for l in doc["layers"] if l["id"] == e["layerId"])
        raise Refused("layer_locked", f"“{name}” katmanı kilitli; üzerindeki nesne malzemeye bağlanamaz. Kilidi Katmanlar panelinden açın.", path)


def component_at(n, k, whose, path):
    if k >= n:
        raise Refused("component_not_found", f"{whose} {n} bileşeni var; {k}. sıra yok (sıra soldan 0'dan sayılır).", path)
    return k


def run(doc, inp):
    """The command on `doc`: (result, the document after) — the document unchanged when refused."""
    table = copy.deepcopy(doc["settings"]["arch"].get("materials", []))
    styles = copy.deepcopy(doc["settings"].get("wallStyles", []))
    op = inp["operation"]
    assigned = {}
    material = None
    left = []
    kind = op["kind"]
    if kind == "define":
        name = op["name"]
        if name_problem(name):
            raise Refused("invalid_material", name_problem(name), "operation.name")
        change = op.get("change", {})
        if "name" in change:
            raise Refused("invalid_material", "Yeni malzemenin adı “name” alanındadır; change.name verilmez.", "operation.change.name")
        if "id" in op and not id_ok(op["id"]):
            raise Refused("invalid_material",
                          f"“{op['id']}” malzeme kimliği olamaz: küçük harf, rakam ve “-” ile en çok 64 harf.", "operation.id")
        mid = op.get("id") or free_id(table, name)
        why = taken(table, mid, name)
        if why:
            raise Refused("material_exists", why, "operation.name")
        table.append(changed({"id": mid, "name": name}, change))
        material = mid
    elif kind == "update":
        at = find(table, op["material"], "operation.material")
        change = op["change"]
        if not change:
            raise Refused("nothing_to_set", "Değiştirilecek bir alan verilmedi: ad, kategori, metraj, tarama ya da kesit kalemi verin.",
                          "operation.change")
        if "name" in change:
            if name_problem(change["name"]):
                raise Refused("invalid_material", name_problem(change["name"]), "operation.change.name")
            why = taken(table, "", change["name"], at)
            if why:
                raise Refused("material_exists", why, "operation.change.name")
        table[at] = changed(table[at], change)
        material = table[at]["id"]
    elif kind == "remove":
        at = find(table, op["material"], "operation.material")
        mid = table[at]["id"]
        in_styles = [s["name"] for s in styles if any(c.get("materialId") == mid for c in s["components"])]
        uids = [uid(doc, e["id"]) for e in doc["entities"]
                if (e["kind"] == "wall" and any(c.get("materialId") == mid for c in e.get("components", [])))
                or (e["kind"] == "slab" and e.get("materialId") == mid)]
        if in_styles or uids:
            said = []
            if in_styles:
                said.append(f"{len(in_styles)} duvar stili ({', '.join(in_styles)})")
            if uids:
                said.append(f"{len(uids)} duvar ya da döşeme")
            raise Refused("material_in_use",
                          f"“{table[at]['name']}” malzemesine bağlı {' ve '.join(said)} var; silinmedi. Önce onları başka malzemeye bağlayın ya da bağdan ayırın.",
                          "operation.material", uids)
        del table[at]
        material = mid
    elif kind == "assign":
        targets = op["targets"]
        if not targets:
            raise Refused("nothing_to_set", "Bağlanacak bir şey verilmedi: duvar stili bileşeni, duvar bileşeni ya da döşeme verin.",
                          "operation.targets")
        m = table[find(table, op["material"], "operation.material")] if op.get("material") is not None else None
        for i, t in enumerate(targets):
            path = f"operation.targets[{i}]"
            if t["kind"] == "wallStyle":
                s = next((s for s in styles if s["id"] == t["style"]), None) or next(
                    (s for s in styles if s["name"].lower() == t["style"].lower()), None)
                if s is None:
                    raise Refused("style_not_found", f"Projede “{t['style']}” kimlikli ya da adlı duvar stili yok.", f"{path}.style")
                k = component_at(len(s["components"]), t["component"], f"“{s['name']}” stilinin", f"{path}.component")
                s["components"][k] = bind(m, s["components"][k]) if m else leave(s["components"][k])
            elif t["kind"] == "wall":
                e = target_entity(doc, assigned, t["uid"], f"{path}.uid")
                if e["kind"] != "wall":
                    raise Refused("not_a_wall", "Nesne duvar değil; bileşen yalnız duvarda olur.", f"{path}.uid")
                check_locked(doc, e, f"{path}.uid")
                k = component_at(len(e["components"]), t["component"], "Duvarın", f"{path}.component")
                e["components"][k] = bind(m, e["components"][k]) if m else leave(e["components"][k])
            else:
                e = target_entity(doc, assigned, t["uid"], f"{path}.uid")
                if e["kind"] != "slab":
                    raise Refused("not_a_slab", "Nesne döşeme değil.", f"{path}.uid")
                check_locked(doc, e, f"{path}.uid")
                if m:
                    e["material"] = m["name"]
                    e["materialId"] = m["id"]
                else:
                    e.pop("materialId", None)
        material = m["id"] if m else None
    elif kind == "adopt":
        def take(text, c):
            m = next((m for m in table if m["name"].lower() == text.lower()), None)
            if m is not None:
                if c is None or (c.get("hatch") == m.get("hatch") and c.get("edge") == m.get("cut")):
                    return m
                if text not in left:
                    left.append(text)
                return None
            if len(table) >= 256 or name_problem(text):
                return None
            m = {"id": free_id(table, text), "name": text}
            if c is not None and "hatch" in c:
                m["hatch"] = c["hatch"]
            if c is not None and "edge" in c:
                m["cut"] = c["edge"]
            table.append(m)
            return m
        for s in styles:
            for k, c in enumerate(s["components"]):
                text = c.get("material")
                if "materialId" in c or text is None or not text.strip():
                    continue
                m = take(text, c)
                if m:
                    s["components"][k] = bind(m, c)
        locked = locked_layers(doc)
        for e in doc["entities"]:
            if e["kind"] == "wall" and e["layerId"] not in locked:
                nxt = copy.deepcopy(e)
                for k, c in enumerate(nxt["components"]):
                    text = c.get("material")
                    if "materialId" in c or text is None or not text.strip():
                        continue
                    m = take(text, c)
                    if m:
                        nxt["components"][k] = bind(m, c)
                if norm(nxt) != norm(e):
                    assigned[e["id"]] = nxt
            elif e["kind"] == "slab" and "materialId" not in e and e["layerId"] not in locked and "material" in e:
                m = take(e["material"], None)
                if m:
                    assigned[e["id"]] = dict(copy.deepcopy(e), materialId=m["id"])
    if "expectedRevision" in inp and inp["expectedRevision"].isdigit():
        # The drawing as the cases open it is never at revision 999.
        r = Refused("revision_conflict",
                    "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                    "expectedRevision")
        r.result["status"] = "conflict"
        r.result["error"]["revision"] = "$current"
        raise r
    if "expectedRevision" in inp:
        raise Refused("invalid_revision",
                      f"Beklenen sürüm “{inp['expectedRevision']}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                      "expectedRevision")
    after, updates, report = follow(doc, styles, table, assigned)
    report["left"] = left
    if kind in ("assign", "adopt") and not updates and not report["styles"] and table == doc["settings"]["arch"].get("materials", []):
        raise Refused("nothing_to_set", "Bağlanacak bir şey değişmiyor: hedefler zaten bu malzemede ya da bağlanacak metin malzeme yok.",
                      "operation")
    warnings = []
    if report["locked"]:
        warnings.append({"code": "material_locked",
                         "message": f"Malzemeye bağlı {len(report['locked'])} nesne kilitli katmanda; eski değerleriyle kaldı. Kilidi açıp yeniden kaydedin.",
                         "path": "report.locked"})
    if left:
        warnings.append({"code": "material_adopt_left",
                         "message": f"{', '.join('“' + t + '”' for t in left)} malzemesinin bazı bileşenlerinin taraması ya da kalemi farklı; görünüşü değişmesin diye bağlanmadı.",
                         "path": "report.left"})
    out = {"materials": table}
    if material is not None:
        out["material"] = material
    out["report"] = report
    out["revision"] = "$current"
    doc2 = copy.deepcopy(doc)
    doc2["settings"]["arch"]["materials"] = table
    if not table:
        doc2["settings"]["arch"].pop("materials")
    doc2["settings"]["wallStyles"] = after
    for e in doc2["entities"]:
        if e["id"] in updates:
            nxt = updates[e["id"]]
            e.clear()
            e.update(nxt)
    written = bool(updates) or after != doc["settings"].get("wallStyles") or table != doc["settings"]["arch"].get("materials", [])
    return {"status": "completed", "output": out, "warnings": warnings}, doc2, written


# ── Cases ───────────────────────────────────────────────────────────────────────────────────────────────────────


def expect_of(doc, written):
    e = {"revision": "changed" if written else "same",
         "settings": {"arch": doc["settings"]["arch"], "wallStyles": doc["settings"]["wallStyles"]},
         "fields": {str(x["id"]): {k: x.get(k) for k in (("components",) if x["kind"] == "wall" else ("material", "materialId"))}
                    for x in doc["entities"] if x["kind"] in ("wall", "slab")}}
    return e


def steps_for(inp, undo=False, plan=True):
    doc = setup()
    out = []
    try:
        result, after, written = run(doc, inp)
    except Refused as r:
        for op in (("validate", "plan", "execute") if plan else ("execute",)):
            res = copy.deepcopy(r.result)
            out.append({"op": op, "input": inp, "result": res, "expect": {"revision": "same"}})
        return out
    if plan:
        out.append({"op": "plan", "input": inp, "result": result, "expect": {"revision": "same"}})
    out.append({"op": "execute", "input": inp, "result": result, "expect": expect_of(after, written)})
    if undo:
        out.append({"op": "undo", "returns": STEP, "expect": expect_of(doc, True)})
        out.append({"op": "redo", "returns": STEP, "expect": expect_of(after, True)})
    return out


def cases():
    c = []

    def case(name, inp, undo=False, plan=True):
        c.append({"name": name, "steps": steps_for(inp, undo, plan)})

    def op(kind, **kw):
        return {"operation": {"kind": kind, **kw}}

    case("Yeni malzeme: adından kimlik, varsayılan kategori ve hacim", op("define", name="Gaz beton"))
    case("Yeni malzeme: kimlik, kategori, alanla metraj, tarama ve kalem",
         op("define", name="Isı yalıtımı", id="eps", change={"category": "insulation", "quantity": "area",
                                                               "hatch": INS, "cut": {"color": "#C08040", "lineWeight": 0.18}}),
         undo=True)
    case("Yeni malzeme: kimliği alınmış addan bir sonraki", op("define", name="Tugla!"))
    case("Yeni malzeme: aynı ad büyük küçük harf ayrımsız reddedilir", op("define", name="tuğla"))
    case("Yeni malzeme: var olan kimlik reddedilir", op("define", name="Kerpiç", id="tugla"))
    case("Yeni malzeme: boş ad", op("define", name="  "))
    case("Yeni malzeme: büyük harfli kimlik", op("define", name="Ahşap", id="Ahsap"))
    case("Yeni malzeme: change.name verilmez", op("define", name="Ahşap", change={"name": "Ahşap 2"}))
    case("Taramayı değiştir: stil, stilin duvarları ve kendi duvarı izler, kilitli kalır",
         op("update", material="tugla", change={"hatch": {"pattern": "ANSI32", "scale": 0.8, "angle": 45.0}}), undo=True)
    case("Yeniden adlandır: bileşenlerin ve döşemenin malzeme metni izler",
         op("update", material="Betonarme", change={"name": "Betonarme C30"}), undo=True)
    case("Kesit kalemini kaldır: bağlı bileşenlerin kenarı duvarınkine döner", op("update", material="tugla", change={"cut": None}))
    case("Kategoriyi değiştir: hiçbir nesne değişmez", op("update", material="TUĞLA", change={"category": "other"}))
    case("Değiştir: değişecek alan yok", op("update", material="tugla", change={}))
    case("Değiştir: olmayan malzeme", op("update", material="kerpic", change={"category": "finish"}))
    case("Değiştir: başka malzemenin adı", op("update", material="tugla", change={"name": "betonarme"}))
    case("Sil: bağlı stil, duvar ve döşemeler varken", op("remove", material="tugla"))
    case("Sil: bağlı döşemeler varken", op("remove", material="betonarme"))
    case("Sil: kullanılmayan malzeme", {"operation": {"kind": "define", "name": "Geçici"}}, plan=False)
    c[-1]["steps"] += [{"op": "execute", "input": op("remove", material="Geçici"),
                        "result": {"status": "completed", "output": {"materials": MATERIALS, "material": "gecici",
                                                                      "report": {"styles": [], "walls": [], "slabs": [], "locked": [], "left": []},
                                                                      "revision": "$current"}, "warnings": []},
                        "expect": {"revision": "changed", "settings": {"arch": setup()["settings"]["arch"]}}}]
    case("Bağla: stilin yalıtımı, stilin iki duvarı izler",
         op("assign", material="tugla", targets=[{"kind": "wallStyle", "style": "Dış duvar", "component": 2}]), undo=True)
    case("Bağla: kendi duvarının iç sıvası ve döşeme betonarmeye",
         op("assign", material="betonarme", targets=[{"kind": "wall", "uid": UIDS[2], "component": 0},
                                                     {"kind": "slab", "uid": UIDS[5]}]))
    case("Bağdan ayır: stilin tuğlası, görünüşü kalır",
         op("assign", targets=[{"kind": "wallStyle", "style": "dis-duvar", "component": 1}]))
    case("Bağla: zaten bağlı olan değişmez", op("assign", material="tugla", targets=[{"kind": "wall", "uid": UIDS[0], "component": 1}]))
    case("Bağla: hedef yok", op("assign", material="tugla", targets=[]))
    case("Bağla: olmayan bileşen sırası", op("assign", material="tugla", targets=[{"kind": "wallStyle", "style": "Dış duvar", "component": 4}]))
    case("Bağla: olmayan duvar stili", op("assign", material="tugla", targets=[{"kind": "wallStyle", "style": "İç duvar", "component": 0}]))
    case("Bağla: döşeme hedefi duvar", op("assign", material="tugla", targets=[{"kind": "slab", "uid": UIDS[0]}]))
    case("Bağla: duvar hedefi çizgi", op("assign", material="tugla", targets=[{"kind": "wall", "uid": UIDS[7], "component": 0}]))
    case("Bağla: kalıcı kimlik değil", op("assign", material="tugla", targets=[{"kind": "slab", "uid": "döşeme-5"}]))
    case("Bağla: çizimde olmayan kimlik", op("assign", material="tugla",
                                           targets=[{"kind": "slab", "uid": "0192f5a1-7850-7000-8000-0000000000ff"}]))
    case("Bağla: kilitli katmandaki döşeme", op("assign", material="tugla", targets=[{"kind": "slab", "uid": UIDS[6]}]))
    case("Bağla: olmayan malzeme", op("assign", material="kerpic", targets=[{"kind": "slab", "uid": UIDS[5]}]))
    case("Metinlerden al: her metinden bir malzeme, görünüşü farklı olan bağlanmaz", op("adopt"), undo=True)
    case("Beklenen sürüm sayı değil", {"operation": {"kind": "define", "name": "Ahşap"}, "expectedRevision": "x"})
    case("Beklenen sürüm eski: çizim değişmiş", {"operation": {"kind": "update", "material": "tugla",
                                                              "change": {"category": "finish"}},
                                                "expectedRevision": "999"})
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data = {
        "format": "kentos.command-cases", "version": 1, "command": "cad.materials.edit", "commandVersion": 1,
        "title": "Malzemeler",
        "note": "ADR 0785. Beklenenler scripts/fixtures/materials_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
                "olmadan. Kurulumda bir stil (çekirdeği tuğlaya bağlı), stilin iki duvarı, kendi bileşenli bir duvar, kilitli "
                "katmanda stilin bir duvarı, betonarmeye bağlı iki döşeme (biri kilitli), metin malzemeli bir döşeme ve bir "
                "çizgi var.",
        "setup": setup(),
        "cases": cases(),
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
