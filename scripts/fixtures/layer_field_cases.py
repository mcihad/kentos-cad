"""The shared cases of a layer's fields (docs/adr/0199 §1, §3): a value
checked against its field (its canonical text or why not), a list of fields'
first problem, fields inferred from the objects' attributes, and a value's
display.

    python3 scripts/fixtures/layer_field_cases.py           # writes the file
    python3 scripts/fixtures/layer_field_cases.py --check   # writes nothing; compares

Writes fixtures/layer-fields/v1/cases.json. The rules are written here from
the ADR on their own, with Python's standard library and no KentOS code. The
contracts (`kentos_contracts::fields`; crates/shared/contracts/tests) and the
web (apps/web/src/model/layerFields.test.ts) give the same values and
messages, word for word.

A field's name in a message is its alias when it has one. Folding is the
Turkish caseless form (I is ı's, İ is i's, any other letter its lowercase
when that is one character). Trimming takes JavaScript's white space.

- Empty (blank after trimming): required refuses it, else it is "".
- Text: as given; `length` counts characters.
- Integer: trimmed, [+-]?digits (ASCII); no '+', no leading zeros, -0 is 0;
  |n| at most 2^53 - 1.
- Decimal: trimmed, [+-]? digits? ([.,] digits?)?, a digit somewhere, at most
  30 digits; '.' as the separator, the whole part without leading zeros (at
  least 0), the fraction as typed, no separator without a fraction, no minus
  when every digit is 0; more fraction digits than `scale` is refused.
- Date: trimmed, YYYY-MM-DD or D.M.YYYY (day and month one or two digits); a
  day of the calendar, year 1..9999; written YYYY-MM-DD.
- Boolean: folded and trimmed evet, true, 1 or hayır, false, 0; written true
  or false.
- Time (docs/adr/0412 §1): H:MM or HH:MM[:SS] (ASCII digits), hour 0..23,
  minutes and seconds 0..59; written HH:MM:SS.
- Date and time (docs/adr/0412 §1): a date as above, then — at the first 'T'
  or space — a time as above (trimmed); a date alone is its midnight; written
  YYYY-MM-DDTHH:MM:SS.
- Big integer (docs/adr/0629 §2): as an integer, from -2^63 to 2^63 - 1.
- Instant (docs/adr/0629 §2): a date as above, then — at the first 'T', 't' or
  space — a time H:MM, HH:MM or HH:MM:SS with at most nine fraction digits
  after the seconds, white space, and the offset Z (z), ±HH:MM, ±HHMM or ±HH,
  at most 18 hours and minutes under 60; written YYYY-MM-DDTHH:MM:SS, the
  fraction without its trailing zeros (none when they all are), Z for a zero
  offset, else ±HH:MM. Ranges compare the moments (the same moment with
  another offset is equal).
- UUID: 32 hex digits, bare or hyphenated 8-4-4-4-12, any case, in braces or
  after urn:uuid: (any case); written lowercase, hyphenated.
- JSON: RFC 8259 (no NaN, no Infinity), at most 1 000 000 characters and 64
  nested levels; written without the white space outside strings.
- Binary: base64 of the standard alphabet, ASCII white space skipped, the
  padding optional; at most 1 MiB decoded; written padded.
- A value list: a label (folded, trimmed) gives its code; else the kind's
  canonical text (a text field's trimmed) must be a code.
- A range: exact, ends included, after the value list.
"""
import base64
import datetime
import json
import re
import sys
from fractions import Fraction
from functools import cmp_to_key
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/layer-fields/v1/cases.json"
JS_SPACE = " \t\n\v\f\r                 　﻿"
MAX_SAFE = 9007199254740991
INT64 = (-2 ** 63, 2 ** 63 - 1)
MAX_JSON, MAX_DEPTH, MAX_BINARY = 1_000_000, 64, 1 << 20
ALPHABET = "abcçdefgğhıijklmnoöpqrsştuüvwxyz"
KIND_WORD = {"text": "metin", "integer": "tam sayı", "decimal": "ondalık sayı", "date": "tarih", "boolean": "evet ya da hayır",
             "datetime": "tarih ve saat", "time": "saat"}


def fold(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


def folded(s):
    return "".join(fold(c) for c in s)


def trim(s):
    return s.strip(JS_SPACE)


def cmp(a, b):
    return (a > b) - (a < b)


def tokens(text):
    return re.findall(r"[0-9]+|.", text, re.S)


def rank(c):
    f = fold(c)
    if not f.isalpha():
        return (0, ord(f))
    i = ALPHABET.find(f)
    return (1, i) if i >= 0 else (2, ord(f))


def natural_cmp(a, b):
    ta, tb = tokens(a), tokens(b)
    for x, y in zip(ta, tb):
        xd, yd = x[0].isdigit() and x.isascii(), y[0].isdigit() and y.isascii()
        if xd and yd:
            o = cmp(int(x), int(y))
        elif xd:
            o = -1
        elif yd:
            o = 1
        else:
            o = cmp(rank(x), rank(y))
        if o:
            return o
    return cmp(len(ta), len(tb)) or cmp(a, b)


def label_of(f):
    return f.get("alias") or f["name"]


def err(code, message):
    return {"error": code, "message": message}


# ── Kinds ──────────────────────────────────────────────────────────────


def parse_integer(s):
    m = re.fullmatch(r"([+-]?)([0-9]+)", s)
    if not m:
        return None
    n = int(m.group(2))
    if m.group(1) == "-":
        n = -n
    return n


def parse_decimal(s):
    """(canonical, digits, fraction digits) or None."""
    m = re.fullmatch(r"([+-]?)([0-9]*)(?:[.,]([0-9]*))?", s)
    if not m or not (m.group(2) or m.group(3)):
        return None
    whole, frac = m.group(2), m.group(3) or ""
    digits = len(whole) + len(frac)
    whole = whole.lstrip("0") or "0"
    zero = set(whole + frac) <= {"0"}
    text = whole + ("." + frac if frac else "")
    if m.group(1) == "-" and not zero:
        text = "-" + text
    return text, digits, len(frac)


def days_in(y, m):
    if m == 2:
        return 29 if (y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)) else 28
    return 30 if m in (4, 6, 9, 11) else 31


def parse_date(s):
    m = re.fullmatch(r"([0-9]{4})-([0-9]{2})-([0-9]{2})", s)
    if m:
        y, mo, d = int(m.group(1)), int(m.group(2)), int(m.group(3))
    else:
        m = re.fullmatch(r"([0-9]{1,2})\.([0-9]{1,2})\.([0-9]{4})", s)
        if not m:
            return None
        d, mo, y = int(m.group(1)), int(m.group(2)), int(m.group(3))
    if not (1 <= y <= 9999 and 1 <= mo <= 12 and 1 <= d <= days_in(y, mo)):
        return None
    return f"{y:04d}-{mo:02d}-{d:02d}"


def parse_time(s):
    m = re.fullmatch(r"([0-9]{1,2}):([0-9]{2})(?::([0-9]{2}))?", s)
    if m is None:
        return None
    h, mi, se = int(m.group(1)), int(m.group(2)), int(m.group(3) or 0)
    if h > 23 or mi > 59 or se > 59:
        return None
    return f"{h:02d}:{mi:02d}:{se:02d}"


def parse_datetime(s):
    cut = [i for i in (s.find("T"), s.find(" ")) if i >= 0]
    if not cut:
        day, clock = s, None
    else:
        i = min(cut)
        day, clock = s[:i], trim(s[i + 1:])
    d = parse_date(day)
    if d is None:
        return None
    t = "00:00:00" if clock is None else parse_time(clock)
    return None if t is None else f"{d}T{t}"


def parse_instant(s):
    """docs/adr/0629 §2: the canonical text of an instant, or None."""
    cut = [i for i in (s.find("T"), s.find("t"), s.find(" ")) if i >= 0]
    if not cut:
        return None
    i = min(cut)
    day = parse_date(s[:i])
    if day is None:
        return None
    rest = trim(s[i + 1:])
    if rest[-1:] in ("Z", "z"):
        clock, minutes = trim(rest[:-1]), 0
    else:
        at = max(rest.rfind("+"), rest.rfind("-"))
        if at < 0:
            return None
        clock, zone = trim(rest[:at]), rest[at:]
        m = re.fullmatch(r"([+-])([0-9]{2})(?::?([0-9]{2}))?", zone)
        if m is None or (len(zone) == 4):
            return None
        h, mi = int(m.group(2)), int(m.group(3) or 0)
        if mi > 59 or h * 60 + mi > 18 * 60:
            return None
        minutes = (-1 if m.group(1) == "-" else 1) * (h * 60 + mi)
    m = re.fullmatch(r"([0-9]{1,2}:[0-9]{2}(?::[0-9]{2})?)(?:\.([0-9]+))?", clock)
    if m is None or (m.group(2) is not None and (m.group(1).count(":") != 2 or len(m.group(2)) > 9)):
        return None
    t = parse_time(m.group(1))
    if t is None:
        return None
    frac = (m.group(2) or "").rstrip("0")
    zone = "Z" if minutes == 0 else f"{'-' if minutes < 0 else '+'}{abs(minutes) // 60:02d}:{abs(minutes) % 60:02d}"
    return f"{day}T{t}{'.' + frac if frac else ''}{zone}"


def moment(v):
    """An instant's (seconds since the epoch, nanoseconds): the day's ordinal and the clock, the offset taken away."""
    c = parse_instant(v)
    day = datetime.date(int(c[0:4]), int(c[5:7]), int(c[8:10])).toordinal()
    secs = day * 86400 + int(c[11:13]) * 3600 + int(c[14:16]) * 60 + int(c[17:19])
    rest = c[19:]
    nanos = 0
    if rest.startswith("."):
        digits = re.match(r"\.([0-9]+)", rest).group(1)
        nanos = int(digits.ljust(9, "0"))
        rest = rest[1 + len(digits):]
    off = 0 if rest == "Z" else (-1 if rest[0] == "-" else 1) * (int(rest[1:3]) * 60 + int(rest[4:6]))
    return (secs - off * 60, nanos)


def parse_uuid(s):
    t = s[9:] if len(s) > 9 and s[:9].lower() == "urn:uuid:" else s
    if t.startswith("{") and t.endswith("}") and len(t) >= 2:
        t = t[1:-1]
    if re.fullmatch(r"[0-9A-Fa-f]{32}", t):
        h = t
    elif re.fullmatch(r"[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}", t):
        h = t.replace("-", "")
    else:
        return None
    h = h.lower()
    return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"


def parse_json(s):
    """("ok", canonical), ("deep", None) or ("bad", None)."""

    def reject(name):
        raise ValueError(name)
    try:
        json.loads(s, parse_constant=reject)
    except (ValueError, RecursionError):
        return ("bad", None)
    pieces = re.findall(r'"(?:[^"\\]|\\.)*"|[^ \t\n\r]', s)
    depth = top = 0
    for p in pieces:
        if p in ("[", "{"):
            depth += 1
            top = max(top, depth)
        elif p in ("]", "}"):
            depth -= 1
    if top > MAX_DEPTH:
        return ("deep", None)
    return ("ok", "".join(pieces))


def parse_binary(s):
    t = re.sub(r"[ \t\n\f\r]", "", s)
    if t == "" or len(t) % 4 == 1:
        return None
    t += "=" * ((4 - len(t) % 4) % 4)
    if not re.fullmatch(r"[A-Za-z0-9+/]*={0,2}", t):
        return None
    try:
        return base64.b64decode(t, validate=True)
    except ValueError:
        return None


def parse_boolean(s):
    f = folded(s)
    if f in ("evet", "true", "1"):
        return "true"
    if f in ("hayır", "false", "0"):
        return "false"
    return None


def number(text):
    return Fraction(text)


def kind_value(f, s, text):
    """The kind's canonical text of trimmed `s`, or an error."""
    k, L = f["kind"], label_of(f)
    if k == "integer":
        n = parse_integer(s)
        if n is None:
            return err("type", f"“{L}” alanı tam sayı ister; “{text}” verildi. Rakamlarla, ondalıksız yazın.")
        if abs(n) > MAX_SAFE:
            return err("magnitude", f"“{L}” alanının sayısı çok büyük; “{text}” verildi. Mutlak değeri en çok {MAX_SAFE} olabilir.")
        return {"value": str(n)}
    if k == "decimal":
        d = parse_decimal(s)
        if d is None:
            return err("type", f"“{L}” alanı ondalık sayı ister; “{text}” verildi. Rakamlarla, ondalığı nokta ya da virgülle yazın.")
        canonical, digits, frac = d
        if digits > 30:
            return err("magnitude", f"“{L}” alanının sayısı çok uzun; “{text}” verildi. En çok 30 rakam olabilir.")
        scale = f.get("scale")
        if scale is not None and frac > scale:
            cut = frac - scale
            if set(canonical[-cut:]) != {"0"}:
                return err("scale", f"“{L}” alanı en çok {scale} ondalık basamak alır; “{text}” verildi. Değer yuvarlanmaz; basamakları azaltarak yazın.")
            # docs/adr/0628 §5: zeros past the scale say nothing (a DBF's N(24,15)); cut, not rounded.
            canonical = canonical[:-cut]
            if canonical.endswith("."):
                canonical = canonical[:-1]
        return {"value": canonical}
    if k == "date":
        v = parse_date(s)
        if v is None:
            return err("type", f"“{L}” alanı tarih ister; “{text}” verildi. GG.AA.YYYY ya da YYYY-AA-GG yazın.")
        return {"value": v}
    if k == "datetime":
        v = parse_datetime(s)
        if v is None:
            return err("type", f"“{L}” alanı tarih ve saat ister; “{text}” verildi. GG.AA.YYYY SS:DD ya da YYYY-AA-GG SS:DD yazın.")
        return {"value": v}
    if k == "time":
        v = parse_time(s)
        if v is None:
            return err("type", f"“{L}” alanı saat ister; “{text}” verildi. SS:DD ya da SS:DD:SS yazın (00:00–23:59).")
        return {"value": v}
    if k == "boolean":
        v = parse_boolean(s)
        if v is None:
            return err("type", f"“{L}” alanı evet ya da hayır ister; “{text}” verildi.")
        return {"value": v}
    if k == "bigint":
        n = parse_integer(s)
        if n is None:
            return err("type", f"“{L}” alanı tam sayı ister; “{text}” verildi. Rakamlarla, ondalıksız yazın.")
        if not INT64[0] <= n <= INT64[1]:
            return err("magnitude", f"“{L}” alanının sayısı çok büyük; “{text}” verildi. {INT64[0]} ile {INT64[1]} arasında olabilir.")
        return {"value": str(n)}
    if k == "instant":
        v = parse_instant(s)
        if v is None:
            return err("type", f"“{L}” alanı saat dilimli tarih ve saat ister; “{text}” verildi. YYYY-AA-GG SS:DD:SS+03:00 ya da …Z yazın.")
        return {"value": v}
    if k == "uuid":
        v = parse_uuid(s)
        if v is None:
            return err("type", f"“{L}” alanı UUID ister; “{text}” verildi. 32 onaltılık rakamı 8-4-4-4-12 düzeninde yazın.")
        return {"value": v}
    if k == "json":
        if len(s) > MAX_JSON:
            return err("magnitude", f"“{L}” alanının JSON değeri çok uzun; en çok {MAX_JSON} karakter olabilir.")
        how, v = parse_json(s)
        if how == "bad":
            return err("type", f"“{L}” alanı geçerli bir JSON değeri ister; “{text}” JSON değil.")
        if how == "deep":
            return err("magnitude", f"“{L}” alanının JSON değeri çok derin; en çok {MAX_DEPTH} düzey iç içe olabilir.")
        return {"value": v}
    if k == "binary":
        b = parse_binary(s)
        if b is None:
            return err("type", f"“{L}” alanı ikili değeri base64 olarak ister; “{text}” base64 değil.")
        if len(b) > MAX_BINARY:
            return err("magnitude", f"“{L}” alanının ikili değeri çok büyük: {len(b)} bayt. En çok {MAX_BINARY} bayt (1 MiB) olabilir.")
        return {"value": base64.b64encode(b).decode()}
    return {"value": text}



def shown(f, v):
    """A range's value as the interface shows it (docs/adr/0436): a date GG.AA.YYYY, a date and time GG.AA.YYYY SS:DD
    (its seconds when some), a time of day SS:DD; a number as it is."""
    if v is None:
        return None
    if f["kind"] == "date":
        return f"{v[8:10]}.{v[5:7]}.{v[0:4]}"
    if f["kind"] == "datetime":
        clock = v[11:]
        clock = clock[:-3] if clock.endswith(":00") else clock
        return f"{v[8:10]}.{v[5:7]}.{v[0:4]} {clock}"
    if f["kind"] == "time":
        return v[:-3] if v.endswith(":00") else v
    if f["kind"] == "instant":
        return display(f, v)
    return v


def order_of(kind):
    """docs/adr/0436, 0629 §2: a date, a date and time or a time of day by its text; an instant by its moment; a number
    exactly."""
    if kind in ("date", "datetime", "time"):
        return lambda t: t
    if kind == "instant":
        return moment
    return number

def check(f, text):
    """A value written to field `f`: its canonical text, or why not."""
    L = label_of(f)
    s = trim(text)
    if s == "":
        if f.get("required"):
            return err("required", f"“{L}” alanı zorunlu; boş bırakılamaz.")
        return {"value": ""}
    if f["kind"] == "text":
        length = f.get("length")
        n = len(text)
        if length is not None and n > length:
            return err("length", f"“{L}” alanı en çok {length} karakter alır; {n} karakter verildi.")
    values = f.get("values")
    if not values and f.get("multiple") and f["kind"] == "text":
        # docs/adr/0409 §1: an unlisted multiple field's values in their order, once each.
        seen = []
        for part in [trim(p) for p in s.split(";")]:
            if part and part not in seen:
                seen.append(part)
        if not seen and f.get("required"):
            return err("required", f"“{L}” alanı zorunlu; boş bırakılamaz.")
        return {"value": "; ".join(seen)}
    if values and f.get("multiple") and f["kind"] == "text":
        # docs/adr/0408 §1: each part a label or a code, written in the list's order once each, joined with "; ".
        picked = set()
        for part in [trim(p) for p in s.split(";")]:
            if not part:
                continue
            at = next((i for i, v in enumerate(values) if folded(trim(v["label"])) == folded(part)), None)
            if at is None:
                at = next((i for i, v in enumerate(values) if v["code"] == part), None)
            if at is None:
                return err("choice", f"“{L}” alanı listedeki değerleri ister; “{part}” listede yok.")
            picked.add(at)
        if not picked and f.get("required"):
            return err("required", f"“{L}” alanı zorunlu; boş bırakılamaz.")
        return {"value": "; ".join(v["code"] for i, v in enumerate(values) if i in picked)}
    if values:
        for v in values:
            if folded(trim(v["label"])) == folded(s):
                return {"value": v["code"]}
    r = kind_value(f, s, text)
    if "error" in r:
        return r
    # A text field's code is matched as trimmed; without a list the text stays as given.
    v = s if f["kind"] == "text" and values else r["value"]
    if values and v not in [c["code"] for c in values]:
        return err("choice", f"“{L}” alanı listedeki değerlerden birini ister; “{text}” listede yok.")
    # docs/adr/0436: a date, a date and time or a time of day by its text, which sorts as written.
    if f["kind"] in ("integer", "decimal", "date", "datetime", "time", "bigint", "instant"):
        lo, hi = f.get("min"), f.get("max")
        order = order_of(f["kind"])
        x = order(v)
        # docs/adr/0438 §4: an end left out is outside too.
        lo_out, hi_out = bool(f.get("minExclusive")), bool(f.get("maxExclusive"))
        below = lo is not None and (x < order(lo) or (lo_out and x == order(lo)))
        above = hi is not None and (x > order(hi) or (hi_out and x == order(hi)))
        if below or above:
            unit = f.get("unit")

            def say(t):
                s = shown(f, t)
                return f"{s} {unit}" if unit and s else s
            lo, hi, v = say(lo), say(hi), say(v)
            if lo is not None and hi is not None:
                out = {(True, True): " (iki uç da hariç)", (True, False): f" ({lo} hariç)",
                       (False, True): f" ({hi} hariç)", (False, False): ""}[(lo_out, hi_out)]
                return err("range", f"“{L}” alanı {lo} ile {hi} arasında olmalı{out}; {v} verildi.")
            if lo is not None:
                if lo_out:
                    return err("range", f"“{L}” alanı {lo} değerinden büyük olmalı; {v} verildi.")
                return err("range", f"“{L}” alanı en az {lo} olmalı; {v} verildi.")
            if hi_out:
                return err("range", f"“{L}” alanı {hi} değerinden küçük olmalı; {v} verildi.")
            return err("range", f"“{L}” alanı en çok {hi} olmalı; {v} verildi.")
    return {"value": v}


# ── A field list's problems ────────────────────────────────────────────


def bare(f):
    """The field with only its kind's rules (a range end or a code is checked so)."""
    return {"name": f["name"], **({"alias": f["alias"]} if "alias" in f else {}), "kind": f["kind"], **({"scale": f["scale"]} if "scale" in f else {})}


def canonical_of(f, v):
    r = kind_value(bare(f), trim(v), v) if trim(v) else {"error": "empty"}
    return r.get("value") == v


def field_problem(f):
    name = f["name"]
    if trim(name) == "":
        return "Alanın adı boş olamaz."
    if trim(name) != name:
        return f"“{name}” alanının adının başında ya da sonunda boşluk var."
    if len(name) > 64:
        return f"“{name}” alanının adı en çok 64 karakter olabilir."
    if any(ord(c) < 32 or 127 <= ord(c) < 160 for c in name):
        return f"“{name}” alanının adında denetim karakteri var."
    if "alias" in f:
        if trim(f["alias"]) == "":
            return f"“{name}” alanının takma adı boş olamaz."
        if len(f["alias"]) > 64:
            return f"“{name}” alanının takma adı en çok 64 karakter olabilir."
    k = f["kind"]
    if "length" in f:
        if k != "text":
            return f"“{name}” alanında uzunluk yalnız metin alanında olur."
        if not 1 <= f["length"] <= 10000:
            return f"“{name}” alanının uzunluğu 1 ile 10000 arasında olmalı."
    if "scale" in f:
        if k != "decimal":
            return f"“{name}” alanında ondalık basamak yalnız ondalık sayı alanında olur."
        if not 0 <= f["scale"] <= 15:
            return f"“{name}” alanının ondalık basamağı 0 ile 15 arasında olmalı."
    if "min" in f or "max" in f:
        if k not in ("integer", "decimal", "date", "datetime", "time", "bigint", "instant"):
            return f"“{name}” alanında aralık yalnız sayı, tarih ve saat alanlarında olur."
        for key, word in (("min", "en azı"), ("max", "en çoğu")):
            if key in f and not canonical_of(f, f[key]):
                return f"“{name}” alanının {word} “{f[key]}” alanın türüne uymuyor."
        order = order_of(k)
        if "min" in f and "max" in f and order(f["min"]) > order(f["max"]):
            return f"“{name}” alanının en azı en çoğundan büyük olamaz."
    if "values" in f:
        if k not in ("text", "integer", "decimal", "bigint"):
            return f"“{name}” alanında değer listesi yalnız metin ve sayı alanlarında olur."
        if not f["values"]:
            return f"“{name}” alanının değer listesi boş; listeyi kaldırın ya da değer ekleyin."
        codes, labels = set(), set()
        for v in f["values"]:
            code, label = v["code"], v["label"]
            if code == "":
                return f"“{name}” alanının değer listesinde boş kod var."
            if k != "text" and not canonical_of(f, code):
                return f"“{name}” alanının değer listesindeki “{code}” kodu alanın türüne uymuyor."
            if trim(label) == "":
                return f"“{name}” alanının değer listesinde “{code}” kodunun etiketi boş."
            if code in codes:
                return f"“{name}” alanının değer listesinde “{code}” kodu iki kez var."
            if folded(trim(label)) in labels:
                return f"“{name}” alanının değer listesinde “{label}” etiketi iki kez var."
            if f.get("multiple") and (";" in code or ";" in label):
                return f"“{name}” çok değerli; değer listesinin kodlarında ve etiketlerinde “;” olamaz."
            codes.add(code)
            labels.add(folded(trim(label)))
    # docs/adr/0419: sharing and summing keep the total to the last digit; the least and greatest are numbers'.
    units = k == "integer" or (k == "decimal" and "scale" in f)
    if f.get("split") == "ratio" and not units:
        return f"“{name}” alanı oranla paylaştırılamaz: bu tam sayı ya da ondalık basamağı verilmiş ondalık sayı alanında olur."
    if f.get("merge") == "sum" and not units:
        return f"“{name}” alanı toplanamaz: bu tam sayı ya da ondalık basamağı verilmiş ondalık sayı alanında olur."
    if f.get("merge") in ("min", "max") and k not in ("integer", "decimal"):
        return f"“{name}” alanında en küçüğü ve en büyüğü yalnız sayı alanlarında olur."
    # docs/adr/0438 §3–§5: the range's left-out ends, the unit, unique, a value's description and days.
    if f.get("minExclusive") and "min" not in f:
        return f"“{name}” alanının alt ucu en azı olmadan hariç tutulamaz."
    if f.get("maxExclusive") and "max" not in f:
        return f"“{name}” alanının üst ucu en çoğu olmadan hariç tutulamaz."
    if "min" in f and "max" in f and (f.get("minExclusive") or f.get("maxExclusive")):
        order = order_of(k)
        if order(f["min"]) == order(f["max"]):
            return f"“{name}” alanının aralığı boş: en azı en çoğuna eşitken uçlar hariç tutulamaz."
    if "unit" in f:
        u = f["unit"]
        if k not in ("integer", "decimal", "bigint"):
            return f"“{name}” alanında birim yalnız sayı alanlarında olur."
        if trim(u) == "" or trim(u) != u or len(u) > 16 or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in u):
            return f"“{name}” alanının birimi boş olamaz, en çok 16 karakterdir, boşlukla başlayıp bitmez."
    if f.get("unique") and k == "boolean":
        return f"“{name}” evet/hayır alanı; tekil olamaz (iki değeri var)."
    if f.get("unique") and f.get("multiple"):
        return f"“{name}” çok değerli; tekil olamaz."
    for v in f.get("values", []):
        code = v["code"]
        if "description" in v and (trim(v["description"]) == "" or len(v["description"]) > 500):
            return f"“{name}” alanının “{code}” değerinin açıklaması boş olamaz ve en çok 500 karakterdir."
        for key, word in (("from", "başlangıcı"), ("until", "bitişi")):
            if key in v and parse_date(v[key]) != v[key]:
                return f"“{name}” alanının “{code}” değerinin geçerlilik {word} “{v[key]}” bir tarih değil (YYYY-AA-GG)."
        if "from" in v and "until" in v and v["from"] > v["until"]:
            return f"“{name}” alanının “{code}” değerinin geçerliliği bitişinden sonra başlıyor."
    # docs/adr/0418: at least one value is offered; the default is not retired.
    if f.get("values") and all(v.get("retired") for v in f["values"]):
        return f"“{name}” alanının bütün değerleri kullanımdan kalkmış; en az biri sunulmalı."
    if f.get("multiple") and k != "text":
        return f"“{name}” alanı çok değerli olamaz: bu yalnız metin alanında olur."
    if "default" in f:
        d = f["default"]
        r = check({**f, "required": False}, d)
        if trim(d) == "" or "error" in r or r["value"] != d:
            return f"“{name}” alanının varsayılanı “{d}” alanın kurallarına uymuyor."
        gone = [v["code"] for v in f.get("values", []) if v.get("retired")]
        parts = [trim(p) for p in d.split(";")] if f.get("multiple") else [d]
        for p in parts:
            if p in gone:
                return f"“{name}” alanının varsayılanı “{p}” kullanımdan kalkmış bir değer."
    return None


def fields_problem(fields):
    seen = set()
    for f in fields:
        p = field_problem(f)
        if p:
            return p
        key = folded(f["name"])
        if key in seen:
            return f"“{f['name']}” adlı iki alan var; alan adları bir kez kullanılır."
        seen.add(key)
    return None


# ── Inferred fields and display ────────────────────────────────────────


def infer(rows):
    keys = {}
    for r in rows:
        for k, v in r.items():
            keys.setdefault(k, [])
            if trim(v) != "":
                keys[k].append(trim(v))
    out = []
    for k in sorted(keys, key=cmp_to_key(natural_cmp)):
        vs = keys[k]
        f = {"name": k, "kind": "text"}
        if vs:
            if all(parse_integer(v) is not None and abs(parse_integer(v)) <= MAX_SAFE for v in vs):
                f = {"name": k, "kind": "integer"}
            elif all(parse_integer(v) is not None and INT64[0] <= parse_integer(v) <= INT64[1] for v in vs):
                # docs/adr/0629 §2: whole numbers past 2^53, within 64 bits.
                f = {"name": k, "kind": "bigint"}
            elif all(parse_decimal(v) is not None and parse_decimal(v)[1] <= 30 for v in vs):
                scale = max(parse_decimal(v)[2] for v in vs)
                f = {"name": k, "kind": "decimal", **({"scale": scale} if scale <= 15 else {})}
                if scale > 15:
                    f = {"name": k, "kind": "text"}
            elif all(parse_date(v) is not None for v in vs):
                f = {"name": k, "kind": "date"}
            elif all(":" in v and parse_datetime(v) is not None for v in vs):
                f = {"name": k, "kind": "datetime"}
            elif all(parse_instant(v) is not None for v in vs):
                f = {"name": k, "kind": "instant"}
            elif all(parse_time(v) is not None for v in vs):
                f = {"name": k, "kind": "time"}
            elif all(folded(v) in ("evet", "hayır", "true", "false") for v in vs):
                f = {"name": k, "kind": "boolean"}
            elif all(re.fullmatch(r"[0-9A-Fa-f]{8}(?:-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}", v) for v in vs):
                # docs/adr/0629 §2: hyphenated UUIDs only (32 bare hex digits may be a hash).
                f = {"name": k, "kind": "uuid"}
        out.append(f)
    return out


def display(f, v):
    if f.get("multiple") and f["kind"] == "text" and f.get("values"):
        labels = {c["code"]: c["label"] for c in f["values"]}
        return "; ".join(labels.get(c, c) for c in (trim(p) for p in v.split(";")) if c)
    if f.get("values"):
        for c in f["values"]:
            if c["code"] == v:
                return c["label"]
    if f["kind"] == "boolean":
        return {"true": "Evet", "false": "Hayır"}.get(v, v)
    if f["kind"] == "date" and re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", v):
        return f"{v[8:10]}.{v[5:7]}.{v[0:4]}"
    if f["kind"] == "datetime" and parse_datetime(v) == v:
        clock = v[11:]
        clock = clock[:-3] if clock.endswith(":00") else clock
        return f"{v[8:10]}.{v[5:7]}.{v[0:4]} {clock}"
    if f["kind"] == "time" and parse_time(v) == v:
        return v[:-3] if v.endswith(":00") else v
    if f["kind"] == "instant" and parse_instant(v) == v:
        rest = v[11:]
        at = min([i for i in (rest.find("Z"), rest.find("+"), rest.find("-")) if i >= 0], default=len(rest))
        zone = "UTC" if rest[at:] == "Z" else rest[at:]
        return f"{v[8:10]}.{v[5:7]}.{v[0:4]} {rest[:at]} {zone}"
    if f["kind"] == "binary":
        b = parse_binary(v)
        return v if b is None else f"ikili, {len(b)} bayt"
    return v


# ── Cases ──────────────────────────────────────────────────────────────

INT = {"name": "Kat", "kind": "integer"}
INT_R = {"name": "Kat", "alias": "Kat sayısı", "kind": "integer", "min": "1", "max": "40"}
DEC = {"name": "Alan", "kind": "decimal"}
DEC2 = {"name": "Alan", "kind": "decimal", "scale": 2, "min": "0"}
DEC0 = {"name": "Kat sayısı", "kind": "decimal", "scale": 0}
DATE = {"name": "Tarih", "kind": "date"}
BOOL = {"name": "Ruhsat", "kind": "boolean"}
# docs/adr/0412 §1: a date and time, a time of day.
DT = {"name": "Tespit", "kind": "datetime"}
TM = {"name": "Vardiya", "kind": "time"}
TEXT5 = {"name": "Kod", "kind": "text", "length": 5}
REQ = {"name": "Ada", "kind": "text", "required": True}
LIST_T = {"name": "Kullanım", "kind": "text", "values": [{"code": "K", "label": "Konut"}, {"code": "T", "label": "Ticaret"}, {"code": "İ", "label": "İşyeri"}]}
LIST_N = {"name": "Tür", "kind": "integer", "values": [{"code": "1", "label": "Mesken"}, {"code": "2", "label": "İşyeri"}]}
# docs/adr/0408 §1: several codes of the list.
MANY = {**LIST_T, "multiple": True}
MANY_REQ = {**MANY, "required": True}
# docs/adr/0409 §1: several free values (objects' ids for a many-to-many reference).
FREE = {"name": "Malikler", "kind": "text", "multiple": True}
# docs/adr/0436: ranges of dates, dates and times, and times of day.
DATE_R = {"name": "Tespit tarihi", "kind": "date", "min": "2000-01-01", "max": "2026-12-31"}
DT_R = {"name": "Tespit", "kind": "datetime", "min": "2026-10-01T08:00:00"}
TM_R = {"name": "Vardiya", "kind": "time", "min": "08:00:00", "max": "17:30:00"}

CHECKS = [
    (INT, "12"), (INT, "+012"), (INT, "-0"), (INT, " 42 "), (INT, "1.5"), (INT, "1,000"), (INT, "9007199254740991"),
    (INT, "-9007199254740992"), (INT, ""), (INT, "١٢"), (INT_R, "0"), (INT_R, "40"), (INT_R, "41"),
    (DEC, "12,50"), (DEC, "+0012.3"), (DEC, ",5"), (DEC, "5."), (DEC, "-0.00"), (DEC, "-0,5"), (DEC, "1e3"), (DEC, "."),
    (DEC, "1234567890123456789012345678901"), (DEC2, "12.345"), (DEC2, "12.34"), (DEC2, "-1"), (DEC2, "007"),
    (DEC2, "12.500"), (DEC2, "105.500000000000000"), (DEC2, "12.3450"), (DEC2, "0,1000"), (DEC0, "100.000"),
    (DEC0, "-0.0"), (DEC0, "7,5"),
    (DATE, "07.10.2026"), (DATE, "7.1.2026"), (DATE, "2026-10-07"), (DATE, "2026-02-29"), (DATE, "2024-02-29"),
    (DATE, "29.02.2023"), (DATE, "2026-1-7"), (DATE, "0000-01-01"), (DATE, "31.04.2026"), (DATE, "1900-02-29"), (DATE, "2000-02-29"),
    (BOOL, "Evet"), (BOOL, "HAYIR"), (BOOL, "true"), (BOOL, "0"), (BOOL, "yes"), (BOOL, " İ"),
    (TEXT5, "abcde"), (TEXT5, "abcdef"), (TEXT5, "  ab  "), (TEXT5, "çğıöşü"),
    (REQ, ""), (REQ, "   "), (REQ, "101"),
    (LIST_T, "K"), (LIST_T, "konut"), (LIST_T, "TİCARET"), (LIST_T, "işyeri"), (LIST_T, "k"), (LIST_T, "Sanayi"), (LIST_T, "İ"),
    (LIST_N, "1"), (LIST_N, "01"), (LIST_N, "işyeri"), (LIST_N, "3"), (LIST_N, "x"),
    (MANY, "K"), (MANY, "T; K"), (MANY, "ticaret;konut; TİCARET"), (MANY, " İ ; ; K "), (MANY, "K; Sanayi"),
    (MANY, ";"), (MANY_REQ, " ; "), (MANY, "işyeri;İ"), (MANY, "K, T"),
    (FREE, "b; a; b"), (FREE, " x ;; y "), ({**FREE, "required": True}, ";"), (FREE, "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001"),
    (DT, "16.10.2026 14:30"), (DT, "2026-10-16T14:30:05"), (DT, "2026-10-16 9:05"), (DT, "2026-10-16"), (DT, "16.10.2026   14:30"),
    (DT, "2026-10-16T24:00"), (DT, "2026-10-16T14:3"), (DT, "31.04.2026 10:00"), (DT, "14:30"), (DT, "2026-10-16T14:30:00Z"),
    (TM, "8:05"), (TM, "08:05:09"), (TM, "23:59"), (TM, "24:00"), (TM, "7:5"), (TM, "12:60"), (TM, "12:30:60"), (TM, "12"),
    (TM, "١٢:٣٠"), (TM, " 06:00 "),
    (DATE_R, "1999-12-31"), (DATE_R, "01.01.2000"), (DATE_R, "2026-12-31"), (DATE_R, "1.1.2027"),
    (DT_R, "2026-10-01T07:59:59"), (DT_R, "01.10.2026 08:00"), (TM_R, "7:59"), (TM_R, "17:30"), (TM_R, "17:31"),
]

PROBLEMS = [
    ("geçerli", [INT, DEC2, DATE, BOOL, TEXT5, REQ, LIST_T, LIST_N]),
    ("boş-ad", [{"name": "", "kind": "text"}]),
    ("boşluklu-ad", [{"name": " Ada", "kind": "text"}]),
    ("uzun-ad", [{"name": "a" * 65, "kind": "text"}]),
    ("denetim-karakteri", [{"name": "A\tda", "kind": "text"}]),
    ("boş-takma-ad", [{"name": "Ada", "alias": " ", "kind": "text"}]),
    ("aynı-ad", [{"name": "Kullanım", "kind": "text"}, {"name": "KULLANIM", "kind": "text"}]),
    ("aynı-ad-i", [{"name": "il", "kind": "text"}, {"name": "İL", "kind": "text"}]),
    ("sayıda-uzunluk", [{"name": "Kat", "kind": "integer", "length": 3}]),
    ("uzunluk-sınırı", [{"name": "Kod", "kind": "text", "length": 0}]),
    ("metinde-basamak", [{"name": "Kod", "kind": "text", "scale": 2}]),
    ("basamak-sınırı", [{"name": "Alan", "kind": "decimal", "scale": 16}]),
    ("tarihte-aralık", [{"name": "Tarih", "kind": "date", "min": "2026-01-01"}]),
    ("aralık-türü", [{"name": "Kat", "kind": "integer", "min": "1.5"}]),
    ("aralık-tek-biçim", [{"name": "Kat", "kind": "integer", "max": "+5"}]),
    ("ters-aralık", [{"name": "Alan", "kind": "decimal", "min": "10.5", "max": "2"}]),
    ("tarihte-liste", [{"name": "Tarih", "kind": "date", "values": [{"code": "2026-01-01", "label": "Yılbaşı"}]}]),
    ("boş-liste", [{"name": "Kod", "kind": "text", "values": []}]),
    ("boş-kod", [{"name": "Kod", "kind": "text", "values": [{"code": "", "label": "Hiç"}]}]),
    ("kod-türü", [{"name": "Tür", "kind": "integer", "values": [{"code": "1", "label": "A"}, {"code": "02", "label": "B"}]}]),
    ("boş-etiket", [{"name": "Kod", "kind": "text", "values": [{"code": "A", "label": " "}]}]),
    ("aynı-kod", [{"name": "Kod", "kind": "text", "values": [{"code": "A", "label": "Bir"}, {"code": "A", "label": "İki"}]}]),
    ("aynı-etiket", [{"name": "Kod", "kind": "text", "values": [{"code": "A", "label": "Konut"}, {"code": "B", "label": "KONUT"}]}]),
    ("varsayılan", [{"name": "Kat", "kind": "integer", "default": "3"}, {"name": "Alan", "kind": "decimal", "scale": 1, "default": "2.5"}]),
    ("varsayılan-tek-biçim", [{"name": "Kat", "kind": "integer", "default": "03"}]),
    ("varsayılan-aralık", [{"name": "Kat", "kind": "integer", "min": "1", "default": "0"}]),
    ("varsayılan-boş", [{"name": "Ada", "kind": "text", "default": ""}]),
    ("varsayılan-liste", [{"name": "Kullanım", "kind": "text", "values": [{"code": "K", "label": "Konut"}], "default": "Konut"}]),
    ("ilk-sorun", [{"name": "Kat", "kind": "integer", "length": 3}, {"name": "", "kind": "text"}]),
    ("çok-değerli", [MANY, {**MANY, "name": "Kullanım 2", "default": "K; İ"}]),
    ("çok-değerli-listesiz", [{"name": "Not", "kind": "text", "multiple": True}]),
    ("çok-değerli-sayı", [{**LIST_N, "multiple": True}]),
    ("çok-değerli-noktalı-virgül", [{"name": "Kod", "kind": "text", "multiple": True, "values": [{"code": "a;b", "label": "AB"}]}]),
    ("çok-değerli-varsayılan-sırası", [{**MANY, "default": "T; K"}]),
    ("tarih-saatte-aralık", [{**DT, "min": "2026-01-01T00:00:00"}]),
    ("tarihte-ters-aralık", [{**DATE, "min": "2026-12-31", "max": "2026-01-01"}]),
    ("tarih-aralığı-tek-biçim", [{**DATE, "min": "1.1.2026"}]),
    ("saatte-aralık", [TM_R]),
    ("metinde-aralık", [{"name": "Kod", "kind": "text", "min": "a"}]),
    ("saatte-liste", [{**TM, "values": [{"code": "08:00:00", "label": "Sabah"}]}]),
    ("saat-varsayılanı", [{**TM, "default": "08:30:00"}, {**DT, "default": "2026-10-16T08:30:00"}]),
    ("saat-varsayılanı-tek-biçim", [{**TM, "default": "8:30"}]),
    ("paylaştırma-ve-toplam", [{"name": "Pay", "kind": "integer", "split": "ratio", "merge": "sum"},
                               {"name": "Alan", "kind": "decimal", "scale": 2, "split": "ratio", "merge": "max"},
                               {"name": "Not", "kind": "text", "split": "clear", "merge": "first"}]),
    ("basamaksız-ondalık-paylaşılmaz", [{"name": "Alan", "kind": "decimal", "split": "ratio"}]),
    ("metin-toplanmaz", [{"name": "Ad", "kind": "text", "merge": "sum"}]),
    ("tarihte-en-küçük-yok", [{"name": "Tarih", "kind": "date", "merge": "min"}]),
    ("kalkmış-değer", [{"name": "Tür", "kind": "text", "values": [{"code": "K", "label": "Konut"}, {"code": "E", "label": "Eski", "retired": True}]}]),
    ("bütün-değerler-kalkmış", [{"name": "Tür", "kind": "text", "values": [{"code": "E", "label": "Eski", "retired": True}]}]),
    ("kalkmış-varsayılan", [{"name": "Tür", "kind": "text", "values": [{"code": "K", "label": "Konut"}, {"code": "E", "label": "Eski", "retired": True}],
                             "default": "E"}]),
    ("çok-değerli-kalkmış-varsayılan", [{**MANY, "values": [dict(v, retired=True) if v["code"] == "T" else v for v in MANY["values"]],
                                         "default": "K; T"}]),
]

INFER = [
    ("karışık", [{"Kat": "3", "Alan": "120.5", "Tarih": "07.10.2026", "Ruhsat": "Evet", "Not": "köşe"},
                 {"Kat": "12", "Alan": "98,25", "Tarih": "2026-01-02", "Ruhsat": "hayır", "Not": "12"},
                 {"Kat": "", "Ad": "  "}]),
    ("sayıdan-metne", [{"No": "1"}, {"No": "1a"}]),
    ("tam-ve-ondalık", [{"V": "1"}, {"V": "2.0"}]),
    ("bir-ve-sıfır", [{"B": "1"}, {"B": "0"}]),
    ("doğal-sıra", [{"a10": "x", "a2": "y", "B": "z", "ç": "w"}]),
    ("çok-basamak", [{"V": "0.1234567890123456"}]),
    ("tarih-ve-saat", [{"T": "16.10.2026 14:30", "S": "8:00"}, {"T": "2026-10-17T09:15:00", "S": "17:45:30"}]),
    ("tarih-ile-tarih-saat", [{"T": "2026-10-16"}, {"T": "2026-10-16 10:00"}]),
]

# docs/adr/0438 §4, §5: ranges with left-out ends and units, unique, values' descriptions and days.
EXCL = {"name": "Kat", "kind": "integer", "min": "0", "max": "60", "minExclusive": True}
EXCL_BOTH = {"name": "Oran", "kind": "decimal", "min": "0", "max": "1", "minExclusive": True, "maxExclusive": True}
AREA_U = {"name": "Taban alanı", "kind": "decimal", "scale": 2, "min": "0", "minExclusive": True, "unit": "m²"}
UPTO = {"name": "Yaş", "kind": "integer", "max": "18", "maxExclusive": True, "unit": "yıl"}
UPTO_DAY = {"name": "Bitiş", "kind": "date", "max": "2027-01-01", "maxExclusive": True}
CHECKS += [(EXCL, "0"), (EXCL, "1"), (EXCL, "60"), (EXCL_BOTH, "0"), (EXCL_BOTH, "0.5"), (EXCL_BOTH, "1"),
           (AREA_U, "0"), (AREA_U, "0.01"), (UPTO, "18"), (UPTO, "17"), (UPTO_DAY, "2027-01-01"), (UPTO_DAY, "31.12.2026")]
PROBLEMS += [
    ("hariç-uçlar", [EXCL, EXCL_BOTH, AREA_U, UPTO, UPTO_DAY]),
    ("hariç-alt-uç-en-azsız", [{"name": "Kat", "kind": "integer", "max": "5", "minExclusive": True}]),
    ("hariç-üst-uç-en-çoksuz", [{"name": "Kat", "kind": "integer", "min": "5", "maxExclusive": True}]),
    ("boş-aralık", [{"name": "Kat", "kind": "integer", "min": "5", "max": "5", "maxExclusive": True}]),
    ("eşit-uçlar-dahil", [{"name": "Kat", "kind": "integer", "min": "5", "max": "5"}]),
    ("metinde-birim", [{"name": "Ad", "kind": "text", "unit": "m"}]),
    ("boş-birim", [{"name": "Alan", "kind": "decimal", "unit": " "}]),
    ("uzun-birim", [{"name": "Alan", "kind": "decimal", "unit": "metrekare-yüzölçümü"}]),
    ("tekil", [{"name": "Ada/Parsel", "kind": "text", "unique": True}, {"name": "No", "kind": "integer", "unique": True}]),
    ("evet-hayır-tekil-olmaz", [{"name": "Ruhsat", "kind": "boolean", "unique": True}]),
    ("çok-değerli-tekil-olmaz", [{**MANY, "unique": True}]),
    ("değer-açıklaması-ve-günleri", [{"name": "Durum", "kind": "text", "values": [
        {"code": "iyi", "label": "İyi", "description": "Kusur yok."},
        {"code": "yeni", "label": "Yeni sınıf", "from": "2026-01-01"},
        {"code": "eski", "label": "Eski sınıf", "from": "2020-01-01", "until": "2025-12-31"}]}]),
    ("boş-açıklama", [{"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi", "description": "  "}]}]),
    ("geçersiz-gün", [{"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi", "until": "2026-02-30"}]}]),
    ("ters-günler", [{"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi", "from": "2026-05-01",
                                                                    "until": "2026-01-01"}]}]),
]

DISPLAY = [(BOOL, "true"), (BOOL, "false"), (BOOL, "evet"), (DATE, "2026-10-07"), (DATE, "07.10.2026"), (LIST_T, "T"), (LIST_T, "X"), (LIST_N, "2"), (DEC, "12.50"),
           (MANY, "K; T"), (MANY, "K; X"), (MANY, ""),
           (DT, "2026-10-16T14:30:00"), (DT, "2026-10-16T14:30:05"), (DT, "16.10.2026 14:30"), (TM, "08:00:00"), (TM, "17:45:30"), (TM, "8:00")]


# docs/adr/0629 §2: a big integer, an instant, a UUID, JSON and binary.
BIG = {"name": "Kimlik", "kind": "bigint"}
BIG_R = {"name": "Sıra", "kind": "bigint", "min": "-10", "max": "9007199254740993"}
INS = {"name": "Kayıt anı", "kind": "instant"}
INS_R = {"name": "Kayıt anı", "kind": "instant", "min": "2026-01-01T00:00:00Z", "max": "2026-12-31T23:59:59+03:00"}
UID = {"name": "Kimlik no", "kind": "uuid"}
JS = {"name": "Ek bilgi", "kind": "json"}
BIN = {"name": "İmza", "kind": "binary"}
UUID_A = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001"
CHECKS += [
    (BIG, "9007199254740993"), (BIG, "-9223372036854775808"), (BIG, "9223372036854775807"), (BIG, "9223372036854775808"),
    (BIG, "-9223372036854775809"), (BIG, "+00042"), (BIG, "-0"), (BIG, "1.0"), (BIG, "12345678901234567890123"), (BIG, " 7 "),
    (BIG, "١٢"), (BIG_R, "-11"), (BIG_R, "9007199254740993"), (BIG_R, "9007199254740994"),
    (INS, "2024-03-01T10:20:30+03:00"), (INS, "2024-03-01 10:20:30.500Z"), (INS, "01.03.2024 10:20+0300"),
    (INS, "2024-03-01T10:20:30.123456789-05"), (INS, "2024-03-01T10:20:30 +03:00"), (INS, "2024-03-01t10:20:30z"),
    (INS, "2024-03-01T10:20:30.0000Z"), (INS, "2024-03-01T10:20:30+00:00"), (INS, "2024-03-01T10:20:30-00:00"),
    (INS, "2024-03-01T10:20:30+0330"), (INS, "2024-03-01T10:20:30+18:00"), (INS, "2024-03-01T10:20:30-18:00"),
    (INS, "2024-03-01T10:20:30"), (INS, "2024-03-01T10:20:30+19:00"), (INS, "2024-03-01T10:20:30+03:60"),
    (INS, "2024-03-01T10:20.5Z"), (INS, "2024-03-01T10:20:30.1234567890Z"), (INS, "2024-03-01Z"), (INS, "2024-02-30T10:00Z"),
    (INS, "2024-03-01T24:00Z"), (INS, "2024-03-01T10:20:30+3:00"),
    (INS_R, "2025-12-31T23:59:59Z"), (INS_R, "2026-01-01T02:59:59+03:00"), (INS_R, "2026-01-01T03:00:00+03:00"),
    (INS_R, "2026-12-31T20:59:59Z"), (INS_R, "2026-12-31T21:00:00Z"),
    (UID, "0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001"), (UID, "{" + UUID_A + "}"), (UID, "urn:uuid:0192f5a07c3e7d4a9b1e4c2f8a6d0001"),
    (UID, "URN:UUID:0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001"), (UID, "0192f5a07c3e7d4a9b1e4c2f8a6d0001"),
    (UID, "0192f5a0-7c3e7d4a-9b1e-4c2f8a6d0001"), (UID, "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d000g"), (UID, "{" + UUID_A),
    (UID, "ıöüçşğıöüçşğıöüçşğıöüçşğıöüçşğıö"),
    (JS, '{"a": 1, "b": [true, null, "x y"]}'), (JS, " [ 1 , 2.50 , -0.0e+3 ] "), (JS, '"metin"'), (JS, "12345678901234567890"),
    (JS, '{"a":'), (JS, "NaN"), (JS, "[1,]"), (JS, '{"a" 1}'), (JS, '"\\u00e7 \\" \\\\"'), (JS, '"sekme\there"'),
    (JS, "[" * 64 + "]" * 64), (JS, "[" * 65 + "]" * 65), (JS, "true"), (JS, "01"), (JS, "{}"), (JS, "[ ]"),
    (BIN, "QUJD"), (BIN, "QUI"), (BIN, "QQ"), (BIN, "QUJD\nREVG"), (BIN, "QR=="), (BIN, "Q"), (BIN, "QUJD!"), (BIN, "QU=D"),
    (BIN, "AAAA"), (BIN, "="), ({**BIN, "required": True}, " "),
]
PROBLEMS += [
    ("büyük-tam-sayı-aralığı-ve-listesi", [BIG_R, {"name": "Tür", "kind": "bigint", "unit": "adet",
                                                   "values": [{"code": "9007199254740993", "label": "Büyük"}]}]),
    ("büyük-tam-sayı-tek-biçim", [{"name": "Sıra", "kind": "bigint", "min": "+5"}]),
    ("büyük-tam-sayı-sınırı", [{"name": "Sıra", "kind": "bigint", "max": "9223372036854775808"}]),
    ("büyük-tam-sayı-toplanmaz", [{"name": "Sıra", "kind": "bigint", "merge": "sum"}]),
    ("an-aralığı", [INS_R]),
    ("an-ters-aralık", [{"name": "An", "kind": "instant", "min": "2026-01-01T03:00:00+03:00", "max": "2025-12-31T23:59:59Z"}]),
    ("an-eşit-uçlar-hariç", [{"name": "An", "kind": "instant", "min": "2026-01-01T03:00:00+03:00", "max": "2026-01-01T00:00:00Z",
                              "maxExclusive": True}]),
    ("an-aralığı-tek-biçim", [{"name": "An", "kind": "instant", "min": "2026-01-01T00:00:00+00:00"}]),
    ("an-tekil", [{"name": "An", "kind": "instant", "unique": True}]),
    ("uuid-aralık-olmaz", [{"name": "Kimlik", "kind": "uuid", "min": UUID_A}]),
    ("uuid-birim-olmaz", [{"name": "Kimlik", "kind": "uuid", "unit": "x"}]),
    ("json-liste-olmaz", [{"name": "Ek", "kind": "json", "values": [{"code": "1", "label": "Bir"}]}]),
    ("ikili-uzunluk-olmaz", [{"name": "İmza", "kind": "binary", "length": 10}]),
    ("ikili-çok-değerli-olmaz", [{"name": "İmza", "kind": "binary", "multiple": True}]),
    ("yeni-türlerin-varsayılanı", [{"name": "Kimlik", "kind": "uuid", "default": UUID_A}, {"name": "Ek", "kind": "json", "default": '{"a":1}'},
                                   {"name": "İmza", "kind": "binary", "default": "QQ=="},
                                   {"name": "An", "kind": "instant", "default": "2026-01-01T00:00:00Z"}]),
    ("json-varsayılanı-tek-biçim", [{"name": "Ek", "kind": "json", "default": '{ "a": 1 }'}]),
    ("uuid-varsayılanı-tek-biçim", [{"name": "Kimlik", "kind": "uuid", "default": UUID_A.upper()}]),
]
INFER += [
    ("büyük-tam-sayı", [{"K": "9007199254740993"}, {"K": "12"}]),
    ("64-biti-aşan", [{"K": "9223372036854775808"}]),
    ("an", [{"A": "2024-03-01T10:20:30+03:00"}, {"A": "2024-03-01 07:20:30Z"}]),
    ("an-ile-tarih-saat", [{"A": "2024-03-01T10:20:30+03:00"}, {"A": "2024-03-01 07:20:30"}]),
    ("uuid", [{"U": UUID_A}, {"U": UUID_A.upper()}]),
    ("çıplak-onaltılık", [{"U": UUID_A.replace("-", "")}]),
]
DISPLAY += [(INS, "2024-03-01T10:20:30.5+03:00"), (INS, "2024-03-01T10:20:30Z"), (INS, "2024-03-01 10:20Z"), (BIN, "QUJD"),
            (BIN, "!!"), (BIG, "9007199254740993"), (UID, UUID_A), (JS, '{"a":1}')]

def build():
    return {
        "format": "kentos.layer-field-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/layer_field_cases.py",
        "title": "Katman alanları: değerin tek biçimi ya da reddi, alan listesinin ilk sorunu, verilerden alanlar, gösterim (ADR 0199)",
        "checks": [{"field": f, "text": t, "want": check(f, t)} for f, t in CHECKS],
        "problems": [{"name": n, "fields": fs, "want": fields_problem(fs)} for n, fs in PROBLEMS],
        "infer": [{"name": n, "rows": rows, "want": infer(rows)} for n, rows in INFER],
        "display": [{"field": f, "value": v, "want": display(f, v)} for f, v in DISPLAY],
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=2) + "\n"


def main():
    want = text_of(build())
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != want:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("layer field cases match")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(want, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
