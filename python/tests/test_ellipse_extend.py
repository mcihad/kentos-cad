"""CAD-ADV-04: analytic ellipse extension through the public typed SDK."""
import json
import math
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]

def drawing():
    raw = json.loads((ROOT / "fixtures/interaction/v1/objects.kcad").read_text())
    raw["settings"].update(workspace="cad", srid=0, drawingUnit="m")
    raw["origin"] = dict(x=0, y=0)
    raw["entities"] = [
        dict(kind="ellipse", id=1, layerId="cizim", attrs={"QA": "ellipse"},
             c=dict(x=0,y=0),major=dict(x=4,y=0),ratio=0.5,t0=0,t1=math.pi/2),
        dict(kind="line",id=2,layerId="cizim",attrs={},a=dict(x=-2,y=0),b=dict(x=-2,y=3))]
    return cad.Document.from_bytes(json.dumps(raw).encode())

class EllipseExtend(unittest.TestCase):
    def test_plan_execute_analytic_source_metadata_and_undo(self):
        doc = drawing()
        source, boundary = [r.uid for r in doc.entities()]
        before = doc.to_bytes()
        picks = [cad.TrimPick(uid=source, at=(0,2))]
        bounds = cad.ObjectsTrimBoundaries(uids=[boundary])
        plan = cad.entities.extend.plan(doc,picks=picks,boundaries=bounds)
        self.assertEqual(doc.to_bytes(),before)
        cad.entities.extend(doc,picks=picks,boundaries=bounds)
        result = doc.entity(source).entity.to_json()
        self.assertEqual(result,plan.changed[0].to_json())
        self.assertEqual(result["kind"],"ellipse")
        self.assertAlmostEqual(result["t1"],2*math.pi/3,places=12)
        self.assertEqual(result["attrs"],{"QA":"ellipse"})
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(),doc.to_bytes())
        self.assertEqual(doc.undo(),"Uzat")
        self.assertEqual(doc.to_bytes(),before)

if __name__ == "__main__":
    unittest.main()
