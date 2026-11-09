#!/usr/bin/env python3
"""The shared cases of `map.work.query` and `map.work.edit` (plan/map-cad.md MAP-01, docs/adr/1200), written from the
ADR without KentOS code, and the source files they read (fixtures/map-work/v1).

    python3 scripts/fixtures/map_work_cases.py           # writes fixtures/commands/v1/map.work.*.json and the files
    python3 scripts/fixtures/map_work_cases.py --check   # writes nothing; compares

The drawing is a map project (cadWork map) with a GNSS campaign (its points imported and contours drawn from it), an
orthophoto and an old orthophoto (rasters showing their files; the old one on a locked layer), a cadastral CAD
reference held by a layer group, a design surface and a free drawing layer. An item's revision is the SHA-256 of its
file (hashlib here); the rules, their order and the sentences are ADR 1200 §2–§3's, written here by hand:

- An object is in the item that names its layer, else the one naming the nearest group above it (the layer profile).
  Its role: `derived` when a derivation names it; else `source` on a campaign's or a reference's layer; else `free`.
- A derivation is out of date when its recorded revision is not its source's revision; read deep, the source's
  revision is its file's content now. Some objects gone: `missing`.
- An item: `missing` (its file is not there), `changed` (deep: its file is not its revision), `incomplete` (a layer it
  names is not in the drawing), `stale` (approved, with out-of-date derived objects on its layers); the first that
  holds is its state, every one says why.
- Each edit is one undo step; the work structure is written whole after it (status `draft` and empty lists unwritten).

ADR 1201 adds: an item's file not at its path is looked for, given the open project's file (`drawing`), at its relative
path from that file's folder, then in the project's search paths (`xrefPaths`: first its relative path there, then its
file name); the file row says `via` (path, relative, searchPath, none), `at` when found elsewhere and the kept
`relative`. `release` lets objects go of their derivations; a relink takes the external references on the old file along
when they loaded the same content, and warns about the others.

ADR 2080 adds, written here from the ADR with hashlib and json:

- An item without a file has a content revision: SHA-256 of the compact JSON list `[[uid, digest], …]` of the objects
  its layers hold, sorted by uid, where an object's digest is SHA-256 of its JSON with sorted keys and without its slot
  id (`kentos.object-digest/1`). A derivation from such an item is judged against it. A quick read gives it on the row
  only where it is judged (a derivation's source, an approved item with a stamp); a deep read on every such row.
- An approval stamps the item's revision (its file's, or its content revision) in `approved`; leaving the approved
  status takes it away; approving an approved item that moved on stamps it again. An approved item whose revision now
  is not its stamp is `revised`, after the other states.
- An object its own layers hold is not derived from an item without a file (`own_content`).
"""

import copy
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILES = ROOT / "fixtures/map-work/v1"

# ── The source files ─────────────────────────────────────────────────────────────────────────────────────────────────

NMEA = (ROOT / "fixtures/gnss/v1/sample.nmea").read_bytes()
# The campaign's later log: the same receiver's file with one more fix (another length, another SHA-256).
NMEA_LATER = NMEA + b"$GPGGA,093410.00,3954.9640000,N,03550.5540000,E,4,12,0.80,1063.500,M,36.874,M,1.0,0001*4A\n"
DXF = ("0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nYAPI\n10\n500020.0\n20\n4400010.0\n30\n0.0\n11\n500030.0\n21\n4400010.0\n"
       "31\n0.0\n0\nENDSEC\n0\nEOF\n").encode()
WRITTEN = {"kampanya.nmea": NMEA, "kampanya-2026-11.nmea": NMEA_LATER, "kadastro.dxf": DXF}
ORTHO = ROOT / "fixtures/raster/v1/files/i8.tif"
OLD_ORTHO = ROOT / "fixtures/raster/v1/files/geographic.tif"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def digest(e):
    """An object's digest (`kentos.object-digest/1`, ADR 1320): SHA-256 of its JSON with sorted keys, no slot id."""
    v = {k: x for k, x in e.items() if k != "id"}
    return sha(json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode())


# docs/adr/3280: an object's four parts, its fields split by name (the slot id in none); each part's digest the first
# 16 digits of the SHA-256 of its fields as JSON with sorted keys.
PARTS = ("geometry", "attributes", "layer", "look")
ATTRIBUTE_FIELDS = ("attrs", "attachments")
LOOK_FIELDS = ("color", "label", "symbol", "lineWeight", "labelPins", "lineType", "linePattern", "lineTypeScale",
               "transparency", "order", "byLayer", "wipeout")


def parts(e):
    split = [{}, {}, {}, {}]
    for k, v in e.items():
        if k == "id":
            continue
        split[1 if k in ATTRIBUTE_FIELDS else 2 if k == "layerId" else 3 if k in LOOK_FIELDS else 0][k] = v
    return [sha(json.dumps(p, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode())[:16] for p in split]


S_GNSS, S_LATER, S_DXF = sha(NMEA), sha(NMEA_LATER), sha(DXF)
S_ORTHO, S_OLD = sha(ORTHO.read_bytes()), sha(OLD_ORTHO.read_bytes())
P_GNSS, P_LATER, P_DXF = "$fixture:map-work/v1/kampanya.nmea", "$fixture:map-work/v1/kampanya-2026-11.nmea", \
    "$fixture:map-work/v1/kadastro.dxf"
P_ORTHO, P_OLD = "$fixture:raster/v1/files/i8.tif", "$fixture:raster/v1/files/geographic.tif"
CONTENT = {P_GNSS: S_GNSS, P_LATER: S_LATER, P_DXF: S_DXF, P_ORTHO: S_ORTHO, P_OLD: S_OLD}

# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────────


def uid(n):
    return f"0192f5c0-1200-7000-8000-{n:012x}"


G1, G2, G3, R1, A1, T1, T2, F1, K1 = (uid(n) for n in range(1, 10))
GHOST = uid(0x999)
XREF_SAME, XREF_OTHER, XREF_ELSE = "0192f5c0-1201-7000-8000-000000000001", "0192f5c0-1201-7000-8000-000000000002", \
    "0192f5c0-1201-7000-8000-000000000003"


def xref_of(path, digest):
    return {"path": path, "digest": digest}


def xref_block(id_, name, path, digest):
    """An external reference's definition (ADR 0348): one line of its source, its source's path and digest."""
    return {"id": id_, "name": name, "base": {"x": 0.0, "y": 0.0},
            "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.0},
                          "b": {"x": 1.0, "y": 0.0}}],
            "xref": xref_of(path, digest)}
X0, Y0 = 500000.0, 4400000.0


def layer(id_, name, kind="layer", locked=False, children=()):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": list(children)}


def point(slot, layer_id, name, x, y):
    return {"kind": "point", "id": slot, "layerId": layer_id, "attrs": {"Ad": name}, "p": {"x": X0 + x, "y": Y0 + y}}


def line(slot, layer_id, a, b):
    return {"kind": "line", "id": slot, "layerId": layer_id, "attrs": {},
            "a": {"x": X0 + a[0], "y": Y0 + a[1]}, "b": {"x": X0 + b[0], "y": Y0 + b[1]}}


def raster(slot, layer_id, file):
    return {"kind": "raster", "id": slot, "layerId": layer_id, "attrs": {},
            "affine": [X0, 1.0, 0.0, Y0 + 10.0, 0.0, -1.0], "width": 10, "height": 10, "bands": 1, "sample": "u8",
            "srid": 5256, "style": {"render": "gray", "bands": [1]}, "file": file}


def item(id_, kind, name, layers=(), file=None, status="draft"):
    out = {"id": id_, "kind": kind, "name": name}
    if status != "draft":
        out["status"] = status
    if layers:
        out["layers"] = list(layers)
    if file is not None:
        out["file"] = {"path": file[0], "revision": file[1]}
    return out


def folder_of(path):
    """A `$tmp:`/`$fixture:` file's folder."""
    return path.rsplit("/", 1)[0]


def resolve(folder, rel):
    """`rel` (parts by "/", ".." only at the start) from `folder`, lexically, keeping the `$tmp:` root."""
    root, rest = folder.split(":", 1)
    parts = [p for p in rest.split("/") if p]
    for part in rel.split("/"):
        if part == "..":
            parts.pop()
        elif part not in ("", "."):
            parts.append(part)
    return root + ":" + "/".join(parts)


def derivation(id_, source, revision, operation, uids):
    return {"id": id_, "source": source, "revision": revision, "operation": operation, "uids": list(uids)}


IMPORT, CONTOURS = "GNSS içe aktarma", "Eş yükselti eğrileri"


def start_work():
    return {
        "items": [
            item("is-1", "campaign", "GNSS ölçümü 2026-10", ["gnss"], (P_GNSS, S_GNSS)),
            item("is-2", "reference", "Ortofoto 2025", ["ortofoto"], (P_ORTHO, S_ORTHO)),
            item("is-3", "reference", "Kadastro CAD altlığı", ["altlik"], (P_DXF, S_DXF)),
            item("is-4", "design", "Tasarım yüzeyi", ["tasarim"], status="review"),
            item("is-5", "reference", "Eski ortofoto", ["kilitli"], (P_OLD, S_OLD)),
        ],
        "derivations": [
            derivation("tr-1", "is-1", S_GNSS, IMPORT, [G1, G2]),
            derivation("tr-2", "is-1", S_GNSS, CONTOURS, [T1, T2]),
        ],
    }


def setup(work="map", map_work="start"):
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work}
    if map_work == "start":
        map_work = start_work()
    if map_work is not None:
        settings["mapWork"] = map_work
    return {
        "format": "kentos.document", "version": 2, "name": "Harita işi", "settings": settings,
        "origin": {"x": X0, "y": Y0},
        "layers": [layer("0", "0"), layer("gnss", "GNSS noktaları"), layer("ortofoto", "Ortofoto"),
                   layer("altlik", "Kadastro altlığı", "group", children=[layer("altlik-yapi", "Yapılar")]),
                   layer("tasarim", "Tasarım yüzeyi"), layer("cizim", "Çizim"),
                   layer("kilitli", "Kilitli", locked=True), layer("bos", "Boş")],
        "activeLayer": "cizim",
        "entities": [point(1, "gnss", "G1", 1.0, 2.0), point(2, "gnss", "G2", 5.0, 2.0), point(3, "gnss", "G3", 9.0, 2.0),
                     raster(4, "ortofoto", P_ORTHO), line(5, "altlik-yapi", (20.0, 10.0), (30.0, 10.0)),
                     line(6, "tasarim", (0.0, 0.0), (10.0, 0.0)), line(7, "tasarim", (0.0, 5.0), (10.0, 5.0)),
                     line(8, "cizim", (0.0, 20.0), (10.0, 25.0)), raster(9, "kilitli", P_OLD)],
        "uids": [G1, G2, G3, R1, A1, T1, T2, F1, K1],
        "styles": {"items": [], "categories": []},
    }


# ── The rules (ADR 1200 §2) ──────────────────────────────────────────────────────────────────────────────────────────


class Drawing:
    """The drawing a case's steps leave: its layers' tree, its objects (uid → layer) and the work structure; the files
    that steps put in `$tmp:` (path → content's SHA-256)."""

    def __init__(self, d, files=None):
        self.d = copy.deepcopy(d)
        self.objects = dict(zip(self.d["uids"], (e["layerId"] for e in self.d["entities"])))
        self.entities = dict(zip(self.d["uids"], copy.deepcopy(self.d["entities"])))
        self.files = dict(CONTENT, **(files or {}))
        self.parents = {}

        def walk(nodes, parent):
            for n in nodes:
                self.parents[n["id"]] = parent
                walk(n["children"], n["id"])
        walk(self.d["layers"], None)

    @property
    def work(self):
        return self.d["settings"].get("mapWork") or {"items": [], "derivations": []}

    def items(self):
        return self.work.get("items", [])

    def derivations(self):
        return self.work.get("derivations", [])

    def get(self, id_):
        return next((i for i in self.items() if i["id"] == id_), None)

    def item_of_layer(self, layer_id):
        at = layer_id
        while at is not None:
            for i in self.items():
                if at in i.get("layers", []):
                    return i
            at = self.parents.get(at)
        return None

    def leaves(self):
        return [lid for lid in self.parents if not self.node(lid)["children"] and self.node(lid)["type"] == "layer"]

    def node(self, lid):
        def find(nodes):
            for n in nodes:
                if n["id"] == lid:
                    return n
                got = find(n["children"])
                if got:
                    return got
            return None
        return find(self.d["layers"])

    def look(self, f, drawing=None):
        """Where an item's file is (ADR 1201 §1): its path, its relative path from the drawing's folder, the search
        paths (each: the relative path there, then the file's name). (where, via)."""
        if f["path"] in self.files:
            return f["path"], "path"
        rel = f.get("relative")
        if drawing is not None and rel is not None:
            at = resolve(folder_of(drawing), rel)
            if at in self.files:
                return at, "relative"
        for base in self.d["settings"].get("xrefPaths", []):
            if not base.startswith("$"):
                if drawing is None:
                    continue
                base = resolve(folder_of(drawing), base)
            if rel is not None and resolve(base, rel) in self.files:
                return resolve(base, rel), "searchPath"
            named = base + "/" + f["path"].rsplit("/", 1)[-1]
            if named in self.files:
                return named, "searchPath"
        return None, "none"

    def file_state(self, i, deep, drawing=None):
        f = i["file"]
        at, via = self.look(f, drawing)
        if at is None:
            return "missing", None, via, None
        if not deep:
            return "found", None, via, at
        now = self.files[at]
        return ("current" if now == f["revision"] else "changed"), now, via, at

    def pairs(self, item_id):
        """ADR 2080 §1: [persistent id, object digest] of the objects an item's layers hold, by id (the list an approval
        keeps since docs/adr/2880)."""
        return sorted([u, digest(self.entities[u])] for u, lid in self.objects.items()
                      if (self.item_of_layer(lid) or {}).get("id") == item_id)

    def parts_of(self, item_id):
        """docs/adr/3280: the parts of the objects of `pairs(item_id)`, in its order."""
        return [parts(self.entities[u]) for u, _ in self.pairs(item_id)]

    def content(self, item_id):
        """ADR 2080 §1: the content revision of an item without a file."""
        return sha(json.dumps(self.pairs(item_id), separators=(",", ":")).encode())

    def kept(self, i):
        """The revision an approval stamps and is judged against: its file's, or its content revision."""
        return i["file"]["revision"] if "file" in i else self.content(i["id"])

    def source_now(self, d, deep, drawing=None):
        i = self.get(d["source"])
        if "file" not in i:
            return self.content(i["id"])
        if deep:
            state, now, _, _ = self.file_state(i, True, drawing)
            if state == "changed":
                return now
        return i["file"]["revision"]

    def stale(self, d, deep, drawing=None):
        return self.source_now(d, deep, drawing) != d["revision"]

    def stale_objects_in(self, it, deep):
        n = 0
        for d in self.derivations():
            if not self.stale(d, deep):
                continue
            for u in d["uids"]:
                if u in self.objects:
                    held = self.item_of_layer(self.objects[u])
                    n += held is not None and held["id"] == it["id"]
        return n

    def tree(self, deep=False, of=None, drawing=None):
        rows, derived, stale, problems = [], [], 0, 0
        for d in self.derivations():
            present = sum(u in self.objects for u in d["uids"])
            why = []
            old = self.stale(d, deep, drawing)
            if old:
                why.append(f"Kaynağı “{self.get(d['source'])['name']}” bu nesneler üretildikten sonra değişti.")
            gone = len(d["uids"]) - present
            if gone:
                why.append(f"{gone} nesnesi çizimde yok.")
                problems += 1
            state = "stale" if old else ("missing" if gone else "current")
            stale += old
            row = {"id": d["id"], "source": d["source"], "operation": d["operation"], "revision": d["revision"],
                   "objects": present, "recorded": len(d["uids"]), "state": state}
            if why:
                row["why"] = why
            derived.append(row)
        counts = {}
        for lid in self.leaves():
            held = self.item_of_layer(lid)
            if held:
                counts[held["id"]] = counts.get(held["id"], 0) + sum(1 for v in self.objects.values() if v == lid)
        for i in self.items():
            why, state = [], "current"

            def worse(s, words):
                nonlocal state
                why.append(words)
                if state == "current":
                    state = s
            row = {"id": i["id"], "kind": i["kind"], "name": i["name"], "status": i.get("status", "draft"),
                   "objects": counts.get(i["id"], 0)}
            if i.get("layers"):
                row["layers"] = list(i["layers"])
            if "file" in i:
                fs, _, via, at = self.file_state(i, deep, drawing)
                row["file"] = {"path": i["file"]["path"], "revision": i["file"]["revision"], "state": fs, "via": via}
                if at is not None and at != i["file"]["path"]:
                    row["file"]["at"] = at
                if "relative" in i["file"]:
                    row["file"]["relative"] = i["file"]["relative"]
                if fs == "missing":
                    worse("missing", f"Dosyası yok: {i['file']['path']}. Yeniden bağla ile yeni yerini gösterin.")
                elif fs == "changed":
                    worse("changed", "Dosyası projenin kabul ettiği içerikten farklı: Yenile ile kabul edin ya da "
                                     "Yeniden bağla ile başka dosyayı gösterin.")
            for lid in i.get("layers", []):
                if lid not in self.parents:
                    worse("incomplete", f"“{lid}” katmanı çizimde yok.")
            if i.get("status") == "approved":
                n = self.stale_objects_in(i, deep)
                if n:
                    worse("stale", f"Onaylı, ama katmanlarındaki {n} türetilmiş nesnenin kaynağı onaydan sonra değişti.")
            judged = "approved" in i or any(d["source"] == i["id"] for d in self.derivations())
            if "file" not in i and (deep or judged):
                row["content"] = self.content(i["id"])
            if "approved" in i:
                row["approved"] = i["approved"]
                now = self.kept(i)
                if i.get("status") == "approved" and now != i["approved"]:
                    # docs/adr/2880: what changed, object by object, when the approval kept its objects' digests.
                    if "file" not in i and i.get("approvedObjects"):
                        then = {u: dg for u, dg in i["approvedObjects"]}
                        nowp = {u: dg for u, dg in self.pairs(i["id"])}
                        added = sorted(u for u in nowp if u not in then)
                        changed_ = sorted(u for u in nowp if u in then and then[u] != nowp[u])
                        removed = [u for u in then if u not in nowp]
                        ch = {"added": len(added), "removed": len(removed), "changed": len(changed_)}
                        listed = sorted(added + changed_)[:1000]
                        if listed:
                            ch["uids"] = listed
                        # docs/adr/3280: with the approval's parts, each changed object's parts against its parts now.
                        if i.get("approvedParts"):
                            then_parts = {u: ps for (u, _), ps in zip(i["approvedObjects"], i["approvedParts"])}
                            by_part, part_objects = dict.fromkeys(PARTS, 0), []
                            for u in changed_:
                                now_parts = parts(self.entities[u])
                                differ = [PARTS[k] for k in range(4) if then_parts[u][k] != now_parts[k]]
                                for name in differ:
                                    by_part[name] += 1
                                part_objects.append({"uid": u, "parts": differ})
                            ch["parts"] = by_part
                            if part_objects:
                                ch["objects"] = part_objects[:1000]
                        row["changes"] = ch
                    what = "dosyası" if "file" in i else "içeriği"
                    worse("revised", f"Onaydan sonra {what} değişti (onaylanan {i['approved'][:12]}, şimdi {now[:12]}): "
                                     "değişikliği denetleyip yeniden onaylayın.")
            problems += state != "current"
            row["state"] = state
            if why:
                row["why"] = why
            rows.append(row)
        out = {"items": rows, "derivations": derived, "stale": stale, "problems": problems, "deep": deep,
               "revision": "$current"}
        if of is not None:
            out["object"] = self.object(of, deep)
        return out

    def object(self, u, deep):
        held = self.item_of_layer(self.objects[u])
        out = {"uid": u, "role": "free"}
        if held:
            out["item"] = held["id"]
        d = next((d for d in self.derivations() if u in d["uids"]), None)
        if d:
            out.update(role="derived", derivation=d["id"], source=d["source"], revision=d["revision"],
                       operation=d["operation"], current=not self.stale(d, deep))
        elif held and held["kind"] in ("campaign", "reference"):
            out.update(role="source", source=held["id"], revision=self.kept(held))
        return out

    def stale_count(self):
        return sum(self.stale(d, False) for d in self.derivations())

    def set_work(self, work):
        work = {k: v for k, v in work.items() if v}
        if work:
            self.d["settings"]["mapWork"] = work
        else:
            self.d["settings"].pop("mapWork", None)


# ── Steps and answers ────────────────────────────────────────────────────────────────────────────────────────────────

UNCHANGED = {"revision": "same", "canUndo": False}
NOT_MAP = {"status": "failed", "error": {
    "code": "not_a_map_project",
    "message": "Harita işi yalnız harita projesinde (KentOS Harita) olur; bu çizim harita projesi değil."}}
BAD_REVISION = {"status": "failed", "error": {
    "code": "invalid_revision",
    "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
               "ya da komutun planından alın.", "path": "expectedRevision"}}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict",
    "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
               "yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_, result, expect=None, command=None):
    s = {"op": op, "input": input_, "result": result}
    if command:
        s["command"] = command
    if expect is not None:
        s["expect"] = expect
    return s


def case(name, steps, note=None, d=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["steps"] = steps
    if d is not None:
        c["setup"] = d
    return c


def copy_file(src, dst):
    return {"op": "copyFile", "from": src, "to": dst}


def ask(dr, deep=False, of=None, op="execute", expect=None, drawing=None):
    """The query's answer; it writes nothing (after a write in the case, the undo step stays: only the revision)."""
    input_ = {}
    if deep:
        input_["deep"] = True
    if drawing is not None:
        input_["drawing"] = drawing
    if of is not None:
        input_["uid"] = of
    return step(op, input_, done(dr.tree(deep, of, drawing)), expect or UNCHANGED, command="map.work.query")


# ── map.work.query ───────────────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    out = []
    base = setup()
    dr = Drawing(base)
    out.append(case("Bütün öğeler ve türetmeler: dosyalar yerinde, hepsi güncel", [ask(dr)],
                    "Hızlı okuma dosyaya yalnız bakar (found); her öğenin katmanlarındaki nesneler sayılır, grubun "
                    "altındaki katman grubun öğesinindir."))
    out.append(case("Derin okuma her dosyanın SHA-256'sını revizyonuyla karşılaştırır", [ask(dr, deep=True)]))
    for name, u in (("GNSS'ten içe aktarılan nokta türetilmiştir: kaynak, revizyon, işlem, güncel", G1),
                    ("Türetme kaydında olmayan arazi ölçümü noktası kaynaktır", G3),
                    ("Ortofotonun rasteri kaynaktır", R1),
                    ("Grubun altındaki katmanın nesnesi grubun öğesinindir (CAD altlığı)", A1),
                    ("Tasarım yüzeyinin eğrisi arazi ölçümünden türetilmiştir; tasarım öğesinin katmanındadır", T1),
                    ("Hiçbir öğenin katmanında olmayan çizgi kullanıcının çizimidir", F1)):
        out.append(case(name, [ask(dr, of=u)]))
    out.append(case("Kimlik UUID değil", [step("execute", {"uid": "G1"}, refused(
        "invalid_uid", "“G1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uid"),
        UNCHANGED)]))
    out.append(case("Çizimde olmayan nesne", [step("execute", {"uid": GHOST}, refused(
        "entity_not_found", f"“{GHOST}” kimlikli nesne çizimde yok.", "uid"), UNCHANGED)]))
    out.append(case("Harita projesi değil", [step("execute", {}, NOT_MAP, UNCHANGED)],
                    d=setup(work="architecture", map_work=None)))
    empty = setup(map_work=None)
    out.append(case("İş yapısı olmayan harita projesi: boş ağaç", [ask(Drawing(empty))], d=empty))
    gone_file = setup()
    gone_file["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:tasindi-1/kampanya.nmea"
    out.append(case("Arazi ölçümünün dosyası yerinde değil: öğe missing, yeniden bağlanmalı", [ask(Drawing(gone_file))],
                    d=gone_file))
    changed = setup()
    changed["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:degisti-1/kampanya.nmea"
    later = Drawing(changed, {"$tmp:degisti-1/kampanya.nmea": S_LATER})
    out.append(case("Arazi ölçümünün dosyası sonradan değişti: hızlı okuma görmez, derin okuma changed ve türevleri güncel değil",
                    [copy_file(P_LATER, "$tmp:degisti-1/kampanya.nmea"), ask(later), ask(later, deep=True),
                     ask(later, deep=True, of=G2)], d=changed))
    no_layer = setup()
    no_layer["settings"]["mapWork"]["items"][3]["layers"] = ["tasarim", "silinen"]
    out.append(case("Öğenin adlandırdığı katman çizimde yok: incomplete", [ask(Drawing(no_layer))], d=no_layer))
    deleted = Drawing(base)
    del deleted.objects[G2]
    out.append(case("Türetilmiş bir nesne silindi: kaydı missing; geri alınca güncel", [
        step("execute", {"uids": [G2]}, "$any", command="cad.entities.delete"),
        ask(deleted, expect={"revision": "same"}), {"op": "undo", "returns": "Sil"}, ask(dr, expect={"revision": "same"})]))
    moved = setup()
    moved["settings"]["mapWork"]["items"][0]["file"]["revision"] = S_LATER
    moved["settings"]["mapWork"]["items"][3]["status"] = "approved"
    md = Drawing(moved)
    out.append(case("Arazi ölçümü yenilendi: iki türetme güncel değil, onaylı tasarım öğesi stale", [ask(md)], d=moved))
    out.append(case("Güncel olmayan türetmenin nesnesi: current false", [ask(md, of=T2)], d=moved))
    out.append(case("Derin okumada dosya kayıtlı revizyonla aynıysa türetme yine güncel değildir (kayıt öğenin dosyasına göre)",
                    [ask(md, deep=True)], d=moved,
                    note="Öğenin revizyonu dosyanın içeriği değil; derin okuma dosyayı changed der, türetmeyi dosyanın "
                         "bugünkü içeriğiyle karşılaştırır: içerik türetmenin revizyonu, türetme güncel."))
    # ADR 1201 §1: a project moved with its data.
    moved_project = setup()
    f0 = moved_project["settings"]["mapWork"]["items"][0]["file"]
    f0.update(path="$tmp:eski-1/kampanya.nmea", relative="veri/kampanya.nmea")
    there = Drawing(moved_project, {"$tmp:proje-1/veri/kampanya.nmea": S_GNSS})
    out.append(case("Taşınan proje: dosya kayıtlı yolunda yok, proje klasöründen göreli yoluyla bulunur (relative)",
                    [copy_file(P_GNSS, "$tmp:proje-1/veri/kampanya.nmea"),
                     ask(there, drawing="$tmp:proje-1/ana.kcad"), ask(there, deep=True, drawing="$tmp:proje-1/ana.kcad"),
                     ask(there, drawing=None)], d=moved_project,
                    note="Göreli yol yalnız açık projenin dosyası (drawing) verilince aranır; verilmeyince dosya missing."))
    beside = setup()
    beside["settings"]["mapWork"]["items"][0]["file"].update(path="$tmp:eski-2/kampanya.nmea",
                                                            relative="../ortak/kampanya.nmea")
    up = Drawing(beside, {"$tmp:ortak/kampanya.nmea": S_GNSS})
    out.append(case("Göreli yol proje klasörünün dışına da çıkabilir (../ortak)",
                    [copy_file(P_GNSS, "$tmp:ortak/kampanya.nmea"), ask(up, drawing="$tmp:proje-2/ana.kcad")],
                    d=beside))
    searched = setup()
    searched["settings"]["xrefPaths"] = ["$tmp:arama-1"]
    searched["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:eski-3/kampanya.nmea"
    found = Drawing(searched, {"$tmp:arama-1/kampanya.nmea": S_GNSS})
    out.append(case("Göreli yolu olmayan dosya projenin arama yolunda adıyla bulunur (searchPath)",
                    [copy_file(P_GNSS, "$tmp:arama-1/kampanya.nmea"), ask(found, drawing="$tmp:proje-3/ana.kcad")],
                    d=searched))
    newer = setup()
    newer["settings"]["mapWork"]["items"][0]["file"].update(path="$tmp:eski-4/kampanya.nmea",
                                                           relative="veri/kampanya.nmea")
    other = Drawing(newer, {"$tmp:proje-4/veri/kampanya.nmea": S_LATER})
    out.append(case("Göreli yolda başka içerik: derin okuma changed, türevler güncel değil",
                    [copy_file(P_LATER, "$tmp:proje-4/veri/kampanya.nmea"),
                     ask(other, deep=True, drawing="$tmp:proje-4/ana.kcad")], d=newer))
    out.append(case("Açık projenin dosyası mutlak yol değil", [step("execute", {"drawing": "proje/ana.kcad"}, refused(
        "invalid_file_path", "“proje/ana.kcad” mutlak bir yol değil; açık projenin dosyasının tam yolunu verin.",
        "drawing"), UNCHANGED)]))
    # ADR 2080: a design's content revision, a derivation from it and an approval's stamp.
    staked = setup()
    staked["settings"]["mapWork"]["derivations"].append(
        derivation("tr-3", "is-4", Drawing(staked).content("is-4"), "Aplikasyon", [F1]))
    s0 = Drawing(staked)
    s1 = Drawing(staked)
    s1.entities[T1]["attrs"] = {"Kot": "1060"}
    out.append(case("Tasarımdan türetilen çizgi: tasarımın içeriği değişmedikçe güncel; bir çizgisinin özniteliği değişince "
                    "güncel değil; geri alınca yine güncel", [
                        ask(s0), ask(s0, of=F1),
                        step("execute", {"uids": [T1], "attrs": {"Kot": "1060"}, "operation": "attributes"}, "$any",
                             command="cad.entities.set"),
                        ask(s1, expect={"revision": "same"}), ask(s1, of=F1, expect={"revision": "same"}),
                        {"op": "undo", "returns": "Değiştir"}, ask(s0, expect={"revision": "same"})], d=staked,
                    note="Dosyası olmayan öğenin revizyonu katmanlarındaki nesnelerin özetidir (kentos.work-content/1); "
                         "türetme onunla karşılaştırılır."))
    approved = setup()
    approved["settings"]["mapWork"]["items"][3].update(status="approved", approved=Drawing(approved).content("is-4"))
    a0 = Drawing(approved)
    a1 = Drawing(approved)
    a1.entities[T2]["attrs"] = {"Kot": "1061"}
    out.append(case("Onaylı tasarımın damgası: içeriği aynıyken güncel; onaydan sonra bir çizgisi değişince revised", [
        ask(a0), step("execute", {"uids": [T2], "attrs": {"Kot": "1061"}, "operation": "attributes"}, "$any",
                      command="cad.entities.set"),
        ask(a1, expect={"revision": "same"})], d=approved))
    # docs/adr/2880: an approval that kept its objects' digests says what changed since, object by object: here the
    # approved list had the first object with another digest, lacked the second (added since) and held one not there
    # now (removed since).
    later = setup()
    now4 = Drawing(later).pairs("is-4")
    assert len(now4) >= 2, now4
    earlier = sorted([[now4[0][0], "0" * 64]] + [list(x) for x in now4[2:]]
                     + [["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff", "1" * 64]])
    later["settings"]["mapWork"]["items"][3].update(
        status="approved", approved=sha(json.dumps(earlier, separators=(",", ":")).encode()), approvedObjects=earlier)
    lt = Drawing(later)
    row4 = next(r for r in lt.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"] == {"added": 1, "removed": 1, "changed": 1, "uids": sorted([now4[0][0], now4[1][0]])}, row4
    out.append(case("Onaydan sonra değişen nesneler: biri değişti, biri eklendi, biri silindi (onay nesne başına özet "
                    "tuttu)", [ask(lt)], d=later))
    # docs/adr/3280: the same approval with its objects' parts: the changed object's geometry and look parts were others.
    with_parts = copy.deepcopy(later)
    lp = Drawing(with_parts)
    by = {u: parts(lp.entities[u]) for u in lp.entities}
    kept_parts = [list(by[u]) if u in by else ["1" * 16] * 4 for u, _ in earlier]
    kept_parts[[u for u, _ in earlier].index(now4[0][0])][0] = "0" * 16
    kept_parts[[u for u, _ in earlier].index(now4[0][0])][3] = "0" * 16
    with_parts["settings"]["mapWork"]["items"][3]["approvedParts"] = kept_parts
    wp = Drawing(with_parts)
    row4 = next(r for r in wp.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"]["parts"] == {"geometry": 1, "attributes": 0, "layer": 0, "look": 1}, row4
    assert row4["changes"]["objects"] == [{"uid": now4[0][0], "parts": ["geometry", "look"]}], row4
    out.append(case("Onaydan sonra değişen nesnenin değişen parçaları: geometrisi ve görünüşü (onay parçaları tuttu)",
                    [ask(wp)], d=with_parts))
    # An attribute set after an approval that kept the parts: the attributes part, and only it.
    attributed = setup()
    attributed["settings"]["mapWork"]["items"][3].update(status="approved", approved=Drawing(attributed).content("is-4"),
                                                         approvedObjects=Drawing(attributed).pairs("is-4"),
                                                         approvedParts=Drawing(attributed).parts_of("is-4"))
    at0 = Drawing(attributed)
    at1 = Drawing(attributed)
    at1.entities[T2]["attrs"] = {"Kot": "1061"}
    row4 = next(r for r in at1.tree(False)["items"] if r["id"] == "is-4")
    assert row4["changes"]["objects"] == [{"uid": T2, "parts": ["attributes"]}], row4
    out.append(case("Onaydan sonra özniteliği değişen nesne: yalnız öznitelikleri", [
        ask(at0), step("execute", {"uids": [T2], "attrs": {"Kot": "1061"}, "operation": "attributes"}, "$any",
                       command="cad.entities.set"),
        ask(at1, expect={"revision": "same"})], d=attributed))
    same = setup()
    same["settings"]["mapWork"]["items"][3].update(status="approved", approved=Drawing(same).content("is-4"),
                                                   approvedObjects=Drawing(same).pairs("is-4"),
                                                   approvedParts=Drawing(same).parts_of("is-4"))
    out.append(case("Onaylı ve nesneleri aynı: değişen yok, güncel", [ask(Drawing(same))], d=same))
    ortho = setup()
    ortho["settings"]["mapWork"]["items"][1].update(status="approved", approved=S_OLD)
    out.append(case("Onaylı ortofotonun damgası başka bir dosya revizyonu: onaydan sonra dosyası değişti (revised)",
                    [ask(Drawing(ortho))], d=ortho))
    out.append(case("Plan yazmaz, aynı ağacı söyler", [ask(dr, op="plan")]))
    out.append(case("Doğrulama yazmaz", [step("validate", {}, done(None), UNCHANGED)]))
    out.append(case("Doğrulama kimliği de denetler", [step("validate", {"uid": GHOST}, refused(
        "entity_not_found", f"“{GHOST}” kimlikli nesne çizimde yok.", "uid"), UNCHANGED)]))
    return out


# ── map.work.edit ────────────────────────────────────────────────────────────────────────────────────────────────────


def edit(input_, out, expect, op="execute"):
    return step(op, input_, out, expect, command="map.work.edit")


def write_step(before, after, input_, fields, label, warnings=(), extra=None, then_undo=True):
    """The execute step and, by default, its undo back to `before`."""
    expect = {"revision": "changed", "canUndo": True, "settings": {"mapWork": after.d["settings"].get("mapWork")}}
    expect.update(extra or {})
    steps = [edit(input_, done(edited(after, **fields), warnings), expect)]
    if then_undo:
        steps.append({"op": "undo", "returns": label,
                      "expect": {"canRedo": True, "settings": {"mapWork": before.d["settings"].get("mapWork")}}})
    return steps


def fail_step(input_, code, message, path):
    return edit(input_, refused(code, message, path), {"revision": "same", "canUndo": False})


def op(o, **kw):
    return {"operation": dict(o, **kw)}


def edit_cases():
    out = []
    base = setup()
    dr = Drawing(base)

    def changed(fn, files=None):
        nd = Drawing(base, files)
        w = copy.deepcopy(nd.work)
        fn(w)
        nd.set_work(w)
        return nd

    # add
    after = changed(lambda w: w["items"].append(item("is-6", "campaign", "GNSS ölçümü 2026-11", ["bos"],
                                                     (P_LATER, S_LATER))))
    out.append(case("Ekle: dosyasıyla yeni arazi ölçümü is-6, revizyonu dosyanın SHA-256'sı; tek adımda geri alınır",
                    write_step(dr, after, op({"op": "add", "kind": "campaign", "name": "GNSS ölçümü 2026-11",
                                              "layers": ["bos"], "file": P_LATER}),
                               {"item": "is-6", "file_revision": S_LATER}, "Harita işine ekle")))
    after = changed(lambda w: w["items"].append(item("is-6", "delivery", "Teslim paketi A")))
    out.append(case("Ekle: dosyasız, katmansız teslim paketi",
                    write_step(dr, after, op({"op": "add", "kind": "delivery", "name": "Teslim paketi A"}),
                               {"item": "is-6"}, "Harita işine ekle")))
    out.append(case("Ekle: başka öğenin katmanı", [fail_step(
        op({"op": "add", "kind": "working", "name": "Çalışma", "layers": ["bos", "gnss"]}), "layer_in_work_item",
        "“gnss” katmanı harita işinin “GNSS ölçümü 2026-10” öğesinde; bir katman tek öğenin olur.",
        "operation.layers[1]")]))
    out.append(case("Ekle: aynı katman iki kez", [fail_step(
        op({"op": "add", "kind": "working", "name": "Çalışma", "layers": ["bos", "bos"]}), "layer_in_work_item",
        "“bos” katmanı iki kez verilmiş.", "operation.layers[1]")]))
    out.append(case("Ekle: çizimde olmayan katman", [fail_step(
        op({"op": "add", "kind": "working", "name": "Çalışma", "layers": ["yok"]}), "layer_not_found",
        "“yok” kimlikli katman çizimde yok.", "operation.layers[0]")]))
    for name, value in (("boş", ""), ("boşlukla başlayan", " Çalışma")):
        out.append(case(f"Ekle: {name} ad", [fail_step(
            op({"op": "add", "kind": "working", "name": value}), "invalid_work_text",
            "Öğenin adı boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.", "operation.name")]))
    out.append(case("Ekle: göreli yol", [fail_step(
        op({"op": "add", "kind": "reference", "name": "Altlık", "file": "veri/plan.dxf"}), "invalid_file_path",
        "“veri/plan.dxf” mutlak bir dosya yolu değil; dosyanın tam yolunu verin.", "operation.file")]))
    out.append(case("Ekle: olmayan dosya", [fail_step(
        op({"op": "add", "kind": "reference", "name": "Altlık", "file": "$tmp:yok/plan.dxf"}), "file_not_found",
        "“$tmp:yok/plan.dxf” dosyası yok.", "operation.file")]))
    # set
    stamp4 = dr.content("is-4")
    pairs4 = dr.pairs("is-4")
    parts4 = dr.parts_of("is-4")
    # docs/adr/2880: an approval of an item without a file keeps its objects' digests; docs/adr/3280: and their parts.
    after = changed(lambda w: w["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4,
                                                   approvedParts=parts4))
    out.append(case("Durum: denetlenen tasarım onaylı teslim revizyonu olur (ofisin kaydı); onay içerik revizyonunu "
                    "damgalar", write_step(dr, after, op({"op": "set", "item": "is-4", "status": "approved"}),
                                           {"item": "is-4", "content": stamp4, "approved": stamp4},
                                           "Harita işi öğesini değiştir")))
    after = changed(lambda w: w["items"][1].update(status="approved", approved=S_ORTHO))
    out.append(case("Onay: dosyası olan ortofotonun damgası dosya revizyonudur",
                    write_step(dr, after, op({"op": "set", "item": "is-2", "status": "approved"}),
                               {"item": "is-2", "file_revision": S_ORTHO, "approved": S_ORTHO},
                               "Harita işi öğesini değiştir")))
    moved_on = setup()
    moved_on["settings"]["mapWork"]["items"][3].update(status="approved", approved="0" * 64)
    mo = Drawing(moved_on)
    mo_after = Drawing(moved_on)
    w = copy.deepcopy(mo_after.work)
    w["items"][3]["approved"] = stamp4
    w["items"][3]["approvedObjects"] = pairs4
    w["items"][3]["approvedParts"] = parts4
    mo_after.set_work(w)
    out.append(case("Yeniden onayla: onaydan sonra değişen tasarım bugünkü içerik revizyonuyla yeniden damgalanır",
                    write_step(mo, mo_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=moved_on))
    kept = setup()
    kept["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4)
    # docs/adr/2880: an unchanged item approved before its objects were kept gains them when approved again.
    ka = Drawing(kept)
    ka_after = Drawing(kept)
    w = copy.deepcopy(ka_after.work)
    w["items"][3]["approvedObjects"] = pairs4
    w["items"][3]["approvedParts"] = parts4
    ka_after.set_work(w)
    out.append(case("Yeniden onayla: değişmemiş, nesneleri tutulmadan onaylanmış öğe nesne başına özetini kazanır",
                    write_step(ka, ka_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=kept))
    # docs/adr/3280: approved with its objects' digests but not their parts (schema 210): approving again keeps the parts.
    no_parts = setup()
    no_parts["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4)
    np = Drawing(no_parts)
    np_after = Drawing(no_parts)
    w = copy.deepcopy(np_after.work)
    w["items"][3]["approvedParts"] = parts4
    np_after.set_work(w)
    out.append(case("Yeniden onayla: değişmemiş, parçaları tutulmadan (şema 210) onaylanmış öğe parçalarını kazanır",
                    write_step(np, np_after, op({"op": "set", "item": "is-4", "status": "approved"}),
                               {"item": "is-4", "content": stamp4, "approved": stamp4}, "Harita işi öğesini değiştir"),
                    d=no_parts))
    whole = setup()
    whole["settings"]["mapWork"]["items"][3].update(status="approved", approved=stamp4, approvedObjects=pairs4,
                                                    approvedParts=parts4)
    out.append(case("Yeniden onayla: değişmemiş onaylı öğe", [fail_step(
        op({"op": "set", "item": "is-4", "status": "approved"}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")], d=whole))
    kd = Drawing(whole)
    kd_after = Drawing(whole)
    w = copy.deepcopy(kd_after.work)
    w["items"][3]["status"] = "review"
    del w["items"][3]["approved"]
    # docs/adr/2880: its objects' digests go with the stamp; docs/adr/3280: and their parts.
    del w["items"][3]["approvedObjects"]
    del w["items"][3]["approvedParts"]
    kd_after.set_work(w)
    out.append(case("Onaydan çıkan öğenin damgası ve nesne başına özeti kalkar",
                    write_step(kd, kd_after, op({"op": "set", "item": "is-4", "status": "review"}), {"item": "is-4"},
                               "Harita işi öğesini değiştir"), d=whole))
    after = changed(lambda w: w["items"][1].update(name="Ortofoto 2025 (30 cm)", layers=["ortofoto", "bos"]))
    out.append(case("Değiştir: ortofotonun adı ve katmanları",
                    write_step(dr, after, op({"op": "set", "item": "is-2", "name": "Ortofoto 2025 (30 cm)",
                                              "layers": ["ortofoto", "bos"]}),
                               {"item": "is-2", "file_revision": S_ORTHO}, "Harita işi öğesini değiştir")))
    out.append(case("Değiştir: aynı değerler", [fail_step(
        op({"op": "set", "item": "is-4", "status": "review"}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")]))
    out.append(case("Değiştir: olmayan öğe", [fail_step(
        op({"op": "set", "item": "is-9", "status": "review"}), "unknown_work_item",
        "Harita işinde “is-9” kimlikli öğe yok.", "operation.item")]))
    renewed = setup()
    renewed["settings"]["mapWork"]["items"][0]["file"]["revision"] = S_LATER
    out.append(case("Onay: katmanlarında güncel olmayan türetilmiş nesne olan öğe onaylanmaz", [fail_step(
        op({"op": "set", "item": "is-4", "status": "approved"}), "approval_stale",
        "“Tasarım yüzeyi” onaylanamaz: katmanlarındaki 2 türetilmiş nesnenin kaynağı değişti; önce onları güncelleyin.",
        "operation.status")], d=renewed))
    # remove
    after = changed(lambda w: (w["items"].pop(0), w.update(derivations=[])))
    out.append(case("Çıkar: arazi ölçümü ve ondan türetme kayıtları gider, nesneler kalır",
                    write_step(dr, after, op({"op": "remove", "item": "is-1"}), {"item": "is-1", "released": 2},
                               "Harita işinden çıkar", extra={"ids": list(range(1, 10))})))
    # relink
    moved = "$tmp:tasindi-2/i8.tif"
    after = changed(lambda w: w["items"][1]["file"].update(path=moved), {moved: S_ORTHO})
    steps = [copy_file(P_ORTHO, moved)]
    steps += write_step(dr, after, op({"op": "relink", "item": "is-2", "file": moved}),
                        {"item": "is-2", "file_revision": S_ORTHO, "relinked": 1}, "Kaynağı yeniden bağla",
                        extra={"fields": {"4": {"file": moved}}})
    steps[-1]["expect"]["fields"] = {"4": {"file": P_ORTHO}}
    out.append(case("Yeniden bağla: taşınan ortofoto tek yerden; rasteri yeni dosyayı gösterir, içerik aynı, türev "
                    "güncel; geri alma ikisini birlikte geri getirir", steps))
    after = changed(lambda w: w["items"][0]["file"].update(path=P_LATER, revision=S_LATER))
    out.append(case("Yeniden bağla: içeriği farklı yeni kayıt dosyası; iki türetme güncel değil",
                    write_step(dr, after, op({"op": "relink", "item": "is-1", "file": P_LATER}),
                               {"item": "is-1", "file_revision": S_LATER}, "Kaynağı yeniden bağla",
                               warnings=[{"code": "source_content_changed", "path": "operation.file", "message":
                                          "Yeni dosyanın içeriği öncekinden farklı: “GNSS ölçümü 2026-10” "
                                          "kaynağından türetilmiş 2 kayıt güncel değil."}])
                    + [{"op": "redo", "returns": "Kaynağı yeniden bağla"}, ask(after, of=G1, expect={"revision": "same"})]))
    out.append(case("Yeniden bağla: aynı dosya", [fail_step(
        op({"op": "relink", "item": "is-1", "file": P_GNSS}), "nothing_to_set", "Değişecek bir şey yok.",
        "operation")]))
    out.append(case("Yeniden bağla: dosyası olmayan öğe", [fail_step(
        op({"op": "relink", "item": "is-4", "file": P_GNSS}), "work_item_no_file",
        "“Tasarım yüzeyi” öğesinin dosyası yok; yalnız dosyası olan bir kaynak yeniden bağlanır ya da yenilenir.",
        "operation.item")]))
    out.append(case("Yenile: dosyası olmayan öğe", [fail_step(
        op({"op": "refresh", "item": "is-4"}), "work_item_no_file",
        "“Tasarım yüzeyi” öğesinin dosyası yok; yalnız dosyası olan bir kaynak yeniden bağlanır ya da yenilenir.",
        "operation.item")]))
    old_moved = "$tmp:tasindi-3/geographic.tif"
    out.append(case("Yeniden bağla: rasteri kilitli katmanda", [copy_file(P_OLD, old_moved), fail_step(
        op({"op": "relink", "item": "is-5", "file": old_moved}), "layer_locked",
        "“Kilitli” katmanı kilitli: üstündeki resim yeni dosyaya bağlanamaz. Kilidi açın.", "operation.item")]))
    out.append(case("Yeniden bağla: yeni yerde dosya yok", [fail_step(
        op({"op": "relink", "item": "is-2", "file": "$tmp:yok/i8.tif"}), "file_not_found",
        "“$tmp:yok/i8.tif” dosyası yok.", "operation.file")]))
    # refresh
    refresh = setup()
    refresh["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:yenile-1/kampanya.nmea"
    rd = Drawing(refresh, {"$tmp:yenile-1/kampanya.nmea": S_LATER})
    rafter = Drawing(refresh, {"$tmp:yenile-1/kampanya.nmea": S_LATER})
    w = copy.deepcopy(rafter.work)
    w["items"][0]["file"]["revision"] = S_LATER
    rafter.set_work(w)
    out.append(case("Yenile: dosyanın bugünkü içeriği revizyon olur; türevleri güncel değil",
                    [copy_file(P_LATER, "$tmp:yenile-1/kampanya.nmea")]
                    + write_step(rd, rafter, op({"op": "refresh", "item": "is-1"}),
                                 {"item": "is-1", "file_revision": S_LATER}, "Kaynağı yenile"), d=refresh))
    out.append(case("Yenile: dosya değişmedi", [fail_step(
        op({"op": "refresh", "item": "is-1"}), "nothing_to_set", "Değişecek bir şey yok.", "operation")]))
    gone = setup()
    gone["settings"]["mapWork"]["items"][0]["file"]["path"] = "$tmp:tasindi-4/kampanya.nmea"
    out.append(case("Yenile: dosya yok", [fail_step(
        op({"op": "refresh", "item": "is-1"}), "file_not_found",
        "“$tmp:tasindi-4/kampanya.nmea” dosyası yok; Yeniden bağla ile yeni yerini gösterin.", "operation.item")],
        d=gone))
    # derive
    after = changed(lambda w: w["derivations"].append(derivation("tr-3", "is-3", S_DXF, "Altlıktan kopya", [F1])))
    out.append(case("Türet: çizgi CAD altlığından kopyalandı diye kaydedilir; nesnenin kaynağı bulunur",
                    write_step(dr, after, op({"op": "derive", "source": "is-3", "uids": [F1, F1],
                                              "operation": "Altlıktan kopya"}),
                               {"item": "is-3", "derivation": "tr-3", "file_revision": S_DXF}, "Kaynaktan türet")
                    + [{"op": "redo", "returns": "Kaynaktan türet"}, ask(after, of=F1, expect={"revision": "same"})]))
    out.append(case("Türet: başka kayıttaki nesne", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [F1, G1], "operation": "Altlıktan kopya"}), "already_derived",
        f"“{G1}” nesnesi “tr-1” türetme kaydında; önce o kaydı silin.", "operation.uids[1]")]))
    after = changed(lambda w: w["derivations"].append(derivation("tr-3", "is-4", stamp4, "Aplikasyon", [F1])))
    out.append(case("Türet: aplikasyon çizgisi dosyası olmayan tasarımdan türetilir; revizyonu tasarımın içerik revizyonu",
                    write_step(dr, after, op({"op": "derive", "source": "is-4", "uids": [F1], "operation": "Aplikasyon"}),
                               {"item": "is-4", "derivation": "tr-3", "content": stamp4}, "Kaynaktan türet")
                    + [{"op": "redo", "returns": "Kaynaktan türet"}, ask(after, of=F1, expect={"revision": "same"})]))
    own = setup()
    own["settings"]["mapWork"]["derivations"].pop(1)
    out.append(case("Türet: dosyası olmayan öğenin kendi katmanındaki nesne", [fail_step(
        op({"op": "derive", "source": "is-4", "uids": [F1, T1], "operation": "Kopya"}), "own_content",
        f"“{T1}” nesnesi “Tasarım yüzeyi” öğesinin kendi katmanında; dosyası olmayan öğe kendi nesnelerinin kaynağı "
        "olamaz.", "operation.uids[1]")], d=own))
    out.append(case("Türet: olmayan kaynak", [fail_step(
        op({"op": "derive", "source": "is-9", "uids": [F1], "operation": "Kopya"}), "unknown_work_item",
        "Harita işinde “is-9” kimlikli öğe yok.", "operation.source")]))
    out.append(case("Türet: nesne yok", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [], "operation": "Kopya"}), "no_entities",
        "Türetilen nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "operation.uids")]))
    out.append(case("Türet: kimlik UUID değil", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": ["F1"], "operation": "Kopya"}), "invalid_uid",
        "“F1” geçerli bir nesne kimliği değil.", "operation.uids[0]")]))
    out.append(case("Türet: çizimde olmayan nesne", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [GHOST], "operation": "Kopya"}), "entity_not_found",
        f"“{GHOST}” kimlikli nesne çizimde yok.", "operation.uids[0]")]))
    out.append(case("Türet: boşluklu işlem adı", [fail_step(
        op({"op": "derive", "source": "is-3", "uids": [F1], "operation": "Kopya "}), "invalid_work_text",
        "Üretim işlemi boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.", "operation.operation")]))
    # forget
    after = changed(lambda w: w["derivations"].pop(0))
    out.append(case("Kaydı sil: noktalar yine arazi ölçümünün, artık kaynak olarak",
                    write_step(dr, after, op({"op": "forget", "derivation": "tr-1"}), {"derivation": "tr-1"},
                               "Türetme kaydını sil")
                    + [{"op": "redo", "returns": "Türetme kaydını sil"}, ask(after, of=G1, expect={"revision": "same"})]))
    out.append(case("Kaydı sil: olmayan kayıt", [fail_step(
        op({"op": "forget", "derivation": "tr-9"}), "unknown_derivation",
        "Harita işinde “tr-9” kimlikli türetme kaydı yok.", "operation.derivation")]))
    # project, revision, plan
    out.append(case("Harita projesi değil", [edit(op({"op": "add", "kind": "delivery", "name": "Paket"}), NOT_MAP,
                                                   {"revision": "same", "canUndo": False})],
                    d=setup(work="architecture", map_work=None)))
    out.append(case("İş yapısı olmayan harita projesinde ilk öğe", write_step(
        Drawing(setup(map_work=None)),
        Drawing(setup(map_work={"items": [item("is-1", "reference", "Ortofoto", ["ortofoto"], (P_ORTHO, S_ORTHO))]})),
        op({"op": "add", "kind": "reference", "name": "Ortofoto", "layers": ["ortofoto"], "file": P_ORTHO}),
        {"item": "is-1", "file_revision": S_ORTHO}, "Harita işine ekle"), d=setup(map_work=None)))
    out.append(case("Geçersiz beklenen sürüm", [edit(dict(op({"op": "refresh", "item": "is-1"}), expectedRevision="abc"),
                                                      BAD_REVISION, {"revision": "same", "canUndo": False})]))
    out.append(case("Beklenen sürüm eski", [edit(dict(op({"op": "refresh", "item": "is-1"}), expectedRevision="999"),
                                                  CONFLICT, {"revision": "same", "canUndo": False})]))
    # release (ADR 1201 §3)
    after = changed(lambda w: w["derivations"][0].update(uids=[G1]))
    out.append(case("Bağımsız yap: G2 kaydından çıkar, kayıt G1 ile kalır; tek adımda geri alınır",
                    write_step(dr, after, op({"op": "release", "uids": [G2]}), {"freed": 1}, "Bağımsız yap")
                    + [{"op": "redo", "returns": "Bağımsız yap"}, ask(after, of=G2, expect={"revision": "same"})]))
    after = changed(lambda w: w.update(derivations=[w["derivations"][0]]))
    out.append(case("Bağımsız yap: kaydın bütün nesneleri; boşalan kayıt kalkar",
                    write_step(dr, after, op({"op": "release", "uids": [T1, T2, T1]}), {"freed": 2, "released": 1},
                               "Bağımsız yap")))
    after = changed(lambda w: w.update(derivations=[]))
    out.append(case("Bağımsız yap: iki kayıttan nesneler birlikte",
                    write_step(dr, after, op({"op": "release", "uids": [G1, T1, G2, T2]}), {"freed": 4, "released": 2},
                               "Bağımsız yap")))
    out.append(case("Bağımsız yap: kullanıcının kendi çizimi", [fail_step(
        op({"op": "release", "uids": [G1, F1]}), "not_derived",
        f"“{F1}” nesnesi hiçbir türetme kaydında değil; zaten bağımsız çizim.", "operation.uids[1]")]))
    out.append(case("Bağımsız yap: kaynak nokta türetilmiş değil", [fail_step(
        op({"op": "release", "uids": [G3]}), "not_derived",
        f"“{G3}” nesnesi hiçbir türetme kaydında değil; zaten bağımsız çizim.", "operation.uids[0]")]))
    out.append(case("Bağımsız yap: nesne verilmedi", [fail_step(
        op({"op": "release", "uids": []}), "no_entities",
        "Bağımsız yapılacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "operation.uids")]))
    out.append(case("Bağımsız yap: kimlik UUID değil", [fail_step(
        op({"op": "release", "uids": ["G1"]}), "invalid_uid", "“G1” geçerli bir nesne kimliği değil.",
        "operation.uids[0]")]))
    # relink takes along the external references on the old file (ADR 1201 §2)
    with_refs = setup()
    with_refs["blocks"] = [xref_block(XREF_SAME, "Kadastro", P_DXF, S_DXF),
                           xref_block(XREF_OTHER, "Kadastro eski", P_DXF, S_LATER),
                           xref_block(XREF_ELSE, "Başka plan", P_GNSS, S_GNSS)]
    rd = Drawing(with_refs)
    moved_dxf = "$tmp:tasindi-3/kadastro.dxf"
    after_refs = Drawing(with_refs, {moved_dxf: S_DXF})
    w = copy.deepcopy(after_refs.work)
    w["items"][2]["file"]["path"] = moved_dxf
    after_refs.set_work(w)
    steps = [copy_file(P_DXF, moved_dxf)]
    steps += write_step(rd, after_refs, op({"op": "relink", "item": "is-3", "file": moved_dxf}),
                        {"item": "is-3", "file_revision": S_DXF, "links": 1}, "Kaynağı yeniden bağla",
                        warnings=[{"code": "link_not_followed", "path": "operation.file", "message":
                                   "“Kadastro eski” dış başvurusu başka bir içerik yüklemiş; yeni dosyaya Dış başvurular › "
                                   "Yolu değiştir ile geçirin."}],
                        extra={"xrefs": {"Kadastro": xref_of(moved_dxf, S_DXF), "Kadastro eski": xref_of(P_DXF, S_LATER),
                                         "Başka plan": xref_of(P_GNSS, S_GNSS)}})
    steps[-1]["expect"]["xrefs"] = {"Kadastro": xref_of(P_DXF, S_DXF)}
    out.append(case("Yeniden bağla: aynı içeriği yükleyen dış başvuru öğeyle birlikte yeni dosyaya geçer, başka içerik "
                    "yükleyen kalır ve söylenir; geri alma hepsini birlikte geri getirir", steps, d=with_refs))
    plan_after = changed(lambda w: w["items"].append(item("is-6", "check", "Topoloji denetimi")))
    for o in ("plan", "validate"):
        result = done(edited(plan_after, item="is-6")) if o == "plan" else done(None)
        out.append(case(f"{'Plan' if o == 'plan' else 'Doğrulama'} yazmaz", [edit(
            op({"op": "add", "kind": "check", "name": "Topoloji denetimi"}), result,
            {"revision": "same", "canUndo": False, "settings": {"mapWork": dr.work}}, op=o)]))
    return out


def edited(dr, item=None, derivation=None, file_revision=None, relinked=0, released=0, links=0, freed=0, content=None,
           approved=None):
    o = {}
    if item is not None:
        o["item"] = item
    if derivation is not None:
        o["derivation"] = derivation
    if file_revision is not None:
        o["fileRevision"] = file_revision
    if content is not None:
        o["content"] = content
    if approved is not None:
        o["approved"] = approved
    if relinked:
        o["relinked"] = relinked
    if released:
        o["released"] = released
    if links:
        o["links"] = links
    if freed:
        o["freed"] = freed
    o["stale"] = dr.stale_count()
    o["revision"] = "$current"
    return o


def doc(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": setup(), "cases": cases}


NOTE = ("ADR 1200. Bağımsız başvuru scripts/fixtures/map_work_cases.py: kurallar ve cümleler ADR'den elle yazılır; "
        "revizyon dosyanın SHA-256'sı (hashlib); kaynak dosyaları fixtures/map-work/v1 ve fixtures/raster/v1/files; "
        "KentOS kodu yok.")


def main():
    texts = {
        OUT / "map.work.query.json": doc("map.work.query", "Harita işini sorgula", query_cases()),
        OUT / "map.work.edit.json": doc("map.work.edit", "Harita işini düzenle", edit_cases()),
    }
    texts = {p: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for p, v in texts.items()}
    files = {FILES / name: data for name, data in WRITTEN.items()}
    if "--check" in sys.argv[1:]:
        bad = [p for p, b in files.items() if not p.exists() or p.read_bytes() != b]
        bad += [p for p, t in texts.items() if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if bad:
            return 1
        print("Harita işi ortak durumları güncel.")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for p, b in files.items():
        p.write_bytes(b)
    for p, t in texts.items():
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
