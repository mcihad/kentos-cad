#!/usr/bin/env python3
"""Independent reference of the temporal layers' time (docs/adr/0210 §3–§5), written from the ADR without KentOS code.

An attribute's text read as a moment (the date and time grammar with zones, years 1–9999, milliseconds since 1970-01-01
on the proleptic Gregorian calendar), a moment written and shown, a moment rounded down to a unit, the slider's
positions (months and years by the calendar, every position from the anchor), its last index and the step it opens with,
an object's time from its start and end texts, whether it shows in a window (the ADR's table) and a layer's summary.
The calendar is Python's `datetime`; the grammar a regular expression of ASCII digits.

Writes fixtures/temporal/v1/cases.json; the geometry core (crates/shared/geometry-core/tests/all/time.rs) and the web
(apps/web/src/model/temporal.wasm.test.ts) must give the same.

    python3 scripts/fixtures/temporal_cases.py           # writes the file
    python3 scripts/fixtures/temporal_cases.py --check   # compares it with the one on disk
"""

import calendar
import datetime as dt
import json
import pathlib
import re
import sys

OUT = pathlib.Path(__file__).resolve().parents[2] / 'fixtures' / 'temporal' / 'v1' / 'cases.json'

SECOND = 1000
MINUTE = 60 * SECOND
HOUR = 60 * MINUTE
DAY = 24 * HOUR
WEEK = 7 * DAY
EPOCH = dt.date(1970, 1, 1).toordinal()
MAX_POSITIONS = 100_000
INF = float('inf')
FIXED = {'second': SECOND, 'minute': MINUTE, 'hour': HOUR, 'day': DAY, 'week': WEEK}

D = '[0-9]'
ISO = re.compile(rf'^({D}{{4}})-({D}{{2}})-({D}{{2}})$')
TR = re.compile(rf'^({D}{{1,2}})\.({D}{{1,2}})\.({D}{{4}})$')
CLOCK = re.compile(rf'^({D}{{2}}):({D}{{2}})(?::({D}{{2}})(?:\.({D}{{1,9}}))?)?(Z|[+-]{D}{{2}}(?::{D}{{2}}|{D}{{2}})?)?$')


def day_of(text):
    """A date's (y, m, d), or None."""
    m = ISO.match(text)
    if m:
        y, mo, d = (int(x) for x in m.groups())
    else:
        m = TR.match(text)
        if not m:
            return None
        d, mo, y = (int(x) for x in m.groups())
    if not (1 <= y <= 9999 and 1 <= mo <= 12):
        return None
    if not 1 <= d <= calendar.monthrange(y, mo)[1]:
        return None
    return y, mo, d


def read(text):
    """§3: 'empty', 'unreadable' or the moment in milliseconds."""
    s = text.strip(' \t')
    if not s:
        return 'empty'
    if not s.isascii():
        return 'unreadable'
    cut = min([i for i in (s.find('T'), s.find(' ')) if i >= 0], default=-1)
    date_text, time_text = (s, None) if cut < 0 else (s[:cut], s[cut + 1:])
    ymd = day_of(date_text)
    if ymd is None:
        return 'unreadable'
    into = offset = 0
    if time_text is not None:
        m = CLOCK.match(time_text)
        if not m:
            return 'unreadable'
        h, mi, sec, frac, zone = m.groups()
        h, mi, sec = int(h), int(mi), int(sec or 0)
        if h > 23 or mi > 59 or sec > 59:
            return 'unreadable'
        ms = int((frac or '').ljust(3, '0')[:3]) if frac else 0
        into = h * HOUR + mi * MINUTE + sec * SECOND + ms
        if zone and zone != 'Z':
            sign = 1 if zone[0] == '+' else -1
            rest = zone[1:].replace(':', '')
            zh, zm = int(rest[:2]), int(rest[2:4] or 0)
            if zh > 23 or zm > 59:
                return 'unreadable'
            offset = sign * (zh * HOUR + zm * MINUTE)
    days = dt.date(*ymd).toordinal() - EPOCH
    return days * DAY + into - offset


def split(t):
    """A moment's date and milliseconds into the day."""
    days, into = divmod(t, DAY)
    return dt.date.fromordinal(days + EPOCH), into


def write(t, date_only):
    d, into = split(t)
    day = f'{d.year:04}-{d.month:02}-{d.day:02}'
    if date_only or into == 0:
        return day
    h, rest = divmod(into, HOUR)
    mi, rest = divmod(rest, MINUTE)
    s, ms = divmod(rest, SECOND)
    return f'{day}T{h:02}:{mi:02}:{s:02}' + (f'.{ms:03}' if ms else '')


def show(t, unit):
    d, into = split(t)
    day = f'{d.day:02}.{d.month:02}.{d.year:04}'
    h, rest = divmod(into, HOUR)
    mi, rest = divmod(rest, MINUTE)
    s = rest // SECOND
    if unit == 'second':
        return f'{day} {h:02}:{mi:02}:{s:02}'
    if unit in ('minute', 'hour'):
        return f'{day} {h:02}:{mi:02}'
    return day


def clock(t, unit):
    """A moment's clock for a step under a day; none for a day or more."""
    _, into = split(t)
    h, rest = divmod(into, HOUR)
    mi, rest = divmod(rest, MINUTE)
    sec = rest // SECOND
    if unit == 'second':
        return f'{h:02}:{mi:02}:{sec:02}'
    if unit in ('minute', 'hour'):
        return f'{h:02}:{mi:02}'
    return None


def date_shown(t):
    d, _ = split(t)
    return f'{d.day:02}.{d.month:02}.{d.year:04}'


def show_window(w, unit):
    """What the slider's position shows (§5): a period inside one day, under a day's step, writes its date once."""
    if 'instant' in w:
        return show(w['instant'], unit)
    a, b = w['range']
    if clock(b, unit) is not None and split(a)[0] == split(b)[0]:
        return f'{show(a, unit)} – {clock(b, unit)}'
    return f'{show(a, unit)} – {show(b, unit)}'


def show_ends(first, last, unit):
    """The slider's ends (§5): under a day's step their clocks within one day, else their dates; a day or more as shown."""
    if clock(first, unit) is None:
        return [show(first, unit), show(last, unit)]
    if split(first)[0] == split(last)[0]:
        return [clock(first, unit), clock(last, unit)]
    return [date_shown(first), date_shown(last)]


def floor_to(t, unit):
    d, into = split(t)
    start = (d.toordinal() - EPOCH) * DAY
    if unit == 'year':
        return (dt.date(d.year, 1, 1).toordinal() - EPOCH) * DAY
    if unit == 'month':
        return (dt.date(d.year, d.month, 1).toordinal() - EPOCH) * DAY
    if unit == 'week':
        return start - d.weekday() * DAY
    if unit == 'day':
        return start
    return start + into - into % FIXED[unit]


def position(anchor, n, unit, k):
    if unit in FIXED:
        return anchor + k * n * FIXED[unit]
    d, into = split(anchor)
    total = d.year * 12 + (d.month - 1) + (12 if unit == 'year' else 1) * n * k
    y, m = divmod(total, 12)
    m += 1
    # Past the calendar's years every position is after (or before) any moment of an extent, which lies within them.
    if y > 9999:
        return INF
    if y < 1:
        return -INF
    day = min(d.day, calendar.monthrange(y, m)[1])
    return (dt.date(y, m, day).toordinal() - EPOCH) * DAY + into


def positions(extent, n, unit):
    """The anchor and the least k whose position is not before the end, by bisection (positions only grow)."""
    anchor = floor_to(extent[0], unit)
    end = extent[1]
    if position(anchor, n, unit, MAX_POSITIONS) < end:
        return None
    lo, hi = 0, MAX_POSITIONS
    while lo < hi:
        mid = (lo + hi) // 2
        if position(anchor, n, unit, mid) >= end:
            hi = mid
        else:
            lo = mid + 1
    return {'anchor': anchor, 'k': lo}


def auto_step(extent):
    for unit in ('year', 'month', 'day', 'hour', 'minute', 'second'):
        p = positions(extent, 1, unit)
        if p is not None and p['k'] >= 5:
            return {'n': 1, 'unit': unit}
    return {'n': 1, 'unit': 'second'}



def object_time(rule, start, end):
    """§4: None (timeless) or (s, e, mode)."""
    mode = 'cumulative' if rule['cumulative'] else ('range' if rule['ranged'] else 'instant')
    s = read(start) if start is not None else 'empty'
    if rule['ranged']:
        e = read(end) if end is not None else 'empty'
        s_ok, e_ok = isinstance(s, int), isinstance(e, int)
        if not s_ok and not e_ok:
            return None
        return (s if s_ok else -INF, e if e_ok else INF, mode)
    if not isinstance(s, int):
        return None
    return (s, s, mode)


def shows(t, window):
    s, e, mode = t
    if 'instant' in window:
        a = window['instant']
        if mode == 'cumulative':
            return s <= a
        if mode == 'range':
            return s <= a < e
        return s == a
    a, b = window['range']
    if mode == 'cumulative':
        return s < b
    if mode == 'range':
        return s < b and e > a
    return a <= s < b


def number(x):
    if x == INF:
        return 'inf'
    if x == -INF:
        return '-inf'
    return x


def summary(rule, values):
    timed = timeless = unreadable = 0
    lo = hi = None
    for start, end in values:
        unreadable += int(start is not None and read(start) == 'unreadable')
        if rule['ranged']:
            unreadable += int(end is not None and read(end) == 'unreadable')
        t = object_time(rule, start, end)
        if t is None:
            timeless += 1
            continue
        timed += 1
        for v in t[:2]:
            if v not in (INF, -INF):
                lo = v if lo is None else min(lo, v)
                hi = v if hi is None else max(hi, v)
    return {'timed': timed, 'timeless': timeless, 'unreadable': unreadable,
            'extent': None if lo is None else [lo, hi]}


READS = [
    '1970-01-01', '01.01.1970', '1.1.1970', '2.1.1970', '1969-12-31', '0001-01-01', '9999-12-31', '2024-02-29',
    '29.02.2024', '2023-02-29', '29.2.2023', '2100-02-29', '2000-02-29', '2024-13-01', '2024-00-10', '2024-04-31',
    '2024-1-01', '24-01-01', '2024/01/01', '  2024-03-05\t', '', '   ', 'yarın', '2024-03-05T', '2024-03-05 ',
    '2024-03-05T12:30', '2024-03-05 12:30', '05.03.2024 12:30', '05.03.2024T12:30', '2024-03-05T12:30:45',
    '2024-03-05T12:30:45.1', '2024-03-05T12:30:45.12', '2024-03-05T12:30:45.123', '2024-03-05T12:30:45.123456789',
    '2024-03-05T12:30:45.1234567891', '2024-03-05T12:30:45.', '2024-03-05T24:00', '2024-03-05T23:60',
    '2024-03-05T23:59:60', '2024-03-05T7:30', '2024-03-05T12:30Z', '2024-03-05T12:30:45Z', '2024-03-05T12:30+03:00',
    '2024-03-05T12:30-03:00', '2024-03-05T12:30+0330', '2024-03-05T12:30+03', '2024-03-05T00:30+03:00',
    '2024-03-05T12:30+24:00', '2024-03-05T12:30+03:60', '2024-03-05T12:30+3', '2024-03-05Z', '2024-03-05+03:00',
    '2024-03-05T12:30 +03:00', '1969-12-31T23:59:59.999', '0001-01-01T00:00:00.001', '9999-12-31T23:59:59.999',
    '2024-03-05T12:30:45.999+14:00', 'şubat', '2024-03-05ü', '０２.03.2024', '2024-03-05TT12:30',
    '2024-03-05T12:30:45.5-00:30', '31.12.1899', '15.10.1582', '04.10.1582 23:59:59',
]

WRITES = [
    ('2024-03-05', False), ('2024-03-05T12:30', False), ('2024-03-05T12:30', True), ('2024-03-05T12:30:45.120', False),
    ('1969-12-31T23:59:59.999', False), ('0001-01-01', False), ('9999-12-31T23:59:59', False),
    ('2024-03-05T00:00:00.001', False),
]

SHOWS = [
    ('2024-03-05T12:30:45', 'second'), ('2024-03-05T12:30:45', 'minute'), ('2024-03-05T12:30:45', 'hour'),
    ('2024-03-05T12:30:45', 'day'), ('2024-03-05T12:30:45', 'week'), ('2024-03-05T12:30:45', 'month'),
    ('2024-03-05T12:30:45', 'year'), ('1969-12-31T23:59:59.999', 'second'), ('0001-01-01', 'year'),
]

# The slider's position and ends (§5): within a day and across one, at midnight, under and over a day's step, before 1970.
SHOW_WINDOWS = [
    ({'range': ['2024-05-01T14:00', '2024-05-01T15:00']}, 'hour'),
    ({'range': ['2024-05-01T23:00', '2024-05-02T00:00']}, 'hour'),
    ({'range': ['2024-05-01T14:00:30', '2024-05-01T14:00:40']}, 'second'),
    ({'range': ['2024-05-01T14:00', '2024-05-01T14:30']}, 'minute'),
    ({'range': ['2024-05-01', '2024-05-02']}, 'day'),
    ({'range': ['2015-01-01', '2016-01-01']}, 'year'),
    ({'range': ['1969-12-31T22:00', '1969-12-31T23:00']}, 'hour'),
    ({'instant': '2024-05-01T14:00'}, 'hour'),
    ({'instant': '2024-05-01T14:00'}, 'day'),
]
SHOW_ENDS = [
    ('2024-05-01T06:00', '2024-05-01T17:00', 'hour'),
    ('2024-05-01T06:00', '2024-05-01T17:00:15', 'second'),
    ('2024-01-01T00:00', '2024-01-31T23:00', 'hour'),
    ('2024-05-01T06:00', '2024-05-02T00:00', 'minute'),
    ('2024-05-01', '2024-05-01', 'hour'),
    ('2010-01-01', '2020-01-01', 'year'),
    ('2024-05-01', '2024-06-01', 'day'),
    ('1969-12-31T01:00', '1969-12-31T23:00', 'hour'),
]

FLOORS = [(t, u) for t in ('2024-03-05T12:34:56.789', '2024-01-01', '1969-12-31T23:59:59.999', '2024-03-04T00:00',
                            '2024-03-10T23:59', '1970-01-01T00:00:00.001')
          for u in ('second', 'minute', 'hour', 'day', 'week', 'month', 'year')]

STEPS = [
    ('2024-01-31', 1, 'month', 1), ('2024-01-31', 1, 'month', 2), ('2024-01-31', 1, 'month', 13),
    ('2024-01-31', 2, 'month', 1), ('2024-02-29', 1, 'year', 1), ('2024-02-29', 1, 'year', 4),
    ('2024-02-29', 1, 'year', -1), ('2023-12-31T18:00', 1, 'month', 2), ('2024-03-05T10:00', 6, 'hour', 5),
    ('2024-03-05', 1, 'week', 3), ('2024-03-05', 3, 'day', -2), ('2024-03-05T10:00', 15, 'minute', 7),
    ('2024-03-05T10:00:00', 30, 'second', 3), ('1970-01-31', 1, 'month', -1), ('2010-01-01', 5, 'year', 3),
]

EXTENTS = [
    ('2010-03-05', '2015-07-01'), ('2024-01-01', '2024-01-01'), ('2024-01-15', '2024-04-02'),
    ('2024-03-05T08:00', '2024-03-06T20:00'), ('2024-03-05T08:00', '2024-03-05T08:20'),
    ('2024-03-05T08:00:00', '2024-03-05T08:00:30'), ('1900-01-01', '2024-12-31'), ('2024-03-05T08:00:00', '2024-03-05T08:00:01'),
    ('2024-03-05', '2024-03-08'),
]
POSITION_STEPS = [(1, 'year'), (2, 'year'), (1, 'month'), (3, 'month'), (1, 'week'), (1, 'day'), (7, 'day'),
                  (1, 'hour'), (1, 'minute'), (1, 'second')]

RULES = [{'ranged': r, 'cumulative': c} for r in (True, False) for c in (False, True)]
VALUES = [
    ('2010-01-01', '2015-01-01'), ('2010-01-01', ''), ('', '2015-01-01'), ('', ''), (None, None), ('yarın', '2015-01-01'),
    ('2010-01-01', 'dün'), ('yarın', 'dün'), ('2012-06-01T12:00', None), (None, '2015-01-01'),
]
WINDOWS = [{'instant': '2012-01-01'}, {'instant': '2010-01-01'}, {'instant': '2015-01-01'}, {'instant': '2009-12-31'},
           {'range': ['2009-01-01', '2010-01-01']}, {'range': ['2014-12-31', '2015-01-01']},
           {'range': ['2015-01-01', '2016-01-01']}, {'range': ['2012-06-01T12:00', '2012-06-01T12:00:00.001']}]

SUMMARIES = [
    ({'ranged': True, 'cumulative': False}, VALUES),
    ({'ranged': False, 'cumulative': False}, VALUES),
    ({'ranged': False, 'cumulative': True}, [('2024-01-01', None), ('2023-05-05T10:00', None), ('', None)]),
    ({'ranged': True, 'cumulative': False}, [('', ''), (None, None)]),
]


def window_of(w):
    if 'instant' in w:
        return {'instant': read(w['instant'])}
    return {'range': [read(w['range'][0]), read(w['range'][1])]}


def cases():
    out = {'format': 'kentos.temporal-cases', 'version': 1, 'source': 'docs/adr/0210 §3–§5; scripts/fixtures/temporal_cases.py'}
    out['reads'] = [{'text': t, 'expect': read(t)} for t in READS]
    out['writes'] = [{'ms': read(t), 'dateOnly': d, 'expect': write(read(t), d)} for t, d in WRITES]
    out['shows'] = [{'ms': read(t), 'unit': u, 'expect': show(read(t), u)} for t, u in SHOWS]
    out['showWindows'] = [{'window': window_of(w), 'unit': u, 'expect': show_window(window_of(w), u)} for w, u in SHOW_WINDOWS]
    out['showEnds'] = [{'first': read(a), 'last': read(b), 'unit': u, 'expect': show_ends(read(a), read(b), u)} for a, b, u in SHOW_ENDS]
    out['floors'] = [{'ms': read(t), 'unit': u, 'expect': floor_to(read(t), u)} for t, u in FLOORS]
    out['steps'] = [{'anchor': read(t), 'n': n, 'unit': u, 'k': k, 'expect': position(read(t), n, u, k)}
                    for t, n, u, k in STEPS]
    out['positions'] = [{'extent': [read(a), read(b)], 'n': n, 'unit': u, 'expect': positions((read(a), read(b)), n, u)}
                        for a, b in EXTENTS for n, u in POSITION_STEPS]
    out['autoSteps'] = [{'extent': [read(a), read(b)], 'expect': auto_step((read(a), read(b)))} for a, b in EXTENTS]
    times = []
    for rule in RULES:
        for start, end in VALUES:
            t = object_time(rule, start, end)
            entry = {'rule': rule, 'start': start, 'end': end,
                     'expect': None if t is None else {'s': number(t[0]), 'e': number(t[1]), 'mode': t[2]}}
            if t is not None:
                entry['windows'] = [{'window': window_of(w), 'expect': shows(t, window_of(w))} for w in WINDOWS]
            times.append(entry)
    out['times'] = times
    out['summaries'] = [{'rule': r, 'values': [list(v) for v in vals], 'expect': summary(r, vals)} for r, vals in SUMMARIES]
    return out


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + '\n'
    if '--check' in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f'{OUT.relative_to(OUT.parents[3])} differs: run without --check and read the difference', file=sys.stderr)
            sys.exit(1)
        print(f'{OUT.relative_to(OUT.parents[3])} matches')
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f'wrote {OUT.relative_to(OUT.parents[3])}')


if __name__ == '__main__':
    main()
