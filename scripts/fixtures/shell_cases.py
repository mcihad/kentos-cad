"""The classic shell's rules for both platforms: the menu bar's and the
toolbar's fold steps, the toolbar's groups and fields, the toolbox's groups,
place, columns and showing, how a command's row looks in a menu, and the
bars' words (format in fixtures/shell/README.md; DESIGN.md §7.1–§7.4).

    python3 scripts/fixtures/shell_cases.py           # writes the file
    python3 scripts/fixtures/shell_cases.py --check   # writes nothing; compares

Writes fixtures/shell/v1/shell.json. The tables and words are written here by
hand from DESIGN.md and the web's bars; every answer below is worked out here,
apart from the web's TypeScript. apps/web/src/ui/shell/shellPlan.test.ts holds
the web to the file, and the desktop's classic shell reads the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys

OUT = 'fixtures/shell/v1/shell.json'

# ── Tables, as DESIGN.md §7.1, §7.3 and §7.4 say them ────────────────

MENUBAR_FOLD = [
    {'brandWord': True, 'crsName': True, 'menuPadding': 9},
    {'brandWord': False, 'crsName': True, 'menuPadding': 9},
    {'brandWord': False, 'crsName': False, 'menuPadding': 9},
    {'brandWord': False, 'crsName': False, 'menuPadding': 6},
]

TOOLBAR_GROUPS = [
    {'label': 'Dosya', 'commands': ['file.new', 'file.open', 'file.save']},
    {'label': 'Geçmiş', 'commands': ['edit.undo', 'edit.redo']},
    {'label': 'Görünüm', 'commands': ['view.zoomExtents', 'tool.zoomWindow', 'view.zoomSelection', 'view.zoomIn', 'view.zoomOut', 'tool.pan']},
    {'label': 'Geçerli özellikler', 'fields': True},
    {'label': 'Çizim ölçeği', 'scale': True},
    {'label': 'Paneller', 'commands': ['view.toolbox', 'view.bottomPanel', 'view.rightPanel']},
]
VIEW_MORE = ['view.zoomIn', 'view.zoomOut', 'tool.pan']
TOOLBAR_WIDTHS = {'layer': [196, 150], 'color': [150, 124], 'lineType': [150, 124], 'weight': [160, 132]}


def toolbar_fold(level):
    """1: the fields narrow; 2: + the three Görünüm commands under ⋯; 3: + Renk, Tip and Kalınlık fold into one and the layer field widens again; 4: + the layer field narrows again."""
    return {
        'layerNarrow': level in (1, 2, 4),
        'propsNarrow': level >= 1,
        'viewMore': level >= 2,
        'propsFolded': level >= 3,
    }


DRAW_COLORS = [
    {'name': 'Siyah', 'value': 'ink'},
    {'name': 'Kırmızı', 'value': '#E5484D'},
    {'name': 'Sarı', 'value': '#F2C94C'},
    {'name': 'Yeşil', 'value': '#5FBF77'},
    {'name': 'Camgöbeği', 'value': '#4CC3D9'},
    {'name': 'Mavi', 'value': '#4F8EF7'},
    {'name': 'Eflatun', 'value': '#C86DD7'},
    {'name': 'Gri', 'value': '#8C9AAA'},
]
LINE_TYPES = {'continuous': 'Sürekli', 'dashed': 'Kesikli', 'dashdot': 'Noktalı kesik', 'dotted': 'Noktalı'}
LINE_WEIGHTS = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7]
PLOT_SCALES = [500, 1000, 2000, 5000, 25000]

TOOLBOX_GROUPS = [
    ('select', 'Seçim'), ('draw', 'Çizim'), ('annotate', 'Açıklama'), ('transform', 'Dönüştür'),
    ('modify', 'Düzenle'), ('area', 'Alan'), ('map', 'Harita'),
]
MARGIN, SNAP, MAX_COLUMNS = 8, 14, 6
UNDOCK = {'x': 20, 'y': 10}

# ── Rules, worked out here ───────────────────────────────────────────


def toolbox_columns(stored):
    return 3 if stored == 3 else 2


def fit_columns(base, fits_from):
    """The chosen count, widened one at a time until the tools fit (they fit from `fits_from` columns on; None: never), at most six."""
    columns = base
    while columns < MAX_COLUMNS and not (fits_from is not None and columns >= fits_from):
        columns += 1
    return columns


def place(x, y, w, h, host_w, host_h):
    max_x = max(MARGIN, host_w - w - MARGIN)
    max_y = max(MARGIN, host_h - h - MARGIN)
    nx = min(max(x, MARGIN), max_x)
    ny = min(max(y, MARGIN), max_y)
    if nx - MARGIN < SNAP:
        nx = MARGIN
    if max_x - nx < SNAP:
        nx = max_x
    if ny - MARGIN < SNAP:
        ny = MARGIN
    if max_y - ny < SNAP:
        ny = max_y
    return {'x': nx, 'y': ny}


def shown(shell, visible, ribbon_toolbox):
    return ribbon_toolbox if shell == 'ribbon' else visible


RADIO_PREFIXES = ('view.renderer.', 'view.symbols.', 'workspace.')


def row_look(cid, checked):
    radio = (cid.startswith('view.theme.') and cid != 'view.theme.toggle') or cid.startswith(RADIO_PREFIXES)
    tool = cid.startswith('tool.')
    return {'icon': checked is None or radio or tool, 'checked': None if tool else checked, 'radio': radio}


def num(v):
    """A number as JSON writes it from JavaScript: whole numbers without a decimal point."""
    return int(v) if float(v).is_integer() else v


# ── Cases ────────────────────────────────────────────────────────────

PLACES = [
    ('near the top-left corner: onto the margin', 12, 12, 120, 400, 1000, 700),
    ('free', 300, 100, 120, 400, 1000, 700),
    ('outside, top-left: clamped', -50, -50, 120, 400, 1000, 700),
    ('outside, bottom-right: clamped', 2000, 2000, 120, 400, 1000, 700),
    ('near the bottom-right corner: onto the margin', 860, 280, 120, 400, 1000, 700),
    ('13 px from the margin: snaps', 21, 21, 120, 400, 1000, 700),
    ('14 px from the margin: stays', 22, 22, 120, 400, 1000, 700),
    ('14 px from the far margins: stays', 858, 278, 120, 400, 1000, 700),
    ('a host smaller than the toolbox: the margin', 50, 50, 120, 400, 100, 300),
    ('fractions stay', 100.5, 50.25, 120, 400, 1000, 700),
]
FITS = [(3, 3), (3, 5), (2, 2), (2, 4), (3, None), (3, 2), (2, 6), (2, 7)]
ROWS = [
    ('file.save', None), ('view.rightPanel', True), ('view.rightPanel', False), ('view.theme.dark', True),
    ('view.theme.light', False), ('view.theme.toggle', None), ('view.renderer.webgpu', False), ('view.symbols.plain', True),
    ('workspace.cad', True), ('tool.line', True), ('tool.select', False), ('view.ribbon', False),
]

file = {
    'format': 'kentos.shell',
    'version': 1,
    'menubar': {
        'folds': MENUBAR_FOLD,
        'texts': {'label': 'Ana menü', 'dirty': 'Kaydedilmemiş değişiklikler var', 'crs': 'Koordinat sistemi', 'crsTip': {'sample': 5256, 'text': 'EPSG:5256. Değiştirmek için tıklayın.'}},
    },
    'toolbar': {
        'groups': TOOLBAR_GROUPS,
        'viewMore': VIEW_MORE,
        'widths': TOOLBAR_WIDTHS,
        'folds': [toolbar_fold(level) for level in range(5)],
        'texts': {'label': 'Araç çubuğu', 'more': 'Diğer görünüm komutları', 'moreTip': 'Yakınlaştır, Uzaklaştır ve Kaydır: pencere daralınca buraya girer.'},
    },
    'fields': {
        'byLayer': 'Katmana göre',
        'colors': DRAW_COLORS,
        'lineTypes': LINE_TYPES,
        'weights': [{'mm': num(w), 'text': f'{w:.2f} mm'} for w in LINE_WEIGHTS],
        'scales': [{'denominator': s, 'text': f'1:{s}'} for s in PLOT_SCALES],
    },
    'toolbox': {
        'groups': [{'id': g, 'label': label} for g, label in TOOLBOX_GROUPS],
        'constants': {'margin': MARGIN, 'snap': SNAP, 'maxColumns': MAX_COLUMNS, 'undock': UNDOCK},
        'texts': {
            'label': 'Çizim araçları', 'grip': 'Taşımak için sürükleyin', 'columns': 'Sütun sayısını değiştir', 'twoColumns': 'İki sütun',
            'threeColumns': 'Üç sütun', 'dock': 'Kenara sabitle', 'undock': 'Serbest bırak',
            'fold': {'sample': 'Çizim', 'text': 'Çizim grubunu katla'}, 'unfold': {'sample': 'Çizim', 'text': 'Çizim grubunu aç'},
            'notReady': 'Geliştirme aşamasında',
        },
        'columns': [{'stored': s, 'columns': toolbox_columns(s)} for s in [3, 2, 1, 4, 0]],
        'fit': [{'base': b, 'fitsFrom': f, 'columns': fit_columns(b, f)} for b, f in FITS],
        'place': [{'title': t, 'x': x, 'y': y, 'w': w, 'h': h, 'hostW': hw, 'hostH': hh, 'expect': place(x, y, w, h, hw, hh)} for t, x, y, w, h, hw, hh in PLACES],
        'undock': [{'x': x, 'y': y, 'expect': {'x': x - UNDOCK['x'], 'y': y - UNDOCK['y']}} for x, y in [(300, 200), (10, 5), (20.5, 10.5)]],
        'shown': [{'shell': s, 'visible': v, 'ribbonToolbox': r, 'shown': shown(s, v, r)} for s in ['classic', 'ribbon'] for v in [True, False] for r in [True, False]],
    },
    'menuRows': [{'id': i, 'checked': c, 'expect': row_look(i, c)} for i, c in ROWS],
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
