# ADR 9356: Formun Uygula'sı, varsayılan formla düzenleme, veri katmanına yakınlaştırma, tablo süzgeci ve Katman özelliklerinin eylemleri

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turları 4 ve 5
  (`gis-eksik-27-11-2026.md`: T7 takip maddesi, T16 P2, T17 P2, T18 P2, T19 P3, T20 P3, T21 P3). Sahibin sözü: “Gis eksik
  27 11 dosyasındaki yeni maddeleri tamamla”.
- **İlgili:**
  - [ADR 9346](9346-record-flow-inspector-default-form-and-write-names.md) (veri katmanının formu, adımların adları, varsayılan form);
  - [ADR 9327](9327-one-scale-and-data-extents.md) (veri katmanlarının kapsamı);
  - [ADR 9348](9348-table-fields-label-search-builder-rows-scope-and-parcel-check.md) (katman süzgeci ve tablonun kapsamı);
  - [ADR 9350](9350-layer-properties-summary.md) (Katman özellikleri).

## Bağlam

- **T7 takip:** veri katmanının formundaki “Değişikliği hazırla” formu kapatıyor, değişikliği katmanın bekleyen
  kayıtlarına geçiriyordu; “hazırla” iki sonucu da söylemiyordu.
- **T16:** Katmana yakınlaştır, kapsamı henüz okunmamış veri katmanında pasifti. Nesne sayısı gösterilince (kapsam
  okununca) açılıyordu; nedeni söylenmiyordu.
- **T17:** formu olmayan katmanda bir satırı Düzenle ve hiçbir şeyi değiştirmeden Vazgeç, projeye “… formu · Taslak”
  bırakıyordu. Bir satırı düzenlemek proje form kitaplığını değiştiriyordu.
- **T18:** katman süzgeci açıkken tablo çubuğuna Kapsam seçicisi ekleniyor, İfade süzgeci alanı sağda kırpılıyordu.
- **T19:** Katman özellikleri doğru bilgiyi veriyor, ama ilgili ayara geçmek için pencereyi kapatıp menüyü yeniden açmak
  gerekiyordu; kaynak yolunun Kopyala'sı yoktu.
- **T20:** Stil'in Renk satırı “fg” diyordu (tema anahtarı).
- **T21:** bölümler klavyeyle değişmiyor, odak görünmüyordu.

## Araştırma

- **QGIS, Öznitelik formu:** katmanın özel formu yoksa öznitelik formu alanlardan kendiliğinden kurulur
  (“Autogenerate”); bir kaydı düzenlemek katmanın form ayarını değiştirmez. Özel form Katman özellikleri › Öznitelik
  formu'nda açıkça seçilir
  ([Customize a form for your data](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#customize-a-form-for-your-data)).
  Düzenleme modunda değişiklik katman belleğinde bekler, Save Layer Edits kaynağa yazar
  ([Editing](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html#saving-edited-layers)).
- **QGIS, Zoom to Layer:** katmanın kapsamı sağlayıcıdan istenir; kapsam okunmadıysa okunur
  ([Layers panel](https://docs.qgis.org/latest/en/docs/user_manual/introduction/general_tools.html#layers-panel)).
- **QGIS, Katman özellikleri:** Bilgi sekmesi salt okunurdur; ayarlar yandaki sekmelerdedir, sekmeler klavyeyle
  dolaşılır ([Information Properties](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#information-properties)).
- **WCAG 2.4.7 (Focus Visible):** klavyeyle kullanılan her arayüzde odak görünür olmalıdır
  ([Understanding 2.4.7](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)).
- **KentOS kararı:**
  - Formun eylemi “Uygula”dır; nereye gittiği yanında yazar.
  - Varsayılan form oturumluktur; kaydı düzenlemek de onunla olur, proje formu değişmez.
  - Yakınlaştırma kapsamı okunmamış katmanda da sunulur, okunmasını bekler.
  - Katman özellikleri her bölümden tek bir eylemle ilgili pencereye geçer ve klavyeyle dolaşılır.

## Karar

### 1. Formun Uygula'sı (T7 takip)

- **Ad:** veri katmanının kaydında formun ana düğmesi “Uygula”dır (ADR 9346'nın “Değişikliği hazırla”sı yerine).
- **Not:** düğmelerin solunda “Değişiklik katmanda bekler. Veri kaynağına yazmak için Kaynağa yaz'ı kullanın.” yazar.
- **Sonrası:**
  - Bildirim “Değişiklik uygulandı: kayıt katmanda bekler. …” der; kalıcı kayıt onayı vermez.
  - Tablonun altında “1 kayıt kaynağa yazılmadı (1 değişen satır)” ve Kaynağa yaz kalır.
  - Kalıcı onay yalnız kaynağa yazım başarılı olunca gelir (değişmedi).
- **Vazgeç** kaydın değerini olduğu gibi bırakır.

### 2. Varsayılan formla düzenleme (T17)

- **Düzenleme:** formu olmayan katmanın kaydı, okurken olduğu gibi düzenlerken de katmanın alanlarından kurulan
  varsayılan formla açılır.
  - Form çizimin kopyasında kurulur, projeye eklenmez (`open_entry_default`).
  - Düzenle satırı katmanın düzenleme oturumuna alır.
- **Uygula:** değişen değerler öznitelik tablosunun hücre yazımıyla aynı komutla yazılır (`cad.entities.set`,
  Öznitelikler).
  - Katmanın alan kuralları denetlenir.
  - Veri katmanında satır oturumda bekler.
  - Form listesi, yayın sürümleri ve projenin formları değişmez.
- **Aynı yol:** yeni çizilen satırın kendiliğinden açılan formu (T8) ve tek nesnenin Formla düzenle'si de formu olmayan
  katmanda varsayılan formu açar; katmana form eklenmez.
- **Özel form:** yalnız Veri › Formlar'ın açık tasarım eylemleriyle (Yeni form) projeye girer. Varsayılan formun sözü
  bunu söyler: “Düzenle aynı formla düzenlemeye açar, katmana form eklenmez; katmanın kendi formu Veri › Formlar'da
  oluşturulur.”

### 3. Veri katmanına yakınlaştır (T16)

- **Sunulur:** Katmana yakınlaştır (Gruba yakınlaştır) altında veri katmanı olan her düğümde açıktır; kapsamın okunmuş
  olması ve Nesne sayısını göster ayarı onu etkilemez.
- **Basınca:**
  - Kapsam elde ise yakınlaştırır.
  - Kapsam okunuyorsa söyler (““yeni” kapsamı kaynaktan okunuyor; gelince yakınlaştırılır.”). Kapsam gelince
    görünüm o arada değişmediyse yakınlaştırır.
  - Geometrili satır yoksa: “… tablosunda geometrili satır yok; yakınlaştırılacak kapsam yok.”
  - Geometri sütunu yoksa: “… tablosunun geometri sütunu yok; haritada yeri olmadığı için yakınlaştırılmaz.”
  - Okunamazsa: “… katmanının kapsamı okunamadı: …”.

### 4. Tablo çubuğu ve süzgeç işareti (T18)

- **Sarılan çubuk:** veri tablosunun çubuğu tek akan satırdır: Tablo | Form, Katman, Sabit, düzenleme durumu, Kapsam,
  Ara, Göster, İfade süzgeci, sonra eylemler. Dar panelde alt satıra sarılır; hiçbir alan kırpılmaz.
- **Süzgeç işareti:** sorulan İfade süzgeci tablonun altında satır sayısının yanında durur (“İfade süzgeci:
  mahalle = 'M2' ×”). × süzgeci kaldırır ve tabloyu yeniden sorar.

### 5. Katman özellikleri (T19–T21)

- **Bölümün eylemi (T19):** her bölüm başlığının yanında en çok bir ikincil eylem durur. Eylem kendi penceresini bu
  katmanın kimliğiyle açar, özet kapanır:

  | Bölüm | Eylem |
  |---|---|
  | Kaynak | Yolu kopyala (dosya ya da adres, parolasız) |
  | Stil | Stil ayarlarını aç (Katman stili) |
  | Alanlar | Alanları aç |
  | Formlar | Formları aç |
  | Genel, Koordinat sistemi | yok |

- **Tema rengi (T20):** stilin rengi tema anahtarıysa adıyla ve şimdiki rengiyle yazılır.
  - Örnek: “Tema ön planı (#E4EAF0, temaya göre değişir)”.
  - Örnek renk tuvalin paletinden çözülür; haritadaki sembolle aynıdır.
  - Adlar: fg “Tema ön planı”, fg-dim “Tema ön planı, soluk”, ink “Çizim mürekkebi”, paper “Kâğıt (arka plan)”.
  - Özel renk onaltılık değeriyle yazılır.
- **Klavye (T21):**
  - Pencere açılınca bölüm listesi klavyeyi alır; seçili bölüm vurgu renginde halkayla çizilir.
  - ↑ ↓ Home End bölüm seçer.
  - Tab (Shift+Tab) halkayı liste, bölümün eylemi (varsa) ve Kapat arasında dolaştırır; Enter halkalı olana basar.
  - Esc kapatır; katman ağacı klavyeyi o katmanda alır.
  - Listenin altında “↑ ↓ bölüm · Tab eylem ve Kapat · Enter basar · Esc kapatır” yazar.

### 6. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Formun Uygula'sı ve notu | yok: Mimari'de veri katmanı ve formlar yoktur | var: CBS tabloları altlık ve veri katmanı olabilir | var: aynı | var |
| Varsayılan formla düzenleme | yok: aynı gerekçe | var: formsuz katmanda Formla düzenle | var: aynı | var |
| Veri katmanına yakınlaştırma | yok: Mimari'de veri katmanı yoktur | var | var | var |
| Tablo çubuğunun sarılması, süzgeç işareti | yok: Tablo sekmesi Mimari'de yoktur | var | var | var |
| Katman özelliklerinin eylemleri, renk adı, klavye | yok: Katman özellikleri yalnız CBS'dedir (ADR 9350) | yok: aynı | yok: aynı | var |

### 7. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Varsayılan formla düzenlemenin yazımı `cad.entities.set`'tir; betik aynı komutla yazar, form gerekmez. Form, yakınlaştırma, tablo çubuğu ve Katman özellikleri arayüzün işidir. |
| MCP | var | Aynı gerekçe. |
| Yapay zekâ ajanı | var | Aynı gerekçe. |
| Eklenti | var | Eklenti öznitelik yazımını aynı komutla yapar (`cad.entities.set` eklentiye açıktır). |

## Açık

- **Ekran okuyucu:** Katman özelliklerinin bölüm listesi erişilebilirlik ağacına adlandırılmış liste olarak verilmiyor.
- **Varsayılan formun çocukları:** varsayılan form ilişkili kayıt öğesi taşımaz; alt kayıtlar özel formla girilir.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `forms::row_target::row_target_tests::a_layer_without_a_form_shows_its_row_through_the_default_form_and_duzenle_edits_it_without_a_form`:
    - satır okumak için varsayılan formla açılıyor;
    - Düzenle varsayılan formu düzenlemeye açıyor, Vazgeç'ten sonra projenin form sayısı aynı, bekleyen kayıt yok;
    - yeniden açılıp `kullanim` “Okul” ve Uygula: form kapanıyor, proje formu yok, söz “Değişiklik uygulandı: kayıt
      katmanda bekler.”, tablonun altı “1 kayıt kaynağa yazılmadı (1 değişen satır)”.
  - `…::a_new_row_drawn_opens_its_form_and_ozniteliklari_gir_opens_it_again`: yeni satır varsayılan formla açılıyor,
    katmana form eklenmiyor.
  - `data_extents_tests::katmana_yakinlastir_waits_for_the_box_and_says_why_there_is_none`:
    - kapsamı okunmamış katmanda menü sunuluyor;
    - tık “kaynaktan okunuyor” diyor, kapsam gelince iki nokta görünümde;
    - satırsız tablo söyleniyor, görünüm kalıyor.
  - `features::follow_tests::the_asked_filter_is_marked_under_the_table_and_taken_off_there`: sorulan süzgeç
    işaretleniyor, × kaldırıyor.
  - `layer_properties_tests::the_sections_lead_on_and_the_keys_go_round_them`:
    - ↓ Kaynak, Tab Yolu kopyala, Enter yolu kopyalıyor;
    - Koordinat sisteminde Tab listeden Kapat'a geçiyor; End ve Home çalışıyor;
    - Alanları aç Alanlar'ı açıyor, özet kapanıyor;
    - Esc katman ağacına klavyeyi `bakim`'de veriyor;
    - “fg” “Tema ön planı” okunuyor.
  - `-- layer_properties forms:: data_extents features:: data_edits data_pending layering processing:: module_tests
    ribbon_tests history::text catalog settings input`: 446 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır
    (render birleşmesinin bağları).
- **CBS paketi:** `cargo test -p kentos-desktop-cbs` 66 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9356/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-form-uygula`: varsayılan form, “Uygula” ve solunda not;
  - `02-tablo-suzgec`: sarılan çubuk, altta “İfade süzgeci: mahalle = 'M2' ×”;
  - `03-ozellikler-kaynak`: Yolu kopyala halkada;
  - `04-ozellikler-stil`: “Tema ön planı (#E4EAF0, temaya göre değişir)”, Stil ayarlarını aç, Stil halkada.
