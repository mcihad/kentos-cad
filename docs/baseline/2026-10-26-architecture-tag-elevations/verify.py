"""Independent expectations for native GUI saves; no KentOS command/render calls."""
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


def read(name):
    return reader.read((B/name).read_bytes())[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


before = read('before.kcad')
assert len(before['entities']) == len(set(before['uids'])) == 14
source = '01a17545-a421-7088-b518-9cce50dfaaa3'
content = '{number} {name}\n{area}\nZemin {floorElevation}\nTavan {ceilingElevation}'
created = copy.deepcopy(before)
actual = read('created.kcad')
assert actual['uids'][:14] == before['uids']
assert len(actual['uids']) == 15 and actual['uids'][-1] not in before['uids']
created['uids'].append(actual['uids'][-1])
created['entities'].append(dict(kind='tag', id=15, layerId='duvar', attrs={},
    of=source, at=dict(x=18., y=5.), anchor=dict(x=10., y=7.), size=3.5,
    shape='rectangle', leader=True, content=content,
    text='Z-002 Çalışma\n22.04 m²\nZemin 3.000\nTavan'))
height = copy.deepcopy(created)
height['entities'][height['uids'].index(source)]['height'] = 2.75
height['entities'][-1]['text'] += ' 5.750'
table = next(e for e in height['entities'] if e['kind'] == 'table')
assert table['cells'][4][0] == 'Z-002' and table['cells'][4][6] == '3.000'
table['cells'][4][6] = '2.750'
dark_height = copy.deepcopy(height)
dark_height['entities'][dark_height['uids'].index(source)]['height'] = 3.125
dark_height['entities'][-1]['text'] = height['entities'][-1]['text'].replace('5.750', '6.125')
next(e for e in dark_height['entities'] if e['kind'] == 'table')['cells'][4][6] = '3.125'
expected = {
    'settings-only.kcad': before,
    'created.kcad': created,
    'height-unchanged.kcad': created,
    'height.kcad': height,
    'undo.kcad': created,
    'redo.kcad': height,
    'invalid.kcad': height,
    'dark-height.kcad': dark_height,
    'cancel.kcad': height,
    'reopened.kcad': height,
}
records = json.loads((B/'models.json').read_text())
assert set(expected) == {r['file'] for r in records}
for record in records:
    assert sha(B/record['file']) == record['sha256']
    assert read(record['file']) == expected[record['file']], record['file']
events = json.loads((B/'gui-events.json').read_text())
for event in events:
    assert sha(B/event['file']) == event['sha256']
    build = json.loads((B/event['build']).read_text())
    assert sha(Path(build['app'])/build['executable']) == build['sha256']
for name in ['after-binary.json', 'after3-binary.json']:
    for file, digest in json.loads((B/name).read_text())['sourceFiles'].items():
        assert sha(R/file) == digest, file
for record in json.loads((B/'test-runs.json').read_text()):
    assert sha(B/record['log']) == record['sha256']
for name, theme, size in [('light18-profile.json', 'light', 18), ('dark13-profile.json', 'dark', 13)]:
    profile = json.loads((B/name).read_text())['user']
    assert profile['appearance.theme'] == theme and profile['appearance.textSize'] == size
print(f'PASS: {len(records)} exact GUI saves, all old objects/settings/UUIDs, '
      f'{len(events)} raw screenshots, immutable binaries, final source and log hashes.')
