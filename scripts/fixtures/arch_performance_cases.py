#!/usr/bin/env python3
"""cad.arch.performance v1 (“Yangın ve performans”, docs/adr/8068): the shared cases, written from the ADR without KentOS
code.

The drawing: a wall of the style “Dış duvar” with a door of the type K01 and a window without a type in it, a column, a
stair, a room, a grid, a wall of the style on a locked layer and a wall without a style. The project has the style, the
type and two materials (Tuğla, Betonarme).

The rules (ADR 8068 §2):
- objects: `fireRating` on a wall, slab, column, beam, roof, railing, stair, ramp or opening; `exit` on a door, a stair
  or a roof (an escape route, ADR 8090); `loadBearing` on a wall that is no curtain wall (ADR 8090) or a column that is
  no post (ADR 8124); `external` on a wall or a slab that is no covering (ADR 8124); else field_not_for_kind at the
  object (in the input's order, the class asked first, then the exit, the bearing, the exterior). None given:
  nothing_to_set. A field absent stays; null takes the object's own away.
- exits (ADR 8124): the doors and stairs of `yes` are exits, those of `no` not, each the author's statement; none at all
  no_entities, a uid in both uid_in_both_lists (at operation.no[i]), then the uids as objects (entity_not_found at
  uids[i] of yes then no), anything but a door or a stair field_not_for_kind (at operation.yes[i] or operation.no[i]),
  the revision, a locked layer (layer_locked at the same place).
- wallStyle / openingType / material: by id, else by name (case aside); none: style_not_found, opening_type_not_found,
  material_not_found. A class null: none. A strength: letters, digits and / . , - , at most 16 (invalid_strength). A
  material's `strength`, `reaction` (ADR 8091) and `roofClass` (ADR 8134): absent stays, null takes it away, none given
  nothing_to_set; a reaction class as TS EN 13501-1 writes it (invalid_declared at operation.reaction.value); a roof
  class as TS EN 13501-5 and BYKHY 28/2 write it, a letter B–F, ROOF, then (t1)…(t4) or nothing (invalid_declared at
  operation.roofClass.value; the reaction is checked first).
- building: the fire protection class; null: none (no empty performance is kept).
- A declared word: not empty, no space at its ends, at most 64 characters, no line break; a document basis with a name
  (at most 128) and, when given, a day YYYY-MM-DD; else invalid_declared.
- Then expectedRevision (invalid_revision, revision_conflict), then a named object on a locked layer (layer_locked).
- One undo step “Yangın ve performans”. The answer: objects: those named, each once, in the drawing's order; a type
  given a class: its objects without their own, in the drawing's order (none when taken away); a material, the building:
  none.
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.performance.json"
COMMAND = "cad.arch.performance"
STEP = "Yangın ve performans"


def U(n):
    return f"0192f5a1-8068-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def base(id, layer_id):
    return {"id": id, "layerId": layer_id, "attrs": {}}


WALL, DOOR, WINDOW, COLUMN, STAIR, ROOM, GRID, LOCKED, BARE, ROOF = range(1, 11)
# Added by `with_more` for the readings and exits (ADR 8124): a floor slab, a post, a door on the locked layer.
SLAB, POST, LOCKED_DOOR = range(11, 14)
OPENINGS = {DOOR, WINDOW}

STYLE = {"id": "dis-duvar", "name": "Dış duvar", "components": [
    {"name": "İç sıva", "material": "İç sıva", "thickness": 0.02, "detail": "detailed"},
    {"name": "Tuğla", "material": "Tuğla", "materialId": "tugla", "edge": {"lineWeight": 0.5},
     "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0}, "detail": "standard"},
    {"name": "Isı yalıtımı", "material": "Isı yalıtımı", "thickness": 0.05,
     "hatch": {"pattern": "ANSI37", "scale": 0.5, "angle": 0.0}, "detail": "standard"},
    {"name": "Dış sıva", "material": "Dış sıva", "thickness": 0.03, "detail": "detailed"}], "width": 0.25}
TYPE = {"id": "k01", "name": "K01", "version": 1, "form": "door", "width": 1.02, "frame": 0.05, "head": 2.1,
        "build": {"sizeBy": "leaf", "fit": 0.01, "leafThickness": 0.04}}
MATERIALS = [{"id": "tugla", "name": "Tuğla", "category": "structure",
              "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0}, "cut": {"lineWeight": 0.5}},
             {"id": "betonarme", "name": "Betonarme", "category": "structure"}]


def drawing():
    ents = {
        WALL: {"kind": "wall", **base(WALL, "duvar"), "pts": [P(0, 0), P(10, 0)], "width": 0.25, "justify": "center",
               "style": "dis-duvar"},
        DOOR: {"kind": "opening", **base(DOOR, "kapi"), "host": U(WALL), "at": 1.5, "width": 1.02, "form": "door",
               "frame": 0.05, "head": 2.1, "style": "k01", "number": "K1", "typeVersion": 1,
               "build": {"sizeBy": "leaf", "fit": 0.01, "leafThickness": 0.04},
               "place": [P(0.99, 0.125), P(0.99, -0.125), P(2.01, 0.125), P(2.01, -0.125)]},
        WINDOW: {"kind": "opening", **base(WINDOW, "kapi"), "host": U(WALL), "at": 5.0, "width": 1.2, "form": "window",
                 "sill": 0.9, "head": 2.1, "number": "P1",
                 "place": [P(4.4, 0.125), P(4.4, -0.125), P(5.6, 0.125), P(5.6, -0.125)]},
        COLUMN: {"kind": "column", **base(COLUMN, "mimari"), "at": P(20, 0), "width": 0.4, "depth": 0.4},
        STAIR: {"kind": "stair", **base(STAIR, "mimari"), "pts": [P(25, 0), P(28.75, 0)], "width": 1.2, "risers": 16,
                "riserHeight": 0.175, "tread": 0.25},
        ROOM: {"kind": "space", **base(ROOM, "mimari"),
               "parts": [{"outer": {"pts": [P(30, 0), P(34, 0), P(34, 3), P(30, 3)]}}], "name": "Oda"},
        GRID: {"kind": "grid", **base(GRID, "mimari"), "pts": [P(-2, -2), P(-2, 6)], "name": "A"},
        LOCKED: {"kind": "wall", **base(LOCKED, "kilitli"), "pts": [P(0, 10), P(5, 10)], "width": 0.25,
                 "justify": "center", "style": "dis-duvar"},
        BARE: {"kind": "wall", **base(BARE, "duvar"), "pts": [P(0, 20), P(5, 20)], "width": 0.2, "justify": "center"},
        # A flat roof (ADR 8090): it takes an escape route role.
        ROOF: {"kind": "roof", **base(ROOF, "mimari"), "pts": [P(40, 0), P(46, 0), P(46, 4), P(40, 4)],
               "slopes": [0.0, 0.0, 0.0, 0.0]},
    }
    return {
        "format": "kentos.document", "version": 2, "name": "Yangın ve performans",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50.0, "workspace": "cad", "wallStyles": [copy.deepcopy(STYLE)],
                     "arch": {"openingTypes": [copy.deepcopy(TYPE)], "materials": copy.deepcopy(MATERIALS)}},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mimari", "Mimari"),
                   layer("kilitli", "Kilitli", locked=True)],
        "activeLayer": "duvar",
        "entities": [ents[i] for i in sorted(ents)],
        "uids": [U(i) for i in sorted(ents)],
        "styles": {"items": [], "categories": []},
    }


def fire_rated(e):
    return e["kind"] in ("wall", "slab", "column", "beam", "roof", "railing", "stair", "ramp", "opening")


def exit_rated(e):
    # A roof as an escape route too (ADR 8090, BYKHY Ek-3/B 4-a).
    return e["kind"] in ("stair", "roof") or (e["kind"] == "opening" and e["form"] == "door")


def load_bearing_rated(e):
    # A wall's load-bearing role (ADR 8090, Ek-3/B 2): not a curtain wall's; a column's that is no post (ADR 8124).
    return (e["kind"] == "wall" and not e.get("curtain")) or (e["kind"] == "column" and not e.get("member"))


def external_rated(e):
    # A wall's or a slab's declared exterior role (ADR 8124): not a covering's.
    return e["kind"] == "wall" or (e["kind"] == "slab" and not e.get("covering"))


def is_door_or_stair(e):
    return e["kind"] == "stair" or (e["kind"] == "opening" and e["form"] == "door")


def is_day(s):
    if len(s) != 10 or s[4] != "-" or s[7] != "-" or not (s[:4] + s[5:7] + s[8:]).isdigit():
        return False
    y, m, d = int(s[:4]), int(s[5:7]), int(s[8:])
    leap = (y % 4 == 0 and y % 100 != 0) or y % 400 == 0
    days = {1: 31, 2: 29 if leap else 28, 3: 31, 4: 30, 5: 31, 6: 30, 7: 31, 8: 31, 9: 30, 10: 31, 11: 30, 12: 31}
    return m in days and 1 <= d <= days[m]


def basis_bad(b):
    if b["kind"] != "document":
        return False
    name = b["name"]
    if not name.strip() or len(name) > 128 or any(ord(c) < 32 for c in name):
        return True
    return "date" in b and not is_day(b["date"])


def declared_bad(d):
    v = d["value"]
    if not v.strip() or v != v.strip() or len(v) > 64 or any(ord(c) < 32 for c in v):
        return True
    return basis_bad(d["basis"])


def reaction_bad(s):
    """A TS EN 13501-1 class as Ek-2/Ç writes it (ADR 8091): A1, A2, B … F, `fl` for a floor, smoke -s1…-s3, droplets
    ,d0…,d2 (or -d2 with no smoke; none on a floor's); A1 and F take neither."""
    import re
    m = re.fullmatch(r"(A1|A2|B|C|D|E|F)(fl)?(-s[1-3])?((?:,|-)d[0-2])?", s)
    if not m:
        return True
    cls, fl, smoke, drop = m.groups()
    if drop and (fl or (drop.startswith("-") and smoke) or (drop.startswith(",") and not smoke)):
        return True
    return cls in ("A1", "F") and bool(smoke or drop)


def roof_class_bad(s):
    """A TS EN 13501-5 class as BYKHY 28/2 and Ek-2/Ç write it (ADR 8134): B, C, D, E or F, ROOF, the test method in
    parentheses or none."""
    import re
    return re.fullmatch(r"[BCDEF]ROOF(\(t[1-4]\))?", s) is None


def strength_bad(s):
    return not s or len(s) > 16 or not all(c.isalnum() or c in "/.,-" for c in s)


def refusal(code, path, status="failed"):
    err = {"code": code, "message": "$any", "path": path}
    out = {"status": status, "error": err}
    if status == "conflict":
        out["error"]["revision"] = "$current"
    return out


def revision_refusal(inp):
    rev = inp.get("expectedRevision")
    if rev is None:
        return None
    if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
        return refusal("invalid_revision", "expectedRevision")
    return refusal("revision_conflict", "expectedRevision", "conflict")


def find(items, key):
    for i, x in enumerate(items):
        if x["id"] == key:
            return i
    for i, x in enumerate(items):
        if x["name"].lower() == key.lower():
            return i
    return None


def put(obj, key, value):
    """`obj[key]` set, or gone when `value` is None."""
    if value is None:
        obj.pop(key, None)
    else:
        obj[key] = value


def run(d, inp):
    """(result, drawing after) for `inp` on `d`; the drawing unchanged on a refusal."""
    op = inp["operation"]
    kind = op["kind"]
    after = copy.deepcopy(d)
    ents = after["entities"]
    uid_of = {e["id"]: u for e, u in zip(d["entities"], d["uids"])}
    id_of = {u: i for i, u in uid_of.items()}
    by_id = {e["id"]: e for e in ents}
    locked_layers = {l["id"] for l in d["layers"] if l["locked"]}
    settings = after["settings"]
    if kind == "objects":
        uids = op["uids"]
        if not uids:
            return refusal("no_entities", "uids"), d
        for i, u in enumerate(uids):
            if not (len(u) == 36 and u == u.lower() and u.count("-") == 4):
                return refusal("invalid_uid", f"uids[{i}]"), d
        seen, found = set(), []
        for i, u in enumerate(uids):
            if u in seen:
                continue
            seen.add(u)
            if u not in id_of:
                return refusal("entity_not_found", f"uids[{i}]"), d
            found.append((i, by_id[id_of[u]]))
        for i, e in found:
            if "fireRating" in op and not fire_rated(e):
                return refusal("field_not_for_kind", f"uids[{i}]"), d
            if "exit" in op and not exit_rated(e):
                return refusal("field_not_for_kind", f"uids[{i}]"), d
            if "loadBearing" in op and not load_bearing_rated(e):
                return refusal("field_not_for_kind", f"uids[{i}]"), d
            if "external" in op and not external_rated(e):
                return refusal("field_not_for_kind", f"uids[{i}]"), d
        if not any(k in op for k in ("fireRating", "exit", "loadBearing", "external")):
            return refusal("nothing_to_set", "operation"), d
        if op.get("fireRating") is not None and declared_bad(op["fireRating"]):
            return refusal("invalid_declared", "operation.fireRating"), d
        if op.get("exit") is not None and basis_bad(op["exit"]["basis"]):
            return refusal("invalid_declared", "operation.exit.basis"), d
        if op.get("loadBearing") is not None and basis_bad(op["loadBearing"]["basis"]):
            return refusal("invalid_declared", "operation.loadBearing.basis"), d
        if op.get("external") is not None and basis_bad(op["external"]["basis"]):
            return refusal("invalid_declared", "operation.external.basis"), d
        r = revision_refusal(inp)
        if r:
            return r, d
        for i, e in found:
            if e["layerId"] in locked_layers:
                return refusal("layer_locked", f"uids[{i}]"), d
        for _, e in found:
            target = e if e["kind"] == "opening" else e.setdefault("arch", {})
            for key in ("fireRating", "exit", "loadBearing", "external"):
                if key in op:
                    put(target, key, op[key])
            if e["kind"] != "opening" and not e["arch"]:
                del e["arch"]
        order = {e["id"]: k for k, e in enumerate(d["entities"])}
        out = [uid_of[e["id"]] for _, e in sorted(found, key=lambda x: order[x[1]["id"]])]
    elif kind == "exits":
        yes, no = op.get("yes", []), op.get("no", [])
        both = yes + no
        place = lambda i: f"operation.yes[{i}]" if i < len(yes) else f"operation.no[{i - len(yes)}]"  # noqa: E731
        if not both:
            return refusal("no_entities", "uids"), d
        for i, u in enumerate(both):
            if not (len(u) == 36 and u == u.lower() and u.count("-") == 4):
                return refusal("invalid_uid", f"uids[{i}]"), d
        for i, u in enumerate(no):
            if u in yes:
                return refusal("uid_in_both_lists", f"operation.no[{i}]"), d
        seen, found = set(), []
        for i, u in enumerate(both):
            if u in seen:
                continue
            seen.add(u)
            if u not in id_of:
                return refusal("entity_not_found", f"uids[{i}]"), d
            found.append((i, by_id[id_of[u]]))
        for i, e in found:
            if not is_door_or_stair(e):
                return refusal("field_not_for_kind", place(i)), d
        r = revision_refusal(inp)
        if r:
            return r, d
        for i, e in found:
            if e["layerId"] in locked_layers:
                return refusal("layer_locked", place(i)), d
        for _, e in found:
            role = exit_(uid_of[e["id"]] in yes)
            if e["kind"] == "opening":
                e["exit"] = role
            else:
                e.setdefault("arch", {})["exit"] = role
        order = {e["id"]: k for k, e in enumerate(d["entities"])}
        out = [uid_of[e["id"]] for _, e in sorted(found, key=lambda x: order[x[1]["id"]])]
    elif kind in ("wallStyle", "openingType"):
        if kind == "wallStyle":
            items, key, path, code = settings["wallStyles"], op["style"], "operation.style", "style_not_found"
        else:
            items, key, path, code = settings["arch"]["openingTypes"], op["type"], "operation.type", \
                "opening_type_not_found"
        at = find(items, key)
        if at is None:
            return refusal(code, path), d
        value = op.get("fireRating")
        if value is not None and declared_bad(value):
            return refusal("invalid_declared", "operation.fireRating"), d
        r = revision_refusal(inp)
        if r:
            return r, d
        put(items[at], "fireRating", value)
        tid = items[at]["id"]
        if kind == "wallStyle":
            mine = [e for e in d["entities"] if e["kind"] == "wall" and e.get("style") == tid
                    and "fireRating" not in (e.get("arch") or {})]
        else:
            mine = [e for e in d["entities"] if e["kind"] == "opening" and e.get("style") == tid
                    and "fireRating" not in e]
        out = [uid_of[e["id"]] for e in mine] if value is not None else []
    elif kind == "material":
        # A field absent stays, null takes it away (ADR 8091); neither: nothing_to_set.
        if "strength" not in op and "reaction" not in op and "roofClass" not in op:
            return refusal("nothing_to_set", "operation"), d
        items = settings["arch"]["materials"]
        at = find(items, op["material"])
        if at is None:
            return refusal("material_not_found", "operation.material"), d
        if op.get("reaction") is not None:
            if declared_bad(op["reaction"]):
                return refusal("invalid_declared", "operation.reaction"), d
            if reaction_bad(op["reaction"]["value"]):
                return refusal("invalid_declared", "operation.reaction.value"), d
        if op.get("roofClass") is not None:
            if declared_bad(op["roofClass"]):
                return refusal("invalid_declared", "operation.roofClass"), d
            if roof_class_bad(op["roofClass"]["value"]):
                return refusal("invalid_declared", "operation.roofClass.value"), d
        if op.get("strength") is not None and strength_bad(op["strength"]):
            return refusal("invalid_strength", "operation.strength"), d
        r = revision_refusal(inp)
        if r:
            return r, d
        for key in ("strength", "reaction", "roofClass"):
            if key in op:
                put(items[at], key, op[key])
        out = []
    else:  # building
        value = op.get("fireProtectionClass")
        if value is not None and declared_bad(value):
            return refusal("invalid_declared", "operation.fireProtectionClass"), d
        r = revision_refusal(inp)
        if r:
            return r, d
        put(settings["arch"], "performance", {"fireProtectionClass": value} if value is not None else None)
        out = []
    result = {"status": "completed", "output": {"uids": out, "revision": "$current"}, "warnings": []}
    return result, after


def state(d, dirty=None, can_undo=None, revision=None):
    """What is held: each object's fire fields (a wall's, a column's … `arch`; an opening's own fields), the styles and
    the architecture settings."""
    fields = {}
    for e in d["entities"]:
        if e["kind"] == "opening":
            fields[str(e["id"])] = {"fireRating": e.get("fireRating"), "exit": e.get("exit")}
        else:
            fields[str(e["id"])] = {"arch": e.get("arch")}
    out = {"ids": [e["id"] for e in d["entities"]], "fields": fields,
           "settings": {"wallStyles": d["settings"]["wallStyles"], "arch": d["settings"]["arch"]}}
    if dirty is not None:
        out["dirty"] = dirty
    if can_undo is not None:
        out["canUndo"] = can_undo
    if revision is not None:
        out["revision"] = revision
    return out


def author(v):
    return {"value": v, "basis": {"kind": "author"}}


def document(v, name, date=None):
    b = {"kind": "document", "name": name}
    if date is not None:
        b["date"] = date
    return {"value": v, "basis": b}


def exit_(yes, basis=None):
    return {"exit": yes, "basis": basis or {"kind": "author"}}


def bearing(yes, basis=None):
    return {"bearing": yes, "basis": basis or {"kind": "author"}}


def cases():
    d = drawing()
    out = []

    def case(name, steps, setup=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    def written(op, setup=None, **extra):
        src = setup or d
        inp = {"operation": op, **extra}
        result, after = run(src, inp)
        assert result["status"] == "completed", (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(after, dirty=True, can_undo=True, revision="changed")},
                {"op": "undo", "returns": STEP, "expect": state(src, revision="changed")},
                {"op": "redo", "returns": STEP, "expect": state(after, revision="changed")}]

    def refused(op, setup=None, **extra):
        src = setup or d
        inp = {"operation": op, **extra}
        result, _ = run(src, inp)
        assert result["status"] != "completed", (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(src, dirty=False, can_undo=False, revision="same")}]

    def objects(uids, **fields):
        return {"kind": "objects", "uids": uids, **fields}

    # A drawing where the wall, the door and the style already declare.
    declared = copy.deepcopy(d)
    for e in declared["entities"]:
        if e["id"] == WALL:
            e["arch"] = {"fireRating": author("REI 60")}
        if e["id"] == DOOR:
            e["fireRating"] = author("EI 30")
    declared["settings"]["wallStyles"][0]["fireRating"] = author("REI 120")

    case("Duvarın kendi sınıfı, müellif beyanıyla", written(objects([U(WALL)], fireRating=author("REI 120"))))
    case("Kapıya belgeli sınıf ve acil çıkış",
         written(objects([U(DOOR)], fireRating=document("EI 60", "Test raporu 2026/41", "2026-11-16"),
                         exit=exit_(True))))
    case("Merdiven acil çıkış değil, sınıfı REI 90",
         written(objects([U(STAIR)], fireRating=author("REI 90"), exit=exit_(False))))
    case("Kolon ve duvar birlikte, çizimin sırasıyla; aynı kimlik bir kez",
         written(objects([U(COLUMN), U(WALL), U(COLUMN)], fireRating=author("R 120"))))
    case("Pencerenin sınıfı: tipi yok, kendi değeri", written(objects([U(WINDOW)], fireRating=author("E 30"))))
    case("Null duvarın kendi sınıfını kaldırır; stilinin sınıfını taşır",
         written(objects([U(WALL)], fireRating=None), setup=declared), setup=declared)
    case("Yalnız acil çıkış verilir, sınıf kalır",
         written(objects([U(DOOR)], exit=exit_(True, {"kind": "document", "name": "Kaçış planı"})), setup=declared),
         setup=declared)
    case("Stili olmayan duvara sınıf", written(objects([U(BARE)], fireRating=author("EI 90"))))
    case("Duvara acil çıkış: alan bu türde yok", refused(objects([U(WALL)], exit=exit_(True))))
    case("Pencereye acil çıkış: yalnız kapı ve merdiven", refused(objects([U(WINDOW)], exit=exit_(True))))
    case("Mahale sınıf: yapı elemanı değil", refused(objects([U(COLUMN), U(ROOM)], fireRating=author("REI 60"))))
    case("Aksa sınıf", refused(objects([U(GRID)], fireRating=author("REI 60"))))
    case("Ne sınıf ne acil çıkış", refused(objects([U(WALL)])))
    case("Boş sınıf yazılmaz", refused(objects([U(WALL)], fireRating=author(""))))
    case("Başında boşluk olan sınıf", refused(objects([U(WALL)], fireRating=author(" REI 60"))))
    case("65 karakterlik sınıf", refused(objects([U(WALL)], fireRating=author("R" * 65))))
    case("Adsız belge", refused(objects([U(WALL)], fireRating=document("REI 60", "  "))))
    case("Gün olmayan tarih", refused(objects([U(WALL)], fireRating=document("REI 60", "Rapor", "2026-02-29"))))
    case("Acil çıkışın belgesi adsız",
         refused(objects([U(DOOR)], exit=exit_(True, {"kind": "document", "name": ""}))))
    case("Kilitli katmandaki duvar", refused(objects([U(WALL), U(LOCKED)], fireRating=author("REI 60"))))
    case("Boş nesne listesi", refused(objects([], fireRating=author("REI 60"))))
    case("Geçersiz kimlik", refused(objects(["duvar"], fireRating=author("REI 60"))))
    case("Çizimde olmayan nesne", refused(objects([U(0xffff)], fireRating=author("REI 60"))))
    case("Stile sınıf: kendi sınıfı olmayan duvarları, kilitlisi de, taşır",
         written({"kind": "wallStyle", "style": "dis-duvar", "fireRating": author("REI 120")}))
    case("Stil adıyla; kendi sınıfı olan duvar sayılmaz",
         written({"kind": "wallStyle", "style": "dış duvar", "fireRating": author("REI 90")},
                 setup=declared), setup=declared)
    case("Stilin sınıfı kaldırılır: taşıyan yok",
         written({"kind": "wallStyle", "style": "dis-duvar", "fireRating": None}, setup=declared), setup=declared)
    case("Olmayan stil", refused({"kind": "wallStyle", "style": "ic-duvar", "fireRating": author("EI 60")}))
    case("Stile boş sınıf", refused({"kind": "wallStyle", "style": "dis-duvar", "fireRating": author("")}))
    case("Kapı tipine sınıf: tipteki kapı taşır",
         written({"kind": "openingType", "type": "k01", "fireRating": author("EI 30")}))
    case("Tip adıyla; kendi sınıfı olan kapı sayılmaz",
         written({"kind": "openingType", "type": "K01", "fireRating": author("EI 60")}, setup=declared),
         setup=declared)
    case("Olmayan tip", refused({"kind": "openingType", "type": "p01", "fireRating": author("EI 30")}))
    # Load-bearing and escape route roles (ADR 8090).
    case("Duvar taşıyıcı, müellif beyanıyla", written(objects([U(WALL)], loadBearing=bearing(True))))
    case("Stilsiz duvar taşıyıcı değil, sınıfıyla birlikte",
         written(objects([U(BARE)], fireRating=author("EI 60"), loadBearing=bearing(False))))
    case("Null duvarın taşıyıcılığını kaldırır", written(objects([U(WALL)], loadBearing=None), setup=bearing_wall(d)),
         setup=bearing_wall(d))
    # A column of the frame carries unless declared not to (ADR 8124; was refused under ADR 8090).
    case("Kolon taşıyıcı değil: beyan çerçevenin okumasını değiştirir",
         written(objects([U(COLUMN)], loadBearing=bearing(False))))
    case("Kapıya taşıyıcılık", refused(objects([U(DOOR)], loadBearing=bearing(True))))
    case("Taşıyıcılığın belgesi adsız",
         refused(objects([U(WALL)], loadBearing=bearing(True, {"kind": "document", "name": " "}))))
    case("Kilitli katmandaki duvara taşıyıcılık", refused(objects([U(LOCKED)], loadBearing=bearing(True))))
    case("Çatı kaçış yolu", written(objects([U(ROOF)], exit=exit_(True))))
    case("Çatıya sınıf ve kaçış yolu birlikte", written(objects([U(ROOF)], fireRating=author("REI 30"), exit=exit_(True))))
    case("Çatıya taşıyıcılık: duvar ve kolon", refused(objects([U(ROOF)], loadBearing=bearing(True))))
    # The drawing's readings changed on the object (ADR 8124): a wall's and a slab's exterior role, a post's bearing.
    more = with_more(d)
    case("Duvar içeride: beyan dış çizginin okumasını değiştirir",
         written(objects([U(WALL)], external={"external": False, "basis": {"kind": "author"}}), setup=more), setup=more)
    case("Döşeme dışarıda, ısı yalıtım projesiyle",
         written(objects([U(SLAB)], external={"external": True, "basis": {"kind": "document", "name": "Isı yalıtım projesi",
                                                                         "date": "2026-11-18"}}), setup=more),
         setup=more)
    case("Null döşemenin beyanını kaldırır: çizimin okuması",
         written(objects([U(SLAB)], external=None), setup=outside_slab(more)), setup=outside_slab(more))
    case("Kapıya dışarıda: duvarı gibidir", refused(objects([U(DOOR)], external={"external": True, "basis": {"kind": "author"}}),
                                                  setup=more), setup=more)
    case("Çatıya dışarıda: çatı hep dışarıda",
         refused(objects([U(ROOF)], external={"external": False, "basis": {"kind": "author"}}), setup=more), setup=more)
    case("Dikmeye taşıyıcılık: çerçevenin kolonu değil",
         refused(objects([U(POST)], loadBearing=bearing(True)), setup=more), setup=more)
    case("Dışarıda beyanının belgesi adsız",
         refused(objects([U(WALL)], external={"external": True, "basis": {"kind": "document", "name": ""}}), setup=more),
         setup=more)
    # Acil çıkışlar: the escape routes' suggestion confirmed (ADR 8124).
    case("Önerileri yaz: kapı ve merdiven evet, ikinci kapı hayır",
         written({"kind": "exits", "yes": [U(STAIR), U(DOOR)], "no": [U(LOCKED_DOOR)]}, setup=unlocked(more)),
         setup=unlocked(more))
    case("Yalnız hayır listesi", written({"kind": "exits", "no": [U(DOOR)]}, setup=more), setup=more)
    case("Beyanı değiştirir: hayır olan merdiven evet",
         written({"kind": "exits", "yes": [U(STAIR)]}, setup=stair_no(more)), setup=stair_no(more))
    case("Hiç nesne yok", refused({"kind": "exits", "yes": [], "no": []}, setup=more), setup=more)
    case("Aynı kapı iki listede", refused({"kind": "exits", "yes": [U(DOOR)], "no": [U(STAIR), U(DOOR)]}, setup=more),
         setup=more)
    case("Pencere çıkış olmaz", refused({"kind": "exits", "yes": [U(DOOR)], "no": [U(WINDOW)]}, setup=more), setup=more)
    case("Çatı kaçış yoludur, acil çıkış listesine girmez", refused({"kind": "exits", "yes": [U(ROOF)]}, setup=more),
         setup=more)
    case("Kilitli katmandaki kapı", refused({"kind": "exits", "no": [U(DOOR), U(LOCKED_DOOR)]}, setup=more), setup=more)
    case("Olmayan kapı", refused({"kind": "exits", "yes": [U(99)]}, setup=more), setup=more)
    case("Betonarmenin dayanımı C30", written({"kind": "material", "material": "betonarme", "strength": "C30"}))
    case("Malzeme adıyla, C30/37", written({"kind": "material", "material": "tuğla", "strength": "C30/37"}))
    case("Dayanım kaldırılır",
         written({"kind": "material", "material": "betonarme", "strength": None},
                 setup=with_strength(d)), setup=with_strength(d))
    case("Boşluklu dayanım", refused({"kind": "material", "material": "betonarme", "strength": "C 30"}))
    # Reaction to fire classes (ADR 8091).
    case("Malzemenin yanıcılık sınıfı A2-s1,d0",
         written({"kind": "material", "material": "tugla", "reaction": author("A2-s1,d0")}))
    case("Döşeme malzemesinin sınıfı Bfl-s1, belgesiyle",
         written({"kind": "material", "material": "betonarme", "reaction": document("Bfl-s1", "Ürün performans beyanı")}))
    case("Sınıf ve dayanım birlikte", written({"kind": "material", "material": "betonarme", "strength": "C30",
                                               "reaction": author("A1")}))
    case("Yalnız sınıf: dayanım kalır",
         written({"kind": "material", "material": "betonarme", "reaction": author("A1")}, setup=with_strength(d)),
         setup=with_strength(d))
    case("Yanıcılık sınıfı kaldırılır",
         written({"kind": "material", "material": "tugla", "reaction": None}, setup=with_reaction(d)),
         setup=with_reaction(d))
    case("Okunmayan sınıf A3", refused({"kind": "material", "material": "tugla", "reaction": author("A3")}))
    case("Damlası olan döşeme sınıfı", refused({"kind": "material", "material": "tugla", "reaction": author("Bfl-s1,d0")}))
    case("Boş sınıf", refused({"kind": "material", "material": "tugla", "reaction": author("")}))
    # A roof covering's external fire performance class (ADR 8134).
    case("Kiremidin dış yangın sınıfı BROOF(t1)",
         written({"kind": "material", "material": "tugla", "roofClass": author("BROOF(t1)")}))
    case("Ek-2/Ç'nin yazdığı gibi yalnız BROOF, belgesiyle",
         written({"kind": "material", "material": "betonarme", "roofClass": document("BROOF", "Ürün performans beyanı")}))
    case("Yanıcılık ve dış yangın sınıfı birlikte",
         written({"kind": "material", "material": "tugla", "reaction": author("A1"), "roofClass": author("BROOF(t4)")}))
    case("Dış yangın sınıfı kaldırılır: yanıcılık sınıfı kalır",
         written({"kind": "material", "material": "tugla", "roofClass": None}, setup=with_roof_class(d)),
         setup=with_roof_class(d))
    case("Okunmayan dış yangın sınıfı Broof", refused({"kind": "material", "material": "tugla",
                                                       "roofClass": author("Broof")}))
    case("Deneme yöntemi t5", refused({"kind": "material", "material": "tugla", "roofClass": author("BROOF(t5)")}))
    case("Boş dış yangın sınıfı", refused({"kind": "material", "material": "tugla", "roofClass": author("")}))
    case("Önce yanıcılık sınıfı okunur", refused({"kind": "material", "material": "tugla", "reaction": author("A3"),
                                                   "roofClass": author("GROOF")}))
    case("Ne dayanım ne sınıf", refused({"kind": "material", "material": "tugla"}))
    case("Alt çizgili dayanım: ada _ ile eklenir", refused({"kind": "material", "material": "betonarme",
                                                            "strength": "C_30"}))
    case("Olmayan malzeme", refused({"kind": "material", "material": "celik", "strength": "S355"}))
    case("Binanın yangın koruma sınıfı",
         written({"kind": "building", "fireProtectionClass": document("Konut, düşük tehlike", "Yangın raporu")}))
    case("Binanın sınıfı kaldırılır",
         written({"kind": "building", "fireProtectionClass": None}, setup=with_class(d)), setup=with_class(d))
    case("Binaya boş sınıf", refused({"kind": "building", "fireProtectionClass": author(" ")}))
    case("Geçersiz sürüm", refused(objects([U(WALL)], fireRating=author("REI 60")), expectedRevision="01"))
    case("Eski sürüm: hiçbir şey yazılmaz",
         refused({"kind": "material", "material": "betonarme", "strength": "C30"}, expectedRevision="9999"))
    return out


def with_more(d):
    """The drawing with a floor slab, a post and a door on the locked layer (ADR 8124)."""
    d = copy.deepcopy(d)
    more = {
        SLAB: {"kind": "slab", **base(SLAB, "mimari"),
               "parts": [{"outer": {"pts": [P(50, 0), P(54, 0), P(54, 3), P(50, 3)]}}], "thickness": 0.15},
        POST: {"kind": "column", **base(POST, "mimari"), "at": P(22, 0), "width": 0.1, "depth": 0.1,
               "member": {"predefined": "POST"}},
        LOCKED_DOOR: {"kind": "opening", **base(LOCKED_DOOR, "kilitli"), "host": U(LOCKED), "at": 1.5, "width": 0.9,
                      "form": "door", "head": 2.1,
                      "place": [P(1.05, 10.125), P(1.05, 9.875), P(1.95, 10.125), P(1.95, 9.875)]},
    }
    d["entities"] += [more[i] for i in sorted(more)]
    d["uids"] += [U(i) for i in sorted(more)]
    return d


def outside_slab(d):
    d = copy.deepcopy(d)
    for e in d["entities"]:
        if e["id"] == SLAB:
            e["arch"] = {"external": {"external": True, "basis": {"kind": "author"}}}
    return d


def unlocked(d):
    d = copy.deepcopy(d)
    for l in d["layers"]:
        l["locked"] = False
    return d


def stair_no(d):
    d = copy.deepcopy(d)
    for e in d["entities"]:
        if e["id"] == STAIR:
            e["arch"] = {"exit": exit_(False)}
    return d


def bearing_wall(d):
    d = copy.deepcopy(d)
    for e in d["entities"]:
        if e["id"] == WALL:
            e["arch"] = {"loadBearing": bearing(True)}
    return d


def with_strength(d):
    d = copy.deepcopy(d)
    d["settings"]["arch"]["materials"][1]["strength"] = "C25"
    return d


def with_reaction(d):
    d = copy.deepcopy(d)
    d["settings"]["arch"]["materials"][0]["reaction"] = author("A2-s1,d0")
    return d


def with_roof_class(d):
    d = copy.deepcopy(d)
    d["settings"]["arch"]["materials"][0]["reaction"] = author("A1")
    d["settings"]["arch"]["materials"][0]["roofClass"] = author("BROOF(t1)")
    return d


def with_class(d):
    d = copy.deepcopy(d)
    d["settings"]["arch"]["performance"] = {"fireProtectionClass": author("Konut")}
    return d


def document_of_cases():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Yangın ve performans: nesnenin sınıfı ve acil çıkışı, stilin ve tipin sınıfı, malzemenin "
                     "dayanımı, binanın yangın koruma sınıfı; retler",
            "note": "ADR 8068. Beklenenler scripts/fixtures/arch_performance_cases.py'nin ADR'den yazılmış "
                    "kurallarından, KentOS kodu olmadan.",
            "setup": drawing(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document_of_cases()
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
