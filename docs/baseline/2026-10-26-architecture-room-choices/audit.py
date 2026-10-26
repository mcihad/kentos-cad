"""Independently check contract scope and old fixture bytes against the parent."""
import hashlib,json,subprocess
from pathlib import Path
B=Path(__file__).resolve().parent;R=B.parents[2];PARENT='ac5c5bf7d'
def old(path):return subprocess.check_output(['git','show',f'{PARENT}:{path}'],cwd=R)
def sha(data):return hashlib.sha256(data).hexdigest()
path='apps/web/src/contracts/generated/commandCatalog.json'
a=json.loads(old(path));b=json.loads((R/path).read_bytes())
assert [c['id']for c in a['commands']]==[c['id']for c in b['commands']]
changed=[x['id']for x,y in zip(a['commands'],b['commands'])if x!=y]
assert changed==['project.changes','project.create','cad.settings.set'],changed
command_count=len(b['commands'])
def previous_schema(v):
 if isinstance(v,list):return [previous_schema(x)for x in v]
 if isinstance(v,dict):
  result={k:previous_schema(x)for k,x in v.items()if k!='description'}
  if isinstance(result.get('properties'),dict):
   for key in ['spaceNames','spaceUsages','spaceNumberPrefixes']:result['properties'].pop(key,None)
  return result
 return v
# Existing project create/change schemas also embed architectural settings.
normalized=previous_schema(b)
settings_old=next(c for c in a['commands']if c['id']=='cad.settings.set')
settings_new=next(c for c in normalized['commands']if c['id']=='cad.settings.set')
assert 'spaceNames'in settings_new['summary'] and 'spaceNumberPrefixes'in settings_new['summary']
settings_new['summary']=settings_old['summary']
assert previous_schema(a)==normalized
path='fixtures/commands/v1/cad.settings.set.json'
a=json.loads(old(path));b=json.loads((R/path).read_bytes())
assert len(a['cases'])==62 and b['cases'][:62]==a['cases']
assert len(b['cases'])==91
same=0
paths=subprocess.check_output(['git','ls-tree','-r','--name-only',PARENT,'fixtures/kcad/v2'],cwd=R,text=True).splitlines()
for path in paths:
 if not path.endswith('.kcad') or path.endswith('broken/schema-version-107.kcad'):continue
 assert (R/path).read_bytes()==old(path),path
 same+=1
new_ts=subprocess.check_output(['git','ls-files','--others','--exclude-standard','apps/web'],cwd=R,text=True).splitlines()
assert not [p for p in new_ts if p.endswith(('.ts','.tsx'))],new_ts
result=dict(parentCommit=PARENT,commands=command_count,changedCommands=changed,oldSettingsCasesUnchanged=62,newSettingsCases=29,oldKcadBytesUnchanged=same,newTsTsxFiles=[],catalogSha256=sha((R/'apps/web/src/contracts/generated/commandCatalog.json').read_bytes()))
(B/'contract-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
