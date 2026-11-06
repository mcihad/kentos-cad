"""Update-publication fixture runner (docs/adr/1320): the typed SDK over file.publish.sheets.update.json, hashlib for the
carried files. The Rust runner is crates/native/headless/tests/headless/sheet_update.rs."""
import copy, hashlib, json, re, tempfile
from pathlib import Path
import kentos.cad as cad

EARLIER = '2026-10-01'


def run_fixture(fx):
    assert fx['command'] == 'file.publish.sheets' and fx.get('runner') == 'publish-update' and len(fx['cases']) >= 20
    publish, status = cad.command('file.publish.sheets'), cad.command('file.publish.status')
    problems = []
    for case in fx['cases']:
        try:
            with tempfile.TemporaryDirectory(prefix='kentos-update-python-') as folder:
                run_case(fx, case, Path(folder), publish, status)
        except AssertionError as e:
            problems.append(f"{case['name']}: {e}")
    assert not problems, '\n'.join(problems)
    return len(fx['cases'])


def run_case(fx, case, root, publish, status):
    first, second, empty = root / 'first', root / 'second', root / 'empty'
    empty.mkdir()
    doc = cad.Document.from_bytes(json.dumps(fx['setup']).encode())
    cad.sheets.edit(doc, ops=[cad.AddSheetPaperOp(sheet=cad.PaperSheet.from_json(s)) for s in fx['book']['sheets']])
    slots = lambda: {r.entity.id: r.uid for r in doc.entities(page_size=10_000)}
    uid = slots()
    fill = lambda v: re.sub(r'\$uidOf:(\d+)', lambda m: uid[int(m.group(1))], v) if isinstance(v, str) else (
        [fill(x) for x in v] if isinstance(v, list) else {k: fill(x) for k, x in v.items()} if isinstance(v, dict) else v)
    run = lambda w, inp, op='execute': w.run(doc, w.input_type.from_json(inp), op=op)
    r = run(publish, dict(path=str(first), mode='separate', **case['first']))
    assert r.status == 'completed', r.error
    m = json.loads((first / 'yayin.json').read_text())
    for page in m['sources']['pages']:
        assert page['objects'] == fill(fx['sources'][page['sheet']]), page['sheet']
    for slot, digest in fx['digests'].items():
        assert m['sources']['objects'][uid[int(slot)]] == digest, slot
    m['date'] = EARLIER
    for page in m['pages']:
        page['date'] = EARLIER
    (first / 'yayin.json').write_text(json.dumps(m))
    file_of = lambda man, s: next(f for f in man['files'] if f['sheets'] == [s])
    for sheet, how in case['tamper'].get('files', {}).items():
        at = first / file_of(m, sheet)['name']
        if how == 'remove':
            at.unlink()
        else:
            at.write_bytes(at.read_bytes() + b'\n% eklendi\n')
    issued = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in first.iterdir()}
    made = []
    for step in case['edits']:
        before = set(slots().values())
        w = cad.command(step['command'])
        r = run(w, fill(step['input']))
        assert r.status == 'completed', (step['command'], r.error)
        made += [u for u in slots().values() if u not in before]
    back = {u: f'$uidOf:{s}' for s, u in uid.items()}
    name = lambda u: '$new' if u in made else back.get(u, f'$unknown:{u}')
    if 'status' in case:
        o = run(status, dict(path=str(first)), 'plan').output.to_json()
        assert o['stale'] == case['status']['stale'], o['stale']
        for row in o['sheets']:
            want = case['status']['sheets'][row['sheet']]
            assert row['state'] == want['state'], (row['sheet'], row['state'])
            got = dict(row['changes'])
            for k in ('changed', 'added', 'removed'):
                got[k] = [name(u) for u in got[k]]
            assert got == want['changes'], (row['sheet'], got)
    u = case['update']
    inp = dict(path=str(second), mode='separate', previous=str(first))
    for k, v in u['input'].items():
        inp[k] = str(first) if v == '$first' else str(empty) if v == '$empty' else v
    r = run(publish, inp, u.get('op', 'execute'))
    assert r.status == u['result']['status'], (r.status, r.error)
    assert {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in first.iterdir()} == issued
    if r.status != 'completed':
        assert r.error.code == u['result']['error']['code'], r.error.code
        assert not second.exists()
        return
    o = r.output.to_json()
    assert o['sheets'] == u['sheets'] and o.get('carried', []) == u['carried'], o.get('carried')
    assert [i['sheet'] for i in o['issues'] if i['code'] == 'revision_unchanged'] == u['notes']
    if u.get('op') == 'plan':
        assert not second.exists()
        return
    now, was = json.loads((second / 'yayin.json').read_text()), json.loads((first / 'yayin.json').read_text())
    for sheet in u['carried']:
        f = file_of(now, sheet)
        assert f['sha256'] == file_of(was, sheet)['sha256'] == hashlib.sha256((second / f['name']).read_bytes()).hexdigest()
        assert next(p for p in now['pages'] if p['sheet'] == sheet)['date'] == EARLIER
    assert run(status, dict(path=str(second)), 'plan').output.to_json()['stale'] == []
