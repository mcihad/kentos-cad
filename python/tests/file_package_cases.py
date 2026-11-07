"""Paketle's resource report (docs/adr/1111) through the typed Python layer: fixtures/commands/v1/file.package.resources.json,
written by scripts/fixtures/package_resource_cases.py from the ADR's rules without KentOS code, as the headless host's
crates/native/headless/tests/headless/package_resources.rs plays it. Each case's drawing is written as a project file
in its work folder by the independent KCAD v2 writer (scripts/fixtures/kcad_v2_reference.py), opened and its package
planned; the answer's `resources` must be the reference's row for row and its warnings the reference's (the resources'
warning word for word). A plan writes no package and leaves the drawing as it was."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path
from typing import Any

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import kcad_v2_reference as kr  # noqa: E402


def placed(value: Any) -> Any:
    """`$fixture:` paths made the fixtures folder's, anywhere in a value."""
    if isinstance(value, str) and value.startswith("$fixture:"):
        return str(ROOT / "fixtures" / value[len("$fixture:"):])
    if isinstance(value, list):
        return [placed(v) for v in value]
    if isinstance(value, dict):
        return {k: placed(v) for k, v in value.items()}
    return value


def told(warnings: list[Any]) -> list[dict[str, str]]:
    """The warnings as the case says them: each code, and the resources' warning's words."""
    return [
        {"code": w.code, "message": w.message} if w.code == "package_resources" else {"code": w.code}
        for w in warnings
    ]


def play(case: dict[str, Any], work: Path) -> None:
    project = work / "proje.kcad"
    project.write_bytes(kr.container(kr.document(placed(case["setup"]))))
    doc = cad.Document.open(project)
    held = doc.info()
    target = work / "paket"
    wrapper = cad.command("file.package")
    inp = wrapper.input_type.from_json({"path": str(target), "from": str(work)})
    result = wrapper.run(doc, inp, op=case["op"])
    want = case["expect"]
    assert result.status == want["status"], (result.status, result.error)
    got = result.output.to_json().get("resources", []) if result.output is not None else []
    assert got == want["resources"], f"\n  want {want['resources']}\n  got  {got}"
    assert told(result.warnings) == want["warnings"], f"warnings\n  want {want['warnings']}\n  got  {told(result.warnings)}"
    assert not target.exists(), "the plan wrote the package"
    after = doc.info()
    assert (after.revision, after.dirty, after.can_undo) == (held.revision, held.dirty, held.can_undo), "the plan wrote"


def run_fixture(fx: dict[str, Any]) -> int:
    assert fx["command"] == "file.package" and len(fx["cases"]) >= 20
    for case in fx["cases"]:
        try:
            with tempfile.TemporaryDirectory(prefix="kentos-package-python-") as name:
                play(case, Path(name))
        except Exception as e:
            raise AssertionError(f"{fx['command']} › {case['name']}: {type(e).__name__}: {e}") from e
    return len(fx["cases"])
