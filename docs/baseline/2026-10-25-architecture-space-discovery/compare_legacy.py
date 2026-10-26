#!/usr/bin/env python3
"""Optional SDK replay of the independently captured pre-change 400-seed plan."""
import json
import hashlib
import datetime
import kentos._native as native
from pathlib import Path
from kentos import cad

base = Path(__file__).resolve().parent
doc = cad.Document.open(base / 'grid-before.kcad')
plan = cad.spaces.create.plan(
    doc, layer_id='cizim',
    spaces=[cad.NewSpace(seed=cad.Vec2(x=500002 + c*4, y=4400002 + r*4))
            for r in range(20) for c in range(20)],
)
capture = base / 'legacy-grid-plan-full.json'
assert plan.to_json() == json.loads(capture.read_text())
report = dict(recordedAt=datetime.datetime.now(datetime.timezone.utc).isoformat(),
              nativeSha256=hashlib.sha256(Path(native.__file__).read_bytes()).hexdigest(),
              captureFileSha256=hashlib.sha256(capture.read_bytes()).hexdigest(),
              sourceCount=39, spaceCount=400, exactFullPlanEqual=True,
              note='Full parsed plan equality against the pre-change SDK capture; no timing claim.')
(base / 'legacy-plan-equality-delivery.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: the complete 400-seed plan exactly matches the pre-change native SDK capture.')
