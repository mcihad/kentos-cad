import json
from pathlib import Path
from kentos import cad
root=Path.cwd()
setup=json.loads((root/'fixtures/commands/v1/cad.spaces.separators.json').read_text())['setup']
setup['entities']=[]
doc=cad.Document.from_bytes(json.dumps(setup).encode())
made=cad.spaces.separators(doc,operation=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=[dict(closed=True,pts=[dict(x=0,y=0),dict(x=4,y=0),dict(x=4,y=3),dict(x=0,y=3)])])))
room=cad.spaces.create(doc,layer_id='mahal',spaces=[cad.NewSpace.from_json(dict(seed=dict(x=1,y=1)))])
before=doc.to_bytes()
cad.entities.delete(doc,uids=made.uids)
e=doc.entity(room.uids[0]).entity.to_json()
q=cad.spaces.quantities(doc).to_json()
result=dict(room=e,quantities=q)
(root/'.run/architecture-space-separators/quantity-break-audit.json').write_text(json.dumps(result,indent=2)+'\n')
doc.undo();assert doc.to_bytes()==before
print(json.dumps(dict(roomBreak=e.get('broken'),quantityBreak=q['spaces'][0].get('boundaryBreak'),area=q['spaces'][0]['area'],rule=q['spaces'][0]['rule']),indent=2))
