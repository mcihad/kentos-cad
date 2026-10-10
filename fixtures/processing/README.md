# İşlem araçlarının ortak durumları

İşlem araçlarının (İşlemler, [docs/PROCESSING.md](../../docs/PROCESSING.md)) iki platformda aynı sonucu vermesi için ortak durumlar. Her durum bir çizimde yerleşik bir aracı ya da modeli çalıştırır ve çalıştırmanın ne yaptığını platformdan bağımsız olarak söyler: yeni katmanlar, eklenen, değişen ve silinen nesneler, seçim, özet, iletiler, geri alma adımı ya da ret iletileri.

- **Web**: `apps/web/src/processing/cases.test.ts` (Vitest). Her durum `ProcessingRunner` ile sayfada, sonra işçinin yolundan (`handleJob`) bir kez daha çalışır; ikisi de beklenenle karşılaştırılır.
- **Masaüstü**: `crates/native/processing` (`kentos-processing`) aynı dosyaları oynatır; büyük işlerin yolu da (çizimin okuma kopyasında hesap, çizimde bitiş) her durumda denenir.

Çalıştırmalar ürün komutu değildir: katalogda kaydı ve `CommandResult`'ı yoktur, belgenin işlemiyle tek geri alma adımında yazar. Python ve yapay zekâ için ileride `cad.processing.run` onları saracak (CLAUDE.md §18).

| Dosya | İçerik |
|---|---|
| `v1/parcels.kcad` | Durumların çizimi (`.kcad` v1): Parsel katmanında yan yana üç parsel (1: 0…20, 2: 20…45, 3: 45…60 doğu; 0…30 kuzey; 1 ile 2 x = 20'yi, 2 ile 3 x = 45'i paylaşır; Ada, Parsel, Nitelik öznitelikleri, etiketleri parsel numarası), kilitli katmanda parsel 4, gizli katmanda parsel 5, Çizim'de çoklu çizgi 6 (30 m ve 15 m) ve 20 m'lik çizgi 7, Mevcut noktalar katmanında parsel 1'in üç köşesinde P00001–P00003 (8–10). Koordinatlar (487000, 4420000)'e göre verilmiştir |
| `v1/cases.json` | Durumlar |
| `v1/queries.kcad`, `v1/queries.json`, `v1/malikler.csv` | Mekânsal ve öznitelik sorgusunun durumları (ADR 0200): Konuma göre seç, İçindekinden ve Çevreleyenden bilgi al, Özet istatistik, Anahtarla birleştir, katmanın alanlarının kuralıyla yazma. Çizim, durumlar ve CSV `scripts/fixtures/spatial_query_cases.py`'nin bağımsız başvurusundan yazılır (`--check` farkı arar): parseller (Parsel katmanının alanları: Ada metin, Parsel ve Ağaç sayısı tam sayı, Taban toplamı 2, Taban ortalaması 1 basamaklı ondalık), biri delikli; yapılar, ağaçlar, iki yol, tapu kayıtları, kilitli katmanda bir parsel |
| `v1/geometry.kcad`, `v1/geometry.json` | Geometri işlemlerinin durumları (ADR 0201): Tampon, Kırp, Gruplayarak birleştir, Kesişim, Fark, Simetrik fark, Birleşim, Geçerliliği denetle, Onar, Sadeleştir, Koordinat sistemine dönüştür. Çizim ve durumlar `scripts/fixtures/geoprocess_cases.py`'nin bağımsız başvurusundan yazılır (`--check` farkı arar): parseller ve imar adaları, yollar, kotlu bir ağaç, hatalı geometriler (papyon, yinelenen köşe, taşan delik, kendini kesen yol), sadeleşecek sınırlar, ED50 / TM30 koordinatlı eski pafta; çizimin sistemi TUREF / TM30. Yeni nesneler ölçüleriyle beklenir (`addedShapes`) |
| `v1/proximity.kcad`, `v1/proximity.json` | Yakınlık analizinin durumları (ADR 0215): En yakını bul (alanlar, semt, en çok uzaklıkla boşaltılanlar), Uzaklık matrisi (Liste, Matris, Özet; merkezden merkeze), En yakın merkeze bağla, Komşu alanlar (ortak düz ve yaylı kenarlar, köşe, örtüşme; komşu sayısı ve adları), En kısa çizgi. Çizim ve durumlar `scripts/fixtures/proximity_cases.py`'nin, KentOS kodu olmadan (uzaklıklar ve en yakın noktalar kapalı biçimde, yaylı ortak kenar açı aralığıyla, örtüşen alanlar dikdörtgenlerin kesişimiyle); `--check` farkı arar |
| `v1/surface.kcad`, `v1/surface.json` | Yüzey analizinin durumları (ADR 0231): Eğim, Bakı, Gölgeli kabartma, Renkli kabartma, Eğrilik, Pürüzlülük, Güneşlenme, Eş yükselti eğrileri ve retler. Çizim ve durumlar `scripts/fixtures/surface_processing_cases.py`'den yazılır (`--check` farkı arar): yüzey analizinin örnek DEM'leri (`fixtures/terrain/v1`) bağlı raster olarak, biri eğik pikselli; sistem TUREF / TM30. Oynatıcılar aracı ev sahibinin dosyalarıyla çalıştırır (`rasters`), yazılanı bellekte toplar |
| `v1/raster-vector.kcad`, `v1/raster-vector.json`, `v1/raster-vector/*.tif` | Raster ve vektörün durumları (ADR 0234): Rasterleştir (nesnelerin kutusuna, bir rasterin ızgarasına; Bayt'a sığmayan değerin reddi), Rasterden alan, Rasterden çizgi (tek bantta Renk'in reddi), Rasterden nokta, Çizgi yakala, Alan kapat (delikleri korunan, sadeleşince değen, kenara ulaşan), Eğrilere kot ver (Aralık 0'ın reddi). Çizim, rasterler ve durumlar `scripts/fixtures/raster_vector_processing_cases.py`'den yazılır (`--check` farkı arar): sınıf, çizgi, yükseklik ve tarama rasterleri, iki parsel paftası, bir ızgara rasteri, yakılacak nesneler ve eğriler; rasterleri GDAL yazar. Oynatıcılar `rasters` ile, yüzey analizindeki gibi |
| `v1/hydrology.kcad`, `v1/hydrology.json`, `v1/hydrology/*.tif` | Hidrolojinin durumları (ADR 0235): Çukur doldur (taşma yüksekliği, dolgu derinliği, en küçük eğimle, çukursuz DEM), Akış yönü (ESRI, dolgusuz, TauDEM), Akış birikimi (D8 alan, uyarlanan üslü çoklu yön, D∞ özgül havza alanı; 0,1'den küçük üssün reddi), Topografik nemlilik indisi, Döküm noktası ve Noktadan havza (rasterin dışındaki ve aynı hücreye düşen noktalar), Havzalar (ana havzalar en küçük alanla, alt havzalar, güzergâhı kesen dereler; güzergâhsızın reddi), Dere ağı (eşikle ve kendiliğinden eşik). Çizim, rasterler ve durumlar `scripts/fixtures/hydrology_processing_cases.py`'den yazılır (`--check` farkı arar): bağımsız başvurunun DEM'leri (drenaj ağı, vadi, teraslar, koni), çıkış noktaları ve iki güzergâh; rasterleri GDAL yazar. Oynatıcılar `rasters` ile, yüzey analizindeki gibi |
| `v1/distance.kcad`, `v1/distance.json`, `v1/distance/*.tif` | Uzaklık ve maliyetin durumları (ADR 0236): Uzaklık yüzeyi (nesnelerden kutu ve payla, en yakın kaynak, rasterin ızgarasında en büyük uzaklıkla; rasterden, en yakın kaynak), Birikimli maliyet (16 ve 8 komşu, en ucuz kaynak, en büyük maliyet ve 64 bit, engeller, yükseklik modeliyle yüzey uzunluğu ve eğim; 0 maliyetin, modelsiz anahtarın ve engeldeki kaynağın reddi), En düşük maliyetli yol (16 ve 8 komşu, sadeleştirme, yükseklik modeliyle; rasterin dışındaki varış söylenir), Maliyet koridoru (yüzde eşiği, en ucuz yoldan küçük değer eşiği). Çizim, rasterler ve durumlar `scripts/fixtures/distance_processing_cases.py`'den yazılır (`--check` farkı arar): bağımsız başvurunun maliyet, engelli maliyet, yükseklik, kaynak ve sıfır maliyet rasterleri, bir ızgara rasteri, kaynaklar, varışlar ve uçlar; rasterleri GDAL yazar. Oynatıcılar `rasters` ile, yüzey analizindeki gibi |
| `v1/suitability.kcad`, `v1/suitability.json`, `v1/suitability/*.tif` | Uygunluk analizinin durumları (ADR 0237): Bulanık üyelik (Doğrusal, Gauss, Büyük; iki rasterin reddi), Bulanık çakıştırma (Ve, Gamma; 0–1 dışındaki üyelik söylenir; tek rasterin reddi), Ağırlıklı toplam (ağırlıklarla, yazılmayan ağırlık 1; sınırın dışındaki ağırlıkta alanın sorunu), Ağırlıklı çakıştırma (etkiler ve sınıf tabloları, kısıt ve boş; toplamı 90 olan ve eksik etkinin reddi), İkili karşılaştırma (ağırlıklı toplamla, dört ölçütün yalnız ağırlıkları, tutarsız karşılaştırmalar; tek rasterin reddi), ROC ile doğrulama (bütün hücreler, yokluk nesneleri, düşük değer daha olası; rasterin dışındaki varlığın reddi). Çizim, rasterler ve durumlar `scripts/fixtures/suitability_processing_cases.py`'den yazılır (`--check` farkı arar): bağımsız başvurunun ölçütleri, sınıflanacak ölçütleri, değer, üyelik ve duyarlılık rasterleri, varlık, yokluk ve rasterin dışındaki nesneler; rasterleri GDAL yazar. Oynatıcılar `rasters` ile, yüzey analizindeki gibi |
| `v1/spatial-stats.kcad`, `v1/spatial-stats-geo.kcad`, `v1/spatial-stats.json` | Mekânsal istatistiğin durumları (ADR 0238): Ortalama merkez (düz; Ağırlık ve Tür'e göre, okunamayan ve olmayan ağırlık, (boş) grubu), Ortanca merkez (Tür'e göre), Standart uzaklık (2 kat), Yön dağılımı (Tür'e göre; tek yerli grup elips vermez), En yakın komşu (kutudan ve verilen alanla; tablo, çizim değişmez), Moran I (kendiliğinden bant; 4 en yakın komşu, standartlaştırmasız), Sıcak nokta (parsellerin ve kazaların kopyaları z, p, güven sınıfı ve rengiyle; katmanın kategorili görünüşü), DBSCAN ve DBSCAN*, k-ortalamalar; retler: coğrafi proje (WGS 84 çizimi), üç parselle Moran I, 50 kümeyle k-ortalamalar; değer alanı seçilmemiş Moran I. Çizimler ve durumlar `scripts/fixtures/spatial_stats_processing_cases.py`'den yazılır (`--check` farkı arar); beklenenler bağımsız başvurunun (`spatial_stats_cases.py`) hesabıdır: kazalar (üç grup ve aykırılar; No, Ağırlık, Tür), 6 × 5 parsel (Değer), boş Çizim katmanı. Yeni katman girdinin katmanının hemen üstündedir (`layerAbove`) |
| `v1/dialog.json` | İşlem penceresinin davranışı: formlar, oturumlar ve saf kuralların tabloları ([aşağıda](#pencere-kentosprocessing-dialog-sürüm-1)) |
| `v1/designer.json` | Model tasarımcısı: modelin düzenlemeleri adım adım ve her adımdan sonraki denetim, tasarımcının bir modelden okudukları, sözleri ve diyagramın geometrisi ([aşağıda](#model-tasarımcısı-kentosmodeldesigner-sürüm-1)) |

## Biçim (`kentos.processing-cases`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.processing-cases"`, `1` |
| `tolerance` | Koordinatların karşılaştırılacağı mesafe, metre (`1e-9`): iki platform aynı Rust çekirdeğini çağırır |
| `measureTolerance` | İsteğe bağlı (`geometry.json`): `addedShapes`'in alan (m²) ve uzunluk (m) payı |
| `documents` | Çizim başına `defaults`: araçların çizimden aldığı varsayılanlar (`DefaultsContext`: uzunluk ve alan ondalığı, açı birimi, çizim ölçeği, çizim yazı tipi, etkin katman) ve `tools`: her aracın ve modelin o çizimdeki varsayılan değerleri, pencerenin açtığı gibi |
| `files` | İsteğe bağlı: dosya parametresinin (ADR 0200 §7) değerinde yazılan ad → bu klasördeki dosya. Oynatıcı dosyayı pencerenin okuduğu gibi Tablo ekle'nin okuyucusuyla okur (ilk sayfa) ve değeri `{ name, rows }` yapar |
| `rasters` | İsteğe bağlı (`surface.json`): bağlı rasterin dosya adı → bu klasöre göre dosya. Oynatıcının ev sahibi rasteri oradan okur (masaüstünde `Files::open_raster`, web'de `RasterRunHost`), çıktının yolu yalnız addır, yazılanı bellekte tutar |
| `cases` | Durumlar |

Bir durum:

| Alan | Anlamı |
|---|---|
| `id`, `title` | Kimlik ve Türkçe açıklama |
| `document` | Çalıştırmadan önce açılan çizim. Açılınca geri alma geçmişi boştur |
| `selection` | Çalıştırmadan önceki seçim (nesne kimlikleri); `selection` kapsamı ve İfadeyle seç'in biçimleri bunu okur |
| `view` | İsteğe bağlı: `visible` kapsamının kutusu, `[minX, minY, maxX, maxY]` mutlak metre; yoksa görünüm yoktur |
| `run` | `{ "tool": "<kimlik>" }` ya da `{ "model": "<kimlik>" }` |
| `values` | Aracın o çizimdeki varsayılanlarının üstüne yazılan değerler, pencerenin verdiği biçimde (`{ "scope": "selection" }`, `{ "layerId": "mevcut" }`, `{ "newName": "Numaralar" }` …) |
| `expect` | Beklenen sonuç |

Beklentiler:

| Alan | Anlamı |
|---|---|
| `status` | `ok`, `invalid` (çalışmadı, değerlerde sorun var) ya da `error` |
| `issues` | `invalid`'de sorunlar, parametre sırasıyla: `param` (parametrenin adı; aracın kendi kuralında yok) ve `message`, tam metin |
| `message` | `error`'da ileti |
| `summary` | `ok`'ta çalıştırmanın özeti, tam metin (geçmişte ve pencerenin alt çubuğunda görünen satır) |
| `log` | Aracın ve çalıştırıcının iletileri, sırasıyla: `{ level, text }` (`info`, `warn`); yazılmazsa hiç ileti yoktur |
| `undo` | Çalıştırmanın tek geri alma adımının adı (aracın adı, modelde modelin adı); `null`: çizim değişmedi, geri alınacak adım yok |
| `layers` | Oluşturulan katmanlar: `id`, `name`, `style` (aracın yeni katman stili, yeni katmanın varsayılanlarının üstüne) |
| `added` | Eklenen nesneler, kimlik sırasıyla; her biri kimlikleri dışındaki bütün alanlarıyla (`kind`, `layerId`, geometri, `label`, `text`, `height`, `rotation`, `attrs` …); geometri alanları (`p`, `a`, `b`, `c`, `pts`, `holes`, `r`, `major`, `ratio`) `tolerance` içinde, öbürleri tam |
| `addedShapes` | `added`'in yerine (geometri işlemleri, ADR 0201): eklenen nesneler kimlik sırasıyla, ölçüleriyle: `kind`, `layerId`, `attrs` tam; `parts`, `holes` (çekirdeğin `geoMeasure`'u, masaüstünde `measured`), `area` ya da `length` `measureTolerance` içinde, `points` (nokta nesnesinin noktaları) ve `vertices` (alanın halkası sonra delikleri, yolun ya da noktanın köşeleri) `tolerance` içinde; nesnenin bunlardan ve geometri alanlarından başka alanı olmaz. Örtüşmenin köşe sırası bağımsız başvurudan öngörülemediği için geometri ölçüleriyle denetlenir; ölçülerin doğruluğu ADR 0149'un bağımsız denetimindedir |
| `updated` | Değişen nesneler: `id`, bütün `attrs` ve varsa `label` |
| `removed` | Silinen nesnelerin kimlikleri |
| `selection` | Çalıştırmadan sonraki seçim |
| `outputs` | Aracın çıktılarından yazılanlar: sayılar, kimlik listeleri ve tablolar (`{ columns, rows }`, metinler) tam |
| `rasterOf` | Yazılan raster dosyaları (yüzey analizi, ADR 0231): ad → yüzey analizinin bağımsız başvurusunda (`fixtures/terrain/v1/cases.json`, tepe) bir durumun adı; dosyanın 0. katı o durumun değerleridir (32 bitte en çok bir birim son basamakta, baytta tam) ve başka dosya yazılmamıştır |
| `contoursOf` | Eklenen nesneler eğri başvurusunun (`fixtures/contours/v1/cases.json`) bu sıradaki durumunun eğrileridir, sırasıyla: çoklu çizgi, köşeleri `tolerance` içinde, her köşenin kotu düzey, `Kot` durumun yazısı, `Tür` Ana ya da Ara, ana eğride `lineWeight` 0,35 (`added`'in yerine) |
| `layerAbove` | Yeni katman → katman: yeni katman o katmanın hemen üstündedir (aynı grup, bir önceki yer; raster dosyaları ve `spatial-stats.json`) |
| `layerBelow` | Yeni katman → katman: yeni katman o katmanın hemen altındadır (aynı grup, bir sonraki yer; interpolasyon, ADR 0232) |
| `rasterVectorOf` | Yazılan raster dosyaları (Rasterleştir, ADR 0234): ad → raster ve vektörün bağımsız başvurusunda (`fixtures/raster-vector/v1/cases.json`) bir durumun adı; dosyanın 0. katı o durumun `raster.values`'ıdır, bit bit (`null` değersiz); başka dosya yazılmamıştır |
| `elevations` | Değişen nesnelerin köşe kotları (Eğrilere kot ver, ADR 0234): kimlik → nesnenin yollarının kotları, yolların sırası çekirdeğin `elevation::paths`'i (halka, sonra delikler ve parçalar), `null` kotsuz köşe |
| `distanceOf` | Yazılan raster dosyaları (uzaklık ve maliyet, ADR 0236): ad → uzaklık ve maliyetin bağımsız başvurusunda (`fixtures/distance/v1/cases.json`) bir durumun adı; dosyanın 0. katı o durumun `raster.values`'ı, durumun kuralıyla (`exact` bit bit, `f32ulp` 32 bitte en çok bir birim son basamakta); başka dosya yazılmamıştır |
| `suitabilityOf` | Yazılan raster dosyaları (uygunluk analizi, ADR 0237): ad → uygunluk analizinin bağımsız başvurusunda (`fixtures/suitability/v1/cases.json`) bir durumun adı; dosyanın 0. katı o durumun `raster.values`'ı, durumun kuralıyla (`exact` bit bit, `f32ulp` ve `f64ulp` 32 bitte en çok bir birim son basamakta); tam sayı sonucun nodata'sı (−2 147 483 648) değersizdir; başka dosya yazılmamıştır |
| `remoteOf` | Yazılan raster dosyaları (uzaktan algılama, ADR 0242): ad → uzaktan algılamanın bağımsız başvurusunda (`fixtures/remote/v1/cases.json`) bir durumun adı; dosyanın 0. katı (bantlar iç içe) o durumun `raster.values`'ı, durumun kuralıyla (`exact` bit bit; `sum` ondalık sonuçta 32 bitte en çok bir birim son basamakta, tam sayı sonuçta en çok bir birim); tam sayı sonucun değersiz hücresi nodata değerini taşır; başka dosya yazılmamıştır |
| `hydrologyOf` | Yazılan raster dosyaları (hidroloji, ADR 0235): ad → hidrolojinin bağımsız başvurusunda (`fixtures/hydrology/v1/cases.json`) bir durumun adı; dosyanın 0. katı o durumun `raster.values`'ı, durumun kuralıyla (`exact` bit bit, `f32ulp` 32 bitte en çok bir birim son basamakta); başka dosya yazılmamıştır |
| `interpolationOf` | Yazılan raster dosyaları (interpolasyon ve yoğunluk, ADR 0232): ad → interpolasyonun bağımsız başvurusunda (`fixtures/interpolation/v1/cases.json`) bir durumun adı; dosyanın 0. katı o durumun `values`'ı, iki bantta ikinci bant `error`'u (32 bitte en çok bir birim son basamakta); başka dosya yazılmamıştır |

## Karşılaştırma kuralları

- Metinler (özet, iletiler, nesnelerin yazıları ve öznitelikleri) tam eşit olmalıdır.
- Eklenen bir nesnenin alan kümesi beklenenle aynı olmalıdır; fazla ya da eksik alan farktır. Yalnız geometri alanları (`p`, `a`, `b`, `c`, `pts`, `holes`) `tolerance` içinde karşılaştırılır, öbür sayılar tam.
- `ok` ve `undo`'su olan her durumda, durum yazmasa da: geri alma adımın adını verir, çizim (nesneler ve katman ağacı) çalıştırmadan önceki hâline döner, başka adım kalmaz; yineleme sonucu geri getirir.
- `undo: null` olan durumda geri alınacak adım yoktur.
- Çalıştırıcı değişiklik kümesini uygulamadan önce öznitelik yazmalarını katmanın alanlarının kuralıyla denetler (ADR 0199 §1, 0200 §3): alana yazılan değer alanın tek biçimine çevrilir; uymayan ilk değer bütün çalıştırmayı `error` ile reddeder: "Öznitelik yazılamadı (#id): <alanın nedeni>" (yeni nesnede "(yeni nesne)"). Değişen anahtarlar kod noktası sırasıyla denetlenir; kaldırılan öznitelik boş sayılır. Araç kendisi de reddedebilir (`refused`: dosyada anahtar sütunu yok): ileti olduğu gibi, çizim değişmez.

## Beklenenlerin kaynağı

Sayılar, adlar, özetler ve ret iletileri araçların kuralından okunur ([docs/PROCESSING.md](../../docs/PROCESSING.md) §4, §11): köşe noktaları parsellerin köşelerindedir, adlar biçimden gelir (`P` + `00001`), ortak köşe ve kenar bir kez sayılır. Köşe ve kenar yazılarının yeri ve dönüşü çekirdeğin cevabıdır (`crates/shared/geometry-core/src/processing`: `corner_text_at`, `edge_lengths`); doğrulukları çekirdeğin kendi testlerindedir, burada iki platformun aynı yeri vermesi sabitlenir. Yerler denetlenmiştir: kenar yazısı kenarın ortasında, dışa (açık çizgide sola) 0,8 m, dikey kenarda 90°; köşe yazısı köşenin dış açıortayında.

## Kurallar

- Durumlar web'in bugünkü davranışını yazar; web geçmeden durum eklenmez.
- Bir araç değişince durumu, iki çalıştırıcı ve bu belge birlikte değişir. Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
- Yeni bir yerleşik araç ya da model en az bir başarılı durum ve bir ret durumuyla gelir.

## Pencere (`kentos.processing-dialog`, sürüm 1)

`v1/dialog.json`, işlem penceresinin (web: `ui/processing/ToolDialog.ts` ve `paramFields.ts`) ne gösterdiğini ve kullanıcının her işinde nasıl değiştiğini tutar. Kurallar sayfasızdır: `apps/web/src/ui/processing/dialogPlan.ts` (form, bölümler, sorunlar, alt satır, yerler, durum), `fieldPlan.ts` (alanlar) ve `dialogTexts.ts` (sözler); pencere onlardan çizer.

- **Web**: `apps/web/src/ui/processing/dialogFixture.test.ts` oynatır (oynatıcı `dialogFixture.ts`). Kaydedici `apps/web/scripts/fixtures/record-processing-dialog.test.ts`, yalnız bilerek: `GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-processing-dialog.test.ts`. Kaydedilen cevaplar okunmuştur; yeniden yazmak farkı okunacak bilinçli bir değişikliktir.
- **Masaüstü**: araç penceresinin planı (`apps/desktop/src/processing/plan.rs`) dosyanın `status` ve `targets` tablolarını oynatır; formlar, oturumlar ve öbür tablolar web'in pencere kurallarını sabitler, masaüstü penceresi aynı kurallarla çizer.

| Alan | Anlamı |
|---|---|
| `document` | Oturumların çizimi (`parcels.kcad`) |
| `texts` | Pencerenin metinleri; bir değerden yapılan metin `{ sample, text }` (ör. `fixFields`: 2 → "Çalıştırmadan önce 2 alanı düzeltin.") |
| `scopes` | Kapsam düğmelerinin adları (Seçili, Görünen, Tümü, Katman) |
| `targetLabels`, `targetShort` | Web'in yer adları ("Bu tarayıcıda", "arka planda"). Masaüstü yerleri kendi sözüyle adlandırır (ADR 0084: "Bu bilgisayarda", "Arka planda"); seçeneklerin `value`, `note`, `disabled` ve `checked`'i ortaktır |
| `forms` | Her yerleşik aracın ve modelin formu, pencerenin açıldığı kimlikle (`model:` önekli): `title`; `rows` (her parametre: `name`, `label`, `description`, `optional` "isteğe bağlı", `stacked` denetim etiketin altında mı, `control`); `side` (kategori yolu ve simgesi, aracın simgesi, açıklaması, yardım paragrafları, modelde sırasıyla adımlar ve düzenleme düğmesi, Önizleme bloğu var mı, takma adlar) |
| `sessions` | Oturumlar |
| `status` | Alt satır ve düğmeler: çalıştırmanın durumu, Çalıştır'a basıldı mı, sorunlar → `status`, `footer` |
| `targets` | Nerede çalışır: aracın bildirdiği yerler, bu evde olanlar, Otomatik'in şimdi seçeceği, model mi, saklanan seçim → seçenekler, ipucu, çalıştırmanın alacağı seçim |
| `restore` | Saklanan değerlerin (son çalıştırma) varsayılanların üstüne geri yüklenmesi: değerler, Gelişmiş'in açık başlaması ve doğrulamanın iletileri |
| `kinds` | Tür çipleri: önce, tıklanan türle yeni değer, sonra |
| `tokens` | Alan adının ifadede yazılışı: çıplak (`Ada`) ya da köşeli parantezde (`[Tapu alanı]`, ayrılmış sözcükler Türkçe katlanarak: `[ve]`, `[Değil]`) |
| `inserts` | İmleçte ekleme (alan çipi, değişken, işlev): metin, seçim, eklenen → yeni metin ve imleç. Önündeki metin boşluk, "(" ya da "," ile bitmiyorsa araya boşluk girer |
| `numbers` | Sayı alanının metni → değer: nokta ya da virgül ondalık; okunamayan `null` (web'de NaN; doğrulama "için geçersiz değer." der) |
| `icons` | İfade satırının simgesi: satır " yok." ya da " boş." içeriyorsa `info`, değilse `check` |

Formdaki denetimler (`control.type` parametrenin türüdür): `features` kapsam düğmeleriyle; `number` birimiyle; `string` (`short`: en çok 2 karakter); `boolean` anahtar; `enum` en çok üç seçenek ve her etiket en çok 22 karakterse düğmeler (`segmented`), yoksa açıklamalı liste (`dropdown`); `layer`; `point`; `field` yeni ad yazılabiliyorsa yazı ve liste (`combo`, "Alan adı"), yoksa yalnız liste (`dropdown`, "Alan seçin"), birden çok ad alıyorsa `multiple` (liste adları işaretler, adlar virgülle yazılır); `expression`; `file` (ADR 0200 §7) Dosya seç… düğmesiyle, `accept`: sunulan uzantılar; `rasterValues` (ADR 0237 §9) `of` parametresinin rasterlerine birer satır, `cell` `number` (`min`, `max`) ya da `text`, `placeholder`; `rasterPairs` `of` parametresinin rasterlerinin her çiftine 17 seçenekli liste. İkisi de etiketin altında (`stacked`).

### Oturum

| Alan | Anlamı |
|---|---|
| `id`, `title` | Kimlik ve Türkçe açıklama |
| `open` | `{ "tool": … }` ya da `{ "model": … }` |
| `selection` | Pencere açılmadan önceki seçim |
| `view` | İsteğe bağlı: görünen alan `[minX, minY, maxX, maxY]`, mutlak metre |
| `available` | Bu evin çalıştırabildiği yerler: masaüstü bugün `["client"]`, web `["client", "worker"]` |
| `given` | Pencereye verilen değerler (geçmişin "Yeniden aç"ı); varsa son değerlerin yerine geçer |
| `last` | Aracın son çalıştırmasının değerleri |
| `choice` | Saklanan yer seçimi; yoksa Otomatik |
| `opened` | Açılınca görünen (bütün görünüş) |
| `steps` | Adımlar: `do` (kullanıcının işi) ve `expect` (bir önceki görünüşten değişenler); `pickObjects` adımında ayrıca `picking` ve `after` (aşağıda). Dosya alanının dosyası `choose` ile verilir: değer `{ name, rows }` (pencerenin seçilen dosyadan yaptığı); son değerlerde dosyanın yalnız adı (masaüstünde yolu) kalır |

Kullanıcının işleri (`do`):

| İş | Anlamı |
|---|---|
| `choose` `{ name, value }` | Seçmek: düğme, anahtar, listeden satır. Seçili olanı yeniden seçmek hiçbir şeyi değiştirmez |
| `type` `{ name, text }` | Yazmak: sayı alanı metni sayı okur (`numbers`), hedef katman alanı yeni katmanın adı olarak (`{ newName }`), öbürleri metin olarak. Değer aynı kalsa da alana dokunulmuş sayılır |
| `scope` `{ name, scope }` | Kapsam düğmesi; Katman etkin katmanla başlar; tür süzgeci kalır; seçili kapsama basmak değiştirmez |
| `scopeLayer` `{ name, layerId }` | Katman kapsamının listesinden bir katman |
| `kind` `{ name, kind }` | Tür çipi: tür çıkar ya da girer; kapsamdaki bütün türler yeniden alınınca süzgeç kalkar |
| `toggleAdvanced` | Gelişmiş ayarlar'ı açıp kapamak |
| `target` | Nerede çalışır'da bir seçenek |
| `reset` | Varsayılanlar |
| `run` | Çalıştır; sorun yoksa çalıştırma bitene kadar sürer |
| `undo` | Çalıştırmadan sonra alt satırdaki "Geri al": çizimin son adımı geri alınır |
| `pick` `{ name, point }` | Nokta alanının Sahneden seç'i: gösterilen nokta ya da `null` (vazgeçti). Pencere olduğu gibi döner; nokta bir seçim gibi yazılır |
| `pickChoice` `{ name, point }` | Bir seçeneği çizimdeki nokta olan seçimin (`picks`, ADR 0088) yanındaki Sahneden seç: nokta, nokta parametresine yazılır ve seçim o seçeneğe geçer; ikisine de dokunulmuş sayılır. `null` (Esc) hiçbir şeyi değiştirmez |
| `pickObjects` `{ name, tolerance, actions, end }` | Girdi nesnelerinin Sahneden seç'i (ADR 0088), tek adımda. Pencere kenara çekilir; seçim saklanır ve boşalır. `actions` sırayla: `click` (nokta; `tolerance`, dünya biriminde seçme açıklığı: ekrandaki açıklık bölü görünümün ölçeği) ya da `box` (`from`, `to`; sağdan sola çizilen kesişim, öbürü pencere). `end`: `done` (Enter, Boşluk ya da hızlı sağ tık) ya da `cancel` (Esc). Alınan türler değerin `kinds`'i, yoksa parametreninki. Tıklama: imlecin altındaki en belirli nesne alınan türdense o; değilse alınan türlerden kenarı erimde en yakın olan; o da yoksa tıklananın içinde olduğu en küçük kapalı şekil (deponun `enclosing`'i), türü alınıyorsa; seçimdeyse çıkar. Kutu: içindekilerden alınan türler eklenir. `done` ve en az bir nesne: alan `{ scope: 'selection' }` olur (tür süzgeci kalır), dokunulmuş sayılır, günlüğe "n nesne seçildi." yazılır. Yoksa (`cancel` ya da hiç nesne): pencere olduğu gibi, önceki seçim geri gelir |

`pickObjects` adımının çizim tarafı:

| Alan | Anlamı |
|---|---|
| `picking` | Seçim (kimlikler, küçükten büyüğe) ve komut satırı (`prompt`: "Alanlar: nesneleri tıklayın ya da pencereyle seçin (n seçili) [Bitti (Enter) / Vazgeç (Esc)]"): seçim başlarken, sonra her tıklama ya da kutudan sonra |
| `after` | Pencere döndüğünde seçim; nesneler alındıysa günlüğün satırı (`said`) |

Görünüş:

| Alan | Anlamı |
|---|---|
| `values` | Değerler, çalıştırmanın alacağı gibi |
| `sections` | `groups`: Girdi (`features`), Ayarlar (öbürleri), Çıktı (`layer`); satırı olanlar, bu sırayla, satırlar parametre adları. `advanced`: Gelişmiş ayarlar'ın satırları ve açık mı (kullanıcı açtıysa ya da satırlarından birinde sorun görünüyorsa açık); görünen gelişmiş parametre yoksa `null` |
| `fields` | Görünen alanların (Gelişmiş'inkiler açıkken) değişen parçaları; sayı, metin, anahtar ve seçim alanları yalnız değerlerini gösterir. Kapsam alanı: seçili düğme (`scope`; önceki adımın çıktısı ilk kapsam görünür), Katman kapsamında katman listesi ve yazısı (olmayan katman "—"), ne okunduğu (`count.text`, boşsa `count.empty` ve uyarı simgesi), tür çipleri (`chips`: tür, ad, sayı, basılı mı, ipucu; kapsamda iki ya da daha çok tür varsa ya da süzgeç varken) ya da aracın uygun türleri notu. Hedef katman: yazısı (seçili katman; yeni adda "(mevcut)" var olan katmanın adıyla, ya da "(yeni)"), yeni katman adı alanı (`name`; mevcut katman seçiliyken `null`), liste (başlıklar, "Yeni: …", katmanlar yollarıyla; kilitliler kapalı ve "kilitli"). Nokta: `text`, `button`, `shown`. Öznitelik alanı: `text`, not (`note`: yazılan alanda "n nesnede var; değeri değişir.", okunan alanda "n nesnede var.", dosyanın sütununda "n satırda var.", "Yeni alan: nesnelere eklenir.", "Bu nesnelerde böyle bir alan yok.", boş ad için boş; `multiple`'da kaynakta olmayan ilk ad için "“ad” kaynakta yok."), liste (alanın kaynağının adları: nesnelerin alanları en çok bulunandan, dosyanın sütunları sırasıyla, ipucu "n nesne" ya da "n satır"; alan yoksa kapalı tek satır). Alanın kaynağı `of`'tur; bir liste ise görünen ilki (katman ya da dosya kaynağı). Dosya: `text` (adı ya da "Dosya seçilmedi"), `note` ("n satır, m sütun", içeriği olmayan değerde "Yeniden seçin: dosyanın içeriği saklanmaz."), `button` ("Dosya seç…", seçiliyken "Başka dosya…"), `chosen`. Kapsam alanının tür notu: araç neredeyse her türü alıyorsa alınmayanlar ("Yardımcı çizgi, ışın alınmaz."). İfade: alan çipleri (ilk 6: ad, yazılışı, ipucu), kalanlar `more` ("+n"), satır (`preview`: simge ve metin; ifade boşken ya da hatalıyken `null`) |
| `issues` | Alanların altındaki sorunlar: dokunulmuş alanınki hemen, Çalıştır'dan sonra bütün alanlarınki (başarılı çalıştırmaya ya da Varsayılanlar'a kadar). Çalıştırıcının çalıştırmadan önce bulduğu (seçim boş, girdinin hepsi kilitli) bir değer değişene ya da yeni çalıştırmaya kadar alanında durur |
| `preview` | Yan paneldeki Önizleme: aracın önizlemesi, bir alanda sorun varken (görünmese de) "Önizleme için alanları düzeltin."; önizlemesi olmayan araçta `null`. `muted`: gösterecek önizleme yok |
| `status` | Alt satır: `kind` (`idle`, `running`, `ok`, `warn`, `error`), `icon`, `text`; çalışırken `progress` (0–100); başarıda `actions`: `zoom` "Seçime yakınlaştır" (araç seçti), `select` "Sonuçları seç" (`pick`: eklenenler, yoksa değişenler ya da seçilenler), `undo` "Geri al" (çizim değişti). Uyarı: Çalıştır'dan sonra aracın kendi kuralı, yoksa "Çalıştırmadan önce n alanı düzeltin.", yoksa çalıştırıcının iletisi |
| `result` | Son çalıştırmanın tablo çıktısı (Özet istatistik; ADR 0200 §7): `columns`, `rows`; formun altında Panoya kopyala (sekmeyle ayrılmış) ve CSV olarak kaydet ile. Yoksa ya da bir değer değişince `null` |
| `footer` | Çalıştır (çalışırken "Çalışıyor…" ve kapalı), Kapat (çalışırken "Durdur": çalıştırmayı durdurur), Varsayılanlar (çalışırken kapalı) |
| `targets` | `options` (`value`, `label`, `note`, `disabled`, `checked`), `hint` (Otomatik seçiliyken ipucu), `choice` (çalıştırmanın alacağı seçim). Birden çok yer varsa önce Otomatik ("şimdi: …"; modelde "adım adım"); sonra aracın bildirdiği her yer: bu evde varsa seçenek (tek yerse "bu çalıştırmada" notuyla, işaretli), yoksa "yakında" ve kapalı. Modelde bildirilen yerler adımlarının bu evdeki yerleridir. Saklanan seçim bu evde yoksa Otomatik (tek yerde o yer) |

`expect` yalnız değişenleri yazar: bir parça değiştiyse bütünü; `values` ve `fields` ad ad, artık görünmeyen alan `null`. Beklenen görünüş, bir öncekinin üstüne bunlar konarak bulunur.

Pencerenin öbür kuralları (görünüşte yok):

- **Açılış:** verilen değerler, yoksa son değerler, varsayılanların üstüne; uymayan değer varsayılana döner (`restore`). Gelişmiş ayarlar, görünen bir gelişmiş değer varsayılanından farklıysa açık başlar.
- **Klavye:** metin alanında Enter, her yerde Ctrl+Enter çalıştırır. Açılışta ilk metin ya da sayı alanı (yoksa seçili düğme) odaklanır; başarısız denemeden sonra ilk sorunlu alan, çalıştırmadan sonra Çalıştır.
- **Sonuçları seç** seçimi `pick` yapar, pencereyi kapatır ve seçime yakınlaştırır; **Seçime yakınlaştır** seçime dokunmadan aynısını yapar.
- **Durdur** çalıştırmayı durdurur; iptal edilen çalıştırmanın satırı `error`'dur ("İşlem iptal edildi; çizim değişmedi.").
- Oturumların çalıştırmaları `cases.json`'daki gibi gerçek çalıştırmalardır; özetleri ve seçtikleri iki platformda aynı çalıştırıcıdan gelir.
- İfade satırlarının metinleri ve ifade hataları dilin çekirdeğindendir (`model/expression`); dil değişince bu dosya kaydediciyle yeniden yazılır ve farkı okunur.

## Model tasarımcısı (`kentos.modelDesigner`, sürüm 1)

`v1/designer.json`, model tasarımcısının kurallarını ve sözlerini tutar. Davranışın bütünü [docs/specs/model-designer.md](../../docs/specs/model-designer.md)'dedir. Kurallar sayfasızdır: modelin düzenlemeleri `apps/web/src/processing/modelEdit.ts`'te, tür uyumu, denetim ve sıra `processing/model.ts`'te, tasarımcının sözleri, kuralları ve diyagramın geometrisi `ui/processing/model/designerPlan.ts`'te durur. Pencere, diyagram, parçalar ve ayarlar (`ModelDesigner.ts`, `ModelCanvas.ts`, `modelPalette.ts`, `modelInspector.ts`) onlardan çizer.

- **Web**: `apps/web/src/ui/processing/model/designerPlan.test.ts` oynatır.
- **Kaydedici**: `python3 scripts/fixtures/designer_cases.py`; `--check` hiçbir şey yazmadan karşılaştırır. Sözler, tablolar ve dosyanın araçları web'den elle yazılmıştır; cevaplar koddan ayrı, betikte bulunur.
- **Masaüstü**: tasarımcı aynı dosyayı oynatır.

Dosya kendi araçlarıyla çalışır (`tools`, kimlikleri `t.` önekli). Web'in `ParamDef`'i biçimindedirler; `visibleWhen` veri olarak yazılır: `{ param, equals }`, “o parametrenin bilinen değeri buysa görünür” demektir. Böylece durumlar yerleşik araçlar değişince bozulmaz.

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.modelDesigner"`, `1` |
| `texts` | Tasarımcının sözleri: başlık, alt çubuk, durum, kaydetme, kapatma sorusu, silme, çizimden nokta, tel menüsü, diyagram, parçalar, ayarlar. Bir değerden yapılan söz `{ sample, text }` |
| `inputTypes` | “Girdi ekle”nin türleri sırasıyla: tür, ad, simge, ipucu |
| `canvas` | Diyagramın sayıları: kutu boyları, ızgara (10), ölçek sınırları (0,35–2), düğmelerin katı (1,25), tekerleğin katsayısı, sığdırma payı (48), boş diyagramın görünümü, sürüklemenin başlama uzaklıkları (diyagramda 3, parçalarda 5), yeni kutunun sağa ve aşağı uzaklığı (290, 100), eğrinin en kısa kolu (40), kenar yazılarının yeri (girişin 8 px solunda biter, ilk satır 6 px üstte, her satır 13 px) |
| `history` | Geri alma: en çok 100 adım; aynı alana 1200 ms içinde yazmak aynı adım |
| `tools`, `defaults` | Dosyanın araçları ve varsayılanları hesaplamanın bağlamı (`DefaultsContext`) |
| `canFeed` | Her türün besleyebildiği parametre türleri |
| `slugs` | Bir addan türeyen kimlik: Türkçe büyük harf, Ç Ğ İ Ö Ş Ü düz harfe, küçük harf, harf ve rakam dışı ayırıcı, sonraki sözcüklerin baş harfi büyük; harfle başlamayana `g` |
| `newModel`, `copyLabel` | Yeni modelin alanları (kimlik dışında); kopyanın adı |
| `sequences` | Boş bir modelden (kimliği `m-fixture`) başlayan düzenleme dizileri. Her adım: işlem (`op`: `addInput`, `addStep`, `setSource`, `removeInput`, `removeStep`, `inputFromParam`, `addOutput`, `caption`, `autoLayout`), sonucu (verilen ad ya da kimlik; yoksa `null`), sonraki model (JSON'u) ve modelin sorunları (`step`, `message`) |
| `models` | Her dizinin son modeli ve tasarımcının ondan okudukları: durum satırı, adımların sırası (ya da döngünün iletisi), kenarlar ve yazıları (sözü, ipucu ve hedef adımın yanındaki yeri), kutuların ikinci satırı, her adımın her parametresinin uygun kaynakları ve kaynak listesinin sözü, her kaynaktan her adıma çekilen telin menüsü, kutuların kapladığı alan |
| `spots` | Yeni kutunun yeri: seçili kutunun sağı, yoksa en alttaki kutunun altı; boş modelde (40, 40) |
| `titles`, `savedLabels` | Pencere başlığı; kaydedilen ad (boşken “Adsız model”) |
| `joins` | Bir değişikliğin önceki geri alma adımına katılıp katılmadığı, sırayla: alanın anahtarı (yoksa `null`), zaman (ms) |
| `geometry` | Kenarların denetim noktaları, kutuların çıkış ve giriş noktaları, sığdırma, yakınlaştırma (`floor`: en az ölçek; yoksa 0,35), sığdırmanın bıraktığı en az ölçek (`floors`: 0,35 ya da sığdırmanın ölçeği, hangisi küçükse), ızgaraya oturtma (JavaScript'in yuvarlamasıyla, yarım yukarı) |

