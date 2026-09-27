"""The ribbon's rules for both platforms: the key tips (letters, levels,
keys), the quick access bar's menu and how a command is added or taken off,
what a right click offers, and a split button's list (format in
fixtures/shell/README.md; docs/specs/ribbon.md).

    python3 scripts/fixtures/ribbon_cases.py           # writes the file
    python3 scripts/fixtures/ribbon_cases.py --check   # writes nothing; compares

Writes fixtures/shell/v1/ribbon.json. The words, the offers and the web's
tab names are written here by hand from the web's ribbon; every answer
below is worked out here, apart from the web's TypeScript.
apps/web/src/ui/ribbon/ribbonPlan.test.ts holds the web to the file (and the
tab names to the ribbon the web builds), and the desktop's ribbon reads the
same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import re
import sys

OUT = 'fixtures/shell/v1/ribbon.json'

# ── Words and tables, as the web's ribbon has them ───────────────────

QUICK_ACCESS = ['file.save', 'edit.undo', 'edit.redo']
OFFERS = ['file.new', 'file.open', 'file.saveAs', 'edit.paste', 'view.zoomExtents', 'tool.zoomWindow', 'tool.pan', 'tools.options', 'view.theme.toggle']
TEXTS = {
    'quickAccess': 'Hızlı erişim',
    'customize': 'Hızlı erişimi özelleştir',
    'customizeTip': 'Şeritteki bir düğmeye sağ tıklayarak da ekleyebilirsiniz.',
    'fixedHint': 'sabit',
    'add': 'Hızlı erişime ekle',
    'remove': 'Hızlı erişimden kaldır',
    'fixed': 'Hızlı erişimde (sabit)',
    'fold': 'Şeridi daralt',
    'pin': 'Şeridi sabitle',
    'foldTip': 'Daraltılmış şerit bir sekmeye tıklayınca çizimin üstünde açılır.',
    'others': None,
    'method': None,
    'panelMore': None,
    'cannotStart': None,
}
SAMPLES = {
    'others': (['Daire'], lambda t: f'{t}: diğer seçenekler'),
    'method': (['Daire', '3 nokta'], lambda t, l: f'{t}: {l}'),
    'panelMore': (['Eğri'], lambda p: f'{p}: diğer araçlar'),
    'cannotStart': (['Daire', '3 nokta'], lambda t, l: f'“{t}: {l}” şu an başlatılamadı.'),
}

# The web's tabs in each ready work mode (app/ribbon.ts ribbonTabs), in order; the selection tab is contextual.
TABS = {
    'hybrid': [('file', 'Dosya'), ('home', 'Giriş'), ('draw', 'Çizim'), ('modify', 'Değiştir'), ('map', 'Harita'), ('view', 'Görünüm'), ('processing', 'İşlemler'), ('tools', 'Araçlar'), ('selection', 'Seçim')],
    'cad': [('file', 'Dosya'), ('home', 'Giriş'), ('draw', 'Çizim'), ('modify', 'Değiştir'), ('map', 'Ölçme'), ('view', 'Görünüm'), ('tools', 'Araçlar'), ('selection', 'Seçim')],
    'gis': [('file', 'Dosya'), ('home', 'Giriş'), ('draw', 'Çizim'), ('modify', 'Değiştir'), ('map', 'Harita'), ('view', 'Görünüm'), ('processing', 'İşlemler'), ('tools', 'Araçlar'), ('selection', 'Seçim')],
}

# ── Answers ─────────────────────────────────────────────────────────

FOLD = {'ç': 'C', 'ğ': 'G', 'ı': 'I', 'i': 'I', 'ö': 'O', 'ş': 'S', 'ü': 'U', 'â': 'A', 'î': 'I', 'û': 'U'}
ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'


def tr_lower(ch):
    return {'I': 'ı', 'İ': 'i'}.get(ch, ch.lower())


def letters_of(label):
    """Upper case, Turkish letters folded, anything but A–Z and 0–9 dropped."""
    out = ''
    for ch in label:
        lower = tr_lower(ch)
        up = FOLD.get(lower, lower.upper())
        if re.fullmatch(r'[A-Z0-9]', up):
            out += up
    return out


def initials(label):
    return ''.join((letters_of(w) or ' ')[0].strip() for w in re.split(r'[\s/–-]+', label))


def assign_key_tips(labels, reserved=()):
    """One letter where a label's first letter is its own; two for the rest, never starting with a single."""
    used = set(reserved)
    out = [''] * len(labels)
    singles = set()
    firsts = [letters_of(l)[:1] for l in labels]
    counts = {}
    for f in firsts:
        counts[f] = counts.get(f, 0) + 1
    for i, f in enumerate(firsts):
        if f and counts[f] == 1 and f not in used:
            out[i] = f
            used.add(f)
            singles.add(f)
    for i, label in enumerate(labels):
        if out[i]:
            continue
        letters = letters_of(label) or 'X'
        ini = initials(label)
        candidates = [ini[:2] if len(ini) >= 2 else ''] + [letters[0] + c for c in letters[1:]] + [letters[0] + c for c in ALPHABET]
        starts = ([letters[0]] if letters[0] and letters[0] not in singles else []) + [c for c in ALPHABET if c not in singles]
        tip = next((c for c in candidates if len(c) == 2 and c not in used and c[0] not in singles), None)
        if not tip:
            tip = next((s + c for s in starts for c in ALPHABET if (s + c) not in used), None)
        out[i] = tip or ''
        if tip:
            used.add(tip)
    return out


def first_level(quick, tab_labels):
    digits = [str(i + 1) for i in range(min(9, quick))]
    return {'quickAccess': digits, 'tabs': assign_key_tips(tab_labels, digits)}


def key_step(level, typed, tips, key, ctrl=False, meta=False):
    if key in ('Alt', 'Shift'):
        return {'kind': 'ignore'}
    if key == 'Escape':
        return {'kind': 'typed', 'typed': ''} if typed else {'kind': 'back'} if level == 'controls' else {'kind': 'hide'}
    if key == 'Backspace' and typed:
        return {'kind': 'typed', 'typed': typed[:-1]}
    ch = letters_of(key)
    if len(ch) != 1 or ctrl or meta:
        return {'kind': 'hide'}
    nxt = typed + ch
    if nxt in tips:
        return {'kind': 'run', 'tip': nxt}
    return {'kind': 'typed', 'typed': nxt} if any(t.startswith(nxt) for t in tips) else {'kind': 'ignore'}


def with_quick_access(kept, cmd, on):
    rest = [x for x in kept if x != cmd]
    return rest + [cmd] if on else rest


def quick_access_menu(bar, exists):
    offers = []
    for c in bar + OFFERS:
        if c not in offers and c in exists:
            offers.append(c)
    rows = [{'kind': 'header', 'label': TEXTS['quickAccess']}]
    for c in offers:
        fixed = c in QUICK_ACCESS
        row = {'kind': 'quick', 'command': c, 'checked': c in bar}
        row.update({'disabled': True, 'hint': TEXTS['fixedHint']} if fixed else {'set': c not in bar})
        rows.append(row)
    return rows + [{'kind': 'separator'}, {'kind': 'command', 'command': 'view.ribbonCollapse'}]


def command_menu(cmd, bar):
    if cmd in QUICK_ACCESS:
        first = {'kind': 'quick', 'command': cmd, 'label': TEXTS['fixed'], 'icon': 'pin', 'disabled': True}
    elif cmd in bar:
        first = {'kind': 'quick', 'command': cmd, 'label': TEXTS['remove'], 'icon': 'close', 'set': False}
    else:
        first = {'kind': 'quick', 'command': cmd, 'label': TEXTS['add'], 'icon': 'pin', 'set': True}
    return [first, {'kind': 'separator'}, {'kind': 'command', 'command': 'view.ribbonCollapse'}]


def split_menu(entries):
    methods = all(e['command'] == entries[0]['command'] for e in entries)
    rows = []
    for e in entries:
        r = {'command': e['command']}
        if e.get('option'):
            r['option'] = e['option']
        r['label'] = e['label']
        if e.get('description'):
            r['hint'] = e['description']
        rows.append(r)
    return {'header': entries[0]['title'] if methods else None, 'rows': rows}


def split_face(e):
    label = e['title'][:-1] if e['title'].endswith('…') else e['title']
    aria = f"{e['title']}: {e['label']}" if e.get('option') or e['label'] != e['title'] else e['title']
    return {'label': label, 'aria': aria}


# ── Cases ───────────────────────────────────────────────────────────

LETTERS = ['Çizim', 'İşlemler', 'Görünüm: ölçü', 'Değiştir', 'Kâğıt', 'ışık', 'I/O 2', 'Yeni proje', 'Kaydet…', '3 nokta', 'Aç–kapat']
ASSIGNS = [
    ('the tabs of the hybrid mode', [l for _, l in TABS['hybrid'][:-1]], []),
    ('one first letter each', ['Dosya', 'Giriş', 'Harita'], []),
    ('shared first letters get two, from the initials first', ['Yeni proje', 'Yapıştır', 'Yakınlaştır', 'Yay'], []),
    ('a reserved tip is never given', ['1. adım', 'Kaydet', 'Kes'], ['1', '2']),
    ('many look-alike labels all get two letters', [f'Komut {i}' for i in range(30)], ['1', '2']),
    ('a label without letters', ['…', '—', 'Dosya'], []),
    ('the controls of a panel', ['Yapıştır', 'Kes', 'Panoya kopyala', 'Özgün koordinatlara yapıştır', 'Seç', 'Pencere yakınlaştır', 'Kaydır', 'Çizgi', 'Çoklu çizgi', 'Daire: Merkez, yarıçap', 'Daire: diğer seçenekler', 'Yay: 3 nokta', 'Yay: diğer seçenekler', 'Kapalı alan'], []),
]
FIRST_LEVELS = [
    (3, 'hybrid', False), (5, 'hybrid', True), (3, 'cad', False), (3, 'gis', False), (12, 'hybrid', False), (0, 'cad', True),
]
TIPS = ['1', '2', '3', 'D', 'GI', 'C', 'DE', 'H', 'GO', 'I', 'A', 'S']
STEPS = [
    ('tabs', '', 'd', False), ('tabs', '', 'D', False), ('tabs', '', 'g', False), ('tabs', 'G', 'i', False), ('tabs', 'G', 'ı', False),
    ('tabs', 'G', 'x', False), ('tabs', '', 'z', False), ('tabs', '', '1', False), ('tabs', '', 'ç', False), ('tabs', 'G', 'Backspace', False),
    ('tabs', '', 'Backspace', False), ('tabs', 'G', 'Escape', False), ('tabs', '', 'Escape', False), ('controls', '', 'Escape', False),
    ('controls', 'G', 'Escape', False), ('tabs', '', 'Alt', False), ('tabs', '', 'Shift', False), ('tabs', '', 'Enter', False),
    ('tabs', '', 'ArrowDown', False), ('tabs', '', ' ', False), ('tabs', '', 'd', True), ('tabs', '', 'Tab', False),
]
BARS = [
    ('the fixed three only', QUICK_ACCESS, OFFERS + QUICK_ACCESS),
    ('two added, one of them an offer', QUICK_ACCESS + ['view.zoomExtents', 'tool.line'], OFFERS + QUICK_ACCESS + ['tool.line']),
    ('an offer this app does not have is not listed', QUICK_ACCESS + ['tool.pan'], [c for c in OFFERS if c != 'file.saveAs'] + QUICK_ACCESS),
]
TOGGLES = [
    ([], 'view.zoomExtents', True), (['view.zoomExtents'], 'tool.line', True), (['view.zoomExtents', 'tool.line'], 'view.zoomExtents', False),
    (['tool.line'], 'tool.line', True), (['tool.line'], 'tool.arc', False),
]
RIGHT_CLICKS = [
    ('file.save', QUICK_ACCESS), ('edit.redo', QUICK_ACCESS), ('view.zoomExtents', QUICK_ACCESS), ('view.zoomExtents', QUICK_ACCESS + ['view.zoomExtents']), ('tool.circle', QUICK_ACCESS + ['tool.line']),
]
CIRCLE = [
    {'command': 'tool.circle', 'title': 'Daire', 'label': 'Merkez, yarıçap'},
    {'command': 'tool.circle', 'option': '2N', 'title': 'Daire', 'label': '2 nokta', 'description': 'Çapın iki ucundan'},
    {'command': 'tool.circle', 'option': '3N', 'title': 'Daire', 'label': '3 nokta', 'description': 'Çember üzerindeki üç noktadan'},
]
FAMILY = [
    {'command': 'tool.rectangle', 'title': 'Dikdörtgen', 'label': 'Dikdörtgen'},
    {'command': 'tool.rectangle3', 'title': 'Döndürülmüş dikdörtgen', 'label': 'Döndürülmüş dikdörtgen'},
    {'command': 'tool.regularPolygon', 'title': 'Düzgün çokgen', 'label': 'Düzgün çokgen'},
]
FACES = [CIRCLE[0], CIRCLE[2], FAMILY[1], {'command': 'tool.hatch', 'title': 'Tarama…', 'label': 'Tarama…'}]


def fill(o):
    for k, v in o.items():
        if v is None:
            sample, fn = SAMPLES[k]
            o[k] = {'sample': sample, 'text': fn(*sample)}
    return o


def tabs_of(mode, selection):
    return [label for tid, label in TABS[mode] if tid != 'selection' or selection]


file = {
    'format': 'kentos.ribbon',
    'version': 1,
    'texts': fill(dict(TEXTS)),
    'quickAccessFixed': QUICK_ACCESS,
    'quickAccessOffers': OFFERS,
    'tabs': {mode: [{'id': tid, 'label': label, **({'contextual': True} if tid == 'selection' else {})} for tid, label in tabs] for mode, tabs in TABS.items()},
    'keyTips': {
        'letters': [{'label': l, 'letters': letters_of(l)} for l in LETTERS],
        'assign': [{'title': t, 'labels': labels, 'reserved': reserved, 'tips': assign_key_tips(labels, reserved)} for t, labels, reserved in ASSIGNS],
        'firstLevel': [{'mode': m, 'selection': sel, 'quickAccess': q, 'tabLabels': tabs_of(m, sel), 'tips': first_level(q, tabs_of(m, sel))} for q, m, sel in FIRST_LEVELS],
        'steps': [{'level': lv, 'typed': ty, 'key': k, **({'ctrl': True} if c else {}), 'tips': TIPS, 'step': key_step(lv, ty, TIPS, k, c)} for lv, ty, k, c in STEPS],
    },
    'quickAccessMenus': [{'title': t, 'bar': bar, 'exists': sorted(set(ex)), 'rows': quick_access_menu(bar, set(ex))} for t, bar, ex in BARS],
    'toggles': [{'kept': k, 'command': c, 'on': on, 'result': with_quick_access(k, c, on)} for k, c, on in TOGGLES],
    'commandMenus': [{'command': c, 'bar': bar, 'rows': command_menu(c, bar)} for c, bar in RIGHT_CLICKS],
    'ribbonMenu': [{'kind': 'command', 'command': 'view.ribbonCollapse'}],
    'splitMenus': [{'entries': e, 'menu': split_menu(e)} for e in [CIRCLE, FAMILY]],
    'splitFaces': [{'entry': e, 'face': split_face(e)} for e in FACES],
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
