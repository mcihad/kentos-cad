"""The shared cases of a multi-line text (docs/adr/0182 §2): where its lines
break and stand, and its letter formats' runs as an editor changes them.

    python3 scripts/fixtures/paragraph_cases.py           # writes the file
    python3 scripts/fixtures/paragraph_cases.py --check   # writes nothing; compares

Writes fixtures/text/v1/paragraph.json. The rules are written here from the
ADR on their own, not from an implementation's output; the geometry core
(crates/shared/geometry-core, `text::paragraph`, natively and through WASM:
`textLayout`, `textRunsToggle`, `textRunsRetext`) is held to them. The
letters' advances are the drawing typefaces' measured widths, read as data
from the table the font recorder writes
(crates/shared/geometry-core/src/text/metrics.rs, measured in Chrome by
apps/web/scripts/fixtures/record-font-metrics.mjs).

- A letter's advance, thousandths of an em: its face's table (the bold table
  for a bold letter) when the table has the code point and a width over 0;
  otherwise the average of the face's “a” to “z”, the sum divided by 26 and
  rounded down. A raised or lowered letter takes 0.6 of it.
- A stretch of letters is as wide as the sum of their advances, left to
  right, over 1000, times the height, times the width factor.
- The text breaks into paragraphs at \\n. Without a box each paragraph is a
  line. With a box each paragraph wraps word by word (a word: letters that
  are not spaces): from the paragraph's first letter (its leading spaces
  stay), the words are taken one by one; when a word would carry the line
  (its letters from the line's first up to the word's last) past the box and
  the line already has a word, the line ends after its last word and the
  next starts at the word, the spaces between them dropped. A word wider than
  the box stays whole on its own line.
- A line ends at its last letter that is not a space.
- The box is the box width, or the widest line (0 for none). The baselines
  are 5/3 of the height times the line spacing apart (the pitch); line i's
  baseline is i pitches under the first; each line stands at its alignment's
  share along (left 0, centre ½, right 1) of what the box has left over it.
- Where the point stands from the origin (the first baseline's left end of the
  box): along, the share times the box; up, for a vertical share u of a one-
  line text (baseline 0, bottom −0.2, middle ½, top 1) and n lines below =
  (n − 1) pitches: u·h − below for the bottom, u·h − below/2 for the middle,
  u·h for the top and the baseline.
- Runs: every letter has a format (bold, italic, underline, raised or lowered,
  a colour); runs are the stretches of one format, formatless letters left
  out. Toggling a format over letters start..end (clipped to the text) takes it
  off them all when they all have it, else puts it on them all; raised and
  lowered exclude each other; a colour is set over them (or cleared, null).
  After an editor's text `before` became `after`: the letters both begin with
  and end with (the end taken from what is left) keep their formats, the ones
  put in between take the format of the letter before them (of the one after
  them at the start; none in an empty text).
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/text/v1"
METRICS = ROOT / "crates/shared/geometry-core/src/text/metrics.rs"

# ── The measured advances (data) ────────────────────────────────────────


def read_metrics():
    src = METRICS.read_text("utf-8")
    first = int(re.search(r"pub const FIRST: u32 = (\d+);", src).group(1))
    last = int(re.search(r"pub const LAST: u32 = (\d+);", src).group(1))
    fonts = json.loads("[" + re.search(r"pub const FONTS: \[&str; \d+\] = \[(.*?)\];", src).group(1) + "]")

    def table(name):
        body = src[src.index(f"pub const {name}:"):]
        body = body[body.index("= [") + 3:]
        rows = []
        depth = 0
        cur = []
        for tok in re.finditer(r"\[|\]|\d+|//[^\n]*", body):
            s = tok.group(0)
            if s.startswith("//"):
                continue
            if s == "[":
                depth += 1
                cur = []
            elif s == "]":
                if depth == 0:
                    break
                depth -= 1
                rows.append(cur)
            else:
                cur.append(int(s))
        return rows

    regular = table("ADVANCES")
    bold = table("BOLD")
    assert len(regular) == len(fonts) == len(bold)
    assert all(len(r) == last - first + 1 for r in regular + bold)
    return first, last, fonts, regular, bold


FIRST, LAST, FONTS, REGULAR, BOLD = read_metrics()


def advance(font, c, bold):
    row = (BOLD if bold else REGULAR)[FONTS.index(font)]
    code = ord(c)
    if FIRST <= code <= LAST and row[code - FIRST] > 0:
        return row[code - FIRST]
    return sum(row[ord("a") - FIRST:ord("z") - FIRST + 1]) // 26


# ── Formats ─────────────────────────────────────────────────────────────

PLAIN = {"bold": False, "italic": False, "underline": False, "script": None, "color": None}


def formatted(f):
    return f["bold"] or f["italic"] or f["underline"] or f["script"] is not None or f["color"] is not None


def spread(runs, n):
    out = [dict(PLAIN) for _ in range(n)]
    for r in runs:
        f = {k: r.get(k, PLAIN[k]) for k in PLAIN}
        for i in range(r["start"], min(r["end"], n)):
            out[i] = dict(f)
    return out


def gather(formats):
    out = []
    for i, f in enumerate(formats):
        if not formatted(f):
            continue
        if out and out[-1]["end"] == i and all(out[-1]["f"][k] == f[k] for k in PLAIN):
            out[-1]["end"] = i + 1
        else:
            out.append({"start": i, "end": i + 1, "f": dict(f)})
    runs = []
    for r in out:
        run = {"start": r["start"], "end": r["end"]}
        for k in ("bold", "italic", "underline"):
            if r["f"][k]:
                run[k] = True
        if r["f"]["script"] is not None:
            run["script"] = r["f"]["script"]
        if r["f"]["color"] is not None:
            run["color"] = r["f"]["color"]
        runs.append(run)
    return runs


def toggle(runs, n, start, end, what):
    formats = spread(runs, n)
    start, end = min(start, n), min(end, n)
    if start >= end:
        return gather(formats)
    if isinstance(what, dict):
        for f in formats[start:end]:
            f["color"] = what["color"]
        return gather(formats)

    def has(f):
        if what in ("bold", "italic", "underline"):
            return f[what]
        return f["script"] == what

    on = not all(has(f) for f in formats[start:end])
    for f in formats[start:end]:
        if what in ("bold", "italic", "underline"):
            f[what] = on
        else:
            f["script"] = what if on else None
    return gather(formats)


def retext(runs, before, after):
    old, new = list(before), list(after)
    formats = spread(runs, len(old))
    head = 0
    while head < min(len(old), len(new)) and old[head] == new[head]:
        head += 1
    room = min(len(old), len(new)) - head
    tail = 0
    while tail < room and old[len(old) - 1 - tail] == new[len(new) - 1 - tail]:
        tail += 1
    inserted = len(new) - head - tail
    if head > 0:
        inherit = formats[head - 1]
    elif len(old) - tail < len(old):
        inherit = formats[len(old) - tail]
    else:
        inherit = dict(PLAIN)
    out = formats[:head] + [dict(inherit) for _ in range(inserted)] + formats[len(old) - tail:]
    return gather(out)


# ── Lines ───────────────────────────────────────────────────────────────

SHARES = {
    None: (0.0, 0.0),
    "baselineCenter": (0.5, 0.0),
    "baselineRight": (1.0, 0.0),
    "bottomLeft": (0.0, -0.2),
    "bottomCenter": (0.5, -0.2),
    "bottomRight": (1.0, -0.2),
    "middleLeft": (0.0, 0.5),
    "middleCenter": (0.5, 0.5),
    "middleRight": (1.0, 0.5),
    "topLeft": (0.0, 1.0),
    "topCenter": (0.5, 1.0),
    "topRight": (1.0, 1.0),
}


def lay_out(t):
    letters = list(t["text"])
    formats = spread(t.get("runs", []), len(letters))
    font = t.get("font", "barlow")
    h = t["height"]
    wf = t.get("widthFactor", 1.0)
    adv = []
    for c, f in zip(letters, formats):
        a = float(advance(font, c, f["bold"]))
        adv.append(a * 0.6 if f["script"] is not None else a)

    def width(a, b):
        s = 0.0
        for x in adv[a:b]:
            s += x
        return s / 1000.0 * h * wf

    lines = []
    start = 0
    for i, c in enumerate(letters + ["\n"]):
        if c != "\n":
            continue
        if t.get("boxWidth") is None:
            lines.append([start, i])
        else:
            wrap(letters, start, i, t["boxWidth"], width, lines)
        start = i + 1
    for line in lines:
        while line[1] > line[0] and letters[line[1] - 1] == " ":
            line[1] -= 1
    widths = [width(a, b) for a, b in lines]
    box = t["boxWidth"] if t.get("boxWidth") is not None else max([0.0] + widths)
    pitch = 5.0 / 3.0 * h * t.get("lineSpacing", 1.0)
    along, up = SHARES[t.get("align")]
    out = [{"start": a, "end": b, "width": w, "x": along * (box - w), "y": i * pitch} for i, ((a, b), w) in enumerate(zip(lines, widths))]
    below = (len(lines) - 1) * pitch
    if up < 0:
        rise = up * h - below
    elif 0 < up < 1:
        rise = up * h - below / 2.0
    else:
        rise = up * h
    return {"lines": out, "width": box, "pitch": pitch, "shares": [along * box, rise]}


def wrap(letters, frm, to, box, width, out):
    start, filled, i = frm, None, frm
    while True:
        while i < to and letters[i] == " ":
            i += 1
        if i >= to:
            break
        word = i
        while i < to and letters[i] != " ":
            i += 1
        if filled is not None and width(start, i) > box:
            out.append([start, filled])
            start = word
        filled = i
    out.append([start, to])


# ── Cases ───────────────────────────────────────────────────────────────


def text(s, **kw):
    t = {"p": {"x": 0, "y": 0}, "text": s, "height": 2.5, "rotation": 0}
    t.update(kw)
    return t


LAYOUT = [
    ("iki satır, kutusuz: her paragraf bir satır", text("Ada 101\nParsel 7")),
    ("boş satır yerini tutar", text("Üst\n\nAlt")),
    ("kutuya sözcük sözcük sarılır", text("Parselin alanı tapuda yazılandan büyüktür", boxWidth=20.0)),
    ("kutudan geniş sözcük kendi satırında, bölünmez", text("Kısa çoook-uzun-bir-sözcük-kutuya-sığmaz son", boxWidth=8.0)),
    ("baştaki boşluklar kalır, satır sonundaki boşluklar yer tutmaz", text("  girinti   ve   boşluklar   \nikinci  ", boxWidth=14.0)),
    ("ortalı: her satır kutunun ortasında", text("Bir\nİkinci satır\nÜç", align="topCenter")),
    ("sağa dayalı, kutulu", text("Sağa dayalı çok satırlı bir not", boxWidth=18.0, align="topRight")),
    ("orta hiza: ilk ve son satırın ortalarının ortası", text("a\nb\nc", align="middleLeft")),
    ("alt hiza: son satırın altı", text("a\nb\nc", align="bottomRight")),
    ("taban hizası: ilk satırın tabanı", text("a\nb", align="baselineCenter")),
    ("satır aralığı 0,25 ve 4", text("Sık\naralık", lineSpacing=0.25)),
    ("satır aralığı 4", text("Seyrek\naralık", lineSpacing=4.0, align="topLeft")),
    (
        "kalın harfler kalın tablodan, üst ve alt simge 0,6",
        text(
            "Alan 450 m2 H2O kalın",
            runs=[{"start": 10, "end": 11, "script": "super"}, {"start": 13, "end": 14, "script": "sub"}, {"start": 16, "end": 21, "bold": True}],
            boxWidth=60.0,
        ),
    ),
    ("kalın sözcük sarmayı değiştirir", text("iiii iiii iiii iiii", runs=[{"start": 0, "end": 19, "bold": True}], boxWidth=6.0)),
    ("genişlik çarpanı ve yükseklik", text("Dar yazı\niki", height=1.5, widthFactor=0.7, boxWidth=4.0)),
    ("tablo dışı harf a–z'nin ortalaması", text("€ 🏠 ∑\nΩ", font="arimo")),
    ("eş aralıklı yazı tipi", text("plex mono\nkutu", font="plex-mono", boxWidth=10.0, align="middleCenter")),
    ("tek satır, kutusuz: bugünkü yazının genişliği", text("Parsel 101", runs=[{"start": 0, "end": 6, "italic": True}])),
    ("yalnız boşluklu paragraf boş satırdır", text("üst\n   \nalt", boxWidth=10.0)),
    ("Türkçe harfler ve uzun paragraf", text("Çağdaş şehir planlaması ölçüm, ifraz ve tevhit işlemlerini kapsar.", boxWidth=25.0, align="bottomLeft", font="overpass")),
]

TOGGLE = [
    ("kalın açılır", [], 10, 2, 5, "bold"),
    ("hepsi kalınsa kalın kalkar", [{"start": 0, "end": 10, "bold": True}], 10, 2, 5, "bold"),
    ("bir kısmı kalınsa hepsi kalın olur ve birleşir", [{"start": 0, "end": 3, "bold": True}], 10, 2, 6, "bold"),
    ("üst simge alt simgeyi kaldırır", [{"start": 2, "end": 6, "script": "sub"}], 10, 2, 6, "super"),
    ("renk verilir", [{"start": 0, "end": 4, "italic": True}], 10, 2, 8, {"color": "#E5484D"}),
    ("renk kaldırılır, biçimsiz kalan yazılmaz", [{"start": 2, "end": 8, "color": "accent"}], 10, 0, 10, {"color": None}),
    ("boş aralık dilimleri olduğu gibi bırakır", [{"start": 1, "end": 3, "underline": True}], 10, 4, 4, "bold"),
    ("aralık yazıdan taşarsa kırpılır", [], 5, 3, 99, "underline"),
    ("eğik üstte eğik altta: biçimler birleşir", [{"start": 0, "end": 5, "bold": True}], 10, 3, 8, "italic"),
]

RETEXT = [
    ("kalın dilimin sonuna yazılan kalın olur", [{"start": 2, "end": 5, "bold": True}], "abcdefghij", "abcdeXfghij"),
    ("dilimden harf silinince kısalır", [{"start": 2, "end": 5, "bold": True}], "abcdefghij", "abdefghij"),
    ("başa yazılan sonraki harfin biçimini alır", [{"start": 0, "end": 3, "italic": True}], "abcdef", "XYabcdef"),
    ("yerine yazılan önceki harfin biçimini alır", [{"start": 0, "end": 2, "bold": True}, {"start": 4, "end": 6, "underline": True}], "abcdef", "abQQQef"),
    ("hepsi silinip yazılınca biçimsiz", [{"start": 0, "end": 3, "bold": True}], "abc", "xyz"),
    ("satır sonu eklenir", [{"start": 0, "end": 4, "color": "#00FF00"}], "abcd", "ab\ncd"),
    ("aynı harfler: sonu baştan sayılmaz", [{"start": 1, "end": 2, "bold": True}], "aaa", "aaaa"),
    ("boş yazıya yazılan biçimsiz", [], "", "yeni"),
]


def build():
    layout = [{"name": n, "text": t, "want": lay_out(t)} for n, t in LAYOUT]
    toggles = [
        {"name": n, "runs": runs, "len": length, "start": s, "end": e, "toggle": what, "want": toggle(runs, length, s, e, what)}
        for n, runs, length, s, e, what in TOGGLE
    ]
    retexts = [{"name": n, "runs": runs, "before": b, "after": a, "want": retext(runs, b, a)} for n, runs, b, a in RETEXT]
    return {
        "paragraph.json": {
            "format": "kentos.text-cases",
            "version": 1,
            "generatedBy": "scripts/fixtures/paragraph_cases.py",
            "title": "Çok satırlı yazı: satırlar, kutu ve biçim dilimleri (ADR 0182 §2)",
            "note": "Satırlar \\n'de biter; kutu (boxWidth) varsa sözcük sözcük sarılır; satır son boşluk olmayan harfinde biter. Harfin genişliği yazı tipinin ölçülmüş ilerlemesi (kalın harf kalın tablodan; tablo dışı harf a–z'nin ortalaması), üst ve alt simge 0,6; toplam / 1000 × yükseklik × genişlik çarpanı. Satır aralığı 5/3 × yükseklik × lineSpacing; satır kutunun kalanının hiza payında. shares: noktanın ilk tabanın kutu solundan uzaklığı (boyuna, yukarı). toggle ve retext: düzenleyicinin dilimleri.",
            "layout": layout,
            "toggle": toggles,
            "retext": retexts,
        }
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=2) + "\n"


def main():
    files = build()
    if "--check" in sys.argv[1:]:
        bad = [name for name, v in files.items() if not (DIR / name).exists() or (DIR / name).read_text("utf-8") != text_of(v)]
        for name in bad:
            print(f"{name}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print(f"paragraph cases match: {', '.join(files)}")
        return 0
    DIR.mkdir(parents=True, exist_ok=True)
    for name, v in files.items():
        (DIR / name).write_text(text_of(v), encoding="utf-8")
    print(f"written: {', '.join(files)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
