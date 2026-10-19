#!/usr/bin/env python3
"""docs/adr/0600's functions against QGIS 4.2's own expression engine, without KentOS code.

Writes fixtures/expression/v2/qgis.json: one object (its attributes, all text, as the drawing keeps them) and cases, each
the language's source and the value it gives. Where the function means what QGIS's means, the value is QGIS's: the
case's `qgis` expression is evaluated by QGIS 4.2 (/usr/share/qgis/python) on the same object, in the C locale (the
language's decimal dot) and in UTC (the language's dates are without a zone, docs/adr/0214 §2.1). Where KentOS means
more or otherwise (Turkish words and letters, a QGIS crash, the random draws), the value is written here from the
ADR's rule, and the case says why (`rule`); its QGIS value, when QGIS gives one, is kept beside it (`qgisSays`).

Values: null, ["n", x], ["t", s], ["b", b], ["j", json] (an array or a map; maps compare without order).

  QT_QPA_PLATFORM=offscreen /usr/bin/python3 scripts/fixtures/expression_qgis_cases.py           # writes the file
  QT_QPA_PLATFORM=offscreen /usr/bin/python3 scripts/fixtures/expression_qgis_cases.py --check   # compares

crates/shared/expression/tests/qgis.rs evaluates each source one object at a time and as a table.
"""

import json
import math
import os
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/expression/v2/qgis.json"

OBJECT = {"Ada": "1245", "Parsel": "12", "Ad": "Kızılay Meydanı", "Nitelik": "Arsa", "Bos": ""}

# (source, qgis expression or None, rule) — rule None: the value is QGIS's.
CASES = [
    # ── Dönüşümler ──
    ("to_bool('yes')", "to_bool('yes')", None),
    ("to_bool(0)", "to_bool(0)", None),
    ("to_bool(2.5)", "to_bool(2.5)", None),
    ("to_bool('')", "to_bool('')", None),
    ("to_bool('abc')", "to_bool('abc')", None),
    ("mantıksal('evet')", None, ("b", True, "Türkçe evet doğrudur")),
    ("mantıksal('Hayır')", None, ("b", False, "Türkçe hayır yanlıştır")),
    ("to_bool('false')", "to_bool('false')", ("b", False, "QGIS boş olmayan her metni doğru sayar; KentOS false, no, off, 0 sözcüklerini yanlış okur")),
    ("to_bool('0')", "to_bool('0')", ("b", False, "QGIS'te '0' doğrudur; KentOS'ta yanlış")),
    ("to_decimal('6°21''16.445\"')", "to_decimal('6°21''16.445\"')", None),
    ("to_decimal('39°55′12.5″')", "to_decimal('39°55′12.5″')", None),
    ("to_decimal('-32°30''')", "to_decimal('-32°30''')", None),
    ("ondalık_derece('32 30 00 B')", None, ("n", -32.5, "B (batı) eksidir")),
    ("ondalık_derece('39 55 12,5 K')", None, ("n", 39 + 55 / 60 + 12.5 / 3600, "virgüllü ondalık, K (kuzey)")),
    ("ondalık_derece('abc')", None, (None, None, "sayısı olmayan metin boş")),
    ("to_dms(6.3545681, 'x', 3)", "to_dms(6.3545681, 'x', 3)", None),
    ("to_dms(-6.3545681, 'x', 3)", "to_dms(-6.3545681, 'x', 3)", None),
    ("to_dms(-39.92, 'y', 1, 'suffix')", "to_dms(-39.92, 'y', 1, 'suffix')", None),
    ("to_dms(32.85, 'x', 2, 'aligned')", "to_dms(32.85, 'x', 2, 'aligned')", None),
    ("to_dms(10.99999999, 'y', 2)", "to_dms(10.99999999, 'y', 2)", None),
    ("to_dms(190, 'x', 0, 'suffix')", "to_dms(190, 'x', 0, 'suffix')", None),
    ("to_dms(0.0000001, 'x', 2)", "to_dms(0.0000001, 'x', 2)", None),
    ("derece_dakika_saniye(-39.92, 'y', 1, 'sonek')", None, ("t", "39°55′12.0″G", "Türkçe yarıküre harfi G (güney)")),
    ("to_dm(6.1545681, 'y', 4)", "to_dm(6.1545681, 'y', 4)", None),
    ("to_dm(-3.5, 'x', 1, 'suffix')", "to_dm(-3.5, 'x', 1, 'suffix')", None),
    ("to_dm(1.999999, 'x', 2, 'aligned')", "to_dm(1.999999, 'x', 2, 'aligned')", None),
    ("to_base64('KentOS')", "to_base64('KentOS')", None),
    ("to_base64('Çorum')", "to_base64('Çorum')", None),
    ("base64_çöz('w4dvcnVt')", None, ("t", "Çorum", "QGIS bayt dizisi verir; KentOS UTF-8 metni")),
    ("base64_çöz('!!')", None, (None, None, "Base64 olmayan metin boş")),
    ("md5('a')", "md5('a')", None),
    ("md5('Çorum')", "md5('Çorum')", None),
    ("sha256('a')", "sha256('a')", None),
    ("hash('Çorum', 'sha1')", "hash('Çorum', 'sha1')", None),
    ("hash('a', 'sha224')", "hash('a', 'sha224')", None),
    ("hash('a', 'sha384')", "hash('a', 'sha384')", None),
    ("hash('a', 'sha512')", "hash('a', 'sha512')", None),
    ("hash('a', 'md5')", "hash('a', 'md5')", None),
    ("özet('a', 'yok')", None, (None, None, "bilinmeyen yöntem boş")),
    # ── Matematik ──
    ("clamp(1, 5, 3)", "clamp(1, 5, 3)", None),
    ("clamp(1, -1, 3)", "clamp(1, -1, 3)", None),
    ("clamp(1, 9, 3)", "clamp(1, 9, 3)", None),
    ("scale_linear(5, 0, 10, 0, 100)", "scale_linear(5, 0, 10, 0, 100)", None),
    ("scale_linear(15, 0, 10, 0, 100)", "scale_linear(15, 0, 10, 0, 100)", None),
    ("scale_linear(2.5, 0, 10, 100, 0)", "scale_linear(2.5, 0, 10, 100, 0)", None),
    ("scale_linear(5, 10, 0, 0, 100)", "scale_linear(5, 10, 0, 0, 100)", None),
    ("scale_exponential(5, 0, 10, 0, 100, 2)", "scale_exponential(5, 0, 10, 0, 100, 2)", None),
    ("scale_exponential(7, 0, 10, 0, 100, 0.5)", "scale_exponential(7, 0, 10, 0, 100, 0.5)", None),
    ("scale_polynomial(5, 0, 10, 0, 100, 2)", "scale_polynomial(5, 0, 10, 0, 100, 2)", None),
    ("scale_polynomial(-1, 0, 10, 0, 100, 2)", "scale_polynomial(-1, 0, 10, 0, 100, 2)", None),
    # ── Metin ──
    ("ltrim('  a ')", "ltrim('  a ')", None),
    ("rtrim('  a ')", "rtrim('  a ')", None),
    ("ltrim('xxaxx', 'x')", "ltrim('xxaxx', 'x')", None),
    ("rtrim('12.500', '0')", "rtrim('12.500', '0')", None),
    ("repeat('ab', 3)", "repeat('ab', 3)", None),
    ("repeat('ab', 0)", "repeat('ab', 0)", None),
    ("repeat('ab', -2)", "repeat('ab', -2)", None),
    ("reverse('Çağrı')", "reverse('Çağrı')", None),
    ("title('kadastro parseli')", "title('kadastro parseli')", None),
    ("title('a b')", "title('a b')", None),
    ("title('KIZILAY meydanı')", "title('KIZILAY meydanı')", ("t", "Kızılay Meydanı", "Türkçe küçük harf: I ı olur (QGIS i yazar)")),
    ("başlık('istanbul caddesi')", None, ("t", "İstanbul Caddesi", "Türkçe büyük harf: i İ olur")),
    ("substr_count('1/2/3', '/')", "substr_count('1/2/3', '/')", None),
    ("substr_count('aaa', 'aa')", "substr_count('aaa', 'aa')", None),
    ("substr_count('aaa', 'aa', true)", "substr_count('aaa', 'aa', true)", None),
    ("substr_count('aaa', '')", "substr_count('aaa', '')", None),
    ("concat_ws('-', 'a', NULL, 'b')", "concat_ws('-', 'a', NULL, 'b')", None),
    ("concat_ws('-', 'a', '', 'b')", "concat_ws('-', 'a', '', 'b')", None),
    ("concat_ws('/', Ada, Parsel)", "concat_ws('/', \"Ada\", \"Parsel\")", None),
    ("format('%1 ada %2 parsel', Ada, Parsel)", "format('%1 ada %2 parsel', \"Ada\", \"Parsel\")", None),
    ("format('%2 %1', 'a', 'b')", "format('%2 %1', 'a', 'b')", None),
    ("format('%2', 'a')", "format('%2', 'a')", None),
    ("format('%1%1', 'x')", "format('%1%1', 'x')", None),
    ("char(176)", "char(176)", None),
    ("char(305)", "char(305)", None),
    ("ascii('Ş')", "ascii('Ş')", None),
    ("ascii('')", "ascii('')", None),
    ("unaccent('Şişlİ çğü')", "unaccent('Şişlİ çğü')", None),
    ("unaccent('Ankara é à ñ')", "unaccent('Ankara é à ñ')", None),
    ("aksansız('Iğdır Kızılay')", None, ("t", "Igdir Kizilay", "ı da i olur (QGIS ı'yı bırakır)")),
    ("wordwrap('Atatürk Bulvarı No 12', 12)", "wordwrap('Atatürk Bulvarı No 12', 12)", None),
    ("wordwrap('Atatürk Bulvarı No 12', -12)", "wordwrap('Atatürk Bulvarı No 12', -12)", None),
    ("wordwrap('aaa bbb ccc ddd', 5)", "wordwrap('aaa bbb ccc ddd', 5)", None),
    ("wordwrap('UNIVERSITY OF QGIS', -3)", "wordwrap('UNIVERSITY OF QGIS', -3)", None),
    ("wordwrap('UNIVERSITY OF QGIS', 13)", "wordwrap('UNIVERSITY OF QGIS', 13)", None),
    ("wordwrap('a,b,c,d', 3, ',')", "wordwrap('a,b,c,d', 3, ',')", None),
    ("wordwrap('kısa', 20)", "wordwrap('kısa', 20)", None),
    ("levenshtein('kittens', 'mitten')", "levenshtein('kittens', 'mitten')", None),
    ("levenshtein('A', 'a')", "levenshtein('A', 'a')", None),
    ("levenshtein('Kızılay', 'Kizilay')", "levenshtein('Kızılay', 'Kizilay')", None),
    ("hamming_distance('abc', 'abd')", "hamming_distance('abc', 'abd')", None),
    ("hamming_distance('abc', 'ab')", "hamming_distance('abc', 'ab')", None),
    ("longest_common_substring('ABABC', 'BABCA')", "longest_common_substring('ABABC', 'BABCA')", None),
    ("longest_common_substring('Abc', 'aBC')", "longest_common_substring('Abc', 'aBC')", None),
    ("longest_common_substring('Kızılay', 'Kızılırmak')", "longest_common_substring('Kızılay', 'Kızılırmak')", None),
    # ── Koşullar ──
    ("nullif(5, 5)", "nullif(5, 5)", None),
    ("nullif(Nitelik, 'Arsa')", "nullif(\"Nitelik\", 'Arsa')", None),
    ("nullif(Nitelik, 'Yol')", "nullif(\"Nitelik\", 'Yol')", None),
    # ── Diziler ve eşlemeler ──
    ("array_all(array(1, 2, 3), array(2, 3))", "array_all(array(1, 2, 3), array(2, 3))", None),
    ("array_all(array(1, 2, 3), array(2, 4))", "array_all(array(1, 2, 3), array(2, 4))", None),
    ("array_count(array('a', 'b', 'a'), 'a')", "array_count(array('a', 'b', 'a'), 'a')", None),
    ("array_insert(array(1, 2), 1, 9)", "array_insert(array(1, 2), 1, 9)", None),
    ("array_insert(array(1, 2), 2, 9)", "array_insert(array(1, 2), 2, 9)", None),
    ("dizi_araya(dizi(1, 2), -1, 9)", None, ("j", [9, 1, 2], "eksi yer başa ekler (QGIS'te çöker)")),
    ("dizi_araya(dizi(1, 2), 5, 9)", None, ("j", [1, 2, 9], "uzunluğu aşan yer sona ekler (QGIS'te çöker)")),
    ("array_prepend(array(1, 2), 0)", "array_prepend(array(1, 2), 0)", None),
    ("array_remove_at(array(1, 2, 3), -1)", "array_remove_at(array(1, 2, 3), -1)", None),
    ("array_remove_at(array(1, 2, 3), 0)", "array_remove_at(array(1, 2, 3), 0)", None),
    ("array_remove_at(array(1, 2, 3), 7)", "array_remove_at(array(1, 2, 3), 7)", None),
    ("array_remove_all(array('a', 'b', 'a'), 'a')", "array_remove_all(array('a', 'b', 'a'), 'a')", None),
    ("array_replace(array('a', 'b', 'c'), 'b', 'x')", "array_replace(array('a', 'b', 'c'), 'b', 'x')", None),
    ("array_replace(array('a', 'b', 'c'), array('a', 'c'), array('x', 'y'))", "array_replace(array('a', 'b', 'c'), array('a', 'c'), array('x', 'y'))", None),
    ("array_replace(array('a', 'b', 'c'), array('a', 'c'), 'z')", "array_replace(array('a', 'b', 'c'), array('a', 'c'), 'z')", None),
    ("array_replace(array('a', 'b'), map('a', 1, 'b', 2))", "array_replace(array('a', 'b'), map('a', 1, 'b', 2))", None),
    ("array_intersect(array(1, 2), array(2, 3))", "array_intersect(array(1, 2), array(2, 3))", None),
    ("array_intersect(array(1, 2), array(4, 3))", "array_intersect(array(1, 2), array(4, 3))", None),
    ("array_majority(array(1, 1, 2))", "array_majority(array(1, 1, 2))", None),
    ("array_majority(array(1, 1, 2, 2, 3))", "array_majority(array(1, 1, 2, 2, 3))", ("j", [1, 2], "öğeler ilk geçtikleri sırayla (QGIS'te karma tablonun sırası)")),
    ("array_majority(array(1, 1, 2, 2, 3), 'median')", "array_majority(array(1, 1, 2, 2, 3), 'median')", None),
    ("array_majority(array(1, 2, 2), 'real_majority')", "array_majority(array(1, 2, 2), 'real_majority')", None),
    ("array_majority(array(1, 2, 3), 'real_majority')", "array_majority(array(1, 2, 3), 'real_majority')", None),
    ("array_minority(array(1, 1, 2))", "array_minority(array(1, 1, 2))", None),
    ("array_minority(array(1, 1, 2, 2, 3), 'real_minority')", "array_minority(array(1, 1, 2, 2, 3), 'real_minority')", ("j", [1, 2, 3], "öğeler ilk geçtikleri sırayla (QGIS'te karma tablonun sırası)")),
    ("array_median(array(1, 3, 2, 4))", "array_median(array(1, 3, 2, 4))", None),
    ("array_median(array(5, 'a', 1))", "array_median(array(5, 'a', 1))", None),
    ("generate_series(1, 5, 2)", "generate_series(1, 5, 2)", None),
    ("generate_series(5, 1, -2)", "generate_series(5, 1, -2)", None),
    ("generate_series(1, 3)", "generate_series(1, 3)", None),
    ("generate_series(5, 1)", "generate_series(5, 1)", None),
    ("map_concat(map('a', 1), map('b', 2))", "map_concat(map('a', 1), map('b', 2))", None),
    ("map_concat(map('a', 1, 'b', 2), map('b', 3))", "map_concat(map('a', 1, 'b', 2), map('b', 3))", None),
    ("map_prefix_keys(map('a', 1, 'b', 'x'), 'k_')", "map_prefix_keys(map('a', 1, 'b', 'x'), 'k_')", None),
    ("url_encode(map('a', 'x y', 'b', 1))", "url_encode(map('a', 'x y', 'b', 1))", None),
    ("url_encode(map('il', 'Çorum'))", "url_encode(map('il', 'Çorum'))", None),
    # ── Tarih ve saat ──
    ("epoch(to_datetime('2020-01-01T00:00:00'))", "epoch(to_datetime('2020-01-01T00:00:00'))", None),
    ("epoch(to_date('2026-10-19'))", "epoch(to_datetime('2026-10-19T00:00:00'))", None),
    ("datetime_from_epoch(1577836800000)", "datetime_from_epoch(1577836800000)", None),
    ("datetime_from_epoch(86400500)", "datetime_from_epoch(86400500)", None),
    ("make_time(12, 30, 15.5)", "make_time(12, 30, 15.5)", None),
    ("make_time(9, 5, 0)", "make_time(9, 5, 0)", None),
    ("to_time('12:30:15')", "to_time('12:30:15')", None),
    ("saat_yap('9:05')", None, ("t", "09:05:00", "saniyesiz saat metni")),
    ("saat_yap(25, 0, 0)", None, (None, None, "24 ve üstü saat boş")),
]


# ── Random draws: docs/adr/0600 §3's SplitMix64, written anew ──
M = (1 << 64) - 1


def mix(z):
    z = (z + 0x9E3779B97F4A7C15) & M
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M
    return z ^ (z >> 31)


def bits(x):
    return struct.unpack("<Q", struct.pack("<d", float(x) + 0.0))[0]


def unit(r):
    return (r >> 11) / float(1 << 53)


def rand(lo, hi, seed):
    lo, hi = math.trunc(lo), math.trunc(hi)
    span = hi - lo + 1
    return min(lo + math.floor(unit(mix(bits(seed))) * span), hi)


def randf(lo, hi, seed):
    return lo + unit(mix(bits(seed))) * (hi - lo)


RANDOM = [
    ("rastgele(1, 100, 7)", ("n", rand(1, 100, 7))),
    ("rand(1, 6, 42)", ("n", rand(1, 6, 42))),
    ("rand(-5, 5, 2.5)", ("n", rand(-5, 5, 2.5))),
    ("rand(10, 1, 3)", (None, None)),
    ("randf(0, 1, 42)", ("n", randf(0, 1, 42))),
    ("rastgele_ondalık(10, 20, 7)", ("n", randf(10, 20, 7))),
    ("rand(1, 100, Ada)", ("n", rand(1, 100, 1245))),
]


def encode(v):
    """A QGIS value as the fixture writes it."""
    from qgis.PyQt.QtCore import QByteArray, QDate, QDateTime, QTime

    if v is None:
        return None
    if isinstance(v, bool):
        return ["b", v]
    if isinstance(v, (int, float)):
        return ["n", float(v)]
    if isinstance(v, str):
        return ["t", v]
    if isinstance(v, (list, dict)):
        return ["j", v]
    if isinstance(v, QByteArray):
        return ["t", bytes(v).decode("utf-8")]
    if isinstance(v, QDateTime):
        d, t = v.date(), v.time()
        if t.hour() == 0 and t.minute() == 0 and t.second() == 0 and t.msec() == 0:
            return ["t", f"{d.year():04}-{d.month():02}-{d.day():02}"]
        text = f"{d.year():04}-{d.month():02}-{d.day():02}T{t.hour():02}:{t.minute():02}:{t.second():02}"
        return ["t", text + (f".{t.msec():03}" if t.msec() else "")]
    if isinstance(v, QDate):
        return ["t", f"{v.year():04}-{v.month():02}-{v.day():02}"]
    if isinstance(v, QTime):
        text = f"{v.hour():02}:{v.minute():02}:{v.second():02}"
        return ["t", text + (f".{v.msec():03}" if v.msec() else "")]
    raise TypeError(f"QGIS gave {type(v)}: {v!r}")


def qgis_values():
    os.environ["TZ"] = "UTC"
    import time

    time.tzset()
    sys.path.insert(0, "/usr/share/qgis/python")
    from qgis.core import QgsApplication, QgsExpression, QgsExpressionContext, QgsFeature, QgsField, QgsFields
    from qgis.PyQt.QtCore import QLocale, QMetaType

    QLocale.setDefault(QLocale(QLocale.Language.C))
    app = QgsApplication([], False)
    app.initQgis()
    fields = QgsFields()
    for name in OBJECT:
        fields.append(QgsField(name, QMetaType.Type.QString))
    feature = QgsFeature(fields)
    for name, value in OBJECT.items():
        feature[name] = value
    out = {}
    for source, q, _ in CASES:
        if q is None:
            continue
        context = QgsExpressionContext()
        context.setFeature(feature)
        e = QgsExpression(q)
        if e.hasParserError():
            raise SystemExit(f"QGIS cannot read {q}: {e.parserErrorString()}")
        value = e.evaluate(context)
        out[source] = None if e.hasEvalError() else encode(value)
    return out


def build():
    said = qgis_values()
    cases = []
    for source, q, rule in CASES:
        case = {"source": source}
        if q is not None:
            case["qgis"] = q
        if rule is None:
            case["value"] = said[source]
        else:
            kind, value, why = rule
            case["value"] = None if kind is None else [kind, float(value) if kind == "n" else value]
            case["rule"] = why
            if q is not None:
                case["qgisSays"] = said[source]
        cases.append(case)
    for source, (kind, value) in RANDOM:
        cases.append({
            "source": source,
            "value": None if kind is None else [kind, float(value)],
            "rule": "SplitMix64 tohumla (docs/adr/0600 §3); QGIS'in üretecinden ayrı",
        })
    return {"format": "kentos.expression-qgis", "version": 1, "object": OBJECT, "cases": cases}


def main():
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check'siz çalıştırın.")
            return 1
    else:
        OUT.write_text(text)
    qgis = sum(1 for c in data["cases"] if "rule" not in c)
    print(f"İfade işlevleri QGIS'le tutarlı: {len(data['cases'])} durum, {qgis}'i QGIS 4.2'nin değeriyle")
    return 0


if __name__ == "__main__":
    sys.exit(main())
