#!/usr/bin/env python3
"""Review of a change to how the core takes fit-point curves and ellipses by
chords (docs/adr/0149 §5.3), before frozen fixtures are updated, with
Python's standard library and no KentOS code.

The core's answers come from two hand-run dumps:

    KENTOS_DUMP_CALLS='entityEdges:rastgele 1|nearestSegment:rastgele 2>entityEdges' \
      cargo test -p kentos-geometry-core --test calls_dump -- --ignored --nocapture | grep '^{"also"' > calls.jsonl
    KENTOS_DUMP_STORE=1 cargo test -p kentos-geometry-core --test store -- --nocapture | grep '^{"got"' > store.jsonl

    python3 scripts/fixtures/curve_outline_review.py calls calls.jsonl
    python3 scripts/fixtures/curve_outline_review.py store store.jsonl

For the calls: every vertex of a curve's chords lies on the true curve
(Barry and Goldman's centripetal Catmull-Rom, or C(t) = c + M cos t + N sin t)
within 1e-8 m; the true curve comes no further than 1e-4 m from them;
a path's running lengths add up; nearestSegment's index is the nearest of
the object's chords; lineSource keeps the other objects' edges. For the
store: every point of an answer that moved lies on a curve of the scene
within 1e-4 m, with the old point's distance beside it.
"""

import json, math, sys

TOL = 1e-4
ON = 1e-8
fixtures = {}
def case(file, fn, name):
    if file not in fixtures:
        fixtures[file] = json.load(open(f'fixtures/geometry/v1/{file}', encoding='utf-8'))
    for c in fixtures[file]['cases']:
        if c['fn'] == fn and c['name'] == name:
            return c
    raise KeyError((file, fn, name))

def spline_curve(e):
    """Samples and a distance function for the true curve, in local coordinates."""
    P = [(p['x'], p['y']) for p in e['pts']]
    o = P[0]
    P = [(x - o[0], y - o[1]) for x, y in P]
    closed = e.get('closed', False)
    n = len(P)
    def at(i):
        if closed: return P[i % n]
        if i < 0: return (2*P[0][0]-P[1][0], 2*P[0][1]-P[1][1])
        if i >= n: return (2*P[n-1][0]-P[n-2][0], 2*P[n-1][1]-P[n-2][1])
        return P[i]
    def knot(a, b):
        v = math.sqrt(math.hypot(b[0]-a[0], b[1]-a[1]))
        return v if v else 1e-6
    def lerp(a, b, ta, tb, t):
        d = (tb - ta) or 1e-12
        u, w = (tb - t)/d, (t - ta)/d
        return (a[0]*u + b[0]*w, a[1]*u + b[1]*w)
    spans = []
    for i in range(n if closed else n - 1):
        p0, p1, p2, p3 = at(i-1), at(i), at(i+1), at(i+2)
        t1 = knot(p0, p1); t2 = t1 + knot(p1, p2); t3 = t2 + knot(p2, p3)
        def f(s, p0=p0, p1=p1, p2=p2, p3=p3, t1=t1, t2=t2, t3=t3):
            t = t1 + (t2 - t1) * s
            a1, a2, a3 = lerp(p0,p1,0.0,t1,t), lerp(p1,p2,t1,t2,t), lerp(p2,p3,t2,t3,t)
            b1, b2 = lerp(a1,a2,0.0,t2,t), lerp(a2,a3,t1,t3,t)
            return lerp(b1,b2,t1,t2,t)
        spans.append(f)
    return o, spans

def ellipse_curve(e):
    o = (e['c']['x'], e['c']['y'])
    M = (e['major']['x'], e['major']['y']); r = e['ratio']
    N = (-M[1]*r, M[0]*r)
    t0, t1 = e['t0'], e['t1']
    sw = (t1 - t0) % (2*math.pi)
    if sw < 1e-12: sw = 2*math.pi
    def f(s):
        t = t0 + sw * s
        return (M[0]*math.cos(t) + N[0]*math.sin(t), M[1]*math.cos(t) + N[1]*math.sin(t))
    return o, [f]

def curve_of(e):
    return spline_curve(e) if e['kind'] == 'spline' else ellipse_curve(e)

def seg_dist(p, a, b):
    dx, dy = b[0]-a[0], b[1]-a[1]
    l2 = dx*dx + dy*dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0]-a[0])*dx + (p[1]-a[1])*dy)/l2))
    return math.hypot(p[0]-(a[0]+dx*t), p[1]-(a[1]+dy*t))

def on_curve(q, spans):
    """The distance from q to the true curve: dense samples, then a golden-section refinement
    around each of the nearest few (a closed curve's start is also its last span's end)."""
    cands = []
    for k, f in enumerate(spans):
        for i in range(401):
            s = i / 400
            p = f(s)
            cands.append((math.hypot(p[0]-q[0], p[1]-q[1]), k, s))
    cands.sort()
    best = float('inf')
    g = (math.sqrt(5) - 1) / 2
    for _, k, s in cands[:6]:
        f = spans[k]
        lo, hi = max(0.0, s - 1/400), min(1.0, s + 1/400)
        for _ in range(80):
            a = hi - g*(hi-lo); b = lo + g*(hi-lo)
            da = math.hypot(*(x - y for x, y in zip(f(a), q))); db = math.hypot(*(x - y for x, y in zip(f(b), q)))
            if da < db: hi = b
            else: lo = a
        p = f((lo+hi)/2)
        best = min(best, math.hypot(p[0]-q[0], p[1]-q[1]))
    return best

def check_outline(e, pts, closed, what):
    o, spans = curve_of(e)
    loc = [(x - o[0], y - o[1]) for x, y in pts]
    off = max(on_curve(q, spans) for q in loc)
    assert off <= ON, f'{what}: a vertex {off:.3e} m off the curve'
    # The curve within TOL of the polyline: samples along it, each against nearby chords.
    nseg = len(loc) if closed else len(loc) - 1
    at = 0; worst = 0.0
    for f in spans:
        for i in range(3001):
            p = f(i / 3000)
            best = (float('inf'), at)
            idx = [(at + nseg + k - 16) % nseg for k in range(96)] if closed else range(max(0, at-16), min(nseg, at+80))
            for j in idx:
                d = seg_dist(p, loc[j], loc[(j+1) % len(loc)])
                if d < best[0]: best = (d, j)
            at = best[1]; worst = max(worst, best[0])
    assert worst <= TOL, f'{what}: the curve {worst:.3e} m from its outline'
    return off, worst

def edge_pts(edges):
    pts = [(edges[0]['a']['x'], edges[0]['a']['y'])]
    for ed in edges:
        assert ed['kind'] == 'seg'
        pts.append((ed['b']['x'], ed['b']['y']))
    return pts



def review_calls(path):
    report = []
    for line in open(path, encoding='utf-8'):
        d = json.loads(line)
        c = case(d['file'], d['fn'], d['name'])
        what = f"{d['fn']}: {d['name']}"
        got = d['got']
        if d['fn'] == 'entityEdges':
            e = c['args'][0]
            if not got:
                report.append(f'{what}: no edges (as before)'); continue
            pts = edge_pts(got)
            closed = abs(pts[0][0]-pts[-1][0]) < 1e-12 and abs(pts[0][1]-pts[-1][1]) < 1e-12 and len(pts) > 2
            if closed: pts = pts[:-1]
            off, worst = check_outline(e, pts, closed, what)
            report.append(f'{what}: {len(got)} kenar, köşe {off:.1e} m, eğri {worst:.2e} m')
        elif d['fn'] == 'pathOf':
            e = c['args'][0]
            pts = edge_pts(got['edges'])
            closed = got['closed']
            if closed: pts = pts[:-1]
            off, worst = check_outline(e, pts, closed, what)
            cum = got['cum']; acc = 0.0
            assert len(cum) == len(got['edges']), f'{what}: cum has {len(cum)} for {len(got["edges"])} edges'
            for i, ed in enumerate(got['edges']):
                assert abs(cum[i] - acc) <= 1e-9 * max(1, acc), f'{what}: cum {i}'
                acc += math.hypot(ed['b']['x']-ed['a']['x'], ed['b']['y']-ed['a']['y'])
            assert abs(got['length'] - acc) <= 1e-9 * max(1, acc)
            report.append(f'{what}: {len(got["edges"])} kenar, köşe {off:.1e} m, eğri {worst:.2e} m, uzunluk tutarlı')
        elif d['fn'] in ('explodeEntity', 'areaOfEntity'):
            e = c['args'][0]
            ring = got['pieces'][0]['pts'] if d['fn'] == 'explodeEntity' else got['outer']['pts']
            pts = [(p['x'], p['y']) for p in ring]
            off, worst = check_outline(e, pts, True, what)
            report.append(f'{what}: {len(pts)} köşe, köşe {off:.1e} m, eğri {worst:.2e} m')
        elif d['fn'] == 'nearestSegment':
            e, p = c['args'][0], c['args'][1]
            edges = d['also']['got']
            pts = edge_pts(edges)
            q = (p['x'], p['y'])
            dists = [seg_dist(q, (ed['a']['x'], ed['a']['y']), (ed['b']['x'], ed['b']['y'])) for ed in edges]
            best = min(range(len(dists)), key=lambda i: (dists[i], i))
            assert got == best, f'{what}: {got} ≠ {best}'
            closed = len(pts) > 2 and abs(pts[0][0]-pts[-1][0]) < 1e-12 and abs(pts[0][1]-pts[-1][1]) < 1e-12
            off, worst = check_outline(e, pts[:-1] if closed else pts, closed, what)
            report.append(f'{what}: kenar {got} en yakını ({dists[got]:.3e} m), köşe {off:.1e} m, eğri {worst:.2e} m')
        elif d['fn'] == 'lineSource':
            objs = c['args'][0]
            old = c['expect']['edges']; new = got['edges']
            curves = [o for o in objs if o['kind'] in ('spline', 'ellipse')]
            # The old edges of the other objects are kept, in order; the curves' edges are new.
            others_old = [ed for ed in old]  # compared below by exact match of non-curve edges
            new_set = [json.dumps(ed, sort_keys=True) for ed in new]
            kept = sum(1 for ed in old if json.dumps(ed, sort_keys=True) in new_set)
            for e in curves:
                o, spans = curve_of(e)
                # each new seg edge on this curve: both ends on it
                on = [ed for ed in new if ed['kind'] == 'seg' and on_curve((ed['a']['x']-o[0], ed['a']['y']-o[1]), spans) <= ON and on_curve((ed['b']['x']-o[0], ed['b']['y']-o[1]), spans) <= ON]
                assert on, f'{what}: no edge of the curve'
                pts = edge_pts(on)
                closed = len(pts) > 2 and abs(pts[0][0]-pts[-1][0]) < 1e-12 and abs(pts[0][1]-pts[-1][1]) < 1e-12
                off, worst = check_outline(e, pts[:-1] if closed else pts, closed, what)
                report.append(f'{what}: eğrinin {len(on)} kenarı, köşe {off:.1e} m, eğri {worst:.2e} m; öbür nesnelerin {kept} eski kenarı yerinde')
    print('\n'.join(report))


def review_store(path):

    store = json.load(open('fixtures/geometry/v1/store-v1.json', encoding='utf-8'))
    ents = {e['id']: e for e in store['entities']}
    curves = [e for e in store['entities'] if e['kind'] in ('spline', 'ellipse') and (e['kind'] == 'ellipse' or len(e['pts']) >= 2)]
    models = [(e['id'], curve_of(e)) for e in curves]
    def to_curves(p):
        best = (float('inf'), None)
        for cid, (o, spans) in models:
            # cheap reject by bounding box
            d = on_curve((p[0]-o[0], p[1]-o[1]), spans)
            if d < best[0]: best = (d, cid)
        return best
    def points(v, path=''):
        """Every point {x, y} in a value, with its path."""
        if isinstance(v, dict):
            if set(v) >= {'x', 'y'} and isinstance(v['x'], (int, float)):
                yield path, (v['x'], v['y'])
            for k, w in v.items():
                yield from points(w, f'{path}.{k}')
        elif isinstance(v, list):
            for i, w in enumerate(v):
                yield from points(w, f'{path}[{i}]')
    cases = {(c['op'], c['name']): c for c in store['cases']}
    for line in open(path, encoding='utf-8'):
        d = json.loads(line)
        c = cases[(d['op'], d['name'])]
        old = dict(points(c['expect'])); new = dict(points(d['got']))
        moved = [k for k in new if k in old and (abs(new[k][0]-old[k][0]) > 1e-9 or abs(new[k][1]-old[k][1]) > 1e-9)]
        extra = ''
        if not moved and d['op'] == 'extend':
            # an arc's end angle: its end point on the arc
            g, og = d['got']['geometry'], c['expect']['geometry']
            if g.get('kind') == 'arc':
                for nm, a in (('a0', 'a0'), ('a1', 'a1')):
                    if abs(g[a] - og[a]) > 1e-12:
                        pn = (g['c']['x'] + g['r']*math.cos(g[a]), g['c']['y'] + g['r']*math.sin(g[a]))
                        po = (og['c']['x'] + og['r']*math.cos(og[a]), og['c']['y'] + og['r']*math.sin(og[a]))
                        new[f'.geometry.{a}'] = pn; old[f'.geometry.{a}'] = po; moved.append(f'.geometry.{a}')
        for k in moved:
            dn, cn = to_curves(new[k]); do, co = to_curves(old[k])
            ok = dn <= 1e-4
            print(f"{d['op']} {d['name']}{k}: yeni uç eğriden {dn:.2e} m (nesne {cn}), eski {do:.2e} m {'✓' if ok else '✗'}")
            assert ok


if __name__ == '__main__':
    mode, path = sys.argv[1], sys.argv[2]
    {'calls': review_calls, 'store': review_store}[mode](path)
