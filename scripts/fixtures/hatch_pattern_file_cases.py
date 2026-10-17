"""Writes the shared cases of PAT files and a project's hatch patterns (docs/adr/0331 §1): fixtures/hatch/v1/pat.json.

- `pat`: PAT files read as AutoCAD's documentation writes them ("About Hatch Pattern Definitions", without its files):
  a header `*NAME[,description]`, then one line a family: `angle, x-origin, y-origin, delta-x, delta-y [, dash-1,
  dash-2, …]` (a dash drawn when positive, a gap when negative, a dot when 0). A `;` starts a comment to the line's
  end; lines are trimmed; names are kept as written, compared without case, at most 64 letters and without a comma.
  A pattern takes at most 64 families, a family at most 16 dashes, every number finite and at most 10⁶ in size, the
  lines' distance (delta-y) not 0. The file says no unit: the reader is told it (`mm`, `inch` on paper, or `drawing`)
  and keeps the numbers as written. What does not read is said with its line, the rest is read; a pattern with a
  wrong family is left out whole.
- `written`: the patterns written back as PAT, numbers in the shortest form that reads back (`.5`, no trailing zeros).
- `scale`: a hatch's metres per unit of its pattern's definition: millimetres on paper times the drawing's scale over
  1000, inches 25.4 times that, the drawing's own units the project unit's ratio to the metre; times the hatch's
  scale factor.

Written from the rules above, without KentOS code. The formats core (`formats::pat`) and the contracts run them.

    python3 scripts/fixtures/hatch_pattern_file_cases.py           # writes the file
    python3 scripts/fixtures/hatch_pattern_file_cases.py --check   # compares it with the one on disk
"""
import json
import re
import sys
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/hatch/v1/pat.json"
MAX_LINES = 64
MAX_DASHES = 16
MAX_NAME = 64
MAX_SIZE = 1.0e6
UNIT_M = {"mm": (1.0, 1000.0), "cm": (1.0, 100.0), "m": (1.0, 1.0), "in": (254.0, 10000.0), "ft": (3048.0, 10000.0)}


def number(text):
    """A PAT number: `.5`, `-0.25`, `1e-2`; none for anything else."""
    t = text.strip()
    if not re.fullmatch(r"[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?", t):
        return None
    return float(t)


def family(parts):
    """A family's numbers, or why they do not read."""
    if len(parts) < 5:
        return None, f"bir aile en az beş sayıdır (açı, x, y, dx, dy), {len(parts)} var"
    values = []
    for p in parts:
        v = number(p)
        if v is None:
            return None, f"“{p.strip()}” bir sayı değil"
        if not (abs(v) <= MAX_SIZE):
            return None, f"“{p.strip()}” çok büyük; en çok {int(MAX_SIZE)} olur"
        values.append(v)
    if len(values) - 5 > MAX_DASHES:
        return None, f"bir ailenin en çok {MAX_DASHES} kesiği olur, {len(values) - 5} var"
    if values[4] == 0:
        return None, "çizgiler arası (dy) 0 olamaz"
    if len(values) > 5 and all(v == 0 for v in values[5:]):
        return None, "kesiklerin hepsi 0 olamaz; en az birinin uzunluğu olmalı"
    line = {"angle": values[0], "origin": [values[1], values[2]], "offset": [values[3], values[4]]}
    if len(values) > 5:
        line["dashes"] = values[5:]
    return line, None


def read_pat(text, unit, source):
    patterns, problems = [], []
    current = None  # {"name", "desc", "line", "lines", "bad"}

    def finish():
        if current is None:
            return
        name = current["name"]
        if current["bad"]:
            return
        if not current["lines"]:
            problems.append({"line": current["line"], "message": f"“{name}” deseninin ailesi yok"})
            return
        if len(current["lines"]) > MAX_LINES:
            problems.append({"line": current["line"],
                             "message": f"“{name}”: bir desen en çok {MAX_LINES} ailedir, {len(current['lines'])} var"})
            return
        if any(p["name"].lower() == name.lower() for p in patterns):
            problems.append({"line": current["line"], "message": f"“{name}” dosyada iki kez var; ilki alındı"})
            return
        p = {"name": name, "unit": unit, "lines": current["lines"], "source": source}
        if current["desc"]:
            p["description"] = current["desc"]
        patterns.append(p)

    for i, raw in enumerate(text.splitlines(), start=1):
        line = raw.split(";", 1)[0].strip()
        if not line:
            continue
        if line.startswith("*"):
            finish()
            current = None
            name, _, desc = line[1:].partition(",")
            name = name.strip()
            if not name:
                problems.append({"line": i, "message": "desenin adı boş"})
                current = {"name": "", "desc": "", "line": i, "lines": [], "bad": True}
                continue
            if len(name) > MAX_NAME:
                problems.append({"line": i, "message": f"“{name[:16]}…”: desenin adı en çok {MAX_NAME} harf olur"})
                current = {"name": name, "desc": "", "line": i, "lines": [], "bad": True}
                continue
            current = {"name": name, "desc": desc.strip(), "line": i, "lines": [], "bad": False}
            continue
        if current is None:
            problems.append({"line": i, "message": "aile bir *AD satırının altında olmalı"})
            continue
        if current["bad"]:
            continue
        fam, why = family(line.split(","))
        if fam is None:
            problems.append({"line": i, "message": f"“{current['name']}”: {why}; desen alınmadı"})
            current["bad"] = True
            continue
        current["lines"].append(fam)
    finish()
    return {"patterns": patterns, "problems": problems}


def pat_case(name, text, unit="mm", source="deneme.pat"):
    return {"name": name, "text": text, "unit": unit, "source": source, "expect": read_pat(text, unit, source)}


def pat_cases():
    many = "*COK\n" + "".join(f"{k},0,0,0,1\n" for k in range(MAX_LINES + 1))
    dashes17 = "*KESIK17\n0,0,0,0,1," + ",".join(["1", "-1"] * 8) + ",1\n"
    dashes16 = "*KESIK16\n0,0,0,0,1," + ",".join(["1", "-1"] * 8) + "\n"
    return [
        pat_case("ANSI31 gibi tek aile", "*ANSI31, ANSI Iron, Brick, Stone masonry\n45, 0,0, 0,3.175\n"),
        pat_case("Kesikli ve noktalı aileler", "*DASHDOT, Dashes and dots\n0, 0,0, 0,6.35, 6.35,-3.175, 0,-3.175\n"),
        pat_case("Birden çok aile ve açıklamasız", "*KARE\n0, 0,0, 0,5\n90, 0,0, 0,5\n"),
        pat_case("Yorumlar, boş satırlar ve satır sonu yorumu",
                 ";%TYPE=AutoCAD\n;; KentOS deneme\n\n*TUGLA, Tuğla ; örgü\n0, 0,0, 0,6.35 ; yatay\n90, 0,0, 6.35,6.35, 6.35,-6.35\n"),
        pat_case("İnç dosyası olduğu gibi tutulur", "*ANSI32, ANSI Steel\n45, 0,0, 0,.375\n45, .176776695,0, 0,.375, 0,-.375\n", unit="inch"),
        pat_case("Çizim birimli dosya", "*PARKE, Parke\n0, 0,0, .2,.1, .3,-.1\n", unit="drawing"),
        pat_case("Üstel ve işaretli sayılar", "*US\n+45, 1e-1,-2, 0,2.5E0, 1.5,-.5\n"),
        pat_case("Birden çok desen, sırasıyla", "*A1, Birinci\n0,0,0,0,1\n*B2, İkinci\n45,0,0,0,2\n"),
        pat_case("16 kesik sınırda", dashes16),
        pat_case("17 kesik reddedilir", dashes17),
        pat_case("65 aile reddedilir", many),
        pat_case("dy 0 reddedilir", "*SIFIR\n0, 0,0, 1,0\n*IYI\n0,0,0,0,1\n"),
        pat_case("Eksik sayı", "*EKSIK\n45, 0,0, 0\n"),
        pat_case("Sayı olmayan değer", "*HARF\n45, 0,0, 0,bir\n"),
        pat_case("Çok büyük sayı", "*BUYUK\n0, 0,0, 0,2000000\n"),
        pat_case("Ailesiz başlık", "*BOS, Ailesiz\n*DOLU\n0,0,0,0,1\n"),
        pat_case("Başlıksız aile", "0,0,0,0,1\n*SONRA\n0,0,0,0,1\n"),
        pat_case("Boş ad", "*, adsız\n0,0,0,0,1\n"),
        pat_case("Çok uzun ad", "*" + "U" * 65 + "\n0,0,0,0,1\n"),
        pat_case("Aynı ad iki kez (harf büyüklüğü sayılmaz)", "*Tugla\n0,0,0,0,1\n*TUGLA\n0,0,0,0,2\n"),
        pat_case("Türkçe ad ve açıklama", "*ÇİM_ALAN, Çim alan · yeşil\n60, 0,0, 2,4, 1,-3\n120, 0,0, 2,4, 1,-3\n"),
        pat_case("Pencere satır sonları ve kenar boşlukları", "  *KESIK , Kesikli  \r\n  0 , 0 , 0 , 0 , 1 , 2 , -1  \r\n"),
        pat_case("Kesiklerin hepsi 0 olamaz", "*SIFIRLAR\n0,0,0,0,1,0,0\n"),
        pat_case("Bozuk aile bütün deseni düşürür", "*YARIM\n0,0,0,0,1\n90,0,0,0,x\n*SAGLAM\n0,0,0,0,1\n"),
    ]


def short(x):
    """The shortest PAT form that reads back: no trailing zeros, `.5` not `0.5`, no exponent."""
    if x == int(x) and abs(x) < 1e15:
        return str(int(x))
    t = format(Decimal(repr(x)), "f")
    if t.startswith("0."):
        t = t[1:]
    elif t.startswith("-0."):
        t = "-" + t[2:]
    return t


def write_pat(patterns):
    out = []
    for p in patterns:
        out.append("*" + p["name"] + ("," + p["description"] if p.get("description") else ""))
        for l in p["lines"]:
            values = [l["angle"], l["origin"][0], l["origin"][1], l["offset"][0], l["offset"][1]] + l.get("dashes", [])
            out.append(",".join(short(v) for v in values))
    return "".join(line + "\n" for line in out)


def written_cases():
    cases = []
    for c in pat_cases():
        if c["expect"]["patterns"]:
            text = write_pat(c["expect"]["patterns"])
            again = read_pat(text, c["unit"], c["source"])
            assert again["patterns"] == c["expect"]["patterns"] and not again["problems"], c["name"]
            cases.append({"name": c["name"], "patterns": c["expect"]["patterns"], "pat": text})
    small = [{"name": "KUCUK", "unit": "mm", "lines": [
        {"angle": 0.1, "origin": [1e-7, -0.5], "offset": [0.0, 1.0 / 3.0], "dashes": [2.5, -1e-6, 0.0]}]}]
    cases.append({"name": "Küçük ve uzun kesirli sayılar", "patterns": small, "pat": write_pat(small)})
    return cases


def metres(unit, plot_scale, drawing_unit, factor):
    if unit == "mm":
        base = plot_scale / 1000.0
    elif unit == "inch":
        base = 25.4 * plot_scale / 1000.0
    else:
        mul, div = UNIT_M[drawing_unit]
        base = mul / div
    return base * factor


def scale_case(name, unit, plot_scale=100.0, drawing_unit="m", factor=1.0):
    return {"name": name, "unit": unit, "plotScale": plot_scale, "drawingUnit": drawing_unit, "factor": factor,
            "metres": metres(unit, plot_scale, drawing_unit, factor)}


def scale_cases():
    return [
        scale_case("Kâğıtta mm, 1:100", "mm"),
        scale_case("Kâğıtta mm, 1:500, ölçek 2", "mm", 500.0, factor=2.0),
        scale_case("Kâğıtta inç, 1:100", "inch"),
        scale_case("Kâğıtta inç, 1:1, ölçek 0.5", "inch", 1.0, factor=0.5),
        scale_case("Çizim birimi metre", "drawing"),
        scale_case("Çizim birimi milimetre", "drawing", drawing_unit="mm"),
        scale_case("Çizim birimi santimetre, ölçek 10", "drawing", drawing_unit="cm", factor=10.0),
        scale_case("Çizim birimi inç", "drawing", drawing_unit="in"),
        scale_case("Çizim birimi ayak", "drawing", drawing_unit="ft"),
    ]


def main() -> int:
    doc = {"format": "kentos.hatch-pattern-file-cases", "version": 1,
           "note": "Written by scripts/fixtures/hatch_pattern_file_cases.py from AutoCAD's PAT documentation and docs/adr/0331; do not edit by hand.",
           "maxLines": MAX_LINES, "maxDashes": MAX_DASHES, "pat": pat_cases(), "written": written_cases(),
           "scale": scale_cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.name} güncel: {len(doc['pat'])} PAT, {len(doc['written'])} yazım, {len(doc['scale'])} ölçek durumu.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


# ── cad.hatchpatterns.edit (docs/adr/0331 §1) ────────────────────────────
#
# Written to fixtures/commands/v1/cad.hatchpatterns.edit.json by `commands()`; the tree, the objects and the steps' form
# are layer_tree_command_cases.py's.

import importlib.util as _ilu


def _tree_module():
    spec = _ilu.spec_from_file_location("layer_tree_command_cases", Path(__file__).with_name("layer_tree_command_cases.py"))
    module = _ilu.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def same_name(a, b):
    return a.strip().lower() == b.strip().lower()


def patterns_problem(defs):
    """The contract's rule for a project's patterns, said so a reader can fix it; none: they hold."""
    for i, d in enumerate(defs):
        name = d["name"].strip()
        if not name:
            return "bir tarama deseninin adı boş"
        if name != d["name"]:
            return f"“{d['name']}” tarama deseninin adının başında ya da sonunda boşluk var"
        if "," in name or len(name) > MAX_NAME:
            return f"“{name}” tarama deseninin adı virgül içeremez ve en çok {MAX_NAME} harf olur"
        if any(same_name(o["name"], name) for o in defs[:i]):
            return f"“{name}” tarama deseni iki kez var"
        n = len(d["lines"])
        if n == 0 or n > MAX_LINES:
            return f"“{name}” tarama deseninin ailesi en az 1, en çok {MAX_LINES} olur, {n} var"
        for l in d["lines"]:
            if len(l.get("dashes", [])) > MAX_DASHES:
                return f"“{name}” tarama deseninin bir ailesinde en çok {MAX_DASHES} kesik olur"
            values = [l["angle"], *l["origin"], *l["offset"], *l.get("dashes", [])]
            if not all(abs(v) <= MAX_SIZE for v in values):
                return f"“{name}” tarama deseninin bir sayısı geçersiz"
            if l["offset"][1] == 0:
                return f"“{name}” tarama deseninin bir ailesinde çizgiler arası (dy) 0"
            if l.get("dashes") and all(v == 0 for v in l["dashes"]):
                return f"“{name}” tarama deseninin bir ailesinin kesiklerinin hepsi 0"
    return None


PROJECT_PATTERNS = [
    {"name": "TUGLA", "description": "Tuğla örgü", "unit": "mm",
     "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [0.0, 6.35]},
               {"angle": 90.0, "origin": [0.0, 0.0], "offset": [6.35, 6.35], "dashes": [6.35, -6.35]}]},
    {"name": "PARKE", "unit": "drawing", "source": "ofis.pat",
     "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [0.2, 0.1], "dashes": [0.3, -0.1]}]},
    {"name": "ESKI", "unit": "inch", "lines": [{"angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 0.125]}]},
]
FILE = ("*TUGLA, Tuğla (dosyadan)\n0, 0,0, 0,10\n"
        "*CIM, Çim\n60, 0,0, 2,4, 1,-3\n120, 0,0, 2,4, 1,-3\n"
        "*NOKTA\n0, 0,0, 1,1, 0,-1\n")
BAD_FILE = "*IYI\n0,0,0,0,1\n*KOTU\n0,0,0,0,x\n*SIFIR\n0,0,0,1,0\n"


def commands():
    T = _tree_module()
    node, step, completed, failed, VALID = T.node, T.step, T.completed, T.failed, T.VALID
    layers = [node("tarama", "Tarama")]
    entities = [{"kind": "hatch", "id": 1, "layerId": "tarama", "attrs": {},
                 "ring": [{"x": T.X, "y": T.Y}, {"x": T.X + 10, "y": T.Y}, {"x": T.X + 10, "y": T.Y + 8}, {"x": T.X, "y": T.Y + 8}],
                 "pattern": {"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": "TUGLA", "scale": 0.1,
                             "lines": PROJECT_PATTERNS[0]["lines"]}}]
    ids = [e["id"] for e in entities]

    def setup(patterns=PROJECT_PATTERNS, **more):
        return T.setup(layers, entities, "tarama", hatchPatterns=patterns, **more)

    def done(**kw):
        out = {"changed": False, "added": [], "replaced": [], "skipped": [], "removed": [], "problems": []}
        out.update(kw)
        out["revision"] = "$current"
        return out

    def wrote(name, input_, output, patterns, setup_doc=None):
        d = setup_doc or setup()
        return {"name": name, "setup": d, "steps": [
            step("plan", input_, completed(output), {"revision": "same"}),
            step("execute", input_, completed(output),
                 {"ids": [e["id"] for e in d["entities"]], "settings": {"hatchPatterns": patterns or None},
                  "dirty": output["changed"], "canUndo": False})]}

    def refused(name, input_, code, message, path, setup_doc=None):
        d = setup_doc or setup()
        return {"name": name, "setup": d, "steps": [step("execute", input_, failed(code, message, path), T.unchanged(d["layers"], ids))]}

    read = read_pat(FILE, "mm", "kentos.pat")
    tugla_file, cim, nokta = read["patterns"]
    out = []
    out.append(wrote("Yükle: dosyanın yeni desenleri eklenir, projede aynı adlı olan kalır",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat"},
                     done(changed=True, added=["CIM", "NOKTA"], skipped=["TUGLA"]), PROJECT_PATTERNS + [cim, nokta]))
    out.append(wrote("Yükle: replace ile aynı adlı olan yerinde değişir",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat", "replace": True},
                     done(changed=True, added=["CIM", "NOKTA"], replaced=["TUGLA"]), [tugla_file] + PROJECT_PATTERNS[1:] + [cim, nokta]))
    out.append(wrote("Yükle: yalnız adı verilenler (harf büyüklüğüne bakılmaz)",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat", "names": ["cim"]},
                     done(changed=True, added=["CIM"]), PROJECT_PATTERNS + [cim]))
    out.append(wrote("Yükle: dosyada olmayan ad söylenir",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat", "names": ["CIM", "YOK"]},
                     done(changed=True, added=["CIM"], problems=[{"line": 0, "message": "“YOK” dosyada yok"}]), PROJECT_PATTERNS + [cim]))
    inch = read_pat(FILE, "inch", "kentos.pat")["patterns"]
    out.append(wrote("Yükle: inç birimiyle; sayılar yazıldığı gibi",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat", "unit": "inch", "names": ["NOKTA"]},
                     done(changed=True, added=["NOKTA"]), PROJECT_PATTERNS + [inch[2]]))
    drawing = read_pat(FILE, "drawing", "kentos.pat")["patterns"]
    out.append(wrote("Yükle: çizim birimiyle",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat", "unit": "drawing", "names": ["CIM"]},
                     done(changed=True, added=["CIM"]), PROJECT_PATTERNS + [drawing[1]]))
    bad = read_pat(BAD_FILE, "mm", "bozuk.pat")
    out.append(wrote("Yükle: okunmayan satırlar söylenir, okunanlar alınır",
                     {"operation": "load", "pat": BAD_FILE, "source": "bozuk.pat"},
                     done(changed=True, added=["IYI"], problems=bad["problems"]), PROJECT_PATTERNS + bad["patterns"]))
    tek = read_pat("*TEK\n0,0,0,0,2\n", "mm", "dosya.pat")["patterns"][0]
    out.append(wrote("Yükle: kaynak verilmezse dosya.pat, birim verilmezse mm",
                     {"operation": "load", "pat": "*TEK\n0,0,0,0,2\n"}, done(changed=True, added=["TEK"]), PROJECT_PATTERNS + [tek]))
    out.append(wrote("Yükle: hiçbiri okunmazsa değişen yok",
                     {"operation": "load", "pat": "*KOTU\n0,0\n"},
                     done(problems=read_pat("*KOTU\n0,0\n", "mm", "dosya.pat")["problems"]), PROJECT_PATTERNS))
    out.append(wrote("Yükle: deseni olmayan projeye",
                     {"operation": "load", "pat": FILE, "source": "kentos.pat"},
                     done(changed=True, added=["TUGLA", "CIM", "NOKTA"]), [tugla_file, cim, nokta], setup_doc=setup([])))
    yeni = {"name": "ZIKZAK", "unit": "mm", "lines": [{"angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 4.0], "dashes": [2.0, -1.0, 0.0, -1.0]}]}
    out.append(wrote("Ekle: yeni desen sona", {"operation": "add", "pattern": yeni}, done(changed=True, added=["ZIKZAK"]),
                     PROJECT_PATTERNS + [yeni]))
    tugla2 = {"name": "Tugla", "unit": "mm", "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [0.0, 8.0]}]}
    out.append(wrote("Ekle: replace ile aynı adlı yerinde değişir", {"operation": "add", "pattern": tugla2, "replace": True},
                     done(changed=True, replaced=["Tugla"]), [tugla2] + PROJECT_PATTERNS[1:]))
    out.append(wrote("Sil: taramalar kendi tanımlarını taşır; kullanılan desen de silinir",
                     {"operation": "remove", "name": "tugla"}, done(changed=True, removed=["tugla"]), PROJECT_PATTERNS[1:]))
    out.append(wrote("Sil: son desen kalkınca liste yazılmaz", {"operation": "remove", "name": "ESKI"},
                     done(changed=True, removed=["ESKI"]), [], setup_doc=setup([PROJECT_PATTERNS[2]])))
    out.append(wrote("Dışa aktar: bütün desenler PAT metni olarak; çizim değişmez", {"operation": "export"},
                     done(pat=write_pat(PROJECT_PATTERNS)), PROJECT_PATTERNS))
    out.append(wrote("Dışa aktar: yalnız adı verilenler, projenin sırasıyla", {"operation": "export", "names": ["eski", "PARKE"]},
                     done(pat=write_pat(PROJECT_PATTERNS[1:])), PROJECT_PATTERNS))
    out.append({"name": "Doğrula: yazmaz", "setup": setup(),
                "steps": [step("validate", {"operation": "remove", "name": "PARKE"}, VALID, T.unchanged(layers, ids))]})
    out.append(refused("yükle: metin yok", {"operation": "load"}, "no_pat", "Yüklenecek PAT dosyasının metnini verin (pat).", "pat"))
    out.append(refused("yükle: boş metin", {"operation": "load", "pat": " \n"}, "no_pat", "Yüklenecek PAT dosyasının metnini verin (pat).", "pat"))
    out.append(refused("ekle: desen yok", {"operation": "add"}, "no_definition", "Eklenecek tarama desenini verin (pattern).", "pattern"))
    out.append(refused("ekle: aynı adlı var", {"operation": "add", "pattern": tugla2}, "invalid_hatch_pattern",
                       "“Tugla” adlı tarama deseni projede var. Değiştirmek için replace verin ya da başka bir ad seçin.", "pattern"))
    zero = {"name": "SIFIR", "unit": "mm", "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [1.0, 0.0]}]}
    out.append(refused("ekle: dy 0", {"operation": "add", "pattern": zero}, "invalid_hatch_pattern",
                       "Tarama deseni geçersiz: " + patterns_problem([zero]) + ".", "pattern"))
    empty = {"name": "BOS", "unit": "mm", "lines": []}
    out.append(refused("ekle: ailesiz", {"operation": "add", "pattern": empty}, "invalid_hatch_pattern",
                       "Tarama deseni geçersiz: " + patterns_problem([empty]) + ".", "pattern"))
    comma = {"name": "A,B", "unit": "mm", "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [0.0, 1.0]}]}
    out.append(refused("ekle: adında virgül", {"operation": "add", "pattern": comma}, "invalid_hatch_pattern",
                       "Tarama deseni geçersiz: " + patterns_problem([comma]) + ".", "pattern"))
    many = {"name": "COK", "unit": "mm", "lines": [{"angle": 0.0, "origin": [0.0, 0.0], "offset": [0.0, 1.0],
                                                     "dashes": [1.0, -1.0] * 9}]}
    out.append(refused("ekle: 18 kesik", {"operation": "add", "pattern": many}, "invalid_hatch_pattern",
                       "Tarama deseni geçersiz: " + patterns_problem([many]) + ".", "pattern"))
    out.append(refused("sil: ad yok", {"operation": "remove"}, "empty_name", "Silinecek tarama deseninin adını verin (name).", "name"))
    out.append(refused("sil: projede yok", {"operation": "remove", "name": "YOK"}, "hatch_pattern_not_found",
                       "“YOK” adlı tarama deseni projede yok.", "name"))
    out.append(refused("dışa aktar: projede olmayan ad", {"operation": "export", "names": ["YOK"]}, "hatch_pattern_not_found",
                       "“YOK” adlı tarama deseni projede yok.", "names"))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "remove", "name": "PARKE", "expectedRevision": "on iki"},
                       "invalid_revision", T.INVALID_REVISION, "expectedRevision"))
    out.append({**T.conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                                  step("execute", {"operation": "remove", "name": "PARKE"},
                                       completed(done(changed=True, removed=["PARKE"])), {"revision": "changed"}),
                                  {"operation": "remove", "name": "ESKI"}, {}), "setup": setup()})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.hatchpatterns.edit", "commandVersion": 1,
            "title": "Tarama desenleri",
            "note": "Written by scripts/fixtures/hatch_pattern_file_cases.py from AutoCAD's PAT documentation, the contract's rules and docs/adr/0331; do not edit by hand.",
            "setup": setup(), "cases": out}


def write_commands(check):
    path = ROOT / "fixtures/commands/v1/cad.hatchpatterns.edit.json"
    doc = commands()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{path.name} güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main() | write_commands("--check" in sys.argv[1:]))
