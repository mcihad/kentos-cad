"""Independent source rectangles, linked GUI objects via public typed Python SDK."""
from pathlib import Path
import copy,json,shutil
import kentos.cad as cad
B=Path(__file__).resolve().parent;R=B.parents[2]
s=copy.deepcopy(json.loads((R/'fixtures/commands/v1/cad.spaces.number.json').read_text())['setup'])
s['name']='Mahal numaralandırma — etiket ve çizelge kabulü';s['entities']=[]
for i,(x,y)in enumerate([(0.,0.),(7.,0.),(0.,5.),(7.,5.)],1):
    s['entities'].append(dict(kind='wall',id=i,layerId='duvar',attrs={},closed=True,width=.2,
        arch=dict(level='zemin'if i<4 else'bir'),pts=[dict(x=x,y=y),dict(x=x+6,y=y),dict(x=x+6,y=y+4),dict(x=x,y=y+4)]))
(B/'empty-source.json').write_text(json.dumps(s,ensure_ascii=False,indent=2)+'\n')
doc=cad.Document.open(B/'empty-source.json')
def run(name,args):
    c=cad.command(name);return c.execute_input(doc,c.input_type.from_json(args))
rooms=[];walls=[r.uid for r in doc.entities()]
for i,(x,y)in enumerate([(0.,0.),(7.,0.),(0.,5.),(7.,5.)]):
    level='zemin'if i<3 else'bir'
    room=run('cad.spaces.create',dict(layerId='duvar',spaces=[dict(seed=dict(x=x+3,y=y+2),number=f'OLD-{i+1}'if i<3 else'Z-002',name=['Salon','Mutfak','Yatak odası','Çalışma'][i],usage='Konut',arch=dict(level=level))]))
    rooms.extend(room.uids)
    run('cad.tags.create',dict(layerId='duvar',tags=[dict(of=room.uids[0],at=dict(x=-2.,y=2.)if i==0 else dict(x=x+3,y=y+2.5),content='{number}\n{name}\n{area}',leader=i==0)]))
run('cad.schedules.create',dict(layerId='duvar',at=dict(x=15.,y=10.),of='spaces'))
doc.save(B/'before.kcad')
(B/'ids.json').write_text(json.dumps(dict(rooms=rooms,walls=walls),indent=2)+'\n')
work=R/'.run/architecture-space-number/gui.kcad';assert not work.exists(),'Do not overwrite an active GUI model';shutil.copy2(B/'before.kcad',work)
print('Prepared 4 source walls,4 linked rooms,4 tags including moved first tag,1 live schedule; 22.04m² each')
