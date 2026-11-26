# ADR 9319: Kaydın formu okumak için açılır; Düzenle aynı formu düzenlemeye açar

- **Durum:** kabul edildi (2026-11-26). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  2, P1; ikon tablosunun “Öznitelik formunu aç” satırı); sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 9149](9149-form-edit-from-a-table-row.md) (tablo satırından Formla düzenle),
  [ADR 9159](9159-read-only-row-in-its-form.md) (salt okunur bağlantının satırı formunda),
  [ADR 9166](9166-data-table-bar-like-the-drawings.md) (veri tablosunun çubuğu),
  [ADR 9176](9176-open-a-source-row-from-the-related-list.md) (ilişkili listeden satır).

## Bağlam

- **Kusur:** yazılabilir bir GeoPackage katmanında düzenleme oturumu kapalıyken tablodan seçilen kaydın formu
  açılmıyordu.
  - Formla düzenle “Katman düzenlemeye kapalı” deyip Düzenlemeyi başlat bağlantısını gösteriyordu.
  - Tablonun Form düğmesi formu açmıyor, kullanıcıyı şeride yönlendiriyordu.
  - Aynı kayıt düzenleme açıkken açılıyordu. Salt okunur bağlantının satırı ADR 9159'dan beri okunarak açılıyordu;
    yazılabilir katmanın kapalı oturumu bu yoldan yararlanmıyordu.
- **İkon tablosu:** kaydı yalnız görmek için ayrı bir “Öznitelik formunu aç” eylemi yoktu. Kalem simgesi
  düzenlemeye, başka bir form simgesi tasarıma ayrılmalıydı.

## Araştırma

- **QGIS:** öznitelik formunun tanımlama (yalnız gösterme), düzenleme, ekleme ve arama kipleri var
  ([QgsAttributeForm](https://api.qgis.org/api/4.2/classQgsAttributeForm.html)). Tanımlama aracı katman düzenlenmezken
  kaydı formunda gösterir.
- **KentOS kararı:** kayıt düzenleme oturumundan bağımsız olarak okunur. Düzenleme, aynı formdan tek basışla açılır.

## Karar

### 1. Düzenleme kapalıyken Formla düzenle kaydı okur

- Yazılabilir katman düzenlenmiyorsa tablodan seçilen satır kaynağından okunur ve formunda okumak için açılır.
  - Satır alınmaz, düzenleme tamponuna girmez, varsayılan konmaz (salt okunur bağlantının yolu, ADR 9159).
  - Formun başı katmanı ve anahtarı söyler, örneğin: “parseller” düzenlemeye açık değil: 22 anahtarlı kayıt yalnız
    okunuyor. Düzenle, katmanın düzenlemesini başlatır ve kaydı bu formda düzenlemeye açar.
  - Başlık satırı kaydı katmanı ve anahtarıyla adlandırır ve okunduğunu söyler (“Kapalı alan · parseller · fid 22 · salt
    okunur”); okunan her kayıtta böyledir (salt okunur bağlantı, okumak için açılan, okumaya sunulan ilişkili öğe).
    Düzenle'den sonra “salt okunur” kalkar.
- **Düzenle:** katmanın oturumu başlatılır (açıksa olduğu gibi kalır) ve aynı satır alınıp düzenlemeye açılır. Oturum
  reddedilirse nedeni söylenir, form olduğu gibi kalır.
- Salt okunur bağlantıda Düzenle sunulmaz (ADR 9159).

### 2. Öznitelik formunu aç (`data.formOpen`)

- Seçili kaydı oturumdan bağımsız olarak okumak için açar.
  - Tabloda ya da haritada seçilen veri satırı kaynağından okunur, katman düzenleniyor olsa da alınmaz.
  - Çizimdeki nesnenin kaydı formunda salt okunur gösterilir.
- **Düzenle:** veri satırında oturumu gerekirse başlatır ve satırı alıp düzenlemeye açar. Nesnede aynı formu
  düzenlemeye açar.
- **Yerleri:**
  - CBS'de Veri › Form'da Formlar'ın yanında, kendi simgesiyle (`formOpen`: göz işaretli form; Formla düzenle kalemli
    formdur, Formlar tasarımdır);
  - Harita'da Veri panelinin taşmasında Formla düzenle'nin yanında;
  - Öznitelikler panelinin Form satırının menüsünde en üstte.
- **Düzeyi:** salt okuma (`catalog/v1/operations.json`, `query`): veri satırı `cad.layers.rows` ile okunur, formun
  denetimi `cad.forms.validate`'in çekirdeğidir. Düzenle, Formla düzenle'nin yoluna geçer.
- Seçim yoksa ya da katmanın formu yoksa ne yapılacağı söylenir.

### 3. Tablonun Form düğmesi

- Veri katmanının tablosunda tek satır seçiliyse Form düğmesi o satırın formunu açar: katman düzenleniyorsa
  düzenlemek, düzenlenmiyorsa okumak için (Düzenle ile).
- Satır seçili değilse ya da katmanın formu yoksa yol söylenir: “Formu açmak için tablodan tek bir satır seçin; katmanın
  formu yoksa Veri › Formlar'da bir form yapın.”

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Kapalı oturumda kaydı okuma, Düzenle | yok: veri katmanı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |
| `data.formOpen` | yok: Mimari formsuzdur (şeridin form komutları Harita ve CBS'nin) | var: çizim nesnesinin formu | yok: aynı gerekçe | var |
| Tablonun Form düğmesi | yok: veri tablosu yok | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Pencereyi açmak masaüstünün görünüş işidir, çizim değişmez. Kayıt `cad.layers.rows` ile okunur, form `cad.forms.*` komutlarıyla doldurulur. |
| MCP | yok | Aynı gerekçe; okuma aynı komutlarla. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe; eklenti kaydı okuyan komutları `document.read` ile çağırır. |

## Açık

- Özel formu olmayan katmanda alanlardan otomatik form üretilmesi. Kaydın ekranı formu belgenin ayarlarında kimliğiyle
  arar (`row_screen`); geçici form ayrı bir çekirdek iştir.
- Haritanın sağ tık menüsünde ve Bilgi sonucunda aynı eylem.

## Doğrulama

26 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- form related chain_tests data_bar ribbon_tests module_tests catalog
  history::text properties`: 371 geçti; tek kırmızı aşağıdaki bağımlılık tabanı.
  - `a_row_chosen_in_the_table_opens_its_record_through_the_form` güncellendi: düzenleme kapalıyken form okumak için
    açılıyor, başlık “fid 22 · salt okunur” diyor, hiçbir şey alınmıyor; Düzenle satırı alıp düzenlemeye açıyor.
  - Yeni sınama `the_tables_form_and_oznitelik_formunu_ac_open_the_row_for_reading`: tablonun Form'u satırı açıyor;
    düzenleme açıkken Öznitelik formunu aç yine yalnız okuyor; Düzenle düzenlemeye açıyor.
- **Programlar:** `cargo test -p kentos-cad --test programs -- module_surface layer_menu`: 31 sınama geçti (yüzey matrisi,
  katman menüsü).
- **Üreticiler:** `python3 scripts/ui/ribbon.py --check` (CBS'nin Form paneli, Harita'nın Veri taşması),
  `KENTOS_WRITE_MODULES=1 cargo test -p kentos-desktop --lib module_catalog` (`catalog/v1/modules.json`),
  `python3 scripts/cad/manifest.py --check` (`data.formOpen` salt okuma düzeyinde; 1095 satır).
- **Resim** (`.run/gui-evidence/kullanim-9319/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-okumak-icin-acilan-kayit`: düzenlemeye kapalı katmanın 22 anahtarlı kaydı formunda; alanlar soluk, Kaydet kapalı,
    başta neden ve Düzenle;
  - `02-veri-form-paneli`: Veri › Form'da Formlar, Formu aç (göz işaretli form), Formla düzenle (kalemli form);
  - `simge/secenekler`: göz, büyüteç, açık form; göz seçildi (büyüteç arama ve süzgeçle karışır).
- **Koşulmayan:** gerçek pencerede fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.
- **Önceden kırmızı, bu işin değil:** `scripts/dev/module_deps.py --check`'in kabuk → CAD ve CBS → CAD tabanı
  (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`; çizim motoru birleşmesinden).
