#!/usr/bin/env python3
"""docs/adr/0729's command cases: façade, wall group and space dimension recipes, from the ADR's rules alone.

    python3 scripts/fixtures/dimension_spans_cases.py           # writes the files
    python3 scripts/fixtures/dimension_spans_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.dimensionsets.create.span.json and cad.dimensionsets.edit.span.json. No KentOS
module or program is imported or run. A wall's lines are its path offset to its left (mitred corners) at the
distances §1 names: its left face, each boundary between its components, its right face. The rows are built by §2:
the reference lines' points on the edges square to the direction, the walls at a chosen wall's ends and the walls
meeting it, its openings' near jambs, what a viewer far off on the rows' side sees, the grids square to the
direction, a space's rings. A value's width (§6) is the drawing face's measured advances, read as data from the table
the font recorder writes (crates/shared/geometry-core/src/text/metrics.rs, as paragraph_cases.py reads it). The
predicates' signs are exact (fractions); the rest is IEEE arithmetic in the ADR's order, so a point's place along the
direction is the same number the rule gives everywhere it is computed the same way.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from copy import deepcopy as cp
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from paragraph_cases import FONTS, advance  # noqa: E402  (the drawing faces' measured advances: data)

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'fixtures/commands/v1'
CREATE = 'cad.dimensionsets.create'
EDIT = 'cad.dimensionsets.edit'

SQUARE = 1e-6  # an edge this close to square counts as parallel or perpendicular (§2)
SAME = 1e-9  # points this close along are one
JOIN = 1e-3  # a path end this close to another's touches it (ADR 0340)
REACH = 1.0  # how far around its sources a recipe reads (§4)


def uid(n: int) -> str:
    return f'0192f5a1-7290-7000-8000-{n:012x}'


S, E, N, W, P, D1, WIN, D2, GA, GB, GC, ROOM = (uid(i) for i in range(1, 13))
OBL, ARCW, OBSRC, ARCSRC, BROKEN, ROOM2, LW, RW, RDOOR, RWIN, LDOOR = (uid(i) for i in range(32, 43))
CS, CE, CN, CW, CD = (uid(i) for i in range(64, 69))
GHOST = uid(0xFFFF)
SET1 = '0192f5a1-7290-7000-8000-00000000f001'
SET2 = '0192f5a1-7290-7000-8000-00000000f002'


def pt(x, y):
    return {'x': float(x), 'y': float(y)}


# ── The drawing ─────────────────────────────────────────────────────────

def layer(id, name, visible=True, locked=False):
    return {'id': id, 'name': name, 'type': 'layer', 'visible': visible, 'locked': locked, 'expanded': True,
            'style': {'color': 'ink', 'lineType': 'continuous', 'lineWeight': 0.25}, 'children': []}


def wall(n, a, b, width=0.25, layer_id='cizim', level=None, more=None, **extra):
    e = {'kind': 'wall', 'id': n, 'layerId': layer_id, 'attrs': {}, 'pts': [pt(*a), pt(*b)] + [pt(*q) for q in more or []],
         'width': width, 'justify': 'center'}
    e.update(extra)
    if level:
        e['arch'] = {'level': level}
    return e


def opening(n, host, at, width, place, form='door', **extra):
    e = {'kind': 'opening', 'id': n, 'layerId': 'cizim', 'attrs': {}, 'host': host, 'at': float(at),
         'width': float(width), 'form': form, 'place': [pt(*q) for q in place]}
    e.update(extra)
    return e


def grid(n, x, name):
    return {'kind': 'grid', 'id': n, 'layerId': 'cizim', 'attrs': {}, 'pts': [pt(x, -2), pt(x, 10)], 'name': name}


def space(n, outer, holes=(), bulges=None):
    part = {'outer': {'pts': [pt(*q) for q in outer]}}
    if bulges:
        part['outer']['bulges'] = [float(b) for b in bulges]
    if holes:
        part['holes'] = [{'pts': [pt(*q) for q in h]} for h in holes]
    return {'kind': 'space', 'id': n, 'layerId': 'cizim', 'attrs': {}, 'parts': [part]}


def document(entities, uids, name='Cephe ve mahal ölçü dizileri'):
    return {
        'format': 'kentos.document', 'version': 2, 'name': name,
        'settings': {'srid': 0, 'lengthDecimals': 3, 'areaDecimals': 2, 'areaUnit': 'm2', 'angleUnit': 'deg',
                     'plotScale': 50.0, 'workspace': 'cad', 'drawingUnit': 'm',
                     'arch': {'levels': [{'id': 'zemin', 'name': 'Zemin kat', 'elevation': 0.0, 'height': 3.0},
                                         {'id': 'kat-1', 'name': '1. kat', 'elevation': 3.0, 'height': 3.0}]}},
        'origin': pt(0, 0),
        'layers': [layer('cizim', 'Çizim'), layer('olcu', 'Ölçü'), layer('gizli', 'Gizli', visible=False),
                   layer('kilitli', 'Kilitli', locked=True)],
        'activeLayer': 'cizim', 'entities': entities, 'uids': uids, 'styles': {'items': [], 'categories': []}}


def house(levels=None, partition_layer='cizim', partition_level=None, extra=()):
    """A 10 × 8 house of four walls drawn corner to corner on their axes, a 10 cm partition at x = 4 from the south
    wall to the north one, two doors on the south wall (1–2 m and 6–7 m), a window on the north wall (6.25–7.75 m),
    grids A, B, C at x = 0, 4, 10, and the west room's space with a 40 × 40 cm column hole."""
    lv = levels or {}
    es = [
        wall(1, (0, 0), (10, 0), level=lv.get(S)),
        wall(2, (10, 0), (10, 8), level=lv.get(E)),
        wall(3, (10, 8), (0, 8), level=lv.get(N)),
        wall(4, (0, 8), (0, 0), level=lv.get(W)),
        wall(5, (4, 0), (4, 8), width=0.1, layer_id=partition_layer, level=partition_level),
        opening(6, S, 6.5, 1.0, [(6, 0.125), (6, -0.125), (7, 0.125), (7, -0.125)]),
        opening(7, N, 3.0, 1.5, [(7.75, 7.875), (7.75, 8.125), (6.25, 7.875), (6.25, 8.125)], form='window'),
        opening(8, S, 1.5, 1.0, [(1, 0.125), (1, -0.125), (2, 0.125), (2, -0.125)]),
        grid(9, 0, 'A'), grid(10, 4, 'B'), grid(11, 10, 'C'),
        space(12, [(0.125, 0.125), (3.95, 0.125), (3.95, 7.875), (0.125, 7.875)],
              holes=[[(1.8, 3.8), (1.8, 4.2), (2.2, 4.2), (2.2, 3.8)]]),
    ]
    ids = [S, E, N, W, P, D1, WIN, D2, GA, GB, GC, ROOM]
    for i, (u, e) in enumerate(extra):
        e = cp(e)
        e['id'] = len(es) + 1
        es.append(e)
        ids.append(u)
    return document(es, ids)


COMPOSITE = [
    {'name': 'İç sıva', 'thickness': 0.02, 'detail': 'standard', 'priority': 5},
    {'name': 'Tuğla', 'thickness': 0.25, 'detail': 'standard', 'priority': 1},
    {'name': 'Dış sıva', 'thickness': 0.03, 'detail': 'standard', 'priority': 5},
]


def core_house():
    """The same house's four walls 30 cm, plaster, brick and plaster (the brick the strongest), a door at 6–7 m."""
    es = [wall(i + 1, a, b, width=0.3, components=cp(COMPOSITE))
          for i, (a, b) in enumerate([((0, 0), (10, 0)), ((10, 0), (10, 8)), ((10, 8), (0, 8)), ((0, 8), (0, 0))])]
    es.append(opening(5, CS, 6.5, 1.0, [(6, 0.15), (6, -0.15), (7, 0.15), (7, -0.15)]))
    return document(es, [CS, CE, CN, CW, CD], name='Çok katmanlı duvarlar')


def l_wall():
    """One open wall of two edges, (0, 6) down to (0, 0) and on to (8, 0), a door on its second edge at 2.5–3.5 m."""
    es = [wall(1, (0, 6), (0, 0), more=[(8, 0)]),
          opening(2, LW, 9.0, 1.0, [(2.5, 0.125), (2.5, -0.125), (3.5, 0.125), (3.5, -0.125)])]
    return document(es, [LW, LDOOR], name='Köşeli duvar')


def room_wall():
    """One closed wall round a 10 × 8 room, counter-clockwise, a door on its south edge and a window on its north."""
    w = wall(1, (0, 0), (10, 0), more=[(10, 8), (0, 8)], closed=True)
    es = [w,
          opening(2, RW, 6.5, 1.0, [(6, 0.125), (6, -0.125), (7, 0.125), (7, -0.125)]),
          opening(3, RW, 21.0, 1.5, [(7.75, 7.875), (7.75, 8.125), (6.25, 7.875), (6.25, 8.125)], form='window')]
    return document(es, [RW, RDOOR, RWIN], name='Kapalı duvar')


# ── A wall's lines (§1) ─────────────────────────────────────────────────

def boundaries(w):
    """The distances of its lines from its path, to its left: its left face, each boundary between components, its
    right face (ADR 0339)."""
    width = w['width']
    left = w.get('offset', 0.0) + {'left': 0.0, 'center': width / 2.0, 'right': width}[w.get('justify', 'center')]
    out = [left]
    comps = w.get('components') or []
    if comps:
        fixed = sum(c['thickness'] for c in comps if c.get('thickness') is not None)
        rest = sum(1 for c in comps if c.get('thickness') is None)
        slack = 1e-9 * (width if width > 1.0 else 1.0)
        ts = None
        if rest == 0 and abs(fixed - width) <= slack:
            ts = [c.get('thickness') or 0.0 for c in comps]
        elif rest == 1 and fixed < width - slack:
            ts = [c['thickness'] if c.get('thickness') is not None else width - fixed for c in comps]
        if ts is not None:
            at = left
            for t in ts[:-1]:
                at -= t
                out.append(at)
    out.append(left - width)
    return out


def offset_path(pts, closed, d):
    """The path moved `d` to its left, each corner where its two edges' moved lines meet."""
    n = len(pts)
    count = n if closed else n - 1

    def moved(k):
        a, b = pts[k], pts[(k + 1) % n]
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        ux, uy = dx / length, dy / length
        return (a[0] + -uy * d, a[1] + ux * d), (ux, uy)

    out = []
    for i in range(n):
        if not closed and i == 0:
            out.append(moved(0)[0])
            continue
        if not closed and i == n - 1:
            _, (ux, uy) = moved(n - 2)
            b = pts[n - 1]
            out.append((b[0] + -uy * d, b[1] + ux * d))
            continue
        (p1, u1), (p2, u2) = moved((i - 1) % count), moved(i % count)
        det = u1[0] * -u2[1] - u1[1] * -u2[0]
        rx, ry = p2[0] - p1[0], p2[1] - p1[1]
        t = (rx * -u2[1] - ry * -u2[0]) / det
        out.append((p1[0] + u1[0] * t, p1[1] + u1[1] * t))
    return out


def ranks(w):
    return [c.get('priority', 3) for c in w.get('components') or []]


def reference_lines(w, count, reference):
    """§1: the faces (the first and the last line), or the core: the band of the strongest components."""
    last = count - 1
    comps = w.get('components') or []
    if reference == 'faces' or count != len(comps) + 1:
        return [0, last]
    rs = ranks(w)
    least = min(rs)
    first = rs.index(least)
    end = len(rs) - 1 - rs[::-1].index(least)
    return [first, end + 1]


# ── Exact predicates ────────────────────────────────────────────────────

def orient(a, b, c):
    """The orientation determinant of a, b, c (positive counter-clockwise), exact."""
    ax, ay = F(a[0]) - F(c[0]), F(a[1]) - F(c[1])
    bx, by = F(b[0]) - F(c[0]), F(b[1]) - F(c[1])
    return ax * by - ay * bx


def in_quad(p, q):
    area = sum(q[i][0] * q[(i + 1) % 4][1] - q[(i + 1) % 4][0] * q[i][1] for i in range(4))
    sign = 1.0 if area >= 0.0 else -1.0
    for i in range(4):
        a, b = q[i], q[(i + 1) % 4]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if length > 0.0 and sign * float(orient(a, b, p)) / length < -SAME:
            return False
    return True


def crosses(a, b, c, d):
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    return o1 * o2 < 0 and o3 * o4 < 0


# ── The frame and the points ────────────────────────────────────────────

class Frame:
    def __init__(self, origin, d):
        self.origin = origin
        self.d = d
        self.n = (-d[1], d[0])

    def t(self, p):
        return (p[0] - self.origin[0]) * self.d[0] + (p[1] - self.origin[1]) * self.d[1]

    def o(self, p):
        return (p[0] - self.origin[0]) * self.n[0] + (p[1] - self.origin[1]) * self.n[1]


LINE, OPENING, GRID = 0, 1, 2
BIG = 1 << 62


def vertex(part, ring, index):
    return {'kind': 'vertex', 'part': part, 'ring': ring, 'index': index}


class Pt:
    def __init__(self, source, place, at, frame, kind):
        self.source, self.place, self.at, self.kind = source, place, at, kind
        self.t, self.o = frame.t(at), frame.o(at)

    def key(self):
        kind = self.place['kind']
        if kind == 'vertex':
            place = (self.place['part'], self.place['ring'], self.place['index'])
        else:
            place = ({'start': BIG - 2, 'end': BIG - 1}.get(kind, BIG), 0, 0)
        # Along first, −0 before +0 as a total order has it; then lines, openings, grids; then the source and place.
        return (self.t, math.copysign(1.0, self.t), self.kind, self.source, place)


def classify(a, b, bulge, frame):
    if bulge != 0.0:
        return 'arc'
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = math.hypot(dx, dy)
    if not math.isfinite(length) or length <= 0.0:
        return 'oblique'
    ux, uy = dx / length, dy / length
    if abs(ux * frame.n[0] + uy * frame.n[1]) <= SQUARE:
        return 'parallel'
    if abs(ux * frame.d[0] + uy * frame.d[1]) <= SQUARE:
        return 'perpendicular'
    return 'oblique'


def settle(pts):
    out = []
    for p in sorted(pts, key=Pt.key):
        if out and p.t - out[-1].t <= SAME:
            continue
        out.append(p)
    return out


class Refused(Exception):
    def __init__(self, broken, source=None, why=None):
        super().__init__(broken)
        self.broken, self.source, self.why = broken, source, why


class Wall:
    def __init__(self, source, e, chosen, frame, reference):
        self.source, self.e, self.chosen = source, e, chosen
        self.pts = [(q['x'], q['y']) for q in e['pts']]
        self.closed = bool(e.get('closed'))
        bulges = e.get('bulges') or [0.0] * len(self.pts)
        self.lines = [offset_path(self.pts, self.closed, d) for d in boundaries(e)]
        self.refs = reference_lines(e, len(self.lines), reference)
        n = len(self.pts)
        count = n if self.closed else n - 1
        self.edges, self.quads = [], []
        left, right = self.lines[0], self.lines[-1]
        for k in range(count):
            to = (k + 1) % n
            kind = classify(self.pts[k], self.pts[to], bulges[k] if k < len(bulges) else 0.0, frame)
            self.edges.append((k, to, kind))
            if kind != 'arc':
                self.quads.append((k, (left[k], left[to], right[to], right[k])))

    def touching(self, p):
        out = []
        n = len(self.pts)
        if not self.closed:
            for end, edge in ((0, 0), (n - 1, n - 2)):
                q = self.pts[end]
                if math.hypot(p[0] - q[0], p[1] - q[1]) <= JOIN and edge not in out:
                    out.append(edge)
        for k, q in self.quads:
            if in_quad(p, q) and k not in out:
                out.append(k)
        return out

    def edge_points(self, k, frame, side):
        frm, to, _ = self.edges[k]
        out = []
        for part in self.refs:
            a, b = self.lines[part][frm], self.lines[part][to]
            sa, sb = side * frame.o(a), side * frame.o(b)
            index, at = (to, b) if sb > sa or (sb == sa and to < frm) else (frm, a)
            out.append(Pt(self.source, vertex(part, 0, index), at, frame, LINE))
        return out

    def end_point(self, v, frame, side):
        best = None
        for part in self.refs:
            p = Pt(self.source, vertex(part, 0, v), self.lines[part][v], frame, LINE)
            if best is None or side * p.o > side * best.o:
                best = p
        return best


def hidden(p, bands, own, frame, side):
    for q in bands:
        if q in own:
            continue
        ts = [frame.t(c) for c in q]
        lo, hi = min(ts), max(ts)
        if not (p.t > lo + SAME and p.t < hi - SAME):
            continue
        far = -math.inf
        for i in range(4):
            a, b = q[i], q[(i + 1) % 4]
            ta, tb = ts[i], ts[(i + 1) % 4]
            if (ta - p.t) * (tb - p.t) > 0.0:
                continue
            oa, ob = frame.o(a), frame.o(b)
            if ta == tb:
                far = max(far, side * oa, side * ob)
            else:
                far = max(far, side * (oa + (p.t - ta) / (tb - ta) * (ob - oa)))
        if far > side * p.o + SAME:
            return True
    return False


# ── A recipe's plan (§2) ────────────────────────────────────────────────

def bbox(points, grow=0.0):
    xs, ys = [q[0] for q in points], [q[1] for q in points]
    return (min(xs) - grow, min(ys) - grow, max(xs) + grow, max(ys) + grow)


def meets(a, b):
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


def xy(q):
    return (q['x'], q['y'])


def reach_box(e):
    """The box an object is read in (§4): a wall's body's, an opening's jambs', a grid's ends', a space's outline's."""
    if e['kind'] == 'wall':
        return bbox([xy(q) for q in e['pts']], e['width'] + abs(e.get('offset', 0.0)))
    if e['kind'] == 'opening':
        return bbox([xy(q) for q in e['place']]) if e['place'] else None
    if e['kind'] == 'grid':
        return bbox([xy(q) for q in e['pts']])
    if e['kind'] == 'space':
        return bbox([xy(q) for part in e['parts'] for q in part['outer']['pts']])
    return None


def uid_order(u):
    """Identities in their bytes' order; one a command writes (a time-ordered UUIDv7, named here by the slot it takes,
    `$uidOf:N`) comes after every fixed one of these drawings, in the order they were written."""
    if u.startswith('$uidOf:'):
        return (1, int(u.split(':')[1]), '')
    return (0, 0, u)


def level_of(e):
    return (e.get('arch') or {}).get('level')


class Drawing:
    def __init__(self, doc):
        self.doc = doc
        self.by_uid = dict(zip(doc['uids'], doc['entities']))
        self.visible = {l['id']: l['visible'] for l in doc['layers']}

    def find(self, u):
        return self.by_uid.get(u)


def sources_of(rec):
    return [rec['wall']] + list(rec['span'].get('walls', []))


def recipe_reach(dr, rec):
    box = None
    for u in sources_of(rec):
        e = dr.find(u)
        r = reach_box(e) if e else None
        if r is None:
            continue
        box = r if box is None else (min(box[0], r[0]), min(box[1], r[1]), max(box[2], r[2]), max(box[3], r[3]))
    return None if box is None else (box[0] - REACH, box[1] - REACH, box[2] + REACH, box[3] + REACH)


class Planned:
    pass


def plan(dr, rec):
    """The recipe's spans and its rows' account, or why it cannot be planned (§2, §3)."""
    span = rec['span']
    plot = dr.doc['settings']['plotScale']
    space_mode = bool(span.get('space'))
    sources, walls, space_e, level = [], [], None, None
    level_set = False
    if space_mode:
        e = dr.find(rec['wall'])
        if e is None:
            raise Refused('hostGone')
        if e['kind'] != 'space':
            raise Refused('unsupportedWall', rec['wall'])
        space_e, level = e, level_of(e)
    else:
        for u in sources_of(rec):
            e = dr.find(u)
            if e is None:
                continue
            if e['kind'] != 'wall':
                raise Refused('unsupportedWall', u)
            here = level_of(e)
            if not level_set:
                level, level_set = here, True
            elif here != level:
                raise Refused('mixedSources', u)
            sources.append(u)
            walls.append((u, e, True))
        if not walls:
            raise Refused('hostGone')
    reach = recipe_reach(dr, rec)
    if reach is None:
        raise Refused('hostGone')
    hosts = list(sources)
    openings, grids = [], []
    for u in sorted(dr.doc['uids'], key=uid_order):
        e = dr.by_uid[u]
        if e['kind'] == 'grid':
            on = level_of(e) is None or level_of(e) == level
            if dr.visible[e['layerId']] and on and len(e['pts']) == 2:
                grids.append((u, e))
            continue
        if e['kind'] == 'opening' and not space_mode:
            # A source's openings wherever they are: a misplaced one breaks the recipe rather than be left out.
            if e['host'] in sources:
                if e.get('broken') or len(e['place']) != 4:
                    raise Refused('openingUnplaced')
                openings.append((u, e))
            continue
        if e['kind'] not in ('wall', 'opening'):
            continue
        box = reach_box(e)
        if box is None or not meets(box, reach):
            continue
        if e['kind'] == 'opening':
            openings.append((u, e))
        elif not space_mode and u not in sources:
            if not dr.visible[e['layerId']] or level_of(e) != level:
                continue
            hosts.append(u)
            walls.append((u, e, False))
    ids = list(hosts)
    opening_ins = []
    for u, e in openings:
        if len(e['place']) != 4:
            continue
        opening_ins.append({'source': len(ids), 'host': hosts.index(e['host']) if e['host'] in hosts else None,
                            'jambs': [xy(q) for q in e['place']], 'flue': e.get('form') == 'flue',
                            'niche_right': (e.get('side', 'left') == 'right') if e.get('form') == 'niche' else None})
        ids.append(u)
    grid_ins = []
    for u, e in grids:
        grid_ins.append({'source': len(ids), 'pts': [xy(q) for q in e['pts']], 'bulges': e.get('bulges')})
        ids.append(u)
    space_in = None
    if space_e is not None:
        space_in = {'source': len(ids), 'uid': rec['wall'], 'parts': [
            [([xy(q) for q in ring['pts']], ring.get('bulges') or [0.0] * len(ring['pts']))
             for ring in [part['outer']] + part.get('holes', [])]
            for part in space_e['parts']]}
        ids.append(rec['wall'])
    d = span['direction']
    length = math.hypot(d['x'], d['y'])
    d = (d['x'] / length, d['y'] / length)
    if space_in is not None:
        origin = space_in['parts'][0][0][0][0]
    else:
        origin = xy(walls[0][1]['pts'][0])
    frame = Frame(origin, d)
    at = xy(rec['at'])
    at_o = frame.o(at)

    def side_of(body):
        return 1.0 if at_o >= (body[0] + body[1]) * 0.5 else -1.0

    reference = span.get('reference', 'faces')
    if space_in is not None:
        rows, side = space_rows(rec, space_in, opening_ins, frame, side_of)
    else:
        rows, side = facade(rec, walls, opening_ins, frame, side_of, reference)
    extent = settle(rows['extent'])
    if not extent:
        raise Refused('hostGone')
    lo, hi = extent[0], extent[-1]
    for g in grid_ins:
        if len(g['pts']) != 2:
            continue
        a, b = g['pts']
        if (g['bulges'] and g['bulges'][0] != 0.0) or classify(a, b, 0.0, frame) != 'perpendicular':
            continue
        oa, ob = frame.o(a), frame.o(b)
        if max(oa, ob) < rows['body'][0] - SAME or min(oa, ob) > rows['body'][1] + SAME:
            continue
        t = frame.t(a)
        if t < lo.t - SAME or t > hi.t + SAME:
            continue
        place, q = ({'kind': 'end'}, b) if side * ob > side * oa else ({'kind': 'start'}, a)
        rows['grids'].append(Pt(g['source'], place, q, frame, GRID))
    angle = math.atan2(d[1], d[0]) * 180.0 / math.pi
    gap = rec['gapMm'] * plot / 1000.0
    places = {r['row']: xy(r['at']) for r in rec.get('rowPlaces', [])}

    def ref(q):
        return {'uid': ids[q.source], 'place': q.place}

    suppressed = rec.get('suppressed', [])
    spans, notes, drawn = [], [], []
    slot = 0
    for row in rec['rows']:
        pts = settle({'openings': rows['openings'], 'walls': rows['walls'], 'grids': [lo, hi] + rows['grids'],
                      'overall': [lo, hi]}[row])
        count = 0
        if row in places:
            row_at = places[row]
        else:
            target = at_o + side * gap * slot
            delta = target - at_o
            row_at = (at[0] + frame.n[0] * delta, at[1] + frame.n[1] * delta)
        for a, b in zip(pts, pts[1:]):
            segment = {'row': row, 'a': ref(a), 'b': ref(b)}
            if segment in suppressed or (segment['a'], segment['b']) in drawn:
                continue
            drawn.append((segment['a'], segment['b']))
            offset = (row_at[0] - a.at[0]) * frame.n[0] + (row_at[1] - a.at[1]) * frame.n[1]
            spans.append({'segment': segment, 'a': a.at, 'b': b.at, 'offset': offset, 'angle': angle, 'at': row_at})
            count += 1
        if count:
            slot += 1
        notes.append({'row': row, 'points': len(pts), 'sources': len({q.source for q in pts}), 'dimensions': count,
                      'reference': reference})
    out = Planned()
    out.spans, out.notes, out.side = spans, notes, side
    out.skipped = [ids[s] for s in sorted(rows['skipped'])]
    out.origin, out.d, out.body = origin, d, rows['body']
    return out


def facade(rec, walls, openings, frame, side_of, reference):
    read = []
    for source, (u, e, chosen) in enumerate(walls):
        x = Wall(source, e, chosen, frame, reference)
        if chosen:
            bad = next((k for _, _, k in x.edges if k in ('oblique', 'arc')), None)
            if bad:
                raise Refused('unsupportedWall', u, bad)
        read.append(x)
    chosen = [i for i, x in enumerate(read) if x.chosen]
    lo, hi = math.inf, -math.inf
    for i in chosen:
        x = read[i]
        for line in (x.lines[0], x.lines[-1]):
            for q in line:
                o = frame.o(q)
                lo, hi = min(lo, o), max(hi, o)
    body = (lo, hi)
    side = side_of(body)
    lines, free, walls_row, bands, skipped, own_of, ends_at = [], [], [], [], [], [], []
    for i in chosen:
        x = read[i]
        for k, q in x.quads:
            bands.append(q)
            if x.edges[k][2] == 'perpendicular':
                for p in x.edge_points(k, frame, side):
                    own_of.append((p, [q]))
                    lines.append(p)
    for i in chosen:
        x = read[i]
        if x.closed:
            continue
        n = len(x.pts)
        for v, edge in ((0, 0), (n - 1, n - 2)):
            if x.edges[edge][2] != 'parallel':
                continue
            e = x.pts[v]
            if any(j != i and read[j].touching(e) for j in chosen):
                continue
            walled = False
            for j, y in enumerate(read):
                if y.chosen:
                    continue
                at = y.touching(e)
                if not at:
                    continue
                if any(y.edges[k][2] in ('oblique', 'arc') for k in at):
                    if y.source not in skipped:
                        skipped.append(y.source)
                    continue
                for k in at:
                    if y.edges[k][2] == 'perpendicular':
                        walled = True
                        ends_at.append((j, k))
                        own = [q for m, q in y.quads if m == k]
                        bands.extend(own[:1])
                        for p in y.edge_points(k, frame, side):
                            own_of.append((p, own[:1]))
                            lines.append(p)
            if not walled:
                p = x.end_point(v, frame, side)
                own_of.append((p, [q for m, q in x.quads if m == edge]))
                free.append(p)
    for j, y in enumerate(read):
        if y.chosen:
            continue
        meet = [k for m, k in ends_at if m == j]
        n = len(y.pts)
        if not y.closed:
            for v, edge in ((0, 0), (n - 1, n - 2)):
                if any(read[i].touching(y.pts[v]) for i in chosen) and edge not in meet:
                    meet.append(edge)
        for k, (frm, to, kind) in enumerate(y.edges):
            if kind == 'arc' or k in meet:
                continue
            a, b = y.pts[frm], y.pts[to]
            if any(kd != 'arc' and crosses(a, b, read[i].pts[f], read[i].pts[t])
                   for i in chosen for f, t, kd in read[i].edges):
                meet.append(k)
        if not meet:
            continue
        if any(y.edges[k][2] in ('oblique', 'arc') for k in meet):
            if y.source not in skipped:
                skipped.append(y.source)
            continue
        for k in meet:
            if y.edges[k][2] == 'perpendicular':
                walls_row.extend(y.edge_points(k, frame, side))
    extent = lines + free
    settled = settle(extent)
    if not settled:
        raise Refused('hostGone')
    elo, ehi = settled[0].t, settled[-1].t
    opening_pts = []
    for o in openings:
        h = o['host']
        if h is None or h >= len(read):
            continue
        x = read[h]
        if not x.chosen or o['flue']:
            continue
        j = o['jambs']
        if classify(j[0], j[2], 0.0, frame) != 'parallel':
            continue
        left_near = side * frame.o(j[0]) >= side * frame.o(j[1])
        if o['niche_right'] is not None and o['niche_right'] == left_near:
            continue
        mid = ((j[0][0] + j[1][0] + j[2][0] + j[3][0]) * 0.25, (j[0][1] + j[1][1] + j[2][1] + j[3][1]) * 0.25)
        own = [q for _, q in x.quads if in_quad(mid, q)]
        if rec['openings'] == 'ends':
            for i in ((0, 2) if left_near else (1, 3)):
                opening_pts.append((Pt(o['source'], vertex(0, 0, i), j[i], frame, OPENING), own))
        else:
            opening_pts.append((Pt(o['source'], {'kind': 'center'}, mid, frame, OPENING), own))
    seen = [p for p, own in own_of + opening_pts if not hidden(p, bands, own, frame, side)]
    walls_all = lines + free + [p for p in walls_row if elo - SAME <= p.t <= ehi + SAME]
    return {'openings': seen, 'walls': walls_all, 'extent': extent, 'grids': [], 'skipped': skipped,
            'body': body}, side


def space_rows(rec, space, openings, frame, side_of):
    lo, hi = math.inf, -math.inf
    for rings in space['parts']:
        for r, (pts, bulges) in enumerate(rings):
            n = len(pts)
            if n < 3:
                raise Refused('unsupportedWall', space['uid'], 'drawn')
            for k in range(n):
                kind = classify(pts[k], pts[(k + 1) % n], bulges[k] if k < len(bulges) else 0.0, frame)
                if kind in ('arc', 'oblique'):
                    raise Refused('unsupportedWall', space['uid'], kind)
                if r == 0:
                    o = frame.o(pts[k])
                    lo, hi = min(lo, o), max(hi, o)
    body = (lo, hi)
    side = side_of(body)
    outer, holes = [], []
    for part, rings in enumerate(space['parts']):
        for r, (pts, _) in enumerate(rings):
            n = len(pts)
            for k in range(n):
                frm, to = k, (k + 1) % n
                if classify(pts[frm], pts[to], 0.0, frame) != 'perpendicular':
                    continue
                sa, sb = side * frame.o(pts[frm]), side * frame.o(pts[to])
                index = to if sb > sa or (sb == sa and to < frm) else frm
                p = Pt(space['source'], vertex(part, r, index), pts[index], frame, LINE)
                (outer if r == 0 else holes).append(p)
    opening_pts = []
    for o in openings:
        if o['flue']:
            continue
        j = o['jambs']

        def on(a, b):
            for rings in space['parts']:
                pts = rings[0][0]
                n = len(pts)
                ccw = sum(pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1] for i in range(n)) > 0.0
                for k in range(n):
                    u, v = pts[k], pts[(k + 1) % n]
                    if classify(u, v, 0.0, frame) != 'parallel':
                        continue
                    length = math.hypot(v[0] - u[0], v[1] - u[1])

                    def near(q):
                        if abs(float(orient(u, v, q)) / length) > 1e-6:
                            return False
                        tu, tv, tq = frame.t(u), frame.t(v), frame.t(q)
                        return min(tu, tv) - SAME <= tq <= max(tu, tv) + SAME
                    ex, ey = (v[0] - u[0]) / length, (v[1] - u[1]) / length
                    out = (ey, -ex) if ccw else (-ey, ex)
                    facing = side * (out[0] * frame.n[0] + out[1] * frame.n[1]) > 0.0
                    if facing and near(a) and near(b):
                        return True
            return False
        pair = (0, 2) if on(j[0], j[2]) else (1, 3) if on(j[1], j[3]) else None
        if pair is None:
            continue
        if o['niche_right'] is not None and o['niche_right'] != (pair == (1, 3)):
            continue
        if rec['openings'] == 'ends':
            for i in pair:
                opening_pts.append(Pt(o['source'], vertex(0, 0, i), j[i], frame, OPENING))
        else:
            mid = ((j[0][0] + j[1][0] + j[2][0] + j[3][0]) * 0.25, (j[0][1] + j[1][1] + j[2][1] + j[3][1]) * 0.25)
            opening_pts.append(Pt(o['source'], {'kind': 'center'}, mid, frame, OPENING))
    return {'openings': outer + opening_pts, 'walls': outer + holes, 'extent': outer, 'grids': [], 'skipped': [],
            'body': body}, side


# ── What a command answers ──────────────────────────────────────────────

FACE = FONTS[0]  # the drawing face the settings name none of: the first (Barlow)


def fixed(x, decimals=3):
    return f'{x:.{decimals}f}'


def tight_ones(dims):
    """§6: the dimensions whose value, in the drawing face at their height with a height to spare, is wider than
    what they measure."""
    out = []
    for i, e in enumerate(dims):
        th = math.radians(e['angle'])
        measured = abs((e['b']['x'] - e['a']['x']) * math.cos(th) + (e['b']['y'] - e['a']['y']) * math.sin(th))
        width = sum(advance(FACE, c, False) for c in fixed(measured)) / 1000.0
        if width * e['height'] + e['height'] > measured:
            out.append(i)
    return out


def height_of(dr):
    return 2.5 * dr.doc['settings']['plotScale'] / 1000.0


def dimension(dr, rec, s):
    return {'kind': 'dimension', 'id': 0, 'layerId': rec['layerId'], 'attrs': {}, 'a': pt(*s['a']), 'b': pt(*s['b']),
            'offset': s['offset'], 'height': height_of(dr), 'style': 'linear', 'angle': s['angle'],
            'refs': {'a': cp(s['segment']['a']), 'b': cp(s['segment']['b'])}}


def message(r):
    named = f' ({r.source})' if r.source else ''
    if r.broken == 'unsupportedWall' and r.why == 'oblique':
        return (f'Kaynağın{named} bir kenarı ölçü doğrultusuna ne paralel ne dik. Cephe ve mahal ölçüsü yalnız '
                'doğrultuya paralel ve dik kenarları ölçer: doğrultuyu duvarlara göre verin ya da eğik duvarı ayrı ölçün.')
    if r.broken == 'unsupportedWall' and r.why == 'arc':
        return (f'Kaynağın{named} yaylı bir kenarı var; cephe ve mahal ölçüsü yaylı kenarı ölçmez. Yaylı duvarı tek '
                'duvar ölçüsüyle ya da ayrı ölçün.')
    if r.broken == 'unsupportedWall' and r.why == 'drawn':
        return f'Kaynak duvarın{named} bir yüzü köşesinde pahlanıyor ya da duvar çizilemiyor; ölçülemez.'
    if r.broken == 'unsupportedWall' and r.source:
        return f'Kaynak{named} bir duvar değil (mahal dizisinde bir mahal değil). Doğru nesneyi seçin.'
    if r.broken == 'unsupportedWall':
        return 'Ölçü doğrultusu, yeri ya da sıra aralığı geçersiz.'
    if r.broken == 'mixedSources':
        return (f'Duvarlar{named} farklı katlarda; cephe ölçüsünün duvarları tek bir katta (ya da hepsi katsız) olmalı. '
                'Duvarlara aynı katı verin ya da ayrı ölçün.')
    if r.broken == 'hostGone':
        return 'Kaynak bulunamadı. Duvarları ya da mahali yeniden seçin.'
    return 'Duvarın bir açıklığı yerleştirilemedi. Önce açıklığın kaynak bağını düzeltin.'


def failed(text, path, uids=()):
    error = {'code': 'invalid_dimension_set', 'message': text, 'path': path}
    if uids:
        error['uids'] = list(uids)
    return {'status': 'failed', 'error': error}


def refused(r, path):
    return failed(message(r), path, [r.source] if r.source else [])


def completed(output, warnings=()):
    return {'status': 'completed', 'output': output, 'warnings': list(warnings)}


def tight_warning(n):
    return {'code': 'dimension_text_tight',
            'message': f'{n} ölçünün değeri iki uzatma çizgisinin arasına sığmıyor; gizlenmedi. Sıra aralığını ya da '
                       'ölçü stilinin yazı yerini değiştirin.'}


def skipped_warning(n):
    return {'code': 'dimension_set_wall_skipped',
            'message': f'Cepheye değen {n} duvar eğik ya da yaylı kenarı yüzünden ölçülmedi.'}


def hidden_warning(name):
    return {'code': 'layer_hidden', 'message': f'“{name}” katmanı gizli; çizilen nesne görünmeyecek.', 'path': 'layerId'}


def layer_warnings(doc, layer_id):
    lay = next(l for l in doc['layers'] if l['id'] == layer_id)
    return [] if lay['visible'] else [hidden_warning(lay['name'])]


# ── The recipes a create makes (§5) ─────────────────────────────────────

def longest_edge(pts, closed, bulges=None):
    n = len(pts)
    best = None
    for k in range(n if closed else n - 1):
        if bulges and k < len(bulges) and bulges[k] != 0.0:
            continue
        a, b = pts[k], pts[(k + 1) % n]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if length > 0.0 and (best is None or length > best[0]):
            best = (length, ((b[0] - a[0]) / length, (b[1] - a[1]) / length))
    return best[1] if best else None


def body_points(dr, rec):
    out = []
    for u in sources_of(rec):
        e = dr.find(u)
        if e is None:
            continue
        if e['kind'] == 'wall':
            pts = [xy(q) for q in e['pts']]
            d = boundaries(e)
            for dist in (d[0], d[-1]):
                out.extend(offset_path(pts, bool(e.get('closed')), dist))
        elif e['kind'] == 'space':
            for part in e['parts']:
                out.extend(xy(q) for q in part['outer']['pts'])
    return out


def readable(d):
    """§5: a direction reads left to right, or upward: one pointing left (or straight down) is turned."""
    if d[0] < -1e-9 or (abs(d[0]) <= 1e-9 and d[1] < 0.0):
        return (-d[0] + 0.0, -d[1] + 0.0)
    return d


def four_sides(dr, base):
    """§5: along the direction south and north of the bodies, square to it east and west, each first row as far out
    from the outermost body as `at`."""
    pts = body_points(dr, base)
    origin = pts[0]
    d = (base['span']['direction']['x'], base['span']['direction']['y'])
    n = (-d[1], d[0])

    def t(q):
        return (q[0] - origin[0]) * d[0] + (q[1] - origin[1]) * d[1]

    def o(q):
        return (q[0] - origin[0]) * n[0] + (q[1] - origin[1]) * n[1]
    t0, t1 = min(t(q) for q in pts), max(t(q) for q in pts)
    o0, o1 = min(o(q) for q in pts), max(o(q) for q in pts)
    ta, oa = t(xy(base['at'])), o(xy(base['at']))
    if oa < o0:
        reach = o0 - oa
    elif oa > o1:
        reach = oa - o1
    elif ta < t0:
        reach = t0 - ta
    elif ta > t1:
        reach = ta - t1
    else:
        reach = min(oa - o0, o1 - oa, ta - t0, t1 - ta)

    def at(tt, oo):
        return pt(origin[0] + d[0] * tt + n[0] * oo, origin[1] + d[1] * tt + n[1] * oo)
    tm, om = (t0 + t1) * 0.5, (o0 + o1) * 0.5
    out = []
    for direction, place in ((d, at(tm, o0 - reach)), (n, at(t1 + reach, om)), (d, at(tm, o1 + reach)),
                             (n, at(t0 - reach, om))):
        rec = cp(base)
        rec['at'] = place
        direction = readable(direction)
        rec['span']['direction'] = {'x': direction[0], 'y': direction[1]}
        out.append(rec)
    return out


def recipe(dr, inp):
    """The recipe a create's input names (§1, §5), or the refusal its sources earn: one, or four with `around`."""
    path = 'space' if 'space' in inp else 'walls'
    if 'space' in inp:
        first, rest = inp['space'], []
    else:
        ids = ([inp['wall']] if 'wall' in inp else []) + list(inp.get('walls', []))
        first, rest = ids[0], ids[1:]
    for u in [first] + rest:
        e = dr.find(u)
        if e is None:
            raise Refused('hostGone', u)
        if e['kind'] != ('space' if path == 'space' else 'wall'):
            raise Refused('unsupportedWall', u)
    e = dr.find(first)
    if 'direction' in inp:
        dx, dy = inp['direction']['x'], inp['direction']['y']
        length = math.hypot(dx, dy)
        direction = (dx / length, dy / length)
    elif e['kind'] == 'wall':
        direction = longest_edge([xy(q) for q in e['pts']], bool(e.get('closed')), e.get('bulges'))
    else:
        outer = e['parts'][0]['outer']
        direction = longest_edge([xy(q) for q in outer['pts']], True, outer.get('bulges'))
    direction = readable(direction)
    span = {'direction': {'x': direction[0], 'y': direction[1]}}
    if rest:
        span['walls'] = rest
    if path == 'space':
        span['space'] = True
    if inp.get('reference', 'faces') != 'faces':
        span['reference'] = inp['reference']
    lev = level_of(e)
    if lev:
        span['level'] = lev
    rec = {'wall': first, 'face': 'left', 'rows': inp.get('rows', ['openings', 'overall']),
           'openings': inp.get('openings', 'ends'), 'layerId': inp['layerId'], 'at': cp(inp['at']),
           'gapMm': inp.get('gapMm', 7.5), 'height': height_of(dr), 'look': {}, 'span': span}
    return (four_sides(dr, rec) if inp.get('around') else [rec]), path


def create_output(dr, inp):
    """A create's plan: its recipes' dimensions, rows and account, its warnings; or its refusal."""
    path = 'space' if 'space' in inp else 'walls'
    try:
        recs, path = recipe(dr, inp)
        plans = [plan(dr, r) for r in recs]
    except Refused as r:
        return refused(r, path), None
    out = {'entities': [], 'spans': [], 'removed': [], 'uids': [], 'revision': '$current'}
    rows, skipped, tight = [], [], []
    for r, pl in zip(recs, plans):
        dims = [dimension(dr, r, s) for s in pl.spans]
        tight.extend(i + len(out['entities']) for i in tight_ones(dims))
        out['entities'].extend(dims)
        out['spans'].extend(cp(s['segment']) for s in pl.spans)
        rows.extend(pl.notes)
        skipped.extend(u for u in pl.skipped if u not in skipped)
    if rows:
        out['rows'] = rows
    if skipped:
        out['skipped'] = skipped
    if tight:
        out['tight'] = tight
    warnings = layer_warnings(dr.doc, inp['layerId'])
    if tight:
        warnings.append(tight_warning(len(tight)))
    if skipped:
        warnings.append(skipped_warning(len(skipped)))
    return completed(out, warnings), (recs, plans)


# ── The cases ───────────────────────────────────────────────────────────

def step(op, inp, result, expect=None, command=None):
    out = {'op': op, 'input': inp, 'result': result, 'within': 1e-9}
    if command:
        out['command'] = command
    if expect is not None:
        out['expect'] = expect
    return out


def unchanged(doc, sets=None):
    return {'ids': [e['id'] for e in doc['entities']], 'revision': 'same', 'canUndo': False, 'dirty': False,
            'dimensionSets': sets if sets is not None else doc.get('dimensionSets', [])}


def case(name, doc, *steps):
    return {'name': name, 'setup': doc, 'steps': list(steps)}


def planned_case(name, doc, inp):
    dr = Drawing(doc)
    result, _ = create_output(dr, inp)
    return case(name, doc, step('plan', inp, result, unchanged(doc)))


def refused_case(name, doc, inp, result, op='plan'):
    return case(name, doc, step(op, inp, result, unchanged(doc)))


def written(dr, inp, first_slot):
    """A create's write: its result, the drawing's slots after it, the new dimensions' fields and the sets."""
    result, (recs, plans) = create_output(dr, inp)
    slot = first_slot
    fields, sets, created = {}, [], []
    for rec, pl in zip(recs, plans):
        members = []
        for s in pl.spans:
            e = dimension(dr, rec, s)
            fields[str(slot)] = {k: e[k] for k in ('a', 'b', 'offset', 'height', 'style', 'angle', 'refs', 'layerId')}
            members.append({'uid': f'$uidOf:{slot}', **cp(s['segment'])})
            created.append('$any')
            slot += 1
        sets.append({**cp(rec), 'id': '$any', 'members': members})
    output = {'set': '$any', 'uids': ['$any'] * len(created), 'created': ['$any'] * len(created), 'removed': [],
              'changed': [], 'revision': '$current'}
    if len(recs) > 1 or inp.get('around'):
        output['sets'] = ['$any'] * len(recs)
    return completed(output, result['warnings']), list(range(first_slot, slot)), fields, sets


def create_cases():
    out = []
    h = house()
    at_south = pt(5, -1.5)
    out.append(planned_case('Güney cephe: tek duvar, varsayılan sıralar ve doğrultu; köşe duvarları uçta, iki kapı',
                            h, {'layerId': 'olcu', 'walls': [S], 'at': at_south}))
    out.append(planned_case('Güney cephe: dört sıra (açıklıklar, duvarlar, akslar, toplam); ince ölçüler sığmıyor',
                            h, {'layerId': 'olcu', 'walls': [S], 'at': at_south,
                                'rows': ['openings', 'walls', 'grids', 'overall']}))
    out.append(planned_case('Güney cephe: açıklık merkezleri, yalnız açıklık sırası',
                            h, {'layerId': 'olcu', 'walls': [S], 'at': at_south, 'openings': 'centers',
                                'rows': ['openings']}))
    out.append(planned_case('Güney ve doğu duvarı bir grup: doğu duvarı dik kaynak',
                            h, {'layerId': 'olcu', 'wall': S, 'walls': [E], 'at': at_south,
                                'direction': pt(1, 0), 'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Kuzey cephe: doğrultu batıya, pencere dışarıdan', h,
                            {'layerId': 'olcu', 'walls': [N], 'at': pt(5, 9.5)}))
    out.append(planned_case('Doğu cephesi: verilen doğrultu, köşe duvarlarının yüzleri',
                            h, {'layerId': 'olcu', 'walls': [E], 'at': pt(11.5, 4), 'direction': pt(0, 1),
                                'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Kuzey cephe aşağıdan bakınca: iç yüzler ve güney kapıları', h,
                            {'layerId': 'olcu', 'walls': [N], 'at': pt(5, 6.5), 'direction': pt(1, 0),
                             'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Mahal: dış halka, kapı ve kolon boşluğu, üç sıra', h,
                            {'layerId': 'olcu', 'space': ROOM, 'at': pt(2, -1.5), 'direction': pt(1, 0),
                             'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Mahal: varsayılan doğrultu en uzun kenarın', h,
                            {'layerId': 'olcu', 'space': ROOM, 'at': pt(5, 4)}))
    out.append(planned_case('Mahal: açıklık merkezleri', h,
                            {'layerId': 'olcu', 'space': ROOM, 'at': pt(2, -1.5), 'direction': pt(1, 0),
                             'openings': 'centers'}))
    out.append(planned_case('Bütün kaynaklar: dört yan tek adımda', h,
                            {'layerId': 'olcu', 'walls': [S, E, N, W], 'at': at_south, 'direction': pt(1, 0),
                             'around': True}))
    out.append(planned_case('Mahal: dört yan', h,
                            {'layerId': 'olcu', 'space': ROOM, 'at': pt(2, -1.5), 'direction': pt(1, 0),
                             'around': True}))
    out.append(planned_case('Hedef katman gizli: aynı plan ve görünür uyarı', h,
                            {'layerId': 'gizli', 'walls': [S], 'at': at_south}))
    out.append(planned_case('Gizli katmandaki bölme okunmaz', house(partition_layer='gizli'),
                            {'layerId': 'olcu', 'walls': [S], 'at': at_south, 'rows': ['openings', 'walls', 'overall']}))
    zemin = {u: 'zemin' for u in (S, E, N, W)}
    out.append(planned_case('Başka kattaki bölme okunmaz, katsız akslar okunur',
                            house(levels=zemin, partition_level='kat-1'),
                            {'layerId': 'olcu', 'walls': [S], 'at': at_south,
                             'rows': ['openings', 'walls', 'grids', 'overall']}))
    oblique = wall(0, (8.5, 0), (9.5, 3))
    arc = wall(0, (2.5, 0), (3.5, 1.5), bulges=[0.3, 0.0])
    out.append(planned_case('Eğik ve yaylı bölme ölçülmez ve söylenir',
                            house(extra=[(OBL, oblique), (ARCW, arc)]),
                            {'layerId': 'olcu', 'walls': [S], 'at': at_south, 'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Çok katmanlı duvarlar: dış yüzler', core_house(),
                            {'layerId': 'olcu', 'walls': [CS], 'at': at_south, 'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Çok katmanlı duvarlar: çekirdek (en güçlü bileşen)', core_house(),
                            {'layerId': 'olcu', 'walls': [CS], 'at': at_south, 'reference': 'core',
                             'rows': ['openings', 'walls', 'overall']}))
    out.append(planned_case('Köşeli açık duvar: dik kenarı ve serbest ucu', l_wall(),
                            {'layerId': 'olcu', 'walls': [LW], 'at': pt(4, -1.5)}))
    out.append(planned_case('Kapalı tek duvar: iç yüzler ve kuzey penceresi görünmez', room_wall(),
                            {'layerId': 'olcu', 'walls': [RW], 'at': at_south, 'rows': ['openings', 'walls', 'overall']}))
    far = house()
    for e in far['entities']:
        for key in ('pts', 'place'):
            for q in e.get(key, []):
                q['x'] += 500000.0
                q['y'] += 4400000.0
        for part in e.get('parts', []):
            for ring in [part['outer']] + part.get('holes', []):
                for q in ring['pts']:
                    q['x'] += 500000.0
                    q['y'] += 4400000.0
    out.append(planned_case('Büyük koordinatlar: güney cephe dört sıra', far,
                            {'layerId': 'olcu', 'walls': [S], 'at': pt(500005, 4399998.5),
                             'rows': ['openings', 'walls', 'grids', 'overall']}))
    # Writes: one step, the dimensions in their slots, the recipe kept; undo and redo.
    for name, inp in [
        ('Güney cephe: yaz, geri al, yinele', {'layerId': 'olcu', 'walls': [S], 'at': at_south,
                                               'rows': ['openings', 'walls', 'overall']}),
        ('Mahal: yaz, geri al, yinele', {'layerId': 'olcu', 'space': ROOM, 'at': pt(2, -1.5), 'direction': pt(1, 0),
                                         'rows': ['openings', 'walls', 'overall']}),
        ('Dört yan: dört dizi tek adımda', {'layerId': 'olcu', 'walls': [S, E, N, W], 'at': at_south,
                                            'direction': pt(1, 0), 'around': True}),
    ]:
        dr = Drawing(h)
        before = [e['id'] for e in h['entities']]
        result, slots, fields, sets = written(dr, inp, len(before) + 1)
        out.append(case(name, h,
                        step('execute', inp, result, {'ids': before + slots, 'fields': fields, 'dimensionSets': sets,
                                                      'canUndo': True, 'dirty': True, 'revision': 'changed'}),
                        {'op': 'undo', 'expect': {'ids': before, 'dimensionSets': [], 'canUndo': False, 'canRedo': True}},
                        {'op': 'redo', 'expect': {'ids': before + slots, 'fields': fields, 'dimensionSets': sets,
                                                  'canUndo': True}, 'within': 1e-9}))
    # Refusals: nothing written (§3).
    out.append(refused_case('Ret: eğik kaynak duvar', house(extra=[(OBSRC, wall(0, (10, 0), (12, 3)))]),
                            {'layerId': 'olcu', 'walls': [S, OBSRC], 'at': at_south},
                            refused(Refused('unsupportedWall', OBSRC, 'oblique'), 'walls')))
    out.append(refused_case('Ret: yaylı kaynak duvar',
                            house(extra=[(ARCSRC, wall(0, (10, 0), (12, 0), bulges=[0.4, 0.0]))]),
                            {'layerId': 'olcu', 'walls': [S, ARCSRC], 'at': at_south, 'direction': pt(1, 0)},
                            refused(Refused('unsupportedWall', ARCSRC, 'arc'), 'walls')))
    out.append(refused_case('Ret: kaynaklar farklı katlarda', house(levels={S: 'zemin', N: 'kat-1'}),
                            {'layerId': 'olcu', 'walls': [S, N], 'at': at_south},
                            refused(Refused('mixedSources', N), 'walls')))
    out.append(refused_case('Ret: kaynak bir duvar değil', h, {'layerId': 'olcu', 'walls': [S, D1], 'at': at_south},
                            refused(Refused('unsupportedWall', D1), 'walls')))
    out.append(refused_case('Ret: kaynak çizimde yok', h, {'layerId': 'olcu', 'walls': [S, GHOST], 'at': at_south},
                            refused(Refused('hostGone', GHOST), 'walls')))
    out.append(refused_case('Ret: mahal bir mahal değil', h, {'layerId': 'olcu', 'space': S, 'at': at_south},
                            refused(Refused('unsupportedWall', S), 'space')))
    out.append(refused_case('Ret: mahal ve duvarlar birlikte', h,
                            {'layerId': 'olcu', 'space': ROOM, 'walls': [S], 'at': at_south},
                            failed('Mahal ölçüsüne duvar verilmez: ya mahali (space) ya duvarları (walls) verin.', 'space')))
    out.append(refused_case('Ret: cephede kenar numarası', h, {'layerId': 'olcu', 'walls': [S], 'edge': 0, 'at': at_south},
                            failed('Kenar (edge) yalnız tek duvar ölçüsünde verilir; cephe ve mahal doğrultusuyla '
                                   'ölçülür.', 'edge')))
    out.append(refused_case('Ret: tek duvar dizisinde doğrultu', h,
                            {'layerId': 'olcu', 'wall': S, 'direction': pt(1, 0), 'at': at_south},
                            failed('Doğrultu (direction) ve ölçülen çizgi (reference) yalnız cephe (walls) ve mahal '
                                   '(space) dizisinde verilir.', 'direction')))
    out.append(refused_case('Ret: sıfır doğrultu', h, {'layerId': 'olcu', 'walls': [S], 'direction': pt(0, 0), 'at': at_south},
                            failed('Ölçü doğrultusu sıfır olamaz.', 'direction')))
    out.append(refused_case('Ret: tek duvar dizisinde duvarlar sırası', h,
                            {'layerId': 'olcu', 'wall': S, 'at': at_south, 'rows': ['walls']},
                            failed('Duvarlar ve akslar sırası yalnız cephe ya da mahal ölçü dizisinde olur.', 'operation')))
    out.append(refused_case('Ret: aynı duvar iki kez', h, {'layerId': 'olcu', 'walls': [S, E, S], 'at': at_south},
                            failed('Aynı duvar iki kez verildi.', 'walls[2]')))
    out.append(refused_case('Ret: yaylı mahal', house(extra=[(ROOM2, space(0, [(5, 1), (9, 1), (9, 7), (5, 7)],
                                                                         bulges=[0.0, 0.2, 0.0, 0.0]))]),
                            {'layerId': 'olcu', 'space': ROOM2, 'at': pt(7, -1.5), 'direction': pt(1, 0)},
                            refused(Refused('unsupportedWall', ROOM2, 'arc'), 'space')))
    out.append(refused_case('Ret: mahale eğik doğrultu', h,
                            {'layerId': 'olcu', 'space': ROOM, 'at': pt(2, -1.5), 'direction': pt(3, 4)},
                            refused(Refused('unsupportedWall', ROOM, 'oblique'), 'space')))
    # A broken opening keeps its last jambs (a document's opening has four, KCAD v2).
    broken = opening(0, S, 8.5, 0.8, [(8.1, 0.125), (8.1, -0.125), (8.9, 0.125), (8.9, -0.125)], broken='offWall')
    out.append(refused_case('Ret: kaynağın yerleşemeyen açıklığı', house(extra=[(BROKEN, broken)]),
                            {'layerId': 'olcu', 'walls': [S], 'at': at_south},
                            refused(Refused('openingUnplaced'), 'walls')))
    out.append(refused_case('Ret: beş sıra', h,
                            {'layerId': 'olcu', 'walls': [S], 'at': at_south,
                             'rows': ['openings', 'walls', 'grids', 'overall', 'openings']},
                            failed('Ölçü dizisinin sıraları boş veya yineleniyor.', 'operation')))
    return out


# ── A drawing as commands leave it (§4) ─────────────────────────────────

def nano(v):
    """A coordinate to a nanometre, rounded half away from zero."""
    x = v * 1e9
    return int(math.copysign(math.floor(abs(x) + 0.5), x))


def interval(row, a, b):
    """An interval's identity apart from the places naming it: its row and its two objects with where they are."""
    ka = (uid_order(a[0]), nano(a[1][0]), nano(a[1][1]))
    kb = (uid_order(b[0]), nano(b[1][0]), nano(b[1][1]))
    return (row, tuple(sorted([ka, kb])))


def seg_key(segment):
    return json.dumps(segment, sort_keys=True)


class Sim:
    """A drawing as commands leave it: its objects in their slots, its recipes with their members."""

    def __init__(self, doc):
        self.doc = cp(doc)
        self.next = max(e['id'] for e in self.doc['entities']) + 1
        self.sets = cp(self.doc.get('dimensionSets', []))

    def index(self, u):
        return self.doc['uids'].index(u) if u in self.doc['uids'] else None

    def entity(self, u):
        i = self.index(u)
        return None if i is None else self.doc['entities'][i]

    def slot(self, u):
        return self.entity(u)['id']

    def add(self, e):
        slot = self.next
        self.next += 1
        e = cp(e)
        e['id'] = slot
        u = f'$uidOf:{slot}'
        self.doc['entities'].append(e)
        self.doc['uids'].append(u)
        return u

    def remove(self, u):
        i = self.index(u)
        del self.doc['entities'][i]
        del self.doc['uids'][i]

    def drawing(self):
        d = cp(self.doc)
        d['dimensionSets'] = self.sets
        return Drawing(d)

    def write(self, rec):
        """A recipe's members planned and written (§4): the same pair, or the same interval, keeps its dimension."""
        dr = self.drawing()
        pl = plan(dr, rec)
        stored = {m['uid']: self.entity(m['uid']) for m in rec.get('members', []) if self.entity(m['uid'])}
        old_by_key = {seg_key({k: m[k] for k in ('row', 'a', 'b')}): m for m in rec.get('members', [])}
        claimed = {old_by_key[seg_key(sp['segment'])]['uid'] for sp in pl.spans
                   if seg_key(sp['segment']) in old_by_key and old_by_key[seg_key(sp['segment'])]['uid'] in stored}
        by_interval = {}
        for m in rec.get('members', []):
            if m['uid'] in claimed or m['uid'] not in stored:
                continue
            d = stored[m['uid']]
            by_interval.setdefault(interval(m['row'], (m['a']['uid'], xy(d['a'])), (m['b']['uid'], xy(d['b']))), m)
        planned = []
        for sp in pl.spans:
            seg = sp['segment']
            m = old_by_key.get(seg_key(seg))
            if m is not None and m['uid'] not in stored:
                m = None
            if m is None:
                found = by_interval.get(interval(seg['row'], (seg['a']['uid'], sp['a']), (seg['b']['uid'], sp['b'])))
                if found is not None and found['uid'] not in claimed:
                    claimed.add(found['uid'])
                    m = found
            planned.append((m['uid'] if m else None, sp))
        kept = {u for u, _ in planned if u}
        removed = [m['uid'] for m in rec.get('members', []) if m['uid'] not in kept]
        for u in removed:
            if self.entity(u):
                self.remove(u)
        members, changed, created = [], [], []
        for u, sp in planned:
            if u:
                e = self.entity(u)
                before = cp(e)
                e.update({'a': pt(*sp['a']), 'b': pt(*sp['b']), 'offset': sp['offset'], 'style': 'linear',
                          'angle': sp['angle'], 'refs': cp(sp['segment'])})
                del e['refs']['row']
                if e != before:
                    changed.append(u)
            else:
                u = self.add(dimension(dr, rec, sp))
                created.append(u)
            members.append({'uid': u, **cp(sp['segment'])})
        rec['members'] = members
        return pl, removed, changed, created

    def create(self, inp):
        dr = self.drawing()
        recs, _ = recipe(dr, inp)
        ids = []
        for i, rec in enumerate(recs):
            rec['id'] = f'$set:{len(self.sets)}'
            rec['members'] = []
            self.write(rec)
            self.sets.append(rec)
            ids.append(rec['id'])
        return ids

    def fields(self, rec):
        out = {}
        for m in rec['members']:
            e = self.entity(m['uid'])
            out[str(e['id'])] = {k: cp(e[k]) for k in ('a', 'b', 'offset', 'angle', 'refs', 'layerId')}
        return out

    def expected_sets(self, ids=None):
        out = []
        for rec in self.sets:
            r = cp(rec)
            if ids is not None:
                r['id'] = ids.get(r['id'], r['id'])
            elif r['id'].startswith('$set:'):
                r['id'] = '$any'
            out.append(r)
        return out

    def state(self, **more):
        out = {'ids': sorted(e['id'] for e in self.doc['entities']), 'dimensionSets': self.expected_sets(),
               'fields': {k: v for rec in self.sets for k, v in self.fields(rec).items()}}
        out.update(more)
        return out


def fixed_members(sim, base):
    """A drawing with its recipes' new identities fixed (`base`, `base + 1` …): an edit's setup."""
    doc = cp(sim.doc)
    names = {}
    for i, u in enumerate(doc['uids']):
        if u.startswith('$uidOf:'):
            names[u] = uid(base + len(names))
            doc['uids'][i] = names[u]
    sets = cp(sim.sets)
    text = json.dumps(sets)
    for old, new in names.items():
        text = text.replace(f'"{old}"', f'"{new}"')
    doc['dimensionSets'] = json.loads(text)
    return doc


def follow_cases():
    """§4: a recipe written, then another command changes its sources; the members follow in the same step."""
    out = []
    rows3 = ['openings', 'walls', 'overall']
    create = {'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5), 'rows': rows3}

    def started(doc, inp):
        sim = Sim(doc)
        sim.create(inp)
        return sim

    def written_step(sim, inp):
        return {'op': 'execute', 'input': inp, 'result': '$any', 'within': 1e-9, 'expect': sim.state(canUndo=True)}

    # A door added on the source: the openings row gains its jambs.
    h = house()
    sim = started(h, create)
    first = written_step(sim, create)
    door = opening(0, S, 3.0, 1.0, [(2.5, 0.125), (2.5, -0.125), (3.5, 0.125), (3.5, -0.125)])
    sim.add(door)
    sim.write(sim.sets[0])
    out.append(case('İzleme: kaynağa kapı eklenince açıklık sırası onu ölçer', h, first,
                    {'op': 'execute', 'command': 'cad.openings.create',
                     'input': {'layerId': 'cizim', 'openings': [{'host': S, 'at': 3.0, 'width': 1.0, 'form': 'door'}]},
                     'result': '$any', 'within': 1e-9, 'expect': sim.state()}))
    # The partition deleted: the walls row no longer stops at it.
    sim = started(h, create)
    first = written_step(sim, create)
    sim.remove(P)
    sim.write(sim.sets[0])
    out.append(case('İzleme: saplanan bölme silinince duvarlar sırası onsuz', h, first,
                    {'op': 'execute', 'command': 'cad.entities.delete', 'input': {'uids': [P]}, 'result': '$any',
                     'within': 1e-9, 'expect': sim.state()}))
    # The source split: its piece joins the sources, nothing measured changes.
    sim = started(h, create)
    first = written_step(sim, create)
    s_e = sim.entity(S)
    piece = cp(s_e)
    s_e['pts'] = [pt(0, 0), pt(5, 0)]
    piece['pts'] = [pt(5, 0), pt(10, 0)]
    new = sim.add(piece)
    sim.entity(D1)['host'] = new
    sim.sets[0]['span']['walls'] = [new]
    sim.write(sim.sets[0])
    out.append(case('İzleme: kaynak bölününce parçası kaynaklara katılır, ölçüler ve kimlikleri aynı', h, first,
                    {'op': 'execute', 'command': 'cad.walls.edit',
                     'input': {'uids': [S], 'operation': {'kind': 'split', 'at': pt(5, 0)}}, 'result': '$any',
                     'within': 1e-9, 'expect': sim.state()}))
    # The source turned: its openings' jambs swap names, the same intervals keep their dimensions.
    sim = started(h, create)
    first = written_step(sim, create)
    s_e = sim.entity(S)
    s_e['pts'] = [pt(10, 0), pt(0, 0)]
    for u, (lo_x, hi_x) in ((D1, (6, 7)), (D2, (1, 2))):
        o = sim.entity(u)
        o['place'] = [pt(hi_x, -0.125), pt(hi_x, 0.125), pt(lo_x, -0.125), pt(lo_x, 0.125)]
    sim.write(sim.sets[0])
    out.append(case('İzleme: kaynak ters çevrilince pervaz adları değişir, aynı aralıklar kimliğini korur', h, first,
                    {'op': 'execute', 'command': 'cad.walls.edit', 'input': {'uids': [S], 'operation': {'kind': 'flip'}},
                     'result': '$any', 'within': 1e-9, 'expect': sim.state()}))
    # A source deleted: it leaves the sources; the last one gone breaks the recipe and keeps its dimensions.
    two = {'layerId': 'olcu', 'walls': [S, N], 'at': pt(5, -1.5), 'direction': pt(1, 0)}
    sim = started(h, two)
    first = written_step(sim, two)
    sim.remove(N)
    sim.sets[0]['span'].pop('walls')
    sim.write(sim.sets[0])
    out.append(case('İzleme: kaynaklardan biri silinince dizi kalanı ölçer', h, first,
                    {'op': 'execute', 'command': 'cad.entities.delete', 'input': {'uids': [N]}, 'result': '$any',
                     'within': 1e-9, 'expect': sim.state()}))
    return out


def edit_setups():
    """The edits' drawings: the house with a south façade recipe (and one wall's recipe), its west room's, the
    plastered house's, the house with its south wall drawn in two."""
    h = house()
    sim = Sim(h)
    sim.create({'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5)})
    sim.sets[0]['id'] = SET1
    facade_doc = fixed_members(sim, 0x100)
    room = Sim(h)
    room.create({'layerId': 'olcu', 'space': ROOM, 'at': pt(2, -1.5), 'direction': pt(1, 0)})
    room.sets[0]['id'] = SET2
    room_doc = fixed_members(room, 0x100)
    plastered = Sim(core_house())
    plastered.create({'layerId': 'olcu', 'walls': [CS], 'at': pt(5, -1.5), 'rows': ['openings', 'walls', 'overall']})
    plastered.sets[0]['id'] = SET1
    plastered_doc = fixed_members(plastered, 0x100)
    halves = house()
    halves['entities'][0]['pts'] = [pt(0, 0), pt(5, 0)]
    half = wall(len(halves['entities']) + 1, (5, 0), (10, 0))
    halves['entities'].append(half)
    halves['uids'].append(uid(13))
    halves['entities'][5]['host'] = uid(13)
    two = Sim(halves)
    two.create({'layerId': 'olcu', 'walls': [S, uid(13)], 'at': pt(5, -1.5), 'direction': pt(1, 0)})
    two.sets[0]['id'] = SET1
    halves_doc = fixed_members(two, 0x100)
    return facade_doc, room_doc, plastered_doc, halves_doc


def edit_output(doc, set_id, operation):
    """An edit's plan (§5): the recipe changed, its members planned with their identities; and what it writes."""
    sim = Sim(doc)
    rec = next(r for r in sim.sets if r['id'] == set_id)
    before = cp(rec)
    kind = operation['kind']
    if kind == 'set':
        for key in ('rows', 'openings', 'gapMm'):
            if key in operation:
                rec[key] = cp(operation[key])
        if 'rows' in operation:
            rec['members'] = [m for m in rec['members'] if m['row'] in operation['rows']]
            if 'rowPlaces' in rec:
                rec['rowPlaces'] = [r for r in rec['rowPlaces'] if r['row'] in operation['rows']]
        if 'reference' in operation:
            if operation['reference'] == 'faces':
                rec['span'].pop('reference', None)
            else:
                rec['span']['reference'] = operation['reference']
    elif kind == 'sources':
        rec['wall'] = operation['walls'][0]
        if operation['walls'][1:]:
            rec['span']['walls'] = operation['walls'][1:]
        else:
            rec['span'].pop('walls', None)
    elif kind == 'move':
        rec['at'] = cp(operation['at'])
    elif kind == 'rowOffset':
        pl = plan(sim.drawing(), rec)
        row = operation['row']
        now = next((tuple(sp['at']) for sp in pl.spans if sp['segment']['row'] == row), xy(rec['at']))
        places = {r['row']: r for r in rec.get('rowPlaces', [])}
        if row in places:
            now = xy(places[row]['at'])
        n = (-pl.d[1], pl.d[0])
        across = (now[0] - pl.origin[0]) * n[0] + (now[1] - pl.origin[1]) * n[1]
        face = pl.body[1] if pl.side > 0.0 else pl.body[0]
        delta = face + pl.side * operation['offset'] - across
        at = pt(now[0] + n[0] * delta, now[1] + n[1] * delta)
        if row in places:
            places[row]['at'] = at
        else:
            rec.setdefault('rowPlaces', []).append({'row': row, 'at': at})
    dr = sim.drawing()
    pl = plan(dr, rec)
    entities_before = {m['uid']: cp(sim.entity(m['uid'])) for m in before['members']}
    planned_sim = Sim(sim.doc)
    planned_sim.sets = sim.sets
    pl, removed, changed, created = planned_sim.write(rec)
    entities, uids = [], []
    for m, sp in zip(rec['members'], pl.spans):
        e = cp(planned_sim.entity(m['uid']))
        if m['uid'].startswith('$uidOf:'):
            e['id'] = 0
        else:
            uids.append(m['uid'])
        entities.append(e)
    dims = entities
    tight = tight_ones(dims)
    output = {'set': set_id, 'entities': entities, 'spans': [cp(sp['segment']) for sp in pl.spans],
              'removed': [m['uid'] for m in before['members'] if m['uid'] not in uids], 'uids': uids,
              'revision': '$current', 'rows': pl.notes}
    if pl.skipped:
        output['skipped'] = pl.skipped
    if tight:
        output['tight'] = tight
    warnings = layer_warnings(doc, rec['layerId'])
    if tight:
        warnings.append(tight_warning(len(tight)))
    if pl.skipped:
        warnings.append(skipped_warning(len(pl.skipped)))
    planned_out = completed(output, warnings)
    after = {m['uid'] for m in rec['members']}
    written_out = completed({'set': set_id, 'uids': [m['uid'] if not m['uid'].startswith('$') else '$any'
                                                     for m in rec['members']],
                             'created': ['$any'] * len(created),
                             'removed': sorted(u for u in entities_before if u not in after),
                             'changed': sorted(u for u in changed if not u.startswith('$')),
                             'revision': '$current'}, warnings)
    return planned_out, written_out, planned_sim


def edit_cases():
    out = []
    facade_doc, room_doc, plastered_doc, halves_doc = edit_setups()

    def planned_edit(name, doc, set_id, operation):
        result, _, _ = edit_output(doc, set_id, operation)
        out.append(case(name, doc, step('plan', {'id': set_id, 'operation': operation}, result, unchanged(doc))))

    def written_edit(name, doc, set_id, operation):
        _, result, sim = edit_output(doc, set_id, operation)
        before = {'ids': sorted(e['id'] for e in doc['entities']), 'dimensionSets': doc['dimensionSets']}
        out.append(case(name, doc,
                        step('execute', {'id': set_id, 'operation': operation}, result,
                             sim.state(canUndo=True, dirty=True, revision='changed')),
                        {'op': 'undo', 'expect': before}))

    def refused_edit(name, doc, set_id, operation, result):
        out.append(case(name, doc, step('plan', {'id': set_id, 'operation': operation}, result, unchanged(doc))))

    rows3 = ['openings', 'walls', 'overall']
    planned_edit('Sıralar: duvarlar sırası eklenir, açıklık ölçüleri kimliğini korur, toplam bir sıra dışa',
                 facade_doc, SET1, {'kind': 'set', 'rows': rows3})
    written_edit('Sıralar: yaz ve geri al', facade_doc, SET1, {'kind': 'set', 'rows': rows3})
    planned_edit('Sıralar: akslar sırası', facade_doc, SET1, {'kind': 'set', 'rows': ['openings', 'grids', 'overall']})
    planned_edit('Açıklık merkezleri', facade_doc, SET1, {'kind': 'set', 'openings': 'centers'})
    planned_edit('Sıra aralığı 10 mm: toplam sırası dışa kayar', facade_doc, SET1, {'kind': 'set', 'gapMm': 10.0})
    planned_edit('Diziyi taşı: bütün sıralar birlikte', facade_doc, SET1, {'kind': 'move', 'at': pt(5, -3)})
    planned_edit('Sıra uzaklığı: toplam dış yüzden 2 m', facade_doc, SET1,
                 {'kind': 'rowOffset', 'row': 'overall', 'offset': 2.0})
    written_edit('Sıra uzaklığı: yaz ve geri al', facade_doc, SET1,
                 {'kind': 'rowOffset', 'row': 'overall', 'offset': 2.0})
    planned_edit('Ölçülen çizgi: çekirdek, kapı ölçüsü kimliğini korur', plastered_doc, SET1,
                 {'kind': 'set', 'reference': 'core'})
    written_edit('Ölçülen çizgi: çekirdek yaz ve geri al', plastered_doc, SET1, {'kind': 'set', 'reference': 'core'})
    planned_edit('Kaynaklar: ikinci yarı çıkınca dizi serbest uca kısalır', halves_doc, SET1,
                 {'kind': 'sources', 'walls': [S]})
    written_edit('Kaynaklar: yaz ve geri al', halves_doc, SET1, {'kind': 'sources', 'walls': [S]})
    planned_edit('Kaynaklar: doğu duvarı katılır, ölçüler aynı', facade_doc, SET1, {'kind': 'sources', 'walls': [S, E]})
    planned_edit('Mahal: duvarlar sırası kolon boşluğunu ölçer', room_doc, SET2, {'kind': 'set', 'rows': rows3})
    written_edit('Mahal: sıralar yaz ve geri al', room_doc, SET2, {'kind': 'set', 'rows': rows3})
    planned_edit('Mahal: sıra uzaklığı', room_doc, SET2, {'kind': 'rowOffset', 'row': 'overall', 'offset': 1.0})
    oblique = house(extra=[(OBL, wall(0, (8.5, 0), (9.5, 3)))])
    osim = Sim(oblique)
    osim.create({'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5)})
    osim.sets[0]['id'] = SET1
    oblique_doc = fixed_members(osim, 0x100)
    planned_edit('Eğik bölme: duvarlar sırasında ölçülmez ve söylenir', oblique_doc, SET1, {'kind': 'set', 'rows': rows3})
    # Refusals: nothing written.
    refused_edit('Ret: cephe dizisinde yüz', facade_doc, SET1, {'kind': 'set', 'face': 'right'},
                 failed('Cephe ve mahal dizisinde yüz, sıraların yanından seçilir; ölçülen çizgi için reference verin.',
                        'operation.face'))
    refused_edit('Ret: mahal dizisine duvar', room_doc, SET2, {'kind': 'sources', 'walls': [S]},
                 failed('Mahal dizisinin kaynağı mahaldir; duvar verilmez.', 'operation.walls'))
    refused_edit('Ret: kaynaksız cephe', facade_doc, SET1, {'kind': 'sources', 'walls': []},
                 failed('Cephenin en az bir duvarı olmalı.', 'operation.walls'))
    refused_edit('Ret: kaynak bir duvar değil', facade_doc, SET1, {'kind': 'sources', 'walls': [S, D1]},
                 refused(Refused('unsupportedWall', D1), 'operation.walls[1]'))
    refused_edit('Ret: kaynak çizimde yok', facade_doc, SET1, {'kind': 'sources', 'walls': [S, GHOST]},
                 refused(Refused('hostGone', GHOST), 'operation.walls[1]'))
    refused_edit('Ret: aynı duvar iki kez', facade_doc, SET1, {'kind': 'sources', 'walls': [S, S]},
                 failed('Aynı duvar iki kez verildi.', 'operation.walls[1]'))
    levels = house(levels={S: 'zemin', N: 'kat-1'})
    lsim = Sim(levels)
    lsim.create({'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5)})
    lsim.sets[0]['id'] = SET1
    levels_doc = fixed_members(lsim, 0x100)
    refused_edit('Ret: kaynaklar farklı katlara düşer', levels_doc, SET1, {'kind': 'sources', 'walls': [S, N]},
                 refused(Refused('mixedSources', N), 'operation'))
    refused_edit('Ret: dört sıradan fazla', facade_doc, SET1,
                 {'kind': 'set', 'rows': ['openings', 'walls', 'grids', 'overall', 'walls']},
                 failed('Ölçü dizisinin sıraları boş veya yineleniyor.', 'operation'))
    return out


def build():
    out = {}
    for suffix, command, cases in [('create.span', CREATE, create_cases() + follow_cases()),
                                   ('edit.span', EDIT, edit_cases())]:
        assert len(cases) >= 20, (suffix, len(cases))
        # The setup most cases share is the file's; a case states its own only when it differs.
        texts = [json.dumps(c['setup'], sort_keys=True) for c in cases]
        common = json.loads(max(set(texts), key=texts.count))
        for c in cases:
            if c['setup'] == common:
                c.pop('setup')
        out[DEST / f'cad.dimensionsets.{suffix}.json'] = {
            'format': 'kentos.command-cases', 'version': 1, 'command': command, 'commandVersion': 1,
            'title': 'Cephe, duvar grubu ve mahal ölçü dizileri: ' + suffix.split('.')[0],
            'note': ('ADR 0729. Bağımsız başvuru scripts/fixtures/dimension_spans_cases.py: duvarın çizgileri yolunun '
                     'ötelenmesi, sıralar ADR §2 kurallarından, sığmayan değer çizim yazı tipinin ölçülmüş '
                     'ilerlemelerinden; KentOS kodu çalıştırılmaz. Yeni kimlikler yalnız yazmada atanır.'),
            'setup': common, 'cases': cases}
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    bad = []
    for path, value in build().items():
        text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        if args.check:
            if not path.exists() or path.read_text() != text:
                bad.append(str(path.relative_to(ROOT)))
        else:
            path.write_text(text)
            print(path.relative_to(ROOT), len(value['cases']), 'bağımsız durum')
    if bad:
        print('Güncel değil: ' + ', '.join(bad))
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
