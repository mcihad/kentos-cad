//! The web commands the desktop runs (`ported.json` is written from it).

/// Web command ids the desktop runs. Keep in the order they were ported.
pub const PORTED: &[&str] = &[
    "file.open",
    "file.save",
    "file.saveAs",
    "view.theme.dark",
    "view.theme.light",
    "view.theme.toggle",
    "view.ribbonCollapse",
    "view.keyTips",
    "commandline.focus",
    "help.about",
    "help.shortcuts",
    "layer.showAll",
    "edit.undo",
    "edit.redo",
    // The tool session (docs/adr/0021): the closed-area tool, its confirm and
    // cancel, and the view commands the interaction traces use.
    "tool.polygon",
    "tool.confirm",
    "tool.cancel",
    "view.zoomIn",
    "view.zoomOut",
    "view.zoomExtents",
    // Typed settings (docs/adr/0023): the settings window, and the drafting
    // aids of the session the tool session reads.
    "tools.options",
    "draft.ortho",
    "draft.rightAngle",
    "draft.polar",
    // The line and polyline tools (docs/adr/0027), each writing through its product command.
    "tool.line",
    "tool.polyline",
    // Selecting, deleting and snapping (docs/adr/0029): the select tool, the erase tool
    // through cad.entities.delete (Delete), F3 and the selection commands.
    "tool.select",
    "tool.erase",
    "draft.snap",
    "draft.grid",
    "edit.deselect",
    "edit.selectAll",
    "edit.invertSelection",
    // Drawing tools, round 2 (docs/adr/0032): point, circle and arc through their own
    // product commands; the rectangles and the regular polygon through cad.polygon.create.
    "tool.point",
    "tool.circle",
    "tool.arc",
    "tool.rectangle",
    "tool.rectangle3",
    "tool.regularPolygon",
    // Modify tools, round 1 (docs/adr/0037): move, copy, rotate, scale and mirror on the
    // selection, through cad.entities.transform.
    "tool.move",
    "tool.copy",
    "tool.rotate",
    "tool.scale",
    "tool.mirror",
    // Modify tools, round 2 (docs/adr/0047): the edge, corner and object tools, through
    // cad.entities.edit.
    "tool.offset",
    "tool.trim",
    "tool.extend",
    "tool.fillet",
    "tool.chamfer",
    "tool.break",
    "tool.join",
    "tool.explode",
    "tool.lengthen",
    "tool.vertex",
    // The cloud interface (docs/adr/0041): signing in and out, the catalog of cloud
    // projects, a drawing uploaded as a new project, and the save conflicts.
    "cloud.signIn",
    "cloud.signOut",
    "cloud.open",
    "cloud.upload",
    "cloud.conflicts",
    // The open project's own actions, Buluta dosya olarak kaydet and Son
    // revizyonu aç (cloud/actions.rs, docs/adr/0073).
    "cloud.uploadFile",
    "cloud.openNewest",
    // Proje geçmişi…: the catalog's Geçmiş tab on the open project (docs/adr/0087).
    "cloud.history",
    "cloud.rename",
    "cloud.delete",
    // Projeyi paylaş: the people, their roles and the invitations (cloud/share.rs, docs/adr/0111).
    "cloud.share",
    // New layers and groups (layering.rs): edits that are not undone, as on the web.
    "layer.new",
    "layer.newGroup",
    // File exchange (exchange/): DXF and coordinate lists in and out, through the shared
    // readers and writers; the source coordinate system is asked, never reprojected.
    "file.import.dxf",
    "file.import.ncn",
    "crs.points",
    "file.export.dxf",
    "file.export.ncn",
    // A new project and the project's settings (project/): the web's windows; the
    // standard layer tree checked against the web's drawings (fixtures/project/v1).
    "file.new",
    "file.settings",
    // The start screen with the recent files (start.rs).
    "file.start",
    // The project types that can be chosen (modes.rs, docs/adr/0165); the announced
    // ones stay pending.
    "workspace.cad",
    "workspace.gis",
    // GeoJSON in and out, a Shapefile in from its files or its zip archive
    // (exchange/gis_import.rs, geojson_export.rs; docs/adr/0046, 0053): what the file
    // says of its coordinate system is shown and checked, never reprojected.
    "file.import.geojson",
    "file.import.shp",
    "file.export.geojson",
    // GNSS içe aktar (exchange/gnss_import.rs, docs/adr/0169 §6): a receiver's GPX or
    // NMEA positions moved from WGS 84 into the project's system, with their accuracy.
    "file.import.gnss",
    // KML/KMZ in (exchange/gis_import.rs) and out, GPX out (exchange/wgs84_export.rs;
    // docs/adr/0222): moved into WGS 84 on the way out, into the project's system on the
    // way in only when the user asks.
    "file.import.kml",
    "file.export.kml",
    "file.export.gpx",
    // Cihaza gönder (exchange/field_send.rs, docs/adr/0169 §4): the selected points as an
    // instrument's coordinate file, through the shared writer.
    "field.send",
    // Modify tools, round 3 (docs/adr/0047, part 2): stretch through cad.entities.edit,
    // the arrays through cad.entities.array, align through cad.entities.transform.
    "tool.stretch",
    "tool.array",
    "tool.arrayPolar",
    "tool.align",
    // The bottom panel's tabs (bottom.rs): F2, the history, the coordinate list, warnings.
    "view.bottomPanel",
    "view.coords",
    // Nokta editörü: the bottom panel's Noktalar tab (points/, docs/adr/0153).
    "point.editor",
    // Veride ara: the bottom panel's Arama tab, and the mark its coordinate goes leaves (search/, docs/adr/0178).
    "data.search",
    "data.unmark",
    // Topoloji kuralları: the bottom panel's Topoloji tab and the rules window (topology/, docs/adr/0202).
    "topology.check",
    "topology.rules",
    // The clipboard and the view tools (docs/adr/0056): cut, copy and the pastes over the
    // session's clipboard, written through the document as on the web; pan, zoom window,
    // zoom to the selection and repeating the last command.
    "edit.cut",
    "edit.copy",
    "edit.paste",
    "edit.pasteOriginal",
    "tool.pan",
    "tool.zoomWindow",
    "view.zoomSelection",
    "view.isolateObjects",
    "view.endObjectIsolation",
    "tool.repeat",
    // The window's arrangement and the server check (view_commands.rs): the side panels
    // (F4), full screen, the command search (Alt+Q); Koordinat sistemi… is Proje ayarları
    // on its page (project/).
    "view.rightPanel",
    "view.properties",
    "view.fullscreen",
    "view.commandSearch",
    "crs.set",
    "server.check",
    // Drawing tools, round 3 (docs/adr/0057): the ellipse, spline, construction lines,
    // parallel line, perpendiculars, donut and divide through cad.entities.create; the
    // revision cloud through cad.polygon.create, the spot elevation through cad.point.create.
    "tool.ellipse",
    "tool.spline",
    "tool.xline",
    "tool.ray",
    "tool.parallel",
    "tool.perpIn",
    "tool.perpOut",
    "tool.donut",
    "tool.revcloud",
    "tool.spot",
    "tool.divide",
    // Yazı and the text field over the drawing, which also edits a text's or a
    // dimension's value on a double click (docs/adr/0060).
    "tool.text",
    // Çok satırlı yazı (docs/adr/0182 §4).
    "tool.mtext",
    // Ölçülendirme: aligned, linear, angular, radius and diameter dimensions
    // through cad.entities.create (docs/adr/0061); ordinate and arc length, its
    // methods by their own names too (docs/adr/0147 §7).
    "tool.dimension",
    // Tarama: the region by a closed object or by the line work, islands left
    // out, through cad.entities.create as “Tarama” (docs/adr/0062).
    "tool.hatch",
    "tool.hatchSelected",
    // Blocks (docs/adr/0144).
    "tool.blockDefine",
    "tool.blockInsert",
    "block.panel",
    "block.find",
    "block.purge",
    // Blok öznitelikleri: a block's attribute definitions (docs/adr/0144 §7).
    "block.attributes",
    // Alan işlemleri: birleştir, kesiştir, çıkar, böl, alana ve çizgiye çevir, içine
    // tıklayarak alan; into the document as the web's (docs/adr/0065).
    // Mesafe ölç, Alan hesapla and Parsel oluştur: the path tool's other
    // shapes; a parcel through cad.entities.create on the parcel layer (docs/adr/0067).
    "tool.measure",
    "tool.area",
    "tool.parcel",
    // Hesap: Önden and Geriden kestirme, Aplikasyon; Çizimden picks a point
    // with the pick tool (calc/, docs/adr/0070). Poligon hesabı and Kutupsal
    // alım with the measurements table (docs/adr/0071).
    "calc.forward",
    "calc.resection",
    "calc.stakeout",
    "calc.traverse",
    "calc.polar",
    "calc.fieldbook",
    // Yatay ağ and Kot ağı dengelemesi: least squares with their statistics,
    // written through cad.entities.edit and .create (calc/network/, docs/adr/0203).
    "calc.network",
    "calc.levelNetwork",
    // Vektör oturtma: control points, Helmert, affine or projective by least squares,
    // written through cad.entities.transform (calc/fit/, docs/adr/0156 §7).
    "transform.fit",
    // Kenar eşleme: the line ends of two sheets put together across their edge,
    // written through cad.entities.edit (calc/edgematch/, docs/adr/0159 §9).
    "transform.edgematch",
    // Koordinat dönüştür: a point or a list between the coordinate systems,
    // nothing written (calc/convert.rs, docs/adr/0167 §4).
    "crs.transform",
    "tool.areaUnion",
    "tool.areaIntersect",
    "tool.areaSubtract",
    "tool.areaSplit",
    "tool.toArea",
    "tool.toPolyline",
    "tool.boundary",
    // Toplu alan: every region the line work closes, its label the attribute (polygonize.rs, docs/adr/0151).
    "tool.polygonize",
    // Bitişik alan: the region a drawn path closes with the neighbouring areas (adjoin.rs, docs/adr/0162 §3).
    "tool.adjoin",
    // Köşelere nokta: a named point at every vertex of the selection (vertex_points.rs, docs/adr/0152 §5).
    "tool.vertexPoints",
    // Etiketleri yazıya çevir: the layers' labels as texts at a scale (labels_to_text.rs, docs/adr/0175 §3).
    "tool.labelsToText",
    // Koordinat yaz and Köşelere koordinat yaz: coordinate labels at points or at the selection's
    // vertices, the latter with its coordinate schedule (coordinate_labels.rs, docs/adr/0185).
    "tool.coordinateLabel",
    "tool.coordinateVertices",
    // docs/adr/0189: Km yaz.
    "tool.stationLabels",
    // docs/adr/0190: Orta hat.
    "tool.centerline",
    // docs/adr/0191: Paralel kaydır.
    "tool.edgeShift",
    // docs/adr/0192: Resim ekle and Resmi kırp.
    "tool.imageInsert",
    "tool.imageClip",
    // docs/adr/0193: Seçilenleri dosyaya kaydet, Başka çizimden al and Dosyadan blok ekle
    // (drawing_exchange.rs).
    "file.saveSelection",
    "file.takeFrom",
    "block.insertFile",
    // docs/adr/0194: Hizala ve dağıt (kentos_interaction::align_distribute).
    "tool.alignDistribute",
    // Eğri boyunca yazı (docs/adr/0196 §4).
    "tool.textAlong",
    "tool.textCurve",
    // Çizim ekleri (docs/adr/0197 §4).
    "tool.tangentLine",
    "tool.fourthCorner",
    "tool.rangeRings",
    // Plan yolu çizimi (docs/adr/0198 §5).
    "tool.planRoad",
    "tool.roadJunctions",
    "tool.medianClose",
    // Katman alanları, öznitelik tablosu ve veri kaynakları (docs/adr/0199): the Tablo tab
    // (features/), the Alanlar window (layer_fields.rs), the Kaynaklar panel (sources/).
    "data.featureTable",
    "layer.fields",
    "data.sources",
    // Grafikler ve alan istatistikleri (docs/adr/0227 §8): the İstatistik tab (statistics/), the
    // Grafik tab and its six kinds (charts/).
    "data.statistics",
    "data.chart",
    "chart.bar",
    "chart.pie",
    "chart.histogram",
    "chart.box",
    "chart.scatter",
    "chart.line",
    // docs/adr/0195: Görünüm kipleri.
    "view.colorMode.color",
    "view.colorMode.mono",
    "view.colorMode.gray",
    // docs/adr/0205 §5: Yazıların boyu.
    "view.annotationSize.legible",
    "view.annotationSize.true",
    "view.annotationSize.screen",
    "view.fills",
    "view.areaEdges",
    "view.pointNames",
    "view.pointCodes",
    "view.transparency",
    // Drawing and editing tools, phase 1 (docs/adr/0140): every corner at once, Parçala
    // with its three methods, direction, thinning, cleaning and property copying.
    "tool.filletAll",
    "tool.chamferAll",
    "tool.split",
    "tool.reverse",
    "tool.simplify",
    // Okunur yap: the texts that read upside down turned half round (docs/adr/0145 §6).
    "tool.readable",
    // Bul ve değiştir's window (find_replace.rs, docs/adr/0145 §6).
    "text.findReplace",
    // Metin dosyası yerleştir: a text file's lines as texts (text_file.rs, docs/adr/0145 §6).
    "tool.placeTextFile",
    // Kılavuz: the arrow, its line, landing and note (leader.rs, docs/adr/0146 §7).
    "tool.leader",
    "tool.cleanup",
    // Topolojik temizlik: ends, vertices, extend and trim within a tolerance (topology.rs, docs/adr/0148).
    "tool.topology",
    "tool.matchProperties",
    // Phase 2 and 3 of docs/adr/0140: the slice, points between two points, the point found from
    // distances, bearings or lines, the angle, the chained and stacked dimensions, the coordinate
    // read (Harita › Koordinatlar) and the array along a path.
    "tool.sector",
    "tool.pointsBetween",
    "tool.intersectPoint",
    "tool.measureAngle",
    "tool.dimContinue",
    "tool.dimBaseline",
    // docs/adr/0147 §7: Hızlı ölçü.
    "tool.quickDimension",
    "crs.query",
    "tool.arrayPath",
    // İşlemler (docs/adr/0084): each tool's and model's window, and Harita's
    // Kenar ölçülerini yaz, which opens Kenar uzunluklarını yaz.
    "processing.run.points.numberVertices",
    "processing.run.annotation.edgeLengths",
    "processing.run.attributes.calculate",
    "processing.run.selection.byExpression",
    // Mekânsal ve öznitelik sorgusu (docs/adr/0200).
    "processing.run.selection.byLocation",
    "processing.run.attributes.fromInside",
    "processing.run.attributes.fromEnclosing",
    "processing.run.statistics.summary",
    "processing.run.attributes.joinByField",
    // Veri aktar (docs/adr/0216).
    "processing.run.attributes.transfer",
    // Yakınlık (docs/adr/0215).
    "processing.run.proximity.nearest",
    "processing.run.proximity.matrix",
    "processing.run.proximity.hub",
    "processing.run.proximity.neighbors",
    "processing.run.proximity.shortestLine",
    // Altyapı bağlarını çiz (docs/adr/3860): a scenario's areas joined to their nearest infrastructure.
    "processing.run.scenarios.links",
    // Katman işlemleri (docs/adr/0217).
    "processing.run.layers.splitByAttribute",
    "processing.run.layers.merge",
    "processing.run.layers.extract",
    "processing.run.layers.deleteDuplicates",
    "processing.run.layers.orderBy",
    // Geometri dönüştürme (docs/adr/0218).
    "processing.run.convert.toPoints",
    "processing.run.convert.vertices",
    "processing.run.convert.pointsToPath",
    "processing.run.convert.bearingLine",
    "processing.run.convert.xyLine",
    "processing.run.convert.tableEllipse",
    "processing.run.convert.tablePoints",
    "processing.run.convert.substring",
    "processing.run.convert.polygonsToLines",
    "processing.run.convert.linesToPolygons",
    "processing.run.convert.densify",
    // Geometri üretimi (docs/adr/0219).
    "processing.run.generate.voronoi",
    "processing.run.generate.delaunay",
    "processing.run.generate.bounding",
    "processing.run.generate.concaveHull",
    "processing.run.generate.pole",
    "processing.run.sample.grid",
    "processing.run.sample.regular",
    "processing.run.sample.random",
    "processing.run.sample.alongLines",
    "processing.run.sample.subdivide",
    "processing.run.generalize.smooth",
    "processing.run.generalize.orthogonalize",
    "processing.run.generalize.building",
    "processing.run.generalize.aggregate",
    "processing.run.generalize.dropParts",
    "processing.run.generalize.eliminate",
    "processing.run.generalize.coverageSimplify",
    // Geometri işlemleri (docs/adr/0201).
    "processing.run.geometry.buffer",
    "processing.run.geometry.clip",
    "processing.run.geometry.dissolve",
    "processing.run.geometry.intersection",
    "processing.run.geometry.difference",
    "processing.run.geometry.symDifference",
    "processing.run.geometry.union",
    "processing.run.geometry.validity",
    "processing.run.geometry.repair",
    // Topolojiyi kurallarla denetle (docs/adr/1980): the project's rules from the toolbox.
    "processing.run.topology.check",
    // İmar planını denetle (docs/adr/3840): the plan's check from the toolbox, its road regions on an error layer.
    "processing.run.zoning.check",
    "processing.run.geometry.simplify",
    "processing.run.geometry.reproject",
    // Ağ analizi (docs/adr/0209 §9).
    "processing.run.network.closestFacility",
    "processing.run.network.odMatrix",
    "processing.run.network.serviceAreas",
    "processing.run.linearRef.createRoutes",
    "processing.run.linearRef.calibrateRoutes",
    "processing.run.linearRef.routeEvents",
    "processing.run.linearRef.locateAlong",
    "processing.run.linearRef.overlayEvents",
    "processing.run.linearRef.extractMeasures",
    "processing.model.builtin.parcelSheet",
    "processing.newModel",
    "map.edgeLengths",
    // The dock's İşlemler tab: the toolbox and this session's runs (docs/adr/0084, part 2).
    "processing.toolbox",
    "processing.history",
    // Nesne izleme, Shift+F3 (docs/adr/0085).
    "draft.tracking",
    // Topolojik düzenleme and its Noktalar da (docs/adr/0160).
    "draft.topology",
    "draft.topologyPoints",
    // The digitizing locks (docs/adr/0166).
    "draft.lock.length",
    "draft.lock.angle",
    "draft.lock.deflection",
    "draft.lock.parallel",
    "draft.lock.perpendicular",
    "draft.lock.reference",
    "draft.lock.construction",
    "draft.lock.keep",
    "draft.lock.clear",
    // Çakışma denetimi and its modes (docs/adr/0162).
    "draft.overlap",
    "draft.overlap.allow",
    "draft.overlap.layer",
    "draft.overlap.layers",
    // The snap kinds one by one and Çizilmekte olan nesneye (docs/adr/0163 §6).
    "draft.snap.endpoint",
    "draft.snap.midpoint",
    "draft.snap.intersection",
    "draft.snap.center",
    "draft.snap.perpendicular",
    "draft.snap.tangent",
    "draft.snap.node",
    "draft.snap.nearest",
    "draft.snap.centroid",
    "draft.snap.extension",
    "draft.snap.parallel",
    "draft.snap.grid",
    "draft.snap.self",
    // The styled drawing's view choices (docs/adr/0090): layer line weights on or off
    // (Kalınlık), symbols at the plot scale or at a fixed size on the screen.
    "view.lineWeights",
    "view.symbols.plot",
    "view.symbols.screen",
    // Katman stili: the active layer's renderer (docs/adr/0091).
    "style.layerStyle",
    // Stil yöneticisi, and symbols given to the selected objects or taken away (docs/adr/0092).
    "style.manager",
    // Yazı stilleri and Ölçü stilleri (docs/adr/0183 §5).
    "style.textStyles",
    "style.dimensionStyles",
    // Tablo stilleri (docs/adr/1127 §4).
    "style.tableStyles",
    // Tablo ekle, Tabloyu düzenle and Tabloyu güncelle (tables/, docs/adr/0184).
    "table.insert",
    "table.edit",
    "table.update",
    "table.export",
    "table.extract",
    "style.assign",
    "style.clearSymbol",
    // Lejant, saved as a PNG on white paper (docs/adr/0093).
    "style.legend",
    // SVG çizim düzenleyicisi, its import, export, document properties and tracing (docs/adr/0095).
    "style.svgEditor",
    // Netcad NCZ in, through the same window as DXF (exchange/drawing_import.rs, docs/adr/0138).
    "file.import.ncz",
    // Navigation of docs/adr/0141 (navigation.rs): the views left, and the objects far from the drawing.
    "view.previous",
    "view.next",
    "view.extentCheck",
    // Dik ayak ölç (docs/adr/0141): a point read against a line, to the log.
    "tool.stationOffset",
    // The selecting tools of docs/adr/0141, Giriş › Seçim's Seç ▾: what a fence crosses, a
    // circle holds or touches, and the areas around a point.
    "tool.selectFence",
    "tool.selectCircle",
    "tool.selectContaining",
    // Seçim ekleri (docs/adr/0187): Çokgenle seç, Benzerini seç, Önceki seçim, Sıradakini seç
    // (Shift+Boşluk) and Seçim süzgeci with its kinds.
    "tool.selectPolygon",
    "tool.selectSimilar",
    "edit.previousSelection",
    "edit.cycleSelection",
    "edit.selectFilter",
    "edit.selectFilter.point",
    "edit.selectFilter.line",
    "edit.selectFilter.polyline",
    "edit.selectFilter.polygon",
    "edit.selectFilter.circle",
    "edit.selectFilter.arc",
    "edit.selectFilter.ellipse",
    "edit.selectFilter.spline",
    "edit.selectFilter.xline",
    "edit.selectFilter.ray",
    "edit.selectFilter.text",
    "edit.selectFilter.dimension",
    "edit.selectFilter.hatch",
    "edit.selectFilter.insert",
    "edit.selectFilter.leader",
    "edit.selectFilter.table",
    // The selection filter's seventeenth kind and its bulk rows (docs/adr/0187 §5, 8 Ekim):
    // Resim, Bütün türler and Hiçbir tür, ticked with the menu open.
    "edit.selectFilter.image",
    "edit.selectFilterAll",
    "edit.selectFilterNone",
    // Kot ver (docs/adr/0142): the vertices of the selection given elevations, through
    // cad.entities.edit as Kot ver; Sabit, Artır and Sıfırla are its methods.
    "tool.setElevation",
    // Çok parçalı alan (docs/adr/0143): the parts joined into one area, an area split into its parts.
    "tool.partsJoin",
    "tool.partsSplit",
    // Delikler (docs/adr/0173 §5): a ring cut from an area, a hole removed, a hole filled with a new area.
    "tool.holeAdd",
    "tool.holeRemove",
    "tool.holeFill",
    // Sürdür (docs/adr/0173 §4): a line or a polyline continued from an end.
    "tool.continue",
    // Biçim değiştir (docs/adr/0173 §2–§3): an area or a path reshaped by a line drawn over it.
    "tool.reshape",
    // Modele dön (MODEL): the sheet mode's own, from a sheet back to the drawing (docs/adr/0164).
    "sheet.model",
    "sheet.drawingSet",
    // Şablonla çiz (docs/adr/0176 §3): an object template's layer, colour and tool (templates.rs).
    "template.draw",
    // Şablonlar: the templates' panel in the dock (templates_panel.rs, docs/adr/0176 §4).
    "template.panel",
    // Şablon düzenleyici: a new template, or one made from the selected object (template_editor.rs).
    "template.new",
    "template.fromSelection",
    // Şablonu uygula (docs/adr/0176 §6): without a template's id it shows the Şablonlar panel (templates.rs).
    "template.apply",
    // docs/adr/0177 §1: the layer actions by an object and Yalıtımı kaldır.
    "tool.layerOff",
    "tool.layerIsolate",
    "tool.layerLock",
    "tool.layerMakeActive",
    "layer.unisolate",
    // docs/adr/0311: Katmanı dondur, Bütün katmanları çöz, Katmanları çevir.
    "tool.layerFreeze",
    "layer.thawAll",
    "layer.translate",
    // docs/adr/0312: Standardı denetle and Standardı uygula (drawing_standard.rs).
    "standards.check",
    "standards.apply",
    // docs/adr/0541: Veri profili (data_profile.rs).
    "profile.window",
    // docs/adr/0557: Alt ağlar (subnetworks_window.rs).
    "network.subnetworks",
    // docs/adr/0569: Olay bağları (event_links_window.rs).
    "linearRef.eventLinks",
    // docs/adr/1340 §6: GNSS arşivi, Ölçüm oturumu ekle, Kabul edilen noktalar (map_survey_panel.rs).
    "map.survey",
    "map.survey.import",
    "map.survey.select",
    // plan/road.md Y2.4: Arazi yüzeyleri and Yüzey oluştur (terrain_window.rs).
    "terrain.surfaces",
    "terrain.new",
    // plan/road.md Y3: Güzergâh and Yeni aks (alignment_window.rs).
    "alignment.editor",
    "alignment.new",
    // plan/road.md Y4: Boykesit and Boykesit oluştur (profile_window.rs).
    "profile.editor",
    "profile.new",
    // plan/road.md Y5: Enkesit, Kesit takımı oluştur and Tip kesit oluştur (section_window.rs).
    "section.editor",
    "section.new",
    "section.template",
    // plan/road.md Y6: Dever and Dever oluştur (superelevation_window.rs).
    "superelevation.editor",
    "superelevation.new",
    // plan/road.md Y7: Koridor, Koridor oluştur and Şevli plan (corridor_window.rs).
    "corridor.editor",
    "corridor.new",
    "corridor.slopePlan",
    // plan/road.md Y8: Kübaj, Kübaj hesabı oluştur and Kübaj cetveli (volume_window.rs).
    "volume.editor",
    "volume.new",
    "volume.export",
    // plan/road.md Y9: Brükner and Brükner oluştur (haul_window.rs).
    "haul.editor",
    "haul.new",
    // plan/road.md Y10: Aplikasyon cetveli (stakeout_window.rs).
    "stakeout.table",
    // plan/road.md Y10.5: Yol paftaları (road_sheets_window.rs).
    "road.sheets",
    // plan/road.md Y13.4: Görünüşler (road_views_window.rs).
    "road.views",
    // plan/road.md Y10.6: Çizgiden aks (road_alignment_fit_window.rs).
    "alignment.fromLine",
    // plan/road.md Y10.6b: Çizimden boykesit (road_profile_fit_window.rs).
    "profile.fromLine",
    // plan/road.md Y11.1: Otokorkuluk (guardrail_window.rs).
    "guardrail.editor",
    // plan/road.md Y11.2: Menfez (culvert_window.rs).
    "culvert.editor",
    // plan/road.md Y11.3: İstinat duvarı (retaining_wall_window.rs).
    "retainingWall.editor",
    // plan/road.md Y14.3: Platform (platform_window.rs).
    "platform.editor",
    // plan/road.md Y15.3: Buluttan arazi (terrain_cloud_window.rs).
    "terrain.cloud",
    // plan/road.md Y16.3: Drenaj (drainage_window.rs).
    "drainage.editor",
    // plan/road.md Y17.3: Araç izi (swept_window.rs).
    "swept.editor",
    // plan/road.md Y18.5: Yüzey analizi (surface_analysis_window.rs).
    "terrain.analysis",
    // plan/road.md Y20: Görüş mesafesi (sight_window.rs).
    "profile.sight",
    // plan/road.md Y25: Kırmızı kot önerisi (propose_window.rs).
    "profile.propose",
    // plan/road.md Y21: Tasarım denetimi (design_check_window.rs).
    "alignment.check",
    // plan/road.md Y22: Platform suyu (platform_water_window.rs).
    "corridor.water",
    // plan/road.md Y23: Taşıma planı (haul_plan_window.rs).
    "haul.plan",
    // plan/road.md Y11.4: Poz metrajı (pay_window.rs).
    "payItems.editor",
    // plan/road.md Y12.1: Ön güzergâh (route_window.rs).
    "route.finder",
    // plan/road.md Y12.2: Kavşak (junction_window.rs).
    "junction.editor",
    // plan/road.md Y10.2: LandXML ver (road_exchange.rs).
    "landxml.export",
    // docs/adr/1031: Makine kontrolüne ver (road_exchange.rs).
    "landxml.machine",
    "landxml.importAlignments",
    "ifc.export",
    // plan/road.md Y12.3: glTF ver (road_exchange.rs) and 3B görünüm (scene3d_window.rs).
    "gltf.export",
    "scene3d.view",
    // docs/adr/0773: Bina programı (program_window.rs).
    "arch.program",
    // docs/adr/8047: IFC dışa aktar (bina) (arch_ifc.rs).
    "arch.ifcExport",
    // docs/adr/0313: Çizgi tipleri (line_types_window.rs).
    "lineTypes.manage",
    // docs/adr/0331 §1: Tarama desenleri (hatch_patterns_window.rs).
    "hatchPatterns.manage",
    // docs/adr/0177 §2: Katmanı eşle and Katmana kopyala.
    "tool.layerMatch",
    "tool.copyToLayer",
    // docs/adr/0177 §3: Kopyasını oluştur and Katmanları birleştir (layer_merge.rs).
    "layer.duplicate",
    "layer.merge",
    // docs/adr/0177 §4–§6: Katman durumları (layer_states.rs), Kullanılmayanları temizle
    // (layer_purge.rs) and Katman listesi (layer_list.rs).
    "layer.states",
    "layer.stateSave",
    "layer.purge",
    "layer.list",
    // docs/adr/0179: Veri karşılaştır (data_compare.rs).
    "data.compare",
    // docs/adr/0180: Kayıtlı ölçüleri denetle and Kayıtlı ölçüleri çizimden yaz (cogo.rs).
    "cogo.check",
    "cogo.update",
    // docs/adr/0181: Genel bakış and Büyüteç (navigation_cards.rs).
    "view.overview",
    "view.magnifier",
    // docs/adr/0204: Raster ekle, Raster stili (rasters/) and Raster oturt (calc/raster_fit.rs).
    "raster.add",
    "raster.style",
    "raster.georef",
    // docs/adr/0243: Mesh ekle (rasters/multidim.rs); Raster ekle reads NetCDF there too.
    "mesh.add",
    // docs/adr/0207: Nokta bulutu (pointclouds/) and İşlemler's point cloud tools; the desktop's
    // only for now (the owner's decision, 8 October): the web shows them waiting.
    "pointcloud.add",
    "pointcloud.style",
    "pointcloud.query",
    "pointcloud.vpcSave",
    "processing.run.pointcloud.areaStats",
    "processing.run.pointcloud.thin",
    "processing.run.pointcloud.ground",
    "processing.run.pointcloud.classify",
    "processing.run.pointcloud.clip",
    "processing.run.pointcloud.merge",
    "processing.run.pointcloud.tile",
    "processing.run.pointcloud.rasterize",
    "processing.run.pointcloud.boundary",
    "processing.run.pointcloud.transform",
    // docs/adr/0208: Harita servisleri (services/): the ready basemaps, Altlığı kaldır and Bağlantılar.
    "basemap.osmStandard",
    "basemap.osmTopo",
    "basemap.osmHot",
    "basemap.cyclosm",
    "basemap.ofmLiberty",
    "basemap.ofmBright",
    "basemap.ofmPositron",
    "basemap.esriImagery",
    "basemap.hgmHarita",
    "basemap.hgmOrtofoto",
    "basemap.googleRoadmap",
    "basemap.googleSatellite",
    "basemap.googleTerrain",
    "basemap.googleHybrid",
    "basemap.maptilerStreets",
    "basemap.maptilerSatellite",
    "basemap.remove",
    "service.connections",
    "service.add",
    "service.feed",
    "service.info",
    // docs/adr/0231: Yüzey analizi, İşlemler's tools on both platforms; Eşyükselti üret and Eğim
    // analizi open two of them.
    "processing.run.surface.slope",
    "processing.run.surface.aspect",
    "processing.run.surface.hillshade",
    "processing.run.surface.colorRelief",
    "processing.run.surface.curvature",
    "processing.run.surface.ruggedness",
    "processing.run.surface.insolation",
    "processing.run.surface.contours",
    "map.contours",
    "analysis.slope",
    // docs/adr/0232: İnterpolasyon and Yoğunluk, İşlemler's tools on both platforms.
    "processing.run.interpolation.idw",
    "processing.run.interpolation.naturalNeighbor",
    "processing.run.interpolation.spline",
    "processing.run.interpolation.kriging",
    "processing.run.interpolation.tin",
    "processing.run.density.kernel",
    "processing.run.density.line",
    // Raster işlemleri and Raster istatistiği (docs/adr/0233).
    "processing.run.raster.calculator",
    "processing.run.raster.reclassify",
    "processing.run.raster.clipByMask",
    "processing.run.raster.mosaic",
    "processing.run.raster.resample",
    "processing.run.raster.zonalStatistics",
    "processing.run.raster.histogram",
    "processing.run.raster.focalStatistics",
    "processing.run.raster.cellStatistics",
    // Raster ve vektör and Taranmış harita (docs/adr/0234).
    "processing.run.raster.rasterize",
    "processing.run.raster.toPolygons",
    "processing.run.raster.toLines",
    "processing.run.raster.toPoints",
    "processing.run.scan.captureLine",
    "processing.run.scan.closeArea",
    "processing.run.scan.contourElevations",
    "processing.run.hydrology.fill",
    "processing.run.hydrology.flowDirection",
    "processing.run.hydrology.flowAccumulation",
    "processing.run.hydrology.wetness",
    "processing.run.hydrology.pourPoint",
    "processing.run.hydrology.watershed",
    "processing.run.hydrology.basins",
    "processing.run.hydrology.streams",
    "processing.run.distance.euclidean",
    "processing.run.distance.cost",
    "processing.run.distance.path",
    "processing.run.distance.corridor",
    "processing.run.suitability.fuzzyMembership",
    "processing.run.suitability.fuzzyOverlay",
    "processing.run.suitability.weightedSum",
    "processing.run.suitability.weightedOverlay",
    "processing.run.suitability.pairwise",
    "processing.run.suitability.roc",
    "processing.run.stats.meanCenter",
    "processing.run.stats.medianCenter",
    "processing.run.stats.standardDistance",
    "processing.run.stats.directionalDistribution",
    "processing.run.stats.nearestNeighbor",
    "processing.run.stats.moransI",
    "processing.run.stats.hotSpot",
    "processing.run.stats.dbscan",
    "processing.run.stats.kMeans",
    "processing.run.remote.composite",
    "processing.run.remote.split",
    "processing.run.remote.index",
    "processing.run.remote.supervised",
    "processing.run.remote.unsupervised",
    "processing.run.remote.accuracy",
    "processing.run.remote.change",
    "processing.run.remote.pansharpen",
    // docs/adr/0243: Çok boyutlu veri (kentos-processing's builtin/multidim/).
    "processing.run.multidim.profile",
    "processing.run.multidim.series",
    "processing.run.multidim.meshCalculator",
    // docs/adr/0209: Ağlar and the network tools (networks/, kentos_interaction::network).
    "network.manage",
    "tool.netRoute",
    "tool.netServiceArea",
    "tool.netTrace",
    // docs/adr/0210: Zaman and Senaryo (temporal/, kentos_interaction::time_version).
    "time.slider",
    "time.layer",
    "time.sensors",
    "tool.timeVersion",
    "tool.timeEnd",
    "time.compare",
    "scenario.create",
    "scenario.show",
    "scenario.base",
    "scenario.compare",
    "scenario.apply",
    // docs/adr/0211: Katman süzgeci (layer_filters.rs).
    "layer.filter",
    "layer.filterFromSelection",
    "layer.filterClear",
    // docs/adr/0212: Etiket motoru (labelling/, kentos_interaction::label_tools).
    "layer.labels",
    "tool.labelMove",
    "tool.labelRotate",
    "tool.labelPin",
    "tool.labelHide",
    "view.pinnedLabels",
    "view.unplacedLabels",
    // Nesne ekleri (docs/adr/0225).
    "tool.identify",
    "layer.info",
    "attachment.add",
    "attachment.addLink",
    "file.import.photos",
    // Yeni pafta and Şablondan pafta from the ribbon's Çıktı (docs/adr/0622 §3): the sheet mode's own
    // new sheet and template gallery, as the tabs under the drawing open them (sheets.rs).
    "sheet.new",
    "sheet.fromTemplate",
];
