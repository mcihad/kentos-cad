"""Writes the shared cases of Yazı tiplerini raporla (docs/adr/0324 §5):

- fixtures/commands/v1/cad.fonts.report.json (`cad.fonts.report` v1)

The rule is the ADR's, written here again without KentOS code: a name ending in `.shx` is an SHX line font; one whose
family is exactly one of KentOS's seven faces is that face; recognized style suffixes do not change the family.
Arial, Helvetica and Liberation Sans are Arimo's
twins, Courier Courier Prime's; any other is missing. A style naming no file is its own face. A style drawn in another
face than the one its file names is a `font_substituted` warning, unless (ADR 1125) its SHX font is embedded: the
project's styles hold an SHX asset of the file's name (lowercase, its basename) whose bytes are a text font (a unifont,
or a shapes file with its font descriptor, shape 0), and the Big Font it names, if any, is embedded too as a bigfont;
such a style is `embedded` and no warning. The faces are those its texts, dimensions, leaders and tables are drawn in
(a text's or a table's own face, a dimension's look's, else the project's), in the contract's order.
The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/font_report_cases.py           # writes the file
    python3 scripts/fixtures/font_report_cases.py --check   # compares it with the one on disk
"""
import base64
import hashlib
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.fonts.report.json"
X, Y = 500000, 4400000
SHX = ROOT / "fixtures/shx/v1"
SHAPES = (b"AutoCAD-86 shapes 1.0\r\n\x1a", b"AutoCAD-86 shapes 1.1\r\n\x1a")
UNIFONT = b"AutoCAD-86 unifont 1.0\r\n\x1a"
BIGFONT = b"AutoCAD-86 bigfont 1.0\r\n\x1a"
# The shapes 1.0 file of one shape (FENCE, no font descriptor) the KCAD samples embed: not a text font.
FENCE = base64.b64decode(json.loads((ROOT / "fixtures/kcad/v2/shape-resources.json").read_text("utf-8"))["styles"]["items"][0]["data"].split(",", 1)[1])

FACES = ["barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono"]
LABELS = {"barlow": "Barlow", "arimo": "Arimo", "overpass": "Overpass", "quicksand": "Quicksand",
          "architects-daughter": "Architects Daughter", "courier-prime": "Courier Prime", "plex-mono": "IBM Plex Mono"}
OWN = [("barlow", "barlow"), ("arimo", "arimo"), ("overpass", "overpass"), ("quicksand", "quicksand"),
       ("architectsdaughter", "architects-daughter"), ("courierprime", "courier-prime"), ("ibmplexmono", "plex-mono"),
       ("plexmono", "plex-mono")]
TWINS = [("arial", "arimo"), ("helvetica", "arimo"), ("liberationsans", "arimo"), ("couriernew", "courier-prime"), ("courier", "courier-prime"), ("cour", "courier-prime")]


def resolve(name):
    """The ADR's rule: (face or None, how)."""
    lower = name.strip().lower()
    leaf = lower.replace("\\", "/").rsplit("/", 1)[-1]
    if leaf.endswith(".shx"):
        return None, "shx"
    for extension in (".ttf", ".otf", ".ttc", ".woff2", ".woff"):
        if leaf.endswith(extension):
            leaf = leaf[:-len(extension)]
            break
    letters = "".join(c for c in leaf if c.isascii() and c.isalnum())
    styles = {"", "regular", "normal", "roman", "medium", "semibold", "bold", "italic", "oblique",
              "bolditalic", "boldoblique", "400", "500", "600", "700", "400italic", "bd", "bi", "i", "b", "z"}
    for stem, face in OWN:
        if letters.startswith(stem) and letters[len(stem):] in styles:
            return face, "own"
    for stem, face in TWINS:
        if letters.startswith(stem) and letters[len(stem):] in styles:
            return face, "equivalent"
    return None, "missing"


def style(i, name, font, file=None, big=None):
    s = {"id": f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d05{i:02d}", "name": name, "font": font}
    if file is not None:
        s["fontFile"] = file
    if big is not None:
        s["bigFontFile"] = big
    return s


def text(i, words, style_id=None, font=None):
    t = {"kind": "text", "id": i, "layerId": "yazi", "attrs": {}, "p": {"x": X + 10 * i, "y": Y}, "text": words,
         "height": 2.5, "rotation": 0}
    if style_id is not None:
        t["textStyle"] = style_id
    if font is not None:
        t["font"] = font
    return t


def dimension(i, font=None):
    d = {"kind": "dimension", "id": i, "layerId": "yazi", "attrs": {}, "a": {"x": X, "y": Y - 10}, "b": {"x": X + 6, "y": Y - 10},
         "offset": -1, "height": 2.5}
    if font is not None:
        d["font"] = font
    return d


def leader(i):
    return {"kind": "leader", "id": i, "layerId": "yazi", "attrs": {}, "pts": [{"x": X, "y": Y - 20}, {"x": X + 4, "y": Y - 16}],
            "text": "Not", "height": 2.5, "rotation": 0}


def setup(styles=(), entities=(), font="barlow", shx=()):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Yazı tipleri",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500,
                     "workspace": "cad", "drawingFont": font, "textStyles": list(styles)},
        "origin": {"x": X, "y": Y},
        "layers": [{"id": "yazi", "name": "Yazı", "type": "layer", "visible": True, "locked": False, "expanded": True,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "yazi",
        "entities": list(entities),
        "styles": {"items": [resource(n, shx_bytes(n.lower())) for n in shx], "categories": []},
        "blocks": [],
    }


def resource(name, data):
    """A project's SHX asset (ADR 0930): its id the lowercase basename."""
    return {"id": "shx:" + name.lower(), "kind": "asset", "name": name, "path": ["CAD", "SHX"], "format": "shx",
            "file": name, "data": "data:application/x-autocad-shx;base64," + base64.b64encode(data).decode(),
            "sha256": hashlib.sha256(data).hexdigest()}


def shx_bytes(name):
    return FENCE if name == "fence.shx" else (SHX / name).read_bytes()


def text_font(data):
    """A unifont, or a shapes file whose shape 0 is a font descriptor (its index's first entry)."""
    if data.startswith(UNIFONT):
        return True
    for head in SHAPES:
        if data.startswith(head):
            at = len(head)
            _, _, count = struct.unpack_from("<HHH", data, at)
            numbers = [struct.unpack_from("<HH", data, at + 6 + 4 * i)[0] for i in range(count)]
            return 0 in numbers
    return False


def embedded(doc, s):
    """The style's SHX font (and Big Font) embedded as their forms (ADR 1125)."""
    items = {i["id"]: i for i in doc.get("styles", {}).get("items", []) if i.get("format") == "shx"}

    def data(file):
        leaf = file.replace("\\", "/").rsplit("/", 1)[-1]
        item = items.get("shx:" + leaf.lower())
        return None if item is None else base64.b64decode(item["data"].split(",", 1)[1])

    font = data(s["fontFile"])
    if font is None or not text_font(font):
        return False
    if "bigFontFile" in s:
        big = data(s["bigFontFile"])
        return big is not None and big.startswith(BIGFONT)
    return True


def report(doc, names=()):
    """The report and its warnings, by the rule (KentOS code not used)."""
    settings = doc["settings"]
    project = settings.get("drawingFont", "barlow")
    used, faces = {}, set()
    for e in doc["entities"]:
        if e["kind"] == "text":
            if "textStyle" in e:
                used[e["textStyle"]] = used.get(e["textStyle"], 0) + 1
            faces.add(e.get("font", project))
        elif e["kind"] == "dimension":
            faces.add(e.get("font", project))
        elif e["kind"] == "leader":
            faces.add(project)
        elif e["kind"] == "table":
            faces.add(e.get("font", project))
    styles, warnings = [], []
    for i, s in enumerate(settings.get("textStyles", [])):
        if "fontFile" in s:
            named, how = resolve(s["fontFile"])
        else:
            named, how = s["font"], "own"
        shx = how == "shx" and embedded(doc, s)
        if named != s["font"] and not shx:
            what = {"shx": "AutoCAD'in SHX çizgi yazı tipi; projeye eklenmemiş ya da okunamıyor", "missing": "KentOS'ta yok"}.get(how, "stil başka bir yazı tipine çevrilmiş")
            warnings.append({"code": "font_substituted",
                             "message": f"“{s['name']}” stili {s.get('fontFile', '')} yazı tipini adlandırıyor ({what}); yazıları {LABELS[s['font']]} ile çizilir.",
                             "path": f"styles[{i}]"})
        row = {"style": s["id"], "name": s["name"]}
        if "fontFile" in s:
            row["file"] = s["fontFile"]
        row.update({"face": s["font"], "how": how})
        if shx:
            row["embedded"] = True
        row["texts"] = used.get(s["id"], 0)
        styles.append(row)
    named = []
    for n in names:
        face, how = resolve(n)
        row = {"name": n}
        if face is not None:
            row["face"] = face
        row["how"] = how
        named.append(row)
    out = {"project": project, "styles": styles, "names": named, "faces": [f for f in FACES if f in faces], "revision": "$current"}
    return out, warnings


def step(op, input_, result):
    return {"op": op, "input": input_, "result": result,
            "expect": {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}}


def case(name, doc, input_=None, op="execute"):
    input_ = input_ or {}
    out, warnings = report(doc, input_.get("names", ()))
    c = {"name": name}
    if doc is not BASE:
        c["setup"] = doc
    c["steps"] = [step(op, input_, {"status": "completed", "output": out, "warnings": warnings})]
    return c


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, {"status": "failed", "error": {"code": code, "message": message, "path": path}})]}


S_OWN = style(1, "Ada no", "arimo")
S_ARIAL = style(2, "Yol adı", "arimo", "arial.ttf")
S_SHX = style(3, "Pafta", "barlow", "romans.shx")
S_TIMES = style(4, "Başlık", "overpass", "times.ttf")
S_BARLOW = style(5, "Not", "barlow", "Barlow-Bold.ttf")
S_TURNED = style(6, "Kot", "overpass", "arialbd.ttf")
S_BIG = style(7, "Çince", "quicksand", "gbcbig.shx")
S_COUR = style(8, "Liste", "courier-prime", "couri.ttf")
U_STYLE = style(20, "SHX yazı", "barlow", "kentos-uni.shx")
UB_STYLE = style(21, "SHX Big", "barlow", "kentos-uni.shx", "kentos-big.shx")
BASE = setup(
    [S_OWN, S_ARIAL, S_SHX, S_TIMES, S_BARLOW, S_TURNED, S_BIG, S_COUR],
    [text(1, "101", S_OWN["id"], "arimo"), text(2, "102", S_OWN["id"], "arimo"), text(3, "Atatürk Cd.", S_ARIAL["id"], "arimo"),
     text(4, "G-23-b", S_SHX["id"], "barlow"), text(5, "Plan notu"), dimension(6, "overpass"), dimension(7), leader(8)],
)
NAMES = ["arial.ttf", "Helvetica Neue", "Liberation Sans", "cour.ttf", "Courier New", "Courier Prime", "romans.shx", "GBCBIG.SHX",
         "times.ttf", "Calibri", "IBM Plex Mono", "plexmono-regular.ttf", "Quicksand-Bold.ttf", "ArchitectsDaughter",
         "OVERPASS", "Barlow Condensed", "arimo.ttf", "Verdana"]
LONG = "a" * 81
MESSAGE_EMPTY = "{}. yazı tipi adı boş. Bir dosya ya da aile adı yazın (arial.ttf, romans.shx)."
MESSAGE_LONG = "{}. yazı tipi adı 80 karakterden uzun. Kısaltın."

CASES = [
    case("Boş çizim: projenin yazı tipi, stil ve yazı yok", setup()),
    case("Stiller ve yazılar: dosyasız stil kendi, Arial ikizi, SHX ve Times yerine stilin yazı tipi, uyarılarla", BASE),
    case("Dosyasız stil kendi yazı tipidir, uyarısız", setup([S_OWN], [text(1, "101", S_OWN["id"], "arimo")])),
    case("arial.ttf Arimo'yla çizilir: eşdeğer, uyarısız", setup([S_ARIAL])),
    case("SHX çizgi yazı tipi okunmaz: stilin yazı tipiyle, uyarıyla", setup([S_SHX])),
    case("Big font da SHX'tir", setup([S_BIG])),
    case("Times KentOS'ta yok: stilin yazı tipiyle, uyarıyla", setup([S_TIMES])),
    case("Barlow-Bold.ttf Barlow'un kendisidir", setup([S_BARLOW])),
    case("Arial adlandırıp Overpass'a çevrilmiş stil uyarılır", setup([S_TURNED])),
    case("couri.ttf Courier Prime'ın ikizidir", setup([S_COUR])),
    case("Stili kullanan yazılar sayılır", setup([S_OWN, S_SHX], [text(1, "a", S_OWN["id"], "arimo"), text(2, "b", S_SHX["id"], "barlow"),
                                                                  text(3, "c", S_SHX["id"], "barlow"), text(4, "d")])),
    case("Yazı tipleri: yazıların, ölçülerin, kılavuzların; sözleşmenin sırasıyla", setup([], [text(1, "a", font="plex-mono"), dimension(2, "quicksand"),
                                                                                           leader(3), text(4, "b", font="arimo")])),
    case("Projenin yazı tipi Overpass: kendi yazı tipi olmayanlar onunla", setup([], [text(1, "a"), dimension(2), leader(3)], font="overpass")),
    case("Adlar aynı kuralla çözülür", setup(), {"names": NAMES}),
    case("Benzer isim aynı metrik değildir: dar, siyah, condensed ve başka aileler", setup(),
         {"names": ["Arial Narrow", "Arial Black", "Helvetica Neue", "Barlow Condensed", "Overpass Mono",
                    "Arimo Display", "Courtney.ttf", "Courier New Condensed", "Arial-custom.ttf"]}),
    case("Dosya yolu yalnız aileyi seçer: kaynağın tam adı korunur", setup(),
         {"names": [r"C:\Windows\Fonts\arialbd.ttf", "/fonts/Arial-BoldItalic.otf", "/fonts/Barlow-400-italic.ttf"]}),
    case("Dar fontu adlandıran eski stilin yüzü değiştirilmez, eksik kaynak açıkça uyarılır",
         setup([style(9, "Dar yazı", "arimo", "Arial Narrow")], [text(1, "Ölçü", style(9, "", "")['id'], "arimo")])),
    case("Bir ad: romans.shx", setup(), {"names": ["romans.shx"]}),
    case("Büyük harf ve boşluk önemsiz: IBM Plex Mono", setup(), {"names": ["  ibm plex mono  "]}),
    case("Bilinmeyen ad eksik sayılır, yüzü verilmez", setup(), {"names": ["Segoe UI"]}),
    case("Plan execute ile aynıdır", BASE, {"names": ["arial.ttf"]}, op="plan"),
    case("Stiller ve adlar birlikte", BASE, {"names": ["times.ttf", "Arial"]}),
    case("Yalnız kılavuz: projenin yazı tipi", setup([], [leader(1)])),
    refused("Boş ad reddedilir", {"names": [""]}, "empty_name", MESSAGE_EMPTY.format(1), "names[0]"),
    refused("Yalnız boşluklu ad reddedilir", {"names": ["   "]}, "empty_name", MESSAGE_EMPTY.format(1), "names[0]"),
    refused("İkinci ad boş: yeri söylenir", {"names": ["arial.ttf", ""]}, "empty_name", MESSAGE_EMPTY.format(2), "names[1]"),
    refused("80 karakterden uzun ad reddedilir", {"names": [LONG]}, "invalid_name", MESSAGE_LONG.format(1), "names[0]"),
    case("Tam 80 karakter kabul edilir", setup(), {"names": ["a" * 80]}),
] + [
    # ADR 1125: a style's embedded SHX font draws its texts; it is no substitute.
    case("Gömülü SHX unifont: stil gömülü, uyarısız", setup([U_STYLE], shx=["kentos-uni.shx"])),
    case("Gömülü unifont ve Big Font: ikisi de gömülü, uyarısız", setup([UB_STYLE], shx=["kentos-uni.shx", "kentos-big.shx"])),
    case("Big Font gömülü değil: stil gömülü sayılmaz, uyarı", setup([UB_STYLE], shx=["kentos-uni.shx"])),
    case("Yazı fontu gömülü değil, yalnız Big Font: uyarı", setup([UB_STYLE], shx=["kentos-big.shx"])),
    case("SHX hiç gömülü değil: eski uyarı, yeni açıklamayla", setup([U_STYLE])),
    case("Dosya adının büyük/küçük harfi önemsiz", setup([style(10, "Büyük", "barlow", "KENTOS-UNI.SHX")], shx=["kentos-uni.shx"])),
    case("Kaynak yolu yalnız dosya adıyla eşlenir", setup([style(11, "Yollu", "barlow", r"C:\CAD\Fonts\kentos-uni.shx")], shx=["kentos-uni.shx"])),
    case("Şekil dosyası (yazı fontu tanımı yok) yazı fontu sayılmaz", setup([style(12, "Çit", "barlow", "fence.shx")], shx=["fence.shx"])),
    case("Big Font yazı fontu olarak adlandırılırsa gömülü sayılmaz", setup([style(13, "Ters", "barlow", "kentos-big.shx")], shx=["kentos-big.shx"])),
    case("Unifont Big Font olarak adlandırılırsa gömülü sayılmaz", setup([style(14, "İki uni", "barlow", "kentos-uni.shx", "kentos-uni.shx")], shx=["kentos-uni.shx"])),
    case("Bir stil gömülü, öbürü değil", setup([U_STYLE, S_SHX], shx=["kentos-uni.shx"])),
    case("Gömülü stilin yazıları sayılır", setup([U_STYLE], [text(1, "SU", U_STYLE["id"]), text(2, "US", U_STYLE["id"]), text(3, "Not")], shx=["kentos-uni.shx"])),
    case("Gömülü stilin yüzü değişmez: Quicksand", setup([style(15, "Q", "quicksand", "kentos-uni.shx")], shx=["kentos-uni.shx"])),
    case("TrueType stilin Big Font'u SHX değildir: gömülü sayılmaz", setup([style(16, "TTF", "arimo", "arial.ttf", "kentos-big.shx")], shx=["kentos-big.shx"])),
    case("Ad sorusu gömülülüğe bakmaz: kentos-uni.shx SHX'tir", setup([U_STYLE], shx=["kentos-uni.shx"]), {"names": ["kentos-uni.shx"]}),
    case("Plan execute ile aynıdır (gömülü)", setup([UB_STYLE], shx=["kentos-uni.shx", "kentos-big.shx"]), op="plan"),
    case("Kullanılmayan gömülü kaynak stilsiz rapora girmez", setup([S_OWN], shx=["kentos-uni.shx"])),
    case("Gömülü kaynak başka adda: stilin dosyası bulunmaz", setup([style(17, "Başka", "barlow", "romans.shx")], shx=["kentos-uni.shx"])),
    case("İki stil aynı gömülü fontu kullanır", setup([U_STYLE, style(18, "İkinci", "overpass", "kentos-uni.shx")], shx=["kentos-uni.shx"])),
    case("Gömülü stille ana çizim: yalnız SHX stilinin uyarısı kalkar", setup([S_OWN, S_ARIAL, U_STYLE, S_TIMES], shx=["kentos-uni.shx"])),
    case("Big Font yerine şekil dosyası: gömülü sayılmaz", setup([style(19, "Şekil BF", "barlow", "kentos-uni.shx", "fence.shx")], shx=["kentos-uni.shx", "fence.shx"])),
]


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.fonts.report",
        "commandVersion": 1,
        "title": "Yazı tiplerini raporla",
        "note": "Written by scripts/fixtures/font_report_cases.py from docs/adr/0324 §5; do not edit by hand.",
        "setup": BASE,
        "cases": CASES,
    }


def main():
    text_ = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text("utf-8") != text_:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/font_report_cases.py")
        print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum güncel")
        return
    OUT.write_text(text_, "utf-8")
    print(f"written: {OUT.relative_to(ROOT)} ({len(CASES)} durum)")


if __name__ == "__main__":
    main()
