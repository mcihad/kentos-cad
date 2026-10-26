"""Public settings call on a dense drawing; no frame/FPS claim."""
import copy
from dataclasses import replace
import hashlib
import importlib.util
import json
from pathlib import Path
from statistics import median
from time import perf_counter_ns

from kentos import cad

B = Path(__file__).resolve().parent
R = B.parents[2]
spec = importlib.util.spec_from_file_location("reader", R / "tools/kcad/kcad.py")
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)
setup = json.loads((R / "fixtures/commands/v1/cad.tags.check.json").read_text())["setup"]
setup["entities"] = []
for i in range(10000):
    x = 5 * i
    setup["entities"].append(dict(
        kind="space", id=i + 1, layerId="mahal", attrs={}, name=f"Özel oda {i + 1}",
        usage="Serbest kullanım", number=f"N-{i:030}", arch=dict(level="zemin"),
        parts=[dict(outer=dict(pts=[dict(x=x, y=0), dict(x=x+4, y=0),
                                   dict(x=x+4, y=3), dict(x=x, y=3)]))],
    ))
for i in range(20000):
    setup["entities"].append(dict(
        kind="line", id=10001 + i, layerId="cizim", attrs={},
        a=dict(x=500000 + i, y=4400000), b=dict(x=500001 + i, y=4400000),
    ))
doc = cad.Document.from_bytes(json.dumps(setup).encode())
old = doc.info().settings.arch
choices = replace(old, space_names=[f"{i:03} " + "ç" * 252 for i in range(256)],
                  space_usages=[f"{i:03} " + "ğ" * 252 for i in range(256)],
                  space_number_prefixes={"zemin": "G-", "kat-1": "01-"})
before = doc.to_bytes()
expected = reader.read(before)[1]
original_entities = hashlib.sha256(json.dumps(
    [expected["entities"], expected["uids"]], sort_keys=True, ensure_ascii=False,
).encode()).hexdigest()
plan_times, write_times = [], []
for _ in range(3):
    start = perf_counter_ns()
    cad.settings.set.plan(doc, arch=choices)
    plan_times.append((perf_counter_ns() - start) / 1e6)
    assert doc.to_bytes() == before
    start = perf_counter_ns()
    cad.settings.set(doc, arch=choices)
    write_times.append((perf_counter_ns() - start) / 1e6)
    changed = copy.deepcopy(expected)
    changed["settings"]["arch"] = choices.to_json()
    assert reader.read(doc.to_bytes())[1] == changed
    cad.settings.set(doc, arch=old)
    assert doc.to_bytes() == before
record = dict(profile="debug, public Python SDK; GUI and Cargo jobs stopped",
              rooms=10000, unrelatedLines=20000, names=256, usages=256,
              charactersPerValue=256, prefixes=2,
              planMs=plan_times, writeMs=write_times,
              planMedianMs=median(plan_times), writeMedianMs=median(write_times),
              preservedEntitiesAndUidsSha256=original_entities,
              invariants="All six calls: complete independent model expectations; clearing preferences restores exact old KCAD bytes.",
              limitation="Local settings CPU smoke test; not release/p95/memory/GPU/viewport/FPS acceptance.")
(B / "timing.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(record, indent=2))
