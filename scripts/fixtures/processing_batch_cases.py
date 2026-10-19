#!/usr/bin/env python3
"""Shared cases of cad.processing.batch (docs/adr/0529), without KentOS code.

A batch is the same run `cad.processing.run` makes, once for each row of a table of parameter values, the rows in
order, each seeing what the rows before it wrote, the whole one undo step written all or not at all. What a row
computes is not worked out here a second time: each row's expected answer is İşlemler's own shared case for that
run (fixtures/processing/v1/cases.json, queries.json, each checked against its independent reference), restated in
the command's words by processing_command_cases.py. What the batch adds is its own rules, written out from the ADR:
the rows' order and the effect of one on the next (the second row's “P-” || Parsel reads the first row's “P-1”),
the batch's values with the row's own over them, the row labels, the one undo step named “Toplu: <araç>”, the
refusals in order with their paths taken into the row, the all-or-nothing rollback when a row stops at run time,
and the batch's record (rows as steps, the counts the shared cases say).

  python3 scripts/fixtures/processing_batch_cases.py          # writes fixtures/commands/v1/cad.processing.batch.json
  python3 scripts/fixtures/processing_batch_cases.py --check  # fails when it differs
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import processing_command_cases as pcc  # noqa: E402

PROCESSING = pcc.PROCESSING
UNCHANGED = pcc.UNCHANGED
uid = pcc.uid
failed = pcc.failed

CASES = {c["id"]: c for c in json.loads((PROCESSING / "cases.json").read_text(encoding="utf-8"))["cases"]}


def row_answer(case_id: str, *, rename: dict | None = None) -> dict:
    """A row's answer: the run case's, its revision `$any` (inside the batch's step the revision stays) and its own
    record `$any` (the record's content is the run cases' to check)."""
    out = copy.deepcopy(pcc.run_case(CASES[case_id], "parcels.kcad")["steps"][0]["result"]["output"])
    out["revision"] = "$any"
    out["record"] = "$any"
    return out


def case_fields(case_id: str) -> dict[str, dict]:
    return copy.deepcopy(pcc.run_case(CASES[case_id], "parcels.kcad")["steps"][0].get("expect", {}).get("fields", {}))


def merged(*parts: dict[str, dict]) -> dict[str, dict]:
    """The objects' fields after the rows in order: each row's attributes over the earlier ones, its label last."""
    out: dict[str, dict] = {}
    for part in parts:
        for k, f in part.items():
            cur = out.setdefault(k, {})
            if "attrs" in f:
                cur["attrs"] = {**cur.get("attrs", {}), **f["attrs"]}
            for other, v in f.items():
                if other != "attrs":
                    cur[other] = v
    return out


def values_of(case_id: str) -> dict:
    return {k: pcc.restated_value(v) for k, v in (CASES[case_id].get("values") or {}).items()}


def rows_out(answers: list[dict], labels: list[str]) -> list[dict]:
    return [{"index": i, "label": labels[i], "ran": a} for i, a in enumerate(answers)]


def summary_of(answers: list[dict]) -> str:
    added = sum(len(a.get("added", [])) for a in answers)
    changed = sum(len(a.get("changed", [])) for a in answers)
    return f"{len(answers)} satır çalıştı: {added} nesne eklendi, {changed} nesne değişti."


def batch_record(tool: str, label: str, rows: list[tuple[str, int, int, str]], *, total: tuple[int, int], selection=(),
                 inputs=(), definition=False, parameters=None, view=None) -> dict:
    steps = [pcc.step_of(f"rows[{i}]", tool, lab, n_in, n_out, text) for i, (lab, n_in, n_out, text) in enumerate(rows)]
    return pcc.record("batch", tool, label, steps=steps, total=total, selection=selection, inputs=inputs,
                      definition=definition, parameters={} if parameters is None else parameters, view=view)


PARCEL = [{"layer": "parsel", "digest": "$any"}]
ALL_PARCELS = [uid(1), uid(2), uid(3)]


def ok_case(name: str, *, input: dict, answers: list[dict], labels: list[str], step: str | None, record: dict | str,
            fields: dict | None = None, undo: bool = True, setup: dict | None = None) -> dict:
    out = {"tool": input["tool"], "summary": summary_of(answers), "rows": rows_out(answers, labels)}
    if step:
        out["step"] = step
    out["record"] = record
    out["revision"] = "$current"
    expect: dict = {"canUndo": bool(step), "revision": "changed" if step else "same"}
    if fields:
        expect["fields"] = fields
    steps = [{"op": "execute", "input": input,
              "result": {"status": "completed", "output": out, "warnings": []}, "expect": expect}]
    if step and undo:
        steps.append({"op": "undo", "returns": step, "expect": {"canUndo": False, "canRedo": True, "revision": "changed"}})
    case = {"name": name, "steps": steps}
    if setup is not None:
        case["setup"] = setup
    return case


def refusal(name: str, input: dict, code: str, message: str, path: str, setup: dict | None = None) -> dict:
    case = {"name": name, "steps": [{"op": "execute", "input": input, "result": failed(code, message, path), "expect": UNCHANGED}]}
    if setup is not None:
        case["setup"] = setup
    return case


def build() -> list[dict]:
    out: list[dict] = []
    area, geom, label = "calculate-area", "calculate-geometry-values", "calculate-label"
    tool = "attributes.calculate"
    calc_label = "Öznitelik hesapla"
    step_name = f"Toplu: {calc_label}"

    # 1. Two rows over the same objects, each its own field: one undo step for both.
    a1, a2 = row_answer(area), row_answer(geom)
    rows = [("Satır 1", 3, 3, a1["summary"]), ("Satır 2", 3, 3, a2["summary"])]
    inp = {"tool": tool, "selection": ALL_PARCELS, "rows": [{}, {"parameters": values_of(geom)}]}
    fields = merged(case_fields(area), case_fields(geom))
    out.append(ok_case(
        "İki satır: Hesap alanı ve Boyut (aynı nesneler, ayrı alanlar), tek geri alma adımı (“Toplu: …”)",
        input=inp, answers=[a1, a2], labels=["Satır 1", "Satır 2"], step=step_name,
        record=batch_record(tool, calc_label, rows, total=(6, 3), selection=ALL_PARCELS, inputs=PARCEL), fields=fields))

    # 2. The same batch planned: the same answer on a copy, the drawing unchanged.
    planned = json.loads(json.dumps(out[0]["steps"][0]))
    planned["op"] = "plan"
    planned["expect"] = {"canUndo": False, "dirty": False, "revision": "same", "ids": list(range(1, 11))}
    out.append({"name": "Aynı toplu iş, plan: aynı sonuç bir kopyada, çizim değişmez", "steps": [planned]})

    # 3. And validated: the values fit, nothing is written.
    out.append({"name": "Aynı toplu iş, validate: bütün satırların değerleri uyuyor, yazılmaz", "steps": [{
        "op": "validate", "input": inp, "result": {"status": "completed", "output": None, "warnings": []}, "expect": UNCHANGED}]})

    # 4. A row sees what the one before it wrote: “'P-' || Parsel” three times gives “P-P-P-1”; labels: given, trimmed, blank.
    l1 = row_answer(label)
    # The shared case leaves the tool's outputs to the tool's own cases; they are the same tool's as
    # calculate-trimmed-name's: how many it wrote and which.
    l1["outputs"] = {"count": 2, "changed": [uid(1), uid(2)]}
    lab_vals = values_of(label)
    # Three applications of “'P-' || Parsel” to “1”: “P-P-P-1”; the shared case gives the first (“P-1”).
    thrice = {
        k: {"attrs": {**f["attrs"], "Parsel": "P-P-" + f["attrs"]["Parsel"]}, "label": "P-P-" + f["label"]}
        for k, f in case_fields(label).items()
    }
    out.append(ok_case(
        "Satır öncekinin yazdığını görür: “'P-' || Parsel” üç kez “P-P-P-1” verir; satır adları: verilen, kırpılan, boş (“Satır N”)",
        input={"tool": tool, "selection": [uid(1), uid(2)], "parameters": lab_vals,
               "rows": [{"label": "  ilk ek  "}, {"label": "   "}, {}]},
        answers=[l1, l1, l1], labels=["ilk ek", "Satır 2", "Satır 3"], step=step_name,
        record=batch_record(tool, calc_label, [("ilk ek", 2, 2, l1["summary"]), ("Satır 2", 2, 2, l1["summary"]),
                                               ("Satır 3", 2, 2, l1["summary"])], total=(6, 2),
                            selection=[uid(1), uid(2)], inputs=PARCEL, parameters=lab_vals),
        fields=thrice))

    # 5. The batch's values and the row's own over them.
    geo = values_of(geom)
    renamed = copy.deepcopy(row_answer(geom))
    renamed["summary"] = renamed["summary"].replace("“Boyut”", "“Boyut2”")
    fields5 = merged(case_fields(geom), {k: {"attrs": {**f["attrs"], "Boyut2": f["attrs"]["Boyut"]}}
                                          for k, f in case_fields(geom).items()})
    out.append(ok_case(
        "Ortak değerler ve satırın kendi değeri: ikinci satır yalnız alanın adını değiştirir, ifade ortaktır",
        input={"tool": tool, "selection": ALL_PARCELS, "parameters": geo, "rows": [{}, {"parameters": {"field": "Boyut2"}}]},
        answers=[row_answer(geom), renamed], labels=["Satır 1", "Satır 2"], step=step_name,
        record=batch_record(tool, calc_label, [("Satır 1", 3, 3, row_answer(geom)["summary"]),
                                               ("Satır 2", 3, 3, renamed["summary"])], total=(6, 3),
                            selection=ALL_PARCELS, inputs=PARCEL, parameters=geo),
        fields=fields5))

    # 6. The same tool on different objects: each row names its own by persistent id.
    one = case_fields(area)
    row_a = copy.deepcopy(row_answer(area))
    row_a["summary"] = row_a["summary"].replace("3 nesnede", "1 nesnede")

    def single(n: int) -> dict:
        r = copy.deepcopy(row_a)
        r["changed"] = [uid(n)]
        r["outputs"] = {"count": 1, "changed": [uid(n)]}
        return r

    out.append(ok_case(
        "Aynı araç farklı nesnelere: her satır kendi nesnesini kalıcı kimliğiyle verir (scope uids)",
        input={"tool": tool, "rows": [
            {"label": "parsel 1", "parameters": {"input": {"scope": "uids", "uids": [uid(1)]}}},
            {"label": "parsel 2", "parameters": {"input": {"scope": "uids", "uids": [uid(2)]}}}]},
        answers=[single(1), single(2)], labels=["parsel 1", "parsel 2"], step=step_name,
        record=batch_record(tool, calc_label, [("parsel 1", 1, 1, row_a["summary"]), ("parsel 2", 1, 1, row_a["summary"])],
                            total=(2, 2), inputs=PARCEL),
        fields={"1": one["1"], "2": one["2"]}))

    # 7. A model as the tool: one row is the model's own shared case.
    model = CASES["model-parcel-sheet"]
    ran_model = copy.deepcopy(pcc.run_case(model, "parcels.kcad"))
    m_out = ran_model["steps"][0]["result"]["output"]
    m_out["revision"] = "$any"
    m_out["record"] = "$any"
    m_rec = pcc.model_record(CASES)
    added = len(m_out["added"])
    m_expect = copy.deepcopy(ran_model["steps"][0]["expect"])
    m_expect.pop("canUndo", None)
    m_expect["revision"] = "changed"
    mname = "Toplu: Parsel ölçü yazıları"
    case7 = ok_case(
        "Model araç olarak: tek satır modelin kendi ortak durumudur (üç adım, kayıtlar, tek geri alma adımı)",
        input={"tool": pcc.PARCEL_MODEL, "selection": [uid(1), uid(2)], "rows": [{}]},
        answers=[m_out], labels=["Satır 1"], step=mname,
        record=batch_record(pcc.PARCEL_MODEL, "Parsel ölçü yazıları", [("Satır 1", m_rec["total"]["recordsIn"],
                                                                    m_rec["total"]["recordsOut"], m_out["summary"])],
                            total=(m_rec["total"]["recordsIn"], m_rec["total"]["recordsOut"]),
                            selection=[uid(1), uid(2)], inputs=PARCEL, definition=True),
        fields=m_expect.get("fields"))
    case7["steps"][0]["expect"] = {**case7["steps"][0]["expect"], "ids": list(range(1, 11)) + list(range(11, 11 + added))}
    out.append(case7)

    # 8. Selecting tools: no edit, so no undo step; the box and the selection are the batch's.
    sel = ["select-visible", "select-new", "select-centroid"]
    answers8 = [row_answer(c) for c in sel]
    # select-visible's shared case leaves the outputs out: the selecting tool's are how many matched and which.
    answers8[0]["outputs"] = {"count": 1, "matched": [uid(2)]}
    rows8 = []
    for c, ans in zip(sel, answers8):
        r = {}
        v = values_of(c)
        if v:
            r["parameters"] = v
        rows8.append(r)
    out.append(ok_case(
        "Seçen araçlar: üç satır, çizim değişmez (geri alma adımı yok), kutu ve seçim toplu işindir",
        input={"tool": "selection.byExpression", "view": CASES["select-visible"]["view"], "rows": rows8},
        answers=answers8, labels=["Satır 1", "Satır 2", "Satır 3"], step=None, record="$any"))

    # 9. A row stops at run time (a field of the layer takes whole numbers only): the earlier row is taken back.
    q = pcc.document("queries.kcad")
    q_attrs = next(e for e in q["entities"] if e["id"] == 1)["attrs"]
    msg = ("Öznitelik yazılamadı (#1): “Ağaç sayısı” alanı tam sayı ister; “çok” verildi. "
           "Rakamlarla, ondalıksız yazın.")
    out.append({
        "name": "Satır çalışırken durur (katmanın tam sayı alanı “çok”u reddeder): önceki satır geri alınır, hiçbir şey yazılmaz",
        "setup": q,
        "steps": [{
            "op": "execute",
            "input": {"tool": tool, "parameters": {"input": {"scope": "layer", "layerId": "parsel"}, "field": "Ağaç sayısı"},
                      "rows": [{"parameters": {"value": "5"}}, {"parameters": {"value": "'çok'"}}]},
            "result": failed("batch_row_failed", f"Satır 2 çalışmadı: {msg} Toplu işin hiçbir sonucu yazılmadı.", "rows[1]"),
            "expect": {"canUndo": False, "dirty": False, "revision": "same", "fields": {"1": {"attrs": q_attrs}}},
        }],
    })

    # 10. The same through a model: its middle step stops, the whole batch is taken back.
    doc = pcc.document("parcels.kcad")
    doc["layers"].append({
        "id": "islem-kenar-olculeri", "name": "Kenar ölçüleri", "type": "layer", "visible": True, "locked": False,
        "expanded": True, "style": {"color": "fg-dim", "lineType": "continuous", "lineWeight": 0.18}, "children": [],
        "fields": [{"name": "Uzunluk (m)", "kind": "integer"}],
    })
    mid = ("2. adım (Kenar uzunluklarını yaz) çalışmadı: Öznitelik yazılamadı (yeni nesne): “Uzunluk (m)” alanı tam sayı "
           "ister; “20.000” verildi. Rakamlarla, ondalıksız yazın. Önceki adımların sonuçları geri alındı.")
    out.append({
        "name": "Model satırının ortadaki adımı durur: modelin geri alması ve toplu işin geri alması, hiçbir şey yazılmaz",
        "setup": doc,
        "steps": [{
            "op": "execute",
            "input": {"tool": pcc.PARCEL_MODEL, "selection": [uid(1), uid(2)], "rows": [{"parameters": {"prefix": "A"}}]},
            "result": failed("batch_row_failed", f"Satır 1 çalışmadı: {mid} Toplu işin hiçbir sonucu yazılmadı.", "rows[0]"),
            "expect": {"canUndo": False, "dirty": False, "revision": "same", "ids": [e["id"] for e in doc["entities"]]},
        }],
    })

    # 11. Refusals before anything runs, in order; a row's path is taken into the row.
    base = {"tool": tool, "selection": [uid(1)]}
    out.append(refusal("Satırı olmayan toplu iş: empty_batch", {**base, "rows": []}, "empty_batch",
                       "Toplu çalıştırmanın satırı yok; en az bir satır verin.", "rows"))
    out.append(refusal("501 satır: too_many_rows (en çok 500)", {**base, "rows": [{} for _ in range(501)]}, "too_many_rows",
                       "501 satır verildi; toplu çalıştırma en çok 500 satır alır. Satırları bölüp ayrı çağırın.", "rows"))
    out.append(refusal("Bilinmeyen araç: unknown_tool, aracın yolu", {"tool": "points.yok", "rows": [{}]}, "unknown_tool",
                       "“points.yok” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.", "tool"))
    out.append(refusal("Bilinmeyen model: unknown_tool", {"tool": "model:builtin.yok", "rows": [{}]}, "unknown_tool",
                       "“model:builtin.yok” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.", "tool"))
    out.append(refusal("Toplu işin seçiminde UUID olmayan kimlik: invalid_uid, yeriyle",
                       {"tool": tool, "selection": [uid(1), "parsel-2"], "rows": [{}]}, "invalid_uid",
                       "“parsel-2” bir nesne kimliği değil (UUID olmalı).", "selection[1]"))
    out.append(refusal("Toplu işin seçiminde çizimde olmayan nesne: entity_not_found",
                       {"tool": tool, "selection": ["00000000-0000-4000-8000-000000000000"], "rows": [{}]},
                       "entity_not_found", "00000000-0000-4000-8000-000000000000 kimlikli nesne çizimde yok.", "selection[0]"))
    expr = pcc.run_case(CASES["refuse-expression"], "parcels.kcad")["steps"][0]["result"]["error"]
    out.append(refusal("İkinci satırın ifadesi yarım: invalid_parameters, satırın yoluyla; ilk satır geçerli olsa da hiçbir şey yazılmaz",
                       {"tool": tool, "selection": [uid(1)], "rows": [{}, {"parameters": {"value": "metin($alan,"}}]},
                       "invalid_parameters", expr["message"], "rows[1].parameters.value"))
    out.append(refusal("Bütün satırların ortak ifadesi yarım: invalid_parameters, ortak yolla",
                       {"tool": tool, "selection": [uid(1)], "parameters": {"value": "metin($alan,"}, "rows": [{}, {}]},
                       "invalid_parameters", expr["message"], "parameters.value"))
    known = ", ".join(p["name"] for p in pcc.INVENTORY_TOOLS[tool]["parameters"])
    out.append(refusal("Satırın olmayan parametresi: invalid_parameters, satırın yoluyla",
                       {"tool": tool, "selection": [uid(1)], "rows": [{}, {"parameters": {"alan": "Hesap"}}]},
                       "invalid_parameters", f"“{calc_label}” aracında “alan” parametresi yok; parametreleri: {known}.",
                       "rows[1].parameters.alan"))
    out.append(refusal("Görünen alandakiler, toplu işte view yokken: invalid_parameters",
                       {"tool": "selection.byExpression", "rows": [{"parameters": {"input": {"scope": "visible"}}}]},
                       "invalid_parameters", "“input”: görünen alandakiler için girdide view (kutu) verilmeli.",
                       "rows[0].parameters.input"))
    out.append(refusal("Satırın nesne girdisinde çizimde olmayan kimlik: entity_not_found, satırın yoluyla",
                       {"tool": tool, "rows": [{"parameters": {"input": {"scope": "uids",
                                                                           "uids": ["00000000-0000-4000-8000-000000000001"]}}}]},
                       "entity_not_found", "00000000-0000-4000-8000-000000000001 kimlikli nesne çizimde yok.",
                       "rows[0].parameters.input.uids[0]"))
    return out


def main() -> int:
    check = "--check" in sys.argv
    cases = build()
    data = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.processing.batch",
        "commandVersion": 1,
        "title": "İşlem aracını toplu çalıştır",
        "note": "Üretici: scripts/fixtures/processing_batch_cases.py (docs/adr/0529). Satırların sonuçları İşlemler'in kendi ortak durumlarından (fixtures/processing/v1/cases.json), komutun sözleriyle; toplu işin kuralları ADR'den.",
        "setup": pcc.document("parcels.kcad"),
        "cases": cases,
    }
    ok = pcc.write("cad.processing.batch.json", data, check)
    if ok:
        print(f"cad.processing.batch durumları tutarlı: {len(cases)} durum")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
