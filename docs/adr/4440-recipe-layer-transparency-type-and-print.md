# 4440 — Tarifin katmanının saydamlığı, türü ve Yazdır'ı

Tarih: 2026-11-10. plan/map-cad.md MAP-20, sekizinci tur. Durum: taslak. ADR 4040'ın “Sonra”sındaki “Katmanın
saydamlığı, türü ve alanlarının tarife girmesi” (alanlar bu turda yok, bkz. Sınırlar).

İlgili:
- [ADR 0226](0226-symbol-layer-extras.md) §3, §4: katman çizimi (saydamlık, karışım kipi, maske);
- [ADR 0260](0260-object-look.md) §4: katmanın Yazdır'ı;
- [ADR 0622](0622-interface-arrangement.md) §5: katman türü (nokta, çizgi, alan, açıklama);
- [ADR 3640](3640-recipe-layers-and-stop-within-a-drawing.md): tarifin katmanları adıyla;
- [ADR 4040](4040-recipe-layer-groups-style-and-stop-within-drawings.md): tarifin katmanının grubu ve tam stili.

## Dayanak

- **AutoCAD ActiveX başvurusu**, [Mask Property](https://help.autodesk.com/cloudhelp/2021/ENU/AutoCAD-ActiveX-Reference/files/GUID-646DBB43-1FB2-473C-97C2-5FEE782C1340.htm)
  (10 Kasım 2026'da okundu): katman durumunun geri yüklenecek özellikleri `AcLayerStateMask` ile seçilir; aralarında
  renk, çizgi türü, çizgi kalınlığı, çizim stili ve `acLsPlot` (“Plotting on or off”, yazdırma açık ya da kapalı) var.
  Kaydetme her katmanın bütün özelliklerini saklar, maske yalnız neyin geri yükleneceğini seçer.
- **AutoCAD LT 2015**, [About Saving and Restoring Layer States and Settings](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT/files/GUID-64C21BD6-4DA6-48CE-9D81-CC98167BCA55.htm)
  (aynı gün okundu): katman durumu katmanın rengi, çizgi türü gibi ayarlarını ve kapalı, donmuş, kilitli oluşunu
  saklar; model uzayında kaydedilen durum düzende geri yüklenirken renk, çizgi türü, kalınlık, **saydamlık** ya da çizim
  stili görünüş penceresi üstüne yazımı olarak geri yüklenebilir. ADR 4040'ın okuması: Layer Translator'ın yeni katmanı
  da saydamlığı taşır.
- **QGIS API**, [QgsMapLayer::StyleCategory](https://api.qgis.org/api/classQgsMapLayer.html) (aynı gün okundu): stil
  kategorilere ayrılır; **Rendering** “scale visibility, simplify method, opacity”, **Fields** “aliases, widgets,
  WMS/WFS, expressions, constraints, virtual fields”, **Symbology** ayrı. Stil kaydedilirken ya da yüklenirken
  kategoriler seçilebilir.
- **QGIS 3.40 kullanıcı kılavuzu**, [Vector Layer Properties](https://docs.qgis.org/3.40/en/docs/user_manual/working_with_vector/vector_properties.html)
  (aynı gün okundu): Symbology'nin **Layer rendering** bölümünde katmanın saydamlığı (opacity) ve karışım kipi (katman
  ve nesne düzeyinde); stil katmanın bütün özelliklerinin birleşimidir, Style menüsü onların hepsini ya da bir kısmını
  dışa ve içe aktarır.
- **KentOS bugün** (ADR 4040):
  - tarif katmanın adını, görünüşünü, gruplarını, bütün stilini ve adladığı proje çizgi türünü taşıyordu;
  - katmanın çizimi (saydamlık, karışım kipi, maske), türü ve Yazdır'ı tarife girmiyordu. Tariften üret'in eklediği
    katman opak, karışık türlü ve yazdırılır çıkıyordu;
  - türlü bir katmanın tutmadığı nesne adımın sonundaki bütünlük denetimine takılıyordu: Tariften üret'in planı geçiyor,
    yazımı `layer_geometry_mismatch` ile reddediliyordu.

## Karar

1. **Tarifin katmanına üç alan** (`road.project.graph`, `RoadRecipeLayer`; ek alanlar, komut sürümü değişmez):
   - `compose`: katmanın saydamlığı (opaklık, 0–1) ve karışım kipi. **Maske tarife girmez:** maske projesinin öbür
     katmanlarını kimlikleriyle keser, başka projede o kimliklerin anlamı yoktur. Opak ve normal katmanda yazılmaz;
   - `geometry`: katmanın türü (nokta, çizgi, alan, açıklama). Karışık katmanda yazılmaz;
   - `plot`: yalnız `false` yazılır (katman ekranda çizilir, yazdırılmaz).
2. **Tariften üret eklediği katmana** (`road.project.produce`) bu üçünü de verir (stili, grupları ve çizgi türüyle
   birlikte, ADR 4040). Adıyla bulunan ya da `layers` ile verilen katman kendi görünüşünü korur (ADR 3640'ın kuralı).
3. **Söz:** `recipe_layer_created` uyarısı ve masaüstü planı eklenenleri katman panellerinin sözleriyle söyler:
   “Tarifteki gibi saydamlığı %40, karışım kipi Çoğalt, türü Çizgi ve yazdırılmaz.” (saydamlık = 1 − opaklık, yüzde).
   Cevabın biçimi değişmez: değerler tarifinkidir, tarif onları zaten söyler.
4. **Tür denetimi çizimden hemen sonra:** her çizimin yazdığı nesneler, gittikleri katmanın türüne göre denetlenir
   (katman eklenmiş, adıyla bulunmuş ya da verilmiş olsun). Tutmayan nesne bütün üretimi `layer_geometry_mismatch` ile
   reddeder (`path: layers`): “Tarifin “cz-1” çizimi (Sıfır hattı): “Kazı noktaları” bir nokta katmanı; çoklu çizgi bu
   katmana yazılmaz. Çizimin katmanını türüne uyan bir katman olarak layers ile verin ya da katmanın türünü Karışık
   yapın; hiçbir şey yazılmadı.” Plan da aynı yolu yürüdüğü için **plan da reddeder**. Bütün çizimler istenince de ret
   (çizim yazıldıktan sonra bulunur; ADR 3240'ın kuralı: denetimden sonraki ret hepsini reddeder).
5. **Retler** (`invalid_project_recipe`, `path`; çizimden önce): tarifin katman tanımında
   - maskeli katman çizimi;
   - boş katman çizimi (saydamlık da karışım kipi de yok);
   - 0–1 dışında opaklık.
6. **Masaüstü** (İş grafiği › Tariften üret…): plan ve sonuç eklenen katmanın aldıklarını söyler; nokta katmanına çizgi
   düşen tarifin planı nedenini söyler, Üret çalışmaz. Katman seçicinin açıklaması: “yoksa tarifteki stili, grupları,
   saydamlığı, türü ve Yazdır'ıyla eklenir”.
7. **Kapsam işareti:** ADR 3640 gibi **Harita + Yol**.

## Sınırlar

- **Alanlar tarife girmez:** katmanın alanları projenin değer listelerine (`domain`), bağlı tablonun sütunlarına
  (`column`), alt türlere bağlanabilir; zorunlu ve tekil kuralları çizimin nesnelerine uygulanır. Başka projede bunların
  karşılığı ayrı bir karar ister (QGIS'te de Fields ayrı bir stil kategorisidir).
- Maske tarife girmez (Karar 1). Katmanın görünür, kilitli ve donmuş oluşu oturumun durumudur, tanımı değildir; tarife
  girmez (AutoCAD'de de katman durumunun ayrı seçilebilen özellikleridir).
- Adıyla bulunan katmanın türü tarifinkinden başka olabilir; söylenmez, yalnız tutmayan nesne reddedilir.

## Bulunan

- Tariften üret'in planı türlü katmanın tutmadığı nesneyi görmüyordu: plan geçiyor, yazım adımın sonunda bütünlük
  denetiminden dönüyordu (bu projenin nokta katmanına tarifin sıfır hatları). Karar 4 ile ikisi aynı sözü söyler; yeni
  ortak durum bunu tutar.

## Kanıt

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/road_project_graph_cases.py`: tarifin katmanı kurulumun katmanının çizimini (maskesiz), türünü ve
    Yazdır'ını taşır. Yeni durum (23): %40 saydam, Çoğalt, etiketleri Drenaj'ı maskeleyen, yazdırılmayan çizgi katmanı.
    Eski 22 durum bayt bayt aynı;
  - `scripts/fixtures/road_project_produce_cases.py`: 64 durum (9 yeni), tarifler `fixtures/recipes/v1` altında elle;
    katman türünün tuttuğu nesneler ADR 0622 §5'ten elle:
    - eklenen katman saydamlığı, karışım kipi, türü ve Yazdır'ıyla, ikinci kez adıyla bulunur; planı;
    - adıyla bulunan katman kendi görünüşünü korur;
    - tarifin nokta katmanına sıfır hattının çizgileri: ret, plan da;
    - bu projenin nokta katmanı: ret, plan da (Bulunan);
    - üç ret.

  Eski 55 durum değişmedi (yalnız dosyanın notu). Rust geçer.
- **Python** (`python/tests/test_recipe_layer_look.py`), testin kendi okumasıyla:
  - A'nın tarifi saydamlığı ve karışım kipini (maskesiz), türü ve Yazdır'ı taşır;
  - B'de katman onlarla eklenir; çizgiler A'nınki, hepsi çizgi katmanının tuttuğu türden;
  - C'de adıyla bulunan katman kendi görünüşünü korur;
  - nokta katmanlı kopyası reddedilir, plan da; maskeli kopyası reddedilir.
- **MCP** (`crates/native/mcp/tests/recipe_layer_look.rs`): eklenen katmanın sözü; grafiğin tarifleri saydamlığı, türü ve
  Yazdır'ı taşır; nokta katmanı (plan ve yazım) ve maskeli tarif reddedilir.
- **Masaüstü** (`recipe_layer_look_tests`):
  - plan eklenecek katmanın aldıklarını söyler, Üret'e dek hiçbir şey yazılmaz; Üret katmanı tarifinki gibi ekler;
  - nokta katmanlı tarifin planı nedenini söyler, Üret hiçbir şey yazmaz.

  Resimler `.run/gui-evidence/tarif-katman-gorunusu-4440/`.

## Sonra

- Katmanın alanlarının tarife girmesi (değer listeleri, bağlı tablo ve alt türlerle birlikte): yapıldı,
  [ADR 4860](4860-stop-within-seven-drawings-and-recipe-layer-fields.md) (bağlı tablonun sütunu girmez).
- Bellek ve CPU bütçesi, gerçek veriyle ölçümle (ADR 3640'ın sınırı).
- Etiket, boykesit, korkuluk, menfez, kavşak, platform ve boru hattı profili çizimlerinin kendi hesabının içinde durması
  (ADR 4040 Sonra): yapıldı, ADR 4860.
- Web (TODOS §0.2).
