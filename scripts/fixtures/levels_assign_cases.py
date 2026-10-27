#!/usr/bin/env python3
"""cad.levels.assign v1 (“Kat ver”, docs/adr/0717): the shared cases, written from the ADR without KentOS code.

The drawing: a project with two levels (Zemin kat at 0, 1. kat at 3 m). A closed 6 × 4 m wall on no level with a
door and its tag, a room seeded inside it on no level (its boundary the wall's inner face, 0.1 m in), a column and a
stair on no level, a ramp on a level the project does not have (kat-9), a grid on no level, a room separator on no
level, a wall on 1. kat, a wall on no level on a locked layer, a line and a plain polyline.

The rules (ADR 0717 §1–§2):
- Without `uids`: every wall, space, column, stair, ramp and room separator on no level or on one the project lacks;
  a grid may stand on none. With `uids`: those (a grid too); an opening, a tag, a line or a plain polyline has no
  level of its own (not_levelled).
- An object on the level already is left; one on a locked layer is left out and counted (layer_locked warning), all
  of them locked is refused; none left to change: nothing_to_assign.
- Refusals in order: no_entities, invalid_uid, entity_not_found, not_levelled, level_not_found, invalid_revision,
  revision_conflict, layer_locked, nothing_to_assign.
- A room is found again on its level's walls (a room on no level looks at every level's walls, ADR 0641): one its
  new level's walls do not enclose keeps its boundary, broken lostEnclosure, and the step warns arch_space_broken.
- One undo step “Kat ver”; the answer lists the objects in the drawing's order: uid, id, kind, the level they were on.
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.levels.assign.json"
COMMAND = "cad.levels.assign"
STEP = "Kat ver"
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]


def U(n):
    return f"0192f5a1-7170-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def base(id, layer_id):
    return {"id": id, "layerId": layer_id, "attrs": {}}


# The objects, by their ids (their place in the file).
WALL, DOOR, TAG, ROOM, COLUMN, STAIR, RAMP, GRID, SEPARATOR, UPPER, LOCKED, LINE, PLAIN = range(1, 14)
KIND = {WALL: "wall", DOOR: "opening", TAG: "tag", ROOM: "space", COLUMN: "column", STAIR: "stair", RAMP: "ramp",
        GRID: "grid", SEPARATOR: "polyline", UPPER: "wall", LOCKED: "wall", LINE: "line", PLAIN: "polyline"}


def door_place():
    """The door's jambs on the wall's first edge (0,0)→(6,0), 0.9 m wide about 2 m: the left face first (ADR 0341)."""
    return [P(1.55, 0.1), P(1.55, -0.1), P(2.45, 0.1), P(2.45, -0.1)]


def drawing(levels=True):
    ents = {
        WALL: {"kind": "wall", **base(WALL, "duvar"), "pts": [P(0, 0), P(6, 0), P(6, 4), P(0, 4)], "width": 0.2,
               "justify": "center", "closed": True},
        DOOR: {"kind": "opening", **base(DOOR, "mimari"), "host": U(WALL), "at": 2.0, "width": 0.9, "form": "door",
               "place": door_place(), "number": "K1"},
        # The tag's anchor: the door's middle on its wall's path, as the document writes it (ADR 0344 §1).
        TAG: {"kind": "tag", **base(TAG, "mimari"), "of": U(DOOR), "at": P(2, -0.8), "text": "K1", "anchor": P(2, 0)},
        ROOM: {"kind": "space", **base(ROOM, "mimari"),
               "parts": [{"outer": {"pts": [P(0.1, 0.1), P(5.9, 0.1), P(5.9, 3.9), P(0.1, 3.9)]}}],
               "seed": P(3, 2), "name": "Oda"},
        COLUMN: {"kind": "column", **base(COLUMN, "mimari"), "at": P(10, 0), "width": 0.4, "depth": 0.4},
        STAIR: {"kind": "stair", **base(STAIR, "mimari"), "pts": [P(12, 0), P(15.75, 0)], "width": 1.2, "risers": 16,
                "riserHeight": 0.175, "tread": 0.25},
        RAMP: {"kind": "ramp", **base(RAMP, "mimari"), "pts": [P(12, 5), P(22, 5)], "width": 1.5, "rise": 0.5,
               "slope": 5.0, "arch": {"level": "kat-9"}},
        GRID: {"kind": "grid", **base(GRID, "mimari"), "pts": [P(-2, -2), P(-2, 6)], "name": "A"},
        SEPARATOR: {"kind": "polyline", **base(SEPARATOR, "mimari"), "lineType": "dashed",
                    "pts": [P(40, 0), P(40, 4)], "spaceSeparator": {}},
        UPPER: {"kind": "wall", **base(UPPER, "duvar"), "pts": [P(0, 10), P(6, 10)], "width": 0.2,
                "justify": "center", "arch": {"level": "kat-1"}},
        LOCKED: {"kind": "wall", **base(LOCKED, "kilitli"), "pts": [P(30, 0), P(34, 0)], "width": 0.2,
                 "justify": "center"},
        LINE: {"kind": "line", **base(LINE, "cizim"), "a": P(0, 20), "b": P(5, 20)},
        PLAIN: {"kind": "polyline", **base(PLAIN, "cizim"), "pts": [P(0, 22), P(5, 22), P(5, 24)]},
    }
    settings = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 50.0, "workspace": "cad"}
    if levels:
        settings["arch"] = {"levels": copy.deepcopy(LEVELS)}
    return {
        "format": "kentos.document", "version": 2, "name": "Kat ver",
        "settings": settings,
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("duvar", "Duvar"), layer("mimari", "Mimari"), layer("cizim", "Çizim"),
                   layer("kilitli", "Kilitli", locked=True)],
        "activeLayer": "duvar",
        "entities": [ents[i] for i in sorted(ents)],
        "uids": [U(i) for i in sorted(ents)],
        "styles": {"items": [], "categories": []},
    }


ALL_IDS = list(range(1, 14))


def level_of(e):
    if e["kind"] == "polyline":
        return (e.get("spaceSeparator") or {}).get("level")
    return (e.get("arch") or {}).get("level")


def levelled(e):
    return e["kind"] in ("wall", "space", "grid", "column", "stair", "ramp") or (
        e["kind"] == "polyline" and "spaceSeparator" in e)


def lacks(e, levels):
    return bool(levels) and levelled(e) and e["kind"] != "grid" and level_of(e) not in levels


def given(e, level):
    """`e` on `level`: its arch's (a separator's spaceSeparator's) level written, its other fields kept."""
    e = copy.deepcopy(e)
    if e["kind"] == "polyline":
        e["spaceSeparator"] = dict(e["spaceSeparator"], level=level)
    else:
        e["arch"] = dict(e.get("arch") or {}, level=level)
    return e


def room_encloses(d, room):
    """Whether the room's level's walls close round its seed: the room on no level looks at every wall (ADR 0641),
    a room on a level only at that level's. In this drawing only the closed wall WALL can enclose the seed."""
    wall = next(e for e in d["entities"] if e["id"] == WALL)
    return level_of(room) is None or level_of(wall) == level_of(room)


# ── The command, as the ADR says it ──────────────────────────────────────────────────────────────────────────


def refusal(code, path, status="failed"):
    err = {"code": code, "message": "$any"}
    if path is not None:
        err["path"] = path
    out = {"status": status, "error": err}
    if status == "conflict":
        out["error"]["revision"] = "$current"
    return out


def run(d, inp):
    """(result, drawing after) for `inp` on `d`; the drawing unchanged on a refusal."""
    levels = [l["id"] for l in ((d["settings"].get("arch") or {}).get("levels") or [])]
    by_id = {e["id"]: e for e in d["entities"]}
    uid_of = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    id_of = {u: i for i, u in uid_of.items()}
    locked_layers = {l["id"] for l in d["layers"] if l["locked"]}
    if "uids" in inp:
        uids = inp["uids"]
        if not uids:
            return refusal("no_entities", "uids"), d
        for i, u in enumerate(uids):
            if not (isinstance(u, str) and len(u) == 36 and u == u.lower() and u.count("-") == 4):
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
            if not levelled(e):
                return refusal("not_levelled", f"uids[{i}]"), d
        candidates = found
    else:
        candidates = [(None, e) for e in d["entities"] if lacks(e, levels)]
    if inp["level"] not in levels:
        return refusal("level_not_found", "level"), d
    rev = inp.get("expectedRevision")
    if rev is not None:
        if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
            return refusal("invalid_revision", "expectedRevision"), d
        return refusal("revision_conflict", "expectedRevision", "conflict"), d
    locked = [(i, e) for i, e in candidates if e["layerId"] in locked_layers]
    open_ = [(i, e) for i, e in candidates if e["layerId"] not in locked_layers]
    if not open_ and locked:
        i = locked[0][0]
        return refusal("layer_locked", None if i is None else f"uids[{i}]"), d
    warnings = []
    if locked:
        w = {"code": "layer_locked", "message": f"{len(locked)} nesne kilitli katmanda olduğu için atlandı. Kat vermek "
                                                 f"için katmanın kilidini Katmanlar panelinden açın."}
        if "uids" in inp:
            w["path"] = "uids"
        warnings.append(w)
    level = inp["level"]
    changed = [e for _, e in open_ if level_of(e) != level]
    if not changed:
        return refusal("nothing_to_assign", "uids" if "uids" in inp else None), d
    after = copy.deepcopy(d)
    new = {e["id"]: given(e, level) for e in changed}
    after["entities"] = [new.get(e["id"], e) for e in after["entities"]]
    # The rooms the step left unenclosed: broken now, not before.
    lost = 0
    for k, e in enumerate(after["entities"]):
        if e["kind"] != "space":
            continue
        was_broken = "broken" in by_id[e["id"]]
        if not room_encloses(after, e):
            if not was_broken:
                lost += 1
            after["entities"][k] = dict(e, broken="lostEnclosure")
    if lost:
        warnings.append({"code": "arch_space_broken",
                         "message": f"{lost} mahalın sınırı yeni katında duvarlarla kapanmıyor; son sınırıyla kaldı. "
                                    f"Duvarlarını da aynı kata verin."})
    # Drawing order: the order the objects stand in the file.
    order = {e["id"]: k for k, e in enumerate(d["entities"])}
    assigned = []
    for e in sorted(changed, key=lambda e: order[e["id"]]):
        a = {"uid": uid_of[e["id"]], "id": e["id"], "kind": e["kind"]}
        if level_of(e) is not None:
            a["from"] = level_of(e)
        assigned.append(a)
    result = {"status": "completed", "output": {"level": level, "assigned": assigned, "revision": "$current"},
              "warnings": warnings}
    return result, after


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────


def state(d, changed_ids=(), dirty=None, can_undo=None, revision=None):
    """What the drawing holds: every object; a room by its level and break only (its boundary is found again)."""
    entities, fields = {}, {}
    for e in d["entities"]:
        if e["kind"] == "space":
            fields[str(e["id"])] = {"arch": e.get("arch"), "broken": e.get("broken")}
        else:
            entities[str(e["id"])] = e
    out = {"ids": [e["id"] for e in d["entities"]], "entities": entities, "fields": fields}
    if dirty is not None:
        out["dirty"] = dirty
    if can_undo is not None:
        out["canUndo"] = can_undo
    if revision is not None:
        out["revision"] = revision
    return out


def cases():
    d = drawing()
    out = []

    def case(name, steps, setup=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    def written(inp, setup=None, undo=True):
        """Execute, then undo and redo: one step “Kat ver”."""
        src = setup or d
        result, after = run(src, inp)
        assert result["status"] == "completed", (inp, result)
        # A door's jambs are placed again as its wall changes: within 1e-9 of the reference's (ADR 0341 §3).
        steps = [{"op": "execute", "input": inp, "result": result, "within": 1e-9,
                  "expect": state(after, dirty=True, can_undo=True, revision="changed")}]
        if undo:
            steps += [{"op": "undo", "returns": STEP, "within": 1e-9, "expect": state(src, revision="changed")},
                      {"op": "redo", "returns": STEP, "within": 1e-9, "expect": state(after, revision="changed")}]
        return steps

    def refused(inp, setup=None):
        src = setup or d
        result, _ = run(src, inp)
        assert result["status"] != "completed", (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(src, dirty=False, can_undo=False, revision="same")}]

    # Without objects named: every object without a level of the project.
    case("Katsızlar zemin kata: duvar, mahal, kolon, merdiven, olmayan kattaki rampa ve ayırıcı; aks, kapı, etiket, "
         "çizgi değişmez, kilitli duvar sayılır", written({"level": "zemin"}))
    case("Katsızlar 1. kata: mahal duvarıyla aynı katta kapanır", written({"level": "kat-1"}))
    result, _ = run(d, {"level": "zemin"})
    case("Plan aynı cevabı verir, çizim değişmez",
         [{"op": "plan", "input": {"level": "zemin"}, "result": result, "expect": state(d, revision="same")}])
    case("Doğrulama: uyarılarla geçer",
         [{"op": "validate", "input": {"level": "zemin"},
           "result": {"status": "completed", "output": None, "warnings": result["warnings"]},
           "expect": state(d, revision="same")}])
    clean, _ = run(d, {"level": "zemin"})
    _, all_on = run(d, {"level": "zemin"})
    # Every object on a level already (the locked wall too: given one in the file).
    tidy = copy.deepcopy(all_on)
    for k, e in enumerate(tidy["entities"]):
        if e["id"] == LOCKED:
            tidy["entities"][k] = given(e, "zemin")
    case("Katsız nesne kalmadı: değişecek nesne yok", refused({"level": "zemin"}, tidy), setup=tidy)
    only_locked = copy.deepcopy(tidy)
    for k, e in enumerate(only_locked["entities"]):
        if e["id"] == LOCKED:
            only_locked["entities"][k] = dict(e)
            only_locked["entities"][k].pop("arch")
    case("Yalnız kilitli katmandaki duvar katsız: reddedilir", refused({"level": "zemin"}, only_locked),
         setup=only_locked)
    flat = drawing(levels=False)
    case("Projede kat yok: kat bulunamaz", refused({"level": "zemin"}, flat), setup=flat)

    # Objects named.
    case("Aks açıkça verilince kat alır", written({"level": "zemin", "uids": [U(GRID)]}))
    case("1. kattaki duvar zemin kata taşınır", written({"level": "zemin", "uids": [U(UPPER)]}))
    case("Mahal tek başına 1. kata: duvarları orada değil, sınırı son hâliyle kalır",
         written({"level": "kat-1", "uids": [U(ROOM)]}))
    case("Duvar tek başına 1. kata: katsız mahal bütün katların duvarlarına bakar, kapanır",
         written({"level": "kat-1", "uids": [U(WALL)]}))
    case("Mahal ve duvarı birlikte 1. kata: mahal kapanır",
         written({"level": "kat-1", "uids": [U(WALL), U(ROOM)]}))
    # The room on zemin first, then its wall moved away: the room loses its walls.
    on_zemin = copy.deepcopy(d)
    for k, e in enumerate(on_zemin["entities"]):
        if e["id"] in (WALL, ROOM):
            on_zemin["entities"][k] = given(e, "zemin")
    case("Zemin kattaki mahalın duvarı 1. kata: mahal kapanmaz, uyarılır",
         written({"level": "kat-1", "uids": [U(WALL)]}, on_zemin), setup=on_zemin)
    case("Mahal ayırıcı açıkça verilir", written({"level": "kat-1", "uids": [U(SEPARATOR)]}))
    case("Olmayan kattaki rampa", written({"level": "kat-1", "uids": [U(RAMP)]}))
    case("Zaten o kattaki duvar: değişecek nesne yok", refused({"level": "kat-1", "uids": [U(UPPER)]}))
    case("Zaten o kattaki duvar ile katsız kolon: yalnız kolon",
         written({"level": "kat-1", "uids": [U(UPPER), U(COLUMN)]}))
    case("Aynı kimlik iki kez: bir kez", written({"level": "zemin", "uids": [U(COLUMN), U(COLUMN)]}))
    case("Kilitli ile açık: kilitli atlanır, sayılır", written({"level": "zemin", "uids": [U(LOCKED), U(STAIR)]}))
    case("Yalnız kilitli: reddedilir", refused({"level": "zemin", "uids": [U(LOCKED)]}))
    for who, name in ((DOOR, "Kapı"), (TAG, "Etiket"), (LINE, "Çizgi"), (PLAIN, "Mahal ayırıcı olmayan çoklu çizgi")):
        case(f"{name}: kendi katı yok",
             refused({"level": "zemin", "uids": [U(COLUMN), U(who)]}))
    case("Katı olmayan nesne olmayan kattan önce söylenir", refused({"level": "yok", "uids": [U(DOOR)]}))
    case("Boş nesne listesi", refused({"level": "zemin", "uids": []}))
    case("Geçersiz kimlik", refused({"level": "zemin", "uids": ["kat"]}))
    case("Çizimde olmayan nesne", refused({"level": "zemin", "uids": [U(0xffff)]}))
    case("Projede olmayan kat", refused({"level": "kat-9", "uids": [U(COLUMN)]}))
    case("Projede olmayan kat, nesnesiz", refused({"level": "çatı"}))
    case("Geçersiz sürüm", refused({"level": "zemin", "expectedRevision": "01"}))
    case("Eski sürüm: hiçbir şey yazılmaz", refused({"level": "zemin", "expectedRevision": "9999"}))
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Kat ver: katsız mimari nesnelere ya da verilen nesnelere projenin katı; mahalın yeniden bulunması, "
                     "kilitli katman, retler",
            "note": "ADR 0717. Beklenenler scripts/fixtures/levels_assign_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan; mahalın sınırı yeniden bulunduğu için yalnız katı ve kopukluğu denetlenir.",
            "setup": drawing(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document()
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
