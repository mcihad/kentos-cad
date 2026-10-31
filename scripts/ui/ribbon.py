#!/usr/bin/env python3
"""The desktop's ribbon (docs/adr/0622 §3–§4): each project type's tabs and the
tabs a selection opens by its objects' kinds, written to catalog/v1/ribbon.json.

The ribbon is kept here as a short Python table and written as JSON for the
desktop to embed (apps/desktop/src/catalog.rs). A split button by key
({"split": "circle"}) takes the inventory's split with that key, its methods
too. Commands are the catalog's ids; İşlemler's tools appear only in the
contextual tabs, the toolbox has every one.

    python3 scripts/ui/ribbon.py          # writes catalog/v1/ribbon.json
    python3 scripts/ui/ribbon.py --check  # fails when the file is not current
"""
import json, sys
from pathlib import Path

def C(i, size='small', works=(), modes=()):
    c = {"command": i, "size": size}
    if works: c["works"] = list(works)
    if modes: c["modes"] = list(modes)
    return c
def L(i, works=(), modes=()): return C(i, 'large', works, modes)
def S(i, works=(), modes=()): return C(i, 'small', works, modes)
# A seldom used command of a panel's ▾ only some CAD works (or one project type) have (docs/adr/0802).
def W(i, works=(), modes=()):
    w = {"command": i}
    if works: w["works"] = list(works)
    if modes: w["modes"] = list(modes)
    return w
def SP(key, size='small', entries=None):
    # `entries`: the split's entries only some CAD works show, by their labels (docs/adr/0802): {"Semt": MAP}.
    s = {"split": key, "size": size}
    if entries: s["entryWorks"] = {label: list(w) for label, w in entries.items()}
    return s
# Boşluk, Niş, Baca: the openings that are no door or window (docs/adr/0719).
OPENINGS = {"key": "archOpenings", "size": "small", "split": [
    {"command": "tool.opening", "title": "Boşluk", "label": "Boşluk"},
    {"command": "tool.niche", "title": "Boşluk", "label": "Niş"},
    {"command": "tool.flue", "title": "Boşluk", "label": "Baca"},
]}
def M(label, blocks, size='small', checklist=False, works=(), modes=()):
    # A block is (title, ids), (title, ids, works) or (title, ids, works, modes): a block only some CAD works, or one
    # project type, have (docs/adr/0802). A menu that is no checklist shows its blocks' commands as one list.
    def block(b):
        out = {"label": b[0], "items": b[1]}
        if len(b) > 2 and b[2]: out["works"] = list(b[2])
        if len(b) > 3 and b[3]: out["modes"] = list(b[3])
        return out
    m = {"menu": label, "size": size, "blocks": [block(b) for b in blocks]}
    if checklist: m["checklist"] = True
    if works: m["works"] = list(works)
    if modes: m["modes"] = list(modes)
    return m
def B(name): return {"builtin": name}
def P(label, icon, items, overflow=(), launcher=None, keep=False, works=(), modes=(), labels=None):
    # `labels`: the panel's name in a CAD work that holds other things (docs/adr/0802), by the work.
    p = {"label": label, "icon": icon, "items": items}
    if overflow: p["overflow"] = list(overflow)
    if launcher: p["launcher"] = launcher
    if keep: p["keep"] = True
    if works: p["works"] = list(works)
    if modes: p["modes"] = list(modes)
    if labels: p["workLabels"] = dict(labels)
    return p
# A CAD tab, panel, button, menu block or ▾ command only some works show (docs/adr/0801, 0802): `works` names them
# (architecture, map); none: every work, and a CAD project not asked its work shows them all. A module's program
# offers only what its works show, and what MODULES gives it (docs/adr/0802): Komut ara, the command line, the
# shortcuts and the ribbon's own run find nothing else.
def T(i, label, panels, works=()):
    t = {"id": i, "label": label, "panels": panels}
    if works: t["works"] = list(works)
    return t
MAP = ["map"]
ARCHITECTURE = ["architecture"]
# A contextual tab's panel, button, menu block or ▾ command one project type alone shows (docs/adr/0802), as `modes`
# names a contextual tab's types: CAD's drafting (Koordinat yaz, Km yaz, ölçekli açıklama) a CBS selection does not
# offer, and CBS's analysis (mekânsal istatistik, uygunluk, uzaktan algılama) no CAD work offers.
IN_CAD = ["cad"]
IN_GIS = ["gis"]
# Mesafe ölç, Alan hesapla, Koordinat oku, Açı ölç and Bilgi: any drawing's queries (AutoCAD's MEASUREGEOM, ID and
# LIST). A map's Ölçme starts with them. A building has no surveying (docs/adr/0802): KentOS Mimari keeps them in
# Görünüm › Sorgu as one button, the last used on it, as AutoCAD's Home › Utilities › Measure. Giriş is full: at
# 1440 px even one more panel leaves buttons without their names (ADR 0622 §3), Görünüm has room for one button.
QUERY = [S("tool.measure"), S("tool.area"), S("crs.query"), S("tool.measureAngle"), S("tool.identify")]
QUERY_SPLIT = {"key": "archQuery", "size": "large", "split": [
    {"command": "tool.measure", "title": "Mesafe ölç", "label": "Mesafe ölç"},
    {"command": "tool.area", "title": "Alan hesapla", "label": "Alan hesapla"},
    {"command": "crs.query", "title": "Koordinat oku", "label": "Koordinat oku"},
    {"command": "tool.measureAngle", "title": "Açı ölç", "label": "Açı ölç"},
    {"command": "tool.identify", "title": "Bilgi", "label": "Bilgi"},
]}
# Ölçülendirme ▾: its Semt, a bearing (a survey measure, docs/adr/0147), is a map's (docs/adr/0802).
def DIMENSION(size): return SP("dimension", size, entries={"Semt": MAP})
def toTab(tab, title): return {"tab": tab, "title": title}
def toCmd(cmd, title): return {"command": cmd, "title": title}

SELECT_KINDS = ["edit.selectFilter.point", "edit.selectFilter.line", "edit.selectFilter.polyline",
    "edit.selectFilter.polygon", "edit.selectFilter.circle", "edit.selectFilter.arc", "edit.selectFilter.ellipse",
    "edit.selectFilter.spline", "edit.selectFilter.xline", "edit.selectFilter.ray", "edit.selectFilter.text",
    "edit.selectFilter.dimension", "edit.selectFilter.hatch", "edit.selectFilter.insert", "edit.selectFilter.leader",
    "edit.selectFilter.table", "edit.selectFilter.image"]
ARCH_KINDS = ["edit.selectFilter.wall", "edit.selectFilter.opening", "edit.selectFilter.space",
    "edit.selectFilter.grid", "edit.selectFilter.column", "edit.selectFilter.stair", "edit.selectFilter.ramp",
    "edit.selectFilter.tag", "edit.selectFilter.viewmark", "edit.selectFilter.slab"]
def select_filter(cad):
    # The status bar's Süzgeç lists the kinds; the drop-down is the same list. The architectural kinds are a building's
    # (docs/adr/0802): KentOS Mimari's, never KentOS Harita's or a CBS project's.
    arch = [("Mimari nesneler", ARCH_KINDS, ARCHITECTURE)] if cad else []
    return M("Seçim süzgeci", [("", ["edit.selectFilter"]), ("Seçilebilir türler", SELECT_KINDS)] + arch
        + [("", ["edit.selectFilterAll", "edit.selectFilterNone"])], checklist=True)

BASEMAPS = M("Altlık", [
    ("OpenStreetMap", ["basemap.osmStandard", "basemap.osmTopo", "basemap.osmHot", "basemap.cyclosm"]),
    ("Vektör altlıklar", ["basemap.ofmLiberty", "basemap.ofmBright", "basemap.ofmPositron"]),
    ("Uydu ve hava fotoğrafı", ["basemap.esriImagery"]),
    ("Türkiye", ["basemap.hgmHarita", "basemap.hgmOrtofoto"]),
    ("Google Haritalar", ["basemap.googleRoadmap", "basemap.googleSatellite", "basemap.googleTerrain", "basemap.googleHybrid"]),
    ("Anahtarlı servisler", ["basemap.maptilerStreets", "basemap.maptilerSatellite"]),
    ("", ["basemap.remove"])], size='large')

# Geographic data and survey points come and go with a map (docs/adr/0802); a building's exchange is drawings.
IMPORT_CAD = M("İçe aktar", [
    ("Çizim", ["file.import.dwg", "file.import.dxf", "file.import.ncz", "file.import.pdf"]),
    ("Coğrafi veri", ["file.import.shp", "file.import.geojson", "file.import.kml", "file.import.gpkg", "file.import.csv"],
     MAP),
    ("Noktalar", ["file.import.ncn", "file.import.gnss", "file.import.photos"], MAP)], size='large')
EXPORT = M("Dışa aktar", [
    ("Çizim", ["file.export.dwg", "file.export.dxf"]),
    ("Coğrafi veri", ["file.export.shp", "file.export.geojson", "file.export.kml", "file.export.gpx",
                      "file.export.gpkg", "file.export.fgb", "file.export.parquet", "file.export.gml",
                      "file.export.csv"], MAP),
    ("Noktalar", ["file.export.ncn", "file.export.xyz"], MAP)], size='large')
# A CBS layer holds one kind of object (docs/adr/0622 §5): Yeni katman ▾ asks which; Karışık is any kind.
NEW_LAYER = M("Yeni katman", [("", ["layer.new"]),
    ("Türüyle", ["layer.newPoint", "layer.newLine", "layer.newArea", "layer.newAnnotation"])])

ADD_DATA = M("Veri ekle", [
    ("Vektör", ["file.import.gpkg", "file.import.shp", "file.import.geojson", "file.import.kml",
                "file.import.csv", "file.import.fgb", "file.import.parquet", "file.import.gml"]),
    ("Raster ve bulut", ["raster.add", "mesh.add", "pointcloud.add"]),
    ("Çizim", ["file.import.dwg", "file.import.dxf", "file.import.ncz", "file.import.pdf"]),
    ("Noktalar", ["file.import.ncn", "file.import.gnss", "file.import.photos"])], size='large')

LAYER_OVERFLOW = ["tool.layerOff", "tool.layerIsolate", "layer.unisolate", "tool.layerLock", "tool.layerFreeze",
    "layer.thawAll", "tool.layerMakeActive", "tool.layerMatch", "tool.copyToLayer", "layer.merge", "layer.states",
    "layer.stateSave", "layer.translate", "layer.purge", "layer.list"]

def file_tab(cad):
    exchange = [IMPORT_CAD, EXPORT, S("file.takeFrom"), S("file.saveSelection")] if cad else \
               [EXPORT, S("file.takeFrom"), S("file.saveSelection")]
    return T("file", "Dosya", [
        P("Proje", "fileNew", [L("file.new"), L("file.open"), S("file.start")]),
        P("Kaydet", "save", [L("file.save"), S("file.saveAs"), S("file.package")]),
        P("Çizimler", "docTabOpen", [S("file.openTab"), S("file.nextTab"), S("file.closeTab")]),
        P("Bulut", "cloud", [L("cloud.open"), L("cloud.upload"), S("cloud.share"), S("cloud.history"), S("cloud.conflicts")],
          overflow=["cloud.uploadFile", "cloud.openNewest", "cloud.rename", "cloud.delete", "server.check"]),
        P("Alışveriş", "import", exchange),
        P("Ayarlar", "folder", [L("file.settings"), L("tools.options")]),
    ])

def survey_tab(cad):
    if not cad:
        # The survey computations (karne, poligon, kutupsal alım, kestirme, ağ dengelemesi, aplikasyon) are KentOS
        # Harita's (docs/adr/0802): a CBS project keeps its points, its recorded measures and fitting.
        return T("survey", "Ölçme", [
            P("Noktalar", "point", [L("point.editor"), S("crs.points"), S("file.import.gnss"), S("cogo.check"),
                S("cogo.update")]),
            P("Oturtma", "vectorFit", [S("transform.fit"), S("transform.edgematch"), S("crs.transform")]),
        ])
    # A CAD project's Ölçme is a map's (docs/adr/0801, 0802): KentOS Mimari has no tab of surveying, its queries are
    # Görünüm's Sorgu. A route's station is a map's query.
    return T("survey", "Ölçme", [
    P("Sorgu", "measure", QUERY + [S("tool.stationOffset")]),
    P("Saha", "fieldBook", [L("calc.fieldbook"), S("file.import.gnss"), S("field.send")]),
    P("Poligon ve alım", "surveyTraverse", [L("calc.traverse"), L("calc.polar"), L("calc.stakeout")]),
    P("Kestirme ve dengeleme", "surveyNetwork", [S("calc.forward"), S("calc.resection"), S("calc.network"), S("calc.levelNetwork")]),
    P("Noktalar", "point", [L("point.editor"), S("crs.points"), S("cogo.check"), S("cogo.update")]),
    P("Oturtma", "vectorFit", [S("transform.fit"), S("transform.edgematch"), S("crs.transform")]),
    ], works=MAP)

def view_tab(cad):
    panels = [
        P("Görünümler", "namedViews", [L("view.namedViews"), S("view.split"), S("view.splitAdd"), S("view.splitLink"), S("view.swipe"),
          S("tool.viewTwist"), S("view.northUp")]),
        P("Koordinatlar", "ucsWorld", [S("ucs.world"), S("ucs.previous"), S("tool.ucsObject")],
          overflow=["tool.ucsTwoPoints", "tool.ucsOrigin", "tool.ucsRotate", "tool.ucsNamed", "view.twistUcs"]),
        P("Görünüş", "colorModeColor", [
            M("Tema", [("", ["view.theme.dark", "view.theme.light"])]),
            M("Renk kipi", [("", ["view.colorMode.color", "view.colorMode.mono", "view.colorMode.gray"])]),
            M("Yazıların boyu", [("", ["view.annotationSize.legible", "view.annotationSize.true", "view.annotationSize.screen"])]),
            M("Sembol boyutu", [("", ["view.symbols.plot", "view.symbols.screen"])]),
            S("view.fills"), S("view.areaEdges"), S("view.transparency")],
          overflow=["view.lineWeights"],
          launcher=toCmd("tools.options", "Uygulama ayarları: görünüm")),
        P("Paneller", "panelRight", [S("view.rightPanel"), S("view.bottomPanel"), S("view.coords"),
          S("view.overview"), S("view.magnifier"), S("view.fullscreen")],
          overflow=["view.keyTips"]),
    ]
    if cad:
        # Kat görünümü (docs/adr/0715): which levels the plan shows, and Üst kat / Alt kat (docs/adr/0720 §5); the
        # status bar's Kat menu has them too.
        panels[2]["items"].insert(4, M("Kat görünümü", [("", ["view.levels.all", "view.levels.active", "view.levels.below"]),
                                                       ("Etkin kat", ["view.levels.up", "view.levels.down"])],
                                       works=ARCHITECTURE))
        panels.insert(0, P("Gezinme", "zoomExtents", [L("view.zoomExtents"), S("tool.zoomWindow"), S("view.zoomSelection"),
            S("tool.pan"), S("view.previous"), S("view.next")], overflow=["view.zoomIn", "view.zoomOut", "view.extentCheck"]))
        # A building's queries (docs/adr/0802): KentOS Mimari has no Ölçme.
        panels.append(P("Sorgu", "measure", [QUERY_SPLIT], works=ARCHITECTURE))
    return T("view", "Görünüm", panels)

def output_tab(cad):
    panels = [
        # A level's plan sheets are a building's, an atlas a map's (docs/adr/0802).
        P("Pafta", "sheet", [L("sheet.new"), S("sheet.fromTemplate"), S("sheet.drawingSet")]
          + ([S("sheet.levels", ARCHITECTURE)] if cad else [])),
        P("Yayınla", "print", [L("file.publish.sheets"), S("file.preflight.sheets"),
            S("file.export.atlas", MAP if cad else ())]),
    ]
    if not cad:
        panels.append(P("Lejant", "legend", [L("style.legend")]))
    return T("output", "Çıktı", panels)

# ── CAD ──────────────────────────────────────────────────────────────────
CAD = [
    file_tab(True),
    T("home", "Giriş", [
        P("Çizim", "line", [L("tool.line"), L("tool.polyline"), SP("circle", "large"), SP("arc", "large"),
            SP("rectangle"), S("tool.polygon"), S("tool.point"), S("tool.ellipse"), S("tool.spline")],
          overflow=["tool.nurbs", "tool.xline", "tool.ray", "tool.parallel", "tool.perpIn", "tool.perpOut", "tool.donut",
            "tool.revcloud", "tool.wipeout", "tool.sector", W("tool.rangeRings", MAP), "tool.tangentLine",
            "tool.fourthCorner", "tool.divide", "tool.pointsBetween", "tool.intersectPoint", W("tool.vertexPoints", MAP),
            W("tool.adjoin", MAP), W("tool.boundary", ARCHITECTURE)], keep=True),
        P("Değiştir", "move", [S("tool.move"), S("tool.rotate"), SP("trim"), S("tool.copy"), S("tool.mirror"),
            SP("corner"), S("tool.offset"), S("tool.scale"), S("tool.erase")],
          overflow=["tool.extend", "tool.stretch", "tool.array", "tool.explode", "tool.break", "tool.join",
            "tool.lengthen", "tool.align", "tool.alignDistribute", "tool.reverse", "tool.matchProperties",
            "tool.vertex", "tool.reshape"],
          launcher=toTab("modify", "Tüm araçlar: Değiştir sekmesi"), keep=True),
        P("Açıklama", "text", [L("tool.text"), DIMENSION("lead"), S("tool.leader"), S("tool.hatch"), S("table.insert")],
          launcher=toTab("annotate", "Tüm araçlar: Açıklama sekmesi")),
        P("Katmanlar", "layers", [B("layers")], overflow=LAYER_OVERFLOW,
          launcher=toCmd("style.layerStyle", "Katman stili…")),
        P("Özellikler", "styles", [B("properties")],
          launcher=toCmd("file.settings", "Proje ayarları: birimler, hassasiyet ve çizim ölçeği")),
    ]),
    T("plan", "Plan", [
        P("Yol", "planRoad", [L("tool.planRoad"), S("tool.roadJunctions"), S("tool.medianClose"), S("tool.centerline")]),
        P("Ada ve parsel", "parcel", [L("tool.parcel"), L("tool.boundary"), S("tool.polygonize"), S("tool.adjoin"),
            S("tool.topology")]),
        P("Alan işlemleri", "areaUnion", [S("tool.areaUnion"), S("tool.areaSubtract"), S("tool.areaIntersect"),
            S("tool.areaSplit"), S("tool.partsJoin"), S("tool.partsSplit")],
          overflow=["tool.holeAdd", "tool.holeRemove", "tool.holeFill", "tool.toArea", "tool.toPolyline", "tool.edgeShift"]),
        # A parcel's edge lengths and its sheet of corner numbers, edges and area are a map's writing (docs/adr/0802).
        P("Plan yazıları", "coordinateLabel", [S("tool.coordinateLabel"), S("tool.coordinateVertices"),
            S("tool.stationLabels"), S("map.edgeLengths"), S("tool.spot"), S("tool.vertexPoints")],
          overflow=["processing.model.builtin.parcelSheet"]),
        P("Topoloji", "topologyCheck", [S("topology.check"), S("topology.rules"), S("draft.topology")]),
    ], works=MAP),
    survey_tab(True),
    T("insert", "Ekle", [
        P("Blok", "blockInsert", [L("tool.blockInsert"), S("tool.blockDefine"), S("block.panel"), S("block.library"),
            S("block.edit")],
          overflow=["block.insertFile", "block.attributes", "block.sync", "block.freeze", "block.card", "block.schedule",
            "block.writeFile", "block.redefineFromFile", "block.import", "block.export", "block.purge"]),
        P("Şablonlar", "templates", [B("templates")], launcher=toCmd("template.panel", "Şablonlar paneli")),
        P("Dış başvuru", "xrefAttach", [L("xref.attach"), S("xref.manager"), S("tool.xclip")]),
        # Rasters, basemaps, map services, a database's tables and addresses are a map's, as in CBS (docs/adr/0802); a
        # building keeps pictures and point clouds (a scan), named so. STAC's satellite scenes are CBS's analysis.
        P("Resim ve raster", "imageInsert", [S("tool.imageInsert"), S("raster.add", MAP), S("pointcloud.add")],
          overflow=["tool.imageClip", W("raster.georef", MAP), W("raster.mosaic", MAP), W("raster.gpkgRead", MAP)],
          labels={"architecture": "Resim ve nokta bulutu"}),
        P("Harita", "basemap", [BASEMAPS, S("service.add"), S("service.feed")],
          overflow=["service.info", "service.connections", "source.postgis"], works=MAP),
        # A layer's map tips and actions ({$enlem}) are a map's (docs/adr/0802); Bilgi keeps its plain card.
        P("Ekler", "attachment", [S("attachment.add"), S("attachment.addLink"), S("file.import.photos", MAP)],
          overflow=[W("layer.info", MAP)]),
        P("Adres", "addressSearch", [S("address.search"), S("address.reverse")],
          overflow=["address.data", "address.compare", "address.providers"], works=MAP),
    ]),
    T("annotate", "Açıklama", [
        P("Yazı", "text", [L("tool.text"), S("tool.mtext"), SP("textAlong"), S("tool.placeTextFile"),
            S("text.findReplace"), S("style.textStyles")],
          # A layer's labels are a map's (docs/adr/0802): a building labels its objects with Etiket.
          overflow=["text.fieldInsert", "text.fieldsUpdate", "text.number", "text.combine", "text.split", "text.check",
            "text.fonts", W("tool.labelsToText", MAP), "tool.readable"]),
        P("Ölçü", "dimension", [DIMENSION("lead"), S("tool.dimContinue"), S("tool.dimBaseline"),
            S("tool.quickDimension"), S("style.dimensionStyles")],
          overflow=["tool.centerMark", "tool.dimCenterLine", "tool.dimElevation", "tool.dimensionReassociate",
            "dimension.space", "dimension.break", "dimension.unbreak"]),
        P("Kılavuz", "leader", [L("tool.leader"), S("leader.styles")],
          overflow=["leader.addArm", "leader.removeArm", "leader.align", "leader.collect", "leader.blocks"]),
        P("Tablo", "table", [L("table.insert"), S("table.edit"), S("table.update")],
          overflow=["table.link", "table.blockSchedule"]),
        P("Tarama", "hatch", [L("tool.hatch"), S("tool.hatchSelected"), S("tool.hatchEdit")]),
        # A map sheet's north arrow (docs/adr/0802): KentOS Mimari has it in Belgele › İşaret.
        P("İşaretleme", "revcloud", [S("tool.revcloud"), S("tool.wipeout"), S("tool.northArrow", MAP)]),
        P("Ölçek", "annotationMake", [S("annotation.make"), S("tool.annotativePlace")],
          overflow=["annotation.reset", "annotation.unmake"]),
    ]),
    T("modify", "Değiştir", [
        P("Pano", "paste", [L("edit.paste"), S("edit.cut"), S("edit.copy")],
          overflow=["edit.pasteOriginal", "tool.copyBase", "edit.pasteBlock"]),
        P("Seçim", "select", [SP("select"), S("edit.selectAll"), select_filter(True)],
          overflow=["edit.deselect", "edit.invertSelection", "edit.previousSelection", "edit.cycleSelection",
            "edit.autoSelect"]),
        P("Dönüştür", "move", [L("tool.move"), L("tool.copy"), L("tool.rotate"), S("tool.scale"), S("tool.mirror"),
            S("tool.stretch"), S("tool.align")], overflow=["tool.alignDistribute", "tool.copyBase"]),
        P("Dizi", "array", [SP("array", "large"), S("tool.arrayEdit")]),
        P("Kenar", "offset", [L("tool.offset"), SP("trim", "large"), SP("extend"), S("tool.break"), SP("split"),
            S("tool.lengthen")], overflow=["tool.edgeShift", "tool.continue", "tool.reshape"]),
        P("Köşe", "fillet", [SP("corner", "large"), S("tool.vertex"), S("tool.curveEdit")]),
        P("Nesne", "join", [S("tool.join"), S("tool.explode"), S("tool.reverse"), S("tool.readable"),
            S("tool.matchProperties"), S("tool.simplify")]),
        P("Kot ve sıra", "setElevation", [SP("setElevation"), S("edit.bringToFront"), S("edit.sendToBack")],
          overflow=["tool.drawOrderAbove", "tool.drawOrderUnder", "edit.textsToFront", "edit.hatchesToBack"]),
    ]),
    T("arch", "Mimari", [
        # Döşeme (docs/adr/0722) beside Mahal: the tab keeps its words at 1440 px.
        P("Çiz", "wall", [L("tool.wall"), L("tool.wallRectangle"), S("tool.wallPartition"),
            S("tool.wallFromLines"), S("tool.space"), S("tool.slab"), S("tool.spaceDiscover"), S("tool.spaceSeparator")]),
        # Boşluk, Niş and Baca (docs/adr/0719) share one small split: the tab keeps its words at 1440 px.
        P("Yerleştir", "door", [L("tool.door"), L("tool.window"), OPENINGS,
            S("tool.column"),
            {"key": "archGrids", "size": "small", "split": [
                {"command": "tool.grid", "title": "Aks", "label": "Aks"},
                {"command": "tool.gridSystem", "title": "Aks", "label": "Aks ağı"},
            ]}, {"key": "archStairs", "size": "small", "split": [
                {"command": "tool.stair", "title": "Merdiven", "label": "Merdiven"},
                {"command": "tool.stairCurved", "title": "Merdiven", "label": "Döner merdiven"},
            ]}, S("tool.ramp"), S("tool.casework")]),
        P("Düzenle", "wall", [S("tool.wallJoins"), S("tool.wallSplit"), S("tool.openingMove"),
            S("tool.spaceSplit"), S("tool.spaceRebound"), S("tool.spaceSources"), S("tool.gridNumber"), S("tool.openingNumber"), S("tool.openingArray")],
          overflow=["tool.wallShift", "tool.spaceNumber", "tool.wallJoin", "tool.wallExtend", "tool.wallTrim", "tool.wallFlip", "tool.wallRestyle",
            "tool.openingFlip", "arch.spaceMerge", "tool.spaceFloor", "tool.spaceCeiling"]),
        # Duvar ölçüleri and İç ölçü zinciri (docs/adr/0711) share one large split: the tab keeps its words at 1440 px.
        # İşaret ▾ and Revizyon ▾ (docs/adr/0721) are small splits, Malzeme lejandı with the schedules: the tab keeps
        # its words at 1440 px.
        P("Belgele", "tag", [{"key": "archDimensions", "size": "large", "split": [
                {"command": "tool.wallDimensions", "title": "Ölçü", "label": "Duvar ölçüleri"},
                {"command": "tool.interiorDimensions", "title": "Ölçü", "label": "İç ölçü zinciri"},
            ]}, S("tool.tag"), S("arch.quantities"),
            {"key": "archSchedules", "size": "small", "split": [
                {"command": "tool.scheduleOpenings", "title": "Çizelge", "label": "Kapı ve pencere çizelgesi"},
                {"command": "tool.scheduleSpaces", "title": "Çizelge", "label": "Mahal çizelgesi"},
                {"command": "tool.scheduleFinishes", "title": "Çizelge", "label": "Mahal kaplama çizelgesi"},
                {"command": "tool.scheduleFixtures", "title": "Çizelge", "label": "Donatı çizelgesi"},
                {"command": "tool.scheduleUnits", "title": "Çizelge", "label": "Bölüm çizelgesi"},
                {"command": "tool.materialLegend", "title": "Çizelge", "label": "Malzeme lejandı"},
            ]}, {"key": "archMarks", "size": "small", "split": [
                {"command": "tool.viewSection", "title": "İşaret", "label": "Kesit"},
                {"command": "tool.viewElevation", "title": "İşaret", "label": "Görünüş"},
                {"command": "tool.viewDetail", "title": "İşaret", "label": "Detay"},
                {"command": "tool.northArrow", "title": "İşaret", "label": "Kuzey oku"},
            ]}, {"key": "archRevisions", "size": "small", "split": [
                {"command": "tool.revisionMark", "title": "Revizyon", "label": "Revizyon işaretle"},
                {"command": "arch.revisionList", "title": "Revizyon", "label": "Revizyon listesi"},
            ]}],
          overflow=["tool.tagAll", "arch.tagsCheck", "arch.wallJoinCheck", "arch.viewRefs"]),
        P("Proje", "archSettings", [S("arch.audit"), S("arch.program"), S("arch.settings"), S("arch.wallStyles"), S("arch.template"),
            M("Evre", [("", ["arch.phase.all", "arch.phase.existing", "arch.phase.demolition", "arch.phase.proposed"])])],
          # Kapı ve pencere tipleri (docs/adr/0772) under ▾: the tab keeps its words at 1440 px.
          # Oda modülleri (docs/adr/0775) and Malzemeler (docs/adr/0785) under ▾ too; Tasarım seçenekleri (docs/adr/0788) last.
          # Kırpma çerçevesi (docs/adr/0789 §2) beside the navigator: a view's area edited on the model.
          overflow=["arch.sheetSet", "arch.navigator", "tool.viewCrop", "arch.openingTypes", "arch.materials", "arch.modules",
                    "tool.modulePlace", "arch.options"]),
    ], works=ARCHITECTURE),
    view_tab(True),
    T("manage", "Yönet", [
        P("Stiller", "styles", [L("style.manager"), S("style.layerStyle"), S("lineTypes.manage"),
            S("hatchPatterns.manage"), S("style.legend")],
          overflow=["style.svgEditor", "style.assign", "style.clearSymbol"]),
        # The drawing standard is an office's (Mimari ofis, A-DUVAR …; docs/adr/0312, 0802): KentOS Mimari's alone.
        P("Standart", "standardsCheck", [S("standards.check", ARCHITECTURE), S("standards.apply", ARCHITECTURE),
            S("layer.impact")]),
        P("Temizlik", "cleanup", [S("tool.cleanup"), S("tool.audit"), S("layer.purge")]),
        # A layer's typed fields, attribute table, statistics, forms and sources are a map's data (docs/adr/0802); a
        # building's data is its objects' own. Charts are CBS's (Veri › Tablo).
        P("Veri", "featureTable", [S("data.featureTable", MAP), S("layer.fields", MAP), S("data.statistics", MAP),
            S("data.compare"), S("data.search")],
          overflow=[W("data.forms", MAP), W("data.formEdit", MAP), W("data.relations", MAP), W("data.joinViews", MAP),
            W("data.formAudit", MAP), "data.unmark", W("data.sources", MAP)]),
        # İşlemler's toolbox and models are a map's (docs/adr/0802): a building keeps İfadeyle seç and Konuma göre seç.
        P("Otomasyon", "terminal", [L("python.console"), S("commandline.focus"), S("processing.toolbox", MAP),
            S("processing.newModel", MAP)]),
        P("Kısıtlar", "constraint", [L("tool.constraint"), S("constraint.show"), S("constraint.parameters")],
          overflow=["constraint.relaxNext"]),
        P("GIS", "exportGis", [S("file.export.gis"), S("gis.derivation")], works=MAP),
    ]),
    output_tab(True),
]

# ── CBS ──────────────────────────────────────────────────────────────────
GIS = [
    file_tab(False),
    T("map", "Harita", [
        P("Gezinme", "zoomExtents", [L("view.zoomExtents"), S("tool.pan"), S("tool.zoomWindow"),
            S("view.zoomSelection"), S("view.previous"), S("view.next")],
          overflow=["view.zoomIn", "view.zoomOut", "view.extentCheck"]),
        P("Katman ekle", "layerAdd", [ADD_DATA, BASEMAPS, S("service.add"), S("service.feed"), S("source.postgis")],
          overflow=["service.info", "service.connections", "raster.stac", "raster.mosaic", "raster.gpkgRead"]),
        P("Seçim", "select", [SP("select", "large"), S("processing.run.selection.byExpression"),
            S("processing.run.selection.byLocation"), S("edit.selectAll"), S("edit.deselect"), S("edit.autoSelect"),
            select_filter(False)],
          overflow=["edit.invertSelection", "edit.previousSelection", "edit.cycleSelection"]),
        P("Sorgu", "identify", [L("tool.identify"), S("tool.measure"), S("tool.area"), S("crs.query"),
            S("tool.measureAngle"), S("address.search")], overflow=["tool.stationOffset"]),
        P("Etiket", "layerLabels", [L("layer.labels"), S("tool.labelMove"), S("tool.labelPin"), S("tool.labelHide")],
          overflow=["tool.labelRotate", "view.pinnedLabels", "view.unplacedLabels", "tool.labelsToText"]),
        P("Koordinat sistemi", "crs", [S("crs.set"), S("crs.transform"), S("view.mapCrs")],
          launcher=toCmd("crs.set", "Proje ayarları: koordinat sistemi")),
    ]),
    # Parsel oluştur and Plan yolu are KentOS Harita's drafting (docs/adr/0802): a CBS parcel is a Kapalı alan with
    # its layer's form.
    T("edit", "Düzenle", [
        P("Oluştur", "line", [L("tool.point"), L("tool.polyline"), L("tool.polygon"), S("tool.line"), SP("circle"),
            SP("rectangle"), S("tool.adjoin"), S("tool.text")],
          overflow=["tool.arc", "tool.ellipse", "tool.spline", "tool.vertexPoints", "tool.divide", "tool.pointsBetween",
            "tool.intersectPoint", "tool.centerline", "tool.sector", "tool.rangeRings", "tool.fourthCorner"], keep=True),
        P("Şablonlar", "templates", [B("templates")], launcher=toCmd("template.panel", "Şablonlar paneli")),
        P("Katmanlar", "layers", [B("layers")], overflow=LAYER_OVERFLOW,
          launcher=toCmd("style.layerStyle", "Katman stili…")),
        P("Değiştir", "move", [S("tool.move"), S("tool.rotate"), SP("trim"), S("tool.vertex"), S("tool.reshape"),
            S("tool.break"), S("tool.offset"), S("tool.join"), S("tool.erase")],
          # Kısıt is CAD's (docs/adr/0802): CBS shows neither the marks nor Parametreler.
          overflow=["tool.copy", "tool.scale", "tool.mirror", "tool.stretch", "tool.extend", "tool.explode",
            "tool.reverse", "tool.lengthen", "tool.continue", "tool.edgeShift", "tool.align", "tool.alignDistribute",
            "tool.simplify", "tool.matchProperties", "tool.setElevation", "tool.curveEdit",
            "edit.bringToFront", "edit.sendToBack", "edit.paste", "edit.cut", "edit.copy", "edit.pasteOriginal"],
          keep=True),
        P("Alan işlemleri", "areaUnion", [S("tool.areaUnion"), S("tool.areaSplit"), S("tool.areaSubtract")],
          overflow=["tool.areaIntersect", "tool.boundary", "tool.polygonize", "tool.partsJoin", "tool.partsSplit",
            "tool.holeAdd", "tool.holeRemove", "tool.holeFill", "tool.toArea", "tool.toPolyline"]),
    ]),
    T("data", "Veri", [
        P("Katman", "layerAdd", [NEW_LAYER, S("layer.newGroup"), S("layer.fields"), S("style.layerStyle"),
            S("layer.filter"), S("layer.crs")],
          overflow=["layer.filterFromSelection", "layer.filterClear", "layer.reproject", "layer.showAll",
            "layer.info", "layer.dataset", "file.import.maplibre", "file.export.maplibre"]),
        P("Tablo", "featureTable", [L("data.featureTable"), S("data.statistics"), S("data.chart")],
          overflow=["chart.bar", "chart.pie", "chart.histogram", "chart.box", "chart.scatter", "chart.line"]),
        P("Form", "forms", [S("data.forms"), S("data.formEdit"), S("data.relations")],
          overflow=["data.joinViews", "data.formAudit", "data.tableStyles", "data.subtypes", "data.contingents",
            "data.layerRules", "data.layerIndexes", "data.domains", "data.duplicate"]),
        P("Kaynak", "sourceLinked", [S("data.sources"), S("source.link"), S("source.write"), S("source.read"),
            S("source.status")],
          overflow=["source.reconcile", "source.relink", "source.unlink", "source.writeTogether", "raster.gpkgWrite"]),
        P("Topoloji", "topologyCheck", [S("draft.topology"), S("topology.check"), S("topology.rules"),
            M("Çakışma", [("", ["draft.overlap.allow", "draft.overlap.layer", "draft.overlap.layers"])])],
          overflow=["draft.topologyPoints", "tool.topology"]),
        P("Ekler", "attachment", [S("attachment.add"), S("attachment.addLink"), S("layer.info")]),
        # A project's hatch patterns serve CAD's hatch objects (docs/adr/0802); CBS fills areas by a layer's symbol. A
        # CBS project's standard is its data profile; the drawing standard (Mimari ofis) is KentOS Mimari's.
        P("Standart", "standardsCheck", [S("profile.window")],
          overflow=["lineTypes.manage", "layer.impact"]),
        P("Stil", "styles", [L("style.manager"), S("data.search"), S("data.unmark")],
          overflow=["style.svgEditor", "style.assign", "style.clearSymbol"]),
    ]),
    T("analysis", "Analiz", [
        P("İşlemler", "processing", [L("processing.toolbox"), S("processing.newModel"), S("processing.history")]),
        P("Ağ analizi", "networks", [S("network.manage"), S("tool.netRoute"), S("tool.netServiceArea"),
            S("tool.netTrace")], overflow=["network.subnetworks", "linearRef.eventLinks"]),
        P("Arazi", "contours", [S("tool.spot"), S("pointcloud.region"), S("pointcloud.profile")],
          overflow=["mesh.query"]),
        P("Zaman ve senaryo", "timeSlider", [S("time.slider"), S("time.layer"), S("scenario.create"),
            S("scenario.compare"), S("data.compare")],
          overflow=["time.sensors", "tool.timeVersion", "tool.timeEnd", "time.compare", "scenario.show",
            "scenario.base", "scenario.apply"]),
        P("Adres", "addressSearch", [S("address.reverse"), S("address.data")],
          overflow=["address.compare", "address.providers"]),
        P("Otomasyon", "terminal", [L("python.console"), S("commandline.focus")]),
    ]),
    survey_tab(False),
    view_tab(False),
    output_tab(False),
]

# ── Contextual tabs: the selection's family (docs/adr/0622 §4) ──────────
def pr(i): return "processing.run." + i
SEL = P("Seçim", "select", [S("view.zoomSelection"), S("edit.deselect"), S("edit.invertSelection")],
    overflow=["tool.selectSimilar", "edit.previousSelection", "file.saveSelection"])

def edit_panel(extra=()):
    return P("Düzenle", "move", [L("tool.move"), S("tool.copy"), S("tool.rotate"), S("tool.scale"), S("tool.mirror"),
        S("tool.erase")] + list(extra))

CONTEXTUAL = [
    {"id": "selection", "label": "Seçim", "kinds": ["*"], "panels": [
        P("Seçim", "select", [B("selection")]),
        P("Dönüştür", "move", [L("tool.move"), L("tool.copy"), L("tool.rotate"), S("tool.scale"), S("tool.mirror"),
            S("tool.stretch"), S("tool.align")], overflow=["tool.alignDistribute"]),
        P("Nesne", "join", [S("tool.erase"), S("tool.explode"), S("tool.join"), S("tool.matchProperties"),
            S("tool.copyToLayer"), S("tool.setElevation")]),
        P("Çizim sırası", "bringToFront", [S("edit.bringToFront"), S("edit.sendToBack")]),
        P("Sembol", "symbolAssign", [S("style.assign"), S("style.clearSymbol")]),
        # A layer's filter is a map's (docs/adr/0802).
        P("Seçilenler", "select", [S("tool.selectSimilar"), S("file.saveSelection"),
            S("layer.filterFromSelection", MAP)]),
    ]},
    {"id": "ctx.point", "label": "Nokta", "kinds": ["point"], "panels": [
        SEL,
        edit_panel([S("tool.setElevation"), S("tool.matchProperties")]),
        # Koordinat yaz and Cihaza gönder are KentOS Harita's drafting and survey (docs/adr/0802), not a CBS selection's.
        P("Nokta", "point", [L("point.editor"), S("tool.coordinateLabel", modes=IN_CAD), S("field.send", modes=IN_CAD),
            S("tool.copyToLayer")]),
        # Kernel density, spatial statistics, the proximity tools, Voronoi, the concave hull and the geostatistical
        # surfaces (Kriging, Spline, Doğal komşu) are CBS analysis (docs/adr/0802); a map keeps Delaunay, the bounding
        # geometry, TIN and IDW for its terrain.
        P("İşlemler", "processing", [
            M("Üret", [("", [pr("generate.delaunay")]), ("", [pr("generate.voronoi")], (), IN_GIS),
                ("", [pr("generate.bounding")]), ("", [pr("generate.concaveHull")], (), IN_GIS),
                ("", [pr("convert.pointsToPath")])], size='large'),
            M("İnterpolasyon", [("", [pr("interpolation.idw")]),
                ("", [pr("interpolation.naturalNeighbor"), pr("interpolation.spline"), pr("interpolation.kriging")], (),
                 IN_GIS),
                ("", [pr("interpolation.tin")]), ("", [pr("density.kernel")], (), IN_GIS)], size='large'),
            M("İstatistik", [("Merkez ve dağılım", [pr("stats.meanCenter"), pr("stats.medianCenter"),
                pr("stats.standardDistance"), pr("stats.directionalDistribution")]),
                ("Desen ve küme", [pr("stats.nearestNeighbor"), pr("stats.moransI"), pr("stats.hotSpot"),
                pr("stats.localMoran"), pr("stats.dbscan"), pr("stats.dbscanSweep"), pr("stats.kMeans")])],
              size='large', modes=IN_GIS),
            M("Yakınlık", [("", [pr("proximity.nearest"), pr("proximity.matrix"), pr("proximity.hub"),
                pr("proximity.shortestLine")], (), IN_GIS), ("", [pr("attributes.fromEnclosing")])], size='large'),
        ]),
    ]},
    {"id": "ctx.line", "label": "Çizgi", "kinds": ["line", "polyline", "arc", "spline", "nurbs", "xline", "ray"],
     "panels": [
        SEL,
        edit_panel([S("tool.stretch")]),
        P("Kenar", "offset", [L("tool.offset"), SP("trim"), SP("extend"), S("tool.break"), SP("split"),
            S("tool.lengthen"), S("tool.join"), S("tool.reverse")],
          overflow=["tool.continue", "tool.reshape", "tool.edgeShift"]),
        P("Köşe", "fillet", [SP("corner"), S("tool.vertex"), S("tool.curveEdit"), S("tool.simplify"),
            S("tool.setElevation")]),
        # Survey points at the corners and a route's chainage (Km yaz) are a map's (docs/adr/0802); Km yaz is CAD
        # drafting, a CBS route has Doğrusal referans.
        P("Dönüştür", "conversion", [S("tool.toArea"), S("tool.vertexPoints", MAP), S("tool.divide"),
            S("tool.stationLabels", MAP, IN_CAD), SP("textAlong")]),
        P("İşlemler", "processing", [
            M("Geometri", [("", [pr("geometry.buffer"), pr("geometry.simplify"), pr("geometry.clip"),
                pr("geometry.intersection"), pr("geometry.difference"), pr("geometry.union"),
                pr("geometry.validity"), pr("geometry.repair")])], size='large'),
            # Generalization (Yumuşat) and line density are CBS analysis (docs/adr/0802).
            M("Dönüştür", [("", [pr("convert.linesToPolygons"), pr("convert.substring"), pr("convert.densify"),
                pr("convert.vertices"), pr("convert.toPoints"), pr("sample.alongLines")]),
                ("", [pr("generalize.smooth"), pr("density.line")], (), IN_GIS)], size='large'),
            M("Doğrusal referans", [("", [pr("linearRef.createRoutes"), pr("linearRef.calibrateRoutes"),
                pr("linearRef.routeEvents"), pr("linearRef.locateAlong"), pr("linearRef.overlayEvents"),
                pr("linearRef.extractMeasures"), pr("linearRef.eventDrift")])], size='large'),
        ]),
    ]},
    {"id": "ctx.area", "label": "Alan", "kinds": ["polygon", "circle", "ellipse"], "panels": [
        SEL,
        edit_panel([S("tool.stretch")]),
        P("Kenar ve köşe", "offset", [S("tool.offset"), S("tool.edgeShift"), S("tool.reshape"), S("tool.vertex"),
            SP("corner"), S("tool.setElevation")]),
        P("Alan", "areaUnion", [L("tool.areaUnion"), S("tool.areaSubtract"), S("tool.areaIntersect"),
            S("tool.areaSplit"), S("tool.partsJoin"), S("tool.partsSplit")],
          overflow=["tool.holeAdd", "tool.holeRemove", "tool.holeFill", "tool.toPolyline"]),
        # A parcel's corner coordinates, edge lengths and sheet are a map's (docs/adr/0802); Köşelere koordinat yaz and
        # Çoklu tara are CAD drafting, a CBS area is filled by its layer's symbol: the panel is CAD's.
        P("Yaz ve tara", "coordinateLabel", [S("tool.coordinateVertices", MAP), S("map.edgeLengths", MAP),
            S("tool.hatchSelected")], overflow=[W("processing.model.builtin.parcelSheet", MAP)], modes=IN_CAD),
        # Random points, ST_Subdivide's pieces, the pole of inaccessibility, generalization, neighbours, the nearest,
        # zonal statistics and rasterizing are CBS analysis (docs/adr/0802), not a map's drafting.
        P("İşlemler", "processing", [
            M("Geometri", [("", [pr("geometry.buffer"), pr("geometry.clip"), pr("geometry.intersection"),
                pr("geometry.difference"), pr("geometry.union"), pr("geometry.symDifference"),
                pr("geometry.dissolve"), pr("geometry.simplify"), pr("geometry.validity"), pr("geometry.repair")])],
              size='large'),
            M("Üret", [("", [pr("generate.bounding")]), ("", [pr("generate.pole")], (), IN_GIS),
                ("", [pr("sample.grid"), pr("sample.regular")]),
                ("", [pr("sample.random"), pr("sample.subdivide")], (), IN_GIS),
                ("", [pr("convert.toPoints"), pr("convert.vertices"), pr("convert.polygonsToLines")])], size='large'),
            M("Genelleştir", [("", [pr("generalize.smooth"), pr("generalize.orthogonalize"),
                pr("generalize.building"), pr("generalize.aggregate"), pr("generalize.dropParts"),
                pr("generalize.eliminate"), pr("generalize.coverageSimplify")])], size='large', modes=IN_GIS),
            M("Sorgu", [("", [pr("attributes.fromInside"), pr("attributes.fromEnclosing")]),
                ("", [pr("proximity.neighbors"), pr("proximity.nearest")], (), IN_GIS),
                ("", [pr("statistics.summary")]), ("", [pr("raster.zonalStatistics")], (), IN_GIS),
                ("", [pr("raster.clipByMask")]), ("", [pr("raster.rasterize")], (), IN_GIS)], size='large'),
        ]),
    ]},
    {"id": "ctx.text", "label": "Yazı", "kinds": ["text"], "panels": [
        SEL,
        edit_panel(),
        # The project's text styles are CAD's (docs/adr/0183, 0802): a CBS text takes its layer's look.
        P("Yazı", "text", [S("text.findReplace"), S("tool.readable"), S("text.combine"), S("text.split"),
            S("text.number"), S("style.textStyles", modes=IN_CAD)],
          overflow=["text.fieldInsert", "text.fieldsUpdate", "text.check", "text.fonts"]),
        P("Eğri boyunca", "textAlong", [SP("textAlong")]),
        # Ölçekli açıklama is a sheet's paper scale (docs/adr/0323): CAD's; CBS sizes labels by the view's scale.
        P("Ölçek", "annotationMake", [S("annotation.make"), S("annotation.unmake"), S("tool.annotativePlace")],
          modes=IN_CAD),
    ]},
    # Dimensions, leaders, hatches, tables and pictures are CAD's objects (docs/adr/0802): a CBS selection of them (an
    # imported DXF's) takes Seçim, as blocks do.
    {"id": "ctx.dimension", "label": "Ölçü", "kinds": ["dimension"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Ölçü", "dimension", [S("dimension.space"), S("dimension.break"), S("dimension.unbreak"),
            S("tool.dimensionReassociate"), S("style.dimensionStyles")]),
        P("Ölçek", "annotationMake", [S("annotation.make"), S("annotation.unmake"), S("tool.annotativePlace")]),
    ]},
    {"id": "ctx.leader", "label": "Kılavuz", "kinds": ["leader"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Kılavuz", "leader", [S("leader.addArm"), S("leader.removeArm"), S("leader.align"), S("leader.collect"),
            S("leader.blocks"), S("leader.styles")]),
        P("Ölçek", "annotationMake", [S("annotation.make"), S("annotation.unmake"), S("tool.annotativePlace")]),
    ]},
    {"id": "ctx.hatch", "label": "Tarama", "kinds": ["hatch"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Tarama", "hatch", [L("tool.hatchEdit"), S("edit.hatchesToBack"), S("hatchPatterns.manage")]),
    ]},
    # Blocks are CAD's (docs/adr/0802: “blok GIS içinde olmasa da olur”): a CBS selection of inserts takes Seçim.
    {"id": "ctx.block", "label": "Blok", "kinds": ["insert"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Blok", "blockInsert", [L("block.edit"), S("tool.explode"), S("block.attributes"), S("block.sync"),
            S("block.freeze"), S("block.card"), S("tool.xclip"), S("block.panel")]),
        P("Çizim sırası", "bringToFront", [S("edit.bringToFront"), S("edit.sendToBack")]),
    ]},
    {"id": "ctx.table", "label": "Tablo", "kinds": ["table"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Tablo", "table", [L("table.edit"), S("table.update"), S("table.link"), S("table.blockSchedule")]),
    ]},
    {"id": "ctx.image", "label": "Resim", "kinds": ["image"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Resim", "imageInsert", [L("tool.imageClip"), S("edit.sendToBack"), S("edit.bringToFront")]),
    ]},
    # A raster's look, its fitting and analysis are a map's (docs/adr/0802): KentOS Mimari adds no raster. A map keeps
    # its terrain (eğim, bakı, gölgeli ve renkli kabartma, eş yükselti), a mosaic, a clip and a resampling, and its
    # scanned sheet's tools; the other surface analyses, hydrology, distance and cost, map algebra, raster statistics,
    # suitability, remote sensing and the multidimensional tools are CBS analysis. A map keeps Bant birleştir for its
    # scenes' bands.
    {"id": "ctx.raster", "label": "Raster", "kinds": ["raster"], "panels": [
        SEL,
        P("Raster", "rasterAdd", [L("raster.style"), S("raster.georef"), S(pr("raster.cog")), S("raster.gpkgWrite"),
            S("raster.mosaic")], works=MAP),
        P("Yüzey", "aspect", [
            M("Yüzey analizi", [("", [pr("surface.slope"), pr("surface.aspect"), pr("surface.hillshade"),
                pr("surface.colorRelief"), pr("surface.contours")]),
                ("", [pr("surface.curvature"), pr("surface.ruggedness"), pr("surface.insolation")], (), IN_GIS)],
              size='large'),
            M("Hidroloji", [("", [pr("hydrology.fill"), pr("hydrology.flowDirection"), pr("hydrology.flowAccumulation"),
                pr("hydrology.wetness"), pr("hydrology.pourPoint"), pr("hydrology.watershed"), pr("hydrology.basins"),
                pr("hydrology.streams")])], size='large', modes=IN_GIS),
            M("Uzaklık ve maliyet", [("", [pr("distance.euclidean"), pr("distance.cost"), pr("distance.path"),
                pr("distance.corridor")])], size='large', modes=IN_GIS),
        ], works=MAP),
        P("Raster işlemleri", "clipRaster", [
            M("Hesap", [("", [pr("raster.calculator"), pr("raster.reclassify")], (), IN_GIS),
                ("", [pr("raster.clipByMask"), pr("raster.mosaic"), pr("raster.resample")])], size='large'),
            M("İstatistik", [("", [pr("raster.zonalStatistics"), pr("raster.histogram"), pr("raster.cellStatistics"),
                pr("raster.focalStatistics")])], size='large', modes=IN_GIS),
            M("Vektöre", [("", [pr("raster.toPolygons")]), ("", [pr("raster.toLines"), pr("raster.toPoints")], (), IN_GIS),
                ("", [pr("scan.captureLine"), pr("scan.closeArea"), pr("scan.contourElevations")])], size='large'),
        ], works=MAP),
        P("Uygulamalar", "weightedOverlay", [
            M("Uygunluk", [("", [pr("suitability.weightedOverlay"), pr("suitability.weightedSum"),
                pr("suitability.fuzzyMembership"), pr("suitability.fuzzyOverlay"), pr("suitability.pairwise"),
                pr("suitability.roc")])], size='large', modes=IN_GIS),
            M("Uzaktan algılama", [("", [pr("remote.index"), pr("remote.supervised"), pr("remote.unsupervised"),
                pr("remote.accuracy"), pr("remote.change"), pr("remote.qaMask"), pr("remote.pansharpen")], (), IN_GIS),
                ("", [pr("remote.composite")]), ("", [pr("remote.split")], (), IN_GIS)], size='large'),
            M("Çok boyutlu", [("", [pr("multidim.profile"), pr("multidim.series"), pr("multidim.meshCalculator"),
                "mesh.query"])], size='large', modes=IN_GIS),
        ], works=MAP),
    ]},
    {"id": "ctx.pointcloud", "label": "Nokta bulutu", "kinds": ["pointcloud"], "panels": [
        SEL,
        P("Nokta bulutu", "pointCloudAdd", [L("pointcloud.style"), S("pointcloud.query"), S("pointcloud.profile"),
            S("pointcloud.region"), S("pointcloud.vpcSave")]),
        P("İşlemler", "processing", [L(pr("pointcloud.ground")), S(pr("pointcloud.thin")), S(pr("pointcloud.classify")),
            S(pr("pointcloud.clip")), S(pr("pointcloud.merge")), S(pr("pointcloud.tile")), S(pr("pointcloud.rasterize")),
            S(pr("pointcloud.boundary")), S(pr("pointcloud.areaStats"))]),
    ]},
    {"id": "ctx.wall", "label": "Duvar", "kinds": ["wall"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Duvar", "wall", [L("tool.wallDimensions"), L("view.properties"), S("tool.wallJoins"), S("tool.wallSplit"), S("tool.wallJoin"), S("tool.wallExtend"),
            S("tool.wallTrim"), S("tool.wallFlip"), S("tool.wallRestyle"), S("arch.wallJoinCheck")]),
        P("Açıklık", "door", [S("tool.door"), S("tool.window"), OPENINGS, S("tool.tag")]),
    ]},
    {"id": "ctx.opening", "label": "Açıklık", "kinds": ["opening"], "modes": ["cad"], "panels": [
        SEL,
        P("Düzenle", "move", [S("tool.copy"), S("tool.erase")]),
        P("Açıklık", "door", [L("view.properties"), S("tool.openingMove"), S("tool.openingFlip"), S("tool.openingNumber"), S("tool.openingArray"), S("tool.tag"),
            S("tool.scheduleOpenings")]),
        # Its type (docs/adr/0772 §1, §4): the types' window, the same type selected, the type left, given, taken back.
        P("Tip", "openingTypes", [L("arch.openingTypes"), S("arch.openingSameType"), S("arch.openingDetach"),
            S("arch.openingPush"), S("arch.openingReset")]),
    ]},
    {"id": "ctx.space", "label": "Mahal", "kinds": ["space"], "modes": ["cad"], "panels": [
        SEL,
        P("Düzenle", "move", [S("tool.move"), S("tool.erase")]),
        P("Mahal", "space", [L("view.properties"), S("arch.quantities"), S("tool.spaceSplit"), S("tool.spaceRebound"), S("tool.spaceSources"), S("tool.spaceNumber"), S("arch.spaceMerge"), S("tool.spaceFloor"),
            S("tool.spaceCeiling"), S("tool.tag"), S("tool.scheduleSpaces")]),
    ]},
    # Döşeme (docs/adr/0722 §6): its own fields in Öznitelikler, a hole cut with the tool's Delik, Metraj.
    {"id": "ctx.slab", "label": "Döşeme", "kinds": ["slab"], "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Döşeme", "slab", [L("view.properties"), L("tool.slab"), S("arch.quantities"), S("tool.space")]),
    ]},
    {"id": "ctx.arch", "label": "Mimari nesne", "kinds": ["grid", "column", "stair", "ramp", "tag", "viewmark"],
     "modes": ["cad"], "panels": [
        SEL,
        edit_panel(),
        P("Mimari", "archGrid", [S("tool.gridNumber"), S("arch.tagsCheck"), S("arch.viewRefs")]),
    ]},
]

# In a CAD project the contextual tabs' processing tools (İşlemler) and a point's survey panel are a map's
# (docs/adr/0802): a building's selection offers editing, not CBS analysis. CBS has no works and shows them all.
# The architectural objects' own panels are a building's (docs/adr/0802): KentOS Harita hands such a drawing to KentOS
# Mimari, and its search finds no wall or space tool.
ARCH_TABS = ("ctx.wall", "ctx.opening", "ctx.space", "ctx.slab", "ctx.arch")
for tab in CONTEXTUAL:
    for panel in tab["panels"]:
        if panel["label"] == "İşlemler" or (tab["id"] == "ctx.point" and panel["label"] == "Nokta"):
            panel["works"] = list(MAP)
        elif tab["id"] in ARCH_TABS and panel["label"] not in ("Seçim", "Düzenle"):
            panel["works"] = list(ARCHITECTURE)

# A module's program offers what its ribbon holds and nothing a ribbon of another module alone holds (docs/adr/0802):
# Komut ara, the command line, the shortcuts and `App::run` find nothing else. These commands only CBS's ribbon holds
# (or only the architectural tabs) belong to a CAD module's program too: every module's (`everywhere`: a layer made,
# the project's coordinate system, Nesne özellikleri, İfadeyle seç and Konuma göre seç), and Harita's (`map`: a layer's
# coordinate system, labels, filter and typed kinds, its value lists and rules, bound sources, CBS's file formats, the
# map's system, the processing history, a dataset's record).
# `only`: commands no ribbon holds that only these modules' programs offer (an empty list: kentos-cad's alone, as the
# project type's switch: a module's projects keep their type). A processing tool or model no ribbon holds or names is
# offered where İşlem araç kutusu is (catalog.rs); every other command no ribbon holds is the shell's, every module's.
MODULES = {
    "everywhere": ["layer.new", "layer.newGroup", "layer.showAll", "crs.set", "view.properties",
        "processing.run.selection.byExpression", "processing.run.selection.byLocation"],
    "map": ["layer.crs", "layer.reproject", "view.mapCrs", "file.import.fgb", "file.import.gml", "file.import.parquet",
        "file.import.maplibre", "file.export.maplibre", "source.link", "source.read", "source.write",
        "source.writeTogether", "source.status", "source.reconcile", "source.relink", "source.unlink", "profile.window",
        "linearRef.eventLinks", "layer.newPoint", "layer.newLine", "layer.newArea", "layer.newAnnotation",
        "layer.filter", "layer.filterClear", "layer.labels", "tool.labelMove", "tool.labelPin", "tool.labelHide",
        "tool.labelRotate", "view.pinnedLabels", "view.unplacedLabels", "draft.overlap.allow", "draft.overlap.layer",
        "draft.overlap.layers", "draft.topologyPoints", "data.domains", "data.subtypes", "data.layerRules",
        "data.layerIndexes", "data.tableStyles", "data.duplicate", "data.contingents", "processing.history", "layer.dataset"],
    "only": {
        # A map's terrain, sheets and parcels (some still to come); İfraz, Aplikasyon and the parcel table are Harita's.
        "analysis.slope": ["harita", "cbs"], "analysis.volume": ["harita", "cbs"], "map.contours": ["harita", "cbs"],
        "map.profile": ["harita", "cbs"], "map.sheet": ["harita", "cbs"], "map.parcelReport": ["harita"],
        "tool.subdivide": ["harita"], "tool.stakeout": ["harita"],
        # Çakışma's lamp (a parcel's neighbours) and the sheet's atlas (a page for each feature).
        "draft.overlap": ["harita", "cbs"], "sheet.atlas": ["harita", "cbs"],
        # Network analysis needs Ağlar…, which only KentOS GIS has.
        "processing.run.network.closestFacility": ["cbs"], "processing.run.network.odMatrix": ["cbs"],
        "processing.run.network.serviceAreas": ["cbs"],
        # The project type's switch: a module's projects keep their module's type.
        "workspace.cad": [], "workspace.gis": [], "workspace.plan3d": [], "workspace.disaster": [],
    },
}

doc = {
    "format": "kentos.ribbon",
    "version": 1,
    "note": "Masaüstünün şeridi (ADR 0622 §3–§4): her proje türünün sekmeleri ve seçimin türüne göre açılan bağlamsal sekmeler. Bölünmüş düğme {\"split\": \"anahtar\"} envanterdeki aynı anahtarlı düğmeyi yöntemleriyle alır.",
    "tabs": {"cad": CAD, "gis": GIS},
    "contextual": CONTEXTUAL,
    "modules": MODULES,
}
OUT = Path(__file__).resolve().parents[2] / "catalog" / "v1" / "ribbon.json"
text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
if "--check" in sys.argv[1:]:
    if OUT.read_text(encoding="utf-8") != text:
        sys.exit("catalog/v1/ribbon.json is not current: python3 scripts/ui/ribbon.py")
    print("catalog/v1/ribbon.json is current")
else:
    OUT.write_text(text, encoding="utf-8")
    print("written", OUT)
