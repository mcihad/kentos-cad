#!/usr/bin/env python3
"""ADR 0630's explicit template policies, independently from KentOS code.
The old ADR 0346 cases remain unchanged. This reference only reads package data.
"""
import argparse
import copy
import json
from pathlib import Path
import template_command_cases as T

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/cad.templates.apply.review.json'
P = T.TEMPLATE
S = T.STANDARD
WALLS = list(zip(['kentos-dis-duvar-25', 'kentos-ic-duvar-15', 'kentos-gazbeton-20',
                  'kentos-betonarme-25', 'kentos-bolme-10'], T.WALL_STYLES))
fold = T.S.fold


def answer(doc, inp):
    settings = doc['settings']
    old_unit = 'm' if settings.get('srid', 0) else settings.get('drawingUnit', 'm')
    mode = inp['settingsMode']
    result = {'unit': old_unit if mode == 'keep' else inp.get('unit', P['unit']),
              'layers': [], 'styles': [], 'blocks': [], 'wallStyles': [], 'levels': [],
              'sheets': [s['name'] for s in P['sheets']], 'revision': '$current'}
    review = {'mode': mode, 'previousUnit': old_unit, 'previousPlotScale': settings['plotScale'],
              'plotScale': settings['plotScale'] if mode == 'keep' else P['plotScale'],
              'spaceMethod': settings.get('arch', {}).get('spaceMethod', P['spaceMethod']), 'preserved': []}
    if 'spaceMethod' in settings.get('arch', {}):
        review['previousSpaceMethod'] = settings['arch']['spaceMethod']

    def preserved(kind, now, key='name'):
        review['preserved'].append({'kind': kind, 'id': now['id'], 'name': now['name'], 'matchedBy': key})

    def by_name(kind, want, have, destination):
        for w in want:
            now = next((h for h in have if fold(h['name'].strip()) == fold(w['name'].strip())), None)
            if now:
                preserved(kind, now)
            else:
                destination.append(w['name'].strip())

    by_name('layer', S['layers'], list(T.S.leaves(doc['layers'])), result['layers'])
    by_name('textStyle', S['textStyles'], settings.get('textStyles', []), result['styles'])
    by_name('dimensionStyle', S['dimensionStyles'], settings.get('dimensionStyles', []), result['styles'])
    by_name('block', [{'name': n} for n in T.LIBRARY], doc.get('blocks', []), result['blocks'])
    for kind, want, have, destination in [
        ('wallStyle', [{'id': i, 'name': n} for i, n in WALLS], settings.get('wallStyles', []), result['wallStyles']),
        ('level', P['levels'], settings.get('arch', {}).get('levels', []), result['levels']),
    ]:
        for w in want:
            now = next((h for h in have if h['id'] == w['id'] or fold(h['name'].strip()) == fold(w['name'].strip())), None)
            if now:
                same_id, same_name = now['id'] == w['id'], fold(now['name'].strip()) == fold(w['name'].strip())
                key = 'idAndName' if same_id and same_name else 'id' if same_id else 'name'
                preserved(kind, now, key)
            else:
                destination.append(w['name'])
    result['review'] = review
    return result


def step(doc, inp, op):
    return {'op': op, 'command': 'cad.templates.apply', 'input': inp,
            'result': {'status': 'completed', 'output': None if op == 'validate' else answer(doc, inp), 'warnings': []},
            'expect': {'revision': 'changed', 'dirty': True} if op == 'execute' else {'revision': 'same'}}


def cases():
    rows = []
    for unit in ('m', 'mm', 'cm', 'in', 'ft'):
        for objects in (False, True):
            for mode in ('keep', 'template'):
                doc = T.setup(objects=objects, unit=unit)
                inp = {'name': 'mimari', 'settingsMode': mode}
                rows.append({'name': f'{unit} / nesneli={objects} / {mode}: plan, doğrula, uygula',
                             'setup': doc, 'steps': [step(doc, inp, op) for op in ('plan', 'validate', 'execute')]})
    for unit in ('m', 'mm', 'in', 'ft'):
        doc = T.setup(objects=True, unit='cm')
        inp = {'name': 'mimari', 'settingsMode': 'template', 'unit': unit}
        rows.append({'name': f'Açık paket politikasıyla {unit}: geometriye dokunmadan', 'setup': doc,
                     'steps': [step(doc, inp, 'execute')]})
    doc = T.setup(objects=True, unit='m', levels=[{'id': 'zemin', 'name': 'Kuruma özel kat', 'elevation': 4.2}],
                  text_styles=[{'id': '0192f5a0-0000-7000-8000-000000000001', 'name': 'A-Yazı',
                                'font': 'courier-prime', 'height': 3.5}], blocks=True, method='gross')
    layer = T.layer('eski', 'A-DUVAR', '#FF0000', weight=0.72)
    layer.update(visible=False, locked=True)
    doc['layers'].append(layer)
    doc['settings']['drawingFont'] = 'courier-prime'
    doc['settings']['wallStyles'] = [{'id': 'kentos-dis-duvar-25', 'name': 'Mevcut özel duvar',
                                      'components': [{'name': 'Özel', 'thickness': 0.31}]}]
    inp = {'name': 'mimari', 'settingsMode': 'keep'}
    rows.append({'name': 'Gizli kilitli katman, mevcut font, kat/duvar kimliği ve blok korunur', 'setup': doc,
                 'steps': [step(doc, inp, 'plan'), step(doc, inp, 'execute')]})
    doc = T.setup(unit='mm', levels=[{'id': 'kurum', 'name': 'zEmİn kat', 'elevation': 7.0}])
    rows.append({'name': 'Katın Türkçe büyük küçük harfli aynı adı korunur', 'setup': doc,
                 'steps': [step(doc, inp, 'execute')]})
    for mode in ('keep', 'template'):
        doc = T.setup(objects=True, unit='cm')
        doc['settings']['srid'] = 5256
        inp = {'name': 'mimari', 'settingsMode': mode}
        if mode == 'template':
            inp['unit'] = 'm'
        rows.append({'name': f'Koordinat sisteminin metre birimi / {mode}', 'setup': doc,
                     'steps': [step(doc, inp, op) for op in ('plan', 'validate', 'execute')]})
    for op in ('validate', 'plan', 'execute'):
        doc = T.setup(objects=True)
        doc['settings']['srid'] = 5256
        rows.append({'name': f'Koordinat sistemine cm ofis birimi uygulanamaz / {op}', 'setup': doc, 'steps': [{
            'op': op, 'command': 'cad.templates.apply',
            'input': {'name': 'mimari', 'settingsMode': 'template'},
            'result': T.refusal('unit_needs_local', 'Koordinat sistemi olan projenin birimi metredir; ofis ayarlarını metre birimiyle uygulayın.', 'unit'),
            'expect': {'revision': 'same'}}]})
    for op in ('validate', 'plan', 'execute'):
        doc = T.setup()
        rows.append({'name': f'Birim ve koruma çelişkisi / {op}', 'setup': doc, 'steps': [{
            'op': op, 'command': 'cad.templates.apply',
            'input': {'name': 'mimari', 'settingsMode': 'keep', 'unit': 'cm'},
            'result': T.refusal('template_settings_conflict', 'Ayarları koru kipinde ayrıca birim verilemez.', 'unit'),
            'expect': {'revision': 'same'}}]})
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    rows = cases()
    assert len(rows) >= 20
    value = {'format': 'kentos.command-cases', 'version': 1, 'command': 'cad.templates.apply', 'commandVersion': 1,
             'title': 'Güvenli mimari şablonu', 'setup': T.setup(), 'cases': rows,
             'note': 'ADR 0630, bağımsız paket-verisi referansı; eski ADR 0346 fixture değiştirilmedi.'}
    text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
    if args.check:
        assert OUT.read_text() == text, f'{OUT} güncel değil'
    else:
        OUT.write_text(text)
    print(f'{len(rows)} bağımsız güvenli şablon durumu')

if __name__ == '__main__':
    main()
