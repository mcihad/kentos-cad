#!/usr/bin/env python3
"""Independent reference of Grafikler ve alan istatistikleri (docs/adr/0227,
`kentos.charts/1`): a field's statistics, the six charts' models, the axes'
ticks, four significant digits and the text's estimated width.

    python3 scripts/fixtures/chart_cases.py           # writes the file
    python3 scripts/fixtures/chart_cases.py --check   # writes nothing; compares

Writes fixtures/charts/v1/cases.json from the ADR's rules alone, with
Python's standard library (exact fractions, the calendar) and no KentOS
code. The geometry core (`ops::charts`; crates/shared/geometry-core/tests/
all/charts.rs) and the web through WASM give the same answers.

The rules, in short (the ADR has them in full):

- A column has a kind (number, text, date, boolean), whether its values sort
  by what they show (a value list), and cells: empty, a value (its canonical
  text and, when it differs, what it shows) or refused (as written). A number
  reads as ADR 0200 §4's decimal (`.` or `,`, no exponent, at most 30
  digits); a date as YYYY-MM-DD; yes or no as true and false; a text with
  JavaScript's spaces trimmed (empty then: empty). A value its kind does not
  read is refused.
- Equal values meet by value: numbers exactly, dates by day, texts as
  trimmed. A value shows its first shown text (a number its own canonical
  text). Order: values (numbers by value, dates by day, false before true,
  texts in the natural order of ADR 0153 §6; a value list by what it shows),
  then the refused in the natural order of their text, then the empty.
- Numbers follow `kentos.statistics/1`: exact sums, least and greatest; the
  mean is the sum over the count half to even at the largest scale plus two
  (or the scale given); the sample deviation in doubles, two passes in the
  values' order, at the mean's scale (the binary value's exact digits, half
  to even). Quartiles are Hyndman and Fan's seventh, exact at the largest
  scale plus two.
"""

import json
import struct
import sys
from datetime import date
from fractions import Fraction
from functools import cmp_to_key
from math import sqrt
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "charts" / "v1" / "cases.json"
ALPHABET = "abcçdefgğhıijklmnoöpqrsştuüvwxyz"
JS_SPACE = " \t\n\v\f\r                 　﻿"
EMPTY = "(boş)"
INVALID = "(uymayan)"
OTHER = "Diğer"
MAX_VALUES = 500
MAX_BARS = 30
MAX_BOXES = 30
MAX_SERIES = 8
EPOCH = date(1970, 1, 1).toordinal()


def cmp(a, b):
    return (a > b) - (a < b)


def trim(s):
    return s.strip(JS_SPACE)


# ── Natural order (docs/adr/0153 §6) ───────────────────────────────────


def fold(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


def tokens(text):
    out, i = [], 0
    while i < len(text):
        if "0" <= text[i] <= "9":
            j = i
            while j < len(text) and "0" <= text[j] <= "9":
                j += 1
            out.append(text[i:j])
            i = j
        else:
            out.append(text[i])
            i += 1
    return out


def rank(c):
    f = fold(c)
    if not f.isalpha():
        return (0, ord(f))
    i = ALPHABET.find(f)
    return (1, i) if i >= 0 else (2, ord(f))


def is_run(t):
    return "0" <= t[0] <= "9"


def natural_cmp(a, b):
    ta, tb = tokens(a), tokens(b)
    for x, y in zip(ta, tb):
        if is_run(x) and is_run(y):
            o = cmp(int(x), int(y))
        elif is_run(x):
            o = -1
        elif is_run(y):
            o = 1
        else:
            o = cmp(rank(x), rank(y))
        if o:
            return o
    return cmp(len(ta), len(tb)) or cmp([ord(c) for c in a], [ord(c) for c in b])


# ── Numbers (ADR 0200 §4) ──────────────────────────────────────────────


class Dec:
    """An exact decimal: its value and its scale (fraction digits)."""

    def __init__(self, value, scale):
        self.value = Fraction(value)
        self.scale = scale

    def text(self, scale=None):
        s = self.scale if scale is None else scale
        m = self.value * 10**s
        assert m.denominator == 1, (self.value, s)
        m = m.numerator
        neg = m < 0
        digits = str(abs(m))
        if s:
            digits = digits.rjust(s + 1, "0")
            body = digits[:-s] + "." + digits[-s:]
        else:
            body = digits
        return ("-" if neg else "") + body


def read_number(text):
    t = trim(text)
    neg = t.startswith("-")
    if neg or t.startswith("+"):
        t = t[1:]
    seps = [i for i, c in enumerate(t) if c in ".,"]
    if len(seps) > 1:
        return None
    if seps:
        whole, frac = t[: seps[0]], t[seps[0] + 1 :]
    else:
        whole, frac = t, ""
    if not whole and not frac:
        return None
    if not all("0" <= c <= "9" for c in whole + frac):
        return None
    whole = whole.lstrip("0")
    if len(whole) + len(frac) > 30:
        return None
    m = int((whole + frac) or "0")
    v = Fraction(m, 10 ** len(frac))
    return Dec(-v if neg else v, len(frac))


def half_even(q):
    """A fraction to the nearest integer, halves to even."""
    f = q.numerator // q.denominator
    r = q - f
    if r > Fraction(1, 2) or (r == Fraction(1, 2) and f % 2 == 1):
        f += 1
    return f


def at_scale(v, s):
    return Dec(Fraction(half_even(v * 10**s), 10**s), s)


def fixed(x, s):
    """A double at s fraction digits, half to even on its exact value; no minus on zero."""
    t = format(x, f".{s}f")
    if t.startswith("-") and all(c in "0." for c in t[1:]):
        t = t[1:]
    return t


def figures(nums, scale=None):
    """kentos.statistics/1 over exact decimals: sum, mean, least, greatest, deviation."""
    if not nums:
        return None
    top = max(d.scale for d in nums)
    out = scale if scale is not None else top + 2
    total = sum((d.value for d in nums), Fraction(0))
    res = {
        "sum": Dec(total, top),
        "mean": at_scale(total / len(nums), out),
        "min": min(nums, key=lambda d: d.value),
        "max": max(nums, key=lambda d: d.value),
        "std": None,
    }
    # min/max: the first of the least (greatest) as written.
    lo = nums[0]
    hi = nums[0]
    for d in nums:
        if d.value < lo.value:
            lo = d
        if d.value > hi.value:
            hi = d
    res["min"], res["max"] = lo, hi
    if len(nums) >= 2:
        xs = [float(d.text()) for d in nums]
        n = float(len(xs))
        acc = 0.0
        for x in xs:
            acc += x
        mean = acc / n
        ss = 0.0
        for x in xs:
            ss += (x - mean) * (x - mean)
        res["std"] = fixed(sqrt(ss / (n - 1.0)), out)
    return res


def quartile(sorted_values, k):
    """Hyndman and Fan's seventh: p = k/4 of exact values sorted ascending."""
    n = len(sorted_values)
    h = Fraction((n - 1) * k, 4)
    i = h.numerator // h.denominator
    f = h - i
    if f == 0:
        return sorted_values[i]
    return sorted_values[i] + f * (sorted_values[i + 1] - sorted_values[i])


def share(count, total):
    if total == 0:
        return "0.0"
    t = half_even(Fraction(count * 1000, total))
    return f"{t // 10}.{t % 10}"


# ── Reading a column ───────────────────────────────────────────────────


def read_date(text):
    if len(text) != 10 or text[4] != "-" or text[7] != "-":
        return None
    try:
        y, m, d = int(text[0:4]), int(text[5:7]), int(text[8:10])
        if not text[0:4].isdigit() or not text[5:7].isdigit() or not text[8:10].isdigit():
            return None
        return date(y, m, d).toordinal() - EPOCH
    except ValueError:
        return None


def date_text(days):
    d = date.fromordinal(days + EPOCH)
    return f"{d.day:02}.{d.month:02}.{d.year:04}"


class Read:
    """A column read: each row's value as (kind, key, payload)."""

    def __init__(self, col):
        self.kind = col["kind"]
        self.labelled = col.get("labelled", False)
        self.vals = []
        self.shown = {}
        for cell in col["cells"]:
            self.vals.append(self.read(cell))

    def read(self, cell):
        if cell is None:
            return ("empty", None)
        if "invalid" in cell:
            t = trim(cell["invalid"])
            return ("invalid", t) if t else ("empty", None)
        canon = cell["v"]
        shown = cell.get("s")
        v = None
        if self.kind == "number":
            d = read_number(canon)
            if d is not None:
                v = ("num", d)
        elif self.kind == "date":
            dd = read_date(canon)
            if dd is not None:
                v = ("date", dd)
        elif self.kind == "boolean":
            if canon in ("true", "false"):
                v = ("bool", canon == "true")
        else:
            t = trim(canon)
            v = ("text", t) if t else ("empty", None)
        if v is None:
            t = trim(canon)
            return ("invalid", t) if t else ("empty", None)
        if v[0] != "empty":
            key = self.key(v)
            if key not in self.shown:
                if shown is not None:
                    self.shown[key] = trim(shown)
                elif v[0] == "num":
                    self.shown[key] = v[1].text()
                elif v[0] == "text":
                    self.shown[key] = v[1]
                else:
                    self.shown[key] = trim(canon)
        return v

    @staticmethod
    def key(v):
        kind, p = v
        if kind == "num":
            return ("num", p.value)
        return (kind, p)

    def show(self, key):
        if key[0] == "empty":
            return EMPTY
        if key[0] == "invalid":
            return key[1]
        return self.shown[key]

    def cmp(self, a, b):
        cls = lambda k: 2 if k[0] == "empty" else 1 if k[0] == "invalid" else 0
        o = cmp(cls(a), cls(b))
        if o:
            return o
        if a[0] == "invalid":
            return natural_cmp(a[1], b[1])
        if a[0] == "empty":
            return 0
        if self.labelled:
            return natural_cmp(self.show(a), self.show(b))
        if a[0] == "text":
            return natural_cmp(a[1], b[1])
        return cmp(a[1], b[1])

    def number(self, r):
        v = self.vals[r]
        return v[1] if v[0] == "num" else None


def groups(read, rows):
    order, members = [], {}
    for r in rows:
        k = Read.key(read.vals[r]) if read.vals[r][0] != "empty" else ("empty", None)
        if k not in members:
            members[k] = []
            order.append(k)
        members[k].append(r)
    order.sort(key=cmp_to_key(read.cmp))
    return [(k, members[k]) for k in order]


def top_scale(read, rows):
    return max((read.number(r).scale for r in rows if read.number(r) is not None), default=0)


# ── A field's statistics (§3) ──────────────────────────────────────────

KIND_LABEL = {"number": "Sayı", "text": "Metin", "date": "Tarih", "boolean": "Evet/hayır"}


def summary(col, read, rows):
    s = {
        "key": col["key"],
        "label": col["label"],
        "kind": KIND_LABEL[col["kind"]],
        "objects": len(rows),
        "filled": 0,
        "empty": 0,
        "invalid": 0,
        "distinct": 0,
    }
    seen = set()
    nums, days = [], []
    for r in rows:
        v = read.vals[r]
        if v[0] == "empty":
            s["empty"] += 1
        elif v[0] == "invalid":
            s["invalid"] += 1
        else:
            s["filled"] += 1
            seen.add(Read.key(v))
            if v[0] == "num":
                nums.append(v[1])
            if v[0] == "date":
                days.append(v[1])
    s["distinct"] = len(seen)
    if col["kind"] == "number" and nums:
        f = figures(nums)
        s["min"], s["max"] = f["min"].text(), f["max"].text()
        s["mean"], s["sum"] = f["mean"].text(), f["sum"].text()
    if col["kind"] == "date" and days:
        s["min"] = read.show(("date", min(days)))
        s["max"] = read.show(("date", max(days)))
    return s


def detail(col, read, rows):
    summ = summary(col, read, rows)
    stats = [["Nesne", str(summ["objects"])], ["Dolu", str(summ["filled"])], ["Boş", str(summ["empty"])]]
    if summ["invalid"]:
        stats.append(["Uymayan", str(summ["invalid"])])
    stats.append(["Benzersiz", str(summ["distinct"])])
    gs = groups(read, rows)
    values = [(k, m) for k, m in gs if k[0] not in ("empty", "invalid")]
    kind = col["kind"]
    if kind == "number":
        nums = [read.number(r) for r in rows if read.number(r) is not None]
        if nums:
            f = figures(nums)
            stats.append(["Toplam", f["sum"].text()])
            stats.append(["Ortalama", f["mean"].text()])
            if f["std"] is not None:
                stats.append(["Std. sapma", f["std"]])
            s = max(d.scale for d in nums)
            vs = sorted(d.value for d in nums)
            stats.append(["En az", f["min"].text()])
            for name, k in (("Birinci çeyrek", 1), ("Ortanca", 2), ("Üçüncü çeyrek", 3)):
                stats.append([name, Dec(quartile(vs, k), s + 2).text()])
            stats.append(["En çok", f["max"].text()])
            stats.append(["Aralık", Dec(vs[-1] - vs[0], s).text()])
            stats.append(["Çeyrekler arası", Dec(quartile(vs, 3) - quartile(vs, 1), s + 2).text()])
    elif kind == "date":
        days = sorted(read.vals[r][1] for r in rows if read.vals[r][0] == "date")
        if days:
            stats.append(["En erken", read.show(("date", days[0]))])
            stats.append(["Ortanca", read.show(("date", days[(len(days) - 1) // 2]))])
            stats.append(["En geç", read.show(("date", days[-1]))])
            stats.append(["Aralık (gün)", str(days[-1] - days[0])])
    elif kind == "boolean":
        stats.append(["Evet", str(sum(1 for r in rows if read.vals[r] == ("bool", True)))])
        stats.append(["Hayır", str(sum(1 for r in rows if read.vals[r] == ("bool", False)))])
    else:
        lengths = [len(read.vals[r][1]) for r in rows if read.vals[r][0] == "text"]
        if lengths:
            stats.append(["En kısa", f"{min(lengths)} karakter"])
            stats.append(["En uzun", f"{max(lengths)} karakter"])
    mode = None
    for k, m in values:
        if mode is None or len(m) > len(mode[1]):
            mode = (k, m)
    if mode is not None and len(mode[1]) > 1:
        stats.append(["En sık", f"{read.show(mode[0])} ({len(mode[1])} nesne)"])
    listed = sorted(values, key=cmp_to_key(lambda a, b: cmp(len(b[1]), len(a[1])) or read.cmp(a[0], b[0])))
    out_values = [
        {"text": read.show(k), "count": len(m), "share": share(len(m), len(rows)), "mark": "value", "rows": m} for k, m in listed[:MAX_VALUES]
    ]
    invalid = [r for r in rows if read.vals[r][0] == "invalid"]
    if invalid:
        out_values.append({"text": INVALID, "count": len(invalid), "share": share(len(invalid), len(rows)), "mark": "invalid", "rows": invalid})
    empty = [r for r in rows if read.vals[r][0] == "empty"]
    if empty:
        out_values.append({"text": EMPTY, "count": len(empty), "share": share(len(empty), len(rows)), "mark": "empty", "rows": empty})
    return {"summary": summ, "stats": stats, "values": out_values, "more": max(0, len(listed) - MAX_VALUES)}


# ── Charts (§4) ────────────────────────────────────────────────────────

AGG_LABEL = {"none": "Yok", "count": "Sayı", "sum": "Toplam", "mean": "Ortalama", "min": "En az", "max": "En çok", "median": "Ortanca"}


def aggregate(agg, read, rows, s):
    if agg == "count":
        return Dec(len(rows), 0)
    nums = [read.number(r) for r in rows if read.number(r) is not None]
    if not nums:
        return None
    if agg == "median":
        vs = sorted(d.value for d in nums)
        return Dec(quartile(vs, 2), s + 2)
    f = figures(nums, s + 2)
    return {"sum": f["sum"], "mean": f["mean"], "min": f["min"], "max": f["max"]}[agg]


def not_numbers(read, rows):
    return sum(1 for r in rows if read.vals[r][0] != "num")


def skipped(n, what):
    return [f"{n} nesnenin {what} sayı değil: hesaba katılmadı."] if n else []


def column(data, key):
    for i, c in enumerate(data["columns"]):
        if c["key"] == key:
            return i
    return None


def label(data, i):
    return data["columns"][i]["label"]


def dtext(d):
    return d.text() if d is not None else ""


def bars(spec, data, reads, rows):
    cat = column(data, spec.get("category"))
    agg = spec.get("aggregate", "count")
    if agg == "none":
        agg = "count"
    value = column(data, spec.get("value")) if agg != "count" else None
    cr = reads[cat]
    vr = reads[value] if value is not None else None
    s = top_scale(vr, rows) if vr else 0
    cats = [(cr.show(k), m, aggregate(agg, vr, m, s)) for k, m in groups(cr, rows)]
    sort = spec.get("sort", "category")
    if sort in ("desc", "asc"):
        def by(a, b):
            if a[2] is not None and b[2] is not None:
                o = cmp(a[2].value, b[2].value)
                return -o if sort == "desc" else o
            if a[2] is not None:
                return -1
            if b[2] is not None:
                return 1
            return 0
        cats.sort(key=cmp_to_key(by))
    notes = []
    folded = False
    if len(cats) > MAX_BARS:
        folded = True
        rest = cats[MAX_BARS - 1 :]
        cats = cats[: MAX_BARS - 1]
        rrows = sorted(r for c in rest for r in c[1])
        cats.append((f"{OTHER} ({len(rest)})", rrows, aggregate(agg, vr, rrows, s)))
        notes.append(f"{len(rest)} kategori “{OTHER}” çubuğunda toplandı.")
    if vr is not None:
        notes += skipped(not_numbers(vr, rows), "değeri")
    columns = [label(data, cat), "Nesne"] + ([AGG_LABEL[agg]] if agg != "count" else [])
    table = [[c[0], str(len(c[1]))] + ([dtext(c[2])] if agg != "count" else []) for c in cats]
    body = [{"label": c[0], "value": dtext(c[2]), "objects": len(c[1]), "other": folded and i == len(cats) - 1} for i, c in enumerate(cats)]
    return {
        "xTitle": label(data, cat),
        "yTitle": f"{AGG_LABEL[agg]} · {label(data, value)}" if value is not None else "Nesne sayısı",
        "notes": notes,
        "elements": [c[1] for c in cats],
        "table": {"columns": columns, "rows": table},
        "body": {"bars": body, "integer": agg == "count" or (agg in ("sum", "min", "max") and s == 0)},
    }


def pie(spec, data, reads, rows, every):
    cat = column(data, spec.get("category"))
    agg = "count" if spec.get("aggregate", "count") in ("count", "none") else "sum"
    value = column(data, spec.get("value")) if agg == "sum" else None
    cr = reads[cat]
    vr = reads[value] if value is not None else None
    ranked = []
    for k, m in groups(cr, every):
        v = aggregate(agg, vr, m, 0)
        if v is not None and v.value > 0:
            ranked.append((k, v))
    ranked.sort(key=cmp_to_key(lambda a, b: -cmp(a[1].value, b[1].value)))
    fold = len(ranked) > MAX_SERIES
    colours = {}
    for i, (k, _) in enumerate(ranked):
        colours[k] = (i, "other" if fold and i >= MAX_SERIES - 1 else i)
    slices, other_rows, other_count, refused = [], [], 0, 0
    for k, m in groups(cr, rows):
        v = aggregate(agg, vr, m, 0)
        if v is None or v.value <= 0:
            refused += 1
            continue
        if k not in colours:
            refused += 1
            continue
        rank_, colour = colours[k]
        if colour == "other":
            other_rows += m
            other_count += 1
        else:
            slices.append((rank_, cr.show(k), m, v, colour))
    slices.sort(key=lambda t: t[0])
    if other_rows:
        other_rows.sort()
        slices.append((10**9, f"{OTHER} ({other_count})", other_rows, aggregate(agg, vr, other_rows, 0), "other"))
    total = sum((sl[3].value for sl in slices), Fraction(0))
    tscale = max([0] + [sl[3].scale for sl in slices])
    notes = []
    if refused:
        notes.append(f"{refused} kategorinin değeri sıfır, eksi ya da yok: dilimi çizilmedi.")
    if vr is not None:
        notes += skipped(not_numbers(vr, rows), "değeri")
    title = f"{AGG_LABEL[agg]} · {label(data, value)}" if value is not None else "Nesne"
    body = []
    table = []
    for sl in slices:
        sh = f"{(t := half_even(sl[3].value * 1000 / total)) // 10}.{t % 10}"
        table.append([sl[1], sl[3].text(), sh])
        body.append({"label": sl[1], "value": sl[3].text(), "share": sh, "color": sl[4]})
    return {
        "xTitle": label(data, cat),
        "yTitle": f"{AGG_LABEL[agg]} · {label(data, value)}" if value is not None else "Nesne sayısı",
        "notes": notes,
        "elements": [sl[2] for sl in slices],
        "table": {"columns": [label(data, cat), title, "Pay (%)"], "rows": table},
        "body": {"slices": body, "total": Dec(total, tscale).text(), "word": "Nesne" if agg == "count" else "Toplam"},
    }


def sturges(n):
    p = 0
    while (1 << p) < n:
        p += 1
    return min(p + 1, 50)


def histogram(spec, data, reads, rows):
    col = column(data, spec.get("value"))
    rd = reads[col]
    nums = [(rd.number(r), r) for r in rows if rd.number(r) is not None]
    notes = skipped(not_numbers(rd, rows), "değeri")
    lab = label(data, col)
    if not nums:
        notes.append("Sayı değeri olan nesne yok.")
        return {"xTitle": lab, "yTitle": "Nesne sayısı", "notes": notes, "elements": [], "table": {"columns": ["Alt sınır", "Üst sınır", "Sayı"], "rows": []},
                "body": {"bins": [], "min": "0", "max": "0", "mean": None, "median": None}}
    s = max(d.scale for d, _ in nums)
    a = min(d.value for d, _ in nums)
    b = max(d.value for d, _ in nums)
    k = 1 if a == b else (min(max(spec["bins"], 1), 100) if spec.get("bins") else sturges(len(nums)))
    members = [[] for _ in range(k)]
    for d, r in nums:
        i = 0 if a == b else min((k * (d.value - a)) // (b - a), k - 1)
        members[int(i)].append(r)
    edge = lambda j: at_scale(a + Fraction(j) * (b - a) / k, s + 2).text()
    table = [[edge(j), edge(j + 1), str(len(m))] for j, m in enumerate(members)]
    f = figures([d for d, _ in nums], s + 2)
    med = Dec(quartile(sorted(d.value for d, _ in nums), 2), s + 2)
    return {
        "xTitle": lab,
        "yTitle": "Nesne sayısı",
        "notes": notes,
        "elements": members,
        "table": {"columns": ["Alt sınır", "Üst sınır", "Sayı"], "rows": table},
        "body": {"bins": [{"lo": t[0], "hi": t[1], "count": int(t[2])} for t in table], "min": Dec(a, s).text(), "max": Dec(b, s).text(), "mean": f["mean"].text(), "median": med.text()},
    }


def boxes(spec, data, reads, rows):
    col = column(data, spec.get("value"))
    rd = reads[col]
    g = column(data, spec.get("group")) if spec.get("group") else None
    s = top_scale(rd, rows)
    if g is not None:
        parts = [(reads[g].show(k), m) for k, m in groups(reads[g], rows)]
    else:
        parts = [(label(data, col), list(rows))]
    notes = skipped(not_numbers(rd, rows), "değeri")
    if len(parts) > MAX_BOXES:
        notes.append(f"{len(parts) - MAX_BOXES} grup gösterilmedi (en çok {MAX_BOXES} kutu).")
        parts = parts[:MAX_BOXES]
    columns = ["Grup", "Nesne", "En az", "Birinci çeyrek", "Ortanca", "Üçüncü çeyrek", "En çok", "Ortalama", "Aykırı"]
    if g is None:
        columns = columns[1:]
    elements, items, table, outs_all = [], [], [], []
    for lab, m in parts:
        elements.append(m)
        nums = [(rd.number(r), r) for r in m if rd.number(r) is not None]
        if not nums:
            items.append({"label": lab, "n": 0, "outliers": []})
            outs_all.append([])
            continue
        vs = sorted(nums, key=lambda t: t[0].value)
        values = [d.value for d, _ in vs]
        q1, med, q3 = quartile(values, 1), quartile(values, 2), quartile(values, 3)
        iqr = q3 - q1
        lo_f, hi_f = q1 - Fraction(3, 2) * iqr, q3 + Fraction(3, 2) * iqr
        inside = [v for v in values if lo_f <= v <= hi_f]
        low = inside[0] if inside else values[0]
        high = inside[-1] if inside else values[-1]
        outs = [(d, r) for d, r in vs if d.value < lo_f or d.value > hi_f]
        f = figures([d for d, _ in nums], s + 2)
        item = {
            "label": lab,
            "n": len(values),
            "min": Dec(values[0], s).text(),
            "q1": Dec(q1, s + 2).text(),
            "median": Dec(med, s + 2).text(),
            "q3": Dec(q3, s + 2).text(),
            "max": Dec(values[-1], s).text(),
            "low": Dec(low, s).text(),
            "high": Dec(high, s).text(),
            "mean": f["mean"].text(),
            "outliers": [],
        }
        row = [lab, str(len(values)), item["min"], item["q1"], item["median"], item["q3"], item["max"], item["mean"], str(len(outs))]
        table.append(row if g is not None else row[1:])
        items.append(item)
        outs_all.append(outs)
    for i, outs in enumerate(outs_all):
        for d, r in outs:
            items[i]["outliers"].append([Dec(d.value, s).text(), len(elements)])
            elements.append([r])
    return {
        "xTitle": label(data, g) if g is not None else "",
        "yTitle": label(data, col),
        "notes": notes,
        "elements": elements,
        "table": {"columns": columns, "rows": table},
        "body": {"boxes": items},
    }


def sig4(x):
    if x == 0.0:
        return "0"
    t = format(x, ".3e")
    mant, exp = t.split("e")
    exp = int(exp)
    neg = mant.startswith("-")
    digits = mant.lstrip("-").replace(".", "")
    point = 1 + exp
    if point <= 0:
        body = "0." + "0" * (-point) + digits
    elif point >= len(digits):
        body = digits + "0" * (point - len(digits))
    else:
        body = digits[:point] + "." + digits[point:]
    return ("-" if neg else "") + body


def trend(xs, ys):
    n = len(xs)
    if n < 2:
        return None
    sx = 0.0
    for x in xs:
        sx += x
    sy = 0.0
    for y in ys:
        sy += y
    mx, my = sx / n, sy / n
    sxx = sxy = syy = 0.0
    for x, y in zip(xs, ys):
        dx, dy = x - mx, y - my
        sxx += dx * dx
        sxy += dx * dy
        syy += dy * dy
    if sxx == 0.0:
        return None
    b = sxy / sxx
    a = my - b * mx
    text = f"y = {sig4(b)}x {'-' if a < 0 else '+'} {sig4(abs(a))}"
    if syy != 0.0:
        text += f"; R² = {sig4(sxy * sxy / (sxx * syy))}"
    return text


def scatter(spec, data, reads, rows):
    xc, yc = column(data, spec.get("x")), column(data, (spec.get("y") or [None])[0])
    xr, yr = reads[xc], reads[yc]
    pts, skip = [], 0
    for r in rows:
        x, y = xr.number(r), yr.number(r)
        if x is not None and y is not None:
            pts.append((x, y, r))
        else:
            skip += 1
    xs = [float(p[0].text()) for p in pts]
    ys = [float(p[1].text()) for p in pts]
    rng = lambda ds: [min(ds, key=lambda d: d.value).text(), max(ds, key=lambda d: d.value).text()] if ds else None
    return {
        "xTitle": label(data, xc),
        "yTitle": label(data, yc),
        "notes": skipped(skip, "X'i ya da Y'si"),
        "elements": [[p[2]] for p in pts],
        "table": {"columns": [label(data, xc), label(data, yc)], "rows": [[p[0].text(), p[1].text()] for p in pts]},
        "body": {"trend": trend(xs, ys) if spec.get("trend", True) else None, "xRange": rng([p[0] for p in pts]), "yRange": rng([p[1] for p in pts]), "count": len(pts)},
    }


def first_extreme(ds, larger):
    best = None
    for d in ds:
        if best is None or (d.value > best.value if larger else d.value < best.value):
            best = d
    return best


def lines(spec, data, reads, rows):
    xc = column(data, spec.get("x"))
    ys = [column(data, k) for k in spec.get("y", []) if column(data, k) is not None]
    agg = spec.get("aggregate", "none")
    xr = reads[xc]
    sc = column(data, spec.get("series")) if len(ys) <= 1 and spec.get("series") else None
    if sc is not None:
        sets = [(reads[sc].show(k), ys[0] if ys else None, m) for k, m in groups(reads[sc], rows)]
    elif not ys:
        sets = [(label(data, xc), None, list(rows))]
    else:
        sets = [(label(data, y), y, list(rows)) for y in ys]
    notes = []
    if len(sets) > MAX_SERIES:
        notes.append(f"{len(sets) - MAX_SERIES} seri gösterilmedi (en çok {MAX_SERIES} seri).")
        sets = sets[:MAX_SERIES]
    s = max([top_scale(reads[y], rows) for y in ys], default=0)
    if agg == "count":
        ylab = "Nesne sayısı"
    elif len(ys) == 1:
        ylab = label(data, ys[0]) if agg == "none" else f"{AGG_LABEL[agg]} · {label(data, ys[0])}"
    else:
        ylab = "Değer" if agg == "none" else AGG_LABEL[agg]
    elements, series, table, skip = [], [], [], 0
    for i, (lab, y, m) in enumerate(sets):
        yr = reads[y] if y is not None else None
        pts = []
        for r in m:
            x = xr.number(r)
            v = yr.number(r) if yr else None
            if x is not None and agg == "count":
                pts.append((x, None, r))
            elif x is not None and v is not None:
                pts.append((x, v, r))
            else:
                skip += 1
        pts.sort(key=cmp_to_key(lambda a, b: cmp(a[0].value, b[0].value)))
        out = []
        if agg == "none":
            for x, v, r in pts:
                out.append([x.text(), v.text(), len(elements)])
                table.append([lab, x.text(), v.text()])
                elements.append([r])
        else:
            j = 0
            while j < len(pts):
                k = j + 1
                while k < len(pts) and pts[k][0].value == pts[j][0].value:
                    k += 1
                rs = [p[2] for p in pts[j:k]]
                v = aggregate(agg, yr, rs, s)
                if v is not None:
                    out.append([pts[j][0].text(), v.text(), len(elements)])
                    table.append([lab, pts[j][0].text(), v.text()])
                    elements.append(rs)
                j = k
        series.append({"label": lab, "color": i, "points": out})
    notes += skipped(skip, "X'i ya da Y'si")
    columns = ["Seri", label(data, xc), ylab]
    if len(series) == 1 and sc is None:
        columns = columns[1:]
        table = [r[1:] for r in table]
    allx = [read_number(p[0]) for se in series for p in se["points"]]
    ally = [read_number(p[1]) for se in series for p in se["points"]]
    rng = lambda ds: [first_extreme(ds, False).text(), first_extreme(ds, True).text()] if ds else None
    return {
        "xTitle": label(data, xc),
        "yTitle": ylab,
        "notes": notes,
        "elements": elements,
        "table": {"columns": columns, "rows": table},
        "body": {"series": series, "xRange": rng(allx), "yRange": rng(ally)},
    }


def chart(spec, data, rows):
    reads = [Read(c) for c in data["columns"]]
    every = list(range(data["rows"]))
    kind = spec["kind"]
    out = {"kind": kind}
    if kind == "bar":
        out.update(bars(spec, data, reads, rows))
    elif kind == "pie":
        out.update(pie(spec, data, reads, rows, every))
    elif kind == "histogram":
        out.update(histogram(spec, data, reads, rows))
    elif kind == "box":
        out.update(boxes(spec, data, reads, rows))
    elif kind == "scatter":
        out.update(scatter(spec, data, reads, rows))
    else:
        out.update(lines(spec, data, reads, rows))
    return out


# ── Axes (§5) ──────────────────────────────────────────────────────────


def magnitude(v):
    """The power of ten of v's first significant digit (v ≠ 0)."""
    v = abs(v)
    e = 0
    while v >= 10:
        v /= 10
        e += 1
    while v < 1:
        v *= 10
        e -= 1
    return e


def dec_digits(v):
    """The fraction digits of an exact decimal fraction, none trailing."""
    s = 0
    while (v * 10**s).denominator != 1:
        s += 1
    return s


def nice(lo_text, hi_text, most, integer, zero):
    lo, hi = read_number(lo_text).value, read_number(hi_text).value
    most = max(most, 2)
    if zero:
        lo, hi = min(lo, 0), max(hi, 0)
    if lo == hi:
        if lo == 0:
            hi = Fraction(1)
        else:
            unit = Fraction(10) ** (magnitude(lo) - 1)
            lo, hi = lo - unit, hi + unit
    span = hi - lo
    start = magnitude(span) - len(str(most)) - 1
    for e in range(start, start + 64):
        for d in (Fraction(1), Fraction(2), Fraction(5, 2), Fraction(5)):
            step = d * Fraction(10) ** e
            if integer and step.denominator != 1:
                continue
            a = (lo / step).__floor__()
            b = -((-hi / step).__floor__())
            if b - a + 1 <= most:
                digits = dec_digits(step)
                return {
                    "lo": Dec(a * step, dec_digits(a * step)).text(),
                    "hi": Dec(b * step, dec_digits(b * step)).text(),
                    "step": Dec(step, digits).text(),
                    "labels": [Dec(k * step, digits).text() for k in range(a, b + 1)],
                }
    return None


# ── Text width (§5) ────────────────────────────────────────────────────

ADVANCES = {
    "Plus Jakarta Sans": (0.472, 0.732),
    "Inter": (0.483, 0.646),
    "IBM Plex Sans": (0.456, 0.600),
    "Source Sans 3": (0.425, 0.497),
    "Noto Sans": (0.480, 0.572),
    "Roboto": (0.454, 0.562),
}


def f32(x):
    return struct.unpack("f", struct.pack("f", x))[0]


def advance(c, family):
    letter, digit = ADVANCES.get(family, (0.520, 0.636))
    if "0" <= c <= "9":
        return f32(digit)
    if c in ".,:;'|!ilı":
        return f32(0.30)
    if c == " ":
        return f32(0.26)
    if c in "-+−":
        return f32(0.55)
    if c == "%":
        return f32(0.98)
    if c == "…":
        return f32(0.85)
    return f32(f32(letter) * f32(1.08))


def width(text, size, family):
    em = f32(0.0)
    for c in text:
        em = f32(em + advance(c, family))
    return f32(em * f32(size))


def fit(text, room, size, family):
    if width(text, size, family) <= f32(room):
        return text
    for n in range(len(text) - 1, 0, -1):
        t = text[:n].rstrip() + "…"
        if width(t, size, family) <= f32(room):
            return t
    return "…"


# ── The cases ──────────────────────────────────────────────────────────


def v(text, shown=None):
    return {"v": text} if shown is None else {"v": text, "s": shown}


def bad(text):
    return {"invalid": text}


# A small parcel layer: its fields as the platforms read them (canonical values; refused and empty cells).
USE_LABELS = {"K": "Konut", "T": "Ticaret", "S": "Sanayi", "P": "Park"}
PARCELS = [
    # Ada, Parsel, Kullanım (value list), Kat, Alan, Tescil, Ruhsat, Mahalle, Not
    ("101", "P1", "K", "3", "412.50", "2019-04-02", "true", "Çarşı", "köşe"),
    ("101", "P2", "K", "4", "398.25", "2019-04-02", "true", "Çarşı", None),
    ("101", "P10", "T", "2", "1200", "2020-11-30", "false", "İstasyon", None),
    ("102", "P3", "K", "3", "412.5", "2021-01-15", None, "ılıca", None),
    ("102", "P4", "S", "1", "2500.75", "2018-07-07", "false", "Organize", None),
    ("102", "P5", "K", "12", "380", "2021-01-15", "true", "Çarşı", "yüksek"),
    ("103", "P6", "P", "0", "980.4", "2017-02-28", "true", "Yeşilyurt", None),
    ("103", "P7", "K", "5", "-15", "2022-12-31", "true", "Çarşı", "hatalı alan"),
    ("103", "P8", "T", "2", "640.125", None, "false", "İstasyon", None),
    ("104", "P9", None, "3", None, "2023-06-01", "true", "ılıca", None),
    (None, "P11", "K", "x3", "512", "2016-02-29", "evet", "Çarşı", None),
    ("104", "P12", "K", "4", "455.5", "2023-06-01", "true", " Çarşı ", None),
]


def parcel_data():
    cols = [
        {"key": "Ada", "label": "Ada", "kind": "number", "cells": []},
        {"key": "Parsel", "label": "Parsel", "kind": "text", "cells": []},
        {"key": "Kullanım", "label": "Kullanım", "kind": "text", "labelled": True, "cells": []},
        {"key": "Kat", "label": "Kat sayısı", "kind": "number", "cells": []},
        {"key": "Alan", "label": "Tapu alanı", "kind": "number", "cells": []},
        {"key": "Tescil", "label": "Tescil tarihi", "kind": "date", "cells": []},
        {"key": "Ruhsat", "label": "Ruhsat", "kind": "boolean", "cells": []},
        {"key": "Mahalle", "label": "Mahalle", "kind": "text", "cells": []},
        {"key": "Not", "label": "Not", "kind": "text", "cells": []},
    ]
    for p in PARCELS:
        ada, parsel, use, kat, alan, tescil, ruhsat, mah, note = p
        cells = [
            v(ada) if ada else None,
            v(parsel),
            v(use, USE_LABELS[use]) if use else None,
            (v(kat) if read_number(kat) else bad(kat)) if kat else None,
            v(alan) if alan else None,
            v(tescil, f"{tescil[8:10]}.{tescil[5:7]}.{tescil[0:4]}") if tescil else None,
            (v(ruhsat, "Evet" if ruhsat == "true" else "Hayır") if ruhsat in ("true", "false") else bad(ruhsat)) if ruhsat else None,
            v(mah),
            v(note) if note else None,
        ]
        for c, cell in zip(cols, cells):
            c["cells"].append(cell)
    return {"rows": len(PARCELS), "columns": cols}


def many_categories():
    """40 neighbourhoods with counts 1…40 (bars past 30, a pie past 8)."""
    cols = [
        {"key": "Mahalle", "label": "Mahalle", "kind": "text", "cells": []},
        {"key": "Nüfus", "label": "Nüfus", "kind": "number", "cells": []},
    ]
    rows = 0
    for i in range(1, 41):
        for k in range(i % 7 + 1):
            cols[0]["cells"].append(v(f"M{i}"))
            cols[1]["cells"].append(v(str(i * 10 + k)))
            rows += 1
    return {"rows": rows, "columns": cols}


def measures():
    """A hundred values with outliers, two groups (histogram, box, scatter)."""
    cols = [
        {"key": "Bölge", "label": "Bölge", "kind": "text", "cells": []},
        {"key": "Fiyat", "label": "Birim fiyat", "kind": "number", "cells": []},
        {"key": "Alan", "label": "Alan (m²)", "kind": "number", "cells": []},
    ]
    n = 0
    for i in range(100):
        region = "Kuzey" if i % 3 else "Güney"
        price = f"{1000 + (i * 37) % 400}.{(i * 7) % 100:02}"
        area = f"{200 + (i * 53) % 300}.{(i * 3) % 10}"
        if i == 17:
            price = "9800.00"  # an outlier
        if i == 42:
            price = "12.50"  # another
        if i == 64:
            area = ""  # no area
        cols[0]["cells"].append(v(region))
        cols[1]["cells"].append(v(price))
        cols[2]["cells"].append(v(area) if area else None)
        n += 1
    return {"rows": n, "columns": cols}


def roc_table():
    """ROC ile doğrulama's table (ADR 0237 §8): thresholds high to low."""
    rows = [
        ("0.95", "12", "15.0", "40", "1.2"),
        ("0.90", "25", "31.3", "120", "3.6"),
        ("0.82", "41", "51.3", "300", "9.1"),
        ("0.75", "55", "68.8", "600", "18.2"),
        ("0.60", "66", "82.5", "1100", "33.3"),
        ("0.60", "70", "87.5", "1100", "33.3"),
        ("0.41", "77", "96.3", "2200", "66.7"),
        ("0.20", "80", "100.0", "3300", "100.0"),
    ]
    heads = ["Eşik", "Doğru pozitif", "Doğru pozitif oranı (%)", "Hücre", "Alan oranı (%)"]
    cols = [{"key": h, "label": h, "kind": "number", "cells": [v(r[i]) for r in rows]} for i, h in enumerate(heads)]
    return {"rows": len(rows), "columns": cols}


def profile_table():
    """Kesit's table (ADR 0243 §8): two lines' stations."""
    rows = [("1", "0", "12.5"), ("1", "10", "13.25"), ("1", "20", "14"), ("2", "0", "8"), ("2", "10", ""), ("2", "20", "9.5"), ("2", "30", "11")]
    heads = [("Çizgi", "text"), ("Uzaklık (m)", "number"), ("Değer", "number")]
    cols = [{"key": h, "label": h, "kind": k, "cells": [v(r[i]) if r[i] else None for r in rows]} for i, (h, k) in enumerate(heads)]
    return {"rows": len(rows), "columns": cols}


def series_table():
    """Zaman serisi's table (ADR 0243 §9): steps, a column a point."""
    rows = [("1", "2026-01-01", "3.5", "4"), ("2", "2026-01-02", "2", "4.5"), ("3", "2026-01-03", "6.25", "5"), ("4", "2026-01-04", "", "3")]
    cols = [
        {"key": "Adım", "label": "Adım", "kind": "number", "cells": [v(r[0]) for r in rows]},
        {"key": "Zaman", "label": "Zaman", "kind": "date", "cells": [v(r[1]) for r in rows]},
        {"key": "K-1", "label": "K-1", "kind": "number", "cells": [v(r[2]) if r[2] else None for r in rows]},
        {"key": "K-2", "label": "K-2", "kind": "number", "cells": [v(r[3]) for r in rows]},
    ]
    return {"rows": len(rows), "columns": cols}


def many_values():
    """520 distinct values: the list names 500."""
    cells = [v(f"D{i:03}") for i in range(520)] + [v("D007"), v("D007"), None, bad("?")]
    return {"rows": len(cells), "columns": [{"key": "Kod", "label": "Kod", "kind": "text", "cells": cells}]}


def field_cases():
    parcels = parcel_data()
    allrows = list(range(parcels["rows"]))
    some = [0, 1, 2, 3, 5, 7, 9, 11]
    cases = []
    for name, data, rows in [("parseller", parcels, allrows), ("parseller-secili", parcels, some), ("cok-deger", many_values(), list(range(many_values()["rows"])))]:
        for i, col in enumerate(data["columns"]):
            read = Read(col)
            cases.append({"name": f"{name} › {col['key']}", "data": name, "column": i, "rows": rows, "summary": summary(col, read, rows), "detail": detail(col, read, rows)})
    # Quartiles of small counts.
    for vals in [["7"], ["1", "2"], ["1", "2", "3"], ["1", "2", "3", "4"], ["1", "2", "3", "4", "5"], ["0.1", "0.15", "2", "3.333", "10", "-4", "6", "6"]]:
        col = {"key": "x", "label": "x", "kind": "number", "cells": [v(t) for t in vals]}
        rows = list(range(len(vals)))
        cases.append({"name": "ceyrek " + " ".join(vals), "data": {"rows": len(vals), "columns": [col]}, "column": 0, "rows": rows, "summary": summary(col, Read(col), rows), "detail": detail(col, Read(col), rows)})
    return cases


def chart_cases():
    parcels = parcel_data()
    allp = list(range(parcels["rows"]))
    selp = [0, 2, 3, 6, 9, 10]
    cats = many_categories()
    allc = list(range(cats["rows"]))
    meas = measures()
    allm = list(range(meas["rows"]))
    roc, prof, ser = roc_table(), profile_table(), series_table()
    sets = {"parseller": parcels, "mahalleler": cats, "olcumler": meas, "roc": roc, "kesit": prof, "zaman": ser}
    specs = [
        ("cubuk-sayi", "parseller", {"kind": "bar", "category": "Kullanım", "aggregate": "count", "sort": "category"}, allp),
        ("cubuk-toplam-azalan", "parseller", {"kind": "bar", "category": "Mahalle", "aggregate": "sum", "value": "Alan", "sort": "desc"}, allp),
        ("cubuk-ortalama-artan", "parseller", {"kind": "bar", "category": "Ada", "aggregate": "mean", "value": "Alan", "sort": "asc"}, allp),
        ("cubuk-ortanca", "parseller", {"kind": "bar", "category": "Ruhsat", "aggregate": "median", "value": "Kat"}, allp),
        ("cubuk-en-az-secili", "parseller", {"kind": "bar", "category": "Kullanım", "aggregate": "min", "value": "Alan"}, selp),
        ("cubuk-tarih", "parseller", {"kind": "bar", "category": "Tescil", "aggregate": "max", "value": "Kat"}, allp),
        ("cubuk-diger", "mahalleler", {"kind": "bar", "category": "Mahalle", "aggregate": "sum", "value": "Nüfus", "sort": "desc"}, allc),
        ("cubuk-diger-kategori", "mahalleler", {"kind": "bar", "category": "Mahalle", "aggregate": "count"}, allc),
        ("pasta-sayi", "parseller", {"kind": "pie", "category": "Kullanım", "aggregate": "count"}, allp),
        ("pasta-toplam-secili", "parseller", {"kind": "pie", "category": "Mahalle", "aggregate": "sum", "value": "Alan"}, selp),
        ("pasta-diger", "mahalleler", {"kind": "pie", "category": "Mahalle", "aggregate": "sum", "value": "Nüfus"}, allc),
        ("pasta-diger-alt-kume", "mahalleler", {"kind": "pie", "category": "Mahalle", "aggregate": "count"}, allc[::5]),
        ("histogram-otomatik", "olcumler", {"kind": "histogram", "value": "Fiyat"}, allm),
        ("histogram-5", "olcumler", {"kind": "histogram", "value": "Alan", "bins": 5}, allm),
        ("histogram-tek", "parseller", {"kind": "histogram", "value": "Ada", "bins": 4}, [0, 1, 2]),
        ("histogram-bos", "parseller", {"kind": "histogram", "value": "Kat"}, [10]),
        ("kutu-gruplu", "olcumler", {"kind": "box", "value": "Fiyat", "group": "Bölge"}, allm),
        ("kutu-tek", "parseller", {"kind": "box", "value": "Alan"}, allp),
        ("sacilim", "olcumler", {"kind": "scatter", "x": "Alan", "y": ["Fiyat"], "trend": True}, allm),
        ("sacilim-dikey", "parseller", {"kind": "scatter", "x": "Ada", "y": ["Kat"], "trend": True}, [0, 1, 2]),
        ("cizgi-roc", "roc", {"kind": "line", "x": "Alan oranı (%)", "y": ["Doğru pozitif oranı (%)"], "aggregate": "none"}, list(range(roc["rows"]))),
        ("cizgi-kesit", "kesit", {"kind": "line", "x": "Uzaklık (m)", "y": ["Değer"], "series": "Çizgi", "aggregate": "none"}, list(range(prof["rows"]))),
        ("cizgi-zaman", "zaman", {"kind": "line", "x": "Adım", "y": ["K-1", "K-2"], "aggregate": "none"}, list(range(ser["rows"]))),
        ("cizgi-toplam", "parseller", {"kind": "line", "x": "Kat", "y": ["Alan"], "aggregate": "sum"}, allp),
        ("cizgi-sayi-seri", "parseller", {"kind": "line", "x": "Kat", "y": [], "series": "Kullanım", "aggregate": "count"}, allp),
    ]
    out = []
    for name, data, spec, rows in specs:
        out.append({"name": name, "data": data, "spec": spec, "rows": rows, "want": chart(spec, sets[data], rows)})
    return sets, out


def tick_cases():
    cases = [
        ("0", "1200", 6, True, True),
        ("0", "37", 5, True, True),
        ("-15", "2500.75", 6, False, True),
        ("12.5", "9800", 8, False, False),
        ("0.001", "0.0137", 5, False, False),
        ("4", "4", 5, False, False),
        ("0", "0", 4, True, True),
        ("-3.2", "-0.4", 4, False, False),
        ("486512.34", "487933.1", 7, False, False),
        ("1", "3", 2, True, True),
        ("0", "7", 30, True, True),
        ("0.5", "0.5", 3, False, False),
    ]
    return [{"lo": lo, "hi": hi, "max": m, "integer": i, "zero": z, "want": nice(lo, hi, m, i, z)} for lo, hi, m, i, z in cases]


def sig_cases():
    xs = [523.4, 0.001, 1.0, -0.000123456, 12345.6, 99999.5, 0.99995, 1e-7, -2.5e9, 0.1 + 0.2, 0.5212, 1.0625]
    return [{"x": x, "want": sig4(x)} for x in xs]


def width_cases():
    cases = []
    for family in ["Plus Jakarta Sans", "Inter", "Source Sans 3", "Sistem yazı tipi"]:
        for text, size, room in [("Konut", 12, 40), ("İstasyon Mahallesi", 12, 60), ("1200.75", 12, 50), ("%58.6", 13, 30), ("Kat sayısı", 15, 200)]:
            cases.append({"text": text, "size": size, "family": family, "room": room, "width": round(width(text, size, family), 4), "fit": fit(text, room, size, family)})
    return cases


def build():
    sets, charts = chart_cases()
    return {
        "format": "kentos.chart-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/chart_cases.py",
        "title": "Grafikler ve alan istatistikleri: istatistikler, grafiklerin modelleri, eksenler, yazı genişliği (ADR 0227)",
        "data": {**sets, "parseller-secili": sets["parseller"], "cok-deger": many_values()},
        "fields": field_cases(),
        "charts": charts,
        "ticks": tick_cases(),
        "sig4": sig_cases(),
        "widths": width_cases(),
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    want = text_of(build())
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != want:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("chart cases match")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(want, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
