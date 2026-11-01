"""Session visibility contract; integer set counts, without running KentOS."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
setup = json.loads((ROOT / 'fixtures/interaction/v1/objects.kcad').read_text())
count = len(setup['entities'])
unchanged = dict(canUndo=False, dirty=False, revision='same')

def step(action, active=False, hidden=0, op='execute'):
    return dict(op=op, input=dict(action=action), result=dict(status='completed',
                output=dict(active=active, hidden=hidden, retained=count-hidden), warnings=[]),
                expect=unchanged)

def refusal(uids, code, path):
    return dict(op='execute', input=dict(action=dict(kind='isolate', uids=uids)),
                result=dict(status='failed', error=dict(code=code, message='$any', path=path)),
                expect=unchanged)

def validate(action):
    return dict(op='validate',input=dict(action=action),result=dict(status='completed',output=None,warnings=[]),expect=unchanged)

cases = [
    dict(name='Salt okunur sorgu', steps=[step(dict(kind='query'))]),
    dict(name='Plan maske uygulamaz', steps=[
        step(dict(kind='isolate', uids=['$uidOf:1']), True, count-1, 'plan'),
        step(dict(kind='query'))]),
    dict(name='Yalıt, sorgula ve bitir: geometri geçmişi ve revizyon değişmez', steps=[
        step(dict(kind='isolate', uids=['$uidOf:1']), True, count-1),
        step(dict(kind='query'), True, count-1), step(dict(kind='end')),
        step(dict(kind='query'))]),
    dict(name='Boş hedef atomik reddedilir', steps=[refusal([], 'no_entities', 'uids')]),
    dict(name='Geçersiz kimlik atomik reddedilir', steps=[refusal(['bad'], 'invalid_uid', 'uids[0]')]),
    dict(name='Olmayan nesne atomik reddedilir', steps=[refusal(
        ['0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff'], 'entity_not_found', 'uids[0]')]),
    dict(name='Pasif maskeyi bitir',steps=[step(dict(kind='end'))]),
    dict(name='Doğrulama maske kurmaz',steps=[validate(dict(kind='isolate',uids=['$uidOf:1'])),step(dict(kind='query'))]),
    dict(name='Bitirme doğrulaması',steps=[validate(dict(kind='end'))]),
    dict(name='Sorgu doğrulaması',steps=[validate(dict(kind='query'))]),
    dict(name='Tekrarlı hedef yalnız bir kez sayılır',steps=[step(dict(kind='isolate',uids=['$uidOf:1','$uidOf:1']),True,count-1)]),
    dict(name='Kilitli katman nesnesi yalıtılabilir; kilidi değişmez',steps=[step(dict(kind='isolate',uids=['$uidOf:6']),True,count-1)]),
    dict(name='Gizli katman kuralı bağımsızdır; maske sayacı onu zorla açmaz',steps=[step(dict(kind='isolate',uids=['$uidOf:7']),True,count-1)]),
    dict(name='Yeni hedefler mevcut maskeyi değiştirir',steps=[step(dict(kind='isolate',uids=['$uidOf:1']),True,count-1),step(dict(kind='isolate',uids=['$uidOf:2','$uidOf:3']),True,count-2),step(dict(kind='query'),True,count-2)]),
    dict(name='Bitirme planı etkin maskeyi değiştirmez',steps=[step(dict(kind='isolate',uids=['$uidOf:1']),True,count-1),step(dict(kind='end'),op='plan'),step(dict(kind='query'),True,count-1)]),
    dict(name='Geçersiz kimlik önceki maskeyi korur',steps=[step(dict(kind='isolate',uids=['$uidOf:1']),True,count-1),refusal(['$uidOf:2','bad'],'invalid_uid','uids[1]'),step(dict(kind='query'),True,count-1)]),
    dict(name='Olmayan nesne önceki maskeyi korur',steps=[step(dict(kind='isolate',uids=['$uidOf:1']),True,count-1),refusal(['$uidOf:2','0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff'],'entity_not_found','uids[1]'),step(dict(kind='query'),True,count-1)]),
    dict(name='Boş hedef önceki maskeyi korur',steps=[step(dict(kind='isolate',uids=['$uidOf:1']),True,count-1),refusal([],'no_entities','uids'),step(dict(kind='query'),True,count-1)]),
    dict(name='Bütün nesneleri yalıtma etkin fakat boş gizli küme verir',steps=[step(dict(kind='isolate',uids=[f'$uidOf:{i}' for i in range(1,count+1)]),True,0)]),
    dict(name='Tekrarlı bitirme geometri geçmişini değiştirmez',steps=[step(dict(kind='end')),step(dict(kind='end'))]),
    dict(name='Yeni maske planı önceki hedefleri korur',steps=[step(dict(kind='isolate',uids=['$uidOf:1']),True,count-1),step(dict(kind='isolate',uids=['$uidOf:2','$uidOf:3']),True,count-2,'plan'),step(dict(kind='query'),True,count-1)]),
]
payload = dict(format='kentos.command-cases', version=1, command='cad.entities.isolate',
               commandVersion=1, title='Geçici nesne yalıtımı', setup=setup, cases=cases,
               note='scripts/fixtures/isolation_cases.py; yalnız oturum maskesi, nesneler ve dosya değişmez.')
path = ROOT / 'fixtures/commands/v1/cad.entities.isolate.json'
text = json.dumps(payload, ensure_ascii=False, indent=2) + '\n'
if '--check' in sys.argv:
    if path.read_text() != text:
        raise SystemExit('isolation cases differ')
else:
    path.write_text(text)
