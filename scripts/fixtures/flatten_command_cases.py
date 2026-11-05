#!/usr/bin/env python3
"""The shared cases of `cad.entities.flatten` (2B düzleme indir; docs/adr/1061 §4; plan/cad-advanced.md CAD-ADV-17).

    python3 scripts/fixtures/flatten_command_cases.py           # writes the file
    python3 scripts/fixtures/flatten_command_cases.py --check   # writes nothing; compares

Written from ADR 1061 §4 with Python's standard library, no KentOS code:

- the elevations of points (and their extra points), lines (za, zb), polylines and areas (zs, their parts' and holes'
  zs) are removed, or with `plane` each becomes the plane's; a vertex without one stays without; an all-null list is
  dropped;
- `measures` removes the measures (line ma, mb; a polyline's and its parts' ms); without it they stay;
- counted: the vertices whose elevation went or changed (one already at the plane is not), their lowest and highest
  elevation, the measures removed; the objects changed in the input's order;
- other kinds are passed over; nothing to change is `nothing_to_flatten`; locked layers are left out (`layer_locked`
  warning; all locked: refused), one undo step “2B düzleme indir”.
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.flatten.json"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def obj(kind, i, layer_id="cizim", **f):
    return {"kind": kind, "id": i, "layerId": layer_id, "attrs": {}, **f}


ENTITIES = [
    obj("point", 1, p=P(0, 0), z=12.5),
    obj("point", 2, p=P(1, 0), z=3.0, parts=[{"p": P(2, 0), "z": 4.0}, {"p": P(3, 0)}]),
    obj("line", 3, a=P(0, 0), b=P(10, 0), za=5.0, zb=7.25, ma=0.0, mb=10.0),
    obj("polyline", 4, pts=[P(0, 0), P(5, 0), P(5, 5)], zs=[1.0, None, 2.5], ms=[0.0, 5.0, 10.0]),
    obj("polygon", 5, pts=[P(0, 0), P(8, 0), P(8, 8), P(0, 8)], zs=[2.0, 2.0, 2.0, 2.0],
        holes=[{"pts": [P(2, 2), P(4, 2), P(4, 4)], "zs": [9.0, None, 9.0]}]),
    obj("polyline", 6, pts=[P(0, 0), P(1, 1)], parts=[{"pts": [P(5, 5), P(6, 6)], "zs": [-1.5, -2.0], "ms": [3.0, 4.0]}]),
    obj("line", 7, a=P(0, 0), b=P(1, 0)),
    obj("circle", 8, c=P(0, 0), r=1.0),
    obj("line", 9, layer_id="kilitli", a=P(0, 0), b=P(1, 0), za=1.0, zb=1.0),
    obj("line", 10, a=P(0, 0), b=P(1, 0), ma=1.0, mb=2.0),
    obj("point", 11, p=P(5, 5), z=0.0),
]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Düzlem",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 100, "workspace": "cad"},
    "origin": P(0, 0),
    "layers": [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "cizim", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
BY_ID = {e["id"]: e for e in ENTITIES}


class Tally:
    def __init__(self):
        self.vertices, self.measures, self.zs = 0, 0, []

    def z(self, z, plane):
        if z is None:
            return None
        if plane != z:
            self.vertices += 1
            self.zs.append(z)
        return plane

    def zs_(self, zs, plane):
        out = [self.z(z, plane) for z in zs]
        return out if any(x is not None for x in out) else None


def set_or_drop(d, key, value):
    if value is None:
        d.pop(key, None)
    else:
        d[key] = value


def flatten(e, plane, measures, t):
    """ADR 1061 §4 on one object; None when nothing changes."""
    out = copy.deepcopy(e)
    if e["kind"] == "point":
        set_or_drop(out, "z", t.z(e.get("z"), plane))
        for part in out.get("parts", []):
            set_or_drop(part, "z", t.z(part.get("z"), plane))
    elif e["kind"] == "line":
        set_or_drop(out, "za", t.z(e.get("za"), plane))
        set_or_drop(out, "zb", t.z(e.get("zb"), plane))
        if measures:
            for k in ("ma", "mb"):
                if out.pop(k, None) is not None:
                    t.measures += 1
    elif e["kind"] in ("polyline", "polygon"):
        def path(d):
            if "zs" in d:
                set_or_drop(d, "zs", t.zs_(d["zs"], plane))
            if measures and "ms" in d:
                t.measures += sum(1 for m in d.pop("ms") if m is not None)
            for h in d.get("holes", []):
                if "zs" in h:
                    set_or_drop(h, "zs", t.zs_(h["zs"], plane))
        path(out)
        for part in out.get("parts", []):
            path(part)
    else:
        return None
    return out if out != e else None


def plan(uids, plane=None, measures=False):
    """The objects changed (slot, object) and the answer's counts; locked ones left out."""
    t, changed, locked = Tally(), [], 0
    for i in dict.fromkeys(uids):
        e = BY_ID[i]
        if e["layerId"] == "kilitli":
            locked += 1
            continue
        f = flatten(e, plane, measures, t)
        if f is not None:
            changed.append((i, f))
    out = {"changed": [f"$uidOf:{i}" for i, _ in changed], "ids": [i for i, _ in changed], "vertices": t.vertices,
           "measures": t.measures, "revision": "$current"}
    if t.zs:
        out["zMin"], out["zMax"] = min(t.zs), max(t.zs)
    return changed, out, locked


def locked_warning(n):
    return {"code": "layer_locked", "message": "$any", "path": "$any"}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


cases = []


def add(name, steps, note=None):
    c = {"name": name, "steps": steps}
    if note:
        c["note"] = note
    cases.append(c)


def ok_case(name, ids, plane=None, measures=False, note=None, undo=True, op="execute"):
    changed, out, locked = plan(ids, plane, measures)
    inp = {"uids": [f"$uidOf:{i}" for i in ids]}
    if plane is not None:
        inp["plane"] = plane
    if measures:
        inp["measures"] = True
    warns = [locked_warning(locked)] if locked else []
    expect = {"entities": {str(i): f for i, f in changed}}
    if op == "execute":
        expect.update({"canUndo": True, "revision": "changed"})
    else:
        expect = {"entities": {str(i): BY_ID[i] for i, _ in changed}, "canUndo": False, "revision": "same"}
    steps = [{"op": op, "input": inp, "result": done(out, warns), "expect": expect}]
    if undo and op == "execute":
        steps.append({"op": "undo", "returns": "2B düzleme indir",
                      "expect": {"entities": {str(i): BY_ID[i] for i, _ in changed}}})
    add(name, steps, note)


ok_case("nokta: kotu kaldırılır, tek adımda geri alınır", [1])
ok_case("çok noktalı nokta: ilk ve ek noktaların kotları; kotsuz ek nokta kotsuz kalır", [2])
ok_case("çizgi: iki ucun kotu gider, ölçüler kalır", [3])
ok_case("çizgi: measures ile ölçüler de gider", [3], measures=True)
ok_case("çoklu çizgi: kotsuz köşe kotsuz kalır, ölçüler kalır", [4])
ok_case("çoklu çizgi: measures ile ölçüler de gider", [4], measures=True)
ok_case("alan: dış halkanın ve deliğin kotları gider", [5])
ok_case("çok parçalı çoklu çizgi: parçanın kotları ve ölçüleri", [6], measures=True)
ok_case("düzlem verilince her kot düzlemin kotu olur", [3, 4, 5], plane=2.0,
        note="5'in kotları zaten 2: değişmez, sayılmaz; deliğin 9'ları 2 olur.")
ok_case("düzlem 0: kotu zaten 0 olan nokta değişmez, öbürleri 0 olur", [1, 11], plane=0.0)
ok_case("birkaç nesne birlikte: en küçük ve en büyük kot", [1, 2, 3, 4, 6])
ok_case("kotsuz nesne ve başka tür atlanır, kotlu olan değişir", [7, 8, 1])
ok_case("kilitli katmandaki atlanır ve söylenir", [9, 3])
ok_case("yalnız ölçüsü olan çizgi: measures ile ölçüleri gider", [10], measures=True)
ok_case("plan önizlemedir: yazmaz", [3, 4], op="plan")
ok_case("tekrarlanan kimlik bir kez sayılır", [1, 1])
add("ölçüleri kalan çizgide kaldırılacak kot yok", [
    {"op": "execute", "input": {"uids": ["$uidOf:10"]},
     "result": failed("nothing_to_flatten", "Seçilen nesnelerde kaldırılacak ya da değişecek kot (Z) yok; çizim zaten düzlemde. Ölçüleri (M) de kaldırmak için measures: true verin.", "uids"),
     "expect": {"canUndo": False, "revision": "same"}}])
add("kotsuz ve ölçüsüz nesneler: değişecek bir şey yok", [
    {"op": "execute", "input": {"uids": ["$uidOf:7", "$uidOf:8"], "measures": True},
     "result": failed("nothing_to_flatten", "Seçilen nesnelerde kaldırılacak kot (Z) ya da ölçü (M) yok; çizim zaten düzlemde.", "uids"),
     "expect": {"canUndo": False, "revision": "same"}}])
add("düzlemdeki nokta düzleme indirilmez", [
    {"op": "execute", "input": {"uids": ["$uidOf:11"], "plane": 0.0},
     "result": failed("nothing_to_flatten", "$any", "uids"), "expect": {"canUndo": False, "revision": "same"}}])
add("hepsi kilitli katmanda: ret", [
    {"op": "execute", "input": {"uids": ["$uidOf:9"]},
     "result": failed("layer_locked", "$any", "$any"), "expect": {"canUndo": False, "revision": "same"}}])
add("nesne verilmedi", [
    {"op": "execute", "input": {"uids": []}, "result": failed("no_entities", "$any", "uids"),
     "expect": {"canUndo": False, "revision": "same"}}])
add("kimlik UUID değil", [
    {"op": "execute", "input": {"uids": ["nokta"]}, "result": failed("invalid_uid", "$any", "$any"),
     "expect": {"canUndo": False, "revision": "same"}}])
add("olmayan nesne", [
    {"op": "execute", "input": {"uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]},
     "result": failed("entity_not_found", "$any", "$any"), "expect": {"canUndo": False, "revision": "same"}}])
add("düzlemin kotu sonsuz", [
    {"op": "execute", "input": {"uids": ["$uidOf:1"], "plane": 0.0}, "nonFinite": {"plane": "Infinity"},
     "result": failed("not_finite", "Düzlemin kotu sayı değil ya da sonsuz.", "plane"),
     "expect": {"canUndo": False, "revision": "same"}}])
add("geçersiz beklenen sürüm", [
    {"op": "execute", "input": {"uids": ["$uidOf:1"], "expectedRevision": "dün"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": {"canUndo": False, "revision": "same"}}])
add("beklenen sürüm tutmazsa çakışma", [
    {"op": "execute", "input": {"uids": ["$uidOf:1"], "expectedRevision": "999"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision",
                                                "revision": "$current"}},
     "expect": {"canUndo": False, "revision": "same"}}])
add("doğrulama yazmaz", [
    {"op": "validate", "input": {"uids": ["$uidOf:1"]}, "result": done(None), "expect": {"canUndo": False, "revision": "same"}}])


def resolved(v):
    if isinstance(v, list):
        return [resolved(i) for i in v]
    if isinstance(v, dict):
        return {k: resolved(i) for k, i in v.items() if i is not None or k == "output"}
    return v


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.flatten", "commandVersion": 1,
           "title": "2B düzleme indir", "note": "Written by scripts/fixtures/flatten_command_cases.py from docs/adr/1061 §4; do not edit by hand.",
           "setup": SETUP, "cases": resolved(cases)}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


main()
