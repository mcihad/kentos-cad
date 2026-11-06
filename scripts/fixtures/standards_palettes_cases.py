#!/usr/bin/env python3
"""The shared cases of cad.standards.palettes (Ofis araç paletleri, docs/adr/1014), written from the ADR without KentOS
code.

    python3 scripts/fixtures/standards_palettes_cases.py           # writes fixtures/commands/v1/cad.standards.palettes.json
    python3 scripts/fixtures/standards_palettes_cases.py --check   # writes nothing; compares

A version 2 standard package may carry `resources.toolPalettes`: palettes of tools, each a catalog command, its
version and its whole input. The palettes are read where the package is bound (never installed in the drawing); a
palette id comes once, the first bound standard's. Each tool is answered with its favourite key
`standart:<package>:<palette>:<tool>`, its digest (SHA-256 of {"command", "commandVersion", "input"} as JSON with the
keys sorted and no spaces) and whether the command takes the input on this drawing (`ready`); when not, the command's
own code and path (`problem`; its words are the command's and not repeated here). A command this host does not run, or
a version other than the one it runs, is `unknown_command`. No bound standard: no palettes. A package whose palettes
break the shape is not bound (cad.settings.set refuses it), so no palette comes; the query's own `invalid_standard`
(a package already in a file that does not read) is held by a Rust test.
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.standards.palettes.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def digest(command, version, input_):
    text = json.dumps({"command": command, "commandVersion": version, "input": input_}, sort_keys=True,
                      separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def tool(id_, title, command, version, input_, **more):
    return {"id": id_, "title": title, "command": command, "commandVersion": version, "input": input_, **more}


LINE = tool("duvar-cizgisi", "Duvar çizgisi", "cad.line.create", 1,
            {"layerId": "duvar", "a": {"x": 0, "y": 0}, "b": {"x": 5, "y": 0}}, category="Mimari")
POINT = tool("nirengi", "Nirengi noktası", "cad.point.create", 1, {"layerId": "duvar", "p": {"x": 1, "y": 1}},
             description="Röper noktası")
NO_LAYER = tool("eksik-katman", "Eksik katmana çizgi", "cad.line.create", 1,
                {"layerId": "yok", "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}})
UNKNOWN = tool("bilinmeyen", "Bilinmeyen komut", "cad.unknown.thing", 1, {})
OLD = tool("eski-surum", "Eski sürüm", "cad.line.create", 9, {"layerId": "duvar", "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}})
BAD_INPUT = tool("bozuk-girdi", "Okunmayan girdi", "cad.line.create", 1, {"layerId": "duvar", "a": "sıfır"})


def package(id_, rev, palettes):
    return {"format": "kentos.drawing-standard", "version": 2, "id": id_, "revision": rev,
            "standard": {"name": id_, "layers": []}, "resources": {"toolPalettes": palettes}}


def palette(id_, name, tools, description=None):
    p = {"id": id_, "name": name, "tools": tools}
    if description is not None:
        p["description"] = description
    return p


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Ofis paletleri",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 100, "workspace": "cad"},
            "origin": {"x": 0, "y": 0},
            "layers": [{"id": "duvar", "name": "Duvar", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": STYLE, "children": []},
                       {"id": "kilitli", "name": "Kilitli", "type": "layer", "visible": True, "locked": True,
                        "expanded": True, "style": STYLE, "children": []}],
            "activeLayer": "duvar", "entities": [], "styles": {"items": [], "categories": []}}


def bind(*packages):
    return {"op": "execute", "command": "cad.settings.set",
            "input": {"standardDocs": [{"text": json.dumps(p, ensure_ascii=False)} for p in packages]},
            "result": "$any", "expect": {"revision": "changed"}}


def state(std, p, t, problem=None):
    out = {"key": f"standart:{std}:{p}:{t['id']}", "digest": digest(t["command"], t["commandVersion"], t["input"]),
           **t, "ready": problem is None}
    if problem is not None:
        out["problem"] = problem
    return out


def problem(code, path):
    return {"code": code, "message": "$any", "path": path}


def read(palettes, op="execute"):
    return {"op": op, "input": {}, "result": {"status": "completed", "output": {"palettes": palettes}, "warnings": []},
            "expect": {"revision": "same"}}


def answered(std, rev, p, tools_states):
    out = {"id": p["id"], "name": p["name"], "standard": std, "revision": rev, "tools": tools_states}
    if "description" in p:
        out["description"] = p["description"]
    return out


def refused(code, path):
    return {"op": "execute", "input": {}, "result": {"status": "failed",
            "error": {"code": code, "message": "$any", "path": path}}, "expect": {"revision": "same"}}


def cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    mimari = palette("mimari", "Mimari", [LINE, POINT], description="Ofisin mimari araçları")
    a = package("ofis-a", "2026.1", [mimari])
    case("Bağlı standart yok: palet yok", [read([])])
    case("İki araçlı palet: ikisi de hazır, anahtar ve özetle", [bind(a), read([answered("ofis-a", "2026.1", mimari, [
        state("ofis-a", "mimari", LINE), state("ofis-a", "mimari", POINT)])])])
    case("Plan aynı cevabı verir", [bind(a), read([answered("ofis-a", "2026.1", mimari, [
        state("ofis-a", "mimari", LINE), state("ofis-a", "mimari", POINT)])], op="plan")])
    case("Doğrulama yazmaz", [bind(a), {"op": "validate", "input": {}, "result": {"status": "completed", "output": None,
        "warnings": []}, "expect": {"revision": "same"}}])
    eksik = palette("eksik", "Eksikler", [NO_LAYER])
    case("Girdisi bu çizimde olmayan katmanı adlandıran araç hazır değil: komutun kodu", [
        bind(package("ofis-a", "2026.1", [eksik])),
        read([answered("ofis-a", "2026.1", eksik, [state("ofis-a", "eksik", NO_LAYER, problem("layer_not_found", "input.layerId"))])])])
    bil = palette("bil", "Bilinmeyen", [UNKNOWN])
    case("Bu konakta çalışmayan komut: unknown_command", [
        bind(package("ofis-a", "2026.1", [bil])),
        read([answered("ofis-a", "2026.1", bil, [state("ofis-a", "bil", UNKNOWN, problem("unknown_command", "command"))])])])
    eski = palette("eski", "Eski", [OLD])
    case("Başka sürümün aracı: unknown_command, sürümün yeri", [
        bind(package("ofis-a", "2026.1", [eski])),
        read([answered("ofis-a", "2026.1", eski, [state("ofis-a", "eski", OLD, problem("unknown_command", "commandVersion"))])])])
    bozuk = palette("bozuk", "Bozuk", [BAD_INPUT])
    case("Komutun okumadığı girdi: invalid_input", [
        bind(package("ofis-a", "2026.1", [bozuk])),
        read([answered("ofis-a", "2026.1", bozuk, [state("ofis-a", "bozuk", BAD_INPUT, problem("invalid_input", "input"))])])])
    karma = palette("karma", "Karma", [LINE, NO_LAYER, UNKNOWN])
    case("Karışık palet: araçlar kendi sırasıyla, her biri kendi durumuyla", [
        bind(package("ofis-a", "2026.1", [karma])),
        read([answered("ofis-a", "2026.1", karma, [
            state("ofis-a", "karma", LINE),
            state("ofis-a", "karma", NO_LAYER, problem("layer_not_found", "input.layerId")),
            state("ofis-a", "karma", UNKNOWN, problem("unknown_command", "command"))])])])
    b_mimari = palette("MIMARI", "B'nin mimarisi", [POINT])
    b_yol = palette("yol", "Yol", [LINE])
    b = package("ofis-b", "2026.3", [b_mimari, b_yol])
    case("İki standart: aynı kimlikli palet ilkinin, öbürü ikincinin", [bind(a, b), read([
        answered("ofis-a", "2026.1", mimari, [state("ofis-a", "mimari", LINE), state("ofis-a", "mimari", POINT)]),
        answered("ofis-b", "2026.3", b_yol, [state("ofis-b", "yol", LINE)])])])
    case("Sıra değişince öncelik de değişir", [bind(b, a), read([
        answered("ofis-b", "2026.3", b_mimari, [state("ofis-b", "MIMARI", POINT)]),
        answered("ofis-b", "2026.3", b_yol, [state("ofis-b", "yol", LINE)])])])
    rev2 = package("ofis-a", "2026.2", [palette("mimari", "Mimari", [dict(LINE, input={**LINE["input"], "b": {"x": 6, "y": 0}}), POINT],
                                                description="Ofisin mimari araçları")])
    moved = dict(LINE, input={**LINE["input"], "b": {"x": 6, "y": 0}})
    case("Yeni revizyonda araç değişince anahtar aynı, özeti başka", [bind(rev2), read([
        answered("ofis-a", "2026.2", rev2["resources"]["toolPalettes"][0], [state("ofis-a", "mimari", moved),
                                                                            state("ofis-a", "mimari", POINT)])])])
    case("Okuma çizimi değiştirmez: hazır araç yürütülmez", [bind(a), read([answered("ofis-a", "2026.1", mimari, [
        state("ofis-a", "mimari", LINE), state("ofis-a", "mimari", POINT)])]),
        {"op": "execute", "command": "cad.drawing.inspect", "input": {"checks": ["health"]}, "result": "$any",
         "expect": {"revision": "same"}}])
    locked = tool("kilitliye", "Kilitli katmana", "cad.line.create", 1,
                  {"layerId": "kilitli", "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}})
    kp = palette("kp", "Kilitli", [locked])
    case("Kilitli katmana çizen araç hazır değil", [bind(package("ofis-a", "2026.1", [kp])), read([
        answered("ofis-a", "2026.1", kp, [state("ofis-a", "kp", locked, problem("layer_locked", "input.layerId"))])])])
    case("Paletsiz sürüm 2 paket: palet yok", [bind({"format": "kentos.drawing-standard", "version": 2, "id": "bos",
        "revision": "1", "standard": {"name": "Boş", "layers": []}, "resources": {}}), read([])])
    case("Sürüm 1 paket: palet yok", [bind({"format": "kentos.drawing-standard", "version": 1, "id": "eski", "revision": "1",
        "standard": {"name": "Eski", "layers": []}}), read([])])

    def refused_bind(p):
        step = bind(p)
        step["result"] = {"status": "failed", "error": "$any"}
        step["expect"] = {"revision": "same"}
        return step

    # A package whose palettes break the shape is not bound at all (cad.settings.set reads it): no palette comes.
    def bad(name, palettes):
        case(name, [refused_bind(package("ofis-a", "2026.1", palettes)), read([])])

    bad("Araçsız palet: geçersiz standart", [palette("bos", "Boş", [])])
    bad("Aynı palet kimliği iki kez: geçersiz", [palette("p", "P", [LINE]), palette("P", "P2", [POINT])])
    bad("Aynı araç kimliği iki kez: geçersiz", [palette("p", "P", [LINE, dict(POINT, id="DUVAR-CIZGISI")])])
    bad("Sürümü sıfır araç: geçersiz", [palette("p", "P", [dict(LINE, commandVersion=0)])])
    bad("Girdisi nesne olmayan araç: geçersiz", [palette("p", "P", [dict(LINE, input=[1, 2])])])
    bad("Kenar boşluklu palet kimliği: geçersiz", [palette(" p", "P", [LINE])])
    case("Bilinmeyen araç alanı: geçersiz", [refused_bind(package("ofis-a", "2026.1", [palette("p", "P", [dict(LINE, icon="x")])])),
                                              read([])])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.standards.palettes", "commandVersion": 1,
            "title": "Ofis araç paletleri",
            "note": "ADR 1014. Bağımsız başvuru scripts/fixtures/standards_palettes_cases.py: kurallar ADR'den elle yazılır; KentOS kodu yok.",
            "setup": setup(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("Palet ortak durumları güncel." if same else "Palet ortak durumları güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
