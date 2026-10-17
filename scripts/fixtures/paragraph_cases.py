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
import math
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
    """A text's lines, box and point (docs/adr/0182 §2), with its layout (docs/adr/0324 §2): indents and tab stops in
    its height, a hanging first line's left indent a stop too, a stacked fraction as wide as its wider part (`#`: both
    parts and a slash side by side), columns of `columns.height` metres. Each line is a piece between tabs."""
    letters = list(t["text"])
    runs = t.get("runs", [])
    formats = spread(runs, len(letters))
    font = t.get("font", "barlow")
    h = t["height"]
    wf = t.get("widthFactor", 1.0)
    lay = t.get("layout", {})
    adv = []
    for ch, f in zip(letters, formats):
        a = float(advance(font, ch, f["bold"]))
        adv.append(a * 0.6 if f["script"] is not None else a)

    def scale(total):
        return total / 1000.0 * h * wf

    def units(heights):
        return heights * 1000.0 / wf

    def summed(a, b):
        total = 0.0
        for x in adv[a:b]:
            total += x
        return total

    stacks = []
    for r in runs:
        if r.get("script") != "stack":
            continue
        start, end = r["start"], min(r["end"], len(letters))
        marks = [k for k in range(start, end) if letters[k] in "/#^"]
        if not marks:
            continue
        q = marks[0]
        num, den = summed(start, q), summed(q + 1, end)
        if letters[q] == "#":
            w = num + float(advance(font, "/", t.get("bold", False) or r.get("bold", False))) * 0.6 + den
        else:
            w = max(num, den)
        stacks.append((start, end, q, w))
    stops = [units(x) for x in lay.get("tabs", [])]
    every = units(4.0)
    left = units(lay.get("leftIndent", 0.0))
    first = units(lay.get("leftIndent", 0.0) + lay.get("indent", 0.0))

    def stop(x):
        later = [s for s in stops if s > x]
        if later:
            nxt = later[0]
        else:
            last = stops[-1] if stops else 0.0
            nxt = last + (math.floor((x - last) / every) + 1.0) * every
        return left if left > x and left < nxt else nxt

    def walk(frm, to, x):
        i = frm
        while i < to:
            if letters[i] == "\t":
                x = stop(x)
                i += 1
                continue
            hit = [s for s in stacks if s[0] == i and s[1] <= to]
            if hit:
                x += hit[0][3]
                i = hit[0][1]
                continue
            x += adv[i]
            i += 1
        return x

    rows = []
    start = 0
    for i, ch in enumerate(letters + ["\n"]):
        if ch != "\n":
            continue
        if t.get("boxWidth") is None:
            rows.append([start, i, first])
        else:
            wrap(letters, start, i, t["boxWidth"], (first, left), lambda a, b, x: scale(walk(a, b, x)), rows)
        start = i + 1
    for row in rows:
        while row[1] > row[0] and letters[row[1] - 1] == " ":
            row[1] -= 1
    measured = []
    for a, b, x0 in rows:
        x0m = scale(x0)
        measured.append((x0m, scale(walk(a, b, x0)) - x0m))
    box = t["boxWidth"] if t.get("boxWidth") is not None else max([0.0] + [x0 + w for x0, w in measured])
    pitch = 5.0 / 3.0 * h * t.get("lineSpacing", 1.0)
    along, up = SHARES[t.get("align")]
    cols = lay.get("columns")
    if cols:
        per, count, step = max(math.floor(cols["height"] / pitch), 1), max(cols["count"], 1), box + cols["gap"] * h
    else:
        per, count, step = None, 1, 0.0
    out = []
    for r, ((a, b, x0), (x0m, width)) in enumerate(zip(rows, measured)):
        col = min(r // per, count - 1) if per else 0
        row = r - col * per if per else r
        shift = along * (box - x0m - width) + col * step
        y = row * pitch
        frm, at, k = a, x0, a
        while True:
            if k == b or letters[k] == "\t":
                end_x = walk(frm, k, at)
                out.append({"start": frm, "end": k, "width": scale(end_x) - scale(at), "x": shift + scale(at), "y": y})
                if k == b:
                    break
                at = stop(end_x)
                frm = k + 1
            k += 1
    if cols:
        depth = max(max(0, (len(rows) if c + 1 == count else min(len(rows), c * per + per)) - c * per) for c in range(count))
        total = count * box + (count - 1) * cols["gap"] * h
    else:
        depth = len(rows)
        total = box
    below = (depth - 1) * pitch if depth > 0 else 0.0
    if up < 0:
        rise = up * h - below
    elif 0 < up < 1:
        rise = up * h - below / 2.0
    else:
        rise = up * h
    return {"lines": out, "width": total, "pitch": pitch, "shares": [along * total, rise]}


def wrap(letters, frm, to, box, indents, end_of, out):
    """One paragraph wrapped word by word: its first line from `indents[0]`, the others from `indents[1]`."""
    start, filled, i, x0 = frm, None, frm, indents[0]
    while True:
        while i < to and letters[i] == " ":
            i += 1
        if i >= to:
            break
        word = i
        while i < to and letters[i] != " ":
            i += 1
        if filled is not None and end_of(start, i, x0) > box:
            out.append([start, filled, x0])
            start = word
            x0 = indents[1]
        filled = i
    out.append([start, to, x0])


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
    # docs/adr/0324: indents, tabs, lists, stacked fractions and columns.
    ("girintili ilk satır, kutuda sarılır", text("Birinci paragrafın ilk satırı girintili, gerisi değil", boxWidth=24.0, layout={"indent": 2.0})),
    ("sol girintili paragraf", text("Sol girintiden başlayan uzun bir paragraf sarılır", boxWidth=24.0, layout={"leftIndent": 3.0})),
    ("madde işaretli liste: asılı girinti ve sekme", text("Malzemeler\n•\tKum ve çakıl karışımı kutuya sığmayınca alt satıra geçer\n•\tÇimento", boxWidth=30.0, layout={"indent": -2.0, "leftIndent": 2.0})),
    ("numaralı liste", text("1.\tBirinci madde\n2.\tİkinci madde\n10.\tOnuncu madde", layout={"indent": -2.0, "leftIndent": 2.0})),
    ("varsayılan sekmeler her dört yükseklikte", text("Ad\tSoyad\tParsel\nAli\tYılmaz\t101/5")),
    ("verilen sekme durakları, sonra varsayılanlar", text("a\tb\tc\td", layout={"tabs": [3.0, 5.5]})),
    ("ortalı satır sekmeli", text("Kot\t+12,50", boxWidth=20.0, align="topCenter", layout={"tabs": [6.0]})),
    ("kesir: pay ve payda üst üste", text("Oran 1/2 ve 15/32", runs=[{"start": 5, "end": 8, "script": "stack"}, {"start": 12, "end": 17, "script": "stack"}])),
    ("kesir: yan yana # ve toleranslı ^", text("Ölçü 3#4 ve 20+0.1^-0.1", runs=[{"start": 5, "end": 8, "script": "stack"}, {"start": 14, "end": 23, "script": "stack"}])),
    ("iki sütun: satırlar sütun yüksekliğince", text("Bir\nİki\nÜç\nDört\nBeş", boxWidth=10.0, layout={"columns": {"count": 2, "gap": 1.0, "height": 8.5}})),
    ("üç sütun, son sütun kalanları alır", text("a\nb\nc\nd\ne\nf\ng\nh", boxWidth=6.0, align="middleCenter", layout={"columns": {"count": 3, "gap": 0.5, "height": 4.5}})),
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
