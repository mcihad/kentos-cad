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
| semantic | 129 | 284 | 44 | 48 | 505 |
| geometry | 1 | 5 | 1 | 2 | 9 |
| processing | 0 | 131 | 22 | 2 | 155 |
| none | 0 | 14 | 6 | 2 | 22 |
| query | 6 | 24 | 2 | 8 | 40 |
| session | 7 | 81 | 57 | 9 | 154 |
| host | 1 | 21 | 14 | 1 | 37 |
| **toplam** | 144 | 560 | 146 | 72 | 922 |

Kapsam: `cad` yalnız CAD şeridinde, `gis` yalnız CBS şeridinde, `shared` ikisinde, `none` hiçbir şeritte (kısayol, menü,
komut satırı).

## CAD tarafı: Python ve AI'ın arayüz gibi yapamadıkları

CBS'ye özgü olmayan 850 satırdan 42'i: geometry 7, none 20, processing 3, query 12.

### Sonucu çağıran hesaplar (geometry) — 7

| Öğe | Ad | Kapsam | Komut | Not |
|---|---|---|---|---|
| `cogo.update` | Kayıtlı ölçüleri çizimden yaz | shared | cad.entities.set |  |
| `crs.points` | Nokta listesi içe aktar… | shared | cad.entities.create |  |
| `tool.placeTextFile` | Metin dosyası yerleştir | cad | cad.entities.create `textFile` |  |
| `tool.subdivide` | İfraz | none | — |  |
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
