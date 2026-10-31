#!/usr/bin/env python3
"""The scene of the bonds in the way while a wall is dragged (docs/adr/0790): fixtures/interaction/v1/bonds-drag.kcad,
written without KentOS code.

arch_bond_cases.py's two rooms (the south outer wall S, the corridor's wall C 1.20 m from it, the partition P, three
doors) with three bonds already in it under fixed ids: d1 keeps the corridor (S's inner face to C's lower face,
1.20 m), S fixed by its axis, C fixed by its axis. Dragging C (its grips, Duvarı kaydır) is refused: C fixed alone
does not free it (d1 then pushes S, fixed), d1 and C's fix together do (docs/adr/0790 §2). The interaction trace
wall-bond-drag.json and the desktop's pictures play it. The file is KCAD v2 by kcad_v2_reference.py's independent
writer.

    python3 scripts/fixtures/bond_drag_scene.py [--check]
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_bond_conflict_cases as bc  # noqa: E402  (the drawing and its bonds)
import kcad_v2_reference as kv  # noqa: E402  (the independent KCAD v2 writer)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/interaction/v1/bonds-drag.kcad"


def scene():
    s = bc.Scene("bonds-drag", [bc.CORRIDOR, bc.FIX_S, bc.FIX_C])
    d = s.setup()
    d["name"] = "Etkileşim izi: yoldaki bağlar"
    return d


def main():
    data = kv.container(kv.document(scene()))
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_bytes() != data:
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches")
        return 0
    OUT.write_bytes(data)
    print(f"{OUT.relative_to(ROOT)} written: {len(data)} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
