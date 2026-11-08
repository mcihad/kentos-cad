#!/usr/bin/env python3
"""The shared cases of `cad.readjustment.sheets` (Ayırma çapı paftaları; plan/map-cad.md MAP-11, docs/adr/2680),
without KentOS code.

    python3 scripts/fixtures/readjustment_sheets_cases.py           # writes fixtures/commands/v1/cad.readjustment.sheets.json
    python3 scripts/fixtures/readjustment_sheets_cases.py --check   # writes nothing; compares

The rules, by hand (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13, as the ADR reads it):

- the scope, its parts and their numbered points are `readjustment_scope_cases.py`'s exact reference (the same
  drawing); a sheet for every parcel the scope finds partly inside, in its order; none: refused; a partial parcel
  without Ada or Parsel, or with another's: refused (the sheet is named by it);
- a sheet's id is `ayirma-capi-k` (k from 1, the first not taken), its name “Ayırma çapı ‹Ada/Parsel›”; the template's
  map frame (KentOS's separation sheet, read from its file) less its label band holds the parcel's box widened by 5%
  on each side: the scale is the smallest of the paper table's standard scales (`papers.json`) not under the box's
  need (metres over paper metres, the larger of the two sides, rounded up); the centre is the box's middle; a scale
  given is kept and the map centred;
- the table of areas: the record as written (its comma a point), the computed area, the tolerance, the parts inside
  and outside, each value's shortest decimal form rounded once to the places by the mode; the difference of the
  written computed area and the record, exactly (the record's places or the policy's, the more); what the part outside
  keeps (the scope's registered − inside, m. 31/2); “—” for what is not there; the table of parts: Giren / Girmeyen (numbered on a side with more than one), the outer ring's
  numbers closed by its first, each hole's after it, the area as above;
- the book then lists the sheets (A4 portrait, the template's nine items, the map and its scale).
"""

import copy
import json
import math
import sys
from decimal import Decimal
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import readjustment_scope_cases as rs  # noqa: E402

ROOT = rs.ROOT
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.readjustment.sheets.json"
TEMPLATE = json.loads((ROOT / "crates" / "shared" / "sheet" / "templates" / "ayirma-capi.json").read_text("utf-8"))
SCALES = json.loads((ROOT / "crates" / "shared" / "sheet" / "data" / "papers.json").read_text("utf-8"))["scales"]
W = 1e-6
STEP = "Ayırma çapı paftaları"
WORD = "Ayırma çapı"
GROUP = "Ayırma çapı noktaları"
MARGIN = 5


def item(template, id_):
    return next(i for i in template["sheet"]["items"] if i["id"] == id_)


MAP_AT = [i["id"] for i in TEMPLATE["sheet"]["items"]].index("harita")
MAP_NAME = item(TEMPLATE, "harita")["name"]


def fit_scale(box):
    """The smallest standard scale the box fits at with its margin (rotation 0: east over the width, north over the
    height), as a host fits a map to objects."""
    m = item(TEMPLATE, "harita")
    band, pad = m["kind"].get("labelBand", 0), m.get("padding", 0)
    width = m["frame"]["width"] - 2 * pad - 2 * band
    height = m["frame"]["height"] - 2 * pad - 2 * band
    w, h = box[2] - box[0], box[3] - box[1]
    need = max((w * 1.0 + h * 0.0) * 1e6 / float(width), (w * 0.0 + h * 1.0) * 1e6 / float(height))
    need *= 1.0 + 2.0 * float(MARGIN) / 100.0
    scale = max(int(math.ceil(need)), 1)
    return next((s for s in SCALES if s >= scale), scale)


def box_of(entity):
    xs = [p["x"] for p in entity["pts"]]
    ys = [p["y"] for p in entity["pts"]]
    return [min(xs), min(ys), max(xs), max(ys)]


def written(v, scale, mode):
    """A computed value at the places by the mode: its shortest decimal form, rounded once."""
    return rs.written(rs.round_to(F(Decimal(repr(v))), scale, mode), scale)


def names(parts):
    on = {s: sum(1 for p in parts if p["side"] == s) for s in ("inside", "outside")}
    seen = {"inside": 0, "outside": 0}
    out = []
    for p in parts:
        seen[p["side"]] += 1
        word = "Giren" if p["side"] == "inside" else "Girmeyen"
        out.append(f"{word} {seen[p['side']]}" if on[p["side"]] > 1 else word)
    return out


def ring_text(r):
    n = [str(x) for x in r["numbers"]]
    return "-".join(n + n[:1])


def boundary(part):
    text = ring_text(part["rings"][0])
    for hole in part["rings"][1:]:
        text += f" (delik {ring_text(hole)})"
    return text


def areas(p, scale, mode):
    rec = p.get("recordedText")
    point = rec.replace(",", ".") if rec is not None else None
    readable = point is not None and rs.plain(point)
    registered = "—" if rec is None else point if readable else rec
    calc = written(p["calculated"], scale, mode)
    diff = rs.written(F(calc) - F(point), max(rs.places(point), scale)) if readable else "—"
    w = lambda v: written(v, scale, mode)  # noqa: E731
    return [["Tescilli yüzölçümü", registered], ["Hesaplanan yüzölçümü", calc], ["Yüzölçümü farkı", diff],
            ["Yanılma sınırı", w(p["limit"])], ["Giren kısım", w(p["inside"])], ["Girmeyen kısım", w(p["outside"])],
            ["Girmeyen kısma kalan (tescilli − giren)", p.get("outsideRecorded", "—")]]


def part_rows(p, scale, mode):
    return [[n, boundary(part), written(part["area"], scale, mode)] for n, part in zip(names(p["parts"]), p["parts"])]


def sheets(entities, chosen, kw, given_scale=None, taken=()):
    """The sheets' answer rows and the book's listing for the partial parcels of the scope."""
    scope_kw = {k: kw[k] for k in ("scale", "mode", "tolerance", "field") if k in kw}
    scope = rs.scope(entities, 0, chosen, **scope_kw)
    scale, mode = kw.get("scale", 2), kw.get("mode", "half_even")
    rows, listing = [], []
    k = 1
    taken = set(taken)
    by_uid = {rs.uid(i + 1): e for i, e in enumerate(entities)}
    for p in scope["parcels"]:
        if p["kind"] != "partial":
            continue
        while f"ayirma-capi-{k}" in taken:
            k += 1
        sid = f"ayirma-capi-{k}"
        taken.add(sid)
        box = box_of(by_uid[p["uid"]])
        centre = [box[0] + (box[2] - box[0]) / 2.0, box[1] + (box[3] - box[1]) / 2.0]
        s = given_scale if given_scale is not None else fit_scale(box)
        name = f"{WORD} {p['parcel']}"
        rows.append({"sheet": sid, "name": name, "uid": p["uid"], "parcel": p["parcel"], "scale": s, "center": centre,
                     "points": len(p["points"]), "areas": areas(p, scale, mode), "parts": part_rows(p, scale, mode)})
        listing.append({"id": sid, "name": name, "width": 210.0, "height": 297.0,
                        "items": len(TEMPLATE["sheet"]["items"]),
                        "maps": [{"item": f"{sid}-item-{MAP_AT}", "name": MAP_NAME, "scale": float(s)}]})
    return rows, listing


def scope_input(chosen=None, **kw):
    inp = {"boundary": rs.uid(1), "scale": kw.get("scale", 2), "rounding": kw.get("mode", "half_even")}
    if chosen is not None:
        inp["uids"] = [rs.uid(i + 1) for i in chosen]
    if "tolerance" in kw:
        inp["tolerance"] = kw["tolerance"]
    if "field" in kw:
        inp["field"] = kw["field"]
    return inp


def made(rows, plan=False):
    if plan:
        return {"sheets": rows, "uids": [], "revision": "$current"}
    return {"group": "$any", "sheets": [{**r, "layer": "$any"} for r in rows], "uids": "$any", "revision": "$current"}


WRITTEN = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}


def ok(name, chosen, entities=rs.MAIN, setup_=None, op="execute", options=None, **kw):
    given = (options or {}).get("scale")
    rows, listing = sheets(entities, chosen, kw, given)
    inp = {"scope": scope_input(chosen, **kw)}
    if options:
        inp["options"] = options
    step = {"op": op, "input": inp, "within": W}
    if op == "validate":
        step["result"] = {"status": "completed", "output": None, "warnings": []}
        step["expect"] = {"revision": "same", "sheets": []}
    else:
        step["result"] = {"status": "completed", "output": made(rows, op == "plan"), "warnings": []}
        step["expect"] = {"revision": "same", "sheets": []} if op == "plan" else {**WRITTEN, "sheets": listing}
    case = {"name": name, "steps": [step]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, inp, code, message, path, setup_=None, status="failed"):
    err = {"code": code, "message": message, "path": path}
    if status == "conflict":
        err["revision"] = "$current"
    case = {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": status, "error": err},
                                     "expect": {"revision": "same", "sheets": []}}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    P = rs.P_
    three = [P(3), P(6), P(10)]
    rows, listing = sheets(rs.MAIN, three, {})
    twins = [rs.BOUNDARY, rs.area_entity(rs.rect(60, 0, 90, 20), "parsel", rs.ident(3, "598.40")),
             rs.area_entity(rs.rect(60, 20, 90, 40), "parsel", rs.ident(3, "600.00"))]
    no_map = copy.deepcopy(TEMPLATE)
    no_map["meta"]["id"] = "user:haritasiz-cap"
    item(no_map, "harita")["id"] = "kroki"
    for it in no_map["sheet"]["items"]:
        if it["kind"].get("map") == "harita":
            it["kind"]["map"] = "kroki"
    base = {"scope": scope_input([P(3)])}
    return [
        ok("kısmen giren üç parsel: üç ayırma çapı paftası, noktalar parsellerin katmanlarında", three),
        ok("kısmen giren parsel: yüzölçümleri ve kısımlar politikanın basamağıyla", [P(3)]),
        ok("kaydı okunmayan parsel: tescilli yazıldığı gibi, fark yok", [P(6)]),
        ok("iki parçası dışarıda kalan parsel: Girmeyen 1 ve 2 aynı paftada (m. 13/3)", [P(10)]),
        ok("köşesi sınırın dışında: girmeyen kısım L biçiminde, sınırı ortak noktalarla", None, rs.CORNER,
           rs.setup(rs.CORNER)),
        ok("sınır parselin içinde: girmeyen kısım delikli, deliği giren kısmın noktaları", None, rs.HOLED,
           rs.setup(rs.HOLED)),
        ok("ölçek verilince harita parseli o ölçekte ortalar", [P(3)], options={"scale": 1000}),
        ok("şablonun öbür soruları cevaplanır", [P(3)],
           options={"values": [{"name": "pafta_no", "value": "G23-c-11-b"}, {"name": "kontrol_eden", "value": "Ayşe Kaya"}]}),
        ok("sıfır basamak: tablolar tam sayıyla", [P(3), P(10)], scale=0),
        ok("sıfırdan uzağa yuvarlama: 1000/3 m² → 333.34", [P(3)], mode="away_from_zero"),
        ok("tolerans 3 mm: 4 mm'lik parça sayılır, parselin çapı olur", [P(9)], tolerance=0.003),
        ok("tamamı giren ve dışarıdaki parseller verilince yalnız kısmen girenin paftası", [P(1), P(3), P(7)]),
        ok("CBS projesinde de", [P(3)], setup_=rs.setup(rs.MAIN, "gis")),
        ok("plan aynı paftaları verir, yazmaz", three, op="plan"),
        ok("doğrulama", three, op="validate"),
        {"name": "geri alma: noktalar, katmanlar ve paftalar tek adımda gider",
         "steps": [{"op": "execute", "input": {"scope": scope_input(three)}, "within": W,
                    "result": {"status": "completed", "output": made(rows), "warnings": []},
                    "expect": {**WRITTEN, "sheets": listing}},
                   {"op": "undo", "returns": STEP, "expect": {"canUndo": False, "canRedo": True, "sheets": []}},
                   {"op": "redo", "returns": STEP, "expect": {"canUndo": True, "canRedo": False, "sheets": listing}}]},
        {"name": "yeniden yapınca var olan pafta söylenir, hiçbir şey yazılmaz",
         "steps": [{"op": "execute", "input": {"scope": scope_input([P(3)])}, "within": W,
                    "result": {"status": "completed", "output": made(sheets(rs.MAIN, [P(3)], {})[0]), "warnings": []},
                    "expect": {**WRITTEN, "sheets": sheets(rs.MAIN, [P(3)], {})[1]}},
                   {"op": "execute", "input": {"scope": scope_input([P(3)])},
                    "result": {"status": "failed", "error": {
                        "code": "separation_sheet_exists",
                        "message": f"“{WORD} 101/3” paftası zaten var; yeniden yapmak için önce o paftayı ve "
                                   f"“{GROUP} › 101/3” katmanını silin.", "path": "scope"}},
                    "expect": {"revision": "same", "sheets": sheets(rs.MAIN, [P(3)], {})[1]}}]},
        refused("kısmen giren parsel yok", {"scope": scope_input([P(1), P(2)])}, "no_partial_parcels",
                "Düzenleme sınırına kısmen giren parsel yok; ayırma çapı yalnız bir kısmı düzenleme sahasında kalan "
                "parsel için düzenlenir (Yönetmelik m. 13/1).", "scope"),
        refused("Ada'sı olmayan kısmen giren parsel", {"scope": scope_input([P(11)])}, "no_identity",
                f"“{rs.uid(P(11) + 1)}” parselinin Ada'sı ya da Parsel'i yok; ayırma çapı paftası parselin kimliğiyle "
                "adlandırılır.", "scope"),
        refused("aynı Ada/Parsel iki kısmen giren parselde", {"scope": scope_input()}, "duplicate_parcel",
                "“101/3” iki parselin kimliği; ayırma çapı paftası parselin kimliğiyle adlandırılır, önce "
                "kimliklerden birini düzeltin.", "scope", rs.setup(twins)),
        refused("bilinmeyen şablon", {**base, "source": {"type": "system", "id": "sys:yok"}}, "unknown_template",
                "“sys:yok” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin.", "source.id"),
        refused("haritası olmayan şablon", {**base, "source": {"type": "custom", "template": no_map}}, "not_fixed_map",
                "Şablonda “harita” kimlikli sabit görünüm penceresi yok; ayırma çapının haritası ona yerleşir.",
                "source"),
        refused("pafta adı verilmez", {**base, "options": {"name": "Çap"}}, "invalid_options",
                f"Ayırma çapı paftalarının adları parsellerinden gelir (“{WORD} 101/10”); options.name verilmez.",
                "options.name"),
        refused("merkez verilmez", {**base, "options": {"center": {"x": 487075.0, "y": 4420010.0}}}, "invalid_options",
                "Her paftanın haritası kendi parselini ortalar; options.center verilmez.", "options.center"),
        refused("parselin özniteliği cevap olarak verilmez", {**base, "options": {"values": [{"name": "ada", "value": "5"}]}},
                "invalid_options", "“ada” her paftada parselin kendi özniteliğinden gelir; cevap olarak verilmez.",
                "options.values[0].name"),
        refused("kapsamın reddi yeriyle: basamak 7", {"scope": {**scope_input([P(3)]), "scale": 7}}, "invalid_input",
                "Basamak 7 en çok 6 olabilir.", "scope.scale"),
        refused("kapsamın reddi yeriyle: sınır çizimde yok", {"scope": {**scope_input([P(3)]), "boundary": rs.uid(99)}},
                "entity_not_found", f"“{rs.uid(99)}” kimlikli nesne çizimde yok.", "scope.boundary"),
        refused("geçersiz sürüm", {**base, "expectedRevision": "01"}, "invalid_revision", "$any", "expectedRevision"),
        refused("eski sürüm: hiçbir şey yazılmaz", {**base, "expectedRevision": "9999"}, "revision_conflict", "$any",
                "expectedRevision", status="conflict"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.readjustment.sheets", "commandVersion": 1,
           "title": "Ayırma çapı paftaları",
           "note": "ADR 2680. Bağımsız başvuru scripts/fixtures/readjustment_sheets_cases.py: kapsam, parçalar ve "
                   "numaralı noktalar readjustment_scope_cases.py'nin kesirli başvurusundan, haritanın ölçeği şablonun "
                   "dosyasındaki çerçeveden ve kâğıt çizelgesinin standart ölçeklerinden, tabloların sayıları elle "
                   "yuvarlanarak; KentOS kodu yok.",
           "setup": rs.setup(rs.MAIN), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Ayırma çapı paftaları ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
