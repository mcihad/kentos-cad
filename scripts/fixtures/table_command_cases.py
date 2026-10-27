#!/usr/bin/env python3
"""Writes the shared cases of `cad.tables.edit` v1 (docs/adr/0327): fixtures/commands/v1/cad.tables.edit.json.

The rules are the ADR's, written here again without KentOS code (the table's own rules, ADR 0184, from
table_cases.py, which states them on its own; a file's rows from table_file_cases.py's reader):

- a cell's words beginning with `=` are its formula; its words are its result: A1 references (`$` aside), A1:C3
  ranges, + − × ÷ ^ (from the left; a sign binds before ^), parentheses, SUM TOPLA, AVERAGE ORTALAMA, MIN, MAX MAK,
  COUNT SAY, ROUND YUVARLA, ABS MUTLAK (arguments apart by `,` or `;`). A cell's number is its trimmed words read as
  `[+-]digits[.digits][e[+-]digits]`; empty is nothing, other words are words. Arithmetic takes nothing as 0 and
  refuses words (`#DEĞER!`); the functions take a range's and a reference's numbers, skipping nothing and words, an
  average of none is `#SAYI/0!`, MIN and MAX of none 0. ÷ 0 is `#SAYI/0!`, no finite result `#SAYI!`, a reference
  outside the table or deleted `#BAŞV!`, what does not read `#AD?`, formulas reading each other in a ring `#DÖNGÜ!`.
  A result's words: its shortest decimal form without exponent (Python's repr, positional) rounded half away from zero
  to 15 significant digits, trailing zeros dropped (as spreadsheets show 0.1 + 0.2 as 0.3), with a cell style's
  decimals rounded half away from zero from that form; nothing is 0;
- rows or columns inserted or deleted move styles (as merged ranges), formulas and fields; a formula's references
  move, a deleted cell's is `#BAŞV!`, a range keeps what is left of it; merging takes the range's formulas and fields
  away; a cell inside a merged range is its top left's;
- a style over a range sets what it gives, a later over an earlier; bold widens what Sığdır asks;
- a field's words are its source's value: an object's area by the project's area unit and decimals; a block's count of
  inserts; written again at every step;
- a block schedule: Blok, the attribute when given, Adet; a row for each block placed (definitions' order) and each
  attribute value (code point order), then Toplam;
- Tabloyu güncelle: a linked file read from its path (from the drawing's folder when relative): the same SHA-256
  leaves it; a cell that differs from the file's copy last read (not a formula's or a field's) was changed by hand and
  is kept while the file's cell there is as it was (`keep`), else the file's is written; the file's rows then as ADR
  0184 §6 says; its digest and copy the new ones.

The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/table_command_cases.py           # writes the file
    python3 scripts/fixtures/table_command_cases.py --check   # compares it with the one on disk
"""
import copy
import hashlib
import json
import re
import sys
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import table_cases as tc  # noqa: E402  (the table's rules, ADR 0184)
import table_file_cases as tf  # noqa: E402  (a file's sheets as a table reads them)
import paragraph_cases as paragraph  # noqa: E402  (the typefaces' measured advances)
from mpmath import mpf  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.tables.edit.json"
FILES = ROOT / "fixtures/commands/v1/tables"
X, Y = 500000, 4400000
FAKE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3fff"
NOT_UID = "T-1"
DOOR = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3d01"
WINDOW = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3d02"
FONT = "barlow"
H = 0.25

M_UID = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
M_NOT_FOUND = "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
M_NOT_TABLE = "Verilen nesne bir tablo değil; bu komut yalnız tabloları değiştirir."
M_LOCKED = "“Kilitli” katmanı kilitli; üzerindeki tablo değişmez. Kilidi Katmanlar panelinden açın."
M_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
M_CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."

# ── Formulas ───────────────────────────────────────────────────────────────

ERR = {"name": "#AD?", "ref": "#BAŞV!", "value": "#DEĞER!", "div": "#SAYI/0!", "num": "#SAYI!", "ring": "#DÖNGÜ!"}
NUMBER = re.compile(r"^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$")
REF = re.compile(r"\$?([A-Za-z]{1,2})\$?(\d+)")
FUNCS = {"SUM": "sum", "TOPLA": "sum", "AVERAGE": "avg", "ORTALAMA": "avg", "MIN": "min", "MAX": "max", "MAK": "max",
         "COUNT": "count", "SAY": "count", "ROUND": "round", "YUVARLA": "round", "ABS": "abs", "MUTLAK": "abs"}


class Err(Exception):
    def __init__(self, code):
        self.code = code


def col_index(letters):
    j = 0
    for ch in letters.upper():
        j = j * 26 + (ord(ch) - ord("A") + 1)
    return j - 1


def col_letters(j):
    out = ""
    while True:
        out = chr(ord("A") + j % 26) + out
        if j < 26:
            return out
        j = j // 26 - 1


def number_of(words):
    s = words.strip()
    if not NUMBER.match(s):
        return None
    return float(s)


def shortest(x):
    if x == 0:
        return "0"
    d = Decimal(repr(x))
    text = format(d, "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    return text


def general(x):
    """A result's words: its shortest form rounded half away from zero to 15 significant digits, as spreadsheets show."""
    text = shortest(x)
    d = Decimal(text)
    significant = "".join(map(str, d.as_tuple().digits)).strip("0")
    if len(significant) <= 15:
        return text
    q = d.quantize(Decimal(1).scaleb(d.adjusted() - 14), rounding=ROUND_HALF_UP)
    out = format(q, "f")
    if "." in out:
        out = out.rstrip("0").rstrip(".")
    return out


def rounded(x, d):
    q = Decimal(general(x)).quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    text = format(q, "f")
    if text.startswith("-") and all(c in "0." for c in text[1:]):
        text = text[1:]
    return text


def fold(name):
    return "".join("I" if c in "iıİ" else c.upper() for c in name)


def tokens(src):
    out, i = [], 0
    while i < len(src):
        c = src[i]
        if c.isspace():
            i += 1
        elif src.startswith(ERR["ref"], i):
            out.append(("bad",))
            i += len(ERR["ref"])
        elif c in "+-*/^":
            out.append(("op", c))
            i += 1
        elif c in "(),;":
            out.append({"(": ("open",), ")": ("close",), ",": ("sep",), ";": ("sep",)}[c])
            i += 1
        elif c.isdigit() or c == ".":
            m = re.match(r"[\d.]+([eE][+-]?\d+)?", src[i:])
            x = number_of(m.group(0))
            if x is None:
                raise Err("name")
            out.append(("num", x))
            i += len(m.group(0))
        elif c == "$" or c.isalpha():
            m = re.match(r"\$?[A-Za-z]{1,2}\$?\d+(?![A-Za-z0-9_])", src[i:])
            if m and c.isascii():
                one = REF.fullmatch(m.group(0))
                r1, c1 = int(one.group(2)) - 1, col_index(one.group(1))
                if int(one.group(2)) == 0:
                    raise Err("name")
                i += len(m.group(0))
                rest = src[i:].lstrip()
                if rest.startswith(":"):
                    rest2 = rest[1:].lstrip()
                    m2 = re.match(r"\$?[A-Za-z]{1,2}\$?\d+(?![A-Za-z0-9_])", rest2)
                    if not m2:
                        raise Err("name")
                    two = REF.fullmatch(m2.group(0))
                    r2, c2 = int(two.group(2)) - 1, col_index(two.group(1))
                    out.append(("range", min(r1, r2), min(c1, c2), max(r1, r2), max(c1, c2)))
                    i = len(src) - len(rest2) + len(m2.group(0))
                else:
                    out.append(("ref", r1, c1))
            else:
                m = re.match(r"[^\W]+", src[i:])
                if not m:
                    raise Err("name")
                out.append(("name", fold(m.group(0))))
                i += len(m.group(0))
        else:
            raise Err("name")
    return out


def parse(src):
    toks, at = tokens(src), [0]

    def peek():
        return toks[at[0]] if at[0] < len(toks) else None

    def take():
        t = peek()
        at[0] += 1
        if t is None:
            raise Err("name")
        return t

    def summ():
        e = prod()
        while peek() and peek()[0] == "op" and peek()[1] in "+-":
            op = take()[1]
            e = ("bin", op, e, prod())
        return e

    def prod():
        e = power()
        while peek() and peek()[0] == "op" and peek()[1] in "*/":
            op = take()[1]
            e = ("bin", op, e, power())
        return e

    def power():
        e = unary()
        while peek() == ("op", "^"):
            take()
            e = ("bin", "^", e, unary())
        return e

    def unary():
        if peek() == ("op", "-"):
            take()
            return ("neg", unary())
        if peek() == ("op", "+"):
            take()
            return unary()
        return primary()

    def primary():
        t = take()
        if t[0] in ("num", "ref", "bad"):
            return t
        if t[0] == "open":
            e = summ()
            if take() != ("close",):
                raise Err("name")
            return e
        if t[0] == "name":
            f = FUNCS.get(t[1])
            if f is None or take() != ("open",):
                raise Err("name")
            args = []
            if peek() == ("close",):
                take()
            else:
                while True:
                    nxt = toks[at[0] + 1] if at[0] + 1 < len(toks) else None
                    if peek() and peek()[0] == "range" and nxt in (("sep",), ("close",)):
                        args.append(take())
                    else:
                        args.append(summ())
                    s = take()
                    if s == ("sep",):
                        continue
                    if s == ("close",):
                        break
                    raise Err("name")
            n = len(args)
            if (f == "round" and n != 2) or (f == "abs" and n != 1) or n == 0:
                raise Err("name")
            return ("call", f, args)
        raise Err("name")

    e = summ()
    if at[0] != len(toks):
        raise Err("name")
    return e


def reads(e):
    if e[0] == "ref":
        yield (e[1], e[2])
    elif e[0] == "range":
        for r in range(e[1], e[3] + 1):
            for c in range(e[2], e[4] + 1):
                yield (r, c)
    elif e[0] == "neg":
        yield from reads(e[1])
    elif e[0] == "bin":
        yield from reads(e[2])
        yield from reads(e[3])
    elif e[0] == "call":
        for a in e[2]:
            yield from reads(a)


def work_out(cells, formulas, decimals):
    """Every formula cell's words: (row, col) → words. Values: ("empty",), ("num", x), ("text", s), ("err", code)."""
    exprs = {}
    for (r, c), f in formulas.items():
        try:
            exprs[(r, c)] = parse(f[1:])
        except Err:
            exprs[(r, c)] = None
    # A cell on a ring: it reaches itself through formulas.
    def reaches(start):
        seen, todo = set(), [start]
        while todo:
            k = todo.pop()
            e = exprs.get(k)
            for d in (reads(e) if e else []):
                if d == start:
                    return True
                if d in exprs and d not in seen:
                    seen.add(d)
                    todo.append(d)
        return False

    ringed = {k for k in exprs if reaches(k)}
    done = {}

    def value(r, c, stack):
        if r >= len(cells) or c >= len(cells[0]):
            return ("err", "ref")
        if (r, c) in exprs:
            return formula_value((r, c), stack)
        w = cells[r][c]
        if w.strip() == "":
            return ("empty",)
        x = number_of(w)
        return ("num", x) if x is not None else ("text", w)

    def formula_value(k, stack):
        if k in done:
            return done[k]
        if k in ringed:
            v = ("err", "ring")
        elif exprs[k] is None:
            v = ("err", "name")
        else:
            v = ev(exprs[k], stack | {k})
        done[k] = v
        return v

    def arith(v):
        if v[0] == "empty":
            return 0.0
        if v[0] == "num":
            return v[1]
        if v[0] == "text":
            raise Err("value")
        raise Err(v[1])

    def ev(e, stack):
        try:
            return ev_raw(e, stack)
        except Err as x:
            return ("err", x.code)

    def ev_raw(e, stack):
        k = e[0]
        if k == "num":
            return ("num", e[1])
        if k == "bad":
            return ("err", "ref")
        if k == "ref":
            return value(e[1], e[2], stack)
        if k == "range":
            raise Err("value")
        if k == "neg":
            return ("num", -arith(ev(e[1], stack)))
        if k == "bin":
            a, b = arith(ev(e[2], stack)), arith(ev(e[3], stack))
            op = e[1]
            if op == "/" and b == 0:
                raise Err("div")
            try:
                z = {"+": lambda: a + b, "-": lambda: a - b, "*": lambda: a * b, "/": lambda: a / b, "^": lambda: a ** b}[op]()
            except (OverflowError, ZeroDivisionError, ValueError):
                raise Err("num")
            if isinstance(z, complex) or z != z or z in (float("inf"), float("-inf")):
                raise Err("num")
            return ("num", float(z))
        f, args = e[1], e[2]
        if f == "round":
            x, n = arith(ev(args[0], stack)), arith(ev(args[1], stack))
            n = int(n)
            if abs(n) > 10:
                raise Err("num")
            return ("num", float(rounded(x, n)))
        if f == "abs":
            return ("num", abs(arith(ev(args[0], stack))))
        xs = []
        for a in args:
            if a[0] in ("range", "ref"):
                for (r, c) in reads(a):
                    v = value(r, c, stack)
                    if v[0] == "num":
                        xs.append(v[1])
                    elif v[0] == "err":
                        raise Err(v[1])
            else:
                xs.append(arith(ev(a, stack)))
        if f == "sum":
            return ("num", sum(xs))
        if f == "count":
            return ("num", float(len(xs)))
        if f == "avg":
            if not xs:
                raise Err("div")
            return ("num", sum(xs) / len(xs))
        if not xs:
            return ("num", 0.0)
        return ("num", min(xs) if f == "min" else max(xs))

    out = {}
    for k in sorted(exprs):
        v = formula_value(k, frozenset())
        d = decimals(*k)
        if v[0] == "empty":
            words = rounded(0.0, d) if d is not None else "0"
        elif v[0] == "num":
            words = rounded(v[1], d) if d is not None else general(v[1])
        elif v[0] == "text":
            words = v[1]
        else:
            words = ERR[v[1]]
        out[k] = words
    return out


def moved(i, at, count):
    if count >= 0:
        return i + count if i >= at else i
    n = -count
    if i < at:
        return i
    if i < at + n:
        return None
    return i - n


def range_moved(a, b, at, count):
    if count >= 0:
        return moved(a, at, count), moved(b, at, count)
    n = -count
    if a >= at and b < at + n:
        return None
    a2 = a if a < at else (at if a < at + n else a - n)
    b2 = b if b < at else (at - 1 if b < at + n else b - n)
    return (a2, b2) if a2 <= b2 else None


def shift(formula, axis, at, count):
    out, i = "", 0
    while i < len(formula):
        c = formula[i]
        if c == "$" or (c.isascii() and c.isalpha()):
            m = re.match(r"[A-Za-z0-9$_]+", formula[i:])
            word = m.group(0)
            one = re.fullmatch(r"(\$?)([A-Za-z]{1,2})(\$?)(\d+)", word)
            if one and int(one.group(4)) > 0:
                rest = formula[i + len(word):]
                m2 = re.match(r"\s*:\s*((\$?)([A-Za-z]{1,2})(\$?)(\d+))(?![A-Za-z0-9_])", rest)
                r1, c1 = int(one.group(4)) - 1, col_index(one.group(2))
                if m2:
                    r2, c2 = int(m2.group(5)) - 1, col_index(m2.group(3))
                    r0, c0, r3, c3 = min(r1, r2), min(c1, c2), max(r1, r2), max(c1, c2)
                    if axis == "rows":
                        mv = range_moved(r0, r3, at, count)
                        box = None if mv is None else (mv[0], c0, mv[1], c3)
                    else:
                        mv = range_moved(c0, c3, at, count)
                        box = None if mv is None else (r0, mv[0], r3, mv[1])
                    if box is None:
                        out += ERR["ref"]
                    else:
                        out += f"{one.group(1)}{col_letters(box[1])}{one.group(3)}{box[0] + 1}:{m2.group(2)}{col_letters(box[3])}{m2.group(4)}{box[2] + 1}"
                    i += len(word) + len(m2.group(0))
                    continue
                if axis == "rows":
                    nr = moved(r1, at, count)
                    place = None if nr is None else (nr, c1)
                else:
                    nc = moved(c1, at, count)
                    place = None if nc is None else (r1, nc)
                out += ERR["ref"] if place is None else f"{one.group(1)}{col_letters(place[1])}{one.group(3)}{place[0] + 1}"
                i += len(word)
                continue
            out += word
            i += len(word)
            continue
        out += c
        i += 1
    return out


# ── The table ──────────────────────────────────────────────────────────────


def pt(x, y):
    return {"x": X + x, "y": Y + y}


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def table(i, cells, layer_id="tablo", header=True, aligns=None, source=None, columns=None, **extra):
    n, m = len(cells), len(cells[0])
    t = {"kind": "table", "id": i, "layerId": layer_id, "attrs": {}, "p": pt(0, 20 * i), "rotation": 0, "height": H,
         "rows": [0.5] * n, "columns": columns or [2.5] + [1.5] * (m - 1), "cells": cells}
    if header:
        t["header"] = True
    if aligns:
        t["aligns"] = aligns
    if source:
        t["source"] = source
    t.update(extra)
    return t


def style_at(t, r, c):
    out = {}
    for s in t.get("styles", []):
        if s["row"] <= r < s["row"] + s["rows"] and s["col"] <= c < s["col"] + s["cols"]:
            for k in ("align", "bold", "color", "fill", "decimals"):
                if k in s:
                    out[k] = s[k]
    return out


def sizes(t):
    """Sığdır's widths: ADR 0184's, a cell style's bold over the heading's and the face's."""
    h = mpf(t["height"])
    widths = [tc.LEAST * h] * len(t["columns"])
    for i, row in enumerate(t["cells"]):
        for j, words in enumerate(row):
            if not words or any(tc.holds(g, i, j) for g in t.get("merges", [])):
                continue
            bold = style_at(t, i, j).get("bold", bool(t.get("header")) and i == 0)
            w = tc.words_width(words, FONT, bold, h) + 2 * tc.PAD * h
            widths[j] = max(widths[j], w)
    return [float(w) for w in widths]


def merge_at(t, r, c):
    for g in t.get("merges", []):
        if tc.holds(g, r, c):
            return g
    return None


def tidy(t):
    for k in ("merges", "styles", "formulas", "fields"):
        if k in t and not t[k]:
            del t[k]
    if t.get("aligns") is None:
        t.pop("aligns", None)
    return t


def core(t, e):
    """The table's own edit (ADR 0184 §6, table_cases.py), its other fields kept."""
    r = tc.edit({**t, "merges": t.get("merges", []), "aligns": t.get("aligns")}, e)
    if "problem" in r:
        raise ValueError(r["problem"])
    out = dict(t)
    out.update({"rows": r["rows"], "columns": r["columns"], "cells": r["cells"], "merges": r["merges"]})
    out["aligns"] = r["aligns"]
    return out


def moved_extras(t, axis, at, count):
    styles = []
    for s in t.get("styles", []):
        lo, ln = ("row", "rows") if axis == "rows" else ("col", "cols")
        if count >= 0:
            a, n = tc.grown(s[lo], s[ln], at, count)
        else:
            a, n = tc.shrunk(s[lo], s[ln], at, -count)
        if n > 0:
            styles.append({**s, lo: a, ln: n})
    t["styles"] = styles

    def place(r, c):
        if axis == "rows":
            nr = moved(r, at, count)
            return None if nr is None else (nr, c)
        nc = moved(c, at, count)
        return None if nc is None else (r, nc)

    fs = []
    for f in t.get("formulas", []):
        p = place(f["row"], f["col"])
        if p:
            fs.append({"row": p[0], "col": p[1], "formula": shift(f["formula"], axis, at, count)})
    t["formulas"] = fs
    ds = []
    for f in t.get("fields", []):
        p = place(f["row"], f["col"])
        if p:
            ds.append({**f, "row": p[0], "col": p[1]})
    t["fields"] = ds


def edited(t, e):
    t = copy.deepcopy(t)
    k = e["kind"]
    if k == "setCell":
        g = merge_at(t, e["row"], e["col"])
        r, c = (g["row"], g["col"]) if g else (e["row"], e["col"])
        t["formulas"] = [f for f in t.get("formulas", []) if (f["row"], f["col"]) != (r, c)]
        t["fields"] = [f for f in t.get("fields", []) if (f["row"], f["col"]) != (r, c)]
        if e["words"].startswith("=") and len(e["words"]) > 1:
            t["formulas"] = sorted(t["formulas"] + [{"row": r, "col": c, "formula": e["words"]}], key=lambda f: (f["row"], f["col"]))
        return core(t, {"kind": "setCell", "row": r, "col": c, "words": e["words"]})
    if k in ("insertRows", "insertColumns"):
        out = core(t, e)
        moved_extras(out, "rows" if k == "insertRows" else "columns", e["at"], e["count"])
        return out
    if k in ("deleteRows", "deleteColumns"):
        out = core(t, e)
        moved_extras(out, "rows" if k == "deleteRows" else "columns", e["from"], -e["count"])
        return out
    if k == "merge":
        out = core(t, e)
        inside = lambda f: e["row"] <= f["row"] < e["row"] + e["rows"] and e["col"] <= f["col"] < e["col"] + e["cols"]  # noqa: E731
        out["formulas"] = [f for f in out.get("formulas", []) if not inside(f)]
        out["fields"] = [f for f in out.get("fields", []) if not inside(f)]
        return out
    if k == "unmerge":
        return core(t, e)
    if k == "style":
        s = {key: e[key] for key in ("row", "col", "rows", "cols")}
        s.update({key: e[key] for key in ("align", "bold", "color", "fill", "decimals") if key in e})
        t.setdefault("styles", []).append(s)
        return t
    if k == "clearStyle":
        def meets(s):
            return s["row"] < e["row"] + e["rows"] and e["row"] < s["row"] + s["rows"] and s["col"] < e["col"] + e["cols"] and e["col"] < s["col"] + s["cols"]
        t["styles"] = [s for s in t.get("styles", []) if not meets(s)]
        return t
    if k == "field":
        rc = (e["row"], e["col"])
        t["formulas"] = [f for f in t.get("formulas", []) if (f["row"], f["col"]) != rc]
        t["fields"] = sorted([f for f in t.get("fields", []) if (f["row"], f["col"]) != rc]
                             + [{"row": e["row"], "col": e["col"], "source": e["source"], **({"format": e["format"]} if "format" in e else {})}],
                             key=lambda f: (f["row"], f["col"]))
        return t
    if k == "unfield":
        t["fields"] = [f for f in t.get("fields", []) if (f["row"], f["col"]) != (e["row"], e["col"])]
        return t
    if k == "rowHeight":
        for i in range(e["from"], e["from"] + e["count"]):
            t["rows"][i] = e["height"]
        return t
    if k == "columnWidth":
        for j in range(e["from"], e["from"] + e["count"]):
            t["columns"][j] = e["width"]
        return t
    if k == "fit":
        widths = sizes(t)
        for j in range(e["from"], e["from"] + e["count"]):
            t["columns"][j] = max(t["columns"][j], widths[j]) if e.get("grow") else widths[j]
        if e.get("rows"):
            t["rows"] = [float(tc.ROW * mpf(t["height"]))] * len(t["rows"])
        return t
    if k == "columnAlign":
        m = len(t["columns"])
        aligns = list(t.get("aligns") or ["left"] * m)
        for j in range(e["from"], e["from"] + e["count"]):
            aligns[j] = e["align"]
        t["aligns"] = None if all(a == "left" for a in aligns) else aligns
        return t
    if k == "header":
        t["header"] = e["on"]
        if not e["on"]:
            t.pop("header", None)
        return t
    if k == "grid":
        if e["grid"] is None:
            t.pop("grid", None)
        else:
            t["grid"] = e["grid"]
        return t
    if k == "frame":
        if e["width"] is None:
            t.pop("frame", None)
        else:
            t["frame"] = e["width"]
        return t
    if k == "unlink":
        t.pop("source", None)
        return t
    raise ValueError(k)


# ── The drawing ────────────────────────────────────────────────────────────

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
            "workspace": "cad"}
UNITS = {"axes": "cad", "unit": "m", "areaUnit": "m2", "lengthDecimals": 3, "areaDecimals": 2}

ROOM = [(0, 0), (5, 0), (5, 4), (0, 4)]  # 20 m²
ROOMS_OLD = [["Oda", "Alan", "Kat"], ["Salon", "24.5", "1"], ["Mutfak", "12.25", "1"]]


def file_bytes(name):
    return (FILES / name).read_bytes()


def digest(name):
    return hashlib.sha256(file_bytes(name)).hexdigest()


def door(i, x, kind_, block=DOOR):
    return {"kind": "insert", "id": i, "layerId": "kapi", "attrs": {"TUR": kind_} if kind_ else {}, "block": block,
            "p": pt(x, -10), "scale": 1, "rotation": 0}


def blocks_schedule(entities, blocks, names, by, columns=()):
    """ADR 0327 §4's rows; ADR 0721 §4's further columns: a row's inserts' distinct values that are not empty, in the
    drawing's order, joined with “; ”; with them a schedule of one block named leaves out its Blok column."""
    defs = [(b["id"], b["name"]) for b in blocks] if not names else [
        (next((b["id"] for b in blocks if b["name"] == n), None), n) for n in names]
    named = not (columns and len(names) == 1)
    head = (["Blok"] if named else []) + ([by] if by else []) + list(columns) + ["Adet"]

    def row(name, value, ins):
        r = ([name] if named else []) + ([value] if value is not None else [])
        for c in columns:
            seen = []
            for e in ins:
                v = e["attrs"].get(c, "")
                if v and v not in seen:
                    seen.append(v)
            r.append("; ".join(seen))
        return r + [str(len(ins))]

    rows, total = [head], 0
    for bid, name in defs:
        ins = [e for e in entities if e["kind"] == "insert" and bid is not None and e["block"] == bid]
        if not names and not ins:
            continue
        if by is None:
            total += len(ins)
            rows.append(row(name, None, ins))
        else:
            values = {}
            for e in ins:
                values.setdefault(e["attrs"].get(by, ""), []).append(e)
            if not values:
                rows.append(row(name, "", []))
            for v in sorted(values):
                total += len(values[v])
                rows.append(row(name, v, values[v]))
    blanks = (1 if named else 0) + (1 if by else 0) + len(columns)
    rows.append(["Toplam"] + [""] * max(blanks - 1, 0) + [str(total)])
    return rows


BLOCKS = [
    {"id": DOOR, "name": "Kapı", "base": {"x": 0, "y": 0},
     "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 0.9, "y": 0}}]},
    {"id": WINDOW, "name": "Pencere", "base": {"x": 0, "y": 0},
     "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 1.2, "y": 0}}]},
]


def drawing():
    rooms = table(1, [["Oda", "Alan", "Kat"], ["Salon", "24.5", "1"], ["Mutfak", "12.25", "1"], ["Toplam", "", ""]],
                  aligns=["left", "right", "right"])
    locked = table(2, [["A", "B"], ["1", "2"]], layer_id="kilitli")
    area = {"kind": "polygon", "id": 3, "layerId": "plan", "attrs": {}, "label": "Salon", "pts": [pt(*p) for p in ROOM]}
    line = {"kind": "line", "id": 4, "layerId": "plan", "attrs": {}, "a": pt(10, 0), "b": pt(14, 0)}
    doors = [door(5, 0, "İç"), door(6, 2, "İç"), door(7, 4, "Dış"), door(8, 6, None, WINDOW)]
    ents = [rooms, locked, area, line] + doors
    sched = table(9, blocks_schedule(ents, BLOCKS, [], "TUR"), source={"kind": "blocks", "by": "TUR"}, columns=[2.5, 1.5, 1.5])
    linked = table(10, [["Oda", "Alan", "Kat"], ["Salon (büyük)", "25", "1"], ["Mutfak", "12.5", "1"]],
                   aligns=["left", "right", "right"],
                   source={"kind": "file", "name": "odalar.csv", "path": "odalar.csv",
                           "digest": "0" * 64, "copy": ROOMS_OLD})
    missing = table(11, [["A"], ["1"]], source={"kind": "file", "name": "yok.csv", "path": "yok.csv"})
    unlinked = table(12, [["A"], ["1"]], source={"kind": "file", "name": "eski.csv"})
    areas = table(13, [["Ad"], ["x"]], header=False)
    same = table(14, ROOMS_NEW, aligns=["left", "right", "right"],
                 source={"kind": "file", "name": "odalar.csv", "path": "odalar.csv", "digest": digest("odalar.csv"), "copy": ROOMS_NEW})
    plain = table(15, [["Ad"], ["x"]], header=False)
    replace = copy.deepcopy(linked)
    replace["id"] = 16
    replace["p"] = pt(0, 320)
    replace["source"]["overrides"] = "replace"
    return ents + [sched, linked, missing, unlinked, areas, same, plain, replace]


ROOMS_NEW = [["Oda", "Alan", "Kat"], ["Salon", "26.5", "1"], ["Mutfak", "12.25", "1"], ["Banyo", "6.0", "1"]]


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Tablolar", "settings": SETTINGS,
        "origin": {"x": X, "y": Y},
        "layers": [layer("plan", "Plan"), layer("tablo", "Tablolar"), layer("kapi", "Kapılar"), layer("kilitli", "Kilitli", True)],
        "activeLayer": "tablo", "entities": drawing(), "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


def ent(i):
    return copy.deepcopy(next(e for e in drawing() if e["id"] == i))


def worked(t, ents=None):
    """The table's computed cells: its block schedule, its fields, then its formulas (ADR 0327 §2–§4)."""
    t = copy.deepcopy(t)
    ents = ents if ents is not None else drawing()
    src = t.get("source") or {}
    if src.get("kind") == "blocks":
        rows = blocks_schedule(ents, BLOCKS, src.get("names", []), src.get("by"), src.get("columns", []))
        n, m = len(rows), max(len(r) for r in rows)
        rows = [r + [""] * (m - len(r)) for r in rows]
        h = mpf(t["height"])
        t["rows"] = [t["rows"][i] if i < len(t["rows"]) else float(tc.ROW * h) for i in range(n)]
        old = t["columns"]
        t["cells"] = rows
        t["columns"] = [0.0] * m
        fitted = sizes(t)
        t["columns"] = [max(old[j], fitted[j]) if j < len(old) else fitted[j] for j in range(m)]
    for f in t.get("fields", []):
        s = f["source"]
        if s["kind"] == "object" and s["value"] == "area":
            words = "20.00"
        elif s["kind"] == "project" and s["value"] == "blockCount":
            bid = next(b["id"] for b in BLOCKS if b["name"] == s["name"])
            words = str(sum(1 for e in ents if e["kind"] == "insert" and e["block"] == bid))
        else:
            raise ValueError(s)
        t["cells"][f["row"]][f["col"]] = words
    if t.get("formulas"):
        res = work_out(t["cells"], {(f["row"], f["col"]): f["formula"] for f in t["formulas"]},
                       lambda r, c: style_at(t, r, c).get("decimals"))
        for (r, c), w in res.items():
            t["cells"][r][c] = w
    return tidy(t)


def after(i, edits):
    t = ent(i)
    for e in edits:
        t = edited(t, e)
    return worked(t)


def ok(changed, report=None):
    out = {"changed": changed, "revision": "$current"}
    if report:
        out["report"] = report
    return {"status": "completed", "output": out, "warnings": []}


def warned(changed, report, warnings):
    r = ok(changed, report)
    r["warnings"] = warnings
    return r


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


ALL = list(range(1, 17))


def edit_case(name, i, edits, extra_expect=None):
    t = after(i, edits)
    expect = {"ids": ALL, "entities": {str(i): t}, "canUndo": True, "dirty": True, "revision": "changed"}
    return {"name": name, "steps": [{"op": "execute",
                                     "input": {"operation": {"kind": "edit", "uid": f"$uidOf:{i}", "edits": edits}},
                                     "result": ok([f"$uidOf:{i}"]), "expect": expect, "within": 1e-9}]}


def refused(name, operation, result, non_finite=None):
    step = {"op": "execute", "input": {"operation": operation}, "result": result,
            "expect": {"ids": ALL, "canUndo": False, "dirty": False, "revision": "same"}}
    if non_finite:
        step["nonFinite"] = non_finite
    return {"name": name, "steps": [step]}


def schedule_column_cases():
    """ADR 0721 §4: a block schedule's further columns (Revizyon listesi's Tarih and Açıklama)."""
    out = []

    def make(source, cells=(("x",),)):
        t = table(17, [list(r) for r in cells], source=source)
        geometry = {k: v for k, v in t.items() if k not in ("id", "layerId", "attrs")}
        return t, {"op": "execute", "command": "cad.entities.create",
                   "input": {"layerId": "tablo", "operation": "table", "objects": [{"geometry": geometry}]},
                   "result": {"status": "completed", "output": {"created": ["$uidOf:17"], "ids": [17], "revision": "$current"},
                              "warnings": []}}

    def with_data(ents, data):
        ents = copy.deepcopy(ents)
        for e in ents:
            if e["id"] in data:
                e["attrs"].update(data[e["id"]])
        return ents

    def set_data(ids, attrs):
        return {"op": "execute", "command": "cad.entities.set",
                "input": {"uids": [f"$uidOf:{i}" for i in ids], "attrs": attrs, "operation": "attributes"}, "result": "$any"}

    # One block named: no Blok column; the doors' kinds one cell, “İç; Dış”.
    t, step = make({"kind": "blocks", "names": ["Kapı"], "columns": ["TUR"]})
    step["expect"] = {"ids": ALL + [17], "entities": {"17": worked(t)}, "canUndo": True, "revision": "changed"}
    step["within"] = 1e-9
    out.append({"name": "Blok çizelgesi sütunla: tek blokta Blok sütunu yok, satırın değerleri “; ” ile", "steps": [step]})

    # Every block, by TUR, with a datum “Not”: the Blok column stays; a row's notes once each.
    ents = with_data(drawing(), {5: {"Not": "Yangın"}, 7: {"Not": "Yangın"}})
    t, step = make({"kind": "blocks", "by": "TUR", "columns": ["Not"]})
    step["expect"] = {"entities": {"17": worked(t, ents)}}
    step["within"] = 1e-9
    out.append({"name": "Blok çizelgesi sütunla: iki blokta Blok sütunu kalır, veri de sütun olur", "steps": [
        set_data([5, 7], {"Not": "Yangın"}), step]})

    # Revizyon listesi's shape: one block by REV with Tarih and Açıklama; a datum changed, the list follows in the
    # same step; undo takes both back.
    src = {"kind": "blocks", "names": ["Kapı"], "by": "TUR", "columns": ["Tarih", "Açıklama"]}
    ents = with_data(drawing(), {5: {"Tarih": "2026-10-20", "Açıklama": "Mutfak kapısı"},
                                 6: {"Tarih": "2026-10-20", "Açıklama": "Banyo kapısı"}})
    t, step = make(src)
    first = worked(t, ents)
    step["expect"] = {"entities": {"17": first}}
    step["within"] = 1e-9
    later = with_data(ents, {7: {"Tarih": "2026-10-27", "Açıklama": "Giriş kapısı genişledi"}})
    out.append({"name": "Blok çizelgesi sütunları aynı adımda izler; geri alma birlikte", "steps": [
        set_data([5], {"Tarih": "2026-10-20", "Açıklama": "Mutfak kapısı"}),
        set_data([6], {"Tarih": "2026-10-20", "Açıklama": "Banyo kapısı"}),
        step,
        {**set_data([7], {"Tarih": "2026-10-27", "Açıklama": "Giriş kapısı genişledi"}),
         "expect": {"entities": {"17": worked(first, later)}}, "within": 1e-9},
        {"op": "undo", "expect": {"entities": {"17": first}}, "within": 1e-9}]})

    # A door gone: its notes leave the cell.
    ents = with_data(drawing(), {5: {"Not": "Yangın"}, 6: {"Not": "Ahşap"}})
    t, step = make({"kind": "blocks", "names": ["Kapı"], "columns": ["Not"]})
    step["expect"] = {"entities": {"17": worked(t, ents)}}
    step["within"] = 1e-9
    gone = [e for e in ents if e["id"] != 6]
    out.append({"name": "Blok çizelgesi sütunu: kapı silinince değeri hücreden çıkar", "steps": [
        set_data([5], {"Not": "Yangın"}), set_data([6], {"Not": "Ahşap"}), step,
        {"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:6"]}, "result": "$any",
         "expect": {"entities": {"17": worked(worked(t, ents), gone)}}, "within": 1e-9}]})

    # Refused: a blank column's name; more than ten columns.
    for name, columns, words in [
        ("Ret: blok çizelgesinin sütun adı boş", ["Tarih", " "], "Blok çizelgesinin blok, öznitelik ve sütun adları boş olamaz."),
        ("Ret: blok çizelgesine on birinci sütun", [f"S{k}" for k in range(11)],
         "Blok çizelgesine 11 sütun verildi; en çok 10 öznitelik ya da veri sütunu olur."),
    ]:
        _, step = make({"kind": "blocks", "names": ["Kapı"], "columns": columns})
        step["result"] = failed("invalid_table", words, "objects[0].geometry.source")
        step["expect"] = {"ids": ALL, "canUndo": False, "dirty": False, "revision": "same"}
        out.append({"name": name, "steps": [step]})
    return out


def cases():
    out = []
    out.append(edit_case("Hücreye yazı: tek satır", 1, [{"kind": "setCell", "row": 3, "col": 2, "words": "Zemin"}]))
    out.append(edit_case("Formül: iki hücrenin toplamı", 1, [{"kind": "setCell", "row": 3, "col": 1, "words": "=B2+B3"}]))
    out.append(edit_case("Formül: TOPLA aralıkla, biçimin iki basamağı", 1, [
        {"kind": "style", "row": 3, "col": 1, "rows": 1, "cols": 1, "decimals": 2},
        {"kind": "setCell", "row": 3, "col": 1, "words": "=TOPLA(B2:B3)"}]))
    out.append(edit_case("Formül: Türkçe ve İngilizce adlar, noktalı virgül", 1, [
        {"kind": "setCell", "row": 3, "col": 1, "words": "=ORTALAMA(B2;B3)"},
        {"kind": "setCell", "row": 3, "col": 2, "words": "=max(C2:C3)*mİn(C2:C3)+SAY(B2:C3)"}]))
    out.append(edit_case("Formül: YUVARLA yarım uzağa, sonuç 15 anlamlı basamak", 1, [
        {"kind": "setCell", "row": 3, "col": 1, "words": "=YUVARLA(2.675;2)"},
        {"kind": "setCell", "row": 3, "col": 2, "words": "=0.1+0.2"}]))
    out.append(edit_case("Formül hataları: sıfıra bölme, yazıyla işlem, okunamayan", 1, [
        {"kind": "setCell", "row": 3, "col": 0, "words": "=B2/0"},
        {"kind": "setCell", "row": 3, "col": 1, "words": "=A2+1"},
        {"kind": "setCell", "row": 3, "col": 2, "words": "=TOPLA("}]))
    out.append(edit_case("Formül: dışarıdaki başvuru ve döngü", 1, [
        {"kind": "setCell", "row": 3, "col": 0, "words": "=Z99"},
        {"kind": "setCell", "row": 3, "col": 1, "words": "=C4"},
        {"kind": "setCell", "row": 3, "col": 2, "words": "=B4"}]))
    out.append(edit_case("Satır ekle: formülün başvuruları ve biçim kayar", 1, [
        {"kind": "setCell", "row": 3, "col": 1, "words": "=TOPLA(B2:B3)"},
        {"kind": "style", "row": 3, "col": 0, "rows": 1, "cols": 3, "bold": True},
        {"kind": "insertRows", "at": 2, "count": 1},
        {"kind": "setCell", "row": 2, "col": 1, "words": "3.25"}]))
    out.append(edit_case("Satır sil: silinen hücreye başvuru #BAŞV!, aralık daralır", 1, [
        {"kind": "setCell", "row": 3, "col": 1, "words": "=TOPLA(B2:B3)+B2"},
        {"kind": "deleteRows", "from": 1, "count": 1}]))
    out.append(edit_case("Sütun ekle ve sil: başvurular sütunlarla", 1, [
        {"kind": "setCell", "row": 3, "col": 2, "words": "=$B$2*TOPLA(B2:B3)"},
        {"kind": "insertColumns", "at": 1, "count": 1},
        {"kind": "deleteColumns", "from": 0, "count": 1}]))
    out.append(edit_case("Biçim: kalın, renk, dolgu ve hiza; Sığdır kalını ölçer", 1, [
        {"kind": "style", "row": 1, "col": 0, "rows": 2, "cols": 3, "bold": True, "color": "#1f4e79", "fill": "#eef2f7", "align": "center"},
        {"kind": "fit", "from": 0, "count": 3}]))
    out.append(edit_case("Biçimi temizle: değen biçimler gider", 1, [
        {"kind": "style", "row": 1, "col": 0, "rows": 1, "cols": 3, "bold": True},
        {"kind": "style", "row": 3, "col": 0, "rows": 1, "cols": 1, "fill": "#ffeecc"},
        {"kind": "clearStyle", "row": 1, "col": 1, "rows": 1, "cols": 1}]))
    out.append(edit_case("Birleştir: aralığın formülü gider; Ayır", 1, [
        {"kind": "setCell", "row": 3, "col": 1, "words": "=B2"},
        {"kind": "merge", "row": 3, "col": 0, "rows": 1, "cols": 2},
        {"kind": "setCell", "row": 3, "col": 1, "words": "Genel"},
        {"kind": "unmerge", "row": 3, "col": 0}]))
    out.append(edit_case("Satır yüksekliği ve sütun genişliği", 1, [
        {"kind": "rowHeight", "from": 0, "count": 2, "height": 0.75},
        {"kind": "columnWidth", "from": 2, "count": 1, "width": 2.25}]))
    out.append(edit_case("Pencerenin düzenlemeleri: sütun hizası, başlık, çizgiler, çerçeve, yazıya sığdır", 1, [
        {"kind": "columnAlign", "from": 0, "count": 2, "align": "center"},
        {"kind": "header", "on": False},
        {"kind": "grid", "grid": "rows"},
        {"kind": "frame", "width": 0.05},
        {"kind": "setCell", "row": 1, "col": 0, "words": "Salon ve yemek odası"},
        {"kind": "fit", "from": 0, "count": 1, "grow": True},
        {"kind": "fit", "from": 1, "count": 2, "rows": True}]))
    out.append(edit_case("Kaynağı bırak düzenlemesi", 10, [{"kind": "unlink"}]))
    out.append(edit_case("Alan: nesnenin alanı projenin basamağıyla", 1, [
        {"kind": "field", "row": 1, "col": 1, "source": {"kind": "object", "uid": "$uidOf:3", "value": "area"}}]))
    out.append(edit_case("Alan: blok sayısı ve formülle", 1, [
        {"kind": "field", "row": 3, "col": 1, "source": {"kind": "project", "value": "blockCount", "name": "Kapı"}},
        {"kind": "setCell", "row": 3, "col": 2, "words": "=B4*2"}]))
    # A field let go of: its words stay.
    t = after(1, [{"kind": "field", "row": 3, "col": 1, "source": {"kind": "project", "value": "blockCount", "name": "Kapı"}}])
    t2 = copy.deepcopy(t)
    t2.pop("fields", None)
    out.append({"name": "Alanı bırak: yazısı kalır", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "edit", "uid": "$uidOf:1", "edits": [
            {"kind": "field", "row": 3, "col": 1, "source": {"kind": "project", "value": "blockCount", "name": "Kapı"}}]}},
         "result": ok(["$uidOf:1"]), "expect": {"entities": {"1": t}}, "within": 1e-9},
        {"op": "execute", "input": {"operation": {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "unfield", "row": 3, "col": 1}]}},
         "result": ok(["$uidOf:1"]), "expect": {"entities": {"1": t2}, "canUndo": True}, "within": 1e-9}]})
    # A door gone: the block schedule and the count follow in the same step.
    ents = [e for e in drawing() if e["id"] != 7]
    counted = after(1, [{"kind": "field", "row": 3, "col": 1, "source": {"kind": "project", "value": "blockCount", "name": "Kapı"}}])
    counted_after = copy.deepcopy(counted)
    counted_after["cells"][3][1] = "2"
    sched = worked(ent(9), ents)
    out.append({"name": "Kapı silinince alan ve blok çizelgesi aynı adımda izler", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "edit", "uid": "$uidOf:1", "edits": [
            {"kind": "field", "row": 3, "col": 1, "source": {"kind": "project", "value": "blockCount", "name": "Kapı"}}]}},
         "result": ok(["$uidOf:1"]), "expect": {"entities": {"1": counted}}, "within": 1e-9},
        {"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:7"]},
         "result": "$any",
         "expect": {"entities": {"1": counted_after, "9": sched}}, "within": 1e-9},
        {"op": "undo", "expect": {"entities": {"1": counted, "9": ent(9)}}, "within": 1e-9}]})
    out.extend(schedule_column_cases())
    # Refresh.
    rooms = copy.deepcopy(ent(10))
    cells, kept, replaced = overridden(rooms, ROOMS_NEW, "keep")
    fresh = refreshed(rooms, cells, ["left", "right", "right"])
    fresh["source"] = {**rooms["source"], "digest": digest("odalar.csv"), "copy": ROOMS_NEW}
    out.append({"name": "Güncelle: bağlı dosya, elle değişen hücreler korunur", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "refresh", "uids": ["$uidOf:10"], "base": "$fixture:commands/v1/tables"}},
         "result": ok(["$uidOf:10"], [{"uid": "$uidOf:10", "status": "updated", "kept": kept, "replaced": replaced}]),
         "expect": {"ids": ALL, "entities": {"10": tidy(fresh)}, "canUndo": True, "dirty": True, "revision": "changed"}, "within": 1e-9}]})
    rooms_r = ent(16)
    cells, kept, replaced = overridden(rooms_r, ROOMS_NEW, "replace")
    fresh = refreshed(rooms_r, cells, ["left", "right", "right"])
    fresh["source"] = {**rooms_r["source"], "digest": digest("odalar.csv"), "copy": ROOMS_NEW}
    out.append({"name": "Güncelle: dosyanınki yazılır (replace)", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "refresh", "uids": ["$uidOf:16"], "base": "$fixture:commands/v1/tables"}},
         "result": ok(["$uidOf:16"], [{"uid": "$uidOf:16", "status": "updated", "kept": kept, "replaced": replaced}]),
         "expect": {"entities": {"16": tidy(fresh)}}, "within": 1e-9}]})
    out.append({"name": "Güncelle: aynı baytlar yazılmaz", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "refresh", "uids": ["$uidOf:14"], "base": "$fixture:commands/v1/tables"}},
         "result": ok([], [{"uid": "$uidOf:14", "status": "unchanged", "kept": 0, "replaced": 0}]),
         "expect": {"ids": ALL, "entities": {"14": ent(14)}, "canUndo": False, "dirty": False, "revision": "same"}}]})
    out.append({"name": "Güncelle: yolunda olmayan dosya ve bağsız tablo söylenir", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "refresh", "uids": ["$uidOf:11", "$uidOf:12"], "base": "$fixture:commands/v1/tables"}},
         "result": warned([], [{"uid": "$uidOf:11", "status": "missing", "kept": 0, "replaced": 0},
                               {"uid": "$uidOf:12", "status": "unlinked", "kept": 0, "replaced": 0}],
                          [{"code": "source_missing", "message": "“yok.csv” dosyası yolunda yok; tablo olduğu gibi kaldı. Dosyaya bağla ile yeniden seçin."},
                           {"code": "source_unlinked", "message": "“eski.csv” tablosu dosyasına bağlı değil; Dosyaya bağla ile dosyayı seçin."}]),
         "expect": {"ids": ALL, "canUndo": False, "revision": "same"}}]})
    sched_cells = tc.schedule("areas", [tc.polygon([(X + x, Y + y) for x, y in ROOM], label="Salon")], UNITS)
    stale = table(17, [["Ad", "Alan (m²)", "Çevre (m)"], ["Salon", "19.00", "17.000"]], aligns=["left", "right", "right"],
                  source={"kind": "areas", "objects": ["$uidOf:3"]})
    geometry = {k: v for k, v in stale.items() if k not in ("id", "layerId", "attrs")}
    geometry["kind"] = "table"
    areas = refreshed(stale, sched_cells["cells"], sched_cells["aligns"])
    out.append({"name": "Güncelle: alan çizelgesi nesnesinden, blok çizelgesi canlı", "steps": [
        {"op": "execute", "command": "cad.entities.create",
         "input": {"layerId": "tablo", "operation": "table", "objects": [{"geometry": geometry}]}, "result": "$any"},
        {"op": "execute", "input": {"operation": {"kind": "refresh", "uids": ["$uidOf:17", "$uidOf:9"]}},
         "result": ok(["$uidOf:17"], [{"uid": "$uidOf:17", "status": "updated", "kept": 0, "replaced": 0},
                                      {"uid": "$uidOf:9", "status": "live", "kept": 0, "replaced": 0}]),
         "expect": {"entities": {"17": tidy({**areas, "source": {"kind": "areas", "objects": ["$uidOf:3"]}})}, "canUndo": True},
         "within": 1e-9}]})
    # Link and unlink.
    rows = tf.read(file_bytes("malzeme.csv"))["sheets"][0]["rows"]
    made = tc.from_rows(rows, False)
    linked = refreshed(ent(15), made["cells"], made["aligns"])
    linked["source"] = {"kind": "file", "name": "malzeme.csv", "path": "malzeme.csv", "digest": digest("malzeme.csv"), "copy": made["cells"]}
    out.append({"name": "Dosyaya bağla: hücreler dosyadan, yol ve özet saklanır", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "link", "uid": "$uidOf:15", "path": "malzeme.csv", "base": "$fixture:commands/v1/tables"}},
         "result": ok(["$uidOf:15"], [{"uid": "$uidOf:15", "status": "updated", "kept": 0, "replaced": 0}]),
         "expect": {"ids": ALL, "entities": {"15": tidy(linked)}, "canUndo": True, "dirty": True}, "within": 1e-9}]})
    gone = ent(10)
    del gone["source"]
    out.append({"name": "Kaynağı bırak", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "unlink", "uids": ["$uidOf:10", "$uidOf:15"]}},
         "result": ok(["$uidOf:10"]), "expect": {"ids": ALL, "entities": {"10": gone}, "canUndo": True}}]})
    # Refusals.
    out.append(refused("Ret: kimlik değil", {"kind": "edit", "uid": NOT_UID, "edits": [{"kind": "setCell", "row": 0, "col": 0, "words": "x"}]},
                       failed("invalid_uid", M_UID.format(NOT_UID), "operation.uid")))
    out.append(refused("Ret: nesne yok", {"kind": "edit", "uid": FAKE, "edits": [{"kind": "setCell", "row": 0, "col": 0, "words": "x"}]},
                       failed("entity_not_found", M_NOT_FOUND.format(FAKE), "operation.uid")))
    out.append(refused("Ret: tablo değil", {"kind": "edit", "uid": "$uidOf:4", "edits": [{"kind": "setCell", "row": 0, "col": 0, "words": "x"}]},
                       failed("not_a_table", M_NOT_TABLE, "operation.uid")))
    out.append(refused("Ret: kilitli katman", {"kind": "edit", "uid": "$uidOf:2", "edits": [{"kind": "setCell", "row": 0, "col": 0, "words": "x"}]},
                       failed("layer_locked", M_LOCKED, "operation.uid")))
    out.append(refused("Ret: düzenleme yok", {"kind": "edit", "uid": "$uidOf:1", "edits": []},
                       failed("invalid_table", "Düzenleme verilmedi; en az bir düzenleme verin.", "operation.edits")))
    out.append(refused("Ret: hücre tablonun dışında", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "setCell", "row": 9, "col": 0, "words": "x"}]},
                       failed("cell_out_of_range", "10. satır, 1. sütun tablonun dışında; tablonun 4 satırı, 3 sütunu var.", "operation.edits[0]")))
    out.append(refused("Ret: aralık tablonun dışında", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "style", "row": 2, "col": 2, "rows": 1, "cols": 2, "bold": True}]},
                       failed("cell_out_of_range", "3. satır, 3. sütundan 1 × 2 aralık tablonun içinde olmalı; tablonun 4 satırı, 3 sütunu var.", "operation.edits[0]")))
    out.append(refused("Ret: renk #rrggbb değil", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "style", "row": 0, "col": 0, "rows": 1, "cols": 1, "fill": "kırmızı"}]},
                       failed("invalid_table", "1. biçimin Dolgu “kırmızı”; #rrggbb olmalı.", "operation.styles")))
    out.append(refused("Ret: satır yüksekliği sıfır", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "rowHeight", "from": 0, "count": 1, "height": 0}]},
                       failed("invalid_table", "Satır yüksekliği 0; sıfırdan büyük ve en çok 1000000 m olmalı.", "operation.edits[0]")))
    out.append(refused("Ret: sonlu olmayan genişlik", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "columnWidth", "from": 0, "count": 1, "width": 1}]},
                       failed("not_finite", "Sütun genişliği sonlu bir sayı olmalı.", "operation.edits[0]"),
                       non_finite={"operation.edits[0].width": "NaN"}))
    out.append(refused("Ret: bütün satırlar silinemez", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "deleteRows", "from": 0, "count": 4}]},
                       failed("invalid_table", "Tablonun bütün satırları silinemez; en az biri kalır.", "operation.edits[0]")))
    out.append(refused("Ret: alanın nesnesi yok", {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "field", "row": 1, "col": 1, "source": {"kind": "object", "uid": FAKE, "value": "area"}}]},
                       failed("entity_not_found", f"Alanın nesnesi “{FAKE}” çizimde yok.", "operation.edits[0]")))
    out.append(refused("Ret: kaynağı olmayan tablo güncellenmez", {"kind": "refresh", "uids": ["$uidOf:1"]},
                       failed("no_source", "Tablonun kaynağı yok: Tabloyu güncelle çizelgeleri ve dosyaya bağlı tabloları yazar.", "operation.uids")))
    out.append(refused("Ret: bağlanacak dosya yok", {"kind": "link", "uid": "$uidOf:15", "path": "yok.csv", "base": "$fixture:commands/v1/tables"},
                       failed("source_missing", "“yok.csv” dosyası okunamadı: yolu doğru mu, dosya var mı?", "operation.path")))
    xls = tf.read(file_bytes("eski.xls"))["problem"]
    out.append(refused("Ret: tablo olarak okunamayan dosya", {"kind": "link", "uid": "$uidOf:15", "path": "eski.xls", "base": "$fixture:commands/v1/tables"},
                       failed("source_unreadable", f"“eski.xls”: {xls}", "operation.path")))
    out.append(refused("Ret: tablo verilmedi", {"kind": "refresh", "uids": []},
                       failed("no_entities", "Tablo verilmedi. En az bir tablonun kimliğini verin.", "operation.uids")))
    out.append({"name": "Ret: beklenen sürüm yazısı geçersiz", "steps": [
        {"op": "execute", "input": {"operation": {"kind": "unlink", "uids": ["$uidOf:10"]}, "expectedRevision": "on iki"},
         "result": failed("invalid_revision", M_REVISION, "expectedRevision"),
         "expect": {"ids": ALL, "canUndo": False, "dirty": False, "revision": "same"}}]})
    out.append({"name": "Ret: beklenen sürüm geçti", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": {"operation": {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "setCell", "row": 0, "col": 0, "words": "x"}]}},
         "result": "$any"},
        {"op": "execute", "input": {"operation": {"kind": "edit", "uid": "$uidOf:1", "edits": [{"kind": "setCell", "row": 0, "col": 0, "words": "y"}]},
                                    "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": M_CONFLICT, "path": "expectedRevision",
                                                     "revision": "$current"}},
         "expect": {"revision": "same"}}]})
    return out


def overridden(t, fresh, rule):
    copy_ = t["source"]["copy"]
    out = [list(r) for r in fresh]
    kept = replaced = 0
    for r, row in enumerate(out):
        for c in range(len(row)):
            if r >= len(t["cells"]) or c >= len(t["cells"][r]) or r >= len(copy_) or c >= len(copy_[r]):
                continue
            own, was = t["cells"][r][c], copy_[r][c]
            if own == was:
                continue
            if rule == "keep" and row[c] == was:
                row[c] = own
                kept += 1
            else:
                replaced += 1
    return out, kept, replaced


def refreshed(t, cells, aligns):
    r = tc.refresh({**t, "merges": t.get("merges", []), "aligns": t.get("aligns")}, cells, aligns, FONT)
    out = copy.deepcopy(t)
    out.update({"rows": r["rows"], "columns": r["columns"], "cells": r["cells"], "merges": r["merges"]})
    out["aligns"] = r["aligns"]
    return tidy(out)


def main():
    doc = {
        "format": "kentos.command-cases", "version": 1, "command": "cad.tables.edit", "commandVersion": 1,
        "title": "Tablonun hücreleri, formülleri, alanları ve dosyası",
        "note": "Written by scripts/fixtures/table_command_cases.py from docs/adr/0327; do not edit by hand.",
        "setup": setup(), "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil")
            return 1
        print(f"{OUT}: {len(doc['cases'])} durum güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT}: {len(doc['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
