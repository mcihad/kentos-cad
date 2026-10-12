# CAD işlem manifesti

Üretilir: `python3 scripts/cad/manifest.py` (ADR 0244). Elle düzenlenmez; düzeyler `catalog/v1/operations.json`'dadır.
Satırlar arayüzün komutları, araçları ve araçların yöntemleridir; tamamı `manifest.json`'da.

## Düzeyler

- **semantic:** Ürün komutu kullanıcının verdiğini alır, sonucu kendisi hesaplar: Python ve MCP aynı işi yeniden hesaplamadan yapar.
- **geometry:** Ürün komutu var ama sonucun geometrisini çağıran hesaplar: arayüz hesaplayıp yazdırır.
- **processing:** İşlemler aracı: yalnız uygulamanın çalıştırıcısından.
- **none:** Belgeyi ya da projeyi değiştirir ama ürün komutu yok: arayüz doğrudan yazar.
- **query:** Yalnız okur ya da ölçer.
- **session:** Görünüm, seçim, çizim yardımcısı ya da aracın adımı: oturumun durumu.
- **host:** Pencere, panel ya da yardım: uygulamanın sunumu.

## Sayılar

| Düzey | cad | shared | none | gis | Toplam |
|---|---|---|---|---|---|
| semantic | 25 | 75 | 5 | 38 | 143 |
| geometry | 26 | 78 | 0 | 8 | 112 |
| processing | 0 | 11 | 2 | 140 | 153 |
| none | 0 | 23 | 57 | 6 | 86 |
| query | 0 | 21 | 0 | 4 | 25 |
| session | 0 | 78 | 27 | 5 | 110 |
| host | 0 | 15 | 7 | 4 | 26 |
| **toplam** | 51 | 301 | 98 | 205 | 655 |

Kapsam: `cad` yalnız CAD şeridinde, `gis` yalnız CBS şeridinde, `shared` ikisinde, `none` hiçbir şeritte (kısayol, menü,
komut satırı).

## CAD tarafı: Python ve AI'ın arayüz gibi yapamadıkları

CBS'ye özgü olmayan 450 satırdan 214'i: geometry 104, none 80, processing 13, query 17.

### Sonucu çağıran hesaplar (geometry) — 104

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `block.insertFile` | Dosyadan blok ekle… | shared | cad.blocks.define |  |
| `calc.forward` | Önden kestirme… | shared | cad.entities.create `forwardIntersection` |  |
| `calc.levelNetwork` | Kot ağı dengelemesi… | shared | cad.entities.edit `levelAdjust` |  |
| `calc.network` | Yatay ağ dengelemesi… | shared | cad.entities.edit `networkAdjust` |  |
| `calc.polar` | Kutupsal alım… | shared | cad.entities.create `polarSurvey` |  |
| `calc.resection` | Geriden kestirme… | shared | cad.entities.create `resection` |  |
| `calc.traverse` | Poligon hesabı… | shared | cad.entities.create `traverse` |  |
| `cogo.update` | Kayıtlı ölçüleri çizimden yaz | shared | cad.entities.set |  |
| `edit.paste` | Yapıştır | shared | cad.entities.create |  |
| `edit.pasteOriginal` | Özgün koordinatlara yapıştır | shared | cad.entities.create |  |
| `raster.georef` | Raster oturt… | shared | cad.entities.edit `rasterGeoref` |  |
| `table.update` | Tabloyu güncelle | cad | cad.entities.edit `tableUpdate` |  |
| `text.findReplace` | Bul ve değiştir | shared | cad.entities.edit `replaceText` |  |
| `tool.adjoin` | Bitişik alan | shared | cad.entities.create `adjoin` |  |
| `tool.arc` | Yay | shared | cad.arc.create | cad.arc.create merkez, yarıçap ve açılar ister; araç bunları tıklamalardan hesaplar. |
| `tool.arc#3 nokta` | Yay: 3 nokta | shared | cad.arc.create |  |
| `tool.arc#Devam` | Yay: Devam | shared | cad.arc.create |  |
| `tool.arc#Merkez, başlangıç, bitiş` | Yay: Merkez, başlangıç, bitiş | shared | cad.arc.create |  |
| `tool.areaIntersect` | Alan kesiştir | shared | cad.entities.edit `areaIntersect` |  |
| `tool.areaSplit` | Alan böl | shared | cad.entities.edit `areaSplit` |  |
| `tool.areaSubtract` | Alan çıkar | shared | cad.entities.edit `areaSubtract` |  |
| `tool.areaUnion` | Alan birleştir | shared | cad.entities.edit `areaUnion` |  |
| `tool.boundary` | İçine tıklayarak alan | shared | cad.entities.create `boundary` |  |
| `tool.break` | Kır | shared | cad.entities.edit `break` |  |
| `tool.centerline` | Orta hat | shared | cad.entities.create `centerline` |  |
| `tool.circle#2 nokta` | Daire: 2 nokta | shared | cad.circle.create |  |
| `tool.circle#3 nokta` | Daire: 3 nokta | shared | cad.circle.create |  |
| `tool.circle#Teğet, teğet, teğet` | Daire: Teğet, teğet, teğet | shared | cad.circle.create |  |
| `tool.circle#Teğet, teğet, yarıçap` | Daire: Teğet, teğet, yarıçap | shared | cad.circle.create |  |
| `tool.cleanup` | Çizimi temizle | shared | cad.entities.edit `cleanup` |  |
| `tool.continue` | Sürdür | shared | cad.entities.edit `continue` |  |
| `tool.coordinateLabel` | Koordinat yaz | cad | cad.entities.create `coordinates` |  |
| `tool.coordinateVertices` | Köşelere koordinat yaz | cad | cad.entities.create `coordinates` |  |
| `tool.copyToLayer` | Katmana kopyala | shared | cad.entities.create |  |
| `tool.dimBaseline` | Baz ölçü | cad | cad.entities.create `dimensionBaseline` |  |
| `tool.dimContinue` | Zincir ölçü | cad | cad.entities.create `dimensionChain` |  |
| `tool.dimension#Kırıklı yarıçap` | Ölçülendirme: Kırıklı yarıçap | cad | cad.entities.create |  |
| `tool.dimension#Yarıçap` | Ölçülendirme: Yarıçap | cad | cad.entities.create |  |
| `tool.dimension#Yay uzunluğu` | Ölçülendirme: Yay uzunluğu | cad | cad.entities.create |  |
| `tool.dimension#Çap` | Ölçülendirme: Çap | cad | cad.entities.create |  |
| `tool.divide` | Böl | shared | cad.entities.create `divide` |  |
| `tool.donut` | Halka | cad | cad.entities.create |  |
| `tool.edgeShift` | Paralel kaydır | shared | cad.entities.edit `edgeShift` |  |
| `tool.explode` | Patlat | shared | cad.entities.edit `explode` |  |
| `tool.fourthCorner` | Dördüncü köşe | shared | cad.entities.create `fourthCorner` |  |
| `tool.hatch` | Tarama | cad | cad.entities.create `hatch` | Bölge araçta bulunur; komut son taramayı alır. |
| `tool.hatchSelected` | Çoklu tara | cad | cad.entities.create `hatch` |  |
| `tool.holeAdd` | Delik ekle | shared | cad.entities.edit `holeAdd` |  |
| `tool.holeFill` | Deliği doldur | shared | cad.entities.edit `holeFill` |  |
| `tool.holeRemove` | Deliği sil | shared | cad.entities.edit `holeRemove` |  |
| `tool.imageClip` | Resmi kırp | shared | cad.entities.edit `imageClip` |  |
| `tool.intersectPoint` | Kesişim noktası | shared | cad.entities.create `intersectPoint` |  |
| `tool.intersectPoint#İki doğru` | Kesişim noktası: İki doğru | shared | cad.entities.create `intersectPoint` |  |
| `tool.intersectPoint#İki doğrultu` | Kesişim noktası: İki doğrultu | shared | cad.entities.create `intersectPoint` |  |
| `tool.intersectPoint#İki uzaklık` | Kesişim noktası: İki uzaklık | shared | cad.entities.create `intersectPoint` |  |
| `tool.join` | Birleştir | shared | cad.entities.edit `join` |  |
| `tool.labelsToText` | Etiketleri yazıya çevir | shared | cad.entities.create `labels` |  |
| `tool.lengthen` | Uzat-kısalt | cad | cad.entities.edit `lengthen` |  |
| `tool.medianClose` | Refüj kapat | shared | cad.entities.edit `medianClose` |  |
| `tool.parallel` | Paralel çizgi | cad | cad.entities.create `parallel` |  |
| `tool.partsJoin` | Parçaları birleştir | shared | cad.entities.edit `partsJoin` |  |
| `tool.partsSplit` | Parçalara ayır | shared | cad.entities.edit `partsSplit` |  |
| `tool.perpIn` | Dik in | cad | cad.entities.create `perpendicularIn` |  |
| `tool.perpOut` | Dik çık | cad | cad.entities.create `perpendicularOut` |  |
| `tool.placeTextFile` | Metin dosyası yerleştir | cad | cad.entities.create `textFile` |  |
| `tool.planRoad` | Plan yolu | shared | cad.entities.create `planRoad` |  |
| `tool.pointsBetween` | Ara nokta | shared | cad.entities.create `pointsBetween` |  |
| `tool.pointsBetween#Eşit aralık` | Ara nokta: Eşit aralık | shared | cad.entities.create `pointsBetween` |  |
| `tool.pointsBetween#Oranla` | Ara nokta: Oranla | shared | cad.entities.create `pointsBetween` |  |
| `tool.pointsBetween#Uzaklıkla` | Ara nokta: Uzaklıkla | shared | cad.entities.create `pointsBetween` |  |
| `tool.polygonize` | Toplu alan | shared | cad.entities.create `polygonize` |  |
| `tool.quickDimension` | Hızlı ölçü | cad | cad.entities.create |  |
| `tool.rangeRings` | Menzil halkaları | shared | cad.entities.create `rangeRings` |  |
| `tool.readable` | Okunur yap | shared | cad.entities.edit `readable` |  |
| `tool.rectangle` | Dikdörtgen | cad | cad.polygon.create | Köşeler iki tıklamadan hesaplanıp cad.polygon.create'e verilir. |
| `tool.rectangle3` | Döndürülmüş dikdörtgen | cad | cad.polygon.create |  |
| `tool.regularPolygon` | Düzgün çokgen | cad | cad.polygon.create |  |
| `tool.reshape` | Biçim değiştir | shared | cad.entities.edit `reshape` |  |
| `tool.revcloud` | Revizyon bulutu | cad | cad.polygon.create |  |
| `tool.reverse` | Yönü çevir | shared | cad.entities.edit `reverse` |  |
| `tool.roadJunctions` | Kavşak temizle | shared | cad.entities.edit `roadJunctions` |  |
| `tool.sector` | Daire dilimi | shared | cad.polygon.create |  |
| `tool.setElevation` | Kot ver | shared | cad.entities.edit `elevation` |  |
| `tool.setElevation#Artır` | Kot ver: Artır | shared | cad.entities.edit `elevation` |  |
| `tool.setElevation#Sabit` | Kot ver: Sabit | shared | cad.entities.edit `elevation` |  |
| `tool.setElevation#Sıfırla` | Kot ver: Sıfırla | shared | cad.entities.edit `elevation` |  |
| `tool.simplify` | Sadeleştir | shared | cad.entities.edit `simplify` |  |
| `tool.split` | Parçala | shared | cad.entities.edit `split` |  |
| `tool.split#Eşit parçalara` | Parçala: Eşit parçalara | shared | cad.entities.edit `split` |  |
| `tool.split#Kesişimlerden` | Parçala: Kesişimlerden | shared | cad.entities.edit `split` |  |
| `tool.split#Uzunluktan` | Parçala: Uzunluktan | shared | cad.entities.edit `split` |  |
| `tool.stationLabels` | Km yaz | cad | cad.entities.create `stations` |  |
| `tool.stretch` | Esnet | cad | cad.entities.edit `stretch` |  |
| `tool.tangentLine` | İki daireye teğet | cad | cad.entities.create `tangentLine` |  |
| `tool.textAlong` | Eğri boyunca yazı | shared | cad.entities.create `textAlong` |  |
| `tool.textCurve` | Yazıyı eğriye oturt | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Doğrultuya döndür` | Yazıyı eğriye oturt: Doğrultuya döndür | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Düzleştir` | Yazıyı eğriye oturt: Düzleştir | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Eğriye oturt` | Yazıyı eğriye oturt: Eğriye oturt | shared | cad.entities.edit `textPath` |  |
| `tool.toArea` | Alana çevir | shared | cad.entities.edit `toArea` |  |
| `tool.toPolyline` | Çizgiye çevir | shared | cad.entities.edit `toPolyline` |  |
| `tool.topology` | Topolojik temizlik | shared | cad.entities.edit `topology` |  |
| `tool.vertex` | Köşe ekle/sil | cad | cad.entities.edit `vertexAdd` |  |
| `tool.vertexPoints` | Köşelere nokta | shared | cad.entities.create `vertexPoints` |  |

### Ürün komutu yok (none) — 80

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `data.compare` | Veri karşılaştır… | shared | — |  |
| `field.send` | Cihaza gönder… | shared | — |  |
| `file.export.dxf` | DXF… | shared | — | Dışa aktarma komutu yok (cad.files.*). |
| `file.export.geojson` | GeoJSON… | shared | — |  |
| `file.export.gpx` | GPX… | shared | — |  |
| `file.export.kml` | KML/KMZ… | shared | — |  |
| `file.export.ncn` | Koordinat listesi (NCN, TXT, CSV)… | shared | — |  |
| `file.export.pdf` | PDF pafta… | shared | — |  |
| `file.import.dxf` | DXF… | shared | — | İçe aktarma komutu yok (cad.files.*). |
| `file.import.geojson` | GeoJSON… | shared | — |  |
| `file.import.gnss` | GNSS (GPX, NMEA)… | shared | — |  |
| `file.import.kml` | KML/KMZ… | shared | — |  |
| `file.import.ncn` | Koordinat listesi (NCN, TXT, CSV)… | shared | — |  |
| `file.import.ncz` | Netcad NCZ… | shared | — |  |
| `file.import.photos` | Konumlu fotoğraflar… | shared | — |  |
| `file.import.shp` | Shapefile… | shared | — |  |
| `file.print` | Yazdır ve pafta çıktısı… | shared | — |  |
| `file.saveSelection` | Seçilenleri dosyaya kaydet… | shared | — |  |
| `file.takeFrom` | Başka çizimden al… | shared | — |  |
| `pointcloud.vpcSave` | Sanal bulut olarak kaydet… | shared | — |  |
| `sheet.align.bottom` | Alta hizala | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.align.center` | Yatayda ortala | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.align.left` | Sola hizala | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.align.middle` | Düşeyde ortala | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.align.right` | Sağa hizala | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.align.top` | Üste hizala | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.alignTo.margins` | Kenar boşluklarına göre | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.alignTo.page` | Sayfaya göre | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.alignTo.selection` | Seçime göre | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.atlas` | Atlas | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.delete` | Paftayı sil… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.deleteItems` | Sil | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.distribute.hCenters` | Yatayda ortaları eşit dağıt | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.distribute.hGaps` | Yatayda aralıkları eşitle | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.distribute.vCenters` | Düşeyde ortaları eşit dağıt | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.distribute.vGaps` | Düşeyde aralıkları eşitle | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.duplicate` | Paftayı çoğalt | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.duplicateItems` | Çoğalt | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.escape` | Seçimi bırak | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.export.kpafta` | .kpafta dosyası olarak… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.export.pdf` | PDF olarak… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.export.png` | PNG olarak… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.export.svg` | SVG olarak… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.fromTemplate` | Şablondan pafta… | shared | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.grid` | Karelaj | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.group` | Grupla | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.hideItems` | Gizle ya da göster | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.importKpafta` | .kpafta dosyasından… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.lockItems` | Kilitle ya da aç | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.matchSize.height` | Aynı yükseklik | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.matchSize.width` | Aynı genişlik | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.model` | Modele dön | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.moveLeft` | Sola taşı | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.moveRight` | Sağa taşı | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.new` | Yeni pafta | shared | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.nudge` | Kaydır | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.open` | Paftayı aç | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.order.back` | Arkaya gönder | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.order.backward` | Bir arkaya | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.order.forward` | Bir öne | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.order.front` | Öne getir | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.pageSetup` | Sayfa ayarları… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.preflight` | Ön denetim | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.print` | Yazdır… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.redo` | Yinele (pafta) | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.rename` | Paftaya ad ver… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.renameItem` | Öğeye ad ver | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.saveTemplate` | Şablon olarak kaydet | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.selectAll` | Tümünü seç | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.tool.hand` | El | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.tool.select` | Seç | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.undo` | Geri al (pafta) | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.ungroup` | Grubu çöz | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.variables` | Değişkenler… | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.zoomIn` | Yakınlaştır | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.zoomOut` | Uzaklaştır | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.zoomPage` | Sayfayı sığdır | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.zoomReal` | Gerçek boyut | none | — | Pafta komutları katalogda yok (plan §17). |
| `sheet.zoomSelection` | Seçime yakınlaş | none | — | Pafta komutları katalogda yok (plan §17). |
| `topology.rules` | Topoloji kuralları | shared | — |  |

### Yalnız İşlemler'den (processing) — 13

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `analysis.slope` | Eğim analizi… | none | — |  |
| `map.contours` | Eşyükselti üret… | none | — |  |
| `processing.run.address.geocode` | Adresleri eşle… | shared | — | kentos.address adayları verir, noktaları yazmaz. |
| `processing.run.address.reverse` | Konumlara adres yaz… | shared | — |  |
| `processing.run.pointcloud.areaStats` | Nokta bulutu alan sorgusu… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.boundary` | Sınır çıkar… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.classify` | Yüksekliğe göre sınıfla… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.clip` | Bulutu kırp… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.ground` | Zemin süzgeci… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.merge` | Bulutları birleştir… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.rasterize` | Rasterleştir… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.thin` | Seyrelt… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |
| `processing.run.pointcloud.tile` | Karola… | shared | — | İşlemler aracı; Python ve MCP'den çalıştırılamaz. |

### Okuma, Python'da yok (query) — 17

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `calc.stakeout` | Aplikasyon… | shared | — |  |
| `chart.bar` | Çubuk grafik | shared | — |  |
| `chart.box` | Kutu grafiği | shared | — |  |
| `chart.histogram` | Histogram | shared | — |  |
| `chart.line` | Çizgi grafik | shared | — |  |
| `chart.pie` | Pasta grafik | shared | — |  |
| `chart.scatter` | Saçılım grafiği | shared | — |  |
| `cogo.check` | Kayıtlı ölçüleri denetle… | shared | — |  |
| `crs.query` | Koordinat oku | shared | — |  |
| `data.chart` | Grafik | shared | — |  |
| `data.search` | Veride ara | shared | — |  |
| `data.statistics` | Alan istatistikleri | shared | — |  |
| `pointcloud.query` | Nokta bulutu XYZ sor | shared | — |  |
| `tool.measure` | Mesafe ölç | shared | — | İki nokta arası uzunluk: noktalardan hesap; ölçme API'si nesne ister. |
| `tool.measureAngle` | Açı ölç | shared | — |  |
| `tool.stationOffset` | Dik ayak ölç | shared | — |  |
| `topology.check` | Topolojiyi denetle | shared | — |  |

## Masaüstünde olmayan CAD tarafı öğeleri — 8

Envanterin masaüstü sütunu web'in donduğu gündeki hâlidir (ADR 0228); ekler masaüstündedir.

- `file.export.pdf` PDF pafta… (pending)
- `file.print` Yazdır ve pafta çıktısı… (pending)
- `sheet.atlas` Atlas (pending)
- `sheet.grid` Karelaj (pending)
- `sheet.zoomIn` Yakınlaştır (partial)
- `sheet.zoomOut` Uzaklaştır (partial)
- `workspace.disaster` Afet Analizi (pending)
- `workspace.plan3d` 3D Plan (pending)
