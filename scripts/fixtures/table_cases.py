#!/usr/bin/env python3
"""The shared cases of a table in the drawing (docs/adr/0184): its layout
(outline, lines, where each cell's words stand), its grips, the sizes that
fit its words, the schedules objects write (Koordinat çizelgesi, Alan
çizelgesi, Öznitelik tablosu), a file's rows made cells and a cell's words
made one line.

    python3 scripts/fixtures/table_cases.py           # writes the file
    python3 scripts/fixtures/table_cases.py --check   # writes nothing; compares

Writes fixtures/table/v1/cases.json. The rules are written here from the ADR
on their own, not from an implementation's output; the geometry core
(`geom::table`, `ops::table`; through WASM `tableLayout`, `tableSizes`,
`tableSchedule`, `tableFromRows`, `tableOneLine`) is held to them. Letters'
advances are the drawing typefaces' measured widths, read as data as
paragraph_cases.py reads them; areas, perimeters and positions are computed
with 50-digit arithmetic (mpmath) as measure_cases.py defines them, and
written by the display rule (ADR 0149) as it states it.

Layout (§2). A table hangs from its top left corner p, turned by its
rotation ρ: a point x along its rows and y down its columns is
p + x·(cos ρ, sin ρ) − y·(−sin ρ, cos ρ). Its corners are (0, 0), (W, 0),
(W, H), (0, H), W its columns' widths summed, H its rows' heights. Its grid
lines run between rows (horizontal, top to bottom) and between columns
(vertical, left to right); the outer ones are drawn always, the inner ones
unless the grid is `outer` (the outline alone) or `rows` (no inner vertical
ones); with `none` nothing. With a frame width f (and a grid that is not
`none`) the outer ones are not lines but a band f wide inside the outline:
four strips, the top (0, 0)–(W, f), the bottom (0, H − f)–(W, H), the left
(0, f)–(f, H − f) and the right (W − f, f)–(W, H − f), each its corners
(x0, y0), (x1, y0), (x1, y1), (x0, y1). An edge of an inner grid line is left out when
one merged range holds the cells on both its sides. The drawn edges along a
grid line that meet are one line: the lines are each grid line's runs, the
horizontal grid lines first (top to bottom, each run left to right), then
the vertical ones (left to right, each run top to bottom). Every cell with
words stands, row by row: a merged range's other cells hold none, its words
fill its box; the words are W_w = (sum of advances)/1000 × h wide (the bold
table in the heading row and with a face that has its own typeface and is
bold; the face's typeface, else the project's), their baseline at the box's
middle plus 0.35 h down; left: the box's left plus 0.5 h, right: its right
less 0.5 h less W_w, centred (the heading row, or a centred column): its
middle less W_w/2.

Grips: p, then each column's right end on the top line. Moving grip 0 moves
p; grip k (k ≥ 1) makes column k−1 as wide as the moved point's distance
along the rows from p less the columns before it, refused unless that is
over h/100, and with a frame unless twice the frame is under the new width.

Sizes (Tablo ekle, Yazıya sığdır): every row 2 h deep; every column the
widest of 2 h and its cells' words plus h (a merged range's words widen no
column).

A cell's words, made one line: every control character a space; past 1000
letters cut to 999 and “…”. A column aligns right when every cell under the
heading row (with one) that has words (spaces aside) reads as a number (an
optional sign, digits, an optional point or comma and digits) and there is
one; left otherwise. A file's rows: those after the last with words dropped,
the columns after the last with words dropped, every row as long as the
longest, each cell made one line; at most 10 000 rows, 100 columns and
100 000 cells.

Edits (§6, Tabloyu düzenle): a cell's words made one line, refused for a
cell outside the table or inside a merged range but its top left. Rows
inserted before row `at` (the count: after the last) are empty and as high
as the row there before (the last one's after it); a range starting at or
below `at` moves down by them, one they fall inside (starting above `at`
and ending below it) grows by them. Rows from..from+count deleted (refused
when none stay): a range loses as many rows as it had among them, starts
count rows up when it started below them, at `from` when it started among
them (its words moving to its new top left), else where it was; with no
row left, or one cell, it is no range. Columns alike; a new column is
aligned left when the table has alignments, a deleted one's alignment goes.
Merging a range (inside, two cells or more): refused when a range meets it
without lying inside it; the ranges inside it go; its cells' words, row by
row, left to right, the empty ones left out, joined by a space and made one
line, are its top left's, the other cells' none; it is the last range.
Unmerging a cell's range removes it, its words where they are.

Tabloyu güncelle (§5): the new cells; row i keeps its height when the table
had it, else 2 h; column j is the wider of its old width (when it had one)
and what the new words ask (the sizes' rule, with the ranges kept); the
alignments are the table's when it keeps its column count, else the
source's (none when all left); a range is kept when it lies inside and its
cells but the top left are empty.

Schedules (§3), numbers by the display rule in the project's unit:
- Koordinat çizelgesi: a row for every place the objects' points (a point,
  each point of a multi-point object) and their paths' vertices stand at, in
  order; two places 1 µm apart or less are one (the first's place); a place's
  name is the first name given there (a point's label, spaces aside; a
  multi-point object's later points the label and “ (2)”, “ (3)” …), its
  elevation the first given there. The unnamed take 1, 2, … in order, past
  every name the table has. Heading: Nokta, the east axis (Y in a CBS
  project, X in a CAD one), the north axis, Z when a place has an elevation.
- Alan çizelgesi: a row for every area, circle and whole ellipse (its sweep,
  (t1 − t0) mod 2π, a whole turn when 0, within 10⁻¹² of a turn): its label
  or a number (as above), its area (in m², dönüm = 1000 m², ha = 10 000 m²;
  a local project's unit squared), its perimeter (holes and parts too); with
  more than one, a last row Toplam with their areas summed.
- Öznitelik tablosu: a row for every object with a label or attributes: Ad
  (its label) when one of them has a label, then a column for every
  attribute name they have, sorted by code point; a missing value is empty.
"""
import json
import sys
from pathlib import Path

from mpmath import cos, mp, mpf, pi, sin

sys.path.insert(0, str(Path(__file__).resolve().parent))
import measure_cases as measure  # noqa: E402  (exact areas, perimeters and ellipse lengths; the display rule)
import paragraph_cases as paragraph  # noqa: E402  (the typefaces' measured advances)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/table/v1/cases.json"
mp.dps = 50

PAD = mpf("0.5")
DROP = mpf("0.35")
ROW = 2
LEAST = 2
TOUCH = mpf("1e-6")
MAX_ROWS, MAX_COLUMNS, MAX_CELLS, MAX_LETTERS = 10_000, 100, 100_000, 1000


def v(x, y):
    return {"x": float(x), "y": float(y)}


# ── Layout ──────────────────────────────────────────────────────────────


def words_width(words, font, bold, h):
    return mpf(sum(paragraph.advance(font, c, bold) for c in words)) / 1000 * mpf(h)


def axes(t):
    r = mpf(t["rotation"]) * pi / 180
    u = (cos(r), sin(r))
    w = (-sin(r), cos(r))
    p = (mpf(t["p"]["x"]), mpf(t["p"]["y"]))

    def at(x, y):
        return (p[0] + u[0] * x - w[0] * y, p[1] + u[1] * x - w[1] * y)

    return at, u


def holds(m, i, j):
    return m["row"] <= i < m["row"] + m["rows"] and m["col"] <= j < m["col"] + m["cols"]


def joined(t, a, b):
    return any(holds(m, *a) and holds(m, *b) for m in t.get("merges", []))


def sums(sizes):
    out = [mpf(0)]
    for s in sizes:
        out.append(out[-1] + mpf(s))
    return out


def runs(drawn):
    out, start = [], None
    for j, d in enumerate(drawn + [False]):
        if d and start is None:
            start = j
        elif not d and start is not None:
            out.append((start, j))
            start = None
    return out


def layout(t, font):
    at, _ = axes(t)
    n, m = len(t["rows"]), len(t["columns"])
    xs, ys = sums(t["columns"]), sums(t["rows"])
    grid = t.get("grid")
    band = t.get("frame") if grid != "none" else None
    lines = []
    if grid != "none":
        for k in range(n + 1):
            outer = k in (0, n)
            if (not outer and grid == "outer") or (outer and band is not None):
                continue
            drawn = [outer or not joined(t, (k - 1, j), (k, j)) for j in range(m)]
            for a, b in runs(drawn):
                lines.append([at(xs[a], ys[k]), at(xs[b], ys[k])])
        for k in range(m + 1):
            outer = k in (0, m)
            if (not outer and grid in ("outer", "rows")) or (outer and band is not None):
                continue
            drawn = [outer or not joined(t, (i, k - 1), (i, k)) for i in range(n)]
            for a, b in runs(drawn):
                lines.append([at(xs[k], ys[a]), at(xs[k], ys[b])])
    own = t.get("font")
    typeface = own or font
    h = mpf(t["height"])
    cells = []
    for i, row in enumerate(t["cells"]):
        for j, words in enumerate(row):
            if not words:
                continue
            span = next((g for g in t.get("merges", []) if holds(g, i, j)), None)
            if span is not None and (span["row"], span["col"]) != (i, j):
                continue
            rows, cols = (span["rows"], span["cols"]) if span else (1, 1)
            x0, x1, y0, y1 = xs[j], xs[j + cols], ys[i], ys[i + rows]
            heading = bool(t.get("header")) and i == 0
            bold = heading or (own is not None and bool(t.get("bold")))
            w = words_width(words, typeface, bold, h)
            align = "center" if heading else (t.get("aligns") or ["left"] * len(t["columns"]))[j]
            if align == "center":
                x = (x0 + x1) / 2 - w / 2
            elif align == "right":
                x = x1 - PAD * h - w
            else:
                x = x0 + PAD * h
            y = (y0 + y1) / 2 + DROP * h
            cells.append({"row": i, "col": j, "at": v(*at(x, y)), "width": float(w), "bold": bold})
    W, H = xs[-1], ys[-1]
    frame = []
    if band is not None:
        f = mpf(band)

        def quad(x0, y0, x1, y1):
            return [v(*at(x0, y0)), v(*at(x1, y0)), v(*at(x1, y1)), v(*at(x0, y1))]

        frame = [quad(0, 0, W, f), quad(0, H - f, W, H), quad(0, f, f, H - f), quad(W - f, f, W, H - f)]
    return {
        "outline": [v(*at(0, 0)), v(*at(W, 0)), v(*at(W, H)), v(*at(0, H))],
        "lines": [[v(*a), v(*b)] for a, b in lines],
        "frame": frame,
        "cells": cells,
    }


def grips(t):
    at, _ = axes(t)
    xs = sums(t["columns"])
    return [v(mpf(t["p"]["x"]), mpf(t["p"]["y"]))] + [v(*at(x, 0)) for x in xs[1:]]


def moved(t, index, to):
    if index == 0:
        return {"p": to, "columns": t["columns"]}
    _, u = axes(t)
    j = index - 1
    left = sum((mpf(c) for c in t["columns"][:j]), mpf(0))
    along = (mpf(to["x"]) - mpf(t["p"]["x"])) * u[0] + (mpf(to["y"]) - mpf(t["p"]["y"])) * u[1]
    w = along - left
    if not w > mpf(t["height"]) / 100:
        return None
    columns = list(t["columns"])
    columns[j] = float(w)
    if t.get("frame") is not None and not 2 * mpf(t["frame"]) < sum((mpf(c) for c in columns), mpf(0)):
        return None
    return {"p": t["p"], "columns": columns}


def sizes(t, font):
    h = mpf(t["height"])
    typeface = t.get("font") or font
    widths = [LEAST * h] * len(t["columns"])
    for i, row in enumerate(t["cells"]):
        for j, words in enumerate(row):
            if not words or any(holds(g, i, j) for g in t.get("merges", [])):
                continue
            bold = (bool(t.get("header")) and i == 0) or (t.get("font") is not None and bool(t.get("bold")))
            w = words_width(words, typeface, bold, h) + 2 * PAD * h
            widths[j] = max(widths[j], w)
    return {"rows": [float(ROW * h)] * len(t["rows"]), "columns": [float(w) for w in widths]}


# ── Edits ───────────────────────────────────────────────────────────────


def grown(start, length, at, count):
    if start >= at:
        return start + count, length
    if start < at < start + length:
        return start, length + count
    return start, length


def shrunk(start, length, frm, count):
    gone = max(0, min(start + length, frm + count) - max(start, frm))
    if start >= frm + count:
        new = start - count
    elif start >= frm:
        new = frm
    else:
        new = start
    return new, length - gone


def edit(t, e):
    rows, columns = list(t["rows"]), list(t["columns"])
    cells = [list(r) for r in t["cells"]]
    merges = [dict(r) for r in t.get("merges") or []]
    aligns = list(t["aligns"]) if t.get("aligns") is not None else None
    n, m = len(rows), len(columns)

    def refused(why):
        return {"problem": why}

    def holder(i, j):
        return next((k for k, g in enumerate(merges) if holds(g, i, j)), None)

    kind = e["kind"]
    if kind == "setCell":
        i, j = e["row"], e["col"]
        if i >= n or j >= m:
            return refused("Hücre tablonun dışında.")
        k = holder(i, j)
        if k is not None and (merges[k]["row"], merges[k]["col"]) != (i, j):
            return refused("Bu hücre birleşik bir alanın içinde; yazısı sol üst hücresindedir.")
        cells[i][j] = one_line(e["words"])["words"]
    elif kind in ("insertRows", "insertColumns"):
        at, count = e["at"], e["count"]
        rows_kind = kind == "insertRows"
        size = n if rows_kind else m
        if at > size or count == 0:
            return refused("Satırların yeri tablonun dışında." if rows_kind else "Sütunların yeri tablonun dışında.")
        if rows_kind and (n + count > MAX_ROWS or (n + count) * m > MAX_CELLS):
            return refused("Tablo bu kadar satır alamaz.")
        if not rows_kind and (m + count > MAX_COLUMNS or n * (m + count) > MAX_CELLS):
            return refused("Tablo bu kadar sütun alamaz.")
        if rows_kind:
            h = rows[min(at, n - 1)]
            rows[at:at] = [h] * count
            cells[at:at] = [[""] * m for _ in range(count)]
            for g in merges:
                g["row"], g["rows"] = grown(g["row"], g["rows"], at, count)
        else:
            w = columns[min(at, m - 1)]
            columns[at:at] = [w] * count
            for r in cells:
                r[at:at] = [""] * count
            if aligns is not None:
                aligns[at:at] = ["left"] * count
            for g in merges:
                g["col"], g["cols"] = grown(g["col"], g["cols"], at, count)
    elif kind in ("deleteRows", "deleteColumns"):
        frm, count = e["from"], e["count"]
        rows_kind = kind == "deleteRows"
        size = n if rows_kind else m
        if count == 0 or frm + count > size:
            return refused("Silinecek satırlar tablonun dışında." if rows_kind else "Silinecek sütunlar tablonun dışında.")
        if count == size:
            return refused("Tablonun en az bir satırı kalmalı." if rows_kind else "Tablonun en az bir sütunu kalmalı.")
        axis, span = ("row", "rows") if rows_kind else ("col", "cols")
        kept = []
        for g in merges:
            words = cells[g["row"]][g["col"]]
            lost = frm <= g[axis] < frm + count
            start, length = shrunk(g[axis], g[span], frm, count)
            if length == 0:
                continue
            h = dict(g)
            h[axis], h[span] = start, length
            kept.append((h, words if lost else ""))
        if rows_kind:
            del rows[frm:frm + count]
            del cells[frm:frm + count]
        else:
            del columns[frm:frm + count]
            for r in cells:
                del r[frm:frm + count]
            if aligns is not None:
                del aligns[frm:frm + count]
        merges = []
        for h, words in kept:
            if words:
                cells[h["row"]][h["col"]] = words
            if h["rows"] * h["cols"] >= 2:
                merges.append(h)
    elif kind == "merge":
        r = {k: e[k] for k in ("row", "col", "rows", "cols")}
        if r["rows"] == 0 or r["cols"] == 0 or r["row"] + r["rows"] > n or r["col"] + r["cols"] > m:
            return refused("Birleştirilecek alan tablonun dışında.")
        if r["rows"] * r["cols"] < 2:
            return refused("Birleştirmek için birden çok hücre seçin.")

        def inside(o):
            return o["row"] >= r["row"] and o["col"] >= r["col"] and o["row"] + o["rows"] <= r["row"] + r["rows"] and o["col"] + o["cols"] <= r["col"] + r["cols"]

        def meets(o):
            return o["row"] < r["row"] + r["rows"] and r["row"] < o["row"] + o["rows"] and o["col"] < r["col"] + r["cols"] and r["col"] < o["col"] + o["cols"]

        if any(meets(o) and not inside(o) for o in merges):
            return refused("Seçilen alan birleşik bir alanı yarıda kesiyor; önce onu ayırın ya da tamamını seçin.")
        merges = [o for o in merges if not inside(o)]
        words = []
        for i in range(r["row"], r["row"] + r["rows"]):
            for j in range(r["col"], r["col"] + r["cols"]):
                if cells[i][j]:
                    words.append(cells[i][j])
                cells[i][j] = ""
        cells[r["row"]][r["col"]] = one_line(" ".join(words))["words"]
        merges.append(r)
    elif kind == "unmerge":
        k = holder(e["row"], e["col"])
        if k is None:
            return refused("Bu hücre birleşik değil.")
        del merges[k]
    return {"rows": rows, "columns": columns, "cells": cells, "merges": merges, "aligns": aligns}


def refresh(t, cells, new_aligns, font):
    h = mpf(t["height"])
    n, m = len(cells), (len(cells[0]) if cells else 0)
    old_rows, old_cols = t["rows"], t["columns"]
    if m == len(old_cols):
        aligns = list(t["aligns"]) if t.get("aligns") is not None else None
    else:
        aligns = list(new_aligns) if any(a != "left" for a in new_aligns) else None
    merges = []
    for g in t.get("merges") or []:
        if g["row"] + g["rows"] <= n and g["col"] + g["cols"] <= m and all(
            (i, j) == (g["row"], g["col"]) or cells[i][j] == ""
            for i in range(g["row"], g["row"] + g["rows"])
            for j in range(g["col"], g["col"] + g["cols"])
        ):
            merges.append(dict(g))
    rows = [old_rows[i] if i < len(old_rows) else float(ROW * h) for i in range(n)]
    fitted = sizes({**t, "rows": rows, "columns": [0.0] * m, "cells": cells, "merges": merges}, font)["columns"]
    columns = [max(old_cols[j], fitted[j]) if j < len(old_cols) else fitted[j] for j in range(m)]
    return {"rows": rows, "columns": columns, "cells": cells, "merges": merges, "aligns": aligns}


# ── Cells ───────────────────────────────────────────────────────────────


def is_control(c):
    import unicodedata

    return unicodedata.category(c) == "Cc"


def one_line(words):
    out = "".join(" " if is_control(c) else c for c in words)
    if len(out) > MAX_LETTERS:
        out = out[: MAX_LETTERS - 1] + "…"
    return {"words": out, "changed": out != words}


def is_number(words):
    s = words.strip(" ")
    if s[:1] in ("+", "-"):
        s = s[1:]
    cut = min([i for i in (s.find("."), s.find(",")) if i >= 0], default=-1)
    whole, part = (s[:cut], s[cut + 1:]) if cut >= 0 else (s, None)

    def digits(t):
        return t != "" and all("0" <= c <= "9" for c in t)

    return digits(whole) and (part is None or digits(part))


def aligns(cells, header):
    m = max((len(r) for r in cells), default=0)
    body = cells[1:] if header else cells
    out = []
    for j in range(m):
        words = [r[j] for r in body if j < len(r) and r[j].strip(" ")]
        out.append("right" if words and all(is_number(w) for w in words) else "left")
    return out


def refused(problem):
    return {"cells": [], "aligns": [], "fitted": 0, "problem": problem}


def as_cells(rows, header):
    n = len(rows)
    m = max((len(r) for r in rows), default=0)
    if n == 0 or m == 0:
        return refused("Tabloya yazılacak satır yok.")
    if n > MAX_ROWS:
        return refused(f"Tablo {n} satır olur; bir tablo en çok {MAX_ROWS} satırdır. Daha azını seçin.")
    if m > MAX_COLUMNS:
        return refused(f"Tablo {m} sütun olur; bir tablo en çok {MAX_COLUMNS} sütundur.")
    if n * m > MAX_CELLS:
        return refused(f"Tablo {n * m} hücre olur; bir tablo en çok {MAX_CELLS} hücredir. Daha azını seçin.")
    fitted = 0
    cells = []
    for r in rows:
        out = []
        for w in r:
            line = one_line(w)
            fitted += line["changed"]
            out.append(line["words"])
        cells.append(out + [""] * (m - len(out)))
    return {"cells": cells, "aligns": aligns(cells, header), "fitted": fitted, "problem": None}


def from_rows(rows, header):
    rows = [list(r) for r in rows]
    while rows and not any(w.strip(" ") for w in rows[-1]):
        rows.pop()
    m = max((max((j + 1 for j, w in enumerate(r) if w.strip(" ")), default=0) for r in rows), default=0)
    return as_cells([r[:m] for r in rows], header)


# ── Schedules ───────────────────────────────────────────────────────────


def length_text(metres, u):
    k = {"m": 1, "cm": 100, "mm": 1000}[u["unit"]]
    return measure.shown(mpf(metres) * k, u["lengthDecimals"])


def area_text(m2, u):
    d = u["areaDecimals"]
    if u["unit"] != "m":
        k = {"cm": 100, "mm": 1000}[u["unit"]]
        return measure.shown(m2 * k * k, d)
    return measure.shown(m2 / {"m2": 1, "donum": 1000, "ha": 10_000}[u["areaUnit"]], d)


def area_label(u):
    if u["unit"] != "m":
        return u["unit"] + "²"
    return {"m2": "m²", "donum": "dönüm", "ha": "ha"}[u["areaUnit"]]


def name_of(o):
    label = (o.get("label") or "").strip(" ")
    return label or None


def numbering(taken):
    n = 1
    while True:
        if str(n) not in taken:
            yield str(n)
        n += 1


def coordinates(objects, u):
    places = []

    def add(p, z, name):
        for place in places:
            if (place["p"][0] - p[0]) ** 2 + (place["p"][1] - p[1]) ** 2 <= TOUCH * TOUCH:
                place["z"] = place["z"] if place["z"] is not None else z
                place["name"] = place["name"] if place["name"] is not None else name
                return
        places.append({"p": p, "z": z, "name": name})

    for o in objects:
        s = o["shape"]
        if s["kind"] == "point":
            label = name_of(o)
            points = [(s["p"], s.get("z"))] + [(q["p"], q.get("z")) for q in s.get("parts") or []]
            for k, (p, z) in enumerate(points):
                name = None if label is None else (label if k == 0 else f"{label} ({k + 1})")
                add((mpf(p["x"]), mpf(p["y"])), z, name)
        else:
            for path in o.get("paths") or []:
                for k, p in enumerate(path["pts"]):
                    zs = path.get("zs") or []
                    add((mpf(p["x"]), mpf(p["y"])), zs[k] if k < len(zs) else None, None)
    if not places:
        return refused("Seçili nesnelerde nokta ya da köşe yok; koordinat çizelgesi noktalardan, çizgilerden ve alanlardan yazılır.")
    numbers = numbering({p["name"] for p in places if p["name"] is not None})
    elevated = any(p["z"] is not None for p in places)
    east, north = ("X", "Y") if u["axes"] == "cad" else ("Y", "X")
    rows = [["Nokta", east, north] + (["Z"] if elevated else [])]
    for p in places:
        row = [p["name"] if p["name"] is not None else next(numbers), length_text(p["p"][0], u), length_text(p["p"][1], u)]
        if elevated:
            row.append(length_text(p["z"], u) if p["z"] is not None else "")
        rows.append(row)
    return as_cells(rows, True)


def area_and_perimeter(s):
    kind = s["kind"]
    if kind == "polygon":
        return measure.polygon_area(s), measure.polygon_perimeter(s)
    if kind == "circle":
        r = mpf(s["r"])
        return pi * r * r, 2 * pi * r
    if kind == "ellipse":
        if measure.turn(s["t0"], s["t1"]) < 2 * pi - mpf("1e-12"):
            return None
        a = (mpf(s["major"]["x"]) ** 2 + mpf(s["major"]["y"]) ** 2) ** mpf("0.5")
        return pi * a * a * mpf(s["ratio"]), measure.ellipse_length(s)
    return None


def areas(objects, u):
    found = [(o, *ap) for o in objects for ap in [area_and_perimeter(o["shape"])] if ap is not None]
    if not found:
        return refused("Seçili nesnelerde alan yok; alan çizelgesi alanlardan, dairelerden ve elipslerden yazılır.")
    numbers = numbering({name_of(o) for o, _, _ in found if name_of(o) is not None})
    rows = [["Ad", f"Alan ({area_label(u)})", f"Çevre ({u['unit']})"]]
    total = mpf(0)
    for o, a, p in found:
        rows.append([name_of(o) or next(numbers), area_text(a, u), length_text(p, u)])
        total += a
    if len(found) > 1:
        rows.append(["Toplam", area_text(total, u), ""])
    return as_cells(rows, True)


def attributes(objects):
    listed = [o for o in objects if name_of(o) is not None or o.get("attrs")]
    if not listed:
        return refused("Seçili nesnelerin özniteliği ya da adı yok.")
    names = sorted({k for o in listed for k in (o.get("attrs") or {})})
    labelled = any(name_of(o) is not None for o in listed)
    rows = [(["Ad"] if labelled else []) + names]
    for o in listed:
        attrs = o.get("attrs") or {}
        rows.append(([name_of(o) or ""] if labelled else []) + [attrs.get(k, "") for k in names])
    return as_cells(rows, True)


def schedule(kind, objects, u):
    if kind == "coordinates":
        return coordinates(objects, u)
    if kind == "areas":
        return areas(objects, u)
    return attributes(objects)


# ── The cases ───────────────────────────────────────────────────────────

PLAIN = {
    "kind": "table",
    "p": v(10, 20),
    "rotation": 0.0,
    "height": 2.5,
    "rows": [6.0, 5.0, 5.0],
    "columns": [20.0, 32.0, 32.0],
    "cells": [["Nokta", "Y", "X"], ["101", "487000.125", "4420000.500"], ["P2", "10.5", ""]],
    "aligns": ["left", "right", "right"],
    "header": True,
}

MERGED = {
    "kind": "table",
    "p": v(-5, 40),
    "rotation": 0.0,
    "height": 1.8,
    "rows": [4.0, 4.0, 4.0, 4.0],
    "columns": [12.0, 10.0, 14.0],
    "cells": [
        ["Parsel listesi", "", ""],
        ["A", "1", "Arsa"],
        ["", "2", "Tarla"],
        ["Toplam", "", ""],
    ],
    "merges": [
        {"row": 0, "col": 0, "rows": 1, "cols": 3},
        {"row": 1, "col": 0, "rows": 2, "cols": 1},
        {"row": 3, "col": 0, "rows": 1, "cols": 2},
    ],
    "aligns": ["left", "center", "right"],
    "header": True,
}

BLOCK = {
    "kind": "table",
    "p": v(0, 0),
    "rotation": 0.0,
    "height": 1.0,
    "rows": [3.0, 3.0, 3.0],
    "columns": [5.0, 5.0, 5.0],
    "cells": [["Blok", "", "c"], ["", "", "f"], ["g", "h", "i"]],
    "merges": [{"row": 0, "col": 0, "rows": 2, "cols": 2}],
}

TURNED = {
    "kind": "table",
    "p": v(487000.0, 4420000.0),
    "rotation": 30.0,
    "height": 2.0,
    "rows": [4.0, 4.0],
    "columns": [15.0, 25.0],
    "cells": [["Ad", "Değer"], ["Şerh", "-12,75"]],
}

FACED = {
    "kind": "table",
    "p": v(100, -50),
    "rotation": -90.0,
    "height": 3.0,
    "rows": [7.0, 7.0],
    "columns": [30.0, 18.0],
    "cells": [["Çizelge", "Ölçü"], ["İğde", "ğüşiöç"]],
    "aligns": ["center", "right"],
    "header": True,
    "font": "arimo",
    "bold": True,
    "italic": True,
}


NARROW = {
    "kind": "table",
    "p": v(0, 0),
    "rotation": 0.0,
    "height": 1.0,
    "rows": [10.0],
    "columns": [6.0, 4.0],
    "cells": [["a", "b"]],
    "frame": 4.5,
}


def with_(t, **kw):
    out = dict(t)
    out.update(kw)
    return out


LAYOUTS = [
    ("üç satır, başlık ve sağa dayalı sayılar", PLAIN, "barlow"),
    ("projenin yazı tipi başka", PLAIN, "plex-mono"),
    ("birleşik alanlar: başlık, iki satır, iki sütun", MERGED, "barlow"),
    ("birleşik alanlar yalnız dış çizgiyle", with_(MERGED, grid="outer"), "barlow"),
    ("birleşik alanlar satır çizgileriyle", with_(MERGED, grid="rows"), "barlow"),
    ("çizgisiz", with_(PLAIN, grid="none"), "barlow"),
    ("iki satır iki sütunluk blok", BLOCK, "overpass"),
    ("30° döndürülmüş, uzak koordinatta", TURNED, "barlow"),
    ("kendi yazı tipi kalın, −90°", FACED, "barlow"),
    ("kendi yazı tipi kalın değil", with_(FACED, bold=False), "barlow"),
    ("başlıksız, hizasız", with_(PLAIN, header=False, aligns=None), "quicksand"),
    ("kalın çerçeve", with_(PLAIN, frame=0.5), "barlow"),
    ("kalın çerçeve, satır çizgileri, birleşik alanlar", with_(MERGED, grid="rows", frame=0.4), "barlow"),
    ("kalın çerçeve, döndürülmüş", with_(TURNED, frame=0.25), "barlow"),
    ("çizgisiz tabloda çerçeve yok", with_(FACED, grid="none", frame=1.0), "barlow"),
]

GRIPS = [
    ("köşe taşınır", PLAIN, 0, v(0, 0)),
    ("ikinci sütunun ucu", PLAIN, 2, v(70.5, 13)),
    ("ilk sütunun ucu kısalır", PLAIN, 1, v(22, 99)),
    ("ilk sütun sıfıra yakın: ret", PLAIN, 1, v(10.02, 20)),
    ("sola: ret", PLAIN, 1, v(5, 20)),
    ("döndürülmüş tablonun sütunu", TURNED, 2, v(487030.0, 4420020.0)),
    ("kalın çerçeve sığar", with_(PLAIN, frame=2.0), 1, v(14, 20)),
    ("kalın çerçeve sığmaz: ret", NARROW, 1, v(0.5, 0)),
]

SIZES = [
    ("başlık kalın, sayılar", PLAIN, "barlow"),
    ("birleşik alanın yazısı sütun genişletmez", MERGED, "barlow"),
    ("boş sütun en az 2 yükseklik", with_(BLOCK, cells=[["", "", ""], ["", "", ""], ["g", "", "i"]], merges=[]), "barlow"),
    ("kendi yazı tipi kalın", FACED, "barlow"),
]

GIS = {"axes": "gis", "unit": "m", "areaUnit": "m2", "lengthDecimals": 3, "areaDecimals": 2}
CAD_MM = {"axes": "cad", "unit": "mm", "areaUnit": "m2", "lengthDecimals": 1, "areaDecimals": 0}


def point(x, y, z=None, label=None, parts=None, attrs=None):
    s = {"kind": "point", "p": v(x, y)}
    if z is not None:
        s["z"] = z
    if parts:
        s["parts"] = [{"p": v(*q[:2]), **({"z": q[2]} if len(q) > 2 else {})} for q in parts]
    return {"shape": s, "label": label, "attrs": attrs or {}}


def path(pts, zs=None, closed=False):
    return {"pts": [v(*p) for p in pts], "closed": closed, "zs": zs if zs is not None else [None] * len(pts)}


def polygon(pts, label=None, holes=None, bulges=None, parts=None, attrs=None, zs=None):
    s = {"kind": "polygon", "pts": [v(*p) for p in pts]}
    if bulges:
        s["bulges"] = bulges
    if holes:
        s["holes"] = [{"pts": [v(*p) for p in h]} for h in holes]
    if parts:
        s["parts"] = [{"pts": [v(*p) for p in q]} for q in parts]
    paths = [path(pts, zs, True)] + [path(h, None, True) for h in holes or []] + [path(q, None, True) for q in parts or []]
    return {"shape": s, "paths": paths, "label": label, "attrs": attrs or {}}


def line(a, b, za=None, zb=None):
    return {"shape": {"kind": "line", "a": v(*a), "b": v(*b)}, "paths": [path([a, b], [za, zb])], "label": None, "attrs": {}}


def other(shape, label=None, attrs=None):
    return {"shape": shape, "paths": [], "label": label, "attrs": attrs or {}}


E0, N0 = 487000.0, 4420000.0
SQUARE = [(E0, N0), (E0 + 20, N0), (E0 + 20, N0 + 20), (E0, N0 + 20)]

SCHEDULES = [
    (
        "koordinat: adlı nokta köşede, ortak köşe, çok noktalı, numaralar adları atlar",
        "coordinates",
        [
            polygon(SQUARE, zs=[100.0, None, 101.25, None]),
            point(E0 + 20, N0, z=99.5, label=" 101 "),
            line((E0 + 20, N0 + 20), (E0 + 35.125, N0 + 27.0625), za=None, zb=103.0),
            point(E0 + 50, N0 + 50, label="K", parts=[(E0 + 51, N0 + 50, 104.0), (E0 + 52, N0 + 50)]),
            point(E0 + 0.0000004, N0 - 0.0000003, label="2"),
            point(E0 - 10.0005, N0 - 10.0015),
            other({"kind": "circle", "c": v(E0, N0), "r": 5.0}),
        ],
        GIS,
    ),
    (
        "koordinat: CAD projesi, milimetre, kotsuz",
        "coordinates",
        [
            {"shape": {"kind": "polyline", "pts": [v(0, 0), v(1.25, 0.5), v(2.5, -0.75)]}, "paths": [path([(0, 0), (1.25, 0.5), (2.5, -0.75)])], "label": None, "attrs": {}},
            point(0.00005, 0.00005, label="A"),
        ],
        CAD_MM,
    ),
    (
        "koordinat: noktası ya da köşesi olmayan nesneler",
        "coordinates",
        [other({"kind": "circle", "c": v(0, 0), "r": 1.0}), other({"kind": "text", "p": v(0, 0), "text": "x", "height": 1.0, "rotation": 0.0})],
        GIS,
    ),
    (
        "alan: delikli parsel, daire, tam elips, yaylı ve çok parçalı alan; yarım elips ve tarama dışarıda",
        "areas",
        [
            polygon(SQUARE, label="5", holes=[[(E0 + 5, N0 + 5), (E0 + 5, N0 + 10), (E0 + 10, N0 + 10), (E0 + 10, N0 + 5)]]),
            other({"kind": "circle", "c": v(E0, N0), "r": 7.5}),
            other({"kind": "ellipse", "c": v(0, 0), "major": v(6, 8), "ratio": 0.5, "t0": 0.0, "t1": 6.283185307179586}),
            other({"kind": "ellipse", "c": v(0, 0), "major": v(6, 8), "ratio": 0.5, "t0": 0.0, "t1": 3.141592653589793}),
            polygon([(0, 0), (10, 0), (10, 10), (0, 10)], label="1", bulges=[0.0, 0.0, 1.0, 0.0]),
            polygon([(0, 0), (4, 0), (4, 3)], parts=[[(10, 10), (13, 10), (13, 14)]]),
            other({"kind": "hatch", "ring": [v(0, 0), v(1, 0), v(1, 1)], "pattern": {"type": "solid", "angle": 0.0, "spacing": 1.0}}),
        ],
        GIS,
    ),
    (
        "alan: dönüm, tek alan, toplam yok",
        "areas",
        [polygon(SQUARE, label="12/3")],
        {"axes": "gis", "unit": "m", "areaUnit": "donum", "lengthDecimals": 2, "areaDecimals": 3},
    ),
    (
        "alan: hektar",
        "areas",
        [polygon([(0, 0), (150, 0), (150, 80), (0, 80)]), polygon([(200, 0), (260, 0), (260, 45)], label="B")],
        {"axes": "gis", "unit": "m", "areaUnit": "ha", "lengthDecimals": 2, "areaDecimals": 4},
    ),
    (
        "alan: CAD projesi, santimetre",
        "areas",
        [polygon([(0, 0), (1.5, 0), (1.5, 0.8), (0, 0.8)], label="Oda"), other({"kind": "circle", "c": v(0, 0), "r": 0.25})],
        {"axes": "cad", "unit": "cm", "areaUnit": "m2", "lengthDecimals": 1, "areaDecimals": 1},
    ),
    (
        "alan: alanı olmayan nesneler",
        "areas",
        [line((0, 0), (1, 1)), point(0, 0, label="1")],
        GIS,
    ),
    (
        "öznitelik: adlar sıralı, eksik değer boş, adsız ve özniteliksiz nesne dışarıda",
        "attributes",
        [
            polygon(SQUARE, label="5", attrs={"Parsel": "5", "Ada": "101", "Nitelik": "Arsa", "Şerh": "Var"}),
            polygon(SQUARE, attrs={"Parsel": "6", "Ada": "101", "Çap": "12.5"}),
            line((0, 0), (1, 1)),
            point(0, 0, label="Taş", attrs={"Kod": "SN"}),
        ],
        GIS,
    ),
    (
        "öznitelik: adsız nesneler, çok satırlı değer tek satır",
        "attributes",
        [point(0, 0, attrs={"Not": "birinci\nikinci"}), point(1, 1, attrs={"Not": "kısa", "No": "-3"})],
        GIS,
    ),
    (
        "öznitelik: hiçbirinin ne adı ne özniteliği var",
        "attributes",
        [line((0, 0), (1, 1)), point(0, 0, label="  ")],
        GIS,
    ),
]

ROWS = [
    (
        "sondaki boş satır ve sütun düşer, satırlar en uzunu kadar",
        [["Ad", "Değer", "Not", ""], ["a", "1,5", "x"], ["b", "-2"], ["c", "+3.25", "", " "], [" ", ""], []],
        True,
    ),
    ("başlıksız: ilk satır da sayı", [["1", "2"], ["3", "x"]], False),
    ("başlık sayıları saymaz, boş sütun sola", [["No", "Boş"], ["7", ""], ["8", " "]], True),
    ("sayı biçimleri", [["a", "b", "c", "d"], ["1.", ".5", "1,2,3", "12"], ["1e3", "٣", "1 000", "-0"]], True),
    ("tek satıra sığdırılan hücreler", [["a\nb\tc", "x" * 1001, "y" * 1000]], False),
    ("hiç satır yok", [[], ["", " "]], True),
    ("101 sütun", [["x"] * 101], True),
    ("10 001 satır", [["x"]] * 10_001, False),
    ("100 100 hücre", [[""]] * 1000 + [[""] * 99 + ["x"]], False),
]

EDITED = {
    "kind": "table",
    "p": v(0, 0),
    "rotation": 0.0,
    "height": 1.0,
    "rows": [2.0, 2.5, 3.0, 2.0],
    "columns": [6.0, 4.0, 5.0],
    "cells": [["Ad", "Değer", "Not"], ["A", "1", "x"], ["", "2", ""], ["b", "3", "son"]],
    "merges": [{"row": 1, "col": 0, "rows": 2, "cols": 1}],
    "aligns": ["left", "right", "center"],
    "header": True,
}

EDITS = [
    ("hücre yazılır", EDITED, {"kind": "setCell", "row": 3, "col": 2, "words": "yeni"}),
    ("birleşik alanın sol üstü, satır sonu boşluk", EDITED, {"kind": "setCell", "row": 1, "col": 0, "words": "B\nC"}),
    ("birleşik alanın içi: ret", EDITED, {"kind": "setCell", "row": 2, "col": 0, "words": "z"}),
    ("tablonun dışı: ret", EDITED, {"kind": "setCell", "row": 4, "col": 0, "words": "z"}),
    ("başa satır, birleşik alan kayar", EDITED, {"kind": "insertRows", "at": 0, "count": 1}),
    ("birleşik alanın içine iki satır, alan büyür", EDITED, {"kind": "insertRows", "at": 2, "count": 2}),
    ("sona satır, sonuncunun yüksekliği", EDITED, {"kind": "insertRows", "at": 4, "count": 1}),
    ("birleşik alanın üst satırı silinir, yazısı kalır", EDITED, {"kind": "deleteRows", "from": 1, "count": 1}),
    ("bütün satırlar: ret", EDITED, {"kind": "deleteRows", "from": 0, "count": 4}),
    ("iki sütun eklenir, sola hizalı", EDITED, {"kind": "insertColumns", "at": 1, "count": 2}),
    ("ilk sütun silinir, birleşik alan gider", EDITED, {"kind": "deleteColumns", "from": 0, "count": 1}),
    ("2 × 2 birleştir, içteki alan ve yazılar", EDITED, {"kind": "merge", "row": 1, "col": 0, "rows": 2, "cols": 2}),
    ("birleşik alanı kesen: ret", EDITED, {"kind": "merge", "row": 2, "col": 0, "rows": 2, "cols": 2}),
    ("tek hücre: ret", EDITED, {"kind": "merge", "row": 0, "col": 0, "rows": 1, "cols": 1}),
    ("ayır", EDITED, {"kind": "unmerge", "row": 2, "col": 0}),
    ("birleşik olmayan hücre: ret", EDITED, {"kind": "unmerge", "row": 0, "col": 1}),
    ("çok satır: ret", EDITED, {"kind": "insertRows", "at": 0, "count": 9_997}),
]

REFRESH = [
    (
        "aynı biçim, değerler değişir: sütun eskisinden darsa genişler",
        EDITED,
        [["Ad", "Değer", "Not"], ["A", "1234567.890", "x"], ["", "2", ""], ["b", "3", ""]],
        ["left", "right", "left"],
        "barlow",
    ),
    (
        "satır eklenir, sütun azalır: hizalar kaynaktan, alan sığar",
        EDITED,
        [["Ad", "Değer"], ["A", "1"], ["", "2"], ["b", "3"], ["c", "4"]],
        ["left", "right"],
        "barlow",
    ),
    (
        "birleşik alanın içine yazı gelir: alan düşer",
        EDITED,
        [["Ad", "Değer", "Not"], ["A", "1", "x"], ["a2", "2", ""], ["b", "3", "son"]],
        ["left", "right", "left"],
        "overpass",
    ),
]

ONE_LINE = ["", "Parsel 5", "a\r\nb", "\x00\x1f\x7f\x85", "ğ" * 1000, "ğ" * 1001, "x y"]


def build():
    return {
        "format": "kentos.table-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/table_cases.py",
        "title": "Tablo: yerleşim, tutamaçlar, ölçüler, çizelgeler ve hücreler (ADR 0184)",
        "layout": [{"name": n, "table": t, "font": f, "want": layout(t, f)} for n, t, f in LAYOUTS],
        "grips": [
            {"name": n, "table": t, "grips": grips(t), "index": i, "to": to, "want": moved(t, i, to)}
            for n, t, i, to in GRIPS
        ],
        "sizes": [{"name": n, "table": t, "font": f, "want": sizes(t, f)} for n, t, f in SIZES],
        "schedules": [
            {"name": n, "kind": k, "objects": objs, "units": u, "want": schedule(k, objs, u)}
            for n, k, objs, u in SCHEDULES
        ],
        "rows": [{"name": n, "rows": r, "header": h, "want": from_rows(r, h)} for n, r, h in ROWS],
        "edits": [{"name": n, "table": t, "edit": e, "want": edit(t, e)} for n, t, e in EDITS],
        "refresh": [
            {"name": n, "table": t, "cells": c, "aligns": a, "font": f, "want": refresh(t, c, a, f)}
            for n, t, c, a, f in REFRESH
        ],
        "oneLine": [{"words": w, "want": one_line(w)} for w in ONE_LINE],
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    doc = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text_of(doc):
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("table cases match")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text_of(doc), encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
