"""Writes the shared cases of Katmanları bul and Katmanları çevir (docs/adr/0311 §4, §5):

- fixtures/commands/v1/cad.layers.find.json      (`cad.layers.find` v1: a name pattern and the layers' properties)
- fixtures/commands/v1/cad.layers.translate.json (`cad.layers.translate` v1: layers merged into a standard's by rules)

The name pattern is AutoCAD's `wcmatch`, written here again from its documentation (`*`, `?`, `#`, `@`, `.`, `[..]`,
`[~..]`, `` ` ``, `,`, a leading `~`), letters folded as Turkish writes them; the properties and the rules from the
contract (crates/shared/contracts/src/cad_layer_find.rs) and the ADR, the tree and the objects as plain JSON, without
KentOS code. The tree, its helpers and the steps' form are layer_tree_command_cases.py's. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_find_cases.py           # writes the files
    python3 scripts/fixtures/layer_find_cases.py --check   # compares them with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


T = _load("layer_tree_command_cases")
node, setup, step, completed, failed, VALID, unchanged, refused = (T.node, T.setup, T.step, T.completed, T.failed, T.VALID,
                                                                   T.unchanged, T.refused)
walk, get, parent_of, removed, inserted, changed = T.walk, T.get, T.parent_of, T.removed, T.inserted, T.changed
STYLE, BLUE, DEFAULT, OSM, X, Y = T.STYLE, T.BLUE, T.DEFAULT, T.OSM, T.X, T.Y
fold = T.fold
INVALID_REVISION, CONFLICT = T.INVALID_REVISION, T.CONFLICT

# ── AutoCAD's wcmatch, from its documentation ────────────────────────────────


def fold_char(c):
    return "ı" if c == "I" else "i" if c == "İ" else c.lower()


def split(pattern):
    """The alternatives: split at commas neither escaped nor in brackets."""
    out, cur, i, in_class = [], "", 0, False
    while i < len(pattern):
        c = pattern[i]
        if c == "`" and i + 1 < len(pattern):
            cur += c + pattern[i + 1]
            i += 2
            continue
        if c == "[" and not in_class:
            in_class = True
        elif c == "]" and in_class:
            in_class = False
        elif c == "," and not in_class:
            out.append(cur)
            cur = ""
            i += 1
            continue
        cur += c
        i += 1
    out.append(cur)
    return out


def compile_alt(alt):
    """(negated, tokens) or None when it does not read. A token is ('c', char), ('#',), ('@',), ('.',), ('?',),
    ('*',) or ('[', negated, [(lo, hi)])."""
    neg = alt.startswith("~")
    body = alt[1:] if neg else alt
    toks, i = [], 0
    while i < len(body):
        c = body[i]
        i += 1
        if c == "`":
            if i >= len(body):
                return None
            toks.append(("c", fold_char(body[i])))
            i += 1
        elif c in "#@.?*":
            toks.append((c,))
        elif c == "[":
            cneg = i < len(body) and body[i] == "~"
            if cneg:
                i += 1
            ranges, closed = [], False
            while i < len(body):
                a = body[i]
                i += 1
                if a == "]" and ranges:
                    closed = True
                    break
                if a == "`" and i < len(body):
                    a = body[i]
                    i += 1
                a = fold_char(a)
                if i + 1 < len(body) and body[i] == "-" and body[i + 1] != "]":
                    b = fold_char(body[i + 1])
                    i += 2
                    ranges.append((min(a, b), max(a, b)))
                else:
                    ranges.append((a, a))
            if not closed:
                return None
            toks.append(("[", cneg, ranges))
        else:
            toks.append(("c", fold_char(c)))
    return neg, toks


def one(tok, c):
    k = tok[0]
    if k == "c":
        return tok[1] == c
    if k == "#":
        return c in "0123456789"
    if k == "@":
        return c.isalpha()
    if k == ".":
        return not c.isalnum()
    if k == "?":
        return True
    if k == "[":
        return any(lo <= c <= hi for lo, hi in tok[2]) != tok[1]
    return False


def whole(toks, name):
    """Recursive matching of the whole name."""
    if not toks:
        return not name
    if toks[0] == ("*",):
        return any(whole(toks[1:], name[i:]) for i in range(len(name) + 1))
    return bool(name) and one(toks[0], name[0]) and whole(toks[1:], name[1:])


def problem(pattern):
    if not pattern.strip():
        return "örüntü boş"
    for alt in split(pattern):
        if compile_alt(alt) is None:
            return "bad"
    return None


def matches(pattern, name):
    folded = "".join(fold_char(c) for c in name)
    for alt in split(pattern):
        c = compile_alt(alt)
        if c is None:
            continue
        neg, toks = c
        if whole(toks, folded) != neg:
            return True
    return False


# ── The drawing ──────────────────────────────────────────────────────────────

RED = {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.5}
DASHED = {"color": "#56B6C2", "lineType": "dashed", "lineWeight": 0.18}


def layers():
    return [
        node("mimari", "Mimari", "group", [
            node("duvar", "A-DUVAR", style=RED),
            node("duvar-eski", "duvar_eski"),
            node("kapi", "A-KAPI", plot=False),
            node("pencere", "a-pencere", frozen=True),
            node("aks", "A-AKS", style=DASHED, locked=True),
        ]),
        node("wall", "Wall", style=RED),
        node("kat1", "Kat1"),
        node("kat2", "Kat2", visible=False),
        node("ic", "İç mekân"),
        node("isik", "Işık"),
        node("bos", "Boş"),
        node("altlik", "OpenStreetMap", service=OSM),
        node("arsiv", "Arşiv", "group", [node("eski-duvar", "duvar-1"), node("eski-yol", "yol 2")], frozen=True),
        node("koruma", "Koruma", "group", [node("sit", "duvar sit")], locked=True),
    ]


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


def ent(id_, layer, k=0):
    return {"kind": "polygon", "id": id_, "layerId": layer, "attrs": {}, "pts": rect(X + 10 * k, Y, 5, 5)}


ENTITIES = [ent(1, "duvar", 0), ent(2, "duvar", 1), ent(3, "duvar-eski", 2), ent(4, "kapi", 3), ent(5, "wall", 4),
            ent(6, "wall", 5), ent(7, "wall", 6), ent(8, "kat1", 7), ent(9, "kat2", 8), ent(10, "ic", 9),
            ent(11, "eski-duvar", 10), ent(12, "sit", 11), ent(13, "pencere", 12)]
IDS = [e["id"] for e in ENTITIES]


def doc_setup(tree=None, active="kat1"):
    return setup(layers() if tree is None else tree, ENTITIES, active)


# ── cad.layers.find ──────────────────────────────────────────────────────────


def chain(nodes, id_):
    out, n = [], get(nodes, id_)
    while n is not None:
        out.append(n)
        n = parent_of(nodes, n["id"])
    return out


def found_of(nodes, n):
    c = chain(nodes, n["id"])
    return {"id": n["id"], "name": n["name"], "path": T.path_of(nodes, n["id"]),
            "objects": sum(1 for e in ENTITIES if e["layerId"] == n["id"]),
            "color": n["style"]["color"], "lineType": n["style"]["lineType"], "lineWeight": n["style"]["lineWeight"],
            "on": all(x["visible"] for x in c), "frozen": any(x.get("frozen") is True for x in c),
            "locked": any(x["locked"] for x in c), "plot": all(x.get("plot") is not False for x in c)}


def find(nodes, q):
    """The layers `q` asks for, in the tree's order."""
    scope = nodes
    if "group" in q:
        scope = [get(nodes, q["group"])]
    out = []
    for n, _ in walk(scope):
        if n["type"] != "layer":
            continue
        f = found_of(nodes, n)
        empty = f["objects"] == 0 and "service" not in n
        ok = (("name" not in q or matches(q["name"], n["name"]))
              and ("color" not in q or fold(q["color"]) == fold(n["style"]["color"]))
              and ("lineType" not in q or q["lineType"] == n["style"]["lineType"])
              and ("lineWeight" not in q or abs(q["lineWeight"] - n["style"]["lineWeight"]) < 1e-9)
              and all(k not in q or q[k] == f[k] for k in ("on", "frozen", "locked", "plot"))
              and ("empty" not in q or q["empty"] == empty))
        if ok:
            out.append(f)
    return out


def found(q, tree=None):
    t = layers() if tree is None else tree
    return completed({"layers": find(t, q), "revision": "$current"})


def find_case(name, q):
    return {"name": name, "steps": [step("execute", q, found(q), unchanged(layers(), IDS)),
                                    step("plan", q, found(q), {"revision": "same"})]}


PATTERN = "“{p}” bir katman adı örüntüsü değil: {why}. * herhangi bir dizi, ? tek karakter, [..] küme olur."


def find_cases():
    t = layers()
    out = [
        find_case("Hepsi: koşulsuz bütün katmanlar, ağacın sırasıyla; gruplar sayılmaz", {}),
        find_case("Ad örüntüsü: A-* büyük küçük harf ayırmadan", {"name": "A-*"}),
        find_case("Ad örüntüsü: içinde geçen (*duvar*)", {"name": "*duvar*"}),
        find_case("Ad örüntüsü: seçenekler virgülle (duvar*,wall*)", {"name": "duvar*,wall*"}),
        find_case("Ad örüntüsü: # rakam (Kat#)", {"name": "Kat#"}),
        find_case("Ad örüntüsü: ? tek karakter (A-???)", {"name": "A-???"}),
        find_case("Ad örüntüsü: küme ve aralık ([a-d]*)", {"name": "[a-d]*"}),
        find_case("Ad örüntüsü: değili (~A-*)", {"name": "~A-*"}),
        find_case("Ad örüntüsü: Türkçe katlama (ı* Işık'ı bulur, i* bulmaz)", {"name": "ı*"}),
        find_case("Ad örüntüsü: Türkçe katlama (i* İç mekân'ı bulur)", {"name": "i*"}),
        find_case("Ad örüntüsü: . harf ve rakam olmayan (duvar.1)", {"name": "duvar.1"}),
        find_case("Ad örüntüsü: ` kaçış (yol` 2)", {"name": "yol` 2"}),
        find_case("Görünenler (on: gözü ve üstündeki grupların gözü açık)", {"on": True}),
        find_case("Gizliler", {"on": False}),
        find_case("Dondurulmuşlar: kendisi ya da grubundan", {"frozen": True}),
        find_case("Kilitliler: kendisi ya da grubundan", {"locked": True}),
        find_case("Yazdırılmayanlar", {"plot": False}),
        find_case("Nesnesizler: servis katmanı boş sayılmaz", {"empty": True}),
        find_case("Renk: büyük küçük harf ayırmadan", {"color": "#e5484d"}),
        find_case("Çizgi tipi ve kalınlık", {"lineType": "dashed", "lineWeight": 0.18}),
        find_case("Grup: yalnız altındakiler, adla birlikte", {"group": "mimari", "name": "A-*"}),
        find_case("Koşulların hepsi: dondurulmamış ve dolu duvar katmanları", {"name": "*duvar*", "frozen": False, "empty": False}),
        {"name": "Doğrula: yazmaz", "steps": [step("validate", {"name": "A-*"}, VALID, unchanged(t, IDS))]},
    ]
    out.append(refused("örüntü: kapanmayan küme", {"name": "[ab"}, "invalid_pattern",
                       PATTERN.format(p="[ab", why="[ ile açılan küme ] ile kapanmıyor"), "name", t, IDS))
    out.append(refused("örüntü: boş", {"name": "  "}, "invalid_pattern", PATTERN.format(p="  ", why="örüntü boş"), "name", t, IDS))
    out.append(refused("renk: geçersiz", {"color": "kırmızı"}, "invalid_color",
                       "“kırmızı” bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın.", "color", t, IDS))
    out.append(refused("kalınlık: aralık dışı", {"lineWeight": -1}, "invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight", t, IDS))
    out.append(refused("grup: çizimde yok", {"group": "yok"}, "layer_not_found",
                       "“yok” kimlikli katman ya da grup çizimde yok. Var olan bir düğümün kimliğini verin.", "group", t, IDS))
    return out


# ── cad.layers.translate ─────────────────────────────────────────────────────


def holding(nodes):
    return [n for n, _ in walk(nodes) if n["type"] == "layer"]


def translate(nodes, entities, active, rules, parent=None):
    """docs/adr/0311 §5 as plain JSON: the tree, the objects and the active layer after, and the answer (new layers
    `$layer:k` in the order they are added)."""
    hold = holding(nodes)
    targets = [next((n for n in hold if fold(n["name"]) == fold(r["to"].strip())), None) for r in rules]
    sources = []
    for n in hold:
        if "service" in n:
            continue
        r = next((i for i, rule in enumerate(rules) if matches(rule["from"], n["name"])), None)
        if r is not None and fold(n["name"]) != fold(rules[r]["to"].strip()):
            sources.append((n, r))
    tree, ents, act = copy.deepcopy(nodes), copy.deepcopy(entities), active
    answer = {"changed": False, "translations": [], "created": [], "restyled": [], "objects": 0, "revision": "$current"}
    if not sources:
        return tree, ents, act, answer

    def given(style, rule):
        s = dict(style)
        for k in ("color", "lineType", "lineWeight"):
            if k in rule:
                s[k] = rule[k]
        return s

    standard = {}
    for n, r in sources:
        if r in standard:
            continue
        rule = rules[r]
        if targets[r] is not None:
            t = get(tree, targets[r]["id"])
            st = given(t["style"], rule)
            if st != t["style"]:
                t["style"] = st
                answer["restyled"].append(t["id"])
            standard[r] = (t["id"], t["name"])
        else:
            new_id = f"$layer:{len(answer['created']) + 1}"
            base = {k: v for k, v in n["style"].items() if k in ("color", "lineType", "lineWeight")}
            new = node(new_id, rule["to"].strip(), style=given(base, rule))
            tree = inserted(tree, new, parent)
            answer["created"].append(new_id)
            standard[r] = (new_id, rule["to"].strip())
    for n, r in sources:
        to, to_name = standard[r]
        moved = [e for e in ents if e["layerId"] == n["id"]]
        for e in moved:
            e["layerId"] = to
        if act == n["id"]:
            act = to
        tree = removed(tree, n["id"])
        answer["objects"] += len(moved)
        answer["translations"].append({"layer": n["id"], "name": n["name"], "rule": r, "to": to, "toName": to_name, "objects": len(moved)})
    answer["changed"] = True
    return tree, ents, act, answer


def unlocked_layers():
    """The tree without its locks: every rule may take part."""
    t = layers()
    for n, _ in walk(t):
        n["locked"] = False
    return t


def translated(name, rules, tree=None, active="kat1", parent=None, extra=None, warnings=()):
    t = unlocked_layers() if tree is None else tree
    after, ents, act, answer = translate(t, ENTITIES, active, rules, parent)
    input_ = {"rules": rules}
    if parent is not None:
        input_["parent"] = parent
    by = {str(e["id"]): e for e in ents}
    if answer["changed"]:
        expect = {"ids": IDS, "layers": after, "entities": by, "activeLayer": act, "canUndo": True, "dirty": True}
        steps = [step("plan", input_, completed(answer, warnings), {"revision": "same", "layers": t}),
                 step("execute", input_, completed(answer, warnings), expect),
                 step("undo", returns="Katmanları çevir", expect={"layers": t, "entities": {str(e["id"]): e for e in ENTITIES}})]
    else:
        steps = [step("execute", input_, completed(answer), unchanged(t, IDS))]
    case = {"name": name, "setup": setup(t, ENTITIES, active), "steps": steps}
    if extra:
        case.update(extra)
    return case


def translate_cases():
    t = unlocked_layers()
    out = [
        translated("Var olan standart katmana: nesneler taşınır, kaynak silinir; tek adım “Katmanları çevir”",
                   [{"from": "duvar*", "to": "A-DUVAR"}]),
        translated("Standart katman yoksa eklenir, kuralın stiliyle (verilmeyenler ilk kaynağınki)",
                   [{"from": "wall", "to": "A-WALL", "color": "#0090FF"}]),
        translated("Var olan standart katman kuralın stilini alır", [{"from": "duvar_*", "to": "A-DUVAR", "lineWeight": 0.7}]),
        translated("İlk uyan kural kazanır; adı to olan katman kendisidir",
                   [{"from": "A-*", "to": "A-DUVAR"}, {"from": "*", "to": "ÖTEKİ"}]),
        translated("Seçenekli örüntü ve büyük küçük harf: kat# Kat1 ve Kat2'yi alır", [{"from": "kat#", "to": "KATLAR"}]),
        translated("Türkçe katlama: standart katman adı katlanarak bulunur", [{"from": "İç*", "to": "ışık"}]),
        translated("Etkin katman kaynaksa standart katman etkin olur", [{"from": "Kat1", "to": "Zemin"}]),
        translated("Yeni katmanlar verilen grubun sonuna eklenir", [{"from": "Wall", "to": "A-WALL"}], parent="mimari"),
        translated("Birden çok kural, birden çok yeni katman sırasıyla",
                   [{"from": "Wall", "to": "B-1"}, {"from": "duvar-1", "to": "B-2"}]),
        translated("Servis katmanı kaynak olmaz", [{"from": "Open*", "to": "Altlık"}]),
        translated("Hiçbir katman uymazsa değişen yok", [{"from": "yok*", "to": "Yok"}]),
        translated("Nesnesiz katman da çevrilir", [{"from": "Boş", "to": "A-DUVAR"}]),
        translated("Dondurulmuş ve gizli katman da çevrilir", [{"from": "Kat2", "to": "A-DUVAR"}, {"from": "a-pen*", "to": "A-DUVAR"}]),
        translated("Gizli standart katmana taşınan nesneler söylenir", [{"from": "Kat1", "to": "kat2"}],
                   warnings=[{"code": "layer_hidden", "message": "“Kat2” katmanı gizli; taşınan nesneler görünmeyecek.", "path": "layerId"}]),
        {"name": "Doğrula: yazmaz", "steps": [step("validate", {"rules": [{"from": "Kat#", "to": "B"}]}, VALID, unchanged(layers(), IDS))]},
    ]
    lt = layers()
    out.append(refused("kuralsız", {"rules": []}, "no_rules",
                       "En az bir çevirme kuralı verin (rules): hangi katmanların (from) hangi katmana (to) gideceği.", "rules", lt, IDS))
    out.append(refused("standart katmanın adı boş", {"rules": [{"from": "A-*", "to": " "}]}, "empty_name",
                       "Standart katmanın adı boş olamaz; bir ad yazın.", "rules[0].to", lt, IDS))
    out.append(refused("örüntü okunmuyor", {"rules": [{"from": "A-[", "to": "B"}]}, "invalid_pattern",
                       PATTERN.format(p="A-[", why="[ ile açılan küme ] ile kapanmıyor"), "rules[0].from", lt, IDS))
    out.append(refused("renk geçersiz", {"rules": [{"from": "A-*", "to": "B", "color": "#12"}]}, "invalid_color",
                       "“#12” bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın.", "rules[0].color", lt, IDS))
    out.append(refused("kalınlık aralık dışı", {"rules": [{"from": "A-*", "to": "B", "lineWeight": 120}]}, "invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.",
                       "rules[0].lineWeight", lt, IDS))
    out.append(T.conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                               step("execute", {"rules": [{"from": "Boş", "to": "Kat1"}]},
                                    completed({"changed": True, "translations": [{"layer": "bos", "name": "Boş", "rule": 0, "to": "kat1",
                                                                                   "toName": "Kat1", "objects": 0}],
                                               "created": [], "restyled": [], "objects": 0, "revision": "$current"}),
                                    {"revision": "changed"}),
                               {"rules": [{"from": "Kat2", "to": "Kat1"}]}, {}))
    out.append(refused("kaynak kilitli (kendisi)", {"rules": [{"from": "A-AKS", "to": "Eksen"}]}, "layer_locked",
                       "“A-AKS” katmanı kilitli; çevrilemez. Kilidini Katmanlar panelinden açın ya da kuralı (“A-AKS”) daraltın.",
                       "rules[0].from", lt, IDS))
    out.append(refused("kaynak kilitli (grubundan)", {"rules": [{"from": "duvar sit", "to": "Sit"}]}, "layer_locked",
                       "“duvar sit” katmanı kilitli; çevrilemez. Kilidini Katmanlar panelinden açın ya da kuralı (“duvar sit”) daraltın.",
                       "rules[0].from", lt, IDS))
    out.append(refused("standart katman kilitli", {"rules": [{"from": "Kat1", "to": "a-aks"}]}, "layer_locked",
                       "Standart katman “A-AKS” kilitli; katmanlar ona çevrilemez. Kilidini Katmanlar panelinden açın.", "rules[0].to", lt, IDS))
    out.append(refused("standart katman servisten", {"rules": [{"from": "Kat1", "to": "OpenStreetMap"}]}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; katmanlar ona çevrilemez. Kurala başka bir ad verin.", "rules[0].to", lt, IDS))
    out.append(refused("grup çizimde yok", {"rules": [{"from": "Kat1", "to": "B"}], "parent": "yok"}, "layer_not_found",
                       "“yok” kimlikli katman ya da grup çizimde yok. Var olan bir grubun kimliğini verin.", "parent", lt, IDS))
    out.append(refused("grup değil", {"rules": [{"from": "Kat1", "to": "B"}], "parent": "kat2"}, "not_a_group",
                       "“Kat2” bir katman; yeni katmanlar yalnız bir grubun içine eklenir. Bir grubun kimliğini verin ya da parent'ı vermeyin (en üst).",
                       "parent", lt, IDS))
    out.append(refused("geçersiz beklenen sürüm", {"rules": [{"from": "Kat1", "to": "B"}], "expectedRevision": "on iki"}, "invalid_revision",
                       INVALID_REVISION, "expectedRevision", lt, IDS))
    return out


def build(command, title, cases, setup_doc):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "Written by scripts/fixtures/layer_find_cases.py from AutoCAD's wcmatch, the contract's rules and docs/adr/0311; do not edit by hand.",
            "setup": setup_doc, "cases": cases}


def files():
    return {
        OUT / "cad.layers.find.json": build("cad.layers.find", "Katmanları bul", find_cases(), doc_setup()),
        OUT / "cad.layers.translate.json": build("cad.layers.translate", "Katmanları çevir", translate_cases(), doc_setup()),
    }


def main() -> int:
    bad = 0
    for path, doc in files().items():
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad += 1
            else:
                print(f"{path.name} güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
