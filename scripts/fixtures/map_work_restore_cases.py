#!/usr/bin/env python3
"""The shared cases of `map.work.restore` (Onaylı hâline getir; plan/map-cad.md MAP-01, docs/adr/4080), written from
the ADR without KentOS code.

    python3 scripts/fixtures/map_work_restore_cases.py           # writes fixtures/commands/v1/map.work.restore.json
    python3 scripts/fixtures/map_work_restore_cases.py --check   # writes nothing; compares

The drawing is map_work_cases.py's map project. Its item is-4 “Tasarım yüzeyi” (the layer `tasarim`: the lines T1 and
T2) is approved on the drawing as it is, its approval keeping its objects' digests, parts and values (docs/adr/2880,
3280, 3680; worked out here with hashlib and json as that script does); each case's drawing is then what happened
after: an object removed, changed, moved to another layer, one added, a layer locked, hidden, taken away or made a
point layer. The rules, by hand:

- an approved object not in the drawing is made again with its persistent id and its value as approved, in the next
  slot (one more than the drawing's last), `restored`; one in the drawing whose digest is not its approved one is set
  back to its value at its slot, `reverted`; both in persistent-id order;
- `uids` names only those, each one of the item's approved objects (`not_approved_object`) and removed or changed
  (`nothing_to_restore`); none: every one removed or changed (none at all: `nothing_to_restore` at `item`);
- the objects added since the approval stay and are counted (`added`); `asApproved` says whether the item's objects
  are then its approved ones;
- every object put back goes onto its approved layer, which must be there (`layer_not_found`), unlocked
  (`layer_locked`) and of its kind (`layer_geometry_mismatch`); a hidden one is warned (`layer_hidden`); a changed object
  on a locked layer now is refused (`layer_locked`). All are put back or none, in one undo step “Onaylı hâline getir”;
- an item not approved, one with a file (its approval keeps its file's revision), one approved before its objects'
  digests (schema 210) or values (213) were kept: refused.

After a restore, the same restore is asked again and finds nothing to put back; undone, it finds it all again.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import map_work_cases as mw  # noqa: E402

OUT = mw.OUT / "map.work.restore.json"
T1, T2, G1 = mw.T1, mw.T2, mw.G1
NEW = mw.uid(0x10)
STEPPED = {"canUndo": True, "dirty": True, "revision": "changed"}
UNCHANGED = mw.UNCHANGED
SAME = {"revision": "same"}
LABEL = "Onaylı hâline getir"


def approved(kot=None, values=True, parts=True, objects=True):
    """map_work_cases.py's drawing with is-4 approved as it is (T2 carrying `Kot` when given)."""
    s = mw.setup()
    if kot is not None:
        s["entities"][s["uids"].index(T2)]["attrs"] = {"Kot": kot}
    d = mw.Drawing(s)
    it = s["settings"]["mapWork"]["items"][3]
    it.update(status="approved", approved=d.content("is-4"))
    if objects:
        it["approvedObjects"] = d.pairs("is-4")
        if parts:
            it["approvedParts"] = d.parts_of("is-4")
        if values:
            it["approvedValues"] = d.values_of("is-4")
    return s


def entity(s, u):
    return copy.deepcopy(s["entities"][s["uids"].index(u)])


def removed(s, *uids):
    """The objects taken out; the rest numbered again in file order (a .kcad file keeps no gap)."""
    for u in uids:
        k = s["uids"].index(u)
        del s["entities"][k]
        del s["uids"][k]
    for n, e in enumerate(s["entities"], start=1):
        e["id"] = n
    return s


def at(s, u):
    """The slot of `u` in the case's drawing."""
    return str(s["entities"][s["uids"].index(u)]["id"])


def as_approved(original, s, u):
    """The object `u` as approved, in its slot in the case's drawing."""
    return dict(entity(original, u), id=int(at(s, u)))


def changed(s, u, **fields):
    s["entities"][s["uids"].index(u)].update(fields)
    return s


def added(s):
    """A line drawn on `tasarim` after the approval (the next slot)."""
    slot = max(e["id"] for e in s["entities"]) + 1
    s["entities"].append(mw.line(slot, "tasarim", (0.0, 10.0), (10.0, 10.0)))
    s["uids"].append(NEW)
    return s


def layer_of(s, lid):
    def find(nodes):
        for n in nodes:
            if n["id"] == lid:
                return n
            got = find(n["children"])
            if got:
                return got
        return None
    return find(s["layers"])


def next_slot(s):
    return max(e["id"] for e in s["entities"]) + 1


def out(restored=(), reverted=(), added_=0, as_approved=True):
    return {"item": "is-4", "restored": list(restored), "reverted": list(reverted), "added": added_,
            "asApproved": as_approved, "revision": "$current"}


def restore(inp, result, op="execute", expect=None):
    s = {"op": op, "input": inp, "result": result}
    s["expect"] = expect if expect is not None else (STEPPED if op == "execute" and result["status"] == "completed"
                                                     else UNCHANGED)
    return s


def nothing(name="Tasarım yüzeyi"):
    return mw.refused("nothing_to_restore", f"“{name}” onaylandığından beri silinen ya da değişen nesnesi yok; onaylı "
                      "hâline getirilecek bir şey yok.", "item")


def case(name, steps, setup, note=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["steps"] = steps
    c["setup"] = setup
    return c


def cases():
    out_ = []
    item = {"item": "is-4"}
    original = approved()
    # An object removed since the approval: made again with its id and value, in the next slot.
    s = removed(approved(), T1)
    back = dict(entity(original, T1), id=next_slot(s))
    out_.append(case("Silinen nesne onaylı hâliyle, aynı kimlikle geri gelir", [
        restore(item, mw.done(out(restored=[T1])), expect=dict(STEPPED, entities={str(back["id"]): back})),
        restore(item, nothing(), op="plan", expect=SAME),
        {"op": "undo", "returns": LABEL},
        restore(item, mw.done(out(restored=[T1])), op="plan", expect=SAME)], s,
        note="T1 silinmişti; onayın değeriyle yeni yuvada, kalıcı kimliği T1"))
    # A changed attribute set back to its approved value.
    kot = approved(kot="1060")
    s = changed(copy.deepcopy(kot), T2, attrs={"Kot": "1061"})
    out_.append(case("Değişen öznitelik onaylanan değerine döner", [
        restore(item, mw.done(out(reverted=[T2])), expect=dict(STEPPED, entities={at(s, T2): as_approved(kot, s, T2)})),
        restore(item, nothing(), op="plan", expect=SAME)], s, note="Kot 1061 → onaylanan 1060"))
    # A changed geometry.
    s = changed(approved(), T2, b={"x": mw.X0 + 12.0, "y": mw.Y0 + 6.0})
    out_.append(case("Geometrisi değişen nesne onaylanan yerine döner", [
        restore(item, mw.done(out(reverted=[T2])), expect=dict(STEPPED, entities={at(s, T2): as_approved(original, s, T2)}))],
        s))
    # Moved off the item's layers: in the drawing still, set back onto its layer.
    s = changed(approved(), T2, layerId="cizim")
    out_.append(case("Öğenin katmanından başka katmana taşınan nesne onaylı katmanına döner", [
        restore(item, mw.done(out(reverted=[T2])), expect=dict(STEPPED, entities={at(s, T2): as_approved(original, s, T2)})),
        restore(item, nothing(), op="plan", expect=SAME)], s, note="T2 Çizim katmanına taşınmıştı"))
    # Removed, changed and added: the added one stays.
    mixed = added(changed(removed(approved(), T1), T2, b={"x": mw.X0 + 12.0, "y": mw.Y0 + 6.0}))
    back = dict(entity(original, T1), id=next_slot(mixed))
    out_.append(case("Silinen ve değişen geri gelir, onaydan sonra eklenen yerinde kalır", [
        restore(item, mw.done(out(restored=[T1], reverted=[T2], added_=1, as_approved=False)),
                expect=dict(STEPPED, entities={str(back["id"]): back, at(mixed, T2): as_approved(original, mixed, T2)})),
        restore(item, nothing(), op="plan", expect=SAME)], mixed))
    out_.append(case("Yalnız verilen nesne", [
        restore({"item": "is-4", "uids": [T1]}, mw.done(out(restored=[T1], added_=1, as_approved=False))),
        restore({"item": "is-4", "uids": [T2]}, mw.done(out(reverted=[T2], added_=1, as_approved=False)), op="plan",
                expect=SAME)], copy.deepcopy(mixed), note="T2 değişmiş kalır; ardından yalnız onun planı"))
    out_.append(case("Aynı nesne iki kez verilince bir kez", [
        restore({"item": "is-4", "uids": [T1, f" {T1} "]}, mw.done(out(restored=[T1], added_=1, as_approved=False)),
                op="plan")], copy.deepcopy(mixed)))
    out_.append(case("Plan yazmaz", [restore(item, mw.done(out(restored=[T1], reverted=[T2], added_=1,
                                                                as_approved=False)), op="plan")],
                     copy.deepcopy(mixed)))
    out_.append(case("Doğrulama yazmaz", [restore(item, mw.done(None), op="validate")], copy.deepcopy(mixed)))
    s = removed(approved(), T1)
    layer_of(s, "tasarim")["visible"] = False
    hidden = {"code": "layer_hidden", "message": "“Tasarım yüzeyi” katmanı gizli; çizilen nesne görünmeyecek.",
              "path": "item"}
    out_.append(case("Gizli katmana uyarıyla", [restore(item, mw.done(out(restored=[T1]), [hidden]))], s))
    # Refusals.
    out_.append(case("Harita projesi değil", [restore(item, mw.NOT_MAP)], mw.setup(work="road", map_work=None)))
    out_.append(case("Olmayan öğe", [restore({"item": "is-9"}, mw.refused(
        "unknown_work_item", "Harita işinde “is-9” kimlikli öğe yok.", "item"))], copy.deepcopy(mixed)))
    out_.append(case("Onaylı olmayan öğe", [restore(item, mw.refused(
        "work_not_approved", "“Tasarım yüzeyi” onaylı değil; onaylı hâline getirilecek bir onayı yok.", "item"))],
        mw.setup()))
    s = mw.setup()
    s["settings"]["mapWork"]["items"][0].update(status="approved", approved=mw.S_GNSS)
    out_.append(case("Dosyalı kaynağın onayı nesneleri tutmaz", [restore({"item": "is-1"}, mw.refused(
        "no_approved_values", "“GNSS ölçümü 2026-10” dosyalı bir kaynak: onayı nesnelerini değil dosyasının "
        "revizyonunu tutar; onaylı hâline getirilemez.", "item"))], s))
    out_.append(case("Nesne başına özet tutmayan onay (şema 210'dan önce)", [restore(item, mw.refused(
        "no_approved_values", "“Tasarım yüzeyi” öğesinin onayı nesne başına özet tutmuyor (şema 210'dan önce ya da "
        "100 000'den çok nesne); onaylı hâline getirilemez, öğeyi yeniden onaylayın.", "item"))],
        removed(approved(objects=False), T1)))
    out_.append(case("Değerleri tutmayan onay (şema 213'ten önce)", [restore(item, mw.refused(
        "no_approved_values", "“Tasarım yüzeyi” öğesinin onayı nesnelerin değerlerini tutmuyor (şema 213'ten önce ya "
        "da birlikte 4 MiB'tan büyük); onaylı hâline getirilemez, öğeyi yeniden onaylayın.", "item"))],
        removed(approved(values=False), T1)))
    out_.append(case("Kimlik UUID değil", [restore({"item": "is-4", "uids": ["x"]}, mw.refused(
        "invalid_uid", "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
        "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden "
        "alın.", "uids[0]"))], copy.deepcopy(mixed)))
    out_.append(case("Öğenin onaylı nesnelerinden biri değil", [restore({"item": "is-4", "uids": [G1]}, mw.refused(
        "not_approved_object", f"“{G1}” “Tasarım yüzeyi” öğesinin onaylı nesnelerinden biri değil.", "uids[0]"))],
        copy.deepcopy(mixed), note="G1 GNSS ölçümünün noktası"))
    out_.append(case("Onaydan sonra eklenen nesne onaylı değil", [restore({"item": "is-4", "uids": [NEW]}, mw.refused(
        "not_approved_object", f"“{NEW}” “Tasarım yüzeyi” öğesinin onaylı nesnelerinden biri değil.", "uids[0]"))],
        copy.deepcopy(mixed)))
    s = removed(approved(), T1)
    out_.append(case("Verilen nesne onaylandığı gibi", [restore({"item": "is-4", "uids": [T2]}, mw.refused(
        "nothing_to_restore", f"“{T2}” onaylandığı gibi; onaylı hâline getirilecek bir şey yok.", "uids[0]"))], s))
    out_.append(case("Onaydan sonra hiçbir şey değişmemiş", [restore(item, nothing())], approved()))
    s = removed(approved(), T1, T2)
    s["layers"] = [n for n in s["layers"] if n["id"] != "tasarim"]
    out_.append(case("Onaylı katmanı çizimde yok", [restore(item, mw.refused(
        "layer_not_found", "“tasarim” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "item"))],
        s, note="Tasarım yüzeyi katmanı nesneleriyle silinmişti"))
    s = removed(approved(), T1)
    layer_of(s, "tasarim")["locked"] = True
    out_.append(case("Onaylı katmanı kilitli", [restore(item, mw.refused(
        "layer_locked", "“Tasarım yüzeyi” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
        "etkinleştirin.", "item"))], s))
    s = removed(approved(), T1)
    layer_of(s, "tasarim")["locked"] = True
    out_.append(case("Verilen nesnenin onaylı katmanı kilitli", [restore({"item": "is-4", "uids": [T1]}, mw.refused(
        "layer_locked", "“Tasarım yüzeyi” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
        "etkinleştirin.", "uids[0]"))], copy.deepcopy(s)))
    s = changed(approved(), T2, layerId="kilitli")
    out_.append(case("Değişen nesne şimdi kilitli katmanda", [restore(item, mw.refused(
        "layer_locked", "“Kilitli” katmanı kilitli; üstündeki nesne onaylı hâline getirilemez. Kilidi Katmanlar "
        "panelinden açın.", "item"))], s))
    s = removed(approved(), T1, T2)
    layer_of(s, "tasarim")["geometry"] = "point"
    out_.append(case("Onaylı katmanı artık nokta katmanı", [restore(item, mw.refused(
        "layer_geometry_mismatch", "“Tasarım yüzeyi” bir nokta katmanı; çizgi bu katmana yazılmaz. Katmanın türünü "
        "Karışık yapın.", "item"))], s))
    out_.append(case("Geçersiz beklenen sürüm", [restore({"item": "is-4", "expectedRevision": "abc"}, mw.BAD_REVISION)],
                     copy.deepcopy(mixed)))
    out_.append(case("Eski sürüme göre hazırlanmış", [restore({"item": "is-4", "expectedRevision": "41"}, mw.CONFLICT)],
                     copy.deepcopy(mixed)))
    return out_


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "map.work.restore", "commandVersion": 1,
           "title": "Onaylı hâline getir",
           "note": "ADR 4080. Bağımsız başvuru scripts/fixtures/map_work_restore_cases.py: onay map_work_cases.py'nin "
                   "kurallarıyla (özet, parçalar, değerler) elle; geri gelen nesne onayın değeri; KentOS kodu yok.",
           "setup": approved(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("Onaylı hâline getir ortak durumları güncel." if ok else f"{OUT.name} güncel değil.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(mw.ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
