"""Checks the command history's Python code (docs/adr/0284) against the SDK.

The desktop's `kentos_native_application::python_call` writes, for a command
and its input, the code that runs it through ``kentos.cad``. This reads that
code as the Rust test wrote it (``KENTOS_PYTHON_CALLS_OUT``, a JSON list of
``{"command", "input", "code"}``) and, for every one, without KentOS's Rust:

- parses it (``ast``);
- finds the wrapper it names (``command = cad.…``) and checks that it is the
  SDK's wrapper of the command (``kentos.cad.COMMANDS``);
- reads the literal given to ``from_json`` (``ast.literal_eval``) and checks
  that it is the input (numbers compared as numbers);
- reads it with the command's input type and writes it back
  (``input_type.from_json(...).to_json()``): the same input, so the code
  runs the command with what the history recorded.

Run with the SDK's Python (``.run/py/bin/python``; ``pnpm py:test`` builds it):

    KENTOS_PYTHON_CALLS_OUT=$PWD/.run/python-calls.json cargo test -p kentos-native-application --test all python_calls
    .run/py/bin/python scripts/python/history_calls.py .run/python-calls.json
"""

from __future__ import annotations

import ast
import json
import sys
from pathlib import Path
from typing import Any


def numbers(value: Any) -> Any:
    """`value` with every number a float: JSON's 5 and 5.0 are one number."""
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, list):
        return [numbers(v) for v in value]
    if isinstance(value, dict):
        return {k: numbers(v) for k, v in value.items()}
    raise TypeError(type(value))


def without_expected(value: Any) -> Any:
    if isinstance(value, dict):
        return {k: v for k, v in value.items() if k != "expectedRevision"}
    return value


def resolved(schema: dict[str, Any], value: Any, root: dict[str, Any]) -> dict[str, Any]:
    """Resolve references and the selected tagged variant, retaining defaults on the property itself."""
    if not isinstance(schema, dict):
        return {}
    out = dict(schema)
    ref = out.pop("$ref", None)
    if isinstance(ref, str) and ref.startswith("#/"):
        target: Any = root
        for part in ref[2:].split("/"):
            target = target[part.replace("~1", "/").replace("~0", "~")]
        out = {**resolved(target, value, root), **out}
    for part in out.get("allOf", []):
        out = {**resolved(part, value, root), **out}
    for choices in (out.get("oneOf", []), out.get("anyOf", [])):
        candidates = [resolved(choice, value, root) for choice in choices]
        chosen = None
        for candidate in candidates:
            properties = candidate.get("properties", {})
            tags = {k: v["const"] for k, v in properties.items() if isinstance(v, dict) and "const" in v}
            if tags and isinstance(value, dict) and all(value.get(k) == v for k, v in tags.items()):
                chosen = candidate
                break
        if chosen is None:
            kind = ("null" if value is None else "boolean" if isinstance(value, bool) else
                    "object" if isinstance(value, dict) else "array" if isinstance(value, list) else
                    "string" if isinstance(value, str) else "number")
            matching = [c for c in candidates if c.get("type") == kind]
            if len(matching) == 1:
                chosen = matching[0]
        if chosen is not None:
            out = {**out, **chosen}
    return out


def but_defaults(back: Any, want: Any, schema: dict[str, Any] | None = None,
                 root: dict[str, Any] | None = None) -> Any:
    """Ignore omitted empty values and only the nonempty defaults explicitly declared by the contract.

    This is a codec check, not a command execution: omitted serde defaults are legitimate, while a provided
    value must survive exactly. In particular an arbitrary true/string must never disappear from the check.
    """
    schema = schema or {}
    root = schema if root is None else root
    schema = resolved(schema, back, root)
    if isinstance(back, dict) and isinstance(want, dict):
        properties = schema.get("properties", {})
        kept = {}
        for k, v in back.items():
            prop = properties.get(k, schema.get("additionalProperties", {}))
            prop = resolved(prop, v, root) if isinstance(prop, dict) else {}
            declared = "default" in prop and json.dumps(numbers(v), sort_keys=True) == json.dumps(numbers(prop["default"]), sort_keys=True)
            if k not in want and (v in (False, None, [], {}) or declared):
                continue
            kept[k] = but_defaults(v, want[k], prop, root) if k in want else v
        return kept
    if isinstance(back, list) and isinstance(want, list) and len(back) == len(want):
        item = schema.get("items", {})
        return [but_defaults(b, w, item, root) for b, w in zip(back, want)]
    return back


_SCHEMAS: dict[str, Any] | None = None


def input_schema(command: str) -> dict[str, Any]:
    global _SCHEMAS
    if _SCHEMAS is None:
        path = Path(__file__).resolve().parents[2] / "apps/web/src/contracts/generated/commandCatalog.json"
        catalog = json.loads(path.read_text())
        # Each input with the catalog's definitions it names by `#/$defs/…` (docs/adr/0731).
        defs = catalog.get("$defs", {})
        _SCHEMAS = {c["id"]: {**c["input"], "$defs": {**defs, **c["input"].get("$defs", {})}}
                    for c in catalog["commands"]}
    return _SCHEMAS[command]


def check(entry: dict[str, Any], cad: Any) -> str | None:
    tree = ast.parse(entry["code"])
    path = None
    literal = None
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and [getattr(t, "id", None) for t in node.targets] == ["command"]:
            path = ast.unparse(node.value)
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "from_json":
            literal = ast.literal_eval(node.args[0])
    if path is None or literal is None:
        return "no wrapper or no input in the code"
    wrapper = eval(path, {"cad": cad})  # noqa: S307 - a dotted path the test wrote
    if wrapper is not cad.COMMANDS[entry["command"]]:
        return f"{path} is not the wrapper of {entry['command']}"
    want = numbers(without_expected(entry["input"]))
    if numbers(literal) != want:
        return "the literal is not the input"
    back = but_defaults(numbers(wrapper.input_type.from_json(literal).to_json()), want, input_schema(entry["command"]))
    if json.dumps(back, sort_keys=True) != json.dumps(want, sort_keys=True):
        return "the SDK reads the input back otherwise"
    return None


def main() -> int:
    import kentos.cad as cad

    entries = json.load(open(sys.argv[1], encoding="utf-8"))
    bad = 0
    for entry in entries:
        try:
            problem = check(entry, cad)
        except Exception as e:  # noqa: BLE001 - every failure is reported
            problem = f"{type(e).__name__}: {e}"
        if problem:
            bad += 1
            print(f"{entry['command']}: {problem}\n{entry['code']}")
    print(f"{len(entries)} çağrı, {bad} sorunlu")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
