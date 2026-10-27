#!/usr/bin/env python3
"""Version 1: independent, portable saved-model/evidence verification.

No app, Cargo, native Python extension or ignored helper is imported. The
standalone KCAD reference reader validates each complete tracked model. Raw
screens/PDFs remain local. Default verification fails if any required raw
artifact/binary is absent; explicit scoped modes never claim GUI acceptance.
"""
from pathlib import Path
from copy import deepcopy
import argparse
import hashlib
import importlib.util
import json
import math
import struct

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]
READER = ROOT/'tools/kcad/kcad.py'
VERIFIER_VERSION = 1
MODELS = {
    '22':'22-single-confirm.kcad', '23':'23-undo.kcad', '24':'24-redo.kcad',
    '28':'28-width-200cm.kcad', '29':'29-opening-deleted.kcad',
    '33':'33-opening-added.kcad', '35':'35-opening-position-7m.kcad',
    '40':'40-row-offset-3m.kcad', '41':'41-one-member-hidden.kcad',
    '44':'44-member-restored.kcad', '46':'46-centers.kcad', '48':'48-reopened.kcad',
    '51':'51-sheet.kcad', '58':'58-live2-cancel.kcad',
    '64':'64-live2-exact-distance.kcad', '65':'65-live2-exact-distance-undo.kcad',
    '66':'66-live2-exact-distance-redo.kcad', '70':'70-live2-cursor-cancel.kcad',
    '73':'73-live2-style-100-close.kcad', '76':'76-live2-style-200.kcad',
    '78':'78-live2-rejected-no-write.kcad', '80':'80-live2-style-200-sheet.kcad',
    '83':'83-live2-navigation-no-write.kcad', '88':'88-live2-detached.kcad',
    '89':'89-live2-detach-undo.kcad',
    '90':'90-live2-source-wall-deleted.kcad', '92':'92-live2-source-delete-undo.kcad',
    '96':'96-live2-whole-set-deleted.kcad', '97':'97-live2-whole-delete-undo.kcad',
    '125':'125-live3-navigation-no-write.kcad', '128':'128-live3-wall-style-cancel.kcad', '130':'130-live3-style-name-saved.kcad',
    '137':'137-live3-style-manually-restored.kcad',
    '141':'141-live3-style-navigation-no-write.kcad',
    '153':'153-live3-single-confirm-created.kcad', '154':'154-live3-single-undo.kcad',
    '155':'155-live3-single-redo.kcad',
    '156':'156-live3-new-process-reopened.kcad', '164':'164-live3-style-draft-cancelled.kcad',
    '165-seed':'165-long-style-seed.kcad', '176':'176-live3-long-style-cancelled.kcad',
    '183':'183-normal-zoom-roundtrip.kcad',
}
checks = []


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024*1024),b''):
            digest.update(chunk)
    return digest.hexdigest()


def check(condition, name):
    if not condition:
        raise AssertionError(name)
    checks.append(name)


def same(actual, expected, name):
    check(actual == expected, name)


def near(actual, expected, name):
    check(math.isfinite(actual) and math.isclose(actual, expected, rel_tol=0, abs_tol=1e-10),
          f'{name}: {actual!r} versus {expected!r}')


def sans(value, *keys):
    return {k: v for k, v in value.items() if k not in keys}


def objects(doc):
    return {uid: sans(entity, 'id') for uid, entity in zip(doc['uids'], doc['entities'])}


def group(doc):
    same(len(doc.get('dimensionSets', [])), 1, 'one retained recipe')
    return doc['dimensionSets'][0]


def one(doc, kind):
    values = [(uid, e) for uid, e in objects(doc).items() if e['kind'] == kind]
    same(len(values), 1, 'one ' + kind)
    return values[0]


def members(doc, row=None):
    by = objects(doc)
    return [(member, by[member['uid']]) for member in group(doc)['members']
            if row is None or member['row'] == row]


def assert_chain(doc, anchors, expected_lengths, label, *, centers=False, gap_mm=10.):
    """Independent horizontal host/jamb math; centre dimensions are projected.

    At centres, real sources have y=0 while the wall face has y=.1. Linear
    dimensions measure along angle=0, not the Euclidean slanted chord.
    """
    by = objects(doc)
    recipe = group(doc)
    wall_uid = recipe['wall']
    wall = by[wall_uid]
    same(wall['pts'], [{'x': 0., 'y': 0.}, {'x': 10., 'y': 0.}], label + ' fixed 10m host')
    near(wall['width'], .2, label + ' host width')
    same(wall.get('justify'), 'center', label + ' centered host')
    near(doc['settings']['plotScale'], 50., label + ' plot scale')
    same(recipe['face'], 'left', label + ' physical face')
    same(recipe['openings'], 'centers' if centers else 'ends', label + ' mode')
    near(recipe['gapMm'], gap_mm, label + ' independently specified paper gap')
    face_y = .2 / 2
    nodes = [(0., face_y)]
    source_nodes = [(wall_uid, {'kind':'vertex','part':0,'ring':0,'index':0})]
    actual_openings = sorted(((uid, e) for uid, e in by.items() if e['kind'] == 'opening'), key=lambda pair:pair[1]['at'])
    same(len(actual_openings), len(anchors), label + ' opening count')
    for (uid, opening), (at, width) in zip(actual_openings, anchors):
        near(opening['at'], at, label + ' opening anchor')
        near(opening['width'], width, label + ' opening width')
        same(opening['host'], wall_uid, label + ' host UUID')
        expected_place = [{'x':at-width/2,'y':face_y}, {'x':at-width/2,'y':-face_y},
                          {'x':at+width/2,'y':face_y}, {'x':at+width/2,'y':-face_y}]
        same(opening['place'], expected_place, label + ' physical jamb corners')
        if centers:
            nodes.append((at, 0.))
            source_nodes.append((uid, {'kind':'center'}))
        else:
            nodes += [(at-width/2,face_y), (at+width/2,face_y)]
            source_nodes += [(uid,{'kind':'vertex','part':0,'ring':0,'index':0}),
                             (uid,{'kind':'vertex','part':0,'ring':0,'index':2})]
    nodes.append((10., face_y))
    source_nodes.append((wall_uid,{'kind':'vertex','part':0,'ring':0,'index':1}))
    pairs = list(zip(nodes, nodes[1:])) + [((0.,face_y),(10.,face_y))]
    refs = list(zip(source_nodes, source_nodes[1:])) + [(source_nodes[0],source_nodes[-1])]
    live = members(doc)
    same(len(live), len(pairs), label + ' active members')
    same([b[0]-a[0] for a,b in pairs], expected_lengths, label + ' independently calculated intervals')
    explicit = {p['row']:p['at'] for p in recipe.get('rowPlaces',[])}
    for index, ((member, dimension), (a,b), (ref_a,ref_b), length) in enumerate(zip(live,pairs,refs,expected_lengths)):
        row = 'overall' if index == len(pairs)-1 else 'openings'
        same(member['row'], row, label + ' row order')
        same(dimension['kind'], 'dimension', label + ' ordinary dimension')
        same(dimension['style'], 'linear', label + ' linear measurement')
        near(dimension['angle'], 0., label + ' angle')
        near(dimension['a']['x'], a[0], label + ' endpoint a.x')
        near(dimension['a']['y'], a[1], label + ' endpoint a.y')
        near(dimension['b']['x'], b[0], label + ' endpoint b.x')
        near(dimension['b']['y'], b[1], label + ' endpoint b.y')
        near(dimension['b']['x']-dimension['a']['x'], length, label + ' projected length')
        for name, (uid, place) in [('a',ref_a),('b',ref_b)]:
            expected = {'uid':uid,'place':place}
            same(member[name],expected,label + ' stable source subplace')
            same(dimension['refs'][name],expected,label + ' dimension/source agreement')
        y = explicit[row]['y'] if row in explicit else recipe['at']['y'] + (0. if row=='openings' else gap_mm*50./1000.)
        near(dimension['offset'], y-a[1], label + ' world row placement')
        near(dimension['height'], .125, label + ' unchanged text height')


def verify_models(docs,reader,proof2):
    base = docs['22']
    wall_uid, wall = one(base,'wall')
    line_uid, line = one(base,'line')
    opening_uid, opening = one(base,'opening')
    same(line['attrs'],{'qa':'unrelated-preserve'},'unrelated sentinel line')
    for key,doc in docs.items():
        if key in ['73','76','78','80','83','125','128','130','137','141','165-seed','176','183']:
            continue  # Separately specified full original/long-style fixtures below.
        by = objects(doc)
        same(len(by),len(doc['entities']),key+' unique persistent identities')
        same(sans(doc,'entities','uids','dimensionSets','sheets'),sans(base,'entities','uids','dimensionSets','sheets'),key+' non-sheet project metadata unchanged')
        if key != '90':
            same(by[wall_uid],wall,key+' host untouched')
        same(by[line_uid],line,key+' unrelated line untouched despite slot renumbering')
        sets = doc.get('dimensionSets',[])
        expected_ids = {m['uid'] for s in sets for m in s['members']}
        if key != '88':  # Detach deliberately leaves ordinary associative dimensions.
            same({uid for uid,e in by.items() if e['kind']=='dimension'},expected_ids,key+' no orphan/missing ordinary members')
        for s in sets:
            same(s['id'],group(docs['153'] if key in ['153','155','156','164'] else docs['64'] if key in ['64','66','70','89','90','92','97'] else base)['id'],key+' stable recipe UUID')
    same(len(base['entities']),7,'22 seven objects')
    assert_chain(base,[(4.,1.)],[3.5,1.,5.5,10.],'22 create')
    same(len(docs['23']['entities']),3,'23 undo restores three source objects')
    same(docs['23'].get('dimensionSets',[]),[],'23 undo removes recipe')
    same(objects(docs['23']),{uid:e for uid,e in objects(base).items() if e['kind']!='dimension'},'23 undo source state exact')
    same(docs['24'],base,'24 redo full document equals22')
    same(group(docs['28']),group(base),'28 full recipe/UIDs/row positions unchanged')
    same(set(docs['28']['uids']),set(base['uids']),'28 all UUIDs unchanged')
    assert_chain(docs['28'],[(4.,2.)],[3.,2.,5.,10.],'28 width200cm')
    overall = members(base,'overall')[0][0]['uid']
    gone = docs['29']; gone_by=objects(gone)
    same(len(gone['entities']),4,'29 four objects after opening deletion')
    same(len(group(gone)['members']),2,'29 rebuilt opening and overall rows')
    check(opening_uid not in gone_by,'29 source opening removed')
    same(gone_by[overall],objects(docs['28'])[overall],'29 overall geometry/UUID survives slot renumbering')
    assert_chain(gone,[],[10.,10.],'29 no openings')
    clicked = 8.000242248062015
    check(clicked != 8.,'33 free click is not mislabeled as exact8m')
    assert_chain(docs['33'],[(4.,2.),(clicked,1.)],[3.,2.,clicked-.5-5.,1.,10.-(clicked+.5),10.],'33 free click')
    same(group(docs['35']),group(docs['33']),'35 numeric source position preserves complete recipe and member IDs')
    same(set(docs['35']['uids']),set(docs['33']['uids']),'35 all UUIDs stable')
    assert_chain(docs['35'],[(4.,2.),(7.,1.)],[3.,2.,1.5,1.,2.5,10.],'35 numeric7m')
    same(sans(group(docs['40']),'rowPlaces'),group(docs['35']),'40 only overall manual row placement changes')
    near(group(docs['40'])['rowPlaces'][0]['at']['y'],.1+3.,'40 actual face plus3m')
    same(group(docs['40'])['rowPlaces'][0]['row'],'overall','40 targets overall row')
    for member,entity in members(docs['35'],'openings'):
        same(objects(docs['40'])[member['uid']],entity,'40 opening rows untouched')
    assert_chain(docs['40'],[(4.,2.),(7.,1.)],[3.,2.,1.5,1.,2.5,10.],'40 overall3m')
    first = group(docs['40'])['members'][0]
    same(set(docs['40']['uids'])-set(docs['41']['uids']),{first['uid']},'41 exactly first opening member UUID removed')
    same(set(docs['41']['uids'])-set(docs['40']['uids']),set(),'41 no new object UUID')
    same(len(group(docs['41'])['members']),5,'41 five active members')
    same(group(docs['41'])['suppressed'],[sans(first,'uid')],'41 exact semantic source pair suppressed')
    for uid,entity in objects(docs['41']).items():
        same(entity,objects(docs['40'])[uid],'41 surviving object unchanged')
    restored = set(docs['44']['uids'])-set(docs['41']['uids'])
    same(len(restored),1,'44 exactly one new object UUID')
    check(first['uid'] not in restored,'44 new identity does not reuse deleted child')
    same(group(docs['44']).get('suppressed',[]),[],'44 clears explicit suppression')
    same(sans(group(docs['44']),'members'),sans(group(docs['40']),'members'),'44 complete recipe and custom row restored')
    for uid,entity in objects(docs['41']).items():
        same(objects(docs['44'])[uid],entity,'44 all existing geometry/UUIDs untouched')
    assert_chain(docs['44'],[(4.,2.),(7.,1.)],[3.,2.,1.5,1.,2.5,10.],'44 restore')
    same(sans(group(docs['46']),'members','openings'),sans(group(docs['44']),'members','openings'),'46 only mode/membership changes')
    same(objects(docs['46'])[overall],objects(docs['44'])[overall],'46 overall UUID and custom row retained')
    assert_chain(docs['46'],[(4.,2.),(7.,1.)],[4.,3.,3.,10.],'46 centers',centers=True)
    same(docs['48'],docs['46'],'48 reopened complete document equals46')
    same(sans(docs['51'],'sheets'),docs['48'],'51 sheet creation leaves every original model field untouched')
    book = docs['51']['sheets']['book']
    same(book['schema'],'kentos.sheet/1','51 native sheet schema')
    same(len(book['sheets']),1,'51 one sheet')
    sheet = book['sheets'][0]
    same(sheet['page']['paper'],'a3','51 A3 paper')
    same(sheet['page']['orientation'],'landscape','51 landscape')
    same(sheet['page']['size'],{'width':420000,'height':297000},'51 exact micrometre page size')
    maps = [item for item in sheet['items'] if item['kind']['type']=='map']
    same(len(maps),1,'51 one map viewport')
    same(maps[0]['kind']['view']['scale'],50,'51 viewport scale1:50')
    same(docs['58'],docs['23'],'58 Escape leaves full original source document unchanged')
    fixed = docs['64']
    same(len(fixed['entities']),7,'64 exactly one four-member group added')
    for uid,entity in objects(docs['58']).items():
        same(objects(fixed)[uid],entity,'64 original source UUID and every field unchanged')
    same(sans(fixed,'entities','uids','dimensionSets'),sans(docs['58'],'entities','uids','dimensionSets'),'64 original settings/project fields unchanged')
    near(group(fixed)['at']['y'],1.1,'64 physical face .1 plus explicit100cm distance')
    same(group(fixed).get('rowPlaces',[]),[],'64 no manual row override masks numeric distance')
    same(group(fixed)['layerId'],'olcu','64 explicit target layer')
    target = [layer for layer in fixed['layers'] if layer['id']=='olcu']
    same(len(target),1,'64 target layer exists')
    same(target[0]['name'],'Ölçü','64 visible target layer name')
    assert_chain(fixed,[(4.,1.)],[3.5,1.,5.5,10.],'64 explicit100cm')
    for index,(_,dimension) in enumerate(members(fixed)):
        near(dimension['offset'],1.5 if index==3 else 1.,'64 fixed offset independent of pointer')
        near(dimension['a']['y']+dimension['offset'],1.6 if index==3 else 1.1,'64 world row y')
    same(docs['65'],docs['58'],'65 real undo full document equals original58')
    same(docs['66'],fixed,'66 real redo full document equals64')
    same(docs['70'],docs['66'],'70 fixed-to-cursor preview plus twoEsc leaves full saved document unchanged')
    original_fixture = BASE/'models/73-live2-style-100-close.kcad'
    same(sha(original_fixture),proof2['fixtureSha256'],'73 original100 fixture matches frozen build provenance')
    original100 = reader.read(original_fixture.read_bytes())[1]
    same(docs['73'],original100,'73 full document equals original100 style fixture')
    near(docs['73']['settings']['plotScale'],100.,'73 original plotScale100 unchanged')
    same(sans(docs['76'],'settings'),sans(docs['73'],'settings'),'76 only project settings may differ')
    same(sans(docs['76']['settings'],'plotScale'),sans(docs['73']['settings'],'plotScale'),'76 only plotScale changes inside settings')
    near(docs['76']['settings']['plotScale'],200.,'76 explicit plotScale200')
    same(docs['78'],docs['76'],'78 unsupported multi-segment source refusal writes nothing')
    same(sans(docs['80'],'sheets'),docs['76'],'80 only sheet book is added')
    style_book = docs['80']['sheets']['book']
    same(style_book['schema'],'kentos.sheet/1','80 standard sheet book')
    same(len(style_book['sheets']),1,'80 one new sheet')
    style_sheet = style_book['sheets'][0]
    same(style_sheet['page']['paper'],'a3','80 A3 paper')
    same(style_sheet['page']['orientation'],'landscape','80 landscape')
    style_maps = [item for item in style_sheet['items'] if item['kind']['type']=='map']
    same(len(style_maps),1,'80 one map viewport')
    same(style_maps[0]['kind']['view']['scale'],200,'80 viewport1:200')
    same(docs['83'],docs['80'],'83 sheet-model navigation changes no saved model field')
    same(docs['88'].get('dimensionSets',[]),[],'88 detach removes the ownership recipe')
    same(sans(docs['88'],'dimensionSets'),sans(docs['64'],'dimensionSets'),'88 only dimensionSets removed; all seven objects, UUIDs, refs and geometry unchanged')
    same(docs['89'],docs['64'],'89 detach undo restores full64 document')
    # Expected from delete/opening_follow/dimension_sets source semantics:
    # remove only the host; freeze existing geometry and add explicit breaks.
    expected_objects = deepcopy(objects(docs['64']))
    del expected_objects[wall_uid]
    expected_objects[opening_uid]['broken'] = 'hostGone'
    for entity in expected_objects.values():
        if entity['kind'] == 'dimension':
            for name in ['a','b']:
                ref = entity['refs'][name]
                ref['broken'] = 'object_gone' if ref['uid'] == wall_uid else 'no_such_place'
    same(len(docs['90']['entities']),6,'90 only host removed, six ordinary objects remain')
    same(objects(docs['90']),expected_objects,'90 only expected missing-source flags change; every cached geometry/UUID/other field preserved')
    expected_recipe = deepcopy(group(docs['64']))
    expected_recipe['broken'] = 'hostGone'
    same(group(docs['90']),expected_recipe,'90 recipe membership and semantic sources retained with hostGone')
    same(sans(docs['90'],'entities','uids','dimensionSets'),sans(docs['64'],'entities','uids','dimensionSets'),'90 all project/settings fields preserved')
    same(docs['92'],docs['64'],'92 host-delete undo restores complete64 document')
    same(docs['96'],docs['58'],'96 whole group deletion preserves the full original source document')
    same(docs['97'],docs['64'],'97 whole group deletion undo restores full64 document')
    same(docs['125'],docs['73'],'125 menu/schedule/keyboard/settings navigation preserves original100 document')
    same(docs['128'],docs['125'],'128 style form cancellation preserves full125')
    renamed = deepcopy(docs['125'])
    choices = [style for style in renamed['settings']['wallStyles'] if style['id']=='kentos-dis-duvar-25']
    same(len(choices),1,'130 one explicit source style')
    choices[0]['name']='Dış duvar 25 QA'
    same(renamed,json.loads((BASE/'observations/129-expected-style-rename.json').read_text()),'130 expected rename was specified before save')
    same(docs['130'],renamed,'130 only explicit style name changes; no Undo claim')
    same(docs['137'],docs['125'],'137 manual name restoration returns full125; NOT an Undo claim')
    same(docs['141'],docs['125'],'141 style navigation returns full125')
    live3=docs['153']
    same(len(live3['entities']),7,'153 one group adds exactly four dimensions')
    same(sans(live3,'entities','uids','dimensionSets'),sans(docs['58'],'entities','uids','dimensionSets'),'153 unrelated project metadata exact')
    for uid,entity in objects(docs['58']).items():
        same(objects(live3)[uid],entity,'153 source and unrelated UUID/every field exact')
    same(group(live3)['layerId'],'olcu','153 explicit target uses wire layerId')
    near(group(live3)['at']['y'],1.1,'153 explicit100cm from physical left face')
    same(group(live3).get('rowPlaces',[]),[],'153 no row override hides default gap')
    same(group(live3).get('suppressed',[]),[],'153 no hidden interval')
    same(group(live3)['rows'],['openings','overall'],'153 both requested rows')
    assert_chain(live3,[(4.,1.)],[3.5,1.,5.5,10.],'153 live3 default7.5mm',gap_mm=7.5)
    for index,(_,dimension) in enumerate(members(live3)):
        near(dimension['offset'],1.375 if index==3 else 1.,'153 independent100cm/7.5mm offsets')
    same(docs['154'],docs['58'],'154 single toolbarUndo restores full seed58')
    same(docs['155'],live3,'155 single toolbarRedo restores full153 document')
    same(docs['156'],docs['155'],'156 fresh native process opens full155 including every UUID/ref/recipe')
    same(docs['164'],docs['156'],'164 wide13 builtin-style draft cancel and font change preserve full156')
    same(docs['183'],docs['125'],'183 resumed normal zoom roundtrip preserves full original100 document')
    # Reconstruct the pre-GUI stress fixture from the old, independently retained
    # single-style document. Never infer its expected content from the GUI save.
    long_seed = deepcopy(docs['125'])
    original_style = long_seed['settings']['wallStyles'][0]
    long_style = deepcopy(original_style)
    long_style.update(id='qa-long-sixteen', name='QA 16 bileşenli duvar')
    long_style['components'].extend({
        'name':f'Ek katman {index:02}', 'detail':'detailed',
        'material':'Test malzemesi', 'thickness':0.002,
    } for index in range(1,13))
    list_styles = []
    for index in range(1,25):
        style = deepcopy(original_style)
        style.update(id=f'qa-list-{index:02}', name=f'QA liste stili {index:02}')
        list_styles.append(style)
    long_seed['settings']['wallStyles'] = [long_style,*list_styles,original_style]
    same(docs['165-seed'],long_seed,'165 pre-GUI long seed is exactly independently specified26styles/16components')
    same(docs['176'],long_seed,'176 inner/outer scroll and last-component draft cancel preserve entire pre-GUI seed')
    for key in ('165-seed','176'):
        same(sha(BASE/'models'/MODELS[key]),'89b83eeb2140ebc5fd76c6fecbd354c331020f06fdb8d42e147d7c77ce8b2aad',key+' exact pre-GUI byte identity')


def image_info(data):
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        return ("image/png", *struct.unpack(">II", data[16:24]))
    check(data.startswith(b"\xff\xd8"),"supported image signature")
    i = 2
    while i < len(data):
        check(data[i] == 255,"JPEG marker")
        while data[i] == 255:
            i += 1
        marker = data[i]
        i += 1
        length = struct.unpack_from(">H",data,i)[0]
        if marker in (192,193,194):
            height,width = struct.unpack_from(">HH",data,i+3)
            return "image/jpeg",width,height
        i += length
    raise AssertionError("JPEG dimensions missing")


def stored_file(root, relative):
    path = Path(relative)
    check(not path.is_absolute() and '..' not in path.parts,"relative evidence path")
    return root/path


def verified_bytes(path, record):
    check(path.is_file(),"required file exists: " + str(path))
    data = path.read_bytes()
    same(len(data),record['bytes'],"byte size: " + str(path))
    same(hashlib.sha256(data).hexdigest(),record['sha256'],"SHA256: " + str(path))
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    scope = parser.add_mutually_exclusive_group()
    scope.add_argument('--models-only',action='store_true',help='Explicit partial model verification; no image/binary acceptance.')
    scope.add_argument('--metadata',action='store_true',help='Manifest and availability only; no artifact bytes verified.')
    parser.add_argument('--evidence-root',type=Path,help='Directory containing the ignored raw screenshots/PDF.')
    parser.add_argument('--check-current-sources',action='store_true',help='Strictly require every captured final live-3 source hash to match this checkout.')
    parser.add_argument('--report',type=Path,help='Optional explicit output JSON; never written by default.')
    args = parser.parse_args()
    if args.metadata and args.check_current_sources:
        parser.error('--metadata never verifies source bytes; use a verification mode with --check-current-sources')
    manifest = json.loads((BASE/'evidence.json').read_text())
    same(manifest['format'],'kentos.architecture-wall-dimensions-evidence','manifest format')
    same(manifest['version'],1,'manifest version')
    same({r['path'] for r in manifest['models']},{'models/'+name for name in MODELS.values()},'exact complete model inventory')
    raw_root = args.evidence_root or ROOT/manifest['defaultRawEvidenceRoot']
    missing_images = [r['path'] for r in manifest['images'] if not stored_file(raw_root,r['path']).is_file()]
    missing_pdf = [r['path'] for r in manifest['pdfs'] if not stored_file(raw_root,r['path']).is_file()]
    missing_logs = [r['path'] for r in manifest.get('testLogs',[]) if not stored_file(raw_root,r['path']).is_file()]
    missing_binaries = [r['binaryRelativeToRepo'] for r in manifest['builds'] if not stored_file(ROOT,r['binaryRelativeToRepo']).is_file()]
    result = {'verifierVersion':VERIFIER_VERSION,'acceptanceStatus':manifest['status'],
              'featureAccepted':manifest['featureAccepted'], 'wholeArchitectureAccepted':manifest['wholeArchitectureAccepted'],
              'acceptanceGates':manifest['acceptanceGates'], 'verifiedModels':0,'decodedSupplementalModels':0,'verifiedImages':0,'verifiedPdfs':0,'verifiedBinaries':0,'verifiedTestLogs':0,
              'missingImages':len(missing_images),'missingPdfs':len(missing_pdf),'missingBinaries':len(missing_binaries),'missingTestLogs':len(missing_logs),
              'missingImageNames':missing_images,'missingPdfNames':missing_pdf,'missingBinaryNames':missing_binaries,'missingTestLogNames':missing_logs}
    if args.metadata:
        result['status']='metadata-only'
        # Availability is not verification; no image/model/binary bytes were read.
        code=0
    else:
        format_record = manifest['formatAudit']
        format_scope = json.loads(verified_bytes(stored_file(BASE,format_record['path']),format_record))
        latest_format = format_scope.get('latestReadOnlyAudit')
        if latest_format:
            audit = json.loads(verified_bytes(stored_file(BASE,latest_format['path']),latest_format))
            same(audit['counts'],latest_format['counts'],'latest formatting scope counts')
            same(sum(len(f['newOrChangedFormatHunks']) for f in audit['files']),36,'preserved36 residual formatting hunks')
        summary_record = manifest['testRunSummary']
        summaries = json.loads(verified_bytes(stored_file(BASE,summary_record['path']),summary_record))
        for summary in summaries:
            same(sum(s['passed'] for s in summary['suites']),summary['totals']['passed'],'test summary passed total')
            same(sum(s['ignored'] for s in summary['suites']),summary['totals']['ignored'],'test summary ignored total')
            same(sum(s['failed'] for s in summary['suites']),summary['totals']['failed'],'test summary failed total')
            same(len(summary['suites']),summary['suiteCount'],'test summary suite count')
        builds={}
        for record in manifest['builds']:
            data=verified_bytes(stored_file(BASE,record['path']),record)
            proof=json.loads(data)
            same(proof['sha256'],record['binarySha256'],'frozen build SHA identity')
            builds[record['id']]=proof
        same(builds['live-1']['sha256'],'496758b7624dd7026507938fc71fdb247d506f284c3fdd6becf21951cfd6a03c','fixed live1 identity')
        same(builds['live-2']['sha256'],'748a754e019f6cd7c7db2756f9601a886d47798986f82829792af69525578533','fixed live2 identity')
        same(builds['live-3']['sha256'],'65a2fd393468dbb6bfb2a4e526cee97f092aa24cd1a6498119965ca33d379406','fixed final live3 identity')
        same(manifest['currentSourceBuild'],'live-3','current source checks use final GUI build')
        spec=importlib.util.spec_from_file_location('independent_wall_dimension_kcad',READER)
        reader=importlib.util.module_from_spec(spec)
        spec.loader.exec_module(reader)
        by_name={r['path']:r for r in manifest['models']}
        docs={key:reader.read(verified_bytes(BASE/'models'/name,by_name['models/'+name]))[1] for key,name in MODELS.items()}
        verify_models(docs,reader,builds['live-2'])
        for record in manifest.get('supplementalModels',[]):
            check(record['geometryAccepted'] is False,'supplemental snapshot is not an asserted geometry case')
            reader.read(verified_bytes(stored_file(BASE,record['path']),record))
            result['decodedSupplementalModels']+=1
        for record in manifest.get('observations',[]):
            verified_bytes(stored_file(BASE,record['path']),record)
        result['verifiedModels']=len(docs)
        result['independentModelChecks']=len(checks)
        result['independentReaderSha256']=sha(READER)
        result['verifierSha256']=sha(Path(__file__))
        if args.models_only:
            result['status']='models-only'
            code=0
        else:
            for record in manifest['images']:
                path=stored_file(raw_root,record['path'])
                if not path.is_file():continue
                data=verified_bytes(path,record)
                same(image_info(data),(record['mimeType'],record['width'],record['height']),'actual image format and dimensions')
                result['verifiedImages']+=1
            for record in manifest['pdfs']:
                path=stored_file(raw_root,record['path'])
                if not path.is_file():continue
                data=verified_bytes(path,record)
                check(data.startswith(b'%PDF-'),'PDF signature')
                result['verifiedPdfs']+=1
            for record in manifest.get('testLogs',[]):
                path=stored_file(raw_root,record['path'])
                if not path.is_file():continue
                verified_bytes(path,record)
                result['verifiedTestLogs']+=1
            for record in manifest['builds']:
                path=stored_file(ROOT,record['binaryRelativeToRepo'])
                if not path.is_file():continue
                same(sha(path),record['binarySha256'],'actual frozen binary bytes')
                result['verifiedBinaries']+=1
            code=int(bool(missing_images or missing_pdf or missing_binaries or missing_logs))
            result['status']='incomplete-evidence' if code else 'evidence-verified-tests-pending'
        if args.check_current_sources:
            captured=builds[manifest['currentSourceBuild']]['sourceFiles']
            result['checkedCurrentSourceBuild']=manifest['currentSourceBuild']
            drift=[name for name,digest in captured.items() if not (ROOT/name).is_file() or sha(ROOT/name)!=digest]
            result['checkedCurrentSources']=len(captured)
            result['sourceDrift']=drift
            if drift:
                result['status']='source-drift'
                code=1
    if args.report:
        args.report.write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps(result,indent=2,ensure_ascii=False))
    return code


if __name__=='__main__':
    try:
        raise SystemExit(main())
    except (AssertionError,OSError,ValueError,KeyError) as error:
        print(json.dumps({'status':'failed','message':str(error),'featureAccepted':False},ensure_ascii=False))
        raise SystemExit(1)
