#!/usr/bin/env python3
"""Independent selected-repeat conversion expectations; no KentOS imports."""
from pathlib import Path
import argparse
import json
from copy import deepcopy

ROOT = Path(__file__).resolve().parents[2]


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--check", action="store_true")
    args = cli.parse_args()
    setup = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
    uid = lambda i: f"$uidOf:{i}"
    baseline = dict(name="Tekrar", source=[uid(1)], groups=[[uid(1)], [uid(2)]])
    cases = []

    def expect(writes=False):
        return dict(ids=list(range(1, 9)), canUndo=writes, dirty=writes,
                    revision="changed" if writes else "same")

    def success(name, input_, op="execute"):
        cases.append(dict(name=name, steps=[dict(op=op, input=input_,
            result=dict(status="completed", output="$any", warnings=[]), expect=expect(op=="execute"))]))

    def refusal(name, patch, code, path, status="failed", revision=False):
        input_ = dict(baseline, **patch)
        error = dict(code=code, message="$any", path=path)
        if revision:
            error["revision"] = "$any"
        cases.append(dict(name=name, steps=[dict(op="execute", input=input_,
            result=dict(status=status, error=error), expect=expect())]))

    for op in ["plan", "execute"]:
        success(f"{op}: iki seçilmiş eş; kök UUID ve yerel slotlar korunur", deepcopy(baseline), op)
        success(f"{op}: kaynak dönüştürülmeden yalnız hedef seçilebilir",
                dict(baseline, groups=[[uid(2)]]), op)
        success(f"{op}: kapalı çoklu çizginin ters köşe sırası",
                dict(name="Kapalı", source=[uid(5)], groups=[[uid(5)], [uid(6)]], rotations=False), op)
    success("açık dönüşüm ölçütleri", dict(baseline, rotations=False, mirrors=False, scales=False,
                                           tolerance=0.000001))
    refusal("boş ad", dict(name=" "), "empty_name", "name")
    refusal("seçim yok", dict(groups=[]), "recognition_conversion_refused", "groups")
    refusal("kaynak yok", dict(source=[]), "recognition_conversion_refused", "groups")
    refusal("kaynak sınırı", dict(source=[uid(1)]*129), "recognition_conversion_refused", "groups")
    refusal("aynı nesneyi paylaşan adaylar", dict(groups=[[uid(1)], [uid(1)]]),
            "recognition_conversion_refused", "groups[1]")
    refusal("eksik aday", dict(groups=[[]]), "recognition_conversion_refused", "groups[0]")
    refusal("kaynak sayısından fazla üye", dict(groups=[[uid(1), uid(2)]]),
            "recognition_conversion_refused", "groups[0]")
    refusal("aynı kaynak iki kez", dict(source=[uid(1), uid(1)], groups=[[uid(1), uid(2)]]),
            "recognition_conversion_refused", "source")
    refusal("bozuk kaynak UUID", dict(source=["bad"], groups=[[uid(1)]]), "invalid_uid", "source[0]")
    refusal("bozuk aday UUID", dict(groups=[["bad"]]), "invalid_uid", "uids[0]")
    refusal("olmayan kaynak", dict(source=["01925f3e-7c1a-7d2b-9e4f-000000000999"]),
            "entity_not_found", "source[0]")
    for value in [0, -1, 1]:
        refusal(f"geçersiz tolerans {value}", dict(tolerance=value), "invalid_tolerance", "tolerance")
    refusal("desteklenmeyen öğe atlanmaz", dict(source=[uid(4)], groups=[[uid(4)]]),
            "recognition_unsupported", "source[0]")
    refusal("Z verisi düzleştirilmez", dict(source=[uid(8)], groups=[[uid(8)]]),
            "recognition_unsupported", "source[0]")
    refusal("görünüş/veri farkı arama sonucu olsa da kaybedilmez", dict(groups=[[uid(3)]]),
            "recognition_conversion_refused", "groups[0]")
    refusal("kapalı ve açık çizgi birleştirilmez", dict(source=[uid(5)], groups=[[uid(7)]]),
            "recognition_conversion_refused", "groups[0]")
    refusal("bozuk revizyon", dict(expectedRevision="bad"), "invalid_revision", "expectedRevision")
    refusal("eski revizyon", dict(expectedRevision="999"), "revision_conflict", "expectedRevision",
            status="conflict", revision=True)
    cases.append(dict(name="kilitli katman değişmez", steps=[
        dict(op="execute", command="cad.layers.set", input=dict(operation="locked", layer="a", value=True),
             result=dict(status="completed", output="$any", warnings=[])),
        dict(op="execute", input=baseline, result=dict(status="failed",error=dict(code="layer_locked",message="$any",path="layerId")),
             expect=dict(ids=list(range(1,9)),canUndo=False,dirty=True,revision="same"))]))
    # Convert one root, then use another untouched source to reach duplicate-name validation.
    cases.append(dict(name="var olan tanımın üzerine yazılmaz", steps=[
        dict(op="execute",input=dict(baseline,groups=[[uid(1)]]),result=dict(status="completed",output="$any",warnings=[])),
        dict(op="execute",input=dict(baseline,source=[uid(2)],groups=[[uid(2)]]),
             result=dict(status="failed",error=dict(code="duplicate_block",message="$any",path="name")),
             expect=dict(ids=list(range(1,9)),canUndo=True,dirty=True,revision="same"))]))
    cases.append(dict(name="tek geri alma ve yineleme", steps=[
        dict(op="execute",input=baseline,result=dict(status="completed",output="$any",warnings=[]),expect=expect(True)),
        dict(op="undo",returns="Tekrarları bloğa dönüştür",expect=dict(ids=list(range(1,9)),canUndo=False)),
        dict(op="redo",returns="Tekrarları bloğa dönüştür",expect=dict(ids=list(range(1,9)),canUndo=True))]))
    result = dict(format="kentos.command-cases",version=1,command="cad.blocks.convert",commandVersion=1,
                  title="Tekrarları bloğa dönüştür",note="Independent script; exact topology/relations and snapshot equality are tested in blocks_convert.rs.",setup=setup,cases=cases)
    target=ROOT/"fixtures/commands/v1/cad.blocks.convert.json"
    text=json.dumps(result,ensure_ascii=False,indent=2)+"\n"
    if args.check:
        if target.read_text()!=text:raise SystemExit("cad.blocks.convert cases out of date")
        print(f"cad.blocks.convert: {len(cases)} independent cases are current")
    else:target.write_text(text)


if __name__ == "__main__":
    main()
