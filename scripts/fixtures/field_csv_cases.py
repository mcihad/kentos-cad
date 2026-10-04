#!/usr/bin/env python3
"""Independent reference of the plain-text field book's reading (docs/adr/0169 §1–§2; step 1).

Writes fixtures/field/v1/csv.json from the rules alone, no KentOS code: a CSV or TXT field book whose columns the user
maps (istasyon, alet yüksekliği, nokta, yatay açı, başucu açısı, eğik uzunluk, prizma yüksekliği, kod) read into the
book's stations and observations.

The rules:

1. Lines end with LF or CR LF; an empty line (spaces only) is skipped. With `header`, the first line that is not empty
   names the columns and is not read.
2. The separator is the first line's most frequent of tab, semicolon and comma; on a tie tab before semicolon before
   comma; a line with none is one cell. Cells are trimmed.
3. A number is read as the Hesap windows read one: trimmed, the first comma a point (only when the separator is not a
   comma), `^[-+]?(\\d+(\\.\\d*)?|\\.\\d+)(e[-+]?\\d+)?$`.
4. Stations: a station cell that is not empty and not the current station's name starts a station (its instrument
   height from its row when given). Without a station column the book has one station, unnamed.
5. A row with a target is an observation: its horizontal reading is required, the zenith, slope distance, target height
   and code are optional. A target height left empty is the station's last one (instruments write it when it changes).
   A row with a station and no target only sets the station (and its instrument height).
6. A cell that should be a number and is not is named with its line. A bad instrument height leaves the whole row
   out; a bad cell of the observation's leaves out only the observation (the row's station still starts); an
   observation without a horizontal reading likewise.
7. The first line that is not empty is said as its cells (trimmed), header or not: the columns to map.
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "field" / "v1" / "csv.json"
NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)
NAMES = {
    "instrumentHeight": "alet yüksekliği",
    "hz": "yatay açı",
    "zenith": "başucu açısı",
    "slope": "eğik uzunluk",
    "targetHeight": "prizma yüksekliği",
}
TEXTS = {
    "row": "Satır {line}: {what} “{text}” sayı değil; satır okunmadı.",
    "point": "Satır {line}: {what} “{text}” sayı değil; nokta okunmadı.",
    "hz": "Satır {line}: {target} noktasının yatay açısı yok; nokta okunmadı.",
}


def number(text, comma):
    t = text.strip()
    if comma:
        t = t.replace(",", ".", 1)
    return float(t) if t and NUMBER.match(t) else None


def separator(line):
    counts = [(line.count(s), -i, s) for i, s in enumerate(["\t", ";", ","])]
    best = max(counts)
    return best[2] if best[0] > 0 else None


def read(text, mapping, header):
    lines = text.replace("\r\n", "\n").split("\n")
    numbered = [(i + 1, l) for i, l in enumerate(lines) if l.strip()]
    if not numbered:
        return {"firstLine": [], "stations": [], "problems": []}
    sep = separator(numbered[0][1])
    first_line = [c.strip() for c in numbered[0][1].split(sep)] if sep else [numbered[0][1].strip()]
    comma = sep != ","
    if header:
        numbered = numbered[1:]
    stations, problems = [], []
    current = None

    def cell(cells, key):
        col = mapping.get(key)
        return cells[col].strip() if col is not None and col < len(cells) else ""

    def read_numbers(cells, line, keys, said):
        values = {}
        for key in keys:
            t = cell(cells, key)
            if not t:
                continue
            v = number(t, comma)
            if v is None:
                problems.append({"line": line, "problem": TEXTS[said].format(line=line, what=NAMES[key], text=t)})
                return None
            values[key] = v
        return values

    for line, raw in numbered:
        cells = [c.strip() for c in (raw.split(sep) if sep else [raw])]
        values = read_numbers(cells, line, ["instrumentHeight"], "row")
        if values is None:
            continue
        name = cell(cells, "station") if mapping.get("station") is not None else None
        if current is None or (name and name != current["station"]):
            current = {"station": name or "", "observations": []}
            stations.append(current)
            current["lastTarget"] = None
        if "instrumentHeight" in values:
            current["instrumentHeight"] = values["instrumentHeight"]
        target = cell(cells, "target")
        if not target:
            continue
        observed = read_numbers(cells, line, ["hz", "zenith", "slope", "targetHeight"], "point")
        if observed is None:
            continue
        values.update(observed)
        if "hz" not in values:
            problems.append({"line": line, "problem": TEXTS["hz"].format(line=line, target=target)})
            continue
        th = values.get("targetHeight", current["lastTarget"])
        current["lastTarget"] = th
        o = {"target": target, "hz": values["hz"], "line": line}
        for key in ("zenith", "slope"):
            if key in values:
                o[key] = values[key]
        if th is not None:
            o["targetHeight"] = th
        code = cell(cells, "code")
        if code:
            o["code"] = code
        current["observations"].append(o)
    for s in stations:
        s.pop("lastTarget", None)
    return {"firstLine": first_line, "stations": stations, "problems": problems}


MAP_FULL = {"station": 0, "instrumentHeight": 1, "target": 2, "hz": 3, "zenith": 4, "slope": 5, "targetHeight": 6, "code": 7}


def cases():
    return [
        ("noktalı virgül ve virgüllü ondalık, başlıklı", MAP_FULL, True,
         "İstasyon;Alet;Nokta;Hz;V;Eğik;Prizma;Kod\n"
         "P1;1,552;P2;0,0012;99,8765;245,678;1,700;POL\n"
         "P1;;101;87,4321;101,2345;63,214;;BINA\n"
         "P1;;102;95,1000;100,5000;70,100;1,500;BINA\n"
         "P2;1,48;P1;0;100,1234;245,679;1,7;POL\n"),
        ("virgülle ayrılmış, başlıksız, CR LF", MAP_FULL, False,
         "P1,1.552,P2,0.0012,99.8765,245.678,1.700,POL\r\n\r\nP1,,101,87.4321,101.2345,63.214,,BINA\r\n"),
        ("sekmeyle ayrılmış, istasyon satırı ayrı", MAP_FULL, False,
         "S1\t1.6\t\t\t\t\t\t\nS1\t\tA\t10.5\t99.5\t100\t1.3\t\nS1\t\tB\t20.25\t\t\t\t\n"),
        ("istasyon sütunu yok", {"target": 0, "hz": 1, "zenith": 2, "slope": 3}, False,
         "A;1;100;50\nB;2;99,5;60\n"),
        ("sayı olmayan hücre ve yatay açısız nokta", MAP_FULL, False,
         "P1;1,5;A;10;100;abc;1,3;\nP1;;B;;100;20;;\nP1;;C;30;100;40;;\n"),
    ]


def build():
    out = []
    for name, mapping, header, text in cases():
        out.append({"name": name, "mapping": mapping, "header": header, "text": text, "expect": read(text, mapping, header)})
    return {"format": "kentos.field-csv", "version": 1, "source": "scripts/fixtures/field_csv_cases.py (docs/adr/0169 §1–§2)", "texts": TEXTS, "names": NAMES, "cases": out}


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = parser.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
