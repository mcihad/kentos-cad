#!/usr/bin/env python3
"""The shared cases of Tablo ekle's file reader (docs/adr/0184 §4).

    python3 scripts/fixtures/table_file_cases.py           # writes the files and the cases
    python3 scripts/fixtures/table_file_cases.py --check   # writes nothing; compares

Writes the sample files of fixtures/table/v1/files (text tables in several
encodings and separators, an Excel workbook built here with zipfile and
hand-written XML, an old Excel file's head, another archive) and
fixtures/table/v1/files.json, each file's sheets as the rule reads them.
The rule is written here from the ADR on its own, not from an
implementation's output: the workbook is read with Python's zipfile and
xml.etree, the text with this script's own record reader. The formats core
(`table_file::read`, natively and through WASM: `readTableFile`) is held to
it.

- A file over 64 MB is refused; one starting with D0 CF 11 E0 (an old Excel
  file) is refused; a zip archive is a workbook when it holds
  xl/workbook.xml, else refused.
- A text file's encoding: a byte order mark (EF BB BF UTF-8, FF FE UTF-16LE,
  FE FF UTF-16BE), else UTF-8 when the bytes are UTF-8, else Windows-1254.
- Its records: fields split by the separator; a field starting with a quote
  (spaces before it aside) is quoted up to the next lone quote ("" one
  quote), line breaks inside it kept, what follows it up to the separator
  added as it is; an unquoted field loses the spaces round it; a record ends
  at a line break (CR LF, LF or CR) outside quotes; a line with nothing on
  it is no record.
- The separator: of tab, semicolon, comma and bar, in that order, the first
  that gives the most of the first 50 records the field count most of them
  have (the larger of two counts as common), when that count is two or
  more; else runs of spaces and tabs (no quoting) when they give at least
  half of those records the count most of them have, two or more; else the
  whole line is one field (without the spaces round it).
- At most 10 001 rows and 101 columns are read; a sheet with more rows says
  it was cut (`cut`).
- A workbook's sheets in xl/workbook.xml's order, each by its name through
  its relationship (a target relative to xl/, or absolute from the root).
  A cell where its reference (A1, from 0) puts it, else after the one before
  in its row (a row where its r says, else after the one before); a shared
  string's text: its t elements' text, those under rPh left out; an inline
  string's alike; a boolean 1 DOĞRU, 0 YANLIŞ; anything else its v's text as
  written; an empty value no cell. The rows end at the last with a cell; a
  row ends at its last cell.
"""
import io
import json
import sys
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/table/v1"
FILES = DIR / "files"
OUT = DIR / "files.json"
MAX_BYTES = 64 * 1024 * 1024
ROWS, COLUMNS = 10_001, 101
SAMPLE = 50

LABELS = {"utf-8": "UTF-8", "utf-16-le": "UTF-16", "utf-16-be": "UTF-16", "cp1254": "Windows-1254 (Türkçe)"}
NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
REL = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id"

# ── Text ────────────────────────────────────────────────────────────────


def encoding(data):
    if data.startswith(b"\xef\xbb\xbf"):
        return "utf-8", 3
    if data.startswith(b"\xff\xfe"):
        return "utf-16-le", 2
    if data.startswith(b"\xfe\xff"):
        return "utf-16-be", 2
    try:
        data.decode("utf-8")
        return "utf-8", 0
    except UnicodeDecodeError:
        return "cp1254", 0


def records(text, sep, limit):
    """Records split by sep (a character, 'spaces' or 'line'), at most limit; and whether more followed."""
    out = []
    i, n = 0, len(text)
    while i < n:
        if len(out) == limit:
            return out, True
        fields, field, quoted, was_quoted, anything = [], "", False, False, False
        while i < n:
            c = text[i]
            i += 1
            if quoted:
                if c == '"':
                    if i < n and text[i] == '"':
                        field += '"'
                        i += 1
                    else:
                        quoted = False
                else:
                    field += c
                continue
            if c in "\r\n":
                if c == "\r" and i < n and text[i] == "\n":
                    i += 1
                break
            anything = True
            if sep not in ("spaces", "line") and c == sep:
                fields.append(field if was_quoted else field.strip(" "))
                field, was_quoted = "", False
            elif sep not in ("spaces", "line") and c == '"' and field.strip(" ") == "" and not was_quoted:
                field, quoted, was_quoted = "", True, True
            elif sep == "spaces" and c in " \t":
                if field:
                    fields.append(field)
                    field = ""
            else:
                field += c
        if not anything and field == "" and not fields and not was_quoted:
            continue
        if sep == "spaces":
            if field:
                fields.append(field)
        else:
            fields.append(field if was_quoted else field.strip(" "))
        out.append(fields)
    return out, False


def steadiness(rows):
    counts = {}
    for r in rows:
        counts[len(r)] = counts.get(len(r), 0) + 1
    if not counts:
        return 0, 0
    return max((k, n) for n, k in counts.items())


def separator(text):
    best = None
    for c in ["\t", ";", ",", "|"]:
        sample, _ = records(text, c, SAMPLE)
        alike, count = steadiness(sample)
        if count >= 2 and (best is None or alike > best[0]):
            best = (alike, c)
    if best:
        return best[1]
    sample, _ = records(text, "spaces", SAMPLE)
    alike, count = steadiness(sample)
    return "spaces" if count >= 2 and 2 * alike >= len(sample) else "line"


def read_text(data):
    enc, skip = encoding(data)
    text = data[skip:].decode(enc, errors="replace")
    rows, cut = records(text, separator(text), ROWS)
    rows = [r[:COLUMNS] for r in rows]
    return {"sheets": [{"rows": rows, **({"cut": True} if cut else {})}], "encoding": LABELS[enc]}


# ── Workbook ────────────────────────────────────────────────────────────


def runs_text(node):
    out = []

    def walk(n, phonetic):
        tag = n.tag.split("}")[-1]
        phonetic = phonetic or tag == "rPh"
        if tag == "t" and not phonetic:
            out.append(n.text or "")
        for ch in n:
            walk(ch, phonetic)

    walk(node, False)
    return "".join(out)


def reference(r):
    letters = ""
    for ch in r:
        if ch.isascii() and ch.isalpha():
            letters += ch
        else:
            break
    if not letters or len(letters) > 3:
        return None
    col = 0
    for ch in letters.upper():
        col = col * 26 + (ord(ch) - ord("A") + 1)
    try:
        row = int(r[len(letters):])
    except ValueError:
        return None
    if row < 1:
        return None
    return col - 1, row - 1


def read_book(data):
    z = zipfile.ZipFile(io.BytesIO(data))
    names = z.namelist()
    if "xl/workbook.xml" not in names:
        return {"sheets": [], "problem": "Arşiv bir Excel çalışma kitabı (XLSX) değil. Tabloyu XLSX, CSV ya da TXT olarak seçin."}
    book = ET.fromstring(z.read("xl/workbook.xml"))
    targets = {}
    if "xl/_rels/workbook.xml.rels" in names:
        for rel in ET.fromstring(z.read("xl/_rels/workbook.xml.rels")):
            targets[rel.get("Id")] = rel.get("Target")
    shared = []
    if "xl/sharedStrings.xml" in names:
        shared = [runs_text(si) for si in ET.fromstring(z.read("xl/sharedStrings.xml")) if si.tag.endswith("}si")]
    sheets = []
    for sheet in book.iter("{%s}sheet" % NS["m"]):
        target = targets.get(sheet.get(REL))
        if target is None:
            continue
        path = target[1:] if target.startswith("/") else "xl/" + target
        if path not in names:
            continue
        doc = ET.fromstring(z.read(path))
        rows, cut, next_row = [], False, 0
        for row in doc.iter("{%s}row" % NS["m"]):
            r = row.get("r")
            i = int(r) - 1 if r and r.isdigit() and int(r) >= 1 else next_row
            next_row = i + 1
            if i >= ROWS:
                cut = True
                continue
            next_col = 0
            for c in row.findall("m:c", NS):
                ref = reference(c.get("r")) if c.get("r") else None
                j = ref[0] if ref else next_col
                next_col = j + 1
                if j >= COLUMNS:
                    continue
                v = c.find("m:v", NS)
                v = v.text if v is not None and v.text is not None else ""
                t = c.get("t")
                if t == "s":
                    try:
                        k = int(v.strip())
                        words = shared[k] if 0 <= k < len(shared) else ""
                    except ValueError:
                        words = ""
                elif t == "inlineStr":
                    inline = c.find("m:is", NS)
                    words = runs_text(inline) if inline is not None else ""
                elif t == "b":
                    words = {"1": "DOĞRU", "0": "YANLIŞ"}.get(v.strip(), v.strip())
                else:
                    words = v
                if words == "":
                    continue
                while len(rows) <= i:
                    rows.append([])
                while len(rows[i]) <= j:
                    rows[i].append("")
                rows[i][j] = words
        sheets.append({"name": sheet.get("name"), "rows": rows, **({"cut": True} if cut else {})})
    if not sheets:
        return {"sheets": [], "problem": "Çalışma kitabında okunacak sayfa yok."}
    return {"sheets": sheets}


def read(data):
    if len(data) > MAX_BYTES:
        return {"sheets": [], "problem": "Dosya 64 MB'tan büyük; bu kadar büyük tablo okunmuyor."}
    if data.startswith(b"\xd0\xcf\x11\xe0"):
        return {"sheets": [], "problem": "Eski Excel biçimi (.xls) okunmuyor. Dosyayı Excel'de XLSX ya da CSV olarak kaydedip seçin."}
    if data[:4] in (b"PK\x03\x04", b"PK\x05\x06"):
        return read_book(data)
    return read_text(data)


# ── The files ───────────────────────────────────────────────────────────

MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
RELS = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"


def workbook():
    """Two sheets: Koordinatlar (shared, inline and rich strings, a phonetic run, numbers, a boolean, an error, a gap) and Notlar (an absolute target)."""
    shared = (
        f'<sst xmlns="{MAIN}" count="5" uniqueCount="5">'
        "<si><t>Nokta</t></si>"
        "<si><t>Y</t></si>"
        "<si><t>X</t></si>"
        "<si><r><t>Sınır </t></r><r><rPr><b/></rPr><t>taşı</t></r><rPh sb=\"0\" eb=\"1\"><t>yok</t></rPh></si>"
        "<si><t xml:space=\"preserve\"> Ağaç  </t></si>"
        "</sst>"
    )
    sheet1 = (
        f'<worksheet xmlns="{MAIN}"><sheetData>'
        '<row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1" t="s"><v>2</v></c><c r="D1" t="inlineStr"><is><t>Not</t></is></c></row>'
        '<row r="2"><c r="A2"><v>101</v></c><c r="B2"><v>487000.125</v></c><c r="C2"><v>4420000.0625</v></c><c r="D2" t="s"><v>3</v></c></row>'
        '<row r="3"><c r="A3"><v>102</v></c><c r="B3"><v>1.5E-3</v></c><c r="C3" t="e"><v>#N/A</v></c><c r="D3" t="b"><v>1</v></c></row>'
        '<row r="5"><c r="A5" t="str"><v>P4</v></c><c r="C5" t="b"><v>0</v></c><c t="s"><v>4</v></c></row>'
        '<row><c><v>7</v></c><c r="C6"><v></v></c></row>'
        "</sheetData></worksheet>"
    )
    sheet2 = (
        f'<worksheet xmlns="{MAIN}"><sheetData>'
        '<row r="2"><c r="B2" t="inlineStr"><is><r><t>İl</t></r><r><t>çe</t></r></is></c></row>'
        "</sheetData></worksheet>"
    )
    book = (
        f'<workbook xmlns="{MAIN}" xmlns:r="{RELS}"><sheets>'
        '<sheet name="Koordinatlar" sheetId="1" r:id="rId1"/>'
        '<sheet name="Notlar" sheetId="2" r:id="rId2"/>'
        "</sheets></workbook>"
    )
    rels = (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>'
        '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet2.xml"/>'
        '<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>'
        "</Relationships>"
    )
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, text in [
            ("[Content_Types].xml", '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>'),
            ("xl/workbook.xml", book),
            ("xl/_rels/workbook.xml.rels", rels),
            ("xl/sharedStrings.xml", shared),
            ("xl/worksheets/sheet1.xml", sheet1),
            ("xl/worksheets/sheet2.xml", sheet2),
        ]:
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 6, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, text.encode("utf-8"))
    return out.getvalue()


def other_zip():
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        info = zipfile.ZipInfo("notlar.txt", date_time=(2026, 10, 6, 0, 0, 0))
        z.writestr(info, b"merhaba")
    return out.getvalue()


def samples():
    semicolon = (
        "Ada;Parsel;Nitelik;Alan\r\n"
        "101;5;\"Arsa; bahçe\";1340,13\r\n"
        "101;6;\"İki\r\nsatır \"\"tırnak\"\"\";747,00\r\n"
        "\r\n"
        "  102 ;  7 ;Tarla;  \r\n"
    )
    comma = "Nokta,Y,X,Z\n101,487000.125,4420000.500,812.4\n102,487042.5,4419996.75,\n"
    tab = "Ad\tKod\tAçıklama\r\nŞ1\tSN\tŞev üstü ğüşıöç\r\nŞ2\tSN\tİşaret\r\n"
    spaces = "101   487000.125   4420000.500\n102   487042.500   4419996.750\n 103 487047.000 4420024.000\n"
    single = "Birinci satır\n  ikinci, virgüllü ; satır  \nüçüncü\n"
    bar = "a|b|c\n1|2|3\n4||6\n"
    many = "".join(f"{i};{i * 2}\n" for i in range(10_003))
    return [
        ("noktali-virgul.csv", b"\xef\xbb\xbf" + semicolon.encode("utf-8")),
        ("virgul.csv", comma.encode("utf-8")),
        ("sekme-1254.txt", tab.encode("cp1254")),
        ("bosluk.txt", spaces.encode("utf-8")),
        ("tek-sutun.txt", single.encode("utf-8")),
        ("utf16.txt", b"\xff\xfe" + tab.encode("utf-16-le")),
        ("cubuk.csv", bar.encode("utf-8")),
        ("cok-satir.csv", many.encode("utf-8")),
        ("kitap.xlsx", workbook()),
        ("eski.xls", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + b"\0" * 504),
        ("baska.zip", other_zip()),
    ]


def build():
    files = samples()
    cases = [{"file": name, "want": read(data)} for name, data in files]
    doc = {
        "format": "kentos.table-file-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/table_file_cases.py",
        "title": "Tablo ekle: CSV, TXT ve XLSX dosyalarının satırları (ADR 0184 §4)",
        "cases": cases,
    }
    return files, doc


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    files, doc = build()
    if "--check" in sys.argv[1:]:
        bad = [name for name, data in files if not (FILES / name).exists() or (FILES / name).read_bytes() != data]
        if not OUT.exists() or OUT.read_text("utf-8") != text_of(doc):
            bad.append(OUT.name)
        for name in bad:
            print(f"{name}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print("table file cases match")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for name, data in files:
        (FILES / name).write_bytes(data)
    OUT.write_text(text_of(doc), encoding="utf-8")
    print(f"written: {len(files)} files, {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
