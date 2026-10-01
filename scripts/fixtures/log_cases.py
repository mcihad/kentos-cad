"""How the log is written, for both platforms: a line's time and level in
the bottom panel's Komut geçmişi and Uyarılar, which lines each tab lists,
the Uyarılar badge, how many lines are kept, the echo of what the user
typed, which lines the status bar shows and for how long, and the rows' look
(format in fixtures/shell/README.md; DESIGN.md §7.7).

    python3 scripts/fixtures/log_cases.py           # writes the file
    python3 scripts/fixtures/log_cases.py --check   # writes nothing; compares

Writes fixtures/shell/v1/log.json. The words, tables and the look are
written here by hand from the web's panel and its style sheets; every answer
below is worked out here, apart from the web's TypeScript: the local times
with Python's own time zone database. apps/web/src/ui/bottom/logPlan.test.ts
holds the web to the file (and the look to panels.css and shell.css), and
the desktop's bottom panel and status bar read the same file.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

OUT = 'fixtures/shell/v1/log.json'
ZONE = 'Europe/Istanbul'
LEVELS = ['command', 'info', 'success', 'warn', 'error']

# ── Words and tables, as the web's bottom panel has them ─────────────

TABS = [
    {'id': 'history', 'label': 'Komut geçmişi', 'icon': 'history'},
    {'id': 'coords', 'label': 'Koordinat listesi', 'icon': 'table'},
    {'id': 'points', 'label': 'Noktalar', 'icon': 'pointEditor'},
    {'id': 'messages', 'label': 'Uyarılar', 'icon': 'warning'},
]
TEXTS = {
    'tabs': 'Alt panel',
    'clear': 'Geçmişi temizle',
    'close': 'Paneli kapat',
    'open': 'Komut geçmişini aç',
    'edge': 'Alt panel yüksekliği',
    'empty': {
        'history': 'Henüz komut çalıştırılmadı. Bir araç seçin ya da komut satırına yazın.',
        'messages': 'Uyarı yok.',
    },
}
LIMIT = 500
FOLLOW_WITHIN = 24
ICON = {'command': None, 'info': None, 'success': 'success', 'warn': 'warning', 'error': 'error'}

# The rows as panels.css draws them, and the status bar's message as shell.css does: tones are the style sheets'
# colour tokens without their --c- (text, text-2, text-3, ok, warn, danger, hover); sizes in CSS px.
LOOK = {
    'list': {'font': 'mono', 'size': 'xs', 'weight': 400, 'lineHeight': 1.5, 'padding': [6, 0]},
    'row': {'columns': [64, 18], 'padding': [1, 12], 'hover': 'hover'},
    'time': 'text-3',
    'iconTop': 2,
    'iconSize': 14,
    'text': {'tone': 'text-2', 'wrap': 'pre-wrap'},
    'levels': {
        'command': {'icon': None, 'text': 'text', 'weight': 500},
        'info': {'icon': None, 'text': 'text-2', 'weight': 400},
        'success': {'icon': 'ok', 'text': 'text-2', 'weight': 400},
        'warn': {'icon': 'warn', 'text': 'warn', 'weight': 400},
        'error': {'icon': 'danger', 'text': 'danger', 'weight': 400},
    },
    'flash': {'gap': 6, 'padding': [0, 12], 'fadeMs': 160, 'icon': {'info': None, 'success': 'ok', 'warn': 'warn', 'error': 'danger'}},
}

# ── Answers ─────────────────────────────────────────────────────────


def local_time(ms):
    """The local wall clock, hours:minutes:seconds, 24-hour; the second shown, never rounded up."""
    t = datetime.fromtimestamp(ms // 1000, ZoneInfo(ZONE))
    return f'{t.hour:02d}:{t.minute:02d}:{t.second:02d}'


def ms_of(iso):
    d = datetime.strptime(iso, '%Y-%m-%dT%H:%M:%S.%fZ').replace(tzinfo=timezone.utc)
    return int(d.timestamp()) * 1000 + d.microsecond // 1000


def listed_in(tab, level):
    return tab == 'history' or level in ('warn', 'error')


def flash(level, text):
    """The status bar: never a command's line nor an indented one (two spaces first); a warning or error 9 s, else 5 s."""
    if level == 'command' or text.startswith('  '):
        return None
    icon = {'warn': 'warning', 'error': 'error', 'success': 'success'}.get(level, 'info')
    return {'icon': icon, 'ms': 9000 if level in ('warn', 'error') else 5000}


def kept(pushed):
    """Ids start at 1; the log keeps the newest LIMIT."""
    length = min(pushed, LIMIT)
    return {'pushed': pushed, 'length': length, 'first': pushed - length + 1, 'last': pushed}


def badge_script(steps):
    """Each step's badge count and the log's length. A line pushed takes the next id (ids go on across a clear);
    'look': the Uyarılar tab is on screen, every line so far is seen; 'clear': Geçmişi temizle."""
    lines, seq, seen, out = [], 0, 0, []
    for step in steps:
        op = step if isinstance(step, str) else step['push']
        times = 1 if isinstance(step, str) else step['times']
        if op == 'look':
            seen = lines[-1][0] if lines else seen
        elif op == 'clear':
            lines = []
        else:
            for _ in range(times):
                seq += 1
                lines.append((seq, op))
            lines = lines[-LIMIT:]
        count = sum(1 for i, level in lines if level in ('warn', 'error') and i > seen)
        out.append({'step': step, 'count': count, 'lines': len(lines)})
    return out


TIMES = [
    ('in the day', '2026-09-27T09:05:07.000Z'),
    ('the second the clock shows, not rounded up', '2026-09-27T20:59:59.999Z'),
    ('local midnight is 00, not 24', '2026-09-27T21:00:00.000Z'),
    ('just after UTC midnight', '2026-09-27T00:00:09.000Z'),
    ('in winter: Istanbul keeps +03:00 all year', '2026-01-15T06:00:00.500Z'),
    ('the next day and year, locally', '2026-12-31T22:59:30.000Z'),
]
ECHOES = ['10,20', '@5<45', 'K', 'çizgi']
FLASHES = [
    ('command', 'Çizgi'),
    ('command', '› 10,20'),
    ('info', 'Çizim kaydedildi.'),
    ('info', '  Y 412345.120  X 4512345.670'),
    ('info', ' Tek boşlukla başlayan satır'),
    ('success', '3 nesne silindi.'),
    ('warn', '“KA” anlaşılamadı. Koordinatı Y,X ya da @dY,dX biçiminde yazın.'),
    ('warn', '  Girintili uyarı'),
    ('error', '“XYZ” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın.'),
]
BADGES = [
    ('warnings and errors count until the tab is on screen', ['warn', 'info', 'error', 'command', 'look', 'success', 'warn', 'look']),
    ('a clear starts the count again, and ids go on after it', [{'push': 'warn', 'times': 5}, 'look', 'clear', 'warn', 'clear', 'error']),
    ('a clear from another tab: the new warning still counts', [{'push': 'warn', 'times': 3}, 'clear', 'warn']),
    ('a warning seen and then dropped (500 kept) is not counted again; a new one is', [{'push': 'warn', 'times': 10}, 'look', {'push': 'info', 'times': 500}, 'warn']),
    ('the tab on screen over an empty log sees nothing', ['look', 'warn', 'look', 'error']),
]

file = {
    'format': 'kentos.log',
    'version': 1,
    'timeZone': ZONE,
    'tabs': TABS,
    'texts': TEXTS,
    'limit': LIMIT,
    'followWithin': FOLLOW_WITHIN,
    'levels': [{'level': lv, 'icon': ICON[lv], 'listedIn': [t for t in ('history', 'messages') if listed_in(t, lv)]} for lv in LEVELS],
    'look': LOOK,
    'times': [{'title': t, 'iso': iso, 'at': ms_of(iso), 'text': local_time(ms_of(iso))} for t, iso in TIMES],
    'echo': [{'typed': e, 'text': f'› {e}'} for e in ECHOES],
    'flash': [{'level': lv, 'text': t, 'shown': flash(lv, t)} for lv, t in FLASHES],
    'kept': [kept(n) for n in [3, 500, 501, 1200]],
    'badge': [{'title': t, 'steps': badge_script(s)} for t, s in BADGES],
}

text = json.dumps(file, ensure_ascii=False, indent=1) + '\n'
if '--check' in sys.argv[1:]:
    with open(OUT, encoding='utf-8') as f:
        if f.read() != text:
            sys.exit(f'{OUT} is not what this script writes: run it without --check and read the difference.')
    print(f'{OUT} matches')
else:
    import os
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)
    print(f'{OUT} written')
