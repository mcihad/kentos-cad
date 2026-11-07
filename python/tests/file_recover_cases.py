"""Kurtar (docs/adr/1013) through the typed Python layer: fixtures/commands/v1/file.recover.json, written by
scripts/fixtures/recover_cases.py from damages made byte by byte to a valid drawing (no KentOS code), as the headless
host's crates/native/headless/tests/headless/recover_cases.rs plays it. Each case's file is copied to its own work
folder as `cizim.kcad`; the answer must be the reference's, a written copy must open in the independent reader
(tools/kcad/kcad.py) with the reference's object ids, an unwritten one must not be there, and the source must be
byte for byte as it was."""

from __future__ import annotations

import importlib.util
import shutil
import tempfile
from pathlib import Path
from typing import Any

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
_SPEC = importlib.util.spec_from_file_location("recover_case_kcad", ROOT / "tools/kcad/kcad.py")
assert _SPEC is not None and _SPEC.loader is not None
_KCAD = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_KCAD)


def placed(value: Any, work: Path) -> Any:
    """`$work:` paths made the case's work folder's."""
    if isinstance(value, str) and value.startswith("$work:"):
        return str(work / value[len("$work:"):])
    return value


def play(case: dict[str, Any], doc: cad.Document, work: Path) -> None:
    source = work / "cizim.kcad"
    shutil.copyfile(ROOT / "fixtures/kcad/recover/v1" / case["file"], source)
    before = source.read_bytes()
    for made in case["make"]:
        (work / made).write_bytes(b"var olan")
    inp = {"path": str(source)} | {k: placed(v, work) for k, v in case["input"].items()}
    wrapper = cad.command("file.recover")
    result = wrapper.run(doc, wrapper.input_type.from_json(inp), op=case["op"])
    want = case["expect"]
    assert source.read_bytes() == before, "the source changed"
    if want.get("status") == "failed":
        assert result.error is not None, f"not refused: {result.status}"
        assert (result.error.code, result.error.path) == (want["error"]["code"], want["error"]["path"]), result.error
        return
    assert result.status == "completed" and result.error is None, result.error
    if case["op"] == "validate":
        assert result.output is None, result.output
        return
    out = result.output.to_json()
    target = placed(want["target"], work)
    assert out["target"] == target, (out["target"], target)
    for key in ("whole", "written", "objectsRead", "objectsKept"):
        assert out[key] == want[key], (key, out[key], want[key])
    for words in want["containerHas"]:
        assert any(words in line for line in out.get("container", [])), (words, out.get("container"))
    lost = out.get("lost", [])
    assert len(lost) == len(want["lost"]), lost
    for got, w in zip(lost, want["lost"]):
        assert got["part"] == w["part"], (got, w)
        if "uid" in w:
            assert got.get("uid") == w["uid"], (got, w)
        if "code" in w:
            assert got.get("reason", "").startswith(f"{w['code']}:"), (got, w)
    path = Path(target)
    if want["written"]:
        _, snapshot = _KCAD.read(path.read_bytes())
        assert snapshot["uids"] == want["uids"], snapshot["uids"]
    elif not case["make"]:
        assert not path.exists(), "nothing is written"


def run_fixture(fx: dict[str, Any]) -> int:
    assert fx["command"] == "file.recover" and len(fx["cases"]) >= 20
    doc = cad.Document.new(workspace="cad", coords="local")
    for case in fx["cases"]:
        try:
            with tempfile.TemporaryDirectory(prefix="kentos-recover-python-") as name:
                play(case, doc, Path(name))
        except Exception as e:
            raise AssertionError(f"{fx['command']} › {case['name']}: {type(e).__name__}: {e}") from e
    return len(fx["cases"])
