#!/usr/bin/env python3
"""Shared cases of `cad.bcf.read` and `cad.bcf.write` (plan/ifc-uyum.md IFC-10, docs/adr/8103), written without KentOS
code.

Reading: the BCF files are IfcOpenShell's BCF library's (bcf-client, its viewer's BCF engine: a 3.0 file with two topics,
a 2.1 file) and broken ones made here with Python's zipfile; what each holds is read here with Python's own zip and XML
readers and set down by the ADR's rules (topics by their folders' names; a 3.0 file's comments and viewpoints in its
topic, a 2.1 file's beside it; texts trimmed, an empty one absent; the selected objects once each by GlobalId, led to the
IFC file named by IfcOpenShell and to the drawing's object when the GlobalId is the compressed form of one of its
persistent ids).

Writing: each case writes a file and reads it back (`cad.bcf.read` in the same case): the topics as given, `Issue` and
`Open` where unsaid, the model file's name, the comments' and the viewpoint's GUIDs made by the ADR's rule (FNV-1a over
128 bits of the topic's GUID and the item's number, written as a UUID of version 8: the viewpoint 1, the comments 1000 +
their order), the viewpoint selecting the topic's objects with the rest visible, its camera the ADR's: perspective, 60°,
16:9, from the south-west and above along (1, 1, −1), at 1.2 × the box's diagonal (at least 1 m) over 2 tan 30° from the
box's middle, the box the objects' bodies' by IfcOpenShell's geometry.

`--files` makes the bcf-client files anew (new GUIDs and dates); otherwise they are read as kept. Writes
fixtures/ifc/v1/bcf/*.bcf (with --files) and fixtures/commands/v1/cad.bcf.{read,write}.json; --check compares. Runs under
IfcOpenShell and its BCF library, offline:

    uv run --offline --no-project --python 3.11 --with bcf-client==0.9.0 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/bcf_cases.py [--files] [--check]
"""

import copy
import io
import json
import math
import sys
import uuid
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.guid

ROOT = Path(__file__).resolve().parents[2]
FILES = ROOT / "fixtures/ifc/v1/bcf"
REL = "ifc/v1/bcf/"
MODEL = "ifc/v1/revizyon/kesit-binasi-r2.ifc"
OUT_READ = ROOT / "fixtures/commands/v1/cad.bcf.read.json"
OUT_WRITE = ROOT / "fixtures/commands/v1/cad.bcf.write.json"
MOVED = "01alMXGa9m080000000007"
RATED = "01alMXGa9m080000000002"
DRAWN = "0192f5a1-4242-7000-8000-000000000007"
WITHIN = 1e-5
SAME = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
# A drawing holding the wall whose GlobalId KentOS wrote as 01alMXGa9m080000000007.
SETUP = {
    "format": "kentos.document", "version": 2, "name": "BCF",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 50.0, "workspace": "cad"},
    "origin": {"x": 0.0, "y": 0.0},
    "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
    "activeLayer": "0",
    "entities": [{"kind": "wall", "id": 1, "layerId": "0", "attrs": {},
                  "pts": [{"x": 0.275, "y": 4.5}, {"x": 4.275, "y": 4.5}], "width": 0.15, "justify": "center"}],
    "uids": [DRAWN],
    "styles": {"items": [], "categories": []},
}


# The bcf-client files.
def made_files():
    from bcf.v3 import model as m3
    from bcf.v3.bcfxml import BcfXml as Bcf3
    from bcf.v2 import model as m2
    from bcf.v2.bcfxml import BcfXml as Bcf2
    from xsdata.models.datatype import XmlDateTime

    ifc = ifcopenshell.open(str(ROOT / "fixtures" / MODEL))
    b = Bcf3.create_new("Kesit binası")
    t = b.add_topic("Duvar kuzeye kaymış", "İç duvar ikinci revizyonda 50 cm kuzeyde.", "statik@buro", "Clash", "Open")
    t.markup.topic.priority = "High"
    t.markup.topic.labels = m3.TopicLabels(label=["Statik", "Mimari"])
    t.markup.topic.assigned_to = "mimar@buro"
    vp = t.add_viewpoint(ifc.by_guid(MOVED))
    t.markup.topic.comments = m3.TopicComments(comment=[m3.Comment(
        date=XmlDateTime.from_string("2026-11-17T13:00:00+03:00"), author="mimar@buro", comment="Bakıyorum.",
        guid=str(uuid.uuid4()), viewpoint=m3.CommentViewpoint(guid=vp.guid))])
    u = b.add_topic("Doğu duvarının yangın sınıfı", "REI 60 yerine REI 90.", "statik@buro", "Issue", "Closed")
    u.add_viewpoint(ifc.by_guid(RATED))
    b.save(FILES / "disaridan-v3.bcf")
    b2 = Bcf2.create_new("Kesit binası")
    w = b2.add_topic("Kolon kirişi kesiyor", "Eski sürümün sorunu.", "statik@buro", "Clash", "Active")
    w.add_viewpoint(ifc.by_guid(MOVED))
    w.markup.comment = [m2.Comment(date=XmlDateTime.from_string("2026-11-17T10:01:00"), author="mimar@buro",
                                   comment="Bakıyorum", guid=str(uuid.uuid4()))]
    b2.save(FILES / "disaridan-v21.bcf")

    def zipped(name, entries):
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
            for n, data in entries:
                info = zipfile.ZipInfo(n, date_time=(1980, 1, 1, 0, 0, 0))
                z.writestr(info, data)
        (FILES / name).write_bytes(buf.getvalue())

    version = lambda v: f'<?xml version="1.0" encoding="UTF-8"?><Version VersionId="{v}"/>'  # noqa: E731
    markup = ('<?xml version="1.0" encoding="UTF-8"?><Markup><Topic Guid="0192f5a1-8103-7000-8000-0000000000aa" '
              'TopicType="Issue" TopicStatus="Open"><Title>Bakışı eksik</Title><CreationDate>2026-11-17T12:00:00Z'
              '</CreationDate><CreationAuthor>a</CreationAuthor><Viewpoints><ViewPoint Guid="0192f5a1-8103-7000-8000-'
              '0000000000bb"><Viewpoint>yok.bcfv</Viewpoint></ViewPoint></Viewpoints></Topic></Markup>')
    zipped("surumsuz.bcf", [("0192f5a1-8103-7000-8000-0000000000aa/markup.bcf", markup)])
    zipped("surum-1.bcf", [("bcf.version", version("1.0"))])
    zipped("bakissiz.bcf", [("bcf.version", version("3.0")), ("0192f5a1-8103-7000-8000-0000000000aa/markup.bcf", markup)])
    (FILES / "zip-degil.bcf").write_text("BCF değil", encoding="utf-8")


# Reading, by Python's zip and XML readers.
def tag(e):
    return e.tag.rsplit("}", 1)[-1]


def child(e, name):
    if e is None:
        return None
    return next((c for c in e if tag(c) == name), None)


def children(e, name):
    return [c for c in e if tag(c) == name] if e is not None else []


def text(e, name):
    c = child(e, name)
    t = (c.text or "").strip() if c is not None else ""
    return t or None


def point(e, name):
    p = child(e, name)
    return [float(text(p, a)) for a in ("X", "Y", "Z")]


def guids(e):
    return [c.get("IfcGuid").strip() for c in children(e, "Component") if (c.get("IfcGuid") or "").strip()]


def read_bcf(path):
    z = zipfile.ZipFile(path)
    names = z.namelist()
    version = ET.fromstring(z.read("bcf.version")).get("VersionId").strip()
    three = version == "3.0"
    topics = []
    for folder in sorted(n[: -len("/markup.bcf")] for n in names if n.endswith("/markup.bcf") and n.count("/") == 1):
        root = ET.fromstring(z.read(folder + "/markup.bcf"))
        t = child(root, "Topic")
        header = child(root, "Header")
        file_list = child(header, "Files") if three else header
        files = [text(f, "Filename") or text(f, "Reference") for f in children(file_list, "File")]
        holder = t if three else root
        comment_nodes = children(child(holder, "Comments"), "Comment") if three else children(holder, "Comment")
        comments = []
        for c in comment_nodes:
            out = {"guid": c.get("Guid", ""), "date": text(c, "Date") or "", "author": text(c, "Author") or ""}
            if text(c, "Comment"):
                out["text"] = text(c, "Comment")
            v = child(c, "Viewpoint")
            if v is not None and v.get("Guid"):
                out["viewpoint"] = v.get("Guid")
            comments.append(out)
        view_nodes = children(child(holder, "Viewpoints"), "ViewPoint") if three else children(holder, "Viewpoints")
        viewpoints = []
        for v in view_nodes:
            file = text(v, "Viewpoint") or "viewpoint.bcfv"
            info = ET.fromstring(z.read(f"{folder}/{file}"))
            comps = child(info, "Components")
            vis = child(comps, "Visibility")
            visible = (vis.get("DefaultVisibility", "").strip() in ("true", "1")) if vis is not None else False
            out = {"guid": info.get("Guid") or v.get("Guid", ""), "selection": guids(child(comps, "Selection")),
                   "visible": visible, "exceptions": guids(child(vis, "Exceptions"))}
            cam = child(info, "PerspectiveCamera")
            if cam is not None:
                out["camera"] = {"kind": "perspective", "eye": point(cam, "CameraViewPoint"),
                                 "direction": point(cam, "CameraDirection"), "up": point(cam, "CameraUpVector"),
                                 "fieldOfView": float(text(cam, "FieldOfView") or 60.0),
                                 "aspect": float(text(cam, "AspectRatio") or 1.0)}
            cam = child(info, "OrthogonalCamera")
            if cam is not None and "camera" not in out:
                out["camera"] = {"kind": "orthogonal", "eye": point(cam, "CameraViewPoint"),
                                 "direction": point(cam, "CameraDirection"), "up": point(cam, "CameraUpVector"),
                                 "scale": float(text(cam, "ViewToWorldScale") or 1.0),
                                 "aspect": float(text(cam, "AspectRatio") or 1.0)}
            if text(v, "Snapshot"):
                out["snapshot"] = text(v, "Snapshot")
            viewpoints.append(out)
        labels = ([(x.text or "").strip() for x in children(child(t, "Labels"), "Label")] if three
                  else [(x.text or "").strip() for x in children(t, "Labels")])
        topic = {"guid": t.get("Guid") or folder, "topicType": t.get("TopicType", ""), "status": t.get("TopicStatus", ""),
                 "title": text(t, "Title") or "", "labels": labels, "created": text(t, "CreationDate") or "",
                 "author": text(t, "CreationAuthor") or "", "files": [f for f in files if f],
                 "comments": comments, "viewpoints": viewpoints}
        for key, name in [("priority", "Priority"), ("modified", "ModifiedDate"), ("assignedTo", "AssignedTo"),
                          ("description", "Description")]:
            if text(t, name):
                topic[key] = text(t, name)
        topics.append(topic)
    return version, topics


def objects_of(topics, model=True, drawn=True):
    ifc = ifcopenshell.open(str(ROOT / "fixtures" / MODEL)) if model else None
    out = []
    for g in sorted({g for t in topics for v in t["viewpoints"] for g in v["selection"]}):
        o = {"globalId": g}
        if ifc is not None:
            try:
                e = ifc.by_guid(g)
                o["model"] = {"globalId": g, "class": e.is_a(), "name": e.Name} if e.Name else {"globalId": g, "class": e.is_a()}
            except RuntimeError:
                pass
        if drawn and str(uuid.UUID(ifcopenshell.guid.expand(g))) == DRAWN:
            o["uid"] = DRAWN
        out.append(o)
    return out


# The ADR's rules for what is written.
def fnv(seed, k):
    h = 0x6C62272E07BB014262B821756295C58D
    for b in seed.encode("utf-8") + k.to_bytes(8, "little"):
        h ^= b
        h = (h * 0x0000000001000000000000000000013B) & ((1 << 128) - 1)
    return h


def guid_from(seed, k):
    n = fnv(seed, k)
    n = (n & ~(0xF << 76) & ~(0x3 << 62)) | (0x8 << 76) | (0x2 << 62)
    h = f"{n:032x}"
    return f"{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}"


def box_of(gids):
    ifc = ifcopenshell.open(str(ROOT / "fixtures" / MODEL))
    s = ifcopenshell.geom.settings()
    s.set("use-world-coords", True)
    b = None
    for g in gids:
        v = ifcopenshell.geom.create_shape(s, ifc.by_guid(g)).geometry.verts
        xs, ys, zs = v[0::3], v[1::3], v[2::3]
        k = [min(xs), min(ys), min(zs), max(xs), max(ys), max(zs)]
        b = k if b is None else [min(b[i], k[i]) for i in range(3)] + [max(b[i + 3], k[i + 3]) for i in range(3)]
    return b


def camera(b):
    middle = [(b[0] + b[3]) / 2, (b[1] + b[4]) / 2, (b[2] + b[5]) / 2]
    diagonal = max(math.sqrt((b[3] - b[0]) ** 2 + (b[4] - b[1]) ** 2 + (b[5] - b[2]) ** 2), 1.0)
    k = 1 / math.sqrt(3)
    d = [k, k, -k]
    dist = diagonal / (2 * math.tan(math.radians(30))) * 1.2
    eye = [middle[i] - d[i] * dist for i in range(3)]
    up = [-d[2] * d[0], -d[2] * d[1], 1 - d[2] * d[2]]
    n = math.sqrt(sum(x * x for x in up))
    return {"kind": "perspective", "eye": eye, "direction": d, "up": [x / n for x in up], "fieldOfView": 60.0,
            "aspect": 16 / 9}


def written_topic(t):
    """What reading back gives for a topic written from `t` (the input)."""
    out = {"guid": t["guid"], "topicType": t.get("topicType", "Issue"), "status": t.get("status", "Open"),
           "title": t["title"].strip(), "labels": t.get("labels", []), "created": t["date"], "author": t["author"].strip(),
           "files": ["kesit-binasi-r2.ifc"] if t.get("model", True) else [],
           "comments": [{"guid": guid_from(t["guid"], 1000 + k), "date": c.get("date", t["date"]), "author": c["author"],
                         "text": c["text"]} for k, c in enumerate(t.get("comments", []))],
           "viewpoints": []}
    for key in ("priority", "assignedTo", "description"):
        if key in t:
            out[key] = t[key]
    if t.get("objects"):
        out["viewpoints"] = [{"guid": guid_from(t["guid"], 1), "selection": t["objects"], "visible": True,
                              "exceptions": [], "camera": camera(box_of(t["objects"]))}]
    return out


def main():
    check = "--check" in sys.argv
    if "--files" in sys.argv:
        FILES.mkdir(parents=True, exist_ok=True)
        made_files()
    model = "$fixture:" + MODEL

    # cad.bcf.read
    read_cases = []

    def read_ok(title, name, with_model=True):
        version, topics = read_bcf(FILES / name)
        inp = {"path": "$fixture:" + REL + name}
        if with_model:
            inp["model"] = model
        out = {"path": inp["path"], "version": version, "topics": topics, "objects": objects_of(topics, with_model)}
        read_cases.append({"name": title, "steps": [{"op": "execute", "input": inp, "within": WITHIN, "expect": SAME,
                                                      "result": {"status": "completed", "output": out, "warnings": []}}]})

    def refused(cases, title, inp, code, at, command=None):
        s = {"op": "execute", "input": inp, "expect": SAME,
             "result": {"status": "failed", "error": {"code": code, "message": "$any", "path": at}}}
        if command:
            s["command"] = command
        cases.append({"name": title, "steps": [s]})

    read_ok("Başka programın BCF 3.0 dosyası: iki konu, modelde ve çizimde nesneleri", "disaridan-v3.bcf")
    read_ok("Modelsiz: nesneler yalnız GlobalId ve çizimdeki karşılığıyla", "disaridan-v3.bcf", with_model=False)
    read_ok("BCF 2.1 dosyası: yorum ve bakış konunun yanında", "disaridan-v21.bcf")
    read_ok("BCF 2.1 dosyası modelsiz", "disaridan-v21.bcf", with_model=False)
    read_ok("KentOS'un yazdığı sorun", "../revizyon/tasinan-duvar.bcf")
    read_ok("KentOS'un yazdığı sorun modelsiz: duvar çizimde bulunur", "../revizyon/tasinan-duvar.bcf", with_model=False)
    refused(read_cases, "Yol boş", {"path": ""}, "invalid_file_path", "path")
    refused(read_cases, "Yol göreli", {"path": "sorunlar.bcf"}, "invalid_file_path", "path")
    refused(read_cases, "BCF uzantısı değil", {"path": "$fixture:" + MODEL}, "invalid_file_path", "path")
    refused(read_cases, "Dosya yok", {"path": "$fixture:" + REL + "yok.bcf"}, "invalid_file_path", "path")
    refused(read_cases, "Model .ifc değil", {"path": "$fixture:" + REL + "disaridan-v3.bcf",
                                            "model": "$fixture:" + REL + "disaridan-v21.bcf"}, "invalid_file_path", "model")
    refused(read_cases, "Model yok", {"path": "$fixture:" + REL + "disaridan-v3.bcf", "model": "$fixture:ifc/v1/yok.ifc"},
            "invalid_file_path", "model")
    refused(read_cases, "Zip değil", {"path": "$fixture:" + REL + "zip-degil.bcf"}, "bcf_unreadable", "path")
    refused(read_cases, "Sürüm dosyası yok", {"path": "$fixture:" + REL + "surumsuz.bcf"}, "bcf_unreadable", "path")
    refused(read_cases, "BCF 1.0 okunmaz", {"path": "$fixture:" + REL + "surum-1.bcf"}, "bcf_unreadable", "path")
    refused(read_cases, "Andığı bakış arşivde yok", {"path": "$fixture:" + REL + "bakissiz.bcf"}, "bcf_unreadable", "path")
    refused(read_cases, "IFC olmayan model", {"path": "$fixture:" + REL + "disaridan-v3.bcf",
                                              "model": "$fixture:arch/v1/ifc/var.ifc"}, "ifc_unreadable", "model")
    refused(read_cases, "BCF okunmazsa model okunmaz", {"path": "$fixture:" + REL + "zip-degil.bcf",
                                                         "model": "$fixture:arch/v1/ifc/var.ifc"}, "bcf_unreadable", "path")

    # Plan: the same answer.
    version, topics = read_bcf(FILES / "disaridan-v21.bcf")
    read_cases.append({"name": "Önizleme aynı sonucu verir", "steps": [{"op": "plan", "input": {
        "path": "$fixture:" + REL + "disaridan-v21.bcf", "model": model}, "within": WITHIN, "expect": SAME,
        "result": {"status": "completed", "output": {"path": "$fixture:" + REL + "disaridan-v21.bcf", "version": version,
                                                     "topics": topics, "objects": objects_of(topics)},
                   "warnings": []}}]})
    # A file another program may name .bcfzip.
    read_cases.append({"name": "Eski uzantı .bcfzip", "steps": [
        {"op": "copyFile", "from": "$fixture:" + REL + "disaridan-v21.bcf", "to": "$tmp:bcf-eski.bcfzip"},
        {"op": "execute", "input": {"path": "$tmp:bcf-eski.bcfzip"}, "within": WITHIN, "expect": SAME,
         "result": {"status": "completed", "output": {"path": "$tmp:bcf-eski.bcfzip", "version": version, "topics": topics,
                                                      "objects": objects_of(topics, model=False)}, "warnings": []}}]})

    # cad.bcf.write
    write_cases = []
    G = lambda n: f"0192f5a1-8103-7000-8000-{n:012x}"  # noqa: E731
    base = {"guid": G(1), "title": "Zemin kattaki iç duvar 50 cm kuzeye taşınmış", "author": "mimar@buro",
            "date": "2026-11-17T12:00:00Z", "objects": [MOVED]}

    def written(title, topics, path=None, with_model=True, op="execute", overwrite=False, pre=()):
        # Every case its own file: the cases share one temporary folder.
        path = path or f"$tmp:bcf-yaz-{len(write_cases) + 1}.bcf"
        inp = {"path": path, "topics": [{k: v for k, v in t.items() if k != "model"} for t in topics]}
        if with_model:
            inp["model"] = model
        if overwrite:
            inp["overwrite"] = True
        out = {"path": path, "topics": len(topics), "viewpoints": sum(1 for t in topics if t.get("objects")),
               "guids": [t["guid"] for t in topics]}
        steps = list(pre) + [{"op": op, "input": inp, "within": WITHIN, "expect": SAME,
                              "result": {"status": "completed", "output": out, "warnings": []}}]
        if op == "execute":
            back = [written_topic(dict(t, model=with_model)) for t in topics]
            steps.append({"op": "execute", "command": "cad.bcf.read", "input": {"path": path, "model": model},
                          "within": WITHIN, "expect": SAME, "result": {"status": "completed", "output": {
                              "path": path, "version": "3.0", "topics": sorted(back, key=lambda t: t["guid"]),
                              "objects": objects_of(back)}, "warnings": []}})
        else:
            steps.append({"op": "execute", "command": "cad.bcf.read", "input": {"path": path}, "expect": SAME,
                          "result": {"status": "failed", "error": {"code": "invalid_file_path", "message": "$any",
                                                                   "path": "path"}}})
        write_cases.append({"name": title, "steps": steps})

    written("Taşınan duvar: bakışı duvarı seçer, kamerası ona bakar", [base])
    written("Bütün alanlarıyla: öncelik, etiketler, atanan, açıklama, iki yorum", [dict(
        base, topicType="Clash", status="Active", priority="High", labels=["Mimari", "Statik"], assignedTo="statik@buro",
        description="İkinci revizyonda y yönünde 0,50 m.\nOdaların alanı değişir.",
        comments=[{"author": "mimar@buro", "text": "Statik projeyle birlikte bakılsın."},
                  {"author": "statik@buro", "date": "2026-11-18T09:30:00+03:00", "text": "Kiriş yerinde."}])])
    written("İki konu, iki duvar; konular GUID sırasıyla okunur", [dict(base, guid=G(3)), dict(
        base, guid=G(2), title="Doğu duvarı REI 90", objects=[RATED])])
    written("İki duvar bir konuda: kamera ikisinin kutusuna bakar", [dict(base, objects=[MOVED, RATED])])
    written("Nesnesiz konu: bakışı yok, model gerekmez", [{"guid": G(4), "title": "Genel not", "author": "mimar@buro",
                                                          "date": "2026-11-17T12:00:00"}], with_model=False)
    written("Başlığın uçlarındaki boşluk alınır", [dict(base, title="  Taşınan duvar  ")])
    written("Önizleme yazmaz", [base], op="plan")
    written("overwrite ile var olan dosyanın üzerine", [base], overwrite=True, path="$tmp:bcf-var-uzerine.bcf",
            pre=[{"op": "copyFile", "from": "$fixture:" + REL + "disaridan-v3.bcf", "to": "$tmp:bcf-var-uzerine.bcf"}])
    w = lambda inp: dict({"path": "$tmp:bcf-ret.bcf", "model": model, "topics": [base]}, **inp)  # noqa: E731
    refused(write_cases, "Yol boş", w({"path": ""}), "invalid_file_path", "path")
    refused(write_cases, "Yol göreli", w({"path": "sorunlar.bcf"}), "invalid_file_path", "path")
    refused(write_cases, ".bcf değil", w({"path": "$tmp:bcf-sorunlar.txt"}), "invalid_file_path", "path")
    refused(write_cases, "Klasörü yok", w({"path": "$tmp:yok/klasor/sorunlar.bcf"}), "invalid_file_path", "path")
    refused(write_cases, "Model yok", w({"model": "$fixture:ifc/v1/yok.ifc"}), "invalid_file_path", "model")
    write_cases.append({"name": "Dosya var, overwrite yok", "steps": [
        {"op": "copyFile", "from": "$fixture:" + REL + "disaridan-v3.bcf", "to": "$tmp:bcf-var.bcf"},
        {"op": "execute", "input": w({"path": "$tmp:bcf-var.bcf"}), "expect": SAME,
         "result": {"status": "failed", "error": {"code": "file_exists", "message": "$any", "path": "path"}}}]})
    refused(write_cases, "Konu yok", w({"topics": []}), "invalid_bcf_topic", "topics")
    refused(write_cases, "GUID büyük harfli", w({"topics": [dict(base, guid=G(1).upper())]}), "invalid_bcf_topic",
            "topics.0.guid")
    refused(write_cases, "Aynı GUID iki konuda", w({"topics": [base, dict(base, title="İkinci")]}), "invalid_bcf_topic",
            "topics.1.guid")
    refused(write_cases, "Başlık boş", w({"topics": [dict(base, title="  ")]}), "invalid_bcf_topic", "topics.0.title")
    refused(write_cases, "Başlık iki satır", w({"topics": [dict(base, title="a\nb")]}), "invalid_bcf_topic", "topics.0.title")
    refused(write_cases, "301 karakterlik başlık", w({"topics": [dict(base, title="x" * 301)]}), "invalid_bcf_topic",
            "topics.0.title")
    refused(write_cases, "Yazar boş", w({"topics": [dict(base, author="")]}), "invalid_bcf_topic", "topics.0.author")
    refused(write_cases, "Gün okunmuyor", w({"topics": [dict(base, date="17.11.2026")]}), "invalid_bcf_topic",
            "topics.0.date")
    refused(write_cases, "Yorumun metni boş", w({"topics": [dict(base, comments=[{"author": "a", "text": " "}])]}),
            "invalid_bcf_topic", "topics.0.comments.0.text")
    refused(write_cases, "GlobalId değil", w({"topics": [dict(base, objects=["duvar-7"])]}), "invalid_bcf_topic",
            "topics.0.objects")
    refused(write_cases, "Nesneli konu modelsiz", {"path": "$tmp:bcf-ret.bcf", "topics": [base]}, "invalid_bcf_topic", "model")
    refused(write_cases, "Modelde olmayan nesne", w({"topics": [dict(base, objects=["0000000000000000000000"])]}),
            "ifc_object_not_found", "topics.0.objects")
    ifc = ifcopenshell.open(str(ROOT / "fixtures" / MODEL))
    storey = ifc.by_type("IfcBuildingStorey")[0].GlobalId
    refused(write_cases, "Gövdesi olmayan nesne", w({"topics": [dict(base, objects=[storey])]}), "ifc_no_body",
            "topics.0.objects")
    refused(write_cases, "IFC olmayan model", w({"model": "$fixture:arch/v1/ifc/var.ifc"}), "ifc_unreadable", "model")
    refused(write_cases, "Konu denetimi modelden önce", w({"model": "$fixture:arch/v1/ifc/var.ifc", "topics": [
        dict(base, title="")]}), "invalid_bcf_topic", "topics.0.title")

    docs = {
        OUT_READ: {"format": "kentos.command-cases", "version": 1, "command": "cad.bcf.read", "commandVersion": 1,
                   "title": "BCF sorunlarını oku: 3.0 ve 2.1, konular, yorumlar, bakışlar; seçilen nesneler modelde ve çizimde",
                   "note": "ADR 8103. Dosyaları IfcOpenShell'in BCF kitaplığı yazar; beklenenler "
                           "scripts/fixtures/bcf_cases.py'nin Python'un zip ve XML okuyucularıyla okumasından, KentOS kodu yok.",
                   "setup": SETUP, "cases": read_cases},
        OUT_WRITE: {"format": "kentos.command-cases", "version": 1, "command": "cad.bcf.write", "commandVersion": 1,
                    "title": "Sorunları BCF olarak yaz: konular, yorumlar, nesneleri seçen ve onlara bakan bakış",
                    "note": "ADR 8103. Yazılan dosya aynı durumda okunur; beklenenler scripts/fixtures/bcf_cases.py'nin "
                            "ADR'den yazılmış kurallarından (GUID kuralı, kamera), kutular IfcOpenShell'in geometrisinden.",
                    "setup": SETUP, "cases": write_cases},
    }
    stale = []
    for p, d in docs.items():
        text_out = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
        if check:
            if not p.exists() or p.read_text(encoding="utf-8") != text_out:
                stale.append(p.name)
        else:
            p.write_text(text_out, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(("güncel: " if check else "yazıldı: ") + f"{len(read_cases)} okuma, {len(write_cases)} yazma durumu")
    return 0


if __name__ == "__main__":
    sys.exit(main())
