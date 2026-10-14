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


def but_defaults(back: Any, want: Any) -> Any:
    """`back` without the fields the input left out and the SDK wrote with
    an empty value (``false``, ``null``, ``[]``, ``{}``): the same input."""
    if isinstance(back, dict) and isinstance(want, dict):
        return {
            k: but_defaults(v, want[k]) if k in want else v
            for k, v in back.items()
            if k in want or v not in (False, None, [], {})
        }
    if isinstance(back, list) and isinstance(want, list) and len(back) == len(want):
        return [but_defaults(b, w) for b, w in zip(back, want)]
    return back


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
    back = but_defaults(numbers(wrapper.input_type.from_json(literal).to_json()), want)
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
