#!/usr/bin/env python3
"""docs/adr/0730's command cases: a dimension recipe's sheet scales, its values placed where they do not fit, its rows'
rounding, and its copies and moves with its objects, from the ADR's rules alone.

    python3 scripts/fixtures/dimension_layout_cases.py           # writes the files
    python3 scripts/fixtures/dimension_layout_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.dimensionsets.create.layout.json, cad.dimensionsets.edit.layout.json,
cad.entities.transform.recipes.json and cad.entities.array.recipes.json. No KentOS module or program is imported or
run. The rows are ADR 0729's (dimension_spans_cases.py, that ADR's independent reference); this script adds §1–§4: a
row's offset at a sheet scale its distance from the bodies' outermost face times the scales' ratio; a value's width the
drawing face's measured advances (the table paragraph_cases.py reads); the placement's candidates in their order with
the ADR's clearances; the rounding sum in whole thousandths; a copy's objects moved by the copy's map, an opening on its
wall's copy, the recipe moved with them and planned on the copies as an edit plans. IEEE arithmetic in the ADR's order,
so a value's place is the number the rule gives everywhere it is computed the same way (1e-9 is allowed).
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from copy import deepcopy as cp
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dimension_spans_cases import (  # noqa: E402  (ADR 0729's reference: the rows)
    CREATE, EDIT, FACE, GA, GB, GC, N, P, S, D1, D2, WIN, E, W, SET1, Drawing, Refused, Sim, advance, case, completed,
    document, fixed, fixed_members, house, layer_warnings, opening, plan, pt, recipe, skipped_warning, step, uid,
    unchanged, wall, xy,
)

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'fixtures/commands/v1'
TRANSFORM = 'cad.entities.transform'
ARRAY = 'cad.entities.array'
PATH = '0192f5a1-7290-7000-8000-000000000050'  # the path array's line
TEXT_GAP = 0.35  # a value's text gap, times its height, when its look gives none (ADR 0183 §3)


# ── §2: a value as a reader sees it ─────────────────────────────────────

def value(e):
    """A recipe member's measured length along its angle and its text (three decimals in metres)."""
    th = (e['angle'] * math.pi) / 180.0
    u = (math.cos(th), math.sin(th))
    measured = abs((e['b']['x'] - e['a']['x']) * u[0] + (e['b']['y'] - e['a']['y']) * u[1])
    return measured, fixed(measured)


def em(text):
    return sum(advance(FACE, c, False) for c in text) / 1000.0


def fits(b):
    return b['width'] + b['height'] <= b['t1'] - b['t0']


def rect(b, lines, along, across):
    c = (b['t0'] + b['t1']) * 0.5 + b['reading'] * along
    o = lines[b['line']][0]
    lo = o + b['reading'] * (b['lift'] + across)
    hi = o + b['reading'] * (b['lift'] + b['height'] + across)
    return (c - b['width'] * 0.5, c + b['width'] * 0.5, min(lo, hi), max(lo, hi), b['line'])


def blocked(r, placed, lines, margin):
    for q in placed:
        if r[0] < q[1] + margin and r[1] > q[0] - margin and r[2] < q[3] and r[3] > q[2]:
            return True
    for k, (o, t0, t1) in enumerate(lines):
        if k != r[4] and r[2] < o < r[3] and r[0] < t1 and r[1] > t0:
            return True
    return False


def place_values(boxes, lines):
    """§2: those that fit and those moved by hand first; each that does not fit takes the first free of beyond its
    reading end, before its reading start, above its line, below it; none free, it stays."""
    out = ['fits'] * len(boxes)
    placed = []
    for i, b in enumerate(boxes):
        if b['fixed'] is not None:
            out[i] = 'fixed'
            placed.append(rect(b, lines, *b['fixed']))
        elif fits(b):
            placed.append(rect(b, lines, 0.0, 0.0))
    for i, b in enumerate(boxes):
        if b['fixed'] is not None or fits(b):
            continue
        h, w, r = b['height'], b['width'], b['reading']
        margin = h * 0.5
        c = (b['t0'] + b['t1']) * 0.5
        end, start = (b['t1'], b['t0']) if r >= 0.0 else (b['t0'], b['t1'])
        beyond = r * (end + r * (margin + w * 0.5) - c)
        before = r * (start - r * (margin + w * 0.5) - c)
        up = h * (1.0 + b['gap'])
        down = min(b['lift'], 0.0) - b['lift'] - h - b['gap'] * h
        for cand in ((beyond, 0.0), (before, 0.0), (0.0, up), (0.0, down)):
            if not blocked(rect(b, lines, *cand), placed, lines, margin):
                out[i] = ('moved', cand[0], cand[1])
                placed.append(rect(b, lines, *cand))
                break
        else:
            out[i] = 'tight'
            placed.append(rect(b, lines, 0.0, 0.0))
    return out


def same_scale(a, b):
    return abs(a - b) <= 1e-9 * max(abs(a), abs(b))


def shift_of(p):
    return {'along': p[1], 'across': p[2]} if isinstance(p, tuple) else None


def norm(s):
    return (0.0, 0.0) if s is None else (s['along'], s['across'])


def row_frame(pl):
    """The rows' frame (§1): its reference line the bodies' outermost face on the rows' side."""
    n = (-pl.d[1], pl.d[0])
    face = pl.body[1] if pl.side > 0.0 else pl.body[0]
    return (pl.origin[0] + n[0] * face, pl.origin[1] + n[1] * face), pl.d, n


def dress(rec, members, frame, plot):
    """§1, §2: `members` (row, dimension, the recipe's last place for it) dressed in place: with sheet scales their
    representations; shifting, their values placed (one moved by hand kept); returns each one's account and the place
    to record."""
    origin, d, n = frame

    def t(q):
        return (q[0] - origin[0]) * d[0] + (q[1] - origin[1]) * d[1]

    def o(q):
        return (q[0] - origin[0]) * n[0] + (q[1] - origin[1]) * n[1]
    shifting = rec.get('tight') == 'shift'
    moved = [None if norm(e.get('textShift')) == norm(r) else norm(e.get('textShift')) for _, e, r in members]
    widths = [em(value(e)[1]) for _, e, _ in members]
    rank = {r: i for i, r in enumerate(rec['rows'])}
    order = sorted(range(len(members)),
                   key=lambda i: (rank[members[i][0]],
                                  (t(xy(members[i][1]['a'])) + t(xy(members[i][1]['b']))) * 0.5))

    def at(scale, fixed_too):
        offsets = []
        for _, e, _ in members:
            delta = o(xy(e['a'])) + e['offset']
            offsets.append(e['offset'] + (scale / plot - 1.0) * delta)
        rows, lines, line_of = [], [], [0] * len(members)
        for i in order:
            e = members[i][1]
            ta, tb = t(xy(e['a'])), t(xy(e['b']))
            t0, t1 = (ta, tb) if ta <= tb else (tb, ta)
            row = members[i][0]
            if row in rows:
                k = rows.index(row)
                line_of[i] = k
                lines[k][1], lines[k][2] = min(lines[k][1], t0), max(lines[k][2], t1)
            else:
                line_of[i] = len(rows)
                rows.append(row)
                lines.append([o(xy(e['a'])) + offsets[i], t0, t1])
        boxes = []
        for i in order:
            e = members[i][1]
            h = e['height'] * scale / plot
            look = e.get('look', {})
            gap = look.get('textGap', TEXT_GAP)
            lift = -(TEXT_GAP * h) if look.get('textPlace') == 'centre' else gap * h
            ta, tb = t(xy(e['a'])), t(xy(e['b']))
            th = (e['angle'] * math.pi) / 180.0
            u = (math.cos(th), math.sin(th))
            sign = -1.0 if (e['b']['x'] - e['a']['x']) * u[0] + (e['b']['y'] - e['a']['y']) * u[1] < 0.0 else 1.0
            line = (u[0] * sign, u[1] * sign)
            rotation = math.atan2(line[1], line[0]) * 180.0 / math.pi
            flip = -1.0 if rotation > 90.0 or rotation <= -90.0 else 1.0
            reads = line[0] * flip * d[0] + line[1] * flip * d[1]
            boxes.append({'line': line_of[i], 't0': min(ta, tb), 't1': max(ta, tb), 'width': widths[i] * h, 'height': h,
                          'lift': lift, 'gap': gap, 'reading': -1.0 if reads < 0.0 else 1.0,
                          'fixed': moved[i] if fixed_too else None})
        placed = place_values(boxes, [tuple(x) for x in lines]) if shifting else \
            ['fits' if fits(b) else 'tight' for b in boxes]
        placements, tight = ['fits'] * len(members), [False] * len(members)
        for k, i in enumerate(order):
            placements[i] = placed[k]
            tight[i] = not fits(boxes[k])
        return offsets, tight, placements
    _, tight, mine = at(plot, shifting)
    others = [(s, at(s, False)) for s in rec.get('scales', []) if not same_scale(s, plot)]
    notes, records = [], []
    for i, (_, e, r) in enumerate(members):
        p = mine[i]
        note = {'tight': tight[i], 'placed': isinstance(p, tuple), 'scales': []}
        if p == 'fixed':
            records.append(r)
        elif shifting:
            s = shift_of(p)
            if s is None:
                e.pop('textShift', None)
            else:
                e['textShift'] = s
            records.append(s)
        else:
            if moved[i] is None and r is not None:
                e.pop('textShift', None)
            records.append(r if moved[i] is not None else None)
        entries = []
        for s, (offsets, tight_s, placements) in others:
            q = placements[i]
            note['scales'].append((s, tight_s[i], isinstance(q, tuple)))
            entry = {'scale': s, 'offset': offsets[i]}
            if shifting:
                entry['textShift'] = shift_of(q) or {'along': 0.0, 'across': 0.0}
            entries.append(entry)
        if rec.get('scales'):
            e['annotative'] = {'at': plot, **({'scales': entries} if entries else {})}
        notes.append(note)
    return notes, records


# ── §3: the rounding ────────────────────────────────────────────────────

def decimal(text):
    negative = text.startswith('-')
    body = text[1:] if negative else text
    whole, _, frac = body.partition('.')
    if not whole.isdigit() or (frac and not frac.isdigit()) or (body.endswith('.')):
        return None
    v = int(whole + frac)
    return (-v if negative else v), len(frac)


def rounding_gap(parts, overall):
    total = decimal(overall)
    if total is None or not parts:
        return None
    s = 0
    for part in parts:
        x = decimal(part)
        if x is None or x[1] != total[1]:
            return None
        s += x[0]
    if s == total[0]:
        return None
    digits = str(abs(s)).rjust(total[1] + 1, '0')
    text = digits if total[1] == 0 else digits[:-total[1]] + '.' + digits[-total[1]:]
    return ('-' if s < 0 else '') + text


def rounding(rec, members):
    """§3: the rows running from the overall's first place to its last whose values add up to other than it says."""
    overall = next(((seg, e) for seg, e in members if seg['row'] == 'overall'), None)
    if overall is None:
        return []
    total = value(overall[1])[1]
    out = []
    for row in rec['rows']:
        if row == 'overall':
            continue
        chain = [(seg, e) for seg, e in members if seg['row'] == row]
        if not chain or chain[0][0]['a'] != overall[0]['a'] or chain[-1][0]['b'] != overall[0]['b'] or \
                any(chain[k][0]['b'] != chain[k + 1][0]['a'] for k in range(len(chain) - 1)):
            continue
        gap = rounding_gap([value(e)[1] for _, e in chain], total)
        if gap is not None:
            out.append({'row': row, 'parts': gap, 'overall': total})
    return out


ROW_NAMES = {'openings': 'Açıklıklar', 'walls': 'Duvarlar', 'grids': 'Akslar', 'overall': 'Toplam'}


def tight_warning(tight, placed):
    if not placed:
        return {'code': 'dimension_text_tight',
                'message': f'{tight} ölçünün değeri iki uzatma çizgisinin arasına sığmıyor; gizlenmedi. Sıra aralığını '
                           'ya da ölçü stilinin yazı yerini değiştirin.'}
    return {'code': 'dimension_text_tight',
            'message': f'{tight} ölçünün değeri iki uzatma çizgisinin arasına sığmıyor: {placed} tanesi yana, üste ya '
                       f'da alta alındı, {tight - placed} tanesi yerinde kaldı.'}


def rounding_warning(r):
    return {'code': 'dimension_set_rounding',
            'message': f'{ROW_NAMES[r["row"]]} sırasının yuvarlanmış değerlerinin toplamı {r["parts"]}; toplam ölçü '
                       f'{r["overall"]} yazıyor. Her değer kendi ölçüsünden yuvarlanır, fark dağıtılmaz.'}


def accounts(rec, notes, base=0):
    """The plan's fields of §1–§3 for one recipe's members, their places counted from `base`."""
    out = {'tight': [base + i for i, x in enumerate(notes) if x['tight']],
           'placed': [base + i for i, x in enumerate(notes) if x['placed']], 'scales': []}
    for k, s in enumerate(x[0] for x in (notes[0]['scales'] if notes else [])):
        out['scales'].append({'scale': s, 'tight': [base + i for i, x in enumerate(notes) if x['scales'][k][1]],
                              'placed': [base + i for i, x in enumerate(notes) if x['scales'][k][2]]})
    return out


def merge_accounts(out, more):
    out['tight'] += more['tight']
    out['placed'] += more['placed']
    for note in more['scales']:
        mine = next((n for n in out['scales'] if n['scale'] == note['scale']), None)
        if mine is None:
            out['scales'].append(cp(note))
        else:
            mine['tight'] += note['tight']
            mine['placed'] += note['placed']


def plan_fields(out, acc, rounds):
    if acc['tight']:
        out['tight'] = acc['tight']
    if acc['placed']:
        out['placed'] = acc['placed']
    scales = []
    for note in acc['scales']:
        x = {'scale': note['scale']}
        if note['tight']:
            x['tight'] = note['tight']
        if note['placed']:
            x['placed'] = note['placed']
        scales.append(x)
    if scales:
        out['scales'] = scales
    if rounds:
        out['rounding'] = rounds


def plan_warnings(doc, layer_id, acc, skipped, rounds):
    warnings = layer_warnings(doc, layer_id)
    if acc['tight']:
        warnings.append(tight_warning(len(acc['tight']), len(acc['placed'])))
    if skipped:
        warnings.append(skipped_warning(len(skipped)))
    warnings.extend(rounding_warning(r) for r in rounds)
    return warnings


# ── A drawing written by the commands, dressed ──────────────────────────

class Layout(Sim):
    """ADR 0729's simulator with §1–§2: every recipe written is dressed after its members are planned."""

    def write(self, rec):
        recorded = {m['uid']: m.get('shift') for m in rec.get('members', [])}
        pl, removed, changed, created = super().write(rec)
        plot = self.doc['settings']['plotScale']
        members = [(m['row'], self.entity(m['uid']), recorded.get(m['uid'])) for m in rec['members']]
        before = [cp(e) for _, e, _ in members]
        notes, records = dress(rec, members, row_frame(pl), plot)
        for m, r in zip(rec['members'], records):
            if r is None:
                m.pop('shift', None)
            else:
                m['shift'] = r
        for (u, (_, e, _)), b in zip(((m['uid'], x) for m, x in zip(rec['members'], members)), before):
            if e != b and u not in changed and u not in created and not u.startswith('$uidOf:'):
                changed.append(u)
        self.last_notes = notes
        return pl, removed, changed, created

    def fields(self, rec):
        out = {}
        for m in rec['members']:
            e = self.entity(m['uid'])
            out[str(e['id'])] = {k: cp(e[k]) for k in ('a', 'b', 'offset', 'angle', 'refs', 'layerId')}
            out[str(e['id'])]['textShift'] = cp(e.get('textShift'))
            out[str(e['id'])]['annotative'] = cp(e.get('annotative'))
        return out


# ── §1–§3 in a create's and an edit's plan ──────────────────────────────

def scales_problem(scales):
    """The contract's rule for a recipe's sheet scales (§1)."""
    if len(scales) > 32:
        return 'Ölçü dizisinin pafta ölçeği çok fazla.'
    if any(not math.isfinite(s) or s < 1.0 or s > 1.0e6 for s in scales):
        return "Ölçü dizisinin pafta ölçeği 1 ile 1000000 arasında bir sayı olmalı (1:n'nin n'si)."
    if any(b <= a or same_scale(a, b) for a, b in zip(scales, scales[1:])):
        return 'Ölçü dizisinin pafta ölçekleri artan sırada ve birbirinden farklı olmalı.'
    return None


def failed(text, path, code='invalid_dimension_set'):
    return {'status': 'failed', 'error': {'code': code, 'message': text, 'path': path}}


def with_layout(rec, inp):
    if inp.get('scales'):
        rec['scales'] = list(inp['scales'])
    if inp.get('tight', 'keep') != 'keep':
        rec['tight'] = inp['tight']
    return rec


def create_plan(doc, inp):
    """A create's plan with §1–§3 (the rows as ADR 0729's reference plans them), or its refusal."""
    problem = scales_problem(inp.get('scales', []))
    if problem:
        return failed(problem, 'scales')
    dr = Drawing(doc)
    try:
        recs, path = recipe(dr, inp)
        recs = [with_layout(r, inp) for r in recs]
        plans = [plan(dr, r) for r in recs]
    except Refused:
        raise AssertionError('the layout cases plan what can be planned')
    plot = doc['settings']['plotScale']
    out = {'entities': [], 'spans': [], 'removed': [], 'uids': [], 'revision': '$current'}
    rows, skipped, rounds = [], [], []
    acc = {'tight': [], 'placed': [], 'scales': []}
    from dimension_spans_cases import dimension
    for r, pl in zip(recs, plans):
        dims = [dimension(dr, r, s) for s in pl.spans]
        members = [(s['segment']['row'], e, None) for s, e in zip(pl.spans, dims)]
        notes, _ = dress(r, members, row_frame(pl), plot)
        merge_accounts(acc, accounts(r, notes, base=len(out['entities'])))
        rounds += rounding(r, [(s['segment'], e) for s, e in zip(pl.spans, dims)])
        out['entities'].extend(dims)
        out['spans'].extend(cp(s['segment']) for s in pl.spans)
        rows.extend(pl.notes)
        skipped.extend(u for u in pl.skipped if u not in skipped)
    if rows:
        out['rows'] = rows
    if skipped:
        out['skipped'] = skipped
    plan_fields(out, acc, rounds)
    return completed(out, plan_warnings(doc, inp['layerId'], acc, skipped, rounds))


class Layout2(Layout):
    """The simulator's create with the input's sheet scales and tightness."""

    def create(self, inp):
        dr = self.drawing()
        recs, _ = recipe(dr, inp)
        ids = []
        for rec in recs:
            with_layout(rec, inp)
            rec['id'] = f'$set:{len(self.sets)}'
            rec['members'] = []
            self.write(rec)
            self.sets.append(rec)
            ids.append(rec['id'])
        return ids


def created_doc(doc, inp, set_id=SET1, base=0x100):
    """A drawing with a recipe written by `inp`: an edit's setup."""
    sim = Layout2(doc)
    sim.create(inp)
    sim.sets[-1]['id'] = set_id
    return fixed_members(sim, base)


def written_create(doc, inp):
    result = create_plan(doc, inp)
    sim = Layout2(doc)
    sim.create(inp)
    created = sum(len(r['members']) for r in sim.sets) - sum(len(r.get('members', [])) for r in doc.get('dimensionSets', []))
    output = {'set': '$any', 'uids': ['$any'] * created, 'created': ['$any'] * created, 'removed': [], 'changed': [],
              'revision': '$current'}
    if inp.get('around'):
        output['sets'] = ['$any'] * 4
    return completed(output, result['warnings']), sim


def rounding_doc():
    """§3: a wall whose parts round to more than its overall, and the input of a façade recipe on it.
    Binary-exact lengths: three parts of 1 + 2⁻¹² read 1.000 each, the whole 3 + 3·2⁻¹² reads 3.001."""
    lo, hi, length = 1.000244140625, 2.00048828125, 3.000732421875
    rd = document([wall(1, (0, 0), (length, 0), width=0.2),
                   opening(2, uid(80), 1.5003662109375, 1.000244140625, [(lo, 0.1), (lo, -0.1), (hi, 0.1), (hi, -0.1)])],
                  [uid(80), uid(81)], name='Yuvarlama')
    return rd, {'layerId': 'olcu', 'walls': [uid(80)], 'at': pt(1.5, -1.5), 'direction': pt(1, 0)}


def create_cases():
    out = []
    h = house()
    plot = h['settings']['plotScale']
    rows4 = ['openings', 'walls', 'grids', 'overall']
    south = {'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5), 'rows': rows4}

    def planned(name, doc, inp):
        out.append(case(name, doc, step('plan', inp, create_plan(doc, inp), unchanged(doc))))

    def written(name, doc, inp):
        result, sim = written_create(doc, inp)
        before = {'ids': sorted(e['id'] for e in doc['entities']), 'dimensionSets': doc.get('dimensionSets', [])}
        out.append(case(name, doc, step('execute', inp, result, sim.state(canUndo=True, dirty=True, revision='changed')),
                        {'op': 'undo', 'expect': before}))

    planned('Pafta ölçeği: 1:100 paftada her sıra dış yüzden iki kat uzakta', h, {**south, 'scales': [100.0]})
    planned('Pafta ölçekleri: 1:100 ve 1:200', h, {**south, 'scales': [100.0, 200.0]})
    planned('Pafta ölçekleri: çizim ölçeği listede, onun temsili üyenin kendisi', h, {**south, 'scales': [50.0, 100.0]})
    planned('Pafta ölçeği: yalnız çizim ölçeği, başka paftada çizilmez', h, {**south, 'scales': [50.0]})
    planned('Pafta ölçeği: 1:20 ayrıntı paftası, sıralar daha yakın', h, {**south, 'scales': [20.0]})
    written('Pafta ölçeği: yaz ve geri al', h, {**south, 'scales': [100.0]})
    planned('Sığmayan yazı: ince duvarların değerleri yana, üste ya da alta', h, {**south, 'tight': 'shift'})
    written('Sığmayan yazı: yaz, yerler üyelere kaydedilir, geri al', h, {**south, 'tight': 'shift'})
    planned('Sığmayan yazı: pafta ölçeklerinde ayrı ayrı yerleşir', h, {**south, 'tight': 'shift', 'scales': [100.0, 200.0]})
    planned('Sığmayan yazı: sıra aralığı 4 mm, yer kalmayanlar yerinde', h, {**south, 'tight': 'shift', 'gapMm': 4.0})
    planned('Sığmayan yazı: yalnız duvarlar sırası', h, {**south, 'tight': 'shift', 'rows': ['walls']})
    planned('Sığmayan yazı: duvarlar ve akslar sırası birlikte', h, {**south, 'tight': 'shift', 'rows': ['walls', 'grids']})
    planned('Kuzey cephesi: ters okuma yönünde sığmayan yazı', h,
            {'layerId': 'olcu', 'walls': [N], 'at': pt(5, 9.5), 'rows': rows4, 'tight': 'shift', 'direction': pt(1, 0)})
    planned('Mahal: pafta ölçeği ve sığmayan yazı', h,
            {'layerId': 'olcu', 'space': uid(12), 'at': pt(2, -1.5), 'direction': pt(1, 0),
             'rows': ['openings', 'walls', 'overall'], 'tight': 'shift', 'scales': [100.0]})
    planned('Dört yan: ölçekler her dizide', h, {'layerId': 'olcu', 'walls': [S, E, N, W], 'at': pt(5, -1.5),
                                                  'direction': pt(1, 0), 'around': True, 'scales': [100.0]})
    rd, rinp = rounding_doc()
    planned('Yuvarlama: parçalar 3.000, toplam 3.001 yazar ve söylenir', rd, rinp)
    written('Yuvarlama: yazarken de söylenir', rd, rinp)
    planned('Yuvarlama: duvarlar sırası da toplamdan farklı', rd, {**rinp, 'rows': ['openings', 'walls', 'overall']})
    for name, scales in (('Ret: pafta ölçekleri azalan sırada', [200.0, 100.0]),
                         ('Ret: pafta ölçeği 1\'den küçük', [0.5]),
                         ('Ret: iki ölçek aynı', [100.0, 100.00000000001]),
                         ('Ret: otuz üç pafta ölçeği', [float(100 + k) for k in range(33)])):
        out.append(case(name, h, step('plan', {**south, 'scales': scales},
                                      failed(scales_problem(scales), 'scales'), unchanged(h))))
    return out


def edit_plan(doc, set_id, operation):
    """An edit's plan and write with §1–§3: the recipe changed, its members planned and dressed."""
    sim = Layout2(doc)
    rec = next(r for r in sim.sets if r['id'] == set_id)
    before = cp(rec)
    kind = operation['kind']
    if kind == 'set':
        problem = scales_problem(operation.get('scales', []))
        if problem:
            return failed(problem, 'operation.scales'), None, None
        for key in ('rows', 'openings', 'gapMm'):
            if key in operation:
                rec[key] = cp(operation[key])
        if 'scales' in operation:
            if operation['scales']:
                rec['scales'] = list(operation['scales'])
            else:
                rec.pop('scales', None)
        if 'tight' in operation:
            if operation['tight'] == 'keep':
                rec.pop('tight', None)
            else:
                rec['tight'] = operation['tight']
    elif kind == 'move':
        rec['at'] = cp(operation['at'])
    elif kind == 'resetText':
        row = operation.get('row')
        for m in rec['members']:
            if row is None or m['row'] == row:
                ts = sim.entity(m['uid']).get('textShift')
                if ts is None:
                    m.pop('shift', None)
                else:
                    m['shift'] = cp(ts)
    entities_before = {m['uid']: cp(sim.entity(m['uid'])) for m in before['members']}
    if kind != 'resetText' and before.get('scales') and not rec.get('scales'):
        for m in rec['members']:
            sim.entity(m['uid']).pop('annotative', None)
    pl, removed, changed, created = sim.write(rec)
    notes = sim.last_notes
    plot = doc['settings']['plotScale']
    entities, uids = [], []
    for m in rec['members']:
        e = cp(sim.entity(m['uid']))
        if m['uid'].startswith('$uidOf:'):
            e['id'] = 0
        else:
            uids.append(m['uid'])
        entities.append(e)
    if kind == 'resetText':
        row = operation.get('row')
        out = {'set': set_id, 'entities': [], 'spans': [], 'removed': [], 'uids': [], 'revision': '$current'}
        for m in before['members']:
            if row is not None and m['row'] != row:
                continue
            now = sim.entity(m['uid'])
            if entities_before[m['uid']].get('textShift') != now.get('textShift'):
                out['entities'].append(cp(now))
                out['uids'].append(m['uid'])
                out['spans'].append({k: cp(m[k]) for k in ('row', 'a', 'b')})
        planned_out = completed(out, layer_warnings(doc, rec['layerId']))
        written_out = completed({'set': set_id, 'uids': [m['uid'] for m in rec['members']], 'created': [],
                                 'removed': [], 'changed': sorted(out['uids']), 'revision': '$current'},
                                layer_warnings(doc, rec['layerId']))
        return planned_out, written_out, sim
    acc = accounts(rec, notes)
    rounds = rounding(rec, [({k: m[k] for k in ('row', 'a', 'b')}, e) for m, e in zip(rec['members'], entities)])
    output = {'set': set_id, 'entities': entities, 'spans': [cp(sp['segment']) for sp in pl.spans],
              'removed': [m['uid'] for m in before['members'] if m['uid'] not in uids], 'uids': uids,
              'revision': '$current', 'rows': pl.notes}
    if pl.skipped:
        output['skipped'] = pl.skipped
    plan_fields(output, acc, rounds)
    warnings = plan_warnings(doc, rec['layerId'], acc, pl.skipped, rounds)
    after = {m['uid'] for m in rec['members']}
    # A member the step changed: planned anew, or its sheet representations gone with the recipe's scales.
    changed = set(changed) | {u for u in entities_before if u in after and sim.entity(u) != entities_before[u]}
    written_out = completed({'set': set_id, 'uids': [m['uid'] if not m['uid'].startswith('$') else '$any'
                                                     for m in rec['members']],
                             'created': ['$any'] * len(created),
                             'removed': sorted(u for u in entities_before if u not in after),
                             'changed': sorted(u for u in changed if not u.startswith('$')),
                             'revision': '$current'}, warnings)
    return completed(output, warnings), written_out, sim


def edit_cases():
    out = []
    h = house()
    rows4 = ['openings', 'walls', 'grids', 'overall']
    south = {'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5), 'rows': rows4}
    plain = created_doc(h, south)
    scaled = created_doc(h, {**south, 'scales': [100.0]})
    shifted = created_doc(h, {**south, 'tight': 'shift'})
    # A value of the shifted recipe moved by hand: its place no longer the recipe's.
    by_hand = cp(shifted)
    rec = by_hand['dimensionSets'][0]
    moved_member = next(m for m in rec['members'] if 'shift' in m)
    i = by_hand['uids'].index(moved_member['uid'])
    by_hand['entities'][i]['textShift'] = {'along': 0.0, 'across': 0.5}
    room = created_doc(h, {'layerId': 'olcu', 'space': uid(12), 'at': pt(2, -1.5), 'direction': pt(1, 0),
                           'rows': ['openings', 'walls', 'overall']}, set_id=SET1)

    def planned_edit(name, doc, operation):
        result, _, _ = edit_plan(doc, SET1, operation)
        out.append(case(name, doc, step('plan', {'id': SET1, 'operation': operation}, result, unchanged(doc))))

    def written_edit(name, doc, operation):
        _, result, sim = edit_plan(doc, SET1, operation)
        before = {'ids': sorted(e['id'] for e in doc['entities']), 'dimensionSets': doc['dimensionSets']}
        out.append(case(name, doc,
                        step('execute', {'id': SET1, 'operation': operation}, result,
                             sim.state(canUndo=True, dirty=True, revision='changed')),
                        {'op': 'undo', 'expect': before}))

    planned_edit('Pafta ölçeği ekle: her üye 1:100 temsilini alır', plain, {'kind': 'set', 'scales': [100.0]})
    written_edit('Pafta ölçeği ekle: yaz ve geri al', plain, {'kind': 'set', 'scales': [100.0]})
    planned_edit('Pafta ölçekleri değişir: 1:100 ve 1:200', scaled, {'kind': 'set', 'scales': [100.0, 200.0]})
    planned_edit('Pafta ölçekleri kalkar: tarifin yazdığı temsiller de', scaled, {'kind': 'set', 'scales': []})
    written_edit('Pafta ölçekleri kalkar: yaz ve geri al', scaled, {'kind': 'set', 'scales': []})
    planned_edit('Ölçekli dizinin sıra aralığı: her ölçekte yeniden', scaled, {'kind': 'set', 'gapMm': 10.0})
    planned_edit('Ölçekli diziyi taşı: ölçeklerdeki uzaklıklar da', scaled, {'kind': 'move', 'at': pt(5, -3)})
    planned_edit('Sığmayan yazı: kaydırmaya geç', plain, {'kind': 'set', 'tight': 'shift'})
    written_edit('Sığmayan yazı: kaydır, yaz ve geri al', plain, {'kind': 'set', 'tight': 'shift'})
    planned_edit('Sığmayan yazı: yerinde bırakmaya dön, tarifin yerleri kalkar', shifted, {'kind': 'set', 'tight': 'keep'})
    written_edit('Sığmayan yazı: yerinde bırak, yaz ve geri al', shifted, {'kind': 'set', 'tight': 'keep'})
    planned_edit('Kaydıran dizinin sıra aralığı: yerler yeniden hesaplanır', shifted, {'kind': 'set', 'gapMm': 5.0})
    planned_edit('Elle taşınan yazı, sıralar değişince yerinde kalır', by_hand, {'kind': 'set', 'rows': ['walls', 'overall']})
    planned_edit('Yazıları tarife döndür: elle taşınan yazı tarifin yerine', by_hand, {'kind': 'resetText'})
    written_edit('Yazıları tarife döndür: yaz ve geri al', by_hand, {'kind': 'resetText'})
    planned_edit('Yazıları tarife döndür: yalnız bir sıranın', by_hand, {'kind': 'resetText', 'row': 'overall'})
    planned_edit('Kaydıran dizi pafta ölçeği alır: ölçekte yerler ayrı', shifted, {'kind': 'set', 'scales': [100.0]})
    planned_edit('Mahal: pafta ölçeği', room, {'kind': 'set', 'scales': [100.0, 200.0]})
    rd, rinp = rounding_doc()
    rounded = created_doc(rd, rinp)
    planned_edit('Yuvarlama: sıra aralığı değişince de söylenir', rounded, {'kind': 'set', 'gapMm': 9.0})
    written_edit('Yuvarlama: duvarlar sırası eklenir, yazılır ve söylenir', rounded,
                 {'kind': 'set', 'rows': ['openings', 'walls', 'overall']})
    written_edit('Mahal: sığmayan yazı, yaz ve geri al', room, {'kind': 'set', 'tight': 'shift'})
    for name, scales in (('Ret: düzenlemede azalan ölçekler', [200.0, 100.0]),
                         ('Ret: düzenlemede sıfır ölçek', [0.0])):
        out.append(case(name, plain, step('plan', {'id': SET1, 'operation': {'kind': 'set', 'scales': scales}},
                                          failed(scales_problem(scales), 'operation.scales'), unchanged(plain))))
    return out


# ── §4: copies and moves with the objects ───────────────────────────────

def m_move(dx, dy):
    return (1.0, 0.0, 0.0, 1.0, dx, dy)


def m_rotate(c, angle):
    co, si = math.cos(angle), math.sin(angle)
    return (co, si, -si, co, c[0] - co * c[0] + si * c[1], c[1] - si * c[0] - co * c[1])


def m_mirror(p, q):
    dx, dy = q[0] - p[0], q[1] - p[1]
    l2 = dx * dx + dy * dy
    a, b = (dx * dx - dy * dy) / l2, (2.0 * dx * dy) / l2
    return (a, b, b, -a, p[0] - a * p[0] - b * p[1], p[1] - b * p[0] + a * p[1])


def apply(m, q):
    return (m[0] * q[0] + m[2] * q[1] + m[4], m[1] * q[0] + m[3] * q[1] + m[5])


def apply_linear(m, v):
    return (m[0] * v[0] + m[2] * v[1], m[1] * v[0] + m[3] * v[1])


def moved_point(m, q):
    return pt(*apply(m, xy(q)))


def readable(d):
    if d[0] < -1e-9 or (abs(d[0]) <= 1e-9 and d[1] < 0.0):
        return (-d[0] + 0.0, -d[1] + 0.0)
    return d


def moved_recipe(rec, m):
    """§4: a recipe's row places through `m`, its direction turned and made to read."""
    out = cp(rec)
    out['at'] = moved_point(m, rec['at'])
    for place in out.get('rowPlaces', []):
        place['at'] = moved_point(m, place['at'])
    if 'span' in out:
        d = rec['span']['direction']
        v = apply_linear(m, (d['x'], d['y']))
        length = math.hypot(v[0], v[1])
        v = readable((v[0] / length, v[1] / length))
        out['span']['direction'] = {'x': v[0], 'y': v[1]}
    return out


def jambs(host, at, width):
    """An opening's four jamb corners on its straight two-point wall, centred at `at` (ADR 0341)."""
    p0, p1 = xy(host['pts'][0]), xy(host['pts'][1])
    length = math.hypot(p1[0] - p0[0], p1[1] - p0[1])
    u = ((p1[0] - p0[0]) / length, (p1[1] - p0[1]) / length)
    left = (-u[1], u[0])
    half = host['width'] * 0.5
    lo, hi = at - width * 0.5, at + width * 0.5
    out = []
    for t in (lo, hi):
        base = (p0[0] + u[0] * t, p0[1] + u[1] * t)
        out.append(pt(base[0] + left[0] * half, base[1] + left[1] * half))
        out.append(pt(base[0] - left[0] * half, base[1] - left[1] * half))
    return out


def reflects(m):
    return m[0] * m[3] - m[1] * m[2] < 0.0


def moved_entity(e, m, copy):
    e = cp(e)
    if e['kind'] in ('wall', 'grid'):
        e['pts'] = [moved_point(m, q) for q in e['pts']]
        if e['kind'] == 'wall' and reflects(m):
            # A mirrored wall's path runs back, so its body stays on the side of its path it was on (ADR 0339 §1).
            n = len(e['pts'])
            e['pts'] = e['pts'][::-1]
            if e.get('bulges'):
                b = e['bulges']
                at = (lambda i: b[i] if i < len(b) else 0.0)
                e['bulges'] = [-at((2 * n - 2 - i) % n) for i in range(n)] if e.get('closed') else \
                    [-at(n - 2 - i) for i in range(n - 1)] + [0.0]
    elif e['kind'] == 'dimension':
        e['a'], e['b'] = moved_point(m, e['a']), moved_point(m, e['b'])
        if copy:
            e.pop('refs', None)
    elif e['kind'] == 'space':
        for part in e['parts']:
            for ring in [part['outer']] + part.get('holes', []):
                ring['pts'] = [moved_point(m, q) for q in ring['pts']]
    return e


def place_openings(sim, uids, mirrored=False):
    """Each opening's jambs from its place along its wall; a mirrored one read back from its moved jambs (ADR 0341): its
    wall's path runs back, so its middle is as far from the other end."""
    for u in uids:
        e = sim.entity(u)
        if e['kind'] == 'opening':
            host = sim.entity(e['host'])
            if mirrored:
                p0, p1 = xy(host['pts'][0]), xy(host['pts'][1])
                e['at'] = math.hypot(p1[0] - p0[0], p1[1] - p0[1]) - e['at']
            e['place'] = jambs(host, e['at'], e['width'])


def whole(sim, uids):
    """§4: the recipes the objects take whole; a part of one refused, as the command says."""
    chosen = set(uids)
    out = []
    for rec in sim.sets:
        first = next((m for m in rec['members'] if m['uid'] in chosen), None)
        if first is None:
            continue
        members = sum(1 for m in rec['members'] if m['uid'] not in chosen)
        sources = sum(1 for s in [rec['wall']] + rec.get('span', {}).get('walls', []) if s not in chosen)
        if members + sources:
            at = uids.index(first['uid'])
            return None, failed(
                f'Seçimde otomatik ölçü grubunun yalnız bir kısmı var: {sources} kaynağı ve {members} ölçüsü '
                'seçilmemiş. Grubu kopyalamak ya da taşımak için bütün kaynaklarını ve ölçülerini seçin, ya da önce '
                'Özellikler panelinden “Diziyi bağımsız ölçülere dönüştür”ü kullanın.', f'uids[{at}]',
                code='dimension_set_owned')
        out.append(rec)
    return out, None


OWNED = ('Seçimde otomatik ölçü grubuna ait ölçü var. Grubun kaynak ilişkileri bu kopyalama işleminde taşınamıyor. '
         'Önce Özellikler panelinden “Diziyi bağımsız ölçülere dönüştür” işlemini kullanın veya otomatik ölçüleri '
         'seçimden çıkarın.')


def copies(sim, uids, items, layer=None):
    """§4: the objects copied by each map of `items` (each map's copies in the objects' order), an opening onto its
    wall's copy, then each whole recipe again over each item's copies, planned as an edit plans."""
    recs, refusal = whole(sim, uids)
    if refusal:
        return refusal
    made = []
    for m in items:
        mapping = {}
        for u in uids:
            e = moved_entity(sim.entity(u), m, True)
            if layer:
                e['layerId'] = layer
            mapping[u] = sim.add(e)
        for u in uids:
            e = sim.entity(mapping[u])
            if e['kind'] == 'opening' and e['host'] in mapping:
                e['host'] = mapping[e['host']]
        place_openings(sim, mapping.values(), reflects(m))
        made.append((m, mapping))
    for m, mapping in made:
        for rec in recs:
            text = json.dumps(moved_recipe(rec, m))
            for old, new in mapping.items():
                text = text.replace(f'"{old}"', f'"{new}"')
            copy = json.loads(text)
            copy['id'] = f'$set:{len(sim.sets)}'
            copy.pop('broken', None)
            if layer:
                copy['layerId'] = layer
            sim.write(copy)
            sim.sets.append(copy)
    return None


def moved_in_place(sim, uids, m):
    """§4: the objects moved by `m` and the recipes they take whole with them, in one change."""
    recs, refusal = whole(sim, uids)
    if refusal:
        return refusal, []
    for u in uids:
        i = sim.index(u)
        sim.doc['entities'][i] = moved_entity(sim.doc['entities'][i], m, False)
    place_openings(sim, uids, reflects(m))
    for rec in recs:
        moved = moved_recipe(rec, m)
        rec.clear()
        rec.update(moved)
        sim.write(rec)
    return None, [u for u in uids]


def everything(doc, extra=()):
    """The house's objects and its recipes' members, in the drawing's order (members last)."""
    members = [m['uid'] for rec in doc.get('dimensionSets', []) for m in rec['members']]
    return [u for u in doc['uids'] if u not in members and u != uid(12)] + list(extra) + members


def transform_cases():
    out = []
    h = house()
    rows4 = ['openings', 'walls', 'grids', 'overall']
    south = {'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5), 'rows': rows4}
    plain = created_doc(h, south)
    scaled = created_doc(h, {**south, 'scales': [100.0, 200.0]})
    shifted = created_doc(h, {**south, 'tight': 'shift'})
    sim = Layout2(h)
    sim.create(south)
    sim.sets[-1]['id'] = SET1
    sim.create({'layerId': 'olcu', 'walls': [N], 'at': pt(5, 9.5), 'direction': pt(1, 0), 'rows': ['openings', 'overall']})
    sim.sets[-1]['id'] = '0192f5a1-7290-7000-8000-00000000f002'
    two = fixed_members(sim, 0x100)

    def written(name, doc, inp, build):
        sim = Layout2(doc)
        refusal = build(sim)
        before = {'ids': sorted(e['id'] for e in doc['entities']), 'dimensionSets': doc['dimensionSets']}
        if refusal is not None:
            out.append(case(name, doc, step('execute', inp, refusal, unchanged(doc))))
            return
        out.append(case(name, doc, step('execute', inp, '$any', sim.state(canUndo=True, dirty=True, revision='changed')),
                        {'op': 'undo', 'expect': before}))

    def copy_case(name, doc, transform, m, uids=None, layer=None):
        uids = uids or everything(doc)
        inp = {'uids': uids, 'copy': True, 'transform': transform}
        if layer:
            inp['layerId'] = layer
        written(name, doc, inp, lambda sim: copies(sim, uids, [m], layer))

    def move_case(name, doc, transform, m, uids=None):
        uids = uids or everything(doc)
        written(name, doc, {'uids': uids, 'transform': transform}, lambda sim: moved_in_place(sim, uids, m)[0])

    copy_case('Kopyala: bütün ev ve ölçüleri 20 m doğuya, dizi kopyalarda yeniden', plain,
              {'kind': 'move', 'dx': 20.0, 'dy': 0.0}, m_move(20.0, 0.0))
    copy_case('Kopyala: kuzeye 30 m, akslar kopyalanmaz ve kesmez', plain,
              {'kind': 'move', 'dx': 0.0, 'dy': 30.0}, m_move(0.0, 30.0),
              uids=[u for u in everything(plain) if u not in (GA, GB, GC)])
    copy_case('Kopyala: aksları da kopyalanır, akslar sırası aynı', plain,
              {'kind': 'move', 'dx': 0.0, 'dy': 30.0}, m_move(0.0, 30.0))
    copy_case('Kopyala: çeyrek tur döndürülmüş kopya, doğrultu döner', plain,
              {'kind': 'rotate', 'center': pt(25, 4), 'angle': math.pi / 2}, m_rotate((25.0, 4.0), math.pi / 2))
    copy_case('Kopyala: yarım tur döndürülmüş kopya, doğrultu okunur yöne', plain,
              {'kind': 'rotate', 'center': pt(15, 4), 'angle': math.pi}, m_rotate((15.0, 4.0), math.pi))
    copy_case('Kopyala: aynalanmış kopya, kapıların pervazları yeniden', plain,
              {'kind': 'mirror', 'a': pt(15, 0), 'b': pt(15, 1)}, m_mirror((15.0, 0.0), (15.0, 1.0)))
    copy_case('Katmana kopyala: dizinin katmanı kopyaların katmanı', plain,
              {'kind': 'move', 'dx': 20.0, 'dy': 0.0}, m_move(20.0, 0.0), layer='olcu')
    copy_case('Kopyala: pafta ölçekli dizi, kopyada da ölçekler', scaled,
              {'kind': 'move', 'dx': 20.0, 'dy': 0.0}, m_move(20.0, 0.0))
    copy_case('Kopyala: yazısı kaydırılan dizi, kopyada aynı yerler', shifted,
              {'kind': 'move', 'dx': 20.0, 'dy': 0.0}, m_move(20.0, 0.0))
    copy_case('Kopyala: iki dizi birlikte', two, {'kind': 'move', 'dx': 20.0, 'dy': 0.0}, m_move(20.0, 0.0))
    move_case('Taşı: bütün ev ve ölçüleri, sıralar birlikte gider', plain,
              {'kind': 'move', 'dx': 0.0, 'dy': 50.0}, m_move(0.0, 50.0))
    move_case('Döndür: çeyrek tur, aynı ölçüler aynı aralıkları ölçer', plain,
              {'kind': 'rotate', 'center': pt(5, 4), 'angle': math.pi / 2}, m_rotate((5.0, 4.0), math.pi / 2))
    move_case('Döndür: yarım tur, kimlikler korunur', plain,
              {'kind': 'rotate', 'center': pt(5, 4), 'angle': math.pi}, m_rotate((5.0, 4.0), math.pi))
    move_case('Aynala: yerinde, sıralar aynı yanda', plain,
              {'kind': 'mirror', 'a': pt(5, 0), 'b': pt(5, 1)}, m_mirror((5.0, 0.0), (5.0, 1.0)))
    move_case('Taşı: ölçekli dizi, ölçeklerdeki temsiller de', scaled,
              {'kind': 'move', 'dx': 3.0, 'dy': 4.0}, m_move(3.0, 4.0))
    move_case('Taşı: yazısı kaydırılan dizi', shifted, {'kind': 'move', 'dx': 3.0, 'dy': 4.0}, m_move(3.0, 4.0))
    move_case('Taşı: iki dizi birlikte', two, {'kind': 'move', 'dx': -7.0, 'dy': 2.0}, m_move(-7.0, 2.0))
    # Refusals: a part of a recipe, a transform that does not keep its form.
    members = [m['uid'] for m in plain['dimensionSets'][0]['members']]
    some = [S, members[0]]
    written('Ret: kaynak ve tek ölçü, grubun kalanı seçilmemiş', plain,
            {'uids': some, 'copy': True, 'transform': {'kind': 'move', 'dx': 1.0, 'dy': 0.0}},
            lambda sim: copies(sim, some, [m_move(1.0, 0.0)]))
    only = list(members)
    written('Ret: bütün ölçüler, kaynak duvar seçilmemiş', plain,
            {'uids': only, 'transform': {'kind': 'move', 'dx': 1.0, 'dy': 0.0}},
            lambda sim: moved_in_place(sim, only, m_move(1.0, 0.0))[0])
    alls = everything(plain)
    first = next(i for i, u in enumerate(alls) if u in members)
    out.append(case('Ret: afin dönüşüm dizinin biçimini korumaz', plain,
                    step('execute', {'uids': alls, 'transform': {'kind': 'affine', 'from': pt(0, 0), 'to': pt(0, 0),
                                                                   'm': [1.0, 0.2, 0.0, 1.0]}},
                         failed(OWNED, f'uids[{first}]', code='dimension_set_owned'), unchanged(plain))))
    return out


def array_cases():
    out = []
    h = house()
    rows4 = ['openings', 'walls', 'grids', 'overall']
    south = {'layerId': 'olcu', 'walls': [S], 'at': pt(5, -1.5), 'rows': rows4}
    plain = created_doc(h, south)
    scaled = created_doc(h, {**south, 'scales': [100.0]})
    scaled2 = created_doc(h, {**south, 'scales': [100.0, 200.0]})
    shifted = created_doc(h, {**south, 'tight': 'shift'})
    sim = Layout2(h)
    sim.create(south)
    sim.sets[-1]['id'] = SET1
    sim.create({'layerId': 'olcu', 'walls': [N], 'at': pt(5, 9.5), 'direction': pt(1, 0), 'rows': ['openings', 'overall']})
    sim.sets[-1]['id'] = '0192f5a1-7290-7000-8000-00000000f002'
    two = fixed_members(sim, 0x100)

    def written(name, doc, inp, items, uids=None):
        uids = uids or everything(doc)
        inp = {'uids': uids, **inp}
        sim = Layout2(doc)
        refusal = copies(sim, uids, items)
        if refusal is not None:
            out.append(case(name, doc, step('execute', inp, refusal, unchanged(doc))))
            return
        before = {'ids': sorted(e['id'] for e in doc['entities']), 'dimensionSets': doc['dimensionSets']}
        out.append(case(name, doc, step('execute', inp, '$any', sim.state(canUndo=True, dirty=True, revision='changed')),
                        {'op': 'undo', 'expect': before}))

    written('Dizi: iki sütun, her öğede dizi', plain, {'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': 20.0,
                                                                  'dy': 0.0}}, [m_move(20.0, 0.0)])
    written('Dizi: üç sütun', plain, {'layout': {'kind': 'grid', 'rows': 1, 'cols': 3, 'dx': 15.0, 'dy': 0.0}},
            [m_move(15.0, 0.0), m_move(30.0, 0.0)])
    written('Dizi: iki satır, iki sütun', plain, {'layout': {'kind': 'grid', 'rows': 2, 'cols': 2, 'dx': 15.0,
                                                               'dy': 15.0}},
            [m_move(15.0, 0.0), m_move(0.0, 15.0), m_move(15.0, 15.0)])
    written('Dizi: pafta ölçekli dizi her öğede', scaled, {'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': 20.0,
                                                                      'dy': 0.0}}, [m_move(20.0, 0.0)])
    written('Dizi: batıya bir sütun', plain, {'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': -20.0, 'dy': 0.0}},
            [m_move(-20.0, 0.0)])
    written('Dizi: kuzeye iki satır', plain, {'layout': {'kind': 'grid', 'rows': 2, 'cols': 1, 'dx': 0.0, 'dy': 15.0}},
            [m_move(0.0, 15.0)])
    written('Dizi: iki satır, üç sütun', plain, {'layout': {'kind': 'grid', 'rows': 2, 'cols': 3, 'dx': 15.0,
                                                              'dy': 20.0}},
            [m_move(15.0, 0.0), m_move(30.0, 0.0), m_move(0.0, 20.0), m_move(15.0, 20.0), m_move(30.0, 20.0)])
    written('Dizi: akslar diziye alınmaz, kopyalarda aks sırası yok', plain,
            {'layout': {'kind': 'grid', 'rows': 2, 'cols': 1, 'dx': 0.0, 'dy': 30.0}}, [m_move(0.0, 30.0)],
            uids=[u for u in everything(plain) if u not in (GA, GB, GC)])
    written('Dizi: yazısı kaydırılan dizi her öğede', shifted,
            {'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': 20.0, 'dy': 0.0}}, [m_move(20.0, 0.0)])
    written('Dizi: iki dizi birlikte her öğede', two,
            {'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': 20.0, 'dy': 0.0}}, [m_move(20.0, 0.0)])
    # Kopyala's Dizi (ADR 0291): the copy k moves k·(dx, dy), or with fit k·(dx, dy) / (count − 1).
    written('Kopyala dizisi: iki kopya doğuya', plain, {'layout': {'kind': 'line', 'count': 3, 'dx': 18.0, 'dy': 0.0}},
            [m_move(18.0, 0.0), m_move(36.0, 0.0)])
    written('Kopyala dizisi: sığdır, son kopya yerin sonunda', plain,
            {'layout': {'kind': 'line', 'count': 3, 'dx': 40.0, 'dy': 0.0, 'fit': True}},
            [m_move((1.0 * 40.0) / 2.0, 0.0), m_move((2.0 * 40.0) / 2.0, 0.0)])
    written('Kopyala dizisi: iki ölçekli dizi kuzeye', scaled2,
            {'layout': {'kind': 'line', 'count': 2, 'dx': 0.0, 'dy': 25.0}}, [m_move(0.0, 25.0)])
    # A polar array that turns its copies (ADR 0047): the copy k turned k · step about the centre.

    def polar(name, doc, center, count, fill):
        step_ = fill / count if abs(abs(fill) - 360.0) < 1e-9 else fill / (count - 1.0)
        items = [m_rotate(center, (step_ * float(k) * math.pi) / 180.0) for k in range(1, count)]
        written(name, doc, {'layout': {'kind': 'polar', 'center': pt(*center), 'count': count, 'fill': float(fill),
                                       'rotate': True}}, items)

    polar('Kutupsal dizi: yarım tur, kopyada doğrultu okunur yöne', plain, (25.0, 4.0), 2, 360.0)
    polar('Kutupsal dizi: dört kopya çeyrek turlarla', plain, (30.0, 4.0), 4, 360.0)
    polar('Kutupsal dizi: çeyrek tura üç öğe, kopyalar 45° arayla', plain, (5.0, 30.0), 3, 90.0)
    polar('Kutupsal dizi: saat yönünde çeyrek tur', plain, (5.0, 30.0), 2, -90.0)
    # Yol boyunca dizi (ADR 0140): along a line, its places 25 m apart; a copy moves by the path's point less its
    # start.
    along = cp(plain)
    path = {'kind': 'line', 'id': max(e['id'] for e in along['entities']) + 1, 'layerId': 'cizim', 'attrs': {},
            'a': pt(0, -20), 'b': pt(60, -20)}
    along['entities'].append(path)
    along['uids'].append(PATH)
    length = 60.0
    places = [(0.0 + 60.0 * (25.0 * k / length), -20.0 + 0.0 * (25.0 * k / length)) for k in (1, 2)]
    written('Yol boyunca dizi: çizgi boyunca 25 m arayla', along,
            {'layout': {'kind': 'path', 'path': PATH, 'count': 3, 'spacing': 25.0, 'align': False}},
            [m_move(x - 0.0, y - -20.0) for x, y in places], uids=everything(plain))
    members = [m['uid'] for m in plain['dimensionSets'][0]['members']]
    written('Ret: dizide grubun bir kısmı', plain, {'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': 20.0,
                                                               'dy': 0.0}}, [m_move(20.0, 0.0)], uids=[S, members[0]])
    alls = everything(plain)
    first = next(i for i, u in enumerate(alls) if u in members)
    written('Ret: kutupsal dizide kaynaksız ölçüler', plain,
            {'layout': {'kind': 'polar', 'center': pt(25, 4), 'count': 2, 'fill': 360.0, 'rotate': True}},
            [m_rotate((25.0, 4.0), math.pi)], uids=list(members))
    out.append(case('Ret: ilişkili dizi ölçü grubunu taşıyamaz', plain,
                    step('execute', {'uids': alls, 'associative': True,
                                     'layout': {'kind': 'grid', 'rows': 1, 'cols': 2, 'dx': 20.0, 'dy': 0.0}},
                         failed(OWNED, f'uids[{first}]', code='dimension_set_owned'), unchanged(plain))))
    return out


def build():
    out = {}
    for name, command, cases, title in [
        ('cad.dimensionsets.create.layout', CREATE, create_cases(), 'Ölçü dizisinin pafta ölçekleri, yazı yeri, yuvarlama: oluştur'),
        ('cad.dimensionsets.edit.layout', EDIT, edit_cases(), 'Ölçü dizisinin pafta ölçekleri, yazı yeri, yuvarlama: düzenle'),
        ('cad.entities.transform.recipes', TRANSFORM, transform_cases(), 'Ölçü dizisi nesneleriyle kopyalanır ve taşınır'),
        ('cad.entities.array.recipes', ARRAY, array_cases(), 'Ölçü dizisi dizinin her öğesinde'),
    ]:
        texts = [json.dumps(c['setup'], sort_keys=True) for c in cases]
        common = json.loads(max(set(texts), key=texts.count))
        for c in cases:
            if c['setup'] == common:
                c.pop('setup')
        out[DEST / f'{name}.json'] = {
            'format': 'kentos.command-cases', 'version': 1, 'command': command, 'commandVersion': 1, 'title': title,
            'note': ('ADR 0730. Bağımsız başvuru scripts/fixtures/dimension_layout_cases.py: sıralar ADR 0729\'un '
                     'başvurusundan, ölçekteki uzaklık, yazının yeri, yuvarlama ve kopya ADR 0730 §1–§4 kurallarından; '
                     'KentOS kodu çalıştırılmaz.'),
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
