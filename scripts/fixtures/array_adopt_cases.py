#!/usr/bin/env python3
"""Reviewed array adoption: independent protocol/metadata expectations, no KentOS imports."""
import argparse
from copy import deepcopy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BLOCK = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001"


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--check", action="store_true")
    args = cli.parse_args()
    setup = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
    setup["settings"]["drawingUnit"] = "mm"
    setup["blocks"] = [dict(id=BLOCK, name="Teknik parça", base=dict(x=0, y=0), units="m",
        entities=[dict(kind="line", id=1, layerId="a", attrs={}, a=dict(x=0, y=0), b=dict(x=1, y=0))])]
    setup["entities"] = [dict(kind="insert", id=k+1, layerId="a", attrs={}, block=BLOCK,
        p=dict(x=2*k+(.5 if k == 1 else 0), y=.25 if k == 1 else 0), scale=1, rotation=0)
        for k in range(8) if k not in [2, 6]]
    setup["entities"].append(dict(kind="line", id=9, layerId="a", attrs={}, a=dict(x=0,y=0), b=dict(x=20,y=0)))
    ids = [1, 2, 4, 5, 6, 8, 9]
    uid = lambda i: f"$uidOf:{i}"
    layout = dict(kind="line", count=8, dx=2, dy=0, fit=False)
    items = [dict(uid=uid(k+1), **(dict(kept=True) if k == 1 else {})) if k not in [2,6] else {} for k in range(8)]
    baseline = dict(source=uid(1), layout=layout, items=items)
    array = dict(block=BLOCK, layerId="a", p=dict(x=0, y=0), scale=1, rotation=0, layout={k: v for k, v in layout.items() if k != "fit"}, items=items)
    cases = []

    def expect(writes=False):
        return dict(ids=ids, canUndo=writes, dirty=writes, revision="changed" if writes else "same",
                    arrays=[array] if writes else [])

    for op in ["validate", "plan", "execute"]:
        cases.append(dict(name=f"{op}: iki eksik yer, açıkça korunan öğe ve değişmeyen kök kimlikler", steps=[dict(
            op=op, input=deepcopy(baseline), result=dict(status="completed", output="$any", warnings=[]),
            expect=expect(op == "execute"))]))

    def refuse(name, patch, code, path, status="failed", revision=False, raw=None):
        error = dict(code=code, message="$any", path=path)
        if revision:
            error["revision"] = "$any"
        case = dict(name=name, steps=[dict(op="execute", input=dict(deepcopy(baseline), **patch),
            result=dict(status=status, error=error), expect=expect())])
        if raw is not None:
            case["setup"] = raw
        cases.append(case)

    refuse("bozuk kaynak kimliği", dict(source="bad"), "invalid_uid", "source")
    refuse("olmayan kaynak", dict(source="01925f3e-7c1a-7d2b-9e4f-000000000999"), "entity_not_found", "source")
    refuse("kaynak blok değil", dict(source=uid(9)), "array_adoption_refused", "source")
    refuse("eksik yer kayıtları atlanamaz", dict(items=items[:-1]), "array_adoption_refused", "items")
    bad = deepcopy(items); bad[0]["kept"] = True
    refuse("kaynak korunmuş olamaz", dict(items=bad), "array_adoption_refused", "items[0]")
    bad = deepcopy(items); bad[2] = dict(kept=True)
    refuse("eksik yer korunmuş olamaz", dict(items=bad), "array_adoption_refused", "items[2]")
    bad = deepcopy(items); bad[7] = dict(uid=uid(1))
    refuse("bir UUID iki yeri alamaz", dict(items=bad), "array_adoption_refused", "items[7]")
    bad = deepcopy(items); bad[1].pop("kept")
    refuse("elle taşınmış nesne kendiliğinden düzeltilmez", dict(items=bad), "array_adoption_refused", "items[1]")
    bad = deepcopy(items)
    for k in [3, 4, 5, 7]:
        bad[k]["kept"] = True
    refuse("yalnız bir normal öğe örüntü sayılmaz", dict(items=bad), "array_adoption_refused", "items")
    for tolerance in [0, -1]:
        refuse(f"geçersiz tolerans {tolerance}", dict(tolerance=tolerance), "invalid_tolerance", "tolerance")
    for value in ["NaN", "Infinity", "-Infinity"]:
        refuse(f"sonlu olmayan aralık {value}", {}, "not_finite", "layout.dx")
        cases[-1]["steps"][0]["nonFinite"] = {"layout.dx": value}
    for last in [{}, dict(uid=uid(5), kept=True)]:
        refuse("eksik veya korunan yerde hesap taşması gizlenemez", dict(
            layout=dict(kind="line", count=3, dx=1e308, dy=0, fit=False), tolerance=1e308,
            items=[dict(uid=uid(1)), dict(uid=uid(4)), last]), "array_adoption_refused", "layout")
    refuse("geçersiz adet", dict(layout=dict(layout, count=1)), "invalid_count", "layout.count")
    refuse("sıfır aralık", dict(layout=dict(layout, dx=0)), "invalid_spacing", "layout")
    refuse("geçersiz kutupsal açı", dict(layout=dict(kind="polar", center=dict(x=0,y=0), count=8, fill=0, rotate=True)), "invalid_fill", "layout.fill")
    refuse("geçersiz yol kimliği", dict(layout=dict(kind="path", path="bad", count=8, align=False)), "invalid_uid", "layout.path")
    refuse("olmayan yol", dict(layout=dict(kind="path", path="01925f3e-7c1a-7d2b-9e4f-000000000999", count=8, align=False)), "entity_not_found", "layout.path")
    refuse("blok yol olamaz", dict(layout=dict(kind="path", path=uid(1), count=8, align=False)), "invalid_path", "layout.path")
    refuse("bozuk revizyon", dict(expectedRevision="bad"), "invalid_revision", "expectedRevision")
    refuse("eski revizyon", dict(expectedRevision="999"), "revision_conflict", "expectedRevision", "conflict", True)
    locked = deepcopy(setup); locked["layers"][0]["locked"] = True
    refuse("kilitli katman", {}, "layer_locked", "source", raw=locked)
    attrs = deepcopy(setup); attrs["entities"][0]["attrs"]["CODE"] = "A"
    refuse("kaynak verisi atılamaz", {}, "array_adoption_refused", "source", raw=attrs)
    # One real Undo removes only metadata; Redo restores precisely the reviewed slot map.
    cases.append(dict(name="tek geri alma ve yineleme", steps=[
        dict(op="execute", input=baseline, result=dict(status="completed", output="$any", warnings=[]), expect=expect(True)),
        dict(op="undo", returns="Mevcut blokları diziye bağla", expect=dict(ids=ids,canUndo=False,arrays=[])),
        dict(op="redo", returns="Mevcut blokları diziye bağla", expect=dict(ids=ids,canUndo=True,arrays=[array])),
    ]))
    result = dict(format="kentos.command-cases", version=1, command="cad.arrays.adopt", commandVersion=1,
        title="Mevcut blokları diziye bağla", note="ADR0873. Independent script; topology/dimensions and exact stale-job/Undo snapshots: arrays_adopt.rs.",
        setup=setup, cases=cases)
    target = ROOT / "fixtures/commands/v1/cad.arrays.adopt.json"
    text = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if target.read_text() != text:
            raise SystemExit("cad.arrays.adopt cases out of date")
        print(f"cad.arrays.adopt: {len(cases)} independent cases are current")
    else:
        target.write_text(text)


if __name__ == "__main__":
    main()
