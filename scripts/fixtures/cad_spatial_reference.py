#!/usr/bin/env python3
"""Check the Decimal oracle against ezdxf's own XYZ/INSERT implementation.

--exports DIR additionally checks KentOS standard DXF output without KentOS XDATA.
Test-only dependency: ezdxf. No KentOS runtime imports.
"""
import argparse
import json
import math
from pathlib import Path

import ezdxf

ROOT = Path(__file__).resolve().parents[2]


def vertices(entity):
    kind = entity.dxftype()
    if kind == 'LINE': return [entity.dxf.start, entity.dxf.end]
    if kind == 'POINT': return [entity.dxf.location]
    if kind == 'LWPOLYLINE': return list(entity.vertices_in_wcs())
    if kind == 'POLYLINE': return list(entity.points_in_wcs())
    if kind in ('SOLID', 'TRACE'):
        return [entity.ocs().to_wcs(entity.dxf.get(f'vtx{i}')) for i in (0, 1, 3, 2)]
    if kind == '3DFACE': return [entity.dxf.get(f'vtx{i}') for i in range(4)]
    raise AssertionError(f'Unexpected primitive: {kind}')


def walk(entities, parents=()):
    for entity in entities:
        if entity.dxftype() == 'INSERT':
            inserts = entity.multi_insert() if entity.mcount > 1 else [entity]
            for insert in inserts:
                yield from walk(insert.block(), (insert.matrix44(), *parents))
        else:
            points = vertices(entity)
            for matrix in parents:
                points = list(matrix.transform_vertices(points))
            yield points


def expected_vertices(entity):
    if entity['kind'] == 'line':
        return [(entity[p]['x'], entity[p]['y'], entity[z]) for p,z in (('a','za'),('b','zb'))]
    if entity['kind'] == 'point': return [(entity['p']['x'],entity['p']['y'],entity['z'])]
    return [(p['x'],p['y'],z) for p,z in zip(entity['pts'],entity['zs'])]


def check(path, expected):
    doc = ezdxf.readfile(path)
    # Deliberately remove our metadata: only standard DXF coordinates count.
    for entity in doc.entitydb.values():
        if entity.xdata:
            for appid in list(entity.xdata.data): entity.discard_xdata(appid)
    factor = {4: .001, 6: 1}[doc.header['$INSUNITS']]
    actual = list(walk(doc.modelspace()))
    assert len(actual) == len(expected), (path, len(actual),len(expected))
    count = 0
    for index,(points,wanted) in enumerate(zip(actual,expected)):
        wanted = expected_vertices(wanted)
        assert len(points) == len(wanted), (path,index,len(points),len(wanted))
        for p,q in zip(points,wanted):
            for a,b in zip(p,q):
                a *= factor
                assert math.isfinite(a) and abs(a-b) <= abs(b)*32*math.ulp(1.)+1e-12, (path,index,a,b)
            count += 1
    return len(actual),count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exports', type=Path)
    args = parser.parse_args()
    cases = json.loads((ROOT/'fixtures/formats/v1/cad-spatial/expected.json').read_text())['cases']
    for label,paths in [('sources',[ROOT/'fixtures'/c['source'] for c in cases]),
                        *([('exports',[args.exports/f"{c['name']}.dxf" for c in cases])] if args.exports else [])]:
        objects = points = 0
        for path,case in zip(paths,cases):
            n,p = check(path,case['expected'])
            objects += n
            points += p
        print(f'ezdxf {ezdxf.__version__}: {label}: {len(cases)} files, {objects} entities, {points} XYZ vertices passed')


if __name__ == '__main__': main()
