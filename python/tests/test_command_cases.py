"""The shared product command cases (fixtures/commands/v1) through the typed
Python layer: every step's input read into its generated type, sent by the
wrapper, its answer read back into the generated types and written again,
then compared with the case, as the web (apps/web/src/product/fixtures.test.ts)
and the desktop (crates/native/application/tests/all/fixtures.rs) compare it.
The format is in fixtures/commands/README.md.

One difference is by design: JSON carries no NaN or ±∞, so a step whose
`nonFinite` puts them into the input is refused here by the SDK before it
reaches the command (`InvalidInput`, `not_finite`), and nothing is written.
"""

from __future__ import annotations

import json
import keyword
import math
import re
import unittest
from pathlib import Path
from typing import Any

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
FILES = sorted((ROOT / "fixtures/commands/v1").glob("*.json"))


class Case:
    def __init__(self, command: str, setup: dict[str, Any], steps: list[dict[str, Any]], at: str) -> None:
        self.wrapper = cad.command(command)
        self.doc = cad.Document.from_bytes(json.dumps(setup).encode())
        info = self.doc.info()
        assert not info.dirty and not info.can_undo, f"{at}: the setup is not clean"
        self.steps = steps
        self.at = at
        self.revisions: dict[str, str] = {}
        self.uids: dict[str, str] = {}
        # Block ids taken by `captureBlock`, and the setup's (docs/adr/0144).
        self.blocks: dict[str, str] = {}
        self.setup_blocks = [b.id for b in self.doc.blocks()]

    def slots(self) -> dict[int, cad.Record]:
        return {r.entity.id: r for r in self.doc.entities(page_size=10_000)}

    def block_named(self, name: str) -> cad.BlockDefinition | None:
        """The drawing's block of that name, Turkish case folded."""
        key = fold(name)
        return next((b for b in self.doc.blocks() if fold(b.name) == key), None)

    def fill(self, value: Any, at: str) -> Any:
        if isinstance(value, str):
            if value.startswith("$blockOf:"):
                block = self.block_named(value[len("$blockOf:"):])
                assert block is not None, f"{at}: {value}: no such block"
                return block.id
            if value.startswith("$block:"):
                return self.blocks[value[len("$block:"):]]
            if value.startswith("$uidOf:"):
                slot = int(value[len("$uidOf:"):])
                record = self.slots().get(slot)
                assert record is not None, f"{at}: no object in slot {slot}"
                return record.uid
            if "$uid:" in value:
                return re.sub(r"\$uid:([A-Za-z0-9_-]+)", lambda m: self.uids[m.group(1)], value)
            if value.startswith("$"):
                name = value[1:]
                return self.doc.revision if name == "current" else self.revisions[name]
            return value
        if isinstance(value, list):
            return [self.fill(v, at) for v in value]
        if isinstance(value, dict):
            return {k: self.fill(v, at) for k, v in value.items()}
        return value

    def run(self) -> None:
        for i, step in enumerate(self.steps):
            at = f"{self.at} › {i + 1} {step['op']}"
            before = self.doc.revision
            op = step["op"]
            if op in ("validate", "plan", "execute"):
                self.command(step, at)
            elif op in ("undo", "redo"):
                label = self.doc.undo() if op == "undo" else self.doc.redo()
                if "returns" in step:
                    assert label == step["returns"], f"{at}: returned {label!r}"
            elif op == "captureRevision":
                self.revisions[step["as"]] = self.doc.revision
            elif op == "captureUid":
                self.uids[step["as"]] = self.slots()[step["id"]].uid
            elif op == "captureBlock":
                block = self.block_named(step["name"])
                assert block is not None, f"{at}: no block {step['name']}"
                self.blocks[step["as"]] = block.id
            else:
                raise AssertionError(f"{at}: unknown op {op}")
            self.check(step.get("expect"), before, at)

    def command(self, step: dict[str, Any], at: str) -> None:
        typed = self.wrapper.input_type.from_json(self.fill(step["input"], at))
        if "nonFinite" in step:
            put_non_finite(typed, step["nonFinite"])
            before = self.doc.revision
            try:
                self.wrapper.run(self.doc, typed, op=step["op"])
            except cad.InvalidInput as e:
                assert e.code == "not_finite", f"{at}: {e}"
            else:
                raise AssertionError(f"{at}: a non-finite number crossed the JSON boundary")
            assert self.doc.revision == before, f"{at}: written"
            return
        outcome = self.wrapper.run(self.doc, typed, op=step["op"])
        got = wire(outcome)
        want = json.loads(json.dumps(step["result"]))
        if want.get("output", {}) and isinstance(want.get("output"), dict) and want["output"].get("uid") == "$uid":
            uid = got["output"]["uid"]
            slot = got["output"]["id"]
            assert self.slots()[slot].uid == uid, f"{at}: output.uid {uid} is not slot {slot}'s"
            want["output"]["uid"] = uid
        same(got, self.fill(want, at), f"{at}: result")

    def check(self, expect: dict[str, Any] | None, before: str, at: str) -> None:
        if not expect:
            return
        info = self.doc.info()
        for key, want in expect.items():
            if key == "ids":
                same([r.entity.id for r in self.doc.entities(page_size=10_000)], want, f"{at}: ids")
            elif key == "entities":
                slots = self.slots()
                for slot, entity in want.items():
                    record = slots.get(int(slot))
                    same(None if record is None else record.entity.to_json(), self.block_ids(entity, at), f"{at}: object {slot}")
            elif key in ("canUndo", "canRedo", "dirty"):
                flag = {"canUndo": info.can_undo, "canRedo": info.can_redo, "dirty": info.dirty}[key]
                assert flag == want, f"{at}: {key} {flag}"
            elif key == "revision":
                moved = "same" if self.doc.revision == before else "changed"
                assert moved == want, f"{at}: revision {moved}"
            elif key == "uids":
                slots = self.slots()
                for slot, name in want.items():
                    uid = slots[int(slot)].uid
                    if name == "new":
                        assert uid not in self.uids.values(), f"{at}: {slot} is not new"
                    else:
                        assert self.uids[name] == uid, f"{at}: {slot} is not {name}"
            elif key == "blocks":
                got = []
                for b in self.doc.blocks():
                    data = b.to_json()
                    data.pop("id")
                    got.append(data)
                same(got, want, f"{at}: blocks")
            elif key == "blockIds":
                for name, id in want.items():
                    block = self.block_named(name)
                    assert block is not None, f"{at}: no block {name}"
                    if id == "new":
                        assert block.id not in self.setup_blocks and block.id not in self.blocks.values(), f"{at}: {name} is not new"
                    else:
                        assert block.id == self.fill(id, at), f"{at}: {name} is {block.id}"
            else:
                raise AssertionError(f"{at}: unknown expectation {key}")

    def block_ids(self, value: Any, at: str) -> Any:
        """An expected object with its block ids filled in (`$blockOf:Ad`, `$block:name`); nothing else is a placeholder."""
        if isinstance(value, str):
            return self.fill(value, at) if value.startswith(("$blockOf:", "$block:")) else value
        if isinstance(value, list):
            return [self.block_ids(v, at) for v in value]
        if isinstance(value, dict):
            return {k: self.block_ids(v, at) for k, v in value.items()}
        return value


def fold(name: str) -> str:
    """A block name as names are compared: Turkish I (I → ı, İ → i), then lower case, character by character."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in name)


def wire(outcome: cad.Outcome[Any]) -> dict[str, Any]:
    """The typed answer written back as the wire's CommandResult."""
    if outcome.error is not None:
        e = outcome.error
        error: dict[str, Any] = {"code": e.code, "message": e.message}
        if e.path is not None:
            error["path"] = e.path
        if e.revision is not None:
            error["revision"] = e.revision
        return {"status": outcome.status, "error": error}
    notes = []
    for n in outcome.warnings:
        note: dict[str, Any] = {"code": n.code, "message": n.message}
        if n.path is not None:
            note["path"] = n.path
        notes.append(note)
    output = outcome.output.to_json() if outcome.output is not None else None
    return {"status": outcome.status, "output": output, "warnings": notes}


def same(got: Any, want: Any, at: str) -> None:
    """Equal as JSON, numbers as numbers (the file's 1 is 1.0)."""
    if isinstance(want, dict):
        assert isinstance(got, dict) and set(got) == set(want), f"{at}: {json.dumps(got)} != {json.dumps(want)}"
        for k in want:
            same(got[k], want[k], f"{at}.{k}")
    elif isinstance(want, list):
        assert isinstance(got, list) and len(got) == len(want), f"{at}: {json.dumps(got)} != {json.dumps(want)}"
        for i, (a, b) in enumerate(zip(got, want)):
            same(a, b, f"{at}[{i}]")
    elif isinstance(want, (int, float)) and not isinstance(want, bool):
        assert isinstance(got, (int, float)) and not isinstance(got, bool) and got == want, f"{at}: {got!r} != {want!r}"
    else:
        assert got == want, f"{at}: {got!r} != {want!r}"


NON_FINITE = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}


def put_non_finite(value: Any, table: dict[str, str]) -> None:
    """NaN or ±∞ at each path of the typed input (`pts[1].y`, `r`,
    `transform.center.x`, `changes[0].geometry.pts[1].x`). A point is a
    tuple (Vec2): its holder takes a new one."""
    for path, text in table.items():
        number = NON_FINITE[text]
        *head, last = re.findall(r"[A-Za-z0-9_]+|\[\d+\]", path)
        if last in ("x", "y") and head:
            *outer, key = head
            holder = _walk(value, outer)
            point = _get(holder, key)
            if isinstance(point, cad.Vec2):
                _set(holder, key, point._replace(**{last: number}))
                continue
        _set(_walk(value, head), last, number)


def _walk(value: Any, parts: list[str]) -> Any:
    for p in parts:
        value = _get(value, p)
    return value


def _get(value: Any, part: str) -> Any:
    return value[int(part[1:-1])] if part.startswith("[") else getattr(value, _py(part))


def _set(value: Any, part: str, new: Any) -> None:
    if part.startswith("["):
        value[int(part[1:-1])] = new
    else:
        setattr(value, _py(part), new)


def _py(name: str) -> str:
    """`layerId` → `layer_id`; a Python keyword gets a trailing underscore, as the generated types name it (`from_`)."""
    s = re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", name).lower()
    return s + "_" if keyword.iskeyword(s) else s


class CommandCases(unittest.TestCase):
    def test_every_case_through_the_typed_layer(self) -> None:
        self.assertTrue(FILES)
        problems = []
        cases = 0
        for path in FILES:
            fixture = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual((fixture["format"], fixture["version"]), ("kentos.command-cases", 1))
            wrapper = cad.command(fixture["command"])
            self.assertEqual(wrapper.version, fixture["commandVersion"])
            for case in fixture["cases"]:
                cases += 1
                at = f"{path.name} › {case['name']}"
                try:
                    Case(fixture["command"], case.get("setup", fixture["setup"]), case["steps"], at).run()
                except (AssertionError, cad.KentosError, KeyError, TypeError, ValueError) as e:
                    problems.append(f"{at}: {type(e).__name__}: {e}")
        self.assertGreaterEqual(cases, 300)
        self.assertEqual(problems, [], f"{len(problems)} of {cases} cases failed:\n" + "\n".join(problems[:40]))


if __name__ == "__main__":
    unittest.main()
