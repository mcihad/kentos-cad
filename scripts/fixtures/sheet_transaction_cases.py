"""Model/paper transaction expectations from ADR 0590, without KentOS code."""
import copy
import json
import sys
from pathlib import Path
from sheets_edit_command_cases import SETUP, sheet

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/sheet/v1/model-transactions.json"


def call(command, input_, objects, sheets, **extra):
    return {"action": "command", "command": command, "input": input_,
            "expect": {"objects": objects, "sheets": sheets}, **extra}


def paper(ops, objects, sheets, **extra):
    return call("cad.sheets.edit", {"ops": ops}, objects, sheets, **extra)


def add(sid="a", name="A"):
    return {"op": "addSheet", "sheet": sheet(sid, name, 50)}


def rename(name):
    return {"op": "renameSheet", "id": "a", "name": name}


def point(objects, sheets, layer="cizim", **extra):
    return call("cad.point.create", {"layerId": layer, "p": {"x": 1, "y": 2}}, objects, sheets, **extra)


def change(steps, objects, sheets, **extra):
    return call("cad.changeset.apply", {"label": "Birlikte", "steps": [
        {"command": step["command"], "input": step["input"]} for step in steps
    ]}, objects, sheets, **extra)


def group(action, objects, sheets):
    return {"action": action, "expect": {"objects": objects, "sheets": sheets}}


def undo(objects, sheets, redo=False, error=None):
    return {"action": "redo" if redo else "undo", "expect": {"objects": objects, "sheets": sheets},
            **({"error": error, "unchanged": True} if error else {"taken": True})}


def build():
    mixed = change([point(0, []), paper([add()], 0, [])], 1, ["A"])
    only = change([paper([add()], 0, [])], 0, ["A"])
    initial = paper([add()], 0, ["A"])
    begin = group("begin", 0, [])
    after = lambda action, n, names: group(action, n, names)
    cases = []
    def case(name, *actions):
        cases.append({"name": name, "actions": list(copy.deepcopy(actions))})
    case("mixed changeset undo and redo", mixed, undo(0, []), undo(1, ["A"], True))
    case("paper-only changeset undo and redo", only, undo(0, []), undo(0, ["A"], True))
    case("mixed script group", begin, point(1, []), paper([add()], 1, ["A"]), after("end", 1, ["A"]), undo(0, []), undo(1, ["A"], True))
    case("paper-only script group", begin, initial, after("end", 0, ["A"]), undo(0, []), undo(0, ["A"], True))
    case("cancel mixed script", begin, point(1, []), paper([add()], 1, ["A"]), after("cancel", 0, []))
    case("cancel paper-only script", begin, initial, after("cancel", 0, []))
    case("cancel preserves an earlier standalone book", initial, after("begin", 0, ["A"]), paper([rename("B")], 0, ["B"]), point(1, ["B"]), after("cancel", 0, ["A"]))
    case("several book writes are one undo", begin, initial, paper([rename("B")], 0, ["B"]), paper([add("b", "C")], 0, ["B", "C"]), after("end", 0, ["B", "C"]), undo(0, []), undo(0, ["B", "C"], True))
    case("failed point after paper refuses entire changeset", change([paper([add()], 0, []), point(0, [], "absent")], 0, [], status="failed", code="layer_not_found", unchanged=True))
    case("failed paper after point refuses entire changeset", change([point(0, []), paper([{"op": "removeSheet", "id": "absent"}], 0, [])], 0, [], status="failed", code="unknown_sheet", unchanged=True))
    case("plan mixed writes nothing", {**mixed, "op": "plan", "expect": {"objects": 0, "sheets": []}, "unchanged": True})
    case("validate paper writes nothing", {**only, "op": "validate", "expect": {"objects": 0, "sheets": []}, "unchanged": True})
    case("standalone paper and model have separate undo", initial, point(1, ["A"]), undo(0, ["A"]), undo(1, ["A"], True))
    case("later paper edit blocks entire mixed undo", mixed, paper([rename("B")], 1, ["B"]), undo(1, ["B"], error="sheet_history_conflict"))
    case("later paper edit blocks entire mixed redo", mixed, undo(0, []), paper([add("b", "B")], 0, ["B"]), undo(0, ["B"], True, "sheet_history_conflict"))
    case("restored book permits undo again", mixed, paper([rename("B")], 1, ["B"]), undo(1, ["B"], error="sheet_history_conflict"), paper([rename("A")], 1, ["A"]), undo(0, []))
    case("restored book permits redo again", mixed, undo(0, []), paper([add("b", "B")], 0, ["B"]), undo(0, ["B"], True, "sheet_history_conflict"), paper([{"op": "removeSheet", "id": "b"}], 0, []), undo(1, ["A"], True))
    case("two mixed changesets undo sequentially", mixed, change([point(0, []), paper([rename("B")], 0, [])], 2, ["B"]), undo(1, ["A"]), undo(0, []), undo(1, ["A"], True), undo(2, ["B"], True))
    case("removal restores book on undo", initial, change([paper([{"op": "removeSheet", "id": "a"}], 0, [])], 0, []), undo(0, ["A"]), undo(0, [], True))
    case("failed changeset preserves existing redo", mixed, undo(0, []), change([paper([add()], 0, []), point(0, [], "absent")], 0, [], status="failed", unchanged=True), undo(1, ["A"], True))
    case("empty script group is no step", begin, after("end", 0, []), {"action": "undo", "taken": False, "expect": {"objects": 0, "sheets": []}, "unchanged": True})
    case("cancel preserves existing redo", mixed, undo(0, []), begin, initial, point(1, ["A"]), after("cancel", 0, []), undo(1, ["A"], True))
    setup = copy.deepcopy(SETUP)
    setup["entities"] = []
    return json.dumps({"format": "kentos.sheet-transactions", "version": 1, "source": "ADR 0590",
                       "setup": setup, "cases": cases}, ensure_ascii=False, indent=2) + "\n"


if __name__ == "__main__":
    text = build()
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            raise SystemExit("sheet transaction cases drift")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(text)
    print(f"Sheet transactions: {len(json.loads(text)['cases'])} independent cases")
