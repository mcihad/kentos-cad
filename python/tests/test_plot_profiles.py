"""Native plot table identities stay distinct through the typed command, KCAD and inverse."""
import json,unittest
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
CASES=json.loads((ROOT/'fixtures/sheet/v1/plot-profiles.json').read_text())['cases']
MODEL=json.loads((ROOT/'fixtures/sheet/v1/page-setups.json').read_text())['cases'][0]['model']
class PlotProfiles(unittest.TestCase):
 def test_twenty_plot_tables_persist_even_before_the_first_sheet(self):
  for c in CASES:
   with self.subTest(case=c['name']):
    doc=cad.Document.from_bytes(json.dumps(MODEL).encode());before=[r.entity.to_json() for r in doc.entities()]
    result=cad.sheets.edit(doc,ops=[dict(op='savePlotProfiles',profiles=[c['exchange']['profile']])]);book=cad.sheets.set(doc,details=True).book.to_json()
    self.assertEqual(book['plotProfiles'][0],c['exchange']['profile']);self.assertEqual(book['sheets'],[])
    reopened=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),book)
    cad.sheets.edit(doc,ops=result.inverse);self.assertEqual(cad.sheets.set(doc,details=True).sheets,[]);self.assertEqual([r.entity.to_json() for r in doc.entities()],before)
 def test_twenty_applied_profiles_are_self_contained_and_survive_library_removal(self):
  import copy
  base=json.loads((ROOT/'fixtures/sheet/v1/page-setups.json').read_text())['cases'][0]
  for c in CASES:
   with self.subTest(case=c['name']):
    profile=c['exchange']['profile'];setup=copy.deepcopy(base['setup']);setup['settings']['profile']=profile['id']
    doc=cad.Document.from_bytes(json.dumps(MODEL).encode());cad.sheets.edit(doc,ops=[dict(op='savePlotProfiles',profiles=[profile]),dict(op='savePageSetups',setups=[setup]),dict(op='addSheet',sheet=base['sheet']),dict(op='applyPageSetup',sheet='page',setup=setup['id'])])
    kept=cad.sheets.set(doc,details=True).book.to_json();self.assertEqual(kept['sheets'][0]['plot']['settings']['profileSnapshot'],profile)
    cad.sheets.edit(doc,ops=[dict(op='savePlotProfiles',profiles=[])]);reopened=cad.Document.from_bytes(doc.to_bytes());self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json()['sheets'][0]['plot']['settings']['profileSnapshot'],profile)
