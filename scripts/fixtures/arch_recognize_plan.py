#!/usr/bin/env python3
"""An office's ground floor plan of the section building (section_building.py) as an explode leaves it
(plan/architecture-advanced.md M48, docs/adr/8048), drawn in centimetres, without KentOS code:

- DUVAR: the walls' outlines as lines — their faces, cut at the doors and windows with their jambs, stopping at the
  columns, meeting at the corners and the tees as one outline;
- KOLON: the 30/30 columns as closed polylines with a solid hatch;
- KAPI: each door's leaf opened 90°, its swing and its two frames (hinged at the jamb nearer its wall's start, opening
  to the wall's left: KentOS's defaults the definition leaves);
- PENCERE: each window's frame and glass lines across its gap and its sill outside;
- AKS: the grids, dash-dotted; MERDIVEN: the first flight's treads (lines 28 cm apart); YAZI: the rooms' names;
  MOBILYA: a bed.

Planı tanı reads it back; the answer is the definition (8 walls, 3 doors, 4 windows, 8 columns).

The second file is the same plan as an office keeps it before EXPLODE: each door, window and column a block drawn
once in its own axes (along its wall from its start jamb, across to the wall's left; its pieces on layer 0) and
inserted turned along its wall on KAPI, PENCERE or KOLON, a window mirrored where its sill is on the wall's left.

Writes fixtures/arch/v1/recognize/kesit-binasi-zemin.dxf and kesit-binasi-zemin-bloklu.dxf; `--check` compares.

    uv run --no-project --python 3.11 --with ezdxf --with shapely python -I scripts/fixtures/arch_recognize_plan.py
"""
import argparse
import io
import math
import sys
from pathlib import Path

import ezdxf
from shapely.geometry import Point, Polygon
from shapely.ops import unary_union

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import section_building as sb  # noqa: E402

# Fixed metadata (no creation time) from the first document made: the files are the same every run.
ezdxf.options.write_fixed_meta_data_for_testing = True
OUT = ROOT / "fixtures/arch/v1/recognize/kesit-binasi-zemin.dxf"
BLOCKS = ROOT / "fixtures/arch/v1/recognize/kesit-binasi-zemin-bloklu.dxf"
CM = 100.0
LEVEL = "zemin"
CENTRE = (6.0, 4.5)


def frame(e):
    """A wall's start, direction, left normal and length."""
    (ax, ay), (bx, by) = (e["pts"][0]["x"], e["pts"][0]["y"]), (e["pts"][1]["x"], e["pts"][1]["y"])
    length = math.hypot(bx - ax, by - ay)
    u = ((bx - ax) / length, (by - ay) / length)
    return (ax, ay), u, (-u[1], u[0]), length


def at(wall, s, r):
    """The point `s` along a wall from its start and `r` to its left."""
    (ax, ay), u, n, _ = frame(wall)
    return (ax + u[0] * s + n[0] * r, ay + u[1] * s + n[1] * r)


def band(wall, s0, s1, r0, r1):
    return Polygon([at(wall, s0, r0), at(wall, s1, r0), at(wall, s1, r1), at(wall, s0, r1)])


def cm(p):
    return (round(p[0] * CM, 6), round(p[1] * CM, 6))


def block(doc, name, draw):
    """A block of `name` drawn once (its pieces on layer 0, in centimetres), unless there is one."""
    if name not in doc.blocks:
        b = doc.blocks.new(name)
        b.block_record.dxf.units = 5
        draw(b)
    return name


def plan(blocks=False):
    ents, uids = sb.d["entities"], sb.d["uids"]
    by_uid = dict(zip(uids, ents))
    walls = [e for e in ents if e["kind"] == "wall" and e["arch"]["level"] == LEVEL]
    wall_uids = {u for u, e in by_uid.items() if e in walls}
    openings = [e for e in ents if e["kind"] == "opening" and e["host"] in wall_uids]
    columns = [e for e in ents if e["kind"] == "column" and e["arch"]["level"] == LEVEL]
    squares = [Polygon([(c["at"]["x"] + dx * c["width"] / 2, c["at"]["y"] + dy * c["width"] / 2)
                        for dx, dy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]) for c in columns]

    # The walls' mass: each wall's band past its ends by half its width (the corners full), less its openings and the
    # columns.
    mass = unary_union([band(w, -w["width"] / 2, frame(w)[3] + w["width"] / 2, -w["width"] / 2, w["width"] / 2)
                        for w in walls])
    for o in openings:
        w = by_uid[o["host"]]
        mass = mass.difference(band(w, o["at"], o["at"] + o["width"], -w["width"], w["width"]))
    mass = mass.difference(unary_union(squares))

    doc = ezdxf.new("R2010", setup=True, units=5)
    doc.header["$INSUNITS"] = 5
    for name, color in (("DUVAR", 7), ("KOLON", 1), ("KAPI", 3), ("PENCERE", 4), ("AKS", 8), ("MERDIVEN", 6),
                        ("YAZI", 2), ("MOBILYA", 9)):
        doc.layers.add(name, color=color)
    msp = doc.modelspace()

    # The outline's edges, a column's own sides left to its outline.
    def on_column(a, b):
        mid = Point((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        return any(all(sq.exterior.distance(q) < 1e-9 for q in (Point(a), Point(b), mid)) for sq in squares)

    polys = list(getattr(mass, "geoms", [mass]))
    edges = []
    for poly in polys:
        for ring in [poly.exterior, *poly.interiors]:
            pts = list(ring.coords)[:-1]
            # Vertices on a straight edge add nothing.
            kept = [p for k, p in enumerate(pts)
                    if abs((p[0] - pts[k - 1][0]) * (pts[(k + 1) % len(pts)][1] - p[1])
                           - (p[1] - pts[k - 1][1]) * (pts[(k + 1) % len(pts)][0] - p[0])) > 1e-12]
            for k, a in enumerate(kept):
                b = kept[(k + 1) % len(kept)]
                if not on_column(a, b):
                    edges.append((a, b))
    edges.sort(key=lambda e: (round(e[0][1], 6), round(e[0][0], 6), round(e[1][1], 6), round(e[1][0], 6)))
    for a, b in edges:
        msp.add_line(cm(a), cm(b), dxfattribs={"layer": "DUVAR"})

    for c, sq in zip(columns, squares):
        if blocks:
            h = c["width"] / 2

            def column(b, h=h):
                pts = [cm(q) for q in ((-h, -h), (h, -h), (h, h), (-h, h))]
                b.add_lwpolyline(pts, close=True, dxfattribs={"layer": "0"})
                b.add_hatch(color=1, dxfattribs={"layer": "0"}).paths.add_polyline_path(pts, is_closed=True)
            name = block(doc, f"KOLON-{round(c['width'] * CM)}", column)
            msp.add_blockref(name, cm((c["at"]["x"], c["at"]["y"])), dxfattribs={"layer": "KOLON"})
            continue
        pts = [cm(p) for p in list(sq.exterior.coords)[:-1]]
        msp.add_lwpolyline(pts, close=True, dxfattribs={"layer": "KOLON"})
        hatch = msp.add_hatch(color=1, dxfattribs={"layer": "KOLON"})
        hatch.paths.add_polyline_path(pts, is_closed=True)

    for o in openings:
        w = by_uid[o["host"]]
        h = w["width"] / 2
        s0, s1 = o["at"], o["at"] + o["width"]
        if blocks:
            # In the block's own axes: x along the wall from the start jamb, y across to its left.
            width, (ax, ay), (u, n) = o["width"], frame(w)[0], frame(w)[1:3]
            turn = math.degrees(math.atan2(u[1], u[0]))
            mirror = False
            if o["form"] == "door":
                f = o["frame"]

                def door(b, width=width, f=f, h=h):
                    r = width - 2 * f
                    b.add_line(cm((f, h)), cm((f, h + r)), dxfattribs={"layer": "0"})
                    b.add_arc(cm((f, h)), r * CM, 0.0, 90.0, dxfattribs={"layer": "0"})
                    for t0, t1 in ((0.0, f), (width - f, width)):
                        b.add_lwpolyline([cm((t0, -h)), cm((t1, -h)), cm((t1, h)), cm((t0, h))], close=True,
                                         dxfattribs={"layer": "0"})
                name = block(doc, f"KAPI-{round(width * CM)}-{round(2 * h * CM)}", door)
                layer = "KAPI"
            else:
                def window(b, width=width, h=h):
                    for r in (-h, -0.01, 0.01, h):
                        b.add_line(cm((0.0, r)), cm((width, r)), dxfattribs={"layer": "0"})
                    b.add_line(cm((-0.05, -(h + 0.03))), cm((width + 0.05, -(h + 0.03))), dxfattribs={"layer": "0"})
                name = block(doc, f"PENCERE-{round(width * CM)}-{round(2 * h * CM)}", window)
                layer = "PENCERE"
                # The sill outside: mirrored where the outside is the wall's left.
                mirror = (CENTRE[0] - ax) * n[0] + (CENTRE[1] - ay) * n[1] <= 0
            attribs = {"layer": layer, "rotation": turn}
            if mirror:
                attribs["yscale"] = -1.0
            msp.add_blockref(name, cm(at(w, s0, 0.0)), dxfattribs=attribs)
            continue
        if o["form"] == "door":
            f = o["frame"]
            hinge = at(w, s0 + f, h)
            r = o["width"] - 2 * f
            _, u, n, _ = frame(w)
            msp.add_line(cm(hinge), cm((hinge[0] + n[0] * r, hinge[1] + n[1] * r)), dxfattribs={"layer": "KAPI"})
            a0 = math.degrees(math.atan2(u[1], u[0]))
            msp.add_arc(cm(hinge), r * CM, a0, a0 + 90.0, dxfattribs={"layer": "KAPI"})
            for t0, t1 in ((s0, s0 + f), (s1 - f, s1)):
                msp.add_lwpolyline([cm(at(w, t0, -h)), cm(at(w, t1, -h)), cm(at(w, t1, h)), cm(at(w, t0, h))],
                                   close=True, dxfattribs={"layer": "KAPI"})
        else:
            for r in (-h, -0.01, 0.01, h):
                msp.add_line(cm(at(w, s0, r)), cm(at(w, s1, r)), dxfattribs={"layer": "PENCERE"})
            # The sill outside: on the side away from the building's middle.
            (ax, ay), u, n, _ = frame(w)
            middle = (CENTRE[0] - ax) * n[0] + (CENTRE[1] - ay) * n[1]
            r = -(h + 0.03) if middle > 0 else h + 0.03
            msp.add_line(cm(at(w, s0 - 0.05, r)), cm(at(w, s1 + 0.05, r)), dxfattribs={"layer": "PENCERE"})

    for e in ents:
        if e["kind"] == "grid":
            a, b = e["pts"]
            msp.add_line(cm((a["x"], a["y"])), cm((b["x"], b["y"])),
                         dxfattribs={"layer": "AKS", "linetype": "DASHDOT"})
        if e["kind"] == "space" and e["arch"]["level"] == LEVEL:
            ring = e["parts"][0]["outer"]["pts"]
            x = sum(p["x"] for p in ring) / len(ring)
            y = sum(p["y"] for p in ring) / len(ring)
            msp.add_text(e["name"], height=20, dxfattribs={"layer": "YAZI"}).set_placement(
                cm((x, y)), align=ezdxf.enums.TextEntityAlignment.MIDDLE_CENTER)
    for k in range(10):
        y = 1.0 + 0.28 * k
        msp.add_line(cm((4.6, y)), cm((5.8, y)), dxfattribs={"layer": "MERDIVEN"})
    msp.add_lwpolyline([cm(p) for p in ((1.0, 1.0), (2.6, 1.0), (2.6, 3.0), (1.0, 3.0))], close=True,
                       dxfattribs={"layer": "MOBILYA"})

    out = io.StringIO()
    doc.write(out)
    return stable(out.getvalue()), len(walls), sum(o["form"] == "door" for o in openings), \
        sum(o["form"] == "window" for o in openings), len(edges)


def stable(text):
    """The file with its CLASSES in their names' order: ezdxf writes them in a set's order, which changes with the
    run's hash seed."""
    head = "  0\nSECTION\n  2\nCLASSES\n"
    start = text.find(head)
    if start < 0:
        return text
    body_start = start + len(head)
    end = text.index("  0\nENDSEC\n", body_start)
    chunks = [c for c in text[body_start:end].split("  0\nCLASS\n") if c]
    chunks.sort(key=lambda c: c.split("\n")[1])
    return text[:body_start] + "".join("  0\nCLASS\n" + c for c in chunks) + text[end:]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, blocks in ((OUT, False), (BLOCKS, True)):
        text, walls, doors, windows, lines = plan(blocks)
        if args.check:
            if not path.exists() or path.read_text() != text:
                sys.exit(f"{path} differs: run without --check")
            print(f"{path.relative_to(ROOT)}: up to date")
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {walls} walls, {doors} doors, {windows} windows, {lines} wall lines")


if __name__ == "__main__":
    main()
