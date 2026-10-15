"""Denetle: shared cases of cad.drawing.audit (docs/adr/0306).

    python3 scripts/fixtures/audit_command_cases.py           # writes the file
    python3 scripts/fixtures/audit_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.drawing.audit.json. The ADR's rules, computed here without KentOS code, over a drawing
that carries what the reader lets through (references to an object that is not in the drawing, styles the project
does not have):

- found, object by object in the drawing's order and for each in this order: a linked text whose object is gone
  (brokenLink), a hatch one of whose boundary objects is gone (brokenHatch), a dimension's live reference to an object
  that is gone (brokenDimension), a text naming a text style the project lacks (missingTextStyle), a dimension naming
  a dimension style it lacks (missingDimensionStyle);
- repaired with `fix` (one undo step “Denetle”): the link goes (labelOf, labelScale), the association goes, the
  reference breaks as `object_gone`, the missing style goes (Standart); an object on a locked layer is left, counted in
  a warning;
- unused, in this order: the layers and groups nothing is on or under (not the active one, not locked; tree order,
  their paths with “ / ”), the definitions nothing places (definitions' order), the text and dimension styles nothing
  names (an object of the drawing or of a definition), in their tables' order; `purge` removes the styles of the kinds
  named (a project setting, not an undo step).
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.drawing.audit.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
LABEL = "Denetle"
GHOST = "0192f5a2-0000-7000-8000-00000000ffff"
GHOST2 = "0192f5a2-0000-7000-8000-00000000fffe"
ROGAR = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e01"
OLD_BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e02"
TS_USED = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101"
TS_FREE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0102"
DS_USED = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001"
DS_FREE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5002"
MISSING = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"


def U(i):
    return f"$uidOf:{i}"


def p(x, y):
    return {"x": x, "y": y}


def node(i, name, locked=False, children=None):
    return {"id": i, "name": name, "type": "group" if children is not None else "layer", "visible": True,
            "locked": locked, "expanded": True, "style": dict(STYLE), "children": children or []}


def base(i, layer="cizim"):
    return {"id": i, "layerId": layer, "attrs": {}}


def text(i, at, words, layer="cizim", **more):
    return {"kind": "text", **base(i, layer), "p": at, "text": words, "height": 1, "rotation": 0, **more}


def dimension(i, y, **more):
    return {"kind": "dimension", **base(i), "a": p(0, y), "b": p(10, y), "offset": 1.5, "height": 0.25, **more}


def line(i):
    return {"kind": "line", **base(i), "a": p(0, 0), "b": p(10, 0)}


def insert(i):
    return {"kind": "insert", **base(i), "block": ROGAR, "p": p(40, 0), "scale": 1, "rotation": 0}


ENTITIES = [
    line(1),
    text(2, p(0, 5), "101", labelOf=GHOST, labelScale=500, textStyle=MISSING),
    text(3, p(0, 8), "Başlık", textStyle=TS_USED),
    {"kind": "hatch", **base(4), "ring": [p(20, 0), p(30, 0), p(30, 10), p(20, 10)],
     "pattern": {"type": "solid", "angle": 0, "spacing": 1}, "assoc": {"outer": GHOST2, "seed": p(25, 5)}},
    dimension(5, -2, refs={"a": {"uid": GHOST, "place": {"kind": "start"}},
                           "b": {"uid": GHOST2, "place": {"kind": "end"}, "broken": "object_gone"}}, dimStyle=DS_USED),
    dimension(6, -6, dimStyle=MISSING),
    insert(7),
    text(8, p(40, 10), "Kilitli", layer="kilitli", labelOf=GHOST, labelScale=500),
]
IDS = [e["id"] for e in ENTITIES]
TREE = [node("cizim", "Çizim"), node("bos", "Boş"), node("eski", "Eski", children=[node("eski-yol", "Eski yol")]),
        node("kilitli", "Kilitli", locked=True)]
BLOCKS = [
    {"id": ROGAR, "name": "Rögar", "base": p(0, 0),
     "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": p(0, 0), "r": 0.5}]},
    {"id": OLD_BLOCK, "name": "Eski blok", "base": p(0, 0),
     "entities": [{"kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": p(0, 0), "b": p(1, 0)}]},
]
TEXT_STYLES = [{"id": TS_USED, "name": "Başlık", "font": "arimo", "bold": True},
               {"id": TS_FREE, "name": "Eski yazı", "font": "overpass"}]
DIM_STYLES = [{"id": DS_USED, "name": "Mimari", "height": 3.5}, {"id": DS_FREE, "name": "Eski ölçü", "height": 2.5}]
UNUSED_TREE = [("layer", "bos", "Boş"), ("layer", "eski-yol", "Eski / Eski yol"), ("group", "eski", "Eski")]


def setup(entities=None, text_styles=None, dim_styles=None):
    return {
        "format": "kentos.document", "version": 1, "name": "Denetle",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad",
                     "textStyles": TEXT_STYLES if text_styles is None else text_styles,
                     "dimensionStyles": DIM_STYLES if dim_styles is None else dim_styles},
        "origin": p(0, 0),
        "layers": TREE,
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES if entities is None else entities),
        "styles": {"items": [], "categories": []},
        "blocks": BLOCKS,
    }


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

MESSAGES = {
    "brokenLink": "Bağlı yazının nesnesi çizimde yok; onarım bağı koparır, yazı kalır.",
    "brokenHatch": "İlişkili taramanın sınır nesnelerinden biri çizimde yok; onarım ilişkiyi koparır, tarama kalır.",
    "brokenDimension": "İlişkili ölçünün bağlı olduğu nesne çizimde yok; onarım bağı “nesne yok” nedeniyle koparır, "
                       "nokta yerinde kalır.",
    "missingTextStyle": "Yazının stili projede yok; onarım yazıyı Standart'a bağlar, görünüşü kalır.",
    "missingDimensionStyle": "Ölçünün stili projede yok; onarım ölçüyü Standart'a bağlar, görünüşü kalır.",
}
# Every reference in the setup points at an object that is not in it: the ghosts.
GONE = {GHOST, GHOST2}


class State:
    """The drawing as the steps leave it."""

    def __init__(self, entities=None, text_styles=None, dim_styles=None):
        self.entities = copy.deepcopy(ENTITIES if entities is None else entities)
        self.text_styles = copy.deepcopy(TEXT_STYLES if text_styles is None else text_styles)
        self.dim_styles = copy.deepcopy(DIM_STYLES if dim_styles is None else dim_styles)
        self.locked = {"kilitli"}

    def ids(self):
        return [e["id"] for e in self.entities]

    def faults(self, e):
        out = []
        text_ids = {s["id"] for s in self.text_styles}
        dim_ids = {s["id"] for s in self.dim_styles}
        if e["kind"] == "text":
            if e.get("labelOf") in GONE:
                out.append("brokenLink")
            if "textStyle" in e and e["textStyle"] not in text_ids:
                out.append("missingTextStyle")
        if e["kind"] == "hatch" and "assoc" in e:
            a = e["assoc"]
            if any(x in GONE for x in [a["outer"], *a.get("islands", []), *a.get("cutouts", [])]):
                out.append("brokenHatch")
        if e["kind"] == "dimension":
            if any("broken" not in r and r["uid"] in GONE for r in e.get("refs", {}).values()):
                out.append("brokenDimension")
            if "dimStyle" in e and e["dimStyle"] not in dim_ids:
                out.append("missingDimensionStyle")
        return out

    @staticmethod
    def repaired(e, faults):
        out = copy.deepcopy(e)
        for f in faults:
            if f == "brokenLink":
                out.pop("labelOf")
                out.pop("labelScale", None)
            elif f == "missingTextStyle":
                out.pop("textStyle")
            elif f == "brokenHatch":
                out.pop("assoc")
            elif f == "brokenDimension":
                for r in out["refs"].values():
                    if "broken" not in r and r["uid"] in GONE:
                        r["broken"] = "object_gone"
            elif f == "missingDimensionStyle":
                out.pop("dimStyle")
        return out

    def unused(self, purge):
        placed = {e["block"] for e in self.entities if e["kind"] == "insert"}
        used_text = {e["textStyle"] for e in self.entities if e["kind"] == "text" and "textStyle" in e}
        used_dims = {e["dimStyle"] for e in self.entities if e["kind"] == "dimension" and "dimStyle" in e}
        out = [{"kind": k, "id": i, "name": n, "purged": False} for k, i, n in UNUSED_TREE]
        out += [{"kind": "block", "id": b["id"], "name": b["name"], "purged": False}
                for b in BLOCKS if b["id"] not in placed]
        free_text = [s for s in self.text_styles if s["id"] not in used_text]
        free_dims = [s for s in self.dim_styles if s["id"] not in used_dims]
        out += [{"kind": "textStyle", "id": s["id"], "name": s["name"], "purged": "textStyles" in purge}
                for s in free_text]
        out += [{"kind": "dimensionStyle", "id": s["id"], "name": s["name"], "purged": "dimensionStyles" in purge}
                for s in free_dims]
        return out, free_text, free_dims

    def run(self, fix, purge):
        findings, after, held = [], {}, 0
        for e in self.entities:
            faults = self.faults(e)
            if not faults:
                continue
            locked = e["layerId"] in self.locked
            findings += [{"code": c, "uid": U(e["id"]), "message": MESSAGES[c], "fixed": fix and not locked}
                         for c in faults]
            if fix and locked:
                held += 1
            elif fix:
                after[e["id"]] = self.repaired(e, faults)
        unused, free_text, free_dims = self.unused(purge)
        warnings = []
        if held:
            warnings.append({"code": "layer_locked", "message": f"{held} nesne kilitli katmanda olduğu için onarılmadı. "
                             "Onarmak için katmanın kilidini Katmanlar panelinden açın.", "path": "fix"})
        purged_text = free_text if "textStyles" in purge else []
        purged_dims = free_dims if "dimensionStyles" in purge else []
        return findings, unused, after, warnings, purged_text, purged_dims

    def write(self, after, purged_text, purged_dims):
        self.entities = [after.get(e["id"], e) for e in self.entities]
        gone = {s["id"] for s in purged_text + purged_dims}
        self.text_styles = [s for s in self.text_styles if s["id"] not in gone]
        self.dim_styles = [s for s in self.dim_styles if s["id"] not in gone]


def step(state, inp, op="execute"):
    findings, unused, after, warnings, purged_text, purged_dims = state.run(inp.get("fix", False), inp.get("purge", []))
    if op == "validate":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": None, "warnings": warnings},
                "expect": {"revision": "same"}}
    out = {"findings": findings, "unused": unused, "changed": [U(i) for i in after], "revision": "$current"}
    s = {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings}}
    if op == "plan":
        s["expect"] = {"ids": state.ids(), "revision": "same"}
        return s
    if after:
        s["expect"] = {"ids": state.ids(), "entities": {str(i): e for i, e in after.items()}, "revision": "changed",
                       "canUndo": True}
    elif purged_text or purged_dims:
        s["expect"] = {"ids": state.ids(), "revision": "changed"}
    else:
        s["expect"] = {"ids": state.ids(), "revision": "same"}
    state.write(after, purged_text, purged_dims)
    return s


def other(command, inp):
    return {"op": "execute", "command": command, "input": inp, "result": "$any"}


def case(name, *inputs, state=None):
    state = state or State()
    return {"name": name, "steps": [step(state, inp) for inp in inputs]}


def by_id(state, *ids):
    return {str(e["id"]): e for e in state.entities if e["id"] in ids}


def deleted(name, uids, *inputs):
    state = State()
    steps = [other("cad.entities.delete", {"uids": [U(i) for i in uids]})]
    # Delete skips what is on a locked layer.
    state.entities = [e for e in state.entities if e["id"] not in uids or e["layerId"] in state.locked]
    steps += [step(state, inp) for inp in inputs]
    return {"name": name, "steps": steps}


def cases():
    out = [
        case("Yalnız denetim: bozuk ilişkiler ve kayıp stiller nesne nesne, kullanılmayan katmanlar, grup, blok ve "
             "stiller; hiçbir şey yazılmaz", {}),
        case("Onarımla yeniden denetim: yalnız kilitli katmandaki kalır", {"fix": True}, {}),
        case("Kullanılmayan yazı stilleri temizlenir (proje ayarı, geri alma adımı değil); yeniden denetimde yoktur",
             {"purge": ["textStyles"]}, {}),
        case("Kullanılmayan ölçü stilleri temizlenir", {"purge": ["dimensionStyles"]}, {}),
        case("Onarım ve iki tür stilin temizliği birlikte", {"fix": True, "purge": ["textStyles", "dimensionStyles"]}, {}),
        case("Kayıp stil Standart olunca kullanılan stiller değişmez: yalnız adı geçmeyenler temizlenir",
             {"fix": True}, {"purge": ["textStyles", "dimensionStyles"]}),
        case("Boş temizlik listesi bir şey silmez", {"purge": []}),
        case("İki kez onarım: ikincisi bir şey yazmaz, uyarı kalır", {"fix": True}, {"fix": True}),
        case("Temizlenecek stil kalmayınca temizlik yazmaz", {"purge": ["textStyles", "dimensionStyles"]},
             {"purge": ["textStyles", "dimensionStyles"]}),
        case("Beklenen sürüm şimdiki sürümse onarım yazılır", {"fix": True, "expectedRevision": "$current"}),
    ]

    state = State()
    first = step(state, {"fix": True})
    out.append({"name": "Onarım tek adım “Denetle”: geri alınır ve yinelenir", "steps": [
        first,
        {"op": "undo", "returns": LABEL, "expect": {"ids": IDS, "entities": by_id(State(), 2, 4, 5, 6)}},
        {"op": "redo", "returns": LABEL, "expect": {"ids": IDS, "entities": by_id(state, 2, 4, 5, 6)}},
    ]})

    state = State()
    steps = [step(state, {"fix": True}),
             other("cad.layers.set", {"operation": "locked", "layer": "kilitli", "value": False})]
    state.locked = set()
    steps.append(step(state, {"fix": True}))
    out.append({"name": "Kilit açılınca kilitli katmandaki yazı da onarılır", "steps": steps})

    out.append(deleted("Yerleştirmesi silinen blok kullanılmayanlara katılır (tanımların sırasıyla)", [7], {}))
    out.append(deleted("Stilini kullanan nesneler silinince stil kullanılmaz olur ve temizlenir", [3, 5],
                       {"purge": ["textStyles", "dimensionStyles"]}, {}))
    out.append(deleted("Onarılacak nesne kalmayınca onarım bir şey yazmaz; kilitli katmandaki silinmez, söylenir", [2, 4, 5, 6, 8], {"fix": True}))

    clean = [line(1), text(2, p(0, 8), "Başlık", textStyle=TS_USED), insert(3), dimension(4, -2, dimStyle=DS_USED)]
    state = State(clean, TEXT_STYLES[:1], DIM_STYLES[:1])
    c = case("Temiz çizim: bulgu yok, yalnız kullanılmayan katmanlar, grup ve blok; onarım bir şey yazmaz",
             {}, {"fix": True, "purge": ["textStyles", "dimensionStyles"]}, state=state)
    c["setup"] = setup(clean, TEXT_STYLES[:1], DIM_STYLES[:1])
    out.append(c)

    out.append({"name": "Plan onarımı ve temizliği gösterir, hiçbir şey yazılmaz", "steps": [
        step(State(), {"fix": True, "purge": ["textStyles"]}, op="plan"),
        step(State(), {}, op="plan"),
    ]})
    out.append({"name": "Doğrulama kilitli katmandakinin uyarısını yalnız onarımda verir", "steps": [
        step(State(), {"fix": True}, op="validate"),
        step(State(), {}, op="validate"),
        step(State(), {"purge": ["textStyles"]}, op="validate"),
    ]})
    out.append({"name": "Beklenen sürüm sayı değil: invalid_revision", "steps": [
        {"op": "execute", "input": {"fix": True, "expectedRevision": "on iki"}, "result": {"status": "failed", "error": {
            "code": "invalid_revision", "message": "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir "
            "tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "path": "expectedRevision"}},
         "expect": {"ids": IDS, "revision": "same", "canUndo": False}}]})
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        other("cad.entities.transform", {"uids": [U(1)], "transform": {"kind": "move", "dx": 1, "dy": 0}}),
        {"op": "execute", "input": {"fix": True, "purge": ["textStyles"], "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline "
                        "göre yeniden hazırlayın."}},
         "expect": {"revision": "same"}}]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.drawing.audit", "commandVersion": 1,
            "title": "Denetle: bozuk ilişkiler, kayıp stiller, onarım, kullanılmayan adlı kaynaklar ve stillerin temizliği",
            "note": "ADR 0306. Bağımsız başvuru: scripts/fixtures/audit_command_cases.py. Kurulumdaki çizim okuyucunun "
                    "geçirdiği bozuklukları taşır: çizimde olmayan nesneye bağlı yazı, tarama ve ölçü bağı; projede olmayan "
                    "stiller.",
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
