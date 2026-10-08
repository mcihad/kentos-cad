"""The shared cases of the project's annotation heights and how annotations
follow the scale (docs/adr/0205 §1, §3).

    python3 scripts/fixtures/annotation_scale_cases.py           # writes the file
    python3 scripts/fixtures/annotation_scale_cases.py --check   # writes nothing; compares

Writes fixtures/text/v1/scale.json. The rules are written here from the ADR
on their own, not from an implementation's output; the contract's rules
(crates/shared/contracts/src/annotation_scale.rs) and the web's
(apps/web/src/model/annotationScale.ts) are held to them. Heights are
compared exactly, so the expressions are the ADR's, in float64:

- A kind's height is the project's, else its default: Yazı, Kılavuz, Ölçü and
  Tablo 2.5 mm, Koordinat yazısı, Km yazısı and Kenar ve köşe yazıları 2 mm.
  A height holds when finite, over 0 and at most 100 mm; a project keeps the
  heights that hold and differ from their default, and none when none is left.
- In the drawing a height is mm / 1000 × the scale.
- When the scale or a height changes, an annotation whose height equals its
  rule's old height takes the new one; any other stays:
  - a text whose style has a fixed height follows the style's mm at the old
    and the new scale; else a text whose `Tür` is “Kenar ölçüsü” or “Köşe
    noktası” follows Kenar ve köşe yazıları, any other Yazı; a linked text
    (`labelOf`) stays; a multi-line text's box width is multiplied by new /
    old;
  - a leader follows Kılavuz; a dimension its style's mm (Standart: Ölçü);
  - a table its text style's fixed height, else Tablo; its rows, columns and
    frame are multiplied by new / old;
  - any other kind stays.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/text/v1/scale.json"

KINDS = ["text", "leader", "dimension", "table", "coordinate", "station", "measure"]
LABEL = {
    "text": "Yazı",
    "leader": "Kılavuz",
    "dimension": "Ölçü",
    "table": "Tablo",
    "coordinate": "Koordinat yazısı",
    "station": "Km yazısı",
    "measure": "Kenar ve köşe yazıları",
}
DEFAULT_MM = {"text": 2.5, "leader": 2.5, "dimension": 2.5, "table": 2.5, "coordinate": 2.0, "station": 2.0, "measure": 2.0}
MAX_MM = 100.0
MEASURE_TEXTS = ("Kenar ölçüsü", "Köşe noktası")


def num(x):
    """A number as Rust's Display and JavaScript's String write it here."""
    if x == int(x) and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def holds(mm):
    return isinstance(mm, (int, float)) and math.isfinite(mm) and 0 < mm <= MAX_MM


def mm_of(heights, kind):
    v = (heights or {}).get(kind)
    return DEFAULT_MM[kind] if v is None else v


def paper(mm, scale):
    return mm / 1000.0 * scale


def problem(heights):
    for kind in KINDS:
        v = heights.get(kind)
        if v is not None and not holds(v):
            return [kind, f"{LABEL[kind]} yüksekliği kâğıtta sıfırdan büyük, en çok {num(MAX_MM)} mm olmalı; {num(v)} verildi."]
    return None


def sanitized(heights):
    out = {k: heights[k] for k in KINDS if heights.get(k) is not None and holds(heights[k]) and heights[k] != DEFAULT_MM[k]}
    return out or None


def is_none(change):
    return change["fromScale"] == change["toScale"] and all(mm_of(change["from"], k) == mm_of(change["to"], k) for k in KINDS)


def span_kind(change, kind):
    return paper(mm_of(change["from"], kind), change["fromScale"]), paper(mm_of(change["to"], kind), change["toScale"])


def span_style(change, mm):
    return paper(mm, change["fromScale"]), paper(mm, change["toScale"])


def moved(height, span):
    was, now = span
    return now if height == was and now != height else None


def follow(e, change, text_styles, dimension_styles):
    def fixed(sid):
        for s in text_styles:
            if s["id"] == sid:
                return s.get("height")
        return None

    kind = e["kind"]
    if kind == "text":
        if "labelOf" in e:
            return None
        mm = fixed(e.get("textStyle"))
        if mm is not None:
            span = span_style(change, mm)
        else:
            span = span_kind(change, "measure" if e["attrs"].get("Tür") in MEASURE_TEXTS else "text")
        now = moved(e["height"], span)
        if now is None:
            return None
        factor = now / e["height"]
        out = dict(e, height=now)
        if "boxWidth" in e:
            out["boxWidth"] = e["boxWidth"] * factor
        return out
    if kind == "leader":
        now = moved(e["height"], span_kind(change, "leader"))
        return None if now is None else dict(e, height=now)
    if kind == "dimension":
        style = next((s for s in dimension_styles if s["id"] == e.get("dimStyle")), None)
        span = span_style(change, style["height"]) if style else span_kind(change, "dimension")
        now = moved(e["height"], span)
        return None if now is None else dict(e, height=now)
    if kind == "table":
        mm = fixed(e.get("textStyle"))
        span = span_style(change, mm) if mm is not None else span_kind(change, "table")
        now = moved(e["height"], span)
        if now is None:
            return None
        factor = now / e["height"]
        out = dict(e, height=now, rows=[r * factor for r in e["rows"]], columns=[c * factor for c in e["columns"]])
        if "frame" in e:
            out["frame"] = e["frame"] * factor
        return out
    return None


def base(i, **attrs):
    return {"id": i, "layerId": "cizim", "attrs": dict(attrs)}


ADA = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0301", "name": "Ada no", "font": "arimo", "height": 3.5}
NOT = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0302", "name": "Not", "font": "barlow"}
OLCU = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0401", "name": "Mimari", "height": 3.0, "arrow": "closed"}
TEXT_STYLES = [ADA, NOT]
DIMENSION_STYLES = [OLCU]


def cases():
    out = []

    def add(name, op, given, expect):
        out.append({"name": name, "op": op, **given, "expect": expect})

    # ── The heights ─────────────────────────────────────────────────────
    for name, h in [
        ("Hiçbiri: varsayılanlar", {}),
        ("Yazı 3,5 mm, koordinat 1,8 mm", {"text": 3.5, "coordinate": 1.8}),
        ("Kılavuz 0 mm", {"leader": 0.0}),
        ("Ölçü 101 mm", {"dimension": 101.0}),
        ("Tablo 100 mm olabilir", {"table": 100.0}),
    ]:
        add(name, "problem", {"heights": h}, problem(h))
    for name, h in [
        ("Varsayılana eşit olan yazılmaz", {"text": 2.5, "measure": 2.0}),
        ("Kendi yükseklikleri kalır, varsayılanlar gider", {"text": 3.0, "leader": 2.5, "station": 1.5}),
        ("Tutmayan değer gider", {"table": -1.0, "dimension": 3.5}),
    ]:
        add(name, "sanitize", {"heights": h}, sanitized(h))
    for name, change in [
        ("Hiçbir şey değişmedi", {"fromScale": 1000.0, "toScale": 1000.0, "from": {}, "to": {"text": 2.5}}),
        ("Ölçek değişti", {"fromScale": 1000.0, "toScale": 500.0, "from": {}, "to": {}}),
        ("Yükseklik değişti", {"fromScale": 1000.0, "toScale": 1000.0, "from": {}, "to": {"leader": 3.0}}),
    ]:
        add(name, "isNone", {"change": change}, is_none(change))

    # ── Following ───────────────────────────────────────────────────────
    scale_only = {"fromScale": 1000.0, "toScale": 500.0, "from": {}, "to": {}}
    heights_only = {"fromScale": 500.0, "toScale": 500.0, "from": {"text": 2.5}, "to": {"text": 3.5, "leader": 3.0, "dimension": 1.8, "table": 3.0, "measure": 2.5}}
    both = {"fromScale": 1000.0, "toScale": 250.0, "from": {"text": 3.0}, "to": {"text": 2.0, "dimension": 2.0}}
    at = lambda kind, scale, heights=None: paper(mm_of(heights, kind), scale)

    text = dict(base(1), kind="text", p={"x": 10.0, "y": 20.0}, text="Ada 101", height=at("text", 1000.0), rotation=0.0)
    paragraph = dict(base(2), kind="text", p={"x": 0.0, "y": 0.0}, text="Uzun bir not", height=at("text", 1000.0), rotation=15.0, boxWidth=40.0, lineSpacing=1.2)
    edge = dict(base(3, **{"Tür": "Kenar ölçüsü", "Uzunluk (m)": "12.345"}), kind="text", p={"x": 5.0, "y": 0.0}, text="12.35", height=at("measure", 1000.0), rotation=30.0, align="baselineCenter")
    corner = dict(base(4, **{"Tür": "Köşe noktası", "Nokta": "1"}), kind="text", p={"x": 0.0, "y": 0.0}, text="1", height=at("measure", 1000.0), rotation=0.0)
    coordinate = dict(base(5), kind="text", p={"x": 0.0, "y": 0.0}, text="Y=487000.00", height=at("coordinate", 1000.0), rotation=0.0)
    own = dict(base(6), kind="text", p={"x": 0.0, "y": 0.0}, text="Elle", height=1.75, rotation=0.0)
    styled = dict(base(7), kind="text", p={"x": 0.0, "y": 0.0}, text="101", height=paper(ADA["height"], 1000.0), rotation=0.0, textStyle=ADA["id"], font="arimo")
    styled_own = dict(styled, id=8, height=at("text", 1000.0))
    no_height_style = dict(base(9), kind="text", p={"x": 0.0, "y": 0.0}, text="Not", height=at("text", 1000.0), rotation=0.0, textStyle=NOT["id"], font="barlow")
    linked = dict(base(10), kind="text", p={"x": 0.0, "y": 0.0}, text="12", height=at("text", 1000.0), rotation=0.0, labelOf="0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0999", labelScale=1000.0)
    leader = dict(base(11), kind="leader", pts=[{"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 3.0}], text="Mevcut bina", height=at("leader", 1000.0), rotation=0.0, arrow="open", arrowSize=1.5)
    dimension = dict(base(12), kind="dimension", a={"x": 0.0, "y": 0.0}, b={"x": 10.0, "y": 0.0}, offset=3.0, height=at("dimension", 1000.0))
    dim_styled = dict(base(13), kind="dimension", a={"x": 0.0, "y": 0.0}, b={"x": 10.0, "y": 0.0}, offset=-2.0, height=paper(OLCU["height"], 1000.0), dimStyle=OLCU["id"], arrow="closed")
    table = dict(base(14), kind="table", p={"x": 0.0, "y": 10.0}, rotation=0.0, height=at("table", 1000.0), rows=[6.0, 5.5, 5.5], columns=[20.0, 12.5], cells=[["Nokta", "Y"], ["1", "487000.00"], ["2", "487010.00"]], header=True, frame=0.7)
    polyline = dict(base(15), kind="polyline", pts=[{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}])

    for change_name, change in [("1:1000'den 1:500'e", scale_only), ("1:1000'den 1:250'ye, Yazı 3 mm'den 2 mm'ye", both)]:
        for name, e in [
            ("yazı", text),
            ("çok satırlı yazının kutusu da", paragraph),
            ("kenar uzunluğu yazısı", edge),
            ("köşe numarası", corner),
            ("koordinat yazısı izlemez", coordinate),
            ("elle değiştirilmiş yazı kalır", own),
            ("stilin sabit yüksekliğindeki yazı", styled),
            ("stilli ama kendi yüksekliğinde: kalır", styled_own),
            ("stilin yüksekliği yok: Yazı'yı izler", no_height_style),
            ("nesneye bağlı yazı kalır", linked),
            ("kılavuz", leader),
            ("Standart ölçü", dimension),
            ("stilli ölçü", dim_styled),
            ("tablo, satırları, sütunları ve çerçevesiyle", table),
            ("çoklu çizgi kalır", polyline),
        ]:
            e2 = dict(e)
            if change is both and e is text:
                e2 = dict(e, height=paper(3.0, 1000.0))
            if change is both and e is paragraph:
                e2 = dict(e, height=paper(3.0, 1000.0))
            add(f"{change_name}: {name}", "follow", {"change": change, "textStyles": TEXT_STYLES, "dimensionStyles": DIMENSION_STYLES, "entity": e2}, follow(e2, change, TEXT_STYLES, DIMENSION_STYLES))
    # Heights alone at 1:500: each kind's objects at its old height.
    for name, e in [
        ("yazı", dict(text, height=at("text", 500.0))),
        ("kenar uzunluğu", dict(edge, height=at("measure", 500.0))),
        ("kılavuz", dict(leader, height=at("leader", 500.0))),
        ("ölçü", dict(dimension, height=at("dimension", 500.0))),
        ("tablo", dict(table, height=at("table", 500.0))),
        ("koordinat yazısı izlemez", dict(coordinate, height=at("coordinate", 500.0))),
    ]:
        add(f"Yükseklikler 1:500'de: {name}", "follow", {"change": heights_only, "textStyles": TEXT_STYLES, "dimensionStyles": DIMENSION_STYLES, "entity": e}, follow(e, heights_only, TEXT_STYLES, DIMENSION_STYLES))
    return out


def main():
    doc = {
        "format": "kentos.annotation-scale-cases",
        "version": 1,
        "title": "Yazı yükseklikleri: projenin yükseklikleri, denetimi ve ölçek ya da yükseklik değişince izleyenler",
        "note": "ADR 0205 §1, §3. scripts/fixtures/annotation_scale_cases.py kurallardan yazar; sözleşmenin (annotation_scale.rs) ve web'in (model/annotationScale.ts) kuralları bunu geçer. Yükseklikler metrede (kâğıtta mm / 1000 × ölçek), eşitlikle.",
        "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
