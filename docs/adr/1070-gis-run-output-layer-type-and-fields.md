# 1070 — CBS projesinde işlem çıktısı katmanının türü ve alan tanımları

Tarih: 2026-11-20. gis-usage-report.md GIS-001, GIS-002. Durum: kabul edildi.

İlgili: [ADR 0622](0622-interface-arrangement.md) §5 (katman türü), [ADR 0199](0199-layer-fields-and-feature-table.md) (katman alanları),
[ADR 0217](0217-layer-split-and-merge.md) §7 (işlemin katmanları), [ADR 0549](0549-processing-into-existing-geopackage.md) §4 (dosyaya yazılan
sütunlar), [ADR 0633](0633-dataset-metadata-and-lineage.md) (veri kümesi sözlüğü).

## Sorun

Kullanım denemesinde bir CBS alan katmanının Tamponu yeni bir katmana yazıldı. Yeni katmanın türü yoktu (Karışık): etkin
yapılınca Nokta aracı açıktı, katmana nokta kaydedildi ve yeniden açılışta aynı katmanda bir alan ve bir nokta vardı
(GIS-001). Tampon kaynak parkın değerlerini taşıdı ama alan tanımlarını taşımadı: kaynakta 0–20 tam sayı olan
`bank_sayisi` çıktıda 21'i kabul etti, görünen adlar kayboldu (GIS-002).

## Dayanak

- **QGIS**, Vector geometry › Buffer
  ([belgesi](https://docs.qgis.org/3.44/en/docs/user_manual/processing_algs/qgis/vectorgeometry.html#buffer), 20 Kasım
  2026'da okundu): çıktı “Buffered [vector: polygon]”, yani türü alan olan katmandır; girdinin alanları çıktıya geçer.
- **ArcGIS Pro**, Buffer
  ([belgesi](https://pro.arcgis.com/en/pro-app/latest/tool-reference/analysis/buffer.htm), 20 Kasım 2026'da okundu):
  çıktı poligon sınıfıdır; birleştirmesiz çıktı girdinin alanlarını taşır, uzaklık alanı (`BUFF_DIST`) sayıdır.
- **KentOS bugün:** katman türü yazımı çizimin bütünlük denetimi reddeder (ADR 0622 §5) ve türe uymayan çizim araçları
  etkin türlü katmanda kapalıdır; eksik olan yeni katmanın türü ve alanlarıdır.

## Karar

1. **Tür.** Bir CBS projesinde (`settings.workspace = gis`) işlemin açtığı yeni katman, işlemin ona yazdığı nesnelerin
   hepsini alan ilk türü alır; denenme sırası Nokta, Alan, Çizgi, Açıklama (yalnız daire ve elipsler alan sayılır).
   Hiçbir tür hepsini almıyorsa katman Karışık kalır. CAD ve karma projelerde yeni katman eskisi gibi türsüzdür.
   Kod: `kentos_processing::output_layer::geometry_of`.
2. **Alanlar.** Yeni katmana başka bir katmanın görünüşü ve alanları verilmemişse (`like`), nesnelerin taşıdığı her
   değer için:
   - girdilerin katmanlarında aynı adlı alan varsa o alan olduğu gibi (tür, görünen ad, aralık, zorunluluk, seçenekler)
     alınır; tekillik yalnız işlemin yazdığı değerler tekrarlanmıyorsa korunur (halkalı tamponda aynı park kodu her
     halkadadır);
   - işlemin kendi yazdığı değerlerin tanımını araç bildirir (`RunResult::fields`): Tampon'un Uzaklık'ı metre birimli
     ondalık sayı, Halka'sı 1'den büyük tam sayıdır;
   - ikisi de olmayan değer eskisi gibi tanımsız değer kalır; değerden tür tahmin edilmez.
   Kod: `output_layer::fields_of`.
3. **Var olan hedef.** Kullanıcı türlü bir katmanı hedef gösterdiğinde işlem hiçbir şey yazmadan önce denetlenir:
   uymayan ilk nesnenin türü ve katmanın türüyle reddedilir (`output_layer::misfit`), öneri yeni ya da türüne uyan
   katmandır.
4. **Geri alma ve kayıt.** Katman, türü ve alanları işlemin tek geri alma adımındadır; kayıt ve yeniden açma korur.
5. **Dosyaya yazım.** Yeni katman alan tanımları taşıdığından GeoPackage yazıcısı sütunları önce bu alanların sırasıyla
   (kaynak tablonun sırası) yazar (ADR 0549 §4); ortak durumlar buna göre yenilendi
   (`processing_command_cases.py`, `processing_source_cases.py`, `processing_batch_sink_cases.py`). Veri kümesi
   sözlüğü (`cad.datasets.describe`) türetilmiş katmanın alanlarını gösterir (`dataset_cases.py`).

## Sınama

- `crates/native/processing/tests/output_layer.rs`: alan katmanı türü, nokta almaması, kayıt sonrası tür ve alanlar,
  0 ve 20 kabul, 21 ve sayı olmayan metin ret, halkada tekilliğin düşmesi, CAD projesinde türsüz kalma, nokta katmanına
  tamponun yazmadan reddi.
- `apps/desktop/src/gis/processing/geometry_tests.rs::a_gis_buffers_layer_is_an_area_layer_that_turns_the_point_tool_off`:
  Tampon etkin katmanken Nokta kapalı, Kapalı alan açık; resim `.run/gui-evidence/gis-001/`.
- `fixtures/processing/v1/geometry.json`: Tampon durumlarının yeni katmanı Uzaklık (ve halkada Halka) alanlarıyla.

## Açık

- Web'in işlem çalıştırıcısı aynısını ister (TODOS.md §0.2).
- Çok kaynaklı işlemlerde (birleşim, kesişim) aynı adlı alanın iki kaynakta farklı tanımı ilk kaynağınkini alır;
  çakışmanın bildirilmesi ayrı iştir.
