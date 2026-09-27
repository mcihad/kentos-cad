"""The catalog's project forms for both platforms: Proje bilgileri, Yeniden
adlandır, Kopyasını oluştur and the conversion to the other storage mode.
What each says, which workspaces it offers, when its button is on, what the
metadata form sends, and the lines and reasons it writes (format in
fixtures/cloud/README.md; docs/adr/0028, 0039).

    python3 scripts/fixtures/forms_cases.py           # writes the file
    python3 scripts/fixtures/forms_cases.py --check   # writes nothing; compares

Writes fixtures/cloud/v1/forms.json. The words are the web's, written here
by hand; every answer below is worked out here, apart from the web's
TypeScript. apps/web/src/app/cloud/formsPlan.test.ts holds the web to the
file, and the desktop's forms read the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import re
import sys

OUT = 'fixtures/cloud/v1/forms.json'

NO_PLACE = 'Proje açabileceğiniz bir çalışma alanınız yok (project.create); kurum yöneticinize başvurun.'


def count_text(objects):
    """Decimal text grouped by threes with dots (1.234.567); anything else as it came."""
    if not re.fullmatch(r'\d+', objects):
        return objects
    head = len(objects) % 3 or 3
    parts = [objects[:head]] + [objects[i:i + 3] for i in range(head, len(objects), 3)]
    return '.'.join(parts)


TEXTS = {
    'cancel': 'Vazgeç',
    'saving': 'Kaydediliyor…',
    'place': 'Çalışma alanı',
    'noPlace': NO_PLACE,
    'fields': {
        'type': 'Tür',
        'typeLabel': 'Proje türü',
        'description': 'Açıklama',
        'descriptionPlaceholder': 'İsteğe bağlı: işin konusu, yeri, dayanağı',
        'tags': 'Etiketler',
        'tagsPlaceholder': 'Virgülle ayırın: Kadıköy, 2026',
    },
    'metadata': {
        'title': 'Proje bilgileri', 'name': 'Proje adı', 'save': 'Kaydet',
        'saved': {'sample': ['Ada 101'], 'text': '“Ada 101” projesinin bilgileri kaydedildi.'},
    },
    'rename': {
        'title': 'Bulut projesini yeniden adlandır', 'name': 'Yeni ad', 'hint': 'Projeye erişimi olan herkes yeni adı görür.', 'save': 'Yeniden adlandır',
        'saved': {'sample': ['Ada 101'], 'text': 'Proje “Ada 101” olarak yeniden adlandırıldı.'},
        'waiting': {'sample': ['Ada 101'], 'text': 'Yeni ad (“Ada 101”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir.'},
    },
    'duplicate': {
        'title': 'Projenin kopyasını oluştur',
        'lead': {'sample': ['Ada 101'], 'text': '“Ada 101” yeni bir proje olarak kopyalanır.'},
        'consequences': [
            'Katmanlar, ayarlar, stiller, açıklama, tür, etiketler ve bütün nesneler kalıcı kimlikleriyle kopyalanır.',
            'Geçmiş (komut günlüğü, olaylar), paylaşımlar ve favoriler kopyalanmaz; arşivlenmiş proje etkin bir kopya olur.',
            'Kopya sizin olur: siz paylaşana kadar yalnız size ve kurum politikasıyla kurum yöneticilerine görünür.',
        ],
        'name': 'Kopyanın adı', 'nameLabel': 'Kopyanın adı', 'placeLabel': 'Kopyanın çalışma alanı', 'make': 'Kopyasını oluştur', 'running': 'Kopyalanıyor…',
        'done': {'sample': ['Ada 101 (kopya)', '1234'], 'text': f'“Ada 101 (kopya)” oluşturuldu: {count_text("1234")} nesne kopyalandı.'},
    },
    'convert': {'name': 'Yeni projenin adı (isteğe bağlı)', 'nameLabel': 'Yeni projenin adı', 'placeLabel': 'Yeni projenin çalışma alanı'},
}


def parse_tags(text):
    return [t for t in (re.sub(r'\s+', ' ', part.strip()) for part in text.split(',')) if t]


def metadata_patch(shown, now):
    out = {}
    if now['name'].strip() != shown['name']:
        out['name'] = now['name'].strip()
    if now['projectType'] != shown['projectType']:
        out['projectType'] = now['projectType']
    if now['description'] != shown['description']:
        out['description'] = now['description']
    if '\n'.join(now['tags']) != '\n'.join(shown['tags']):
        out['tags'] = list(now['tags'])
    return out


def workspace_name(kind, name):
    return name if kind == 'organization' else 'Kişisel'


def places(memberships, first):
    ok = [m for m in memberships if m['active'] and m['seat'] and 'project.create' in m['capabilities']]
    ok.sort(key=lambda m: 0 if m['tenantId'] == first else 1)  # stable: the rest keep the account's order
    return [{'tenantId': m['tenantId'], 'label': workspace_name(m['tenantKind'], m['tenantName'])} for m in ok]


def convert_form(p, open_dirty):
    db = p['storage'] == 'file'
    to = 'database' if db else 'file'
    consequences = [
        'Projenin en yeni revizyonu aktarılır: her nesne kalıcı kimliğiyle; ayarlar, katmanlar ve stiller dosyadan. Analitik CAD tanımları korunur, GIS çizgisine indirgenmez.'
        if db else 'Projenin şimdiki hâli (tek anlık görüntüsü) yeni dosya projesinin 1. revizyonu olur.',
        f'“{p["name"]}” olduğu gibi kalır: aynı çizimin iki yazılabilir sahibi olmaz, yeni proje başka bir projedir.',
        'Yeni proje sizin olur; geçmiş, paylaşım ve favoriler gelmez. Açıklama, tür ve etiketler gelir.',
    ]
    if db:
        consequences.append('Sunucunun almadığı bir nesne (±1 000 000 000 sınırını aşan değer) varsa hiçbir proje oluşturulmaz ve nesne söylenir.')
    if db and open_dirty:
        consequences.append('Açık çizimdeki kaydedilmemiş değişiklikler aktarılmaz: önce Kaydet ile yeni revizyon yazın.')
    return {
        'to': to,
        'title': "PostGIS'e aktar" if db else 'Dosya projesine çevir',
        'lead': f'“{p["name"]}” projesinden {"nesne nesne veritabanında saklanan" if db else "dosya olarak (KCAD revizyonları) saklanan"} yeni bir proje oluşturulur.',
        'consequences': consequences,
        'placeholder': f'{p["name"]} ({"PostGIS" if db else "dosya"})',
        'running': 'Veritabanına aktarılıyor…' if db else 'Dosya projesi oluşturuluyor…',
    }


def converted_line(name, objects, to):
    tail = ' veritabanına aktarıldı' if to == 'database' else ', 1. revizyon'
    return f'“{name}” oluşturuldu: {count_text(objects)} nesne{tail}.'


def failure_reason(err):
    if err.get('kind') == 'api':
        code = err.get('error') or ('network' if err['status'] == 0 else 'http')
        if code == 'conflict':
            return 'Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.'
        if code == 'network':
            return 'Sunucuya ulaşılamadı; bağlantınızı denetleyip yeniden deneyin.'
        return err.get('message') or err['fallback']
    if err.get('kind') == 'error':
        return err['message']
    return 'İşlem tamamlanamadı.'


def convert_reason(err):
    base = failure_reason(err)
    m = re.match(r'entities\[(\d+)\]', err.get('path') or '') if err.get('kind') in ('api', 'error') else None
    return f'{base} (dosyanın {int(m[1]) + 1}. nesnesi; hiçbir proje oluşturulmadı).' if m else base


# ── Cases ────────────────────────────────────────────────────────────

SHOWN = {'name': 'Ada 101', 'projectType': 'cad', 'description': 'Kadastro çalışması', 'tags': ['Kadıköy', '2026']}
NOW = [
    ('nothing changed', {'name': 'Ada 101', 'projectType': 'cad', 'description': 'Kadastro çalışması', 'tags': ['Kadıköy', '2026']}),
    ('the name, with spaces around it', {'name': '  Ada 102 ', 'projectType': 'cad', 'description': 'Kadastro çalışması', 'tags': ['Kadıköy', '2026']}),
    ('only spaces around the same name', {'name': ' Ada 101 ', 'projectType': 'cad', 'description': 'Kadastro çalışması', 'tags': ['Kadıköy', '2026']}),
    ('an empty name', {'name': '   ', 'projectType': 'gis', 'description': 'Kadastro çalışması', 'tags': ['Kadıköy', '2026']}),
    ('the type and the description', {'name': 'Ada 101', 'projectType': 'subdivision', 'description': '', 'tags': ['Kadıköy', '2026']}),
    ('the tags in another order', {'name': 'Ada 101', 'projectType': 'cad', 'description': 'Kadastro çalışması', 'tags': ['2026', 'Kadıköy']}),
    ('a tag less', {'name': 'Ada 101', 'projectType': 'cad', 'description': 'Kadastro çalışması', 'tags': ['Kadıköy']}),
]
TAGS = ['Kadıköy, 2026', '  a   b ,, c ', '', ' , ', 'tek', 'a,a']
RENAMES = [('Ada 102', 'Ada 101'), ('Ada 101', 'Ada 101'), ('  Ada 101  ', 'Ada 101'), ('', 'Ada 101'), ('   ', 'Ada 101'), (' Ada 102 ', 'Ada 101')]
DUPLICATES = [(2, 'Ada 101 (kopya)'), (0, 'Ada 101 (kopya)'), (1, '  '), (1, 'x')]


def membership(tid, name, kind='organization', active=True, seat=True, caps=('project.create',)):
    return {'tenantId': tid, 'tenantSlug': tid, 'tenantName': name, 'tenantKind': kind, 'role': 'member', 'seat': seat, 'active': active, 'capabilities': list(caps)}


MEMBERSHIPS = [
    membership('t-kisisel', 'Ayşe Yılmaz', kind='personal'),
    membership('t-buro', 'Büro'),
    membership('t-belediye', 'Belediye', caps=()),
    membership('t-eski', 'Eski Kurum', active=False),
    membership('t-koltuksuz', 'Koltuksuz Kurum', seat=False),
    membership('t-ortak', 'Ortak Çalışma'),
]
CONVERTS = [
    ('a file project, not open here', {'name': 'Ada 101', 'storage': 'file'}, False),
    ('a file project open here with unsaved changes', {'name': 'Ada 101', 'storage': 'file'}, True),
    ('a database project', {'name': 'Ada 101', 'storage': 'database'}, False),
    ('a database project, open and changed (nothing more to say)', {'name': 'Ada 101', 'storage': 'database'}, True),
]
COUNTS = ['0', '7', '999', '1000', '1234', '1234567', '12345678901234567890', '', '12a', '-5']
FAILURES = [
    ('a conflict', {'kind': 'api', 'status': 409, 'error': 'conflict', 'message': 'Sürüm değişti.'}),
    ('no connection', {'kind': 'api', 'status': 0, 'fallback': 'x'}),
    ('the server’s words', {'kind': 'api', 'status': 422, 'error': 'invalid', 'message': 'Ad boş olamaz.'}),
    ('an error of the page', {'kind': 'error', 'message': 'Beklenmeyen yanıt.'}),
    ('not an error at all', {'kind': 'other'}),
]
CONVERT_FAILURES = [
    ('an object out of range', {'kind': 'api', 'status': 422, 'error': 'invalid', 'message': 'Değer ±1 000 000 000 sınırını aşıyor.', 'path': 'entities[41].geometry'}),
    ('the first object', {'kind': 'api', 'status': 422, 'error': 'invalid', 'message': 'Geçersiz nesne.', 'path': 'entities[0]'}),
    ('another field', {'kind': 'api', 'status': 422, 'error': 'invalid', 'message': 'Ad boş olamaz.', 'path': 'name'}),
    ('no field', {'kind': 'api', 'status': 422, 'error': 'invalid', 'message': 'Dosya okunamadı.'}),
    ('no connection', {'kind': 'api', 'status': 0, 'fallback': 'x'}),
]

file = {
    'format': 'kentos.forms',
    'version': 1,
    'texts': TEXTS,
    'limits': {'name': 200, 'description': 2000},
    'tags': [{'text': t, 'tags': parse_tags(t)} for t in TAGS],
    'copyNames': [{'name': n, 'copy': f'{n} (kopya)'} for n in ['Ada 101', 'Ada 101 (kopya)', '']],
    'metadata': {
        'shown': SHOWN,
        'cases': [{'title': t, 'now': now, 'patch': metadata_patch(SHOWN, now), 'savable': bool(now['name'].strip()) and bool(metadata_patch(SHOWN, now))} for t, now in NOW],
    },
    'rename': [{'typed': a, 'current': b, 'savable': bool(a.strip()) and a.strip() != b} for a, b in RENAMES],
    'duplicate': [{'places': n, 'name': name, 'savable': n > 0 and bool(name.strip())} for n, name in DUPLICATES],
    'places': {
        'memberships': MEMBERSHIPS,
        'cases': [{'first': f, 'places': places(MEMBERSHIPS, f)} for f in ['t-buro', 't-ortak', 't-kisisel', 't-belediye', 't-yok']],
    },
    'convert': [{'title': t, 'project': p, 'openDirty': d, 'expect': convert_form(p, d)} for t, p, d in CONVERTS],
    'counts': [{'objects': c, 'text': count_text(c)} for c in COUNTS],
    'convertedLines': [{'name': 'Ada 101 (PostGIS)', 'objects': '1234567', 'to': 'database', 'text': converted_line('Ada 101 (PostGIS)', '1234567', 'database')},
                       {'name': 'Ada 101 (dosya)', 'objects': '42', 'to': 'file', 'text': converted_line('Ada 101 (dosya)', '42', 'file')}],
    'failures': [{'title': t, 'error': e, 'text': failure_reason(e)} for t, e in FAILURES],
    'convertFailures': [{'title': t, 'error': e, 'text': convert_reason(e)} for t, e in CONVERT_FAILURES],
}

text = json.dumps(file, ensure_ascii=False, indent=1) + '\n'
if '--check' in sys.argv[1:]:
    with open(OUT, encoding='utf-8') as f:
        if f.read() != text:
            sys.exit(f'{OUT} is not what this script writes: run it without --check and read the difference.')
    print(f'{OUT} matches')
else:
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)
    print(f'{OUT} written')
