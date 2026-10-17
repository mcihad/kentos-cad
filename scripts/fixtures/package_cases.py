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
- the missing file's link and the database's address are left as they were, the database without a relative path.
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


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def main():
    out = Path(tempfile.mkstemp(suffix=".json")[1])
    env = dict(os.environ, KENTOS_PACKAGE_OUT=str(out), CARGO_BUILD_JOBS="2")
    run = subprocess.run(["cargo", "test", "-q", "-p", "kentos-headless", "--test", "headless",
                          "a_project_is_packed"], cwd=ROOT, env=env)
    if run.returncode != 0:
        print("başsız sunucunun paket testi düştü", file=sys.stderr)
        return 1
    package = Path(json.loads(out.read_text("utf-8"))["package"])
    problems = []
    found = sorted(str(p.relative_to(package)) for p in package.rglob("*") if p.is_file())
    want = sorted(["kadastro.kcad", *SOURCES])
    if found != want:
        problems.append(f"paketin dosyaları {found} ≠ {want}")
    for rel, src in SOURCES.items():
        p = package / rel
        if p.is_file() and sha(p) != sha(src):
            problems.append(f"{rel} kaynağıyla aynı değil")
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
    if problems:
        print("; ".join(problems), file=sys.stderr)
        return 1
    print(f"Paket tutarlı: {len(SOURCES)} kopya kaynaklarıyla bayt bayt aynı, bağlar paketin içinde, bağlı katmanın "
          "göreli yolu ve sürümü doğru, kalanlar olduğu gibi")
    return 0


if __name__ == "__main__":
    sys.exit(main())
