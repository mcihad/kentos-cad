"""Independent update-publication cases (docs/adr/1320): which sheets an edit makes stale and which an update carries,
the sources each page reads and their content digests, and the 1:500/1:1000/1:5000 acceptance numbers. Never imports
KentOS or reads its output: a sheet is stale when an edited object's box meets its viewport's ground before or after
the edit (each viewport's ground is its frame times its scale about its centre), and an object's digest is SHA-256 of
its JSON with sorted keys, without its slot id."""
import copy, hashlib, json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/file.publish.sheets.update.json'
sys.path.insert(0, str(ROOT / 'scripts/fixtures'))
from sheets_edit_command_cases import sheet  # noqa: E402

E0, N0 = 500000.0, 4540000.0  # TUREF / TM30 (EPSG:5254): false easting 500 km, about 41° N.
COLS, ROWS, W, H = 20, 3, 20.0, 20.0
FRAME = dict(left=20.0, top=10.0, width=390.0, height=230.0)  # mm on the A3 page, the sheet core's map frame.

def layer(lid, name, color):
    return dict(id=lid, name=name, type='layer', visible=True, locked=False, expanded=True,
                style=dict(color=color, lineType='continuous', lineWeight=0.25), children=[])

def parcel(n):
    c, r = (n - 1) % COLS, (n - 1) // COLS
    x, y = E0 + c * W, N0 + r * H
    return dict(kind='polygon', id=n, layerId='parsel', attrs=dict(Ada='101', Parsel=str(n)),
                pts=[dict(x=x, y=y), dict(x=x + W, y=y), dict(x=x + W, y=y + H), dict(x=x, y=y + H)])

def n_of(col, row=0):
    return row * COLS + col + 1

MODEL = dict(format='kentos.document', version=1, name='Parseller',
             settings=dict(srid=5254, lengthDecimals=2, areaDecimals=2, areaUnit='m2', angleUnit='deg', plotScale=1000,
                           workspace='cad', drawingFont='barlow', drawingUnit='m'),
             origin=dict(x=E0, y=N0), layers=[layer('parsel', 'Parseller', '#1A1A1A')], activeLayer='parsel',
             entities=[parcel(n) for n in range(1, COLS * ROWS + 1)], styles=dict(items=[], categories=[]))

# The four sheets: west and east at 1:500, the west part at 1:1000, everything at 1:5000.
SHEETS = [('bati-500', 'Batı 1:500', 500, (E0 + 90.0, N0 + 30.0), 'P-1'),
          ('dogu-500', 'Doğu 1:500', 500, (E0 + 310.0, N0 + 30.0), 'P-2'),
          ('genel-1000', 'Genel 1:1000', 1000, (E0 + 100.0, N0 + 30.0), 'P-3'),
          ('butun-5000', 'Bütün 1:5000', 5000, (E0 + 200.0, N0 + 30.0), 'P-4')]
IDS = [s[0] for s in SHEETS]

def make_sheet(sid, name, scale, centre, number):
    s = sheet(sid, name, scale)
    s['items'][0]['kind']['view']['center'] = dict(x=centre[0], y=centre[1])
    s['items'].append(dict(id=sid + '-baslik', name='Başlık', frame=dict(left=30000, top=250000, width=360000, height=10000),
                           kind=dict(type='text', content='[% @pafta_no %] [% @pafta_adi %] Rev. [% @revizyon %] [% @tarih %]',
                                     style=dict(size=3500))))
    s['drawingInfo'] = dict(number=number, title=name, revision='A', status='', subset=[])
    return s

BOOK = dict(schema='kentos.sheet/1', sheets=[make_sheet(*s) for s in SHEETS], masters=[], assets=[], variables=[])

def ground(sid):
    _, _, scale, (cx, cy), _ = next(s for s in SHEETS if s[0] == sid)
    hw, hh = FRAME['width'] / 2 * scale / 1000, FRAME['height'] / 2 * scale / 1000
    return (cx - hw, cy - hh, cx + hw, cy + hh)

def box(e):
    xs, ys = [p['x'] for p in e['pts']], [p['y'] for p in e['pts']]
    return (min(xs), min(ys), max(xs), max(ys))

def meets(a, b):
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]

def shows(sid, e):
    return meets(box(e), ground(sid))

def digest(e):
    v = {k: x for k, x in e.items() if k != 'id'}
    return hashlib.sha256(json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()

def sources(entities, sid):
    return [e['id'] for e in entities if shows(sid, e)]

UID = lambda n: f'$uidOf:{n}'
NEW = '$new'

def moved(e, dx, dy):
    e = copy.deepcopy(e)
    e['pts'] = [dict(x=p['x'] + dx, y=p['y'] + dy) for p in e['pts']]
    return e

cases = []

def case(name, edits=(), *, model=None, layers_changed=False, revised=(), tamper=None, update=None, sheets=None,
         first=None, refuse=None):
    """`edits`: (command, input, after) with `after` the edited objects' new JSON by slot (None: deleted), or a new
    object under NEW. The expected states, changes and carried sheets follow from the boxes."""
    before = {e['id']: e for e in MODEL['entities']}
    after = copy.deepcopy(before)
    steps, added = [], []
    for command, inp, objects in edits:
        steps.append(dict(op='edit', command=command, input=inp))
        for k, v in objects.items():
            if k == NEW:
                added.append(v)
            elif v is None:
                after.pop(k)
            else:
                after[k] = v
    for sid, rev in revised:
        steps.append(dict(op='edit', command='cad.sheets.edit', input=dict(ops=[dict(
            op='setDrawingInfo', id=sid, info=dict(number=dict(SHEETS_INFO)[sid], title=dict(NAMES)[sid], revision=rev,
                                                   status='', subset=[]))])))
    revised_ids = [s for s, _ in revised]
    if refuse:
        cases.append(dict(name=name, first=first or {}, edits=steps, tamper=tamper or {},
                          update=dict(input=update or {}, result=dict(status=refuse[0], error=dict(code=refuse[1])))))
        return
    rows, stale = {}, []
    for sid in IDS:
        was = [n for n in sources(before.values(), sid)]
        now = [n for n in sources(after.values(), sid)]
        new_here = [i for i, e in enumerate(added) if shows(sid, e)]
        changed = [n for n in now if n in was and digest(after[n]) != digest(before[n])]
        drawn = [n for n in changed if after[n]['pts'] != before[n]['pts']]
        removed = [n for n in was if n not in now]
        came = [n for n in now if n not in was]
        c = dict(changed=[UID(n) for n in changed], added=[UID(n) for n in came] + [NEW] * len(new_here),
                 removed=[UID(n) for n in removed], sheet=sid in revised_ids, parts=['layers'] if layers_changed else [])
        is_stale = bool(drawn or removed or came or new_here or layers_changed or sid in revised_ids)
        state = 'stale' if is_stale else 'current'
        if tamper and sid in tamper.get('files', {}):
            state = 'fileChanged'
        rows[sid] = dict(state=state, changes=c)
        if state != 'current':
            stale.append(sid)
    chosen = sheets or IDS
    carried = [s for s in chosen if rows[s]['state'] == 'current' and not (update or {}).get('_none')]
    upd = {k: v for k, v in (update or {}).items() if not k.startswith('_')}
    regenerated = [s for s in chosen if s not in carried]
    # A changed page under its unchanged revision is said so; a page made again only because its file changed is not.
    notes = [s for s in regenerated if rows[s]['state'] == 'stale' and s not in revised_ids]
    cases.append(dict(name=name, first=first or {}, edits=steps, tamper=tamper or {},
                      status=dict(stale=stale, sheets=rows),
                      update=dict(input=upd, sheets=chosen, carried=carried, regenerated=regenerated,
                                  notes=notes, result=dict(status='completed'))))

SHEETS_INFO = [(s[0], s[4]) for s in SHEETS]
NAMES = [(s[0], s[1]) for s in SHEETS]
P = {e['id']: e for e in MODEL['entities']}
T = lambda n, dx, dy: ('cad.entities.transform', dict(uids=[UID(n)], transform=dict(kind='move', dx=dx, dy=dy)), {n: moved(P[n], dx, dy)})

case('nothing changed: every page carried')
case('an east parcel moved: only the pages showing it are made again', [T(n_of(17, 1), 1.0, 0.0)])
case('a west parcel moved: its three pages made again under an unchanged revision', [T(n_of(3), 0.0, 1.0)])
case('a middle parcel seen at 1:500 east, 1:1000 and 1:5000', [T(n_of(12, 2), -0.5, 0.0)])
case('a parcel half in the west frame', [T(n_of(9), 0.25, 0.0)])
case('a parcel moved out of every frame is removed from its pages', [T(n_of(17), 0.0, 1000.0)])
case('a parcel moved from west to east is removed there and added here', [T(n_of(2, 1), 300.0, 0.0)])
attrs = copy.deepcopy(P[n_of(5)]); attrs['attrs']['Malik'] = 'Hazine'
case('an attribute alone changes no page: its object is listed, the pages carried',
     [('cad.entities.set', dict(uids=[UID(n_of(5))], attrs=dict(Malik='Hazine'), operation='attributes'), {n_of(5): attrs})])
case('a deleted east parcel is removed from its pages', [('cad.entities.delete', dict(uids=[UID(n_of(18))]), {n_of(18): None})])
new = dict(kind='polygon', layerId='parsel', attrs={}, pts=[dict(x=E0 + 300.0, y=N0 + 70.0), dict(x=E0 + 310.0, y=N0 + 70.0),
                                                            dict(x=E0 + 310.0, y=N0 + 80.0), dict(x=E0 + 300.0, y=N0 + 80.0)])
case('a new east parcel is added to its pages',
     [('cad.entities.create', dict(layerId='parsel', objects=[dict(geometry=dict(kind='polygon', pts=new['pts']))]), {NEW: new})])
case('a layer colour changes every page and names the layers',
     [('cad.layers.edit', dict(operation='style', layer='parsel', color='#0055AA'), {})], layers_changed=True)
case('an advanced revision makes its sheet again with no note', revised=[('bati-500', 'B')])
case('a moved parcel with one revision advanced', [T(n_of(3), 0.0, 1.0)], revised=[('bati-500', 'B')])
case('a delivered file altered on disk is made again, the rest carried', tamper=dict(files={'bati-500': 'alter'}))
case('a delivered file removed is made again', tamper=dict(files={'dogu-500': 'remove'}))
case('a renamed pattern carries the same bytes under new names', [T(n_of(17, 1), 1.0, 0.0)], update=dict(naming='{id}'))
case('an update of two sheets in a chosen order', [T(n_of(17, 1), 1.0, 0.0)], sheets=['dogu-500', 'bati-500'],
     update=dict(sheets=['dogu-500', 'bati-500']))
case('another resolution carries nothing', update=dict(dpi=150, _none=True))
case('a delivery without geo-reference carries nothing into a geo-referenced one', first=dict(geo=False), update=dict(_none=True))
case('the plan of an update says what it carries and writes nothing', [T(n_of(17, 1), 1.0, 0.0)], update=dict(_plan=True))
case('an update is not a single PDF', update=dict(mode='single'), refuse=('failed', 'invalid_input'))
case('an update is not resumable', update=dict(resumable=True), refuse=('failed', 'invalid_input'))
case('the earlier delivery is an absolute folder', update=dict(previous='teslim'), refuse=('failed', 'invalid_file_path'))
case('a folder without a publication is not an earlier delivery', update=dict(previous='$empty'), refuse=('failed', 'no_previous'))
case('an update never overwrites the earlier delivery', update=dict(path='$first'), refuse=('failed', 'invalid_input'))

# The plan step writes nothing: its expected `carried` is what execute would carry.
for c in cases:
    if c.get('update', {}).get('input', {}).pop('_plan', None):
        c['update']['op'] = 'plan'

DIGESTS = {str(e['id']): digest(e) for e in MODEL['entities']}
SOURCES = {sid: [UID(n) for n in sources(MODEL['entities'], sid)] for sid in IDS}
# Acceptance: each page's viewport ground (TM30 metres) and a parcel edge's paper length (mm) at its scale.
ACCEPTANCE = dict(frame=FRAME, sheets=[dict(sheet=sid, scale=scale, ground=list(ground(sid)), edge=W * 1000 / scale)
                                       for sid, _, scale, _, _ in SHEETS])
fixture = dict(format='kentos.command-cases', version=1, command='file.publish.sheets', commandVersion=1,
               host='headless-file', runner='publish-update', title='Update publication from an earlier delivery',
               note='docs/adr/1320. Each case publishes the book into $first, applies its edits, asks file.publish.status of $first '
                    'and publishes an update with previous=$first into $second. Stale sheets, page sources, object digests and '
                    'carried sheets are computed here from boxes and JSON, never read from KentOS. $uidOf:N is slot N\'s '
                    'persistent id; $new an object the case creates.',
               setup=MODEL, book=BOOK, digests=DIGESTS, sources=SOURCES, acceptance=ACCEPTANCE, cases=cases)
text = json.dumps(fixture, ensure_ascii=False, indent=1) + '\n'
if '--check' in sys.argv:
    assert OUT.read_text() == text, 'file.publish.sheets.update.json is out of date: run scripts/fixtures/sheet_update_cases.py'
else:
    OUT.write_text(text)
print(len(cases), 'cases')
