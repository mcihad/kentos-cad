"""Compare remaining rustfmt changes against the exact accepted base, without edits."""
from pathlib import Path
import difflib
import json
import subprocess

BASE = 'ea7d1baeeb2e949062eb3aec6d900d4b697c7983'
ROOT = Path(__file__).resolve().parents[3]
FILES = ['crates/shared/geometry-core/src/ops/subref.rs', 'crates/shared/geometry-core/tests/all/subref.rs', 'crates/shared/geometry-core/src/store/snap.rs', 'crates/shared/geometry-core/tests/all/snap.rs', 'crates/native/application/src/blocks.rs', 'crates/native/application/tests/all/opening_placement.rs', 'crates/shared/contracts/src/subref.rs', 'crates/shared/commands/src/catalog.rs', 'crates/native/interaction/tests/all/dimension.rs', 'crates/native/mcp/tests/opening_placement.rs']


def remaining(code):
    run = subprocess.run(
        ['rustfmt', '--emit', 'stdout', '--edition', '2024', '--config', 'skip_children=true'],
        input=code, text=True, capture_output=True, check=True,
    )
    before, after = code.splitlines(True), run.stdout.splitlines(True)
    return [(before[i:j], after[k:l]) for op, i, j, k, l in
            difflib.SequenceMatcher(None, before, after, autojunk=False).get_opcodes()
            if op != 'equal']


rows = []
for name in FILES:
    old = subprocess.check_output(['git', 'show', BASE + ':' + name], cwd=ROOT, text=True)
    before, after = remaining(old), remaining((ROOT / name).read_text())
    assert before == after, name
    rows.append(dict(file=name, remainingFormattingChanges=len(after), identicalToBase=True))
report = dict(base=BASE, files=rows, newFormattingChanges=0, workspaceFormattingClean=False)
(Path(__file__).parent / 'format-baseline.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
