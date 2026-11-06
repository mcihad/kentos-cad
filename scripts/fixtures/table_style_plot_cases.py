#!/usr/bin/env python3
"""What a table that follows a table style prints (docs/adr/1127 §1), at two plot scales, written from the ADR without
KentOS code: fixtures/table/v1/styled-plot.json, read by crates/native/plot/tests/all/table_styles.rs.

    python3 scripts/fixtures/table_style_plot_cases.py           # writes the file
    python3 scripts/fixtures/table_style_plot_cases.py --check   # writes nothing; compares

The rules, as the ADR gives them. A table style's cell styles are on the paper (mm); a table draws them at the
project's plot scale S, a millimetre S/1000 metres on the ground. Its rows are of three kinds: the title (its first row
when that row is one merged range across every column), the heading (the next row when the table has one) and the data
(the rest). A cell's words are its kind's height, else the table's; they start its kind's horizontal margin in from the
cell's left side (left aligned), else half their height; their baseline is 0.35 heights under the box's middle (ADR
0184 §2). A line between cells takes the look of the cell above it (a horizontal line; the top line the first row's) or
left of it (a vertical line; the left line the first column's); a line through a merged range is not drawn; the runs of
edges on one grid line in one look are one line. A look is the kind's border weight (paper mm; none: the table's) and
colour (none: the table's); a kind that sets neither draws its lines as the table's. The map prints at 1:S, so the
words' size on the paper is the kind's height in mm, a border's width its weight.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/table/v1/styled-plot.json"
DROP = 0.35
PAD = 0.5

ROWS_MM = [9.0, 7.0, 6.0, 6.0]
COLUMNS_MM = [30.0, 20.0]
HEIGHT_MM = 2.0
CELLS = [["Parsel listesi", ""], ["Ada", "Alan"], ["101", "512"], ["102", "640"]]
STYLE = {"id": "ofis", "name": "OFIS",
         "title": {"align": "left", "height": 3.5, "horizontalMargin": 1.0, "borderWeight": 0.7,
                   "borderColor": "#c0392b"},
         "header": {"align": "left", "height": 2.5, "borderColor": "#2244dd"},
         "data": {"align": "left", "horizontalMargin": 1.2, "verticalMargin": 0.8}}
P = (10.0, 20.0)


def kinds():
    """Each row's kind: the title, the heading, the data."""
    out = ["data"] * len(ROWS_MM)
    title = len(COLUMNS_MM) > 1  # its first row is one merged range across every column
    if title:
        out[0] = "title"
    out[1 if title else 0] = "header"
    return out


def case(scale):
    k = scale / 1000.0
    rows = [r * k for r in ROWS_MM]
    columns = [c * k for c in COLUMNS_MM]
    ys = [0.0]
    for r in rows:
        ys.append(ys[-1] + r)
    xs = [0.0]
    for c in columns:
        xs.append(xs[-1] + c)
    kind = kinds()

    def point(x, y):
        return [P[0] + x, P[1] - y]

    labels = []
    for i, row in enumerate(CELLS):
        cell = STYLE[kind[i]]
        for j, words in enumerate(row):
            if not words:
                continue
            span = len(COLUMNS_MM) if i == 0 else 1
            h_mm = cell.get("height", HEIGHT_MM)
            h = h_mm * k
            margin = cell["horizontalMargin"] * k if "horizontalMargin" in cell else PAD * h
            x = xs[j] + margin
            y = (ys[i] + ys[i + 1]) / 2.0 + DROP * h
            labels.append({"text": words, "at": point(x, y), "size": h_mm, "span": span})

    def look(i):
        c = STYLE[kind[i]]
        if "borderWeight" not in c and "borderColor" not in c:
            return None
        return [c.get("borderWeight"), c.get("borderColor")]

    lines = []

    def runs(edges, at):
        start, was = None, None
        for e, here in enumerate(edges + [False]):
            if here is not False and start is not None and here == was:
                continue
            if start is not None:
                lines.append({"line": at(start, e), "look": was})
                start = None
            if here is not False:
                start, was = e, here
    n, m = len(rows), len(columns)
    merged = lambda a, b: a[0] == 0 and b[0] == 0  # the title row's cells are one range
    for r in range(n + 1):
        above = max(r - 1, 0)
        edges = [look(above) if r in (0, n) or not merged((r - 1, j), (r, j)) else False for j in range(m)]
        runs(edges, lambda s, e, r=r: [point(xs[s], ys[r]), point(xs[e], ys[r])])
    for c in range(m + 1):
        left = max(c - 1, 0)
        edges = [look(i) if c in (0, m) or not merged((i, c - 1), (i, c)) else False for i in range(n)]
        runs(edges, lambda s, e, c=c: [point(xs[c], ys[s]), point(xs[c], ys[e])])
    table = {"kind": "table", "id": 1, "attrs": {}, "p": {"x": P[0], "y": P[1]}, "rotation": 0.0,
             "height": HEIGHT_MM * k, "rows": rows, "columns": columns, "cells": CELLS,
             "merges": [{"row": 0, "col": 0, "rows": 1, "cols": 2}], "header": True, "tableStyle": "ofis"}
    center = point(xs[-1] / 2.0, ys[-1] / 2.0)
    return {"scale": scale, "table": table, "center": center, "labels": labels,
            "borders": [l for l in lines if l["look"] is not None],
            "plain": [l["line"] for l in lines if l["look"] is None]}


def document():
    return {"format": "kentos.table-style-plot", "version": 1,
            "note": "ADR 1127 §1. Bağımsız başvuru scripts/fixtures/table_style_plot_cases.py: kurallar ADR'den elle "
                    "yazılır; KentOS kodu yok.",
            "tableStyle": STYLE, "cases": [case(100.0), case(500.0)]}


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("güncel" if ok else f"{OUT.relative_to(ROOT)} güncel değil")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
