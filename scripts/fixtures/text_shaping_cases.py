"""Independent HarfBuzz/FriBidi oracle for CAD-ADV-10's opt-in glyph layout.

Requires the tools named in the fixture's versions; never calls KentOS.
--check compares generated reference data without changing the repository.
These cases exercise established Latin/Arabic/Hebrew characters; they do not
claim full Unicode-version conformance (the tools' data versions differ).
"""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/text/v1/shaping.json'


def run(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs).stdout


def line_order(text, start, end, base, with_levels=False):
    """FriBidi C paragraph levels + actual wrapped-line L1/L2, no KentOS."""
    library = ctypes.util.find_library('fribidi')
    if library is None:
        directory = Path(run(['pkg-config', '--variable=libdir', 'fribidi']).strip())
        library = next((str(directory / name) for name in ['libfribidi.dylib', 'libfribidi.so']
                        if (directory / name).is_file()), None)
    if library is None: raise RuntimeError('FriBidi C kütüphanesi bulunamadı.')
    api = ctypes.CDLL(library)
    u32, i32, level = ctypes.c_uint32, ctypes.c_int, ctypes.c_int8
    pointer = ctypes.POINTER
    api.fribidi_get_bidi_type.argtypes = [u32]
    api.fribidi_get_bidi_type.restype = u32
    api.fribidi_get_bidi_types.argtypes = [pointer(u32), i32, pointer(u32)]
    api.fribidi_get_bracket_types.argtypes = [pointer(u32), i32, pointer(u32), pointer(u32)]
    api.fribidi_get_par_embedding_levels_ex.argtypes = [pointer(u32), pointer(u32), i32, pointer(u32), pointer(level)]
    api.fribidi_get_par_embedding_levels_ex.restype = level
    api.fribidi_reorder_line.argtypes = [u32, pointer(u32), i32, i32, u32, pointer(level), pointer(u32), pointer(i32)]
    api.fribidi_reorder_line.restype = level
    n = len(text)
    chars = (u32 * n)(*map(ord, text))
    types, brackets, levels = (u32 * n)(), (u32 * n)(), (level * n)()
    mapping = (i32 * n)(*range(n))
    direction = u32(api.fribidi_get_bidi_type(ord({'auto': '!', 'ltr': 'A', 'rtl': 'א'}[base])))
    api.fribidi_get_bidi_types(chars, n, types)
    api.fribidi_get_bracket_types(chars, n, types, brackets)
    assert api.fribidi_get_par_embedding_levels_ex(types, brackets, n, ctypes.byref(direction), levels)
    assert api.fribidi_reorder_line(0, types, end - start, start, direction, levels, chars, mapping)
    order = list(mapping)[start:end]
    return (order, list(levels)) if with_levels else order


def build():
    from fontTools.ttLib import TTFont
    source = (ROOT / 'crates/shared/geometry-core/src/text/font_data.rs').read_text()
    files = re.findall(r'include_bytes!\("([^"]+)"\)', source)
    keys = re.findall(r'\("([^"]+)", (\d+), (true|false)\)', source)
    assert len(files) == len(keys) == 29
    cases = []
    for relative, (family, weight, italic) in zip(files, keys):
        path = (ROOT / 'crates/shared/geometry-core/src/text' / relative).resolve()
        text = ('العربية' if family == 'noto-sans-arabic' else
                'שָׁלוֹם' if family == 'noto-sans-hebrew' else
                '⌀ ≤ ≥ ± ×' if family == 'noto-sans-math' else 'AV — Türkçe Ög\u0306ç')
        direction = 'rtl' if family in ['noto-sans-arabic', 'noto-sans-hebrew'] else 'ltr'
        glyphs = json.loads(run(['hb-shape', str(path), text, '--no-glyph-names', '--output-format=json',
                                 '--utf8-clusters', '--cluster-level=0', '--shapers=ot', '--language=und',
                                 '--features=kern=1,liga=1,clig=1', '--direction=' + direction]))
        assert all(g['g'] != 0 for g in glyphs), (family, text, glyphs)
        cases.append(dict(name=path.name, family=family, weight=int(weight), italic=italic == 'true',
                          text=text, direction=direction, unitsPerEm=TTFont(path)['head'].unitsPerEm,
                          fontSha256=hashlib.sha256(path.read_bytes()).hexdigest(), glyphs=glyphs))
    bidi = []
    for text in ['ABC אבג 123', 'Türkçe / العربية / עברית 123', '(אבג) ABC 12', 'אבג 12 ABC',
                 'A [אבג] B', 'A\u2067אבג 123\u2069 B']:
        for base in ['auto', 'ltr', 'rtl']:
            args = ['fribidi', '--vtol', '--novisual', '--nopad', '--nobreak']
            if base != 'auto': args.append('--' + base)
            positions = [int(p) for p in run(args, input=text + '\n').split()]
            assert sorted(positions) == list(range(len(text)))
            bidi.append(dict(text=text, direction=base, visualToLogical=positions))
    wrapped = []
    for text, start, end in [('אבג ABC 12 (XYZ) סוף', 4, 10),
                             ('אבג ABC 12 (XYZ) סוף', 8, 17),
                             ('A אבג 12 XYZ Z', 6, 12),
                             ('A \u2067אבג 123\u2069 B', 4, 10)]:
        for base in ['auto', 'ltr', 'rtl']:
            wrapped.append(dict(text=text, start=start, end=end, direction=base,
                                visualToLogical=line_order(text, start, end, base)))
    styled = []
    for text, family, rtl, split in [('سلام', 'noto-sans-arabic', True, 2),
                                     ('AVATAR', 'arimo', False, 2)]:
        spans = []
        for start, end, weight in [(0, split, 400), (split, len(text), 600)]:
            face = next(c for c in cases if c['family'] == family and c['weight'] == weight and not c['italic'])
            path = next((ROOT / 'crates/shared/geometry-core/src/text' / f).resolve()
                        for f in files if Path(f).name == face['name'])
            glyphs = json.loads(run(['hb-shape', str(path), text[start:end], '--no-glyph-names',
                '--output-format=json', '--utf8-clusters', '--cluster-level=0', '--shapers=ot',
                '--language=und', '--features=kern=1,liga=1,clig=1', '--direction=' + ('rtl' if rtl else 'ltr'),
                '--text-before=' + text[:start], '--text-after=' + text[end:]]))
            spans.append(dict(start=start, end=end, weight=weight, glyphs=glyphs, unitsPerEm=face['unitsPerEm']))
        styled.append(dict(text=text, family=family, direction='rtl' if rtl else 'ltr', spans=spans))
    from fontTools.pens.basePen import BasePen
    class ContourPen(BasePen):
        def __init__(self, glyphs):
            super().__init__(glyphs)
            self.ops = []
        def _moveTo(self, p):
            self.first = p
            self.current = p
            self.ops.append(['move', list(p)])
        def _lineTo(self, p):
            self.current = p
            self.ops.append(['line', list(p)])
        def _qCurveToOne(self, a, p):
            self.current = p
            self.ops.append(['quad', list(a), list(p)])
        def _curveToOne(self, a, b, p):
            self.current = p
            self.ops.append(['cubic', list(a), list(b), list(p)])
        def _closePath(self):
            # Close's implicit straight edge is represented explicitly.
            if self.current != self.first: self.ops.append(['line', list(self.first)])
            self.ops.append(['close'])
    outlines = []
    for filename, letters in [('Arimo-400.ttf', 'IÖğç'), ('NotoSansArabic-400.ttf', 'سلم'),
                               ('NotoSansHebrew-400.ttf', 'שָם'), ('NotoSansMath-400.ttf', '⌀≤±')]:
        path = next((ROOT / 'crates/shared/geometry-core/src/text' / f).resolve()
                    for f in files if Path(f).name == filename)
        font = TTFont(path)
        glyphs, cmap = font.getGlyphSet(), font.getBestCmap()
        for letter in letters:
            name = cmap[ord(letter)]
            pen = ContourPen(glyphs)
            glyphs[name].draw(pen)
            outlines.append(dict(font=files.index(next(f for f in files if Path(f).name == filename)),
                                 id=font.getGlyphID(name), source=letter,
                                 unitsPerEm=font['head'].unitsPerEm, ops=pen.ops))
    flow_cases = []
    samples = [
        dict(text='AV Ög\u0306ç\nİstanbul 2500 mm\tÖlçü', family='arimo', direction='ltr', height=.25,
             widthFactor=.8, boxWidth=1.25, lineSpacing=1.1, along=.5,
             flow=dict(indent=.5, leftIndent=1, tabs=[2,4], columns=dict(count=2,gap=1,height=.9))),
        dict(text='Ög\u0306ç\r\n\rİstanbul\n', family='arimo', direction='ltr', height=2,
             widthFactor=1, boxWidth=None, lineSpacing=1, along=0, flow={}),
        dict(text='مرحبا بالعالم 123 العربية 456 السلام عليكم', family='noto-sans-arabic', direction='rtl', height=.25,
             widthFactor=1, boxWidth=2, lineSpacing=1, along=1,
             flow=dict(indent=-.2,leftIndent=.2, columns=dict(count=3,gap=1,height=.84))),
        dict(text='AVATAR supercalifragilistic \t X', family='arimo', direction='ltr', height=.25,
             widthFactor=1, boxWidth=.4, lineSpacing=1, along=0, flow=dict(leftIndent=.2,tabs=[2])),
    ]
    for sample in samples:
        height, wf, flow = sample['height'], sample['widthFactor'], sample['flow']
        face = next(c for c in cases if c['family'] == sample['family'] and c['weight'] == 400 and not c['italic'])
        path = next((ROOT / 'crates/shared/geometry-core/src/text' / f).resolve()
                    for f in files if Path(f).name == face['name'])
        def shape(body, begin, end):
            if begin == end: return [], 0
            visual,levels = line_order(body,begin,end,sample['direction'],with_levels=True)
            runs = []
            for index in visual:
                level = levels[index]
                step = -1 if level % 2 else 1
                if runs and runs[-1][0] == level and runs[-1][1][-1]+step == index:
                    runs[-1][1].append(index)
                else: runs.append((level,[index]))
            glyphs = []
            for level,indices in runs:
                a,b = min(indices),max(indices)+1
                made = json.loads(run(['hb-shape',str(path),body[a:b],'--no-glyph-names','--output-format=json',
                    '--utf8-clusters','--cluster-level=0','--shapers=ot','--language=und',
                    '--features=kern=1,liga=1,clig=1','--direction='+('rtl' if level % 2 else 'ltr'),
                    '--text-before='+body[begin:a],'--text-after='+body[b:end]]))
                byte_offset=len(body[:a].encode())-len(body[:begin].encode())
                for g in made:
                    g['cl'] += byte_offset
                    glyphs.append(g)
            assert all(g['g'] != 0 for g in glyphs)
            return glyphs, sum(g['ax'] for g in glyphs)/face['unitsPerEm']
        def stop(at):
            import math
            stops = [t*height for t in flow.get('tabs',[])]
            last = stops[-1] if stops else 0
            next_stop = next((t for t in stops if t>at), last+(math.floor((at-last)/(4*height))+1)*4*height)
            left = flow.get('leftIndent',0)*height
            return left if at<left<next_stop else next_stop
        def measure(body, begin, end, indent, offset):
            end = begin+len(body[begin:end].rstrip(' '))
            pieces,at,part = [],indent,begin
            for bound in [m.start()+begin for m in re.finditer('\t',body[begin:end])]+[end]:
                glyphs,advance = shape(body,part,bound)
                pieces.append(dict(start=offset+part,end=offset+bound,x=at,glyphs=glyphs,advance=advance))
                at += advance*height*wf
                if bound<end: at=stop(at)
                part=bound+1
            return dict(pieces=pieces,indent=indent,width=at-indent)
        rows,start = [],0
        text = sample['text']
        bounds = [(m.start(),m.end()) for m in re.finditer(r'\r\n|\r|\n',text)]+[(len(text),len(text))]
        for end,after in bounds:
            body,offset = text[start:end],start
            start = after
            first = (flow.get('leftIndent',0)+flow.get('indent',0))*height
            if sample['boxWidth'] is None:
                rows.append(measure(body,0,len(body),first,offset)); continue
            words = list(re.finditer('[^ ]+',body))
            if not words: rows.append(measure(body,0,len(body),first,offset)); continue
            begin,indent,filled = 0,first,None
            for word in words:
                candidate = measure(body,begin,word.end(),indent,offset)
                if filled is not None and indent+candidate['width']>sample['boxWidth']:
                    rows.append(filled)
                    begin=word.start(); indent=flow.get('leftIndent',0)*height
                    candidate=measure(body,begin,word.end(),indent,offset)
                filled=candidate
            rows.append(filled)
        width = sample['boxWidth'] if sample['boxWidth'] is not None else max(r['indent']+r['width'] for r in rows)
        pitch=5/3*height*sample['lineSpacing']
        columns=flow.get('columns')
        per=max(1,int(columns['height']/pitch)) if columns else len(rows)
        count=columns['count'] if columns else 1
        step=width+columns['gap']*height if columns else 0
        pieces,depth=[],0
        for index,row in enumerate(rows):
            col=min(index//per,count-1); number=index-col*per
            depth=max(depth,number+1)
            shift=sample['along']*(width-row['indent']-row['width'])+col*step
            for piece in row['pieces']:
                piece.update(x=piece['x']+shift,y=-number*pitch)
                pieces.append(piece)
        flow_cases.append(dict(**sample, width=width*count+(count-1)*(columns['gap']*height if columns else 0),
                               pitch=pitch,depth=depth,pieces=pieces,unitsPerEm=face['unitsPerEm']))
    atoms = []
    for text,ranges in [('A 1/2 אבג',[(2,5)]),('אבג 1 / 2 ABC',[(4,9)]),
                        ('سعليكمم',[(1,6)]),('A אבג B',[(2,5)]),('ABC XX YY Z',[(4,6),(7,9)])]:
        shadow, mapping, previous = '', [], 0
        for index,(a,b) in enumerate(ranges):
            shadow += text[previous:a]+'\ufffc'
            mapping.extend(dict(scalar=i) for i in range(previous,a))
            mapping.append(dict(atom=index))
            previous=b
        shadow += text[previous:]
        mapping.extend(dict(scalar=i) for i in range(previous,len(text)))
        isolated = []
        if text == 'سعليكمم':
            path = next((ROOT / 'crates/shared/geometry-core/src/text' / f).resolve()
                        for f in files if Path(f).name == 'NotoSansArabic-400.ttf')
            for index in [0,len(text)-1]:
                glyphs = json.loads(run(['hb-shape',str(path),text[index],'--no-glyph-names',
                    '--output-format=json','--utf8-clusters','--cluster-level=0','--shapers=ot',
                    '--language=und','--features=kern=1,liga=1,clig=1','--direction=rtl']))
                isolated.append(dict(scalar=index,glyphs=[g['g'] for g in glyphs]))
        for base in ['auto','ltr','rtl']:
            order=line_order(shadow,0,len(shadow),base)
            atoms.append(dict(text=text,direction=base,ranges=ranges,isolated=isolated,
                              visual=[mapping[i] for i in order]))
    fractions=[]
    for text,family,base,weight in [('1/2','arimo','ltr',400),('12#345','arimo','ltr',600),
                                   ('+0.2^-0.1','arimo','ltr',400),('١٢/٣٤','noto-sans-arabic','rtl',400),
                                   ('1 / 2','arimo','ltr',400)]:
        face=next(c for c in cases if c['family']==family and c['weight']==weight and not c['italic'])
        path=next((ROOT/'crates/shared/geometry-core/src/text'/f).resolve() for f in files if Path(f).name==face['name'])
        mark=next(i for i,c in enumerate(text) if c in '/#^')
        def piece(part,source):
            if not part: return dict(glyphs=[],advance=0)
            order,levels=line_order(part,0,len(part),base,with_levels=True)
            runs=[]
            for index in order:
                level=levels[index];step=-1 if level%2 else 1
                if runs and runs[-1][0]==level and runs[-1][1][-1]+step==index: runs[-1][1].append(index)
                else: runs.append((level,[index]))
            made=[];pen=0
            for level,indices in runs:
                a,b=min(indices),max(indices)+1
                glyphs=json.loads(run(['hb-shape',str(path),'--text='+part[a:b],'--no-glyph-names','--output-format=json',
                    '--utf8-clusters','--cluster-level=0','--shapers=ot','--language=und',
                    '--features=kern=1,liga=1,clig=1','--direction='+('rtl' if level%2 else 'ltr'),
                    '--text-before='+part[:a],'--text-after='+part[b:]]))
                for g in glyphs:
                    assert g['g']!=0
                    made.append(dict(id=g['g'],pen=pen/face['unitsPerEm'],x=(pen+g['dx'])/face['unitsPerEm'],
                        y=g['dy']/face['unitsPerEm'],advance=g['ax']/face['unitsPerEm'],
                        byte=len(text[:source].encode())+len(part[:a].encode())+g['cl']))
                    pen+=g['ax']
            return dict(glyphs=made,advance=pen/face['unitsPerEm'])
        upper,lower=piece(text[:mark],0),piece(text[mark+1:],mark+1)
        n,d=upper['advance']*.6,lower['advance']*.6
        parts=[];bars=[]
        if text[mark]=='#':
            slash=piece('/',mark);gap=slash['advance']*.6
            parts=[(upper,0,.45),(slash,n,0),(lower,n+gap,-.25)];width=n+gap+d
        else:
            width=max(n,d)
            parts=[(upper,(width-n)/2,.45),(lower,(width-d)/2,-.25)]
            if text[mark]=='/': bars.append([0,.33-.06*.6,width,.33])
        glyphs=[]
        for p,x,y in parts:
            for g in p['glyphs']:
                glyphs.append(dict(**{k:v for k,v in g.items() if k not in ['x','y','pen','advance']},
                    x=x+g['x']*.6,y=y+g['y']*.6,pen=x+g['pen']*.6,advance=g['advance']*.6,size=.6))
        glyphs.sort(key=lambda g:g['pen'])
        fractions.append(dict(text=text,family=family,direction=base,weight=weight,width=width,glyphs=glyphs,bars=bars))
    mixed_scripts=[]
    path=next((ROOT/'crates/shared/geometry-core/src/text'/f).resolve() for f in files if Path(f).name=='NotoSans-400.ttf')
    upm=TTFont(path)['head'].unitsPerEm
    for parts in [[('AV ','Latn'),('Ελληνικά ','Grek'),('Привет','Cyrl')],
                  [('Latin (','Latn'),('Ελληνικά','Grek'),(') tail','Latn')],
                  [('  (Ελληνικά) ','Grek'),('Привет','Cyrl')],
                  [('Привет [','Cyrl'),('Latin (','Latn'),('Ελληνικά','Grek'),(')','Latn'),('] мир','Cyrl')]]:
        text=''.join(p for p,_ in parts);byte=0;cursor=0;glyphs=[]
        for part,script in parts:
            end=byte+len(part.encode())
            before=text.encode()[:byte].decode();after=text.encode()[end:].decode()
            made=json.loads(run(['hb-shape',str(path),'--text='+part,'--no-glyph-names','--output-format=json',
                '--utf8-clusters','--cluster-level=0','--shapers=ot','--language=und','--script='+script,
                '--features=kern=1,liga=1,clig=1','--direction=ltr','--text-before='+before,'--text-after='+after]))
            for g in made:
                assert g['g']!=0
                glyphs.append(dict(id=g['g'],byte=byte+g['cl'],x=(cursor+g['dx'])/upm,
                                   y=g['dy']/upm,advance=g['ax']/upm))
                cursor+=g['ax']
            byte=end
        mixed_scripts.append(dict(text=text,family='noto-sans',glyphs=glyphs,advance=cursor/upm,
                                  scripts=[dict(text=p,script=s) for p,s in parts]))
    return dict(format='kentos.text-shaping-reference', version=1,
                harfBuzz=run(['hb-shape', '--version']).splitlines()[0],
                friBidi=run(['fribidi', '--version']).splitlines()[:3], cases=cases, bidi=bidi, wrapped=wrapped, styled=styled, outlines=outlines, flow=flow_cases, atoms=atoms, fractions=fractions, mixedScripts=mixed_scripts)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    made = json.dumps(build(), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if OUT.read_text() != made: raise SystemExit('Metin şekillendirme referansı güncel değil.')
    else: OUT.write_text(made)
    print('29 HarfBuzz yüzü, 18 yön, 12 bağlamlı satır, 2 biçimli metin ve 13 FontTools konturu, 4 fiziksel akış ve 15 satır içi nesne, 5 kesir ve 4 karışık script örneği doğrulandı.')


if __name__ == '__main__': main()
