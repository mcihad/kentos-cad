#!/usr/bin/env python3
"""Independent full-size CAD opening cases: decimal SI factors, no KentOS imports."""
import json
from decimal import Decimal as D
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/formats/v1/cad-open.json'

# The drawing scale an opened drawing takes (docs/adr/0626 §5): the middle of its texts' heights over 2.5 mm on
# paper, the nearest of the usual scales by ratio; none, the unit's own (a part 1:1, a room in cm 1:50, a building
# in metres 1:100).
SCALES = [D(n) for n in ['1', '2', '5', '10', '20', '25', '50', '100', '200', '250', '500', '1000', '2000', '2500',
                         '5000', '10000', '25000', '50000']]
UNIT_SCALE = {'mm': D(1), 'cm': D(50), 'm': D(100), 'in': D(1), 'ft': D(1)}

def opened_scale(heights_m, unit):
    if not heights_m:
        return UNIT_SCALE[unit]
    h = sorted(heights_m)[(len(heights_m) - 1) // 2]
    raw = h / D('0.0025')
    if raw <= 1:
        return D(1)
    # By ratio: the smaller of raw/s and s/raw, so no logarithm is needed (a tie keeps the smaller scale).
    def far(s):
        return raw / s if raw >= s else s / raw
    return min(SCALES, key=far)

def dxf(code, length, scale, texts=()):
    tags = [(0,'SECTION'),(2,'HEADER'),(9,'$ACADVER'),(1,'AC1015')]
    if code is not None: tags += [(9,'$INSUNITS'),(70,code)]
    tags += [(0,'ENDSEC'),(0,'SECTION'),(2,'TABLES'),(0,'TABLE'),(2,'LAYER'),
             (0,'LAYER'),(2,'0'),(70,0),(62,7),(6,'CONTINUOUS'),(0,'ENDTAB'),
             (0,'TABLE'),(2,'BLOCK_RECORD'),(0,'BLOCK_RECORD'),(2,'Yatak'),(70,4),
             (0,'ENDTAB'),(0,'ENDSEC'),(0,'SECTION'),(2,'BLOCKS'),(0,'BLOCK'),
             (8,'0'),(2,'Yatak'),(70,0),(10,0),(20,0),(3,'Yatak'),(0,'LINE'),
             (8,'0'),(10,0),(20,0),(11,2100),(21,0),(0,'ENDBLK'),(0,'ENDSEC'),
             (0,'SECTION'),(2,'ENTITIES'),(0,'LINE'),(8,'0'),(10,0),(20,0),(11,length),(21,0),
             (0,'INSERT'),(8,'0'),(2,'Yatak'),(10,0),(20,0),(41,scale),(42,scale)]
    for i, (kind, height) in enumerate(texts):
        if kind == 'TEXT':
            tags += [(0,'TEXT'),(8,'0'),(10,i*100),(20,500),(40,height),(1,f'Yazı {i}')]
        else:
            tags += [(0,'MTEXT'),(8,'0'),(10,i*100),(20,700),(40,height),(1,f'Not {i}')]
    tags += [(0,'ENDSEC'),(0,'EOF')]
    return ''.join(f'{k}\n{v}\n' for k,v in tags)

cases = []
for code, unit, factor in [(4,'mm',D('.001')),(5,'cm',D('.01')),(6,'m',D(1)),
                           (1,'in',D('.0254')),(2,'ft',D('.3048'))]:
    for length in ['10','3500','.125','1000000']:
        # Existing placement resolves to the same 2.1 m bed in every source unit.
        scale = D('.001') / factor
        cases.append(dict(name=f'{unit}-{length}', dxf=dxf(code,length,scale), unit=unit,
                          length=float(D(length)*factor), blockLength=2.1, insertScale=1.0, assumed=False,
                          scale=float(opened_scale([], unit))))
for code in [None, 0, 999]:
    cases.append(dict(name=f'undeclared-{code}', dxf=dxf(code,'1000',1), unit='mm', length=1.,
                      blockLength=2.1, insertScale=1., assumed=True, scale=1.0))
cases.append(dict(name='misdeclared-original-is-preserved', dxf=dxf(4,'1000','.1'), unit='mm', length=1.,
                  blockLength=2.1, insertScale=.1, assumed=False, scale=1.0))
# The drawing's texts say its scale: their middle height, not one odd text (docs/adr/0626 §5).
for code, unit, factor, texts in [
    (4, 'mm', D('.001'), [('TEXT', '250'), ('TEXT', '250'), ('TEXT', '5000')]),
    (4, 'mm', D('.001'), [('TEXT', '2.5'), ('MTEXT', '2.5')]),
    (4, 'mm', D('.001'), [('MTEXT', '125'), ('TEXT', '125'), ('TEXT', '1')]),
    (5, 'cm', D('.01'), [('TEXT', '12.5')]),
    (6, 'm', D(1), [('TEXT', '2.5'), ('TEXT', '2'), ('TEXT', '3')]),
    (6, 'm', D(1), [('MTEXT', '.3')]),
    (1, 'in', D('.0254'), [('TEXT', '.125')]),
    (2, 'ft', D('.3048'), [('TEXT', '2')]),
]:
    heights = [D(h) * factor for _, h in texts]
    expected = opened_scale(heights, unit)
    scale = D('.001') / factor
    cases.append(dict(name=f'{unit}-yazilar-{"-".join(h for _, h in texts)}', dxf=dxf(code, '1000', scale, texts),
                      unit=unit, length=float(D(1000) * factor), blockLength=2.1, insertScale=1.0, assumed=False,
                      scale=float(expected)))
text=json.dumps(dict(format='kentos.cad-open', cases=cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:
    assert OUT.read_text() == text, f'{OUT} is stale'
else:
    OUT.write_text(text)
print(f'CAD açılış başvurusu: {len(cases)} durum')
