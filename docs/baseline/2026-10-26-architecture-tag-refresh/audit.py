"""Check old cases, command identities, generated contracts and untouched saved drawings."""
from pathlib import Path
import hashlib,json,subprocess
B=Path(__file__).resolve().parent;R=B.parents[2]
def old(path):return subprocess.check_output(['git','show','30e8a7dd5:'+path],cwd=R)
def sha(data):return hashlib.sha256(data).hexdigest()
fp='fixtures/commands/v1/cad.tags.create.json'
a=json.loads(old(fp));b=json.loads((R/fp).read_text());lookup={v['name']:v for v in b['cases']}
assert len(lookup)==len(b['cases'])
assert all(lookup[v['name']]==v for v in a['cases'])
other=['cad.tags.edit','cad.tags.check','cad.schedules.create','cad.schedules.edit']
for name in other:
    p='fixtures/commands/v1/'+name+'.json';assert (R/p).read_bytes()==old(p),p
cp='apps/web/src/contracts/generated/commandCatalog.json'
a_catalog=json.loads(old(cp));b_catalog=json.loads((R/cp).read_text())
ca={v['id']:v for v in a_catalog['commands']};cb={v['id']:v for v in b_catalog['commands']}
assert set(ca)==set(cb);changed=[k for k in ca if ca[k]!=cb[k]];assert changed==['cad.tags.create'],changed
for name in ['TagAll','TagsCreated','TagsPlan']:
    path=name+'.ts';actual=(R/'apps/web/src/contracts/generated'/path).read_text()
    generated=(R/'.run/architecture-contours/contracts-ts'/path).read_text()
    assert actual.splitlines()==[line.rstrip() for line in generated.splitlines()],path
    assert 'refresh' in actual,path
files=subprocess.check_output(['git','ls-tree','-r','--name-only','30e8a7dd5'],cwd=R,text=True).splitlines()
old_drawings=[p for p in files if p.endswith('.kcad')]
# One binary git diff against the recorded parent detects any tracked file mutation.
for p in old_drawings:
    assert (R/p).read_bytes()==old(p),p
out=dict(parentCommit='30e8a7dd5',oldCreateCases=len(a['cases']),newCreateCases=len(b['cases'])-len(a['cases']),otherCommandFixturesUnchanged=other,commandIds=len(ca),changedCommands=changed,existingTsContracts=['TagAll','TagsCreated','TagsPlan'],oldKcadFilesByteIdentical=len(old_drawings),oldCreateSha256=sha(old(fp)))
(B/'audit.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
