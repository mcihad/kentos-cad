#!/usr/bin/env python3
"""Pafta seti with a cropping view recipe (docs/adr/0781 §2), from the ADR's rule, without KentOS code.

Writes fixtures/commands/v1/cad.sheets.set.crop.json; `--check` compares and writes nothing. On top of ADR 0774's
reference (`view_recipe_sheet_cases.py`, whose frames, levels, phases and categories this reuses): a map linked to a
view whose recipe crops (`crop: true`) draws only what its box shares with the view's area: given objects, its frame
`shows` one only when the object's box also meets the box of the view's four corners. The recipe's wall detail
(`detail`) changes how walls look, never what a frame shows.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sheet_set_cases as ssc  # noqa: E402
import view_recipe_sheet_cases as vrs  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.sheets.set.crop.json"
P = ssc.P

# A narrow cropping view round the first column: its area [1, 0.5, 3, 2.5].
DAR = vrs.named("Dar", P(2, 1.5), 2.0, 2.0, {"version": 1, "level": {"id": "zemin"}, "scale": 100, "crop": True}, "v-dar")
VIEWS = vrs.VIEWS + [DAR]
SHEETS = vrs.BOOK + [ssc.sheet("a-108", "A-108 Dar detay", [vrs.linked(ssc.frame("dar", "Dar", center=P(0, 0), scale=500), DAR)])]
SETUP = copy.deepcopy(vrs.SETUP)
SETUP["name"] = "Kırpan tarifli pafta seti"
SETUP["settings"]["namedViews"] = VIEWS
PRELUDE = [{"op": "execute", "command": "cad.sheets.edit", "note": "pafta kitabı: kurulum kitap veremez",
            "input": {"ops": [{"op": "addSheet", "sheet": s} for s in SHEETS]},
            "result": vrs.ANY, "expect": {"revision": "changed"}}]


def book(views=None):
    return vrs.Book(VIEWS if views is None else views, SHEETS)


def dar(recipe):
    """The views after Dar's recipe became `recipe` (its version one more)."""
    views = copy.deepcopy(VIEWS)
    views[-1]["recipe"] = {"version": 2, **recipe}
    return views


def edit(name, recipe):
    return vrs.views_edit({"kind": "recipe", "name": name, "recipe": recipe})


def cases():
    out = []

    def add(name, steps):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "steps": copy.deepcopy(PRELUDE) + steps})

    step = vrs.step
    add("Kırpan tarifli harita da görünümün kimliğini ve tarif sürümünü söyler", [step(book())])
    add("Görünümün alanındaki zemin kolonu dar haritada", [step(book(), [1])])
    add("1. kat kolonu dar haritada yok: katı başka", [step(book(), [2])])
    add("Alana değen katsız çizgi dar haritada", [step(book(), [3])])
    add("Alanın dışındaki tarama dar haritada yok, ötekilerde var", [step(book(), [4])])
    add("Projede olmayan katın kolonu alanın dışında: dar haritada yok", [step(book(), [6])])
    add("Alanın altındaki yıkılacak kolon dar haritada yok", [step(book(), [7])])
    add("Alanın altındaki yeni kolon dar haritada yok", [step(book(), [8])])
    add("Çerçevenin dışındaki kolon hiçbir haritada", [step(book(), [5])])
    add("Bütün nesneler: dar harita yalnız alanındakileri sayar", [step(book(), vrs.IDS)])
    add("Girdinin sırası korunur", [step(book(), [4, 3, 1])])
    add("Boş nesne listesi hiçbir haritayı etkilemez", [step(book(), [])])
    add("Kırpma kalkınca dar harita taramayı da gösterir",
        [edit("Dar", {"level": {"id": "zemin"}, "scale": 100}),
         step(book(dar({"level": {"id": "zemin"}, "scale": 100})), [4, 1])])
    add("Kırpma açıkça kapatılınca da kalkar",
        [edit("Dar", {"level": {"id": "zemin"}, "scale": 100, "crop": False}),
         step(book(dar({"level": {"id": "zemin"}, "scale": 100})), [4, 6])])
    add("Katsız kırpan tarif: 1. kat kolonu alana değdiği için gösterilir",
        [edit("Dar", {"scale": 100, "crop": True}), step(book(dar({"scale": 100, "crop": True})), [1, 2])])
    add("Duvar ayrıntısı neyin gösterildiğini değiştirmez",
        [edit("Dar", {"level": {"id": "zemin"}, "scale": 100, "crop": True, "detail": "detailed"}),
         step(book(dar({"level": {"id": "zemin"}, "scale": 100, "crop": True, "detail": "detailed"})), [1, 4])])
    add("Şematik ayrıntılı kırpan tarif aynı nesneleri sayar",
        [edit("Dar", {"level": {"id": "zemin"}, "scale": 100, "crop": True, "detail": "schematic"}),
         step(book(dar({"level": {"id": "zemin"}, "scale": 100, "crop": True, "detail": "schematic"})), vrs.IDS)])
    zemin = copy.deepcopy(VIEWS)
    zemin[0]["recipe"] = {"version": 2, "level": {"id": "zemin"}, "hidden": ["hatches"], "scale": 100, "crop": True}
    add("Geniş görünümün kırpması alanındakilerden hiçbirini düşürmez",
        [edit("Zemin", {"level": {"id": "zemin"}, "hidden": ["hatches"], "scale": 100, "crop": True}),
         step(book(zemin), [1, 3, 6])])
    add("Geniş görünümün kırpması uzaktaki kolonu yine göstermez",
        [edit("Zemin", {"level": {"id": "zemin"}, "hidden": ["hatches"], "scale": 100, "crop": True}),
         step(book(zemin), [5])])
    gone = VIEWS[:-1]
    add("Kırpan görünüm silinince harita kayıp: nesneler bilinmeyen olarak etkilenir",
        [vrs.views_edit({"kind": "delete", "name": "Dar"}), step(book(gone), [1, 4])])
    add("Kırpan görünüm silinince haritanın zemini bilinmez", [vrs.views_edit({"kind": "delete", "name": "Dar"}), step(book(gone))])
    add("Kırpma ve gizlenen tür birlikte: dar haritada tarama zaten yok",
        [edit("Dar", {"level": {"id": "zemin"}, "hidden": ["hatches"], "scale": 100, "crop": True}),
         step(book(dar({"level": {"id": "zemin"}, "hidden": ["hatches"], "scale": 100, "crop": True})), [4, 3])])
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.sheets.set", "commandVersion": 1,
           "title": "Pafta seti ve kırpan görünüş tarifi (ADR 0781)",
           "note": "scripts/fixtures/view_recipe_crop_cases.py yazar; ADR 0347, 0774 ve 0781'in kurallarından, KentOS kodu olmadan.",
           "setup": SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} bağımsız başvurudan farklı.")
            return 1
        print(f"cad.sheets.set (kırpma): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
