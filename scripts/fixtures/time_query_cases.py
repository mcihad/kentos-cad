#!/usr/bin/env python3
"""The shared cases of cad.time.query (docs/adr/0537; plan/gis.md CBS-33).

    python3 scripts/fixtures/time_query_cases.py           # writes fixtures/commands/v1/cad.time.query.json
    python3 scripts/fixtures/time_query_cases.py --check   # compares

No KentOS code is used. A moment is read, an object's valid time made and a window's rule applied by
scripts/fixtures/temporal_cases.py (ADR 0210 §3–§4, written from the ADR); a version's system time is the ADR 0537
rule written here: [recorded start, recorded end) with an empty end still current, an empty or unreadable start never
known; a version is known on a day it was recorded by and not yet superseded. The drawing holds parcels with
versions, a late correction, a version superseded on a day, an empty valid range, a zone, a timeless object, an
unrecorded and an unreadable one; an instant layer, a cumulative one, one without a system time, one without a time.
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import temporal_cases as tc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.time.query.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

PARSEL_TIME = {"start": "gecerlilik_baslangic", "end": "gecerlilik_bitis", "key": "parsel_no",
               "recordedStart": "kayit_baslangic", "recordedEnd": "kayit_bitis"}


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": children or [], **extra}


# (id, attrs) of the parcels: each a version of a parcel by its number.
PARCELS = [
    (1, {"parsel_no": "101", "gecerlilik_baslangic": "2020-01-01", "gecerlilik_bitis": "2024-03-01",
         "kayit_baslangic": "2020-01-05", "kayit_bitis": "2024-03-02"}),
    (2, {"parsel_no": "101", "gecerlilik_baslangic": "2024-03-01", "kayit_baslangic": "2024-03-02"}),
    # A late correction: valid since 2022, recorded only in 2025 (09:00 in Türkiye).
    (3, {"parsel_no": "102", "gecerlilik_baslangic": "2022-06-01", "kayit_baslangic": "2025-01-10T09:00:00+03:00"}),
    (4, {"parsel_no": "103", "gecerlilik_baslangic": "2021-01-01", "gecerlilik_bitis": "2023-01-01",
         "kayit_baslangic": "2021-01-02", "kayit_bitis": "2023-01-01"}),
    # Timeless (no valid time), recorded.
    (5, {"parsel_no": "104", "kayit_baslangic": "2022-01-01"}),
    # Never recorded.
    (6, {"parsel_no": "105", "gecerlilik_baslangic": "2019-01-01"}),
    # Unreadable: a word for the valid start and the recorded start.
    (7, {"parsel_no": "106", "gecerlilik_baslangic": "dün", "kayit_baslangic": "?"}),
    # An empty valid range: [s, s).
    (8, {"parsel_no": "107", "gecerlilik_baslangic": "2023-05-05", "gecerlilik_bitis": "2023-05-05",
         "kayit_baslangic": "2023-05-05"}),
    # A zone: 00:30 in Türkiye on 1 June is 21:30 UTC on 31 May.
    (9, {"parsel_no": "108", "gecerlilik_baslangic": "2024-06-01T00:30:00+03:00", "kayit_baslangic": "2024-06-01"}),
]
FAULTS = [(10, {"an": "2024-06-01T10:00:00Z"}), (11, {"an": "2024-06-02"})]
SINCE = [(12, {"baslangic": "2020-01-01", "bitis": "2021-01-01"}), (13, {"baslangic": "2025-01-01"})]
PLAIN = [(14, {"baslangic": "2020-01-01", "bitis": "2030-01-01"})]


def square(id_, layer, attrs):
    x, y = 500000 + 30 * id_, 4400000
    pts = [{"x": x, "y": y}, {"x": x + 20, "y": y}, {"x": x + 20, "y": y + 20}, {"x": x, "y": y + 20}]
    return {"kind": "polygon", "id": id_, "layerId": layer, "attrs": attrs, "pts": pts}


def drawing():
    layers = [
        node("parsel", "Parsel", time=PARSEL_TIME),
        node("ariza", "Arıza", time={"start": "an"}),
        node("birikim", "Yapı", time={"start": "baslangic", "end": "bitis", "cumulative": True}),
        node("plan", "Plan", time={"start": "baslangic", "end": "bitis"}),
        node("zamansiz", "Zamansız"),
        node("grup", "Grup", type_="group", children=[node("ic", "İç")]),
    ]
    ents = [square(i, "parsel", a) for i, a in PARCELS] + [square(i, "ariza", a) for i, a in FAULTS]
    ents += [square(i, "birikim", a) for i, a in SINCE] + [square(i, "plan", a) for i, a in PLAIN]
    return {"format": "kentos.document", "version": 1, "name": "Zamanda sorgu",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 500000.0, "y": 4400000.0}, "layers": layers, "activeLayer": "parsel", "entities": ents,
            "styles": {"items": [], "categories": []}}


LAYER_TIMES = {"parsel": PARSEL_TIME, "ariza": {"start": "an"},
               "birikim": {"start": "baslangic", "end": "bitis", "cumulative": True},
               "plan": {"start": "baslangic", "end": "bitis"}}
OBJECTS = {"parsel": PARCELS, "ariza": FAULTS, "birikim": SINCE, "plan": PLAIN}


def utc(t):
    w = tc.write(t, False)
    return w + "Z" if "T" in w else w


def recorded_interval(start, end):
    """ADR 0537: [start, end), an empty end +∞; None when the start is empty or unreadable, or the end unreadable."""
    s = tc.read(start) if start is not None else "empty"
    e = tc.read(end) if end is not None else "empty"
    if not isinstance(s, int) or e == "unreadable":
        return None
    return (s, tc.INF if e == "empty" else e)


def answer(layer, at=None, frm=None, to=None, known=None):
    t = LAYER_TIMES[layer]
    rule = {"ranged": "end" in t, "cumulative": t.get("cumulative", False)}
    window = None
    if at is not None:
        window = {"instant": tc.read(at)}
    elif frm is not None:
        window = {"range": (tc.read(frm), tc.read(to))}
    k = tc.read(known) if known is not None else None
    out = {"layer": layer}
    if window and "instant" in window:
        out["at"] = utc(window["instant"])
    if window and "range" in window:
        out["from"], out["to"] = utc(window["range"][0]), utc(window["range"][1])
    if k is not None:
        out["knownAt"] = utc(k)
    shown, not_valid, not_known, timeless, unrecorded, unreadable = [], 0, 0, 0, 0, 0
    for id_, attrs in OBJECTS[layer]:
        start, end = attrs.get(t["start"]), attrs.get(t.get("end")) if "end" in t else None
        unreadable += (start is not None and tc.read(start) == "unreadable") + (
            rule["ranged"] and end is not None and tc.read(end) == "unreadable")
        valid = tc.object_time(rule, start, end if rule["ranged"] else None)
        timeless += valid is None
        ok = valid is None or window is None or tc.shows(valid, window)
        not_valid += not ok
        if "recordedStart" in t:
            rs, re_ = attrs.get(t["recordedStart"]), attrs.get(t.get("recordedEnd"))
            unreadable += (rs is not None and tc.read(rs) == "unreadable") + (re_ is not None and tc.read(re_) == "unreadable")
            iv = recorded_interval(rs, re_)
            unrecorded += iv is None
            if k is not None and not (iv is not None and iv[0] <= k < iv[1]):
                not_known += ok
                ok = False
        if ok:
            shown.append(f"$uidOf:{id_}")
    out.update({"shown": shown, "notValid": not_valid, "notKnown": not_known, "timeless": timeless,
                "unrecorded": unrecorded, "unreadable": unreadable, "revision": "$current"})
    return out


def done(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result, "expect": {"revision": "same"}}


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def asked(name, layer, op="execute", **kw):
    inp = {"layer": layer}
    for key, field in (("at", "at"), ("frm", "from"), ("to", "to"), ("known", "knownAt")):
        if key in kw:
            inp[field] = kw[key]
    return case(name, step(inp, done(answer(layer, kw.get("at"), kw.get("frm"), kw.get("to"), kw.get("known"))), op))


def moment_refused(text, path):
    return failed("invalid_moment",
                  f"“{text}” bir an değil: bir tarih (2024-06-01 ya da 01.06.2024) ya da tarih ve saat (2024-06-01T14:30, "
                  "bölgesiyle 2024-06-01T14:30+03:00) yazın.", path)


def cases():
    return [
        asked("1 Haziran 2024'te geçerli parseller, bugün bilindiği gibi", "parsel", at="2024-06-01"),
        asked("1 Haziran 2024'te geçerli, 1 Ocak 2025'te bilinen: geç gelen düzeltme yok", "parsel", at="2024-06-01",
              known="2025-01-01"),
        asked("1 Haziran 2024'te geçerli, 1 Şubat 2025'te bilinen: geç gelen düzeltme var", "parsel", at="2024-06-01",
              known="2025-02-01"),
        asked("geç gelen kaydın bilinmeye başladığı an, bölgesiyle", "parsel", at="2024-06-01",
              known="2025-01-10T06:00:00Z"),
        asked("bir dakika önce bilinmiyordu", "parsel", at="2024-06-01", known="2025-01-10T05:59:00Z"),
        asked("2023 ortasında geçerli olan, o gün bilindiği gibi", "parsel", at="2023-06-01", known="2023-06-01"),
        asked("sürümün bittiği an: aralık bitişi içermez, yenisi geçerli", "parsel", at="2024-03-01"),
        asked("eski sürümün yerine başkasının geçtiği gün eski sürüm bilinmez", "parsel", at="2024-02-01",
              known="2024-03-02"),
        asked("bir gün önce eski sürüm bilinir", "parsel", at="2024-02-01", known="2024-03-01"),
        asked("2022 boyunca geçerli olanlar (aralık)", "parsel", frm="2022-01-01", to="2023-01-01"),
        asked("boş geçerlilik aralığı: bir anda hiç, onu içeren aralıkta görünür", "parsel", frm="2023-05-01",
              to="2023-06-01"),
        asked("boş geçerlilik aralığı kendi anında görünmez", "parsel", at="2023-05-05"),
        asked("bölge: Türkiye'de 1 Haziran 00:30, UTC'de 31 Mayıs 21:30", "parsel", frm="2024-05-31T21:00:00Z",
              to="2024-05-31T22:00:00Z"),
        asked("bölgeli an: 1 Haziran 00:00 Türkiye saati", "parsel", at="2024-06-01T00:00:00+03:00"),
        asked("gün.ay.yıl biçimi", "parsel", at="01.06.2024"),
        asked("yalnız bilinen gün: geçerlilikten bağımsız o gün bilinenler", "parsel", known="2024-01-01"),
        asked("geçerlilik de bilinen gün de yoksa hepsi", "parsel"),
        asked("anlık katman: tam anında", "ariza", at="2024-06-01T10:00:00Z"),
        asked("anlık katman: bir aralıkta", "ariza", frm="2024-06-01", to="2024-06-03"),
        asked("birikimli katman: başladıktan sonra hep", "birikim", at="2026-01-01"),
        asked("kayıt zamanı tutmayan katman: geçerlilikle", "plan", at="2025-01-01"),
        asked("plan aynı cevabı verir", "parsel", op="plan", at="2024-06-01", known="2025-02-01"),
        case("validate yalnız denetler", step({"layer": "parsel", "at": "2024-06-01"},
                                               {"status": "completed", "output": None, "warnings": []}, op="validate")),
        case("kayıt zamanı tutmayan katmanda bilinen gün sorulamaz", step(
            {"layer": "plan", "knownAt": "2025-01-01"},
            failed("not_recorded", "“Plan” katmanı sürümlerinin kayıt zamanını tutmuyor; Zaman ayarları'nda kayıt "
                   "başlangıcı alanını seçin.", "knownAt"))),
        case("zaman ayarı olmayan katman", step({"layer": "zamansiz", "at": "2024-01-01"}, failed(
            "not_temporal", "“Zamansız” katmanının zaman ayarı yok; önce Zaman ayarları ile başlangıç alanını seçin.",
            "layer"))),
        case("grup", step({"layer": "grup"}, failed(
            "not_a_layer", "“Grup” bir katman grubu; zaman katmanın ayarıdır. Grubun içinden bir katman seçin.", "layer"))),
        case("olmayan katman", step({"layer": "yok"}, failed(
            "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))),
        case("okunmayan an", step({"layer": "parsel", "at": "geçen yıl"}, moment_refused("geçen yıl", "at"))),
        case("okunmayan bilinen gün", step({"layer": "parsel", "knownAt": "31.02.2024"},
                                           moment_refused("31.02.2024", "knownAt"))),
        case("an ve aralık birlikte", step({"layer": "parsel", "at": "2024-01-01", "from": "2024-01-01"}, failed(
            "invalid_period", "Geçerlilik zamanı ya bir an (at) ya bir aralıktır (from ve to); ikisi birden verilmez.",
            "at"))),
        case("aralığın yalnız başlangıcı", step({"layer": "parsel", "from": "2024-01-01"}, failed(
            "invalid_period", "Aralığın başlangıcı (from) ve bitişi (to) birlikte verilir.", "to"))),
        case("aralığın bitişi başlangıcından önce", step({"layer": "parsel", "from": "2024-01-02", "to": "2024-01-01"},
                                                         failed("invalid_period", "Aralığın bitişi başlangıcından sonra "
                                                                "olmalı; [from, to) bitişi içermez.", "to"))),
        case("aralığın bitişi başlangıcına eşit", step({"layer": "parsel", "from": "2024-01-01", "to": "2024-01-01"},
                                                       failed("invalid_period", "Aralığın bitişi başlangıcından sonra "
                                                              "olmalı; [from, to) bitişi içermez.", "to"))),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.time.query", "commandVersion": 1,
            "title": "Zamanda sorgu",
            "note": ("ADR 0537. Bağımsız başvuru scripts/fixtures/time_query_cases.py: anların okunuşu, geçerlilik "
                     "zamanı ve pencerenin kuralı temporal_cases.py'den (ADR 0210), kayıt zamanı ADR 0537'nin "
                     "kuralıyla; KentOS kodu yok."),
            "setup": drawing(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"Zamanda sorgu durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
