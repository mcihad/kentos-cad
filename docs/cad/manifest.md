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
| semantic | 57 | 134 | 38 | 38 | 267 |
| geometry | 19 | 60 | 0 | 8 | 87 |
| processing | 0 | 11 | 2 | 140 | 153 |
| none | 0 | 20 | 0 | 6 | 26 |
| query | 1 | 23 | 1 | 4 | 29 |
| session | 0 | 80 | 48 | 5 | 133 |
| host | 0 | 15 | 14 | 4 | 33 |
| **toplam** | 77 | 343 | 103 | 205 | 728 |

Kapsam: `cad` yalnız CAD şeridinde, `gis` yalnız CBS şeridinde, `shared` ikisinde, `none` hiçbir şeritte (kısayol, menü,
komut satırı).

## CAD tarafı: Python ve AI'ın arayüz gibi yapamadıkları

CBS'ye özgü olmayan 523 satırdan 133'i: geometry 79, none 20, processing 13, query 21.

### Sonucu çağıran hesaplar (geometry) — 79

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `calc.forward` | Önden kestirme… | shared | cad.entities.create `forwardIntersection` |  |
| `calc.levelNetwork` | Kot ağı dengelemesi… | shared | cad.entities.edit `levelAdjust` |  |
| `calc.network` | Yatay ağ dengelemesi… | shared | cad.entities.edit `networkAdjust` |  |
| `calc.polar` | Kutupsal alım… | shared | cad.entities.create `polarSurvey` |  |
| `calc.resection` | Geriden kestirme… | shared | cad.entities.create `resection` |  |
| `calc.traverse` | Poligon hesabı… | shared | cad.entities.create `traverse` |  |
| `cogo.update` | Kayıtlı ölçüleri çizimden yaz | shared | cad.entities.set |  |
| `raster.georef` | Raster oturt… | shared | cad.entities.edit `rasterGeoref` |  |
| `table.update` | Tabloyu güncelle | cad | cad.entities.edit `tableUpdate` |  |
| `text.findReplace` | Bul ve değiştir | shared | cad.entities.edit `replaceText` |  |
| `tool.adjoin` | Bitişik alan | shared | cad.entities.create `adjoin` |  |
| `tool.areaIntersect` | Alan kesiştir | shared | cad.entities.edit `areaIntersect` |  |
| `tool.areaSplit` | Alan böl | shared | cad.entities.edit `areaSplit` |  |
| `tool.areaSubtract` | Alan çıkar | shared | cad.entities.edit `areaSubtract` |  |
| `tool.areaUnion` | Alan birleştir | shared | cad.entities.edit `areaUnion` |  |
| `tool.centerline` | Orta hat | shared | cad.entities.create `centerline` |  |
| `tool.continue` | Sürdür | shared | cad.entities.edit `continue` |  |
| `tool.coordinateLabel` | Koordinat yaz | cad | cad.entities.create `coordinates` |  |
| `tool.coordinateVertices` | Köşelere koordinat yaz | cad | cad.entities.create `coordinates` |  |
| `tool.copyToLayer` | Katmana kopyala | shared | cad.entities.create |  |
| `tool.dimBaseline` | Baz ölçü | cad | cad.entities.create `dimensionBaseline` |  |
| `tool.dimContinue` | Zincir ölçü | cad | cad.entities.create `dimensionChain` |  |
| `tool.donut` | Halka | cad | cad.entities.create |  |
| `tool.edgeShift` | Paralel kaydır | shared | cad.entities.edit `edgeShift` |  |
| `tool.fourthCorner` | Dördüncü köşe | shared | cad.entities.create `fourthCorner` |  |
| `tool.holeAdd` | Delik ekle | shared | cad.entities.edit `holeAdd` |  |
| `tool.holeFill` | Deliği doldur | shared | cad.entities.edit `holeFill` |  |
| `tool.holeRemove` | Deliği sil | shared | cad.entities.edit `holeRemove` |  |
| `tool.imageClip` | Resmi kırp | shared | cad.entities.edit `imageClip` |  |
| `tool.intersectPoint` | Kesişim noktası | shared | cad.entities.create `intersectPoint` |  |
| `tool.intersectPoint#İki doğru` | Kesişim noktası: İki doğru | shared | cad.entities.create `intersectPoint` |  |
| `tool.intersectPoint#İki doğrultu` | Kesişim noktası: İki doğrultu | shared | cad.entities.create `intersectPoint` |  |
| `tool.intersectPoint#İki uzaklık` | Kesişim noktası: İki uzaklık | shared | cad.entities.create `intersectPoint` |  |
| `tool.labelsToText` | Etiketleri yazıya çevir | shared | cad.entities.create `labels` |  |
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
| `tool.reshape` | Biçim değiştir | shared | cad.entities.edit `reshape` |  |
| `tool.revcloud` | Revizyon bulutu | cad | cad.polygon.create |  |
| `tool.revcloud#Dikdörtgen` | Revizyon bulutu: Dikdörtgen | cad | cad.polygon.create |  |
| `tool.revcloud#Nesne` | Revizyon bulutu: Nesne | cad | cad.polygon.create |  |
| `tool.revcloud#Çokgen` | Revizyon bulutu: Çokgen | cad | cad.polygon.create |  |
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
| `tool.tangentLine` | İki daireye teğet | cad | cad.entities.create `tangentLine` |  |
| `tool.textAlong` | Eğri boyunca yazı | shared | cad.entities.create `textAlong` |  |
| `tool.textCurve` | Yazıyı eğriye oturt | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Doğrultuya döndür` | Yazıyı eğriye oturt: Doğrultuya döndür | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Düzleştir` | Yazıyı eğriye oturt: Düzleştir | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Eğriye oturt` | Yazıyı eğriye oturt: Eğriye oturt | shared | cad.entities.edit `textPath` |  |
| `tool.toArea` | Alana çevir | shared | cad.entities.edit `toArea` |  |
| `tool.toPolyline` | Çizgiye çevir | shared | cad.entities.edit `toPolyline` |  |
| `tool.topology` | Topolojik temizlik | shared | cad.entities.edit `topology` |  |
| `tool.vertexPoints` | Köşelere nokta | shared | cad.entities.create `vertexPoints` |  |
| `tool.xline#Açıortay` | Yardımcı çizgi: Açıortay | cad | cad.entities.create |  |
| `tool.xline#Paralel` | Yardımcı çizgi: Paralel | cad | cad.entities.create |  |

### Ürün komutu yok (none) — 20

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

### Okuma, Python'da yok (query) — 21

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `block.schedule` | Blok çizelgesi | cad | cad.blocks.query | count, instances ve schedule raporları (ADR 0317 §3). |
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
| `layer.impact` | Değişikliğin etkisi | shared | cad.styles.impact |  |
| `pointcloud.query` | Nokta bulutu XYZ sor | shared | — |  |
| `sheet.preflight` | Ön denetim | none | — |  |
| `standards.check` | Standardı denetle | shared | cad.standards.check | Bulgular komutun cevabıdır; pencere onları gösterir (ADR 0312 §2). |
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
