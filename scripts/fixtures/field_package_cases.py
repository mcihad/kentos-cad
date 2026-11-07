#!/usr/bin/env python3
"""The shared cases of `cad.field.package` (Saha paketi; plan/map-cad.md MAP-19, docs/adr/1920), without KentOS code.

    python3 scripts/fixtures/field_package_cases.py           # writes fixtures/commands/v1/cad.field.package.json
    python3 scripts/fixtures/field_package_cases.py --check   # writes nothing; compares

The drawing is asbuilt_cases.py's (TUREF / TM36, the design points on `tasarim` with their names in Ad, the kerb on
`bordur`, the design surface “Tasarım”). The rules, by hand:

- the design's points are the points of its layer in the drawing's order, their names in the field asked (none: Ad);
  its lines the lines and polylines of its layer (a polyline's vertices, a line's two ends);
- the digest is asbuilt_cases.py's: SHA-256 of `p<TAB>name<TAB>x<TAB>y<TAB>z` (six decimals, `-` for none) for each
  design point, then `l<TAB>x,y;x,y` for each line, each line ended by a newline;
- the system is EPSG's 5256, “TUREF / TM36”; the project says no heights' system and no epoch, so none is written;
  a project that says them (TUREF at 2005.0, TUDKA-99's orthometric heights by TG-20) has them written, the heights
  in the project's words with EPSG:5775's name as EPSG's registry has it (“Antalya height”);
- the limits and the notes are written as given; a limit not given is not written.

The file's bytes and SHA-256 are the command's (`$any`); a plan writes nothing (no size, no digest).
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import asbuilt_cases as asb  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.field.package.json"
W = 1e-9
ANY = "$any"
STAMP = "2026-11-07T10:00:00"
SAME = {"revision": "same", "canUndo": False}
EPOCH = 2005.0
HEIGHTS = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20"}}
HEIGHTS_WORDS = "Ortometrik yükseklik (Antalya height; geoit TG-20)"
NOT_MAP = ("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                                 "ikisinden biri değil.")


def with_heights():
    """The drawing in a project that says its epoch and heights' system."""
    s = asb.setup()
    s["settings"]["coordinateEpoch"] = EPOCH
    s["settings"]["heights"] = copy.deepcopy(HEIGHTS)
    return s


def points():
    design, _, _ = asb.ids()
    out = []
    for (name, x, y, z), u in zip(asb.DESIGN, design):
        p = {"name": name, "at": [asb.X0 + x, asb.Y0 + y], "uid": u}
        if z is not None:
            p["z"] = z
        out.append(p)
    return out


def answer(inp, written=True):
    design, lines = "design" in inp, "lines" in inp
    out = {"path": inp["path"], "bytes": ANY if written else 0, "sha256": ANY if written else "",
           "digest": asb.digest(with_design=design, with_lines=lines), "revision": "$current",
           "points": points() if design else [], "lines": len(asb.LINES) if lines else 0,
           "vertices": sum(len(pts) for _, pts in asb.LINES) if lines else 0}
    if "surface" in inp:
        out["surface"] = {"id": "t-1", "name": "Tasarım", "digest": ANY}
    out.update({"epsg": "EPSG:5256", "system": "TUREF / TM36"})
    for k in ("horizontal", "vertical", "quality"):
        if k in inp:
            out[k] = inp[k]
    if inp.get("notes"):
        out["notes"] = inp["notes"]
    out["stamp"] = inp.get("stamp", ANY)
    return out


NAMES = iter(range(1, 1000))


def inp(**over):
    """An input; each its own file, the runner's temporary folder being one for the whole run."""
    base = {"design": "tasarim", "lines": "bordur", "surface": "Tasarım", "horizontal": 0.02, "vertical": 0.03,
            "quality": 0.05, "notes": ["Kontrol noktaları KN1 ve KN2'den başlanır.", "Kotlar TUDKA-99'a göredir."],
            "path": f"$tmp:saha-{next(NAMES)}.zip", "stamp": STAMP}
    base.update(over)
    return {k: v for k, v in base.items() if v is not None}


def ok(name, i, op="execute", setup_=None, extra=None):
    out = None if op == "validate" else answer(i, written=op == "execute")
    if out is not None and extra:
        out.update(extra)
    case = {"name": name, "steps": [{"op": op, "input": i, "result": {"status": "completed", "output": out,
                                                                     "warnings": []}, "expect": SAME, "within": W}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, i, code, message, path_=None, setup_=None, steps_before=()):
    err = {"code": code, "message": message}
    if path_ is not None:
        err["path"] = path_
    steps = list(steps_before) + [{"op": "execute", "input": i, "result": {"status": "failed", "error": err},
                                   "expect": SAME}]
    case = {"name": name, "steps": steps}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    bad = "invalid_field_package"
    def once(path):
        return {"op": "execute", "input": inp(path=path),
                "result": {"status": "completed", "output": answer(inp(path=path)), "warnings": []},
                "expect": SAME, "within": W}
    again = inp(path="$tmp:saha-iki.zip", overwrite=True)
    out = [
        ok("tasarım noktaları, bordür ve yüzey; üç kabul sınırı ve iki notla", inp()),
        ok("yalnız tasarım noktaları", inp(lines=None, surface=None)),
        ok("yalnız bordür", inp(design=None, surface=None)),
        ok("sınırsız ve notsuz: verilmeyen sınır yazılmaz", inp(horizontal=None, vertical=None, quality=None, notes=None)),
        ok("yalnız yatay sınır", inp(vertical=None, quality=None)),
        ok("adlar başka öznitelikte değil: Ad", inp(field="Ad")),
        ok("plan yazmaz", inp(), op="plan"),
        ok("doğrulama", inp(), op="validate"),
        ok("CBS projesinde yüzeysiz", inp(surface=None), setup_=asb.setup("gis")),
        ok("Yol projesinde", inp(), setup_=asb.setup("road")),
        ok("projenin epoku ve kot sistemi yazılır", inp(), setup_=with_heights(),
           extra={"epoch": EPOCH, "heights": HEIGHTS_WORDS}),
        {"name": "var olan paketin üzerine overwrite ile yazılır",
         "steps": [once("$tmp:saha-iki.zip"), {"op": "execute", "input": again,
                          "result": {"status": "completed", "output": answer(again), "warnings": []},
                          "expect": SAME, "within": W}]},
        refused("var olan pakete overwrite'sız yazılmaz", inp(path="$tmp:saha-uc.zip"), "file_exists", ANY, "path",
                steps_before=[once("$tmp:saha-uc.zip")]),
        refused("tasarım yok", inp(design=None, lines=None), bad,
                "Pakete girecek tasarım yok: tasarım noktalarının ya da çizgilerinin katmanını verin.", "design"),
        refused("sıfır yatay sınır", inp(horizontal=0.0), bad,
                "Yatay kabul sınırı 0'dan büyük bir sayı olmalı (m); verilmeyecekse boş bırakın.", "horizontal"),
        refused("eksi düşey sınır", inp(vertical=-0.01), bad,
                "Düşey kabul sınırı 0'dan büyük bir sayı olmalı (m); verilmeyecekse boş bırakın.", "vertical"),
        refused("sıfır kalite sınırı", inp(quality=0.0), bad,
                "Ölçü kalitesi sınırı 0'dan büyük bir sayı olmalı (m); verilmeyecekse boş bırakın.", "quality"),
        refused("boş ad özniteliği", inp(field=" "), bad, "Adların özniteliği boş; özniteliğin adını yazın (yoksa Ad).",
                "field"),
        refused("21 not", inp(notes=[f"not {k}" for k in range(21)]), bad, "Pakette en çok 20 kalite notu olur.", "notes"),
        refused("501 harflik not", inp(notes=["kısa", "ş" * 501]), bad, "Bir kalite notu en çok 500 harf olur.",
                "notes[1]"),
        refused("tarih olmayan damga", inp(stamp="7 Kasım"), bad,
                "“7 Kasım” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.", "stamp"),
        refused("olmayan nokta katmanı", inp(design="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "design"),
        refused("olmayan çizgi katmanı", inp(lines="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "lines"),
        refused("olmayan yüzey", inp(surface="Kazı"), "unknown_terrain", "“Kazı” kimlikli ya da adlı yüzey projede yok.",
                "surface"),
        refused("kurulamayan yüzey", inp(surface="Boş"), "terrain_unbuildable", ANY, None),
        refused("CBS projesinde yüzey", inp(), NOT_MAP[0], NOT_MAP[1], None, setup_=asb.setup("gis")),
        refused("göreli yol", inp(path="saha.zip"), "invalid_file_path",
                "Paketin yolu boş ya da göreli; mutlak bir .zip yolu verin.", "path"),
        refused("zip olmayan yol", inp(path="$tmp:saha.csv"), "invalid_file_path", ANY, "path"),
        refused("olmayan klasör", inp(path="$tmp:yok/saha.zip"), "invalid_file_path", ANY, "path"),
    ]
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.field.package", "commandVersion": 1,
           "title": "Saha paketi",
           "note": "ADR 1920. Bağımsız başvuru scripts/fixtures/field_package_cases.py: Uygulama kontrolünün çizimi, tasarım "
                   "özeti onun kuralıyla (SHA-256), noktalar ve sınırlar elle; KentOS kodu yok. Dosyanın baytları komutun.",
           "setup": asb.setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Saha paketi ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
