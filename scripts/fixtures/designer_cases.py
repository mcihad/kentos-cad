"""The model designer's rules for both platforms: the model's edits (inputs,
steps, sources, outputs, layout), what can feed what, the checks that stop a
model, the order its steps run in, the designer's words, and the diagram's
geometry (format in fixtures/processing/README.md; docs/specs/model-designer.md).

    python3 scripts/fixtures/designer_cases.py           # writes the file
    python3 scripts/fixtures/designer_cases.py --check   # writes nothing; compares

Writes fixtures/processing/v1/designer.json. The words, the tables and the
fixture's own tools are written here by hand from the web's designer; every
answer below is worked out here, apart from the web's TypeScript
(JavaScript's rounding, half up, where the web rounds; its Turkish upper
case where it folds a label into a name).
apps/web/src/ui/processing/model/designerPlan.test.ts holds the web to the
file, and the desktop's designer reads the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import copy
import json
import math
import re
import sys

OUT = 'fixtures/processing/v1/designer.json'

# ── The fixture's tools (ParamDef as the web writes it; visibleWhen as data) ──

TOOLS = [
    {
        'id': 't.corners', 'label': 'Köşe noktalarını numarala', 'icon': 'numbering',
        'parameters': [
            {'type': 'features', 'name': 'input', 'label': 'Girdi nesneleri', 'kinds': ['polygon', 'polyline'], 'default': {'scope': 'selection'}},
            {'type': 'string', 'name': 'prefix', 'label': 'Önek', 'default': 'P', 'allowEmpty': True},
            {'type': 'number', 'name': 'start', 'label': 'İlk numara', 'default': 1, 'min': 1, 'max': 99999, 'integer': True},
            {'type': 'enum', 'name': 'from', 'label': 'Başlangıç köşesi', 'options': [{'value': 'north', 'label': 'Kuzeybatı'}, {'value': 'point', 'label': 'Seçilen noktaya en yakın'}], 'default': 'north'},
            {'type': 'point', 'name': 'startPoint', 'label': 'Başlangıç noktası', 'visibleWhen': {'param': 'from', 'equals': 'point'}},
            {'type': 'layer', 'name': 'target', 'label': 'Nokta katmanı', 'default': {'newName': 'Köşe noktaları'}, 'advanced': True},
        ],
        'outputs': [{'name': 'points', 'label': 'Köşe noktaları', 'type': 'features'}, {'name': 'count', 'label': 'Nokta sayısı', 'type': 'number'}],
    },
    {
        'id': 't.lengths', 'label': 'Kenar uzunluklarını yaz', 'icon': 'dimension',
        'parameters': [
            {'type': 'features', 'name': 'input', 'label': 'Girdi nesneleri', 'default': {'scope': 'selection'}},
            {'type': 'number', 'name': 'decimals', 'label': 'Ondalık', 'default': 2, 'min': 0, 'max': 4, 'integer': True, 'unit': 'basamak'},
            {'type': 'string', 'name': 'suffix', 'label': 'Sonek', 'optional': True},
        ],
        'outputs': [{'name': 'labels', 'label': 'Ölçü yazıları', 'type': 'features'}],
    },
    {
        'id': 't.calc', 'label': 'Öznitelik hesapla', 'icon': 'calculator',
        'parameters': [
            {'type': 'features', 'name': 'input', 'label': 'Nesneler', 'default': {'scope': 'selection'}},
            {'type': 'field', 'name': 'field', 'label': 'Alan'},
            {'type': 'expression', 'name': 'value', 'label': 'Değer'},
            {'type': 'boolean', 'name': 'label', 'label': 'Etiket yap', 'default': False},
            {'type': 'string', 'name': 'unit', 'label': 'Birim', 'default': 'm'},
        ],
        'outputs': [{'name': 'changed', 'label': 'Değişen nesneler', 'type': 'features'}],
    },
    {
        'id': 't.select', 'label': 'İfadeyle seç', 'icon': 'select',
        'parameters': [
            {'type': 'features', 'name': 'input', 'label': 'Nesneler', 'default': {'scope': 'visible'}},
            {'type': 'expression', 'name': 'where', 'label': 'Koşul'},
            {'type': 'string', 'name': 'note', 'label': 'Not'},
            {'type': 'expression', 'name': 'order', 'label': 'Sıra', 'default': '$id', 'advanced': True},
        ],
        'outputs': [{'name': 'selected', 'label': 'Seçilenler', 'type': 'features'}, {'name': 'summary', 'label': 'Özet', 'type': 'string'}],
    },
]
TOOL = {t['id']: t for t in TOOLS}
DEFAULTS = {'lengthDecimals': 3, 'areaDecimals': 2, 'angleUnit': 'grad', 'plotScale': 1000, 'activeLayer': 'L1', 'drawingFont': 'barlow'}

INPUT_TYPES = [
    {'type': 'features', 'label': 'Nesneler', 'icon': 'select', 'description': 'Seçili, görünen, bütün nesneler ya da bir katman'},
    {'type': 'number', 'label': 'Sayı', 'icon': 'units', 'description': 'Mesafe, adet, ondalık basamak …'},
    {'type': 'string', 'label': 'Metin', 'icon': 'text', 'description': 'Önek, alan adı, ifade …'},
    {'type': 'boolean', 'label': 'Evet / hayır', 'icon': 'check', 'description': 'Açık ya da kapalı bir seçenek'},
    {'type': 'layer', 'label': 'Katman', 'icon': 'layers', 'description': 'Sonuçların yazılacağı katman'},
    {'type': 'point', 'label': 'Nokta', 'icon': 'point', 'description': 'Haritada gösterilen bir nokta'},
]
PARAM_TYPES = ['features', 'number', 'string', 'boolean', 'enum', 'layer', 'point', 'expression', 'field']

CANVAS = {
    'inputW': 190, 'inputH': 52, 'stepW': 240, 'stepH': 60, 'grid': 10, 'zoomMin': 0.35, 'zoomMax': 2, 'zoomStep': 1.25,
    'wheel': 0.0015, 'fitPad': 48, 'empty': {'x': 24, 'y': 24, 'k': 1}, 'drag': 3, 'paletteDrag': 5, 'column': 290, 'row': 100, 'bend': 40,
    'labelGap': 8, 'labelRise': 6, 'labelRow': 13,
}
HISTORY = {'depth': 100, 'coalesceMs': 1200}


def js_round(x):
    return math.floor(x + 0.5)


# ── The model's rules (processing/model.ts, modelEdit.ts) ──────────────

def can_feed(src, to):
    if src == to:
        return True
    if to == 'string':
        return src in ('number', 'enum')
    if to in ('expression', 'field'):
        return src == 'string'
    return False


def fold_turkish(s):
    """Turkish upper case, then the dotted and cedilla letters plain (İ → I …)."""
    s = s.strip().replace('i', 'İ').upper()
    for a, b in [('Ç', 'C'), ('Ğ', 'G'), ('İ', 'I'), ('Ö', 'O'), ('Ş', 'S'), ('Ü', 'U')]:
        s = s.replace(a, b)
    return s


def slug(label):
    words = [w for w in re.split(r'[^a-z0-9]+', fold_turkish(label).lower()) if w]
    s = ''.join(w if i == 0 else w[0].upper() + w[1:] for i, w in enumerate(words))
    return s if re.match(r'^[a-z]', s) else f'g{s}'


def unique(base, taken):
    name, k = base, 2
    while taken(name):
        name = f'{base}{k}'
        k += 1
    return name


def step_name(step):
    t = TOOL.get(step['tool'])
    return step.get('caption') or (t['label'] if t else None) or step['tool']


def default_value(p):
    if 'default' in p:
        return copy.deepcopy(p['default'])
    t = p['type']
    if t == 'features':
        return {'scope': (p.get('scopes') or ['selection'])[0]}
    if t == 'number':
        return p.get('min', 0)
    if t in ('string', 'expression', 'field'):
        return ''
    if t == 'boolean':
        return False
    if t == 'enum':
        return p['options'][0]['value']
    if t == 'layer':
        return {'layerId': DEFAULTS['activeLayer']}
    return None


def visible(p, known):
    w = p.get('visibleWhen')
    return not w or known.get(w['param'], MISSING) == w['equals']


MISSING = object()


def needs_source(p):
    if p.get('optional') or 'default' in p:
        return False
    return p['type'] in ('point', 'expression', 'field') or (p['type'] == 'string' and not p.get('allowEmpty'))


def order_steps(model):
    deps = {s['id']: {v['step'] for v in s['values'].values() if v['kind'] == 'output'} for s in model['steps']}
    order, ready = [], [s['id'] for s in model['steps'] if not deps[s['id']]]
    while ready:
        sid = ready.pop(0)
        order.append(sid)
        for other in [s['id'] for s in model['steps']]:
            d = deps[other]
            if sid not in d:
                continue
            d.discard(sid)
            if not d:
                ready.append(other)
    if len(order) == len(model['steps']):
        return order
    stuck = [s['id'] for s in model['steps'] if s['id'] not in order]
    return {'error': f"Modelde döngü var: {' → '.join(stuck)} birbirini bekliyor."}


def check_model(model):
    issues = []
    ids = [s['id'] for s in model['steps']]
    if len(set(ids)) != len(ids):
        issues.append({'message': 'Adım kimlikleri benzersiz olmalı.'})
    steps = {s['id']: s for s in model['steps']}
    inputs = {i['name']: i for i in model['inputs']}
    for s in model['steps']:
        tool = TOOL.get(s['tool'])
        if not tool:
            issues.append({'step': s['id'], 'message': f"Bilinmeyen işlem aracı: {s['tool']}"})
            continue
        known = {}
        for p in tool['parameters']:
            src = s['values'].get(p['name'])
            if src and src['kind'] == 'value':
                known[p['name']] = src['value']
            elif 'default' in p:
                known[p['name']] = p['default']
        for p in tool['parameters']:
            src = s['values'].get(p['name'])
            if not src:
                if needs_source(p) and visible(p, known):
                    issues.append({'step': s['id'], 'message': f"“{p['label']}” için bir değer ya da bağlantı gerekiyor."})
                continue
            if src['kind'] == 'input':
                i = inputs.get(src['name'])
                if not i:
                    issues.append({'step': s['id'], 'message': f"“{p['label']}” olmayan bir model girdisine bağlı: {src['name']}"})
                elif not can_feed(i['type'], p['type']):
                    issues.append({'step': s['id'], 'message': f"“{p['label']}” “{i['label']}” girdisinden beslenemez: türler uymuyor."})
            if src['kind'] == 'output':
                frm = steps.get(src['step'])
                ft = TOOL.get(frm['tool']) if frm else None
                out = next((o for o in (ft or {}).get('outputs', []) if o['name'] == src['output']), None)
                if not out:
                    issues.append({'step': s['id'], 'message': f"“{p['label']}” olmayan bir çıktıya bağlı: {src['step']}.{src['output']}"})
                elif not can_feed(out['type'], p['type']):
                    issues.append({'step': s['id'], 'message': f"“{p['label']}” “{out['label']}” çıktısından beslenemez: türler uymuyor."})
    o = order_steps(model)
    if isinstance(o, dict):
        issues.append({'message': o['error']})
    return issues


def dependents(model, sid):
    found = set()

    def visit(i):
        for s in model['steps']:
            if s['id'] not in found and any(v['kind'] == 'output' and v['step'] == i for v in s['values'].values()):
                found.add(s['id'])
                visit(s['id'])
    visit(sid)
    return found


def sources_for(model, sid, p):
    out = [{'src': {'kind': 'input', 'name': i['name']}, 'label': i['label'], 'group': 'Girdi'} for i in model['inputs'] if can_feed(i['type'], p['type'])]
    down = dependents(model, sid)
    for s in model['steps']:
        if s['id'] == sid or s['id'] in down:
            continue
        for o in TOOL.get(s['tool'], {}).get('outputs', []):
            if can_feed(o['type'], p['type']):
                out.append({'src': {'kind': 'output', 'step': s['id'], 'output': o['name']}, 'label': o['label'], 'group': step_name(s)})
    return out


def add_input(model, type_, label, at=None):
    name = unique(slug(label), lambda n: any(i['name'] == n for i in model['inputs']))
    d = {'type': type_, 'name': name, 'label': label}
    if type_ == 'features':
        d['default'] = {'scope': 'selection'}
    elif type_ == 'number':
        d['default'] = 0
    elif type_ == 'string':
        d.update({'default': '', 'allowEmpty': True})
    elif type_ == 'boolean':
        d['default'] = False
    elif type_ == 'layer':
        d['default'] = {'newName': label}
    model['inputs'].append(d)
    model['inputPositions'] = {**model.get('inputPositions', {}), name: at or {'x': 40, 'y': 40 + (len(model['inputs']) - 1) * 90}}
    return name


def next_free_spot(model):
    xs = [s.get('position', {}).get('x', 0) for s in model['steps']]
    return {'x': max(xs) + 280 if xs else 300, 'y': 60}


def add_step(model, tool_id, at=None, frm=None):
    tool = TOOL.get(tool_id)
    sid = unique(slug(tool['label'] if tool else tool_id), lambda n: any(s['id'] == n for s in model['steps']))
    step = {'id': sid, 'tool': tool_id, 'values': {}, 'position': at or next_free_spot(model)}
    model['steps'].append(step)
    if tool and frm:
        def matches(o):
            if frm['kind'] == 'input':
                return o['src']['kind'] == 'input' and o['src']['name'] == frm['name']
            return o['src']['kind'] == 'output' and o['src']['step'] == frm['id']
        for p in tool['parameters']:
            hit = next((o for o in sources_for(model, sid, p) if matches(o)), None)
            if hit:
                step['values'][p['name']] = hit['src']
                break
    return sid


def set_source(model, sid, param, src):
    step = next((s for s in model['steps'] if s['id'] == sid), None)
    if not step:
        return
    if src:
        step['values'][param] = src
    else:
        step['values'].pop(param, None)


def remove_input(model, name):
    model['inputs'] = [i for i in model['inputs'] if i['name'] != name]
    model.get('inputPositions', {}).pop(name, None)
    for s in model['steps']:
        for p in [p for p, v in s['values'].items() if v['kind'] == 'input' and v['name'] == name]:
            del s['values'][p]


def remove_step(model, sid):
    model['steps'] = [s for s in model['steps'] if s['id'] != sid]
    for s in model['steps']:
        for p in [p for p, v in s['values'].items() if v['kind'] == 'output' and v['step'] == sid]:
            del s['values'][p]
    model['outputs'] = [o for o in model['outputs'] if o['from']['step'] != sid]


def set_caption(model, sid, text):
    step = next((s for s in model['steps'] if s['id'] == sid), None)
    if not step:
        return
    if text.strip():
        step['caption'] = text.strip()
    else:
        step.pop('caption', None)


def input_type_for(p):
    t = p['type']
    if t in ('features', 'number', 'string', 'boolean', 'layer', 'point'):
        return t
    if t in ('expression', 'field'):
        return 'string'
    return None


def assign(target, values):
    """Object.assign with undefined values: a key given None leaves the JSON."""
    for k, v in values.items():
        if v is None:
            target.pop(k, None)
        else:
            target[k] = v


def input_from_param(model, sid, param):
    step = next((s for s in model['steps'] if s['id'] == sid), None)
    tool = TOOL.get(step['tool']) if step else None
    p = next((x for x in tool['parameters'] if x['name'] == param), None) if tool else None
    t = input_type_for(p) if p else None
    if not step or not t:
        return None
    pos = step.get('position') or {}
    name = add_input(model, t, p['label'], {'x': pos.get('x', 300) - 290, 'y': pos.get('y', 40)})
    created = next(i for i in model['inputs'] if i['name'] == name)
    d = default_value(p)
    if t == 'features' and p['type'] == 'features':
        assign(created, {'kinds': list(p['kinds']) if p.get('kinds') else None, 'default': d})
    elif t == 'number' and p['type'] == 'number':
        assign(created, {'default': d, 'min': p.get('min'), 'max': p.get('max'), 'integer': p.get('integer'), 'unit': p.get('unit')})
    elif t == 'string':
        assign(created, {'default': d if isinstance(d, str) else '', 'allowEmpty': p.get('allowEmpty') if p['type'] == 'string' else None})
    elif d is not None:
        created['default'] = d
    set_source(model, sid, param, {'kind': 'input', 'name': name})
    return name


def output_name(model, out):
    return f"{out}{len(model['outputs']) + 1}" if any(o['name'] == out for o in model['outputs']) else out


def add_output(model, sid, out):
    step = next(s for s in model['steps'] if s['id'] == sid)
    o = next(x for x in TOOL[step['tool']]['outputs'] if x['name'] == out)
    name = output_name(model, out)
    model['outputs'].append({'name': name, 'label': o['label'], 'from': {'step': sid, 'output': out}})
    return name


def edges_of(model):
    out = {}
    for s in model['steps']:
        for p, src in s['values'].items():
            if src['kind'] == 'value':
                continue
            frm = {'kind': 'input', 'name': src['name']} if src['kind'] == 'input' else {'kind': 'step', 'id': src['step']}
            key = f"{frm['kind']}:{frm.get('name', frm.get('id'))}→{s['id']}"
            out.setdefault(key, {'from': frm, 'to': s['id'], 'params': []})['params'].append(p)
    return list(out.values())


def auto_layout(model):
    ids = order_steps(model)
    order = ids if isinstance(ids, list) else [s['id'] for s in model['steps']]
    depth = {}
    for sid in order:
        s = next(x for x in model['steps'] if x['id'] == sid)
        deps = [depth.get(v['step'], 0) for v in s['values'].values() if v['kind'] == 'output']
        depth[sid] = max(deps) + 1 if deps else 1
    rows = {}

    def place(col):
        row = rows.get(col, 0)
        rows[col] = row + 1
        return {'x': 40 + col * 290, 'y': 40 + row * 100}
    model['inputPositions'] = {i['name']: place(0) for i in model['inputs']}
    for sid in order:
        s = next(x for x in model['steps'] if x['id'] == sid)
        s['position'] = place(depth.get(sid, 1))


# ── The designer (designerPlan.ts) ────────────────────────────────────

def status(problems, steps, inputs):
    if problems:
        return {'kind': 'warn', 'text': f"{len(problems)} sorun var; model kaydedilebilir ama çalışmaz. {problems[0]['message']}"}
    return {'kind': 'ok', 'text': f'{steps} adım, {inputs} girdi. Model çalışmaya hazır.' if steps else 'Soldan bir girdi ve bir araç ekleyerek başlayın.'}


def source_text(model, src):
    if not src:
        return 'Aracın varsayılanı'
    if src['kind'] == 'value':
        return 'Sabit değer'
    if src['kind'] == 'input':
        i = next((i for i in model['inputs'] if i['name'] == src['name']), None)
        return f"Girdi: {i['label'] if i else src['name']}"
    step = next((s for s in model['steps'] if s['id'] == src['step']), None)
    out = next((o for o in TOOL.get(step['tool'], {}).get('outputs', []) if o['name'] == src['output']), None) if step else None
    return f"{step_name(step) if step else src['step']} › {out['label'] if out else src['output']}"


def step_meta(step, problem):
    if problem:
        return {'warn': problem}
    tool = TOOL.get(step['tool'])
    if step.get('caption') and tool:
        return {'text': tool['label']}
    links = sum(1 for v in step['values'].values() if v['kind'] != 'value')
    return {'text': f'{links} bağlantı' if links else 'Bağlantı yok'}


def edge_label(labels):
    return f'{labels[0]} +{len(labels) - 1}' if len(labels) > 1 else (labels[0] if labels else '')


def connect_choices(model, frm, to):
    step = next((s for s in model['steps'] if s['id'] == to), None)
    tool = TOOL.get(step['tool']) if step else None
    if not step or not tool:
        return None
    source_step = next((s for s in model['steps'] if s['id'] == frm.get('id')), None) if frm['kind'] == 'step' else None
    outputs = [None] if frm['kind'] == 'input' else TOOL.get(source_step['tool'] if source_step else '', {}).get('outputs', [])
    items = []
    for out in outputs:
        for p in tool['parameters']:
            fits = next((o for o in sources_for(model, to, p) if ((o['src']['kind'] == 'input' and o['src']['name'] == frm['name']) if frm['kind'] == 'input'
                         else (o['src']['kind'] == 'output' and o['src']['step'] == frm['id'] and o['src']['output'] == (out or {}).get('name')))), None)
            if not fits:
                continue
            current = step['values'].get(p['name'])
            same = current == fits['src']
            item = {'param': p['name'], 'label': f"{out['label']} → {p['label']}" if out else p['label']}
            if current and not same:
                item['detail'] = 'Mevcut bağlantının yerine geçer'
            item.update({'checked': same, 'src': fits['src']})
            items.append(item)
    what = next((i['label'] for i in model['inputs'] if i['name'] == frm['name']), 'undefined') if frm['kind'] == 'input' else (step_name(source_step) if source_step else 'undefined')
    return {'header': f'{step_name(step)}: hangi girdi?', 'items': items, 'none': None if items else f'“{what}” bu adımın hiçbir girdisine uymuyor'}


def spot_near(model, selected):
    pos = None
    if selected and selected['kind'] == 'input':
        pos = model.get('inputPositions', {}).get(selected['name'])
    elif selected and selected['kind'] == 'step':
        pos = next((s.get('position') for s in model['steps'] if s['id'] == selected['id']), None)
    if pos:
        return {'x': pos['x'] + CANVAS['column'], 'y': pos['y']}
    ys = [s.get('position', {}).get('y', 0) for s in model['steps']] + [p['y'] for p in model.get('inputPositions', {}).values()]
    return {'x': 40, 'y': max(ys) + CANVAS['row'] if ys else 40}


def joins_script(changes):
    """Each change: its field key (or none) and when; whether it joins the undo step before."""
    last, out = None, []
    for key, at in changes:
        j = bool(key) and last is not None and last[0] == key and at - last[1] < HISTORY['coalesceMs']
        out.append({'key': key, 'at': at, 'joins': j})
        last = (key, at) if key else None
    return out


# ── The diagram's geometry ────────────────────────────────────────────

def curve(a, b):
    dx = max(CANVAS['bend'], abs(b['x'] - a['x']) / 2)
    return {'c1': {'x': a['x'] + dx, 'y': a['y']}, 'c2': {'x': b['x'] - dx, 'y': b['y']}}


def edge_labels(model):
    """Each edge's label at its target step: right-aligned left of the entry, one row per incoming edge upward, the
    lowest source nearest the entry (ties: the edges' order)."""
    edges = edges_of(model)

    def source_y(frm):
        if frm['kind'] == 'input':
            return model.get('inputPositions', {}).get(frm['name'], {'x': 40, 'y': 40})['y'] + CANVAS['inputH'] / 2
        step = next((s for s in model['steps'] if s['id'] == frm['id']), None)
        return (step.get('position') if step and step.get('position') else {'x': 0, 'y': 0})['y'] + CANVAS['stepH'] / 2
    rows = {}
    into = {}
    for i, e in enumerate(edges):
        into.setdefault(e['to'], []).append(i)
    for indices in into.values():
        # Python's sort is stable: descending source y, ties in the edges' order.
        for row, i in enumerate(sorted(indices, key=lambda k: -source_y(edges[k]['from']))):
            rows[i] = row
    out = []
    for i, e in enumerate(edges):
        step = next((s for s in model['steps'] if s['id'] == e['to']), None)
        if not step:
            continue
        tool = TOOL.get(step['tool'])
        names = [next((d['label'] for d in (tool or {}).get('parameters', []) if d['name'] == p), p) for p in e['params']]
        pos = step.get('position') or {'x': 0, 'y': 0}
        entry = {'x': pos['x'], 'y': pos['y'] + CANVAS['stepH'] / 2}
        out.append({'from': e['from'], 'to': e['to'], 'text': edge_label(names), 'title': ', '.join(names),
                    'at': {'x': entry['x'] - CANVAS['labelGap'], 'y': entry['y'] - CANVAS['labelRise'] - rows[i] * CANVAS['labelRow']}})
    return out


def bounds(model):
    boxes = [{**model.get('inputPositions', {}).get(i['name'], {'x': 40, 'y': 40}), 'w': CANVAS['inputW'], 'h': CANVAS['inputH']} for i in model['inputs']]
    boxes += [{**(s.get('position') or {'x': 0, 'y': 0}), 'w': CANVAS['stepW'], 'h': CANVAS['stepH']} for s in model['steps']]
    if not boxes:
        return None
    x = min(b['x'] for b in boxes)
    y = min(b['y'] for b in boxes)
    return {'x': x, 'y': y, 'w': max(b['x'] + b['w'] for b in boxes) - x, 'h': max(b['y'] + b['h'] for b in boxes) - y}


def fit_view(b, width, height):
    if not b or not width:
        return dict(CANVAS['empty'])
    pad = CANVAS['fitPad']
    k = min(1, (width - pad * 2) / max(1, b['w']), (height - pad * 2) / max(1, b['h']))
    return {'k': k, 'x': (width - b['w'] * k) / 2 - b['x'] * k, 'y': (height - b['h'] * k) / 2 - b['y'] * k}


def zoom_at(view, p, f):
    k = min(CANVAS['zoomMax'], max(CANVAS['zoomMin'], view['k'] * f))
    wx = (p['x'] - view['x']) / view['k']
    wy = (p['y'] - view['y']) / view['k']
    return {'k': k, 'x': p['x'] - wx * k, 'y': p['y'] - wy * k}


def snap(v):
    return js_round(v / CANVAS['grid']) * CANVAS['grid']


# ── Cases ───────────────────────────────────────────────────────────

def new_model():
    return {'id': 'm-fixture', 'label': 'Yeni model', 'category': 'points', 'description': '', 'inputs': [], 'steps': [], 'outputs': [], 'inputPositions': {}}


def apply(model, op):
    kind = op['op']
    if kind == 'addInput':
        return add_input(model, op['type'], op['label'], op.get('at'))
    if kind == 'addStep':
        return add_step(model, op['tool'], op.get('at'), op.get('from'))
    if kind == 'setSource':
        return set_source(model, op['step'], op['param'], op['src'])
    if kind == 'removeInput':
        return remove_input(model, op['name'])
    if kind == 'removeStep':
        return remove_step(model, op['id'])
    if kind == 'inputFromParam':
        return input_from_param(model, op['step'], op['param'])
    if kind == 'addOutput':
        return add_output(model, op['step'], op['output'])
    if kind == 'caption':
        return set_caption(model, op['step'], op['caption'])
    if kind == 'autoLayout':
        return auto_layout(model)
    raise ValueError(kind)


SEQUENCES = [
    ('a chain built from the palette, then its first step removed and the diagram laid out', [
        {'op': 'addInput', 'type': 'features', 'label': 'Parseller'},
        {'op': 'addInput', 'type': 'string', 'label': 'Nokta öneki'},
        {'op': 'addStep', 'tool': 't.corners', 'from': {'kind': 'input', 'name': 'parseller'}},
        {'op': 'setSource', 'step': 'koseNoktalariniNumarala', 'param': 'prefix', 'src': {'kind': 'input', 'name': 'noktaOneki'}},
        {'op': 'addStep', 'tool': 't.lengths', 'from': {'kind': 'step', 'id': 'koseNoktalariniNumarala'}},
        {'op': 'addOutput', 'step': 'koseNoktalariniNumarala', 'output': 'points'},
        {'op': 'addOutput', 'step': 'kenarUzunluklariniYaz', 'output': 'labels'},
        {'op': 'addStep', 'tool': 't.corners', 'at': {'x': 300, 'y': 300}},
        {'op': 'addOutput', 'step': 'koseNoktalariniNumarala2', 'output': 'points'},
        {'op': 'inputFromParam', 'step': 'kenarUzunluklariniYaz', 'param': 'decimals'},
        {'op': 'removeStep', 'id': 'koseNoktalariniNumarala'},
        {'op': 'autoLayout'},
    ]),
    ('what stops a model: missing values, a missing input, types that do not fit, a cycle, an unknown tool', [
        {'op': 'addStep', 'tool': 't.calc'},
        {'op': 'addStep', 'tool': 't.select'},
        {'op': 'setSource', 'step': 'ifadeyleSec', 'param': 'note', 'src': {'kind': 'value', 'value': 'x'}},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'field', 'src': {'kind': 'output', 'step': 'ifadeyleSec', 'output': 'summary'}},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'value', 'src': {'kind': 'output', 'step': 'ifadeyleSec', 'output': 'summary'}},
        {'op': 'setSource', 'step': 'ifadeyleSec', 'param': 'input', 'src': {'kind': 'output', 'step': 'oznitelikHesapla', 'output': 'changed'}},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'input', 'src': {'kind': 'input', 'name': 'yok'}},
        {'op': 'setSource', 'step': 'ifadeyleSec', 'param': 'where', 'src': {'kind': 'output', 'step': 'oznitelikHesapla', 'output': 'changed'}},
        {'op': 'setSource', 'step': 'ifadeyleSec', 'param': 'input', 'src': None},
        {'op': 'addStep', 'tool': 't.gone'},
        {'op': 'inputFromParam', 'step': 'ifadeyleSec', 'param': 'where'},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'input', 'src': {'kind': 'output', 'step': 'ifadeyleSec', 'output': 'nothing'}},
        {'op': 'removeInput', 'name': 'kosul'},
        {'op': 'autoLayout'},
    ]),
    ('a parameter shown only for one choice is needed only then', [
        {'op': 'addStep', 'tool': 't.corners'},
        {'op': 'setSource', 'step': 'koseNoktalariniNumarala', 'param': 'from', 'src': {'kind': 'value', 'value': 'point'}},
        {'op': 'setSource', 'step': 'koseNoktalariniNumarala', 'param': 'startPoint', 'src': {'kind': 'value', 'value': {'x': 412100, 'y': 4512100}}},
        {'op': 'setSource', 'step': 'koseNoktalariniNumarala', 'param': 'from', 'src': None},
        {'op': 'setSource', 'step': 'koseNoktalariniNumarala', 'param': 'startPoint', 'src': None},
    ]),
    ('each kind of parameter made a model input', [
        {'op': 'addStep', 'tool': 't.corners', 'at': {'x': 600, 'y': 200}},
        {'op': 'inputFromParam', 'step': 'koseNoktalariniNumarala', 'param': 'input'},
        {'op': 'inputFromParam', 'step': 'koseNoktalariniNumarala', 'param': 'prefix'},
        {'op': 'inputFromParam', 'step': 'koseNoktalariniNumarala', 'param': 'start'},
        {'op': 'inputFromParam', 'step': 'koseNoktalariniNumarala', 'param': 'from'},
        {'op': 'inputFromParam', 'step': 'koseNoktalariniNumarala', 'param': 'target'},
        {'op': 'inputFromParam', 'step': 'koseNoktalariniNumarala', 'param': 'startPoint'},
        {'op': 'addStep', 'tool': 't.calc', 'at': {'x': 900, 'y': 200}},
        {'op': 'inputFromParam', 'step': 'oznitelikHesapla', 'param': 'field'},
        {'op': 'inputFromParam', 'step': 'oznitelikHesapla', 'param': 'label'},
        {'op': 'addStep', 'tool': 't.lengths', 'at': {'x': 900, 'y': 400}},
        {'op': 'inputFromParam', 'step': 'kenarUzunluklariniYaz', 'param': 'suffix'},
    ]),
    ('one source feeding two parameters of a step is one edge; names taken get a number', [
        {'op': 'addInput', 'type': 'string', 'label': 'Metin', 'at': {'x': 40, 'y': 40}},
        {'op': 'addInput', 'type': 'string', 'label': 'Metin', 'at': {'x': 40, 'y': 140}},
        {'op': 'addStep', 'tool': 't.calc', 'at': {'x': 330, 'y': 40}},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'field', 'src': {'kind': 'input', 'name': 'metin'}},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'value', 'src': {'kind': 'input', 'name': 'metin'}},
        {'op': 'addStep', 'tool': 't.select', 'at': {'x': 330, 'y': 160}, 'from': {'kind': 'input', 'name': 'metin2'}},
        {'op': 'addStep', 'tool': 't.calc', 'from': {'kind': 'step', 'id': 'ifadeyleSec'}},
        {'op': 'setSource', 'step': 'oznitelikHesapla', 'param': 'input', 'src': {'kind': 'output', 'step': 'ifadeyleSec', 'output': 'selected'}},
        {'op': 'caption', 'step': 'ifadeyleSec', 'caption': '  Seçim  '},
        {'op': 'setSource', 'step': 'ifadeyleSec', 'param': 'note', 'src': {'kind': 'value', 'value': 'kontrol'}},
        {'op': 'caption', 'step': 'oznitelikHesapla', 'caption': 'Ada alanı'},
        {'op': 'caption', 'step': 'oznitelikHesapla', 'caption': '   '},
        {'op': 'caption', 'step': 'oznitelikHesapla2', 'caption': 'Hesap'},
    ]),
]


def run_sequence(title, ops):
    model = new_model()
    steps = []
    for op in ops:
        result = apply(model, op)
        issues = check_model(model)
        steps.append({'op': op, 'result': result, 'model': copy.deepcopy(model), 'issues': issues})
    return {'title': title, 'steps': steps}, model


sequences, finals = [], []
for title, ops in SEQUENCES:
    seq, final = run_sequence(title, ops)
    sequences.append(seq)
    finals.append((title, final))


def model_facts(title, model):
    """What the designer reads off a model: sources for each parameter, edges, order, statuses, box texts, wires."""
    problems = check_model(model)
    first = {}
    for p in problems:
        if p.get('step') and p['step'] not in first:
            first[p['step']] = p['message']
    sources = []
    for s in model['steps']:
        for p in TOOL.get(s['tool'], {}).get('parameters', []):
            sources.append({'step': s['id'], 'param': p['name'], 'options': sources_for(model, s['id'], p), 'text': source_text(model, s['values'].get(p['name']))})
    wires = []
    for frm in [{'kind': 'input', 'name': i['name']} for i in model['inputs']] + [{'kind': 'step', 'id': s['id']} for s in model['steps']]:
        for s in model['steps']:
            if frm.get('id') == s['id']:
                continue
            wires.append({'from': frm, 'to': s['id'], 'choices': connect_choices(model, frm, s['id'])})
    edges = edges_of(model)
    order = order_steps(model)
    return {
        'title': title,
        'model': model,
        'status': status(problems, len(model['steps']), len(model['inputs'])),
        'order': order,
        'edges': edges,
        'edgeLabels': edge_labels(model),
        'stepMeta': [{'step': s['id'], 'meta': step_meta(s, first.get(s['id']))} for s in model['steps']],
        'sources': sources,
        'wires': wires,
        'bounds': bounds(model),
    }


A = {'x': 230, 'y': 66}
POINTS = [(A, {'x': 330, 'y': 70}), (A, {'x': 830, 'y': 190}), ({'x': 520, 'y': 90}, {'x': 300, 'y': 400}), (A, A), ({'x': 12.5, 'y': 7}, {'x': 101.25, 'y': -33})]
FITS = [(None, 1000, 600), ({'x': 40, 'y': 40, 'w': 530, 'h': 160}, 1000, 600), ({'x': 40, 'y': 40, 'w': 2000, 'h': 900}, 1000, 600), ({'x': -200, 'y': 10, 'w': 900, 'h': 1400}, 820, 560), ({'x': 40, 'y': 40, 'w': 190, 'h': 52}, 0, 0)]
ZOOMS = [({'x': 0, 'y': 0, 'k': 1}, {'x': 500, 'y': 300}, 1.25), ({'x': 24, 'y': 24, 'k': 1.9}, {'x': 100, 'y': 100}, 1.25), ({'x': -30, 'y': 12, 'k': 0.4}, {'x': 400, 'y': 250}, 1 / 1.25), ({'x': 10, 'y': 10, 'k': 1}, {'x': 0, 'y': 0}, math.exp(-120 * CANVAS['wheel'])), ({'x': 10, 'y': 10, 'k': 1}, {'x': 250, 'y': 125}, math.exp(480 * CANVAS['wheel']))]
SNAPS = [0, 4, 5, 14.99, 15, -15, -16, 123.4, 1234.5]
SLUGS = ['Nokta öneki', 'Parseller', 'Köşe noktalarını numarala', 'İfadeyle seç', '3 nokta', '', '  Çıktı katmanı  ', 'ALAN (m²)', 'a_b-c', 'Işık ve ıslak', 'Kâğıt', 'Öznitelik hesapla']
SPOTS = [None, {'kind': 'input', 'name': 'metin'}, {'kind': 'step', 'id': 'ifadeyleSec'}, {'kind': 'step', 'id': 'yok'}]
TITLES = [('Parsel ölçü yazıları', False), ('Parsel ölçü yazıları', True), ('', False), ('', True)]
SAVED_LABELS = ['Model', '   ', '', ' Ada ']
JOINS = [('label', 0), ('label', 1000), ('label', 2300), (None, 2400), ('label', 2500), ('description', 2600), ('label', 2700), ('label', 3899), ('label', 5099)]

TEXTS = {
    'title': 'Model tasarımcısı',
    'titleOf': None,
    'notFound': None,
    'footer': {'layout': 'Düzenle', 'layoutTip': 'Kutuları bağlantı sırasına göre sütunlara dizer', 'close': 'Kapat', 'saveRun': 'Kaydet ve çalıştır…', 'save': 'Kaydet'},
    'status': {'problems': None, 'ready': None, 'empty': 'Soldan bir girdi ve bir araç ekleyerek başlayın.'},
    'save': {'unnamed': 'Adsız model', 'saved': None},
    'unsaved': {'after': 'Pencere kapanırsa bu değişiklikler kaybolur.', 'verb': 'kapat'},
    'remove': {'title': 'Modeli sil', 'question': None, 'action': 'Modeli sil', 'done': None},
    'pick': {'point': 'Nokta', 'command': None, 'unnamed': 'nokta'},
    'connect': {'header': None, 'fromStep': None, 'replaces': 'Mevcut bağlantının yerine geçer', 'none': None},
    'canvas': {'label': 'Model diyagramı', 'zoomOut': 'Uzaklaş', 'zoomIn': 'Yakınlaş', 'fit': 'Tümünü göster (çift tık)', 'port': 'Sürükleyip bir adımın üzerine bırakın', 'inputMeta': None, 'links': None, 'noLinks': 'Bağlantı yok'},
    'palette': {
        'label': 'Model parçaları', 'inputs': 'Girdi ekle', 'tools': 'Araçlar', 'search': 'Araç ara', 'empty': 'Aramayla eşleşen araç yok.', 'toolNote': 'Tıklayın ya da tuvale sürükleyin',
        'tip': 'Bir aracı tıklayın ya da tuvale sürükleyin. Seçili kutu varsa yeni adım ona bağlanır; bağlantıyı değiştirmek için kutunun sağındaki noktadan sürükleyin.',
    },
    'inspector': {
        'label': 'Seçilen kutunun ayarları',
        'model': {
            'kind': 'Model', 'lead': 'Adı ve açıklaması araç kutusunda ve menüde görünür.', 'name': 'Ad', 'nameAria': 'Model adı', 'category': 'Kategori', 'description': 'Açıklama',
            'descriptionAria': 'Model açıklaması', 'descriptionHint': 'Ne yapar, tek cümle', 'outputs': 'Model çıktıları', 'output': None, 'noStep': 'adım yok', 'removeOutput': None,
            'addOutput': 'Çıktı ekle', 'noOutput': 'Eklenebilecek çıktı yok', 'remove': 'Modeli sil',
        },
        'problems': {'title': 'Sorunlar', 'titleCount': None, 'ready': 'Model çalışmaya hazır.', 'start': 'Başlamak için soldan bir girdi ve bir araç ekleyin.', 'ofStep': None},
        'input': {
            'kind': None, 'lead': 'Model çalıştırılırken kullanıcıdan istenir.', 'label': 'Etiket', 'labelAria': 'Girdi etiketi', 'variable': None, 'description': 'Açıklama',
            'descriptionAria': 'Girdi açıklaması', 'descriptionHint': 'Pencerede etiketin altında görünür', 'optional': 'İsteğe bağlı', 'default': 'Varsayılan', 'scopeAria': 'Varsayılan kapsam',
            'scopes': {'selection': 'Seçili', 'visible': 'Görünen', 'all': 'Tümü'}, 'kinds': 'Uygun nesneler', 'kindsNote': 'Hiçbiri seçili değilse her tür alınır; adımlar kendi türlerini ayrıca süzer.',
            'min': 'En az', 'max': 'En çok', 'none': 'yok', 'integer': 'Tam sayı', 'textDefault': 'Varsayılan metin', 'allowEmpty': 'Boş bırakılabilir', 'newLayer': 'Varsayılan yeni katman',
            'newLayerAria': 'Varsayılan yeni katman adı', 'newLayerNote': 'Çalıştırırken var olan bir katman da seçilebilir.', 'pointNote': 'Çalıştırırken haritada gösterilir ya da Y,X yazılır.',
            'users': 'Kullanan adımlar', 'noUsers': 'Henüz hiçbir adım bu girdiyi kullanmıyor. Kutunun sağındaki noktadan bir adıma sürükleyin.', 'remove': 'Girdiyi sil',
        },
        'step': {
            'unknown': 'Bilinmeyen araç', 'unknownNote': None, 'caption': 'Başlık', 'captionAria': 'Adım başlığı', 'captionNote': 'Diyagramda ve iletilerde görünür.', 'params': 'Parametreler',
            'advanced': None, 'optional': 'isteğe bağlı', 'sourceAria': None, 'toolDefault': 'Aracın varsayılanı', 'fixed': 'Sabit değer', 'modelInput': 'Model girdisi',
            'asInput': 'Yeni model girdisi yap', 'asInputNote': 'Model çalıştırılırken bu değer sorulur', 'inputSource': None, 'outputSource': None, 'remove': 'Adımı sil',
        },
    },
}
# Texts made from values: a sample each, and the text the web writes for it.
SAMPLES = {
    'titleOf': (['Parsel ölçü yazıları', True], lambda label, dirty: f"Model tasarımcısı: {label or 'adsız'}{' •' if dirty else ''}"),
    'notFound': (['m-1a2b'], lambda i: f'Model bulunamadı: {i}.'),
    'status.problems': ([2, '“Alan” için bir değer ya da bağlantı gerekiyor.'], lambda n, first: f'{n} sorun var; model kaydedilebilir ama çalışmaz. {first}'),
    'status.ready': ([3, 2], lambda s, i: f'{s} adım, {i} girdi. Model çalışmaya hazır.'),
    'save.saved': (['Parsel ölçü yazıları', 2], lambda label, n: f"“{label}” modeli kaydedildi{f'; {n} sorun giderilene kadar çalışmaz' if n else ''}."),
    'remove.question': (['Ada özeti'], lambda label: f'“{label}” modeli silinsin mi? Bu geri alınamaz.'),
    'remove.done': (['Ada özeti'], lambda label: f'“{label}” modeli silindi.'),
    'pick.command': (['Başlangıç noktası'], lambda label: f'Model tasarımcısı: {label}'),
    'connect.header': (['Kenar uzunluklarını yaz'], lambda s: f'{s}: hangi girdi?'),
    'connect.fromStep': (['Köşe noktaları', 'Girdi nesneleri'], lambda o, p: f'{o} → {p}'),
    'connect.none': (['Parseller'], lambda w: f'“{w}” bu adımın hiçbir girdisine uymuyor'),
    'canvas.inputMeta': (['Nesneler', True], lambda t, o: f"Girdi: {t}{', isteğe bağlı' if o else ''}"),
    'canvas.links': ([2], lambda n: f'{n} bağlantı'),
    'inspector.model.output': (['Köşe noktalarını numarala', 'Köşe noktaları'], lambda s, o: f'{s} › {o}'),
    'inspector.model.removeOutput': (['Köşe noktaları'], lambda label: f'“{label}” çıktısını kaldır'),
    'inspector.problems.titleCount': ([3], lambda n: f'Sorunlar ({n})'),
    'inspector.problems.ofStep': (['İfadeyle seç', '“Koşul” için bir değer ya da bağlantı gerekiyor.'], lambda s, m: f'{s}: {m}'),
    'inspector.input.kind': (['Sayı'], lambda t: f'Girdi: {t}'),
    'inspector.input.variable': (['noktaOneki'], lambda n: f'Değişken adı: {n}'),
    'inspector.step.unknownNote': (['t.gone'], lambda t: f'“{t}” bu sürümde yok. Adımı silin ya da aracı sağlayan eklentiyi yükleyin.'),
    'inspector.step.advanced': ([1], lambda n: f'Gelişmiş ({n})'),
    'inspector.step.sourceAria': (['Önek'], lambda label: f'{label}: kaynak'),
    'inspector.step.inputSource': (['Nokta öneki'], lambda label: f'Girdi: {label}'),
    'inspector.step.outputSource': (['Köşe noktalarını numarala', 'Nokta sayısı'], lambda s, o: f'{s} › {o}'),
}


def fill(o, path=''):
    for k, v in o.items():
        at = f'{path}.{k}' if path else k
        if v is None:
            sample, fn = SAMPLES[at]
            o[k] = {'sample': sample, 'text': fn(*sample)}
        elif isinstance(v, dict):
            fill(v, at)
    return o


final_models = [model_facts(t, m) for t, m in finals]
spot_model = finals[4][1]

file = {
    'format': 'kentos.modelDesigner',
    'version': 1,
    'texts': fill(copy.deepcopy(TEXTS)),
    'inputTypes': INPUT_TYPES,
    'canvas': CANVAS,
    'history': HISTORY,
    'tools': TOOLS,
    'defaults': DEFAULTS,
    'canFeed': [{'from': f, 'feeds': [t for t in PARAM_TYPES if can_feed(f, t)]} for f in PARAM_TYPES],
    'slugs': [{'label': s, 'slug': slug(s)} for s in SLUGS],
    'newModel': {k: v for k, v in new_model().items() if k != 'id'},
    'copyLabel': {'label': 'Parsel ölçü yazıları', 'copy': 'Parsel ölçü yazıları (kopya)'},
    'sequences': sequences,
    'models': final_models,
    'spots': [{'selected': s, 'spot': spot_near(spot_model, s)} for s in SPOTS] + [{'selected': None, 'empty': True, 'spot': spot_near(new_model(), None)}],
    'titles': [{'label': label, 'dirty': d, 'title': f"Model tasarımcısı: {label or 'adsız'}{' •' if d else ''}"} for label, d in TITLES],
    'savedLabels': [{'label': s, 'saved': s if s.strip() else 'Adsız model'} for s in SAVED_LABELS],
    'joins': joins_script(JOINS),
    'geometry': {
        'curves': [{'a': a, 'b': b, 'curve': curve(a, b)} for a, b in POINTS],
        'ports': {'input': {'at': {'x': 40, 'y': 130}, 'port': {'x': 230, 'y': 156}}, 'step': {'at': {'x': 330, 'y': 40}, 'port': {'x': 570, 'y': 70}, 'entry': {'x': 330, 'y': 70}}},
        'fits': [{'bounds': b, 'width': w, 'height': h, 'view': fit_view(b, w, h)} for b, w, h in FITS],
        'zooms': [{'view': v, 'at': p, 'factor': f, 'result': zoom_at(v, p, f)} for v, p, f in ZOOMS],
        'snaps': [{'value': v, 'snapped': snap(v)} for v in SNAPS],
    },
}
text = json.dumps(file, ensure_ascii=False, indent=1) + '\n'
if '--check' in sys.argv[1:]:
    with open(OUT, encoding='utf-8') as f:
        if f.read() != text:
            sys.exit(f'{OUT} is not what this script writes: run it without --check and read the difference.')
    print(f'{OUT} matches')
else:
    import os
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)
    print(f'{OUT} written')
