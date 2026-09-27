"""The language since docs/adr/0100 §4, evaluated here independently of the
engine: durum … son (CASE), içinde (IN), arasında (BETWEEN), gibi and benzer
(LIKE, ILIKE), boş / boş değil and IS [NOT] NULL, the power ^, and the
functions kök, tavan, taban, pi, sol, sağ, bul, birleştir and sağdoldur.

    python3 scripts/fixtures/expression_language.py           # writes the file
    python3 scripts/fixtures/expression_language.py --check   # writes nothing; compares

Writes fixtures/expression/v2/language.json: objects (their attributes, all
text, as the drawing keeps them) and, for each source, the value it gives on
each object, or that it does not compile. The rules are the language's as
docs/PROCESSING.md §5 states them, written anew here:
- a field reads its attribute's text, none when the object has no such
  attribute; empty text and none are both “boş”;
- numbers are read from text by the grammar [+-](d+[.d*]|.d+)[e[+-]d+]
  after trimming; equality of numbers holds within 1e-9 (relative above 1);
  text compares in the Turkish alphabet;
- a condition holds for true, a number other than 0, and text that is not
  empty;
- ^ binds tighter than the sign before it and groups from the right; its
  value is exact (fractions, rounded once) for whole exponents, else
  e^(y·ln x) to 60 digits, rounded once: the correctly rounded power. The
  engine's power (libm's, fdlibm's algorithm: the same bits on every
  platform) is “nearly rounded”, within one unit in the last place, so a
  case such a power decides carries "ulp": 1 (16 ^ (1/3): the engine
  2.519842099789746, the true value 0.41 units from 2.5198420997897464);
  whole powers are exact. kök is the square root to 60 digits, rounded once;
- positions and lengths of text count UTF-16 units, as JavaScript does;
- a value that is not a finite number is boş; text past JavaScript's longest
  string makes the whole expression boş, but only where it is computed: the
  branches durum does not choose are not.

Rust (crates/shared/expression/tests/language.rs) and the web
(apps/web/src/model/expression/language.test.ts) compare the engine with this
file to the bit (within one unit in the last place where "ulp": 1).
"""
import decimal
import json
import math
import sys
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures/expression/v2/language.json"

# V8's longest string, in UTF-16 units.
MAX_STRING_UNITS = (1 << 29) - 24

OBJECTS = [
    {"Ada": "1245", "Parsel": "12", "Nitelik": "Arsa", "Kat": "3", "Ad": "Çınar",
     "Kod": "P00012", "Durum": "açık", "Son": "5", "Gibi": "x", "Not": "", "Oran": "0.5"},
    {"Ada": "1246", "Parsel": "7", "Nitelik": "Tarla", "Kat": "5", "Ad": "çam",
     "Kod": "a%b", "Durum": "kapalı", "Son": "", "Gibi": "", "Not": "iyi", "Oran": "2"},
    {"Ada": "", "Parsel": "12.0", "Nitelik": "Yol", "Kat": "x", "Ad": "Işık",
     "Kod": "a😀b", "Durum": "", "Oran": "-8"},
    {},
    {"Ada": " 12 ", "Parsel": "1e3", "Nitelik": "arsa", "Kat": "-2.5", "Ad": "İzmir",
     "Kod": "12_4", "Durum": "açık", "Son": "-3", "Gibi": "Gibi", "Not": " ", "Oran": "16"},
    {"Ada": "12", "Parsel": "0", "Nitelik": "ARSA ", "Kat": "4", "Ad": "Ceviz",
     "Kod": "\\x", "Durum": "AÇIK", "Oran": "1e308"},
]

SOURCES = [
    # durum … son: the first condition that holds, else yoksa's value, else boş.
    "durum eğer Kat > 4 ise 'yüksek' eğer Kat > 2 ise 'orta' yoksa 'alçak' son",
    "durum eğer Kat > 4 ise 'yüksek' son",
    "CASE WHEN Nitelik = 'Arsa' THEN 1 WHEN Nitelik = 'Tarla' THEN 2 ELSE 0 END",
    "durum eğer Ada ise Ada yoksa 'adasız' son",
    # Not is a keyword (değil): the field is written in brackets.
    "durum eğer [Not] ise 'var' yoksa 'yok' son",
    "durum eğer Oran ise Oran * 2 son",
    "durum eğer Kat arasında 3 ve 5 ise 'bina' son || '!'",
    "durum eğer doğru ise 1 yoksa doldur('x', 1e12) son",
    "durum eğer Kat > 4 ise doldur('x', 1e12) yoksa 'kısa' son",
    "durum eğer yanlış ise 1 son = boş",
    "durum eğer Kat = 3 ise durum eğer Ada = '1245' ise 'iç' son son",
    # içinde: equal (as = compares) to one of the items; eager.
    "Nitelik içinde ('Arsa', 'Tarla')",
    "Nitelik değil içinde ('Arsa', 'Tarla')",
    "Parsel içinde (12, 7)",
    "Kat IN (3, '5', 4)",
    "Ada içinde ('', 'x')",
    "Ada NOT IN (1245)",
    "Durum içinde ('açık')",
    "1 içinde (1, doldur('x', 1e12))",
    # arasında: low ≤ x ≤ high in <'s order; false with an empty value.
    "Kat arasında 3 ve 5",
    "Kat değil arasında 3 ve 5",
    "Parsel BETWEEN 7 AND 12",
    "Ad arasında 'Çal' ve 'Dut'",
    "Ad arasında 'c' ve 'd'",
    "Ada arasında 1000 ve 2000",
    "Kat arasında Son ve 10",
    # gibi / benzer: % any text, _ one character, \\ the next as it is.
    "Ada gibi '12%'",
    "Kod gibi 'P___12'",
    "Kod gibi 'a\\%b'",
    "Kod gibi '%\\_%'",
    "Kod gibi 'a_b'",
    "Kod gibi '\\\\x'",
    "Ad gibi 'ç%'",
    "Ad benzer 'ç%'",
    "Ad ILIKE 'CIN%'",
    "Nitelik benzer 'arsa'",
    "Nitelik benzer 'arsa%'",
    "Nitelik değil gibi 'Arsa'",
    "Ad LIKE '%'",
    "Parsel gibi 12",
    "[Not] benzer ' '",
    # boş / boş değil / IS [NOT] NULL: none or empty text.
    "Ada boş",
    "Ada boş değil",
    "[Not] IS NULL",
    "Son is not null",
    "Ada boş = doğru",
    "durum eğer Kat boş ise 'yok' yoksa Kat son",
    # ^: tighter than the sign, from the right.
    "2 ^ 10",
    "-2 ^ 2",
    "(-2) ^ 2",
    "2 ^ 3 ^ 2",
    "2 ^ -1",
    "Kat ^ 2",
    "Oran ^ 0.5",
    "Oran ^ 2",
    "10 ^ -2",
    "0 ^ -1",
    "1.5 ^ 3 * 2",
    "Oran ^ (1 / 3)",
    "(-8) ^ (1 / 3)",
    # Numbers.
    "kök(Oran)",
    "sqrt(2)",
    "tavan(Kat)",
    "taban(Kat)",
    "floor(-0.5) = 0",
    "tavan(-0.5)",
    "tavan('x')",
    "yuvarla(pi(), 4)",
    "pi() * 10 ^ 2",
    # Text.
    "sol(Kod, 3)",
    "sağ(Kod, 2)",
    "LEFT(Kod, 2)",
    "right(Ad, 10)",
    "sol(Ada, 0) = ''",
    "sol(Kod, -1)",
    "sol(12.5, 3)",
    "bul(Kod, '0')",
    "bul(Ad, 'ı')",
    "strpos(Kod, 'b')",
    "bul(Kod, '')",
    "bul(Ada, boş)",
    "birleştir('Ada ', Ada, ' Parsel ', Parsel)",
    "concat(Kat, boş, doğru)",
    "birleştir(Ada)",
    "sağdoldur(Parsel, 5, '_')",
    "sağdoldur(Kod, 4)",
    "rpad(Ada, 2, 'xyz')",
    "sağdoldur(Ada, 3, '')",
    "sağdoldur(Ada, 1e12)",
    # Fields named like the new words stay fields.
    "Durum = 'açık'",
    "Son || Gibi",
    "Gibi gibi 'G%'",
    "durum eğer Durum ise Durum son",
    # What does not compile.
    "Kat arasında 3",
    "Kat arasında 3 ve",
    "Nitelik içinde 'Arsa'",
    "Nitelik içinde ()",
    "durum eğer Kat > 3 'x' son",
    "durum eğer Kat > 3 ise 'x'",
    "durum son",
    "Ada IS 3",
    "Ada boş / 2",
    "pi(1)",
    "kök()",
    "sol('a')",
    "2 ^",
]


# ── Values ───────────────────────────────────────────────────────────────

class Thrown(Exception):
    """Text past JavaScript's longest string: the whole expression is boş."""


class Invalid(Exception):
    """The source does not compile."""


def units(s):
    """UTF-16 units of a text."""
    return len(s.encode("utf-16-le")) // 2


def unit_slice(s, start, end):
    """JavaScript's slice by UTF-16 units; half a surrogate pair reads as U+FFFD."""
    b = s.encode("utf-16-le", "surrogatepass")[2 * start:2 * end]
    out = []
    for i in range(0, len(b), 2):
        u = b[i] | (b[i + 1] << 8)
        out.append(u)
    text = []
    i = 0
    while i < len(out):
        u = out[i]
        if 0xD800 <= u < 0xDC00 and i + 1 < len(out) and 0xDC00 <= out[i + 1] < 0xE000:
            text.append(chr(0x10000 + ((u - 0xD800) << 10) + (out[i + 1] - 0xDC00)))
            i += 2
        elif 0xD800 <= u < 0xE000:
            text.append("�")
            i += 1
        else:
            text.append(chr(u))
            i += 1
    return "".join(text)


SPACE = set(" \t\n\r\x0b\x0c      　﻿") | {chr(c) for c in range(0x2000, 0x200B)}


def trim(s):
    a, b = 0, len(s)
    while a < b and s[a] in SPACE:
        a += 1
    while b > a and s[b - 1] in SPACE:
        b -= 1
    return s[a:b]


def read_number(s):
    """A number by the language's grammar, or None."""
    t = trim(s)
    i = 0
    if t[:1] in ("+", "-"):
        i = 1
    j = i
    while j < len(t) and t[j].isascii() and t[j].isdigit():
        j += 1
    whole = j - i
    frac = 0
    if j < len(t) and t[j] == ".":
        j += 1
        k = j
        while j < len(t) and t[j].isascii() and t[j].isdigit():
            j += 1
        frac = j - k
    if whole == 0 and frac == 0:
        return None
    if j < len(t) and t[j] in "eE":
        k = j + 1
        if k < len(t) and t[k] in "+-":
            k += 1
        m = k
        while k < len(t) and t[k].isascii() and t[k].isdigit():
            k += 1
        if k == m:
            return None
        j = k
    if j != len(t):
        return None
    x = float(t)
    return x if math.isfinite(x) else None


def number_text(x):
    """JavaScript's String(x) with float noise dropped (12 significant digits)."""
    if x == 0:
        return "0"
    if x == int(x) and abs(x) < 1e21:
        return str(int(x))
    s = "%.12g" % x
    assert "e" not in s, f"the fixture writes plain numbers as text: {x}"
    return s


def is_empty(v):
    return v is None or v == ""


def truthy(v):
    if v is None:
        return False
    if isinstance(v, bool):
        return v
    if isinstance(v, float):
        return v != 0
    return v != ""


def to_number(v):
    if isinstance(v, bool):
        return 1.0 if v else 0.0
    if isinstance(v, float):
        return v if math.isfinite(v) else None
    if isinstance(v, str):
        return read_number(v)
    return None


def text(v):
    if v is None:
        return ""
    if isinstance(v, bool):
        return "doğru" if v else "yanlış"
    if isinstance(v, float):
        return number_text(v)
    return v


def lower_tr(s):
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in s)


ALPHABET = "abcçdefgğhıijklmnoöpqrsştuüvwxyz"


def collation_key(s):
    """The Turkish alphabet's order: other characters, then digits, then
    letters; case aside. The fixture's texts differ in a letter or a digit,
    or one is the start of the other; `compare` checks it."""
    key = []
    for c in lower_tr(s):
        if c in ALPHABET:
            key.append((2, ALPHABET.index(c)))
        elif c.isascii() and c.isdigit():
            key.append((1, int(c)))
        else:
            key.append((0, ord(c)))
    return key


def equals(a, b):
    if is_empty(a) or is_empty(b):
        return is_empty(a) and is_empty(b)
    if isinstance(a, bool) or isinstance(b, bool):
        return truthy(a) == truthy(b)
    na, nb = to_number(a), to_number(b)
    if na is not None and nb is not None:
        return abs(na - nb) <= 1e-9 * max(1.0, abs(na), abs(nb))
    return text(a) == text(b)


def compare(a, b):
    """The sign of a − b in <'s order, None with an empty value."""
    if is_empty(a) or is_empty(b):
        return None
    na, nb = to_number(a), to_number(b)
    if na is not None and nb is not None:
        return (na > nb) - (na < nb)
    ka, kb = collation_key(text(a)), collation_key(text(b))
    if ka == kb:
        assert text(a) == text(b), f"the fixture's texts differ in case only: {a!r} {b!r}"
    for x, y in zip(ka, kb):
        if x != y:
            assert x[0] > 0 or y[0] > 0, f"two signs decide the order: {a!r} {b!r}"
            break
    return (ka > kb) - (ka < kb)


def fold(s):
    """Case and the Turkish letters' marks aside (no trimming)."""
    out = []
    for c in s:
        for u in ("İ" if c == "i" else c).upper():
            out.append({"Ç": "C", "Ğ": "G", "İ": "I", "Ö": "O", "Ş": "S", "Ü": "U"}.get(u, u))
    return "".join(out)


def pattern_parts(p):
    parts = []
    i = 0
    while i < len(p):
        c = p[i]
        if c == "%":
            parts.append(("any",))
        elif c == "_":
            parts.append(("one",))
        elif c == "\\" and i + 1 < len(p):
            i += 1
            parts.append(("char", p[i]))
        else:
            parts.append(("char", c))
        i += 1
    return parts


def fits(t, p):
    """Whether text t (code points) fits pattern p, by trying every split."""
    parts = pattern_parts(p)
    memo = {}

    def go(i, j):
        if (i, j) in memo:
            return memo[(i, j)]
        if j == len(parts):
            r = i == len(t)
        elif parts[j][0] == "any":
            r = any(go(k, j + 1) for k in range(i, len(t) + 1))
        elif i == len(t):
            r = False
        elif parts[j][0] == "one":
            r = go(i + 1, j + 1)
        else:
            r = t[i] == parts[j][1] and go(i + 1, j + 1)
        memo[(i, j)] = r
        return r

    return go(0, 0)


def like(x, p, folded):
    if is_empty(x) or p is None:
        return False
    tx, tp = text(x), text(p)
    if folded:
        tx, tp = fold(tx), fold(tp)
    return fits(tx, tp)


PRECISE = decimal.Context(prec=60)


def power(x, y):
    """x ** y: exact for whole exponents; else e^(y·ln x) to 60 digits."""
    if y == int(y) and abs(y) <= 64:
        if x == 0 and y < 0:
            return math.inf
        r = Fraction(x) ** int(y)
        try:
            return float(r)
        except OverflowError:
            return math.inf if r > 0 else -math.inf
    if x < 0:
        return math.nan
    if x == 0:
        return 0.0
    r = PRECISE.exp(PRECISE.multiply(Decimal(y), PRECISE.ln(Decimal(x))))
    NEARLY.append(True)
    return float(r)


# Set while a case is evaluated: a power with an exponent that is not whole was taken.
NEARLY = []


def square_root(x):
    return float(PRECISE.sqrt(Decimal(x))) if x >= 0 else math.nan


def js_round(x):
    return math.floor(x + 0.5)


def pad(v, n, fill, default, end):
    if v is None or to_number(n) is None:
        return None
    f = text(fill) if fill is not None else ""
    f = unit_slice(f, 0, 1) if f else default
    target = max(0, js_round(to_number(n)))
    t = text(v)
    if target <= units(t):
        return t
    if target > MAX_STRING_UNITS:
        raise Thrown()
    filling = f * (target - units(t))
    return t + filling if end else filling + t


def number_fn(f):
    def call(args):
        x = to_number(args[0])
        if x is None:
            return None
        r = f(x)
        return r if math.isfinite(r) else None
    return call


def side(args, left):
    v, n = args
    count = to_number(n)
    if v is None or count is None:
        return None
    t = text(v)
    total = units(t)
    k = min(math.trunc(count), total) if count > 0 else 0
    return unit_slice(t, 0, k) if left else unit_slice(t, total - k, total)


def find(args):
    t, s = args
    if t is None or s is None:
        return None
    t, s = text(t), text(s)
    i = t.find(s)
    return 0.0 if i < 0 else float(units(t[:i]) + 1)


def ceil(x):
    """Math.ceil: -0 for -1 < x < 0."""
    r = float(math.ceil(x))
    return -0.0 if r == 0 and x < 0 else r


FUNCTIONS = {
    # The folded name: least and most arguments (None: any), and the function.
    "KOK": (1, 1, number_fn(square_root)),
    "TAVAN": (1, 1, number_fn(ceil)),
    "TABAN": (1, 1, number_fn(lambda x: float(math.floor(x)))),
    "PI": (0, 0, lambda args: math.pi),
    "SOL": (2, 2, lambda args: side(args, True)),
    "SAG": (2, 2, lambda args: side(args, False)),
    "BUL": (2, 2, find),
    "BIRLESTIR": (1, None, lambda args: "".join(text(a) for a in args)),
    "SAGDOLDUR": (2, 3, lambda args: pad(args[0], args[1], args[2] if len(args) > 2 else None, " ", True)),
    "DOLDUR": (2, 3, lambda args: pad(args[0], args[1], args[2] if len(args) > 2 else None, "0", False)),
    "YUVARLA": (1, 2, lambda args: yuvarla(args)),
}
for english, turkish in [("SQRT", "KOK"), ("CEIL", "TAVAN"), ("FLOOR", "TABAN"), ("LEFT", "SOL"),
                         ("RIGHT", "SAG"), ("STRPOS", "BUL"), ("CONCAT", "BIRLESTIR"),
                         ("RPAD", "SAGDOLDUR"), ("LPAD", "DOLDUR"), ("ROUND", "YUVARLA")]:
    FUNCTIONS[english] = FUNCTIONS[turkish]


def yuvarla(args):
    """yuvarla(x, d) for the fixture's π only: the rounded decimal, exactly."""
    x = to_number(args[0])
    d = to_number(args[1]) if len(args) > 1 else 0.0
    if x is None or d is None:
        return None
    return float(round(Fraction(x) * 10 ** int(d)) / Fraction(10) ** int(d))


# ── The grammar ──────────────────────────────────────────────────────────

OPS = ["<>", "<=", ">=", "!=", "==", "||", "=", "<", ">", "+", "-", "*", "/", "%", "^", "(", ")", ","]


def tokenize(src):
    toks = []
    i = 0
    while i < len(src):
        c = src[i]
        if c in SPACE:
            i += 1
            continue
        if c.isascii() and c.isdigit() or (c == "." and i + 1 < len(src) and src[i + 1].isdigit()):
            j = i
            while j < len(src) and src[j].isdigit():
                j += 1
            if j + 1 < len(src) and src[j] == "." and src[j + 1].isdigit():
                j += 1
                while j < len(src) and src[j].isdigit():
                    j += 1
            if j < len(src) and src[j] in "eE":
                k = j + 1
                if k < len(src) and src[k] in "+-":
                    k += 1
                if k < len(src) and src[k].isdigit():
                    while k < len(src) and src[k].isdigit():
                        k += 1
                    j = k
            toks.append(("num", float(src[i:j])))
            i = j
        elif c in "'\"":
            j = i + 1
            out = []
            while True:
                if j >= len(src):
                    raise Invalid("unclosed text")
                if src[j] == c:
                    if j + 1 < len(src) and src[j + 1] == c:
                        out.append(c)
                        j += 2
                        continue
                    break
                out.append(src[j])
                j += 1
            toks.append(("str", "".join(out)))
            i = j + 1
        elif c == "[":
            j = src.index("]", i)
            toks.append(("field", trim(src[i + 1:j])))
            i = j + 1
        elif c.isalpha() or c == "_":
            j = i
            while j < len(src) and (src[j].isalnum() or src[j] == "_"):
                j += 1
            toks.append(("word", src[i:j]))
            i = j
        else:
            op = next((o for o in OPS if src.startswith(o, i)), None)
            if op is None:
                raise Invalid(f"stray {c!r}")
            toks.append(("op", op))
            i += len(op)
    toks.append(("end", None))
    return toks


def key(word):
    return fold(trim(word))


KEYWORDS = {"VE": "and", "AND": "and", "VEYA": "or", "OR": "or", "DEGIL": "not", "NOT": "not",
            "DOGRU": "true", "TRUE": "true", "YANLIS": "false", "FALSE": "false", "BOS": "null", "NULL": "null"}
WORDS = {"DURUM": "case", "CASE": "case", "EGER": "when", "WHEN": "when", "ISE": "then", "THEN": "then",
         "YOKSA": "else", "ELSE": "else", "SON": "end", "END": "end", "ICINDE": "in", "IN": "in",
         "ARASINDA": "between", "BETWEEN": "between", "GIBI": "like", "LIKE": "like",
         "BENZER": "ilike", "ILIKE": "ilike", "IS": "is"}
COMPARISONS = {"=": "eq", "==": "eq", "!=": "ne", "<>": "ne", "<": "lt", "<=": "le", ">": "gt", ">=": "ge"}


class Parser:
    def __init__(self, toks):
        self.toks = toks
        self.i = 0

    def peek(self, k=0):
        return self.toks[min(self.i + k, len(self.toks) - 1)]

    def next(self):
        t = self.toks[self.i]
        self.i += 1
        return t

    def keyword(self, k=0):
        t = self.peek(k)
        return KEYWORDS.get(key(t[1])) if t[0] == "word" else None

    def word(self, k=0):
        t = self.peek(k)
        return WORDS.get(key(t[1])) if t[0] == "word" else None

    def op(self):
        t = self.peek()
        return t[1] if t[0] == "op" else None

    def parse(self):
        e = self.disjunction()
        if self.peek()[0] != "end":
            raise Invalid("tokens left")
        return e

    def disjunction(self):
        a = self.conjunction()
        while self.keyword() == "or":
            self.next()
            a = ("or", a, self.conjunction())
        return a

    def conjunction(self):
        a = self.comparison()
        while self.keyword() == "and":
            self.next()
            a = ("and", a, self.comparison())
        return a

    def comparison(self):
        a = self.additive()
        while True:
            if self.op() in COMPARISONS:
                op = COMPARISONS[self.next()[1]]
                a = (op, a, self.additive())
            elif self.keyword() == "null":
                self.next()
                negated = self.keyword() == "not"
                if negated:
                    self.next()
                a = ("isnull", a, negated)
            elif self.word() == "is":
                self.next()
                negated = self.keyword() == "not"
                if negated:
                    self.next()
                if self.keyword() != "null":
                    raise Invalid("IS without NULL")
                self.next()
                a = ("isnull", a, negated)
            elif self.word() in ("in", "between", "like", "ilike") or (
                    self.keyword() == "not" and self.word(1) in ("in", "between", "like", "ilike")):
                negated = self.keyword() == "not"
                if negated:
                    self.next()
                w = self.word()
                self.next()
                if w == "in":
                    if self.op() != "(" or self.peek(1) == ("op", ")"):
                        raise Invalid("içinde without a list")
                    self.next()
                    items = [self.disjunction()]
                    while self.op() == ",":
                        self.next()
                        items.append(self.disjunction())
                    if self.op() != ")":
                        raise Invalid("list not closed")
                    self.next()
                    a = ("in", a, items, negated)
                elif w == "between":
                    low = self.additive()
                    if self.keyword() != "and":
                        raise Invalid("arasında without ve")
                    self.next()
                    a = ("between", a, low, self.additive(), negated)
                else:
                    a = ("like", a, self.additive(), w == "ilike", negated)
            else:
                return a

    def additive(self):
        a = self.multiplicative()
        while self.op() in ("+", "-", "||"):
            op = {"+": "add", "-": "sub", "||": "join"}[self.next()[1]]
            a = (op, a, self.multiplicative())
        return a

    def multiplicative(self):
        a = self.unary()
        while self.op() in ("*", "/", "%"):
            op = {"*": "mul", "/": "div", "%": "rem"}[self.next()[1]]
            a = (op, a, self.unary())
        return a

    def unary(self):
        if self.keyword() == "not":
            self.next()
            return ("not", self.unary())
        if self.op() in ("-", "+"):
            sign = self.next()[1]
            a = self.unary()
            return ("neg", a) if sign == "-" else ("pos", a)
        base = self.primary()
        if self.op() == "^":
            self.next()
            return ("pow", base, self.unary())
        return base

    def primary(self):
        t = self.next()
        if t[0] == "num":
            return ("lit", t[1])
        if t[0] == "str":
            return ("lit", t[1])
        if t[0] == "field":
            return ("field", t[1])
        if t[0] == "op" and t[1] == "(":
            e = self.disjunction()
            if self.op() != ")":
                raise Invalid("( not closed")
            self.next()
            return e
        if t[0] == "word":
            if self.op() == "(":
                return self.call(t[1])
            if WORDS.get(key(t[1])) == "case" and self.word() == "when":
                return self.case()
            k = KEYWORDS.get(key(t[1]))
            if k in ("true", "false"):
                return ("lit", k == "true")
            if k == "null":
                return ("lit", None)
            if k is not None:
                raise Invalid(f"keyword {t[1]} where a value stands")
            return ("field", t[1])
        raise Invalid(f"unexpected {t}")

    def call(self, name):
        f = FUNCTIONS.get(key(name))
        if f is None:
            raise Invalid(f"unknown function {name}")
        self.next()
        args = []
        if self.op() != ")":
            args.append(self.disjunction())
            while self.op() == ",":
                self.next()
                args.append(self.disjunction())
        if self.op() != ")":
            raise Invalid("call not closed")
        self.next()
        least, most, _ = f
        if len(args) < least or (most is not None and len(args) > most):
            raise Invalid(f"{name} takes {least}…{most}")
        return ("call", key(name), args)

    def case(self):
        whens = []
        while self.word() == "when":
            self.next()
            c = self.disjunction()
            if self.word() != "then":
                raise Invalid("ise expected")
            self.next()
            whens.append((c, self.disjunction()))
        otherwise = None
        if self.word() == "else":
            self.next()
            otherwise = self.disjunction()
        if self.word() != "end":
            raise Invalid("son expected")
        self.next()
        return ("case", whens, otherwise)


# ── Evaluation ───────────────────────────────────────────────────────────

def ev(n, obj):
    kind = n[0]
    if kind == "lit":
        return n[1]
    if kind == "field":
        return obj.get(n[1])
    if kind == "case":
        for c, v in n[1]:
            if truthy(ev(c, obj)):
                return ev(v, obj)
        return ev(n[2], obj) if n[2] is not None else None
    if kind == "in":
        x = ev(n[1], obj)
        items = [ev(i, obj) for i in n[2]]
        return any(equals(x, i) for i in items) != n[3]
    if kind == "between":
        x, low, high = ev(n[1], obj), ev(n[2], obj), ev(n[3], obj)
        lo, hi = compare(x, low), compare(x, high)
        inside = lo is not None and lo >= 0 and hi is not None and hi <= 0
        return inside != n[4]
    if kind == "like":
        return like(ev(n[1], obj), ev(n[2], obj), n[3]) != n[4]
    if kind == "isnull":
        return is_empty(ev(n[1], obj)) != n[2]
    if kind == "not":
        return not truthy(ev(n[1], obj))
    if kind == "pos":
        return ev(n[1], obj)
    if kind == "neg":
        x = to_number(ev(n[1], obj))
        return None if x is None else -x
    if kind == "call":
        _, _, f = FUNCTIONS[n[1]]
        return f([ev(a, obj) for a in n[2]])
    a, b = ev(n[1], obj), ev(n[2], obj)
    if kind == "or":
        return truthy(a) or truthy(b)
    if kind == "and":
        return truthy(a) and truthy(b)
    if kind == "eq":
        return equals(a, b)
    if kind == "ne":
        return not equals(a, b)
    if kind in ("lt", "le", "gt", "ge"):
        c = compare(a, b)
        if c is None:
            return False
        return {"lt": c < 0, "le": c <= 0, "gt": c > 0, "ge": c >= 0}[kind]
    if kind == "join":
        return text(a) + text(b)
    na, nb = to_number(a), to_number(b)
    if kind == "add":
        if a is None or b is None:
            return None
        if na is not None and nb is not None:
            return na + nb
        return text(a) + text(b)
    if na is None or nb is None:
        return None
    if kind in ("div", "rem") and nb == 0:
        return None
    if kind == "sub":
        return na - nb
    if kind == "mul":
        return na * nb
    if kind == "div":
        return na / nb
    if kind == "rem":
        return math.fmod(na, nb)
    if kind == "pow":
        return power(na, nb)
    raise AssertionError(kind)


def value(n, obj):
    try:
        v = ev(n, obj)
    except Thrown:
        return None
    if isinstance(v, float) and not math.isfinite(v):
        return None
    return v


def encode(v):
    if v is None:
        return None
    if isinstance(v, bool):
        return ["b", v]
    if isinstance(v, float):
        return ["n", "-0" if v == 0 and math.copysign(1, v) < 0 else v]
    return ["t", v]


def build():
    cases = []
    for src in SOURCES:
        try:
            tree = Parser(tokenize(src)).parse()
        except Invalid:
            cases.append({"source": src, "compiles": False})
            continue
        NEARLY.clear()
        case = {"source": src, "values": [encode(value(tree, o)) for o in OBJECTS]}
        if NEARLY:
            case["ulp"] = 1
        cases.append(case)
    return {
        "format": "kentos.expression-language",
        "version": 1,
        "note": "Dilin §4 ekleri (durum … son, içinde, arasında, gibi, benzer, boş / boş değil, ^, yeni işlevler), "
                "motordan bağımsız hesaplanmış: her kaynağın her nesnedeki değeri ya da derlenmediği. "
                "Değer: null boş, [\"n\", x] sayı (\"-0\" eksi sıfır), [\"t\", s] metin, [\"b\", b] doğru/yanlış. "
                "\"ulp\": 1 olan durumda sayı tam olmayan üslü bir kuvvetten gelir: motorunki (libm) doğru yuvarlanmış "
                "değerin en çok bir son basamak birimi yanındadır. "
                "Üretici scripts/fixtures/expression_language.py (--check yalnız karşılaştırır).",
        "objects": OBJECTS,
        "cases": cases,
    }


def main():
    doc = build()
    out = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != out:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: {len(doc['cases'])} durum, {len(OBJECTS)} nesne; güncel.")
        return
    OUT.write_text(out, encoding="utf-8")
    print(f"{OUT} yazıldı: {len(doc['cases'])} durum.")


if __name__ == "__main__":
    main()
