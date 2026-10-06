"""The workbench's kept layout, for both platforms: what is kept, its
defaults, how a stored value is read (a field no longer kept, the toolbox's,
is dropped: docs/adr/0155), the sizes as the window allows them, the dock's split as it is dragged, and the ribbon's kept
tab, quick access bar and split choices (format in fixtures/shell/README.md;
CLAUDE.md §4.4 “Yerleşim”).

    python3 scripts/fixtures/layout_cases.py           # writes the file
    python3 scripts/fixtures/layout_cases.py --check   # writes nothing; compares

Writes fixtures/shell/v1/layout.json. The fields, defaults, rules and limits
are written here by hand from the web's layout; every answer below is worked
out here, apart from the web's TypeScript (JavaScript's rounding, half up,
where the web rounds). apps/web/src/app/layoutPlan.test.ts holds the web to
the file, and the desktop's layout reads the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

OUT = 'fixtures/shell/v1/layout.json'

# ── What is kept, as the web's layout has it ────────────────────────

KEY = 'kentos.ui.v1'
SAVE_MS = 250
DEFAULTS = {
    'theme': 'dark',
    'rightVisible': True,
    'dockWidth': 312,
    'layersFraction': 0.5,
    'bottomExpanded': False,
    'bottomHeight': 190,
    'bottomTab': 'history',
    'dockTab': 'layers',
    'processingTab': 'tools',
    'processingFolded': [],
    'ribbonTab': 'home',
    'ribbonCollapsed': False,
    'ribbonQuickAccess': [],
    'ribbonSplits': {},
    'overview': False,
    'magnifier': False,
    'magnifierZoom': 4,
}
DOCK = {'min': 240, 'max': 560, 'maxShare': 0.5, 'reset': 312}
LAYERS = {'min': 0.15, 'max': 0.85, 'reset': 0.5}
BOTTOM = {'min': 96, 'maxShare': 0.6, 'reset': 190}
FIELDS = {
    'theme': {'kind': 'enum', 'values': ['dark', 'light']},
    'rightVisible': {'kind': 'boolean'},
    'dockWidth': {'kind': 'number', 'min': DOCK['min'], 'max': DOCK['max']},
    'layersFraction': {'kind': 'number', 'min': LAYERS['min'], 'max': LAYERS['max']},
    'bottomExpanded': {'kind': 'boolean'},
    'bottomHeight': {'kind': 'number', 'min': BOTTOM['min']},
    'bottomTab': {'kind': 'enum', 'values': ['history', 'coords', 'points', 'search', 'messages']},
    'dockTab': {'kind': 'enum', 'values': ['layers', 'processing', 'blocks', 'templates']},
    'processingTab': {'kind': 'enum', 'values': ['tools', 'history']},
    'processingFolded': {'kind': 'texts'},
    'ribbonTab': {'kind': 'text'},
    'ribbonCollapsed': {'kind': 'boolean'},
    'ribbonQuickAccess': {'kind': 'texts'},
    'ribbonSplits': {'kind': 'textMap'},
    'overview': {'kind': 'boolean'},
    'magnifier': {'kind': 'boolean'},
    'magnifierZoom': {'kind': 'number', 'min': 2, 'max': 16},
}
QUICK_FIXED = ['file.save', 'edit.undo', 'edit.redo']

# ── Answers ─────────────────────────────────────────────────────────

MISSING = object()


def js_round(x):
    """JavaScript's Math.round: half up."""
    return math.floor(x + 0.5)


def is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def read_field(rule, v):
    """A stored value by its field's rule, or MISSING when it cannot be taken."""
    kind = rule['kind']
    if kind == 'enum':
        return v if isinstance(v, str) and v in rule['values'] else MISSING
    if kind == 'boolean':
        return v if isinstance(v, bool) else MISSING
    if kind == 'number':
        if not is_number(v) or not math.isfinite(v):
            return MISSING
        return min(max(v, rule.get('min', -math.inf)), rule.get('max', math.inf))
    if kind == 'text':
        return v if isinstance(v, str) else MISSING
    if kind == 'texts':
        return [x for x in v if isinstance(x, str)] if isinstance(v, list) else MISSING
    if kind == 'textMap':
        return {k: x for k, x in v.items() if isinstance(x, str)} if isinstance(v, dict) else MISSING
    raise ValueError(kind)


def read_layout(text):
    """Each field the store holds and its rule takes; the default otherwise, and for all when it is not a JSON object.
    A field the layout does not know is dropped."""
    stored = None
    if text is not None:
        try:
            stored = json.loads(text)
        except ValueError:
            stored = None
    saved = stored if isinstance(stored, dict) else {}
    out = json.loads(json.dumps(DEFAULTS))
    for key, rule in FIELDS.items():
        if key in saved:
            v = read_field(rule, saved[key])
            if v is not MISSING:
                out[key] = v
    return out


def dock_width_on(kept, window):
    return js_round(min(max(kept, DOCK['min']), min(DOCK['max'], window * DOCK['maxShare'])))


def bottom_height_on(kept, window):
    return js_round(min(max(kept, BOTTOM['min']), window * BOTTOM['maxShare']))


def layers_dragged(start, dy, height):
    return min(LAYERS['max'], max(LAYERS['min'], start + dy / height))


def quick_access(kept, exists):
    out = list(QUICK_FIXED)
    for c in kept:
        if c in exists and c not in QUICK_FIXED and c not in out:
            out.append(c)
    return out


def split_current(entries, kept):
    """The index of the entry last chosen (its command and option, | between), or 0."""
    keys = [f"{e['command']}|{e.get('option', '')}" for e in entries]
    return keys.index(kept) if kept in keys else 0


def start_tab(kept, tabs):
    return kept if any(t['id'] == kept and not t.get('contextual') for t in tabs) else 'home'


# ── Cases ───────────────────────────────────────────────────────────

FULL = {
    'theme': 'light', 'rightVisible': False, 'dockWidth': 400, 'layersFraction': 0.35, 'bottomExpanded': True, 'bottomHeight': 260,
    'bottomTab': 'messages', 'dockTab': 'processing', 'processingTab': 'history', 'processingFolded': ['points'],
    'ribbonTab': 'draw', 'ribbonCollapsed': True, 'ribbonQuickAccess': ['view.zoomExtents', 'tool.line'],
    'ribbonSplits': {'circle': 'tool.circle|3N', 'rectangle': 'tool.regularPolygon|'},
    'overview': True, 'magnifier': True, 'magnifierZoom': 8,
}
compact = lambda o: json.dumps(o, ensure_ascii=False, separators=(',', ':'))
READS = [
    ('nothing stored', None),
    ('not JSON', '{"theme":"light",'),
    ('a JSON list, not an object', '["light"]'),
    ('a JSON number', '42'),
    ('JSON null', 'null'),
    ('an empty object', '{}'),
    ('every field kept, none a default', compact(FULL)),
    ('values of the wrong type are not taken', compact({
        'theme': 'blue', 'rightVisible': 'yes', 'dockWidth': '300', 'bottomTab': 'log', 'ribbonTab': 5, 'processingFolded': 'points',
        'ribbonSplits': ['circle'], 'ribbonQuickAccess': {'0': 'tool.line'}, 'layersFraction': None, 'bottomExpanded': 1,
    })),
    ('sizes out of their limits are brought within them; a tall panel waits for the window', compact({
        'dockWidth': 100, 'layersFraction': 0.95, 'bottomHeight': 20,
    })),
    ('a size too large, a share too small, a very tall panel', compact({'dockWidth': 900, 'layersFraction': 0.05, 'bottomHeight': 5000})),
    ('a number too large for a double is not finite', '{"dockWidth":1e999,"bottomHeight":-1e999}'),
    ('lists keep their texts, maps their text values', compact({
        'processingFolded': [{'id': 'x'}, 'points', 3, None, 'tables', True],
        'ribbonQuickAccess': ['tool.line', ['tool.arc']], 'ribbonSplits': {'circle': 'tool.circle|2N', 'rectangle': 5, 'arc': None},
    })),
    ('the Bloklar tab in front is kept (docs/adr/0144)', compact({'dockTab': 'blocks'})),
    ('the Şablonlar tab in front is kept (docs/adr/0176 §4)', compact({'dockTab': 'templates'})),
    ('a dock tab the dock does not have is not taken', compact({'dockTab': 'styles'})),
    ('fields the layout does not know are dropped', compact({'theme': 'light', 'oldPanel': True, 'ribbonTabs': ['home']})),
    ('the toolbox of the classic shell is no longer kept: its fields are dropped (docs/adr/0155)', compact({
        'theme': 'light', 'toolboxVisible': False, 'toolboxDocked': True, 'toolboxX': 240, 'toolboxY': 80, 'toolboxColumns': 2,
        'toolboxFolded': ['draw'], 'ribbonToolbox': True, 'ribbonTab': 'draw',
    })),
    ('Genel bakış and Büyüteç are kept; a zoom out of its range is brought within it (docs/adr/0181)', compact({
        'overview': True, 'magnifier': True, 'magnifierZoom': 64,
    })),
    ('a fraction and sizes that are not whole are kept as they are', compact({'layersFraction': 0.333, 'dockWidth': 313.5, 'bottomHeight': 200.25})),
]
DOCK_WIDTHS = [(312, 1440), (560, 1440), (560, 1100), (560, 1101), (200, 1440), (313.5, 1440), (400, 400), (1000, 2560)]
BOTTOM_HEIGHTS = [(190, 900), (600, 900), (600, 650), (50, 900), (400, 651), (190, 150), (5000, 1440)]
LAYERS_DRAGS = [(0.5, 100, 600), (0.5, -400, 600), (0.8, 50, 400), (0.3, 0, 500), (0.15, -10, 300)]
QUICK = [
    ([], ['tool.line']),
    (['view.zoomExtents', 'tool.line'], ['view.zoomExtents', 'tool.line']),
    (['edit.undo', 'tool.line', 'tool.line', 'gone.command', 'view.grid'], ['tool.line', 'view.grid', 'edit.undo']),
]
CIRCLE = [{'command': 'tool.circle'}, {'command': 'tool.circle', 'option': '2N'}, {'command': 'tool.circle', 'option': '3N'},
          {'command': 'tool.circle', 'option': 'TTY'}, {'command': 'tool.circle', 'option': 'TTT'}]
FAMILY = [{'command': 'tool.rectangle'}, {'command': 'tool.rectangle3'}, {'command': 'tool.regularPolygon'}]
SPLITS = [
    (CIRCLE, 'tool.circle|3N'),
    (CIRCLE, None),
    (CIRCLE, 'tool.circle|'),
    (CIRCLE, 'tool.circle|KTT'),
    (CIRCLE, 'tool.arc|3N'),
    (FAMILY, 'tool.regularPolygon|'),
    (FAMILY, 'tool.regularPolygon'),
]
TABS = [{'id': 'file'}, {'id': 'home'}, {'id': 'draw'}, {'id': 'modify'}, {'id': 'view'}, {'id': 'selection', 'contextual': 'selection'}]
CAD_TABS = [t for t in TABS if t['id'] != 'modify']
START_TABS = [('draw', TABS), ('view', TABS), ('selection', TABS), ('gone', TABS), ('modify', CAD_TABS), ('', TABS)]

file = {
    'format': 'kentos.layout',
    'version': 1,
    'key': KEY,
    'saveMs': SAVE_MS,
    'defaults': DEFAULTS,
    'fields': FIELDS,
    'limits': {'dockWidth': DOCK, 'layersFraction': LAYERS, 'bottomHeight': BOTTOM},
    'reads': [{'title': t, 'stored': s, 'layout': read_layout(s)} for t, s in READS],
    'dockWidths': [{'kept': k, 'window': w, 'shown': dock_width_on(k, w)} for k, w in DOCK_WIDTHS],
    'bottomHeights': [{'kept': k, 'window': w, 'shown': bottom_height_on(k, w)} for k, w in BOTTOM_HEIGHTS],
    'layersDrags': [{'start': s, 'dy': dy, 'height': h, 'fraction': layers_dragged(s, dy, h)} for s, dy, h in LAYERS_DRAGS],
    'ribbon': {
        'quickAccessFixed': QUICK_FIXED,
        'quickAccess': [{'kept': k, 'exists': e, 'bar': quick_access(k, e)} for k, e in QUICK],
        'splits': [{'entries': e, 'kept': k, 'current': split_current(e, k)} for e, k in SPLITS],
        'startTabs': [{'kept': k, 'tabs': t, 'tab': start_tab(k, t)} for k, t in START_TABS],
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
