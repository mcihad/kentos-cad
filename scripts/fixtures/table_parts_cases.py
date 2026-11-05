#!/usr/bin/env python3
"""The shared cases of a table's parts and cells of several lines (docs/adr/1030 §7; plan/cad-advanced.md CAD-ADV-13).

    python3 scripts/fixtures/table_parts_cases.py           # writes the file
    python3 scripts/fixtures/table_parts_cases.py --check   # writes nothing; compares

Writes fixtures/table/v1/parts.json with Python's standard library (exact fractions), from ADR 1030 §7 alone, no
KentOS code. The geometry core (`geom::table`: `parts`, `layout`, `overflowing`, `fit_depth`) is held to it by
crates/shared/geometry-core/tests/all/table_parts.rs.

The rules, in the table's axes (x along its rows, y down its columns from its top left corner p; the cases are not
turned, so a point is p + (x, −y)):

- A cell's words split at their line breaks; each line with words stands apart, its number `line` from 0; its
  baseline at the box's middle plus 0.35 h down plus (k − (n − 1)/2) · 5/3 h for line k of n; a left aligned line
  starts 0.5 h in from the box's left. (The cases' cells are left aligned by a style over all of them, the heading's
  too, so a line's start does not depend on a typeface's advances; centred and right aligned words are the layout
  reference's, scripts/fixtures/table_cases.py.)
- Parts (breaks: height d, gap g, once): from row 0 a part takes rows while it stays at most d deep, the heading's copy
  first in every part but the first (unless `once`, no heading, or a heading merged with the row under it), at least
  one row, the first part the heading and one row more. A part that would cut a merged range ends at the range's first
  row, or (when that is the part's first row, or the heading's in the first part) after the range; once moved past a
  range it only moves past further ones. Part k stands k · (W + g) along the rows, W the columns' widths.
- Each part is laid out as a table of its rows: its grid lines (horizontal top to bottom, each run left to right, then
  vertical left to right, each run top to bottom; an inner edge left out when one merged range holds both its cells),
  its frame band's four strips (top, bottom, left, right), its cells; part after part. The outline runs from p to the
  last part's right side and down the deepest part.
- Sığmayan hücre: a cell whose line is wider than its box less 0.5 h each side (the cases' lines are measured as their
  letters' count times a stated advance: the overflow cases use a monospaced typeface's 0.6 em), or whose lines are
  deeper than its box (h for the first line, 5/3 h for each further one).
- A row's depth that holds its lines (Yazıya sığdır): 2 h, and 5/3 h more for each further line.
"""
import argparse
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "table" / "v1" / "parts.json"
PAD, DROP, LINE, ROW = F(1, 2), F(35, 100), F(5, 3), 2


def P(x, y):
    return {"x": float(x), "y": float(y)}


def holds(m, i, j):
    return m["row"] <= i < m["row"] + m["rows"] and m["col"] <= j < m["col"] + m["cols"]


def joined(t, a, b):
    return any(holds(m, *a) and holds(m, *b) for m in t.get("merges", []))


def sums(xs):
    out = [F(0)]
    for x in xs:
        out.append(out[-1] + F(x))
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


def parts(t):
    n = len(t["rows"])
    rows = [F(r) for r in t["rows"]]
    b = t.get("breaks")
    if not b:
        return [(list(range(n)), F(0))]
    merges = t.get("merges", [])
    header = bool(t.get("header"))
    again = header and not b.get("once") and not any(m["row"] == 0 and m["rows"] > 1 for m in merges)
    width = sum((F(c) for c in t["columns"]), F(0))
    gap = max(F(b.get("gap", 0)), F(0))
    depth = F(b["height"])
    out, start = [], 0
    while start < n:
        first = not out
        used = rows[0] if (not first and again) else F(0)
        end = start
        while end < n and (end == start or used + rows[end] <= depth):
            used += rows[end]
            end += 1
        if first and header and end == 1 and n > 1:
            end = 2
        on = False
        while True:
            cut = next((m for m in merges if m["row"] < end < m["row"] + m["rows"]), None)
            if cut is None:
                break
            if not on and cut["row"] > start and not (first and header and cut["row"] <= 1):
                end = cut["row"]
            else:
                on = True
                end = cut["row"] + cut["rows"]
        out.append((([0] if (not first and again) else []) + list(range(start, end)), len(out) * (width + gap)))
        start = end
    return out


def layout(t):
    px, py = F(t["p"]["x"]), F(t["p"]["y"])

    def at(x, y):
        return P(px + x, py - y)

    h = F(t["height"])
    m = len(t["columns"])
    xs0 = sums(t["columns"])
    W = xs0[-1]
    grid = t.get("grid")
    band = t.get("frame") if grid != "none" else None
    split = parts(t)
    lines, frame, cells = [], [], []
    for rows_, x in split:
        n = len(rows_)
        xs = [c + x for c in xs0]
        ys = sums([t["rows"][r] for r in rows_])
        if grid != "none":
            for k in range(n + 1):
                outer = k in (0, n)
                if (not outer and grid == "outer") or (outer and band is not None):
                    continue
                drawn = [outer or not joined(t, (rows_[k - 1], j), (rows_[k], j)) for j in range(m)]
                lines += [[at(xs[a], ys[k]), at(xs[b], ys[k])] for a, b in runs(drawn)]
            for k in range(m + 1):
                outer = k in (0, m)
                if (not outer and grid in ("outer", "rows")) or (outer and band is not None):
                    continue
                drawn = [outer or not joined(t, (rows_[i], k - 1), (rows_[i], k)) for i in range(n)]
                lines += [[at(xs[k], ys[a]), at(xs[k], ys[b])] for a, b in runs(drawn)]
        for kk, i in enumerate(rows_):
            for j, words in enumerate(t["cells"][i]):
                if not words:
                    continue
                span = next((g for g in t.get("merges", []) if holds(g, i, j)), None)
                if span is not None and (span["row"], span["col"]) != (i, j):
                    continue
                rs, cs = (span["rows"], span["cols"]) if span else (1, 1)
                x0, y0, y1 = xs[j], ys[kk], ys[kk + rs]
                split_words = words.split("\n")
                count = len(split_words)
                for line, text in enumerate(split_words):
                    if not text:
                        continue
                    y = (y0 + y1) / 2 + DROP * h + (line - F(count - 1, 2)) * LINE * h
                    cells.append({"row": i, "col": j, "line": line, "at": at(x0 + PAD * h, y)})
        if band is not None:
            f = F(band)
            d = ys[-1]

            def quad(a, b, c, e):
                return [at(a, b), at(c, b), at(c, e), at(a, e)]

            frame += [quad(x, 0, x + W, f), quad(x, d - f, x + W, d), quad(x, f, x + f, d - f),
                      quad(x + W - f, f, x + W, d - f)]
    right = split[-1][1] + W
    deep = max(sum((F(t["rows"][r]) for r in rows_), F(0)) for rows_, _ in split)
    return {
        "parts": [{"rows": r, "x": float(x)} for r, x in split],
        "outline": [at(0, 0), at(right, 0), at(right, deep), at(0, deep)],
        "lines": lines,
        "frame": frame,
        "cells": cells,
    }


def overflowing(t, advance):
    """The cells that do not fit, `advance` each letter's width in the text height (a monospaced typeface)."""
    h = F(t["height"])
    xs, ys = sums(t["columns"]), sums(t["rows"])
    out = []
    for i, row in enumerate(t["cells"]):
        for j, words in enumerate(row):
            if not words:
                continue
            span = next((g for g in t.get("merges", []) if holds(g, i, j)), None)
            if span is not None and (span["row"], span["col"]) != (i, j):
                continue
            rs, cs = (span["rows"], span["cols"]) if span else (1, 1)
            lines = words.split("\n")
            wide = any(len(l) * advance * h + 2 * PAD * h > xs[j + cs] - xs[j] for l in lines)
            deep = (len(lines) - 1) * LINE * h + h > ys[i + rs] - ys[i]
            if wide or deep:
                out.append([i, j])
    return out


def depths(t):
    h = F(t["height"])
    return [float((ROW + (max(len(w.split("\n")) for w in row) - 1) * LINE) * h) for row in t["cells"]]


def table(**f):
    t = {"kind": "table", "p": P(10, 20), "rotation": 0.0, "height": 0.25, "rows": [1.0] + [0.5] * 7,
         "columns": [3.0, 2.0], "cells": [["Blok", "Adet"]] + [[c, str(k)] for k, c in enumerate("ABCDEF", 1)]
         + [["Toplam", "21"]], "header": True,
         # Every cell left aligned (the heading's too), so that a line's start is its box's left plus 0.5 h.
         "styles": [{"row": 0, "col": 0, "rows": 8, "cols": 2, "align": "left"}]}
    t.update(f)
    return t


LONG = table(breaks={"height": 2.5, "gap": 1.0})
LINES = {"kind": "table", "p": P(0, 0), "rotation": 0.0, "height": 0.5, "rows": [1.0, 3.0, 1.0],
         "columns": [6.0, 8.0], "cells": [["Kalem", "Açıklama"], ["Kapı", "Ahşap\nTek kanat\n90 × 210"],
                                         ["Masa", "Toplantı\n\nmasası"]],
         "styles": [{"row": 0, "col": 0, "rows": 3, "cols": 2, "align": "left"}]}
MERGED = table(breaks={"height": 2.5, "gap": 1.0}, merges=[{"row": 3, "col": 0, "rows": 2, "cols": 1}],
               cells=[r if i != 4 else ["", r[1]] for i, r in enumerate(table()["cells"])])
LAYOUTS = [
    ("parçalara bölünmüş, başlık her parçada", LONG),
    ("başlık yalnız ilk parçada, aralık yarım metre", table(breaks={"height": 2.5, "gap": 0.5, "once": True})),
    ("parçalar kalın çerçeveli", table(breaks={"height": 2.5, "gap": 1.0}, frame=0.1)),
    ("parçalar yalnız dış çizgili", table(breaks={"height": 2.5, "gap": 1.0}, grid="outer")),
    ("birleşik alan bölünmez", MERGED),
    ("bir satır sığmasa da parça bir satır alır, ilki başlıkla bir satır",
     table(breaks={"height": 0.25, "gap": 0.0})),
    ("başlıksız tablonun parçaları", table(breaks={"height": 1.5, "gap": 2.0}, header=False)),
    ("tek parçaya sığan bölme", table(breaks={"height": 100.0, "gap": 1.0})),
    ("bölmesiz tablo tek parçadır", table()),
    ("çok satırlı hücreler, boş satır atlanır", LINES),
]
OVERFLOWS = [
    ("geniş ve derin hücreler", {"kind": "table", "p": P(0, 0), "rotation": 0.0, "height": 1.0,
                                 "rows": [2.0, 2.0, 4.0], "columns": [4.0, 6.0], "font": "plex-mono",
                                 "cells": [["abc", "abcdefgh"], ["a\nb", "abcd"], ["a\nb", "x"]]}),
]


def build():
    return {
        "format": "kentos.table-parts-cases",
        "version": 1,
        "note": "ADR 1030 §7. Independent: scripts/fixtures/table_parts_cases.py (exact fractions, no KentOS code).",
        "layout": [{"name": n, "table": t, "want": layout(t)} for n, t in LAYOUTS],
        "overflow": [{"name": n, "table": t, "advance": 0.6, "want": overflowing(t, F(3, 5))} for n, t in OVERFLOWS],
        "depths": [{"name": n, "table": t, "want": depths(t)} for n, t in LAYOUTS[-1:]],
    }


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--check", action="store_true")
    args = cli.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"stale: {OUT.relative_to(ROOT)}")
            sys.exit(1)
        print("ok")
        return
    OUT.write_text(text)
    print("written")


if __name__ == "__main__":
    main()
