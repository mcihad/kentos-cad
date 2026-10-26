"""Independent expectations for saved GUI actions; no KentOS command/geometry code."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path

B = Path(__file__).resolve().parent
R = B.parents[2]
spec = importlib.util.spec_from_file_location('reader', R/'tools/kcad/kcad.py')
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


def read(file):
    return reader.read((B/file).read_bytes())[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def entity(model, uid):
    return model['entities'][model['uids'].index(uid)]


before = read('before.kcad')
room = '01a17545-a419-75c4-b0da-5b7c30b39b76'
assert len(before['entities']) == len(set(before['uids'])) == 13
created = read('created.kcad')
assert len(created['entities']) == len(set(created['uids'])) == 14
assert created['uids'][:-1] == before['uids']
assert created['uids'][-1] not in before['uids']
expected = copy.deepcopy(before)
expected['uids'].append(created['uids'][-1])
expected['entities'].append(dict(
    kind='tag', id=14, layerId='duvar', attrs={}, of=room, at=dict(x=16, y=2),
    content='Mahal {number}\n{name}\nNet {area}', shape='rounded', size=3.5,
    leader=True, anchor=dict(x=3, y=2), text='Mahal U-099\nSalon\nNet 20.16 m²',
))
assert created == expected, 'Only the explicitly configured new semantic tag may be added'
renamed = copy.deepcopy(expected)
entity(renamed, room)['name'] = 'Oturma'
for e in renamed['entities']:
    if e['kind'] == 'tag' and e['of'] == room:
        # The old tag uses the same name token, without the new "Mahal" prefix.
        e['text'] = e['text'].replace('\nSalon\n', '\nOturma\n')
    if e['kind'] == 'table':
        assert e['cells'][1][1] == 'Salon'
        e['cells'][1][1] = 'Oturma'
expectations = {
    'settings-only.kcad': before,
    'invalid-template.kcad': before,
    'invalid-size.kcad': before,
    'preview-save.kcad': before,
    'created.kcad': expected,
    'create-undo.kcad': before,
    'create-redo.kcad': expected,
    'source-renamed.kcad': renamed,
    'rename-undo.kcad': expected,
    'cancelled.kcad': expected,
    'reopened.kcad': expected,
}
records = json.loads((B/'models.json').read_text())
assert set(expectations) == {r['file'] for r in records}
for record in records:
    assert sha(B/record['file']) == record['sha256']
    assert read(record['file']) == expectations[record['file']], record['file']
events = json.loads((B/'gui-events.json').read_text())
for e in events:
    assert sha(B/e['file']) == e['sha256'], e['file']
    build = json.loads((B/e['build']).read_text())
    assert sha(Path(build['app'])/build['executable']) == build['sha256']
build = json.loads((B/'after2-binary.json').read_text())
for file, hash_value in build['sourceFiles'].items():
    assert sha(R/file) == hash_value, file
for run in json.loads((B/'test-runs.json').read_text()):
    assert sha(B/run['log']) == run['sha256'], run['name']
for name, theme, size in [('light18-profile.json', 'light', 18), ('dark13-profile.json', 'dark', 13)]:
    profile = json.loads((B/name).read_text())['user']
    assert profile['appearance.textSize'] == size and profile['appearance.theme'] == theme
print(f'PASS: {len(records)} actual GUI saves, exact complete models/UUIDs/settings; '
      'one configured 3.5mm tag at (16,2), source/name/tag/table follow, single undo/redo, '
      f'preview/cancel/fresh reopen; {len(events)} raw image hashes, binaries, final sources and logs.')
