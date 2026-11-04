"""Independently check the frozen-glyph PDF against C HarfBuzz's oracle.

No KentOS geometry/PDF module is imported. FontTools reads the actual embedded
subset outlines, pypdf reads CID/Unicode streams, and the reference GIDs/clusters
come from fixtures/text/v1/shaping.json (external hb-shape, not this writer).
Development-only requirements: fonttools and pypdf; no runtime dependency.

  KENTOS_TEXT_PDF_OUT=/tmp/text-pdf cargo test -p kentos-sheet --test all frozen_glyph_pdf
  .run/py/bin/python scripts/fixtures/text_pdf_check.py /tmp/text-pdf/frozen-glyph-oracle.pdf

The destination directory must already exist. This proves exact glyph/source
pairs for 29 faces; it does not certify PDF/A/UA or every viewer's copy behavior.
"""
from pathlib import Path
import io,json,re,sys,hashlib
from pypdf import PdfReader
from pypdf.generic import IndirectObject,DictionaryObject,NameObject
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import DecomposingRecordingPen
R=Path(__file__).resolve().parents[2]
def contour(font,gid):
 gs=font.getGlyphSet();pen=DecomposingRecordingPen(gs);gs[font.getGlyphOrder()[gid]].draw(pen);return repr(pen.value)
manifest=json.loads((R/'crates/shared/geometry-core/data/text-fonts/manifest.json').read_text())
oracle=json.loads((R/'fixtures/text/v1/shaping.json').read_text())
expected={};names={}
for meta in manifest['profileFaces']:
 raw=(R/meta['file']).read_bytes();assert hashlib.sha256(raw).hexdigest()==meta['sha256']
 font=TTFont(io.BytesIO(raw));name=font['name'].getDebugName(6)
 case=next(c for c in oracle['cases'] if (c['family'],c['weight'],c['italic'])==(meta['family'],meta['weight'],meta['italic']))
 source=case['text'].encode();clusters=sorted({g['cl'] for g in case['glyphs']}|{len(source)});ends=dict(zip(clusters,clusters[1:]));seen=set();pairs=set()
 for g in case['glyphs']:
  text=source[g['cl']:ends[g['cl']]].decode() if g['cl'] not in seen else ''
  seen.add(g['cl']);pairs.add((contour(font,g['g']),text))
 expected[meta['index']]=pairs;names.setdefault(name,[]).append(meta['index'])
r=PdfReader(sys.argv[1]);matched=set();count=0
for generation,objects in r.xref.items():
 for n in objects:
  if not n:continue
  o=IndirectObject(n,generation,r).get_object()
  if not isinstance(o,DictionaryObject) or o.get('/Subtype')!='/Type0':continue
  cid=o['/DescendantFonts'][0].get_object();mapping=cid['/CIDToGIDMap']
  if isinstance(mapping,NameObject):continue
  name=str(o['/BaseFont']).split('+',1)[-1];sf=TTFont(io.BytesIO(cid['/FontDescriptor']['/FontFile2'].get_data()));raw=mapping.get_data();cmap=o['/ToUnicode'].get_data()
  blocks=re.findall(rb'\d+ beginbfchar\s+(.*?)endbfchar',cmap,re.S)
  codes={int(a,16):bytes.fromhex(b.decode()).decode('utf-16-be') for block in blocks for a,b in re.findall(rb'<([0-9A-Fa-f]{4})> <([0-9A-Fa-f]*)>',block)}
  assert len(codes)==len(raw)//2-1,(name,'each CID needs a mapping, including empty continuation')
  got=set()
  for code,source in codes.items():
   gid=int.from_bytes(raw[code*2:code*2+2],'big');assert gid>0;got.add((contour(sf,gid),source));count+=1
  candidates=[i for i in names[name] if expected[i]==got]
  assert len(candidates)==1,(name,'actual glyph/source pairs differ from external C HarfBuzz',candidates)
  matched.add(candidates[0])
assert len(matched)==29,matched
print('Exact C HarfBuzz glyph/source pairs match for all 29 frozen faces;',count,'CID codes verified.')
