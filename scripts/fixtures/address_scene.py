#!/usr/bin/env python3
"""Adres arama's scene and traces (docs/adr/0229 §13), from the locator's reference without KentOS code.

Writes fixtures/interaction/v1/addresses.kcad: the metres scene of scripts/fixtures/geocode_cases.py (three
neighbourhoods, Çankaya and Ankara round them, five streets and twenty-two doors) on styled layers with their
labels, the project's address data (Numarataj) in its settings and Adres noktaları the active layer; and the
traces address-search.json (Adres ara: the tab, an address found, Git and its mark, Nokta ekle with the match's
attributes and its undo, a tie, no candidate, an address code) and address-reverse.json (Konumdan adres: a door,
a street, a region; Esc). The rows, statuses, places and attributes are the reference's (`find`, `reverse`,
`label`); the words of the tab are docs/adr/0229 §13's.

    python3 scripts/fixtures/address_scene.py [--check]
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import address_processing_cases as proc  # noqa: E402
import geocode_cases as g  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "interaction" / "v1"
E, N = proc.E, proc.N
CENTER = (E + 200.0, N + 200.0)
DECIMALS = 3


def layer(id_, name, color, weight=0.25, line="continuous", fill=None):
    style = {"color": color, "lineType": line, "lineWeight": weight}
    if fill:
        style["fill"] = fill
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": style, "children": []}


def drawing():
    doc, scene, _events, _apps = proc.drawing()
    # Only the address data's layers, styled; the doors and streets labelled.
    keep = {"kapi", "yol", "mahalle", "ilce", "il"}
    entities = [e for e in doc["entities"] if e["layerId"] in keep]
    for e in entities:
        if e["layerId"] == "kapi" and e["attrs"].get("KAPI_NO"):
            e["label"] = e["attrs"]["KAPI_NO"]
        elif e["layerId"] == "yol":
            e["label"] = e["attrs"]["YOL_ADI"]
        elif e["layerId"] == "mahalle":
            e["label"] = e["attrs"]["AD"]
    doc["entities"] = entities
    doc["name"] = "Adres arama"
    doc["settings"]["lengthDecimals"] = DECIMALS
    doc["layers"] = [
        layer("il", "İl", "#868E96", 0.5, "dashed"),
        layer("ilce", "İlçe", "#ADB5BD", 0.35, "dashed"),
        layer("mahalle", "Mahalle", "#2F9E44", 0.35, fill="#2F9E441A"),
        layer("yol", "Yol orta hattı", "#F08C00", 0.7),
        layer("kapi", "Kapı", "#E8590C"),
        layer("adres", "Adres noktaları", "#E5484D"),
    ]
    doc["activeLayer"] = "adres"
    return doc, scene


def rel(p):
    return [round(p[0] - CENTER[0], 9), round(p[1] - CENTER[1], 9)]


def cells(c, place=False):
    """A row as the tab shows it: Adres | Tür | Puan (a place's distance) | Mahalle | İlçe."""
    if place:
        points = "—" if c["distance"] is None else f"{c['distance']:.{DECIMALS}f} m"
    else:
        points = str(c["score"])
    t = c["texts"]
    return " | ".join([c["label"], c["kind"], points, t["neighbourhood"], t["district"]])


def status(cands, st):
    line = f"{len(cands)} aday"
    if cands:
        line += f" · En iyisi: {st} ({cands[0]['score']})"
    return line


NOT_FOUND = "Aday yok: yazımı, kaynağı ve adres verisinin alanlarını denetleyin."
FIELDS = [("Mahalle", "neighbourhood"), ("Yol", "street"), ("Kapı no", "door"), ("İlçe", "district"), ("İl", "province"),
          ("Posta kodu", "postcode")]


def attrs_of(c, st, text):
    a = {"Durum": st, "Puan": str(c["score"]), "Eşleşme türü": c["kind"], "Eşleşen adres": c["label"]}
    for name, key in FIELDS:
        if c["texts"][key]:
            a[name] = c["texts"][key]
    a["Kaynak"] = proc.LOCATOR["name"]
    a["Girdi adresi"] = text
    return a


def search_trace(loc, count):
    steps = [{"run": "address.search", "note": "Adres ara: alt panelin Adres sekmesi açılır, kutu klavyeyi alır; kaynak projenin adres verisi (Numarataj)",
              "expect": {"panel": "Adres", "address": {"count": "", "rows": []}, "mark": None}}]

    def ask(text, note):
        cands, st = g.find(loc, text, g.parse(text), proc.LOCATOR["minScore"], 20)
        want = {"count": status(cands, st), "rows": [cells(c) for c in cands]} if cands else {"count": NOT_FOUND, "rows": []}
        steps.append({"panel": "Adres", "fill": {"Adres ara": text}, "key": "Enter", "note": note, "expect": {"address": want}})
        return cands, st

    text = "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara"
    cands, st = ask(text, "Adres tek satırda, kısaltmalarıyla: 4 numaralı kapı eşleşir (100); aynı yolun öbür kapıları ve yol düzeyi adaylar daha düşük")
    steps.append({"shot": "adres-ara", "note": "Adres sekmesi: kaynak, adres, adaylar puanlarıyla, durum satırı"})
    best = cands[0]
    steps.append({"panel": "Adres", "row": 1, "note": "Satıra basmak görünümü adaya getirir ve çizimde işaretler (kaydedilmez)",
                  "expect": {"viewCenter": rel(best["at"]), "mark": rel(best["at"])}})
    steps.append({"panel": "Adres", "press": "Nokta ekle", "note": "Nokta ekle: adayın yerine etkin katmana (Adres noktaları) eşleşmenin öznitelikleriyle bir nokta, tek adım",
                  "expect": {"newest": {"kind": "point", "points": [rel(best["at"])], "layer": "Adres noktaları", "attrs": attrs_of(best, st, text)},
                             "entities": count + 1, "canUndo": True}})
    steps.append({"run": "edit.undo", "note": "Geri al noktayı kaldırır", "expect": {"entities": count, "canUndo": False}})
    cands, st = ask("Atatürk Bulvarı No:2", "Mahallesiz: iki mahallede de 2 numaralı kapı var, eşit puan: Belirsiz")
    second = cands[1]
    steps.append({"panel": "Adres", "row": 2, "note": "İkinci aday: görünüm ona gelir, işaret onun yerinde",
                  "expect": {"viewCenter": rel(second["at"]), "mark": rel(second["at"])}})
    steps.append({"shot": "adres-belirsiz", "note": "Belirsiz: iki mahallenin aynı numaralı kapıları; ikincisi seçili ve işaretli"})
    ask("İnönü Caddesi No:1", "Adres verisinde olmayan yol: aday yok")
    ask("1000000002", "Adres kodu (UAVT): kapının kendisi, Kod 100")
    trace = {
        "format": "kentos.interaction-trace",
        "version": 1,
        "id": "address-search",
        "title": "Adres ara (ADR 0229 §13): alt panelin Adres sekmesi projenin adres verisinde arar; adres tek satırda, kısaltmalarıyla okunur, adaylar puanlarıyla; satıra basmak görünümü getirir ve işaretler; Nokta ekle eşleşmenin öznitelikleriyle tek adımda yazar, geri alınır; mahallesiz adres iki mahallede Belirsiz; olmayan yol aday vermez; adres kodu kapının kendisini bulur",
        "source": "Masaüstünün address/ modülü, çekirdeğin ops::geocode'u, kentos_processing::address; beklenenler scripts/fixtures/geocode_cases.py'nin başvurusundan (scripts/fixtures/address_scene.py), ADR 0229",
        "covers": ["GIS-40"],
        "document": "addresses.kcad",
        "view": {"center": list(CENTER), "metresPerPixel": 0.5},
        "clickTolerance": 1.0,
        "steps": steps,
    }
    return trace


def reverse_trace(loc):
    steps = [{"run": "address.reverse", "note": "Konumdan adres: çizimde tıklanan yerin adresi Adres sekmesinin kaynağından",
              "expect": {"tool": "addressReverse"}}]

    def answer(at):
        p = (E + at[0], N + at[1])
        place = g.reverse(loc, p, proc.LOCATOR["reach"])
        expect = {"panel": "Adres", "tool": "addressReverse"}
        if place is None:
            expect["address"] = {"count": "Bu yerin yakınında adres yok.", "rows": []}
        else:
            expect["address"] = {"count": "Tıklanan yerin adresi", "rows": [cells(place, place=True)]}
            expect["mark"] = rel(place["at"])
        return p, expect

    # A click kenetlenir: the door itself, its address 0 m away.
    p, expect = answer((295.0, 60.0))
    steps.append({"click": rel(p), "note": "Kenet açıkken kapı noktasına tıklamak: kapının kendisi, uzaklığı 0", "expect": expect})
    # Typed places are exact (Y,X in a CBS project).
    for at, note in [
        ((150.0, 250.0), "Yazılan yer (Y,X), Bahçelievler'de: yakında kapı yok, en yakın yol (Gazi Mustafa Kemal Bulvarı) ve mahallesi; uzaklığı 50 m"),
        ((470.0, 470.0), "İlçenin dışında, ilin içinde: kapı ve yol uzak, yalnız il"),
    ]:
        p, expect = answer(at)
        steps.append({"text": f"{p[0]:.0f},{p[1]:.0f}"})
        steps.append({"key": "Enter", "note": note, "expect": expect})
        if at == (150.0, 250.0):
            steps.append({"shot": "adres-konumdan", "note": "Konumdan adres: yolun en yakın noktası işaretli, satırda uzaklık"})
    steps.append({"key": "Esc", "note": "Esc biter", "expect": {"tool": "select"}})
    return {
        "format": "kentos.interaction-trace",
        "version": 1,
        "id": "address-reverse",
        "title": "Konumdan adres (ADR 0229 §8, §13): tıklanan ya da yazılan yerin adresi projenin adres verisinden: uzaklık içindeki en yakın kapı, yoksa en yakın yol, ve içinde kaldığı mahalle, ilçe ve il; sekmede tek satır, yeri işaretli; Esc biter",
        "source": "kentos_interaction::address_reverse, masaüstünün address/ modülü, çekirdeğin Locator::reverse'ü; beklenenler scripts/fixtures/geocode_cases.py'nin başvurusundan (scripts/fixtures/address_scene.py), ADR 0229",
        "covers": ["GIS-40"],
        "document": "addresses.kcad",
        "view": {"center": list(CENTER), "metresPerPixel": 0.5},
        "draft": {"snap": True},
        "clickTolerance": 1.0,
        "steps": steps,
    }


def main():
    doc, scene = drawing()
    loc = g.build(scene)
    outs = [
        (DIR / "addresses.kcad", json.dumps(doc, ensure_ascii=False, indent=1) + "\n"),
        (DIR / "address-search.json", json.dumps(search_trace(loc, len(doc["entities"])), ensure_ascii=False, indent=1) + "\n"),
        (DIR / "address-reverse.json", json.dumps(reverse_trace(loc), ensure_ascii=False, indent=1) + "\n"),
    ]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        return 1 if bad else print("adres sahnesi ve izleri tutarlı") or 0
    for p, t in outs:
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
