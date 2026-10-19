#!/usr/bin/env python3
"""The shared cases of `cad.images.edit` (docs/adr/0350 §1; plan/cad.md CAD-REF-16).

    python3 scripts/fixtures/images_edit_cases.py           # writes the file
    python3 scripts/fixtures/images_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.images.edit.json with Python's standard library, no KentOS code, from ADR 0350 §1–§2:

- Göm (`embed`) reads each linked image's file, looked for as an external reference's source (ADR 0349 §1: its path
  when absolute, its relative path from the open drawing's folder, the project's search paths with its relative path and
  then its name); a PNG's size from its IHDR, a JPEG's from its frame header, read here with `struct`; the library's
  picture is `resim-` and the SHA-256's first sixteen hex digits, named by the file's stem, under Resimler, kept once;
- Yeniden bağla (`relink`) points the images at the file given (kept as given), read from the drawing's folder when
  relative, never from the search paths;
- images on a locked layer are left out and counted, all locked refused; the refusals in the contract's order.

The files read are the repository's fixtures (`$fixture:` paths).
"""
import hashlib
import json
import os
import posixpath
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.images.edit.json"
FIX = ROOT / "fixtures"
sys.path.insert(0, str(Path(__file__).resolve().parent))
os.environ["KENTOS_NO_MAIN"] = "1"
from assets_check_cases import find, fx, real  # noqa: E402  (the search rule of ADR 0349 §1)

MAX_BYTES = 32 * 1024 * 1024
SOF = {0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF}


def size(b):
    """(format, width, height) from the header, or None."""
    if b[:8] == b"\x89PNG\r\n\x1a\n":
        if b[12:16] != b"IHDR" or len(b) < 24:
            return None
        w, h = struct.unpack(">II", b[16:24])
        return ("png", w, h) if w and h else None
    if b[:2] != b"\xff\xd8":
        return None
    i = 2
    while True:
        if i >= len(b) or b[i] != 0xFF:
            return None
        while i + 1 < len(b) and b[i + 1] == 0xFF:
            i += 1
        if i + 1 >= len(b):
            return None
        m = b[i + 1]
        if m == 0x01 or 0xD0 <= m <= 0xD7:
            i += 2
            continue
        if m in (0xD9, 0xDA) or i + 4 > len(b):
            return None
        length = struct.unpack(">H", b[i + 2:i + 4])[0]
        if length < 2:
            return None
        if m in SOF:
            if i + 9 > len(b):
                return None
            h, w = struct.unpack(">HH", b[i + 5:i + 9])
            return ("jpeg", w, h) if w and h else None
        i += 2 + length


def stem(name):
    base, dot, ext = name.rpartition(".")
    return base if dot and base and ext.lower() in ("png", "jpg", "jpeg") else name


def picture(path):
    """The library's picture for the file at `path` (found): its id, its listed item and its size."""
    b = Path(real(path)).read_bytes()
    fmt, w, h = size(b)
    ident = "resim-" + hashlib.sha256(b).hexdigest()[:16]
    return ident, fmt, w, h


# ── Drawings ─────────────────────────────────────────────────────────────

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad"}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)]


def library_item(path):
    """The picture as the library keeps it (only the listed fields matter to the cases)."""
    ident, fmt, w, h = picture(path)
    b = Path(real(path)).read_bytes()
    import base64
    return {"kind": "asset", "id": ident, "name": stem(posixpath.basename(path)), "path": ["Resimler"], "format": fmt,
            "data": f"data:image/{fmt};base64,{base64.b64encode(b).decode()}", "width": w, "height": h}


def drawing(entities=(), settings=None, items=()):
    return {"format": "kentos.document", "version": 1, "name": "Resimler",
            "settings": {**SETTINGS, **(settings or {})}, "origin": {"x": 0.0, "y": 0.0}, "layers": LAYERS,
            "activeLayer": "cizim", "entities": [{**e, "id": k + 1} for k, e in enumerate(entities)],
            "styles": {"items": list(items), "categories": []}}


def image(file=None, asset=None, layer_id="cizim", x=0.0):
    e = {"kind": "image", "layerId": layer_id, "attrs": {}, "p": {"x": x, "y": 0.0}, "width": 4.0, "height": 3.0,
         "rotation": 0.0}
    if file is not None:
        e["file"] = file
    if asset is not None:
        e["asset"] = asset
    return e


def line():
    return {"kind": "line", "layerId": "cizim", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 5.0, "y": 0.0}}


LOGO = fx("interaction/v1/logo.png")
RGB = fx("raster/v1/files/rgb.png")
GRAY = fx("raster/v1/files/gray16.png")
JPEG = fx("exif/v1/konum.jpg")
FAKE = fx("attachments/v1/sahte.jpg")
DRAWING = fx("raster/v1/ana.kcad")
UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7600"

cases = []


def sourced(n, found, *, asset=None, file=None):
    ident, fmt, w, h = picture(found)
    out = {"uid": f"$uidOf:{n}"}
    if asset is not None:
        out["asset"] = asset
    if file is not None:
        out["file"] = file
    out.update({"read": found, "width": w, "height": h})
    return out


def done(images, assets=(), warnings=()):
    out = {"images": images}
    if assets:
        out["assets"] = list(assets)
    out["revision"] = "$current"
    return {"status": "completed", "output": out, "warnings": list(warnings)}


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def listed(items):
    return [{"id": i["id"], "kind": i["kind"], "name": i["name"], "path": i["path"]} for i in items]


def add(name, setup, input_, result, expect=None, note=None, op="execute"):
    step = {"op": op, "input": input_, "result": result}
    if expect is not None:
        step["expect"] = expect
    elif result["status"] == "failed" or op != "execute":
        step["expect"] = {"canUndo": False, "revision": "same"}
    c = {"name": name, "setup": setup, "steps": [step]}
    if note:
        c["note"] = note
    cases.append(c)


def embedded(n_items_before, fields, items_after):
    return {"canUndo": True, "revision": "changed", "fields": fields, "library": listed(items_after)}


def embed_case(name, entities, input_, embeds, *, settings=None, items=(), warnings=(), drawing_file=None, note=None):
    """Göm over `entities`: `embeds` lists (slot, the file as kept, where it is found)."""
    images, assets, after, fields = [], [], list(items), {}
    for slot, found in embeds:
        ident = picture(found)[0]
        images.append(sourced(slot, found, asset=ident))
        if not any(i["id"] == ident for i in after):
            item = library_item(found)
            after.append(item)
            assets.append(ident)
        fields[str(slot)] = {"asset": ident, "file": None}
    images.sort(key=lambda s: int(s["uid"].split(":")[1]))
    add(name, drawing(entities, settings, items), input_, done(images, assets, warnings),
        expect={"canUndo": True, "revision": "changed", "fields": fields, "library": listed(after)}, note=note)


def ok_found(path, drawing_file=None, search=()):
    found, _, _ = find(path, None, drawing_file, list(search))
    assert found is not None, path
    return found


# Göm.
embed_case("Göm: mutlak yoldaki bağlı resim kitaplığa girer, resim gömülü olur", [image(LOGO)],
           {"operation": {"kind": "embed", "uids": ["$uidOf:1"]}}, [(1, LOGO)])
embed_case("Göm: verilmezse çizimin bütün bağlı resimleri, sırasıyla", [image(RGB), line(), image(GRAY)],
           {"operation": {"kind": "embed"}}, [(1, RGB), (3, GRAY)])
embed_case("Göm: aynı dosyayı gösteren iki resim kitaplıkta bir resim", [image(LOGO), image(LOGO, x=5.0)],
           {"operation": {"kind": "embed"}}, [(1, LOGO), (2, LOGO)])
embed_case("Göm: kitaplıkta olan resim yeniden eklenmez", [image(LOGO)], {"operation": {"kind": "embed"}}, [(1, LOGO)],
           items=[library_item(LOGO)])
embed_case("Göm: gömülü resimler atlanır, yalnız bağlı olan gömülür",
           [image(asset=picture(LOGO)[0]), image(RGB, x=5.0)], {"operation": {"kind": "embed"}}, [(2, RGB)],
           items=[library_item(LOGO)])
embed_case("Göm: göreli yol açık çizimin klasöründen okunur", [image("files/gray16.png")],
           {"operation": {"kind": "embed"}, "drawing": DRAWING}, [(1, ok_found("files/gray16.png", DRAWING))])
embed_case("Göm: yerinde olmayan dosya arama yolunda adıyla bulunur", [image("/eski/proje/gray16.png")],
           {"operation": {"kind": "embed"}},
           [(1, ok_found("/eski/proje/gray16.png", None, [fx("raster/v1/files")]))],
           settings={"xrefPaths": ["/ortak", fx("raster/v1/files")]})
embed_case("Göm: JPEG'in boyu çerçeve başlığından", [image(JPEG)], {"operation": {"kind": "embed"}}, [(1, JPEG)])
embed_case("Göm: kilitli katmandaki resim atlanır ve sayılır", [image(LOGO), image(RGB, layer_id="kilitli", x=5.0)],
           {"operation": {"kind": "embed"}}, [(1, LOGO)],
           warnings=[{"code": "layer_locked", "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}])
add("Göm: verilen resimlerin hepsi kilitli katmanda", drawing([image(LOGO, layer_id="kilitli")]),
    {"operation": {"kind": "embed", "uids": ["$uidOf:1"]}},
    refused("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"))
add("Göm: verilen resim zaten gömülü", drawing([image(asset=picture(LOGO)[0])], items=[library_item(LOGO)]),
    {"operation": {"kind": "embed", "uids": ["$uidOf:1"]}},
    refused("nothing_to_embed", "Verilen resimler zaten gömülü; gömülecek bağlı resim yok.", "uids"))
add("Göm: çizimde bağlı resim yok", drawing([line()]), {"operation": {"kind": "embed"}},
    refused("nothing_to_embed", "Çizimde gömülecek bağlı resim yok.", "operation"))
add("Göm: dosya bulunamadı, bakılan yerler söylenir", drawing([image("/yok/foto.png")]),
    {"operation": {"kind": "embed", "uids": ["$uidOf:1"]}},
    refused("file_not_found", "foto.png bulunamadı. Bakılan yerler: /yok/foto.png.", "uids[0]"))
add("Göm: PNG ya da JPEG olmayan dosya", drawing([image(FAKE)]), {"operation": {"kind": "embed"}},
    refused("import_unreadable", "sahte.jpg PNG ya da JPEG değil ya da bozuk; resim olarak okunamadı.", "operation"))
add("Göm: göreli yol, çizimin dosyası yok", drawing([image("foto.png")]),
    {"operation": {"kind": "embed", "uids": ["$uidOf:1"]}},
    refused("invalid_file_path", "“foto.png” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin.", "uids[0]"))
add("Göm: resim olmayan nesne", drawing([image(LOGO), line()]),
    {"operation": {"kind": "embed", "uids": ["$uidOf:1", "$uidOf:2"]}},
    {"status": "failed", "error": {"code": "not_an_image", "message": "$any", "path": "uids[1]"}})
add("Göm: çizimde olmayan nesne", drawing([image(LOGO)]), {"operation": {"kind": "embed", "uids": [UID]}},
    refused("entity_not_found", f"“{UID}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"))
add("Göm: boş kimlik listesi", drawing([image(LOGO)]), {"operation": {"kind": "embed", "uids": []}},
    refused("no_entities", "Kaynağı değişecek resim verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"))
add("Göm: geçersiz kimlik", drawing([image(LOGO)]), {"operation": {"kind": "embed", "uids": ["resim-1"]}},
    refused("invalid_uid", "“resim-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "uids[0]"))


# Yeniden bağla.
def relink_case(name, entities, uids, file, found, slots, items=(), drawing_file=None, note=None):
    input_ = {"operation": {"kind": "relink", "uids": uids, "file": file}}
    if drawing_file:
        input_["drawing"] = drawing_file
    images = [sourced(n, found, file=file) for n in slots]
    add(name, drawing(entities, items=items), input_, done(images),
        expect={"canUndo": True, "revision": "changed", "fields": {str(n): {"file": file, "asset": None} for n in slots},
                "library": listed(items)}, note=note)


relink_case("Yeniden bağla: bağlı resim başka dosyayı gösterir", [image("/yok/eski.png")], ["$uidOf:1"], RGB, RGB, [1])
relink_case("Yeniden bağla: gömülü resim bağlı olur, kitaplıktaki resim kalır", [image(asset=picture(LOGO)[0])],
            ["$uidOf:1"], RGB, RGB, [1], items=[library_item(LOGO)])
relink_case("Yeniden bağla: göreli yol yazıldığı gibi kalır, çizimin klasöründen okunur", [image(LOGO)], ["$uidOf:1"],
            "files/gray16.png", ok_found("files/gray16.png", DRAWING), [1], drawing_file=DRAWING)
relink_case("Yeniden bağla: iki resim aynı dosyayı gösterir", [image(LOGO), line(), image(GRAY, x=5.0)],
            ["$uidOf:3", "$uidOf:1"], JPEG, JPEG, [1, 3])
add("Yeniden bağla: boş yol", drawing([image(LOGO)]),
    {"operation": {"kind": "relink", "uids": ["$uidOf:1"], "file": "  "}},
    refused("invalid_file_path", "Resmin dosya yolu boş; bir PNG ya da JPEG dosyasının yolunu verin.", "operation.file"))
add("Yeniden bağla: dosya yok", drawing([image(LOGO)]),
    {"operation": {"kind": "relink", "uids": ["$uidOf:1"], "file": "/yok/yeni.png"}},
    refused("file_not_found", "yeni.png bulunamadı. Bakılan yerler: /yok/yeni.png.", "operation.file"))
add("Yeniden bağla: arama yollarına bakılmaz", drawing([image(LOGO)], settings={"xrefPaths": [fx("raster/v1/files")]}),
    {"operation": {"kind": "relink", "uids": ["$uidOf:1"], "file": "/yok/rgb.png"}},
    refused("file_not_found", "rgb.png bulunamadı. Bakılan yerler: /yok/rgb.png.", "operation.file"))
add("Yeniden bağla: PNG ya da JPEG olmayan dosya", drawing([image(LOGO)]),
    {"operation": {"kind": "relink", "uids": ["$uidOf:1"], "file": FAKE}},
    refused("import_unreadable", "sahte.jpg PNG ya da JPEG değil ya da bozuk; resim olarak okunamadı.", "operation.file"))
add("Yeniden bağla: kilitli katman dosyadan önce söylenir", drawing([image(LOGO, layer_id="kilitli")]),
    {"operation": {"kind": "relink", "uids": ["$uidOf:1"], "file": "/yok/yeni.png"}},
    refused("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"))
add("plan yazmadan ne olacağını söyler", drawing([image(LOGO)]), {"operation": {"kind": "embed"}},
    done([sourced(1, LOGO, asset=picture(LOGO)[0])], [picture(LOGO)[0]]), op="plan",
    expect={"canUndo": False, "revision": "same", "library": []})
add("eski sürüme dayanan istek", drawing([image(LOGO)]), {"operation": {"kind": "embed"}, "expectedRevision": "x"},
    {"status": "failed", "error": {"code": "invalid_revision", "message": "$any", "path": "expectedRevision"}})
add("çizim bu arada değişti", drawing([image(LOGO)]), {"operation": {"kind": "embed"}, "expectedRevision": "999"},
    {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}})
cases.append({"name": "doğrulama", "setup": drawing([image(LOGO)]), "steps": [
    {"op": "validate", "input": {"operation": {"kind": "embed"}}, "result": {"status": "completed", "output": None, "warnings": []}}]})


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build():
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.images.edit",',
           '  "commandVersion": 1,', '  "title": "Resmin kaynağı: Göm ve Yeniden bağla",',
           f'  "note": {compact("Written by scripts/fixtures/images_edit_cases.py from docs/adr/0350 §1–§2; do not edit by hand.")},',
           f'  "setup": {compact(drawing())},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},', f'      "setup": {compact(c["setup"])},']
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


main()
