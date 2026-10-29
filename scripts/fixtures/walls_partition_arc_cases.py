#!/usr/bin/env python3
"""Independent 50-digit reference for measured partitions on arc edges (docs/adr/0734); no KentOS code.

An arc edge a → b with bulge β = tan(θ/4) (θ its sweep, counter-clockwise positive) lies on the circle whose centre
is on the chord's perpendicular bisector, cot(θ/2) half-chords to its left; its radius is the half chord over
|sin(θ/2)|. Walking the arc, the left offset d moves to radius r − sgn(θ)·d. A partition on an arc source is its
concentric arc: at radius r − sgn(θ)·(centre offset + side·axis distance), spanning the source's angles, a length
along its own axis, or between two limits (lines or arcs) at the meeting points nearest the source's middle.

    uv run --no-project --with mpmath python3 scripts/fixtures/walls_partition_arc_cases.py --check
"""
import argparse
import copy
import json
from pathlib import Path

import mpmath as mp

from walls_rectangle_cases import SETUP

mp.mp.dps = 50
M = mp.mpf
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/cad.walls.partition.arcs.json'
QUARTER = float(mp.tan(mp.pi / 8))  # a quarter turn's bulge, as the file stores it


def point(p):
    return {'x': float(p[0]), 'y': float(p[1])}


def circle(a, b, bulge):
    """Centre, radius, start angle and sweep of the arc a → b."""
    a, b, bulge = (M(a[0]), M(a[1])), (M(b[0]), M(b[1])), M(bulge)
    sweep = 4 * mp.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    half = mp.sqrt(dx * dx + dy * dy) / 2
    unit = (-dy / (2 * half), dx / (2 * half))
    reach = half * mp.cot(sweep / 2)
    c = ((a[0] + b[0]) / 2 + unit[0] * reach, (a[1] + b[1]) / 2 + unit[1] * reach)
    r = half / abs(mp.sin(sweep / 2))
    return c, r, mp.atan2(a[1] - c[1], a[0] - c[0]), sweep


def at(c, r, angle):
    return (c[0] + r * mp.cos(angle), c[1] + r * mp.sin(angle))


def sgn(x):
    return -1 if x < 0 else 1


def left_of(w):
    """The wall's left face's offset from its path (justification and offset)."""
    width = M(w['width'])
    return M(w.get('offset', 0)) + {'left': 0, 'center': width / 2, 'right': width}[w.get('justify', 'center')]


def on_arc(p, c, a0, sweep):
    along = mp.fmod(sgn(sweep) * (mp.atan2(p[1] - c[1], p[0] - c[0]) - a0), 2 * mp.pi)
    if along < 0:
        along += 2 * mp.pi
    return along <= abs(sweep) + M('1e-12') or 2 * mp.pi - along <= M('1e-12')


class Carrier:
    """The partition's centre line: ('line', origin, u) or ('circle', c, r, a0, sweep); t its length along."""

    def __init__(self, kind, *data):
        self.kind, self.data = kind, data

    def at(self, t):
        if self.kind == 'line':
            o, u = self.data
            return (o[0] + u[0] * t, o[1] + u[1] * t)
        c, r, a0, sweep = self.data
        return at(c, r, a0 + sgn(sweep) * t / r)

    def along(self, p):
        if self.kind == 'line':
            o, u = self.data
            return (p[0] - o[0]) * u[0] + (p[1] - o[1]) * u[1]
        c, r, a0, sweep = self.data
        half = abs(sweep) / 2
        angle = sgn(sweep) * (mp.atan2(p[1] - c[1], p[0] - c[0]) - a0) - half
        angle -= 2 * mp.pi * mp.floor((angle + mp.pi) / (2 * mp.pi))
        return r * (half + angle)

    def meets(self, limit):
        """Lengths along of the meeting points on the limit: ('line', at, v) s in [0, 1], or ('arc', c, r, a0, sweep)."""
        found = []
        if limit[0] == 'line':
            _, p0, v = limit
            # Points p0 + v·s on the carrier.
            if self.kind == 'line':
                o, u = self.data
                den = u[0] * v[1] - u[1] * v[0]
                if den == 0:
                    raise ValueError('parallel')
                q = (p0[0] - o[0], p0[1] - o[1])
                t = (q[0] * v[1] - q[1] * v[0]) / den
                s = (q[0] * u[1] - q[1] * u[0]) / den
                if 0 <= s <= 1:
                    found.append(t)
            else:
                c, r, _, _ = self.data
                w = (p0[0] - c[0], p0[1] - c[1])
                aa, bb = v[0] ** 2 + v[1] ** 2, v[0] * w[0] + v[1] * w[1]
                disc = bb * bb - aa * (w[0] ** 2 + w[1] ** 2 - r * r)
                if disc >= 0:
                    for s in [(-bb - mp.sqrt(disc)) / aa, (-bb + mp.sqrt(disc)) / aa]:
                        if 0 <= s <= 1:
                            found.append(self.along((p0[0] + v[0] * s, p0[1] + v[1] * s)))
        else:
            _, c2, r2, a0, sweep = limit
            if self.kind == 'line':
                o, u = self.data
                w = (o[0] - c2[0], o[1] - c2[1])
                b = u[0] * w[0] + u[1] * w[1]
                disc = b * b - (w[0] ** 2 + w[1] ** 2 - r2 * r2)
                if disc >= 0:
                    for t in [-b - mp.sqrt(disc), -b + mp.sqrt(disc)]:
                        if on_arc(self.at(t), c2, a0, sweep):
                            found.append(t)
            else:
                c, r, _, _ = self.data
                d = (c2[0] - c[0], c2[1] - c[1])
                dist = mp.sqrt(d[0] ** 2 + d[1] ** 2)
                if dist <= M('1e-9') * max(1, r, r2):
                    raise ValueError('concentric')
                e = (d[0] / dist, d[1] / dist)
                x = (r * r - r2 * r2 + dist * dist) / (2 * dist)
                if r * r - x * x >= 0:
                    h = mp.sqrt(r * r - x * x)
                    for side in (-1, 1):
                        p = (c[0] + e[0] * x - e[1] * side * h, c[1] + e[1] * x + e[0] * side * h)
                        if on_arc(p, c2, a0, sweep):
                            found.append(self.along(p))
        return found


def limit_of(w):
    a, b = (M(w['pts'][0]['x']), M(w['pts'][0]['y'])), (M(w['pts'][1]['x']), M(w['pts'][1]['y']))
    centre = left_of(w) - M(w['width']) / 2
    bulge = (w.get('bulges') or [0])[0]
    if bulge == 0:
        v = (b[0] - a[0], b[1] - a[1])
        length = mp.sqrt(v[0] ** 2 + v[1] ** 2)
        n = (-v[1] / length, v[0] / length)
        return ('line', (a[0] + n[0] * centre, a[1] + n[1] * centre), v)
    c, r, a0, sweep = circle(a, b, bulge)
    return ('arc', c, r - sgn(sweep) * centre, a0, sweep)


def partition(source, side, axis, distance, width, extent, limits=()):
    """The partition's path, bulge, source face, face bulge, dimension, clear and axis distances; or a refusal."""
    a = (M(source['pts'][0]['x']), M(source['pts'][0]['y']))
    b = (M(source['pts'][1]['x']), M(source['pts'][1]['y']))
    bulge = (source.get('bulges') or [0])[0]
    ws, wp, distance = M(source['width']), M(width), M(distance)
    axis_distance = distance if axis else distance + (ws + wp) / 2
    clear = axis_distance - (ws + wp) / 2
    left = left_of(source)
    offset = left - ws / 2 + side * axis_distance
    face_offset = left if side == 1 else left - ws
    if bulge == 0:
        v = (b[0] - a[0], b[1] - a[1])
        length = mp.sqrt(v[0] ** 2 + v[1] ** 2)
        u = (v[0] / length, v[1] / length)
        n = (-u[1], u[0])
        carrier = Carrier('line', (a[0] + n[0] * offset, a[1] + n[1] * offset), u)
        span = length
    else:
        c, r, a0, sweep = circle(a, b, bulge)
        rp = r - sgn(sweep) * offset
        if not rp - wp / 2 > 0:
            return 'Bölme yayın merkezine ulaşıyor; daha küçük bir uzaklık ya da yayın dış yanını seçin.'
        carrier = Carrier('circle', c, rp, a0, sweep)
        span = rp * abs(sweep)
    if extent['kind'] == 'source':
        t0, t1 = M(0), span
    elif extent['kind'] == 'length':
        anchor, length = M(extent['anchor']), M(extent['length'])
        t0, t1 = {'forward': (anchor, anchor + length), 'backward': (anchor - length, anchor),
                  'both': (anchor - length / 2, anchor + length / 2)}[extent.get('direction', 'forward')]
        if t0 < 0 or t1 > span:
            return 'Verilen boy kaynak kenarın uçlarını aşıyor; boyu veya başlangıcı düzeltin.'
    else:
        middle, ts = span / 2, []
        for w in limits:
            try:
                found = carrier.meets(limit_of(w))
            except ValueError as why:
                return {'parallel': 'Sınır duvarı bölmeye paralel; kesişen başka bir kenar seçin.',
                        'concentric': 'Sınır duvarı bölmeyle eş merkezli; kesişen başka bir kenar seçin.'}[str(why)]
            if not found:
                return 'Bölme seçilen sınır kenarına ulaşmıyor; başka bir kenar seçin.'
            ts.append(min(found, key=lambda t: (abs(t - middle), t)))
        t0, t1 = min(ts), max(ts)
    path = [carrier.at(t0), carrier.at(t1)]
    if carrier.kind == 'line':
        face = [(a[0] + n[0] * face_offset, a[1] + n[1] * face_offset), (b[0] + n[0] * face_offset, b[1] + n[1] * face_offset)]
        mid = (face[0][0] + u[0] * (t0 + t1) / 2, face[0][1] + u[1] * (t0 + t1) / 2)
        return {'path': path, 'bulge': None, 'face': face, 'faceBulge': None,
                'dimension': [mid, (mid[0] + n[0] * side * clear, mid[1] + n[1] * side * clear)],
                'clear': clear, 'axis': axis_distance}
    face_r = r - sgn(sweep) * face_offset
    middle = a0 + sgn(sweep) * (t0 + t1) / 2 / rp
    return {'path': path, 'bulge': mp.tan(sgn(sweep) * (t1 - t0) / rp / 4),
            'face': [at(c, face_r, a0), at(c, face_r, a0 + sweep)], 'faceBulge': bulge,
            'dimension': [at(c, face_r, middle), at(c, face_r - sgn(sweep) * side * clear, middle)],
            'clear': clear, 'axis': axis_distance}


def wall(id_, pts, width, bulge=None, justify='center', offset=None):
    w = {'kind': 'wall', 'id': id_, 'layerId': 'base', 'attrs': {}, 'pts': [point(p) for p in pts], 'width': width,
         'justify': justify}
    if bulge is not None:
        w['bulges'] = [bulge, 0.0]  # the document keeps one per point, an open path's last one 0
    if offset:
        w['offset'] = offset
    return w


def shifted(p, origin):
    return (p[0] + origin[0], p[1] + origin[1])


def case(name, entities, side, axis, distance, width, extent, unit='m', limits=(), plan=False, undo=False,
         origin=(0, 0)):
    setup = copy.deepcopy(SETUP)
    setup['entities'] = entities
    setup['settings']['drawingUnit'] = unit
    source = entities[0]
    steps = [{'op': 'captureUid', 'id': 1, 'as': 'source'}]
    ext = {'kind': extent['kind']}
    if extent['kind'] == 'length':
        ext.update({k: extent[k] for k in ('anchor', 'length', 'direction') if k in extent})
    if extent['kind'] == 'between':
        steps += [{'op': 'captureUid', 'id': 2, 'as': 'start'}, {'op': 'captureUid', 'id': 3, 'as': 'end'}]
        ext.update({'start': {'wall': '$uid:start', 'edge': 0}, 'end': {'wall': '$uid:end', 'edge': 0}})
    inp = {'source': {'wall': '$uid:source', 'edge': 0}, 'side': 'left' if side == 1 else 'right',
           'measure': 'axis' if axis else 'clear', 'distance': distance, 'extent': ext,
           'wall': {'layerId': 'duvar', 'walls': [{'pts': [], 'closed': False, 'width': width}]}}
    ids = [e['id'] for e in entities]
    before = {'ids': ids, 'entities': {str(e['id']): e for e in entities}, 'canUndo': False, 'revision': 'same'}
    got = partition(source, side, axis, distance, width, extent, limits)
    within = 1e-8 if origin != (0, 0) else 1e-11
    if isinstance(got, str):
        steps.append({'op': 'execute', 'input': inp, 'result': {'status': 'failed', 'error': {
            'code': 'invalid_wall', 'message': got, 'path': 'extent'}}, 'expect': {**before}})
        return {'name': name, 'setup': setup, 'steps': steps}
    new_id = max(ids) + 1
    created = {'kind': 'wall', 'id': new_id, 'layerId': 'duvar', 'attrs': {}, 'pts': [point(p) for p in got['path']]}
    if got['bulge'] is not None:
        created['bulges'] = [float(got['bulge']), 0.0]
    created.update({'width': width, 'justify': 'center'})
    if plan:
        output = {'wall': {'walls': [{**created, 'id': 0}], 'revision': '$current'},
                  'sourceFace': [point(p) for p in got['face']]}
        if got['faceBulge'] is not None:
            output['sourceFaceBulge'] = got['faceBulge']
        output.update({'dimension': [point(p) for p in got['dimension']], 'clearDistance': float(got['clear']),
                       'axisDistance': float(got['axis'])})
        steps.append({'op': 'plan', 'input': inp, 'within': within,
                      'result': {'status': 'completed', 'output': output, 'warnings': []}, 'expect': before})
    after = {'ids': ids + [new_id], 'entities': {**before['entities'], str(new_id): created}, 'canUndo': True,
             'revision': 'changed'}
    steps.append({'op': 'execute', 'input': inp, 'within': within, 'result': {'status': 'completed', 'output': {
        'uids': ['$any'], 'ids': [new_id], 'revision': '$current'}, 'warnings': []}, 'expect': after})
    if undo:
        steps.append({'op': 'undo', 'returns': 'Duvar', 'within': within,
                      'expect': {**before, 'canRedo': True, 'revision': 'changed'}})
        steps.append({'op': 'redo', 'returns': 'Duvar', 'within': within, 'expect': {**after, 'revision': 'changed'}})
    return {'name': name, 'setup': setup, 'steps': steps}


def build():
    cases = []
    ccw = ((5, 0), (0, 5), QUARTER)  # a quarter of the circle of radius 5 about the origin, counter-clockwise
    cw = ((0, 5), (5, 0), -QUARTER)  # the same arc walked clockwise
    units = ['m', 'mm', 'cm', 'in', 'ft']
    # Source extent: both turns, both sides, clear and axis, three justifications, two origins.
    for i, (turn, side, axis, justify, width) in enumerate([
        (ccw, 1, False, 'center', 0.1), (ccw, -1, False, 'center', 0.15), (ccw, 1, True, 'left', 0.1),
        (ccw, -1, True, 'right', 0.2), (cw, 1, False, 'center', 0.1), (cw, -1, False, 'left', 0.15),
        (cw, 1, True, 'right', 0.2), (cw, -1, True, 'center', 0.1)]):
        origin = (500000, 4400000) if i % 3 == 2 else (0, 0)
        a, b, bulge = turn
        source = wall(1, [shifted(a, origin), shifted(b, origin)], 0.2, bulge, justify, 0.05 if i % 2 else None)
        cases.append(case(f'Arc source {i}: {"ccw" if bulge > 0 else "cw"}, {"left" if side == 1 else "right"}, '
                          f'{"axis" if axis else "clear"}, {justify}, unit {units[i % 5]}',
                          [source], side, axis, 1.5, width, {'kind': 'source'}, units[i % 5], plan=i % 2 == 0,
                          undo=i == 0, origin=origin))
    # A length along the partition's own axis from the source start's radial line.
    source = wall(1, [ccw[0], ccw[1]], 0.2, ccw[2])
    for direction, anchor in [('forward', 0.5), ('backward', 4.0), ('both', 2.5)]:
        cases.append(case(f'Arc length {direction}', [copy.deepcopy(source)], 1, False, 1.5, 0.1,
                          {'kind': 'length', 'anchor': anchor, 'length': 2.0, 'direction': direction}, plan=True))
    cases.append(case('Arc length past the end is refused', [copy.deepcopy(source)], 1, False, 1.5, 0.1,
                      {'kind': 'length', 'anchor': 4.0, 'length': 2.0, 'direction': 'forward'}))
    # Between two radial walls at 20° and 70°, inside and outside, both turns.
    def radial(id_, degrees, origin=(0, 0)):
        d = (mp.cos(mp.radians(degrees)), mp.sin(mp.radians(degrees)))
        return wall(id_, [shifted((d[0] * 1, d[1] * 1), origin), shifted((d[0] * 9, d[1] * 9), origin)], 0.2)
    for name, turn, side in [('ccw inside', ccw, 1), ('ccw outside', ccw, -1), ('cw outside', cw, 1),
                             ('cw inside', cw, -1)]:
        src = wall(1, [turn[0], turn[1]], 0.2, turn[2])
        walls = [src, radial(2, 70), radial(3, 20)]
        cases.append(case(f'Arc between radial walls, {name}', walls, side, False, 1.5, 0.1, {'kind': 'between'},
                          limits=walls[1:], plan=name == 'ccw inside'))
    # Between a radial wall and an arc wall about (0, 6): the circles meet once on its arc.
    limit_arc = wall(3, [(0, 3), (3, 6)], 0.2, QUARTER)
    walls = [wall(1, [ccw[0], ccw[1]], 0.2, ccw[2]), radial(2, 20), limit_arc]
    cases.append(case('Arc between a radial wall and an arc wall', walls, 1, False, 1.5, 0.1, {'kind': 'between'},
                      limits=walls[1:], plan=True))
    # A straight source between a straight wall and an arc wall about (10, 0).
    walls = [wall(1, [(0, 0), (10, 0)], 0.2), wall(2, [(1, -1), (1, 5)], 0.2), wall(3, [(10, 4), (6, 0)], 0.2, QUARTER)]
    cases.append(case('Straight source between a straight wall and an arc wall', walls, 1, False, 2.0, 0.1,
                      {'kind': 'between'}, limits=walls[1:], plan=True))
    walls = [wall(1, [(500000, 4400000), (500010, 4400000)], 0.2), wall(2, [(500001, 4399999), (500001, 4400005)], 0.2),
             wall(3, [(500010, 4400004), (500006, 4400000)], 0.2, QUARTER)]
    cases.append(case('Straight source and arc limit far from the origin', walls, 1, True, 2.0, 0.15,
                      {'kind': 'between'}, 'mm', limits=walls[1:], origin=(500000, 4400000)))
    # Refusals: the centre, a limit not reached, a concentric limit.
    cases.append(case('Partition past the arc centre is refused', [wall(1, [ccw[0], ccw[1]], 0.2, ccw[2])], 1, False,
                      4.9, 0.1, {'kind': 'source'}))
    walls = [wall(1, [ccw[0], ccw[1]], 0.2, ccw[2]), radial(2, 20), wall(3, [(0, 7), (-3, 10)], 0.2, QUARTER)]
    cases.append(case('An arc limit the partition does not reach is refused', walls, 1, False, 1.5, 0.1,
                      {'kind': 'between'}, limits=walls[1:]))
    walls = [wall(1, [ccw[0], ccw[1]], 0.2, ccw[2]), radial(2, 20), wall(3, [(7, 0), (0, 7)], 0.2, QUARTER)]
    cases.append(case('A concentric arc limit is refused', walls, 1, False, 1.5, 0.1, {'kind': 'between'},
                      limits=walls[1:]))
    return {'format': 'kentos.command-cases', 'version': 1, 'command': 'cad.walls.partition', 'commandVersion': 1,
            'title': 'Independent 50-digit measured partitions on arc edges', 'setup': cases[0]['setup'],
            'cases': cases}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--check', action='store_true')
    args = p.parse_args()
    value = build()
    text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            raise SystemExit('Arc partition fixtures are stale')
    else:
        OUT.write_text(text)
    print(f'cad.walls.partition arcs: {len(value["cases"])} independent cases')


if __name__ == '__main__':
    main()
