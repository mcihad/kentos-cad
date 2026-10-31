#!/usr/bin/env python3
"""A wall style's components bound to the project's materials as `cad.styles.edit` writes them (docs/adr/0791), from
the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.styles.edit.materials.json; `--check` compares and writes nothing. The drawing is the
one `materials_cases.py` sets up (a wall style whose core is bound to Tuğla, two walls of it, a wall of its own, a
locked wall of the style, slabs). The rules:

- a wall style created, updated or taken from a file: each component with a `materialId` the project has takes the
  material's text, hatch and cut pen (the rest its own: name, thickness, detail, priority, wrap); one whose material the
  project has not refuses the whole write (`material_not_found`, at the component's `materialId`; for a file, every
  style of it is looked at, a skipped one too);
- a component without a `materialId` is as it is given;
- the walls of a changed style take its components (ADR 0339 §2; their width when every component has a thickness),
  those on locked layers kept and counted; one undo step “Duvar stili”.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import materials_cases as M  # noqa: E402  (the reference model of ADR 0785, itself without KentOS code)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.styles.edit.materials.json"
LABEL = "Duvar stili"
FORMAT = "kentos.annotation-styles"


class Refused(Exception):
    def __init__(self, code, message, path, status="failed", revision=False):
        self.result = {"status": status, "error": {"code": code, "message": message, "path": path}}
        if revision:
            self.result["error"]["revision"] = "$current"


def bound(doc, style, path):
    """The style with its bound components the project's materials' (`M.bind`); refused for an unknown material."""
    table = doc["settings"]["arch"].get("materials", [])
    s = copy.deepcopy(style)
    for i, c in enumerate(s["components"]):
        if "materialId" not in c:
            continue
        m = next((m for m in table if m["id"] == c["materialId"]), None)
        if m is None:
            raise Refused("material_not_found",
                          f"“{s['name']}” duvar stilinin {i + 1}. bileşeni (“{c['name']}”) projede olmayan “{c['materialId']}” "
                          "malzemesine bağlı. Önce malzemeyi projeye alın (Malzemeler › Kitaplık) ya da bileşeni bağdan ayırın.",
                          f"{path}.components[{i}].materialId")
        s["components"][i] = M.bind(m, c)
    return s


def follow(doc, after):
    """The walls of each style after the table becomes `after`: the updates (id → wall), how many a locked layer keeps."""
    before = {s["id"]: s for s in doc["settings"].get("wallStyles", [])}
    now = {s["id"]: s for s in after}
    locked = M.locked_layers(doc)
    updates, kept = {}, 0
    for w in doc["entities"]:
        if w["kind"] != "wall" or w.get("style") not in before:
            continue
        old = before[w["style"]]
        nxt = copy.deepcopy(w)
        new = now.get(old["id"])
        if new is None:
            nxt.pop("style")
        elif M.norm(new) == M.norm(old):
            continue
        else:
            nxt["components"] = copy.deepcopy(new["components"])
            for k in ("caps", "fill"):
                if k in new:
                    nxt[k] = new[k]
                else:
                    nxt.pop(k, None)
            if new["components"] and all("thickness" in c for c in new["components"]):
                nxt["width"] = sum(c["thickness"] for c in new["components"])
        if M.norm(nxt) == M.norm(w):
            continue
        if w["layerId"] in locked:
            kept += 1
            continue
        updates[w["id"]] = nxt
    return updates, kept


def run(doc, inp):
    if inp.get("expectedRevision") is not None:
        raise Refused("revision_conflict",
                      "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                      "expectedRevision", status="conflict", revision=True)
    table = copy.deepcopy(doc["settings"].get("wallStyles", []))
    op = inp["operation"]
    lists = {}
    if op == "create":
        s = bound(doc, inp["wall"], "wall")
        table.append(s)
        style = s["id"]
    elif op == "update":
        s = bound(doc, inp["wall"], "wall")
        table = [s if t["id"] == s["id"] else t for t in table]
        style = s["id"]
    elif op == "rename":
        for t in table:
            if t["id"] == inp["style"]:
                t["name"] = inp["name"].strip()
        style = inp["style"]
    elif op == "remove":
        table = [t for t in table if t["id"] != inp["style"]]
        style = inp["style"]
    else:  # import
        incoming = json.loads(inp["library"])["styles"]
        added, replaced, skipped = [], [], []
        for s in incoming:
            s = bound(doc, s, "library")
            at = next((i for i, t in enumerate(table) if t["name"].strip().lower() == s["name"].strip().lower()), None)
            if at is not None and inp.get("replace"):
                s["id"], s["name"] = table[at]["id"], table[at]["name"]
                replaced.append(s["id"])
                table[at] = s
            elif at is not None:
                skipped.append(s["name"].strip())
            else:
                added.append(s["id"])
                table.append(s)
        lists = {"added": added, "replaced": replaced, "skipped": skipped}
        style = (added or replaced or [""])[0]
    updates, locked = follow(doc, table)
    doc2 = copy.deepcopy(doc)
    doc2["settings"]["wallStyles"] = table
    for e in doc2["entities"]:
        if e["id"] in updates:
            nxt = updates[e["id"]]
            e.clear()
            e.update(nxt)
    out = {"style": style, "changed": True, "followed": len(updates), "locked": locked}
    for k in ("added", "replaced", "skipped"):
        if lists.get(k):
            out[k] = lists[k]
    out["revision"] = "$current"
    return {"status": "completed", "output": out, "warnings": []}, doc2


def expect_of(doc):
    """What a written step leaves: the materials' expectation, the style table absent when it is empty."""
    e = M.expect_of(doc, True)
    if not e["settings"]["wallStyles"]:
        e["settings"]["wallStyles"] = None
    return e


def steps_for(inp, undo=False):
    doc = M.setup()
    try:
        result, after = run(doc, inp)
    except Refused as r:
        return [{"op": op, "input": inp, "result": copy.deepcopy(r.result), "expect": {"revision": "same"}}
                for op in ("validate", "execute")]
    steps = [{"op": "execute", "input": inp, "result": result, "expect": expect_of(after)}]
    if undo:
        steps.append({"op": "undo", "returns": LABEL, "expect": expect_of(doc)})
        steps.append({"op": "redo", "returns": LABEL, "expect": expect_of(after)})
    return steps


def comp(name, material=None, mid=None, thickness=None, hatch=None, edge=None, detail="standard", **more):
    c = M.comp(name, material, mid, thickness, hatch, edge, detail)
    c.update(more)
    return c


ODD = {"pattern": "DOTS", "scale": 2.0, "angle": 30.0}


def style(id_, name, comps, width=0.25):
    """A style; its width only when a component has no thickness (else the components' sum, not given)."""
    s = {"id": id_, "name": name, "components": comps}
    if not all("thickness" in c for c in comps):
        s["width"] = width
    return s


DIS = M.setup()["settings"]["wallStyles"][0]


def dis(**at):
    """Dış duvar with its components changed at the places given."""
    s = copy.deepcopy(DIS)
    for i, c in at.items():
        s["components"][int(i[1:])] = c
    return s


def library(styles):
    return json.dumps({"format": FORMAT, "version": 1, "kind": "wall", "styles": styles}, ensure_ascii=False,
                      sort_keys=True, separators=(",", ":"))


def cases():
    c = []

    def case(name, inp, undo=False):
        c.append({"name": name, "steps": steps_for(inp, undo)})

    def wall(op, s, **kw):
        return {"kind": "wall", "operation": op, "wall": s, **kw}

    case("Oluştur: tuğlaya bağlı bileşen malzemenin metnini, taramasını ve kalemini alır",
         wall("create", style("ic-duvar", "İç duvar", [comp("Sıva", "Sıva", thickness=0.02),
                                                       comp("Çekirdek", "Kerpiç", "tugla", hatch=ODD, edge={"lineWeight": 0.1})])),
         undo=True)
    case("Oluştur: iki malzemeye bağlı bileşenler", wall("create", style("perde", "Perde", [
        comp("Beton", None, "betonarme", thickness=0.2), comp("Tuğla", None, "tugla", thickness=0.1)], width=0.3)))
    case("Oluştur: bağlı bileşen kalınlığını, ayrıntısını ve önceliğini korur, dönen sıva kalır", wall("create", style(
        "dis-2", "Dış duvar 2", [comp("Taşıyıcı", "x", "tugla", thickness=0.19, detail="detailed", priority=1),
                                 comp("Sıva", "Sıva", thickness=0.03, detail="detailed", wrap=True)])))
    case("Oluştur: bağsız bileşenler verildiği gibi", wall("create", style("bolme", "Bölme", [
        comp("Gazbeton", "Gazbeton", thickness=0.1, hatch=ODD)], width=0.1)))
    case("Oluştur: projede olmayan malzeme reddedilir", wall("create", style("ic-duvar", "İç duvar", [
        comp("Sıva", "Sıva", thickness=0.02), comp("Çekirdek", "Kerpiç", "kerpic")])))
    case("Oluştur: kimliği kurala uymayan malzeme projede yok", wall("create", style("ic-duvar", "İç duvar", [
        comp("Çekirdek", "Tuğla", "Tugla")])))
    case("Değiştir: yalıtımı betonarmeye bağla, stilin duvarları izler, kilitli kalır",
         wall("update", dis(c2=comp("Isı yalıtımı", "Isı yalıtımı", "betonarme", thickness=0.05, hatch=M.INS))), undo=True)
    case("Değiştir: tuğlayı bağdan ayır, metin ve görünüş kalır, duvarlar izler",
         wall("update", dis(c1=comp("Tuğla", "Tuğla", hatch=M.BRICK, edge={"lineWeight": 0.5}))))
    case("Değiştir: bağlı tuğlanın elle değişen taraması malzemeninkine döner, öbür değişiklik kalır",
         wall("update", dis(c1=comp("Tuğla", "Tuğla", "tugla", hatch=ODD), c0=comp("İç sıva", "İç sıva", thickness=0.025,
                                                                                 detail="detailed"))))
    case("Oluştur: bağlı bileşenin metni malzemenin adı olur, kalemi verilmemişken malzemeninkini alır",
         wall("create", style("harman", "Harman", [comp("Tuğla", "Harman tuğlası", "tugla", thickness=0.19)], width=0.19)))
    case("Değiştir: bağlı bileşenin adı kendisinin kalır", wall("update", dis(c1=comp("Çekirdek", "Tuğla", "tugla"))))
    case("Değiştir: dış sıvayı olmayan malzemeye bağlamak reddedilir",
         wall("update", dis(c3=comp("Dış sıva", "Dış sıva", "dis-siva", thickness=0.03, detail="detailed"))))
    case("Değiştir: yalıtımı olmayan malzemeye bağlamak reddedilir",
         wall("update", dis(c2=comp("Isı yalıtımı", "EPS", "eps", thickness=0.05))))
    case("Değiştir: beklenen sürüm eski", dict(wall("update", dis(c2=comp("Isı yalıtımı", "Isı yalıtımı", "betonarme",
                                                                                thickness=0.05))), expectedRevision="999"))
    case("Yeniden adlandır: bağlı stil, duvarlar değişmez", {"kind": "wall", "operation": "rename", "style": "dis-duvar",
                                                             "name": "Dış duvar 25"})
    case("Sil: bağlı stil, duvarlar bağını yitirir, görünüşleri kalır", {"kind": "wall", "operation": "remove",
                                                                         "style": "dis-duvar"})
    case("Dosyadan al: yeni stilin bağlı bileşeni projenin malzemesini alır", {"kind": "wall", "operation": "import",
         "library": library([style("019a0000-0000-7000-8000-00000000e001", "İç duvar",
                                   [comp("Çekirdek", "Tuğla", "tugla", thickness=0.1, hatch=ODD)], width=0.1)])})
    case("Dosyadan al: bağsız bileşenli stil verildiği gibi", {"kind": "wall", "operation": "import",
         "library": library([style("019a0000-0000-7000-8000-00000000e002", "Bölme", [comp("Alçı", "Alçı", thickness=0.1)],
                                   width=0.1)])})
    case("Dosyadan al ve değiştir: Dış duvarın yalıtımı betonarmeye bağlı, duvarlar izler",
         {"kind": "wall", "operation": "import", "replace": True,
          "library": library([dis(c2=comp("Isı yalıtımı", "?", "betonarme", thickness=0.05))])}, undo=True)
    case("Dosyadan al: projede olmayan malzemeye bağlı stil reddedilir", {"kind": "wall", "operation": "import",
         "library": library([style("019a0000-0000-7000-8000-00000000e003", "İç duvar",
                                   [comp("Çekirdek", "Gazbeton", "gazbeton", thickness=0.1)], width=0.1)])})
    case("Dosyadan al: atlanacak stil de olmayan malzemeye bağlıysa reddedilir", {"kind": "wall", "operation": "import",
         "library": library([dis(c2=comp("Isı yalıtımı", "EPS", "eps", thickness=0.05))])})
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data = {
        "format": "kentos.command-cases", "version": 1, "command": "cad.styles.edit", "commandVersion": 1,
        "title": "Duvar stilinde malzeme bağı",
        "note": "ADR 0791. Beklenenler scripts/fixtures/style_materials_cases.py'nin ADR'den yazılmış kurallarından, KentOS "
                "kodu olmadan (bağın kuralı ve kurulum materials_cases.py'nin modeli).",
        "setup": M.setup(),
        "cases": cases(),
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
