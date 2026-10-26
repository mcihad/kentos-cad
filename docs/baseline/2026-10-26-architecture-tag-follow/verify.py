"""Check real GUI saves with the standalone KCAD reader, no command/geometry code."""
import copy
from decimal import Decimal as D
import hashlib
import importlib.util
import json
from pathlib import Path

B = Path(__file__).resolve().parent
R = B.parents[2]
spec = importlib.util.spec_from_file_location('reader', R/'tools/kcad/kcad.py')
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


def read(name):
    return reader.read((B/name).read_bytes())[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def entity(model, uid):
    return model['entities'][model['uids'].index(uid)]


def area(points):
    return abs(sum(D(str(a['x']))*D(str(b['y']))-D(str(b['x']))*D(str(a['y']))
                   for a, b in zip(points, points[1:]+points[:1])))/2


wall = 'd564c949-7c64-5016-9938-e78a8229cda0'
room = '01a17545-a419-75c4-b0da-5b7c30b39b76'
tag = '01a17545-a41f-716d-bdad-9be63b2ce826'
table = '01a17545-a424-7558-829b-d2f3d950acfc'
before = read('before.kcad')
assert len(before['entities']) == len(set(before['uids'])) == 13
assert entity(before, wall)['width'] == .2
assert abs(area(entity(before, room)['parts'][0]['outer']['pts'])-D('22.04')) < D('1e-10')
assert entity(before, tag)['at'] == dict(x=-2, y=2)

# The pointer target was calibrated against the visible 6 by 4 metre wall,
# independently of the saved model. Subpixel placement has a 3 cm GUI bound.
moved = read('dragged.kcad')
at = entity(moved, tag)['at']
assert abs(at['x']+1) < .03 and abs(at['y']-5) < .03
dragged = copy.deepcopy(before)
entity(dragged, tag)['at'] = copy.deepcopy(at)
assert moved == dragged, 'Dragging must change only Tag.at, nothing else'

thick = copy.deepcopy(dragged)
entity(thick, wall)['width'] = .4
entity(thick, room)['parts'] = [dict(outer=dict(pts=[dict(x=x, y=y)
    for x, y in [(.2, .2), (5.8, .2), (5.8, 3.8), (.2, 3.8)]]))]
net = (D(6)-D('.4'))*(D(4)-D('.4'))
perimeter = 2*((D(6)-D('.4'))+(D(4)-D('.4')))
assert net == D('20.16') and perimeter == D('18.4')
entity(thick, tag)['text'] = f'U-099\nSalon\n{net:.2f} m²'
entity(thick, tag)['anchor'] = dict(x=3, y=2)
entity(thick, table)['cells'][1][4:6] = [f'{net:.2f}', f'{perimeter:.3f}']


def remove(model, uid):
    i = model['uids'].index(uid)
    model['uids'].pop(i)
    model['entities'].pop(i)


room_deleted = copy.deepcopy(thick)
remove(room_deleted, room)
entity(room_deleted, tag)['broken'] = 'targetGone'
schedule = entity(room_deleted, table)
schedule['cells'].pop(1)
schedule['rows'].pop(1)
schedule['source']['members'].pop(0)
for style in schedule['styles']:
    style['rows'] = 3
wall_deleted = copy.deepcopy(thick)
remove(wall_deleted, wall)
entity(wall_deleted, room)['broken'] = 'lostEnclosure'
template = copy.deepcopy(thick)
entity(template, tag)['content'] = '{number}\n{name}\nNet {area}'
entity(template, tag)['text'] = 'U-099\nSalon\nNet 20.16 m²'

expectations = {
    'failed-drag.kcad': before, 'dragged.kcad': dragged,
    'drag-undo.kcad': before, 'drag-redo.kcad': dragged,
    'thickened.kcad': thick, 'thick-undo.kcad': dragged, 'thick-redo.kcad': thick,
    'room-deleted.kcad': room_deleted, 'room-delete-undo.kcad': thick,
    'wall-deleted.kcad': wall_deleted, 'wall-delete-undo.kcad': thick,
    'template-edited.kcad': template, 'template-undo.kcad': thick,
    'template-redo.kcad': template, 'compact-preview-save.kcad': template,
    'compact-cancel-save.kcad': template, 'compact-escape-save.kcad': template,
    'reopened-save.kcad': template,
}
records = json.loads((B/'models.json').read_text())
assert set(expectations) == {r['file'] for r in records}
for record in records:
    name = record['file']
    actual, expected = read(name), copy.deepcopy(expectations[name])
    assert sha(B/name) == record['sha256'], name
    if name in ('room-deleted.kcad', 'wall-deleted.kcad'):
        # The file reader numbers surviving slots densely. UUIDs, sequence,
        # links and every other model field must remain exactly as expected.
        for model in (actual, expected):
            for e in model['entities']:
                e.pop('id')
    if expectations[name] not in (before, dragged):
        # Only recomputed room contour and tag anchor/position permit FP64
        # rounding; no tolerance for any other model data or source identity.
        if room in actual['uids']:
            ap = entity(actual, room)['parts'][0]['outer']['pts']
            ep = entity(expected, room)['parts'][0]['outer']['pts']
            assert len(ap) == len(ep) == 4
            for a, e in zip(ap, ep):
                assert a.keys() == e.keys()
                for axis in ('x', 'y'):
                    assert abs(a[axis]-e[axis]) < 1e-9, (name, a, e)
                    e[axis] = a[axis]
            assert abs(area(ap)-net) < D('1e-9'), name
        for key in ('at', 'anchor'):
            a, e = entity(actual, tag)[key], entity(expected, tag)[key]
            assert a.keys() == e.keys()
            for axis in ('x', 'y'):
                assert abs(a[axis]-e[axis]) < 1e-10, (name, key, a, e)
                e[axis] = a[axis]
    assert actual == expected, name

# Undo/redo, cancellation, and a fresh process load must be exact, including
# binary model data; the tolerance above must not hide changes on these paths.
for a, b in [('failed-drag', 'drag-undo'), ('dragged', 'drag-redo'),
             ('dragged', 'thick-undo'), ('thickened', 'thick-redo'),
             ('thickened', 'room-delete-undo'), ('thickened', 'wall-delete-undo'),
             ('thickened', 'template-undo'), ('template-edited', 'template-redo'),
             ('template-edited', 'compact-preview-save'),
             ('template-edited', 'compact-cancel-save'),
             ('template-edited', 'compact-escape-save'), ('template-edited', 'reopened-save')]:
    assert read(a+'.kcad') == read(b+'.kcad'), (a, b)
events = json.loads((B/'gui-events.json').read_text())
for e in events:
    assert sha(B/e['file']) == e['sha256'], e['file']
    build = json.loads((B/e['build']).read_text())
    assert sha(Path(build['app'])/build['executable']) == build['sha256']
build = json.loads((B/'after-binary.json').read_text())
for file, hash_value in build['sourceFiles'].items():
    assert sha(R/file) == hash_value, file
for run in json.loads((B/'test-runs.json').read_text()):
    assert sha(B/run['log']) == run['sha256'], run['name']
for name, theme, size in [('dark13-profile.json', 'dark', 13), ('light18-profile.json', 'light', 18)]:
    profile = json.loads((B/name).read_text())['user']
    assert profile['appearance.textSize'] == size and profile['appearance.theme'] == theme
print(f'PASS: {len(records)} real disk snapshots; independently expected tag placement, '
      '20.16 m² room/tag/table, source deletion and warnings, template, exact undo/redo, '
      f'preview/cancel/reopen; {len(events)} raw screenshot hashes, binaries, source and logs.')
