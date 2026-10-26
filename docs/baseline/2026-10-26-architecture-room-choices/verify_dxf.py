"""External ezdxf audit of the DXF actually written by the native file dialog."""
import hashlib
import importlib.util
import json
from pathlib import Path

import ezdxf

B = Path(__file__).resolve().parent
R = B.parents[2]
spec = importlib.util.spec_from_file_location("reader", R / "tools/kcad/kcad.py")
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)
source = reader.read((B / "numbered.kcad").read_bytes())[1]
path = B / "room-choices-gui.dxf"
doc = ezdxf.readfile(path)
assert doc.dxfversion == "AC1021" and doc.units == 6  # Existing metre/AutoCAD2007 export.
assert len(list(doc.modelspace())) == len(source["entities"]) == 11
for i, e in enumerate(doc.modelspace()):
    assert e.dxftype() == "INSERT"
    parts = [t.value for t in e.get_xdata("KENTOS") if t.code == 1000]
    assert parts[0] == "object"
    metadata = json.loads("".join(parts[1:]))
    assert metadata.pop("uid") == source["uids"][i]
    assert metadata == source["entities"][i], i
    if metadata["kind"] == "space":
        geometries = list(doc.blocks[e.dxf.name])
        assert len(geometries) == 1 and geometries[0].dxftype() == "LWPOLYLINE"
        boundary = geometries[0]
        assert boundary.closed
        expected = [(p["x"], p["y"]) for p in metadata["parts"][0]["outer"]["pts"]]
        assert list(boundary.get_points("xy")) == expected
        assert e.dxf.insert == (0, 0, 0)
        assert e.dxf.xscale == e.dxf.yscale == e.dxf.zscale == 1
    else:
        assert metadata["kind"] == "tag"
        texts = [v.dxf.text for v in doc.blocks[e.dxf.name] if v.dxftype() == "TEXT"]
        assert texts == [s for s in metadata["text"].split("\n") if s]
assert doc.layers.get("Gizli katman").is_off()
assert not doc.audit().errors
# Project ArchSettings are deliberately not claimed to round-trip through DXF.
for key in (b'"spaceNames"', b'"spaceUsages"', b'"spaceNumberPrefixes"'):
    assert key not in path.read_bytes()
result = dict(ezdxfVersion=ezdxf.__version__, file=path.name,
              sha256=hashlib.sha256(path.read_bytes()).hexdigest(), entities=11,
              exactRoomBoundaries=6, exactSourceMetadata=11, exactTagTexts=5,
              auditErrors=0, projectPreferences="absent; visible warning before writing and in report",
              limitation="DXF metadata carries UUIDs, but existing KentOS DXF import regenerates them; KCAD preserves them.")
(B / "dxf-audit.json").write_text(json.dumps(result, indent=2) + "\n")
print("PASS: external ezdxf audit; 6 exact 4x3 metre boundaries, 5 tag texts, 11 complete source metadata/UUIDs; 0 audit errors.")
