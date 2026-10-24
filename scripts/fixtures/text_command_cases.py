"""Writes the shared cases of a text's fields and the text commands (docs/adr/0325):

- fixtures/commands/v1/cad.fields.edit.json (`cad.fields.edit` v1)
- fixtures/commands/v1/cad.texts.edit.json (`cad.texts.edit` v1)
- fixtures/commands/v1/cad.texts.check.json (`cad.texts.check` v1)

The rules are the ADR's, written here again without KentOS code:

- A field is a range of a text's letters (Unicode scalar values) bound to a source; its letters are its value, its
  format's prefix and suffix around it. A number is written by the display rule (rounded to seven decimals from the
  float's exact value, then to the digits shown, a half away from zero): a length in the drawing unit, an area in the
  project's area unit, unless the format names one. A value that cannot be read is `####` with the reason.
- Letters `start..end` replaced by `n` letters: a place before them stays, one after them shifts by `n - (end -
  start)`, one inside them stays inside the new ones; letters put in between two push a range's start there along,
  not a range's end. A field the replaced letters touch goes.
- The document writes the fields again at every step whose changes reach their sources.
- Bul ve değiştir replaces each match left to right, none overlapping, caseless the Turkish way (I is ı's, İ is i's)
  unless told; a whole word has no letter, digit or `_` beside it. Numarala writes `start + i × step` with at least
  `digits` digits. Combined and split texts stand at their top: a text's first baseline is `rise(up, h, lines,
  pitch)` under its point (`up` its alignment's share of the height, `pitch` 5/3 × height × spacing), its top a
  height over that; paragraph `k` starts `k` pitches under the first.
- Yazı denetimi: two spaces or more in a row not at a line's start; a word written twice with only white space
  between (caseless the Turkish way, not numbers); brackets and “ ” matched as a stack, a lone `"`; broken fields.

The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/text_command_cases.py           # writes the files
    python3 scripts/fixtures/text_command_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import math
import sys
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import text_along_command_cases as along_ref  # noqa: E402  (Eğriye oturt, Doğrultuya döndür, Düzleştir)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
X, Y = 500000, 4400000
BROKEN = "####"
WORDS = {"removed": "nesne silindi", "noValue": "nesnenin bu değeri yok",
         "notNamed": "adı verilen öznitelik ya da değişken yok", "cycle": "yazılar birbirini okuyor (döngü)"}
UNIT_METRES = {"mm": 0.001, "cm": 0.01, "m": 1.0, "km": 1000.0, "m2": 1.0, "donum": 1000.0, "ha": 10000.0}
UNIT_NAMES = {"mm": "mm", "cm": "cm", "m": "m", "km": "km", "m2": "m²", "donum": "dönüm", "ha": "ha"}
VALUE_LABELS = {"area": "Alan", "perimeter": "Çevre", "length": "Uzunluk", "layer": "Katman", "label": "Etiket",
                "attribute": "Öznitelik", "text": "Yazı", "x": "Doğu (X)", "y": "Kuzey (Y)"}
UPS = {None: 0.0, "baselineCenter": 0.0, "baselineRight": 0.0, "bottomLeft": -0.2, "bottomCenter": -0.2,
       "bottomRight": -0.2, "middleLeft": 0.5, "middleCenter": 0.5, "middleRight": 0.5, "topLeft": 1.0,
       "topCenter": 1.0, "topRight": 1.0}
ALONGS = {None: 0.0, "baselineCenter": 0.5, "baselineRight": 1.0, "bottomLeft": 0.0, "bottomCenter": 0.5,
          "bottomRight": 1.0, "middleLeft": 0.0, "middleCenter": 0.5, "middleRight": 1.0, "topLeft": 0.0,
          "topCenter": 0.5, "topRight": 1.0}
FAKE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3fff"
NOT_UID = "R-12"


# ── The display rule ──────────────────────────────────────────────────────────────────────────────────────────────


def fixed(v, d):
    """`v` with `d` decimals: rounded to seven from its exact value, then to `d`, a half away from zero."""
    x = Decimal(abs(v))
    if d >= 7:
        body = x.quantize(Decimal(1).scaleb(-d), ROUND_HALF_UP)
    else:
        body = x.quantize(Decimal("1e-7"), ROUND_HALF_UP).quantize(Decimal(1).scaleb(-d), ROUND_HALF_UP)
    text = f"{body:f}"
    return "-" + text if v < 0 and any(c in "123456789" for c in text) else text


# ── Letters and ranges ────────────────────────────────────────────────────────────────────────────────────────────


def shifted(p, start, end, n, end_of):
    if start == end:
        return p + n if p > start or (p == start and not end_of) else p
    if p <= start:
        return p
    if p >= end:
        return p - (end - start) + n
    return start + min(p - start, n)


def replace_letters(t, start, end, with_, own=None):
    """The text `t` with letters start..end replaced by `with_`, its runs and fields moved."""
    out = copy.deepcopy(t)
    n = len(with_)
    out["text"] = t["text"][:start] + with_ + t["text"][end:]
    runs = []
    for r in t.get("runs", []):
        s, e = shifted(r["start"], start, end, n, False), shifted(r["end"], start, end, n, True)
        if s < e:
            runs.append({**r, "start": s, "end": e})
    fields = []
    for i, f in enumerate(t.get("fields", [])):
        if i == own:
            fields.append({**f, "start": start, "end": start + n})
            continue
        apart = (f["end"] <= start or f["start"] >= start) if start == end else (f["end"] <= start or f["start"] >= end)
        if apart:
            fields.append({**f, "start": shifted(f["start"], start, end, n, False), "end": shifted(f["end"], start, end, n, True)})
    set_list(out, "runs", runs)
    set_list(out, "fields", fields)
    return out


def set_list(t, key, items):
    if items:
        t[key] = items
    else:
        t.pop(key, None)


# ── Values ────────────────────────────────────────────────────────────────────────────────────────────────────────


def slot(placeholder):
    return int(placeholder[len("$uidOf:"):])


def ring_area(pts):
    a = 0.0
    for i, p in enumerate(pts):
        q = pts[(i + 1) % len(pts)]
        a += p["x"] * q["y"] - q["x"] * p["y"]
    return abs(a) / 2


def ring_length(pts, closed):
    n = len(pts) if closed else len(pts) - 1
    return sum(math.hypot(pts[(i + 1) % len(pts)]["x"] - pts[i]["x"], pts[(i + 1) % len(pts)]["y"] - pts[i]["y"]) for i in range(n))


def measure(e, value):
    """An object's area, perimeter, length, east or north in metres; None when it has none (the shapes cases use)."""
    k = e["kind"]
    if value == "area":
        return ring_area(e["pts"]) if k == "polygon" else (math.pi * e["r"] * e["r"] if k == "circle" else None)
    if value == "perimeter":
        return ring_length(e["pts"], True) if k == "polygon" else (2 * math.pi * e["r"] if k == "circle" else None)
    if value == "length":
        if k == "line":
            return math.hypot(e["b"]["x"] - e["a"]["x"], e["b"]["y"] - e["a"]["y"])
        if k == "polygon":
            return ring_length(e["pts"], True)
        if k == "polyline":
            return ring_length(e["pts"], False)
        return None
    if k in ("point", "text"):
        return e["p"]["x" if value == "x" else "y"]
    return None


def value(doc, f):
    """The value field `f` writes in `doc`, or the reason it cannot."""
    fmt = f.get("format", {})
    s = doc["settings"]
    src = f["source"]
    if src["kind"] == "object":
        e = doc["by_id"].get(slot(src["uid"]))
        if e is None:
            return None, "removed"
        v = src["value"]
        if v == "layer":
            name = doc["layer_names"][e["layerId"]]
            raw = (name, None) if name else (None, "noValue")
        elif v == "label":
            raw = (e["label"], None) if e.get("label") else (None, "noValue")
        elif v == "attribute":
            a = e.get("attrs", {}).get(src["name"])
            raw = (None, "notNamed") if a is None else ((None, "noValue") if a == "" else (a, None))
        elif v == "text":
            raw = (e["text"], None) if e["kind"] == "text" and e["text"] else (None, "noValue")
        else:
            m = measure(e, v)
            if m is None:
                return None, "noValue"
            if v == "area":
                unit = UNIT_METRES[fmt["unit"]] if "unit" in fmt else {"m2": 1.0, "donum": 1000.0, "ha": 10000.0}[s["areaUnit"]]
                d = fmt.get("decimals", s["areaDecimals"])
            else:
                unit = UNIT_METRES[fmt["unit"]] if "unit" in fmt else 1.0
                d = fmt.get("decimals", s["lengthDecimals"])
            t = fixed(m / unit, d)
            raw = (t.replace(".", ",") if fmt.get("comma") else t, None)
    else:
        v = src["value"]
        if v == "name":
            raw = (doc["name"], None) if doc["name"] else (None, "noValue")
        elif v == "scale":
            raw = (f"1:{fixed(s['plotScale'], 0)}", None)
        elif v == "unit":
            raw = (s.get("drawingUnit", "m"), None)
        elif v == "crs":
            raw = ("Yerel" if s["srid"] == 0 else f"EPSG:{s['srid']}", None)
        else:
            var = next((x for x in s.get("variables", []) if x["name"] == src["name"]), None)
            if var is None:
                raw = (None, "notNamed")
            elif "value" not in var or var["value"] is None:
                raw = (None, "noValue")
            elif isinstance(var["value"], bool):
                raw = ("Evet" if var["value"] else "Hayır", None)
            elif isinstance(var["value"], (int, float)):
                x = float(var["value"])
                if "decimals" in fmt:
                    t = fixed(x, fmt["decimals"])
                    t = t.replace(".", ",") if fmt.get("comma") else t
                else:
                    t = fixed(x, 7).rstrip("0").rstrip(".")
                raw = (t, None)
            else:
                raw = (var["value"], None) if var["value"] else (None, "noValue")
    if raw[0] is None:
        return raw
    return fmt.get("prefix", "") + raw[0] + fmt.get("suffix", ""), None


def reaches(doc, frm, to):
    seen, nxt = set(), [frm]
    while nxt:
        u = nxt.pop()
        if u == to:
            return True
        if u in seen:
            continue
        seen.add(u)
        e = doc["by_id"].get(u)
        if e and e["kind"] == "text":
            nxt.extend(slot(f["source"]["uid"]) for f in e.get("fields", []) if f["source"]["kind"] == "object" and f["source"]["value"] == "text")
    return False


def written(doc, own, t):
    """`t` (slot `own`) with its fields written again in `doc`; a text field on a ring with it broken."""
    ring = {own}
    for f in t.get("fields", []):
        if f["source"]["kind"] == "object" and f["source"]["value"] == "text":
            s = slot(f["source"]["uid"])
            if reaches(doc, s, own):
                ring.add(s)
    out = copy.deepcopy(t)
    for i in range(len(t.get("fields", []))):
        f = out["fields"][i]
        src = f["source"]
        if src["kind"] == "object" and src["value"] == "text" and slot(src["uid"]) in ring:
            v, why = None, "cycle"
        else:
            v, why = value(doc, f)
        letters = BROKEN if v is None else v
        if out["text"][f["start"]:f["end"]] != letters:
            out = replace_letters(out, f["start"], f["end"], letters, i)
        if why is None:
            out["fields"][i].pop("broken", None)
        else:
            out["fields"][i]["broken"] = why
    return out


def broken_warnings(t):
    return [{"code": "field_broken", "message": f"“{t['text']}” yazısının {i + 1}. alanı okunamadı: {WORDS[f['broken']]}; harfleri ####.",
             "path": "edit"} for i, f in enumerate(t.get("fields", [])) if "broken" in f]


# ── Documents ─────────────────────────────────────────────────────────────────────────────────────────────────────


def layer(id_, name, visible=True, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def text(i, words, x, y, layer_id="yazi", **more):
    return {"kind": "text", "id": i, "layerId": layer_id, "attrs": {}, "p": {"x": x, "y": y}, "text": words, "height": 0.25,
            "rotation": 0, **more}


def document(entities, name="Kentsel proje", **settings):
    s = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500,
         "workspace": "cad", **settings}
    return {"format": "kentos.document", "version": 1, "name": name, "settings": s, "origin": {"x": X, "y": Y},
            "layers": [layer("mekan", "Mekanlar"), layer("yazi", "Yazılar"), layer("kilitli", "Kilitli", locked=True),
                       layer("gizli", "Gizli", visible=False)],
            "activeLayer": "yazi", "entities": entities, "styles": {"items": [], "categories": []}, "blocks": []}


ROOM = {"kind": "polygon", "id": 1, "layerId": "mekan", "attrs": {"Ad": "Salon", "No": "101", "Boş": ""}, "label": "S-1",
        "pts": [{"x": X, "y": Y}, {"x": X + 6, "y": Y}, {"x": X + 6, "y": Y + 4}, {"x": X, "y": Y + 4}]}
WALL = {"kind": "line", "id": 2, "layerId": "mekan", "attrs": {}, "a": {"x": X, "y": Y - 2}, "b": {"x": X + 3, "y": Y - 6}}
POINT = {"kind": "point", "id": 3, "layerId": "mekan", "attrs": {}, "p": {"x": X + 3.25, "y": Y + 1.5}}
FIELD_TEXTS = [
    text(4, "Salon ", X + 1, Y + 2),
    text(5, "Alan: XX", X + 1, Y + 1),
    text(6, "Kat 1 planı", X + 1, Y + 7, runs=[{"start": 6, "end": 11, "bold": True}]),
    text(7, "Kilitli", X + 1, Y + 8, "kilitli"),
    text(8, "Ref: ", X + 1, Y + 9),
]
FIELDS_BASE = document([ROOM, WALL, POINT, *FIELD_TEXTS],
                       variables=[{"name": "is_no", "label": "İş numarası", "kind": "text", "value": "2026/41"},
                                  {"name": "oran", "kind": "number", "value": 0.125},
                                  {"name": "onay", "kind": "bool", "value": True},
                                  {"name": "bos", "kind": "text"}])


class Doc:
    """A document as the cases follow it: its objects by slot, its layers' names, its settings and name."""

    def __init__(self, setup):
        self.d = {"by_id": {e["id"]: copy.deepcopy(e) for e in setup["entities"]},
                  "layer_names": {n["id"]: n["name"] for n in setup["layers"]}, "settings": setup["settings"],
                  "name": setup["name"]}
        self.locked = {n["id"] for n in setup["layers"] if n["locked"]}
        self.hidden = {n["id"] for n in setup["layers"] if not n["visible"]}

    def get(self, i):
        return self.d["by_id"].get(i)

    def follow(self, changed):
        """The texts whose fields read an object in `changed` written again, and those that read them."""
        out = {}
        queue = [i for i, e in self.d["by_id"].items() if e["kind"] == "text" and any(
            (f["source"]["kind"] == "object" and slot(f["source"]["uid"]) in changed) or f["source"]["kind"] == "project"
            for f in e.get("fields", []))]
        seen = set()
        while queue:
            i = queue.pop(0)
            if i in seen:
                continue
            seen.add(i)
            t = self.d["by_id"][i]
            nxt = written(self.d, i, t)
            if nxt != t:
                self.d["by_id"][i] = nxt
                out[i] = nxt
                if nxt["text"] != t["text"]:
                    queue.extend(j for j, e in self.d["by_id"].items() if e["kind"] == "text" and any(
                        f["source"]["kind"] == "object" and slot(f["source"]["uid"]) == i for f in e.get("fields", [])))
        return out


# ── cad.fields.edit ───────────────────────────────────────────────────────────────────────────────────────────────


def obj(i, v, name=None):
    s = {"kind": "object", "uid": f"$uidOf:{i}", "value": v}
    if name is not None:
        s["name"] = name
    return s


def proj(v, name=None):
    s = {"kind": "project", "value": v}
    if name is not None:
        s["name"] = name
    return s


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def same():
    return {"revision": "same"}


class FieldsCase:
    COMMAND = "cad.fields.edit"

    def __init__(self, name, setup=FIELDS_BASE):
        self.name, self.setup, self.doc, self.steps = name, setup, Doc(setup), []
        self.gone = set()

    def capture(self, ids):
        """Takes the persistent ids of objects a later step removes: `$uid:oN` names them after."""
        for i in ids:
            self.steps.append({"op": "captureUid", "as": f"o{i}", "id": i})
            self.gone.add(i)

    def names(self, value):
        """`value` with the removed objects named by their captured ids."""
        if isinstance(value, str):
            return f"$uid:o{slot(value)}" if value.startswith("$uidOf:") and slot(value) in self.gone else value
        if isinstance(value, list):
            return [self.names(v) for v in value]
        if isinstance(value, dict):
            return {k: self.names(v) for k, v in value.items()}
        return value

    def insert(self, i, start, end, source, fmt=None, op="execute"):
        t = self.doc.get(i)
        if start == end:
            t2 = replace_letters(t, start, end, "#")
            new = {"start": start, "end": start + 1, "source": source}
        else:
            t2 = copy.deepcopy(t)
            new = {"start": start, "end": end, "source": source}
        if fmt is not None:
            new["format"] = fmt
        fields = t2.get("fields", [])
        at = next((k for k, f in enumerate(fields) if f["start"] >= start), len(fields))
        fields.insert(at, new)
        t2["fields"] = fields
        nxt = written(self.doc.d, i, t2)
        inp = {"edit": {"kind": "insert", "uid": f"$uidOf:{i}", "start": start, "end": end, "source": source}}
        if fmt is not None:
            inp["edit"]["format"] = fmt
        self.write(op, inp, [(i, nxt)])
        return self

    def write(self, op, inp, changes, command=None):
        warnings = [w for _, t in changes for w in broken_warnings(t)]
        real = [(i, t) for i, t in changes if t != self.doc.get(i)]
        if op == "plan":
            out = {"texts": [t for _, t in real], "revision": "$current"}
            self.steps.append({"op": "plan", "input": inp, "result": done(out, warnings), "expect": same()})
            return self
        out = {"changed": [f"$uidOf:{i}" for i, _ in real], "revision": "$current"}
        for i, t in real:
            self.doc.d["by_id"][i] = t
        followed = self.doc.follow({i for i, _ in real})
        step = {"op": "execute", "input": inp, "result": done(out, warnings)}
        if command or type(self).COMMAND != "cad.fields.edit":
            step["command"] = command or "cad.fields.edit"
        step["expect"] = {"entities": {str(i): t for i, t in [*real, *followed.items()]}, "canUndo": True, "dirty": True,
                          "revision": "changed"} if real else same()
        self.steps.append(step)
        return self

    def remove(self, i, index):
        t = copy.deepcopy(self.doc.get(i))
        t["fields"].pop(index)
        set_list(t, "fields", t["fields"])
        return self.write("execute", {"edit": {"kind": "remove", "uid": f"$uidOf:{i}", "index": index}}, [(i, t)])

    def format(self, i, index, fmt):
        t = copy.deepcopy(self.doc.get(i))
        if fmt is None:
            t["fields"][index].pop("format", None)
        else:
            t["fields"][index]["format"] = fmt
        return self.write("execute", {"edit": {"kind": "format", "uid": f"$uidOf:{i}", "index": index, "format": fmt}},
                          [(i, written(self.doc.d, i, t))])

    def update(self, uids=None):
        ids = uids if uids is not None else [i for i, e in sorted(self.doc.d["by_id"].items()) if e["kind"] == "text" and e.get("fields")
                                             and e["layerId"] not in self.doc.locked]
        inp = {"edit": {"kind": "update"}}
        if uids is not None:
            inp["edit"]["uids"] = [f"$uidOf:{i}" for i in uids]
        return self.write("execute", inp, [(i, written(self.doc.d, i, self.doc.get(i))) for i in ids])

    def set_label(self, i, label):
        e = self.doc.get(i)
        e["label"] = label
        followed = self.doc.follow({i})
        self.steps.append({"op": "execute", "command": "cad.entities.set", "input": {"uids": [f"$uidOf:{i}"], "label": label, "operation": "label"},
                           "result": done({"changed": [f"$uidOf:{i}"], "ids": [i], "revision": "$current"}),
                           "expect": {"entities": {str(k): t for k, t in followed.items()}, "canUndo": True, "revision": "changed"}})
        return self

    def scale(self, i, factor):
        e = self.doc.get(i)
        c = e["pts"][0]
        e["pts"] = [{"x": c["x"] + (p["x"] - c["x"]) * factor, "y": c["y"] + (p["y"] - c["y"]) * factor} for p in e["pts"]]
        followed = self.doc.follow({i})
        self.steps.append({"op": "execute", "command": "cad.entities.transform",
                           "input": {"uids": [f"$uidOf:{i}"], "transform": {"kind": "scale", "center": c, "factor": factor}},
                           "result": done({"changed": [f"$uidOf:{i}"], "created": [], "locked": [], "revision": "$current"}),
                           "expect": {"entities": {str(k): t for k, t in followed.items()}, "canUndo": True, "revision": "changed"}})
        return self

    def delete(self, i):
        self.capture([i])
        before = {k: copy.deepcopy(v) for k, v in self.doc.d["by_id"].items()}
        del self.doc.d["by_id"][i]
        followed = self.doc.follow({i})
        self.steps.append({"op": "execute", "command": "cad.entities.delete", "input": {"uids": [f"$uidOf:{i}"]},
                           "result": done({"removed": [f"$uid:o{i}"], "locked": [], "revision": "$current"}),
                           "expect": {"entities": {str(k): self.names(t) for k, t in followed.items()}, "canUndo": True, "revision": "changed"}})
        # Undo gives the object and the texts' letters back.
        self.steps.append({"op": "undo", "returns": "Sil", "expect": {"entities": {str(k): self.names(before[k]) for k in followed}}})
        self.doc.d["by_id"] = before
        return self

    def undo(self, label, texts):
        self.steps.append({"op": "undo", "returns": label, "expect": {"entities": {str(i): t for i, t in texts}}})
        return self

    def conflict(self, inp):
        """A write prepared against an earlier revision: nothing written."""
        self.steps.append({"op": "execute", "input": {**inp, "expectedRevision": "$old"},
                           "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": M_CONFLICT,
                                                                       "path": "expectedRevision", "revision": "$current"}},
                           "expect": same()})
        return self

    def refuse(self, inp, code, message, path):
        self.steps.append({"op": "execute", "input": inp, "result": failed(code, message, path), "expect": same()})
        return self

    def case(self):
        c = {"name": self.name}
        if self.setup is not FIELDS_BASE:
            c["setup"] = self.setup
        c["steps"] = self.steps
        return c


def insert(i, start, end, source, fmt=None):
    e = {"kind": "insert", "uid": f"$uidOf:{i}", "start": start, "end": end, "source": source}
    if fmt is not None:
        e["format"] = fmt
    return {"edit": e}


M_UID = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
M_NOT_FOUND = "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
M_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
M_CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
M_NOT_TEXT = "Verilen nesne bir yazı değil; bu komut yalnız yazıları değiştirir."
M_LOCKED = "“Kilitli” katmanı kilitli; üzerindeki yazı değişmez. Kilidi Katmanlar panelinden açın."


def fields_cases():
    c = []
    salon = FIELDS_BASE["entities"][3]
    c.append(FieldsCase("Mahal etiketine odanın alanı, iki basamak ve m² ile").insert(4, 6, 6, obj(1, "area"), {"decimals": 2, "suffix": " m²"}).case())
    c.append(FieldsCase("Harflerin yerine alan: XX odanın alanı olur").insert(5, 6, 8, obj(1, "area")).case())
    c.append(FieldsCase("Çevre projenin uzunluk basamağıyla").insert(4, 6, 6, obj(1, "perimeter")).case())
    c.append(FieldsCase("Çizginin uzunluğu santimetre, basamaksız").insert(4, 6, 6, obj(2, "length"), {"decimals": 0, "unit": "cm", "suffix": " cm"}).case())
    c.append(FieldsCase("Katmanın adı").insert(4, 6, 6, obj(1, "layer")).case())
    c.append(FieldsCase("Etiket").insert(4, 6, 6, obj(1, "label"), {"prefix": "(", "suffix": ")"}).case())
    c.append(FieldsCase("Öznitelik adıyla").insert(4, 0, 5, obj(1, "attribute", "No")).case())
    c.append(FieldsCase("Olmayan öznitelik kopuk: ####, uyarıyla").insert(4, 6, 6, obj(1, "attribute", "Yok")).case())
    c.append(FieldsCase("Boş öznitelik kopuk: değeri yok").insert(4, 6, 6, obj(1, "attribute", "Boş")).case())
    c.append(FieldsCase("Noktanın doğusu ve kuzeyi").insert(4, 6, 6, obj(3, "x")).insert(4, 16, 16, obj(3, "y"), {"decimals": 2, "prefix": " / "}).case())
    c.append(FieldsCase("Açık çizginin alanı yok: kopuk").insert(4, 6, 6, obj(2, "area")).case())
    c.append(FieldsCase("Başka yazının harfleri (çapraz başvuru)").insert(8, 5, 5, obj(6, "text")).case())
    c.append(FieldsCase("Yazı kendini okuyamaz: döngü").insert(8, 5, 5, obj(8, "text")).case())
    c.append(FieldsCase("Birbirini okuyan iki yazı: ikincisi döngüyü kapatır").insert(8, 5, 5, obj(4, "text")).insert(4, 6, 6, obj(8, "text")).case())
    c.append(FieldsCase("Projenin ölçeği, birimi ve koordinat sistemi").insert(4, 6, 6, proj("scale")).insert(4, 11, 11, proj("unit"), {"prefix": ", "})
             .insert(4, 14, 14, proj("crs"), {"prefix": ", "}).case())
    c.append(FieldsCase("Projenin adı").insert(4, 0, 0, proj("name"), {"suffix": " — "}).case())
    c.append(FieldsCase("Değişkenler: metin, sayı basamakla ve basamaksız, evet/hayır").insert(4, 6, 6, proj("variable", "is_no"))
             .insert(4, 13, 13, proj("variable", "oran"), {"decimals": 2, "comma": True, "prefix": " "})
             .insert(4, 18, 18, proj("variable", "oran"), {"prefix": " "}).insert(4, 24, 24, proj("variable", "onay"), {"prefix": " "}).case())
    c.append(FieldsCase("Değeri olmayan ve olmayan değişken kopuk").insert(4, 6, 6, proj("variable", "bos")).insert(4, 10, 10, proj("variable", "yok")).case())
    c.append(FieldsCase("Ondalık virgülle dönüm").insert(4, 6, 6, obj(1, "area"), {"decimals": 3, "unit": "donum", "comma": True, "suffix": " dönüm"}).case())
    c.append(FieldsCase("Biçim dilimi alanın arkasında kayar").insert(6, 4, 5, obj(1, "attribute", "No")).case())
    c.append(FieldsCase("Öndeki alan arkadakini kaydırır").insert(4, 6, 6, obj(1, "area")).insert(4, 0, 0, obj(1, "label"), {"suffix": " "}).case())
    c.append(FieldsCase("Etiket değişince alan aynı adımda izler").insert(4, 6, 6, obj(1, "label")).set_label(1, "Salon-2").case())
    c.append(FieldsCase("Oda büyüyünce alanı izler").insert(4, 6, 6, obj(1, "area"), {"decimals": 2}).scale(1, 2).case())
    c.append(FieldsCase("Oda silinince alan kopar, geri alınca döner").insert(4, 6, 6, obj(1, "area"), {"decimals": 2}).delete(1).case())
    c.append(FieldsCase("Okuduğu yazı değişince zincir izler").insert(8, 5, 5, obj(4, "text")).insert(4, 6, 6, obj(1, "label")).case())
    c.append(FieldsCase("Alanı kaldır: harfleri yazı olarak kalır").insert(4, 6, 6, obj(1, "area")).remove(4, 0).case())
    c.append(FieldsCase("Biçimi değiştir ve projeninkine dön").insert(4, 6, 6, obj(1, "area")).format(4, 0, {"decimals": 0, "suffix": " m²"}).format(4, 0, None).case())
    c.append(FieldsCase("Alanları yenile: değişen yoksa adım yok").insert(4, 6, 6, obj(1, "area")).update().case())
    c.append(FieldsCase("Planla: yazı nasıl olacak, çizim değişmez").insert(4, 6, 6, obj(1, "area"), {"decimals": 1}, op="plan").case())
    c.append(FieldsCase("Geri al alanı ve harflerini kaldırır").insert(4, 6, 6, obj(1, "area")).undo("Alan ekle", [(4, salon)]).case())
    c.append(FieldsCase("Kimlik değil").refuse({"edit": {**insert(4, 0, 0, proj("scale"))["edit"], "uid": NOT_UID}},
                                            "invalid_uid", M_UID.format(NOT_UID), "edit.uid").case())
    c.append(FieldsCase("Yazı yok").refuse({"edit": {**insert(4, 0, 0, proj("scale"))["edit"], "uid": FAKE}}, "entity_not_found", M_NOT_FOUND.format(FAKE), "edit.uid").case())
    c.append(FieldsCase("Yazı değil").refuse(insert(1, 0, 0, proj("scale")), "not_a_text", M_NOT_TEXT, "edit.uid").case())
    c.append(FieldsCase("Kilitli katmandaki yazı").refuse(insert(7, 0, 0, proj("scale")), "layer_locked", M_LOCKED, "edit.uid").case())
    c.append(FieldsCase("Yer yazının dışında").refuse(insert(4, 3, 9, proj("scale")), "invalid_text_field",
                                                       "Alanın yeri 3–9: başı sonundan sonra olamaz, sonu yazının harf sayısını (6) aşamaz.", "edit.start").case())
    c.append(FieldsCase("Kaynak nesne yok").refuse(insert(4, 6, 6, {"kind": "object", "uid": FAKE, "value": "area"}), "entity_not_found",
                                                    f"Alanın kaynağı “{FAKE}” çizimde yok. Var olan bir nesnenin kimliğini verin.", "edit.source.uid").case())
    c.append(FieldsCase("Alanlar üst üste gelmez").insert(5, 6, 8, obj(1, "area"))
             .refuse(insert(5, 7, 7, proj("scale")), "invalid_text_field", "Alanın yeri 7–7 yazının 1. alanına değiyor; alanlar üst üste gelmez.", "edit.start").case())
    c.append(FieldsCase("Öznitelik adsız").refuse(insert(4, 6, 6, obj(1, "attribute")), "invalid_text_field",
                                                   "1. alan: Öznitelik alanı özniteliğin adını ister (`name`).", "edit").case())
    c.append(FieldsCase("Alan uzunluk biriminde yazılmaz").refuse(insert(4, 6, 6, obj(1, "area"), {"unit": "cm"}), "invalid_text_field",
                                                                   "1. alan: Alan değeri cm biriminde yazılamaz.", "edit").case())
    c.append(FieldsCase("Boş biçim").refuse(insert(4, 6, 6, obj(1, "area"), {}), "invalid_text_field",
                                             "1. alan: Boş biçim yazılmaz; alanı kaldırın.", "edit").case())
    c.append(FieldsCase("Sürüm yazısı geçersiz").refuse({**insert(4, 6, 6, proj("scale")), "expectedRevision": "on iki"}, "invalid_revision",
                                                         M_REVISION, "expectedRevision").case())
    rc = FieldsCase("Eski sürüme hazırlanmış alan yazılmaz")
    rc.steps.append({"op": "captureRevision", "as": "old"})
    c.append(rc.insert(4, 6, 6, obj(1, "area")).conflict(insert(5, 0, 0, proj("scale"))).case())
    c.append(FieldsCase("Kaldırılacak alan yok").refuse({"edit": {"kind": "remove", "uid": "$uidOf:4", "index": 0}}, "field_not_found",
                                                         "Yazının 0 alanı var; 0. sıradaki yok (sıra 0'dan başlar).", "edit.index").case())
    return c


# ── cad.texts.edit ────────────────────────────────────────────────────────────────────────────────────────────────


def fold(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


def word_char(c):
    return c.isalnum() or c == "_"


def find_all(text_, find, caseless, whole):
    out, i = [], 0
    same_ = (lambda a, b: fold(a) == fold(b)) if caseless else (lambda a, b: a == b)
    while i < len(text_):
        if i + len(find) <= len(text_) and all(same_(text_[i + k], find[k]) for k in range(len(find))):
            end = i + len(find)
            if not whole or ((i == 0 or not word_char(text_[i - 1])) and (end == len(text_) or not word_char(text_[end]))):
                out.append((i, end))
                i = end
                continue
        i += 1
    return out


def rise(up, h, lines, pitch):
    below = (lines - 1) * pitch
    if up < 0:
        return up * h - below
    if 0 < up < 1:
        return up * h - below / 2
    return up * h


def top_of(align):
    return {0.0: "topLeft", 0.5: "topCenter", 1.0: "topRight"}[ALONGS[align]]


def over(t, lines):
    h = t["height"]
    pitch = (5 / 3) * h * t.get("lineSpacing", 1.0)
    return h - rise(UPS[t.get("align")], h, lines, pitch)


def up_dir(t):
    r = t["rotation"] * math.pi / 180
    return -math.sin(r), math.cos(r)


EDIT_TEXTS = [
    text(2, "Kat planı: 1. kat", X, Y + 30),
    text(3, "KAT ve kat", X, Y + 28, runs=[{"start": 7, "end": 10, "bold": True}]),
    text(4, "Katman", X, Y + 26),
    text(5, "Gizli kat", X, Y + 24, "gizli"),
    text(6, "Kilitli kat", X, Y + 22, "kilitli"),
    text(7, "Ada 101", X, Y + 10),
    text(8, "Parsel 5", X + 2, Y + 8, runs=[{"start": 7, "end": 8, "color": "#E5484D"}]),
    text(9, "Pafta G-23", X - 1, Y + 12, align="middleCenter"),
    text(10, "Birinci satır\nİkinci satır\n\nDördüncü", X, Y - 10, runs=[{"start": 14, "end": 20, "italic": True}]),
    text(11, "K", X, Y - 20),
    text(12, "D", X, Y - 21),
    text(13, "Tek satır", X, Y - 22, rotation=90),
    text(14, "Sol\nSağ", X + 10, Y - 10, rotation=90, align="bottomRight"),
]
EDIT_BASE = document([ROOM, *EDIT_TEXTS])
M_LOCKED_ALL = "Verilen yazıların hepsi kilitli katmanlarda; hiçbiri değişmez. Kilidi Katmanlar panelinden açın."


class TextsCase(FieldsCase):
    COMMAND = "cad.texts.edit"

    def __init__(self, name, setup=EDIT_BASE):
        super().__init__(name, setup)

    def case(self):
        c = {"name": self.name}
        if self.setup is not EDIT_BASE:
            c["setup"] = self.setup
        c["steps"] = self.steps
        return c

    def replace(self, find, with_, uids=None, case_=False, whole=False):
        targets = uids if uids is not None else [i for i, e in sorted(self.doc.d["by_id"].items()) if e["kind"] == "text" and e["layerId"] not in self.doc.hidden]
        changes, locked, blank, count, opened = [], 0, 0, 0, 0
        for i in targets:
            t = self.doc.get(i)
            hits = find_all(t["text"], find, not case_, whole)
            if not hits:
                opened += t["layerId"] not in self.doc.locked
                continue
            if t["layerId"] in self.doc.locked:
                locked += 1
                continue
            opened += 1
            nxt = t
            for s, e in reversed(hits):
                nxt = replace_letters(nxt, s, e, with_)
            if all(ch.isspace() for ch in nxt["text"]):
                blank += 1
                continue
            count += len(hits)
            changes.append((i, nxt))
        edit = {"kind": "replace", "find": find, "with": with_}
        if uids is not None:
            edit["uids"] = [f"$uidOf:{i}" for i in uids]
        if case_:
            edit["matchCase"] = True
        if whole:
            edit["wholeWord"] = True
        if uids is not None and opened == 0 and locked > 0:
            return self.refuse({"edit": edit}, "layer_locked", M_LOCKED_ALL, "edit.uids")
        warnings = []
        if locked:
            warnings.append({"code": "layer_locked", "message": f"{locked} yazı kilitli katmanda olduğu için değişmedi.", "path": "edit.uids"})
        if blank:
            warnings.append({"code": "text_left_blank", "message": f"{blank} yazı değiştirince boş kalacağı için olduğu gibi bırakıldı.", "path": "edit.with"})
        changes = [(i, written(self.doc.d, i, t) if t.get("fields") else t) for i, t in changes]
        return self.edit_step({"edit": edit}, changes, [], [], warnings, replaced=count)

    def edit_step(self, inp, changes, added, removed, warnings, replaced=None, op="execute"):
        warnings = warnings + [w for _, t in changes for w in broken_warnings(t)]
        real = [(i, t) for i, t in changes if t != self.doc.get(i)]
        if op == "plan":
            out = {"texts": [t for _, t in real] + added, "revision": "$current"}
            if removed:
                out["removed"] = [f"$uidOf:{i}" for i in removed]
            self.steps.append({"op": "plan", "input": inp, "result": done(out, warnings), "expect": same()})
            return self
        next_id = max(self.doc.d["by_id"]) + 1
        new_ids = list(range(next_id, next_id + len(added)))
        self.capture(removed)
        out = {"changed": [f"$uidOf:{i}" for i, _ in real]}
        if added:
            out["added"] = [f"$uidOf:{i}" for i in new_ids]
        if removed:
            out["removed"] = [f"$uid:o{i}" for i in removed]
        if replaced is not None:
            out["replaced"] = replaced
        out["revision"] = "$current"
        for i, t in real:
            self.doc.d["by_id"][i] = t
        for i, t in zip(new_ids, added):
            self.doc.d["by_id"][i] = {**t, "id": i}
        for i in removed:
            del self.doc.d["by_id"][i]
        wrote_any = real or added or removed
        expect = same()
        if wrote_any:
            expect = {"ids": sorted(self.doc.d["by_id"]),
                      "entities": {str(i): self.doc.d["by_id"][i] for i in [*[i for i, _ in real], *new_ids]},
                      "canUndo": True, "dirty": True, "revision": "changed"}
        self.steps.append({"op": op, "input": inp, "result": done(out, warnings), "expect": self.names(expect), "within": 1e-9})
        return self

    def number(self, uids, start=1, step=1, digits=None, prefix="", suffix="", place="replace"):
        changes = []
        for k, i in enumerate(uids):
            n = start + step * k
            body = prefix + ("-" if n < 0 else "") + str(abs(n)).zfill(digits or 1) + suffix
            t = self.doc.get(i)
            if place == "replace":
                nxt = {**copy.deepcopy(t), "text": body}
                nxt.pop("runs", None)
                nxt.pop("fields", None)
            elif place == "prefix":
                nxt = replace_letters(t, 0, 0, body)
            else:
                nxt = replace_letters(t, len(t["text"]), len(t["text"]), body)
            changes.append((i, nxt))
        edit = {"kind": "number", "uids": [f"$uidOf:{i}" for i in uids], "start": start, "step": step}
        if digits is not None:
            edit["digits"] = digits
        if prefix:
            edit["prefix"] = prefix
        if suffix:
            edit["suffix"] = suffix
        edit["place"] = place
        return self.edit_step({"edit": edit}, changes, [], [], [])

    def combine(self, uids, op="execute"):
        texts = sorted((self.doc.get(i) for i in uids), key=lambda t: (-t["p"]["y"], t["p"]["x"]))
        first = copy.deepcopy(texts[0])
        lines = first["text"].count("\n") + 1
        vx, vy = up_dir(first)
        h = over(first, lines)
        first["p"] = {"x": first["p"]["x"] + h * vx, "y": first["p"]["y"] + h * vy}
        first["align"] = top_of(texts[0].get("align"))
        runs, fields = list(first.get("runs", [])), list(first.get("fields", []))
        for t in texts[1:]:
            at = len(first["text"]) + 1
            first["text"] += "\n" + t["text"]
            runs += [{**r, "start": r["start"] + at, "end": r["end"] + at} for r in t.get("runs", [])]
            fields += [{**f, "start": f["start"] + at, "end": f["end"] + at} for f in t.get("fields", [])]
        set_list(first, "runs", runs)
        set_list(first, "fields", fields)
        return self.edit_step({"edit": {"kind": "combine", "uids": [f"$uidOf:{i}" for i in uids]}}, [(texts[0]["id"], first)], [],
                              [t["id"] for t in texts[1:]], [], op=op)

    def split(self, i, op="execute"):
        t = self.doc.get(i)
        paras = t["text"].split("\n")
        lines = len(paras)
        vx, vy = up_dir(t)
        pitch = (5 / 3) * t["height"] * t.get("lineSpacing", 1.0)
        top = over(t, lines)
        added, at = [], 0
        for k, words in enumerate(paras):
            a, b = at, at + len(words)
            at = b + 1
            if not words.strip():
                continue
            piece = copy.deepcopy(t)
            piece["text"] = words
            set_list(piece, "runs", [{**r, "start": max(r["start"], a) - a, "end": min(r["end"], b) - a}
                                     for r in t.get("runs", []) if max(r["start"], a) < min(r["end"], b)])
            set_list(piece, "fields", [{**f, "start": f["start"] - a, "end": f["end"] - a} for f in t.get("fields", [])
                                       if f["start"] >= a and f["end"] <= b])
            d = top - k * pitch
            piece["p"] = {"x": t["p"]["x"] + d * vx, "y": t["p"]["y"] + d * vy}
            piece["align"] = top_of(t.get("align"))
            added.append(piece)
        return self.edit_step({"edit": {"kind": "split", "uid": f"$uidOf:{i}"}}, [], added, [i], [], op=op)


def texts_edit_cases():
    c = []
    c.append(TextsCase("Bul ve değiştir: görünen bütün yazılarda, Türkçe büyük-küçük harf bir").replace("kat", "blok").case())
    c.append(TextsCase("Bütün sözcük: Katman değişmez").replace("kat", "blok", whole=True).case())
    c.append(TextsCase("Büyük-küçük harf ayrı").replace("KAT", "BLOK", case_=True).case())
    c.append(TextsCase("Türkçe İ ve ı: İKİNCİ ikinci'yi bulur").replace("İKİNCİ", "2.", uids=[10]).case())
    c.append(TextsCase("Biçim dilimi değiştirilenin arkasında kayar").replace("ve", "ile", uids=[3]).case())
    c.append(TextsCase("Kilitli katmandaki yazı uyarıyla atlanır").replace("kilitli", "açık").case())
    c.append(TextsCase("Verilen yazıların hepsi kilitli: ret").replace("kat", "blok", uids=[6]).case())
    c.append(TextsCase("Boş kalacak yazı atlanır").replace("Katman", "", uids=[4]).case())
    c.append(TextsCase("Bulunamayan: adım yok").replace("yok", "var").case())
    c.append(TextsCase("Gizli katmandaki yazı verilince değişir").replace("kat", "blok", uids=[5]).case())
    fc = TextsCase("Alanın önündeki harfler değişince alan kayar")
    fc.insert(4, 6, 6, obj(1, "area"), {"decimals": 1, "prefix": " "})
    c.append(fc.replace("Katman", "Kat", uids=[4]).case())
    c.append(TextsCase("Numarala: K-01, K-02").number([11, 12], digits=2, prefix="K-").case())
    c.append(TextsCase("Numarala başına, beşer artarak").number([7, 8], start=10, step=5, prefix="", suffix=". ", place="prefix").case())
    c.append(TextsCase("Numarala sonuna, eksiye inerek").number([11, 12, 4], start=1, step=-1, digits=3, prefix=" #", place="suffix").case())
    c.append(TextsCase("Yazıları birleştir: yukarıdan aşağıya, en üsttekinin hizasıyla").combine([7, 8, 9]).case())
    c.append(TextsCase("Birleştir: biçimler kayar").combine([8, 7]).case())
    c.append(TextsCase("Birleştir planı: çizim değişmez").combine([11, 12], op="plan").case())
    c.append(TextsCase("Satırlara ayır: boş satır atlanır, biçim kendi satırında").split(10).case())
    c.append(TextsCase("Satırlara ayır: döndürülmüş, sağ alttan hizalı").split(14).case())
    sc = TextsCase("Satırlara ayır ve geri al")
    sc.split(10)
    sc.steps.append({"op": "undo", "returns": "Satırlara ayır", "expect": {"ids": list(range(1, 15)), "entities": {"10": EDIT_TEXTS[8]}}})
    c.append(sc.case())
    c.append(TextsCase("Boş arama").refuse({"edit": {"kind": "replace", "find": "", "with": "x"}}, "empty_text", "Aranan yazı boş. Bulunacak harfleri verin.", "edit.find").case())
    c.append(TextsCase("Tek yazı birleşmez").refuse({"edit": {"kind": "combine", "uids": ["$uidOf:7", "$uidOf:7"]}}, "too_few_objects",
                                                     "Birleştirmek için en az iki yazı gerekir.", "edit.uids").case())
    c.append(TextsCase("Tek paragraflı yazı ayrılmaz").refuse({"edit": {"kind": "split", "uid": "$uidOf:13"}}, "not_splittable",
                                                               "tek paragraflı yazı satırlara ayrılmaz.", "edit.uid").case())
    c.append(TextsCase("Numaralanacak yazı yok").refuse({"edit": {"kind": "number", "uids": []}}, "no_entities",
                                                         "Yazı verilmedi. En az bir yazının kimliğini verin.", "edit.uids").case())
    c.append(TextsCase("Basamak 12'yi aşamaz").refuse({"edit": {"kind": "number", "uids": ["$uidOf:11"], "digits": 13}}, "invalid_number",
                                                       "Numaralar ±1000000000000000 içinde, basamak en çok 12 olmalı.", "edit.start").case())
    c.append(TextsCase("Yazı değil").refuse({"edit": {"kind": "split", "uid": "$uidOf:1"}}, "not_a_text", M_NOT_TEXT, "edit.uid").case())
    c.append(TextsCase("Kimlik değil").refuse({"edit": {"kind": "split", "uid": NOT_UID}}, "invalid_uid", M_UID.format(NOT_UID), "edit.uid").case())
    c.append(TextsCase("Yazı yok").refuse({"edit": {"kind": "split", "uid": FAKE}}, "entity_not_found", M_NOT_FOUND.format(FAKE), "edit.uid").case())
    c.append(TextsCase("Sürüm yazısı geçersiz").refuse({"edit": {"kind": "combine", "uids": ["$uidOf:7", "$uidOf:8"]}, "expectedRevision": "on iki"},
                                                        "invalid_revision", M_REVISION, "expectedRevision").case())
    tc = TextsCase("Eski sürüme hazırlanmış düzenleme yazılmaz")
    tc.steps.append({"op": "captureRevision", "as": "old"})
    c.append(tc.number([11], prefix="K-").conflict({"edit": {"kind": "split", "uid": "$uidOf:10"}}).case())
    bc = TextsCase("Kopuk alanlı yazıda değiştirme uyarır")
    bc.insert(4, 6, 6, obj(1, "attribute", "Yok"), {"prefix": " "})
    c.append(bc.replace("Katman", "Kat", uids=[4]).case())
    c.append(TextsCase("Kilitli yazı numaralanmaz").refuse({"edit": {"kind": "number", "uids": ["$uidOf:6"]}}, "layer_locked", M_LOCKED, "edit.uids[0]").case())
    return c


# ── cad.texts.check ───────────────────────────────────────────────────────────────────────────────────────────────


def findings(uid, t):
    s = t["text"]
    out = []
    i = 0
    while i < len(s):
        if s[i] == " ":
            j = i
            while j < len(s) and s[j] == " ":
                j += 1
            if j - i >= 2 and not (i == 0 or s[i - 1] == "\n"):
                out.append((i, 0, {"uid": uid, "kind": "doubleSpace", "start": i, "end": j, "words": f"Art arda {j - i} boşluk."}))
            i = j
        else:
            i += 1
    words, i = [], 0
    while i < len(s):
        if word_char(s[i]):
            j = i
            while j < len(s) and word_char(s[j]):
                j += 1
            words.append((i, j))
            i = j
        else:
            i += 1
    for (a0, a1), (b0, b1) in zip(words, words[1:]):
        if all(c.isspace() for c in s[a1:b0]) and not all(c.isnumeric() for c in s[b0:b1]) and \
                "".join(map(fold, s[a0:a1])) == "".join(map(fold, s[b0:b1])):
            out.append((b0, 1, {"uid": uid, "kind": "repeatedWord", "start": b0, "end": b1, "words": f"“{s[b0:b1]}” art arda iki kez yazılmış."}))
    opened, lone, straight = [], [], []
    pairs = {")": "(", "]": "[", "}": "{", "”": "“"}
    for i, ch in enumerate(s):
        if ch in "([{“":
            opened.append((i, ch))
        elif ch in pairs:
            if opened and opened[-1][1] == pairs[ch]:
                opened.pop()
            else:
                lone.append((i, ch))
        elif ch == '"':
            straight.append(i)
    bad = [(i, f"“{ch}” kapanmıyor.") for i, ch in opened] + [(i, f"“{ch}” açılmamış.") for i, ch in lone]
    if len(straight) % 2 == 1:
        bad.append((straight[-1], "Tırnak (\") kapanmıyor."))
    for i, words_ in sorted(bad, key=lambda b: b[0]):
        out.append((i, 2, {"uid": uid, "kind": "unbalanced", "start": i, "end": i + 1, "words": words_}))
    for f in t.get("fields", []):
        if "broken" in f:
            out.append((f["start"], 3, {"uid": uid, "kind": "brokenField", "start": f["start"], "end": f["end"],
                                         "words": f"Alan okunamadı: {WORDS[f['broken']]}."}))
    return [o for _, _, o in sorted(out, key=lambda o: (o[0], o[1]))]


CHECK_TEXTS = [
    text(2, "Bodrum  kat", X, Y),
    text(3, "  girinti\n  ikinci satır", X, Y - 1),
    text(4, "ve Ve ve", X, Y - 2),
    text(5, "İzmir izmir IŞIK ışık", X, Y - 3),
    text(6, "1 1 adet", X, Y - 4),
    text(7, "(açık [iç] kapalı)", X, Y - 5),
    text(8, "(kapanmayan ve) fazla)", X, Y - 6),
    text(9, "“tırnak” ve \"tek", X, Y - 7),
    text(10, "kat\nkat", X, Y - 8),
    text(11, "temiz yazı", X, Y - 9),
    text(12, "gizli  yazı", X, Y - 10, "gizli"),
    text(13, "{süslü] ters", X, Y - 11),
]
CHECK_BASE = document([ROOM, *CHECK_TEXTS])


def check_case(name, inp=None, setup=CHECK_BASE, steps_before=()):
    doc = Doc(setup)
    ids = [int(u[len("$uidOf:"):]) for u in (inp or {}).get("uids", [])]
    shown = sorted(i for i, e in doc.d["by_id"].items() if e["kind"] == "text" and (ids and i in ids or not ids and e["layerId"] not in doc.hidden))
    out = {"findings": [f for i in shown for f in findings(f"$uidOf:{i}", doc.get(i))], "texts": len(shown), "revision": "$current"}
    c = {"name": name}
    if setup is not CHECK_BASE:
        c["setup"] = setup
    c["steps"] = [*steps_before, {"op": "execute", "input": inp or {}, "result": done(out), "expect": same()}]
    return c


def texts_check_cases():
    c = [check_case("Bütün görünen yazılar: gizli katmandaki okunmaz")]
    for i, name in [(2, "Çift boşluk"), (3, "Satır başındaki boşluk girintidir"), (4, "Yinelenen sözcük, büyük-küçük harf bir"),
                    (5, "Türkçe İ ve I"), (6, "Sayılar sözcük değildir"), (7, "İç içe parantezler dengeli"),
                    (8, "Fazladan kapanan parantez"), (9, "Tek kalan düz tırnak"), (10, "Satır sonu da boşluktur"),
                    (11, "Temiz yazı"), (13, "Ters parantez çifti")]:
        c.append(check_case(name, {"uids": [f"$uidOf:{i}"]}))
    c.append(check_case("Gizli katmandaki yazı verilince okunur", {"uids": ["$uidOf:12"]}))
    c.append(check_case("Verilen yazılar çizimin sırasıyla", {"uids": ["$uidOf:8", "$uidOf:2", "$uidOf:8"]}))
    c.append(check_case("Boş çizim", setup=document([])))
    broken_setup = document([ROOM, text(2, "Alan ", X, Y)])
    fc = FieldsCase("x", broken_setup)
    fc.insert(2, 5, 5, obj(1, "area"))
    fc.delete(1)
    fc.steps.pop()  # not the undo
    steps = [{**s, "command": "cad.fields.edit"} if "command" not in s else s for s in fc.steps]
    # The text as the delete left it: its field broken.
    gone = Doc(broken_setup)
    gone.d["by_id"][2] = fc.doc.get(2)
    del gone.d["by_id"][1]
    after = written(gone.d, 2, gone.get(2))
    out = {"findings": findings("$uidOf:2", after), "texts": 1, "revision": "$current"}
    c.append({"name": "Kopuk alan bulgudur", "setup": broken_setup,
              "steps": [*steps, {"op": "execute", "input": {}, "result": done(out), "expect": {"revision": "same"}}]})
    c.append({"name": "Kimlik değil", "steps": [{"op": "execute", "input": {"uids": [NOT_UID]},
                                                  "result": failed("invalid_uid", M_UID.format(NOT_UID), "uids[0]"), "expect": same()}]})
    c.append({"name": "Yazı yok", "steps": [{"op": "execute", "input": {"uids": [FAKE]},
                                              "result": failed("entity_not_found", M_NOT_FOUND.format(FAKE), "uids[0]"), "expect": same()}]})
    c.append({"name": "Yazı değil", "steps": [{"op": "execute", "input": {"uids": ["$uidOf:1"]},
                                                "result": failed("not_a_text", M_NOT_TEXT, "uids[0]"), "expect": same()}]})
    c.append({**check_case("Planla execute ile aynıdır"), "name": "Planla execute ile aynıdır"})
    c[-1]["steps"][0]["op"] = "plan"
    return c


def build(command, title, setup, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "Written by scripts/fixtures/text_command_cases.py from docs/adr/0325 (Eğriye oturt, Doğrultuya döndür, "
                    "Düzleştir from text_along_command_cases.py, docs/adr/0196 §4); do not edit by hand.",
            "setup": setup, "cases": cases}


def main():
    files = {
        "cad.fields.edit.json": build("cad.fields.edit", "Yazının alanlarını düzenle", FIELDS_BASE, fields_cases()),
        "cad.texts.edit.json": build("cad.texts.edit", "Yazıları düzenle", EDIT_BASE, texts_edit_cases() + along_ref.curve_cases(
            document, X, Y, {"uid": M_UID, "notFound": M_NOT_FOUND, "notText": M_NOT_TEXT, "locked": M_LOCKED,
                             "fake": FAKE})),
        "cad.texts.check.json": build("cad.texts.check", "Yazıları denetle", CHECK_BASE, texts_check_cases()),
    }
    stale = []
    for name, content in files.items():
        text_ = json.dumps(content, ensure_ascii=False, indent=2) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if not path.exists() or path.read_text("utf-8") != text_:
                stale.append(name)
            continue
        path.write_text(text_, "utf-8")
        print(f"written: {path.relative_to(ROOT)} ({len(content['cases'])} durum)")
    if "--check" in sys.argv:
        if stale:
            sys.exit(f"güncel değil: {', '.join(stale)}: python3 scripts/fixtures/text_command_cases.py")
        print("text_command_cases: " + ", ".join(f"{n} {len(c['cases'])} durum" for n, c in files.items()) + " güncel")


if __name__ == "__main__":
    main()
