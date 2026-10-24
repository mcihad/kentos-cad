"""Eğri boyunca yazı's shared command cases (docs/adr/0622 §9, 0196 §4), without KentOS code.

- `along(...)`: `cad.annotations.write`'s `along` work, imported by annotations_write_cases.py: the text written on the
  piece of the curve its letters take, where the click puts it (the curve's path nearest the click; the piece turned
  so that it reads), its letters as long as the drawing typeface's measured advances say;
- `curve_cases()`: `cad.texts.edit`'s `fit`, `turn` and `straighten`, imported by text_command_cases.py: Eğriye oturt
  puts each one-line text on the curve where its box's middle falls (the box 0.23·h under its baseline to 1.15·h over
  it, as long as its letters), Doğrultuya döndür turns each straight text to the clicked edge's direction, readable
  (over 90° and at most 270° turned half round), Düzleştir makes each curved text straight at its first letter, along
  its direction.

The letters, the pieces and the straightening are text_along_cases.py's (written from docs/adr/0196 on their own).
"""
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import text_along_cases as tac  # noqa: E402

# Alignment shares (docs/adr/0145): along (left 0, centre ½, right 1) and up (baseline 0, bottom −0.2, middle ½, top 1).
ALONG = {"baselineLeft": 0.0, "baselineCenter": 0.5, "baselineRight": 1.0, "bottomLeft": 0.0, "bottomCenter": 0.5,
         "bottomRight": 1.0, "middleLeft": 0.0, "middleCenter": 0.5, "middleRight": 1.0, "topLeft": 0.0,
         "topCenter": 0.5, "topRight": 1.0}
UP = {"baselineLeft": 0.0, "baselineCenter": 0.0, "baselineRight": 0.0, "bottomLeft": -0.2, "bottomCenter": -0.2,
      "bottomRight": -0.2, "middleLeft": 0.5, "middleCenter": 0.5, "middleRight": 0.5, "topLeft": 1.0,
      "topCenter": 1.0, "topRight": 1.0}


def length_of(words, height, font, width=1.0):
    return sum(tac.advances({"text": words, "height": height, "widthFactor": width, "drawingFont": font}))


def placed(curve, click, words, height, align, font):
    """The text's fields as the command writes it: its point, turn and piece."""
    share = ALONG.get(align, 0.0)
    framed = tac.piece(curve, click, length_of(words, height, font), share)
    out = {"kind": "text", "text": words, "height": height, "align": align, "p": framed["p"],
           "rotation": framed["rotation"], "path": framed["path"]}
    return out


def box_middle(t, font):
    """A straight text's box's middle: w·(½ − a) along and h·(0.46 − b) up from its point."""
    w = length_of(t["text"], t["height"], font, t.get("widthFactor", 1.0))
    a = ALONG.get(t.get("align"), 0.0)
    b = UP.get(t.get("align"), 0.0)
    r = math.radians(t["rotation"])
    T, N = (math.cos(r), math.sin(r)), (-math.sin(r), math.cos(r))
    along, up = w * (0.5 - a), t["height"] * (0.46 - b)
    return (t["p"]["x"] + T[0] * along + N[0] * up, t["p"]["y"] + T[1] * along + N[1] * up)


def fitted(t, curve, font):
    framed = tac.piece(curve, box_middle(t, font), length_of(t["text"], t["height"], font,
                                                              t.get("widthFactor", 1.0)), 0.5)
    return {"p": framed["p"], "rotation": framed["rotation"], "path": framed["path"]}


def turned(direction):
    return tac.turn_of(direction)


def straightened(t, font):
    s = tac.straight({**t, "drawingFont": font})
    return {"p": s["p"], "rotation": s["rotation"]}


# ── cad.texts.edit: Eğriye oturt, Doğrultuya döndür, Düzleştir ───────────────────────────────────────────────────

FONT = "barlow"
TAKEN = {
    "fit": "Verilen yazıların hiçbiri eğriye oturmaz: yalnız tek satırlık, nesneye bağlı olmayan yazılar oturur.",
    "turn": "Verilen yazıların hiçbiri döndürülmez: eğri boyunca yazıyı önce Düzleştir.",
    "straighten": "Verilen yazıların hiçbiri eğri boyunca değil; düzleştirilecek yazı yok.",
}
STEP = {"fit": "Eğriye oturt", "turn": "Doğrultuya döndür", "straighten": "Düzleştir"}


def curve_cases(document, X, Y, messages):
    """The cases, each with the drawing built by `document`; `messages` the command's shared words (uid, not found,
    not a text, locked)."""

    def W(x, y):
        return {"x": X + x, "y": Y + y}

    def text(i, words, x, y, layer="yazi", **more):
        return {"kind": "text", "id": i, "layerId": layer, "attrs": {}, "p": W(x, y), "text": words, "height": 2,
                "rotation": 0, **more}

    line = {"kind": "line", "id": 4, "layerId": "mekan", "attrs": {}, "a": W(-10, 0), "b": W(60, 0)}
    arc = {"kind": "arc", "id": 5, "layerId": "mekan", "attrs": {}, "c": W(100, 0), "r": 20, "a0": math.radians(20),
           "a1": math.radians(160)}
    curved_on_arc = placed(arc, (X + 100, Y + 21), "Çevre", 2, "bottomCenter", FONT)
    curved = {"kind": "text", "id": 6, "layerId": "yazi", "attrs": {}, **{k: v for k, v in curved_on_arc.items()
                                                                          if k != "kind"}, "rotation": curved_on_arc["rotation"]}
    entities = [
        text(1, "Ada yolu", 0, 5),
        text(2, "Kanal", 20, 5, rotation=30, align="middleCenter"),
        text(3, "İki\nsatır", 40, 5),
        line,
        arc,
        curved,
        text(7, "Kilit", 0, -5, layer="kilitli"),
        {"kind": "line", "id": 8, "layerId": "mekan", "attrs": {}, "a": W(0, -20), "b": W(10, -10)},
        {"kind": "line", "id": 9, "layerId": "mekan", "attrs": {}, "a": W(0, -30), "b": W(-10, -40)},
    ]
    setup = document(entities)
    by_id = {e["id"]: e for e in entities}
    ids = [e["id"] for e in entities]
    U = lambda i: f"$uidOf:{i}"  # noqa: E731

    def done(name, edit, changed, skipped=0):
        out = {"changed": [U(i) for i in changed], "revision": "$current"}
        if skipped:
            out["skipped"] = skipped
        step = {"op": "execute", "input": {"edit": edit}, "within": 1e-9,
                "result": {"status": "completed", "output": out, "warnings": []},
                "expect": {"ids": ids, "fields": {str(i): f for i, f in changed.items()}, "canUndo": True,
                           "dirty": True}}
        back = {"op": "undo", "returns": STEP[edit["kind"]], "expect": {"ids": ids, "canUndo": False}}
        return {"name": name, "setup": setup, "steps": [step, back]}

    def refused(name, edit, code, message, path, non_finite=None):
        step = {"op": "execute", "input": {"edit": edit}}
        if non_finite:
            step["nonFinite"] = non_finite
        step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
        step["expect"] = {"ids": ids, "dirty": False, "revision": "same"}
        return {"name": name, "setup": setup, "steps": [step]}

    def fit(uids, curve):
        return {"kind": "fit", "uids": [U(i) for i in uids], "curve": U(curve)}

    def turn(uids, edge, at):
        return {"kind": "turn", "uids": [U(i) for i in uids], "edge": U(edge), "at": W(*at)}

    def flat(uids):
        return {"kind": "straighten", "uids": [U(i) for i in uids]}

    def fitted_of(i, curve):
        return fitted(by_id[i], curve, FONT)

    def turned_to(d):
        return {"rotation": turned({"x": d[0], "y": d[1]})}

    curved_t = {**curved}
    return [
        done("Eğriye oturt: yazı çizginin üstüne, kutusunun ortası eğride", fit([1], 4), {1: fitted_of(1, line)}),
        done("Eğriye oturt: iki yazı yayın üstüne", fit([1, 2], 5), {1: fitted_of(1, arc), 2: fitted_of(2, arc)}),
        done("Eğriye oturt: çok satırlı yazı atlanır ve sayılır", fit([1, 3], 4), {1: fitted_of(1, line)}, skipped=1),
        refused("Eğriye oturt: oturacak yazı yok", fit([3], 4), "text_not_taken", TAKEN["fit"], "edit.uids"),
        refused("Eğriye oturt: eğri yazı olamaz", fit([1], 2), "not_a_path",
                "Yazıların oturacağı nesne çizgi, yay, daire, alan ya da eğri olmalı.", "edit.curve"),
        refused("Eğriye oturt: eğri yok", {**fit([1], 4), "curve": messages["fake"]}, "entity_not_found",
                messages["notFound"].format(messages["fake"]), "edit.curve"),
        refused("Eğriye oturt: kilitli katmandaki yazı", fit([7], 4), "layer_locked", messages["locked"],
                "edit.uids[0]"),
        done("Doğrultuya döndür: kenarın doğrultusuna", turn([1, 2], 8, (5, -15)), {1: turned_to((10, 10)),
                                                                                    2: turned_to((10, 10))}),
        done("Doğrultuya döndür: ters okunan doğrultu yarım tur döner", turn([1], 9, (-5, -35)),
             {1: turned_to((-10, -10))}),
        done("Doğrultuya döndür: yayın teğeti, okunur", turn([1], 5, (100 + 10 * math.sqrt(2), 10 * math.sqrt(2))),
             {1: turned_to((-1, 1))}),
        done("Doğrultuya döndür: eğri boyunca yazı atlanır", turn([1, 6], 8, (5, -15)), {1: turned_to((10, 10))},
             skipped=1),
        refused("Doğrultuya döndür: döndürülecek yazı yok", turn([6], 8, (5, -15)), "text_not_taken", TAKEN["turn"],
                "edit.uids"),
        refused("Doğrultuya döndür: tıklanan yer sonlu değil", turn([1], 8, (5, -15)), "not_finite",
                "Tıklanan yerin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla "
                "verin.", "edit.at.x", non_finite={"edit.at.x": "NaN"}),
        refused("Doğrultuya döndür: kenar yazı olamaz", turn([1], 2, (20, 5)), "not_a_path",
                "Doğrultusu alınacak nesnenin kenarı yok; çizgi, yay ya da alan verin.", "edit.edge"),
        done("Düzleştir: eğri boyunca yazı düz olur", flat([6]), {6: straightened(curved_t, FONT)}),
        done("Düzleştir: düz yazı atlanır", flat([6, 1]), {6: straightened(curved_t, FONT)}, skipped=1),
        refused("Düzleştir: düzleştirilecek yazı yok", flat([1]), "text_not_taken", TAKEN["straighten"], "edit.uids"),
        refused("Düzleştir: yazı değil", flat([4]), "not_a_text", messages["notText"], "edit.uids[0]"),
        refused("Düzleştir: kimlik değil", {"kind": "straighten", "uids": ["R-12"]}, "invalid_uid",
                messages["uid"].format("R-12"), "edit.uids[0]"),
    ]
