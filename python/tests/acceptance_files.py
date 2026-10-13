"""What an acceptance drawing's DXF and PDF hold, read with the standard library alone (no KentOS
code): the DXF's group codes, the PDF's page sizes and its stroked straight segments. The facts
they are held to are worked out by scripts/fixtures/acceptance_room.py (fixtures/acceptance/v1)."""

from __future__ import annotations

import re
import shutil
import subprocess
import zlib
from typing import Any

MM = 72.0 / 25.4  # PDF points per millimetre


# ------------------------------------------------------------------ DXF


def dxf_entities(data: bytes) -> tuple[dict[str, str], list[list[tuple[int, str]]]]:
    """The header's variables (the value's first group) and the ENTITIES section's entities as
    their (code, value) pairs."""
    lines = data.decode("utf-8").replace("\r\n", "\n").split("\n")
    pairs = [(int(lines[i].strip()), lines[i + 1].strip()) for i in range(0, len(lines) - 1, 2)]
    header: dict[str, str] = {}
    for i, (c, v) in enumerate(pairs):
        if c == 9 and i + 1 < len(pairs):
            header[v] = pairs[i + 1][1]
    start = next(i for i, (c, v) in enumerate(pairs) if c == 2 and v == "ENTITIES")
    end = next(i for i in range(start, len(pairs)) if pairs[i] == (0, "ENDSEC"))
    ents: list[list[tuple[int, str]]] = []
    for c, v in pairs[start + 1:end]:
        if c == 0:
            ents.append([(c, v)])
        else:
            ents[-1].append((c, v))
    return header, ents


def _points(e: list[tuple[int, str]]) -> list[list[float]]:
    xs = [float(v) for c, v in e if c == 10]
    ys = [float(v) for c, v in e if c == 20]
    return [list(p) for p in zip(xs, ys)]


def dxf_problems(data: bytes, want: dict[str, Any]) -> list[str]:
    """Where the DXF differs from the facts: units, counts by type, the closed polylines'
    vertices, the lines' ends in order, the dimensions' measurements, the hatches' pattern and
    boundary."""
    tol = want["tolerance"]
    out: list[str] = []
    header, ents = dxf_entities(data)

    def close(a: float, b: float) -> bool:
        return abs(a - b) <= tol

    def same(ps: list[list[float]], qs: list[list[float]]) -> bool:
        return len(ps) == len(qs) and all(close(p[0], q[0]) and close(p[1], q[1]) for p, q in zip(ps, qs))

    if int(header.get("$INSUNITS", "-1")) != want["insunits"]:
        out.append(f"$INSUNITS {header.get('$INSUNITS')}, beklenen {want['insunits']}")
    counts: dict[str, int] = {}
    for e in ents:
        counts[e[0][1]] = counts.get(e[0][1], 0) + 1
    if counts != want["counts"]:
        out.append(f"nesneler {counts}, beklenen {want['counts']}")
    by = {t: [e for e in ents if e[0][1] == t] for t in counts}
    for e, w in zip(by.get("LWPOLYLINE", []), want["polylines"]):
        flags = int(next(v for c, v in e if c == 70))
        if bool(flags & 1) != w["closed"] or not same(_points(e), w["points"]):
            out.append(f"LWPOLYLINE {_points(e)}, beklenen {w['points']}")
    for i, (e, w) in enumerate(zip(by.get("LINE", []), want["lines"])):
        g = {c: float(v) for c, v in e if c in (10, 20, 11, 21)}
        if not same([[g[10], g[20]], [g[11], g[21]]], w):
            out.append(f"LINE {i}: {[[g[10], g[20]], [g[11], g[21]]]}, beklenen {w}")
    for i, (e, w) in enumerate(zip(by.get("DIMENSION", []), want["dimensions"])):
        m = float(next(v for c, v in e if c == 42))
        if not close(m, w):
            out.append(f"DIMENSION {i}: ölçü {m}, beklenen {w}")
    for i, (e, w) in enumerate(zip(by.get("HATCH", []), want["hatches"])):
        angle = float(next(v for c, v in e if c == 52))
        spacing = float(next(v for c, v in e if c == 41))
        # The boundary's vertices follow the elevation point (the first 10/20).
        boundary = _points(e)[1:1 + len(w["boundary"])]
        if not (close(angle, w["angle"]) and close(spacing, w["spacing"]) and same(boundary, w["boundary"])):
            out.append(f"HATCH {i}: açı {angle}, aralık {spacing}, sınır {boundary}")
    return out


# ------------------------------------------------------------------ PDF


_OBJ = re.compile(rb"(\d+)\s+0\s+obj(.*?)endobj", re.S)
_TOKEN = re.compile(rb"\(|<<|>>|<[0-9A-Fa-f\s]*>|\[|\]|/[^\s/\[\]()<>{}%]+|[-+]?(?:\d+\.?\d*|\.\d+)|[A-Za-z'\"*]+")


def _objects(data: bytes) -> dict[int, tuple[bytes, bytes | None]]:
    """Every object's dictionary text and its stream's bytes, inflated where FlateDecode."""
    out: dict[int, tuple[bytes, bytes | None]] = {}
    for m in _OBJ.finditer(data):
        body = m.group(2)
        k = body.find(b"stream")
        if k < 0:
            out[int(m.group(1))] = (body, None)
            continue
        head = body[:k]
        raw = body[k + len(b"stream"):]
        raw = raw[2:] if raw.startswith(b"\r\n") else raw[1:]
        raw = raw[: raw.rfind(b"endstream")].rstrip(b"\r\n")
        if b"/FlateDecode" in head:
            raw = zlib.decompress(raw)
        out[int(m.group(1))] = (head, raw)
    return out


def _skip_string(s: bytes, i: int) -> int:
    """The index after a literal string starting at ``i`` (its ``(``)."""
    depth, i = 1, i + 1
    while depth:
        c = s[i : i + 1]
        if c == b"\\":
            i += 2
            continue
        depth += 1 if c == b"(" else -1 if c == b")" else 0
        i += 1
    return i


def _segments(content: bytes) -> list[tuple[float, float, float, float]]:
    """The stroked straight segments of a content stream, in page points, through ``cm``."""
    out: list[tuple[float, float, float, float]] = []
    stack: list[float] = []
    ctm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    saved: list[list[float]] = []
    path: list[list[tuple[float, float]]] = []
    i = 0

    def apply(x: float, y: float) -> tuple[float, float]:
        a, b, c, d, e, f = ctm
        return a * x + c * y + e, b * x + d * y + f

    while i < len(content):
        if content[i : i + 1] == b"(":
            i = _skip_string(content, i)
            stack.clear()
            continue
        m = _TOKEN.match(content, i)
        if not m:
            i += 1
            continue
        t = m.group(0)
        i = m.end()
        if re.fullmatch(rb"[-+]?(?:\d+\.?\d*|\.\d+)", t):
            stack.append(float(t))
            continue
        op = t.decode("latin-1")
        if op == "cm" and len(stack) >= 6:
            a, b, c, d, e, f = stack[-6:]
            A, B, C, D, E, F = ctm
            ctm = [a * A + b * C, a * B + b * D, c * A + d * C, c * B + d * D, e * A + f * C + E, e * B + f * D + F]
        elif op == "q":
            saved.append(ctm[:])
        elif op == "Q" and saved:
            ctm = saved.pop()
        elif op == "m" and len(stack) >= 2:
            path.append([apply(stack[-2], stack[-1])])
        elif op == "l" and len(stack) >= 2 and path:
            path[-1].append(apply(stack[-2], stack[-1]))
        elif op == "h" and path and path[-1]:
            path[-1].append(path[-1][0])
        elif op == "re" and len(stack) >= 4:
            x, y, w, h = stack[-4:]
            path.append([apply(x, y), apply(x + w, y), apply(x + w, y + h), apply(x, y + h), apply(x, y)])
        elif op in ("S", "s", "B", "B*", "b", "b*"):
            for sub in path:
                for p, q in zip(sub, sub[1:]):
                    out.append((p[0], p[1], q[0], q[1]))
            path = []
        elif op in ("f", "F", "f*", "n"):
            path = []
        if not t.startswith(b"/") and t not in (b"[", b"]", b"<<", b">>") and not t.startswith(b"<"):
            stack.clear()
    return out


def pdf_pages(data: bytes) -> list[dict[str, Any]]:
    """Each page in order: its width and height (mm) and its stroked segments (mm, from the bottom left)."""
    objs = _objects(data)
    pages_root = next(n for n, (h, _) in objs.items() if re.search(rb"/Type\s*/Pages\b", h))
    kids = [int(k) for k in re.findall(rb"(\d+)\s+0\s+R", re.search(rb"/Kids\s*\[(.*?)\]", objs[pages_root][0], re.S).group(1))]
    out = []
    for kid in kids:
        head = objs[kid][0]
        box = [float(v) for v in re.search(rb"/MediaBox\s*\[(.*?)\]", head, re.S).group(1).split()]
        contents = re.search(rb"/Contents\s*(\[.*?\]|\d+\s+0\s+R)", head, re.S).group(1)
        content = b"\n".join(objs[int(r)][1] or b"" for r in re.findall(rb"(\d+)\s+0\s+R", contents))
        segs = [tuple(v / MM for v in s) for s in _segments(content)]
        out.append({"width": (box[2] - box[0]) / MM, "height": (box[3] - box[1]) / MM, "segments": segs})
    return out


def pdf_problems(data: bytes, want: dict[str, Any]) -> list[str]:
    """Where the PDF differs from the facts: the pages, their sizes, and on each the segment drawn
    between the two given points (either way round)."""
    tol = want["tolerance"]
    pages = pdf_pages(data)
    out: list[str] = []
    if len(pages) != len(want["pages"]):
        return [f"{len(pages)} sayfa, beklenen {len(want['pages'])}"]
    for n, (p, w) in enumerate(zip(pages, want["pages"]), 1):
        if abs(p["width"] - w["width"]) > tol or abs(p["height"] - w["height"]) > tol:
            out.append(f"sayfa {n}: {p['width']:.3f} × {p['height']:.3f} mm, beklenen {w['width']} × {w['height']}")
        (ax, ay), (bx, by) = w["segment"]
        hit = any(
            (abs(s[0] - ax) <= tol and abs(s[1] - ay) <= tol and abs(s[2] - bx) <= tol and abs(s[3] - by) <= tol)
            or (abs(s[0] - bx) <= tol and abs(s[1] - by) <= tol and abs(s[2] - ax) <= tol and abs(s[3] - ay) <= tol)
            for s in p["segments"]
        )
        if not hit:
            near = sorted(p["segments"], key=lambda s: abs(s[1] - ay) + abs(s[0] - ax))[:3]
            out.append(f"sayfa {n}: {w['segment']} çizgisi yok; en yakınlar {near}")
    return out


def pdf_text_problems(path: str, want: dict[str, Any]) -> list[str] | None:
    """Where each page's words (poppler's pdftotext, an independent reader) lack the facts' texts;
    None without pdftotext."""
    tool = shutil.which("pdftotext")
    if tool is None:
        return None
    out: list[str] = []
    for n, page in enumerate(want["pages"], 1):
        text = subprocess.run([tool, "-layout", "-f", str(n), "-l", str(n), path, "-"],
                              capture_output=True, text=True, check=True).stdout
        for word in page.get("texts", []):
            if word not in text:
                out.append(f"sayfa {n}: “{word}” yazısı yok")
    return out
