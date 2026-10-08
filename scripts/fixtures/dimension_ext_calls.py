#!/usr/bin/env python3
"""Which of a dimension's lines are extension lines (docs/adr/0205 §6), in the frozen call fixtures.

The `layoutDimension` cases of fixtures/geometry/v1/calls-p3-offset-annotation.json were recorded from the
TypeScript core (docs/adr/0008) before the layout said which of its lines are extension lines (`ext`). This script
works the field out from each recorded expectation alone, with Python's standard library and no KentOS code, and
writes it into the case (`--check`: says whether the file holds what it would write):

- an aligned or linear dimension's extension lines (its style none, "aligned" or "linear") are the lines at the
  start of its list that leave a measured point (a or b) half its height (the gap) away from it, along the way from
  the point (the dimension line, after them, runs across that way);
- an angle's are the lines at the start of its list on the ray from its centre (c) through a or b, from half its
  height past the point;
- a radius's, a diameter's and any other's: none.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "geometry" / "v1" / "calls-p3-offset-annotation.json"


def xy(p):
    return (p["x"], p["y"])


def leaves(line, p, gap, scale):
    """Whether `line` starts `gap` from `p` and runs on away from it."""
    s, e = xy(line[0]), xy(line[1])
    d = (s[0] - p[0], s[1] - p[1])
    run = (e[0] - s[0], e[1] - s[1])
    tol = 1e-9 * max(1.0, scale)
    if abs(math.hypot(*d) - gap) > tol:
        return False
    cross = d[0] * run[1] - d[1] * run[0]
    return abs(cross) <= tol * max(1.0, math.hypot(*run)) and d[0] * run[0] + d[1] * run[1] > 0


def ext_of(dim, want):
    style = dim.get("style")
    h = dim["height"]
    gap = 0.5 * h
    a, b = xy(dim["a"]), xy(dim["b"])
    scale = max(abs(v) for v in (*a, *b))
    out = []
    if style in (None, "aligned", "linear"):
        for i, line in enumerate(want["lines"][:2]):
            if any(leaves(line, p, gap, scale) for p in (a, b)) and i == len(out):
                out.append(i)
    elif style == "angular":
        c = xy(dim["c"])
        for i, line in enumerate(want["lines"][:2]):
            starts = []
            for p in (a, b):
                r = math.hypot(p[0] - c[0], p[1] - c[1])
                u = ((p[0] - c[0]) / r, (p[1] - c[1]) / r)
                starts.append((c[0] + u[0] * (r + gap), c[1] + u[1] * (r + gap)))
            s = xy(line[0])
            near = any(math.hypot(s[0] - q[0], s[1] - q[1]) <= 1e-9 * max(1.0, scale) for q in starts)
            if near and i == len(out):
                out.append(i)
    return out


def closing(line: str, start: int) -> int:
    """The place of the brace that closes the object opening at `start` (no braces in the expectations' strings)."""
    depth = 0
    for k in range(start, len(line)):
        if line[k] == "{":
            depth += 1
        elif line[k] == "}":
            depth -= 1
            if depth == 0:
                return k
    raise ValueError("unclosed object")


def main() -> int:
    check = "--check" in sys.argv[1:]
    lines = FILE.read_text(encoding="utf-8").split("\n")
    n = 0
    for i, line in enumerate(lines):
        if '"fn":"layoutDimension"' not in line:
            continue
        c = json.loads(line.strip().rstrip(","))
        if c["expect"] is None:
            continue
        ext = ext_of(c["args"][0], c["expect"])
        n += 1
        if check:
            if c["expect"].get("ext") != ext:
                print(f"{c['name']}: ext {c['expect'].get('ext')} ≠ {ext}; run without --check")
                return 1
            continue
        if "ext" in c["expect"]:
            continue
        # The field goes last in the expectation, written as the file writes its numbers (compact).
        at = line.index('"expect":{') + len('"expect":')
        end = closing(line, at)
        lines[i] = line[:end] + ',"ext":[' + ",".join(str(k) for k in ext) + "]" + line[end:]
    if not check:
        FILE.write_text("\n".join(lines), encoding="utf-8")
    print(f"{FILE.relative_to(ROOT)}: {n} ölçünün uzatma çizgileri {'denetlendi' if check else 'yazıldı'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
