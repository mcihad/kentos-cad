#!/usr/bin/env python3
"""Paketle's ways for bound sources and what is left out (docs/adr/0596), checked without KentOS code.

    python3 scripts/fixtures/package_options_cases.py          # writes fixtures/package/v1/cases.json
    python3 scripts/fixtures/package_options_cases.py --check  # fails when the file differs

Each case is a drawing (bound GeoPackage tables, a database's link, linked pictures) and a `file.package` call; its
answer is built here from ADR 0596's rules and ADR 0506's: the copies (their names taken once, their sizes and SHA-256s
read with hashlib), the links left and why, the links embedded, and, for a package made, its project's links and the
bound layers' object counts (the tables' rows, read with sqlite3). crates/native/headless/tests/headless.rs
(`package_cases`) plays them.

The setup:

- `links`: tables bound in order (`kaynak-1` …) from a copy of fixtures/geopackage/v1/qgis.gpkg in the work folder named
  `file`; each on a new layer `import-<table>`. `gone`: the file removed after the project is saved.
- `databases`: the links made PostGIS links (`postgresql://cbs@127.0.0.1:1/kent`, table `public.<table>`): no
  database answers there, so a copy is refused `source_unreachable`.
- `pictures`: linked images in the active layer with their lasting ids; `foto/logo.png` is in the work folder, any other
  path is missing.

The project is saved in the work folder (`from`), opened, and packed into `$work:paket`.
"""

import hashlib
import json
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/package/v1/cases.json"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"
LOGO = ROOT / "fixtures/interaction/v1/logo.png"
ADDRESS = "postgresql://cbs@127.0.0.1:1/kent"
UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{:04x}"


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def rows(table):
    with sqlite3.connect(f"file:{GPKG}?mode=ro", uri=True) as c:
        return c.execute(f'SELECT count(*) FROM "{table}"').fetchone()[0]


def answer(setup, given, op):
    """What `file.package` answers, and for an execute the package's project."""
    links = []
    for k, l in enumerate(setup.get("links", [])):
        database = k in setup.get("databases", [])
        links.append(
            {
                "id": f"kaynak-{k + 1}",
                "table": l["table"],
                "layer": f"import-{l['table']}",
                "file": l["file"],
                "gone": l.get("gone", False),
                "database": database,
                "objects": rows(l["table"]),
            }
        )
    pictures = setup.get("pictures", [])
    leave = given.get("leaveOut", [])
    known = {l["id"] for l in links} | {p["uid"] for p in pictures}
    for i, entry in enumerate(leave):
        if entry not in known:
            return {"status": "failed", "error": {"code": "invalid_input", "path": f"leaveOut[{i}]"}}
    geopackages = given.get("geopackages", "copy")
    databases = given.get("databases", "keep")
    files, left, embedded = [], [], []
    names = {}

    def take(name, kind, source):
        if name in names:
            files[names[name]]["links"] += 1
            return
        names[name] = len(files)
        files.append({"path": f"veri/{name}", "kind": kind, "links": 1, "bytes": source.stat().st_size, "sha256": sha(source)})

    for p in pictures:
        if p["uid"] in leave:
            left.append({"kind": "picture", "path": p["file"], "why": "excluded", "at": p["uid"]})
        elif p["file"] == "foto/logo.png":
            take("logo.png", "picture", LOGO)
        else:
            left.append({"kind": "picture", "path": p["file"], "why": "missing", "at": p["uid"]})
    project = []
    counts = {}
    for l in links:
        path = ADDRESS if l["database"] else f"$work:{l['file']}"
        table = f"public.{l['table']}" if l["database"] else l["table"]
        counts[l["layer"]] = l["objects"]
        if l["id"] in leave:
            left.append({"kind": "source", "path": path, "why": "excluded", "at": l["id"], "objects": l["objects"]})
            counts[l["layer"]] = 0
            continue
        way = databases if l["database"] else geopackages
        if way == "embed":
            embedded.append({"at": l["id"], "layer": l["layer"], "path": path, "table": table, "objects": l["objects"]})
            continue
        if way == "keep":
            left.append({"kind": "source", "path": path, "why": "database" if l["database"] else "kept", "at": l["id"]})
            project.append([l["id"], "postgis" if l["database"] else "geopackage", None if l["database"] else l["file"], table])
            continue
        if l["database"]:
            if op == "validate":
                continue
            return {"status": "failed", "error": {"code": "source_unreachable", "path": "databases"}}
        if l["gone"]:
            left.append({"kind": "source", "path": path, "why": "missing", "at": l["id"]})
            project.append([l["id"], "geopackage", l["file"], table])
            continue
        take(l["file"], "source", GPKG)
        project.append([l["id"], "geopackage", f"veri/{l['file']}", table])
    out = {"status": "completed"}
    if op == "validate":
        return out
    out.update({"files": files, "left": left, "embedded": embedded, "warnings": ["package_left"] if left else []})
    if op == "execute":
        out["project"] = {"sources": project, "layers": counts}
    return out


TWO = {"links": [{"table": "parsel", "file": "kadastro.gpkg"}, {"table": "yol", "file": "kadastro.gpkg"}]}
ONE = {"links": [{"table": "parsel", "file": "kadastro.gpkg"}]}
MIXED = {
    "links": [{"table": "parsel", "file": "kadastro.gpkg"}, {"table": "yol", "file": "kadastro.gpkg"}],
    "databases": [1],
}
PICTURES = {
    "links": [{"table": "parsel", "file": "kadastro.gpkg"}],
    "pictures": [
        {"uid": UID.format(0x3000), "file": "foto/logo.png"},
        {"uid": UID.format(0x3001), "file": "foto/logo.png"},
        {"uid": UID.format(0x3002), "file": "/yok/kayip.jpg"},
    ],
}
GONE = {"links": [{"table": "parsel", "file": "kadastro.gpkg", "gone": True}]}
TWO_FILES = {"links": [{"table": "parsel", "file": "kadastro.gpkg"}, {"table": "nokta", "file": "noktalar.gpkg"}]}

CASES = [
    ("GeoPackage'lar varsayılanla kopyalanır", ONE, {}, "execute"),
    ("Aynı dosyanın iki tablosu tek kopyayı paylaşır", TWO, {}, "execute"),
    ("İki dosyanın tabloları iki kopya", TWO_FILES, {}, "execute"),
    ("Bağlı bırak: dosya kopyalanmaz, bağ kalır", ONE, {"geopackages": "keep"}, "execute"),
    ("Göm: bağ kalkar, katman nesneleriyle kalır", ONE, {"geopackages": "embed"}, "execute"),
    ("Göm iki bağı da gömer", TWO, {"geopackages": "embed"}, "execute"),
    ("Yerinde olmayan dosya kopyalanmaz, söylenir", GONE, {}, "execute"),
    ("Yerinde olmayan dosyanın bağı gömülür", GONE, {"geopackages": "embed"}, "execute"),
    ("Yerinde olmayan dosya bağlı bırakılır, aranmaz", GONE, {"geopackages": "keep"}, "execute"),
    ("Dışarıda bırakılan bağ: katman boşalır, bağ kalkar", TWO, {"leaveOut": ["kaynak-1"]}, "execute"),
    ("İki bağ da dışarıda", TWO, {"leaveOut": ["kaynak-1", "kaynak-2"]}, "execute"),
    ("Dışarıda bırakmak gömmeden önce gelir", TWO, {"geopackages": "embed", "leaveOut": ["kaynak-2"]}, "execute"),
    ("Veritabanı varsayılanla bağlı kalır", MIXED, {}, "execute"),
    ("Veritabanı gömülür, GeoPackage kopyalanır", MIXED, {"databases": "embed"}, "execute"),
    ("Veritabanı dışarıda bırakılır", MIXED, {"leaveOut": ["kaynak-2"]}, "execute"),
    ("Veritabanı ve GeoPackage gömülür", MIXED, {"databases": "embed", "geopackages": "embed"}, "execute"),
    ("Ulaşılamayan veritabanının kopyası reddedilir", MIXED, {"databases": "copy"}, "execute"),
    ("Planda da veritabanı okunur ve reddedilir", MIXED, {"databases": "copy"}, "plan"),
    ("Doğrulama veritabanını okumaz", MIXED, {"databases": "copy"}, "validate"),
    ("Dışarıda bırakılan veritabanı kopyalanmaz", MIXED, {"databases": "copy", "leaveOut": ["kaynak-2"]}, "execute"),
    ("Resimler kopyalanır, eksik olan söylenir", PICTURES, {}, "execute"),
    ("Dışarıda bırakılan resim kopyalanmaz, öbürü kopyalanır", PICTURES, {"leaveOut": [UID.format(0x3000)]}, "execute"),
    ("İki resim de dışarıda: dosya yok", PICTURES, {"leaveOut": [UID.format(0x3000), UID.format(0x3001)]}, "execute"),
    ("Planda dosyalar sayılır, yazılmaz", PICTURES, {"leaveOut": ["kaynak-1"]}, "plan"),
    ("Olmayan bağ adı reddedilir", TWO, {"leaveOut": ["kaynak-9"]}, "execute"),
    ("İkinci öğe yoksa yolu onu gösterir", TWO, {"leaveOut": ["kaynak-1", "xref:plan"]}, "execute"),
    ("Olmayan nesne kimliği reddedilir", PICTURES, {"leaveOut": [UID.format(0x3999)]}, "plan"),
    ("Ret veritabanı okunmadan gelir", MIXED, {"databases": "copy", "leaveOut": ["kaynak-3"]}, "execute"),
]


def document():
    cases = []
    for name, setup, given, op in CASES:
        cases.append({"name": name, "setup": setup, "op": op, "input": given, "expect": answer(setup, given, op)})
    return {
        "format": "kentos.package-cases",
        "version": 1,
        "command": "file.package",
        "note": "Written by scripts/fixtures/package_options_cases.py (docs/adr/0596); see its docstring.",
        "cases": cases,
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} is not what the reference writes; run without --check and read the diff")
            return 1
        print(f"{len(CASES)} cases agree")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{len(CASES)} cases written to {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
