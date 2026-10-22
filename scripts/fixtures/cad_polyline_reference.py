"""Independent ezdxf reader acceptance for the standard planar CAD-IO-15 scope.
Run with an installed KentOS Python build and ezdxf (test dependencies only).
Creates temporary files; no golden output is regenerated. Not an AutoCAD visual test.
"""
import json
from pathlib import Path
import tempfile
import ezdxf
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
def pairs(*p):return ''.join(f'{k}\n{v}\n' for k,v in p)
def source(kind,nz,closed):
    vertices=[(0,0,.2,.4,.5),(3,0,.3,.3,0),(3,4,.1,.2,0)]
    normal=pairs((210,0),(220,0),(230,nz))
    if kind=='LWPOLYLINE':
        body=pairs((0,kind),(90,3),(70,int(closed)),(38,7))+normal
        for x,y,a,b,bulge in vertices:body+=pairs((10,x),(20,y),(40,a),(41,b),(42,bulge))
    else:
        body=pairs((0,kind),(66,1),(70,int(closed)),(10,0),(20,0),(30,7))+normal
        for x,y,a,b,bulge in vertices:body+=pairs((0,'VERTEX'),(10,x),(20,y),(30,0),(40,a),(41,b),(42,bulge))
        body+=pairs((0,'SEQEND'))
    return pairs((0,'SECTION'),(2,'HEADER'),(9,'$ACADVER'),(1,'AC1032'),(9,'$INSUNITS'),(70,6),(0,'ENDSEC'),(0,'SECTION'),(2,'ENTITIES'))+body+pairs((0,'ENDSEC'),(0,'EOF'))
def main():
    setup=json.loads((ROOT/'fixtures/commands/v1/file.import.dxf.json').read_text())['setup']
    count=0
    with tempfile.TemporaryDirectory() as folder:
        folder=Path(folder)
        for kind in ['LWPOLYLINE','POLYLINE']:
            for nz in [1,-1]:
                for closed in [False,True]:
                    doc=cad.Document.from_bytes(json.dumps(setup).encode())
                    src=folder/'foreign.dxf';src.write_text(source(kind,nz,closed))
                    cad.file.import_.dxf(doc,path=str(src),closed_polylines=cad.ClosedPolylines.LINE)
                    out=folder/'out.dxf'
                    result=doc.run('file.export.dxf',{'path':str(out),'overwrite':True,'annotations':{'metadata':False}})
                    assert result['status']=='completed',result
                    independent=ezdxf.readfile(out)
                    paths=list(independent.modelspace().query('LWPOLYLINE'))
                    assert len(paths)==1
                    p=paths[0]
                    # Strip private payload before checking the standard fields.
                    p.discard_xdata("KENTOS")
                    assert bool(p.closed)==closed
                    assert p.dxf.elevation==7*nz
                    points=list(p.get_points('xyseb'))
                    expected=[(0,0,.2,.4,.5*nz),(3*nz,0,.3,.3,0),(3*nz,4,.1,.2,0)]
                    if not closed:expected[-1]=(3*nz,4,0,0,0)
                    assert points==expected,(kind,nz,closed,points,expected)
                    assert not p.has_xdata('KENTOS'), 'must stand on standard DXF records'
                    count+=1
    print(f'ezdxf {ezdxf.__version__}: {count} independent width/bulge/closure/elevation/extrusion checks passed')
if __name__=='__main__':main()
