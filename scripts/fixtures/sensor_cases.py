#!/usr/bin/env python3
"""The shared cases of cad.sensors.ingest and cad.sensors.latest (docs/adr/0646 §3–§5; plan/gis.md CBS-33).

    python3 scripts/fixtures/sensor_cases.py           # writes fixtures/commands/v1/cad.sensors.{ingest,latest}.json
    python3 scripts/fixtures/sensor_cases.py --check   # compares

No KentOS code is used. A moment is read and written by scripts/fixtures/temporal_cases.py (ADR 0210 §3, written from
the ADR); a field's value is checked by scripts/fixtures/layer_field_cases.py (ADR 0199); the assets' order is that
script's natural order. The rules written here are ADR 0646's: a duplicate is an asset's observation at a moment the
asset's current versions (no recorded end) or the batch's earlier rows already hold; a late one is earlier than the
latest of those; `first` keeps the old, `last` the new (a layer with a recorded end ends the old version at the receipt,
one without changes the old in place), `all` both. The latest of an asset is its observation at the latest moment no
later than the one asked, among the versions known then (or the current ones), a tie to the later receipt, then to
the later in the drawing.

The SensorThings files (fixtures/sensors/v1) are written here from the OGC SensorThings API 1.1 part 1's
Observations answer: `value`, `@iot.nextLink`, `phenomenonTime` (an instant or an interval `a/b`), `result`,
`resultQuality`.
"""
import json
import sys
from functools import cmp_to_key
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layer_field_cases as lf  # noqa: E402
import temporal_cases as tc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILES = ROOT / "fixtures/sensors/v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
STEP = "Sensör gözlemleri"
MAX_OBSERVATIONS = 100_000

OLCUM_TIME = {"start": "olcum_zamani", "key": "sensor", "recordedStart": "alim", "recordedEnd": "alim_bitis"}
OLCUM_FIELDS = [
    {"name": "sensor", "kind": "text"},
    {"name": "olcum_zamani", "kind": "text"},
    {"name": "alim", "kind": "text"},
    {"name": "alim_bitis", "kind": "text"},
    {"name": "deger", "alias": "Değer", "kind": "decimal", "min": "-50", "max": "100"},
    {"name": "kalite", "alias": "Kalite", "kind": "text",
     "values": [{"code": "iyi", "label": "İyi"}, {"code": "supheli", "label": "Şüpheli"}]},
    {"name": "gec", "kind": "text"},
]
SAYAC_TIME = {"start": "an", "key": "sayac_no"}
DUZ_TIME = {"start": "an", "key": "no", "recordedStart": "alim"}
TIMES = {"olcum": OLCUM_TIME, "sayac": SAYAC_TIME, "duz": DUZ_TIME, "kilitli": SAYAC_TIME}
FIELDS = {"olcum": OLCUM_FIELDS}
NAMES = {"olcum": "Ölçüm", "sayac": "Sayaç", "duz": "Düz", "kilitli": "Kilitli", "plan": "Plan",
         "zamansiz": "Zamansız", "grup": "Grup"}


def node(id_, type_="layer", children=None, **extra):
    return {"id": id_, "name": NAMES.get(id_, id_), "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": children or [], **extra}


# (id, layer, attrs): the stored observations.
STORED = [
    (1, "olcum", {"sensor": "S1", "olcum_zamani": "2024-06-01T10:00:00Z", "deger": "12.5", "kalite": "iyi",
                  "alim": "2024-06-01T10:01:00Z"}),
    (2, "olcum", {"sensor": "S1", "olcum_zamani": "2024-06-01T10:10:00Z", "deger": "13", "alim": "2024-06-01T10:11:00Z"}),
    # A version superseded at 10:30 by the one after it.
    (3, "olcum", {"sensor": "S1", "olcum_zamani": "2024-06-01T10:05:00Z", "deger": "99", "kalite": "supheli",
                  "alim": "2024-06-01T10:06:00Z", "alim_bitis": "2024-06-01T10:30:00Z"}),
    (4, "olcum", {"sensor": "S1", "olcum_zamani": "2024-06-01T10:05:00Z", "deger": "12.8", "kalite": "iyi",
                  "alim": "2024-06-01T10:30:00Z"}),
    (5, "olcum", {"sensor": "S2", "olcum_zamani": "2024-06-01T09:00:00Z", "deger": "7", "alim": "2024-06-01T09:02:00Z"}),
    # A zone: 02:00 in Türkiye on 1 June is 23:00 UTC on 31 May.
    (6, "olcum", {"sensor": "S10", "olcum_zamani": "2024-06-01T02:00:00+03:00", "deger": "3", "alim": "2024-06-01"}),
    (7, "olcum", {"sensor": "S3", "olcum_zamani": "dün", "deger": "1", "alim": "2024-06-01"}),
    (8, "olcum", {"sensor": " ", "olcum_zamani": "2024-06-01T08:00:00Z", "deger": "2", "alim": "2024-06-01"}),
    # Two current versions of one moment: the later receipt is the latest.
    (9, "olcum", {"sensor": "S20", "olcum_zamani": "2024-06-01T08:00:00Z", "deger": "21", "alim": "2024-06-01T08:02:00Z"}),
    (10, "olcum", {"sensor": "S20", "olcum_zamani": "2024-06-01T08:00:00Z", "deger": "20", "alim": "2024-06-01T08:01:00Z"}),
    # No value.
    (11, "olcum", {"sensor": "S30", "olcum_zamani": "2024-06-01T07:00:00Z", "alim": "2024-06-01T07:01:00Z"}),
    (20, "sayac", {"sayac_no": "A", "an": "2024-06-01", "deger": "100"}),
    (21, "sayac", {"sayac_no": "A", "an": "2024-06-01T01:00:00Z", "deger": "105"}),
    (22, "sayac", {"sayac_no": "B", "an": "2024-06-01T00:30:00Z", "deger": "40"}),
    (30, "duz", {"no": "N1", "an": "2024-06-01", "deger": "5", "alim": "2024-06-01T00:00:10Z"}),
]


def drawing():
    layers = [
        node("olcum", time=OLCUM_TIME, fields=OLCUM_FIELDS),
        node("sayac", time=SAYAC_TIME),
        node("duz", time=DUZ_TIME),
        node("plan", time={"start": "baslangic"}),
        node("zamansiz"),
        {**node("kilitli", time=SAYAC_TIME), "locked": True},
        node("grup", type_="group", children=[node("ic")]),
    ]
    ents = [{"kind": "record", "id": i, "layerId": layer, "attrs": attrs} for i, layer, attrs in STORED]
    return {"format": "kentos.document", "version": 1, "name": "Sensörler",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 500000.0, "y": 4400000.0}, "layers": layers, "activeLayer": "olcum", "entities": ents,
            "styles": {"items": [], "categories": []}}


# ── Values ────────────────────────────────────────────────────────────


def utc(t):
    w = tc.write(t, False)
    return w + "Z" if "T" in w else w


def canonical(v):
    return json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def text_of(v):
    """§4: a result or a quality as written: a number its shortest text, text as is, a boolean true/false, an
    object or a list its canonical JSON; null none."""
    if v is None:
        return None
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, (int, float)):
        return json.dumps(v)
    if isinstance(v, str):
        return v
    return canonical(v)


def moment_message(text):
    return (f"“{text}” bir an değil: bir tarih (2024-06-01 ya da 01.06.2024) ya da tarih ve saat (2024-06-01T14:30, "
            "bölgesiyle 2024-06-01T14:30+03:00) yazın.")


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


# ── The document the cases run on ─────────────────────────────────────


class Doc:
    def __init__(self):
        self.ents = {i: {"layer": layer, "attrs": dict(a)} for i, layer, a in STORED}
        self.order = [i for i, _, _ in STORED]

    def of(self, layer):
        return [(i, self.ents[i]["attrs"]) for i in self.order if self.ents[i]["layer"] == layer]


# ── cad.sensors.ingest ────────────────────────────────────────────────


def ingest(doc, inp, plan=False):
    """The answer of `cad.sensors.ingest` and the document's changes: (result, changes) with changes a list of
    ('add', attrs) and ('set', id, attrs); a refusal has none."""
    layer = inp["layer"]
    t = TIMES[layer]
    fields = {f["name"]: f for f in FIELDS.get(layer, [])}
    received = tc.read(inp["receivedAt"]) if "receivedAt" in inp else None
    rows = inp["rows"]
    dup, late_rule = inp.get("duplicates", "first"), inp.get("late", "accept")
    stored = {}  # (asset, t) -> [ids of current versions]
    latest = {}
    for i, attrs in doc.of(layer):
        a = (attrs.get(t["key"]) or "").strip()
        m = tc.read(attrs.get(t["start"]) or "")
        ended = "recordedEnd" in t and (attrs.get(t["recordedEnd"]) or "").strip() != ""
        if not a or not isinstance(m, int) or ended:
            continue
        stored.setdefault((a, m), []).append(i)
        latest[a] = max(latest.get(a, m), m)
    pending = {}  # (asset, t) -> index in adds
    adds, sets, replaced, rejected = [], {}, [], []
    skipped = late = 0

    def reject(k, asset, at, code, message):
        rejected.append({"index": k, "asset": asset, "at": at, "code": code, "message": message})

    for k, row in enumerate(rows):
        asset, at = (row.get("asset") or "").strip(), row.get("at") or ""
        if not asset:
            reject(k, row.get("asset") or "", at, "invalid_asset",
                   "Gözlemin varlığı (asset) boş; sensörün ya da sayacın kimliğini verin.")
            continue
        m = tc.read(at)
        if m == "empty":
            reject(k, asset, at, "invalid_moment",
                   "Gözlemin anı boş; olay zamanını verin (at; SensorThings'te phenomenonTime).")
            continue
        if not isinstance(m, int):
            reject(k, asset, at, "invalid_moment", moment_message(at))
            continue
        key = (asset, m)
        is_dup = key in pending or key in stored
        if is_dup and dup == "first":
            skipped += 1
            continue
        is_late = not is_dup and asset in latest and m < latest[asset]
        if is_late and late_rule == "reject":
            reject(k, asset, at, "late_observation",
                   f"{asset} için {utc(latest[asset])} anında gözlem var; {utc(m)} ondan önce (geç gelen gözlem).")
            continue
        attrs = {t["key"]: asset, t["start"]: utc(m)}
        if "recordedStart" in t:
            attrs[t["recordedStart"]] = utc(received)
        given = {}
        value, quality = text_of(row.get("value")), text_of(row.get("quality"))
        if value is not None:
            given[inp["valueField"]] = value
        if quality is not None and "qualityField" in inp:
            given[inp["qualityField"]] = quality
        if is_late and "lateField" in inp:
            given[inp["lateField"]] = "evet"
        bad = None
        for name in sorted(given):
            f = fields.get(name)
            if f is None:
                continue
            r = lf.check(f, given[name])
            if "error" in r:
                code = "attribute_required" if r["error"] == "required" else "invalid_attribute"
                bad = (code, r["message"])
                break
            given[name] = r["value"]
        if bad:
            reject(k, asset, at, *bad)
            continue
        attrs.update({n: v for n, v in given.items() if v != ""})
        late += is_late
        latest[asset] = max(latest.get(asset, m), m)
        if is_dup and dup == "last" and key in pending:
            adds[pending[key]] = attrs
            skipped += 1
            continue
        if is_dup and dup == "last" and key in stored:
            for i in stored[key]:
                if i not in replaced:
                    replaced.append(i)
                old = sets.get(i, dict(doc.ents[i]["attrs"]))
                if "recordedEnd" in t:
                    old[t["recordedEnd"]] = utc(received)
                else:
                    for name in (inp["valueField"], inp.get("qualityField")):
                        if name is None:
                            continue
                        if name in attrs:
                            old[name] = attrs[name]
                        else:
                            old.pop(name, None)
                    if "recordedStart" in t:
                        old[t["recordedStart"]] = attrs[t["recordedStart"]]
                    if "lateField" in inp and inp["lateField"] in attrs:
                        old[inp["lateField"]] = attrs[inp["lateField"]]
                sets[i] = old
            if "recordedEnd" not in t:
                continue
        pending[key] = len(adds)
        adds.append(attrs)
    out = {}
    if plan:
        out["adds"] = len(adds)
    else:
        out["added"] = ["$uid"] * len(adds)
    out.update({"replaced": [f"$uidOf:{i}" for i in replaced], "skipped": skipped, "late": late,
                "rejected": rejected, "pages": inp.get("pages_read", 0), "more": inp.get("more", False),
                "receivedAt": utc(received) if received is not None else "$any",
                "revision": "$current"})
    changes = [("add", a) for a in adds] + [("set", i, a) for i, a in sets.items()]
    return out, changes


def sta_rows(name):
    """§4: a SensorThings Observations answer's rows (the file's own, no other page)."""
    data = json.loads(sta_files()[name])
    items = data["value"] if isinstance(data, dict) else data
    rows = []
    for o in items:
        pt = o.get("phenomenonTime")
        at = pt.split("/")[0] if isinstance(pt, str) else ""
        rows.append({"at": at, "value": o.get("result"), "quality": o.get("resultQuality")})
    more = isinstance(data, dict) and "@iot.nextLink" in data
    return rows, more


def ingest_case(name, inp, *, op="execute", rows=None, file=None, undo=False):
    doc = Doc()
    full = dict(inp)
    if file is not None:
        file_rows, more = sta_rows(file)
        full["rows"] = [{**r, "asset": inp["asset"]} for r in file_rows]
        full["pages_read"], full["more"] = 1, more
    else:
        full["rows"] = inp["observations"]
    out, changes = ingest(doc, full, plan=(op == "plan"))
    written = op == "execute" and bool(changes)
    expect = {"revision": "changed" if written else "same"}
    if written:
        next_id = max(doc.order) + 1
        ents = {}
        for c in changes:
            if c[0] == "add":
                ents[str(next_id)] = {"kind": "record", "id": next_id, "layerId": inp["layer"], "attrs": c[1]}
                next_id += 1
            else:
                ents[str(c[1])] = {"kind": "record", "id": c[1], "layerId": inp["layer"], "attrs": c[2]}
        expect["entities"] = ents
    steps = [{"op": op, "input": inp, "result": done(out), "expect": expect}]
    if undo and written:
        steps.append({"op": "undo", "returns": STEP, "expect": {"revision": "changed"}})
    return {"name": name, "steps": steps}


def refused(name, inp, result, op="execute"):
    return {"name": name, "steps": [{"op": op, "input": inp, "result": result, "expect": {"revision": "same"}}]}


def obs(asset, at, value=None, quality=None):
    o = {"asset": asset, "at": at}
    if value is not None:
        o["value"] = value
    if quality is not None:
        o["quality"] = quality
    return o


R = "2024-06-01T10:31:00Z"
OLC = {"layer": "olcum", "valueField": "deger", "qualityField": "kalite", "receivedAt": R}


def ingest_cases():
    two = [obs("S1", "2024-06-01T10:20:00Z", 13.5, "İyi"), obs("S1", "2024-06-01T10:30:00Z", 13.75)]
    return [
        ingest_case("iki yeni gözlem: değer sayı, kalite etiketiyle", {**OLC, "observations": two}, undo=True),
        ingest_case("geç gelen gözlem kabul edilir ve işaretlenir",
                    {**OLC, "lateField": "gec", "observations": [obs("S1", "2024-06-01T09:50:00Z", 12)]}),
        ingest_case("geç gelen gözlem işaret alanı olmadan da sayılır",
                    {**OLC, "observations": [obs("S2", "2024-06-01T08:00:00Z", 6)]}),
        ingest_case("geç gelen gözlem reddedilir", {**OLC, "late": "reject",
                                                     "observations": [obs("S1", "2024-06-01T09:50:00Z", 12)]}),
        ingest_case("yinelenen gözlem: ilki kalır",
                    {**OLC, "observations": [obs("S1", "2024-06-01T10:10:00Z", 14)]}),
        ingest_case("yinelenen gözlem: sonuncusu geçer, eski sürümün kayıt bitişi yazılır",
                    {**OLC, "duplicates": "last", "observations": [obs("S1", "2024-06-01T10:10:00Z", 14)]}, undo=True),
        ingest_case("bölgeli yazılmış aynı an da yinelenendir",
                    {**OLC, "duplicates": "last", "observations": [obs("S1", "2024-06-01T13:10:00+03:00", 14)]}),
        ingest_case("yinelenen gözlem: hepsi yazılır",
                    {**OLC, "duplicates": "all", "observations": [obs("S1", "2024-06-01T10:10:00Z", 14)]}),
        ingest_case("partinin içinde yinelenen: ilki kalır",
                    {**OLC, "observations": [obs("S2", "2024-06-01T11:00:00Z", 1), obs("S2", "2024-06-01T11:00:00Z", 2)]}),
        ingest_case("partinin içinde yinelenen: sonuncusu kalır",
                    {**OLC, "duplicates": "last",
                     "observations": [obs("S2", "2024-06-01T11:00:00Z", 1), obs("S2", "2024-06-01T11:00:00Z", 2)]}),
        ingest_case("yerine yenisi geçmiş sürüm yinelenen sayılmaz, şimdiki sürüm sayılır",
                    {**OLC, "duplicates": "last", "observations": [obs("S1", "2024-06-01T10:05:00Z", 12.9, "Şüpheli")]}),
        ingest_case("kayıt bitişi tutmayan katmanda sonuncusu eskinin değerini yerinde değiştirir",
                    {"layer": "duz", "valueField": "deger", "receivedAt": R, "duplicates": "last",
                     "observations": [obs("N1", "2024-06-01", 6)]}, undo=True),
        ingest_case("kayıt zamanı tutmayan katman: alım anı yazılmaz",
                    {"layer": "sayac", "valueField": "deger", "receivedAt": R, "duplicates": "last",
                     "observations": [obs("A", "2024-06-01T01:00:00Z", 106), obs("B", "2024-06-01T02:00:00Z", 41)]}),
        ingest_case("alım anı verilmezse şimdi",
                    {"layer": "sayac", "valueField": "deger", "observations": [obs("C", "2024-06-01", "7")]}),
        ingest_case("yeni varlık, partide geç gelen",
                    {**OLC, "observations": [obs("S4", "2024-06-01T10:00:00Z", 1), obs("S4", "2024-06-01T09:00:00Z", 2)]}),
        ingest_case("satır retleri: boş varlık, okunmayan an, sayı olmayan değer, aralık dışı değer, listede olmayan kalite",
                    {**OLC, "observations": [obs("", "2024-06-01T11:00:00Z", 1), obs("S5", "dün", 1),
                                             obs("S5", "2024-06-01T11:00:00Z", "abc"),
                                             obs("S5", "2024-06-01T11:01:00Z", 150),
                                             obs("S5", "2024-06-01T11:02:00Z", 1, "bozuk"),
                                             obs("S5", "2024-06-01T11:03:00Z", "2,5")]}),
        ingest_case("bütün satırlar reddedilirse adım açılmaz",
                    {**OLC, "observations": [obs("", "2024-06-01T11:00:00Z", 1), obs("S5", "31.02.2024", 1)]}),
        ingest_case("bölgeli alım anı UTC yazılır",
                    {**OLC, "receivedAt": "2024-06-01T13:31:00+03:00",
                     "observations": [obs("S6", "2024-06-01T10:00:00Z", 4, "Şüpheli")]}),
        ingest_case("değeri olmayan gözlem", {**OLC, "observations": [obs("S7", "2024-06-01T10:00:00Z")]}),
        ingest_case("SensorThings dosyası: değer listesi, aralıklı an, boş sonuç, sonraki sayfa",
                    {**OLC, "asset": "S8", "file": "$fixture:sensors/v1/observations.json"}, file="observations.json"),
        ingest_case("SensorThings dosyası: gözlemlerin dizisi",
                    {"layer": "sayac", "valueField": "deger", "qualityField": "kalite", "receivedAt": R, "asset": "D",
                     "file": "$fixture:sensors/v1/list.json"}, file="list.json"),
        ingest_case("plan aynı sonucu verir, yazmaz", {**OLC, "duplicates": "last", "lateField": "gec",
                                                       "observations": [obs("S1", "2024-06-01T10:10:00Z", 14),
                                                                        obs("S1", "2024-06-01T09:00:00Z", 1),
                                                                        obs("S9", "2024-06-01T09:00:00Z", 1)]},
                    op="plan"),
        refused("validate yalnız denetler", {**OLC, "observations": two},
                {"status": "completed", "output": None, "warnings": []}, op="validate"),
        refused("geçersiz beklenen sürüm", {**OLC, "expectedRevision": "abc", "observations": two},
                failed("invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                       "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")),
        refused("çizim değişmiş", {**OLC, "expectedRevision": "999", "observations": two},
                {"status": "conflict", "error": {
                    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                    "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                    "revision": "$current"}}),
        refused("JSON olmayan dosya", {**OLC, "asset": "S8", "file": "$fixture:sensors/v1/not-json.json"},
                failed("import_unreadable", "not-json.json bir SensorThings gözlem listesi olarak okunamadı: JSON değil.",
                       "file")),
        refused("gözlem dizisi olmayan dosya", {**OLC, "asset": "S8", "file": "$fixture:sensors/v1/no-value.json"},
                failed("import_unreadable", "no-value.json bir SensorThings gözlem listesi olarak okunamadı: "
                       "gözlemlerin dizisi (value) yok.", "file")),
        refused("olmayan dosya", {**OLC, "asset": "S8", "file": "$fixture:sensors/v1/yok.json"},
                failed("file_not_found", "$fixture:sensors/v1/yok.json okunamadı: dosya yok ya da açılamıyor.", "file")),
        refused("göreli yol", {**OLC, "asset": "S8", "file": "gozlem.json"},
                failed("invalid_file_path", "“gozlem.json” mutlak bir dosya yolu değil; dosyanın tam yolunu verin "
                       "(/klasör/ad.json).", "file")),
        refused("dosyada varlık yok", {**OLC, "file": "$fixture:sensors/v1/list.json"},
                failed("invalid_input", "Dosyanın ve adresin gözlemleri tek bir varlığındır; varlığı (asset) verin.",
                       "asset")),
        refused("kaynak yok", {**OLC}, failed("invalid_source", "Gözlemleri tek bir kaynaktan verin: observations, "
                                              "file ya da url.", "observations")),
        refused("iki kaynak", {**OLC, "asset": "S8", "observations": two, "file": "$fixture:sensors/v1/list.json"},
                failed("invalid_source", "Gözlemleri tek bir kaynaktan verin: observations, file ya da url.",
                       "observations")),
        refused("boş gözlem listesi", {**OLC, "observations": []},
                failed("invalid_count", f"Gözlem sayısı 1 ile {MAX_OBSERVATIONS} arasında olmalı.", "observations")),
        refused("değer alanı boş", {**OLC, "valueField": " ", "observations": two},
                failed("invalid_input", "Değer alanının adını (valueField) verin.", "valueField")),
        refused("sayfa sayısı dışarıda", {**OLC, "url": "https://example.org/v1.1/Datastreams(1)/Observations",
                                          "asset": "S8", "pages": 0},
                failed("invalid_input", "Sayfa sayısı (pages) 1 ile 100 arasında olmalı.", "pages")),
        refused("okunmayan alım anı", {**OLC, "receivedAt": "bugün", "observations": two},
                failed("invalid_moment", moment_message("bugün"), "receivedAt")),
        refused("zaman ayarı olmayan katman", {"layer": "zamansiz", "valueField": "deger", "observations": two},
                failed("not_temporal", "“Zamansız” katmanının zaman ayarı yok; önce Zaman ayarları ile başlangıç "
                       "alanını seçin.", "layer")),
        refused("kimlik alanı olmayan katman", {"layer": "plan", "valueField": "deger", "observations": two},
                failed("no_asset_field", "“Plan” katmanının zaman ayarında kimlik alanı yok; gözlemler varlığa göre "
                       "tutulur. Zaman ayarları'nda kimlik alanını seçin.", "layer")),
        refused("olmayan katman", {"layer": "yok", "valueField": "deger", "observations": two},
                failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                       "layer")),
        refused("kilitli katman", {"layer": "kilitli", "valueField": "deger", "observations": two},
                failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir "
                       "katmanı etkinleştirin.", "layer")),
        refused("grup", {"layer": "grup", "valueField": "deger", "observations": two},
                failed("not_a_layer", "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir "
                       "katman seçin.", "layer")),
    ]


def ingest_file():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sensors.ingest", "commandVersion": 1,
            "title": "Sensör gözlemlerini al",
            "note": ("ADR 0646 §3–§4. Bağımsız başvuru scripts/fixtures/sensor_cases.py: anlar temporal_cases.py'den, "
                     "alanların kuralı layer_field_cases.py'den, yinelenen ve geç gözlemin kuralı ADR 0646'dan; "
                     "SensorThings dosyaları fixtures/sensors/v1. KentOS kodu yok."),
            "setup": drawing(), "cases": ingest_cases()}


# ── cad.sensors.latest ────────────────────────────────────────────────


def latest(inp):
    layer = inp["layer"]
    t = TIMES[layer]
    at = tc.read(inp["at"])
    known = tc.read(inp["knownAt"]) if "knownAt" in inp else None
    best, unreadable = {}, 0
    for pos, (i, attrs) in enumerate(Doc().of(layer)):
        a = (attrs.get(t["key"]) or "").strip()
        m = tc.read(attrs.get(t["start"]) or "")
        if not a or not isinstance(m, int):
            unreadable += 1
            continue
        rs = tc.read(attrs.get(t["recordedStart"]) or "") if "recordedStart" in t else None
        if known is not None:
            re_ = tc.read(attrs.get(t["recordedEnd"]) or "") if "recordedEnd" in t else "empty"
            if not isinstance(rs, int) or re_ == "unreadable":
                continue
            if not (rs <= known < (tc.INF if re_ == "empty" else re_)):
                continue
        elif "recordedEnd" in t and (attrs.get(t["recordedEnd"]) or "").strip():
            continue
        if m > at:
            continue
        rank = (m, rs if isinstance(rs, int) else -tc.INF, pos)
        if a not in best or rank > best[a][0]:
            best[a] = (rank, i, attrs, m)
    stale_after = inp.get("staleAfter")
    assets, stale = [], 0
    for a in sorted(best, key=cmp_to_key(lf.natural_cmp)):
        _, i, attrs, m = best[a]
        item = {"asset": a, "uid": f"$uidOf:{i}", "observedAt": utc(m)}
        if (attrs.get(inp["valueField"]) or "") != "":
            item["value"] = attrs[inp["valueField"]]
        if "qualityField" in inp and (attrs.get(inp["qualityField"]) or "") != "":
            item["quality"] = attrs[inp["qualityField"]]
        age = (at - m) / 1000
        item["ageSeconds"] = float(age)
        is_stale = stale_after is not None and age > stale_after
        item["stale"] = is_stale
        stale += is_stale
        assets.append(item)
    missing = []
    for e in inp.get("expectedAssets", []):
        if e not in best and e not in missing:
            missing.append(e)
    out = {"layer": layer, "at": utc(at)}
    if known is not None:
        out["knownAt"] = utc(known)
    out.update({"assets": assets, "stale": stale, "missing": missing, "unreadable": unreadable,
                "revision": "$current"})
    return out


def latest_case(name, inp, op="execute"):
    return refused(name, inp, done(latest(inp)), op)


OL = {"layer": "olcum", "valueField": "deger", "qualityField": "kalite"}


def latest_cases():
    return [
        latest_case("öğlen her varlığın son gözlemi, doğal sırayla", {**OL, "at": "2024-06-01T12:00:00Z"}),
        latest_case("10:07'de: yerine yenisi geçmiş sürüm alınmaz", {**OL, "at": "2024-06-01T10:07:00Z"}),
        latest_case("10:07'de, 10:20'de bilindiği gibi: eski sürüm", {**OL, "at": "2024-06-01T10:07:00Z",
                                                                     "knownAt": "2024-06-01T10:20:00Z"}),
        latest_case("bilinen gün sürümün yerine başkasının geçtiği an: yenisi",
                    {**OL, "at": "2024-06-01T10:07:00Z", "knownAt": "2024-06-01T10:30:00Z"}),
        latest_case("bilinen gün kayıttan önce: hiç gözlem yok", {**OL, "at": "2024-06-01T12:00:00Z",
                                                                  "knownAt": "2024-05-01"}),
        latest_case("eskime süresi: bir saatten eski olanlar eskimiş", {**OL, "at": "2024-06-01T10:40:00Z",
                                                                        "staleAfter": 3600}),
        latest_case("eskime süresine eşit yaş eskimiş sayılmaz", {**OL, "at": "2024-06-01T11:10:00Z",
                                                                  "staleAfter": 3600}),
        latest_case("beklenen ve gelmeyen varlıklar", {**OL, "at": "2024-06-01T12:00:00Z",
                                                       "expectedAssets": ["S1", "S7", "S2", "S7", "S40"]}),
        latest_case("hiçbir gözlemden önce", {**OL, "at": "2024-05-01", "expectedAssets": ["S1"]}),
        latest_case("aynı anda iki şimdiki sürüm: alımı geç olan", {**OL, "at": "2024-06-01T08:30:00Z"}),
        latest_case("değeri olmayan gözlem", {**OL, "at": "2024-06-01T07:30:00Z"}),
        latest_case("bölgeli an", {**OL, "at": "2024-06-01T02:00:00+03:00"}),
        latest_case("kalite alanı verilmezse kalite yazılmaz", {"layer": "olcum", "valueField": "deger",
                                                               "at": "2024-06-01T12:00:00Z"}),
        latest_case("kayıt zamanı tutmayan katman", {"layer": "sayac", "valueField": "deger",
                                                     "at": "2024-06-01T00:45:00Z", "staleAfter": 600}),
        latest_case("gün.ay.yıl biçimi", {"layer": "sayac", "valueField": "deger", "at": "02.06.2024"}),
        latest_case("kayıt bitişi tutmayan katman: kayıt başlangıcıyla bilinen",
                    {"layer": "duz", "valueField": "deger", "at": "2024-06-02", "knownAt": "2024-06-01T00:00:05Z"}),
        latest_case("plan aynı cevabı verir", {**OL, "at": "2024-06-01T12:00:00Z", "staleAfter": 7200}, op="plan"),
        refused("validate yalnız denetler", {**OL, "at": "2024-06-01T12:00:00Z"},
                {"status": "completed", "output": None, "warnings": []}, op="validate"),
        refused("sorulan an verilmezse şimdi", {"layer": "sayac", "valueField": "deger"}, done(
            {"layer": "sayac", "at": "$any", "assets": "$any", "stale": 0, "missing": [], "unreadable": 0,
             "revision": "$current"})),
        refused("kayıt zamanı tutmayan katmanda bilinen gün", {"layer": "sayac", "valueField": "deger",
                                                              "at": "2024-06-02", "knownAt": "2024-06-01"},
                failed("not_recorded", "“Sayaç” katmanı sürümlerinin kayıt zamanını tutmuyor; Zaman ayarları'nda "
                       "kayıt başlangıcı alanını seçin.", "knownAt")),
        refused("okunmayan an", {**OL, "at": "geçen hafta"}, failed("invalid_moment", moment_message("geçen hafta"),
                                                                     "at")),
        refused("okunmayan bilinen gün", {**OL, "at": "2024-06-01", "knownAt": "31.02.2024"},
                failed("invalid_moment", moment_message("31.02.2024"), "knownAt")),
        refused("eksi eskime süresi", {**OL, "at": "2024-06-01", "staleAfter": -1},
                failed("invalid_input", "Eskime süresi (staleAfter) sıfır ya da pozitif bir saniye olmalı.",
                       "staleAfter")),
        refused("değer alanı boş", {"layer": "olcum", "valueField": "", "at": "2024-06-01"},
                failed("invalid_input", "Değer alanının adını (valueField) verin.", "valueField")),
        refused("zaman ayarı olmayan katman", {"layer": "zamansiz", "valueField": "deger", "at": "2024-06-01"},
                failed("not_temporal", "“Zamansız” katmanının zaman ayarı yok; önce Zaman ayarları ile başlangıç "
                       "alanını seçin.", "layer")),
        refused("kimlik alanı olmayan katman", {"layer": "plan", "valueField": "deger", "at": "2024-06-01"},
                failed("no_asset_field", "“Plan” katmanının zaman ayarında kimlik alanı yok; gözlemler varlığa göre "
                       "tutulur. Zaman ayarları'nda kimlik alanını seçin.", "layer")),
        refused("grup", {"layer": "grup", "valueField": "deger", "at": "2024-06-01"},
                failed("not_a_layer", "“Grup” bir katman grubu; zaman katmanın ayarıdır. Grubun içinden bir katman "
                       "seçin.", "layer")),
        refused("olmayan katman", {"layer": "yok", "valueField": "deger", "at": "2024-06-01"},
                failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                       "layer")),
    ]


def latest_file():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sensors.latest", "commandVersion": 1,
            "title": "Sensörlerin son gözlemi",
            "note": ("ADR 0646 §5. Bağımsız başvuru scripts/fixtures/sensor_cases.py: anlar temporal_cases.py'den, "
                     "doğal sıra layer_field_cases.py'den, son gözlemin kuralı ADR 0646'dan. KentOS kodu yok."),
            "setup": drawing(), "cases": latest_cases()}


# ── The SensorThings files ────────────────────────────────────────────


def sta_files():
    obs_page = {
        "@iot.count": 5,
        "@iot.nextLink": "https://example.org/v1.1/Datastreams(8)/Observations?$skip=4",
        "value": [
            {"@iot.id": 101, "phenomenonTime": "2024-06-01T11:00:00Z", "resultTime": "2024-06-01T11:00:05Z",
             "result": 21.5, "resultQuality": "İyi"},
            {"@iot.id": 102, "phenomenonTime": "2024-06-01T11:10:00Z/2024-06-01T11:15:00Z", "result": 22},
            {"@iot.id": 103, "phenomenonTime": "2024-06-01T11:20:00Z", "result": None},
            {"@iot.id": 104, "phenomenonTime": "2024-06-01T11:05:00Z", "result": "23.25",
             "resultQuality": "Şüpheli"},
        ],
    }
    lst = [
        {"phenomenonTime": "2024-06-01T05:00:00Z", "result": 3, "resultQuality": {"kod": 1}},
        {"phenomenonTime": "2024-06-01T06:00:00Z", "result": True},
        {"phenomenonTime": "2024-06-01T07:00:00Z", "result": [1, 2]},
        {"result": 4},
    ]
    return {"observations.json": json.dumps(obs_page, ensure_ascii=False, indent=1) + "\n",
            "list.json": json.dumps(lst, ensure_ascii=False, indent=1) + "\n",
            "not-json.json": "{ bu JSON değil\n",
            "no-value.json": json.dumps({"@iot.count": 0}) + "\n"}


def outputs():
    out = {OUT / "cad.sensors.ingest.json": json.dumps(ingest_file(), ensure_ascii=False, indent=1) + "\n",
           OUT / "cad.sensors.latest.json": json.dumps(latest_file(), ensure_ascii=False, indent=1) + "\n"}
    for name, text in sta_files().items():
        out[FILES / name] = text
    return out


def main():
    files = outputs()
    if "--check" in sys.argv:
        stale = [p for p, t in files.items() if not p.exists() or p.read_text("utf-8") != t]
        if stale:
            for p in stale:
                print(f"{p.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"Sensör durumları tutarlı: alma {len(ingest_cases())}, son gözlem {len(latest_cases())} durum")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    # The files first: the cases read them.
    for p, t in sorted(files.items(), key=lambda kv: "cad." in kv[0].name):
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
