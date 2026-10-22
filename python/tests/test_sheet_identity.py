"""Viewport queries retain source uid/measurement and respect freeze, representation and polygon clips."""
import json,unittest
from pathlib import Path
import kentos.cad as cad
CASES=json.loads((Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-identities.json').read_text())['cases']
class SheetIdentities(unittest.TestCase):
    def test_twenty_explicit_viewports_share_source_ids_measurement_and_snap_order(self):
        for c in CASES:
            with self.subTest(case=c['name']):
                doc=cad.Document.from_bytes(json.dumps(c['model']).encode());uids=[r.uid for r in doc.entities()]
                cad.sheets.edit(doc,ops=[{'op':'addSheet','sheet':c['sheet']},c['operation']]);before=doc.to_bytes()
                scope=cad.SheetModelScope(sheet='page',item='viewport')
                chosen=cad.entities.select(doc,region=cad.AllSelectRegion(),model_scope=scope)
                self.assertEqual(chosen.uids,[uids[i-1] for i in c['expected']['selectedSlots']])
                snap=c['snap'];result=cad.snap.find(doc,model_scope=scope,at=(snap['at']['x'],snap['at']['y']),aperture=snap['aperture'],kinds=['node'])
                self.assertEqual(result.snaps[0].uid,uids[4]);self.assertAlmostEqual(doc.measure(uids[0]).length,5.)
                picks=cad.entities.pick(doc,model_scope=scope,at=result.snaps[0].point,tolerance=snap['aperture']/10)
                self.assertEqual(picks.picks[0].uid,uids[4]);self.assertEqual(doc.to_bytes(),before)
                grouped=cad.changeset.apply(doc,model_scope=scope,steps=[cad.ChangesetStep(command='cad.entities.select',input={'region':{'kind':'all'}})])
                self.assertEqual(grouped.steps[0].output['uids'],chosen.uids)
                self.assertEqual(len(list(doc.entities())),5)
                self.assertEqual(len(cad.entities.select(doc,region=cad.AllSelectRegion()).uids),5)
