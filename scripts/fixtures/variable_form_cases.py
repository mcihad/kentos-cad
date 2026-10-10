#!/usr/bin/env python3
"""Independent reference of Proje ayarları › Değişkenler's form (docs/adr/0214 §2.3, §4).

Writes fixtures/project/v1/variable-form.json from the rules alone, no KentOS code: the project's variables as the
form's rows of texts (name, label, kind, value), the rows read back into variables with what is said of a row that
does not hold, the row a new variable starts as, a row whose kind changes, and the `@` values every expression of the
project reads besides them (the built-in ones).

The rules:

1. A name is trimmed and one leading `@` dropped. It is a letter or `_`, then letters, digits and `_`, at most 64
   characters; it may not be a built-in variable's (proje_adi, koordinat_sistemi, epsg, olcek, tarih, simdi,
   katman_adi, katman, kullanici). Names are compared with Turkish letters and case aside (ı i İ I → I, ğ → G, ü → U,
   ş → S, ö → O, ç → C, then upper case): a later row with an earlier row's name is said to repeat it.
2. A label is trimmed.
3. A value is read by its kind; an empty (blank) text is no value:
   - text: trimmed, at most 4 000 characters;
   - number: as the Hesap windows read a number: trimmed, its first comma a point,
     ^[-+]?(\\d+(\\.\\d*)?|\\.\\d+)(e[-+]?\\d+)?$ (any case), finite;
   - date: YYYY-AA-GG, or G.A.YYYY / GG.AA.YYYY written back as YYYY-AA-GG; a day of the calendar, years 1–9999;
   - true/false: "doğru" or "yanlış" (the form's switch writes them).
4. More than 200 rows: the list is said to be too long.
5. A variable written for the form: its name, label and kind as they are; its value empty when none, a text or a date
   as it is, true/false as "doğru"/"yanlış", a number as the shortest text that reads back as it, without an exponent
   (-0 is "0").
6. A new row: the first free name of degisken1, degisken2 … (compared as rule 1), no label, text, no value.
7. A row's kind changes: to true/false its value is "doğru" when it was, else "yanlış"; from true/false to another kind
   its value is emptied; between text, number and date the text stays (rule 3 says whether it holds).
8. The built-in values, after the project's own variables: @proje_adi the project's name; @koordinat_sistemi the name
   of its system (the registry's, or its own definition's; none for a local project); @epsg the registry's code (none
   for a local project or a definition); @olcek the scale's denominator; @tarih today, @simdi now to the second, both
   ISO and local, as the time core writes them (a moment at midnight is its date, docs/adr/0210 §3); @kullanici the
   signed-in person's name, none when no one is. A variable of the project is described
   by its label, else "Projenin değişkeni (metin | sayı | doğru/yanlış | tarih)".
"""

import argparse
import json
import re
import sys
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "project" / "v1" / "variable-form.json"

BUILTINS = [
    ("proje_adi", "Projenin adı"),
    ("koordinat_sistemi", "Koordinat sisteminin adı"),
    ("epsg", "Koordinat sisteminin EPSG kodu"),
    ("olcek", "Çizim ölçeğinin paydası"),
    ("tarih", "Bugünün tarihi"),
    ("simdi", "Şimdiki tarih ve saat"),
    ("katman_adi", "Değerlendirilen nesnenin katmanının adı"),
    ("katman", "Değerlendirilen nesnenin katmanının adı"),
    ("kullanici", "Oturumdaki kullanıcının adı"),
]
# The two systems the cases name, as the registry names them.
REGISTRY = {5254: "TUREF / TM30", 2320: "ED50 / TM30"}
KIND_NAME = {"text": "metin", "number": "sayı", "bool": "doğru/yanlış", "date": "tarih"}
TEXTS = {
    "nameEmpty": "Bir ad yazın: harf ya da _ ile başlar, harf, rakam ve _ içerir.",
    "number": "Sayı yazın (1.5 ya da 1,5); değeri yoksa boş bırakın.",
    "date": "Tarihi YYYY-AA-GG ya da GG.AA.YYYY yazın; değeri yoksa boş bırakın.",
    "bool": "Doğru ya da yanlış seçin.",
    "tooMany": "En çok 200 değişken olabilir.",
}
TRUE, FALSE = "doğru", "yanlış"
NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)
FOLD = {"ı": "I", "i": "I", "İ": "I", "I": "I", "ğ": "G", "Ğ": "G", "ü": "U", "Ü": "U", "ş": "S", "Ş": "S", "ö": "O",
        "Ö": "O", "ç": "C", "Ç": "C"}


def key(name):
    return "".join(FOLD.get(c, c.upper()) for c in name)


def name_problem(name):
    if not name:
        return TEXTS["nameEmpty"]
    if len(name) > 64:
        return f"“{name}” değişken adı 64 karakterden uzun."
    first_ok = name[0] == "_" or name[0].isalpha()
    rest_ok = all(c == "_" or c.isalpha() or c.isdigit() for c in name)
    if not (first_ok and rest_ok):
        return f"“{name}” değişken adı olamaz: bir harf ya da _ ile başlar, harf, rakam ve _ içerir."
    if any(key(b) == key(name) for b, _ in BUILTINS):
        return f"@{name} yerleşik bir değişkendir; başka ad seçin."
    return None


def leap(y):
    return y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)


def calendar_day(y, m, d):
    if not (1 <= y <= 9999 and 1 <= m <= 12):
        return False
    days = [31, 29 if leap(y) else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1]
    return 1 <= d <= days


def date_value(t):
    m = re.fullmatch(r"(\d{4})-(\d{2})-(\d{2})", t)
    if m:
        y, mo, d = map(int, m.groups())
        return t if calendar_day(y, mo, d) else None
    m = re.fullmatch(r"(\d{1,2})\.(\d{1,2})\.(\d{4})", t)
    if m:
        d, mo, y = map(int, m.groups())
        return f"{y:04d}-{mo:02d}-{d:02d}" if calendar_day(y, mo, d) else None
    return None


def number(text):
    t = text.strip().replace(",", ".", 1)
    if not t or not NUMBER.match(t):
        return None
    x = float(t)
    return x if x not in (float("inf"), float("-inf")) else None


def value_of(kind, text):
    """(value, problem) of a typed value."""
    t = text.strip()
    if not t:
        return None, None
    if kind == "text":
        return (t, None) if len(t) <= 4000 else (None, "Değer 4000 karakterden uzun.")
    if kind == "number":
        x = number(t)
        return (x, None) if x is not None else (None, TEXTS["number"])
    if kind == "date":
        d = date_value(t)
        return (d, None) if d is not None else (None, TEXTS["date"])
    if t == TRUE:
        return True, None
    if t == FALSE:
        return False, None
    return None, TEXTS["bool"]


def number_text(x):
    if x == 0:
        return "0"
    return format(Decimal(repr(x)).normalize(), "f")


def value_text(value):
    if value is None:
        return ""
    if isinstance(value, bool):
        return TRUE if value else FALSE
    if isinstance(value, float):
        return number_text(value)
    return value


def variable(name, kind, value, label=""):
    """A variable as a project keeps it: a label only when there is one, a value only when there is one."""
    v = {"name": name}
    if label:
        v["label"] = label
    v["kind"] = kind
    if value is not None:
        v["value"] = value
    return v


def row(name, kind="text", value="", label=""):
    return {"name": name, "label": label, "kind": kind, "value": value}


def rows_of(variables):
    return [row(v["name"], v.get("kind", "text"), value_text(v.get("value")), v.get("label", "")) for v in variables]


def read(rows):
    variables, problems, seen = [], [], set()
    for r in rows:
        name = r["name"].strip()
        if name.startswith("@"):
            name = name[1:]
        named = name_problem(name)
        if named is None and key(name) in seen:
            named = f"@{name} adı yukarıda var; başka ad seçin."
        if named is None:
            seen.add(key(name))
        value, valued = value_of(r["kind"], r["value"])
        problems.append({"name": named, "value": valued})
        if named is None and valued is None:
            variables.append(variable(name, r["kind"], value, r["label"].strip()))
    return {"variables": variables, "problems": problems, "list": TEXTS["tooMany"] if len(rows) > 200 else None}


def new_row(rows):
    taken = {key(r["name"].strip().lstrip("@")) for r in rows}
    n = 1
    while key(f"degisken{n}") in taken:
        n += 1
    return row(f"degisken{n}")


def with_kind(r, kind):
    out = dict(r, kind=kind)
    if kind == "bool":
        out["value"] = TRUE if r["value"].strip() == TRUE else FALSE
    elif r["kind"] == "bool":
        out["value"] = ""
    return out


def wall(iso):
    """A local moment as the time core writes it (docs/adr/0210 §3): today's date; now to the second, a moment at
    midnight being its date."""
    day, time = iso.split("T")
    second = time[:8]
    return day, day if second == "00:00:00" else f"{day}T{second}"


def builtins(case):
    s = case["settings"]
    own = [{"name": v["name"], "value": v.get("value"), "description": v.get("label") or f"Projenin değişkeni ({KIND_NAME[v.get('kind', 'text')]})"}
           for v in s.get("variables", [])]
    custom = s.get("customCrs")
    system = custom["name"] if custom else (None if s["srid"] == 0 else REGISTRY[s["srid"]])
    epsg = None if custom or s["srid"] == 0 else float(s["srid"])
    today, now = wall(case["now"])
    values = [("proje_adi", case["name"]), ("koordinat_sistemi", system), ("epsg", epsg), ("olcek", float(s["plotScale"])),
              ("tarih", today), ("simdi", now), ("kullanici", case["user"] or None)]
    described = dict(BUILTINS)
    return own + [{"name": n, "value": v, "description": described[n]} for n, v in values]


def cases():
    reads = [
        ("geçerli satırlar", [row("is_no", "text", " 2026/41 ", " İş numarası "), row("@Katsayı", "number", "1,5"),
                               row("teslim", "date", "28.2.2026"), row("onaylı", "bool", TRUE), row("boş", "number", "  ")]),
        ("sayılar", [row("a", "number", "-0.25"), row("b", "number", "1e3"), row("c", "number", ".5"), row("d", "number", "+7"),
                     row("e", "number", "1.000,5"), row("f", "number", "12a"), row("g", "number", "1e400")]),
        ("tarihler", [row("t1", "date", "2024-02-29"), row("t2", "date", "2026-02-29"), row("t3", "date", "1.10.2026"),
                      row("t4", "date", "2026/10/01"), row("t5", "date", "31.04.2026"), row("t6", "date", "0000-01-01")]),
        ("adlar", [row(""), row("1a"), row("Proje_Adı"), row("ada no"), row("_özel"), row("ç" * 65), row("@@x")]),
        ("tekrar eden ad", [row("İş_No"), row("is_no"), row("IS_NO"), row("ada")]),
        ("doğru/yanlış", [row("x", "bool", FALSE), row("y", "bool", ""), row("z", "bool", "evet")]),
        ("uzun metin", [row("not", "text", "a" * 4001), row("not2", "text", "ğ" * 4000)]),
    ]
    texts = [
        ("türler", [variable("is_no", "text", "2026/41", "İş numarası"), variable("Katsayı", "number", 1.5),
                    variable("tam", "number", 2026.0), variable("küçük", "number", 1e-7), variable("büyük", "number", 1e21),
                    variable("eksi_sıfır", "number", -0.0), variable("teslim", "date", "2026-02-28"),
                    variable("onaylı", "bool", True), variable("ret", "bool", False), variable("boş", "text", None)]),
    ]
    new_rows = [
        ("boş liste", []),
        ("dolu", [row("degisken1"), row("@Degisken2"), row("degisken4")]),
        ("Türkçe harfle", [row("DEĞİŞKEN1")]),
    ]
    kinds = [
        ("metinden sayıya", row("a", "text", "12"), "number"),
        ("sayıdan tarihe", row("a", "number", "12"), "date"),
        ("metinden doğru/yanlışa", row("a", "text", "doğru"), "bool"),
        ("sayıdan doğru/yanlışa", row("a", "number", "1"), "bool"),
        ("doğru/yanlıştan metne", row("a", "bool", TRUE), "text"),
    ]
    builtin_cases = [
        {"case": "kayıttaki sistem", "name": "Kızılay", "user": "Ayşe Yılmaz", "now": "2026-10-10T14:30:05.250",
         "settings": {"srid": 5254, "plotScale": 500, "variables": [variable("is_no", "text", "2026/41", "İş numarası"),
                                                                    variable("katsayı", "number", 1.5)]}},
        {"case": "yerel proje", "name": "Şantiye", "user": "", "now": "2026-01-01T00:00:00.000",
         "settings": {"srid": 0, "plotScale": 1000, "variables": []}},
        {"case": "özel sistem", "name": "Ada 101", "user": "M. Demir", "now": "2026-12-31T23:59:59.999",
         "settings": {"srid": 0, "plotScale": 2000, "customCrs": {"name": "Şantiye sistemi"},
                      "variables": [variable("teslim", "date", "2027-03-01")]}},
    ]
    return {
        "reads": [{"case": c, "rows": rows, **read(rows)} for c, rows in reads],
        "rows": [{"case": c, "variables": vs, "rows": rows_of(vs)} for c, vs in texts],
        "newRows": [{"case": c, "rows": rows, "row": new_row(rows)} for c, rows in new_rows],
        "kinds": [{"case": c, "row": r, "kind": k, "result": with_kind(r, k)} for c, r, k in kinds],
        "builtins": [{**c, "variables": builtins(c)} for c in builtin_cases],
        "tooMany": read([row(f"v{i}") for i in range(201)])["list"],
    }


def build():
    return {"format": "kentos.variable-form", "version": 1, "source": "scripts/fixtures/variable_form_cases.py (docs/adr/0214 §2.3, §4)", "messages": TEXTS, **cases()}


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
    OUT.write_text(text, encoding="utf-8")
    b = build()
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(b['reads'])} okuma, {len(b['rows'])} yazım, {len(b['newRows'])} yeni satır, "
          f"{len(b['kinds'])} tür değişimi, {len(b['builtins'])} yerleşik değer durumu.")


if __name__ == "__main__":
    main()
