# CAD işlem manifesti

Üretilir: `python3 scripts/cad/manifest.py` (ADR 0244). Elle düzenlenmez; düzeyler `catalog/v1/operations.json`'dadır.
Satırlar arayüzün komutları, araçları ve araçların yöntemleridir; tamamı `manifest.json`'da.

## Düzeyler

- **semantic:** Ürün komutu kullanıcının verdiğini alır, sonucu kendisi hesaplar: Python ve MCP aynı işi yeniden hesaplamadan yapar.
- **geometry:** Ürün komutu var ama sonucun geometrisini çağıran hesaplar: arayüz hesaplayıp yazdırır.
- **processing:** İşlemler aracı: uygulamanın çalıştırıcısıyla; Python ve MCP'den cad.processing.run ile (ADR 0508), kendi anlamsal komutu yok.
- **none:** Belgeyi ya da projeyi değiştirir ama ürün komutu yok: arayüz doğrudan yazar.
- **query:** Yalnız okur ya da ölçer.
- **session:** Görünüm, seçim, çizim yardımcısı ya da aracın adımı: oturumun durumu.
- **host:** Pencere, panel ya da yardım: uygulamanın sunumu.

## Sayılar

| Düzey | cad | shared | none | gis | Toplam |
|---|---|---|---|---|---|
| semantic | 104 | 258 | 43 | 45 | 450 |
| geometry | 2 | 29 | 1 | 3 | 35 |
| processing | 0 | 130 | 22 | 2 | 154 |
| none | 0 | 14 | 6 | 2 | 22 |
| query | 6 | 24 | 2 | 8 | 40 |
| session | 2 | 79 | 57 | 9 | 147 |
| host | 0 | 21 | 14 | 1 | 36 |
| **toplam** | 114 | 555 | 145 | 70 | 884 |

Kapsam: `cad` yalnız CAD şeridinde, `gis` yalnız CBS şeridinde, `shared` ikisinde, `none` hiçbir şeritte (kısayol, menü,
komut satırı).

## CAD tarafı: Python ve AI'ın arayüz gibi yapamadıkları

CBS'ye özgü olmayan 814 satırdan 67'i: geometry 32, none 20, processing 3, query 12.

### Sonucu çağıran hesaplar (geometry) — 32

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `calc.forward` | Önden kestirme… | shared | cad.entities.create `forwardIntersection` |  |
| `calc.levelNetwork` | Kot ağı dengelemesi… | shared | cad.entities.edit `levelAdjust` |  |
| `calc.network` | Yatay ağ dengelemesi… | shared | cad.entities.edit `networkAdjust` |  |
| `calc.polar` | Kutupsal alım… | shared | cad.entities.create `polarSurvey` |  |
| `calc.resection` | Geriden kestirme… | shared | cad.entities.create `resection` |  |
| `calc.traverse` | Poligon hesabı… | shared | cad.entities.create `traverse` |  |
| `cogo.update` | Kayıtlı ölçüleri çizimden yaz | shared | cad.entities.set |  |
| `crs.points` | Nokta listesi içe aktar… | shared | cad.entities.create |  |
| `tool.coordinateLabel` | Koordinat yaz | shared | cad.entities.create `coordinates` |  |
| `tool.coordinateVertices` | Köşelere koordinat yaz | shared | cad.entities.create `coordinates` |  |
| `tool.copyToLayer` | Katmana kopyala | shared | cad.entities.create |  |
| `tool.imageClip` | Resmi kırp | shared | cad.entities.edit `imageClip` |  |
| `tool.labelsToText` | Etiketleri yazıya çevir | shared | cad.entities.create `labels` |  |
| `tool.medianClose` | Refüj kapat | shared | cad.entities.edit `medianClose` |  |
| `tool.placeTextFile` | Metin dosyası yerleştir | cad | cad.entities.create `textFile` |  |
| `tool.planRoad` | Plan yolu | shared | cad.entities.create `planRoad` |  |
| `tool.quickDimension` | Hızlı ölçü | cad | cad.entities.create |  |
| `tool.roadJunctions` | Kavşak temizle | shared | cad.entities.edit `roadJunctions` |  |
| `tool.split` | Parçala | shared | cad.entities.edit `split` |  |
| `tool.split#Eşit parçalara` | Parçala: Eşit parçalara | shared | cad.entities.edit `split` |  |
| `tool.split#Kesişimlerden` | Parçala: Kesişimlerden | shared | cad.entities.edit `split` |  |
| `tool.split#Uzunluktan` | Parçala: Uzunluktan | shared | cad.entities.edit `split` |  |
| `tool.stationLabels` | Km yaz | shared | cad.entities.create `stations` |  |
| `tool.subdivide` | İfraz | none | — |  |
| `tool.textAlong` | Eğri boyunca yazı | shared | cad.entities.create `textAlong` |  |
| `tool.textCurve` | Yazıyı eğriye oturt | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Doğrultuya döndür` | Yazıyı eğriye oturt: Doğrultuya döndür | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Düzleştir` | Yazıyı eğriye oturt: Düzleştir | shared | cad.entities.edit `textPath` |  |
| `tool.textCurve#Eğriye oturt` | Yazıyı eğriye oturt: Eğriye oturt | shared | cad.entities.edit `textPath` |  |
| `tool.topology` | Topolojik temizlik | shared | cad.entities.edit `topology` |  |
| `transform.edgematch` | Kenar eşleme… | shared | cad.entities.edit `edgematch` |  |
| `transform.fit` | Vektör oturtma… | shared | cad.entities.transform | Dönüşüm araçta kontrol noktalarından çözülür. |

### Ürün komutu yok (none) — 20

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `analysis.volume` | Hacim hesabı… | none | — |  |
| `data.compare` | Veri karşılaştır… | shared | — |  |
| `field.send` | Cihaza gönder… | shared | — |  |
| `file.export.geojson` | GeoJSON… | shared | — |  |
| `file.export.gpx` | GPX… | shared | — |  |
| `file.export.kml` | KML/KMZ… | shared | — |  |
| `file.export.ncn` | Koordinat listesi (NCN, TXT, CSV)… | shared | — |  |
| `file.export.pdf` | PDF pafta… | none | — |  |
| `file.import.gnss` | GNSS (GPX, NMEA)… | shared | — |  |
| `file.import.ncn` | Koordinat listesi (NCN, TXT, CSV)… | shared | — |  |
| `file.import.ncz` | Netcad NCZ… | shared | — |  |
| `file.import.photos` | Konumlu fotoğraflar… | shared | — |  |
| `file.print` | Yazdır ve pafta çıktısı… | none | — |  |
| `file.saveSelection` | Seçilenleri dosyaya kaydet… | shared | — |  |
| `file.takeFrom` | Başka çizimden al… | shared | — |  |
| `map.parcelReport` | Parsel alan çizelgesi | none | — |  |
| `map.profile` | Boy kesit al… | none | — |  |
| `map.sheet` | Pafta bölümlemesi… | none | — |  |
| `pointcloud.vpcSave` | Sanal bulut olarak kaydet… | shared | — |  |
| `topology.rules` | Topoloji kuralları | shared | — |  |

### Yalnız İşlemler'den (processing) — 3

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `analysis.slope` | Eğim analizi… | none | — |  |
| `map.contours` | Eşyükselti üret… | none | — |  |
| `map.edgeLengths` | Kenar ölçülerini yaz… | shared | — |  |

### Okuma, Python'da yok (query) — 12

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `calc.stakeout` | Aplikasyon… | shared | — |  |
| `cogo.check` | Kayıtlı ölçüleri denetle… | shared | — |  |
| `crs.query` | Koordinat oku | shared | — |  |
| `data.chart` | Grafik | shared | — |  |
| `data.search` | Veride ara | shared | — |  |
| `data.statistics` | Alan istatistikleri | shared | — |  |
| `service.info` | Servis bilgisi | shared | — |  |
| `sheet.preflight` | Ön denetim | none | — |  |
| `tool.measure` | Mesafe ölç | shared | — | İki nokta arası uzunluk: noktalardan hesap; ölçme API'si nesne ister. |
| `tool.measureAngle` | Açı ölç | shared | — |  |
| `tool.stakeout` | Aplikasyon | none | — |  |
| `tool.stationOffset` | Dik ayak ölç | shared | — |  |

## Masaüstünde olmayan CAD tarafı öğeleri — 13

Envanterin masaüstü sütunu web'in donduğu gündeki hâlidir (ADR 0228); ekler masaüstündedir.

- `analysis.volume` Hacim hesabı… (pending)
- `file.export.pdf` PDF pafta… (pending)
- `file.print` Yazdır ve pafta çıktısı… (pending)
- `map.parcelReport` Parsel alan çizelgesi (pending)
- `map.profile` Boy kesit al… (pending)
- `map.sheet` Pafta bölümlemesi… (pending)
- `sheet.grid` Karelaj (pending)
- `sheet.zoomIn` Yakınlaştır (partial)
- `sheet.zoomOut` Uzaklaştır (partial)
- `tool.stakeout` Aplikasyon (pending)
- `tool.subdivide` İfraz (pending)
- `workspace.disaster` Afet Analizi (pending)
- `workspace.plan3d` 3D Plan (pending)
