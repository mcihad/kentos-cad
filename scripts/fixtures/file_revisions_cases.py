"""An open file project's revisions for both platforms: what the drawing
knows of the server's newest revision and how it learns it, the save cell's
state, what Kaydet does first, the resync, the questions and what their
answers do, and the words (docs/specs/file-revisions.md; format in
fixtures/cloud/README.md; docs/adr/0038).

    python3 scripts/fixtures/file_revisions_cases.py           # writes the file
    python3 scripts/fixtures/file_revisions_cases.py --check   # writes nothing; compares

Writes fixtures/cloud/v1/file-revisions.json. The words are the web's,
written here by hand; every answer below is worked out here, apart from the
web's TypeScript (apps/web/src/app/cloud/fileRevisionsPlan.ts), which
apps/web/src/app/cloud/fileRevisionsPlan.test.ts holds to the file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys
from datetime import datetime
from zoneinfo import ZoneInfo

OUT = 'fixtures/cloud/v1/file-revisions.json'
TZ = 'Europe/Istanbul'


# ── Words ───────────────────────────────────────────────────────────────────

def when(iso):
    """The interface's date and time: gg.aa.yyyy ss:dd in the device's zone (here the file's)."""
    return datetime.fromisoformat(iso.replace('Z', '+00:00')).astimezone(ZoneInfo(TZ)).strftime('%d.%m.%Y %H:%M')


def who(n):
    parts = [p for p in (n['by'], when(n['at']) if n['at'] else '') if p]
    return f' ({", ".join(parts)})' if parts else ''


def open_at(base, lower=False):
    a = 'a' if lower else 'A'
    return f'{a}çık çizimin henüz revizyonu yok' if base == '0' else f'{a}çık çizim revizyon {base}'


TEXTS = {
    'busy': lambda name: f'“{name}” kaydediliyor; son revizyonu açmak için kaydın bitmesini bekleyin.',
    'unreadableDeleted': lambda name: f'“{name}” bulut projesi silindi; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin.',
    'unreadableRevoked': lambda name: f'“{name}” projesine erişiminiz kaldırıldı; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin.',
    'resyncFailed': lambda name: f'“{name}” projesinin kaçırılan olayları alınamadı ve sunucu yanıt vermedi; başkasının kaydettiği bir revizyon şimdilik bilinmiyor. Yarım dakika sonra yeniden sorulur; çizim olduğu gibi duruyor.',
    'detached': lambda name: f'Çizim yerel dosyaya kaydedildi ve “{name}” bulut projesinden ayrıldı; proje olduğu gibi duruyor.',
    'openFailed': lambda name, why: f'“{name}” son revizyonu açılamadı: {why}',
}
MARKS = {'newest': 'En yeni', 'base': 'Açık çizim'}
RESYNC_RETRY_MS = 30000


def newer_line(name, newer, base, dirty):
    on = 'Açık çizimin henüz revizyonu yok' if base == '0' else f'Açık çizimin dayandığı revizyon: {base}'
    then = ('kaydedilmemiş değişiklikleriniz Kaydet ile bu revizyonun üzerine yazılamaz. Ayrı kopya, yerel dosya ya da son revizyon için durum çubuğundaki kayıt durumuna tıklayın.'
            if dirty else 'yeni revizyonu açmak için durum çubuğundaki kayıt durumuna tıklayın.')
    return f'“{name}” başka bir yerde kaydedildi: revizyon {newer["revision"]}{who(newer)}. {on}; {then} Kendiliğinden yeniden yüklenmez.'


def newer_tip(newer, dirty):
    head = f'Sunucuda daha yeni revizyon var: {newer["revision"]}{who(newer)}'
    return f'{head}; kaydedilmemiş değişiklikleriniz Kaydet ile onun üzerine yazılamaz. Seçenekler için tıklayın.' if dirty else f'{head}; açmak için tıklayın.'


def marks(revision, current, open_base):
    return ([MARKS['newest']] if revision == current else []) + ([MARKS['base']] if revision == open_base else [])


# ── The state ───────────────────────────────────────────────────────────────

def opened(base, dirty=False, writable=True):
    return {'base': base, 'newer': None, 'conflict': None, 'dirty': dirty, 'stage': 'idle', 'failed': False, 'writable': writable, 'ended': None}


def nr(revision, by='', at=None):
    return {'revision': revision, 'by': by, 'at': at}


def step(s, i):
    same = (s, False)
    k = i['kind']
    if s['ended'] and k != 'dirty':
        return same
    if k == 'dirty':
        return same if i['dirty'] == s['dirty'] else ({**s, 'dirty': i['dirty']}, False)
    if k == 'newest':
        n = i['newest']
        if n is None or int(n['revision']) <= int(s['base']):
            return same
        known = s['newer']
        if known and int(n['revision']) < int(known['revision']):
            return same
        if known and n['revision'] == known['revision']:
            by = known['by'] or n['by']
            at = known['at'] if known['at'] is not None else n['at']
            if by == known['by'] and at == known['at']:
                return same
            return {**s, 'newer': nr(known['revision'], by, at)}, False
        conflict = {'expected': s['conflict']['expected'], 'actual': n['revision']} if s['conflict'] else None
        return {**s, 'newer': n, 'conflict': conflict}, True
    if k == 'stage':
        return {**s, 'stage': i['stage'], 'failed': False if i['stage'] == 'encoding' else s['failed']}, False
    if k == 'committed':
        newer = s['newer'] if s['newer'] and int(s['newer']['revision']) > int(i['revision']) else None
        return {**s, 'base': i['revision'], 'newer': newer, 'dirty': i['dirty'], 'stage': 'idle', 'failed': False}, False
    if k == 'refused':
        newer = s['newer'] if s['newer'] and int(s['newer']['revision']) >= int(i['actual']) else nr(i['actual'])
        return {**s, 'conflict': {'expected': s['base'], 'actual': newer['revision']}, 'newer': newer, 'stage': 'idle'}, False
    if k == 'failed':
        return {**s, 'stage': 'idle', 'failed': True}, False
    if k == 'unchanged':
        return ({**s, 'failed': False}, False) if s['failed'] else same
    if k == 'access':
        return same if i['writable'] == s['writable'] else ({**s, 'writable': i['writable']}, False)
    if k == 'ended':
        return {**s, 'ended': i['why'], 'stage': 'idle'}, False
    raise ValueError(k)


def cell_state(s):
    if s['ended']:
        return s['ended']
    if s['stage'] != 'idle':
        return s['stage']
    if s['conflict']:
        return 'conflict'
    if s['newer']:
        return 'outdated'
    if not s['writable']:
        return 'readonly'
    if s['failed']:
        return 'error'
    return 'pending' if s['dirty'] else 'saved'


def save_step(s):
    if s['ended']:
        return 'ended'
    if not s['writable']:
        return 'readonly'
    if s['conflict']:
        return 'conflict'
    if not s['dirty'] and s['base'] != '0':
        return 'unchanged'
    if s['newer']:
        return 'behind'
    return 'save'


# The save cell, as the status bar says it (ui/statusbar/cellsPlan.ts; progress 0 here).
def cell_text(state, s):
    base = s['base']
    newer = s['newer']['revision'] if s['newer'] else '?'
    actual = s['conflict']['actual'] if s['conflict'] else '?'
    return {
        'saved': 'Henüz revizyon yok' if base == '0' else f'Buluta kaydedildi · r{base}',
        'pending': 'Kaydedilmedi' if base == '0' else f'Kaydedilmedi · r{base} üstüne',
        'encoding': 'Dosya hazırlanıyor…',
        'uploading': 'Yükleniyor %0',
        'verifying': 'Sunucu doğruluyor…',
        'conflict': f'Çakışma: r{actual} kaydedilmiş',
        'outdated': f'Yeni revizyon: r{newer}' + (' · kaydedilmedi' if s['dirty'] else ''),
        'error': 'Kayıt hatası',
        'readonly': 'Salt okunur',
        'deleted': 'Proje silindi',
        'revoked': 'Erişim kaldırıldı',
        'archived': 'Proje arşivde',
    }[state]


def cell_action(state):
    if state == 'conflict':
        return 'cloud.conflicts'
    if state == 'outdated':
        return 'cloud.openNewest'
    if state in ('readonly', 'encoding', 'uploading', 'verifying'):
        return None
    return 'file.save'


def cell(s):
    state = cell_state(s)
    return {'state': state, 'text': cell_text(state, s), 'action': cell_action(state)}


# ── Events and the newest revision ──────────────────────────────────────────

def read_events(events, own):
    cursor, access, newest = None, False, False
    for e in events:
        cursor = e['seq']
        mine = bool(e.get('requestId')) and e['requestId'] in own
        if e['kind'] == 'project.deleted':
            return {'cursor': cursor, 'end': {'why': 'deleted', 'quiet': False}, 'access': False, 'newest': False}
        if e['kind'] == 'project.archived':
            return {'cursor': cursor, 'end': {'why': 'archived', 'quiet': mine}, 'access': False, 'newest': False}
        if e['kind'] == 'project.access':
            access = True
        if e['kind'] == 'project.file' and not mine:
            newest = True
    return {'cursor': cursor, 'end': None, 'access': access, 'newest': newest}


def newest_of(r):
    if not r.get('current'):
        return None
    listed = next((x for x in r['revisions'] if x['revision'] == r['current']), None)
    return nr(r['current'], listed['createdByName'] if listed else '', listed['createdAt'] if listed else None)


def resync_step(a):
    k = a['kind']
    if k == 'project':
        if a['state'] == 'trashed':
            return {'kind': 'end', 'why': 'deleted', 'reason': ''}
        if a['state'] == 'archived':
            return {'kind': 'end', 'why': 'archived', 'reason': ''}
        return {'kind': 'follow', 'cursor': a['eventCursor']}
    if k == 'deleted':
        return {'kind': 'end', 'why': 'deleted', 'reason': ''}
    if k == 'notFound':
        return {'kind': 'end', 'why': 'revoked', 'reason': ''}
    if k == 'forbidden':
        return {'kind': 'end', 'why': 'revoked', 'reason': a['message']}
    return {'kind': 'retry'}


# ── Questions ───────────────────────────────────────────────────────────────

def answer(value, label, does, work, kind=None, aside=False):
    a = {'value': value, 'label': label}
    if kind:
        a['kind'] = kind
    if aside:
        a['aside'] = True
    a['does'] = does
    a['work'] = work
    return a


def conflict_question(name, s):
    c = s['conflict'] or ({'expected': s['base'], 'actual': s['newer']['revision']} if s['newer'] else None)
    if not c:
        return None
    # A standing conflict is always with the newer revision known (step): its who and when.
    assert not s['conflict'] or (s['newer'] and s['newer']['revision'] == s['conflict']['actual']), s
    w = who(s['newer']) if s['newer'] else ''
    on = 'çiziminiz henüz kaydedilmiş bir revizyona dayanmıyor' if c['expected'] == '0' else f'çiziminizin dayandığı revizyon {c["expected"]}'
    dirty = s['dirty']
    work = (lambda had: had if dirty else 'none')
    return {
        'id': 'conflict',
        'title': 'Dosya başka biri tarafından kaydedildi',
        'message': f'“{name}” siz çalışırken başka biri tarafından kaydedildi: sunucuda revizyon {c["actual"]}{w} var; {on}. Hiçbir şey yazılmadı; iki dosya birleştirilmez.',
        'details': [
            f'Ayrı kopya olarak kaydet: çiziminiz yeni bir bulut dosya projesi olur ve açık proje o olur; “{name}” olduğu gibi kalır.',
            'Yerel dosyaya kaydet: çiziminiz bu bilgisayara .kcad olarak kaydedilir ve çizim buluttaki projeden ayrılır.',
            f'Son revizyonu aç: revizyon {c["actual"]} açılır; bu çizimdeki kaydedilmemiş değişiklikler atılır.' if dirty else f'Son revizyonu aç: revizyon {c["actual"]} açılır.',
        ],
        'answers': [
            answer('latest', 'Son revizyonu aç', 'latest', work('dropped'), 'danger', True),
            answer('stay', 'Vazgeç', 'nothing', work('kept')),
            answer('local', 'Yerel dosyaya kaydet', 'local', work('saved')),
            answer('copy', 'Ayrı kopya olarak kaydet', 'copy', work('saved'), 'primary'),
        ],
        'cancel': 'stay',
    }


def unsaved_question(name, base):
    return {
        'id': 'unsaved',
        'title': 'Kaydedilmemiş değişiklikler',
        'message': f'“{name}” içinde kaydedilmemiş değişiklikler var. Sunucudaki en yeni revizyon açılırsa bu değişiklikler atılır ({open_at(base, True)}). Saklamak için önce Kaydet ile kaydedin.',
        'details': [],
        'answers': [
            answer('discard', 'Kaydetmeden aç', 'latest', 'dropped', aside=True),
            answer('stay', 'Vazgeç', 'nothing', 'kept'),
        ],
        'cancel': 'stay',
    }


def newest_question(name, base, newer):
    message = (f'“{name}” başka bir yerde kaydedildi: revizyon {newer["revision"]}{who(newer)}. {open_at(base)}; kaydedilmemiş değişikliği yok.'
               if newer else f'“{name}” projesinin sunucudaki en yeni revizyonu açılsın mı? {open_at(base)}; kaydedilmemiş değişikliği yok.')
    return {
        'id': 'newest',
        'title': 'Son revizyonu aç',
        'message': message,
        'details': [],
        'answers': [
            answer('later', 'Sonra', 'nothing', 'none'),
            answer('open', 'Son revizyonu aç', 'latest', 'none', 'primary'),
        ],
        'cancel': 'later',
    }


def offer(name, s, busy, via):
    def ask(q):
        return {'kind': 'ask', 'question': q} if q else {'kind': 'none'}
    if via == 'conflict':
        return ask(conflict_question(name, s))
    if busy:
        return {'kind': 'say', 'tone': 'info', 'line': TEXTS['busy'](name)}
    if s['ended'] == 'deleted':
        return {'kind': 'say', 'tone': 'warn', 'line': TEXTS['unreadableDeleted'](name)}
    if s['ended'] == 'revoked':
        return {'kind': 'say', 'tone': 'warn', 'line': TEXTS['unreadableRevoked'](name)}
    if s['conflict'] or (s['dirty'] and s['newer']):
        return ask(conflict_question(name, s))
    if s['dirty']:
        return ask(unsaved_question(name, s['base']))
    return ask(newest_question(name, s['base'], s['newer']))


# ── The tip (ui/statusbar/cellsPlan.ts fileTip) ─────────────────────────────

LINK = {'none': '', 'connecting': 'bağlanıyor', 'online': 'canlı', 'reconnecting': 'yeniden bağlanıyor', 'offline': 'çevrimdışı', 'auth_required': 'oturum gerekli'}


def js_round(x):
    """JavaScript's Math.round: halves go up (Python's round goes to even)."""
    return math.floor(x + 0.5)


def ago(at, now):
    if at is None:
        return 'henüz yok'
    s = js_round((now - at) / 1000)
    return f'{s} sn önce' if s < 60 else f'{js_round(s / 60)} dk önce'


def file_tip(t):
    link = LINK[t['link']]
    last, newer = t['lastSaved'], t['newer']
    lines = [
        f'{t["where"]}, dosya olarak saklanıyor (KCAD revizyonları).',
        'Projenin henüz revizyonu yok.' if t['base'] == '0' else f"Çizimin dayandığı revizyon: {t['base']}.",
        'Kendiliğinden kaydedilmez: Kaydet (Ctrl+S) yeni bir revizyon yazar; arada başkası kaydettiyse üzerine yazılmaz.',
        f'Bu pencerenin son kaydı: revizyon {last["revision"]}, {ago(last["at"], t["now"])}.' if last else '',
        newer_tip({**newer, 'at': newer.get('at')}, t['dirty']) if newer else '',
        t['error'],
        f'Canlı bağlantı: {link}.' if link else '',
        'Kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor.' if t['dirty'] else '',
    ]
    return {'title': 'Bulut kaydı: dosya projesi', 'description': ' '.join(x for x in lines if x)}


# ── The cases ───────────────────────────────────────────────────────────────

NAME = 'Kadastro paftası 2026'
AT = '2026-09-27T11:32:00Z'
AT2 = '2026-09-27T08:05:00Z'
MEHMET = nr('5', 'Mehmet Demir', AT)
S4 = opened('4')


def with_(s, **kw):
    return {**s, **kw}


CELL_STATES = [
    ('opened at a revision, clean', S4),
    ('without a revision yet', opened('0')),
    ('unsaved changes', with_(S4, dirty=True)),
    ('a newer revision, clean', with_(S4, newer=MEHMET)),
    ('a newer revision over unsaved work', with_(S4, newer=MEHMET, dirty=True)),
    ('a newer revision while read-only: offered all the same', with_(S4, newer=MEHMET, writable=False)),
    ('read-only', with_(S4, writable=False, dirty=True)),
    ('a conflict outranks the newer revision', with_(S4, newer=MEHMET, conflict={'expected': '4', 'actual': '5'}, dirty=True)),
    ('a Kaydet on its way outranks a conflict-to-be', with_(S4, newer=MEHMET, dirty=True, stage='uploading')),
    ('the last Kaydet failed', with_(S4, dirty=True, failed=True)),
    ('a newer revision outranks the last failure', with_(S4, dirty=True, failed=True, newer=MEHMET)),
    ('ended outranks everything', with_(S4, newer=MEHMET, dirty=True, conflict={'expected': '4', 'actual': '5'}, ended='revoked')),
    ('archived', with_(S4, ended='archived', writable=False)),
]

STEPS = [
    ('an edit', S4, {'kind': 'dirty', 'dirty': True}),
    ('the same dirty again: nothing', with_(S4, dirty=True), {'kind': 'dirty', 'dirty': True}),
    ('a newer revision heard: said', S4, {'kind': 'newest', 'newest': MEHMET}),
    ('the drawing\'s own revision: nothing', S4, {'kind': 'newest', 'newest': nr('4', 'Ayşe Yılmaz', AT2)}),
    ('an older one: nothing', S4, {'kind': 'newest', 'newest': nr('3', 'Ayşe Yılmaz', AT2)}),
    ('none yet: nothing', opened('0'), {'kind': 'newest', 'newest': None}),
    ('the first revision of a project opened without one: said', opened('0', dirty=True), {'kind': 'newest', 'newest': nr('1', 'Mehmet Demir', AT)}),
    ('the same newer revision again: said once', with_(S4, newer=MEHMET), {'kind': 'newest', 'newest': MEHMET}),
    ('a late, older answer never lowers it', with_(S4, newer=nr('6', 'Zeynep Kaya', AT)), {'kind': 'newest', 'newest': MEHMET}),
    ('a later revision: said again', with_(S4, newer=MEHMET), {'kind': 'newest', 'newest': nr('6', 'Zeynep Kaya', AT)}),
    ('who and when filled in after a refusal named only the number: not said again', with_(S4, newer=nr('5'), conflict={'expected': '4', 'actual': '5'}, dirty=True), {'kind': 'newest', 'newest': MEHMET}),
    ('a later revision over a standing conflict: the conflict is with it', with_(S4, newer=MEHMET, conflict={'expected': '4', 'actual': '5'}, dirty=True), {'kind': 'newest', 'newest': nr('6', 'Zeynep Kaya', AT)}),
    ('Kaydet begins: the last failure is forgotten', with_(S4, dirty=True, failed=True), {'kind': 'stage', 'stage': 'encoding'}),
    ('uploading', with_(S4, dirty=True, stage='encoding'), {'kind': 'stage', 'stage': 'uploading'}),
    ('committed, clean', with_(S4, dirty=True, stage='verifying'), {'kind': 'committed', 'revision': '5', 'dirty': False}),
    ('committed with edits made meanwhile', with_(S4, dirty=True, stage='verifying'), {'kind': 'committed', 'revision': '5', 'dirty': True}),
    ('committed under a later revision someone else saved meanwhile', with_(S4, dirty=True, stage='verifying', newer=nr('6', 'Zeynep Kaya', AT)), {'kind': 'committed', 'revision': '5', 'dirty': False}),
    ('refused: the server names the number only', with_(S4, dirty=True, stage='verifying'), {'kind': 'refused', 'actual': '5'}),
    ('refused: who and when known stay', with_(S4, dirty=True, newer=MEHMET), {'kind': 'refused', 'actual': '5'}),
    ('refused below a later revision known: the conflict is with the later one', with_(S4, dirty=True, stage='verifying', newer=nr('6', 'Zeynep Kaya', AT)), {'kind': 'refused', 'actual': '5'}),
    ('failed', with_(S4, dirty=True, stage='uploading'), {'kind': 'failed'}),
    ('unchanged: the failure is forgotten', with_(S4, failed=True), {'kind': 'unchanged'}),
    ('unchanged without a failure: nothing', S4, {'kind': 'unchanged'}),
    ('the right to write taken', with_(S4, dirty=True), {'kind': 'access', 'writable': False}),
    ('the right to write back', with_(S4, dirty=True, writable=False), {'kind': 'access', 'writable': True}),
    ('deleted', with_(S4, dirty=True, newer=MEHMET), {'kind': 'ended', 'why': 'deleted'}),
    ('after the end a revision changes nothing', with_(S4, ended='deleted'), {'kind': 'newest', 'newest': MEHMET}),
    ('after the end a second end changes nothing', with_(S4, ended='archived'), {'kind': 'ended', 'why': 'deleted'}),
    ('after the end the drawing\'s changes still count', with_(S4, ended='deleted'), {'kind': 'dirty', 'dirty': True}),
]

SAVE_STEPS = [
    ('nothing is saved after the end', with_(S4, ended='deleted', dirty=True)),
    ('read-only', with_(S4, writable=False, dirty=True)),
    ('a standing conflict: asked again', with_(S4, conflict={'expected': '4', 'actual': '5'}, newer=MEHMET, dirty=True)),
    ('clean at a revision: unchanged', S4),
    ('clean and a newer revision: unchanged (nothing to write)', with_(S4, newer=MEHMET)),
    ('unsaved and a newer revision: behind, nothing uploaded', with_(S4, newer=MEHMET, dirty=True)),
    ('unsaved: saved', with_(S4, dirty=True)),
    ('without a revision yet, clean: its first is written', opened('0')),
    ('without a revision yet, someone saved the first: behind', with_(opened('0'), newer=nr('1', 'Mehmet Demir', AT))),
]

MINE = ['web-kaydet-1', 'web-arsiv-1']
EVENTS = [
    ('nothing', []),
    ('someone else\'s revision', [{'seq': '41', 'kind': 'project.file', 'requestId': 'baskasi-1'}]),
    ('this window\'s own revision', [{'seq': '41', 'kind': 'project.file', 'requestId': 'web-kaydet-1'}]),
    ('a revision without a request id is someone else\'s', [{'seq': '41', 'kind': 'project.file'}]),
    ('two revisions: the newest is asked once', [{'seq': '41', 'kind': 'project.file', 'requestId': 'baskasi-1'}, {'seq': '42', 'kind': 'project.file', 'requestId': 'baskasi-2'}]),
    ('a grant changed, then a revision', [{'seq': '41', 'kind': 'project.access', 'requestId': 'yonetici'}, {'seq': '42', 'kind': 'project.file', 'requestId': 'baskasi-1'}]),
    ('others\' events of no concern here', [{'seq': '41', 'kind': 'project.metadata', 'requestId': 'baskasi-1'}, {'seq': '42', 'kind': 'project.checkpoint', 'requestId': 'baskasi-2'}]),
    ('deleted: nothing after it counts', [{'seq': '41', 'kind': 'project.file', 'requestId': 'baskasi-1'}, {'seq': '42', 'kind': 'project.deleted', 'requestId': 'baskasi-2'}, {'seq': '43', 'kind': 'project.access', 'requestId': 'yonetici'}]),
    ('archived by someone else', [{'seq': '41', 'kind': 'project.archived', 'requestId': 'baskasi-1'}]),
    ('archived by this window: quiet', [{'seq': '41', 'kind': 'project.archived', 'requestId': 'web-arsiv-1'}]),
]


def rev(revision, by, at):
    return {'revision': revision, 'size': 2048, 'sha256': '0' * 64, 'createdBy': 'u2', 'createdByName': by, 'createdAt': at}


NEWEST = [
    ('none yet', {'revisions': []}),
    ('the newest, with who and when', {'current': '5', 'revisions': [rev('5', 'Mehmet Demir', AT), rev('4', 'Ayşe Yılmaz', AT2)]}),
    ('the newest not in the list: its number only', {'current': '7', 'revisions': [rev('5', 'Mehmet Demir', AT)]}),
    ('a name that does not show', {'current': '5', 'revisions': [rev('5', '', AT)]}),
]

RESYNC = [
    ('the project as it was: followed from its cursor now', {'kind': 'project', 'state': 'active', 'eventCursor': '118'}),
    ('archived meanwhile', {'kind': 'project', 'state': 'archived', 'eventCursor': '118'}),
    ('in the trash (a manager sees it)', {'kind': 'project', 'state': 'trashed', 'eventCursor': '118'}),
    ('deleted (410)', {'kind': 'deleted'}),
    ('gone for this account (404)', {'kind': 'notFound'}),
    ('its organisation may not be used now (403)', {'kind': 'forbidden', 'message': 'Kurum üyeliğiniz etkin değil.'}),
    ('no answer', {'kind': 'unreachable'}),
    ('refused otherwise', {'kind': 'failed', 'message': 'Sunucu hatası.'}),
]

OFFERS = [
    ('a newer revision, clean: the plain question', with_(S4, newer=MEHMET), False, 'newest'),
    ('Son revizyonu aç with no newer revision known, clean', S4, False, 'newest'),
    ('without a revision yet, clean', opened('0'), False, 'newest'),
    ('unsaved work, no newer revision known: the unsaved question', with_(S4, dirty=True), False, 'newest'),
    ('unsaved work without a revision yet', with_(opened('0'), dirty=True), False, 'newest'),
    ('a newer revision over unsaved work: the conflict\'s question', with_(S4, newer=MEHMET, dirty=True), False, 'newest'),
    ('a standing conflict: the conflict\'s question', with_(S4, newer=MEHMET, conflict={'expected': '4', 'actual': '5'}, dirty=True), False, 'newest'),
    ('a Kaydet on its way: a line, no question', with_(S4, newer=MEHMET, dirty=True, stage='uploading'), True, 'newest'),
    ('deleted: a line', with_(S4, ended='deleted', dirty=True), False, 'newest'),
    ('access taken: a line', with_(S4, ended='revoked'), False, 'newest'),
    ('archived: the newest can still be read', with_(S4, ended='archived', writable=False, newer=None), False, 'newest'),
    ('a refused Kaydet: the conflict\'s question, who not known yet', with_(S4, newer=nr('5'), conflict={'expected': '4', 'actual': '5'}, dirty=True), False, 'conflict'),
    ('a refused Kaydet, who known', with_(S4, newer=MEHMET, conflict={'expected': '4', 'actual': '5'}, dirty=True), False, 'conflict'),
    ('a conflict over a clean drawing (its changes undone): nothing to drop', with_(S4, newer=MEHMET, conflict={'expected': '4', 'actual': '5'}), False, 'conflict'),
    ('a first revision saved elsewhere before this drawing\'s', with_(opened('0'), newer=nr('1', 'Mehmet Demir', AT), conflict={'expected': '0', 'actual': '1'}, dirty=True), False, 'conflict'),
    ('Kayıt çakışmalarını çöz with nothing to solve', S4, False, 'conflict'),
]

LINES = [
    ('clean', MEHMET, '4', False),
    ('over unsaved work', MEHMET, '4', True),
    ('who not known', nr('5'), '4', False),
    ('when not known', nr('5', 'Mehmet Demir'), '4', True),
    ('when only', nr('5', '', AT), '4', False),
    ('without a revision yet', nr('1', 'Mehmet Demir', AT), '0', True),
]

TIPS = [
    {'where': 'Büro › Kadastro paftası 2026', 'base': '4', 'lastSaved': {'revision': '4', 'at': 1789999880000}, 'now': 1790000000000, 'newer': MEHMET, 'error': '', 'link': 'online', 'dirty': False},
    {'where': 'Büro › Kadastro paftası 2026', 'base': '4', 'lastSaved': None, 'now': 1790000000000, 'newer': MEHMET, 'error': '', 'link': 'online', 'dirty': True},
    {'where': 'Büro › Kadastro paftası 2026', 'base': '4', 'lastSaved': None, 'now': 1790000000000, 'newer': nr('5'), 'error': '', 'link': 'offline', 'dirty': True},
    {'where': 'Büro › Kadastro paftası 2026', 'base': '4', 'lastSaved': None, 'now': 1790000000000, 'newer': None, 'error': '', 'link': 'offline', 'dirty': True},
]

CELLS = [
    ('a newer revision, clean', with_(S4, newer=MEHMET)),
    ('a newer revision over unsaved work', with_(S4, newer=MEHMET, dirty=True)),
    ('a newer revision, read-only', with_(S4, newer=MEHMET, writable=False, dirty=True)),
    ('the conflict', with_(S4, newer=MEHMET, conflict={'expected': '4', 'actual': '5'}, dirty=True)),
]

MARK_CASES = [
    ('the newest, and the open drawing\'s', '5', '5', '5'),
    ('the newest, the drawing is older', '5', '5', '4'),
    ('the open drawing\'s, a newer one exists', '4', '5', '4'),
    ('neither', '3', '5', '4'),
    ('the project is not open here', '5', '5', None),
]


def trace(title, start, script):
    """Steps in order: an input, Kaydet (`save`: what it does first; `behind` takes it as refused), or what a click offers."""
    s = start
    out = []
    for item in script:
        if 'input' in item:
            s, say = step(s, item['input'])
            out.append({'input': item['input'], 'say': say, 'cell': cell(s)})
        elif 'save' in item:
            first = save_step(s)
            if first == 'behind':
                s, _ = step(s, {'kind': 'refused', 'actual': s['newer']['revision']})
            elif first == 'unchanged':
                s, _ = step(s, {'kind': 'unchanged'})
            out.append({'save': first, 'cell': cell(s)})
        else:
            o = offer(NAME, s, item.get('busy', False), item['offer'])
            q = o.get('question')
            out.append({'offer': item['offer'], 'busy': item.get('busy', False), 'expect': {'kind': o['kind'], **({'id': q['id'], 'answers': [a['value'] for a in q['answers']]} if q else {}), **({'line': o['line']} if o['kind'] == 'say' else {})}})
    return {'title': title, 'start': start, 'steps': out, 'end': s}


I = lambda **kw: {'input': kw}  # noqa: E731
TRACES = [
    trace('someone else saves while the drawing is clean', S4, [
        I(kind='newest', newest=MEHMET),
        {'offer': 'newest'},
        I(kind='newest', newest=MEHMET),
    ]),
    trace('unsaved work, then someone else saves; Kaydet uploads nothing', S4, [
        I(kind='dirty', dirty=True),
        I(kind='newest', newest=MEHMET),
        {'offer': 'newest'},
        {'save': True},
        {'offer': 'conflict'},
        {'save': True},
    ]),
    trace('Kaydet refused before the event came: who and when follow', S4, [
        I(kind='dirty', dirty=True),
        {'save': True},
        I(kind='stage', stage='encoding'),
        I(kind='stage', stage='uploading'),
        I(kind='stage', stage='verifying'),
        I(kind='refused', actual='5'),
        {'offer': 'conflict'},
        I(kind='newest', newest=MEHMET),
        {'offer': 'conflict'},
    ]),
    trace('a newer revision heard while a Kaydet goes, then the refusal', S4, [
        I(kind='dirty', dirty=True),
        {'save': True},
        I(kind='stage', stage='encoding'),
        I(kind='stage', stage='uploading'),
        I(kind='newest', newest=MEHMET),
        {'offer': 'newest', 'busy': True},
        I(kind='stage', stage='verifying'),
        I(kind='refused', actual='5'),
    ]),
    trace('a Kaydet committed with edits made while it went', S4, [
        I(kind='dirty', dirty=True),
        {'save': True},
        I(kind='stage', stage='encoding'),
        I(kind='stage', stage='uploading'),
        I(kind='stage', stage='verifying'),
        I(kind='committed', revision='5', dirty=True),
        {'offer': 'newest'},
    ]),
    trace('a failed Kaydet, then a newer revision', S4, [
        I(kind='dirty', dirty=True),
        {'save': True},
        I(kind='stage', stage='encoding'),
        I(kind='failed'),
        I(kind='newest', newest=MEHMET),
        {'save': True},
    ]),
    trace('a failed Kaydet undone: Kaydet finds nothing to write', S4, [
        I(kind='dirty', dirty=True),
        {'save': True},
        I(kind='stage', stage='encoding'),
        I(kind='failed'),
        I(kind='dirty', dirty=False),
        {'save': True},
    ]),
    trace('read-only: a newer revision is still offered', opened('4', writable=False), [
        I(kind='newest', newest=MEHMET),
        {'offer': 'newest'},
        I(kind='dirty', dirty=True),
        {'save': True},
        {'offer': 'newest'},
    ]),
    trace('a late answer never lowers the newer revision', S4, [
        I(kind='newest', newest=nr('6', 'Zeynep Kaya', AT)),
        I(kind='newest', newest=MEHMET),
    ]),
    trace('a resync finds a revision; later the project is deleted', S4, [
        I(kind='dirty', dirty=True),
        I(kind='newest', newest=MEHMET),
        I(kind='ended', why='deleted'),
        {'save': True},
        {'offer': 'newest'},
        I(kind='newest', newest=nr('6', 'Zeynep Kaya', AT)),
    ]),
]

file = {
    'format': 'kentos.fileRevisions',
    'version': 1,
    'timeZone': TZ,
    'resyncRetryMs': RESYNC_RETRY_MS,
    'texts': {
        'busy': {'sample': [NAME], 'text': TEXTS['busy'](NAME)},
        'unreadableDeleted': {'sample': [NAME], 'text': TEXTS['unreadableDeleted'](NAME)},
        'unreadableRevoked': {'sample': [NAME], 'text': TEXTS['unreadableRevoked'](NAME)},
        'resyncFailed': {'sample': [NAME], 'text': TEXTS['resyncFailed'](NAME)},
        'detached': {'sample': [NAME], 'text': TEXTS['detached'](NAME)},
        'openFailed': {'sample': [NAME, 'Sunucuya ulaşılamadı.'], 'text': TEXTS['openFailed'](NAME, 'Sunucuya ulaşılamadı.')},
    },
    'marks': MARKS,
    'who': [{'newer': n, 'text': who(n)} for n in (MEHMET, nr('5', 'Mehmet Demir'), nr('5', '', AT), nr('5'))],
    'cellStates': [{'title': t, 'state': s, 'expect': cell_state(s)} for t, s in CELL_STATES],
    'steps': [{'title': t, 'from': s, 'input': i, 'expect': dict(zip(('state', 'say'), step(s, i)))} for t, s, i in STEPS],
    'saveSteps': [{'title': t, 'state': s, 'expect': save_step(s)} for t, s in SAVE_STEPS],
    'events': [{'title': t, 'own': MINE, 'events': e, 'expect': read_events(e, MINE)} for t, e in EVENTS],
    'newest': [{'title': t, 'revisions': r, 'expect': newest_of(r)} for t, r in NEWEST],
    'resync': [{'title': t, 'answer': a, 'expect': resync_step(a)} for t, a in RESYNC],
    'offers': [{'title': t, 'input': {'name': NAME, 'state': s, 'busy': b, 'via': v}, 'expect': offer(NAME, s, b, v)} for t, s, b, v in OFFERS],
    'lines': [{'title': t, 'name': NAME, 'newer': n, 'base': b, 'dirty': d, 'text': newer_line(NAME, n, b, d)} for t, n, b, d in LINES],
    'tips': [{'input': t, 'expect': file_tip(t)} for t in TIPS],
    'cells': [{'title': t, 'state': s, 'expect': cell(s)} for t, s in CELLS],
    'historyMarks': [{'title': t, 'revision': r, 'current': c, 'openBase': b, 'expect': marks(r, c, b)} for t, r, c, b in MARK_CASES],
    'traces': TRACES,
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
