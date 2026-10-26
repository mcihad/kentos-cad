"""Compare remaining rustfmt changes against the exact accepted base, without edits."""
from pathlib import Path
import difflib
import json
import subprocess

BASE = '5d217ca4da11157bf345cdb792e3171b1f655783'
ROOT = Path(__file__).resolve().parents[3]
FILES = [
    'crates/shared/commands/src/cad_tags.rs',
    'crates/shared/commands/src/catalog.rs',
    'crates/shared/commands/src/capabilities.rs',
    'crates/native/application/src/tags.rs',
    'crates/native/interaction/src/tag_tools.rs',
    'crates/native/interaction/tests/all/tag_settings.rs',
    'crates/native/mcp/tests/tag_refresh.rs',
]


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
