"""The language's additions of docs/adr/0214, evaluated here independently
of the engine: dates and times, `@` variables, regular expressions, arrays
and maps, aggregates over a layer, spatial relations to another layer and a
value from another layer.

    python3 scripts/fixtures/expression_extras.py           # writes the file
    python3 scripts/fixtures/expression_extras.py --check   # writes nothing; compares

Writes fixtures/expression/v2/extras.json: the project's variables, a small
world (parcels, protected areas, stops, quarters and a stream, as the
drawing's objects with their attributes) and, for each source, its value on
each parcel, or that it does not compile, and the `@` names it did not know.
The rules are the ADR's, written anew here on top of the language's own
reference (expression_language.py: numbers read from text, equality, order,
the Turkish alphabet):
- a date is ISO text; it is read by ADR 0210 §3's grammar (YYYY-AA-GG or
  G.A.YYYY, then after T or one space SS:DD[:ss[.f…]] and a zone) and
  worked in whole milliseconds on the proleptic Gregorian calendar
  (Python's datetime for the weeks and the days of the year);
- an array or a map shows as JSON with JavaScript's number text; it equals
  only the same array or map; an empty one is false;
- regular expressions are Python's `re` with ASCII classes (as regex-lite's),
  `$n`, `${name}` and `$$` written by the regex crate's rule;
- the aggregates' numbers follow kentos.statistics/1 (ADR 0200 §4): exact
  decimal sums (Python's Decimal), the mean the exact sum over the count in
  double precision, the median of an even count the exact mean of the two,
  the sample deviation two passes in double precision;
- the relations are ADR 0200's on shapely's geometry: Kesişen intersects
  (touching counts), Uzaklıkta within the distance, İçeren covers, İçinde
  kalan is covered, Merkezi içinde the centroid covered; the overlaps are
  shapely's intersections. The world's coordinates are whole metres, so
  no relation stands within the core's 1 mm tolerance.

Rust (crates/shared/expression/tests/language.rs) and the web
(apps/web/src/model/expression/extras.test.ts) compare the engine with this
file to the bit, but for the overlaps' and distances' numbers ("near": true,
within 1e-9 relative).
"""
import datetime
import json
import math
import re
import sys
from decimal import Decimal, localcontext
from pathlib import Path

import shapely
from shapely.geometry import LineString, Point, Polygon

sys.path.insert(0, str(Path(__file__).resolve().parent))
import expression_language as base  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/expression/v2/extras.json"

Invalid = base.Invalid
Thrown = base.Thrown

# ── The world ─────────────────────────────────────────────────────────────


def rect(x0, y0, x1, y1):
    return [{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}]


def polygon(i, layer, pts, attrs):
    return {"id": i, "layerId": layer, "kind": "polygon", "pts": pts, "attrs": attrs}


def point(i, layer, x, y, attrs):
    return {"id": i, "layerId": layer, "kind": "point", "p": {"x": x, "y": y}, "attrs": attrs}


def polyline(i, layer, pts, attrs):
    return {"id": i, "layerId": layer, "kind": "polyline", "pts": [{"x": x, "y": y} for x, y in pts], "attrs": attrs}


# Six parcels in a row of 20 m × 30 m (the fourth 20 m × 40 m), two quarters,
# two protected areas, three stops and a stream.
LAYERS = [
    {"id": "parsel", "name": "Parsel", "objects": [
        polygon(1, "parsel", rect(0, 0, 20, 30), {"Ada": "101", "Parsel": "1", "Mahalle": "K1", "Nitelik": "Arsa",
                                                  "Tarih": "2024-01-31", "Etiketler": "imar,ifraz", "Kat": "3",
                                                  "Bilgi": "{\"kat\": 3, \"ad\": \"A\"}"}),
        polygon(2, "parsel", rect(20, 0, 40, 30), {"Ada": "101", "Parsel": "2", "Mahalle": "K1", "Nitelik": "Bahçe",
                                                   "Tarih": "05.03.2021 14:30", "Etiketler": "imar", "Kat": "",
                                                   "Bilgi": "[1, [2, 3]]"}),
        polygon(3, "parsel", rect(40, 0, 60, 30), {"Ada": "102", "Parsel": "3", "Mahalle": "K2", "Nitelik": "Arsa",
                                                   "Tarih": "2020-02-29T23:59:59.5", "Etiketler": "", "Kat": "5",
                                                   "Bilgi": "{"}),
        polygon(4, "parsel", rect(60, 0, 80, 40), {"Ada": "102", "Parsel": "4", "Mahalle": "K2", "Nitelik": "Tarla",
                                                   "Tarih": "x", "Etiketler": "a,,b", "Kat": "2,5"}),
        polygon(5, "parsel", rect(80, 0, 100, 30), {"Ada": "103", "Parsel": "5", "Mahalle": "K9", "Nitelik": "Arsa",
                                                    "Tarih": "2026-10-10T09:05:07Z", "Etiketler": "x", "Kat": "4"}),
        polygon(6, "parsel", rect(130, 0, 150, 30), {"Ada": "103", "Parsel": "6", "Nitelik": "Yol",
                                                     "Tarih": "2024-12-31T00:00", "Kat": "1"}),
    ]},
    {"id": "sit", "name": "Sit alanı", "objects": [
        polygon(11, "sit", rect(10, 10, 50, 50), {"Ad": "Kale", "Derece": "1"}),
        polygon(12, "sit", rect(55, 20, 70, 60), {"Ad": "Höyük", "Derece": "2"}),
    ]},
    {"id": "durak", "name": "Durak", "objects": [
        point(21, "durak", 10, 40, {"Ad": "Çarşı"}),
        point(22, "durak", 90, 35, {"Ad": "Okul"}),
        point(23, "durak", 300, 300, {"Ad": "Uzak"}),
        point(24, "durak", 70, 20, {"Ad": "Pazar"}),
    ]},
    {"id": "mahalle", "name": "Mahalle", "objects": [
        polygon(31, "mahalle", rect(-10, -10, 45, 60), {"Kod": "K1", "Ad": "Kızılay"}),
        polygon(32, "mahalle", rect(45, -10, 120, 60), {"Kod": "K2", "Ad": "Bahçelievler"}),
    ]},
    {"id": "dere", "name": "Dere", "objects": [
        polyline(41, "dere", [(-10, 15), (160, 15)], {"Ad": "Hatip"}),
    ]},
]

# The project's variables, then the built-in ones the caller gives (docs/adr/0214 §2.3).
VARIABLES = [
    {"name": "is_no", "value": "2026/41"},
    {"name": "Katsayı", "value": 1.5},
    {"name": "onaylı", "value": True},
    {"name": "proje_adi", "value": "Kızılay Etüdü"},
    {"name": "koordinat_sistemi", "value": "TUREF / TM33"},
    {"name": "epsg", "value": 5254.0},
    {"name": "olcek", "value": 1000.0},
    {"name": "tarih", "value": "2026-10-10"},
    {"name": "simdi", "value": "2026-10-10T14:30:00"},
    {"name": "kullanici", "value": "cihad"},
]

EVALUATED = "parsel"

SOURCES = [
    # ── Variables (§2.3) ──
    "@proje_adi",
    "@Proje_Adı || ' / ' || @is_no",
    "@KATSAYI * 2",
    "@onaylı ve Nitelik = 'Arsa'",
    "@olcek / 100",
    "@yok",
    "varsayılan(@yok, 'tanımsız') || @hiç",
    "@katman_adi",
    "şimdi()",
    "bugün()",
    "tarih_farkı(Tarih, bugün(), 'gün')",
    # ── Dates (§2.2) ──
    "tarih(2024, 2, 29)",
    "tarih(2023, 2, 29)",
    "tarih(2024, 13, 1)",
    "tarih(Tarih)",
    "tarih_saat(Tarih)",
    "tarih_saat(2021, 3, 5, 14, 30, 5.5)",
    "tarih_saat(2021, 3, 5)",
    "tarih_saat(2021, 3, 5, 24)",
    "yıl(Tarih)",
    "ay(Tarih) * 100 + gün(Tarih)",
    "saat(Tarih) || ':' || dakika(Tarih) || ':' || saniye(Tarih)",
    "hafta(Tarih)",
    "hafta('2021-01-03') || '/' || hafta('2021-01-04') || '/' || hafta('2020-12-31') || '/' || hafta('2026-12-31')",
    "haftanın_günü(Tarih)",
    "yılın_günü(Tarih)",
    "tarih_ekle(Tarih, 1, 'ay')",
    "tarih_ekle(Tarih, -1, 'yıl')",
    "tarih_ekle(Tarih, 36, 'saat')",
    "tarih_ekle(Tarih, 2, 'hafta')",
    "tarih_ekle(Tarih, 0.5, 'gün')",
    "tarih_ekle(Tarih, 1, 'months')",
    "tarih_ekle(Tarih, 1, 'fortnight')",
    "tarih_ekle('9999-12-31', 1, 'gün')",
    "tarih_farkı('2024-01-31', Tarih, 'ay')",
    "tarih_farkı('2020-02-29', Tarih, 'yıl')",
    "tarih_farkı(Tarih, '2024-01-31', 'yıl')",
    "tarih_farkı(Tarih, '2026-10-10T12:00', 'saat')",
    "tarih_farkı(Tarih, '2026-10-10', 'hafta')",
    "tarih_biçimle(Tarih, 'GG.AA.YYYY')",
    "tarih_biçimle(Tarih, 'GGGG, G AAAA YYYY ''saat'' SS:DD:ss')",
    "tarih_biçimle(Tarih, 'GGG G AAA YY, S.D.s ''''Y''')",
    "tarih_biçimle(Tarih, boş)",
    "Tarih < '2024-06-01'",
    # ── Regular expressions (§2.4) ──
    "eşleşir(Ada, '^10[12]$')",
    "eşleşir(Nitelik, '(?i)^a')",
    "eşleşir([Yok], '.')",
    "düzenli_bul(Etiketler, ',')",
    "düzenli_bul('Çağrı 12', '\\d')",
    "düzenli_parça(Etiketler, '([a-z]+)$')",
    "düzenli_parça(Tarih, '\\d{4}')",
    "düzenli_parça(Nitelik, 'q')",
    "düzenli_gruplar(Tarih, '(\\d+)-(\\d+)(-x)?')",
    "düzenli_gruplar(Tarih, '\\d')",
    "düzenli_değiştir(Tarih, '(\\d+)-(\\d+)-(\\d+)', '$3/$2/$1')",
    "düzenli_değiştir(Etiketler, ',', ' + ')",
    "düzenli_değiştir(Nitelik, '(?P<ilk>.)', '${ilk}$$')",
    "düzenli_değiştir(Ada, '1', Parsel)",
    "eşleşir('a', Etiketler)",
    "eşleşir(Ada, '(')",
    # ── Arrays and maps (§2.5) ──
    "dizi(1, 'Arsa', doğru, boş, 0.1 + 0.2)",
    "dizi()",
    "dizi(1e21, 0.0000001, -0, 1 / 3)",
    "dizi(Ada, dizi(Parsel, Kat))",
    "dizi_uzunluğu(metin_dizi(Etiketler))",
    "dizi_uzunluğu('a')",
    "dizi_öğe(metin_dizi(Etiketler), 0)",
    "dizi_öğe(metin_dizi(Etiketler), -1)",
    "dizi_öğe(metin_dizi(Etiketler), 5)",
    "dizi_ilk(dizi(Ada, Parsel)) || dizi_son(dizi(Ada, Parsel))",
    "dizi_içerir(dizi(101, 103), Ada)",
    "dizi_bul(dizi('3', 5, 'x'), Kat)",
    "dizi_ekle(metin_dizi(Etiketler), Parsel)",
    "dizi_birleştir(dizi(1), metin_dizi(Etiketler))",
    "dizi_birleştir(dizi(1), 'x')",
    "dizi_benzersiz(dizi(1, '1', 2, 'a', 'a', boş, boş))",
    "dizi_sırala(dizi(3, 'Çam', '12', boş, 'Ceviz', 1, doğru, yanlış, 'dut'))",
    "dizi_sırala(dizi(3, 'Çam', '12', boş, 'Ceviz', 1), yanlış)",
    "dizi_ters(metin_dizi(Etiketler))",
    "dizi_dilim(dizi(0, 1, 2, 3, 4), 1, -2)",
    "dizi_dilim(dizi(0, 1, 2, 3, 4), -2, 9)",
    "dizi_dilim(dizi(0, 1, 2), 2, 1)",
    "dizi_metin(dizi(Ada, boş, Parsel, 2.5, doğru), '/', '?')",
    "dizi_metin(dizi(1, 2))",
    "metin_dizi(Etiketler, ',', '')",
    "metin_dizi(Ada, '')",
    "metin_dizi([Yok])",
    "dizi_topla(dizi(0.1, 0.2, 'x', '0,5', doğru))",
    "dizi_ortalama(dizi(1, 2, 4))",
    "dizi_en_düşük(dizi(3, '12', 1.5))",
    "dizi_en_yüksek(dizi('Çam', 'Ceviz', 'Dut'))",
    "dizi_topla(dizi('x'))",
    "eşleme('ad', Nitelik, 'kat', Kat, 'ad', Ada)",
    "eşleme()",
    "eşleme_değeri(json_oku(Bilgi), 'kat')",
    "eşleme_değeri(json_oku(Bilgi), 'yok')",
    "eşleme_içerir(eşleme('a', 1), 'a')",
    "eşleme_anahtarları(eşleme('b', 1, 'a', 2))",
    "eşleme_değerleri(eşleme('b', 1, 'a', dizi(2)))",
    "eşleme_ekle(eşleme('a', 1), 'b', Parsel)",
    "eşleme_ekle(eşleme('a', 1), 'a', 2)",
    "eşleme_sil(eşleme('a', 1, 'b', 2), 'a')",
    "json_oku(Bilgi)",
    "json_oku('\"A\\\\nB\"')",
    "json_oku(' 12.5 ')",
    "json_yaz(Nitelik)",
    "json_yaz(boş) || json_yaz(doğru) || json_yaz(1 / 4)",
    "json_yaz(json_oku(Bilgi))",
    "dizi(1, 2) = dizi(1, 2)",
    "dizi(1, 2) = '[1,2]'",
    "dizi() veya 0",
    "eğer(metin_dizi(Etiketler), 'dolu', 'boş')",
    "uzunluk(dizi(1, 2))",
    "dizi(1) || '!'",
    "metin(eşleme('a', 1))",
    "eşleme('a')",
    # ── Aggregates (§2.6) ──
    "topla($alan)",
    "$alan / topla($alan)",
    "topla($alan, Mahalle)",
    "topla($alan, Ada, Nitelik = 'Arsa')",
    "ortalama(Kat)",
    "ortalama(Kat, Ada)",
    "say(Kat)",
    "say($id, Ada)",
    "say(Kat, Ada, Kat > 4)",
    "say_benzersiz(Nitelik)",
    "say_benzersiz(Nitelik, Mahalle)",
    "en_düşük($alan)",
    "en_yüksek(Kat, Ada)",
    "en_düşük(Nitelik)",
    "orta_değer(Kat)",
    "orta_değer($id, Ada)",
    "std_sapma($alan)",
    "std_sapma(Kat, Ada)",
    "değerleri_birleştir(Parsel)",
    "değerleri_birleştir(Parsel, '-', Ada)",
    "değerleri_birleştir(Kat, ';', Mahalle, Kat > 2)",
    "değerleri_dizi(Parsel, Ada)",
    "değerleri_dizi(Kat)",
    "topla(Kat, boş, yanlış)",
    "katman_toplamı('Sit alanı', 'topla', Derece)",
    "katman_toplamı('SİT ALANI', 'count', Ad)",
    "katman_toplamı('Durak', 'değerleri_birleştir', Ad, Ad != 'Uzak')",
    "katman_toplamı('Durak', 'değerleri_dizi', Ad)",
    "katman_toplamı('Yok', 'topla', 1)",
    "katmandan('Mahalle', Ad, 'Kod', Mahalle)",
    "katmandan('Mahalle', $alan, 'Kod', Mahalle)",
    "katmandan('Sit alanı', Ad, 'Derece', $id)",
    # ── Spatial relations (§2.7) ──
    "kesişir('Sit alanı')",
    "kesişir('Sit alanı', 0, Derece = 2)",
    "kesişir('Durak')",
    "kesişir('Durak', 15)",
    "kesişir('Dere')",
    "kesişir('Parsel')",
    "kesişen_sayısı('Parsel')",
    "kesişen_sayısı('Durak', 25)",
    "kesişenler('Sit alanı', Ad)",
    "kesişenler('Parsel', Parsel)",
    "kesişenler('Durak', Ad, 1000, Ad != 'Okul')",
    "kapsar('Durak')",
    "içinde_kalır('Mahalle')",
    "içinde_kalır('Mahalle', Kod = 'K2')",
    "merkezi_içinde('Mahalle', Kod = 'K1')",
    "merkezi_içinde('Sit alanı')",
    "uzaklık('Durak')",
    "uzaklık('Sit alanı')",
    "uzaklık('Durak', Ad = 'Uzak')",
    "uzaklık('Yok')",
    "en_yakın('Durak', Ad)",
    "en_yakın('Durak', Ad, Ad != 'Çarşı')",
    "en_yakın('Parsel', Parsel)",
    "kesişim_alanı('Sit alanı')",
    "kesişim_alanı('Sit alanı') / $alan",
    "kesişim_alanı('Mahalle')",
    "kesişim_uzunluğu('Sit alanı')",
    "kesişim_uzunluğu('Mahalle', Kod = 'K2')",
    "kesişir('Sit alanı') ve topla($alan, Ada) > 1000",
    "eğer(kesişir('Durak', 15), en_yakın('Durak', Ad), '-')",
    # ── What does not compile ──
    "kesişir(Ad)",
    "katman_toplamı('Durak', 'yuvarla', Ad)",
    "katman_toplamı('Durak', Nitelik, Ad)",
    "katmandan('Mahalle', Ad, Kod, Mahalle)",
    "değerleri_birleştir(Parsel, Ada)",
    "topla(say($id))",
    "topla(kesişir('Durak'))",
    "tarih_ekle(Tarih, 1)",
    "eşleşir(Ada, '(')",
    "eşleme('a', 1, 'b')",
    "@",
    "şimdi(1)",
]

# ── Values: arrays and maps on top of the language's ─────────────────────


class Arr:
    def __init__(self, items):
        self.items = list(items)


class Map:
    def __init__(self, pairs):
        self.pairs = []
        for k, v in pairs:
            for i, (k2, _) in enumerate(self.pairs):
                if k2 == k:
                    self.pairs[i] = (k, v)
                    break
            else:
                self.pairs.append((k, v))


def compound(v):
    return isinstance(v, (Arr, Map))


def js_number(x):
    """JavaScript's Number::toString of a finite double."""
    if x == 0:
        return "0"
    if x < 0:
        return "-" + js_number(-x)
    d = Decimal(repr(x)).normalize()
    sign, digits, exp = d.as_tuple()
    s = "".join(map(str, digits))
    k = len(s)
    n = exp + k
    if k <= n <= 21:
        return s + "0" * (n - k)
    if 0 < n <= 21:
        return s[:n] + "." + s[n:]
    if -6 < n <= 0:
        return "0." + "0" * (-n) + s
    e = n - 1
    mant = s[0] + ("." + s[1:] if k > 1 else "")
    return f"{mant}e{'+' if e >= 0 else '-'}{abs(e)}"


def json_string(s):
    return json.dumps(s, ensure_ascii=False)


def json_of(v):
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        return js_number(v) if math.isfinite(v) else "null"
    if isinstance(v, Arr):
        return "[" + ",".join(json_of(i) for i in v.items) + "]"
    if isinstance(v, Map):
        return "{" + ",".join(json_string(k) + ":" + json_of(i) for k, i in v.pairs) + "}"
    return json_string(v)


def text(v):
    return json_of(v) if compound(v) else base.text(v)


def is_empty(v):
    return base.is_empty(v)


def truthy(v):
    if isinstance(v, Arr):
        return bool(v.items)
    if isinstance(v, Map):
        return bool(v.pairs)
    return base.truthy(v)


def to_number(v):
    return None if compound(v) else base.to_number(v)


def equals(a, b):
    if is_empty(a) or is_empty(b):
        return is_empty(a) and is_empty(b)
    if isinstance(a, bool) or isinstance(b, bool):
        return truthy(a) == truthy(b)
    if compound(a) or compound(b):
        return type(a) is type(b) and json_of(a) == json_of(b)
    return base.equals(a, b)


def compare(a, b):
    a = text(a) if compound(a) else a
    b = text(b) if compound(b) else b
    return base.compare(a, b)


def item(v):
    """A value as an array keeps it: a number that is not finite is null."""
    if isinstance(v, float) and not math.isfinite(v):
        return None
    return v


def key(v):
    """The key values are grouped and told apart by: numbers by value, text as it is; none for empty."""
    if is_empty(v):
        return None
    if isinstance(v, bool):
        return ("b", v)
    if compound(v):
        return ("c", type(v).__name__, json_of(v))
    x = to_number(v)
    if x is not None:
        return ("n", x + 0.0)
    return ("t", v)


# ── kentos.statistics/1's numbers (ADR 0200 §4) ──────────────────────────

def stat_number(t):
    """A text as the statistics rule reads it: sign, digits, one . or , and no exponent; at most 30 digits."""
    s = base.trim(t)
    if s[:1] in "+-" and s:
        sign, s = s[0], s[1:]
    else:
        sign = ""
    if not s or s.count(".") + s.count(",") > 1:
        return None
    body = s.replace(",", ".")
    if not all(c.isascii() and (c.isdigit() or c == ".") for c in body):
        return None
    digits = body.replace(".", "")
    if not digits or len(digits) > 30:
        return None
    if body.startswith(".") or body.endswith("."):
        return None
    return Decimal(sign + body)


def number_of(v):
    if isinstance(v, bool) or v is None or compound(v):
        return None
    if isinstance(v, float):
        return stat_number(repr_plain(v)) if math.isfinite(v) else None
    return stat_number(v)


def repr_plain(x):
    """The shortest digits of a double, written without an exponent (Rust's Display)."""
    d = Decimal(repr(x))
    return format(d, "f")


def exact_sum(nums):
    with localcontext() as c:
        c.prec = 200
        return sum(nums, Decimal(0))


def sum_or_mean(values, mean):
    nums = [d for d in map(number_of, values) if d is not None]
    if not nums:
        return None
    s = float(exact_sum(nums))
    return s / len(nums) if mean else s


def turkish_max(texts, greatest):
    """The greatest (the last of equals) or the least (the first) in Turkish order."""
    best = None
    for t in texts:
        if best is None:
            best = t
            continue
        c = base.compare(t, best)
        if (greatest and c >= 0) or (not greatest and c < 0):
            best = t
    return best


def extreme(values, greatest):
    nums = [d for d in map(number_of, values) if d is not None]
    if nums:
        return float(max(nums) if greatest else min(nums))
    texts = [v for v in values if isinstance(v, str) and v != ""]
    return turkish_max(texts, greatest)


def median(values):
    nums = sorted(d for d in map(number_of, values) if d is not None)
    if not nums:
        return None
    n = len(nums)
    if n % 2:
        return float(nums[n // 2])
    with localcontext() as c:
        c.prec = 200
        return float((nums[n // 2 - 1] + nums[n // 2]) / 2)


def std_dev(values):
    xs = [float(d) for d in map(number_of, values) if d is not None]
    if len(xs) < 2:
        return None
    mean = 0.0
    for x in xs:
        mean += x
    mean /= len(xs)
    ss = 0.0
    for x in xs:
        ss += (x - mean) * (x - mean)
    return math.sqrt(ss / (len(xs) - 1))


def aggregate(op, values, separator):
    given = [v for v in values if not is_empty(v)]
    if op == "sum":
        return sum_or_mean(values, False)
    if op == "mean":
        return sum_or_mean(values, True)
    if op == "count":
        return float(len(given))
    if op == "count_distinct":
        return float(len({key(v) for v in given}))
    if op in ("min", "max"):
        return extreme(values, op == "max")
    if op == "median":
        return median(values)
    if op == "stdev":
        return std_dev(values)
    if op == "concat":
        return separator.join(item_text(v) for v in given) if given else None
    if op == "array":
        return Arr(item(v) for v in values)
    raise AssertionError(op)


def item_text(v):
    if v is None:
        return ""
    if isinstance(v, float):
        return base.number_text(v)
    return text(v)


AGGREGATES = {"TOPLA": "sum", "SUM": "sum", "ORTALAMA": "mean", "MEAN": "mean", "AVG": "mean",
              "SAY": "count", "COUNT": "count", "SAY_BENZERSIZ": "count_distinct",
              "COUNT_DISTINCT": "count_distinct", "EN_DUSUK": "min", "MINIMUM": "min",
              "EN_YUKSEK": "max", "MAXIMUM": "max", "ORTA_DEGER": "median", "MEDIAN": "median",
              "STD_SAPMA": "stdev", "STDEV": "stdev", "STDDEV": "stdev",
              "DEGERLERI_BIRLESTIR": "concat", "CONCATENATE": "concat",
              "DEGERLERI_DIZI": "array", "ARRAY_AGG": "array"}

# ── Dates (ADR 0210 §3) ──────────────────────────────────────────────────

SECOND, MINUTE, HOUR, DAY = 1000, 60000, 3600000, 86400000
EPOCH = datetime.date(1970, 1, 1)


def digits(s, lo, hi):
    if not (lo <= len(s) <= hi) or not all(c in "0123456789" for c in s):
        return None
    return int(s)


def read_day(s):
    if len(s) == 10 and s[4] == "-" and s[7] == "-":
        y, m, d = digits(s[0:4], 4, 4), digits(s[5:7], 2, 2), digits(s[8:10], 2, 2)
    else:
        p = s.split(".")
        if len(p) != 3:
            return None
        d, m, y = digits(p[0], 1, 2), digits(p[1], 1, 2), digits(p[2], 4, 4)
    if None in (y, m, d) or not (1 <= y <= 9999 and 1 <= m <= 12):
        return None
    try:
        return datetime.date(y, m, d)
    except ValueError:
        return None


def read_zone(s):
    if s == "Z":
        return 0
    if s[:1] not in ("+", "-"):
        return None
    sign = 1 if s[0] == "+" else -1
    r = s[1:]
    if len(r) == 2:
        h, m = digits(r, 2, 2), 0
    elif len(r) == 4:
        h, m = digits(r[:2], 2, 2), digits(r[2:], 2, 2)
    elif len(r) == 5 and r[2] == ":":
        h, m = digits(r[:2], 2, 2), digits(r[3:], 2, 2)
    else:
        return None
    if h is None or m is None or h > 23 or m > 59:
        return None
    return sign * (h * HOUR + m * MINUTE)


def read_clock(s):
    cut = next((i for i, c in enumerate(s) if i >= 5 and c in "Z+-"), len(s))
    t, z = s[:cut], s[cut:]
    offset = read_zone(z) if z else 0
    if offset is None or len(t) < 5 or t[2] != ":":
        return None
    h, mi = digits(t[0:2], 2, 2), digits(t[3:5], 2, 2)
    rest = t[5:]
    sec, ms = 0, 0
    if rest:
        if not rest.startswith(":"):
            return None
        rest = rest[1:]
        whole, _, frac = rest.partition(".")
        sec = digits(whole, 2, 2)
        if "." in rest:
            if not (1 <= len(frac) <= 9) or not all(c in "0123456789" for c in frac):
                return None
            ms = int((frac + "000")[:3])
    if None in (h, mi, sec) or h > 23 or mi > 59 or sec > 59:
        return None
    return h * HOUR + mi * MINUTE + sec * SECOND + ms, offset


def moment(v):
    """A value as a moment: (milliseconds, written as a date alone), or None."""
    if not isinstance(v, str):
        return None
    s = v.strip(" \t")
    if not s or not s.isascii():
        return None
    cut = min((i for i in (s.find("T"), s.find(" ")) if i >= 0), default=-1)
    day, clock = (s, None) if cut < 0 else (s[:cut], s[cut + 1:])
    d = read_day(day)
    if d is None:
        return None
    into, offset = (0, 0) if clock is None else (read_clock(clock) or (None, None))
    if into is None:
        return None
    return (d - EPOCH).days * DAY + into - offset, cut < 0


def civil(t):
    days, into = divmod(t, DAY)
    return EPOCH + datetime.timedelta(days=days), into


def write(t, date_only):
    try:
        d, into = civil(t)
    except OverflowError:
        return None
    if not (1 <= d.year <= 9999):
        return None
    day = f"{d.year:04d}-{d.month:02d}-{d.day:02d}"
    if date_only or into == 0:
        return day
    h, mi, s, ms = into // HOUR, into % HOUR // MINUTE, into % MINUTE // SECOND, into % SECOND
    return f"{day}T{h:02d}:{mi:02d}:{s:02d}" + (f".{ms:03d}" if ms else "")


def whole(v, lo, hi):
    x = to_number(v)
    if x is None or x != int(x) or not (lo <= x <= hi):
        return None
    return int(x)


def days_in(y, m):
    return (datetime.date(y + (m == 12), m % 12 + 1, 1) - datetime.date(y, m, 1)).days


UNITS = {"YIL": "y", "YEAR": "y", "YEARS": "y", "AY": "mo", "MONTH": "mo", "MONTHS": "mo",
         "HAFTA": 7 * DAY, "WEEK": 7 * DAY, "WEEKS": 7 * DAY, "GUN": DAY, "DAY": DAY, "DAYS": DAY,
         "SAAT": HOUR, "HOUR": HOUR, "HOURS": HOUR, "DAKIKA": MINUTE, "MINUTE": MINUTE,
         "MINUTES": MINUTE, "SANIYE": SECOND, "SECOND": SECOND, "SECONDS": SECOND}


def js_round(x):
    return math.floor(x + 0.5)


def add_months(t, k):
    d, into = civil(t)
    total = d.year * 12 + (d.month - 1) + k
    y, m = divmod(total, 12)
    m += 1
    if not (1 <= y <= 9999):
        return None
    day = min(d.day, days_in(y, m))
    return (datetime.date(y, m, day) - EPOCH).days * DAY + into


def months_between(a, b):
    (da, ia), (db, ib) = civil(a), civil(b)
    months = (db.year * 12 + db.month) - (da.year * 12 + da.month)
    if months > 0 and (db.day, ib) < (da.day, ia):
        months -= 1
    elif months < 0 and (db.day, ib) > (da.day, ia):
        months += 1
    return months


MONTHS = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
          "Kasım", "Aralık"]
DAYS = ["Pazartesi", "Salı", "Çarşamba", "Perşembe", "Cuma", "Cumartesi", "Pazar"]
SHORT_DAYS = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cmt", "Paz"]
TOKENS = ["YYYY", "AAAA", "GGGG", "AAA", "GGG", "YY", "AA", "GG", "SS", "DD", "ss", "A", "G", "S", "D", "s"]


def date_format(t, fmt):
    d, into = civil(t)
    h, mi, s = into // HOUR, into % HOUR // MINUTE, into % MINUTE // SECOND
    values = {"YYYY": f"{d.year:04d}", "YY": f"{d.year % 100:02d}", "AAAA": MONTHS[d.month - 1],
              "AAA": MONTHS[d.month - 1][:3], "AA": f"{d.month:02d}", "A": str(d.month),
              "GGGG": DAYS[d.weekday()], "GGG": SHORT_DAYS[d.weekday()], "GG": f"{d.day:02d}",
              "G": str(d.day), "SS": f"{h:02d}", "S": str(h), "DD": f"{mi:02d}", "D": str(mi),
              "ss": f"{s:02d}", "s": str(s)}
    out = []
    i = 0
    while i < len(fmt):
        c = fmt[i]
        if c == "'":
            j = fmt.find("'", i + 1)
            if j == i + 1:
                out.append("'")
                i = j + 1
            elif j < 0:
                out.append(fmt[i + 1:])
                i = len(fmt)
            else:
                out.append(fmt[i + 1:j])
                i = j + 1
            continue
        tok = next((k for k in TOKENS if fmt.startswith(k, i)), None)
        if tok is None:
            out.append(c)
            i += 1
        else:
            out.append(values[tok])
            i += len(tok)
    return "".join(out)


def f_date(args):
    if len(args) == 1:
        m = moment(args[0])
        return None if m is None else write(m[0] // DAY * DAY, True)
    if len(args) < 3:
        return None
    y, mo, d = whole(args[0], 1, 9999), whole(args[1], 1, 12), whole(args[2], 1, 31)
    if None in (y, mo, d) or d > days_in(y, mo):
        return None
    return write((datetime.date(y, mo, d) - EPOCH).days * DAY, True)


def f_date_time(args):
    if len(args) == 1:
        m = moment(args[0])
        return None if m is None else write(m[0], False)
    y, mo, d = (whole(args[0], 1, 9999), whole(args[1], 1, 12) if len(args) > 1 else None,
                whole(args[2], 1, 31) if len(args) > 2 else None)
    h = whole(args[3], 0, 23) if len(args) > 3 else 0
    mi = whole(args[4], 0, 59) if len(args) > 4 else 0
    sec = to_number(args[5]) if len(args) > 5 else 0.0
    if None in (y, mo, d, h, mi, sec) or not (0 <= sec < 60) or d > days_in(y, mo):
        return None
    ms = js_round(sec * 1000.0)
    return write((datetime.date(y, mo, d) - EPOCH).days * DAY + h * HOUR + mi * MINUTE + ms, False)


def part(f):
    def run(args):
        m = moment(args[0])
        if m is None:
            return None
        d, into = civil(m[0])
        return float(f(d, into))
    return run


def f_date_add(args):
    m = moment(args[0])
    n = to_number(args[1])
    unit = UNITS.get(base.fold(base.trim(text(args[2])))) if args[2] is not None else None
    if m is None or n is None or unit is None:
        return None
    t, date_only = m
    if unit in ("y", "mo"):
        k = js_round(n * (12.0 if unit == "y" else 1.0))
        if abs(k) > 200000:
            return None
        r = add_months(t, int(k))
        if r is None:
            return None
    else:
        delta = js_round(n * unit)
        if not math.isfinite(delta) or abs(delta) > 4e14:
            return None
        r = t + int(delta)
    return write(r, date_only and r % DAY == 0)


def f_date_diff(args):
    a, b = moment(args[0]), moment(args[1])
    unit = UNITS.get(base.fold(base.trim(text(args[2])))) if args[2] is not None else None
    if a is None or b is None or unit is None:
        return None
    if unit in ("y", "mo"):
        months = months_between(a[0], b[0])
        if unit == "mo":
            return float(months)
        # Whole years toward zero; no year is 0, not −0.
        return float(math.trunc(months / 12)) + 0.0
    return (b[0] - a[0]) / unit


def f_date_format(args):
    m = moment(args[0])
    if m is None or args[1] is None:
        return None
    return date_format(m[0], text(args[1]))


# ── Regular expressions (§2.4) ───────────────────────────────────────────

def pattern(p):
    if len(p) > 1000:
        return None
    try:
        return re.compile(p, re.ASCII)
    except re.error:
        return None


def expand(m, rep):
    """The regex crate's replacement: $$, ${name}, $name (the longest name)."""
    out = []
    i = 0
    while i < len(rep):
        c = rep[i]
        if c != "$":
            out.append(c)
            i += 1
            continue
        if rep.startswith("$$", i):
            out.append("$")
            i += 2
            continue
        if rep.startswith("${", i):
            j = rep.find("}", i)
            if j < 0:
                out.append(c)
                i += 1
                continue
            name, i = rep[i + 2:j], j + 1
        else:
            j = i + 1
            while j < len(rep) and (rep[j].isascii() and (rep[j].isalnum() or rep[j] == "_")):
                j += 1
            if j == i + 1:
                out.append(c)
                i += 1
                continue
            name, i = rep[i + 1:j], j
        try:
            g = m.group(int(name)) if name.isdigit() else m.group(name)
        except (IndexError, re.error):
            g = None
        out.append(g or "")
    return "".join(out)


def regex_fn(kind):
    def run(args):
        t, p = args[0], args[1]
        if t is None or p is None:
            return False if kind == "match" else None
        r = pattern(text(p))
        if r is None:
            return None
        s = text(t)
        if kind == "match":
            return r.search(s) is not None
        if kind == "replace":
            # Every match replaced; with none the text is as it was.
            rep = text(args[2])
            return r.sub(lambda mm: expand(mm, rep), s)
        m = r.search(s)
        if kind == "find":
            return 0.0 if m is None else float(base.units(s[:m.start()]) + 1)
        if m is None:
            return None
        if kind == "part":
            return (m.group(1) or "") if r.groups else m.group(0)
        return Arr(g or "" for g in m.groups())
    return run


# ── Arrays and maps (§2.5) ───────────────────────────────────────────────

def arr(v):
    return v.items if isinstance(v, Arr) else None


def place(i, n):
    if i is None or not math.isfinite(i):
        return None
    i = float(math.trunc(i))
    at = n + i if i < 0 else i
    return int(at) if 0 <= at < n else None


def sort_rank(v):
    if v is None or v == "":
        return None
    if isinstance(v, bool):
        return (2, v)
    if compound(v):
        return (3, ("" if isinstance(v, Arr) else "") + json_of(v))
    x = to_number(v)
    if x is not None:
        return (0, x + 0.0)
    return (1, tuple(base.collation_key(v)), v)


def f_sort(args):
    items = arr(args[0])
    if items is None:
        return None
    ascending = len(args) < 2 or truthy(args[1])
    full = [v for v in items if sort_rank(v) is not None]
    empty = [v for v in items if sort_rank(v) is None]
    full.sort(key=sort_rank, reverse=not ascending)
    return Arr(full + empty)


def f_slice(args):
    items = arr(args[0])
    a, b = to_number(args[1]), to_number(args[2])
    if items is None or a is None or b is None:
        return None
    n = float(len(items))

    def at(i):
        i = float(math.trunc(i))
        return n + i if i < 0 else i
    lo, hi = max(at(a), 0.0), min(at(b), n - 1)
    return Arr(items[int(lo):int(hi) + 1] if lo <= hi and lo < n else [])


def f_to_text(args):
    items = arr(args[0])
    if items is None:
        return None
    sep = text(args[1]) if len(args) > 1 else ","
    blank = text(args[2]) if len(args) > 2 else ""
    return sep.join(blank if v is None else item_text(v) for v in items)


def f_from_text(args):
    if args[0] is None:
        return None
    t = text(args[0])
    sep = text(args[1]) if len(args) > 1 else ","
    blank = text(args[2]) if len(args) > 2 and args[2] is not None else None
    parts = list(t) if sep == "" else t.split(sep)
    return Arr(None if blank is not None and p == blank else p for p in parts)


def f_map(args):
    if len(args) % 2:
        return None
    return Map((text(args[k]), item(args[k + 1])) for k in range(0, len(args), 2))


def from_json(s):
    def constant(c):
        raise ValueError(c)

    def value(o):
        if isinstance(o, list):
            return Arr(value(i) for i in o)
        if isinstance(o, dict):
            return Map((k, value(v)) for k, v in o.items())
        if isinstance(o, bool) or o is None or isinstance(o, str):
            return o
        return float(o)
    try:
        o = json.loads(base.trim(s), parse_constant=constant,
                       object_pairs_hook=lambda pairs: dict(Map(pairs).pairs))
    except (ValueError, RecursionError):
        return None, False
    return value(o), True


def f_from_json(args):
    if args[0] is None:
        return None
    v, ok = from_json(text(args[0]))
    return v if ok else None


def map_of(v):
    return v.pairs if isinstance(v, Map) else None


def f_map_get(args, has):
    pairs = map_of(args[0])
    if pairs is None:
        return None
    k = text(args[1])
    found = [v for kk, v in pairs if kk == k]
    if has:
        return bool(found)
    return found[0] if found else None


def f_map_insert(args):
    pairs = map_of(args[0])
    if pairs is None:
        return None
    return Map(list(pairs) + [(text(args[1]), item(args[2]))])


def f_map_delete(args):
    pairs = map_of(args[0])
    if pairs is None:
        return None
    k = text(args[1])
    return Map((kk, v) for kk, v in pairs if kk != k)


def distinct(items):
    seen, out = set(), []
    for v in items:
        k = key(v)
        if k not in seen:
            seen.add(k)
            out.append(v)
    return out


def array_fn(f):
    def run(args):
        items = arr(args[0])
        return None if items is None else f(items, args)
    return run


EXTRAS = {
    "TARIH": (1, 3, f_date), "MAKE_DATE": None, "TO_DATE": None,
    "TARIH_SAAT": (1, 6, f_date_time), "MAKE_DATETIME": None, "TO_DATETIME": None, "TARIHSAAT": None,
    "YIL": (1, 1, part(lambda d, i: d.year)), "YEAR": None,
    "AY": (1, 1, part(lambda d, i: d.month)), "MONTH": None,
    "GUN": (1, 1, part(lambda d, i: d.day)), "DAY": None,
    "SAAT": (1, 1, part(lambda d, i: i // HOUR)), "HOUR": None,
    "DAKIKA": (1, 1, part(lambda d, i: i % HOUR // MINUTE)), "MINUTE": None,
    "SANIYE": (1, 1, part(lambda d, i: (i % MINUTE) / 1000)), "SECOND": None,
    "HAFTA": (1, 1, part(lambda d, i: d.isocalendar()[1])), "WEEK": None,
    "HAFTANIN_GUNU": (1, 1, part(lambda d, i: d.isoweekday())),
    "YILIN_GUNU": (1, 1, part(lambda d, i: d.timetuple().tm_yday)),
    "TARIH_EKLE": (3, 3, f_date_add), "TARIHEKLE": None,
    "TARIH_FARKI": (3, 3, f_date_diff), "TARIHFARKI": None,
    "TARIH_BICIMLE": (2, 2, f_date_format),
    "ESLESIR": (2, 2, regex_fn("match")), "REGEXP_LIKE": None,
    "DUZENLI_BUL": (2, 2, regex_fn("find")), "REGEXP_MATCH": None,
    "DUZENLI_PARCA": (2, 2, regex_fn("part")), "REGEXP_SUBSTR": None,
    "DUZENLI_GRUPLAR": (2, 2, regex_fn("groups")), "REGEXP_MATCHES": None,
    "DUZENLI_DEGISTIR": (3, 3, regex_fn("replace")), "REGEXP_REPLACE": None,
    "DIZI": (0, None, lambda args: Arr(item(a) for a in args)), "ARRAY": None,
    "DIZI_UZUNLUGU": (1, 1, array_fn(lambda items, args: float(len(items)))), "ARRAY_LENGTH": None,
    "DIZI_OGE": (2, 2, array_fn(lambda items, args: (lambda p: None if p is None else items[p])(
        place(to_number(args[1]), len(items))))), "ARRAY_GET": None,
    "DIZI_ILK": (1, 1, array_fn(lambda items, args: items[0] if items else None)), "ARRAY_FIRST": None,
    "DIZI_SON": (1, 1, array_fn(lambda items, args: items[-1] if items else None)), "ARRAY_LAST": None,
    "DIZI_ICERIR": (2, 2, array_fn(lambda items, args: any(equals(v, args[1]) for v in items))),
    "ARRAY_CONTAINS": None,
    "DIZI_BUL": (2, 2, array_fn(lambda items, args: float(next(
        (i for i, v in enumerate(items) if equals(v, args[1])), -1)))), "ARRAY_FIND": None,
    "DIZI_EKLE": (2, 2, array_fn(lambda items, args: Arr(items + [item(args[1])]))), "ARRAY_APPEND": None,
    "DIZI_BIRLESTIR": (2, 2, array_fn(lambda items, args: None if arr(args[1]) is None else
                                      Arr(items + arr(args[1])))), "ARRAY_CAT": None,
    "DIZI_BENZERSIZ": (1, 1, array_fn(lambda items, args: Arr(distinct(items)))), "ARRAY_DISTINCT": None,
    "DIZI_SIRALA": (1, 2, f_sort), "ARRAY_SORT": None,
    "DIZI_TERS": (1, 1, array_fn(lambda items, args: Arr(reversed(items)))), "ARRAY_REVERSE": None,
    "DIZI_DILIM": (3, 3, f_slice), "ARRAY_SLICE": None,
    "DIZI_METIN": (1, 3, f_to_text), "ARRAY_TO_STRING": None,
    "METIN_DIZI": (1, 3, f_from_text), "STRING_TO_ARRAY": None,
    "DIZI_TOPLA": (1, 1, array_fn(lambda items, args: sum_or_mean(items, False))), "ARRAY_SUM": None,
    "DIZI_ORTALAMA": (1, 1, array_fn(lambda items, args: sum_or_mean(items, True))), "ARRAY_MEAN": None,
    "DIZI_EN_DUSUK": (1, 1, array_fn(lambda items, args: extreme(items, False))), "ARRAY_MIN": None,
    "DIZI_EN_YUKSEK": (1, 1, array_fn(lambda items, args: extreme(items, True))), "ARRAY_MAX": None,
    "ESLEME": (0, None, f_map), "MAP": None,
    "ESLEME_DEGERI": (2, 2, lambda args: f_map_get(args, False)), "MAP_GET": None,
    "ESLEME_ICERIR": (2, 2, lambda args: f_map_get(args, True)), "MAP_EXIST": None,
    "ESLEME_ANAHTARLARI": (1, 1, lambda args: None if map_of(args[0]) is None else
                           Arr(k for k, _ in map_of(args[0]))), "MAP_AKEYS": None,
    "ESLEME_DEGERLERI": (1, 1, lambda args: None if map_of(args[0]) is None else
                         Arr(v for _, v in map_of(args[0]))), "MAP_AVALS": None,
    "ESLEME_EKLE": (3, 3, f_map_insert), "MAP_INSERT": None,
    "ESLEME_SIL": (2, 2, f_map_delete), "MAP_DELETE": None,
    "JSON_OKU": (1, 1, f_from_json), "FROM_JSON": None,
    "JSON_YAZ": (1, 1, lambda args: json_of(item(args[0]))), "TO_JSON": None,
}
# English names share their Turkish entry.
_last = None
for _name in list(EXTRAS):
    if EXTRAS[_name] is None:
        EXTRAS[_name] = EXTRAS[_last]
    else:
        _last = _name

# The functions that look at other objects: least and most arguments, and the roles.
WORLD = {
    **{k: (1, 3, ("value", "group", "condition"), op) for k, op in [
        ("TOPLA", "sum"), ("SUM", "sum"), ("ORTALAMA", "mean"), ("MEAN", "mean"), ("AVG", "mean"),
        ("SAY", "count"), ("COUNT", "count"), ("SAY_BENZERSIZ", "count_distinct"),
        ("COUNT_DISTINCT", "count_distinct"), ("EN_DUSUK", "min"), ("MINIMUM", "min"),
        ("EN_YUKSEK", "max"), ("MAXIMUM", "max"), ("ORTA_DEGER", "median"), ("MEDIAN", "median"),
        ("STD_SAPMA", "stdev"), ("STDEV", "stdev"), ("STDDEV", "stdev"),
        ("DEGERLERI_DIZI", "array"), ("ARRAY_AGG", "array")]},
    "DEGERLERI_BIRLESTIR": (1, 4, ("value", "text", "group", "condition"), "concat"),
    "CONCATENATE": (1, 4, ("value", "text", "group", "condition"), "concat"),
    "KATMAN_TOPLAMI": (3, 4, ("layer", "text", "value", "condition"), "aggregate"),
    "AGGREGATE": (3, 4, ("layer", "text", "value", "condition"), "aggregate"),
    "KATMANDAN": (4, 4, ("layer", "value", "text", "own"), "from"),
    "KESISIR": (1, 3, ("layer", "own", "condition"), "intersects"),
    "OVERLAY_INTERSECTS": (1, 3, ("layer", "own", "condition"), "intersects"),
    "KESISEN_SAYISI": (1, 3, ("layer", "own", "condition"), "intersect_count"),
    "KESISENLER": (2, 4, ("layer", "value", "own", "condition"), "list"),
    "KAPSAR": (1, 2, ("layer", "condition"), "contains"),
    "OVERLAY_CONTAINS": (1, 2, ("layer", "condition"), "contains"),
    "ICINDE_KALIR": (1, 2, ("layer", "condition"), "within"),
    "OVERLAY_WITHIN": (1, 2, ("layer", "condition"), "within"),
    "MERKEZI_ICINDE": (1, 2, ("layer", "condition"), "center"),
    "UZAKLIK": (1, 2, ("layer", "condition"), "distance"),
    "EN_YAKIN": (2, 3, ("layer", "value", "condition"), "nearest"),
    "OVERLAY_NEAREST": (2, 3, ("layer", "value", "condition"), "nearest"),
    "KESISIM_ALANI": (1, 2, ("layer", "condition"), "area"),
    "KESISIM_UZUNLUGU": (1, 2, ("layer", "condition"), "length"),
}

VARS = {"ALAN": "area", "UZUNLUK": "length", "CEVRE": "length", "ID": "id", "KATMAN": "layer"}

# ── The grammar ──────────────────────────────────────────────────────────


def tokenize(src):
    """The language's tokens, with `$name` and `@name`; a lone `@` does not read."""
    toks = []
    i = 0
    while i < len(src):
        c = src[i]
        if c in ("$", "@"):
            j = i + 1
            if j < len(src) and (src[j].isalpha() or src[j] == "_"):
                while j < len(src) and (src[j].isalnum() or src[j] == "_"):
                    j += 1
                toks.append(("var" if c == "$" else "at", src[i + 1:j]))
                i = j
                continue
            raise Invalid(f"lone {c}")
        k = i
        # The rest is the language's own tokenizer on the run up to the next $ or @ outside text and brackets.
        quote, bracket = None, False
        while k < len(src):
            ch = src[k]
            if quote:
                if ch == quote:
                    if k + 1 < len(src) and src[k + 1] == quote:
                        k += 1
                    else:
                        quote = None
            elif bracket:
                bracket = ch != "]"
            elif ch in "'\"":
                quote = ch
            elif ch == "[":
                bracket = True
            elif ch in "$@":
                break
            k += 1
        toks.extend(t for t in base.tokenize(src[i:k]) if t[0] != "end")
        i = k
    toks.append(("end", None))
    return toks


class Parser(base.Parser):
    def __init__(self, toks):
        super().__init__(toks)
        self.inner = 0

    def primary(self):
        t = self.peek()
        if t[0] == "var":
            self.next()
            if base.key(t[1]) not in VARS:
                raise Invalid(f"unknown ${t[1]}")
            return ("var", VARS[base.key(t[1])])
        if t[0] == "at":
            self.next()
            return ("at", t[1])
        return super().primary()

    def call(self, name):
        k = base.key(name)
        if k in WORLD:
            least, most, roles, op = WORLD[k]
            if self.inner:
                raise Invalid("a call to other objects inside another")
            self.next()
            args = []
            if self.op() != ")":
                while True:
                    role = roles[len(args)] if len(args) < len(roles) else None
                    inner = role in ("value", "group", "condition")
                    self.inner += inner
                    args.append(self.disjunction())
                    self.inner -= inner
                    if self.op() != ",":
                        break
                    self.next()
            if self.op() != ")":
                raise Invalid("call not closed")
            self.next()
            if len(args) < least or len(args) > most:
                raise Invalid(f"{name} takes {least}…{most}")
            for role, a in zip(roles, args):
                if role in ("layer", "text") and not (a[0] == "lit" and isinstance(a[1], str)):
                    raise Invalid("a constant text is wanted")
            if op == "aggregate" and base.key(args[1][1]) not in AGGREGATES:
                raise Invalid("unknown aggregate")
            return ("world", k, op, roles, args)
        if k in ("SIMDI", "NOW", "BUGUN", "TODAY"):
            self.next()
            if self.op() != ")":
                raise Invalid("şimdi() takes no value")
            self.next()
            return ("at", "simdi" if k in ("SIMDI", "NOW") else "tarih")
        if k in EXTRAS or k in CORE:
            least, most, _ = EXTRAS[k] if k in EXTRAS else CORE[k]
            self.next()
            args = []
            if self.op() != ")":
                args.append(self.disjunction())
                while self.op() == ",":
                    self.next()
                    args.append(self.disjunction())
            if self.op() != ")":
                raise Invalid("call not closed")
            self.next()
            if len(args) < least or (most is not None and len(args) > most):
                raise Invalid(f"{name} takes {least}…{most}")
            if k in ("ESLESIR", "DUZENLI_BUL", "DUZENLI_PARCA", "DUZENLI_GRUPLAR", "DUZENLI_DEGISTIR",
                     "REGEXP_LIKE", "REGEXP_MATCH", "REGEXP_SUBSTR", "REGEXP_MATCHES", "REGEXP_REPLACE") \
                    and args[1][0] == "lit" and isinstance(args[1][1], str) and pattern(args[1][1]) is None:
                raise Invalid("a pattern that does not read")
            if k in ("ESLEME", "MAP") and len(args) % 2:
                raise Invalid("keys and values in pairs")
            return ("xcall", k, args) if k in EXTRAS else ("call", k, args)
        return super().call(name)


# ── Evaluation ───────────────────────────────────────────────────────────


class Context:
    """What an expression sees: the object, its layer, the world and the variables."""

    def __init__(self, obj, layer, world):
        self.obj = obj
        self.layer = layer
        self.world = world


def geometry_of(o):
    if o["kind"] == "polygon":
        return Polygon([(p["x"], p["y"]) for p in o["pts"]])
    if o["kind"] == "point":
        return Point(o["p"]["x"], o["p"]["y"])
    return LineString([(p["x"], p["y"]) for p in o["pts"]])


def builtin(name, ctx):
    o = ctx.obj
    g = geometry_of(o)
    if name == "area":
        return float(g.area) if o["kind"] == "polygon" else None
    if name == "length":
        return float(g.length) if o["kind"] in ("polygon", "polyline") else None
    if name == "id":
        return float(o["id"])
    if name == "layer":
        return ctx.layer["name"]
    raise AssertionError(name)


VARIABLE_VALUES = {base.fold(v["name"]): v["value"] for v in reversed(VARIABLES)}


def variable(name, unknown):
    k = base.fold(name)
    if k in VARIABLE_VALUES:
        v = VARIABLE_VALUES[k]
        return float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else v
    if k in ("KATMAN_ADI", "KATMAN"):
        return ("layer",)
    if name not in unknown:
        unknown.append(name)
    return None


def ev(n, ctx):
    kind = n[0]
    if kind == "lit":
        return n[1]
    if kind == "field":
        return ctx.obj["attrs"].get(n[1])
    if kind == "var":
        return builtin(n[1], ctx)
    if kind == "at":
        v = variable(n[1], [])
        return ctx.layer["name"] if v == ("layer",) else v
    if kind == "xcall":
        _, _, f = EXTRAS[n[1]]
        return f([ev(a, ctx) for a in n[2]])
    if kind == "world":
        return world_call(n, ctx)
    if kind == "case":
        for c, v in n[1]:
            if truthy(ev(c, ctx)):
                return ev(v, ctx)
        return ev(n[2], ctx) if n[2] is not None else None
    if kind == "in":
        x = ev(n[1], ctx)
        items = [ev(i, ctx) for i in n[2]]
        return any(equals(x, i) for i in items) != n[3]
    if kind == "between":
        x, low, high = ev(n[1], ctx), ev(n[2], ctx), ev(n[3], ctx)
        lo, hi = compare(x, low), compare(x, high)
        return (lo is not None and lo >= 0 and hi is not None and hi <= 0) != n[4]
    if kind == "like":
        x, p = ev(n[1], ctx), ev(n[2], ctx)
        return base.like(text(x) if compound(x) else x, text(p) if compound(p) else p, n[3]) != n[4]
    if kind == "isnull":
        return is_empty(ev(n[1], ctx)) != n[2]
    if kind == "not":
        return not truthy(ev(n[1], ctx))
    if kind == "pos":
        return ev(n[1], ctx)
    if kind == "neg":
        x = to_number(ev(n[1], ctx))
        return None if x is None else -x
    if kind == "call":
        _, _, f = CORE[n[1]] if n[1] in CORE else base.FUNCTIONS[n[1]]
        return f([ev(a, ctx) for a in n[2]])
    a, b = ev(n[1], ctx), ev(n[2], ctx)
    if kind == "or":
        return truthy(a) or truthy(b)
    if kind == "and":
        return truthy(a) and truthy(b)
    if kind == "eq":
        return equals(a, b)
    if kind == "ne":
        return not equals(a, b)
    if kind in ("lt", "le", "gt", "ge"):
        c = compare(a, b)
        if c is None:
            return False
        return {"lt": c < 0, "le": c <= 0, "gt": c > 0, "ge": c >= 0}[kind]
    if kind == "join":
        return text(a) + text(b)
    na, nb = to_number(a), to_number(b)
    if kind == "add":
        if a is None or b is None:
            return None
        if na is not None and nb is not None:
            return na + nb
        return text(a) + text(b)
    if na is None or nb is None:
        return None
    if kind in ("div", "rem") and nb == 0:
        return None
    if kind == "sub":
        return na - nb
    if kind == "mul":
        return na * nb
    if kind == "div":
        return na / nb
    if kind == "rem":
        return math.fmod(na, nb)
    if kind == "pow":
        return base.power(na, nb)
    raise AssertionError(kind)


def f_text(args):
    v = args[0]
    if len(args) < 2:
        return text(v) if v is not None else ""
    raise AssertionError("metin(x, d) is not in this file's sources")


# The language's functions this file's sources use, on its values (arrays and maps among them).
CORE = {
    "VARSAYILAN": (2, None, lambda args: next((a for a in args if not is_empty(a)), None)),
    "EGER": (3, 3, lambda args: args[1] if truthy(args[0]) else args[2]),
    "UZUNLUK": (1, 1, lambda args: None if args[0] is None else float(base.units(text(args[0])))),
    "METIN": (1, 2, f_text),
}


def layer_named(world, name):
    k = base.fold(base.trim(name))
    return next((l for l in world if base.fold(base.trim(l["name"])) == k), None)


def inner_values(node, layer, world):
    return [value_of(node, Context(o, layer, world)) for o in layer["objects"]]


def value_of(node, ctx):
    try:
        v = ev(node, ctx)
    except Thrown:
        return None
    if isinstance(v, float) and not math.isfinite(v):
        return None
    return v


def world_call(n, ctx):
    _, k, op, roles, args = n
    world = ctx.world
    named = dict(zip(roles, args))
    if op in ("sum", "mean", "count", "count_distinct", "min", "max", "median", "stdev", "array", "concat"):
        layer = ctx.layer
        values = inner_values(named["value"], layer, world)
        mask = [truthy(v) for v in inner_values(named["condition"], layer, world)] if "condition" in named \
            else [True] * len(values)
        if "group" in named:
            groups = [key(v) for v in inner_values(named["group"], layer, world)]
            mine = groups[layer["objects"].index(ctx.obj)]
            chosen = [v for v, g, m in zip(values, groups, mask) if m and g == mine]
        else:
            chosen = [v for v, m in zip(values, mask) if m]
        separator = named["text"][1] if "text" in named else ", "
        return aggregate(op, chosen, separator)
    layer = layer_named(world, named["layer"][1])
    if layer is None:
        return None
    if op == "aggregate":
        values = inner_values(named["value"], layer, world)
        mask = [truthy(v) for v in inner_values(named["condition"], layer, world)] if "condition" in named \
            else [True] * len(values)
        return aggregate(AGGREGATES[base.key(named["text"][1])], [v for v, m in zip(values, mask) if m], ", ")
    if op == "from":
        want = key(ev(named["own"], ctx))
        if want is None:
            return None
        field = named["text"][1]
        for o in layer["objects"]:
            if key(o["attrs"].get(field)) == want:
                return value_of(named["value"], Context(o, layer, world))
        return None
    return spatial(op, named, ctx, layer)


def spatial(op, named, ctx, layer):
    world = ctx.world
    g = geometry_of(ctx.obj)
    others = [o for o in layer["objects"] if o["id"] != ctx.obj["id"]]
    if "condition" in named:
        others = [o for o in others if truthy(value_of(named["condition"], Context(o, layer, world)))]
    d = to_number(ev(named["own"], ctx)) if "own" in named else None
    near = d is not None and d > 0

    def related(how):
        out = []
        for o in others:
            h = geometry_of(o)
            if how == "intersects":
                ok = h.distance(g) <= d + 1e-9 * max(d, 1.0) if near else g.intersects(h)
            elif how == "contains":
                ok = g.covers(h)
            elif how == "within":
                ok = h.covers(g)
            else:
                ok = h.covers(g.centroid)
            if ok:
                out.append(o)
        return out
    if op == "intersects":
        return bool(related("intersects"))
    if op == "intersect_count":
        return float(len(related("intersects")))
    if op == "list":
        return Arr(item(value_of(named["value"], Context(o, layer, world))) for o in related("intersects"))
    if op in ("contains", "within", "center"):
        return bool(related(op))
    if op in ("distance", "nearest"):
        if not others:
            return None
        best = min(others, key=lambda o: geometry_of(o).distance(g))
        if op == "distance":
            return float(geometry_of(best).distance(g))
        return value_of(named["value"], Context(best, layer, world))
    if op == "area":
        if not isinstance(g, Polygon):
            return 0.0
        return float(sum(g.intersection(geometry_of(o)).area for o in related("intersects")
                         if isinstance(geometry_of(o), Polygon)))
    if op == "length":
        line = g.boundary if isinstance(g, Polygon) else g
        return float(sum(line.intersection(geometry_of(o)).length for o in related("intersects")
                         if isinstance(geometry_of(o), Polygon)))
    raise AssertionError(op)


def encode(v):
    if compound(v):
        return ["t", json_of(v)]
    return base.encode(v)


NEAR = ("uzaklık", "kesişim_alanı", "kesişim_uzunluğu")


def unknown_names(tree):
    found = []

    def walk(n):
        if isinstance(n, tuple):
            if n and n[0] == "at":
                variable(n[1], found)
            for x in n:
                walk(x)
        elif isinstance(n, list):
            for x in n:
                walk(x)
    walk(tree)
    return found


def build():
    world = LAYERS
    evaluated = next(l for l in world if l["id"] == EVALUATED)
    cases = []
    for src in SOURCES:
        try:
            tree = Parser(tokenize(src)).parse()
        except Invalid:
            cases.append({"source": src, "compiles": False})
            continue
        case = {"source": src,
                "values": [encode(value_of(tree, Context(o, evaluated, world))) for o in evaluated["objects"]]}
        unknown = unknown_names(tree)
        if unknown:
            case["unknown"] = unknown
        if any(w in src for w in NEAR):
            case["near"] = True
        cases.append(case)
    return {
        "format": "kentos.expression-extras",
        "version": 1,
        "note": "ADR 0214'ün ekleri (tarih ve saat, @ değişkenleri, düzenli ifade, dizi ve eşleme, toplama, "
                "mekânsal ilişki, başka katmandan değer), motordan bağımsız hesaplanmış: her kaynağın "
                "Parsel katmanının her nesnesindeki değeri ya da derlenmediği. Değer: null boş, [\"n\", x] "
                "sayı, [\"t\", s] metin (dizi ve eşleme JSON'u), [\"b\", b] doğru/yanlış. \"near\": sayıları "
                "1e-9 göreli yakınlıkla karşılaştırılır (uzaklık ve örtüşmeler). \"unknown\": değeri olmayan @ adları. "
                "Üretici scripts/fixtures/expression_extras.py (--check yalnız karşılaştırır).",
        "variables": VARIABLES,
        "evaluated": EVALUATED,
        "layers": LAYERS,
        "cases": cases,
    }


def main():
    doc = build()
    out = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != out:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: {len(doc['cases'])} durum; güncel.")
        return
    OUT.write_text(out, encoding="utf-8")
    print(f"{OUT} yazıldı: {len(doc['cases'])} durum.")


if __name__ == "__main__":
    main()
