"""The shared cases of Katman stili's values in the project's drawing unit
(docs/adr/0165 §2, the 13 Ekim addendum).

    python3 scripts/fixtures/style_units_cases.py           # writes the file
    python3 scripts/fixtures/style_units_cases.py --check   # writes nothing; compares

Writes fixtures/style/v1/units.json. The rules are written here from the ADR
on their own, not from an implementation's output; the desktop's classes and
counts (crates/native/style/tests/all/units.rs) are held to them, and the
web's, when it takes up the addendum, will be:

- The drawing keeps its geometry in metres. A renderer's expression reads it in
  the project's drawing unit: `$uzunluk` is the length in metres times the
  units to a metre (100 for cm, 1 000 for mm), `$alan` the area in square
  metres times their square (10 000, 1 000 000). An object without a length
  or an area has no value.
- “Değerlerden sınıfla” lists the distinct values as text, numbers by value
  (a number is written in its shortest form: 3000000, 0.5), with how many
  objects have each.
- “Sınıfla” makes `n` classes of equal interval between the smallest and the
  largest number: class i starts at lo + i · (hi − lo) / n and ends where the
  next starts, the last at hi; a bound is written to two decimals (halves
  upward) in the label “a – b”.
- The Nesne column counts the drawn objects: an object goes to the first
  category of its value (none for an object without a value), or to the first
  class that takes its number (from the lower bound, up to the upper bound;
  the last class also takes it); a rule takes the objects of its parent that
  meet its condition and a “değilse” rule what no sibling took.

Everything here is in exact fractions: the geometry is a set of axis-aligned
rectangles and paths, so lengths and areas are exact too.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/style/v1/units.json"

# The units a project is drawn in: their name and how many of them make a metre.
UNITS = [("m", 1), ("cm", 100), ("mm", 1000)]


def num(x):
    """A number as JavaScript's String writes it here: whole numbers without a point."""
    x = float(x)
    if x == int(x) and abs(x) < 1e21:
        return str(int(x))
    return repr(x)


def rect(id_, w, h):
    """A rectangle `w` × `h` m, `id_ · 10` m east of the origin."""
    w, h = F(w), F(h)
    x = F(id_ * 10)
    pts = [(x, F(0)), (x + w, F(0)), (x + w, h), (x, h)]
    entity = {
        "kind": "polygon",
        "id": id_,
        "layerId": "k",
        "attrs": {},
        "pts": [{"x": float(px), "y": float(py)} for px, py in pts],
    }
    return entity, {"area": w * h, "length": 2 * (w + h)}


def path(id_, length):
    """An open path `length` m long, along the east axis."""
    length = F(length)
    x = F(id_ * 10)
    entity = {
        "kind": "polyline",
        "id": id_,
        "layerId": "k",
        "attrs": {},
        "pts": [{"x": float(x), "y": 0.0}, {"x": float(x + length), "y": 0.0}],
    }
    return entity, {"area": None, "length": length}


def point(id_, x, y):
    entity = {"kind": "point", "id": id_, "layerId": "k", "attrs": {}, "p": {"x": float(x), "y": float(y)}}
    return entity, {"area": None, "length": None}


# Three areas of 3, 1 and 0.5 m² (7, 4 and 3 m round), a path of 2 m and a point.
OBJECTS = [rect(1, 2, F(3, 2)), rect(2, 1, 1), rect(3, F(1, 2), 1), path(4, 2), point(5, 3, 4)]
ENTITIES = [e for e, _ in OBJECTS]
MEASURES = [m for _, m in OBJECTS]


def value(measure, expr, per_metre):
    """The expression's value of an object, in the unit; None when the object has none."""
    if expr == "$alan":
        return None if measure["area"] is None else measure["area"] * per_metre * per_metre
    return None if measure["length"] is None else measure["length"] * per_metre


def rounded(x):
    """The label's bound: two decimals, halves upward (Math.round)."""
    return num(F(math.floor(x * 100 + F(1, 2)), 100))


def classes(numbers, n):
    """`n` classes of equal interval between the smallest and the largest number."""
    lo, hi = min(numbers), max(numbers)
    if hi == lo:
        return [(lo, hi)]
    step = (hi - lo) / n
    return [(lo + i * step, hi if i == n - 1 else lo + (i + 1) * step) for i in range(n)]


def class_of(x, cls):
    """The first class that takes the number: from its lower bound up to its upper one, the last also takes it."""
    for i, (lo, hi) in enumerate(cls):
        if x >= lo and (x < hi or (i == len(cls) - 1 and x <= hi)):
            return i
    return None


def expression_case(expr, per_metre):
    values = [value(m, expr, per_metre) for m in MEASURES]
    texts = [None if v is None else num(v) for v in values]
    numbers = [v for v in values if v is not None]
    # The distinct values, numbers by value, with how many objects have each.
    distinct = sorted(set(numbers))
    unique = [{"value": num(v), "count": sum(1 for x in numbers if x == v)} for v in distinct]
    # A category for each distinct value: an object goes to the one with its value; the rest have none.
    categories = {"counts": [u["count"] for u in unique], "rest": len(values) - len(numbers)}
    graduated = []
    for n in (2, 4):
        cls = classes(numbers, n)
        taken = [class_of(v, cls) for v in values if v is not None]
        counts = [taken.count(i) for i in range(len(cls))]
        graduated.append(
            {
                "n": n,
                "classes": [
                    {"min": float(lo), "max": float(hi), "label": f"{rounded(lo)} – {rounded(hi)}"}
                    for lo, hi in cls
                ],
                "counts": counts,
                "rest": len(values) - len(numbers),
            }
        )
    return {
        "expr": expr,
        "values": texts,
        "numbers": [None if v is None else float(v) for v in values],
        "unique": unique,
        "categories": categories,
        "graduated": graduated,
    }


def rules_case(per_metre):
    """Larger than 2 m², longer than 3.5 m round, and what neither took."""
    big = f"$alan > {num(2 * per_metre * per_metre)}"
    long = f"$uzunluk > {num(F(7, 2) * per_metre)}"
    rules = [
        {"id": "a", "label": "Büyük", "filter": big},
        {"id": "b", "label": "Uzun", "filter": long},
        {"id": "c", "label": "Diğerleri", "isElse": True},
    ]
    first =[i for i, m in enumerate(MEASURES) if m["area"] is not None and m["area"] > 2]
    second = [i for i, m in enumerate(MEASURES) if m["length"] is not None and m["length"] > F(7, 2)]
    rest = [i for i in range(len(MEASURES)) if i not in first and i not in second]
    return {
        "rules": rules,
        "counts": [
            {"path": [0], "count": len(first)},
            {"path": [1], "count": len(second)},
            {"path": [2], "count": len(rest)},
        ],
    }


def cases():
    out = []
    for name, per_metre in UNITS:
        out.append(
            {
                "unit": name,
                "perMetre": per_metre,
                "expressions": [expression_case(e, per_metre) for e in ("$alan", "$uzunluk")],
                "rules": rules_case(per_metre),
            }
        )
    return out


def main():
    doc = {
        "format": "kentos.style-units",
        "version": 1,
        "title": "Katman stili: ifadelerin değerleri projenin çizim biriminde",
        "note": "ADR 0165'in 13 Ekim eki. scripts/fixtures/style_units_cases.py kurallardan yazar (kesirlerle, bir uygulamanın çıktısından değil); masaüstünün sınıfları ve sayıları crates/native/style/tests/all/units.rs'te bunu geçer. Geometri hep metrede (çizimin kendisi), ifadeler birimde: $uzunluk metre × birimin metredeki sayısı, $alan metrekare × karesi. Değerlerden sınıfla ve Sınıfla'nın yazdığı değerler, Nesne sütunu ve kuralların sayıları her birimde.",
        "layerName": "Kadastro",
        "entities": ENTITIES,
        "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
