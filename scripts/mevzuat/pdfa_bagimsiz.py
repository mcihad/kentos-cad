#!/usr/bin/env python3
"""A permit package checked without KentOS's code (docs/adr/8066, plan/bim-plan.md BIM-05 "bağımsız doğrulama"): every PDF
of a delivery folder read by qpdf and Poppler, as another program reads it, against the regulation's text:

- qpdf --check: the file's structure;
- pdfinfo: one page (Md. 8: every drawing its own file), the page an A4 multiple (Md. 8: "A4 ve katları"; each side a
  multiple of 210 or 297 mm within 1.5 mm, at least A4);
- pdfinfo -meta: the XMP packet's PDF/A identification (part 2, conformance B; Md. 4/2, KentOS's K13 reading);
- pdffonts: every font embedded (ISO 19005-2 6.2.11.4.1);
- pdfimages -list and pdftotext: not a scanned page (e-PYS Md. 6/4): text or fonts on it, or no page-sized picture;
- the name: EK-2 Tablo 2.1's nine fields joined by "_" (2.1.1), lengths 6, 2, 2, 4, 2, 3, 3, 2, 3;
- the delivery list (teslim-listesi.csv), when there is one: each delivered file once — the PDFs, and a BIM permit
  package's IFC model (docs/adr/8075) — its SHA-256 the file's. KentOS's own records (yayin.json, the form's record
  kalite-kontrol-formu.json) and the list itself are not delivered files.

It prints one JSON object: each file's problems (none: it passes) and the list's. Exit status 0 when nothing failed.

    python3 -I scripts/mevzuat/pdfa_bagimsiz.py <delivery folder>
"""

import csv
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

MM = 25.4 / 72.0
LENGTHS = (6, 2, 2, 4, 2, 3, 3, 2, 3)


def run(*args):
    r = subprocess.run(args, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def a4_multiple(side):
    return any(abs(side - round(side / a) * a) <= 1.5 and round(side / a) >= 1 for a in (210.0, 297.0))


def check(path: Path):
    problems = []
    code, out, err = run("qpdf", "--check", str(path))
    if code not in (0, 3):  # 3: warnings only
        problems.append(f"qpdf: {(err or out).strip().splitlines()[-1] if (err or out).strip() else code}")
    code, info, _ = run("pdfinfo", str(path))
    pages = re.search(r"^Pages:\s+(\d+)", info, re.M)
    if not pages or int(pages.group(1)) != 1:
        problems.append(f"{pages.group(1) if pages else '?'} sayfa: her çizim ayrı dosya, bir sayfa olmalı")
    size = re.search(r"^Page size:\s+([\d.]+) x ([\d.]+) pts", info, re.M)
    if size:
        w, h = float(size.group(1)) * MM, float(size.group(2)) * MM
        if not (a4_multiple(w) and a4_multiple(h) and min(w, h) >= 208.5 and max(w, h) >= 295.5):
            problems.append(f"kâğıt {w:.0f}×{h:.0f} mm: A4 ve katları değil")
    else:
        problems.append("sayfa boyu okunamadı")
    _, meta, _ = run("pdfinfo", "-meta", str(path))
    part = re.search(r"pdfaid:part[>=\"']+\s*(\d)", meta)
    conf = re.search(r"pdfaid:conformance[>=\"']+\s*([ABU])", meta)
    if not (part and part.group(1) == "2" and conf and conf.group(1) == "B"):
        problems.append("XMP'de PDF/A-2b kimliği yok")
    _, fonts, _ = run("pdffonts", str(path))
    rows = [line for line in fonts.splitlines()[2:] if line.strip()]
    unembedded = [line.split()[0] for line in rows if len(line.split()) >= 5 and line.split()[-5] == "no"]
    if unembedded:
        problems.append(f"gömülmemiş yazı tipi: {', '.join(unembedded)}")
    _, images, _ = run("pdfimages", "-list", str(path))
    _, text, _ = run("pdftotext", "-q", str(path), "-")
    image_rows = [line for line in images.splitlines()[2:] if line.strip()]
    if image_rows and not rows and not text.strip():
        problems.append("taranmış sayfa: yazısız, yazı tipsiz, yalnız resim")
    name = path.stem.split("_")
    if len(name) != len(LENGTHS) or any(len(f) != n or not re.fullmatch(r"[A-Z0-9]+", f) for f, n in zip(name, LENGTHS)):
        problems.append("ad EK-2 Tablo 2.1'in dokuz alanı değil")
    return problems


def main(folder: Path):
    files = sorted(folder.glob("*.pdf"))
    verdict = {"files": {f.name: check(f) for f in files}, "list": []}
    listed = folder / "teslim-listesi.csv"
    if listed.exists():
        with listed.open(encoding="utf-8-sig", newline="") as fh:
            rows = list(csv.DictReader(fh, delimiter=";"))
        names = [r["Dosya"] for r in rows]
        own = {"yayin.json", "teslim-listesi.csv", "kalite-kontrol-formu.json"}
        delivered = sorted(p.name for p in folder.iterdir() if p.is_file() and p.name not in own)
        if sorted(names) != delivered:
            verdict["list"].append("liste klasördeki dosyaları birer kez saymıyor")
        for r in rows:
            p = folder / r["Dosya"]
            if p.exists() and hashlib.sha256(p.read_bytes()).hexdigest() != r["SHA-256"]:
                verdict["list"].append(f"{r['Dosya']}: SHA-256 listedekiyle aynı değil")
    else:
        verdict["list"].append("teslim-listesi.csv yok")
    print(json.dumps(verdict, ensure_ascii=False, sort_keys=True))
    failed = any(verdict["files"].values()) or verdict["list"] or not files
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main(Path(sys.argv[1]))
