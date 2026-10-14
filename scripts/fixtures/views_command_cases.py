"""The shared cases of the product command cad.views.edit: the project's named
views (docs/adr/0285 §1).

    python3 scripts/fixtures/views_command_cases.py           # writes the file
    python3 scripts/fixtures/views_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.views.edit.json. The rule is written here from
the ADR, without KentOS code: a named view is a name, the world point in the
middle of the area, the world width and height it shows and its twist, the
angle of the screen's x axis from the world's x (east), counter-clockwise, in
radians, kept in [0, 2π) (fmod's remainder, a negative one turned once). Save
keeps a view under a name (one of that name is replaced, with a warning, in
its place), Rename gives a view another name in its place, Delete removes one.
Names are compared trimmed; 1–64 characters, no control character; at most
100 views. The checks, their order, codes, paths and messages are written by
hand from the contract (crates/shared/contracts/src/cad_views.rs).

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys
import unicodedata

TAU = 2.0 * math.pi
REACH = 1.0e7
MOST_SIZE = 2.0e7
MOST_VIEWS = 100
MOST_NAME = 64

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": style, "children": []}


def P(x, y):
    return {"x": x, "y": y}


E0, N0 = 487000, 4420000

ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Parsel": "112/4"}, "pts": [P(E0, N0), P(E0 + 40, N0), P(E0 + 40, N0 + 30), P(E0, N0 + 30)]},
    {"kind": "line", "id": 2, "layerId": "yol", "attrs": {}, "a": P(E0 - 10, N0 - 5), "b": P(E0 + 60, N0 + 35)},
    {"kind": "line", "id": 3, "layerId": "kilitli", "attrs": {}, "a": P(E0, N0 + 50), "b": P(E0 + 20, N0 + 50)},
]
LAYERS = [layer("parsel", "Parsel"), layer("yol", "Yol"), layer("kilitli", "Kilitli katman", locked=True)]
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 1000, "workspace": "cad", "drawingFont": "barlow"}


def setup(settings=SETTINGS):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Ürün komutu",
        "settings": settings,
        "origin": P(E0, N0),
        "layers": LAYERS,
        "activeLayer": "parsel",
        "entities": ENTITIES,
        "styles": {"items": [], "categories": []},
    }


SETUP = setup()
IDS = [e["id"] for e in ENTITIES]

# ── The rule (docs/adr/0285 §1) ────────────────────────────────────────


def norm(a):
    """An angle in [0, 2π): fmod's remainder, a negative one turned once; −0 is 0."""
    r = math.fmod(a, TAU)
    if r < 0.0:
        r += TAU
    if r >= TAU:
        r = 0.0
    return r + 0.0


def name_ok(name):
    name = name.strip()
    return 0 < len(name) <= MOST_NAME and not any(unicodedata.category(c) == "Cc" for c in name)


def view(name, center, width, height, twist=0.0):
    return {"name": name, "center": P(*center), "width": width, "height": height, "twist": norm(twist)}


# ── Results ───────────────────────────────────────────────────────────


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
# After the writes before them the drawing is dirty: a refusal only leaves the revision as it was.
LATER = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same"}
NON_FINITE = "Görünümün sayıları sonlu olmalı; NaN ya da sonsuz bir değer verildi. Sayıyı yeniden verin."
FAR_TEXT = "Görünümün ortası çizimden çok uzak: koordinatlar en çok 10 000 km olur. Çizimdeki bir noktayı verin."
SIZE_TEXT = "Görünümün eni ve boyu 0'dan büyük ve en çok 20 000 km olmalı. Gösterilecek alanın boyunu verin."
NAME_TEXT = "Görünüm adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın."
TOO_MANY = "Projede en çok 100 adlı görünüm olur. Kullanmadığınız birini silin ya da var olanın üzerine kaydedin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
REVISION = "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."


def unknown(name):
    return failed("unknown_view", f"“{name}” adlı bir görünüm yok. Adlı görünümlerden birini seçin ya da geçerli görünümü bu adla kaydedin.", "edit.name")


def exists(name):
    return failed("view_exists", f"“{name}” adlı başka bir görünüm var. Başka bir ad yazın ya da önce onu silin.", "edit.to")


def replaced(name):
    return {"code": "view_replaced", "message": f"“{name}” adlı görünümün yerine yenisi kaydedildi.", "path": "edit.name"}


def run(op, **more):
    return {"op": "execute", "input": {"edit": op, **more}}


def save(name, center, width, height, twist=None):
    op = {"kind": "save", "name": name, "center": P(*center), "width": width, "height": height}
    if twist is not None:
        op["twist"] = twist
    return op


def rename(name, to):
    return {"kind": "rename", "name": name, "to": to}


def delete(name):
    return {"kind": "delete", "name": name}


def copy(v):
    return json.loads(json.dumps(v))


class Project:
    """The named views the cases go through, by the ADR's rule."""

    def __init__(self, views=None):
        self.views = list(views or [])

    def step(self, op, **more):
        """A written step: the rule's new list, and the step's result and expectation."""
        old = copy(self.views)
        warnings = []
        kept = None
        k = op["kind"]
        if k == "save":
            name = op["name"].strip()
            kept = view(name, (op["center"]["x"], op["center"]["y"]), op["width"], op["height"], op.get("twist", 0.0))
            at = next((j for j, v in enumerate(self.views) if v["name"] == name), None)
            if at is None:
                self.views.append(kept)
            else:
                self.views[at] = kept
                warnings.append(replaced(name))
        elif k == "rename":
            name, to = op["name"].strip(), op["to"].strip()
            at = next(j for j, v in enumerate(self.views) if v["name"] == name)
            self.views[at] = {**self.views[at], "name": to}
            kept = self.views[at]
        elif k == "delete":
            name = op["name"].strip()
            self.views = [v for v in self.views if v["name"] != name]
        changed = self.views != old
        output = {"views": copy(self.views), "changed": changed, "revision": "$current"}
        if kept is not None:
            output["view"] = copy(kept)
        step = run(op, **more)
        step["result"] = {"status": "completed", "output": output, "warnings": warnings}
        expect = {"ids": IDS, "settings": {"namedViews": copy(self.views) or None}, "canUndo": False, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        step["expect"] = expect
        return step


cases = []
HERE = (E0 + 20, N0 + 15)

# ── Save ───────────────────────────────────────────────────────────────

p = Project()
steps = [p.step(save("Ada 112", HERE, 60.0, 40.0, math.pi / 6)), p.step(save("Ada 112", HERE, 60.0, 40.0, math.pi / 6))]
cases.append({
    "name": "Kaydet: görünüm adıyla saklanır, çıktı onu ve bütün listeyi söyler; proje ayarıdır, geri alma adımı değildir; aynısını yeniden kaydetmek uyarır ama bir şey değiştirmez",
    "steps": steps,
})

p = Project()
steps = [p.step(save("Kuzey yukarıda", HERE, 120.0, 80.0))]
assert p.views[0]["twist"] == 0.0
cases.append({"name": "dönüş verilmezse 0'dır: kuzey yukarıda", "steps": steps})

p = Project()
steps = [p.step(save("Batı", HERE, 50.0, 30.0, -math.pi / 2)), p.step(save("Yedi", HERE, 50.0, 30.0, 7.0)), p.step(save("Tam tur", HERE, 50.0, 30.0, TAU))]
assert p.views[0]["twist"] == norm(-math.pi / 2) and p.views[2]["twist"] == 0.0
cases.append({"name": "dönüş [0, 2π) aralığına indirgenerek saklanır: −π/2 3π/2, 7 7 − 2π, 2π 0 olur", "steps": steps})

p = Project()
steps = [p.step(save("  Okul bahçesi  ", HERE, 25.5, 12.25, 0.5))]
assert p.views[0]["name"] == "Okul bahçesi"
cases.append({"name": "ad baştaki ve sondaki boşluklar atılarak saklanır", "steps": steps})

p = Project()
steps = [
    p.step(save("A", HERE, 10.0, 10.0)),
    p.step(save("B", (E0, N0), 20.0, 10.0, math.pi)),
    p.step(save("C", (E0 + 40, N0 + 30), 5.0, 5.0)),
    p.step(save(" A ", (E0 + 1, N0 + 2), 30.0, 15.0, math.pi / 2)),
]
assert [v["name"] for v in p.views] == ["A", "B", "C"] and p.views[0]["width"] == 30.0
cases.append({"name": "aynı adla Kaydet öbürünün yerine geçer (uyarıyla), listedeki yeri kalır", "steps": steps})

p = Project()
LONG = "Ç" * MOST_NAME
steps = [p.step(save(LONG, HERE, MOST_SIZE, MOST_SIZE)), p.step(save("Uç", (REACH, -REACH), 0.001, 0.001))]
cases.append({"name": "sınırlar: 64 karakterlik ad, 20 000 km'lik en ve boy, ortası tam 10 000 km'de görünüm saklanır", "steps": steps})

# ── Rename ─────────────────────────────────────────────────────────────

p = Project()
steps = [p.step(save("A", HERE, 10.0, 10.0)), p.step(save("B", HERE, 20.0, 20.0)), p.step(save("C", HERE, 30.0, 30.0)), p.step(rename("B", "Bahçe"))]
assert [v["name"] for v in p.views] == ["A", "Bahçe", "C"]
cases.append({"name": "Yeniden adlandır: görünümün adı değişir, listedeki yeri ve sayıları kalır", "steps": steps})

p = Project()
steps = [p.step(save("Giriş", HERE, 10.0, 10.0)), p.step(rename(" Giriş ", "  Ana giriş ")), p.step(rename("Ana giriş", "Ana giriş"))]
assert p.views[0]["name"] == "Ana giriş"
cases.append({"name": "Yeniden adlandır'da iki ad da kırpılır; aynı ada adlandırmak bir şey değiştirmez", "steps": steps})

p = Project()
steps = [p.step(save("A", HERE, 10.0, 10.0)), p.step(save("B", HERE, 20.0, 20.0))]
steps.append({**run(rename("A", "B")), "result": exists("B"), "expect": LATER})
steps.append({**run(rename("Yok", "C")), "result": unknown("Yok"), "expect": LATER})
cases.append({"name": "başka bir görünümün adına adlandırmak view_exists, olmayan görünüm unknown_view ile reddedilir; liste değişmez", "steps": steps})

# ── Delete ─────────────────────────────────────────────────────────────

p = Project()
steps = [p.step(save("A", HERE, 10.0, 10.0)), p.step(save("B", HERE, 20.0, 20.0)), p.step(save("C", HERE, 30.0, 30.0)), p.step(delete(" B "))]
assert [v["name"] for v in p.views] == ["A", "C"]
steps.append({**run(delete("B")), "result": unknown("B"), "expect": LATER})
cases.append({"name": "Sil görünümü siler, öbürlerinin sırası kalır; silinmiş ad reddedilir", "steps": steps})

p = Project()
steps = [p.step(save("Tek", HERE, 10.0, 10.0)), p.step(delete("Tek"))]
assert p.views == []
cases.append({"name": "son görünüm silinince projede adlı görünüm kalmaz (ayar yazılmaz)", "steps": steps})

cases.append({
    "name": "boş projede silmek ve adlandırmak: unknown_view",
    "steps": [
        {**run(delete("Bir")), "result": unknown("Bir"), "expect": NOTHING},
        {**run(rename("Bir", "İki")), "result": unknown("Bir"), "expect": NOTHING},
    ],
})

# ── Names ──────────────────────────────────────────────────────────────

cases.append({
    "name": "geçersiz ad: boş, 64 karakterden uzun, denetim karakterli; Yeniden adlandır'ın yeni adı op.to'da",
    "steps": [
        {**run(save("   ", HERE, 10.0, 10.0)), "result": failed("invalid_view_name", NAME_TEXT, "edit.name"), "expect": NOTHING},
        {**run(save("A" * (MOST_NAME + 1), HERE, 10.0, 10.0)), "result": failed("invalid_view_name", NAME_TEXT, "edit.name"), "expect": NOTHING},
        {**run(save("A\tB", HERE, 10.0, 10.0)), "result": failed("invalid_view_name", NAME_TEXT, "edit.name"), "expect": NOTHING},
        {**run(delete("")), "result": failed("invalid_view_name", NAME_TEXT, "edit.name"), "expect": NOTHING},
        {**run(rename("A", " ")), "result": failed("invalid_view_name", NAME_TEXT, "edit.to"), "expect": NOTHING},
        {**run(rename("A", "B\u0007")), "result": failed("invalid_view_name", NAME_TEXT, "edit.to"), "expect": NOTHING},
    ],
})
assert not name_ok("A\tB") and name_ok("Ç" * MOST_NAME) and not name_ok("   ")

MANY = [view(f"G{k}", (E0 + k, N0), 10.0, 10.0) for k in range(MOST_VIEWS)]
p = Project(MANY)
steps = [{**run(save("Yeni", HERE, 10.0, 10.0)), "result": failed("too_many_views", TOO_MANY, "edit.name"), "expect": NOTHING}]
steps.append(p.step(save("G7", HERE, 99.0, 33.0, 1.0)))
steps.append(p.step(rename("G8", "Sekiz")))
cases.append({
    "name": "100 adlı görünüm varken yeni ad reddedilir; var olanın üzerine kaydetmek ve adlandırmak serbesttir",
    "setup": setup({**SETTINGS, "namedViews": MANY}),
    "steps": steps,
})

# ── Numbers ────────────────────────────────────────────────────────────

cases.append({
    "name": "10 000 km'den uzak orta reddedilir",
    "steps": [
        {**run(save("Uzak", (2e7, 0), 10.0, 10.0)), "result": failed("far_point", FAR_TEXT, "edit.center"), "expect": NOTHING},
        {**run(save("Uzak", (0, -1.5e7), 10.0, 10.0)), "result": failed("far_point", FAR_TEXT, "edit.center"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "en ve boy 0'dan büyük ve en çok 20 000 km: sıfır, eksi ve büyüğü reddedilir",
    "steps": [
        {**run(save("Düz", HERE, 0.0, 10.0)), "result": failed("invalid_view_size", SIZE_TEXT, "edit.width"), "expect": NOTHING},
        {**run(save("Düz", HERE, 10.0, -5.0)), "result": failed("invalid_view_size", SIZE_TEXT, "edit.height"), "expect": NOTHING},
        {**run(save("Düz", HERE, 3e7, 10.0)), "result": failed("invalid_view_size", SIZE_TEXT, "edit.width"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "sonlu olmayan sayı her şeyden önce reddedilir: orta, en, boy, dönüş",
    "steps": [
        {**run(save("N", HERE, 10.0, 10.0)), "nonFinite": {"edit.width": "NaN"}, "result": failed("not_finite", NON_FINITE, "edit.width"), "expect": NOTHING},
        {**run(save("N", (2e7, 0), 10.0, 10.0)), "nonFinite": {"edit.center.x": "Infinity"}, "result": failed("not_finite", NON_FINITE, "edit.center"), "expect": NOTHING},
        {**run(save("   ", HERE, 0.0, 10.0, 0.0)), "nonFinite": {"edit.twist": "NaN"}, "result": failed("not_finite", NON_FINITE, "edit.twist"), "expect": NOTHING},
        {**run(save("N", HERE, 10.0, 10.0)), "nonFinite": {"edit.height": "-Infinity"}, "result": failed("not_finite", NON_FINITE, "edit.height"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "denetimlerin sırası: uzak orta, sonra en ve boy, sonra ad",
    "steps": [
        {**run(save("   ", (2e7, 0), 0.0, 10.0)), "result": failed("far_point", FAR_TEXT, "edit.center"), "expect": NOTHING},
        {**run(save("   ", HERE, 0.0, 10.0)), "result": failed("invalid_view_size", SIZE_TEXT, "edit.width"), "expect": NOTHING},
    ],
})

p = Project()
steps = [p.step(save("Kilitli yer", (E0 + 10, N0 + 50), 30.0, 10.0))]
cases.append({"name": "görünüm nesneleri değiştirmez: kilitli katmanın üstündeki yer de saklanır, nesneler aynı kalır", "steps": steps})

p = Project()
steps = [p.step(save("Eski", HERE, 10.0, 10.0)), p.step(rename("Eski", "Yeni")), p.step(save("Eski", (E0, N0), 40.0, 30.0, 3.0))]
assert [v["name"] for v in p.views] == ["Yeni", "Eski"]
cases.append({"name": "adlandırılan görünümün eski adı boşalır: o adla Kaydet yeni görünümü sona ekler", "steps": steps})

p = Project()
steps = [p.step(save("Döner", HERE, 10.0, 10.0, 1.0)), p.step(save("Döner", HERE, 10.0, 10.0, 1.0 + TAU))]
assert p.views[0]["twist"] == 1.0
cases.append({
    "name": "dönüşü tam tur fazla verilen aynı görünüm: indirgenen dönüş aynıdır, Kaydet uyarır ama bir şey değiştirmez",
    "steps": steps,
})

# ── Revision ───────────────────────────────────────────────────────────

p = Project()
cases.append({
    "name": "beklenen sürüm: geçersiz yazı ve eskisi reddedilir; doğrusu yazar",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {**run(save("S", HERE, 10.0, 10.0), expectedRevision="07"), "result": failed("invalid_revision", REVISION, "expectedRevision"), "expect": NOTHING},
        p.step(save("S", HERE, 10.0, 10.0), expectedRevision="$r0"),
        {**run(save("T", HERE, 10.0, 10.0), expectedRevision="$r0"), "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}, "expect": {**NOTHING, "dirty": True}},
    ],
})

assert len(cases) >= 20, len(cases)


# ── The file ─────────────────────────────────────────────────────────


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write(command, title, note, cases):
    out = ["{"]
    head = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note}
    for key, value in head.items():
        out.append(f'  "{key}": {compact(value)},')
    s = SETUP
    out.append('  "setup": {')
    for key in ["format", "version", "name", "settings", "origin"]:
        out.append(f'    "{key}": {compact(s[key])},')
    out.append('    "layers": [')
    out.append(",\n".join(f"      {compact(item)}" for item in s["layers"]))
    out.append("    ],")
    out.append(f'    "activeLayer": {compact(s["activeLayer"])},')
    out.append('    "entities": [')
    out.append(",\n".join(f"      {compact(e)}" for e in s["entities"]))
    out.append("    ],")
    out.append(f'    "styles": {compact(s["styles"])}')
    out.append("  },")
    out.append('  "cases": [')
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    text = "\n".join(out) + "\n"
    path = f"fixtures/commands/v1/{command}.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


write(
    "cad.views.edit",
    "Adlı görünümler",
    "ADR 0285 §1: adlı görünüm bir ad, alanın ortasındaki dünya noktası, gösterilen dünya eni ve boyu ve dönüşüdür (ekranın x "
    "ekseninin doğudan saat yönünün tersine açısı, radyan, [0, 2π)). Kaydet, Yeniden adlandır, Sil; proje ayarıdır, geri alma adımı "
    "değildir, nesneler değişmez. Kural ve beklenenler scripts/fixtures/views_command_cases.py'de KentOS kodu olmadan yazılmıştır.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
