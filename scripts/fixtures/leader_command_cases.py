"""Writes the shared cases of `cad.leaders.edit` v1 (docs/adr/0326): fixtures/commands/v1/cad.leaders.edit.json.

The rules are the ADR's, written here again without KentOS code:

- a leader's first arm is its `pts` (its tip the first vertex, the landing's start the last); each other arm runs from
  its own tip through its bends to that last vertex;
- the landing runs from the last vertex 2h along side·u, u the note's direction (its rotation), side +1 when the last
  segment that has a length points along u (projection 0 or more) and −1 otherwise (docs/adr/0146 §2); a leader with
  a note or content blocks has one;
- content blocks stand in a row from the landing's end: a block's definition box about its base point [x₀, x₁] ×
  [y₀, y₁] (a line's ends, a circle's centre ± r), scaled; on the right its x₀ side at the end plus what the boxes
  before take, on the left its x₁ side; its box's middle on the landing's line; turned as the note;
- Hizala keeps the first leader; each other's last vertex goes to P₀ + t·d (P₀ the first's last vertex, d the given
  direction, t its own projection on d), or with a spacing s those ahead (t ≥ 0) at s, 2s … in order of t and those
  behind at −s, −2s … nearest first;
- Topla: the others' first arms (their last vertex left out) and other arms become the first's arms, their blocks
  follow its own; the others go, their blocks stay;
- a bound tip stands on its place's point and follows it; its object gone, it keeps its point, broken `object_gone`.

The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/leader_command_cases.py           # writes the file
    python3 scripts/fixtures/leader_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.leaders.edit.json"
X, Y = 500000, 4400000
FAKE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3fff"
NOT_UID = "K-1"
BALLOON = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3b01"
STYLE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3c01"
M_UID = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
M_NOT_FOUND = "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
M_NOT_LEADER = "Verilen nesne bir kılavuz değil; bu komut yalnız kılavuzları değiştirir."
M_LOCKED = "“Kilitli” katmanı kilitli; üzerindeki kılavuz değişmez. Kilidi Katmanlar panelinden açın."
M_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
M_CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def pt(x, y):
    return {"x": X + x, "y": Y + y}


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def leader(i, pts, text=None, layer_id="kilavuz", h=0.5):
    e = {"kind": "leader", "id": i, "layerId": layer_id, "attrs": {}, "pts": [pt(*p) for p in pts], "height": h, "rotation": 0}
    if text is not None:
        e["text"] = text
    return e


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "mekan", "attrs": {}, "pts": [pt(0, 0), pt(6, 0), pt(6, 4), pt(0, 4)]},
    {"kind": "point", "id": 2, "layerId": "mekan", "attrs": {}, "p": pt(10, 2)},
    {"kind": "line", "id": 3, "layerId": "mekan", "attrs": {}, "a": pt(20, 0), "b": pt(24, 3)},
    leader(4, [(2, 2), (5, 6)], "Salon"),
    leader(5, [(10, 2), (12, 8)], "Nokta"),
    leader(6, [(20, 0), (22, 5)], "Hat"),
    leader(7, [(30, 10), (32, 12)], "Kilitli", "kilitli"),
    {"kind": "insert", "id": 8, "layerId": "kilavuz", "attrs": {"NO": "1"}, "block": BALLOON, "p": pt(30, 0), "scale": 1, "rotation": 0},
    {"kind": "insert", "id": 9, "layerId": "kilavuz", "attrs": {"NO": "2"}, "block": BALLOON, "p": pt(35, 0), "scale": 1, "rotation": 0},
    leader(10, [(30, -5), (33, -2)]),
    leader(11, [(40, -5), (43, -2)]),
]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kılavuzlar",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500,
                 "workspace": "cad",
                 "leaderStyles": [{"id": STYLE, "name": "Not", "height": 2.5, "arrow": "dot", "arrowSize": 1.5, "mask": True}]},
    "origin": {"x": X, "y": Y},
    "layers": [layer("mekan", "Mekanlar"), layer("kilavuz", "Kılavuzlar"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "kilavuz",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
    "blocks": [{"id": BALLOON, "name": "Balon", "base": {"x": 0, "y": 0},
                "entities": [{"kind": "circle", "id": 1, "layerId": "", "attrs": {}, "c": {"x": 0, "y": 0}, "r": 0.5}],
                "attributes": [{"tag": "NO", "p": {"x": 0, "y": 0}, "height": 0.4, "rotation": 0}]}],
}
BOXES = {BALLOON: (-0.5, 0.5, -0.5, 0.5)}


def landing_end(l):
    """The landing's end and the side, or None without a note or blocks."""
    if "text" not in l and not l.get("blocks"):
        return None
    pts = l["pts"]
    r = l["rotation"] * math.pi / 180
    u = (math.cos(r), math.sin(r))
    last = pts[-1]
    side = 1.0
    for q in reversed(pts[:-1]):
        v = (last["x"] - q["x"], last["y"] - q["y"])
        if math.hypot(*v) > 0:
            side = 1.0 if v[0] * u[0] + v[1] * u[1] >= 0 else -1.0
            break
    h = l["height"]
    return {"x": last["x"] + side * u[0] * 2 * h, "y": last["y"] + side * u[1] * 2 * h}, side, u


class Case:
    def __init__(self, name):
        self.name = name
        self.by = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
        self.steps = []
        self.gone = set()

    def uid(self, i):
        return f"$uid:o{i}" if i in self.gone else f"$uidOf:{i}"

    def names(self, v):
        if isinstance(v, str) and v.startswith("$uidOf:") and int(v[7:]) in self.gone:
            return f"$uid:o{v[7:]}"
        if isinstance(v, list):
            return [self.names(x) for x in v]
        if isinstance(v, dict):
            return {k: self.names(x) for k, x in v.items()}
        return v

    def capture(self, ids):
        for i in ids:
            if i not in self.gone:
                self.steps.append({"op": "captureUid", "as": f"o{i}", "id": i})
                self.gone.add(i)

    def place_blocks(self, l):
        """The leader's content blocks placed: their new entities."""
        out = {}
        laid = landing_end(l)
        if laid is None:
            return out
        end, side, u = laid
        v = (-u[1], u[0])
        acc = 0.0
        for b in l.get("blocks", []):
            ins = self.by.get(int(b[7:])) if b.startswith("$uidOf:") else None
            if ins is None:
                continue
            x0, x1, y0, y1 = BOXES[ins["block"]]
            s = ins["scale"]
            along = acc - s * x0 if side > 0 else -acc - s * x1
            up = -s * (y0 + y1) / 2
            p = {"x": end["x"] + u[0] * along + v[0] * up, "y": end["y"] + u[1] * along + v[1] * up}
            acc += s * (x1 - x0)
            if p != ins["p"] or ins["rotation"] != l["rotation"]:
                ins = {**ins, "p": p, "rotation": l["rotation"]}
                self.by[ins["id"]] = ins
                out[ins["id"]] = ins
        return out

    def edit(self, edit, changes, removed=(), op="execute"):
        """A step of the command: the leaders it changes (id → leader), those it takes away."""
        real = {i: l for i, l in changes.items() if l != self.by.get(i)}
        if op == "plan":
            out = {"leaders": list(real.values()), "revision": "$current"}
            if removed:
                out["removed"] = [f"$uidOf:{i}" for i in removed]
            self.steps.append({"op": "plan", "input": {"edit": edit}, "result": {"status": "completed", "output": out, "warnings": []},
                               "expect": {"revision": "same"}})
            return self
        self.capture(removed)
        out = {"changed": [f"$uidOf:{i}" for i in real]}
        if removed:
            out["removed"] = [f"$uid:o{i}" for i in removed]
        out["revision"] = "$current"
        followed = {}
        for i, l in real.items():
            self.by[i] = l
        for i in removed:
            del self.by[i]
        for i, l in real.items():
            followed.update(self.place_blocks(l))
        expect = {"revision": "same"}
        if real or removed:
            expect = {"ids": sorted(self.by), "entities": {str(i): self.by[i] for i in [*real, *followed]},
                      "canUndo": True, "dirty": True, "revision": "changed"}
        self.steps.append({"op": "execute", "input": {"edit": self.names(edit)}, "result": {"status": "completed", "output": out, "warnings": []},
                           "expect": self.names(expect), "within": 1e-9})
        return self

    def refuse(self, edit, code, message, path, extra=None):
        inp = {"edit": edit, **(extra or {})}
        self.steps.append({"op": "execute", "input": inp, "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                           "expect": {"revision": "same"}})
        return self

    def other(self, command, inp, result, changes, removed=()):
        """Another command's step, and what follows it."""
        self.capture(removed)
        for i, e in changes.items():
            self.by[i] = e
        for i in removed:
            del self.by[i]
        followed = {}
        for i, e in list(self.by.items()):
            if e["kind"] != "leader":
                continue
            l = copy.deepcopy(e)
            arms = [("tip", l.get("tip"), l["pts"])] + [(k, a.get("tip"), a["pts"]) for k, a in enumerate(l.get("arms", []))]
            touched = False
            for key, t, pts in arms:
                if t is None or "broken" in t:
                    continue
                of = int(t["uid"][7:]) if t["uid"].startswith("$uidOf:") else None
                if of in changes:
                    pts[0] = place_point(self.by[of], t["place"])
                    touched = True
                elif of in removed:
                    t["broken"] = "object_gone"
                    touched = True
            blocks = [b for b in l.get("blocks", []) if int(b[7:]) not in removed]
            if blocks != l.get("blocks", []):
                if blocks:
                    l["blocks"] = blocks
                else:
                    l.pop("blocks")
                touched = True
            if touched:
                self.by[i] = l
                followed[i] = l
                followed.update(self.place_blocks(l))
            elif any(int(b[7:]) in changes for b in l.get("blocks", [])):
                pass
        for i in changes:
            if self.by[i]["kind"] == "leader":
                followed.update(self.place_blocks(self.by[i]))
        self.steps.append({"op": "execute", "command": command, "input": inp, "result": self.names(result),
                           "expect": self.names({"ids": sorted(self.by), "entities": {str(k): v for k, v in followed.items()},
                                                 "canUndo": True, "revision": "changed"}), "within": 1e-9})
        return self

    def case(self):
        return {"name": self.name, "steps": self.steps}


def place_point(e, place):
    if place["kind"] == "vertex":
        return e["pts"][place["index"]]
    if place["kind"] == "node":
        return e["p"]
    if place["kind"] == "start":
        return e["a"]
    return e["b"]


def leader_with(c, i, **more):
    l = copy.deepcopy(c.by[i])
    for k, v in more.items():
        if v is None:
            l.pop(k, None)
        else:
            l[k] = v
    return l


def cases():
    out = []
    c = Case("Kol ekle: ucu tek nokta")
    out.append(c.edit({"kind": "addArm", "uid": "$uidOf:4", "pts": [pt(4, 1)]}, {4: leader_with(c, 4, arms=[{"pts": [pt(4, 1)]}])}).case())
    c = Case("Kol ekle: ucu ve kırığıyla")
    out.append(c.edit({"kind": "addArm", "uid": "$uidOf:4", "pts": [pt(1, 3), pt(3, 5)]},
                      {4: leader_with(c, 4, arms=[{"pts": [pt(1, 3), pt(3, 5)]}])}).case())
    vertex = {"kind": "vertex", "part": 0, "ring": 0, "index": 2}
    c = Case("Kol ekle: ucu odanın köşesine bağlı")
    out.append(c.edit({"kind": "addArm", "uid": "$uidOf:4", "pts": [pt(0, 0)], "tip": {"uid": "$uidOf:1", "place": vertex}},
                      {4: leader_with(c, 4, arms=[{"pts": [pt(6, 4)], "tip": {"uid": "$uidOf:1", "place": vertex}}])}).case())
    c = Case("Kol sil: ek kol")
    c.edit({"kind": "addArm", "uid": "$uidOf:4", "pts": [pt(4, 1)]}, {4: leader_with(c, 4, arms=[{"pts": [pt(4, 1)]}])})
    out.append(c.edit({"kind": "removeArm", "uid": "$uidOf:4", "arm": 1}, {4: leader_with(c, 4, arms=None)}).case())
    c = Case("Kol sil: ilk kol, ek kol ilk olur")
    c.edit({"kind": "addArm", "uid": "$uidOf:4", "pts": [pt(4, 1), pt(4, 4)]}, {4: leader_with(c, 4, arms=[{"pts": [pt(4, 1), pt(4, 4)]}])})
    out.append(c.edit({"kind": "removeArm", "uid": "$uidOf:4", "arm": 0}, {4: leader_with(c, 4, arms=None, pts=[pt(4, 1), pt(4, 4), pt(5, 6)])}).case())
    out.append(Case("Tek kol kalkmaz").refuse({"kind": "removeArm", "uid": "$uidOf:4", "arm": 0}, "invalid_leader",
                                               "Kılavuzun tek kolu kalkmaz; kılavuzu silin.", "edit.arm").case())
    out.append(Case("Olmayan kol").refuse({"kind": "removeArm", "uid": "$uidOf:4", "arm": 3}, "arm_not_found",
                                          "Kılavuzun 1 kolu var; 3. sıradaki yok (sıra 0'dan başlar).", "edit.arm").case())
    c = Case("Hizala: yatay, izdüşümüyle")
    p0 = c.by[4]["pts"][-1]
    ch = {}
    for i in (5, 6):
        q = c.by[i]["pts"][-1]
        t = (q["x"] - p0["x"]) * 1.0 + (q["y"] - p0["y"]) * 0.0
        ch[i] = leader_with(c, i, pts=c.by[i]["pts"][:-1] + [{"x": p0["x"] + t * 1.0, "y": p0["y"] + t * 0.0}])
    out.append(c.edit({"kind": "align", "uids": ["$uidOf:4", "$uidOf:5", "$uidOf:6"], "angle": 0}, ch).case())
    c = Case("Hizala: dikey, 3 m aralıkla, önde ve arkada sırayla")
    r = 90 * math.pi / 180
    d = (math.cos(r), math.sin(r))
    p0 = c.by[5]["pts"][-1]
    others = [(i, (c.by[i]["pts"][-1]["x"] - p0["x"]) * d[0] + (c.by[i]["pts"][-1]["y"] - p0["y"]) * d[1]) for i in (4, 6)]
    ahead = [i for i, t in sorted(others, key=lambda o: o[1]) if t >= 0]
    behind = [i for i, t in sorted(others, key=lambda o: o[1], reverse=True) if t < 0]
    places = {i: 3 * (k + 1) for k, i in enumerate(ahead)} | {i: -3 * (k + 1) for k, i in enumerate(behind)}
    ch = {i: leader_with(c, i, pts=c.by[i]["pts"][:-1] + [{"x": p0["x"] + t * d[0], "y": p0["y"] + t * d[1]}]) for i, t in sorted(places.items())}
    out.append(c.edit({"kind": "align", "uids": ["$uidOf:5", "$uidOf:4", "$uidOf:6"], "angle": 90, "spacing": 3}, ch).case())
    out.append(Case("Hizalamak için iki kılavuz gerekir").refuse({"kind": "align", "uids": ["$uidOf:4", "$uidOf:4"], "angle": 0}, "too_few_objects",
                                                                 "Bu düzenleme en az 2 kılavuz ister.", "edit.uids").case())
    nf = Case("Doğrultu sonlu olmalı")
    nf.steps.append({"op": "execute", "input": {"edit": {"kind": "align", "uids": ["$uidOf:4", "$uidOf:5"], "angle": 0}},
                     "nonFinite": {"edit.angle": "NaN"},
                     "result": {"status": "failed", "error": {"code": "not_finite",
                                                              "message": "Doğrultu sonlu bir sayı değil (NaN ya da sonsuz). Doğrultuyu derece olarak sonlu bir sayıyla verin.",
                                                              "path": "edit.angle"}}, "expect": {"revision": "same"}})
    out.append(nf.case())
    out.append(Case("Aralık sıfırdan büyük olmalı").refuse({"kind": "align", "uids": ["$uidOf:4", "$uidOf:5"], "angle": 0, "spacing": -1},
                                                            "invalid_leader", "Aralık -1; sıfırdan büyük olmalı.", "edit.spacing").case())
    c = Case("Not: yaz ve kaldır")
    c.edit({"kind": "content", "uid": "$uidOf:4", "text": "Salon 24 m²\nZemin: parke"}, {4: leader_with(c, 4, text="Salon 24 m²\nZemin: parke")})
    out.append(c.edit({"kind": "content", "uid": "$uidOf:4", "text": None}, {4: leader_with(c, 4, text=None)}).case())
    out.append(Case("Boş not").refuse({"kind": "content", "uid": "$uidOf:4", "text": "  "}, "empty_text",
                                      "Not boş; notu kaldırmak için null verin.", "edit.text").case())
    c = Case("Blok içerik: balon kolun ucunda")
    out.append(c.edit({"kind": "blocks", "uid": "$uidOf:10", "blocks": ["$uidOf:8"]}, {10: leader_with(c, 10, blocks=["$uidOf:8"])}).case())
    c = Case("Blok içerik: iki balon yan yana, not kalkar")
    out.append(c.edit({"kind": "blocks", "uid": "$uidOf:4", "blocks": ["$uidOf:8", "$uidOf:9"]},
                      {4: leader_with(c, 4, blocks=["$uidOf:8", "$uidOf:9"], text=None)}).case())
    out.append(Case("Blok içerik yerleştirme olmalı").refuse({"kind": "blocks", "uid": "$uidOf:10", "blocks": ["$uidOf:1"]}, "not_an_insert",
                                                             "Verilen nesne bir blok yerleştirmesi değil; kılavuzun içeriği bloktur.", "edit.blocks[0]").case())
    c = Case("Topla: iki balonlu kılavuz birinde")
    c.edit({"kind": "blocks", "uid": "$uidOf:10", "blocks": ["$uidOf:8"]}, {10: leader_with(c, 10, blocks=["$uidOf:8"])})
    c.edit({"kind": "blocks", "uid": "$uidOf:11", "blocks": ["$uidOf:9"]}, {11: leader_with(c, 11, blocks=["$uidOf:9"])})
    first = leader_with(c, 10, arms=[{"pts": c.by[11]["pts"][:-1]}], blocks=["$uidOf:8", "$uidOf:9"])
    out.append(c.edit({"kind": "collect", "uids": ["$uidOf:10", "$uidOf:11"]}, {10: first}, removed=[11]).case())
    out.append(Case("Notlu kılavuz toplanmaz").refuse({"kind": "collect", "uids": ["$uidOf:4", "$uidOf:5"]}, "invalid_leader",
                                                       "Yalnız blok içerikli kılavuzlar toplanır; notlu kılavuz toplanmaz.", "edit.uids[0]").case())
    c = Case("Görünüş: açık ok, boy, yükseklik, zemin")
    out.append(c.edit({"kind": "look", "uids": ["$uidOf:4", "$uidOf:5"], "arrow": "open", "arrowSize": 2, "height": 1, "mask": True},
                      {i: leader_with(c, i, arrow="open", arrowSize=2, height=1, mask=True) for i in (4, 5)}).case())
    c = Case("Görünüş: dolu oka dön")
    c.edit({"kind": "look", "uids": ["$uidOf:4"], "arrow": "dot"}, {4: leader_with(c, 4, arrow="dot")})
    out.append(c.edit({"kind": "look", "uids": ["$uidOf:4"], "arrow": "filled"}, {4: leader_with(c, 4, arrow=None)}).case())
    out.append(Case("Bilinmeyen ok başı").refuse({"kind": "look", "uids": ["$uidOf:4"], "arrow": "kalp"}, "invalid_leader",
                                                 "“kalp” bir ok başı değil (filled, open, dot, none …).", "edit.arrow").case())
    c = Case("Stil: değerleri yazılır ve izler")
    styled = leader_with(c, 4, arrow="dot", arrowSize=1.5, height=2.5 / 1000 * 500, mask=True, leaderStyle=STYLE)
    c.edit({"kind": "style", "uids": ["$uidOf:4"], "style": STYLE}, {4: styled})
    out.append(c.edit({"kind": "style", "uids": ["$uidOf:4"], "style": None}, {4: leader_with(c, 4, leaderStyle=None)}).case())
    out.append(Case("Olmayan stil").refuse({"kind": "style", "uids": ["$uidOf:4"], "style": FAKE}, "style_not_found",
                                           f"“{FAKE}” kimlikli kılavuz stili projede yok.", "edit.style").case())
    node = {"kind": "node", "part": 0}
    c = Case("Bağla: uç noktaya gider, nokta taşınınca izler")
    c.edit({"kind": "bind", "uid": "$uidOf:5", "arm": 0, "tip": {"uid": "$uidOf:2", "place": node}},
           {5: leader_with(c, 5, tip={"uid": "$uidOf:2", "place": node})})
    moved = {**c.by[2], "p": pt(11, 1)}
    out.append(c.other("cad.entities.transform", {"uids": ["$uidOf:2"], "transform": {"kind": "move", "dx": 1, "dy": -1}},
                       {"status": "completed", "output": {"changed": ["$uidOf:2"], "created": [], "locked": [], "revision": "$current"}, "warnings": []},
                       {2: moved}).case())
    c = Case("Bağla: nokta silinince uç kopar")
    c.edit({"kind": "bind", "uid": "$uidOf:5", "arm": 0, "tip": {"uid": "$uidOf:2", "place": node}},
           {5: leader_with(c, 5, tip={"uid": "$uidOf:2", "place": node})})
    out.append(c.other("cad.entities.delete", {"uids": ["$uidOf:2"]},
                       {"status": "completed", "output": {"removed": ["$uidOf:2"], "locked": [], "revision": "$current"}, "warnings": []},
                       {}, removed=[2]).case())
    c = Case("Bağı çöz")
    c.edit({"kind": "bind", "uid": "$uidOf:6", "arm": 0, "tip": {"uid": "$uidOf:3", "place": {"kind": "end"}}},
           {6: leader_with(c, 6, tip={"uid": "$uidOf:3", "place": {"kind": "end"}}, pts=[pt(24, 3), pt(22, 5)])})
    out.append(c.edit({"kind": "bind", "uid": "$uidOf:6", "arm": 0, "tip": None}, {6: leader_with(c, 6, tip=None)}).case())
    out.append(Case("Nesnede olmayan yer").refuse({"kind": "bind", "uid": "$uidOf:4", "arm": 0,
                                                   "tip": {"uid": "$uidOf:1", "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 9}}},
                                                  "no_such_place", "Nesnenin böyle bir yeri yok: köşe, uç ya da merkez nesnede olmalı.", "edit.tip.place").case())
    c = Case("Blokları olan kılavuz silinince blokları da gider")
    c.edit({"kind": "blocks", "uid": "$uidOf:10", "blocks": ["$uidOf:8"]}, {10: leader_with(c, 10, blocks=["$uidOf:8"])})
    out.append(c.other("cad.entities.delete", {"uids": ["$uidOf:10"]},
                       {"status": "completed", "output": {"removed": ["$uidOf:10"], "locked": [], "revision": "$current"}, "warnings": []},
                       {}, removed=[10, 8]).case())
    c = Case("Balon silinince kılavuzun listesinden düşer")
    c.edit({"kind": "blocks", "uid": "$uidOf:4", "blocks": ["$uidOf:8", "$uidOf:9"]}, {4: leader_with(c, 4, blocks=["$uidOf:8", "$uidOf:9"], text=None)})
    out.append(c.other("cad.entities.delete", {"uids": ["$uidOf:8"]},
                       {"status": "completed", "output": {"removed": ["$uidOf:8"], "locked": [], "revision": "$current"}, "warnings": []},
                       {}, removed=[8]).case())
    c = Case("Kılavuz taşınınca balonu gelir")
    c.edit({"kind": "blocks", "uid": "$uidOf:10", "blocks": ["$uidOf:8"]}, {10: leader_with(c, 10, blocks=["$uidOf:8"])})
    moved = leader_with(c, 10, pts=[{"x": p["x"] + 2, "y": p["y"]} for p in c.by[10]["pts"]])
    out.append(c.other("cad.entities.transform", {"uids": ["$uidOf:10"], "transform": {"kind": "move", "dx": 2, "dy": 0}},
                       {"status": "completed", "output": {"changed": ["$uidOf:10"], "created": [], "locked": [], "revision": "$current"}, "warnings": []},
                       {10: moved}).case())
    c = Case("Planla: çizim değişmez")
    out.append(c.edit({"kind": "addArm", "uid": "$uidOf:4", "pts": [pt(4, 1)]}, {4: leader_with(c, 4, arms=[{"pts": [pt(4, 1)]}])}, op="plan").case())
    out.append(Case("Kilitli katmandaki kılavuz").refuse({"kind": "content", "uid": "$uidOf:7", "text": "x"}, "layer_locked", M_LOCKED, "edit.uid").case())
    out.append(Case("Kılavuz değil").refuse({"kind": "content", "uid": "$uidOf:1", "text": "x"}, "not_a_leader", M_NOT_LEADER, "edit.uid").case())
    out.append(Case("Kimlik değil").refuse({"kind": "content", "uid": NOT_UID, "text": "x"}, "invalid_uid", M_UID.format(NOT_UID), "edit.uid").case())
    out.append(Case("Kılavuz yok").refuse({"kind": "content", "uid": FAKE, "text": "x"}, "entity_not_found", M_NOT_FOUND.format(FAKE), "edit.uid").case())
    out.append(Case("Kılavuz verilmedi").refuse({"kind": "look", "uids": [], "mask": True}, "no_entities",
                                                "Kılavuz verilmedi. En az bir kılavuzun kimliğini verin.", "edit.uids").case())
    out.append(Case("Sürüm yazısı geçersiz").refuse({"kind": "content", "uid": "$uidOf:4", "text": "x"}, "invalid_revision", M_REVISION,
                                                     "expectedRevision", {"expectedRevision": "on iki"}).case())
    c = Case("Eski sürüme hazırlanmış düzenleme yazılmaz")
    c.steps.append({"op": "captureRevision", "as": "old"})
    c.edit({"kind": "content", "uid": "$uidOf:4", "text": "Yeni"}, {4: leader_with(c, 4, text="Yeni")})
    c.steps.append({"op": "execute", "input": {"edit": {"kind": "content", "uid": "$uidOf:5", "text": "x"}, "expectedRevision": "$old"},
                    "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": M_CONFLICT, "path": "expectedRevision",
                                                                "revision": "$current"}}, "expect": {"revision": "same"}})
    out.append(c.case())
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.leaders.edit", "commandVersion": 1,
            "title": "Kılavuzları düzenle", "note": "Written by scripts/fixtures/leader_command_cases.py from docs/adr/0326; do not edit by hand.",
            "setup": SETUP, "cases": cases()}


def main():
    content = build()
    text_ = json.dumps(content, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text_:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/leader_command_cases.py")
        print(f"{OUT.relative_to(ROOT)}: {len(content['cases'])} durum güncel")
        return
    OUT.write_text(text_, "utf-8")
    print(f"written: {OUT.relative_to(ROOT)} ({len(content['cases'])} durum)")


if __name__ == "__main__":
    main()
