#!/usr/bin/env python3
"""Independent full-size CAD opening cases: decimal SI factors, no KentOS imports."""
import json
from decimal import Decimal as D
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/formats/v1/cad-open.json'

def dxf(code, length, scale):
    tags = [(0,'SECTION'),(2,'HEADER'),(9,'$ACADVER'),(1,'AC1015')]
    if code is not None: tags += [(9,'$INSUNITS'),(70,code)]
    tags += [(0,'ENDSEC'),(0,'SECTION'),(2,'TABLES'),(0,'TABLE'),(2,'LAYER'),
             (0,'LAYER'),(2,'0'),(70,0),(62,7),(6,'CONTINUOUS'),(0,'ENDTAB'),
             (0,'TABLE'),(2,'BLOCK_RECORD'),(0,'BLOCK_RECORD'),(2,'Yatak'),(70,4),
             (0,'ENDTAB'),(0,'ENDSEC'),(0,'SECTION'),(2,'BLOCKS'),(0,'BLOCK'),
             (8,'0'),(2,'Yatak'),(70,0),(10,0),(20,0),(3,'Yatak'),(0,'LINE'),
             (8,'0'),(10,0),(20,0),(11,2100),(21,0),(0,'ENDBLK'),(0,'ENDSEC'),
             (0,'SECTION'),(2,'ENTITIES'),(0,'LINE'),(8,'0'),(10,0),(20,0),(11,length),(21,0),
             (0,'INSERT'),(8,'0'),(2,'Yatak'),(10,0),(20,0),(41,scale),(42,scale),
             (0,'ENDSEC'),(0,'EOF')]
    return ''.join(f'{k}\n{v}\n' for k,v in tags)

cases = []
for code, unit, factor in [(4,'mm',D('.001')),(5,'cm',D('.01')),(6,'m',D(1)),
                           (1,'in',D('.0254')),(2,'ft',D('.3048'))]:
    for length in ['10','3500','.125','1000000']:
        # Existing placement resolves to the same 2.1 m bed in every source unit.
        scale = D('.001') / factor
        cases.append(dict(name=f'{unit}-{length}', dxf=dxf(code,length,scale), unit=unit,
                          length=float(D(length)*factor), blockLength=2.1, insertScale=1.0, assumed=False))
for code in [None, 0, 999]:
    cases.append(dict(name=f'undeclared-{code}', dxf=dxf(code,'1000',1), unit='mm', length=1.,
                      blockLength=2.1, insertScale=1., assumed=True))
cases.append(dict(name='misdeclared-original-is-preserved', dxf=dxf(4,'1000','.1'), unit='mm', length=1.,
                  blockLength=2.1, insertScale=.1, assumed=False))
text=json.dumps(dict(format='kentos.cad-open', cases=cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:
    assert OUT.read_text() == text, f'{OUT} is stale'
else:
    OUT.write_text(text)
print(f'CAD açılış başvurusu: {len(cases)} durum')
