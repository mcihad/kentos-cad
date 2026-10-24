#!/usr/bin/env python3
"""Independent cases for cad.walls.rectangle v1 (plan/architecture.md).

Only the command's public contract is used. No KentOS module, executable, shape
reference or generated answer is imported. Coordinates are calculated with an
80-digit Decimal frame; sine/cosine use their series. A rectangle is traversed
counter-clockwise, retaining the first input corner. The reference line is the
inner face (left justification), outer face (right), or centre of the wall.

All wire lengths are metres, even when a document displays mm/cm/in/ft. A
4 x 3 inner rectangle has 12 m2 clear area; with 0.2 m walls an outer rectangle
has 3.6 x 2.6 = 9.36 m2, and a centre rectangle 3.8 x 2.8 = 10.64 m2.

The fixed comparison budgets are 1e-12 m near the origin and 1e-8 m at survey
coordinates (several binary64 coordinate ULPs). These cover independent
transcendental arithmetic, not a relaxed geometric acceptance threshold.

    python3 scripts/fixtures/walls_rectangle_cases.py
    python3 scripts/fixtures/walls_rectangle_cases.py --check
"""

import argparse
import copy
import json
import math
import sys
from decimal import Decimal, getcontext
from pathlib import Path

sys.dont_write_bytecode = True
getcontext().prec = 80
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.walls.rectangle.json"
PI = Decimal("3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986")
COMMAND = "cad.walls.rectangle"
STEP = "Duvar"
DEGENERATE = ("Dikdörtgenin bir kenarı sıfır uzunlukta: köşeler çakışıyor ya da dönüşün doğrultusunda bir doğru üzerinde. "
              "Başka bir köşe verin.")
COLLAPSED = ("Dikdörtgenin içi duvar kalınlığından sonra kapanıyor. Boyları büyütün, kalınlığı azaltın ya da İç net seçin.")
FIXED = {"id": "perde-20", "name": "Perde 20", "components": [
    {"name": "Beton", "material": "Beton", "thickness": 0.2, "detail": "standard"}]}
FLEXIBLE = {"id": "bolme-15", "name": "Bölme 15", "width": 0.15, "components": [
    {"name": "Sıva", "material": "Sıva", "thickness": 0.02, "detail": "detailed"},
    {"name": "Tuğla", "material": "Tuğla", "detail": "standard"},
    {"name": "Sıva", "material": "Sıva", "thickness": 0.02, "detail": "detailed"}]}
STYLES = [FIXED, FLEXIBLE]


def point(x, y):
    return {"x": 0.0 if x == 0 else float(x), "y": 0.0 if y == 0 else float(y)}


def layer(id_, name, kind="layer", visible=True, locked=False, children=()):
    return {"id": id_, "name": name, "type": kind, "visible": visible, "locked": locked,
            "expanded": True, "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25},
            "children": list(children)}


SENTINEL = {"kind": "line", "id": 1, "layerId": "base", "attrs": {},
            "a": point(-1000, -1000), "b": point(-999, -1000)}
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Dikdörtgen mimari duvar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "drawingUnit": "m", "plotScale": 100, "workspace": "cad", "wallStyles": STYLES,
                 "arch": {"levels": [{"id": "zemin", "name": "Zemin", "elevation": 0.0, "height": 3.0}]}},
    "origin": point(0, 0),
    "layers": [layer("duvar", "Duvar"), layer("base", "Mevcut çizim"),
               layer("kilitli", "Kilitli katman", locked=True),
               layer("gizli", "Gizli katman", visible=False),
               layer("grup", "Grup", kind="group", children=[layer("alt", "Alt katman")])],
    "activeLayer": "duvar", "styles": {"items": [], "categories": []}, "entities": [SENTINEL],
}
BEFORE = {"ids": [1], "entities": {"1": SENTINEL}, "canUndo": False, "canRedo": False,
          "dirty": False, "revision": "same"}


def decimal(value):
    return Decimal(value) if isinstance(value, int) else Decimal.from_float(float(value))


def trig(rotation):
    """sin and cos rounded once from 80-digit series, independently of libm."""
    angle = decimal(rotation) % (2 * PI)
    if angle > PI:
        angle -= 2 * PI
    total_sin = term_sin = angle
    total_cos = term_cos = Decimal(1)
    for n in range(1, 120):
        term_sin *= -angle * angle / ((2 * n) * (2 * n + 1))
        term_cos *= -angle * angle / ((2 * n - 1) * (2 * n))
        total_sin += term_sin
        total_cos += term_cos
        if max(abs(term_sin), abs(term_cos)) < Decimal("1e-77"):
            break
    return total_sin, total_cos


def coordinates(p):
    return decimal(p["x"]), decimal(p["y"])


def path(frame, rotation=0):
    """Construct in the rectangle's orthonormal frame, then retain its first corner in CCW order."""
    kind = frame["kind"]
    s, c = trig(rotation)
    if kind == "edge":
        a, b = coordinates(frame["a"]), coordinates(frame["b"])
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = (dx * dx + dy * dy).sqrt()
        width = decimal(frame["width"])
        assert length > 0 and width != 0
        c, s = dx / length, dy / length
        along, across = length, width
    else:
        a = coordinates(frame["a"] if kind == "corners" else frame["corner"])
        q = coordinates(frame["b"] if kind == "corners" else frame["toward"])
        dx, dy = q[0] - a[0], q[1] - a[1]
        u, v = dx * c + dy * s, -dx * s + dy * c
        if kind == "corners":
            along, across = u, v
        else:
            length = decimal(frame["length"])
            width = decimal(frame["width"]) if kind == "size" else decimal(frame["area"]) / length
            along = length if u >= 0 else -length
            across = width if v >= 0 else -width
    assert along != 0 and across != 0
    u = (c * along, s * along)
    v = (-s * across, c * across)
    corners = [a, (a[0] + u[0], a[1] + u[1]),
               (a[0] + u[0] + v[0], a[1] + u[1] + v[1]), (a[0] + v[0], a[1] + v[1])]
    if along * across < 0:
        corners = [corners[0], corners[3], corners[2], corners[1]]
    # Translation before shoelace prevents large coordinates from obscuring a small room's orientation.
    shifted = [(p[0] - a[0], p[1] - a[1]) for p in corners]
    area = sum(p[0] * q[1] - p[1] * q[0] for p, q in zip(shifted, shifted[1:] + shifted[:1])) / 2
    assert area > 0
    assert abs(area - abs(along * across)) < Decimal("1e-65")
    assert corners[0] == a
    return [point(*p) for p in corners], float(abs(along)), float(abs(across))


def areas(length, width, wall_width, reference):
    """Clear and outer areas follow physical reference faces, not the stored entity's bounding box."""
    if reference == "inner":
        return length * width, (length + 2 * wall_width) * (width + 2 * wall_width)
    if reference == "outer":
        return (length - 2 * wall_width) * (width - 2 * wall_width), length * width
    return (length - wall_width) * (width - wall_width), (length + wall_width) * (width + wall_width)


def style_of(inp):
    key = inp.get("style")
    if key is None:
        return None
    return next(s for s in STYLES if key == s["id"] or key.lower() == s["name"].lower())


def expected_wall(inp, id_=0):
    pts, length, width = path(inp["frame"], inp.get("rotation", 0))
    style = style_of(inp)
    thickness = inp.get("width", (0.2 if style is FIXED else style["width"]) if style else 0.2)
    reference = inp.get("reference", "inner")
    clear, outer = areas(length, width, thickness, reference)
    assert clear > 0 and outer >= clear
    e = {"kind": "wall", "id": id_, "layerId": inp["layerId"], "attrs": inp.get("attrs", {}),
         "pts": pts, "closed": True, "width": thickness,
         "justify": {"inner": "left", "outer": "right", "center": "center"}[reference]}
    for field in ["color", "lineWeight", "lineType", "label", "height", "arch"]:
        if field in inp:
            e[field] = copy.deepcopy(inp[field])
    if style:
        e["style"] = style["id"]
        e["components"] = copy.deepcopy(style["components"])
    return e, {"referenceArea": length * width, "clearArea": clear, "outerArea": outer}


def rectangle(a=point(0, 0), b=point(4, 3), **kwargs):
    return {"layerId": "duvar", "frame": {"kind": "corners", "a": a, "b": b}, **kwargs}


def success_step(op, inp, tolerance=1e-12, next_id=2):
    entity, proof = expected_wall(inp, 0 if op != "execute" else next_id)
    warnings = ([{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.",
                  "path": "layerId"}] if inp["layerId"] == "gizli" else [])
    if op == "validate":
        output, expected = None, copy.deepcopy(BEFORE)
    elif op == "plan":
        output, expected = {"walls": [entity], "revision": "$current"}, copy.deepcopy(BEFORE)
    else:
        output = {"uids": ["$any"], "ids": [next_id], "revision": "$current"}
        expected = {"ids": [1, next_id], "entities": {"1": SENTINEL, str(next_id): entity},
                    "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed",
                    "settings": {"wallStyles": STYLES}}
    return {"op": op, "input": inp, "within": tolerance,
            "result": {"status": "completed", "output": output, "warnings": warnings}, "expect": expected}, proof


def success(name, inp, op="execute", tolerance=1e-12, setup=None, undo=False):
    step, proof = success_step(op, inp, tolerance)
    case = {"name": name, "note": "Bağımsız fiziksel alan kontrolü: " + json.dumps(proof, ensure_ascii=False), "steps": [step]}
    if setup:
        case["setup"] = setup
    if undo:
        assert op == "execute"
        case["steps"] += [
            {"op": "undo", "returns": STEP, "expect": {"ids": [1], "entities": {"1": SENTINEL},
             "canUndo": False, "canRedo": True, "dirty": True, "revision": "changed"}},
            {"op": "redo", "returns": STEP, "within": tolerance, "expect": {"ids": [1, 2],
             "entities": step["expect"]["entities"], "canUndo": True, "canRedo": False, "revision": "changed"}}]
    return case


def refused(name, inp, code, message, path_, non_finite=None, op="execute"):
    step = {"op": op, "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": copy.deepcopy(BEFORE)}
    if non_finite:
        step["nonFinite"] = non_finite
    return {"name": name, "steps": [step]}


def cases():
    out = []
    for reference in [None, "inner", "outer", "center"]:
        extra = {} if reference is None else {"reference": reference}
        out.append(success(f"4×3 m referansı {reference or 'verilmedi: inner'}; fiziksel alan ve tek geri alma adımı",
                           rectangle(**extra), undo=True))
    for x, y in [(4, 3), (-4, 3), (-4, -3), (4, -3)]:
        for ref in ["inner", "outer", "center"]:
            out.append(success(f"İlk köşe korunur, CCW: karşı köşe {x},{y}, referans {ref}",
                               rectangle(b=point(x, y), reference=ref)))
    for theta in [math.pi / 6, -math.pi / 6, math.pi / 2]:
        out.append(success(f"Radyan dönüş {theta}: iki karşı köşe dönüşün yerel eksenlerine izdüşürülür",
                           rectangle(rotation=theta)))
    for x, y in [(1, 1), (-1, 1), (-1, -1), (1, -1)]:
        out.append(success(f"Boy/en modu 4×3 m; yön çeyreği {x},{y}, CCW ilk köşeden",
                           {"layerId": "duvar", "frame": {"kind": "size", "corner": point(2, 5),
                            "length": 4, "width": 3, "toward": point(2 + x, 5 + y)}}))
    out.append(success("Boy/en modu 30° döndürülür; kalınlık ve açık dış referans",
                       {"layerId": "duvar", "frame": {"kind": "size", "corner": point(1, 2),
                        "length": 4, "width": 3, "toward": point(8, 8)}, "rotation": math.pi / 6,
                        "reference": "outer", "width": 0.25}))
    out.append(success("Alan/boy modu: 12 m² / 4 m = 3 m; iç net referans",
                       {"layerId": "duvar", "frame": {"kind": "area", "corner": point(2, 5),
                        "area": 12, "length": 4, "toward": point(3, 6)}}))
    for width in [3, -3]:
        out.append(success(f"Kenar modu en {width}: eğik 3-4-5 kenar, dönüş alanı geometriyi değiştirmez",
                           {"layerId": "duvar", "frame": {"kind": "edge", "a": point(10, 10),
                            "b": point(13, 14), "width": width}, "rotation": 0.7}))
    out.append(success("Sabit bileşenli proje stili: girişte kalınlık yoksa toplamı 0.2 m",
                       rectangle(style="Perde 20", reference="outer")))
    out.append(success("Sabit stile aynı kalınlık açıkça verilebilir; proje stili kopyalanmaz",
                       rectangle(style="perde-20", width=0.2)))
    out.append(success("Esnek proje stilinin yeni genişliği 0.15 m",
                       rectangle(style="bolme-15")))
    out.append(success("Esnek stil 0.3 m genişlik alır, bileşen kaynağı korunur",
                       rectangle(style="BÖLME 15", width=0.3)))
    out.append(success("Yükseklik, kat/evre/durum, etiket, öznitelik ve nesnenin görünüşü saklanır",
                       rectangle(width=0.25, height=2.8, arch={"level": "zemin", "phase": "new",
                                 "status": "proposed", "discipline": "architecture"},
                                 attrs={"kod": "D01"}, label="D01", color="#7F7F7F", lineWeight=0.5,
                                 lineType="dashed")))
    out.append(success("Plan bütün duvarı id 0 ile gösterir; belgeye ve undo'ya dokunmaz",
                       rectangle(reference="center", style="perde-20"), op="plan"))
    out.append(success("Validate yalnız doğrular; duvar, undo ve revision yazılmaz", rectangle(), op="validate"))
    out.append(success("Gizli katman açık uyarıyla aynı duvarı alır", rectangle(layerId="gizli")))
    for unit in ["mm", "cm", "m", "in", "ft"]:
        setup = copy.deepcopy(SETUP)
        setup["settings"]["drawingUnit"] = unit
        out.append(success(f"Gösterim birimi {unit}: telde 4×3 m ve 0.2 m kalınlık değişmez",
                           rectangle(), setup=setup))
    out.append(success("Büyük koordinat: 500000,4400000 başlangıç ve küçük mimari boylar korunur",
                       rectangle(a=point(500000, 4400000), b=point(500004, 4400003)), tolerance=1e-8))
    out.append(success("Büyük koordinatta 30° boy/en ve dış yüz referansı",
                       {"layerId": "duvar", "frame": {"kind": "size", "corner": point(500000, 4400000),
                        "length": 4, "width": 3, "toward": point(500010, 4400010)},
                        "rotation": math.pi / 6, "reference": "outer", "width": 0.25}, tolerance=1e-8))
    # Contract-level refusals: none of these are derived by running the new command.
    for frame, name, message, where in [
        ({"kind": "corners", "a": point(0, 0), "b": point(0, 3)}, "sıfır yatay boy", DEGENERATE, "frame"),
        ({"kind": "corners", "a": point(0, 0), "b": point(4, 0)}, "sıfır dikey boy", DEGENERATE, "frame"),
        ({"kind": "edge", "a": point(0, 0), "b": point(0, 0), "width": 3}, "uzunluksuz kenar", DEGENERATE, "frame"),
        ({"kind": "edge", "a": point(0, 0), "b": point(4, 0), "width": 0}, "sıfır kenar eni",
         "Genişlik sıfır olamaz: kenarın solu için artı, sağı için eksi bir genişlik verin.", "frame.width"),
        ({"kind": "size", "corner": point(0, 0), "length": 0, "width": 3, "toward": point(1, 1)},
         "sıfır sayısal uzunluk", "Uzunluk sıfırdan büyük olmalı.", "frame.length"),
        ({"kind": "size", "corner": point(0, 0), "length": 4, "width": -3, "toward": point(1, 1)},
         "negatif sayısal en", "Genişlik sıfırdan büyük olmalı.", "frame.width"),
        ({"kind": "area", "corner": point(0, 0), "area": 0, "length": 4, "toward": point(1, 1)},
         "sıfır alan", "Alan sıfırdan büyük olmalı.", "frame.area"),
    ]:
        out.append(refused(f"Geçersiz çerçeve: {name}; belge olduğu gibi kalır", {"layerId": "duvar", "frame": frame},
                           "degenerate_shape" if message == DEGENERATE else "invalid_distance", message, where))
    for reference, width in [("outer", 1.5), ("outer", 2), ("center", 3), ("center", 4)]:
        for op in ["validate", "plan", "execute"]:
            out.append(refused(f"İç boşluk kapanır: {reference}, kalınlık {width}, {op} yazmadan reddeder",
                               rectangle(reference=reference, width=width), "invalid_wall", COLLAPSED, "frame", op=op))
    for width in [0, -0.2, 101]:
        out.append(refused(f"Geçersiz duvar kalınlığı {width}; üst düzey width yolu",
                           rectangle(width=width), "invalid_wall",
                           f"Duvarın genişliği {width}; sıfırdan büyük, en çok 100 m olmalı.", "width"))
    out.append(refused("Sabit stil toplamından farklı açık genişlik reddedilir, ignore edilmez",
                       rectangle(style="perde-20", width=0.3), "invalid_wall",
                       "Duvarın genişliği 0.3 m, bileşenlerin toplamı 0.2 m: kalınlığı verilmeyen bileşen olmadığından genişlik toplama eşit olmalı.", "width"))
    # Existing public wall-style refusal: project names precede the built-in
    # names. Exercise each operation without resolving a fabricated style or
    # deriving an expected answer from the new native command.
    for op in ["validate", "plan", "execute"]:
        out.append(refused(f"Olmayan duvar stili {op} sırasında style yoluyla reddedilir",
                           rectangle(style="Ahşap 12"), "style_not_found",
                           "“Ahşap 12” adında ya da kimliğinde duvar stili yok. Var olanlar: Perde 20, Bölme 15, Dış duvar 25, İç duvar 15, Gazbeton 20, Betonarme perde 25, Bölme duvar 10.",
                           "style", op=op))
    out.append(refused("Olmayan kat kimliği yeni girişte arch.level olarak raporlanır",
                       rectangle(arch={"level": "yok"}), "level_not_found",
                       "Projede “yok” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", "arch.level"))
    out.append(refused("Sıfır duvar yüksekliği reddedilir", rectangle(height=0), "invalid_wall",
                       "Duvarın yüksekliği 0; sıfırdan büyük olmalı.", "height"))
    for layer_id, code, message in [
        ("kilitli", "layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."),
        ("yok", "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
        ("grup", "not_a_layer", "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin."),
    ]:
        out.append(refused(f"Katmana yazma reddi: {code}; mevcut nesne korunur", rectangle(layerId=layer_id), code, message, "layerId"))
    out.append(refused("Çizgi kalınlığı 100 kâğıt mm üstünde olamaz", rectangle(lineWeight=101), "invalid_line_weight",
                       "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"))
    out.append(refused("Başında sıfır olan revision reddedilir", rectangle(expectedRevision="07"), "invalid_revision",
                       "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    for field, word, name in [("width", "NaN", "Genişlik"), ("height", "Infinity", "Yükseklik"), ("rotation", "-Infinity", "Dönüş")]:
        out.append(refused(f"Sonlu olmayan {field} açık alan yoluyla reddedilir", rectangle(**{field: 0.2}), "not_finite",
                           f"{name} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", field, {field: word}))
    out.append(refused("İlk köşenin NaN koordinatı duvar noktalarına dönüştürülmeden reddedilir", rectangle(), "not_finite",
                       "İlk köşenin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                       "frame.a.x", {"frame.a.x": "NaN"}))
    out.append(refused("Edge modunda geometride kullanılmayan rotation da sonlu olmalı",
                       {"layerId": "duvar", "frame": {"kind": "edge", "a": point(0, 0), "b": point(4, 0), "width": 3}, "rotation": 0},
                       "not_finite", "Dönüş sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "rotation", {"rotation": "NaN"}))
    first, _ = success_step("execute", rectangle())
    out.append({"name": "Eski revision ile yazma çatışması: ikinci duvar yazılmaz, ilk duvar ve yabancı çizgi korunur", "steps": [
        {"op": "captureRevision", "as": "prepared"}, first,
        {"op": "execute", "input": rectangle(a=point(10, 10), b=point(14, 13), expectedRevision="$prepared"),
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
          "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."}},
         "expect": {"ids": [1, 2], "entities": first["expect"]["entities"], "canUndo": True, "canRedo": False,
                    "dirty": True, "revision": "same"}}]})
    return out


def document():
    # Independent scalar checks make the meaning of inner/outer/centre explicit.
    assert areas(4, 3, 0.2, "inner")[0] == 12
    assert math.isclose(areas(4, 3, 0.2, "outer")[0], 9.36, rel_tol=0, abs_tol=1e-14)
    assert math.isclose(areas(4, 3, 0.2, "center")[0], 10.64, rel_tol=0, abs_tol=1e-14)
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Dikdörtgen duvar: iç net/dış/eksen referansı, CCW, stiller, plan, undo ve retler",
            "note": "Beklenenler scripts/fixtures/walls_rectangle_cases.py'nin bağımsız 80 basamaklı çerçeve hesabından; KentOS/geometri kodu veya yeni komut çıktısı kullanılmadı. Tel birimi metre; belge birimi yalnız gösterim. İlk mevcut nesne 1, yeni kapalı duvar 2. Her başarıdaki note fiziksel alanların bağımsız kontrolünü verir.",
            "setup": SETUP, "cases": cases()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = document()
    encoded = json.dumps(doc, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != encoded:
            print(f"{OUT.relative_to(ROOT)} bağımsız başvurudan farklı.")
            return 1
        print(f"{COMMAND}: {len(doc['cases'])} durum bağımsız başvuruyla aynı.")
    else:
        OUT.write_text(encoded)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
