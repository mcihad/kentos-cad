"""The status bar's cloud cells for both platforms: the save cell's words,
state, tip and click, the server cell's words and tip, and the account menu's
rows with why an action on the open project is off (format in
fixtures/cloud/README.md; CLAUDE.md §21.1, docs/adr/0038).

    python3 scripts/fixtures/cells_cases.py           # writes the file
    python3 scripts/fixtures/cells_cases.py --check   # writes nothing; compares

Writes fixtures/cloud/v1/cells.json. The words are the web's, written here by
hand; every answer below is worked out here, apart from the web's TypeScript.
apps/web/src/ui/statusbar/cellsPlan.test.ts holds the web to the file, and the
desktop's status bar reads the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

OUT = 'fixtures/cloud/v1/cells.json'


def js_round(x):
    """JavaScript's Math.round: halves go up (Python's round goes to even)."""
    return math.floor(x + 0.5)


SAVE = {
    'saved': lambda n: 'Buluta kaydedildi',
    'pending': lambda n: f'Kaydedilecek: {n}',
    'saving': lambda n: 'Kaydediliyor…',
    'offline_pending': lambda n: f'Çevrimdışı: {n} bekliyor',
    'conflict': lambda n: f'Çakışma: {n}',
    'error': lambda n: 'Kayıt hatası',
    'readonly': lambda n: 'Salt okunur',
    'deleted': lambda n: 'Proje silindi',
    'revoked': lambda n: 'Erişim kaldırıldı',
    'archived': lambda n: 'Proje arşivde',
}
LINK = {'none': '', 'connecting': 'bağlanıyor', 'online': 'canlı', 'reconnecting': 'yeniden bağlanıyor', 'offline': 'çevrimdışı', 'auth_required': 'oturum gerekli'}


def file_text(f):
    s, base = f['state'], f['base']
    return {
        'saved': 'Henüz revizyon yok' if base == '0' else f'Buluta kaydedildi · r{base}',
        'pending': 'Kaydedilmedi' if base == '0' else f'Kaydedilmedi · r{base} üstüne',
        'encoding': 'Dosya hazırlanıyor…',
        'uploading': f'Yükleniyor %{js_round(f["progress"] * 100)}',
        'verifying': 'Sunucu doğruluyor…',
        'conflict': f'Çakışma: r{f["conflictActual"] or "?"} kaydedilmiş',
        'outdated': f'Yeni revizyon: r{f["newerRevision"] or "?"}',
        'error': 'Kayıt hatası',
        'readonly': 'Salt okunur',
        'deleted': 'Proje silindi',
        'revoked': 'Erişim kaldırıldı',
        'archived': 'Proje arşivde',
    }[s]


def view(inp):
    if inp['kind'] == 'none':
        return {'hidden': True, 'text': '', 'state': None}
    if inp['kind'] == 'file':
        return {'hidden': False, 'text': file_text(inp), 'state': inp['state']}
    n = inp['conflicts'] if inp['state'] == 'conflict' else inp['pending']
    return {'hidden': False, 'text': SAVE[inp['state']](n), 'state': inp['state']}


def action(inp):
    if inp['kind'] == 'none':
        return None
    s = inp['state']
    if s == 'conflict':
        return 'cloud.conflicts'
    if inp['kind'] == 'file':
        if s == 'outdated':
            return 'cloud.openNewest'
        if s in ('readonly', 'encoding', 'uploading', 'verifying'):
            return None
        return 'file.save'
    return None if s == 'readonly' else 'file.save'


def ago(at, now):
    if at is None:
        return 'henüz yok'
    s = js_round((now - at) / 1000)
    return f'{s} sn önce' if s < 60 else f'{js_round(s / 60)} dk önce'


def db_tip(t):
    title = 'Bulut kaydı'
    w = t['where']
    if t['state'] == 'deleted':
        return {'title': title, 'description': f'{w} sunucuda silindi. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin.'}
    if t['state'] == 'revoked':
        return {'title': title, 'description': f'{w} projesine erişiminiz kaldırıldı. Değişiklikler buluta gönderilmiyor; bu cihazda saklanıyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Erişim için proje sahibine başvurun.'}
    if t['state'] == 'archived':
        return {'title': title, 'description': f'{w} arşivlenmiş: salt okunurdur, değişiklikler buluta gönderilmiyor. Tıklayın ya da Ctrl+S: çizimi yerel bir dosyaya kaydedin. Proje sahibi ya da yöneticisi arşivden çıkarınca projeyi yeniden açın.'}
    lines = [
        f'{w}. Değişiklikler kendiliğinden kaydedilir; Ctrl+S hemen gönderir.',
        f'Son kayıt: {ago(t["lastSaved"], t["now"])}. Canlı bağlantı: {LINK[t["link"]]}.',
        t['error'],
        '' if t['durableDrafts'] else 'Bu tarayıcı taslakları saklayamıyor: kaydedilmeden kapanırsa değişiklikler kaybolur.',
    ]
    return {'title': title, 'description': ' '.join(x for x in lines if x)}


def file_tip(t):
    link = LINK[t['link']]
    last, newer = t['lastSaved'], t['newer']
    lines = [
        f'{t["where"]}, dosya olarak saklanıyor (KCAD revizyonları).',
        'Projenin henüz revizyonu yok.' if t['base'] == '0' else f"Çizimin dayandığı revizyon: {t['base']}.",
        'Kendiliğinden kaydedilmez: Kaydet (Ctrl+S) yeni bir revizyon yazar; arada başkası kaydettiyse üzerine yazılmaz.',
        f'Bu pencerenin son kaydı: revizyon {last["revision"]}, {ago(last["at"], t["now"])}.' if last else '',
        f'Sunucuda daha yeni revizyon var: {newer["revision"]}{f" ({newer["by"]})" if newer["by"] else ""}; açmak için tıklayın.' if newer else '',
        t['error'],
        f'Canlı bağlantı: {link}.' if link else '',
        'Kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor.' if t['dirty'] else '',
    ]
    return {'title': 'Bulut kaydı: dosya projesi', 'description': ' '.join(x for x in lines if x)}


SERVER = {'checking': 'Sunucu…', 'online': 'Sunucu: bağlı', 'offline': 'Sunucu: yok', 'incompatible': 'Sunucu: uyumsuz'}


def server_tip(t):
    # health: the contract's Health (status "ok", service, version, commit when known, contracts).
    h = t['health']
    build = f'{h["service"]} {h["version"]}' + (f' ({h["commit"][:8]})' if h.get('commit') else '') if h else ''
    s = t['state']
    if s == 'online':
        text = f'{build}, sözleşme sürümü {h["contracts"]}.' if h else ''
    elif s == 'incompatible':
        # The reason says both contract versions: the tip does not say the server's again.
        text = (f'{build}. ' if h else '') + t['detail']
    elif s == 'checking':
        text = 'Sunucuya soruluyor…'
    else:
        text = t['detail'] + ' Çizim sunucusuz çalışır; kayıt yerel .kcad dosyasına yapılır.' + (' Geliştirmede sunucuyu “pnpm api” ile başlatın.' if t['dev'] else '')
    return {'title': 'KentOS sunucusu', 'description': (f'{text} ' if text else '') + 'Hesap, bulut projeleri ve bağlantı denetimi için tıklayın.'}


ACTIONS = [('cloud.history', 'project.history'), ('cloud.share', 'project.share'), ('cloud.rename', 'project.edit'), ('cloud.delete', 'project.delete')]


def account_rows(t):
    rows = [{'kind': 'header', 'label': (f'{t["user"]}{f" · {t["project"]["tenantName"]}" if t["project"] else ""}' if t['user'] is not None else 'Oturum açılmadı')},
            {'command': 'cloud.signOut' if t['user'] is not None else 'cloud.signIn'},
            {'kind': 'separator'},
            {'command': 'cloud.open'}, {'command': 'cloud.upload'}, {'command': 'cloud.uploadFile'}]
    for command, permission in ACTIONS:
        if t['project'] and command in t['disabled'] and permission in t['denied']:
            rows.append({'command': command, 'detail': f'Bu projede yetkiniz yok ({permission}); proje sahibine ya da yöneticisine başvurun.'})
        else:
            rows.append({'command': command})
    rows += [{'kind': 'separator'}, {'command': 'server.check'}]
    return rows


# ── Cases ────────────────────────────────────────────────────────────

FILE = {'kind': 'file', 'state': 'saved', 'base': '12', 'progress': 0.426, 'conflictActual': '13', 'newerRevision': '14'}


def fcase(**over):
    return {**FILE, **over}


FILE_STATES = ['saved', 'pending', 'encoding', 'uploading', 'verifying', 'conflict', 'outdated', 'error', 'readonly', 'deleted', 'revoked', 'archived']
VIEWS = (
    [{'kind': 'none'}]
    + [{'kind': 'database', 'state': s, 'pending': 3, 'conflicts': 2} for s in SAVE]
    + [fcase(state=s) for s in FILE_STATES]
    + [fcase(state='saved', base='0'), fcase(state='pending', base='0'), fcase(state='uploading', progress=0.005), fcase(state='uploading', progress=0.995),
       fcase(state='conflict', conflictActual=None), fcase(state='outdated', newerRevision=None)]
)
NOW = 1_790_000_000_000
AGOS = [None, NOW, NOW - 59_400, NOW - 59_600, NOW - 89_000, NOW - 90_000, NOW - 150_000, NOW - 3_600_000, NOW + 5_000]
WHERE = 'Büro › Ada 101'
DB_TIPS = [
    {'state': 'saved', 'where': WHERE, 'lastSaved': NOW - 12_000, 'now': NOW, 'link': 'online', 'error': '', 'durableDrafts': True},
    {'state': 'pending', 'where': WHERE, 'lastSaved': None, 'now': NOW, 'link': 'reconnecting', 'error': '', 'durableDrafts': False},
    {'state': 'error', 'where': WHERE, 'lastSaved': NOW - 600_000, 'now': NOW, 'link': 'offline', 'error': 'Sunucu isteği reddetti: sürüm eski.', 'durableDrafts': True},
    {'state': 'deleted', 'where': WHERE, 'lastSaved': None, 'now': NOW, 'link': 'online', 'error': '', 'durableDrafts': True},
    {'state': 'revoked', 'where': WHERE, 'lastSaved': None, 'now': NOW, 'link': 'online', 'error': '', 'durableDrafts': True},
    {'state': 'archived', 'where': WHERE, 'lastSaved': None, 'now': NOW, 'link': 'online', 'error': '', 'durableDrafts': True},
]
FILE_TIPS = [
    {'where': WHERE, 'base': '0', 'lastSaved': None, 'now': NOW, 'newer': None, 'error': '', 'link': 'none', 'dirty': False},
    {'where': WHERE, 'base': '12', 'lastSaved': {'revision': '12', 'at': NOW - 30_000}, 'now': NOW, 'newer': None, 'error': '', 'link': 'online', 'dirty': True},
    {'where': WHERE, 'base': '12', 'lastSaved': None, 'now': NOW, 'newer': {'revision': '14', 'by': 'Mehmet Demir'}, 'error': 'Yükleme kesildi.', 'link': 'offline', 'dirty': False},
    {'where': WHERE, 'base': '12', 'lastSaved': None, 'now': NOW, 'newer': {'revision': '14', 'by': ''}, 'error': '', 'link': 'none', 'dirty': False},
]
HEALTH = {'status': 'ok', 'service': 'kentosd', 'version': '0.9.0', 'commit': '4d67c97af7fe8fa6', 'contracts': 7}
NO_COMMIT = {k: v for k, v in HEALTH.items() if k != 'commit'}
INCOMPATIBLE = 'Sunucu sözleşme sürümü 7, uygulama 8 bekliyor. Uygulamayı ya da sunucuyu güncelleyin.'
SERVER_TIPS = [
    {'state': 'online', 'health': HEALTH, 'detail': '', 'dev': False},
    {'state': 'online', 'health': NO_COMMIT, 'detail': '', 'dev': False},
    {'state': 'incompatible', 'health': HEALTH, 'detail': INCOMPATIBLE, 'dev': False},
    {'state': 'incompatible', 'health': NO_COMMIT, 'detail': INCOMPATIBLE, 'dev': True},
    {'state': 'checking', 'health': None, 'detail': '', 'dev': False},
    {'state': 'offline', 'health': None, 'detail': 'Sunucuya ulaşılamadı.', 'dev': True},
    {'state': 'offline', 'health': None, 'detail': 'Sunucuya ulaşılamadı.', 'dev': False},
]
ACCOUNTS = [
    ('signed out', {'user': None, 'project': None, 'disabled': ['cloud.history', 'cloud.share', 'cloud.rename', 'cloud.delete'], 'denied': []}),
    ('signed in, no project open', {'user': 'Ayşe Yılmaz', 'project': None, 'disabled': ['cloud.history', 'cloud.share', 'cloud.rename', 'cloud.delete'], 'denied': ['project.history', 'project.share']}),
    ('a project open, every right', {'user': 'Ayşe Yılmaz', 'project': {'tenantName': 'Büro'}, 'disabled': [], 'denied': []}),
    ('a viewer: no share, rename or delete', {'user': 'Mehmet Demir', 'project': {'tenantName': 'Büro'}, 'disabled': ['cloud.share', 'cloud.rename', 'cloud.delete'], 'denied': ['project.share', 'project.edit', 'project.delete']}),
    ('off for another reason: no detail', {'user': 'Ayşe Yılmaz', 'project': {'tenantName': 'Büro'}, 'disabled': ['cloud.rename'], 'denied': []}),
    ('a right lacking but the command on: no detail', {'user': 'Ayşe Yılmaz', 'project': {'tenantName': 'Büro'}, 'disabled': [], 'denied': ['project.delete']}),
    ('an empty name, signed in', {'user': '', 'project': None, 'disabled': [], 'denied': []}),
]

file = {
    'format': 'kentos.cells',
    'version': 1,
    'saveTexts': [{'state': s, 'sample': 3, 'text': fn(3)} for s, fn in SAVE.items()],
    'linkTexts': LINK,
    'views': [{'input': v, 'view': view(v), 'action': action(v)} for v in VIEWS],
    'ago': [{'at': a, 'now': NOW, 'text': ago(a, NOW)} for a in AGOS],
    'databaseTips': [{'input': t, 'expect': db_tip(t)} for t in DB_TIPS],
    'fileTips': [{'input': t, 'expect': file_tip(t)} for t in FILE_TIPS],
    'serverTexts': SERVER,
    'serverTips': [{'input': t, 'expect': server_tip(t)} for t in SERVER_TIPS],
    'projectActions': [{'command': c, 'permission': p} for c, p in ACTIONS],
    'accountRows': [{'title': title, 'input': t, 'rows': account_rows(t)} for title, t in ACCOUNTS],
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
