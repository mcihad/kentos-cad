#!/usr/bin/env python3
"""Independent immutable-source policy, hash guards and deterministic context expectations."""
import argparse,copy,json,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'fixtures/sheet/v1/source-snapshot.json'
def make():
 base=json.loads((ROOT/'fixtures/sheet/v1/delivery.json').read_text());book=copy.deepcopy(base['book']);book['sheets']=book['sheets'][:2]
 for s in book['sheets']:
  s['items']=s['items'][:2];s['items'][0]['frame']=dict(left=20000,top=20000,width=20000,height=20000);s['items'][0]['kind']['viewLocked']=True;s['items'][1]['kind']['content']='YAKALANAN-İĞÜŞ';s['items'][1]['frame']=dict(left=20000,top=45000,width=100000,height=10000);s['export']['dpi']=96
 a='interaction/v1/logo.png';b='raster/v1/files/rgb.png'
 ah=hashlib.sha256((ROOT/'fixtures'/a).read_bytes()).hexdigest();bh=hashlib.sha256((ROOT/'fixtures'/b).read_bytes()).hexdigest()
 cases=[]
 def add(name,**kw):cases.append(dict(name=name,**kw))
 for edit in ['line','title','metadata','layers','variables']:
  add('captured-'+edit,edit=edit,expectOld=True)
 for unit in ['mm','cm','m']:add('image-in-'+unit,unit=unit,image=True,expectImageSha=ah)
 add('image-changed-before-first-use',image=True,change='before',expectError=True)
 add('image-changed-after-first-use',image=True,change='after',expectImageSha=ah)
 add('image-deleted-before-first-use',image=True,change='delete-before',expectError=True)
 add('image-deleted-after-first-use',image=True,change='delete-after',expectImageSha=ah)
 add('missing-then-created',missing=True,change='appear',expectError=True)
 add('same-file-aliases',image=True,aliases=True,expectImageSha=ah,expectImages=1)
 add('two-sources-one-name',image=True,second=True,expectImages=2,expectShas=sorted([ah,bh]))
 add('fixed-author',image=True,author='Ölçme Ekibi',expectImageSha=ah)
 add('fixed-date',image=True,date='2020-02-29',expectImageSha=ah)
 add('dpi-is-output-only',dpi=150,expectOld=True)
 add('context-author-guard',guard='author',expectMismatch=True)
 add('context-date-guard',guard='date',expectMismatch=True)
 add('resource-version-guard',guard='image',image=True,expectMismatch=True)
 add('document-only-guard',guard='document-only',image=True,expectMismatch=False)
 add('document-content-guard',guard='document',expectMismatch=True)
 return dict(version=1,reference='Policy cases written without KentOS; image SHA-256 values calculated with hashlib. Capture precedes edits; changed unavailable first-use images refuse, already-read bytes stay immutable.',model=base['model'],book=book,imageA=a,imageB=b,shaA=ah,shaB=bh,cases=cases)
def main():
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');args=p.parse_args();data=json.dumps(make(),ensure_ascii=False,indent=2)+'\n'
 if args.check:assert OUT.read_text()==data,'snapshot fixture stale'
 else:OUT.write_text(data)
if __name__=='__main__':main()
