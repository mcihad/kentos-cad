"""The sheet core's magnetic model (docs/sheet/design.md §8a): NOAA/NCEI's World Magnetic Model
WMM2025 as JSON, made from its official coefficient file.

    python3 scripts/geodesy/wmm_coefficients.py           # writes crates/shared/sheet/data/wmm2025.json
    python3 scripts/geodesy/wmm_coefficients.py --check   # writes nothing; fails when the table is stale
    python3 scripts/geodesy/wmm_coefficients.py --fetch   # downloads NOAA's zip again and checks it is the file kept

The official files are kept in fixtures/sheet/v1/wmm/, so the table is made again without the network:
WMM.COF and WMM2025_TestValues.txt as they come in WMM2025COF.zip, and WMM2025_TEST_VALUES.txt from the
model's page (the test values the core is held to, within 0.01°). The coefficient file's SHA-256 is
written into the table and checked here: another file is refused.

The model is a work of the U.S. Government (NOAA's National Centers for Environmental Information):
public domain in the United States (17 U.S.C. § 105), no licence needed. Standard library only.
"""
import hashlib
import json
import sys
import urllib.request
import zipfile
from io import BytesIO
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
COF = ROOT / "fixtures/sheet/v1/wmm/WMM.COF"
OUT = ROOT / "crates/shared/sheet/data/wmm2025.json"

PAGE = "https://www.ncei.noaa.gov/products/world-magnetic-model"
ZIP = "https://www.ncei.noaa.gov/sites/default/files/2024-12/WMM2025COF.zip"
# The coefficient file's digest (WMM.COF in WMM2025COF.zip, header “2025.0 WMM-2025 11/13/2024”).
SHA256 = "dfa8597825af4e0b87ff4198a5b4fb661b3c49f4cd090cd0164e0259b075582f"

META = {
    "schema": "kentos.sheet.wmm/1",
    "model": "WMM2025",
    "producer": "NOAA National Centers for Environmental Information (NCEI) ve British Geological Survey (BGS)",
    # Released on 17 December 2024 (the model's page); the file's own header dates its coefficients.
    "released": "2024-12-17",
    "validFrom": 2025.0,
    "validUntil": 2030.0,
    "source": ZIP + " (WMM.COF)",
    "page": PAGE,
    "licence": "Kamu malı: ABD hükümetinin eseri (NOAA/NCEI), ABD'de telif yok (17 U.S.C. § 105); lisans gerekmez",
    "generator": "scripts/geodesy/wmm_coefficients.py",
}


def parse(text):
    """The header (epoch, model, date) and every (n, m, g, h, ġ, ḣ) line up to the nines."""
    lines = [l for l in text.splitlines() if l.strip()]
    epoch, model, dated = lines[0].split()
    rows = []
    for line in lines[1:]:
        if line.strip().startswith("9999"):
            break
        n, m, g, h, gd, hd = line.split()
        rows.append([int(n), int(m), float(g), float(h), float(gd), float(hd)])
    return float(epoch), model, dated, rows


def iso(us_date):
    month, day, year = us_date.split("/")
    return f"{int(year):04d}-{int(month):02d}-{int(day):02d}"


def build(raw):
    digest = hashlib.sha256(raw).hexdigest()
    if digest != SHA256:
        raise SystemExit(f"WMM.COF beklenen dosya değil: SHA-256 {digest} (beklenen {SHA256}).")
    epoch, model, dated, rows = parse(raw.decode("ascii"))
    if model != "WMM-2025" or epoch != 2025.0:
        raise SystemExit(f"WMM.COF'un başlığı beklenmedik: {model} {epoch}.")
    degree = max(r[0] for r in rows)
    # Every (n, m) of degree 1…12 once.
    want = [(n, m) for n in range(1, degree + 1) for m in range(0, n + 1)]
    if [(r[0], r[1]) for r in rows] != want:
        raise SystemExit("WMM.COF'un katsayıları eksik ya da sırasız.")
    return {
        **META,
        "epoch": epoch,
        "coefficientsDated": iso(dated),
        "sha256": digest,
        "maxDegree": degree,
        "coefficients": rows,
    }


def text(table):
    """The header fields one a line, then one coefficient row a line: readable, small diffs."""
    head = {k: v for k, v in table.items() if k != "coefficients"}
    lines = ["{"]
    for k, v in head.items():
        lines.append(f"  {json.dumps(k)}: {json.dumps(v, ensure_ascii=False)},")
    lines.append('  "coefficients": [')
    rows = [f"    {json.dumps(r)}" for r in table["coefficients"]]
    lines.append(",\n".join(rows))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def fetch():
    """NOAA's zip again: its WMM.COF must be the file kept (the digest above)."""
    with urllib.request.urlopen(ZIP, timeout=60) as r:
        data = r.read()
    with zipfile.ZipFile(BytesIO(data)) as z:
        name = next(n for n in z.namelist() if n.endswith("/WMM.COF") or n == "WMM.COF")
        raw = z.read(name)
    digest = hashlib.sha256(raw).hexdigest()
    if digest != SHA256:
        print(f"NOAA'nın dosyası değişmiş: SHA-256 {digest} (tutulan {SHA256}).", file=sys.stderr)
        return 1
    print(f"NOAA'nın WMM.COF'u tutulanla aynı ({SHA256[:16]}…).")
    return 0


def main():
    if "--fetch" in sys.argv[1:]:
        return fetch()
    want = text(build(COF.read_bytes()))
    if "--check" in sys.argv[1:]:
        have = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        if have != want:
            print(
                f"{OUT.relative_to(ROOT)} WMM.COF'a göre güncel değil: python3 scripts/geodesy/wmm_coefficients.py ile yeniden üretin.",
                file=sys.stderr,
            )
            return 1
        print("WMM2025 tablosu güncel.")
        return 0
    OUT.write_text(want, encoding="utf-8")
    print(f"WMM2025: {len(json.loads(want)['coefficients'])} katsayı satırı yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
