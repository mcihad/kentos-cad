"""The share dialog's words and rules, for both platforms: the roles and what
they allow, who may be changed or taken away, the rows, the notes, the
questions, the invitations' checks, waits and links, the commands the dialog
sends and the lines it writes (format in fixtures/cloud/README.md).

    python3 scripts/fixtures/share_cases.py           # writes the file
    python3 scripts/fixtures/share_cases.py --check   # writes nothing; compares

Writes fixtures/cloud/v1/share.json. The texts are the web's words, written
here by hand; every answer below is worked out here, apart from the web's
TypeScript. apps/web/src/app/cloud/sharePlan.test.ts holds the web to the
file, and the desktop's Paylaş window reads the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys
from datetime import datetime, timedelta, timezone
from urllib.parse import urlencode
from zoneinfo import ZoneInfo
import re

OUT = 'fixtures/cloud/v1/share.json'
TZ = 'Europe/Istanbul'
ZONE = ZoneInfo(TZ)

ROLE_LABEL = {'owner': 'Sahip', 'manager': 'Yönetici', 'editor': 'Düzenleyici', 'commenter': 'Yorumcu', 'viewer': 'Görüntüleyici'}
GRANT_ROLES = ['viewer', 'commenter', 'editor', 'manager']
INVITE_ROLES = ['viewer', 'commenter', 'editor']
ROLE_HINT = {
    'viewer': 'Projeyi açar, görür, indirir ve geçmişine bakar; değiştiremez.',
    'commenter': 'Görüntüleyicinin yapabildikleri ve yorum.',
    'editor': 'Nesneleri çizer, değiştirir ve siler.',
    'manager': 'Düzenleyicinin yapabildikleri; ad, ayar ve katmanları değiştirir, projeyi paylaşır.',
}
BLOCK_TEXT = {
    'expired': 'Paylaşımın süresi doldu',
    'notMember': 'Kurumun üyesi değil',
    'inactive': 'Hesabı ya da kurum üyeliği etkin değil',
    'noSeat': 'Kurumda koltuğu yok',
    'guestsOff': 'Misafir; kurum dışarıdan misafir kabul etmiyor',
}
STORAGE_TEXT = {
    'database': {
        'title': 'Yönetilen PostGIS veritabanı',
        'detail': 'Nesneler sunucudaki veritabanında tek tek saklanır; her kayıt tek işlemde yazılır ve erişimi olan herkes hemen görür. Paylaşım alıcıya veritabanı hesabı ya da parolası vermez.',
    },
    'file': {
        'title': 'Dosya (KCAD revizyonları)',
        'detail': 'Proje sunucuda değişmez .kcad revizyonları olarak saklanır; her kayıt yeni bir revizyondur ve dayandığı revizyonla karşılaştırılır, arada başkası kaydettiyse üzerine yazılmaz. Paylaşım alıcıya dosya deposuna ayrı bir erişim vermez.',
    },
}
STATE_LABEL = {'pending': 'Bekliyor', 'accepted': 'Kabul edildi', 'revoked': 'Geri alındı', 'expired': 'Süresi doldu'}
INVITE_DAYS, INVITE_MAX_DAYS = 14, 90
DAY_CHOICES = [1, 3, 7, 14, 30, 60, 90]

# ── Small helpers, as the rules read ─────────────────────────────────


def date_text(iso):
    """dd.mm.yyyy in the device's local time (the fixture's zone)."""
    t = datetime.fromisoformat(iso.replace('Z', '+00:00')).astimezone(ZONE)
    return f'{t.day:02d}.{t.month:02d}.{t.year}'


def tr_upper(c):
    return {'i': 'İ', 'ı': 'I'}.get(c, c.upper())


def initials(name):
    words = [w for w in re.split(r'\s+', name) if w][:2]
    return ''.join(tr_upper(w[0]) for w in words) or '?'


ALPHABET = 'aAbBcCçÇdDeEfFgGğĞhHıIiİjJkKlLmMnNoOöÖpPqQrRsSşŞtTuUüÜvVwWxXyYzZ'


def tr_key(s):
    # Turkish order, letter by letter (the names here differ in their letters, not their case alone).
    return [ALPHABET.index(ch) if ch in ALPHABET else 1000 + ord(ch) for ch in s]


def source_text(p):
    grant = ''
    if p.get('grant'):
        grant = f"paylaşım: {ROLE_LABEL[p['grant']]}" + (f", {date_text(p['expiresAt'])} tarihine kadar" if p.get('expiresAt') else '')
    if p.get('blocked'):
        return ' · '.join(x for x in [BLOCK_TEXT[p['blocked']], grant] if x)
    via = p.get('via')
    if via == 'owner':
        return 'Proje sahibi'
    if via == 'policy':
        return ' · '.join(x for x in ['Kurum politikası: kurum yöneticisi', grant] if x)
    kind = 'Misafir (davetle)' if p.get('guest') else 'Paylaşım'
    return f"{kind}, {date_text(p['expiresAt'])} tarihine kadar" if p.get('expiresAt') else kind


GUEST_ROLE = 'Misafirin rolü davetle verilir. Değiştirmek için erişimini kaldırıp yeni rolle yeniden davet edin.'


def fixed_reason(row):
    if row['you']:
        return 'Kendi erişiminizi buradan değiştiremezsiniz; proje sahibine ya da başka bir yöneticiye başvurun.'
    if row['owner']:
        return 'Proje sahibinin erişimi paylaşımla değişmez.'
    if not row.get('grant'):
        return 'Kurum politikasından gelen erişim paylaşımla değişmez.'
    return 'Bu erişimi değiştirme yetkiniz yok.'


OUTSIDE = 'Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.'


def policy_text(kind, admins):
    if kind == 'personal':
        return 'Kişisel alanınızdaki bu proje yalnız paylaştığınız ve davet ettiğiniz kişilere açıktır.'
    if admins:
        return f'Kurumun politikası açık: kurum sahibi ve yöneticileri paylaşılmamış kurum projelerine de yönetici olarak erişir. Paylaşım kurumun üyeleriyledir; {OUTSIDE}'
    return f'Kurumun politikası kapalı: kurum yöneticileri de yalnız kendileriyle paylaşılan projelere erişir. Paylaşım kurumun üyeleriyledir; {OUTSIDE}'


def people_view(lst, me, may_share):
    rows = []
    for p in lst['people']:
        you = p['userId'] == me
        owner = p['userId'] == lst['ownerId']
        other = may_share and not you and not owner and bool(p.get('grant'))
        row = {
            'userId': p['userId'],
            'name': p['displayName'] or 'Adı görünmeyen hesap',
            'roleLabel': ROLE_LABEL[p['role']] if p.get('role') else 'Erişemiyor',
            'source': source_text(p),
            'blocked': not p.get('role'),
            'you': you,
            'owner': owner,
            'guest': p['guest'],
            'canChange': other and not p['expired'] and p.get('via') == 'grant' and not p['guest'],
            'canRevoke': other,
        }
        for k in ('email', 'grant', 'expiresAt'):
            if p.get(k) is not None:
                row[k] = p[k]
        row['initials'] = initials(row['name'])
        row['sub'] = ' · '.join(x for x in [row.get('email'), row['source']] if x)
        row['roleTip'] = GUEST_ROLE if (not (row['canChange'] and row.get('grant'))) and row['guest'] else None
        row['fixed'] = None if row['canRevoke'] else fixed_reason(row)
        rows.append(row)
    rows.sort(key=lambda r: (0 if r['owner'] else 1, tr_key(r['name'])))
    st = STORAGE_TEXT[lst['storage']]
    return {
        'storage': {'lead': f"Saklama: {st['title']}. ", 'detail': st['detail']},
        'rows': rows,
        'count': f"{sum(1 for r in rows if not r['blocked'])} kişi erişebiliyor",
        'policy': policy_text(lst['tenantKind'], lst['adminsAccessAllProjects']),
    }


def email_problem(email):
    e = email.strip().lower()
    if not e:
        return 'Davet edilecek kişinin e-posta adresini yazın.'
    at = e.find('@')
    domain = e[at + 1:]
    ok = (len(e) <= 254 and not re.search(r'[\s\x00-\x1f\x7f-\x9f]', e) and at > 0 and '@' not in domain
          and '.' in domain and not domain.startswith('.') and not domain.endswith('.'))
    return None if ok else 'Davet için geçerli bir e-posta adresi yazın (ör. ad@kurum.gov.tr).'


def iso_ms(t):
    return t.astimezone(timezone.utc).strftime('%Y-%m-%dT%H:%M:%S.') + f'{t.microsecond // 1000:03d}Z'


def invite_expiry(days, now_iso):
    if days == INVITE_DAYS:
        return None
    now = datetime.fromisoformat(now_iso.replace('Z', '+00:00'))
    wait_ms = min(days * 86_400_000, INVITE_MAX_DAYS * 86_400_000 - 10 * 60_000)
    return iso_ms(now + timedelta(milliseconds=wait_ms))


def end_of_day(date):
    m = re.fullmatch(r'(\d{4})-(\d{2})-(\d{2})', date)
    if not m:
        return None
    try:
        t = datetime(int(m[1]), int(m[2]), int(m[3]), 23, 59, 59, tzinfo=ZONE)
    except ValueError:
        return None
    return iso_ms(t)


def invitation_sub(i):
    by = f"Davet eden: {i['createdByName'] or 'görünmüyor'}, {date_text(i['createdAt'])}"
    s = i['state']
    if s == 'pending':
        return f"{by} · {date_text(i['expiresAt'])} tarihine kadar bekler"
    if s == 'accepted':
        return f"{by} · Kabul eden: {i.get('acceptedByName') or 'görünmüyor'}" + (f", {date_text(i['acceptedAt'])}" if i.get('acceptedAt') else '')
    if s == 'expired':
        return f"{by} · Süresi {date_text(i['expiresAt'])} tarihinde doldu"
    return by


def invite_question(email, invitations, access):
    e = email.strip().lower()
    waiting = next((i for i in invitations if i['state'] == 'pending' and i['email'] == e), None)
    person = next((p for p in (access or {}).get('people', []) if p.get('role') and (p.get('email') or '').lower() == e), None)
    if not waiting and not person:
        return None
    out = {}
    if waiting:
        out['waiting'] = waiting
    if person and person.get('role'):
        out['holder'] = {'name': person['displayName'] or e, 'role': ROLE_LABEL[person['role']]}
    return out


def invite_ask(address, q):
    details = []
    if q.get('waiting'):
        details.append(f"{date_text(q['waiting']['createdAt'])} tarihli bekleyen davet geri alınır; onun bağlantısı artık çalışmaz. Yeni bağlantıyı yeniden iletmeniz gerekir.")
    if q.get('holder'):
        details.append(f"{q['holder']['name']} projeye zaten {q['holder']['role']} olarak erişebiliyor. Davet ancak daha güçlü bir rol verir; rolü düşürmez.")
    return {'title': 'Bekleyen davet var' if q.get('waiting') else 'Zaten erişebiliyor', 'message': f'“{address}” için yeni bir davet gönderilsin mi?',
            'details': details, 'go': 'Yeni davet gönder', 'cancel': 'Vazgeç'}


def accepted_how(a):
    if a['role'] == 'owner':
        return 'Projenin sahibisiniz; davet bir şey değiştirmedi.'
    if a['guest']:
        return 'Misafir olarak: kurumun dışındasınız ve yalnız bu projeyi görürsünüz.'
    if a['tenantKind'] == 'personal':
        return 'Paylaşımla: proje bir kişisel alanda.'
    return 'Kurum üyesi olarak paylaşımla.'


def failure_text(err, failed):
    if 'plain' in err:
        return f"{failed}: {err['plain']}"
    status = err['status']
    code = err.get('error') or ('network' if status == 0 else 'http')
    message = err.get('message') or err['fallback']
    transient = err.get('retryable') is True or status in (0, 408, 502, 503, 504)
    if code == 'network':
        return f'{failed}: sunucuya ulaşılamadı. Bağlantınızı denetleyip yeniden deneyin.'
    if status == 401:
        return f'{failed}: oturumunuz sona erdi. Yeniden giriş yapıp tekrar deneyin.'
    if code == 'not_found' and message == 'Proje bulunamadı.':
        return f'{failed}: proje bulunamadı; silinmiş ya da size erişimi kaldırılmış olabilir. Pencereyi kapatıp proje listesini yenileyin.'
    if transient:
        return f'{failed}: sunucu şu an yanıt vermiyor. Birazdan yeniden deneyin.'
    return f'{failed}: {message}'


# ── The file ─────────────────────────────────────────────────────────

T_PEOPLE = {
    'add': 'Kişi ekle', 'role': 'Rol', 'until': 'Bitiş (isteğe bağlı)', 'untilLabel': 'Bitiş tarihi (isteğe bağlı)', 'share': 'Paylaş',
    'title': 'Erişimi olanlar', 'listLabel': 'Erişimi olanlar', 'loading': 'Erişimi olanlar yükleniyor…', 'you': ' (siz)', 'remove': 'Kaldır',
    'removeLabel': {'sample': 'Mehmet Demir', 'text': 'Mehmet Demir erişimini kaldır'},
    'roleLabel': {'sample': 'Mehmet Demir', 'text': 'Mehmet Demir için rol'},
    'count': {'sample': 3, 'text': '3 kişi erişebiliyor'},
    'adding': {'sample': 'Mehmet Demir', 'text': 'Mehmet Demir ekleniyor…'},
    'changing': {'sample': 'Mehmet Demir', 'text': 'Mehmet Demir için rol değiştiriliyor…'},
    'revoking': {'sample': 'Mehmet Demir', 'text': 'Mehmet Demir için erişim kaldırılıyor…'},
    'badDate': 'Bitiş tarihi okunamadı: takvimden bir gün seçin ya da alanı boş bırakın.',
    'readFailed': 'Erişim listesi okunamadı', 'shareFailed': 'Paylaşılamadı', 'changeFailed': 'Rol değiştirilemedi', 'revokeFailed': 'Erişim kaldırılamadı',
    'guestRole': GUEST_ROLE,
    'storage': {'sample': 'Yönetilen PostGIS veritabanı', 'text': 'Saklama: Yönetilen PostGIS veritabanı. '},
}
T_FIND = {'placeholder': 'Ad ya da e-posta yazın', 'label': 'Paylaşılacak kişi', 'list': 'Bulunan kişiler',
          'has': {'sample': 'Düzenleyici', 'text': 'şu an Düzenleyici'}, 'failed': 'Kişi aranamadı'}
T_INVITES = {
    'email': 'E-postayla davet et', 'emailLabel': 'Davet edilecek e-posta', 'placeholder': 'ad@kurum.gov.tr', 'role': 'Rol', 'roleLabel': 'Davetin rolü',
    'wait': 'Geçerlilik', 'waitLabel': 'Davetin geçerlilik süresi', 'send': 'Davet et', 'title': 'Davetler', 'listLabel': 'Davetler', 'loading': 'Davetler yükleniyor…',
    'empty': 'Bekleyen ya da son 30 günde sonuçlanmış davet yok. Kurum dışından biriyle çalışmak için yukarıdan e-postayla davet edin.',
    'noRight': 'Davetleri görmek ve göndermek için bu projede paylaşım yetkiniz olmalı (project.share).',
    'count': {'sample': 2, 'text': '2 bekliyor'}, 'revoke': 'Geri al',
    'revokeLabel': {'sample': 'ali@ornek.org', 'text': 'ali@ornek.org davetini geri al'},
    'inviting': {'sample': 'ali@ornek.org', 'text': '“ali@ornek.org” davet ediliyor…'},
    'revoking': {'sample': 'ali@ornek.org', 'text': '“ali@ornek.org” için davet geri alınıyor…'},
    'created': 'Davet oluşturuldu. Bağlantıyı kopyalayıp davet ettiğiniz kişiye iletin.',
    'readFailed': 'Davetler okunamadı', 'sendFailed': 'Davet gönderilemedi', 'revokeFailed': 'Davet geri alınamadı', 'linkLabel': 'Davet bağlantısı',
    'copy': 'Kopyala', 'copied': 'Kopyalandı', 'copiedSay': 'Bağlantı panoya kopyalandı; davet ettiğiniz kişiye iletin.',
    'copyFailed': 'Bağlantı panoya kopyalanamadı: alandaki bağlantı seçildi, Ctrl+C ile kopyalayın.',
    'noLink': 'Ancak bağlantı bu yanıtta yok; sunucu onu yalnız ilk yanıtta verir.',
    'noLinkWarn': 'Bağlantıyı almak için aynı adrese yeniden davet gönderin; bu davetin bağlantısı artık çalışmaz.',
}
TEXTS = {'title': 'Projeyi paylaş', 'tabs': {'label': 'Paylaşım', 'people': 'Kişiler', 'invites': 'Davetler'}, 'close': 'Kapat',
         'people': T_PEOPLE, 'find': T_FIND, 'invites': T_INVITES}


def person(**p):
    return {'expired': False, 'guest': False, **p}


ACCESS = {
    'tenantKind': 'organization', 'storage': 'database', 'adminsAccessAllProjects': True, 'ownerId': 'ayse', 'ownerName': 'Ayşe Yılmaz',
    'people': [
        person(userId='ayse', displayName='Ayşe Yılmaz', email='ayse@buro.com.tr', role='owner', via='owner'),
        person(userId='zeynep', displayName='Zeynep Kaya', role='manager', via='policy', grant='viewer'),
        person(userId='dilek', displayName='Dilek Er', role='manager', via='grant', grant='manager'),
        person(userId='mehmet', displayName='Mehmet Demir', email='mehmet@ornek.com.tr', role='editor', via='grant', grant='editor', expiresAt='2026-12-31T20:59:59Z'),
        person(userId='bora', displayName='Bora Tan', blocked='expired', grant='viewer', expiresAt='2026-01-01T12:00:00Z', expired=True),
        person(userId='can', displayName='Çağla Öz', blocked='noSeat', grant='editor'),
        person(userId='irem', displayName='İrem Işık', role='manager', via='policy'),
        person(userId='misafir', displayName='Işıl Nur', email='isil@disari.org', role='editor', via='grant', grant='editor', guest=True),
        person(userId='adsiz', displayName='', role='viewer', via='grant', grant='viewer'),
    ],
}


def access(**over):
    return {**ACCESS, **over}


PEOPLE = [
    ('manager', 'Yönetici olarak (Dilek): başkalarının paylaşımları değişir ve kaldırılır; sahip, kendisi ve politika kilitli; misafirin rolü davetle', 'dilek', True, {}),
    ('owner', 'Sahip olarak (Ayşe)', 'ayse', True, {}),
    ('no-right', 'Paylaşma yetkisi olmayan (listeyi görse de): her satır kilitli', 'mehmet', False, {}),
    ('file-personal', 'Kişisel alanda dosya projesi', 'ayse', True, {'tenantKind': 'personal', 'storage': 'file', 'adminsAccessAllProjects': False}),
    ('policy-off', 'Kurum politikası kapalı', 'dilek', True, {'adminsAccessAllProjects': False}),
]

SOURCES = [
    person(userId='a', displayName='A', role='owner', via='owner'),
    person(userId='b', displayName='B', role='manager', via='policy'),
    person(userId='c', displayName='C', role='manager', via='policy', grant='viewer', expiresAt='2026-06-30T12:00:00Z'),
    person(userId='d', displayName='D', role='editor', via='grant', grant='editor'),
    person(userId='e', displayName='E', role='editor', via='grant', grant='editor', expiresAt='2026-12-31T20:59:59Z'),
    person(userId='f', displayName='F', role='viewer', via='grant', grant='viewer', guest=True),
    person(userId='g', displayName='G', role='viewer', via='grant', grant='viewer', guest=True, expiresAt='2027-01-15T09:00:00Z'),
    person(userId='h', displayName='H', blocked='expired', grant='viewer', expiresAt='2026-01-01T12:00:00Z', expired=True),
    person(userId='i', displayName='I', blocked='notMember', grant='editor'),
    person(userId='j', displayName='J', blocked='inactive'),
    person(userId='k', displayName='K', blocked='guestsOff', grant='commenter', guest=True),
]

INVITATION = {'id': 'inv-1', 'email': 'ali@ornek.org', 'role': 'viewer', 'state': 'pending', 'createdBy': 'dilek', 'createdByName': 'Dilek Er',
              'createdAt': '2026-09-20T09:00:00Z', 'expiresAt': '2026-10-04T09:00:00Z'}


def inv(**over):
    return {**INVITATION, **over}


INVITATIONS = [
    inv(id='a', email='a@ornek.org', state='accepted', createdAt='2026-09-25T12:00:00Z', acceptedByName='Ali Veli', acceptedAt='2026-09-26T08:00:00Z'),
    inv(id='b', email='b@ornek.org', state='pending', createdAt='2026-09-21T12:00:00Z', expiresAt='2026-10-05T12:00:00Z'),
    inv(id='c', email='c@ornek.org', state='revoked', createdAt='2026-09-26T12:00:00Z'),
    inv(id='d', email='d@ornek.org', state='pending', createdAt='2026-09-24T12:00:00Z', expiresAt='2026-10-08T12:00:00Z', role='editor'),
    inv(id='e', email='e@ornek.org', state='expired', createdAt='2026-08-01T12:00:00Z', expiresAt='2026-08-15T12:00:00Z'),
]
SUBS = [
    inv(),
    inv(state='accepted', acceptedByName='Ali Veli', acceptedAt='2026-09-21T10:00:00Z'),
    inv(state='accepted', acceptedByName='', acceptedAt=None),
    inv(state='expired', expiresAt='2026-10-04T09:00:00Z'),
    inv(state='revoked'),
    inv(createdByName=''),
]
SUBS = [{k: v for k, v in s.items() if v is not None} for s in SUBS]

QUESTIONS = [
    ('nothing', 'yeni@ornek.org'),
    ('waiting', ' B@Ornek.org '),
    ('holder', 'mehmet@ornek.com.tr'),
    ('both', 'isil@disari.org'),
    ('ended-invitation', 'e@ornek.org'),
]
Q_INVITATIONS = INVITATIONS + [inv(id='f', email='isil@disari.org', state='pending', createdAt='2026-09-22T12:00:00Z')]

EMAILS = ['', '   ', 'ali@ornek.org', ' Ali@Ornek.ORG ', 'ali', '@ornek.org', 'ali@ornek', 'ali@.org', 'ali@ornek.', 'ali@ornek@org', 'ali veli@ornek.org',
          'ali@ornek.org\t', 'a' * 243 + '@ornek.org', 'a' * 245 + '@ornek.org', 'ayşe@ornek.com.tr']

EXPIRY_NOW = '2026-09-27T09:30:00.000Z'
ENDS = ['2026-12-31', '2026-03-29', '2026-10-25', '2028-02-29', '2026-02-29', '2026-02-30', '2026-04-31', '2026-13-01', '31.12.2026', '', 'yarın']
DATES = ['2026-12-31T20:59:59Z', '2026-12-31T21:00:00Z', '2026-01-01T12:00:00Z', '2026-06-30T23:30:00Z']

ACCEPTED = [
    {'role': 'owner', 'guest': False, 'tenantKind': 'organization'},
    {'role': 'editor', 'guest': True, 'tenantKind': 'organization'},
    {'role': 'viewer', 'guest': False, 'tenantKind': 'personal'},
    {'role': 'manager', 'guest': False, 'tenantKind': 'organization'},
]
ACC_BASE = {'tenantId': 't', 'tenantName': 'Büro', 'projectId': 'p', 'projectName': 'Ada 101'}

FAILURES = [
    ({'status': 0, 'fallback': 'x'}, 'Paylaşılamadı'),
    ({'status': 401, 'error': 'unauthenticated', 'message': 'Oturum yok.'}, 'Paylaşılamadı'),
    ({'status': 404, 'error': 'not_found', 'message': 'Proje bulunamadı.'}, 'Erişim listesi okunamadı'),
    ({'status': 404, 'error': 'not_found', 'message': 'Davet bulunamadı.'}, 'Davet geri alınamadı'),
    ({'status': 422, 'error': 'invalid', 'message': 'Bu kişi “Büro” kurumunun üyesi değil.'}, 'Paylaşılamadı'),
    ({'status': 503, 'error': 'unavailable', 'message': 'x'}, 'Rol değiştirilemedi'),
    ({'status': 429, 'error': 'rate_limited', 'message': 'Çok sık.', 'retryable': True}, 'Davet gönderilemedi'),
    ({'status': 408, 'fallback': 'Zaman aşımı.'}, 'Kişi aranamadı'),
    ({'status': 403, 'error': 'forbidden', 'message': 'Bu projede paylaşım yetkiniz yok.'}, 'Erişim listesi okunamadı'),
    ({'plain': 'bozuk'}, 'Paylaşılamadı'),
]


def envelope(name, tenant, project, inp):
    return {'commandName': name, 'version': 1, 'tenantId': tenant, 'projectId': project, 'expectedVersions': {}, 'input': inp}


ENVELOPES = [
    {'command': 'share', 'args': ['t1', 'p1', 'u1', 'editor'], 'expect': envelope('project.share', 't1', 'p1', {'userId': 'u1', 'role': 'editor'})},
    {'command': 'share', 'args': ['t1', 'p1', 'u1', 'viewer', '2026-12-31T20:59:59.000Z'], 'expect': envelope('project.share', 't1', 'p1', {'userId': 'u1', 'role': 'viewer', 'expiresAt': '2026-12-31T20:59:59.000Z'})},
    {'command': 'revoke', 'args': ['t1', 'p1', 'u1'], 'expect': envelope('project.access.revoke', 't1', 'p1', {'userId': 'u1'})},
    {'command': 'invite', 'args': ['t1', 'p1', ' ali@ornek.org ', 'commenter'], 'expect': envelope('project.invite', 't1', 'p1', {'email': 'ali@ornek.org', 'role': 'commenter'})},
    {'command': 'invite', 'args': ['t1', 'p1', 'ali@ornek.org', 'editor', '2026-10-04T09:30:00.000Z'], 'expect': envelope('project.invite', 't1', 'p1', {'email': 'ali@ornek.org', 'role': 'editor', 'expiresAt': '2026-10-04T09:30:00.000Z'})},
    {'command': 'invitationRevoke', 'args': ['t1', 'p1', 'inv-1'], 'expect': envelope('project.invitation.revoke', 't1', 'p1', {'invitationId': 'inv-1'})},
]

TOKEN = '0123456789abcdef' * 4

file = {
    'format': 'kentos.share',
    'version': 1,
    'timeZone': TZ,
    'roles': {'labels': ROLE_LABEL, 'grant': GRANT_ROLES, 'invite': INVITE_ROLES, 'hints': ROLE_HINT},
    'blocks': BLOCK_TEXT,
    'storage': STORAGE_TEXT,
    'invitationStates': STATE_LABEL,
    'inviteDays': {'default': INVITE_DAYS, 'max': INVITE_MAX_DAYS, 'choices': [{'days': d, 'text': f'{d} gün (varsayılan)' if d == INVITE_DAYS else f'{d} gün'} for d in DAY_CHOICES]},
    'texts': TEXTS,
    'access': ACCESS,
    'people': [{'id': i, 'title': t, 'me': me, 'mayShare': may, 'list': over, 'expect': people_view(access(**over), me, may)} for i, t, me, may, over in PEOPLE],
    'sources': [{'holder': p, 'text': source_text(p)} for p in SOURCES],
    'shareTips': [{'mayShare': m, 'chosen': c, 'text': ('Bu projede paylaşım yetkiniz yok (project.share).' if not m else '' if c else 'Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin.')}
                  for m in (False, True) for c in (False, True)],
    'shared': [{'name': 'Mehmet Demir', 'role': r, 'changed': ch, 'had': had,
                'text': (f'Mehmet Demir zaten {ROLE_LABEL[r]} rolündeydi; değişen bir şey yok.' if not ch else f'Mehmet Demir artık {ROLE_LABEL[r]}.' if had else f'Mehmet Demir projeye {ROLE_LABEL[r]} olarak eklendi.')}
               for r, ch, had in [('editor', False, True), ('viewer', True, True), ('manager', True, False), ('commenter', True, False)]],
    'revokeQuestions': [{'name': n, 'project': 'Ada 101', 'guest': g, 'expect': {
        'title': 'Erişimi kaldır', 'message': f'{n}, “Ada 101” projesine artık erişemesin mi?',
        'details': ['Projeyi şu anda açık tutuyorsa kaydı hemen durur; gönderilmemiş değişiklikleri kendi cihazında kalır.', 'Daha önce indirdiği kopyalar ve ekranında gördükleri geri alınamaz.',
                    'Kurum dışından olduğu için erişimini yeniden davetle geri verebilirsiniz.' if g else 'Yeniden paylaşarak erişimini geri verebilirsiniz.'],
        'action': 'Erişimi kaldır'}} for n, g in [('Mehmet Demir', False), ('Işıl Nur', True)]],
    'finder': {
        'nobody': [{'query': q, 'personal': pe, 'expect': {
            'text': (f'“{q}” ile eşleşen kimse yok. Kurumlarınızın dışından biriyle “Davetler”den e-postayla paylaşabilirsiniz.' if pe else f'“{q}” ile eşleşen etkin bir kurum üyesi yok. Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.'),
            'invite': None if email_problem(q) else f'“{q.strip()}” adresine e-postayla davet gönder…'}}
            for q, pe in [('zz', False), ('ali@ornek.org', False), ('ali@ornek.org', True), ('ali@ornek', True)]],
        'notes': [{'role': r, 'text': f'şu an {ROLE_LABEL[r]}' if r else None} for r in [None, 'owner', 'viewer', 'manager']],
        'searches': [{'query': q, 'searches': len(re.sub(r'\s+', '', q.strip())) >= 2} for q in ['', 'a', ' a ', 'a b', 'ab', 'Ay']],
    },
    'emails': [{'email': e, 'problem': email_problem(e)} for e in EMAILS],
    'expiry': [{'days': d, 'now': EXPIRY_NOW, 'expiresAt': invite_expiry(d, EXPIRY_NOW)} for d in DAY_CHOICES + [2, 91]],
    'endOfDay': [{'date': d, 'expiresAt': end_of_day(d)} for d in ENDS],
    'dates': [{'iso': d, 'text': date_text(d)} for d in DATES],
    'inviteTips': [{'mayShare': m, 'email': e, 'text': ('Bu projede davet yetkiniz yok (project.share).' if not m else '' if e.strip() else 'Önce davet edilecek e-posta adresini yazın.')}
                   for m, e in [(False, 'ali@ornek.org'), (True, ''), (True, '  '), (True, 'ali@ornek.org')]],
    'invitations': {
        'sorted': {'list': INVITATIONS, 'order': [i['id'] for i in sorted(INVITATIONS, key=lambda i: (0 if i['state'] == 'pending' else 1, -datetime.fromisoformat(i['createdAt'].replace('Z', '+00:00')).timestamp()))]},
        'counts': [{'states': s, 'text': (f"{sum(1 for x in s if x == 'pending')} bekliyor" if s else '')} for s in [[], ['accepted'], ['pending', 'accepted', 'pending']]],
        'subs': [{'invitation': s, 'text': invitation_sub(s)} for s in SUBS],
        'questions': [{'id': i, 'email': e, 'expect': (lambda q: None if q is None else {'question': q, 'ask': invite_ask(e.strip(), q)})(invite_question(e, Q_INVITATIONS, ACCESS))} for i, e in QUESTIONS],
        'questionInvitations': Q_INVITATIONS,
        'rules': {'personal': 'Davet edilen, bağlantıyı açıp davetin gönderildiği e-postanın hesabıyla girince projeye paylaşımla erişir. Bağlantıyı siz iletirsiniz; KentOS e-posta göndermez.',
                  'organization': 'Davet edilen, bağlantıyı açıp davetin gönderildiği e-postanın hesabıyla girince projeye erişir: kurumun üyesiyse paylaşımla, değilse misafir olarak (yalnız bu projeyi görür; rolü en çok Düzenleyici; kurum misafir almıyorsa kabul edilmez). Bağlantıyı siz iletirsiniz; KentOS e-posta göndermez.'},
        'links': [
            {'change': {'invitation': inv(role='editor', expiresAt='2026-10-04T09:00:00Z'), 'token': TOKEN, 'replayed': False}, 'days': 14, 'expect': {
                'what': '“ali@ornek.org” davet edildi: Düzenleyici, 04.10.2026 tarihine kadar bekler.',
                'warn': 'Bu bağlantı yalnız şimdi gösterilir: sunucu onu saklamaz, pencere kapanınca yeniden gösterilemez. Kopyalayıp davet ettiğiniz kişiye kendiniz iletin; 14 gün içinde, bir kez kullanılabilir. Kaybederseniz yeniden davet edin (eski bağlantı çalışmaz olur).',
                'link': True}},
            {'change': {'invitation': inv(), 'replayed': True}, 'days': 7, 'expect': {
                'what': '“ali@ornek.org” davet edildi: Görüntüleyici, 04.10.2026 tarihine kadar bekler. Ancak bağlantı bu yanıtta yok; sunucu onu yalnız ilk yanıtta verir.',
                'warn': 'Bağlantıyı almak için aynı adrese yeniden davet gönderin; bu davetin bağlantısı artık çalışmaz.', 'link': False}},
        ],
        'revokeQuestion': {'email': 'ali@ornek.org', 'expect': {'title': 'Daveti geri al', 'message': '“ali@ornek.org” adresine gönderilen davet geri alınsın mı?',
                                                                  'details': ['Davetin bağlantısı artık çalışmaz; açan kişi projeye erişemez.', 'Kişiye ayrıca haber vermeniz gerekmez; isterseniz daha sonra yeniden davet edebilirsiniz.'],
                                                                  'action': 'Daveti geri al', 'cancel': 'Vazgeç'}},
        'initials': [{'email': e, 'text': t} for e, t in [('ali@ornek.org', 'A'), ('isil@disari.org', 'İ'), ('', '?')]],
        'link': {'token': TOKEN, 'base': 'https://kentos.example/app/', 'link': 'https://kentos.example/app/?' + urlencode({'davet': TOKEN})},
    },
    'accepted': [{'accepted': {**ACC_BASE, **a}, 'text': accepted_how(a)} for a in ACCEPTED],
    'envelopes': ENVELOPES,
    'failures': [{'error': e, 'failed': f, 'text': failure_text(e, f)} for e, f in FAILURES],
    'lines': [
        {'line': 'shareLog', 'args': ['Ada 101', 'Mehmet Demir artık Yönetici.'], 'text': '“Ada 101”: Mehmet Demir artık Yönetici.'},
        {'line': 'roleChanged', 'args': ['Mehmet Demir', 'manager'], 'text': 'Mehmet Demir artık Yönetici.'},
        {'line': 'revoked', 'args': ['Mehmet Demir'], 'text': 'Mehmet Demir artık projeye erişemiyor.'},
        {'line': 'invitedLog', 'args': ['Ada 101', 'ali@ornek.org', 'editor'], 'text': '“Ada 101”: “ali@ornek.org” Düzenleyici olarak davet edildi.'},
        {'line': 'invitationRevokedLog', 'args': ['Ada 101', 'ali@ornek.org'], 'text': '“Ada 101”: “ali@ornek.org” için davet geri alındı.'},
        {'line': 'invitationRevokedSay', 'args': ['ali@ornek.org'], 'text': '“ali@ornek.org” için davet geri alındı; bağlantısı artık çalışmaz.'},
    ],
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
