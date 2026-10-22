"""Independent DXF group streams for CAD-IO layout/viewport/metadata tests. No KentOS imports."""
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/formats/v1/cad-io/layouts.dxf'
def pairs(*p): return ''.join(f'{k}\n{v}\n' for k,v in p)
def section(name, body): return pairs((0,'SECTION'),(2,name))+body+pairs((0,'ENDSEC'))
def layout(name, handle, owner, width, height, order):
    return pairs((0,'LAYOUT'),(5,handle),(100,'AcDbPlotSettings'),(44,width),(45,height),(72,1),
        (100,'AcDbLayout'),(1,name),(71,order),(330,owner))
def build():
    header=section('HEADER',pairs((9,'$ACADVER'),(1,'AC1021'),(9,'$INSUNITS'),(70,6)))
    table=section('TABLES',pairs((0,'TABLE'),(2,'BLOCK_RECORD'),(70,1),(0,'BLOCK_RECORD'),(5,'2B'),(2,'*Paper_Space0'),(0,'ENDTAB')))
    blocks=section('BLOCKS',pairs((0,'BLOCK'),(2,'*Paper_Space0'),(70,0),(10,0),(20,0),
        (0,'LINE'),(5,'201'),(8,'0'),(10,10),(20,20),(11,30),(21,40),(0,'ENDBLK')))
    entities=section('ENTITIES',pairs((0,'LINE'),(5,'101'),(8,'0'),(10,0),(20,0),(11,100),(21,0),
        (0,'LINE'),(5,'102'),(8,'0'),(67,1),(410,'Plan A'),(10,10),(20,20),(11,30),(21,40),
        (0,'TEXT'),(5,'103'),(8,'0'),(67,1),(410,'Plan A'),(10,15),(20,50),(40,3.5),(1,'PAFTA A'),(50,30),
        (0,'VIEWPORT'),(5,'104'),(67,1),(410,'Plan A'),(8,'0'),(10,100),(20,80),(40,100),(41,80),
        (68,1),(69,2),(12,5),(22,10),(17,100),(27,200),(45,200),(51,90),(16,0),(26,0),(36,1)))
    objects=section('OBJECTS',layout('Plan A','1E','1B',420,297,1)+layout('Plan B','2E','2B',210,297,2))
    return (header+table+blocks+entities+objects+pairs((0,'EOF'))).encode()
if __name__=='__main__':
    data=build()
    if '--check' in sys.argv:
        if OUT.read_bytes()!=data:raise SystemExit('CAD-IO fixture drift')
    else: OUT.write_bytes(data)
