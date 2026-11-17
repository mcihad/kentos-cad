#!/usr/bin/env python3
"""The issue KentOS wrote, read by others (docs/adr/8103; plan/ifc-uyum.md IFC-10's first acceptance): IfcOpenShell's
BCF library (bcf-client, the BCF engine of its viewer) opens fixtures/ifc/v1/revizyon/tasinan-duvar.bcf, which KentOS
wrote (crates/native/application/tests/all/ifc_revisions.rs), and IfcOpenShell opens the IFC file the topic names
(fixtures/ifc/v1/revizyon/kesit-binasi-r2.ifc). The topic's viewpoint must select the wall that moved (its GlobalId, an
IfcWall in the file) and its camera must look at that wall where the second revision has it (the ray from its eye
along its direction passes the wall's box, by IfcOpenShell's geometry, within a tenth of a millimetre of its middle);
the comment and the file's name must read back. Prints what it found; exits 1 when something differs.

    uv run --offline --no-project --python 3.11 --with bcf-client==0.9.0 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/bcf_check.py
"""

import json
import math
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
from bcf.v3.bcfxml import BcfXml

ROOT = Path(__file__).resolve().parents[2]
BCF = ROOT / "fixtures/ifc/v1/revizyon/tasinan-duvar.bcf"
IFC = ROOT / "fixtures/ifc/v1/revizyon/kesit-binasi-r2.ifc"
NOTE = ROOT / "fixtures/ifc/v1/revizyon/kesit-binasi-r2.json"


def main():
    note = json.loads(NOTE.read_text(encoding="utf-8"))
    moved = note["moved"]["globalId"]
    problems = []
    bcf = BcfXml.load(str(BCF))
    version = bcf.version.version_id
    topics = list(bcf.topics.values())
    if version != "3.0":
        problems.append(f"sürüm {version}")
    if len(topics) != 1:
        problems.append(f"{len(topics)} konu")
    topic = topics[0]
    markup = topic.markup
    title = markup.topic.title
    files = [f.filename for f in (markup.header.files.file if markup.header and markup.header.files else [])]
    comments = [c.comment for c in (markup.topic.comments.comment if markup.topic.comments else [])]
    views = list(topic.viewpoints.values())
    if len(views) != 1:
        problems.append(f"{len(views)} bakış")
    info = views[0].visualization_info
    selected = [c.ifc_guid for c in info.components.selection.component]
    camera = info.perspective_camera
    eye = [camera.camera_view_point.x, camera.camera_view_point.y, camera.camera_view_point.z]
    look = [camera.camera_direction.x, camera.camera_direction.y, camera.camera_direction.z]
    f = ifcopenshell.open(str(IFC))
    wall = f.by_guid(selected[0])
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    v = ifcopenshell.geom.create_shape(settings, wall).geometry.verts
    xs, ys, zs = v[0::3], v[1::3], v[2::3]
    box = [min(xs), min(ys), min(zs), max(xs), max(ys), max(zs)]
    middle = [(box[0] + box[3]) / 2, (box[1] + box[4]) / 2, (box[2] + box[5]) / 2]
    to = [m - e for m, e in zip(middle, eye)]
    n = math.sqrt(sum(d * d for d in look))
    unit = [d / n for d in look]
    along = sum(t * u for t, u in zip(to, unit))
    off = math.sqrt(max(0.0, sum(t * t for t in to) - along * along))
    if selected != [moved]:
        problems.append(f"seçilen {selected}, beklenen {moved}")
    if not wall.is_a("IfcWall"):
        problems.append(f"{wall.is_a()} bir duvar değil")
    if any(abs(a - b) > 1e-4 for a, b in zip(box, note["moved"]["after"])):
        problems.append(f"duvarın kutusu {box}, ikinci revizyonda {note['moved']['after']}")
    if along <= 0 or off > 1e-4:
        problems.append(f"kamera duvara bakmıyor: ışın ortadan {off:.6f} m uzakta, önünde {along:.3f} m")
    if files != ["kesit-binasi-r2.ifc"]:
        problems.append(f"dosyalar {files}")
    if comments != ["Statik projeyle birlikte bakılsın."]:
        problems.append(f"yorumlar {comments}")
    print(json.dumps({"version": version, "title": title, "selected": selected, "class": wall.is_a(), "name": wall.Name,
                      "box": [round(x, 4) for x in box], "rayOffMiddle": round(off, 9), "distance": round(along, 3),
                      "files": files, "comments": comments}, ensure_ascii=False, indent=2))
    if problems:
        print("uyuşmuyor: " + "; ".join(problems), file=sys.stderr)
        return 1
    print("uyuyor: bağımsız okuyucu aynı duvarı gösteriyor")
    return 0


if __name__ == "__main__":
    sys.exit(main())
