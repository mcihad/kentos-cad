#!/usr/bin/env python3
"""Contract cases for bulk room discovery; no KentOS imports or captured answers.

Two 4.8×7.8 m rooms are independently implied by a 10×8 m perimeter,
200 mm walls and an x=5 partition. Ring start/order and label seeds are
deliberately unspecified here; independent physical-size assertions also
run in spaces_discover.rs, the Python suite and the real-GUI Decimal verifier.
"""
import argparse
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/cad.spaces.discover.json'

def point(x, y):
    return dict(x=x, y=y)

def build():
    # Existing setup metadata only; no expected command result is imported.
    setup = json.loads((ROOT / 'fixtures/commands/v1/cad.walls.rectangle.json').read_text())['setup']
    setup['entities'] = [
        dict(kind='wall', id=1, layerId='base', attrs={}, width=.2, closed=True,
             arch=dict(level='zemin'), pts=[point(0, 0), point(10, 0), point(10, 8), point(0, 8)]),
        dict(kind='wall', id=2, layerId='base', attrs={}, width=.2,
             arch=dict(level='zemin'), pts=[point(5, 0), point(5, 8)]),
    ]
    original = {'ids': [1, 2], 'canUndo': False, 'revision': 'same'}
    inp = dict(layerId='duvar', level='zemin')

    def result(output, warnings=None):
        return dict(status='completed', output=output, warnings=warnings or [])

    def proposal(method='net', existing=False, level='zemin'):
        space = dict(parts='$any', seed='$any')
        if method != 'net':
            space['method'] = method
        if level is not None:
            space['arch'] = dict(level=level)
        return dict(space=space, existing=['$any'] if existing else [], conflicts=[])

    cases = []
    for unit in ['mm', 'cm', 'm', 'in', 'ft']:
        for method in ['net', 'centerline', 'gross']:
            s = copy.deepcopy(setup)
            s['settings']['drawingUnit'] = unit
            args = dict(inp, method=method)
            plan = dict(proposals=[proposal(method), proposal(method)], revision='$current')
            repeat = dict(proposals=[proposal(method, True), proposal(method, True)], revision='$current')
            cases.append(dict(name=f'{unit}/{method}: two rooms, readonly plan, repeat, one undo', setup=s, steps=[
                dict(op='validate', input=args, result=result(None), expect=original),
                dict(op='plan', input=args, result=result(plan), expect=original),
                dict(op='execute', input=args, result=result(dict(uids=['$any', '$any'], ids=[3, 4], revision='$current')),
                     expect=dict(ids=[1, 2, 3, 4], canUndo=True, revision='changed')),
                dict(op='plan', input=args, result=result(repeat), expect=dict(ids=[1, 2, 3, 4], canUndo=True)),
                dict(op='execute', input=args, result=result(dict(uids=[], ids=[], revision='$current')),
                     expect=dict(ids=[1, 2, 3, 4], canUndo=True)),
                dict(op='undo', returns='Mahal', expect=dict(ids=[1, 2], canUndo=False, canRedo=True)),
                dict(op='redo', returns='Mahal', expect=dict(ids=[1, 2, 3, 4], canUndo=True, canRedo=False)),
            ]))
    for bounds, count in [([point(6, 3), point(9, 5)], 1), ([point(20, 20), point(21, 21)], 0)]:
        args = dict(inp, bounds=bounds)
        cases.append(dict(name=f'Region representative filter: {count} whole rooms', steps=[
            dict(op='plan', input=args, result=result(dict(proposals=[proposal()] * count, revision='$current')), expect=original)]))
    empty = copy.deepcopy(setup)
    empty['entities'] = []
    cases.append(dict(name='No walls: empty readonly plan and no empty undo', setup=empty, steps=[
        dict(op='plan', input=inp, result=result(dict(proposals=[], revision='$current')), expect=dict(ids=[], canUndo=False)),
        dict(op='execute', input=inp, result=result(dict(uids=[], ids=[], revision='$current')), expect=dict(ids=[], canUndo=False, revision='same'))]))
    for scope in ['level', 'legacy']:
        s = copy.deepcopy(setup)
        s['entities'] = [dict(kind='wall', id=1, layerId='base', attrs={}, width=.2,
                              pts=[point(0, 0), point(0, 8), point(10, 8), point(10, 0)])]
        if scope == 'level':
            s['entities'][0]['arch'] = dict(level='zemin')
        args = inp if scope == 'level' else dict(layerId='duvar')
        cases.append(dict(name=f'Physical open U is not closed: {scope}', setup=s, steps=[
            dict(op='plan', input=args, result=result(dict(proposals=[], revision='$current')), expect=dict(ids=[1], canUndo=False))]))
    for field, value, code, path in [
        ('level', 'missing', 'level_not_found', 'level'),
        ('layerId', 'missing', 'layer_not_found', 'layerId'),
        ('layerId', 'kilitli', 'layer_locked', 'layerId'),
        ('layerId', 'grup', 'not_a_layer', 'layerId'),
        ('expectedRevision', 'bad', 'invalid_revision', 'expectedRevision'),
        ('expectedRevision', '999', 'revision_conflict', 'expectedRevision'),
        ('bounds', [point(0, 0), point(0, 8)], 'invalid_space', 'bounds'),
    ]:
        error = dict(code=code, path=path, message='$any')
        status = 'conflict' if code == 'revision_conflict' else 'failed'
        if status == 'conflict':
            error['revision'] = '$current'
        cases.append(dict(name=f'Atomic refusal: {field}={value}', steps=[dict(
            op='execute', input=dict(inp, **{field: value}), result=dict(status=status, error=error), expect=original)]))
    for path, nonfinite in [('bounds[0].x', 'NaN'), ('bounds[1].y', 'Infinity')]:
        cases.append(dict(name=f'Nonfinite region {path}', steps=[dict(
            op='plan', input=dict(inp, bounds=[point(0, 0), point(10, 8)]), nonFinite={path: nonfinite},
            result=dict(status='failed', error=dict(code='not_finite', path=path, message='$any')), expect=original)]))
    for kind, code, path in [
        ('width', 'invalid_wall', 'walls[1].width'),
        ('vertex', 'invalid_wall', 'walls[1].pts[1]'),
        ('required', 'attribute_required', 'attrs.Ad'),
        ('typed', 'invalid_attribute', 'attrs.Ad'),
    ]:
        s = copy.deepcopy(setup)
        args = dict(inp)
        if kind == 'width':
            s['entities'][0]['width'] = -.2
        elif kind == 'vertex':
            s['entities'][0]['pts'][1] = point(0, 0)
        else:
            layer = next(row for row in s['layers'] if row['id'] == 'duvar')
            layer['fields'] = [dict(name='Ad', kind='text', required=True) if kind == 'required'
                               else dict(name='Ad', kind='integer')]
            args['attrs'] = dict(Ad=' ' if kind == 'required' else 'abc')
        cases.append(dict(name=f'Atomic source/attribute refusal: {kind}', setup=s, steps=[dict(
            op='execute', input=args, result=dict(status='failed', error=dict(code=code, path=path, message='$any')),
            expect=original)]))
    hidden = dict(inp, layerId='gizli')
    cases.append(dict(name='Hidden destination is explicit warning', steps=[dict(
        op='plan', input=hidden, result=result(dict(proposals=[proposal(), proposal()], revision='$current'),
        [dict(code='layer_hidden', path='layerId', message='“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.')]), expect=original)]))
    return dict(format='kentos.command-cases', version=1, command='cad.spaces.discover', commandVersion=1,
                title='Independent bulk room command contract', setup=setup, cases=cases)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    value = build()
    text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            raise SystemExit('Bulk room fixtures are stale')
    else:
        OUT.write_text(text)
    print(f'cad.spaces.discover: {len(value["cases"])} independent command cases')

if __name__ == '__main__':
    main()
