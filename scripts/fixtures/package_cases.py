#!/usr/bin/env python3
"""Paketle, checked without KentOS code (docs/adr/0506, plan/gis.md CBS-36).

    python3 scripts/fixtures/package_cases.py

Runs crates/native/headless's package test (a bound GeoPackage, a linked picture twice, a raster with its world file, a
missing file and a database's link; packed, the folder moved and the old data removed), then reads the moved package
with the independent KCAD reader (tools/kcad/kcad.py) and hashlib:

- the folder holds the project file and veri/ with exactly the four copies, each byte for byte its source fixture;
- every relative link resolves inside the folder; the two pictures share one copy, the raster's world file sits
  beside its copy under its name;
- the bound layer's source keeps relative veri/kadastro.gpkg, its file's SHA-256 is the link's revision;
- the missing file's link and the database's address are left as they were, the database without a relative path;
- the project's own data profile and mapping (profiller/ beside the project file, docs/adr/0558) are copied under the
  same names, byte for byte, read here as JSON (a profile's name is its file's name); the folder's other file and the
  folder inside it are not in the package; a file the disk does not give is left out, said (`unreadable`);
- paket.json (docs/adr/0350 §4) names the project file and every copy with their sizes and SHA-256s as read here.

Then the transmittal test (an external reference, an image found by a search path, a style naming an SHX font): the
reference's source is copied byte for byte, so the block's kept digest is the copy's SHA-256 and its relative path
`veri/plan.kcad`; the image's link is its copy; the manifest lists the copies and says the missing nested source and the
substituted font.
"""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402

FIX = ROOT / "fixtures"
SOURCES = {
    "veri/logo.png": FIX / "interaction/v1/logo.png",
    "veri/rgb.png": FIX / "raster/v1/files/rgb.png",
    "veri/rgb.pgw": FIX / "raster/v1/files/rgb.pgw",
    "veri/kadastro.gpkg": FIX / "geopackage/v1/qgis.gpkg",
}
# The project's profile and mapping (docs/adr/0558): `profiller/` beside the project file, as the commands look for them.
PROFILES = {
    "profiller/park-bahce.profil.json": FIX / "profiles/v1/files/park-bahce.profil.json",
    "profiller/sivas-bel.eslem.json": FIX / "profiles/v1/files/sivas-bel.eslem.json",
}


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def manifest_problems(package, project):
    """paket.json against the folder as read here."""
    problems = []
    m = json.loads((package / "paket.json").read_text("utf-8"))
    if m.get("format") != "kentos.package/1":
        problems.append(f"manifestin biçimi {m.get('format')}")
    p = m.get("project", {})
    data = (package / project).read_bytes()
    if p != {"path": project, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}:
        problems.append(f"manifestin projesi {p}")
    for f in m.get("files", []):
        b = (package / f["path"]).read_bytes()
        if f["bytes"] != len(b) or f["sha256"] != hashlib.sha256(b).hexdigest():
            problems.append(f"manifestteki {f['path']} dosyasıyla aynı değil")
    listed = sorted(f["path"] for f in m.get("files", []))
    there = sorted(str(q.relative_to(package)) for folder in ("veri", "profiller")
                   for q in (package / folder).rglob("*") if q.is_file()
                   ) if any((package / f).is_dir() for f in ("veri", "profiller")) else []
    if listed != there:
        problems.append(f"manifestin dosyaları {listed} ≠ {there}")
    return problems, m


def transmittal():
    out = Path(tempfile.mkstemp(suffix=".json")[1])
    env = dict(os.environ, KENTOS_TRANSMIT_OUT=str(out), CARGO_BUILD_JOBS="3")
    run = subprocess.run(["cargo", "test", "-q", "-p", "kentos-headless", "--test", "headless",
                          "a_package_carries_the_references"], cwd=ROOT, env=env)
    if run.returncode != 0:
        return ["başsız sunucunun dış başvurulu paket testi düştü"]
    package = Path(json.loads(out.read_text("utf-8"))["package"])
    problems, m = manifest_problems(package, "ofis.kcad")
    src = FIX / "xref/v1/plan.kcad"
    if sha(package / "veri/plan.kcad") != sha(src):
        problems.append("dış başvurunun kopyası kaynağıyla aynı değil")
    _, doc = kcad.read((package / "ofis.kcad").read_bytes())
    tops = [b for b in doc.get("blocks", []) if "xref" in b and "parent" not in b["xref"]]
    if len(tops) != 1 or tops[0]["xref"].get("relative") != "veri/plan.kcad" \
            or tops[0]["xref"].get("digest") != sha(src):
        problems.append(f"dış başvurunun bağı {[t['xref'] for t in tops]}")
    files = [e.get("file") for e in doc["entities"] if e.get("kind") == "image"]
    if files != ["veri/logo.png"] or sha(package / "veri/logo.png") != sha(FIX / "interaction/v1/logo.png"):
        problems.append(f"resmin bağı {files}")
    left = [(x["kind"], x["why"]) for x in m.get("left", [])]
    if left != [("xref", "missing"), ("font", "substituted")]:
        problems.append(f"manifestin kalanları {left}")
    return problems


def main():
    out = Path(tempfile.mkstemp(suffix=".json")[1])
    env = dict(os.environ, KENTOS_PACKAGE_OUT=str(out), CARGO_BUILD_JOBS="3")
    run = subprocess.run(["cargo", "test", "-q", "-p", "kentos-headless", "--test", "headless",
                          "a_project_is_packed"], cwd=ROOT, env=env)
    if run.returncode != 0:
        print("başsız sunucunun paket testi düştü", file=sys.stderr)
        return 1
    package = Path(json.loads(out.read_text("utf-8"))["package"])
    problems = []
    found = sorted(str(p.relative_to(package)) for p in package.rglob("*") if p.is_file())
    # A file the disk does not give (the test locks one, unless it may read it anyway) is left out, said.
    locked = "profiller/kilitli.profil.json" not in found
    if not locked:
        PROFILES["profiller/kilitli.profil.json"] = FIX / "profiles/v1/files/peyzaj.profil.json"
    want = sorted(["kadastro.kcad", "paket.json", *SOURCES, *PROFILES])
    if found != want:
        problems.append(f"paketin dosyaları {found} ≠ {want}")
    for rel, src in {**SOURCES, **PROFILES}.items():
        p = package / rel
        if p.is_file() and sha(p) != sha(src):
            problems.append(f"{rel} kaynağıyla aynı değil")
    for rel, fmt in (("profiller/park-bahce.profil.json", "kentos.data-profile"),
                     ("profiller/sivas-bel.eslem.json", "kentos.profile-mapping")):
        try:
            held = json.loads((package / rel).read_text("utf-8"))
        except (OSError, ValueError) as e:
            problems.append(f"{rel} okunamadı: {e}")
            continue
        stem = Path(rel).name.split(".")[0]
        if held.get("format") != fmt or held.get("name") != stem:
            problems.append(f"{rel} paketin içinde bir profil ya da eşleme gibi okunmuyor")
    _, doc = kcad.read((package / "kadastro.kcad").read_bytes())
    files = [e.get("file") for e in doc["entities"] if "file" in e]
    if files != ["veri/logo.png", "veri/logo.png", "/yok/kayip.jpg", "veri/rgb.png"]:
        problems.append(f"bağlı dosyalar {files}")
    for f in files:
        if not f.startswith("/") and not (package / f).is_file():
            problems.append(f"{f} paketin içinde yok")
    sources = doc["settings"]["sources"]
    gp = next((s for s in sources if s["kind"] == "geopackage"), None)
    db = next((s for s in sources if s["kind"] == "postgis"), None)
    if gp is None or gp.get("relative") != "veri/kadastro.gpkg":
        problems.append(f"bağlı katmanın göreli yolu {gp and gp.get('relative')}")
    elif sha(package / gp["relative"]) != gp["revision"]:
        problems.append("kopyanın SHA-256'sı bağın sürümü değil")
    if db is None or db["path"] != "postgresql://cbs@veri.belediye.local/kent" or "relative" in db:
        problems.append(f"veritabanının bağı değişti: {db}")
    manifest_problems_, manifest = manifest_problems(package, "kadastro.kcad")
    problems += manifest_problems_
    kinds = {f["path"]: (f["kind"], f["links"]) for f in manifest.get("files", [])}
    for rel, kind in (("profiller/park-bahce.profil.json", "profile"), ("profiller/sivas-bel.eslem.json", "mapping")):
        if kinds.get(rel) != (kind, 0):
            problems.append(f"manifestte {rel} {kinds.get(rel)}; {(kind, 0)} olmalı")
    left = [(x["kind"], x["why"]) for x in manifest.get("left", [])]
    if locked and ("profile", "unreadable") not in left:
        problems.append(f"okunamayan profil manifestin kalanlarında yok: {left}")
    problems += transmittal()
    if problems:
        print("; ".join(problems), file=sys.stderr)
        return 1
    print(f"Paket tutarlı, manifestiyle; dış başvurulu paket de: {len(SOURCES)} kopya ve {len(PROFILES)} profil ve eşleme dosyası kaynaklarıyla bayt bayt aynı, "
          "bağlar paketin içinde, bağlı katmanın göreli yolu ve sürümü doğru, kalanlar olduğu gibi")
    return 0


if __name__ == "__main__":
    sys.exit(main())
