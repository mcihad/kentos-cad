"""The shared cases of a text's editing rules (docs/adr/0145 §3): Artır's next
number, Bul ve değiştir's matching (with or without wildcards), and Okunur
yap's turn.

    python3 scripts/fixtures/text_cases.py           # writes the files
    python3 scripts/fixtures/text_cases.py --check   # writes nothing; compares

Writes fixtures/text/v1/increment.json, pattern.json, readable.json,
realign.json and file.json. The
rules are written here from the ADR on their own, not from an
implementation's output; the geometry core (crates/shared/geometry-core,
`text::edit` and `TextPlace::readable`, natively and through WASM) is held to
them.

- Artır: the digits a text ends with, as a decimal number, one more, as many
  digits as they were at least (`009` → `010`, `99` → `100`); a text that does
  not end with an ASCII digit has no next.
- Bul ve değiştir, without wildcards: every occurrence of the text looked for,
  left to right, none overlapping, replaced; with “tam sözcük” only where the
  character before and after it (if any) is not a letter, a digit or `_`.
  With wildcards: the pattern matches the whole text; `*` is any run of
  characters, empty too, each taking as few as it can from the left; the
  replacement's first `*` is what the pattern's first `*` took, and so on (a
  `*` past them is empty). Case folded, when asked, Turkish: I → ı, İ → i,
  every other letter its one lowercase letter. No match: no text.
- Okunur yap: a text turned more than 90° and at most 270° (its turn taken
  from 0 up to 360) reads upside down; it turns half round about the middle of
  its box, so the box stays where it was: its point moves by w·(1 − 2a) along
  and h·(0.92 − 2b) up the old baseline, where w is its width, h its height,
  a and b its alignment's shares along and up (the box is 0.23 h under the
  baseline and 1.15 h over it: 0.92 is the sum). Its turn is 180° more, taken
  from 0 up to 360. Any other text is left alone.
- Metin dosyası yerleştir's file (§6): its lines end at \\r\\n, \\n or \\r, a
  last line break ends the last line, a byte order mark is no letter, each
  line is trimmed as JavaScript trims; refused: not UTF-8, more than 10 000
  lines, nothing but empty lines.
- Hizayı değiştir (Öznitelikler's Hiza, §6): the text stays where it is; its
  point becomes the new alignment's point of the same box: it moves by
  w·(a′ − a) along its baseline and h·(b′ − b) up from it, a, b the old
  alignment's shares and a′, b′ the new one's.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/text/v1"

# ── Artır ───────────────────────────────────────────────────────────────


def increment(text):
    i = len(text)
    while i > 0 and "0" <= text[i - 1] <= "9":
        i -= 1
    if i == len(text):
        return None
    digits = text[i:]
    n = str(int(digits) + 1)
    return text[:i] + n.rjust(len(digits), "0")


INCREMENT = [
    ("sayı", "101"),
    ("bölü: sondaki sayı", "101/12"),
    ("harf ve sayı", "P9"),
    ("sıfır dolgusu korunur", "A-009"),
    ("dolgu taşınca uzar", "099"),
    ("dokuzlar bir basamak ekler", "999"),
    ("sıfır", "0"),
    ("adada parsel", "Ada 1284"),
    ("uzun sayı", "12345678901234567890"),
    ("ondalık sayının son basamağı", "1.5"),
    ("sonu sayı değil", "R-?"),
    ("boş yazı", ""),
    ("sonda boşluk", "101 "),
    ("ASCII olmayan rakam sayı sayılmaz", "Parsel ٣"),
    ("Türkçe harfler korunur", "Ağaç-07"),
]

# ── Bul ve değiştir ─────────────────────────────────────────────────────


def fold(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


def same(a, b, caseless):
    return fold(a) == fold(b) if caseless else a == b


def word_char(c):
    return c.isalnum() or c == "_"


def replace_plain(text, find, repl, caseless, whole):
    if not find:
        return None
    out, i, hit = [], 0, False
    n, m = len(text), len(find)
    while i < n:
        if i + m <= n and all(same(text[i + k], find[k], caseless) for k in range(m)):
            before = text[i - 1] if i > 0 else None
            after = text[i + m] if i + m < n else None
            if not whole or ((before is None or not word_char(before)) and (after is None or not word_char(after))):
                out.append(repl)
                i += m
                hit = True
                continue
        out.append(text[i])
        i += 1
    return "".join(out) if hit else None


def match_glob(text, pattern, caseless):
    """The captures of the stars, each as short as it can be from the left; None without a match."""
    parts = pattern.split("*")

    def go(ti, pi, caps):
        # parts[pi] must match at ti; then a star (if any) and the rest.
        lit = parts[pi]
        if ti + len(lit) > len(text) or not all(same(text[ti + k], lit[k], caseless) for k in range(len(lit))):
            return None
        ti += len(lit)
        if pi == len(parts) - 1:
            return caps if ti == len(text) else None
        for end in range(ti, len(text) + 1):
            r = go(end, pi + 1, caps + [text[ti:end]])
            if r is not None:
                return r
        return None

    return go(0, 0, [])


def replace_glob(text, find, repl, caseless):
    if not find:
        return None
    caps = match_glob(text, find, caseless)
    if caps is None:
        return None
    out, k = [], 0
    for c in repl:
        if c == "*":
            out.append(caps[k] if k < len(caps) else "")
            k += 1
        else:
            out.append(c)
    return "".join(out)


def replace(text, find, repl, wildcard, caseless, whole):
    return replace_glob(text, find, repl, caseless) if wildcard else replace_plain(text, find, repl, caseless, whole)


PATTERN = [
    # name, text, find, replace, wildcard, caseless, whole
    ("her geçen yer", "Ada 101 / Ada 102", "Ada", "Parsel", False, False, False),
    ("bulunamayan", "Ada 101", "Pafta", "Parsel", False, False, False),
    ("büyük küçük harf ayrı", "ada 101", "Ada", "Parsel", False, False, False),
    ("büyük küçük harf ayrılmaz: Türkçe I ve ı", "IŞIK 3", "ışık", "Lamba", False, True, False),
    ("Türkçe İ ve i", "İL SINIRI", "il", "İlçe", False, True, False),
    ("tam sözcük", "Ada Adalar Ada", "Ada", "Parsel", False, False, True),
    ("tam sözcük: rakamla bitişik değil", "A1 A 1A", "A", "B", False, False, True),
    ("örtüşmeyen, soldan", "aaaa", "aa", "b", False, False, False),
    ("boş aranan", "Ada", "", "x", False, False, False),
    ("joker: önek", "Ada 101", "Ada *", "Parsel *", True, False, False),
    ("joker: bütün yazıya uyar", "Adalar 3", "Ada *", "Parsel *", True, False, False),
    ("joker: iki yıldız", "101/12", "*/*", "*-*", True, False, False),
    ("joker: kısa yakalar", "1/2/3", "*/*", "*-*", True, False, False),
    ("joker: sonek", "R-12", "*-12", "*-13", True, False, False),
    ("joker: yalnız yıldız", "Ada 5", "*", "[*]", True, False, False),
    ("joker: değiştirmede yıldız yok", "Ada 5", "Ada *", "Yok", True, False, False),
    ("joker: fazla yıldız boş kalır", "Ada 5", "Ada *", "* / *", True, False, False),
    ("joker: harf ayrılmaz", "ADA 7", "ada *", "Parsel *", True, True, False),
    ("joker: yıldızsız kalıp bütün yazıdır", "Ada", "Ada", "Parsel", True, False, False),
    ("joker: yıldızsız kalıp parçaya uymaz", "Ada 1", "Ada", "Parsel", True, False, False),
    ("joker: boş yakalama", "Ada ", "Ada *", "P*", True, False, False),
]

# ── Okunur yap ──────────────────────────────────────────────────────────

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


def readable(p, height, rotation, align, width):
    r = rotation % 360.0
    if not (90.0 < r <= 270.0):
        return None
    a, b = SHARES[align]
    t = math.radians(r)
    ux, uy = math.cos(t), math.sin(t)
    vx, vy = -uy, ux
    along = width * (1.0 - 2.0 * a)
    up = height * (0.92 - 2.0 * b)
    return {"p": {"x": p[0] + ux * along + vx * up, "y": p[1] + uy * along + vy * up}, "rotation": (r + 180.0) % 360.0}


READABLE = [
    # name, p, height, rotation, align, width
    ("ters: 180°", (487100.0, 4420200.0), 2.0, 180.0, None, 10.0),
    ("ters: 200° → 20°", (487100.0, 4420200.0), 2.0, 200.0, None, 10.0),
    ("dik yukarıdan aşağı: 270° → 90°", (487100.0, 4420200.0), 2.5, 270.0, None, 8.0),
    ("tam 90° okunur, değişmez", (487100.0, 4420200.0), 2.0, 90.0, None, 10.0),
    ("90°'den biraz fazla", (487100.0, 4420200.0), 2.0, 90.5, None, 10.0),
    ("okunur: 30°", (487100.0, 4420200.0), 2.0, 30.0, None, 10.0),
    ("okunur: 300°", (487100.0, 4420200.0), 2.0, 300.0, None, 10.0),
    ("eksi dönüş: −150° (210°)", (487100.0, 4420200.0), 2.0, -150.0, None, 10.0),
    ("360'tan büyük: 560° (200°)", (487100.0, 4420200.0), 2.0, 560.0, None, 10.0),
    ("ortalı yazı: kutunun ortası yerinde", (487100.0, 4420200.0), 2.0, 180.0, "middleCenter", 10.0),
    ("sağ üst hizalı, 210°", (487100.0, 4420200.0), 1.5, 210.0, "topRight", 6.4),
    ("taban orta, 135°", (0.0, 0.0), 3.0, 135.0, "baselineCenter", 12.25),
    ("sol alt, 250°", (486512.34, 4420187.52), 2.0, 250.0, "bottomLeft", 7.5),
]


def realign(p, height, rotation, align, to, width):
    a, b = SHARES[align]
    a2, b2 = SHARES[to]
    t = math.radians(rotation)
    ux, uy = math.cos(t), math.sin(t)
    vx, vy = -uy, ux
    along = width * (a2 - a)
    up = height * (b2 - b)
    return {"x": p[0] + ux * along + vx * up, "y": p[1] + uy * along + vy * up}


REALIGN = [
    # name, p, height, rotation, align (old), to (new), width
    ("sol tabandan ortaya", (487100.0, 4420200.0), 2.0, 0.0, None, "middleCenter", 10.0),
    ("ortadan sol tabana: eskiye döner", (487105.0, 4420201.0), 2.0, 0.0, "middleCenter", None, 10.0),
    ("sağ üstten sol alta", (487100.0, 4420200.0), 2.5, 0.0, "topRight", "bottomLeft", 8.0),
    ("30° dönük, sol tabandan sağ tabana", (487100.0, 4420200.0), 2.0, 30.0, None, "baselineRight", 12.0),
    ("200° dönük, orta alttan sağ ortaya", (486512.34, 4420187.52), 1.5, 200.0, "bottomCenter", "middleRight", 6.4),
    ("aynı hiza: yer değişmez", (487100.0, 4420200.0), 2.0, 45.0, "topCenter", "topCenter", 9.0),
    ("eksi dönüş: −90°, sol üstten orta tabana", (0.0, 0.0), 3.0, -90.0, "topLeft", "baselineCenter", 12.25),
]


MAX_LINES = 10_000


def file_lines(data):
    """The file's lines, or why it is refused (a kind): not UTF-8, more than
    10 000 lines, nothing but empty lines. (More than 1 MB is left to the
    runners' own tests: the file would not sit in a fixture.)"""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return {"error": "notUtf8"}
    if text.startswith("﻿"):
        text = text[1:]
    pieces = []
    line = ""
    i = 0
    while i < len(text):
        c = text[i]
        if c == "\r":
            if i + 1 < len(text) and text[i + 1] == "\n":
                i += 1
            pieces.append(line)
            line = ""
        elif c == "\n":
            pieces.append(line)
            line = ""
        else:
            line += c
        i += 1
    pieces.append(line)
    if pieces and pieces[-1] == "":
        pieces.pop()
    if len(pieces) > MAX_LINES:
        return {"error": "tooMany"}
    trimmed = [js_trim(p) for p in pieces]
    if not any(trimmed):
        return {"error": "empty"}
    return {"lines": trimmed} if len(trimmed) <= 20 else {"count": len(trimmed), "first": trimmed[0], "last": trimmed[-1]}


# JavaScript's String.prototype.trim: its white space and line terminators.
JS_SPACE = set("\t\n\x0b\x0c\r \xa0                　﻿")


def js_trim(s):
    a, b = 0, len(s)
    while a < b and s[a] in JS_SPACE:
        a += 1
    while b > a and s[b - 1] in JS_SPACE:
        b -= 1
    return s[a:b]


FILE = [
    # name, bytes
    ("satırlar \\n ile", "Ada 101\nAda 102".encode()),
    ("Windows satır sonları, son satır sonu", "Ada 101\r\nAda 102\r\n".encode()),
    ("eski Mac satır sonları", "Ada 101\rAda 102".encode()),
    ("boş satır yerini tutar", "Ada 101\n\nAda 103".encode()),
    ("satırlar kırpılır (sekme, boşluk)", "  Ada 101  \n\tPark\t".encode()),
    ("bayt sırası işareti harf değildir", "﻿Ada 101\n".encode()),
    ("sondaki iki satır sonu: bir boş satır kalır", "Ada 101\n\n".encode()),
    ("Türkçe harfler", "Çınaraltı Sokağı\nİğdır Caddesi".encode()),
    ("yalnız boş satırlar: boş", "\n  \n\t\n".encode()),
    ("boş dosya", b""),
    ("UTF-8 değil (Latin-5 ş)", b"Kar\xfe\xfdyaka"),
    ("tam 10 000 satır", ("a\n" * 10_000).encode()),
    ("10 001 satır: çok", ("a\n" * 10_001).encode()),
]


def build():
    increment_cases = [{"name": n, "text": t, "next": increment(t)} for n, t in INCREMENT]
    pattern_cases = [
        {"name": n, "text": t, "find": f, "replace": r, "wildcard": w, "caseless": c, "wholeWord": wh, "expect": replace(t, f, r, w, c, wh)}
        for n, t, f, r, w, c, wh in PATTERN
    ]
    readable_cases = []
    for n, p, h, rot, align, width in READABLE:
        c = {"name": n, "p": {"x": p[0], "y": p[1]}, "height": h, "rotation": rot, "width": width, "expect": readable(p, h, rot, align, width)}
        if align:
            c["align"] = align
        readable_cases.append(c)
    realign_cases = []
    for n, p, h, rot, align, to, width in REALIGN:
        c = {"name": n, "p": {"x": p[0], "y": p[1]}, "height": h, "rotation": rot, "width": width}
        if align:
            c["align"] = align
        c["to"] = to
        c["expect"] = realign(p, h, rot, align, to, width)
        realign_cases.append(c)
    file_cases = []
    for n, data in FILE:
        c = {"name": n}
        try:
            c["text"] = data.decode("utf-8")
        except UnicodeDecodeError:
            c["hex"] = data.hex()
        c["expect"] = file_lines(data)
        file_cases.append(c)
    common = {"format": "kentos.text-cases", "version": 1}
    return {
        "increment.json": {
            **common,
            "title": "Artır: yazının sonundaki sayının bir fazlası (ADR 0145 §3)",
            "note": "Yazının bittiği ASCII rakamları ondalık sayı olarak bir artar, en az eski basamak sayısıyla (sıfır dolgusu korunur, taşınca uzar). Sonu rakam olmayan yazının sonrakisi yoktur (null).",
            "cases": increment_cases,
        },
        "pattern.json": {
            **common,
            "title": "Bul ve değiştir: düz ve jokerli eşleme (ADR 0145 §3)",
            "note": "Jokersiz: aranan her geçtiği yerde, soldan, örtüşmeden değişir; tam sözcükte önündeki ve ardındaki karakter harf, rakam ya da _ olamaz. Jokerli: kalıp bütün yazıya uyar, * boş da olabilen herhangi bir dizidir ve soldan en kısa olanı alır; değiştirmedeki i. * kalıbın i. *'ının aldığıdır (fazlası boş). Büyük küçük harf ayrılmazsa Türkçe katlama: I → ı, İ → i, öbürleri tek küçük harfleri. Uymayan: null.",
            "cases": pattern_cases,
        },
        "readable.json": {
            **common,
            "title": "Okunur yap: ters okunan yazı kutusunun ortası çevresinde yarım döner (ADR 0145 §3)",
            "note": "Dönüşü 0–360'a getirilince 90°'den büyük, en çok 270° olan yazı ters okunur. Noktası eski taban çizgisi boyunca w·(1 − 2a), ona dik h·(0,92 − 2b) kayar (w genişlik, h yükseklik, a ve b hizanın boyuna ve yukarı payları; kutu taban çizgisinin 0,23 h altından 1,15 h üstüne); dönüşü 180° artar, 0–360'a getirilir. Öbürleri değişmez (null). Genişlik burada verilir: çekirdekte yazı tipinin ölçüsüdür.",
            "cases": readable_cases,
        },
        "file.json": {
            **common,
            "title": "Metin dosyası yerleştir: dosyanın satırları (ADR 0145 §6)",
            "note": "Satırlar \\r\\n, \\n ya da \\r ile biter; son satır sonu son satırı bitirir; bayt sırası işareti harf değildir; her satır JavaScript'in trim'iyle kırpılır, boş satır yerini tutar. Reddedilir: UTF-8 olmayan (notUtf8), 10 000'den çok satırlı (tooMany), yalnız boş satırlı (empty). 1 MB sınırı koşucuların kendi testlerindedir. Girdi `text` (UTF-8) ya da `hex` (baytlar); uzun sonuç `count`, `first`, `last` ile.",
            "cases": file_cases,
        },
        "realign.json": {
            **common,
            "title": "Hizayı değiştir: yazı yerinde kalır, noktası kutunun yeni hizadaki noktası olur (ADR 0145 §6)",
            "note": "Noktası taban çizgisi boyunca w·(a′ − a), ona dik h·(b′ − b) kayar (w genişlik, h yükseklik; a, b eski, a′, b′ yeni hizanın boyuna ve yukarı payları; hizasız: sol taban, 0 ve 0). Dönüş değişmez. to: null sol taban çizgisidir. Genişlik burada verilir: çekirdekte yazı tipinin ölçüsüdür.",
            "cases": realign_cases,
        },
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
        print(f"text cases match: {', '.join(files)}")
        return 0
    DIR.mkdir(parents=True, exist_ok=True)
    for name, v in files.items():
        (DIR / name).write_text(text_of(v), encoding="utf-8")
    print(f"written: {', '.join(files)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
