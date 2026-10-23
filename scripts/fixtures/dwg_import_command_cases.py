"""DWG command expectations from the independent ASCII DXF reference, not KentOS output.
Binary containers in fixtures/formats/v1/dwg are codec-produced (cargo run -p kentos-dwg
--example corpus); their bytes/hash are receipts, not a DWG conformance oracle.
Run with --check to verify the expectations and receipt hashes.
"""
import copy
import hashlib
import json
import sys
from pathlib import Path
import dxf_import_command_cases as reference
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/file.import.dwg.json'

def build():
    _, text = reference.build()
    doc = json.loads(text)
    # The basic mm corpus covers units, routing, block collision, atomic refusal,
    # validate/plan and undo; unsupported source types have their own codec tests.
    indices = [0, 1, 2, 3, *range(13, 23), *range(28, 40), *range(40,len(doc["cases"]))]
    doc['cases'] = [copy.deepcopy(doc['cases'][i]) for i in indices]
    def convert(v):
        if isinstance(v, dict):
            out = {convert(k): convert(x) for k, x in v.items()}
            if 'bytes' in out and 'sha256' in out and 'file' in out:
                path = ROOT / 'fixtures/formats/v1/dwg' / out['file']
                data = path.read_bytes()
                out['bytes'] = len(data)
                out['sha256'] = hashlib.sha256(data).hexdigest()
            # Diagnostic prose may come from either codec; exact refusal codes,
            # paths, warning structure, geometry and revision still match.
            if 'code' in out and 'message' in out:
                out['message'] = '$any'
            return out
        if isinstance(v, list): return [convert(x) for x in v]
        if isinstance(v, str):
            return v.replace('formats/v1/dxf-units/', 'formats/v1/dwg/').replace('formats/v1/dxf-import/', 'formats/v1/dwg/').replace('.dxf', '.dwg').replace('-dxf', '-dwg').replace('DXF', 'DWG')
        return v
    doc = convert(doc)
    doc['command'] = 'file.import.dwg'
    doc['title'] = 'DWG içe aktar'
    doc['note'] = __doc__.strip()
    # A JSON file is not a DWG signature. Verify a real truncated signature separately.
    case = doc['cases'][indices.index(33)]
    case['steps'][0]['input']['path'] = '$fixture:formats/v1/dwg/truncated.dwg'
    case['steps'][0]['result']['error']['code'] = 'import_unreadable'
    extra = copy.deepcopy(case)
    extra['name'] = 'Desteklenmeyen DWG imzası: çizim ve undo değişmez'
    extra['steps'][0]['input']['path'] = '$fixture:formats/v1/dwg/unsupported.dwg'
    extra['steps'][0]['result']['error']['code'] = 'unsupported_cad_version'
    doc['cases'].append(extra)
    return json.dumps(doc, ensure_ascii=False, indent=2) + '\n'

if __name__ == '__main__':
    text = build()
    if '--check' in sys.argv:
        if OUT.read_text() != text: raise SystemExit('DWG cases drift: regenerate')
    else:
        OUT.write_text(text)
    print(f'DWG: {len(json.loads(text)["cases"])} reference cases')
