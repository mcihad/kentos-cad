"""Writes fixtures/commands/v1/cad.attachments.inspect.json and the files it reads (fixtures/attachments/v1): the shared
cases of `cad.attachments.inspect` v1 (Ekleri incele, docs/adr/0442 §7), their expectations built here without KentOS
code from the ADR's rule: the media type by the first bytes (PNG, JPEG, GIF, TIFF, PDF, ZIP; a ZIP by its extension too),
else by the name's extension, else `application/octet-stream`; a web address by its extension only; the size and SHA-256
of the bytes; a photo's moment and place as its EXIF says, taken from the EXIF reader's independent reference
(fixtures/exif/v1/cases.json, written by scripts/fixtures/exif_cases.py from docs/adr/0225 §3).

    python3 scripts/fixtures/attachments_inspect_cases.py           # writes the files
    python3 scripts/fixtures/attachments_inspect_cases.py --check   # compares them with the ones on disk
"""
import base64
import copy
import hashlib
import io
import json
import struct
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.attachments.inspect.json"
FILES = ROOT / "fixtures/attachments/v1"
EXIF = ROOT / "fixtures/exif/v1"
EXIF_CASES = {c["file"]: c for c in json.loads((EXIF / "cases.json").read_text(encoding="utf-8"))["cases"]}


def docx():
    """A tiny ZIP named .docx, its time fixed so the bytes do not change."""
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_STORED) as z:
        info = zipfile.ZipInfo("word/document.xml", date_time=(2026, 10, 17, 12, 0, 0))
        z.writestr(info, "<w:document/>")
    return out.getvalue()


def png(w, h):
    """A w × h grey PNG without compression trickery (zlib stored blocks)."""
    import zlib
    raw = b"".join(b"\x00" + b"\x80" * w for _ in range(h))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 0, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


# The files the cases read, written here (the photos are the EXIF reader's own).
WRITTEN = {
    "rapor.docx": docx(),
    "plan.pdf": b"%PDF-1.4\n% KentOS\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF\n",
    "kroki.gif": b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00"
                 b"\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;",
    "olcu.csv": "Nokta;Y;X\nP1;500000.000;4400000.000\n".encode(),
    "not.xyz": b"bilinmeyen bir dosya\n",
    "sahte.jpg": b"Bu bir JPEG degil.\n",
}


def file_bytes(rel):
    """The bytes of a file the cases name, relative to fixtures/attachments/v1."""
    if rel in WRITTEN:
        return WRITTEN[rel]
    p = (FILES / rel).resolve()
    return p.read_bytes() if p.is_file() else None


# ---- The rule, written from the ADR ----

SNIFF = [(b"\x89PNG\r\n\x1a\n", "image/png"), (b"\xff\xd8\xff", "image/jpeg"), (b"GIF87a", "image/gif"),
         (b"GIF89a", "image/gif"), (b"II*\x00", "image/tiff"), (b"MM\x00*", "image/tiff"), (b"%PDF-", "application/pdf")]
ZIPS = {"docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ods": "application/vnd.oasis.opendocument.spreadsheet", "kmz": "application/vnd.google-earth.kmz"}
EXTENSIONS = {"png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg", "gif": "image/gif", "tif": "image/tiff",
              "tiff": "image/tiff", "pdf": "application/pdf", "txt": "text/plain", "csv": "text/csv",
              "json": "application/json", "xml": "application/xml", "dxf": "image/vnd.dxf",
              "kcad": "application/vnd.kentos.kcad", "doc": "application/msword", "xls": "application/vnd.ms-excel",
              **ZIPS, "zip": "application/zip", "mp3": "audio/mpeg", "wav": "audio/wav", "mp4": "video/mp4"}


def extension(name):
    for c in "?#":
        name = name.split(c)[0]
    last = name.split("/")[-1]
    return last.rsplit(".", 1)[1].lower() if "." in last else ""


def media_type(data, name):
    if data is None:
        return EXTENSIONS.get(extension(name))
    for head, m in SNIFF:
        if data.startswith(head):
            return m
    if data.startswith(b"PK\x03\x04"):
        return ZIPS.get(extension(name), "application/zip")
    return EXTENSIONS.get(extension(name), "application/octet-stream")


def facts(a, folder_given, assets):
    out = {"name": a["name"]}
    data, label = None, a["name"]
    if "asset" in a:
        out["source"] = "asset"
        data = assets.get(a["asset"])
        missing = data is None
    elif "file" in a:
        out["source"] = "file"
        label = a["file"]
        data = file_bytes(a["file"]) if folder_given else None
        missing = data is None
    else:
        out["source"] = "url"
        label = a["url"]
        missing = False
    if missing:
        out["missing"] = True
        return out
    m = media_type(data, label)
    if m:
        out["mime"] = m
    if data is not None:
        out["size"] = len(data)
        out["sha256"] = hashlib.sha256(data).hexdigest()
        if m in ("image/jpeg", "image/tiff"):
            ex = EXIF_CASES.get(Path(a.get("file", "")).name) or EXIF_CASES.get(a.get("photo", ""), {})
            for k in ("taken", "latitude", "longitude"):
                if k in ex:
                    out[k] = ex[k]
    return out


# ---- The drawing ----

P = lambda x, y: {"x": 500000 + x, "y": 4400000 + y}  # noqa: E731


def square(x):
    return [P(x, 0), P(x + 10, 0), P(x + 10, 10), P(x, 10)]


GREY = png(4, 3)
PHOTO = (EXIF / "konum.jpg").read_bytes()


def asset(data, name, fmt, w, h):
    mime = {"png": "image/png", "jpeg": "image/jpeg"}[fmt]
    return {"kind": "asset", "id": "resim-" + hashlib.sha256(data).hexdigest()[:16], "name": name, "path": ["Ekler"],
            "format": fmt, "data": f"data:{mime};base64," + base64.b64encode(data).decode(), "width": w, "height": h}


ASSETS = [asset(GREY, "cephe", "png", 4, 3), asset(PHOTO, "konum", "jpeg", 8, 8)]
ASSET_BYTES = {ASSETS[0]["id"]: GREY, ASSETS[1]["id"]: PHOTO}

FILED = [
    {"name": "Konum fotoğrafı", "file": "../../exif/v1/konum.jpg"},
    {"name": "GPS'siz", "file": "../../exif/v1/gpssiz.jpg"},
    {"name": "EXIF'siz", "file": "../../exif/v1/exifsiz.jpg"},
    {"name": "TIFF", "file": "../../exif/v1/yalniz-tiff.tif"},
    {"name": "Metin", "file": "../../exif/v1/metin.txt"},
    {"name": "Kayıp", "file": "yok.jpg"},
]
DOCUMENTS = [
    {"name": "Rapor", "file": "rapor.docx", "note": "Denetim raporu"},
    {"name": "Plan", "file": "plan.pdf"},
    {"name": "Kroki", "file": "kroki.gif"},
    {"name": "Ölçüler", "file": "olcu.csv"},
    {"name": "Bilinmeyen", "file": "not.xyz"},
    {"name": "Sahte JPEG", "file": "sahte.jpg"},
    {"name": "Klasör", "file": "."},
]
LINKS = [
    {"name": "Rapor", "url": "https://ornek.org/belgeler/rapor.pdf"},
    {"name": "Sayfa", "url": "https://ornek.org/kayit"},
    {"name": "Foto", "url": "https://ornek.org/foto.JPG?boy=800#ust"},
    {"name": "Arşiv", "url": "https://ornek.org/paket.zip"},
]
LIBRARY = [
    {"name": "Cephe", "asset": ASSETS[0]["id"]},
    {"name": "Konum", "asset": ASSETS[1]["id"], "photo": "konum.jpg"},
    {"name": "Silinmiş", "asset": "resim-0000000000000000"},
]
ATTACHED = {1: FILED, 2: DOCUMENTS, 3: LINKS, 4: LIBRARY, 5: []}


def stored(a):
    """An attachment as the drawing keeps it (the twin's `photo` hint aside)."""
    return {k: v for k, v in a.items() if k != "photo"}


ENTITIES = []
for k, xs in ATTACHED.items():
    e = {"kind": "polygon", "id": k, "layerId": "bina", "attrs": {}, "pts": square(20 * k)}
    if xs:
        e["attachments"] = [stored(a) for a in xs]
    ENTITIES.append(e)
IDS = [e["id"] for e in ENTITIES]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Ekler", "settings": {
        "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
        "workspace": "gis", "drawingFont": "barlow"}, "origin": P(0, 0),
        "layers": [{"id": "bina", "name": "Bina", "type": "layer", "visible": True, "locked": False, "expanded": True,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "bina", "entities": ENTITIES, "styles": {"items": copy.deepcopy(ASSETS), "categories": []}}


UNCHANGED = {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}
FOLDER = "$fixture:attachments/v1"


def answer(slots, folder=True):
    return {"objects": [{"uid": f"$uidOf:{s}", "attachments": [facts(a, folder, ASSET_BYTES) for a in ATTACHED[s]]}
                        for s in slots]}


def ok(name, slots, folder=True, op="execute"):
    inp = {"uids": [f"$uidOf:{s}" for s in slots]}
    if folder:
        inp["folder"] = FOLDER
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "completed",
                                                                         "output": answer(slots, folder), "warnings": []},
                                     "expect": UNCHANGED}]}


def refused(name, inp, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": UNCHANGED}]}


GONE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff"


def cases():
    return [
        ok("fotoğraflar diskten: tür, boy, özet, EXIF'in anı ve konumu; bulunamayan", [1]),
        ok("belgeler: ZIP uzantısıyla, PDF ve GIF baytlarından, CSV uzantısından, bilinmeyen, sahte uzantı, klasör", [2]),
        ok("adresler yalnız adıyla: uzantı, sorgu ve parça, büyük harf, uzantısız", [3]),
        ok("kitaplığın fotoğrafları: PNG, EXIF'li JPEG, kitaplıkta olmayan", [4]),
        ok("eki olmayan nesne", [5]),
        ok("verilen sırayla", [4, 1, 3]),
        ok("aynı nesne iki kez", [3, 3]),
        ok("klasör verilmeyince göreli dosyalar okunmaz", [1], folder=False),
        ok("klasörsüz belgeler de okunmaz; adresler ve kitaplık yine", [2, 3, 4], folder=False),
        ok("plan aynı cevabı verir", [1, 4], op="plan"),
        ok("bütün nesneler", [1, 2, 3, 4, 5]),
        {"name": "doğrulama hiçbir şey yazmaz", "steps": [{"op": "validate", "input": {"uids": ["$uidOf:1"]},
                                                            "result": {"status": "completed", "output": None,
                                                                       "warnings": []}, "expect": UNCHANGED}]},
        refused("nesne verilmedi", {"uids": []}, "invalid_count",
                "Bir çağrıda 1 ile 1000 arasında nesne incelenir; 0 verildi.", "uids"),
        refused("1 000'den çok nesne", {"uids": ["$uidOf:1"] * 1001}, "invalid_count",
                "Bir çağrıda 1 ile 1000 arasında nesne incelenir; 1001 verildi.", "uids"),
        refused("geçersiz kimlik", {"uids": ["B-1"]}, "invalid_uid",
                "“B-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uids/0"),
        refused("ikinci kimlik geçersiz", {"uids": ["$uidOf:1", "B-2"]}, "invalid_uid",
                "“B-2” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uids/1"),
        refused("çizimde olmayan nesne", {"uids": [GONE]}, "entity_not_found",
                f"“{GONE}” kimlikli nesne çizimde yok; silinmiş olabilir. Çizimdeki bir nesnenin kimliğini verin.",
                "uids/0"),
        refused("göreli klasör", {"uids": ["$uidOf:1"], "folder": "ekler"}, "invalid_folder",
                "“ekler” mutlak bir yol değil; çizimin klasörünü tam yoluyla verin.", "folder"),
        refused("önce sayı denetlenir", {"uids": [], "folder": "ekler"}, "invalid_count",
                "Bir çağrıda 1 ile 1000 arasında nesne incelenir; 0 verildi.", "uids"),
        refused("kimlik klasörden önce", {"uids": ["B-1"], "folder": "ekler"}, "invalid_uid",
                "“B-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uids/0"),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.attachments.inspect", "commandVersion": 1,
            "title": "Ekleri incele",
            "note": "ADR 0442 §7: eklerin baytlarından türü, boyu, özeti ve EXIF'i. Written by "
                    "scripts/fixtures/attachments_inspect_cases.py without KentOS code; do not edit by hand.",
            "setup": setup(), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    n = len(json.loads(text)["cases"])
    check = "--check" in sys.argv[1:]
    stale = [name for name, data in WRITTEN.items() if not (FILES / name).exists() or (FILES / name).read_bytes() != data]
    if check:
        if stale or not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} ya da {FILES.relative_to(ROOT)} yeniden üretilenle aynı değil "
                  "(betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({n} durum)")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for name, data in WRITTEN.items():
        (FILES / name).write_bytes(data)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({n} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
