"""Writes fixtures/commands/v1/cad.changeset.apply.json: the shared cases of `cad.changeset.apply` v1 (docs/adr/0253 §8).

The expectations are built here from the contract's rules (crates/shared/commands/src/cad_changeset.rs: the checks and
their order, the codes, paths and words; docs/adr/0253 §8), and from what the steps' own commands answer as their own
cases write it (fixtures/commands/v1/cad.point.create.json, cad.entities.set.json, cad.entities.delete.json,
cad.layers.set.json, cad.settings.set.json: outputs, messages), without KentOS code:

- the steps run in order inside one change: a step refused refuses the set, with the step's code, at
  `steps[i].input.<its path>`, its message after “<i+1>. adım: ”, and nothing of any step stays, the project's
  settings and the layers' flags included;
- the objects, layers and blocks the steps write are one undo step named `label` (“Değişiklik kümesi” when it is
  absent or blank); the settings and the layers' flags are no undo step;
- a later step names an earlier one's output by a whole text: `$ad` its `uid`, `$ad[i]` its `created[i]`,
  `$ad.alan` its `alan`; a text that names a step not before it is refused (`unknown_reference`); a text that is
  no step's name (or no name at all, `$5`) is the text;
- every step's output says the set's revision; a step's warnings come in the set's answer at `steps[i].input.<path>`;
- the plan runs the set on a copy: the steps' outputs, the revision to expect, nothing written.

    python3 scripts/fixtures/changeset_command_cases.py           # writes the file
    python3 scripts/fixtures/changeset_command_cases.py --check   # compares it with the one on disk
"""
import json
import sys

# The fixture's own placeholders begin with “$” (`$current`, `$uidOf:3`); a change set's reference to a step is
# written “$$ad”, the text “$ad” itself (fixtures/commands/README.md).
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.changeset.apply.json"

LABEL = "Değişiklik kümesi"
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
LOCKED = "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
HIDDEN = "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek."
NOT_FOUND = "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
NO_STEPS = "Değişiklik kümesinde hiç adım yok; en az bir komut verin."
NESTED = "Değişiklik kümesi iç içe verilemez; adımlarını bu kümeye ekleyin."


def invalid_revision(text: str) -> str:
    return f"Beklenen sürüm “{text}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."


def side_effect(command: str) -> str:
    return f"{command} dosyaya yazar; yan etkisi geri alınamadığı için değişiklik kümesine giremez. Kümeden sonra ayrıca çağırın."


def unknown_command(command: str) -> str:
    return f"“{command}” bu çizimde çalışan bir komut değil."


def unknown_version(command: str, v: int) -> str:
    return f"{command} için {v}. sürüm yok; bu sürüm 1. sürümü çalıştırır."


def bad_name(name: str) -> str:
    return f"Adımın adı “{name}” olamaz: bir harf ya da _ ile başlamalı, harf, rakam ve _ içermeli ve bir kez verilmeli."


def unknown_reference(step: int, text: str) -> str:
    return (
        f"{step}. adımın “{text}” başvurusu daha önceki bir adımın çıktısını adlamıyor: adı önceki bir adımın “as”'i "
        "olmalı, alanı ve sırası o adımın çıktısında bulunmalı."
    )


def within(step: int, message: str) -> str:
    return f"{step}. adım: {message}"


def layer(lid: str, name: str, visible=True, locked=False) -> dict:
    return {"id": lid, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Değişiklik kümesi",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "cad", "drawingFont": "barlow"},
    "origin": {"x": 487000, "y": 4420000},
    "layers": [layer("yapi", "Yapı"), layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False)],
    "activeLayer": "yapi",
    "entities": [
        {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": {"x": 487000, "y": 4420000}, "b": {"x": 487010, "y": 4420000}},
        {"kind": "point", "id": 2, "layerId": "yapi", "attrs": {}, "p": {"x": 487005, "y": 4420005}},
    ],
    "styles": {"items": [], "categories": []},
}
LAYERS = SETUP["layers"]


def pc(layer_id: str, x: float, y: float, **extra) -> dict:
    return {"command": "cad.point.create", "input": {"layerId": layer_id, "p": {"x": x, "y": y}, **extra}}


def point(slot: int, layer_id: str, x: float, y: float, **extra) -> dict:
    return {"kind": "point", "id": slot, "layerId": layer_id, **({"color": extra["color"]} if "color" in extra else {}),
            "attrs": extra.get("attrs", {}), "p": {"x": x, "y": y}}


def made(slot: int) -> dict:
    """A point's output (cad.point.create.json): its persistent id, its slot, the set's revision."""
    return {"command": "cad.point.create", "output": {"uid": f"$uidOf:{slot}", "id": slot, "revision": "$current"}}


def done(steps, warnings=None, label=LABEL, revision="$current") -> dict:
    return {"status": "completed", "output": {"label": label, "steps": steps, "revision": revision}, "warnings": warnings or []}


def failed(code: str, message: str, path: str) -> dict:
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def execute(input_, result, expect=None, note=None) -> dict:
    step = {"op": "execute", "input": input_, "result": result}
    if expect:
        step["expect"] = expect
    if note:
        step["note"] = note
    return step


NOTHING = {"ids": [1, 2], "canUndo": False, "dirty": False, "revision": "same"}


def cases() -> list:
    out = []

    def case(name, steps, setup=None, note=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        if note:
            c["note"] = note
        out.append(c)

    case(
        "İki nokta tek değişiklik: çıktılar sırayla, tek geri alma adımı “Değişiklik kümesi”",
        [
            execute(
                {"steps": [pc("yapi", 487020, 4420020), pc("yapi", 487030, 4420030)]},
                done([made(3), made(4)]),
                {"ids": [1, 2, 3, 4], "entities": {"3": point(3, "yapi", 487020, 4420020), "4": point(4, "yapi", 487030, 4420030)},
                 "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"},
            ),
            {"op": "undo", "returns": LABEL, "expect": {"ids": [1, 2], "canRedo": True}},
            {"op": "redo", "returns": LABEL, "expect": {"ids": [1, 2, 3, 4]}},
        ],
    )
    case(
        "Adı verilen küme o adla geri alınır",
        [
            execute(
                {"label": "Röper noktaları", "steps": [pc("yapi", 487020, 4420020)]},
                done([made(3)], label="Röper noktaları"),
                {"ids": [1, 2, 3]},
            ),
            {"op": "undo", "returns": "Röper noktaları", "expect": {"ids": [1, 2]}},
        ],
    )
    case(
        "Boş ya da yalnız boşluk ad verilmemiş gibidir",
        [
            execute({"label": " \t ", "steps": [pc("yapi", 487020, 4420020)]}, done([made(3)])),
            {"op": "undo", "returns": LABEL},
        ],
    )
    case(
        "“$ad” önceki adımın uid'i: yeni nokta aynı kümede boyanır",
        [
            execute(
                {"steps": [
                    {**pc("yapi", 487020, 4420020), "as": "nokta"},
                    {"command": "cad.entities.set", "input": {"uids": ["$$nokta"], "color": "#E5484D", "operation": "color"}},
                ]},
                done([made(3), {"command": "cad.entities.set", "output": {"changed": ["$uidOf:3"], "ids": [3], "revision": "$current"}}]),
                {"ids": [1, 2, 3], "entities": {"3": point(3, "yapi", 487020, 4420020, color="#E5484D")}},
            ),
            {"op": "undo", "returns": LABEL, "expect": {"ids": [1, 2], "canUndo": False}},
        ],
    )
    case(
        "“$ad[i]” önceki adımın created[i]'si: ikinci yeni nokta aynı kümede boyanır",
        [
            execute(
                {"steps": [
                    {"command": "cad.entities.create", "as": "yeni",
                     "input": {"layerId": "yapi", "objects": [
                         {"geometry": {"kind": "point", "p": {"x": 487040, "y": 4420040}}},
                         {"geometry": {"kind": "point", "p": {"x": 487050, "y": 4420050}}}]}},
                    {"command": "cad.entities.set", "input": {"uids": ["$$yeni[1]"], "color": "#30A46C", "operation": "color"}},
                ]},
                done([
                    {"command": "cad.entities.create", "output": {"created": ["$uidOf:3", "$uidOf:4"], "ids": [3, 4], "revision": "$current"}},
                    {"command": "cad.entities.set", "output": {"changed": ["$uidOf:4"], "ids": [4], "revision": "$current"}},
                ]),
                {"ids": [1, 2, 3, 4],
                 "entities": {"3": point(3, "yapi", 487040, 4420040), "4": point(4, "yapi", 487050, 4420050, color="#30A46C")}},
            ),
        ],
    )
    case(
        "Bir adım reddederse küme reddeder: kod adımın, yol ve ileti adımın, hiçbir şey yazılmaz",
        [
            execute(
                {"steps": [pc("yapi", 487020, 4420020), pc("kilitli", 487030, 4420030)]},
                failed("layer_locked", within(2, LOCKED), "steps[1].input.layerId"),
                NOTHING,
            ),
        ],
    )
    case(
        "Reddedilen küme proje ayarlarını ve katman bayraklarını da geri alır",
        [
            execute(
                {"steps": [
                    {"command": "cad.settings.set", "input": {"plotScale": 500}},
                    {"command": "cad.layers.set", "input": {"operation": "visible", "layer": "yapi", "value": False}},
                    pc("yok", 487030, 4420030),
                ]},
                failed("layer_not_found", within(3, NOT_FOUND), "steps[2].input.layerId"),
                {**NOTHING, "settings": {"plotScale": 1000}, "layers": LAYERS},
            ),
        ],
    )
    case(
        "Ayar ve görünürlük geri alma adımı değildir; nesneler tek adımdır",
        [
            execute(
                {"steps": [
                    {"command": "cad.layers.set", "input": {"operation": "visible", "layer": "gizli", "value": True}},
                    pc("yapi", 487020, 4420020),
                    {"command": "cad.settings.set", "input": {"plotScale": 500}},
                ]},
                done([
                    {"command": "cad.layers.set", "output": {"changed": True, "active": "yapi", "revision": "$current"}},
                    made(3),
                    {"command": "cad.settings.set", "output": {"changed": ["plotScale"], "followed": 0, "locked": 0, "revision": "$current"}},
                ]),
                {"ids": [1, 2, 3], "settings": {"plotScale": 500}},
            ),
            {"op": "undo", "returns": LABEL,
             "expect": {"ids": [1, 2], "settings": {"plotScale": 500},
                        "layers": [LAYERS[0], LAYERS[1], {**LAYERS[2], "visible": True}]}},
        ],
    )
    case(
        "Adımın uyarısı kümenin cevabında adımın yoluyla, adımın çıktısında kendi yoluyla",
        [
            execute(
                {"steps": [pc("gizli", 487020, 4420020)]},
                done(
                    [{**made(3), "warnings": [{"code": "layer_hidden", "message": HIDDEN, "path": "layerId"}]}],
                    warnings=[{"code": "layer_hidden", "message": HIDDEN, "path": "steps[0].input.layerId"}],
                ),
                {"ids": [1, 2, 3]},
            ),
        ],
    )
    case("Adımsız küme reddedilir", [execute({"steps": []}, failed("no_steps", NO_STEPS, "steps"), NOTHING)])
    case(
        "Küme iç içe verilemez",
        [execute({"steps": [pc("yapi", 1, 1), {"command": "cad.changeset.apply", "input": {"steps": []}}]},
                 failed("nested_changeset", NESTED, "steps[1].command"), NOTHING)],
    )
    case(
        "Dosyaya yazan komut kümeye giremez",
        [execute({"steps": [{"command": "file.export.dxf", "input": {"path": "/tmp/a.dxf"}}]},
                 failed("side_effect_in_changeset", side_effect("file.export.dxf"), "steps[0].command"), NOTHING)],
    )
    case(
        "Bilinmeyen komut adımın komut yoluyla reddedilir",
        [execute({"steps": [pc("yapi", 1, 1), {"command": "cad.ufo.create", "input": {}}]},
                 failed("unknown_command", unknown_command("cad.ufo.create"), "steps[1].command"), NOTHING)],
    )
    case(
        "Başka sürüm reddedilir",
        [execute({"steps": [{**pc("yapi", 1, 1), "version": 2}]},
                 failed("unknown_version", unknown_version("cad.point.create", 2), "steps[0].version"), NOTHING)],
    )
    case(
        "Adımın adı harf ya da _ ile başlar",
        [execute({"steps": [{**pc("yapi", 1, 1), "as": "1nokta"}]},
                 failed("invalid_input", bad_name("1nokta"), "steps[0].as"), NOTHING)],
    )
    case(
        "Bir ad iki adıma verilemez",
        [execute({"steps": [{**pc("yapi", 1, 1), "as": "n"}, {**pc("yapi", 2, 2), "as": "n"}]},
                 failed("invalid_input", bad_name("n"), "steps[1].as"), NOTHING)],
    )
    case(
        "Sonraki adımın adı önceden anılamaz",
        [execute(
            {"steps": [
                {"command": "cad.entities.set", "input": {"uids": ["$$sonra"], "color": "#E5484D", "operation": "color"}},
                {**pc("yapi", 487020, 4420020), "as": "sonra"},
            ]},
            failed("unknown_reference", unknown_reference(1, "$sonra"), "steps[0].input.uids[0]"),
            NOTHING,
        )],
    )
    case(
        "Önceki adımın çıktısında olmayan alan anılamaz",
        [execute(
            {"steps": [
                {**pc("yapi", 487020, 4420020), "as": "n"},
                {"command": "cad.entities.set", "input": {"uids": ["$$n.yok"], "color": "#E5484D", "operation": "color"}},
            ]},
            failed("unknown_reference", unknown_reference(2, "$n.yok"), "steps[1].input.uids[0]"),
            NOTHING,
        )],
    )
    case(
        "Bir adımın adı olmayan “$…” metni metindir",
        [execute(
            {"steps": [pc("yapi", 487020, 4420020, attrs={"Fiyat": "$$5", "Not": "$$yok", "Kod": "$$a b"})]},
            done([made(3)]),
            {"ids": [1, 2, 3], "entities": {"3": point(3, "yapi", 487020, 4420020, attrs={"Fiyat": "$$5", "Kod": "$$a b", "Not": "$$yok"})}},
        )],
    )
    case(
        "Çizim beklenen sürümde değilse hiçbir şey yazılmaz",
        [execute({"steps": [pc("yapi", 1, 1)], "expectedRevision": "7"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 NOTHING)],
    )
    case(
        "Beklenen sürüm ondalık bir tamsayı yazısıdır",
        [execute({"steps": [pc("yapi", 1, 1)], "expectedRevision": "x"},
                 failed("invalid_revision", invalid_revision("x"), "expectedRevision"), NOTHING)],
    )
    case(
        "Beklenen sürüm tutarsa yazılır",
        [
            {"op": "captureRevision", "as": "hazir"},
            execute({"steps": [pc("yapi", 487020, 4420020)], "expectedRevision": "$hazir"}, done([made(3)]), {"ids": [1, 2, 3]}),
        ],
    )
    case(
        "Plan kümeyi kopyada çalıştırır: adımların çıktıları ve beklenecek sürüm; hiçbir şey yazılmaz",
        [
            {"op": "captureUid", "id": 2, "as": "nokta"},
            {"op": "plan",
             "input": {"steps": [
                 {"command": "cad.entities.set", "input": {"uids": ["$uid:nokta"], "color": "#30A46C", "operation": "color"}},
                 {"command": "cad.layers.set", "input": {"operation": "visible", "layer": "gizli", "value": True}},
             ]},
             "result": done([
                 {"command": "cad.entities.set", "output": {"changed": ["$uid:nokta"], "ids": [2], "revision": "$current"}},
                 {"command": "cad.layers.set", "output": {"changed": True, "active": "yapi", "revision": "$current"}},
             ]),
             "expect": {**NOTHING, "layers": LAYERS, "entities": {"2": SETUP["entities"][1]}}},
        ],
    )
    case(
        "Doğrulama kümeyi denetler, uyarıları verir, hiçbir şey yazmaz",
        [
            {"op": "validate", "input": {"steps": [pc("gizli", 487020, 4420020)]},
             "result": {"status": "completed", "output": None,
                        "warnings": [{"code": "layer_hidden", "message": HIDDEN, "path": "steps[0].input.layerId"}]},
             "expect": NOTHING},
            {"op": "validate", "input": {"steps": [pc("yapi", 1, 1), pc("kilitli", 2, 2)]},
             "result": failed("layer_locked", within(2, LOCKED), "steps[1].input.layerId"),
             "expect": NOTHING},
        ],
    )
    return out


def build() -> dict:
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.changeset.apply",
        "commandVersion": 1,
        "title": "Değişiklik kümesi: birden çok komut tek iş olarak",
        "note": "docs/adr/0253 §8. Adımlar sırayla, tek değişiklikte; biri reddederse hepsi geri alınır (proje ayarları ve katman "
                "bayrakları da). Nesneler tek geri alma adımıdır, ayarlar değildir. Başvurular: “$ad”, “$ad[i]”, “$ad.alan”. "
                "Bu dosyayı scripts/fixtures/changeset_command_cases.py yazar.",
        "setup": SETUP,
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
