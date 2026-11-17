#!/usr/bin/env python3
"""Packages for the administration's review (docs/adr/8104; plan/bim-plan.md BIM-18's first acceptance), made here from
KentOS's own package (fixtures/bim/v1/paket/kentos: the declared section building published with its BIM part, on
TUREF / TM36, EPSG:5256) without KentOS code: the IFC edits are IfcOpenShell's, the PDF edits byte for byte, and every
list (yayin.json, teslim-listesi.csv, the form's record) is brought up to the files as an author's program would, so
each package breaks only what its name says:

- bozuk/ — the first acceptance's three: the model on TM33 (EPSG:5255: another zone); the ground floor's east wall
  (01alMXGa9m080000000002) without its fire rating (Pset_WallCommon.FireRating taken away), the form still saying Evet;
  the ground floor's plan (UP-03) made from the drawing's revision 8, the rest from 9.
- dosyalar/ — the list against the folder: UP-04 missing, UP-02 changed after the list, a note the list does not name,
  the site plan declared PDF/A-3.
- taranmis/ — UP-01 replaced by a page that is only a picture (a scanned sheet), its list brought up.
- okunmayan/ — the model replaced by text that is not STEP, its list brought up.
- listesiz/ — the model and two sheets, no yayin.json.
- formsuz/ — the package without its form record.

And review_packages.json: what each package breaks, by layer (the review's cases read it). `--check` compares. Runs
under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/review_packages.py [--check]
"""

import csv
import hashlib
import io
import json
import shutil
import sys
import tempfile
import zlib
from pathlib import Path

import ifcopenshell
import ifcopenshell.util.element as ue

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "fixtures/bim/v1/paket"
GOOD = BASE / "kentos"
NOTE = BASE / "review_packages.json"
MODEL = "260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc"
UP01 = "260123_00_MM_XXXX_GN_MMC_NNN_01_000.pdf"
UP02 = "260123_00_MM_XXXX_PL_VAZ_NNN_01_000.pdf"
UP03 = "260123_00_MM_XXXX_PL_MMC_K00_01_000.pdf"
UP04 = "260123_00_MM_XXXX_PL_MMC_K01_01_000.pdf"
FORM = "kalite-kontrol-formu.json"
LIST = "yayin.json"
DELIVERY = "teslim-listesi.csv"
RATED = "01alMXGa9m080000000002"
OLD_SHA = hashlib.sha256(b"cizimin 8. revizyonu").hexdigest()


def sha(b):
    return hashlib.sha256(b).hexdigest()


def files_of(folder):
    return {p.name: p.read_bytes() for p in sorted(folder.iterdir()) if p.is_file()}


def relisted(files, drop=(), keep_listing=()):
    """yayin.json, the delivery list and the form's record brought up to `files` (name -> bytes); `keep_listing` names
    files whose old listing is kept (changed after the list)."""
    old = json.loads(files[LIST])
    listing = {f["name"]: f for f in old["files"]}
    new_files = []
    for f in old["files"]:
        n = f["name"]
        if n in drop:
            new_files.append(f)  # still listed
            continue
        if n not in files:
            continue
        if n in keep_listing:
            new_files.append(f)
            continue
        b = files[n]
        new_files.append(dict(f, bytes=len(b), sha256=sha(b)))
    old["files"] = new_files
    if FORM in files:
        form = json.loads(files[FORM])
        if MODEL in files:
            form["model"]["bytes"] = len(files[MODEL])
            form["model"]["sha256"] = sha(files[MODEL])
        fb = (json.dumps(form, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        files[FORM] = fb
        old["bim"]["form"]["bytes"] = len(fb)
        old["bim"]["form"]["sha256"] = sha(fb)
    files[LIST] = (json.dumps(old, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if DELIVERY in files:
        text = files[DELIVERY].decode("utf-8-sig")
        rows = list(csv.reader(io.StringIO(text), delimiter=";"))
        out = [rows[0]]
        for r in rows[1:]:
            n = r[1]
            if n in files and n not in keep_listing and n not in drop:
                r[2], r[3] = sha(files[n]), str(len(files[n]))
            out.append(r)
        buf = io.StringIO()
        csv.writer(buf, delimiter=";", lineterminator="\n").writerows(out)
        files[DELIVERY] = ("﻿" + buf.getvalue()).encode("utf-8")
    for n in drop:
        files.pop(n, None)
    return files


def ifc_edit(b, edit):
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "m.ifc"
        p.write_bytes(b)
        f = ifcopenshell.open(str(p))
        edit(f)
        f.write(str(p))
        return p.read_bytes()


def to_tm33(f):
    crs = f.by_type("IfcProjectedCRS")[0]
    crs.Name = "EPSG:5255"
    crs.Description = "TUREF / TM33 (EPSG:5255)"
    crs.MapZone = "33"


def without_fire_rating(f):
    wall = f.by_guid(RATED)
    pset = f.by_id(ue.get_psets(wall)["Pset_WallCommon"]["id"])
    keep = [p for p in pset.HasProperties if p.Name != "FireRating"]
    gone = [p for p in pset.HasProperties if p.Name == "FireRating"]
    pset.HasProperties = keep
    for p in gone:
        f.remove(p)


def older_source(pdf):
    """The sheet's source said as the drawing's revision 8: the same length in its keywords and their XMP copy."""
    out = pdf
    for q in (b'"', b"&quot;"):
        out = out.replace(q + b"revision" + q + b":" + q + b"9" + q, q + b"revision" + q + b":" + q + b"8" + q)
    import re
    out = re.sub(rb'(sha256(?:"|&quot;):(?:"|&quot;))([0-9a-f]{64})((?:"|&quot;)\}\})',
                 lambda m: m.group(1) + OLD_SHA.encode() + m.group(3), out)
    assert out != pdf and len(out) == len(pdf)
    return out


def scanned_pdf():
    """A one-page PDF whose page is only a picture: an 8 × 8 grey image drawn over the page (no line, no text)."""
    img = zlib.compress(bytes([128] * 64))
    content = b"q 595 0 0 842 0 0 cm /Im1 Do Q"
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /XObject << /Im1 5 0 R >> >> /Contents 4 0 R >>",
        b"<< /Length %d >>\nstream\n" % len(content) + content + b"\nendstream",
        b"<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray /BitsPerComponent 8 "
        b"/Filter /FlateDecode /Length %d >>\nstream\n" % len(img) + img + b"\nendstream",
    ]
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for i, o in enumerate(objs, start=1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    for off in offsets:
        out += b"%010d 00000 n \n" % off
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return bytes(out)


def packages():
    good = files_of(GOOD)
    made = {}

    f = dict(good)
    f[MODEL] = ifc_edit(f[MODEL], lambda m: (to_tm33(m), without_fire_rating(m)))
    f[UP03] = older_source(f[UP03])
    made["bozuk"] = (relisted(f), {
        "konum": ["review_place_zone"],
        "pafta": [["review_sheet_revision", UP03]],
        "model": [RATED],
        "form": ["review_form_disagrees"],
    })

    f = dict(good)
    f[UP02] = f[UP02] + b"\n% sonradan eklendi\n"
    f[UP01] = f[UP01].replace(b"<pdfaid:part>2</pdfaid:part>", b"<pdfaid:part>3</pdfaid:part>")
    f["notlar.txt"] = "Müellifin notu.\n".encode("utf-8")
    made["dosyalar"] = (relisted(f, drop=(UP04,), keep_listing=(UP02,)), {
        "paket": [["review_file_missing", UP04], ["review_file_changed", UP02], ["review_file_unlisted", "notlar.txt"]],
        "pafta": [["review_sheet_not_pdfa", UP01]],
    })

    f = {k: v for k, v in good.items() if k not in (UP02, UP03, UP04)}
    f[UP01] = scanned_pdf()
    lst = json.loads(f[LIST])
    lst["files"] = [x for x in lst["files"] if x["name"] in (UP01, MODEL)]
    f[LIST] = json.dumps(lst).encode("utf-8")
    made["taranmis"] = (relisted(f), {"pafta": [["review_sheet_not_pdfa", UP01], ["review_sheet_scanned", UP01]]})

    f = {k: v for k, v in good.items() if k not in (UP02, UP03, UP04)}
    f[MODEL] = b"Bu bir IFC dosyasi degil.\n"
    lst = json.loads(f[LIST])
    lst["files"] = [x for x in lst["files"] if x["name"] in (UP01, MODEL)]
    f[LIST] = json.dumps(lst).encode("utf-8")
    made["okunmayan"] = (relisted(f), {"model": ["review_model_unreadable"]})

    f = {k: v for k, v in good.items() if k in (MODEL, UP03, UP04)}
    made["listesiz"] = (f, {"paket": ["review_no_list"]})

    f = {k: v for k, v in good.items() if k in (MODEL, UP03, LIST)}
    lst = json.loads(f[LIST])
    lst["files"] = [x for x in lst["files"] if x["name"] in (UP03, MODEL)]
    f[LIST] = (json.dumps(lst, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")
    made["formsuz"] = (f, {"paket": ["review_no_form"]})
    return made


def main():
    check = "--check" in sys.argv
    made = packages()
    stale = []
    note = {"source": "fixtures/bim/v1/paket/kentos", "writer": "IfcOpenShell " + ifcopenshell.version,
            "system": 5256, "packages": {k: v[1] for k, v in made.items()}}
    for name, (files, _) in made.items():
        folder = BASE / name
        if check:
            have = files_of(folder) if folder.is_dir() else {}
            if have != files:
                stale.append(name)
            continue
        if folder.exists():
            shutil.rmtree(folder)
        folder.mkdir(parents=True)
        for n, b in files.items():
            (folder / n).write_bytes(b)
    text = json.dumps(note, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not NOTE.exists() or NOTE.read_text(encoding="utf-8") != text:
            stale.append(NOTE.name)
    else:
        NOTE.write_text(text, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(("güncel: " if check else "yazıldı: ") + ", ".join(made))
    return 0


if __name__ == "__main__":
    sys.exit(main())
