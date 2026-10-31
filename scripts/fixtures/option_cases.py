#!/usr/bin/env python3
"""Design options (docs/adr/0788, ADV-05c): the shared cases of `cad.options.edit`, `cad.options.query` and a view
recipe's options (`cad.views.edit` steps in the edit cases), written from the ADR's rules without KentOS code.

The drawing: a 6 × 4 m flat on the ground floor (four walls), a kitchen's part of it (a partition wall at x = 3, its
door, a drawn space “Mutfak”, the space's tag, a counter), a door in the outer wall, a drawn space “Salon” and its tag,
a text, a counter on a locked layer and a line. The cases define the kitchen as a set's first option, copy it as an
independent alternative, move objects in and out, make another option primary, accept and delete options, rename, and
read the sets back with their counts, areas and lengths, and compare two options.

The rules (docs/adr/0788 §1–§2):

- A set's id comes from its name (Turkish letters folded, other characters a hyphen, as a door type's id), numbered
  on when taken; its first option is “a”, named “A” unless given; a copy takes the first letter a–z no option of the
  set has, named in capitals unless given.
- An object is in one option at most. A main-model object leaning on an option's object (an opening's wall, a tag's
  object, a dimension's points), or an option's object leaning on another option's, refuses the write: the first such
  object in the drawing's order, its first such reference in text order.
- A copy is the option's objects in the drawing's order, each a new object, its texts naming another of them made the
  copies'; on a locked layer it is refused. Accepting keeps one option as the main model and deletes the others'
  objects (the set goes); deleting removes a non-primary option with its objects.
- An option's objects gone from the drawing fall out of it at the next write; a read counts them as missing.
- A read counts an option's objects by kind (in the kinds' order), sums its spaces' areas (shoelace) and its walls'
  lengths. Two options compare as Veri karşılaştır pairs by place: objects of one kind whose points lie within the search
  distance pair from the nearest up; a pair is the same when its points are within the tolerance and its other values
  equal; the unpaired are removed (only in a) or added (only in b).

    python3 scripts/fixtures/option_cases.py          # writes the cases
    python3 scripts/fixtures/option_cases.py --check  # compares with the files
"""

import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.options.edit.json"
QUERY = ROOT / "fixtures/commands/v1/cad.options.query.json"

BLOCK = "0192f5a1-7880-7000-8000-0000000000b1"
STEP = "Tasarım seçeneği"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
THICK = 0.1


def UID(n):
    return f"0192f5a1-7880-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


WALLS = {1: ((0, 0), (6, 0)), 2: ((6, 0), (6, 4)), 3: ((6, 4), (0, 4)), 4: ((0, 4), (0, 0)), 5: ((3, 0), (3, 4))}


def wall(i):
    a, b = WALLS[i]
    return {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": THICK,
            "justify": "center", "arch": {"level": "zemin"}}


def jambs(a, b, at, width):
    """An opening's jambs on the straight wall a → b: its start's left and right face points, then its end's."""
    ux, uy = b[0] - a[0], b[1] - a[1]
    n = math.hypot(ux, uy)
    ux, uy = ux / n, uy / n
    lx, ly = -uy, ux
    h = THICK / 2
    out = []
    for s in (at - width / 2, at + width / 2):
        cx, cy = a[0] + ux * s, a[1] + uy * s
        out += [P(cx + lx * h, cy + ly * h), P(cx - lx * h, cy - ly * h)]
    return out


def door(i, host, at, width, number):
    a, b = WALLS[host]
    return {"kind": "opening", "id": i, "layerId": "kapi", "attrs": {}, "host": UID(host), "at": at,
            "width": width, "form": "door", "frame": 0.05, "number": number, "place": jambs(a, b, at, width)}


def rect(x0, y0, x1, y1):
    return [{"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}]


def setup_entities():
    return [
        wall(1), wall(2), wall(3), wall(4), wall(5),
        door(6, 5, 2.0, 0.8, "K2"),
        {"kind": "space", "id": 7, "layerId": "mimari", "attrs": {}, "parts": rect(3.05, 0.05, 5.95, 3.95),
         "number": "M1", "name": "Mutfak", "arch": {"level": "zemin"}},
        {"kind": "tag", "id": 8, "layerId": "mimari", "attrs": {}, "of": UID(7), "at": P(4.5, 2.0), "content": "{number}",
         "text": "M1"},
        {"kind": "insert", "id": 9, "layerId": "mimari", "attrs": {}, "block": BLOCK, "p": P(4.0, 3.5),
         "scale": 1.0, "rotation": 0.0, "arch": {"level": "zemin"}},
        door(10, 1, 1.5, 0.9, "K1"),
        {"kind": "space", "id": 11, "layerId": "mimari", "attrs": {}, "parts": rect(0.05, 0.05, 2.95, 3.95),
         "number": "S1", "name": "Salon", "arch": {"level": "zemin"}},
        {"kind": "tag", "id": 12, "layerId": "mimari", "attrs": {}, "of": UID(11), "at": P(1.5, 2.0),
         "content": "{number}", "text": "S1"},
        {"kind": "text", "id": 13, "layerId": "mimari", "attrs": {}, "p": P(1.0, 1.0), "text": "Not", "height": 0.25, "rotation": 0.0},
        {"kind": "insert", "id": 14, "layerId": "kilit", "attrs": {}, "block": BLOCK, "p": P(1.0, 3.5),
         "scale": 1.0, "rotation": 0.0, "arch": {"level": "zemin"}},
        {"kind": "line", "id": 15, "layerId": "duvar", "attrs": {}, "a": P(0, -1), "b": P(6, -1)},
    ]


def setup():
    ents = setup_entities()
    return {
        "format": "kentos.document", "version": 2, "name": "Tasarım seçenekleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mimari", "Mimari"),
                   layer("kilit", "Kilit", locked=True)],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Tezgah", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0.6, 0)}]}],
        "entities": ents,
    }


IDS = list(range(1, 16))
KITCHEN = [5, 6, 7, 8, 9]
LAYER_NAMES = {"duvar": "Duvar", "kapi": "Kapı", "mimari": "Mimari", "kilit": "Kilit"}
LOCKED = {"kilit"}
KIND_WORDS = {"wall": "duvar", "opening": "açıklık", "space": "mahal", "insert": "donatı", "dimension": "ölçü",
              "tag": "etiket", "text": "yazı"}
NEVER_LEAN = {"line", "point", "circle", "arc", "polygon", "wall", "ellipse", "spline", "nurbs"}


def fold_id(name):
    """A door type's id from a name (contracts' `opening_type_id_of`), “secenek” for an empty one."""
    table = {"ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ı": "i", "I": "i", "İ": "i", "i": "i", "ö": "o", "Ö": "o",
             "ş": "s", "Ş": "s", "ü": "u", "Ü": "u"}
    out = ""
    for c in name:
        if c in table:
            c = table[c]
        elif c.isascii() and c.isalnum():
            c = c.lower()
        else:
            c = "-"
        if c == "-" and (not out or out.endswith("-")):
            continue
        out += c
    out = out.rstrip("-") or "tip"
    if out.startswith("kentos-"):
        out = "tip-" + out
    out = out[:60]
    return "secenek" if out == "tip" else out


class Refused(Exception):
    def __init__(self, code, message, path, status="failed"):
        self.error = {"code": code, "message": message, "path": path}
        self.status = status


class Doc:
    """The drawing as the cases change it: objects by slot in the drawing's order, their ids, the option sets."""

    def __init__(self):
        self.ents = {e["id"]: copy.deepcopy(e) for e in setup_entities()}
        self.uid = {i: UID(i) for i in self.ents}
        self.next = 16
        self.sets = []

    def slot_of(self, uid):
        for s, u in self.uid.items():
            if u == uid and s in self.ents:
                return s
        return None

    def present(self, uid):
        return self.slot_of(uid) is not None

    @staticmethod
    def out(uid):
        """A copy's id as the cases name it."""
        if uid.startswith("0192f5a1-7880-7000-8000-0000000"):
            n = int(uid[-12:], 16)
            if n >= 0x1000:
                return f"$uidOf:{n - 0x1000}"
        return uid

    def references(self, e):
        out = set()

        def walk(v):
            if isinstance(v, str) and len(v) == 36 and self.present(v):
                out.add(v)
            elif isinstance(v, list):
                for x in v:
                    walk(x)
            elif isinstance(v, dict):
                for x in v.values():
                    walk(x)
        walk(e)
        return sorted(out)

    def pruned(self):
        for s in self.sets:
            for o in s["options"]:
                o["members"] = [m for m in o.get("members", []) if self.present(m)]

    def group(self):
        g = {}
        for i, s in enumerate(self.sets):
            for j, o in enumerate(s["options"]):
                for m in o.get("members", []):
                    g[m] = (i, j)
        return g

    def leaning(self):
        g = self.group()
        if not g:
            return
        for slot in sorted(self.ents):
            e = self.ents[slot]
            if e["kind"] in NEVER_LEAN:
                continue
            me = self.uid[slot]
            mine = g.get(me)
            for r in self.references(e):
                theirs = g.get(r)
                if theirs is not None and theirs != mine:
                    other = KIND_WORDS.get(self.ents[self.slot_of(r)]["kind"], "nesne")
                    where = "Ana modeldeki" if mine is None else \
                        f"“{self.sets[mine[0]]['name']}” kümesinin “{self.sets[mine[0]]['options'][mine[1]]['name']}” seçeneğindeki"
                    s, o = theirs
                    raise Refused("option_leaning",
                                  f"{where} {KIND_WORDS.get(e['kind'], 'nesne')} ({me}) “{self.sets[s]['name']}” kümesinin "
                                  f"“{self.sets[s]['options'][o]['name']}” seçeneğindeki bir {other} nesnesine bağlı ({r}); "
                                  "bağlı nesneyi de aynı seçeneğe alın ya da bağı kaldırın.", "operation.uids")


def name_ok(name, path):
    if not name.strip() or name.strip() != name or len(name) > 120:
        raise Refused("invalid_input", f"“{name}” seçenek adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 harf.", path)


def set_at(sets, key):
    for i, s in enumerate(sets):
        if s["id"] == key:
            return i
    for i, s in enumerate(sets):
        if s["name"].lower() == key.strip().lower():
            return i
    raise Refused("option_set_not_found", f"Projede “{key}” adlı ya da kimlikli tasarım seçeneği kümesi yok.", "operation.set")


def option_at(s, key, path="operation.option"):
    for j, o in enumerate(s["options"]):
        if o["id"] == key:
            return j
    for j, o in enumerate(s["options"]):
        if o["name"].lower() == key.strip().lower():
            return j
    raise Refused("option_not_found", f"“{s['name']}” kümesinde “{key}” adlı ya da kimlikli seçenek yok.", path)


def uids_ok(doc, uids, nothing):
    if not uids:
        raise Refused("no_entities", f"{nothing} En az bir nesnenin kalıcı kimliğini verin.", "operation.uids")
    for i, u in enumerate(uids):
        ok = len(u) == 36 and all(c == "-" if k in (8, 13, 18, 23) else c in "0123456789abcdef" for k, c in enumerate(u))
        if not ok:
            raise Refused("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.",
                          f"operation.uids[{i}]")
    seen, out = set(), []
    for i, u in enumerate(uids):
        if u in seen:
            continue
        seen.add(u)
        if not doc.present(u):
            raise Refused("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                          f"operation.uids[{i}]")
        out.append((i, u))
    return out


def unlocked(doc, slots):
    for s in slots:
        lay = doc.ents[s]["layerId"]
        if lay in LOCKED:
            raise Refused("layer_locked", f"“{LAYER_NAMES[lay]}” katmanı kilitli; seçeneğin nesneleri yazılamaz. Katmanın kilidini açın.", "operation")


def clean(s):
    out = {"id": s["id"], "name": s["name"], "primary": s["primary"], "options": []}
    for o in s["options"]:
        x = {"id": o["id"], "name": o["name"]}
        if o.get("members"):
            x["members"] = [Doc.out(m) for m in o["members"]]
        out["options"].append(x)
    return out


def edit(doc, op):
    """Runs one `cad.options.edit` on `doc` (changing it): the output, or Refused."""
    doc.pruned()
    sets = copy.deepcopy(doc.sets)
    adds, removes, created, deleted, released = [], [], [], [], []
    set_id, option = None, None
    k = op["kind"]
    if k == "define":
        name_ok(op["name"], "operation.name")
        if any(s["name"].lower() == op["name"].lower() for s in sets):
            raise Refused("option_set_exists", f"Projede “{op['name']}” adlı bir seçenek kümesi var.", "operation.name")
        if "id" in op:
            sid = op["id"]
            if any(s["id"] == sid for s in sets):
                raise Refused("option_set_exists", f"Projede “{sid}” kimlikli bir seçenek kümesi var.", "operation.id")
        else:
            stem = fold_id(op["name"])
            sid, n = stem, 2
            while any(s["id"] == sid for s in sets):
                sid, n = f"{stem}-{n}", n + 1
        first = op.get("option", "A")
        name_ok(first, "operation.option")
        found = uids_ok(doc, op["uids"], "Seçeneğe alınacak nesne verilmedi.")
        taken = {m for s in sets for o in s["options"] for m in o.get("members", [])}
        for i, u in found:
            if u in taken:
                raise Refused("option_member_taken", "Bu nesne zaten bir tasarım seçeneğinde; önce o seçenekten çıkarın.", f"operation.uids[{i}]")
        sets.append({"id": sid, "name": op["name"], "primary": "a", "options": [{"id": "a", "name": first, "members": [u for _, u in found]}]})
        set_id, option = sid, "a"
    elif k == "copy":
        si = set_at(sets, op["set"])
        oi = option_at(sets[si], op["option"])
        if len(sets[si]["options"]) >= 12:
            raise Refused("option_set_full", f"“{sets[si]['name']}” kümesinde en çok 12 seçenek olur; birini silin ya da kabul edin.", "operation.set")
        used = {o["id"] for o in sets[si]["options"]}
        letter = next(chr(c) for c in range(ord("a"), ord("z") + 1) if chr(c) not in used)
        new_name = op.get("name", letter.upper())
        name_ok(new_name, "operation.name")
        if any(o["name"].lower() == new_name.lower() for o in sets[si]["options"]):
            raise Refused("option_exists", f"“{sets[si]['name']}” kümesinde “{new_name}” adlı bir seçenek var.", "operation.name")
        source = set(sets[si]["options"][oi].get("members", []))
        originals = [s for s in sorted(doc.ents) if doc.uid[s] in source]
        unlocked(doc, originals)
        new = {}
        for s in originals:
            slot = doc.next + len(new)
            new[doc.uid[s]] = UID(0x1000 + slot)

        def remap(v):
            if isinstance(v, str):
                return new.get(v, v)
            if isinstance(v, list):
                return [remap(x) for x in v]
            if isinstance(v, dict):
                return {kk: remap(x) for kk, x in v.items()}
            return v
        for s in originals:
            e = remap(copy.deepcopy(doc.ents[s]))
            adds.append((new[doc.uid[s]], e))
        members = [u for u, _ in adds]
        option = {"id": letter, "name": new_name}
        if members:
            option["members"] = members
        sets[si]["options"].append(option)
        if op.get("primary"):
            sets[si]["primary"] = letter
        set_id, option = sets[si]["id"], letter
    elif k == "addMembers":
        si = set_at(sets, op["set"])
        oi = option_at(sets[si], op["option"])
        found = uids_ok(doc, op["uids"], "Seçeneğe alınacak nesne verilmedi.")
        taken = {m for s in sets for o in s["options"] for m in o.get("members", [])}
        for i, u in found:
            if u in taken:
                raise Refused("option_member_taken", "Bu nesne zaten bir tasarım seçeneğinde; önce o seçenekten çıkarın.", f"operation.uids[{i}]")
        sets[si]["options"][oi].setdefault("members", []).extend(u for _, u in found)
        set_id, option = sets[si]["id"], sets[si]["options"][oi]["id"]
    elif k == "removeMembers":
        si = set_at(sets, op["set"])
        found = uids_ok(doc, op["uids"], "Seçenekten çıkarılacak nesne verilmedi.")
        for i, u in found:
            at = next((j for j, o in enumerate(sets[si]["options"]) if u in o.get("members", [])), None)
            if at is None:
                raise Refused("not_in_option", f"Nesne “{sets[si]['name']}” kümesinin bir seçeneğinde değil.", f"operation.uids[{i}]")
            sets[si]["options"][at]["members"].remove(u)
            released.append(u)
        set_id = sets[si]["id"]
    elif k == "primary":
        si = set_at(sets, op["set"])
        oi = option_at(sets[si], op["option"])
        o = sets[si]["options"][oi]
        if sets[si]["primary"] == o["id"]:
            raise Refused("nothing_to_set", f"“{o['name']}” zaten birincil seçenek.", "operation.option")
        sets[si]["primary"] = o["id"]
        set_id, option = sets[si]["id"], o["id"]
    elif k == "accept":
        si = set_at(sets, op["set"])
        oi = option_at(sets[si], op["option"]) if "option" in op else \
            next(j for j, o in enumerate(sets[si]["options"]) if o["id"] == sets[si]["primary"])
        gone = [m for j, o in enumerate(sets[si]["options"]) if j != oi for m in o.get("members", []) if doc.present(m)]
        unlocked(doc, [doc.slot_of(m) for m in gone])
        option = sets[si]["options"][oi]["id"]
        released = list(sets[si]["options"][oi].get("members", []))
        removes = [doc.slot_of(m) for m in gone]
        deleted = gone
        del sets[si]
    elif k == "delete":
        si = set_at(sets, op["set"])
        oi = option_at(sets[si], op["option"])
        o = sets[si]["options"][oi]
        if o["id"] == sets[si]["primary"]:
            raise Refused("option_primary", f"“{o['name']}” birincil seçenek; silinemez. Önce başka bir seçeneği birincil yapın ya da seçeneği kabul edin.", "operation.option")
        gone = [m for m in o.get("members", []) if doc.present(m)]
        unlocked(doc, [doc.slot_of(m) for m in gone])
        removes = [doc.slot_of(m) for m in gone]
        deleted = gone
        option = o["id"]
        del sets[si]["options"][oi]
        set_id = sets[si]["id"]
    elif k == "rename":
        name_ok(op["name"], "operation.name")
        si = set_at(sets, op["set"])
        name = op["name"]
        if "option" not in op:
            if sets[si]["name"] == name:
                raise Refused("nothing_to_set", "Küme zaten bu adda.", "operation.name")
            if any(j != si and s["name"].lower() == name.lower() for j, s in enumerate(sets)):
                raise Refused("option_set_exists", f"Projede “{name}” adlı bir seçenek kümesi var.", "operation.name")
            sets[si]["name"] = name
        else:
            oi = option_at(sets[si], op["option"])
            if sets[si]["options"][oi]["name"] == name:
                raise Refused("nothing_to_set", "Seçenek zaten bu adda.", "operation.name")
            if any(j != oi and o["name"].lower() == name.lower() for j, o in enumerate(sets[si]["options"])):
                raise Refused("option_exists", f"“{sets[si]['name']}” kümesinde “{name}” adlı bir seçenek var.", "operation.name")
            sets[si]["options"][oi]["name"] = name
            option = sets[si]["options"][oi]["id"]
        set_id = sets[si]["id"]
    if k in ("define", "addMembers", "removeMembers"):
        saved = doc.sets
        doc.sets = sets
        try:
            doc.leaning()
        finally:
            doc.sets = saved
    # Written.
    doc.sets = sets
    for u, e in adds:
        slot = doc.next
        doc.next += 1
        e["id"] = slot
        doc.ents[slot] = e
        doc.uid[slot] = u
        created.append(u)
    for s in removes:
        del doc.ents[s]
    out = {}
    s = next((x for x in sets if x["id"] == set_id), None) if set_id else None
    if s is not None:
        out["set"] = clean(s)
    if option is not None:
        out["option"] = option
    if created:
        out["created"] = [Doc.out(u) for u in created]
    if deleted:
        # A deleted copy is named by the id captured when it was made: its slot is empty now.
        out["deleted"] = [Doc.out(u).replace("$uidOf:", "$uid:c") for u in deleted]
    if released:
        out["released"] = [Doc.out(u) for u in released]
    out["revision"] = "$current"
    return out


# ── Reading ──────────────────────────────────────────────────────────────────────────────────────────────────────


def ring_area(pts):
    a = 0.0
    for i in range(len(pts)):
        p, q = pts[i], pts[(i + 1) % len(pts)]
        a += p["x"] * q["y"] - q["x"] * p["y"]
    return abs(a) / 2


def row(doc, s, o):
    objs = [doc.ents[doc.slot_of(m)] for m in o.get("members", []) if doc.present(m)]
    kinds, area, length = {}, 0.0, 0.0
    for e in objs:
        kinds[e["kind"]] = kinds.get(e["kind"], 0) + 1
        if e["kind"] == "space":
            for part in e["parts"]:
                area += ring_area(part["outer"]["pts"]) - sum(ring_area(h["pts"]) for h in part.get("holes", []))
        if e["kind"] == "wall":
            pts = e["pts"]
            length += sum(math.hypot(pts[i + 1]["x"] - pts[i]["x"], pts[i + 1]["y"] - pts[i]["y"]) for i in range(len(pts) - 1))
    return {"id": o["id"], "name": o["name"], "primary": o["id"] == s["primary"], "objects": len(objs),
            "missing": len(o.get("members", [])) - len(objs),
            "kinds": [{"kind": kk, "count": kinds[kk]} for kk in sorted(kinds)], "spaceArea": area, "wallLength": length}


def points(e):
    """The points an object is placed by, as the comparison takes them: a path (either way) or fixed."""
    k = e["kind"]
    if k == "wall":
        a, b = e["pts"]
        return "path", [(a["x"], a["y"]), ((a["x"] + b["x"]) / 2, (a["y"] + b["y"]) / 2), (b["x"], b["y"])]
    if k == "opening":
        pts = [(p["x"], p["y"]) for p in e["place"]]
        mids = [((pts[i][0] + pts[i + 1][0]) / 2, (pts[i][1] + pts[i + 1][1]) / 2) for i in range(len(pts) - 1)]
        return "path", [x for pair in zip(pts, mids + [None]) for x in pair if x is not None]
    if k == "space":
        pts = [(p["x"], p["y"]) for p in e["parts"][0]["outer"]["pts"]]
        return "path", pts
    if k == "tag":
        return "path", [(e["at"]["x"], e["at"]["y"])]
    if k in ("insert", "text"):
        return "fixed", [(e["p"]["x"], e["p"]["y"])]
    raise ValueError(k)


def others(e):
    keys = {"wall": ("width", "justify"), "opening": ("at", "width", "form", "frame"),
            "space": ("number", "name"), "tag": ("text",), "insert": ("block", "scale", "rotation"),
            "text": ("text", "height")}[e["kind"]]
    return [e.get(x) for x in keys], e["attrs"], e["layerId"]


def distance(a, b):
    ka, pa = points(a)
    _, pb = points(b)
    if len(pa) != len(pb):
        return None
    d = lambda xs, ys: max(math.hypot(x[0] - y[0], x[1] - y[1]) for x, y in zip(xs, ys))  # noqa: E731
    if ka == "path":
        return min(d(pa, pb), d(pa, pb[::-1]))
    return d(pa, pb)


def compare(doc, s, a, b, search=0.5, tolerance=0.001):
    oa = [(m, doc.ents[doc.slot_of(m)]) for m in a.get("members", []) if doc.present(m)]
    ob = [(m, doc.ents[doc.slot_of(m)]) for m in b.get("members", []) if doc.present(m)]
    cands = []
    for i, (_, ea) in enumerate(oa):
        for j, (_, eb) in enumerate(ob):
            if ea["kind"] != eb["kind"]:
                continue
            dd = distance(ea, eb)
            if dd is not None and dd <= search:
                cands.append((dd, i, j))
    cands.sort()
    pa, pb = {}, {}
    for dd, i, j in cands:
        if i in pa or j in pb:
            continue
        pa[i], pb[j] = (j, dd), (i, dd)
    out = {"set": s["id"], "a": a["id"], "b": b["id"], "same": 0}
    added, removed, changed = [], [], []
    for i, (ua, ea) in enumerate(oa):
        if i not in pa:
            removed.append(ua)
            continue
        j, dd = pa[i]
        ub, eb = ob[j]
        oth_a, attrs_a, lay_a = others(ea)
        oth_b, attrs_b, lay_b = others(eb)
        geo = dd <= tolerance and oth_a == oth_b
        att = attrs_a == attrs_b and lay_a == lay_b
        if geo and att:
            out["same"] += 1
        else:
            changed.append({"a": Doc.out(ua), "b": Doc.out(ub), "what": "geometry" if att else ("attributes" if geo else "both")})
    for j, (ub, _) in enumerate(ob):
        if j not in pb:
            added.append(ub)
    if added:
        out["added"] = [Doc.out(u) for u in added]
    if removed:
        out["removed"] = [Doc.out(u) for u in removed]
    if changed:
        out["changed"] = changed
    ra, rb = row(doc, s, a), row(doc, s, b)
    out["spaceArea"] = rb["spaceArea"] - ra["spaceArea"]
    out["wallLength"] = rb["wallLength"] - ra["wallLength"]
    ka = {x["kind"]: x["count"] for x in ra["kinds"]}
    kb = {x["kind"]: x["count"] for x in rb["kinds"]}
    diffs = [{"kind": kk, "a": ka.get(kk, 0), "b": kb.get(kk, 0)} for kk in sorted(set(ka) | set(kb)) if ka.get(kk, 0) != kb.get(kk, 0)]
    if diffs:
        out["kinds"] = diffs
    return out


def query(doc, q):
    sets = doc.sets
    chosen = list(sets)
    out = {}
    if "set" in q:
        i = next((i for i, s in enumerate(sets) if s["id"] == q["set"]), None)
        if i is None:
            i = next((i for i, s in enumerate(sets) if s["name"].lower() == q["set"].strip().lower()), None)
        if i is None:
            raise Refused("option_set_not_found", f"Projede “{q['set']}” adlı ya da kimlikli tasarım seçeneği kümesi yok.", "set")
        chosen = [sets[i]]
    if "uid" in q:
        u = q["uid"]
        ok = len(u) == 36 and all(c == "-" if k in (8, 13, 18, 23) else c in "0123456789abcdef" for k, c in enumerate(u))
        if not ok:
            raise Refused("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "uid")
        if not doc.present(u):
            raise Refused("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uid")
        hit = next(((s, o) for s in sets for o in s["options"] if u in o.get("members", [])), None)
        if hit is None:
            raise Refused("not_in_option", "Nesne bir tasarım seçeneğinde değil: ana modelin.", "uid")
        chosen = [s for s in chosen if s["id"] == hit[0]["id"]]
        out["set"], out["option"] = hit[0]["id"], hit[1]["id"]
    if "compare" in q:
        c = q["compare"]
        i = next((i for i, s in enumerate(sets) if s["id"] == c["set"] or s["name"].lower() == c["set"].lower()), None)
        if i is None:
            raise Refused("option_set_not_found", f"Projede “{c['set']}” adlı ya da kimlikli tasarım seçeneği kümesi yok.", "compare.set")
        s = sets[i]

        def find(key, path):
            o = next((o for o in s["options"] if o["id"] == key), None) or \
                next((o for o in s["options"] if o["name"].lower() == key.lower()), None)
            if o is None:
                raise Refused("option_not_found", f"“{s['name']}” kümesinde “{key}” seçeneği yok.", path)
            return o
        a, b = find(c["a"], "compare.a"), find(c["b"], "compare.b")
        if a["id"] == b["id"]:
            raise Refused("invalid_input", "Bir seçenek kendisiyle karşılaştırılmaz; iki ayrı seçenek verin.", "compare.b")
        for key in ("search", "tolerance"):
            if key in c and not c[key] >= 0:
                raise Refused("invalid_input", "Arama uzaklığı ve tolerans sonlu, sıfır ya da daha büyük olmalı.", f"compare.{key}")
        out["compared"] = compare(doc, s, a, b, c.get("search", 0.5), c.get("tolerance", 0.001))
    rows = [{"id": s["id"], "name": s["name"], "primary": s["primary"], "options": [row(doc, s, o) for o in s["options"]]}
            for s in chosen]
    result = {"sets": rows, **out, "revision": "$current"}
    missing = sum(o["missing"] for r in rows for o in r["options"])
    warnings = [{"code": "option_member_gone", "message": f"Seçeneklerin {missing} nesnesi çizimde yok (silinmiş); bir sonraki seçenek yazmasında seçeneklerden düşer."}] if missing else []
    return result, warnings


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────


def mapped(v):
    """An output with the copies' ids as the cases name them."""
    if isinstance(v, str):
        return Doc.out(v)
    if isinstance(v, list):
        return [mapped(x) for x in v]
    if isinstance(v, dict):
        return {k: mapped(x) for k, x in v.items()}
    return v


class Case:
    def __init__(self, name, other=False):
        self.name = name
        # In the query file the edits are another command's steps.
        self.other = other
        self.doc = Doc()
        self.steps = []

    def edit(self, op, expect=None, quiet=False, plan=False, expected_revision=None):
        before_ids = sorted(self.doc.ents)
        inp = {"operation": op}
        if expected_revision is not None:
            inp["expectedRevision"] = expected_revision
        saved = copy.deepcopy((self.doc.ents, self.doc.uid, self.doc.next, self.doc.sets))
        out = {}
        try:
            if expected_revision == "x1":
                raise Refused("invalid_revision", "Beklenen sürüm “x1” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
            if expected_revision == "999":
                raise Refused("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", status="conflict")
            out = edit(self.doc, op)
            result = ANY if quiet else {"status": "completed", "output": out, "warnings": []}
            if plan:
                self.doc.ents, self.doc.uid, self.doc.next, self.doc.sets = saved
        except Refused as r:
            self.doc.ents, self.doc.uid, self.doc.next, self.doc.sets = saved
            err = dict(r.error)
            if r.status == "conflict":
                err["revision"] = "$current"
            result = {"status": r.status, "error": err}
            expect = {**(expect or {}), "ids": before_ids, "revision": "same"}
        step = {"op": "plan" if plan else "execute", "input": inp, "result": result}
        if self.other:
            step = {"op": step["op"], "command": "cad.options.edit", "input": inp, "result": result}
        if expect:
            step["expect"] = expect
        self.steps.append(step)
        # The copies' ids, captured to name them once deleted.
        if not plan:
            for u in out.get("created", []):
                self.steps.append({"op": "captureUid", "id": int(u.split(":")[1]), "as": "c" + u.split(":")[1]})
        return self

    def run(self, command, inp, change):
        change(self.doc)
        self.steps.append({"op": "execute", "command": command, "input": inp, "result": ANY})
        return self

    def query(self, q, within=True):
        def inner(v):
            if isinstance(v, str) and v.startswith("$uidOf:"):
                return UID(0x1000 + int(v.split(":")[1]))
            if isinstance(v, dict):
                return {k: inner(x) for k, x in v.items()}
            return v
        try:
            out, warnings = query(self.doc, inner(q))
            result = {"status": "completed", "output": out, "warnings": warnings}
        except Refused as r:
            result = {"status": "failed", "error": r.error}
        step = {"op": "execute", "command": "cad.options.query", "input": q, "result": result}
        if within:
            step["within"] = 1e-9
        self.steps.append(step)
        return self

    def undo(self, state, expect):
        self.doc.ents, self.doc.uid, self.doc.next, self.doc.sets = state
        self.steps.append({"op": "undo", "returns": STEP, "expect": expect})
        return self

    def snapshot(self):
        return copy.deepcopy((self.doc.ents, self.doc.uid, self.doc.next, self.doc.sets))

    def done(self):
        return {"name": self.name, "steps": self.steps}


DEFINE = {"kind": "define", "name": "Mutfak", "uids": [UID(i) for i in KITCHEN]}
COPY = {"kind": "copy", "set": "mutfak", "option": "a"}


def ids(doc):
    return sorted(doc.ents)


def edit_cases():
    out = []
    c = Case("Tanımla: mutfağın beş nesnesi kümenin birincil A seçeneği olur, yerlerinde kalır")
    c.edit(DEFINE, {"ids": IDS, "revision": "changed", "canUndo": True})
    out.append(c.done())

    c = Case("Tanımla ve geri al: proje seçeneksiz kalır")
    s = c.snapshot()
    c.edit(DEFINE)
    c.undo(s, {"ids": IDS, "revision": "changed", "canRedo": True, "settings": {"arch": {"levels": setup()["settings"]["arch"]["levels"]}}})
    out.append(c.done())

    c = Case("Tanımla: kimlik ve ilk seçeneğin adı verilir")
    c.edit({"kind": "define", "name": "Mutfak düzeni", "id": "md", "option": "Ada mutfak", "uids": [UID(i) for i in KITCHEN]})
    out.append(c.done())

    c = Case("Tanımla: ad Türkçe harfleriyle kimlik olur, ikinci küme ad çakışmasız")
    c.edit({"kind": "define", "name": "Çalışma köşesi", "uids": [UID(13)]})
    c.edit({"kind": "define", "name": "Mutfak", "uids": [UID(i) for i in KITCHEN]})
    out.append(c.done())

    c = Case("Tanımla'nın planı yazmaz")
    c.edit(DEFINE, {"revision": "same"}, plan=True)
    out.append(c.done())

    c = Case("Ret: ana modeldeki kapı seçeneğe alınan duvara bağlı")
    c.edit({"kind": "define", "name": "Duvar", "uids": [UID(5)]})
    out.append(c.done())

    c = Case("Ret: ana modeldeki etiket seçeneğe alınan mahale bağlı")
    c.edit({"kind": "define", "name": "Mahal", "uids": [UID(7)]})
    out.append(c.done())

    c = Case("Ret: aynı adda küme")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "define", "name": "mutfak", "uids": [UID(13)]})
    out.append(c.done())

    c = Case("Ret: nesne zaten bir seçenekte")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "define", "name": "Başka", "uids": [UID(13), UID(9)]})
    out.append(c.done())

    for name, uids in (("Ret: nesne verilmedi", []), ("Ret: kimlik geçersiz", ["x"]),
                       ("Ret: çizimde olmayan nesne", [UID(99)])):
        c = Case(name)
        c.edit({"kind": "define", "name": "Mutfak", "uids": uids})
        out.append(c.done())

    c = Case("Ret: adın başında boşluk")
    c.edit({"kind": "define", "name": " Mutfak", "uids": [UID(13)]})
    out.append(c.done())

    # ── Kopyala ──────────────────────────────────────────────────────────────────────────────────────────────────
    c = Case("Kopyala: A'nın bağımsız kopyası B olur; kapı, etiket ve bağlar kopyalara")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, {"ids": IDS + [16, 17, 18, 19, 20],
                  "fields": {"16": {"pts": [P(3, 0), P(3, 4)]}, "17": {"host": "$uidOf:16", "at": 2.0, "number": "K2"},
                             "19": {"of": "$uidOf:18", "text": "M1"}, "20": {"p": P(4.0, 3.5)}},
                  "revision": "changed"})
    out.append(c.done())

    c = Case("Kopyala: birincil yapılarak, adıyla")
    c.edit(DEFINE, quiet=True)
    c.edit({**COPY, "name": "Ada", "primary": True})
    out.append(c.done())

    c = Case("Kopyala ve geri al: kopyalar ve B gider")
    c.edit(DEFINE, quiet=True)
    s = c.snapshot()
    c.edit(COPY, quiet=True)
    c.undo(s, {"ids": IDS, "revision": "changed", "canRedo": True})
    c.query({})
    out.append(c.done())

    c = Case("Kopyala: üçüncü seçenek C harfini alır")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "copy", "set": "Mutfak", "option": "B"})
    out.append(c.done())

    c = Case("Ret: kümede on iki seçenek varken kopyalamak")
    c.edit(DEFINE, quiet=True)
    for _ in range(11):
        c.edit(COPY, quiet=True)
    c.edit(COPY)
    out.append(c.done())

    c = Case("Ret: kümede olmayan seçenek")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "copy", "set": "mutfak", "option": "z"})
    out.append(c.done())

    c = Case("Ret: projede olmayan küme")
    c.edit({"kind": "copy", "set": "banyo", "option": "a"})
    out.append(c.done())

    c = Case("Ret: kopyanın adı kümede var")
    c.edit(DEFINE, quiet=True)
    c.edit({**COPY, "name": "a"})
    out.append(c.done())

    c = Case("Ret: kilitli katmandaki nesneyi kopyalamak")
    c.edit({"kind": "define", "name": "Dolap", "uids": [UID(14)]}, quiet=True)
    c.edit({"kind": "copy", "set": "dolap", "option": "a"})
    out.append(c.done())

    # ── Ekle ve çıkar ─────────────────────────────────────────────────────────────────────────────────────────────
    c = Case("Seçeneğe ekle: yazı A'ya katılır")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "addMembers", "set": "mutfak", "option": "a", "uids": [UID(13)]})
    out.append(c.done())

    c = Case("Ret: dış duvarı eklemek ana modelin kapısını bağsız bırakır")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "addMembers", "set": "mutfak", "option": "a", "uids": [UID(1)]})
    out.append(c.done())

    c = Case("Seçenekten çıkar: tezgah ana modele döner")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "removeMembers", "set": "mutfak", "uids": [UID(9)]})
    out.append(c.done())

    c = Case("Ret: etiketi çıkarmak ana modelde seçeneğin mahaline bağlı etiket bırakır")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "removeMembers", "set": "mutfak", "uids": [UID(8)]})
    out.append(c.done())

    c = Case("Ret: seçenekte olmayan nesneyi çıkarmak")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "removeMembers", "set": "mutfak", "uids": [UID(13)]})
    out.append(c.done())

    # ── Birincil, kabul, sil ─────────────────────────────────────────────────────────────────────────────────────
    c = Case("Birincil yap: B")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "primary", "set": "mutfak", "option": "b"}, {"ids": IDS + [16, 17, 18, 19, 20], "revision": "changed"})
    out.append(c.done())

    c = Case("Birincil yap'ın planı yazmaz")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "primary", "set": "mutfak", "option": "b"}, {"revision": "same"}, plan=True)
    out.append(c.done())

    c = Case("Ret: zaten birincil")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "primary", "set": "mutfak", "option": "a"})
    out.append(c.done())

    c = Case("Kabul et: B ana model olur, A'nın nesneleri silinir, küme kalkar; tek geri alma hepsini getirir")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    s = c.snapshot()
    c.edit({"kind": "accept", "set": "mutfak", "option": "b"}, {"ids": [1, 2, 3, 4] + list(range(10, 21)), "revision": "changed"})
    c.undo(s, {"ids": IDS + [16, 17, 18, 19, 20], "revision": "changed"})
    out.append(c.done())

    c = Case("Kabul et: seçenek verilmezse birincil kalır, kopyalar silinir")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "accept", "set": "mutfak"}, {"ids": IDS, "revision": "changed"})
    out.append(c.done())

    c = Case("Ret: kabulde silinecek nesne kilitli katmanda")
    c.edit({"kind": "define", "name": "Not", "uids": [UID(13)]}, quiet=True)
    c.edit({"kind": "copy", "set": "not", "option": "a"}, quiet=True)
    c.edit({"kind": "addMembers", "set": "not", "option": "a", "uids": [UID(14)]}, quiet=True)
    c.edit({"kind": "accept", "set": "not", "option": "b"})
    out.append(c.done())

    c = Case("Seçeneği sil: B nesneleriyle gider")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "delete", "set": "mutfak", "option": "b"}, {"ids": IDS, "revision": "changed"})
    out.append(c.done())

    c = Case("Ret: birincil seçenek silinmez")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "delete", "set": "mutfak", "option": "a"})
    out.append(c.done())

    # ── Ad ───────────────────────────────────────────────────────────────────────────────────────────────────────
    c = Case("Yeniden adlandır: küme ve seçenek")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "rename", "set": "mutfak", "name": "Mutfak düzeni"})
    c.edit({"kind": "rename", "set": "mutfak", "option": "b", "name": "Ada"})
    out.append(c.done())

    c = Case("Ret: ad aynı")
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "rename", "set": "mutfak", "option": "a", "name": "A"})
    out.append(c.done())

    c = Case("Ret: seçeneğin yeni adı kümede var")
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "rename", "set": "mutfak", "option": "b", "name": "a"})
    out.append(c.done())

    # ── Silinen nesne, sürüm ─────────────────────────────────────────────────────────────────────────────────────
    c = Case("Silinen nesne bir sonraki yazmada seçenekten düşer")
    c.edit(DEFINE, quiet=True)

    def gone(doc):
        del doc.ents[9]
    c.run("cad.entities.delete", {"uids": [UID(9)]}, gone)
    c.edit({"kind": "rename", "set": "mutfak", "name": "Mutfak 2"})
    out.append(c.done())

    c = Case("Ret: beklenen sürüm geçersiz")
    c.edit(DEFINE, expected_revision="x1")
    out.append(c.done())

    c = Case("Ret: çizim değişmiş")
    c.edit(DEFINE, expected_revision="999")
    out.append(c.done())
    return out


def moved(slot, dx):
    def change(doc):
        e = doc.ents[slot]
        if e["kind"] == "wall":
            e["pts"] = [P(p["x"] + dx, p["y"]) for p in e["pts"]]
            # Its door follows it.
            for o in doc.ents.values():
                if o["kind"] == "opening" and o["host"] == doc.uid[slot]:
                    o["place"] = [P(p["x"] + dx, p["y"]) for p in o["place"]]
        else:
            e["p"] = P(e["p"]["x"] + dx, e["p"]["y"])
    return change


def query_cases():
    out = []
    c = Case("Seçeneksiz proje: küme yok", other=True)
    c.query({})
    out.append(c.done())

    c = Case("Tanımdan sonra: A'nın sayıları, mahalin alanı, duvarın boyu", other=True)
    c.edit(DEFINE, quiet=True)
    c.query({})
    out.append(c.done())

    c = Case("Kümeyi adıyla", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "define", "name": "Not", "uids": [UID(13)]}, quiet=True)
    c.query({"set": "Mutfak"})
    out.append(c.done())

    c = Case("Kümeyi kimliğiyle", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit({"kind": "define", "name": "Not", "uids": [UID(13)]}, quiet=True)
    c.query({"set": "not"})
    out.append(c.done())

    c = Case("Ret: projede olmayan küme", other=True)
    c.edit(DEFINE, quiet=True)
    c.query({"set": "banyo"})
    out.append(c.done())

    c = Case("Nesnenin seçeneği", other=True)
    c.edit(DEFINE, quiet=True)
    c.query({"uid": UID(7)})
    out.append(c.done())

    c = Case("Kopyanın seçeneği B", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.query({"uid": "$uidOf:18"})
    out.append(c.done())

    c = Case("Ret: ana modelin nesnesi", other=True)
    c.edit(DEFINE, quiet=True)
    c.query({"uid": UID(11)})
    out.append(c.done())

    c = Case("Ret: kimlik geçersiz", other=True)
    c.query({"uid": "x"})
    out.append(c.done())

    c = Case("Ret: çizimde olmayan nesne", other=True)
    c.query({"uid": UID(99)})
    out.append(c.done())

    c = Case("Kopyadan sonra iki seçenek aynı sayıda", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.query({"set": "mutfak"})
    out.append(c.done())

    c = Case("Karşılaştır: değişmemiş kopya, hepsi aynı", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b"}})
    out.append(c.done())

    c = Case("Karşılaştır: B'nin bölme duvarı 30 cm kaydı; duvar ve kapısı değişti", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.run("cad.entities.transform", {"uids": ["$uidOf:16"], "transform": {"kind": "move", "dx": 0.3, "dy": 0.0}}, moved(16, 0.3))
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b"}})
    out.append(c.done())

    c = Case("Karşılaştır: B'nin tezgahı silindi; yalnız A'da", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)

    def gone(doc):
        del doc.ents[20]
    c.run("cad.entities.delete", {"uids": ["$uidOf:20"]}, gone)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b"}})
    out.append(c.done())

    c = Case("Karşılaştır: B'ye yazı eklendi; yalnız B'de", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "addMembers", "set": "mutfak", "option": "b", "uids": [UID(13)]}, quiet=True)
    c.query({"compare": {"set": "mutfak", "a": "A", "b": "B"}})
    out.append(c.done())

    c = Case("Karşılaştır: B'nin tezgahı 80 cm kaydı; arama 50 cm'de eşleşmez", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.run("cad.entities.transform", {"uids": ["$uidOf:20"], "transform": {"kind": "move", "dx": 0.8, "dy": 0.0}}, moved(20, 0.8))
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b"}})
    out.append(c.done())

    c = Case("Karşılaştır: aynı kayma 1 m aramada değişen çift", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.run("cad.entities.transform", {"uids": ["$uidOf:20"], "transform": {"kind": "move", "dx": 0.8, "dy": 0.0}}, moved(20, 0.8))
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b", "search": 1.0}})
    out.append(c.done())

    c = Case("Ret: seçenek kendisiyle karşılaştırılmaz", other=True)
    c.edit(DEFINE, quiet=True)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "A"}})
    out.append(c.done())

    c = Case("Ret: karşılaştırmada olmayan seçenek", other=True)
    c.edit(DEFINE, quiet=True)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "c"}})
    out.append(c.done())

    c = Case("Ret: karşılaştırmada olmayan küme", other=True)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b"}})
    out.append(c.done())

    c = Case("Ret: eksi arama uzaklığı", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b", "search": -1.0}})
    out.append(c.done())

    c = Case("Silinen nesne eksik sayılır ve söylenir", other=True)
    c.edit(DEFINE, quiet=True)

    def gone9(doc):
        del doc.ents[9]
    c.run("cad.entities.delete", {"uids": [UID(9)]}, gone9)
    c.query({})
    out.append(c.done())

    c = Case("Kabulden sonra küme yok", other=True)
    c.edit(DEFINE, quiet=True)
    c.edit(COPY, quiet=True)
    c.edit({"kind": "accept", "set": "mutfak", "option": "b"}, quiet=True)
    c.query({})
    out.append(c.done())
    return out


def view_cases():
    """A view recipe's options (docs/adr/0788 §3) through `cad.views.edit`."""
    out = []
    save = {"edit": {"kind": "save", "name": "Mutfak B", "center": P(3, 2), "width": 8.0, "height": 6.0}}

    def steps(recipe, result):
        return [
            {"op": "execute", "input": {"operation": DEFINE}, "result": ANY},
            {"op": "execute", "input": {"operation": COPY}, "result": ANY},
            {"op": "execute", "command": "cad.views.edit", "input": save, "result": ANY},
            {"op": "execute", "command": "cad.views.edit",
             "input": {"edit": {"kind": "recipe", "name": "Mutfak B", "recipe": recipe}}, "result": result},
        ]
    out.append({"name": "Görünüş tarifi: B seçeneğini gösterir", "steps": steps({"options": [{"set": "mutfak", "option": "b"}]}, ANY)})
    out.append({"name": "Tarif kat ve seçenekle", "steps": steps({"level": {"id": "zemin"}, "options": [{"set": "mutfak", "option": "a"}]}, ANY)})
    out.append({"name": "Ret: tarifte olmayan küme", "steps": steps(
        {"options": [{"set": "banyo", "option": "a"}]},
        {"status": "failed", "error": {"code": "option_set_not_found", "message": "Projede “banyo” kimlikli tasarım seçeneği kümesi yok. Tasarım seçenekleri'ndeki kümelerden birini verin.", "path": "edit.recipe.options[0].set"}})})
    out.append({"name": "Ret: tarifte olmayan seçenek", "steps": steps(
        {"options": [{"set": "mutfak", "option": "z"}]},
        {"status": "failed", "error": {"code": "option_not_found", "message": "“Mutfak” kümesinde “z” kimlikli seçenek yok. Kümenin seçeneklerinden birini verin.", "path": "edit.recipe.options[0].option"}})})
    out.append({"name": "Ret: bir küme iki kez", "steps": steps(
        {"options": [{"set": "mutfak", "option": "a"}, {"set": "mutfak", "option": "b"}]},
        {"status": "failed", "error": {"code": "invalid_recipe", "message": "“mutfak” kümesi görünüş tarifinde iki kez seçilmiş. Tarifi düzeltip yeniden verin.", "path": "edit.recipe"}})})
    out.append({"name": "Ret: seçenek kimlik değil", "steps": steps(
        {"options": [{"set": "Mutfak", "option": "b"}]},
        {"status": "failed", "error": {"code": "invalid_recipe", "message": "Görünüş tarifinin seçeneği bir kümenin ve seçeneğin kimliğiyle verilir. Tarifi düzeltip yeniden verin.", "path": "edit.recipe"}})})
    # docs/adr/0788 §3 with docs/adr/0783 §2: like the level, the option a view shows is its own; a template carries none.
    template = {"edit": {"kind": "template", "name": "Uygulama", "recipe": {"scale": 50}}}
    follows = steps({"template": "Uygulama", "options": [{"set": "mutfak", "option": "b"}]}, ANY)
    out.append({"name": "Şablona bağlı tarif kendi seçeneğini tutar",
                "steps": follows[:3] + [{"op": "execute", "command": "cad.views.edit", "input": template, "result": ANY}] + follows[3:]})
    out.append({"name": "Ret: şablon tasarım seçeneği taşımaz", "steps": steps(
        {"options": [{"set": "mutfak", "option": "b"}]}, ANY)[:3] + [
        {"op": "execute", "command": "cad.views.edit",
         "input": {"edit": {"kind": "template", "name": "Uygulama", "recipe": {"scale": 50, "options": [{"set": "mutfak", "option": "b"}]}}},
         "result": {"status": "failed", "error": {"code": "invalid_recipe", "message": "Görünüş şablonu tasarım seçeneği taşımaz: hangi seçeneğin gösterileceği görünümündür. Şablonu düzeltip yeniden verin.", "path": "edit.recipe"}}}]})
    return out


def write(command, title, cases, rest=""):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/option_cases.py yazar: ADR 0788'in kurallarından, KentOS kodu olmadan. Değiştirmeyin; betiği düzeltin." + rest,
            "setup": setup(), "cases": cases}


def main():
    files = {
        EDIT: write("cad.options.edit", "Tasarım seçenekleri (docs/adr/0788)", edit_cases() + view_cases()),
        QUERY: write("cad.options.query", "Tasarım seçeneklerini oku (docs/adr/0788)", query_cases()),
    }
    check = "--check" in sys.argv
    bad = False
    for path, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)}: başvurudan farklı", file=sys.stderr)
                bad = True
            else:
                print(f"{data['command']} ({path.name}): {len(data['cases'])} durum başvuruyla aynı.")
        else:
            path.write_text(text)
            print(f"{path.relative_to(ROOT)}: {len(data['cases'])} durum yazıldı.")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
