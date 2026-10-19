#!/usr/bin/env python3
"""The shared cases of `cad.assets.check` (docs/adr/0349 §2; plan/cad.md CAD-REF-19).

    python3 scripts/fixtures/assets_check_cases.py           # writes the file
    python3 scripts/fixtures/assets_check_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.assets.check.json with Python's standard library, no KentOS code, from ADR 0349:

- a linked file is looked for at its path when absolute, then at its relative path from the open drawing's folder; an
  external reference also in the project's search paths (`xrefPaths`), each first with its relative path, then with its
  file's name; the places looked in are said when nothing is found;
- its state: `changed` when a SHA-256 is kept (an external reference's `digest`, a linked table's `digest`, a bound
  layer's `revision`) and the file's is another (`digests` asked), `missing` when not found, `unloaded`, `remote` for an
  address, `database` for a database's table, else `ok`;
- the order: external references by block, the drawing's objects (an object's file, then its attachments), the other
  blocks' objects, the bound layers' sources.

The files looked at are the repository's fixtures (`$fixture:` paths); their SHA-256 is read here.
"""
import copy
import hashlib
import json
import os
import posixpath
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.assets.check.json"
FIX = ROOT / "fixtures"


def fx(rel):
    return f"$fixture:{rel}"


def real(text):
    """A `$fixture:` path as the file system names it (to read it here)."""
    return str(FIX / text[len("$fixture:"):]) if text.startswith("$fixture:") else text


def sha(text):
    return hashlib.sha256(Path(real(text)).read_bytes()).hexdigest()


def is_file(text):
    return Path(real(text)).is_file()


def join(folder, rel):
    """`rel` (`/` between its parts) from `folder`, lexically (`..` takes back the part before it); a `$fixture:` text
    stays one."""
    prefix = "$fixture:" if folder.startswith("$fixture:") else ""
    parts = [p for p in folder[len(prefix):].split("/")]
    for part in rel.split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            if parts and parts[-1] != "":
                parts.pop()
            continue
        parts.append(part)
    return prefix + "/".join(parts)


def find(path, relative, drawing, search):
    """ADR 0349 §1: (found, via, tried)."""
    tried = []

    def look(p):
        if p not in tried:
            tried.append(p)
        return is_file(p)

    absolute = path.startswith("/") or path.startswith("$fixture:")
    if absolute and look(path):
        return path, "path", []
    folder = posixpath.dirname(drawing) if drawing else None
    rel = relative if relative is not None else (None if absolute or not path.strip() else path)
    if folder and rel:
        p = join(folder, rel)
        if look(p):
            return p, "relative", []
    name = posixpath.basename(path)
    for s in search:
        base = s if (s.startswith("/") or s.startswith("$fixture:")) else (join(folder, s) if folder else None)
        if base is None:
            continue
        if rel:
            p = join(base, rel)
            if look(p):
                return p, "searchPath", []
        p = f"{base}/{name}"
        if look(p):
            return p, "searchPath", []
    return None, "none", tried


def status(kind, path, *, relative=None, drawing=None, search=(), digest=None, digests=True, uid=None, block=None,
           source=None, name=None):
    found, via, tried = find(path, relative, drawing, list(search))
    current = None
    if found is None:
        state = "missing"
    elif digest is not None and digests:
        now = sha(found)
        state, current = ("ok", None) if now == digest else ("changed", now)
    else:
        state = "ok"
    out = {"kind": kind}
    for k, v in (("uid", uid), ("block", block), ("source", source)):
        if v is not None:
            out[k] = v
    out["name"] = name if name is not None else posixpath.basename(path)
    out["path"] = path
    if relative is not None:
        out["relative"] = relative
    if found is not None:
        out["found"] = found
    out["via"] = via
    out["state"] = state
    if digest is not None:
        out["digest"] = digest
    if current is not None:
        out["current"] = current
    if tried:
        out["tried"] = tried
    return out


def address(kind, url, uid=None, block=None, name=None):
    out = {"kind": kind}
    if uid is not None:
        out["uid"] = uid
    if block is not None:
        out["block"] = block
    out.update({"name": name if name is not None else posixpath.basename(url.split("?")[0].split("#")[0]), "path": url,
                "via": "address", "state": "remote"})
    return out


def counts(assets):
    c = {"ok": 0, "changed": 0, "missing": 0, "other": 0}
    for a in assets:
        key = a["state"] if a["state"] in ("ok", "changed", "missing") else "other"
        c[key] += 1
    return c


def answer(assets):
    return {"status": "completed", "output": {"assets": assets, "counts": counts(assets), "revision": "$current"},
            "warnings": []}


# ── Drawings ─────────────────────────────────────────────────────────────

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad"}


def layer(i, name, kind="layer"):
    return {"id": i, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def drawing(entities=(), blocks=None, settings=None, layers=None):
    d = {"format": "kentos.document", "version": 1, "name": "Bağlı dosyalar",
         "settings": {**SETTINGS, **(settings or {})}, "origin": {"x": 0.0, "y": 0.0},
         "layers": layers or [layer("cizim", "Çizim")], "activeLayer": "cizim",
         "entities": [{**e, "id": k + 1} for k, e in enumerate(entities)], "styles": {"items": [], "categories": []}}
    if blocks:
        d["blocks"] = blocks
    return d


def image(file, **f):
    return {"kind": "image", "layerId": "cizim", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "width": 4.0, "height": 3.0,
            "rotation": 0.0, "file": file, **f}


def raster(file=None, url=None):
    e = {"kind": "raster", "layerId": "cizim", "attrs": {}, "affine": [0.0, 1.0, 0.0, 10.0, 0.0, -1.0], "width": 10,
         "height": 10, "bands": 1, "sample": "u8", "srid": 0, "style": {"render": "gray", "bands": [1]}}
    if file:
        e["file"] = file
    if url:
        e["url"] = url
    return e


def cloud(*sources):
    return {"kind": "pointcloud", "layerId": "cizim", "attrs": {}, "sources": list(sources),
            "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0], "count": 1000, "srid": 0, "style": {"render": "rgb", "size": 2}}


def cloud_file(f):
    return {"file": f, "format": "xyz", "count": 1000, "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0]}


def cloud_url(u):
    return {"url": u, "format": "copc", "count": 1000, "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0]}


def table(path, digest):
    return {"kind": "table", "layerId": "cizim", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
            "height": 0.25, "rows": [0.5], "columns": [2.0], "cells": [["Oda"]],
            "source": {"kind": "file", "name": posixpath.basename(path), "path": path, "digest": digest,
                       "copy": [["Oda"]]}}


def line(**f):
    return {"kind": "line", "layerId": "cizim", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 5.0, "y": 0.0}, **f}


PLAN = fx("xref/v1/plan.kcad")
PLAN_SHA = sha(PLAN)
XREF_ID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400"
BLOCK_ID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7500"
PNG = fx("raster/v1/files/gray16.png")
TIF = fx("raster/v1/files/i8.tif")
CSV = fx("pointcloud/v1/files/bulut.csv")
TABLE = fx("table/v1/files/cubuk.csv")
PDF = fx("interaction/v1/belgeler/tapu-101-5.pdf")
GPKG = fx("geopackage/v1/qgis.gpkg")


def xref_block(path, digest, ident=XREF_ID, name="Plan", relative=None, unloaded=False):
    x = {"path": path, "digest": digest}
    if relative is not None:
        x["relative"] = relative
    if unloaded:
        x["unloaded"] = True
    else:
        x["objects"] = ["0192f5a1-5555-7000-8000-000000000001"]
    return {"id": ident, "name": name, "base": {"x": 0.0, "y": 0.0},
            "entities": [] if unloaded else [{**line(layerId=""), "id": 1}], "xref": x}


def insert(block):
    return {"kind": "insert", "layerId": "cizim", "attrs": {}, "block": block, "p": {"x": 0.0, "y": 0.0}, "scale": 1.0,
            "rotation": 0.0}


def source_link(path, revision, ident="kaynak-1", layer_id="bagli"):
    return {"id": ident, "kind": "geopackage", "path": path, "table": "parsel", "layer": layer_id,
            "revision": revision, "digest": "0" * 64, "srid": 5256, "writable": True}


cases = []


def add(name, setup, input_, assets, note=None, op="execute"):
    c = {"name": name, "setup": setup, "steps": [{"op": op, "input": input_, "result": answer(assets),
                                                  "expect": {"canUndo": False, "revision": "same"}}]}
    if note:
        c["note"] = note
    cases.append(c)


DRAWING_FILE = fx("raster/v1/ana.kcad")  # the open drawing's file beside the rasters (it need not be there)

add("bağlı dosyası olmayan çizim", drawing([line()]), {}, [])
add("mutlak yoldaki resim bulunur", drawing([image(PNG)]), {},
    [status("image", PNG, uid="$uidOf:1")])
add("göreli yoldaki resim açık çizimin klasöründen bulunur", drawing([image("files/gray16.png")]), {"drawing": DRAWING_FILE},
    [status("image", "files/gray16.png", drawing=DRAWING_FILE, uid="$uidOf:1")])
add("göreli yoldaki resim, açık çizimin dosyası verilmezse bulunamaz ve hiçbir yer denenmez",
    drawing([image("files/gray16.png")]), {}, [status("image", "files/gray16.png", uid="$uidOf:1")])
add("olmayan mutlak yol denenen yerle söylenir", drawing([image("/yok/foto.png")]), {"drawing": DRAWING_FILE},
    [status("image", "/yok/foto.png", drawing=DRAWING_FILE, uid="$uidOf:1")],
    note="Mutlak yolun dosyası yoksa adı açık çizimin klasöründe aranmaz: yalnız dış başvuru arama yollarında aranır.")
add("bağlı raster ve adresten okunan raster", drawing([raster(TIF), raster(url="https://ornek.org/orto.tif?x=1")]), {},
    [status("raster", TIF, uid="$uidOf:1"), address("raster", "https://ornek.org/orto.tif?x=1", uid="$uidOf:2")])
add("nokta bulutunun dosyası ve adresi sırasıyla", drawing([cloud(cloud_file(CSV), cloud_url("https://ornek.org/bulut.copc.laz"))]), {},
    [status("pointCloud", CSV, uid="$uidOf:1"), address("pointCloud", "https://ornek.org/bulut.copc.laz", uid="$uidOf:1")])
add("bağlı tablonun dosyası okunduğu gibi", drawing([table(TABLE, sha(TABLE))]), {},
    [status("table", TABLE, digest=sha(TABLE), uid="$uidOf:1")])
add("bağlı tablonun dosyası değişti: şimdiki özetiyle", drawing([table(TABLE, "0" * 64)]), {},
    [status("table", TABLE, digest="0" * 64, uid="$uidOf:1")])
add("özet istenmezse değişmiş tablo bulundu sayılır", drawing([table(TABLE, "0" * 64)]), {"digests": False},
    [status("table", TABLE, digest="0" * 64, digests=False, uid="$uidOf:1")])
add("eklerin dosyası adıyla, adresi ayrı", drawing([line(attachments=[{"name": "Tapu", "file": PDF},
                                                                    {"name": "Kayıt", "url": "https://tkgm.gov.tr/a"}])]), {},
    [status("attachment", PDF, uid="$uidOf:1", name="Tapu"), address("attachment", "https://tkgm.gov.tr/a", uid="$uidOf:1", name="Kayıt")])
add("resmin kendi dosyasından sonra ekleri", drawing([image(PNG, attachments=[{"name": "Not", "file": "/yok/not.txt"}])]), {},
    [status("image", PNG, uid="$uidOf:1"), status("attachment", "/yok/not.txt", uid="$uidOf:1", name="Not")])
XREF = drawing([insert(XREF_ID)], blocks=[xref_block(PLAN, PLAN_SHA)])
add("yüklü dış başvuru, kayıtlı yolunda ve özetiyle aynı", XREF, {}, [status("xref", PLAN, digest=PLAN_SHA, block=XREF_ID, name="Plan")])
add("değişmiş dış başvuru", drawing([insert(XREF_ID)], blocks=[xref_block(PLAN, "1" * 64)]), {},
    [status("xref", PLAN, digest="1" * 64, block=XREF_ID, name="Plan")])
add("boşaltılmış dış başvurunun dosyasına bakılmaz", drawing([insert(XREF_ID)], blocks=[xref_block("/yok/plan.kcad", PLAN_SHA, unloaded=True)]), {},
    [{"kind": "xref", "block": XREF_ID, "name": "Plan", "path": "/yok/plan.kcad", "via": "none", "state": "unloaded",
      "digest": PLAN_SHA}])
add("bulunamayan dış başvuru: kayıtlı yol, göreli yol ve arama yolları denenir",
    drawing([insert(XREF_ID)], blocks=[xref_block("/yok/plan.kcad", PLAN_SHA, relative="mimari/plan.kcad")],
            settings={"xrefPaths": ["/ortak", "eski"]}),
    {"drawing": fx("xref/v1/ana.kcad")},
    [status("xref", "/yok/plan.kcad", relative="mimari/plan.kcad", drawing=fx("xref/v1/ana.kcad"), search=["/ortak", "eski"],
            digest=PLAN_SHA, block=XREF_ID, name="Plan")])
add("dış başvuru mutlak arama yolunda adıyla bulunur",
    drawing([insert(XREF_ID)], blocks=[xref_block("/yok/plan.kcad", PLAN_SHA)], settings={"xrefPaths": [fx("xref/v1")]}), {},
    [status("xref", "/yok/plan.kcad", search=[fx("xref/v1")], digest=PLAN_SHA, block=XREF_ID, name="Plan")])
add("dış başvuru göreli arama yolunda göreli yoluyla bulunur",
    drawing([insert(XREF_ID)], blocks=[xref_block("/yok/plan.kcad", PLAN_SHA, relative="v1/plan.kcad")],
            settings={"xrefPaths": ["/ortak", "."]}),
    {"drawing": fx("xref/ana.kcad")},
    [status("xref", "/yok/plan.kcad", relative="v1/plan.kcad", drawing=fx("xref/ana.kcad"), search=["/ortak", "."],
            digest=PLAN_SHA, block=XREF_ID, name="Plan")],
    note="Göreli yol açık çizimin klasöründe de aynı yerdir: arama yoluna gelmeden bulunur.")
add("arama yolu açık çizimin dosyası olmadan göreli ise atlanır",
    drawing([insert(XREF_ID)], blocks=[xref_block("/yok/plan.kcad", PLAN_SHA)], settings={"xrefPaths": ["v1", fx("xref/v1")]}), {},
    [status("xref", "/yok/plan.kcad", search=["v1", fx("xref/v1")], digest=PLAN_SHA, block=XREF_ID, name="Plan")])
add("arama yolu resimlere uygulanmaz",
    drawing([image("/yok/gray16.png")], settings={"xrefPaths": [fx("raster/v1/files")]}), {},
    [status("image", "/yok/gray16.png", uid="$uidOf:1")])
add("sıradan bloğun nesnesinin bağlı dosyası bloğuyla",
    drawing([insert(BLOCK_ID)], blocks=[{"id": BLOCK_ID, "name": "Pafta başlığı", "base": {"x": 0.0, "y": 0.0},
                                         "entities": [{**image(PNG, layerId=""), "id": 1}]}]), {},
    [status("image", PNG, block=BLOCK_ID)])
SOURCES_LAYERS = [layer("cizim", "Çizim"), layer("bagli", "Parsel"), layer("pg", "Kadastro")]
add("bağlı katmanın kaynağı okunduğu gibi", drawing(settings={"sources": [source_link(GPKG, sha(GPKG))]}, layers=SOURCES_LAYERS), {},
    [status("source", GPKG, digest=sha(GPKG), source="kaynak-1")])
add("bağlı katmanın kaynağı değişti", drawing(settings={"sources": [source_link(GPKG, "2" * 64)]}, layers=SOURCES_LAYERS), {},
    [status("source", GPKG, digest="2" * 64, source="kaynak-1")])
add("bağlı katmanın kaynağı yerinde yok, göreli yolundan bulunur",
    drawing(settings={"sources": [{**source_link("/yok/qgis.gpkg", sha(GPKG)), "relative": "../geopackage/v1/qgis.gpkg"}]},
            layers=SOURCES_LAYERS),
    {"drawing": fx("xref/ana.kcad")},
    [status("source", "/yok/qgis.gpkg", relative="../geopackage/v1/qgis.gpkg", drawing=fx("xref/ana.kcad"), digest=sha(GPKG),
            source="kaynak-1")])
add("veritabanı tablosuna bakılmaz",
    drawing(settings={"sources": [{**source_link("postgresql://kentos@localhost:5432/kadastro", "3" * 64, ident="kaynak-2",
                                                 layer_id="pg"), "kind": "postgis", "table": "public.parsel"}]},
            layers=SOURCES_LAYERS), {},
    [{"kind": "source", "source": "kaynak-2", "name": "public.parsel", "path": "postgresql://kentos@localhost:5432/kadastro",
      "via": "database", "state": "database"}])
MIXED = drawing([insert(XREF_ID), image(PNG), table(TABLE, "0" * 64)], blocks=[xref_block(PLAN, PLAN_SHA)],
                settings={"sources": [source_link(GPKG, sha(GPKG))]}, layers=SOURCES_LAYERS)
MIXED_ALL = [status("xref", PLAN, digest=PLAN_SHA, block=XREF_ID, name="Plan"), status("image", PNG, uid="$uidOf:2"),
             status("table", TABLE, digest="0" * 64, uid="$uidOf:3"), status("source", GPKG, digest=sha(GPKG), source="kaynak-1")]
add("sıra: dış başvurular, çizimin nesneleri, kaynaklar; sayılar durumlarına göre", MIXED, {}, MIXED_ALL)
add("yalnız istenen türler", MIXED, {"kinds": ["image", "source"]}, [MIXED_ALL[1], MIXED_ALL[3]])
add("yalnız dış başvurular", MIXED, {"kinds": ["xref"]}, [MIXED_ALL[0]])
add("plan aynı cevabı verir, yazmaz", MIXED, {}, MIXED_ALL, op="plan")
cases.append({"name": "doğrulama", "setup": MIXED, "steps": [{"op": "validate", "input": {},
                                                              "result": {"status": "completed", "output": None, "warnings": []}}]})


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build():
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.assets.check",',
           '  "commandVersion": 1,', '  "title": "Bağlı dosyalar: nerede bulundukları ve durumları",',
           f'  "note": {compact("Written by scripts/fixtures/assets_check_cases.py from docs/adr/0349 §1–§2; do not edit by hand.")},',
           f'  "setup": {compact(drawing())},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(s)}" for s in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


def main():
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


if os.environ.get("KENTOS_NO_MAIN") is None:
    main()
