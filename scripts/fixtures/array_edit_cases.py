"""Diziyi düzenle: shared cases of cad.arrays.edit (docs/adr/0301).

    python3 scripts/fixtures/array_edit_cases.py           # writes the file
    python3 scripts/fixtures/array_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.arrays.edit.json. The ADR's rules, computed here without KentOS code (the array the
cases start from is made the way scripts/fixtures/array_assoc_cases.py says, ADR 0300):

- Items are numbered in the layout's order from 0, a grid row after row (row r, column c is r × cols + c).
- layout: every item laid out again; an item keeps its number (between two grids its row and column); a removed one
  stays removed, a kept one stays where it is; a new place gets a new insert (the slots that follow, in the places'
  order), a place that is gone takes its insert away (in the items' order). Each place is the first place moved by
  the layout (grid j·dx east and i·dy north; line k·(dx, dy); path the path's point less its start).
- placement: the first place's p, scale, rotation and mirror; every item that is neither removed nor kept follows
  (a place keeps the first place's scale, turn and mirror; the grid's moves are not turned).
- replace: the items' inserts take the named block and are kept. remove: the items' inserts are deleted and the
  items stay removed. reset: the items (all without a list) as the layout makes them: a removed one gets a new insert
  that copies the first item that is neither removed nor kept, a kept one goes to its place with the array's block.
- The checks' order as the contract lists it; one undo step “Diziyi düzenle”.
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import array_assoc_cases as A  # noqa: E402  (the array the cases start from, ADR 0300)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.arrays.edit.json"
U = A.U
p = A.p
LABEL = "Diziyi düzenle"
TREE = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3a01", "name": "Ağaç", "base": p(0, 0),
        "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": p(0, 0), "r": 2}]}
GRID = A.GRID
CENTRE = (10, 0)  # the circle's centre: the block's base point and the first place
LOCKED = "Dizinin ya da öğelerinden birinin katmanı kilitli; dizi düzenlenmedi. Katmanın kilidini Katmanlar panelinden açın."
ITEMS = "Öğe numaralarını verin; her biri bir kez. Öğeler yerleşimin sırasıyla 0'dan numaralanır (ızgarada satır satır)."


def setup():
    s = A.setup()
    s["blocks"] = [copy.deepcopy(TREE)]
    return s


def moves(layout):
    if layout["kind"] == "path":
        return A.moves(layout, A.PATH)
    return A.moves(layout)


def insert(slot, at, name="Dizi 1", scale=1, rotation=0, mirror=False):
    e = {"kind": "insert", "id": slot, "layerId": "cizim", "attrs": {}, "block": f"$blockOf:{name}", "p": p(*at),
         "scale": scale, "rotation": rotation}
    if mirror:
        e["mirror"] = True
    return e


def places(layout, first=CENTRE):
    return [(first[0] + dx, first[1] + dy) for dx, dy in moves(layout)]


def record(layout, items, first=CENTRE, scale=1, rotation=0, mirror=False):
    """The array's record: `items` one per place, a slot, (slot, "kept") or None (removed)."""
    out = []
    for item in items:
        if item is None:
            out.append({})
        elif isinstance(item, tuple):
            out.append({"uid": U(item[0]), "kept": True})
        else:
            out.append({"uid": U(item)})
    r = {"block": "$blockOf:Dizi 1", "layerId": "cizim", "p": p(*first), "scale": scale, "rotation": rotation,
         "layout": layout, "items": out}
    if mirror:
        r["mirror"] = True
    return r


def start(layout=GRID):
    """The steps that make the circle an associative array, and its items' slots (the drawing's blocks are the
    array's and Ağaç: the making step's block list is left to array_assoc_cases.py)."""
    steps, slots = A.create([1], layout)
    for step in steps:
        step.get("expect", {}).pop("blocks", None)
        if step["op"] in ("execute", "plan"):
            step.setdefault("command", "cad.entities.array")
    return steps, slots


def edit(e, **more):
    return {"array": "$array:0", "edit": e, **more}


def done(changed=(), created=(), removed=()):
    return {"status": "completed", "warnings": [],
            "output": {"changed": [U(s) for s in changed], "created": [U(s) for s in created],
                       "removed": list(removed), "revision": "$current"}}


def refused(code, message, path, ids):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def same_state(ids, arrays):
    return {"ids": ids, "arrays": arrays, "revision": "same"}


def capture(slots):
    return [{"op": "captureUid", "id": s, "as": f"s{s}"} for s in slots]


def cases():
    out = []
    grid_slots = list(range(6, 12))
    others = [2, 3, 4, 5]

    # ── layout ──
    bigger = {"kind": "grid", "rows": 3, "cols": 4, "dx": 5, "dy": 4}
    steps, slots = start()
    new = {3: 12, 7: 13, 8: 14, 9: 15, 10: 16, 11: 17}
    items = [slots[0], slots[1], slots[2], 12, slots[3], slots[4], slots[5], 13, 14, 15, 16, 17]
    pl = places(bigger)
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": bigger}), "result": done(created=list(new.values())),
                  "expect": {"ids": sorted(others + items), "arrays": [record(bigger, items)],
                             "entities": {str(s): insert(s, pl[j]) for j, s in new.items()}, "revision": "changed"}})
    steps.append({"op": "undo", "returns": LABEL, "expect": {"ids": sorted(others + slots), "arrays": [record(GRID, slots)]}})
    steps.append({"op": "redo", "returns": LABEL, "expect": {"ids": sorted(others + items), "arrays": [record(bigger, items)]}})
    out.append({"name": "Izgara 2×3'ten 3×4'e: öğeler satır ve sütunlarını korur, altı yeni yerleştirme; adım “Diziyi düzenle”, geri alınır ve yinelenir",
                "steps": steps})

    smaller = {"kind": "grid", "rows": 1, "cols": 2, "dx": 5, "dy": 4}
    steps, slots = start()
    steps += capture(slots[2:])
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": smaller}),
                  "result": done(removed=[f"$uid:s{s}" for s in slots[2:]]),
                  "expect": {"ids": sorted(others + slots[:2]), "arrays": [record(smaller, slots[:2])]}})
    out.append({"name": "Izgara 1×2'ye: kalkan yerlerin dört yerleştirmesi silinir", "steps": steps})

    spaced = {"kind": "grid", "rows": 2, "cols": 3, "dx": 8, "dy": 6}
    steps, slots = start()
    pl = places(spaced)
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": spaced}), "result": done(changed=slots[1:]),
                  "expect": {"arrays": [record(spaced, slots)], "entities": {str(s): insert(s, pl[j]) for j, s in enumerate(slots)}}})
    out.append({"name": "Aralıklar 8 ve 6: ilk yer kalır, öbür beş öğe yerine gider", "steps": steps})

    line = {"kind": "line", "count": 4, "dx": 0, "dy": 10}
    steps, slots = start()
    steps += capture(slots[4:])
    pl = places(line)
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": line}),
                  "result": done(changed=slots[1:4], removed=[f"$uid:s{s}" for s in slots[4:]]),
                  "expect": {"ids": sorted(others + slots[:4]), "arrays": [record(line, slots[:4])],
                             "entities": {str(s): insert(s, pl[j]) for j, s in enumerate(slots[:4])}}})
    out.append({"name": "Izgaradan dört yerli doğruya: öğeler numaralarını korur, fazla iki yerleştirme silinir", "steps": steps})

    steps, slots = start()
    moved = insert(slots[4], (15, 5))
    steps.append({**A.helper("cad.entities.transform", {"uids": [U(slots[4])], "transform": {"kind": "move", "dx": 0, "dy": 1}})})
    pl = places(spaced)
    expect = {str(s): insert(s, pl[j]) for j, s in enumerate(slots)}
    expect[str(slots[4])] = moved
    items = list(slots)
    items[4] = (slots[4], "kept")
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": spaced}),
                  "result": done(changed=[s for s in slots[1:] if s != slots[4]]),
                  "expect": {"arrays": [record(spaced, items)], "entities": expect}})
    out.append({"name": "Elle taşınan öğe korunur: aralık değişince yerinde kalır", "steps": steps})

    steps, slots = start()
    steps.append(A.helper("cad.entities.delete", {"uids": [U(slots[2])]}))
    items = [slots[0], slots[1], None, 12, slots[3], slots[4], slots[5], 13, 14, 15, 16, 17]
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": bigger}), "result": done(created=[12, 13, 14, 15, 16, 17]),
                  "expect": {"arrays": [record(bigger, items)], "ids": sorted(others + [s for s in items if s is not None])}})
    out.append({"name": "Elle silinen öğe silinmiş kalır: büyüyen ızgara onu geri getirmez", "steps": steps})

    steps, slots = start(A.ON_PATH)
    longer = {**A.ON_PATH, "count": 5}
    pl = places(longer)
    items = slots + [9, 10]
    steps.append({"op": "execute", "input": edit({"kind": "layout", "layout": longer}), "result": done(changed=slots[1:], created=[9, 10]),
                  "expect": {"arrays": [record(longer, items)], "entities": {str(s): insert(s, pl[j]) for j, s in enumerate(items)}}})
    out.append({"name": "Yol dizisinde adet 3'ten 5'e: öğeler yola yeniden eşit dağılır, iki yeni yerleştirme", "steps": steps})

    # ── placement ──
    steps, slots = start()
    pl = places(GRID, (20, 1))
    steps.append({"op": "execute", "input": edit({"kind": "placement", "p": p(20, 1)}), "result": done(changed=slots),
                  "expect": {"arrays": [record(GRID, slots, first=(20, 1))], "entities": {str(s): insert(s, pl[j]) for j, s in enumerate(slots)}}})
    out.append({"name": "İlk yer taşınınca bütün dizi izler", "steps": steps})

    steps, slots = start()
    pl = places(GRID)
    steps.append({"op": "execute", "input": edit({"kind": "placement", "scale": 2, "rotation": 0.5, "mirror": True}), "result": done(changed=slots),
                  "expect": {"arrays": [record(GRID, slots, scale=2, rotation=0.5, mirror=True)],
                             "entities": {str(s): insert(s, pl[j], scale=2, rotation=0.5, mirror=True) for j, s in enumerate(slots)}}})
    out.append({"name": "Ölçek, dönüş ve ayna öğelere geçer; aralıklar dönmez", "steps": steps})

    steps, slots = start()
    steps.append({**A.helper("cad.entities.transform", {"uids": [U(slots[1])], "transform": {"kind": "move", "dx": 0, "dy": 1}})})
    pl = places(GRID, (10, 3))
    items = list(slots)
    items[1] = (slots[1], "kept")
    steps.append({"op": "execute", "input": edit({"kind": "placement", "p": p(10, 3)}),
                  "result": done(changed=[s for s in slots if s != slots[1]]),
                  "expect": {"arrays": [record(GRID, items, first=(10, 3))], "entities": {str(slots[1]): insert(slots[1], (15, 1))}}})
    out.append({"name": "Dizi taşınınca elle taşınan öğe yerinde kalır", "steps": steps})

    # ── replace, remove, reset ──
    steps, slots = start()
    items = list(slots)
    items[1], items[4] = (slots[1], "kept"), (slots[4], "kept")
    steps.append({"op": "execute", "input": edit({"kind": "replace", "items": [1, 4], "block": "Ağaç"}), "result": done(changed=[slots[1], slots[4]]),
                  "expect": {"arrays": [record(GRID, items)],
                             "entities": {str(slots[1]): insert(slots[1], (15, 0), name="Ağaç"), str(slots[4]): insert(slots[4], (15, 4), name="Ağaç")}}})
    out.append({"name": "Değiştir: iki öğe Ağaç bloğunu yerleştirir ve korunur", "steps": steps})

    steps, slots = start()
    steps += capture([slots[1], slots[4]])
    items = list(slots)
    items[1] = items[4] = None
    steps.append({"op": "execute", "input": edit({"kind": "remove", "items": [4, 1]}), "result": done(removed=[f"$uid:s{slots[4]}", f"$uid:s{slots[1]}"]),
                  "expect": {"arrays": [record(GRID, items)], "ids": sorted(others + [s for s in items if s is not None])}})
    steps.append({"op": "undo", "returns": LABEL, "expect": {"arrays": [record(GRID, slots)], "ids": sorted(others + slots)}})
    out.append({"name": "Sil: iki öğenin yerleştirmesi gider, öğeler boş kalır; geri alınır", "steps": steps})

    steps, slots = start()
    steps.append(A.helper("cad.entities.delete", {"uids": [U(slots[1])]}))
    steps.append(A.helper("cad.entities.transform", {"uids": [U(slots[4])], "transform": {"kind": "move", "dx": 0, "dy": 1}}))
    steps.append({"op": "execute", "input": edit({"kind": "replace", "items": [5], "block": "Ağaç"}), "result": "$any"})
    pl = places(GRID)
    items = list(slots)
    items[1] = 12
    steps.append({"op": "execute", "input": edit({"kind": "reset"}), "result": done(changed=[slots[4], slots[5]], created=[12]),
                  "expect": {"arrays": [record(GRID, items)], "entities": {str(s): insert(s, pl[j]) for j, s in enumerate(items)}}})
    out.append({"name": "Sıfırla: silinen geri gelir, taşınan ve değiştirilen dizinin bloğuyla yerine döner", "steps": steps})

    steps, slots = start()
    steps.append(A.helper("cad.entities.delete", {"uids": [U(slots[1]), U(slots[2])]}))
    items = list(slots)
    items[1], items[2] = 12, None
    steps.append({"op": "execute", "input": edit({"kind": "reset", "items": [1]}), "result": done(created=[12]),
                  "expect": {"arrays": [record(GRID, items)], "entities": {"12": insert(12, (15, 0))}}})
    out.append({"name": "Sıfırla yalnız öğe 1'i: öteki silinen boş kalır", "steps": steps})

    steps, slots = start()
    steps.append({"op": "execute", "input": edit({"kind": "reset"}), "result": done(),
                  "expect": {"arrays": [record(GRID, slots)], "ids": sorted(others + slots)}})
    out.append({"name": "Değişmemiş dizide Sıfırla hiçbir yerleştirmeyi değiştirmez", "steps": steps})

    steps, slots = start()
    pl = places(bigger)
    steps.append({"op": "plan", "input": edit({"kind": "layout", "layout": bigger}),
                  "result": {"status": "completed", "warnings": [], "output": {
                      "changed": [], "created": [{**insert(0, pl[j])} for j in (3, 7, 8, 9, 10, 11)], "removed": [], "revision": "$current"}},
                  "expect": {"ids": sorted(others + slots), "arrays": [record(GRID, slots)], "revision": "same"}})
    out.append({"name": "Plan yeni yerleştirmeleri gösterir; hiçbir şey yazılmaz", "steps": steps})

    # ── refusals ──
    def refusal(name, e, code, message, path, before=(), array=None, gone=(), non_finite=None, more=None):
        steps, slots = start()
        steps += list(before)
        step = {"op": "execute", "input": {**(edit(e) if array is None else {"array": array, "edit": e}), **(more or {})}}
        if non_finite:
            step["nonFinite"] = non_finite
        steps.append({**step,
                      "result": refused(code, message, path, None),
                      "expect": {"ids": sorted(others + [s for s in slots if s not in gone]), "revision": "same"}})
        out.append({"name": name, "steps": steps})

    refusal("Dizi kimliği UUID değil: invalid_uid", {"kind": "reset"}, "invalid_uid",
            "“dizi” geçerli bir dizi kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği diziyi kuran komutun çıktısından (array) alın.",
            "array", array="dizi")
    refusal("Böyle bir dizi yok: array_not_found", {"kind": "reset"}, "array_not_found",
            "“0192f5a3-3333-7000-8000-00000000ffff” kimlikli ilişkili dizi çizimde yok: kaldırılmış ya da başka bir çizimin olabilir.",
            "array", array="0192f5a3-3333-7000-8000-00000000ffff")
    refusal("Satır 0: invalid_count, yolu edit.layout.rows", {"kind": "layout", "layout": {**GRID, "rows": 0}}, "invalid_count",
            "Satır × sütun 2 ile 10 000 arasında olmalı; satır ve sütun en az 1'dir. Başka bir satır ve sütun sayısı verin.", "edit.layout.rows")
    refusal("Ölçek 0: invalid_scale", {"kind": "placement", "scale": 0}, "invalid_scale",
            "Ölçek sıfırdan büyük olmalı. Başka bir ölçek verin.", "edit.scale")
    refusal("Boş öğe listesi: invalid_item", {"kind": "remove", "items": []}, "invalid_item", ITEMS, "edit.items")
    refusal("İki kez verilen öğe: invalid_item", {"kind": "remove", "items": [1, 1]}, "invalid_item", ITEMS, "edit.items[1]")
    refusal("Yerlerin dışındaki öğe: invalid_item", {"kind": "replace", "items": [6], "block": "Ağaç"}, "invalid_item",
            "Dizinin 6 yeri var; öğe 6 yok. 0 ile 5 arasında bir numara verin.", "edit.items[0]")
    refusal("Elle silinmiş öğe Değiştir'de: item_removed", {"kind": "replace", "items": [2], "block": "Ağaç"}, "item_removed",
            "Öğe 2 elle silinmiş; önce Sıfırla ile geri getirin.", "edit.items[0]",
            before=[A.helper("cad.entities.delete", {"uids": ["$uidOf:8"]})], gone=(8,))
    refusal("Tanımsız blok: unknown_block", {"kind": "replace", "items": [0], "block": "Rögar"}, "unknown_block",
            "“Rögar” adlı blok çizimde tanımlı değil. Var olan bir bloğun adını verin.", "edit.block")
    refusal("Dizinin katmanı kilitli: layer_locked, hiçbir şey yazılmaz", {"kind": "reset"}, "layer_locked", LOCKED, "array",
            before=[{"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": "cizim", "value": True}, "result": "$any"}])
    refusal("Yol dizisinin yolu olmayan nesne: entity_not_found", {"kind": "layout", "layout": {**A.ON_PATH, "path": "0192f5a2-0000-7000-8000-00000000ffff"}},
            "entity_not_found",
            "“0192f5a2-0000-7000-8000-00000000ffff” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
            "edit.layout.path")
    refusal("Sütunlar arası aralık 0: invalid_spacing", {"kind": "layout", "layout": {**GRID, "dx": 0}}, "invalid_spacing",
            "Sütunlar arasındaki aralık (dY) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin.", "edit.layout.dx")
    refusal("Kutupsal dizinin açısı 0: invalid_fill", {"kind": "layout", "layout": {**A.POLAR, "fill": 0}}, "invalid_fill",
            "Doldurma açısı 0 ile ±360 derece arasında olmalı; 0 olamaz. Başka bir açı verin.", "edit.layout.fill")
    refusal("Yol bir nokta: invalid_path", {"kind": "layout", "layout": {**A.ON_PATH, "path": U(5)}}, "invalid_path",
            "Yol bir çizgi, yay, daire ya da çoklu çizgi olmalı ve bir uzunluğu olmalı. Başka bir nesneyi yol olarak seçin.",
            "edit.layout.path")
    refusal("Dönüş sonlu değil: not_finite", {"kind": "placement", "rotation": 0}, "not_finite",
            "Dönüş açısı sonlu bir sayı değil (NaN ya da sonsuz). Açıyı sonlu bir sayıyla verin.", "edit.rotation",
            non_finite={"edit.rotation": "NaN"})
    refusal("Beklenen sürüm sayı değil: invalid_revision", {"kind": "reset"}, "invalid_revision",
            "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
            "expectedRevision", more={"expectedRevision": "on iki"})
    steps, slots = start()
    steps.append({"op": "captureRevision", "as": "old"})
    steps.append(A.helper("cad.entities.transform", {"uids": [U(2)], "transform": {"kind": "move", "dx": 1, "dy": 0}}))
    steps.append({"op": "execute", "input": edit({"kind": "reset"}, expectedRevision="$old"),
                  "result": {"status": "conflict", "error": {
                      "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
                      "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."}},
                  "expect": {"revision": "same"}})
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": steps})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.arrays.edit", "commandVersion": 1,
            "title": "Diziyi düzenle: yerleşim, ilk yer, öğeleri değiştirme, silme ve sıfırlama",
            "note": "ADR 0301. Bağımsız başvuru: scripts/fixtures/array_edit_cases.py (dizinin kuruluşu array_assoc_cases.py, ADR 0300).",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
