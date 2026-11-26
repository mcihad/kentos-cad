# ADR 9316: Etiket CBS'nindir; CAD'de sonradan verilen ad da yazı olur

- **Durum:** kabul edildi (2026-11-26). Sahibin sözleri (26 Kasım 2026):
  - “hiç nokta ismi yok noktaları ekledim, daha sonra isimlendirmek istesem bile artık isimleri ekrana yazmıyor”;
  - “ribbon üzerindeki nokta adları ikonu var onu nokta etiketleri olarak değiştir, o etiketi açıp kapatsın”;
    “sadece etiketi açıp kapatacak, nokta yazısını değiştirmeyecek”;
  - “cad projelerinde etiket menüsü de gözükmesin katman sağ tuşta, desteği olsun sistemde … bunu sadece gis içinde
    kullanalım, ayrıca stil import işleri de cad projelerinde olmasın, stil seçme olsun ama stil import falan olmasın”;
  - “etiket kavramı cbs içindir cad içinde aksi belirtilmedikçe olmaz”, “claude.md içerisine de yaz bunu”,
    “cad processing tool incele ve oraya da bak”;
  - “nokta etiketleri düğmesini de kaldır cad içerisinden, o gis içerisinde olacak”.
- **İlgili:**
  - [ADR 1540](1540-point-display-and-editing.md) (Nokta adları ve kodları);
  - [ADR 9195](9195-module-separation.md) §5 (katman menüsü modül ve tür tablosu);
  - [ADR 9310](9310-processing-tools-by-module.md) (işlem araçları modüllere);
  - [ADR 9311](9311-arial-times-point-names-labels-off.md) (etiketler kapalı başlar);
  - [ADR 9314](9314-point-names-follow-their-points.md) (nokta adı noktaya bağlı yazıdır);
  - CLAUDE.md, “Etiket CBS'nindir” kesin kuralı.

## Bağlam

- **Sonradan verilen ad:** ADR 9314'ten beri CAD'de noktanın adı noktaya bağlı bir yazıdır. Yazıyı yalnız noktayı
  oluşturan komut yazıyordu (`cad.point.create`'in `nameText`'i). Adsız eklenen noktaya sonradan Noktalar'dan ya da
  Öznitelikler'den ad verilince ad ekranda görünmüyordu: yazısı yoktu, etiketler de kapalıydı (ADR 9311).
- **Nokta adları düğmesi:** CAD şeridindeki Nokta adları (`view.pointNames`) etiketin içindeki adı açıp kapatıyordu.
  Etiketler kapalıyken hiçbir şey yapmıyordu.
- **CAD'deki etiket yüzeyleri:** Harita'nın ve Yol'un katman menüsünde Etiketler… vardı; Harita'nınkinde Stil yükle ve
  kaydet, SLD, QML ve MapLibre de vardı. Şerit dosyası Harita'ya ve Yol'a etiket komutlarını ve MapLibre stil
  alışverişini de veriyordu. Köşe noktalarını numarala yeni katmanını bir etiket stiliyle kuruyordu.

## Karar

### 1. Kural

- Etiket kavramı yalnız CBS'dedir. CAD modüllerinde (Mimari, Harita, Yol) sahip açıkça istemedikçe etiket sunulmaz:
  - katman menüsünde Etiketler yoktur;
  - şeritte ve Komut ara'da etiket komutu yoktur;
  - Uygulama ayarları'nda etiket satırı yoktur;
  - işlem aracının yeni katmanında etiket stili yoktur.
- CAD'de nokta adı noktaya bağlı yazıdır (ADR 9314).
- Etiket desteği ortak çekirdekte kalır: etiket motoru, katmanın etiket ayarı ve `.kcad` alanları. Etiketli bir CAD
  projesi açılırsa etiketleri yine çizilir; CAD'in arayüzü onları değiştirmeyi sunmaz.
- CAD'de stil seçilir (Katman stili). Stil içe alma ve başka programların stil dosyaları yalnız CBS'dedir: Stil yükle
  ve kaydet, SLD, QML, MapLibre.
- Kural CLAUDE.md'nin “Çalışırken” bölümüne kesin kural olarak yazıldı.

### 2. Sonradan verilen ad yazı olur

- `cad.entities.set`'e `nameText` eklendi. Bu adımda yeni ad alan bir noktanın adını yazan yazısı yoksa yazı aynı
  adımda yazılır.
  - Yazı `cad.point.create`'in kuralıyla yazılır (`point::name_text_at`): noktanın sağ üstünde, projenin ölçü yazısı
    boyunda, yazı tipinde, dik, noktanın katmanında. Noktaya ölçeksiz bağlıdır.
  - Ad `label` ile ya da bir haritanın noktasında adı yansıtan `Ad` özniteliğiyle verilebilir.
- **Yazı yazılmayan durumlar:**
  - adını yazan yazısı olan nokta ikinci yazı almaz; yazısı yeni adı alır (ADR 9314);
  - nokta olmayan nesne yazı almaz;
  - adı kaldırılan ya da adı değişmeyen nokta yazı almaz.
- **Çıktı ve plan:** çıktıda yazıların kimlikleri `texts` ve `textIds`; planda yazılacak yazılar `texts`.
- **Adım:** `nameText` ile etiket işleminin adımı “Ad değiştir”dir (CAD'de etiket yoktur); onsuz “Etiket değiştir”
  kalır.
- **Masaüstü:** Harita'da ve Yol'da üç yer `nameText` ile yazar (`points::edit::names_as_texts`):
  - Noktalar'ın Ad hücresi;
  - Öznitelikler'in Nokta adı satırı (aynı yazma yolu);
  - Noktalar'ın toplu adlandırması.
- **Ortak durumlar:** `fixtures/commands/v1/cad.entities.set.json`'a üç durum eklendi (üretici
  `scripts/fixtures/set_command_cases.py`):
  - adsız noktaya ad: yazı, plan ve geri alma;
  - yazısı olan nokta: ikinci yazı yok, yazı yeni adı alır;
  - nokta olmayan, adı kaldırılan ve adı değişmeyen nesne: yazı yok.

### 3. Nokta etiketleri, CBS'de

- **Komut ve ayar:** `view.pointNames` (Nokta adları) yerine `view.pointLabels` (Nokta etiketleri) geldi.
  `graphics.pointNames` ayarının (öntanımlı açık) yerine `graphics.pointLabels` geldi (öntanımlı kapalı).
- **Açıkken** her nokta adıyla etiketlenir:
  - etiketleri açık katmanda katmanın etiketiyle;
  - etiketleri kapalı katmanda katmanın etiket stiliyle ya da noktanın öntanımlı etiketiyle.
  - Bunun için etiket motoru kapalı katmanı tek etiketli okur (`label_layers_json`'un `point_labels`'ı). Metni
    yalnız noktalara verilir; katmanın öbür nesneleri etiketsiz kalır.
- **Kapalıyken** etiketleri katmanların kendi ayarı belirler.
- **Nokta kodları** etiketi olan noktada kodu adın ardından yazar; etiketi olmayan noktaya yazmaz.
- **Yazıya dokunmaz:** nokta adının yazısı çizimin nesnesidir. Düğme yalnız etiketi açıp kapatır.
- **Yeri:** CBS şeridinin Görünüm › Görünüş panelinde, öbür görünüm tercihlerinin yanında, Nokta kodları'yla. Harita
  › Etiket paneli 1440 pikselde sığmadığı için oraya konmadı. Uygulama ayarları › Görünüm'de yalnız CBS'de görünür
  (`AppSetting::PointLabels`). CAD şeridinden ikisi de kalktı.
- **Simge:** `viewPointLabels`, noktadan kısa bir çizgiyle etiketine. Üç seçenekten biri seçildi; seçenek sayfası
  `.run/gui-evidence/kullanim-9316/simge/`'dedir.

### 4. CAD'den kalkanlar

| Yüzey | Önce | Şimdi |
|---|---|---|
| Katman menüsü › Görünüş › Etiketler… | Harita, Yol, CBS | CBS |
| Katman menüsü › Stil yükle…, Stili kaydet…, SLD, QML, MapLibre | Harita, CBS | CBS |
| Etiket komutları (`layer.labels`, `tool.labelMove`, `tool.labelPin`, `tool.labelHide`, `tool.labelRotate`, `view.pinnedLabels`, `view.unplacedLabels`) | Harita, Yol, CBS | CBS |
| Etiketleri yazıya çevir (`tool.labelsToText`) | Harita, CBS | CBS |
| MapLibre stilinden görünüş al, MapLibre stili dışa aktar (`file.import.maplibre`, `file.export.maplibre`) | Harita, Yol, CBS | CBS |
| Nokta adları, sonra Nokta etiketleri; Nokta kodları | Harita, Yol (şerit) | CBS (şerit, ayar) |
| Katman stili › Katman çizimi › Maske › Etiketler | hepsi | CBS; İşaretler hepsinde |
| Katman stili… (stil seçme) | hepsi | hepsi |

- Şerit dosyasında `scripts/ui/ribbon.py`'nin `map` listesi Harita'ya ve Yol'a verilir. Etiket ve MapLibre stil
  komutları bu listeden çıktı.
- Profillerde `LayerEntry::Labels`, `StyleFiles`, `Sld`, `Qml` ve `MapLibre` Harita'da ve Yol'da yok.
- ADR 9195 §5'in tablosu buna göre güncellendi.

### 5. İşlem araçları

- **CAD'in araçları** (`CAD_DRAWING`, Harita ve Yol) etiket kurmaz.
  - Köşe noktalarını numarala'nın yeni katmanı etiket stilsiz kurulur.
  - “Nokta” seçeneğinin ipucu artık “Adlı nokta nesnesi; adı yazılmaz” der.
  - Kenar uzunluklarını yaz zaten yazı yazar.
- **Ortak araçlar** (`SHARED`) bir nesnenin adını “boşsa etiket” diye okur. CAD'de bu, noktanın adıdır.
  - Örnekler: Delaunay üçgenlemesinin köşe adları, Öznitelik hesapla'nın “Etiketi de güncelle” seçeneği.
  - Metinleri CBS ile ortak olduğu için değişmedi; sahip şimdilik kalmasını istedi (aşağıda).
- Uzaklık araçları (matris, merkez, en kısa çizgi, komşular) yalnız CBS'dedir (ADR 9310).

### 6. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Etiket yüzeyleri (menü, şerit, ayar, stil dosyaları) | yok: kural | yok: kural | yok: kural | var |
| Nokta etiketleri, Nokta kodları | yok: kural | yok: kural | yok: kural | var |
| Sonradan verilen adın yazısı (`nameText`) | yok: nokta listesi yok | var | var | yok: CBS adı etiketle gösterir |

### 7. Kanallar

| Kanal | `nameText` | Nokta etiketleri |
|---|---|---|
| Python | var: `cad.entities.set(..., name_text=True)` | yok: masaüstünün görünüm tercihi, çizim değişmez |
| MCP | var: aynı komut | yok: aynı gerekçe |
| Yapay zekâ ajanı | var: MCP ile | yok: aynı gerekçe |
| Eklenti | var: SDK'nın aynı komutu | yok: aynı gerekçe |

- Her kanal aynı komutu çağırır. Doğrulama, plan ve adım aynıdır.

## Şimdilik kalanlar

Sahip 26 Kasım 2026'da ikisi için “Şimdilik kalsın” dedi. Sahip yeniden açmadıkça önerilmez.

- Ortak işlem araçlarının metinlerinde “etiket”, nesnenin adı anlamında geçiyor (“boşsa etiket”, “Etiketi de
  güncelle”). CAD'de bunlar noktanın adıdır. CBS ile ortak metinler modüle göre ayrılırsa CAD'de “ad” denebilir.
- Stil yöneticisinin İçe aktar'ı (sembol kitaplığına `.kstil`, resim, pano) her modülde duruyor. Katman stili
  dosyası değildir, sembol kitaplığıdır.

## Doğrulama

26 Kasım 2026'da koşuldu:

- **Ortak durumlar:** `KENTOS_CASES_ONLY=cad.entities.set,cad.point.create cargo test -p kentos-native-application
  --test all every_command_case` geçti (`cad.entities.set` 65 durum, üçü yeni); Python'da aynı süzgeçle
  `test_command_cases` geçti.
- **Çekirdek:** `cargo test -p kentos-scene-query -p kentos-project -p kentos-native-style` ve
  `cargo test -p kentos-processing` geçti. Köşe numaralamanın üç durumu yeni katmanı etiket stilsiz bekler
  (`fixtures/processing/v1/cases.json`).
- **Nokta aracı:** `cargo test -p kentos-interaction --test all survey_points wipeouts` geçti. Yeni sınama
  `in_a_map_a_named_point_writes_its_name_as_a_text`: Harita'da ad yazısı aynı adımda yazılır, modülü belirsiz
  projede yazılmaz.
- **Masaüstü:** `cargo test -p kentos-desktop --lib -- module_catalog points:: module_tests ribbon_tests history::text
  catalog settings layer_style properties`: 298 sınama geçti.
  - Yeni sınamalar: `labels_and_other_programs_style_files_are_a_cbs_s_alone`,
    `nokta_etiketleri_and_kodlari_are_settings_and_change_only_the_labels`,
    `a_point_named_later_in_a_map_gets_its_name_text`.
  - Kırmızı tek sınama `the_modules_keep_to_their_dependency_baseline`'dır. Render motoru birleşmesinin (e6c3b29b6)
    üç bağı (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`) HEAD'in kendi dosyalarında da
    aynı sonucu verir; bu dilimden değildir.
- **Programlar:** `cargo test -p kentos-cad --test programs -- layer_menu module_surface wizard program_tests`: 36
  sınama geçti. Katman menüsü tablosu ve iki kullanım izi (survey-points, template-tools) bunlara dahildir.
- **Üreticiler (`--check` ile):** `scripts/ui/ribbon.py`, `scripts/cad/manifest.py`, `scripts/python/sdk.py`,
  `scripts/fixtures/set_command_cases.py`, `processing_command_cases.py`, `processing_batch_cases.py`. Ayar şeması,
  komut kataloğu, TypeScript türleri ve `catalog/v1/modules.json` yeniden yazıldı.
- **Boyut:** `interaction/src/core/path.rs` 2724 satırdan 2713'e indi (`EntitiesSetProperties::new`).
  `scene-query/src/spatial.rs` HEAD'deki boyunda kaldı (2816); tabanın üstündeki fazlası önceki işlerden.
- **Resimler** (`.run/gui-evidence/kullanim-9316/`, büyütülerek bakıldı):
  - `cbs-nokta-etiketleri.png`: Görünüm › Görünüş'te Nokta etiketleri ve Nokta kodları açık, noktalar “ad kod”;
  - `harita-sonradan-verilen-ad.png`: adsız eklenip Noktalar'da N7 adı verilen nokta, adı sağ üstünde yazı;
  - `menu-harita-çizim-görünüş.png`, `menu-yol-çizim-görünüş.png`: Görünüş'te Etiketler ve stil dosyaları yok;
  - `menu-cbs-veri-tablosu-görünüş.png`: hepsi var;
  - `simge/nokta-etiketleri-simge-secenekleri.png`.
- **Python takımı:** 884 sınama koştu, 10'u kırmızı. Hepsi bu dilimden önce de kırmızıydı:
  - Harita işi örneklerinin göreli yolları (7);
  - BIM ve öbür komut durumları;
  - profil kitaplıkları;
  - panel bölmesi.
  `KENTOS_CASES_ONLY=cad.entities,cad.point,cad.processing,cad.labels,cad.text` ile komut durumları geçti.
- **Koşulmayan:** gerçek pencerede fareyle deneme (`scripts/usage/real/`) yapılmadı.
