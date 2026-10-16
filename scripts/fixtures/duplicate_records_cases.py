"""Writes fixtures/commands/v1/cad.records.duplicate.json: the shared cases of `cad.records.duplicate` v1 (Kaydı çoğalt,
docs/adr/0415), their expectations built here from the ADR's rules without KentOS code: the copies and their order, the
composite relations' children followed (their field taking the copy's id), the unique fields left empty, the locked
layers, the checks' order and words. A case first ties its children to their parents with `cad.entities.set` (an
object's id is the drawing's) and may lock a layer with `cad.layers.set`. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/duplicate_records_cases.py           # writes the file
    python3 scripts/fixtures/duplicate_records_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.records.duplicate.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
LABEL = "Kaydı çoğalt"


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": copy.deepcopy(STYLE), "children": children or [], **extra}


SETUP_LAYERS = [
    node("bina", "Bina", fields=[{"name": "Kod", "kind": "text"}, {"name": "Kat", "kind": "integer"}]),
    node("kat", "Kat planı", fields=[{"name": "Bina", "kind": "text"}, {"name": "No", "kind": "integer"}]),
    node("oda", "Oda", fields=[{"name": "Kat", "kind": "text"}, {"name": "Ad", "kind": "text"}]),
    node("malik", "Malik", fields=[{"id": "bina-kimligi", "name": "Bina no", "kind": "text"}, {"name": "Ad", "kind": "text"}]),
    node("arsiv", "Arşiv", fields=[{"name": "Bina", "kind": "text"}]),
    node("eski", "Eski binalar", fields=[{"name": "Kod", "kind": "text"}]),
]
RELATIONS = [
    {"id": "katlar", "name": "Binanın katları", "parent": "bina", "child": "kat", "field": "Bina", "onDelete": "cascade"},
    {"id": "odalar", "name": "Katın odaları", "parent": "kat", "child": "oda", "field": "Kat", "onDelete": "cascade"},
    {"id": "malikler", "name": "Binanın malikleri", "parent": "bina", "child": "malik", "field": "bina-kimligi"},
    {"id": "arsiv", "name": "Binanın arşivi", "parent": "bina", "child": "arsiv", "field": "Bina", "onDelete": "cascade"},
]
FORMS = [{"id": "bina-form", "name": "Bina denetimi", "layer": "bina", "draft": {"revision": 1, "items": [
    {"type": "field", "id": "kod", "field": "Kod", "unique": True},
    {"type": "field", "id": "kat", "field": "Kat"},
]}}]

X, Y = 500000, 4400000


def square(x):
    return [{"x": X + x, "y": Y}, {"x": X + x + 10, "y": Y}, {"x": X + x + 10, "y": Y + 8}, {"x": X + x, "y": Y + 8}]


def record(id_, layer, attrs):
    return {"kind": "record", "id": id_, "layerId": layer, "attrs": attrs}


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "bina", "attrs": {"Kod": "B-1", "Kat": "2"}, "label": "B-1", "pts": square(0)},
    {"kind": "polygon", "id": 2, "layerId": "bina", "attrs": {"Kod": "B-2", "Kat": "1"}, "pts": square(20)},
    record(3, "kat", {"No": "0"}),
    record(4, "kat", {"No": "1"}),
    record(5, "kat", {"No": "0"}),
    record(6, "oda", {"Ad": "Salon"}),
    record(7, "oda", {"Ad": "Mutfak"}),
    record(8, "malik", {"Ad": "Ayşe"}),
    record(9, "arsiv", {}),
    {"kind": "point", "id": 10, "layerId": "eski", "attrs": {"Kod": "E-1"}, "p": {"x": X, "y": Y - 10}},
]
# Who belongs to whom: (child, its field's name, its parent).
TIES = [(3, "Bina", 1), (4, "Bina", 1), (5, "Bina", 2), (6, "Kat", 3), (7, "Kat", 4), (8, "Bina no", 1), (9, "Bina", 1)]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Kaydı çoğalt",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow",
                         "forms": copy.deepcopy(FORMS), "relations": copy.deepcopy(RELATIONS)},
            "origin": {"x": X, "y": Y}, "layers": copy.deepcopy(SETUP_LAYERS), "activeLayer": "bina",
            "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []}}


def uid(slot):
    return f"$uidOf:{slot}"


def tied():
    """The drawing after its children are tied: the entities by slot."""
    es = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
    for child, field, parent in TIES:
        es[child]["attrs"][field] = uid(parent)
    return es


def tie_steps(locked=()):
    steps = [{"op": "execute", "command": "cad.entities.set",
              "input": {"uids": [uid(c)], "operation": "attributes", "attrs": {f: uid(p)}},
              "result": completed({"changed": [uid(c)], "ids": [c], "revision": "$current"})} for c, f, p in TIES]
    for layer in locked:
        steps.append({"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": layer, "value": True},
                      "result": completed({"changed": True, "active": "bina", "revision": "$current"})})
    return steps


def field_name(layer, key):
    for n in SETUP_LAYERS:
        if n["id"] == layer:
            for f in n.get("fields", []):
                if f.get("id", f["name"]) == key:
                    return f["name"]
    return key


UNIQUE = {"bina": [("Bina denetimi", "Kod")]}


def duplicate(slots, children=True, locked=()):
    """The copies, the drawing after and the warnings (docs/adr/0415 §1)."""
    es = tied()
    order = sorted(es)
    nxt = max(es) + 1
    copies, done = [], {}
    cleared, skipped_children, made_children = set(), 0, 0

    def copy_of(slot, field=None, parent_copy=None):
        nonlocal nxt
        e = copy.deepcopy(es[slot])
        new = nxt
        nxt += 1
        e["id"] = new
        if field:
            e["attrs"][field] = uid(parent_copy)
        for form, name in UNIQUE.get(e["layerId"], []):
            if name in e["attrs"]:
                del e["attrs"][name]
                cleared.add((form, name))
        es[new] = e
        done[slot] = new
        copies.append({"from": uid(slot), "uid": uid(new), "id": new})
        return new

    def kids(slot):
        """The composite relations' children of `slot`, in the drawing's order: (child, its field's name, locked)."""
        layer = es[slot]["layerId"]
        out = []
        for r in RELATIONS:
            if r["parent"] != layer or r.get("onDelete") != "cascade":
                continue
            name = field_name(r["child"], r["field"])
            for c in order:
                if es[c]["layerId"] == r["child"] and es[c]["attrs"].get(name) == uid(slot):
                    out.append((c, name, r["child"] in locked))
        return out

    def below(slot, seen):
        for c, _, _ in kids(slot):
            if c not in seen:
                seen.add(c)
                below(c, seen)
        return seen

    def follow(slot, new):
        nonlocal skipped_children, made_children
        for c, name, shut in kids(slot):
            if c in done:
                continue
            if shut:
                skipped_children += 1
                continue
            cn = copy_of(c, name, new)
            made_children += 1
            follow(c, cn)

    firsts = list(dict.fromkeys(slots))
    open_ = [s for s in firsts if es[s]["layerId"] not in locked]
    skipped = len(firsts) - len(open_)
    # An object that is a composite child of another one given is copied with it only.
    under = set()
    if children:
        for s in open_:
            under |= below(s, set())
    for s in open_:
        if s in done or s in under:
            continue
        new = copy_of(s)
        if children:
            follow(s, new)
    warnings = []
    if skipped:
        warnings.append({"code": "layer_locked", "message": f"{skipped} nesne kilitli katmanda olduğu için atlandı. "
                                                              "Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"})
    if skipped_children:
        warnings.append({"code": "layer_locked", "message": f"Kilitli katmandaki {skipped_children} alt kayıt çoğaltılmadı; "
                                                              "katmanın kilidini Katmanlar panelinden açın.", "path": "uids"})
    for form, name in sorted(cleared):
        warnings.append({"code": "unique_cleared", "message": f"“{form}” formunda tekil olan “{name}” alanı kopyalarda boş bırakıldı.",
                         "path": "uids"})
    return copies, es, warnings, made_children


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_, result, expect=None):
    s = {"op": op, "input": input_, "result": result}
    if expect is not None:
        s["expect"] = expect
    return s


def dup_case(name, slots, children=True, locked=(), redo=False):
    copies, es, warnings, made = duplicate(slots, children, locked)
    input_ = {"uids": [uid(s) for s in slots]}
    if not children:
        input_["children"] = False
    before_ids = sorted(e["id"] for e in ENTITIES)
    after_ids = sorted(es)
    new = {str(c["id"]): es[c["id"]] for c in copies}
    expect = {"ids": after_ids, "entities": new, "canUndo": True, "revision": "changed"}
    plan = {"count": len(copies), "children": made, "revision": "$current"}
    steps = tie_steps(locked) + [
        step("plan", input_, completed(plan, warnings)),
        step("execute", input_, completed({"copies": copies, "revision": "$current"}, warnings), expect),
        {"op": "undo", "returns": LABEL, "expect": {"ids": before_ids}},
    ]
    if redo:
        steps.append({"op": "redo", "returns": LABEL, "expect": {"ids": after_ids, "entities": new}})
    return {"name": name, "steps": steps}


def refused(name, input_, code, message, path, locked=()):
    return {"name": name, "steps": tie_steps(locked) + [step("execute", input_, failed(code, message, path), {"ids": sorted(e["id"] for e in ENTITIES)})]}


NOT_UID = "“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). " \
          "Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
GONE = "0192f5a1-25a0-7000-8000-00000000dead"


def cases():
    out = [
        dup_case("bina katları ve katların odalarıyla: alanlar kopyanın kimliğini alır, tekil kod boş kalır, malik kopyalanmaz; "
                 "tek geri alma adımı “Kaydı çoğalt”, yinelenir", [1], redo=True),
        dup_case("tek katlı bina", [2]),
        dup_case("iki bina sırasıyla", [1, 2]),
        dup_case("iki bina ters sırayla", [2, 1]),
        dup_case("alt kayıtsız: yalnız bina", [1], children=False),
        dup_case("yalnız bir kat: odasıyla, binası değil", [3]),
        dup_case("kat ve binası birlikte: kat binasıyla bir kez kopyalanır", [1, 3]),
        dup_case("önce kat sonra binası: kat yalnız binasıyla kopyalanır", [3, 1]),
        dup_case("oda: alt kaydı yok", [6]),
        dup_case("malik kendi başına çoğaltılabilir", [8]),
        dup_case("aynı kimlik iki kez: bir kopya", [2, 2]),
        dup_case("formsuz katmandaki nokta: kodu kalır", [10]),
        dup_case("arşiv katmanı kilitli: arşiv kaydı çoğaltılmaz, sayısı söylenir", [1], locked=("arsiv",)),
        dup_case("kilitli katmandaki nesne atlanır, öbürü çoğaltılır", [10, 2], locked=("eski",)),
        dup_case("kat katmanı kilitli: katlar ve odaları çoğaltılmaz", [1], locked=("kat",)),
    ]
    out.append(refused("boş liste", {"uids": []}, "no_entities",
                       "Çoğaltılacak nesne yok. En az bir nesnenin kalıcı kimliğini verin.", "uids"))
    out.append(refused("geçersiz kimlik", {"uids": [uid(1), "B-1"]}, "invalid_uid", NOT_UID.format(u="B-1"), "uids[1]"))
    out.append(refused("çizimde olmayan nesne", {"uids": [uid(1), GONE]}, "entity_not_found",
                       f"“{GONE}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                       "uids[1]"))
    out.append(refused("hepsi kilitli katmanda", {"uids": [uid(10)]}, "layer_locked",
                       "“Eski binalar” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]", locked=("eski",)))
    out.append(refused("1001 nesne", {"uids": [uid(1)] * 1001}, "invalid_count",
                       "En çok 1000 nesne birlikte çoğaltılır; 1001 verildi.", "uids"))
    out.append(refused("geçersiz beklenen sürüm", {"uids": [uid(1)], "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                       "expectedRevision"))
    out.append({"name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "once"}] + tie_steps() + [
        step("execute", {"uids": [uid(1)], "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. "
                                                                                     "Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                             "path": "expectedRevision", "revision": "$current"}},
             {"ids": sorted(e["id"] for e in ENTITIES)})]})
    out.append({"name": "doğrulama: geçerli girdi hiçbir şey yazmaz", "steps": tie_steps() + [
        step("validate", {"uids": [uid(1)]}, completed(None, duplicate([1])[2]), {"ids": sorted(e["id"] for e in ENTITIES)})]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.records.duplicate", "commandVersion": 1,
            "title": "Kaydı çoğalt",
            "note": "Written by scripts/fixtures/duplicate_records_cases.py from docs/adr/0415; do not edit by hand.",
            "setup": setup(), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({len(build()['cases'])} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
