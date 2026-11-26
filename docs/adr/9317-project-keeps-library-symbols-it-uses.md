# ADR 9317: Proje kullandığı Kitaplığım sembollerini yanına alır

- **Durum:** kabul edildi (2026-11-26). Sahibin sözleri (26 Kasım 2026):
  - “stiller veya dolgular her neyse yazdırma önizlemesinde(sheet) gözükmüyor neden, bu isteğe bağlı bir şey mi yoksa
    sıkıntı mı var”, “taramalar değil”;
  - önerilen düzeltmeye “Evet, düzelt”.
- **İlgili:**
  - [ADR 0092](0092-desktop-style-manager.md) (Stil yöneticisi; Sistem, Kitaplığım, Proje; Kopyala);
  - [ADR 0613](0613-unified-print-preview.md) (baskı önizlemesi ve ortak çıktı);
  - CLAUDE.md §25 kural 5 (CAD projesinin tamamı proje dosyasındadır; CBS proje dosyası stilleri tutar).

## Bağlam

- **Kitaplıklar:** Stil kitaplığının üç kaynağı var.
  - Sistem: KentOS ile gelir, her bilgisayarda aynıdır.
  - Kitaplığım: kullanıcının dosyasıdır (`kitaplik.kstil`), projeler arasında ortaktır, yalnız o bilgisayarda
    durur.
  - Proje: proje dosyasının içindedir.
- **Başvuru:** Katman stilinin yuvası ve nesnenin kendi sembolü kitaplık öğesini kimliğiyle gösterir
  (`{"ref": id}`, `symbol`).
  - Kitaplığım'dan seçilen sembol de, "Kitaplığıma kaydet" ile kaydedilen de böyle gösteriliyordu.
  - Sembolün kendisi projeye girmiyordu.
- **Görünen kusur:** çizim alanı ve pafta masaüstünün kitaplığını (Kitaplığım dahil) kullanır; sembolü bulur ve
  dolguyu çizer.
  - Baskı önizlemesi, PDF, Yazdır, PNG ve SVG ile pafta yayını çekirdeğin kitaplığını kullanır (`library_of`:
    sistem ve proje).
  - Bu yollar sembolü bulamayınca katmanı yalın görünüşüyle çiziyordu: dolgu yok, ince kenar çizgisi var. Bir uyarı da
    yoktu.
  - Aynı proje başka bir bilgisayarda, bulutta, Python'da ya da MCP'de de sembolsüz açılıyordu.
- **Yeniden üretim:** ekran dışı pencerede aynı proje çizim alanında, paftada ve baskı önizlemesinde resimlendi.
  - Katman dolgu rengi, sistem kitaplığının desenli Orman stili ve balastro üçünde de aynıydı.
  - Kitaplığım sembolünü kullanan alan yalnız baskı önizlemesinde boştu.
- **Kural gereği olan:** daire ve kapalı çoklu çizgi katmanın alan dolgusunu ne ekranda ne kâğıtta alır. Stil motoru
  bunları çizgi sayar (`classify::geometry_class`); dolguyu alan nesneleri ve taramalar alır. Bu kararla değişmedi.

## Araştırma

- **ArcGIS Pro:** stilden bir sembol uygulanınca stile bağ tutulmaz; sembolün kopyası uygulanır, stilin sonraki
  değişikliği katmana yansımaz ([Styles](https://pro.arcgis.com/en/pro-app/3.4/help/projects/styles.htm)).
- **QGIS:** Stil yöneticisinin öğeleri kullanıcı profilindeki `symbology-style.db`'dedir ve projeler arasında
  ortaktır ([The Style Manager](https://docs.qgis.org/3.44/en/docs/user_manual/style_library/style_manager.html)).
  Katmanın sembolojisi proje dosyasına yazılır
  ([Working with Project Files](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/project_files.html)).
- **AutoCAD:** yüklenen çizgi tipinin tanımı çizim kaydedilince çizimin içinde saklanır; `acad.lin` artık gerekmez
  ([About Linetypes](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-LT/files/GUID-20B4D4B3-1220-426A-847B-5BBE36EC6FDF.htm)).
- **KentOS kararı:** proje, kullandığı kullanıcı öğesinin kopyasını taşır. Kitaplığım seçilecek yerdir, canlı bağ
  değildir.

## Karar

### 1. Seçerken projenin kopyası

- Katman stilinin "Kitaplıktan seç…"i, "Kitaplığıma kaydet"i ve Stil yöneticisinin "Seçili nesnelere uygula"sı
  Kitaplığım öğesini doğrudan vermez: projenin kopyasını verir (`StyleLibrary::project_copy`).
  - Kopya yeni kimlik alır (`p-…`); adı ve kategorisi aynıdır.
  - Sembolün çizimleri (SVG, resim) ve başvurduğu öbür öğeler (bir dolgunun ortasındaki işaret) aynı kimlikle
    yanında gelir. Stil yöneticisinin Kopyala › Projeye'si de böyle yapar (ADR 0092).
  - Aynı sembol ikinci kez seçilince yeni kopya açılmaz: türü, adı, kategorisi ve içeriği aynı olan proje öğesi
    kullanılır.
- **ADR 0092'den değişen:** "Kitaplığıma kaydet" sembolü yine Kitaplığım'a (Sembollerim) yazar. Yuva artık
  Kitaplığım öğesini değil, projenin kopyasını gösterir.
- Proje kitaplığına yazmak çizimin bir düzenlemesidir, geri alma adımı değildir (ADR 0092). Katman stili ya da
  sembol atama kendi adımıyla geri alınırsa kopya projede kalır; Stil yöneticisinden silinebilir.

### 2. Kaydederken ve yazdırırken yanına alma

- Çizim kaydedilmeden ve yazdırılmadan önce, hâlâ Kitaplığım'a başvurduğu öğeler aynı kimlikle projeye alınır
  (`StyleLibrary::take_along`). Çizimdeki başvurular değişmez, geri alma adımı açılmaz.
- **Başvuruların toplandığı yerler** (`kentos_native_application::library_refs`):
  - nesnelerin ve blok tanımlarındaki nesnelerin sembolü;
  - katmanların ve katman durumlarının çizicisi (`{"ref": id}` her yerde);
  - proje kitaplığının öğelerinin gerektirdikleri (şablonun sembolü, sembolün çizimleri ve işaretleri).
- **Çağrıldığı yerler:**
  - her kayıt (`start_save`: dosya, kopya, veri kaynağı, bulut);
  - Baskı önizlemesi, Yazdır, Dışa aktar (PDF, PNG, SVG);
  - pafta yayınının denetimi ve yayını.
- Bir şey alınınca komut geçmişi söyler: "Kitaplığım'dan projeye alındı: “Deneme”."
- Bu yol eski projeleri ve seçim dışındaki yolları kapsar: Kitaplığım şablonuyla çizilen nesneler, katman durumları,
  başka pencerelerden gelen çiziciler.
- **Aynı kimliğin sonucu:** böyle bir proje açıkken o kimlik projenin kopyasını gösterir (kitaplık önce projeye
  bakar). Kitaplığım dosyasındaki asıl öğe olduğu gibi kalır.

### 3. Ön denetim

- Haritanın gösterdiği bir katman, ne projenin ne sistemin kitaplığında olan bir sembolle çiziyorsa Ön denetim uyarır
  (`symbol_missing`, uyarı).
  - Her harita için her sembolde bir kez söylenir.
  - İleti: "“Çizim” katmanının çizdiği “u-…” sembolü ne projenin ne sistemin kitaplığında var; baskıda katmanın yalın
    görünüşüyle çizilir."
  - Çözüm: "Projeyi sembolün Kitaplığım'da bulunduğu bilgisayarda kaydedin ya da Stil yöneticisinden başka bir sembol
    seçin."
- Bulgu ortak ön denetimdedir: masaüstünün PDF'i, `file.export.pdf`, Python'un `export_pdf`'i ve MCP aynı bulguyu
  döndürür.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Seçerken projenin kopyası | var | var | var | var |
| Kaydederken ve yazdırırken yanına alma | var | var | var | var |
| Ön denetimin `symbol_missing`'i | var | var | var | var |

Katman stili, Stil yöneticisi ve Kitaplığım her modüldedir. CAD projesinin tamamı, CBS projesinin stilleri proje
dosyasındadır (CLAUDE.md §25 kural 5); kural dördüne de uyar.

### 5. Kanallar

| Kanal | Kopya ve yanına alma | `symbol_missing` |
|---|---|---|
| Python | yok: Kitaplığım masaüstünün kullanıcı dosyasıdır; Python sistem ve proje öğelerine başvurur, bunlar zaten projededir | var: `Document.export_pdf(...)`'ın `findings`'i |
| MCP | yok: aynı gerekçe | var: `file.export.pdf`'in bulguları |
| Yapay zekâ ajanı | yok: aynı gerekçe | var: MCP ile |
| Eklenti | yok: aynı gerekçe | var: SDK'nın aynı dışa aktarması |

- Kanallar bir öğeyi projeye `cad.library.edit`'in kopyalamasıyla açıkça alabilir.
- Masaüstü proje kitaplığını Stil yöneticisinin bugünkü yoluyla yazar (`Styles::changed`, ADR 0092). Kanalların bu
  işi kendi hesabıyla yaptığı bir yer yoktur.

## Açık

- Şablon düzenleyicisinde Kitaplığım sembolü seçilen proje şablonu, sembolü kaydedene ya da yazdırana kadar
  Kitaplığım'ı gösterir; ikinci yol onu yanına alır.
- Stil yöneticisi öğeyi yalnız kimliğiyle seçer. Aynı kimlik hem Kitaplığım'da hem projede olunca iki ağaç da
  projenin kopyasını açar. Bu, Kopyala › Projeye'nin aynı kimlikle taşıdığı çizimlerde de bugün böyledir.

## Doğrulama

26 Kasım 2026'da koşuldu:

- **Kitaplık:** `cargo test -p kentos-style-library --lib` geçti. Yeni sınama
  `a_project_takes_along_the_user_items_it_uses`: kopya adını tutar, çizimi yanında gelir, aynı sembole tek kopya;
  yanına almada sembol, başvurduğu işaret ve işaretin çizimi alınır, kullanılmayan alınmaz.
- **Çekirdek:** `cargo test -p kentos-native-application --lib library_refs` geçti: katman, katman durumu, nesne,
  blok nesnesi ve proje öğelerinin başvuruları.
- **Ön denetim:** `cargo test -p kentos-plot --test all missing_symbols` geçti: eksik sembol bir kez söylenir ve
  yalın görünüşle basılır; projedeki sembol söylenmez ve dolgusu sayfadadır.
- **Çizim çekirdeği:** `cargo test -p kentos-plot`: 120 sınamadan 119'u geçti. Kırmızı olan
  `bands::a_part_s_band_holds_room_in_the_book_and_one_not_read_is_said` CBS projesinde bağlı katman kurar; bunu
  `bound_in_gis` kuralı reddeder (ADR 9196, 23 Kasım 2026). Bu dilimden değildir.
- **Pafta çıktısı:** `RUST_MIN_STACK=67108864 cargo test -p kentos-headless --test headless -- sheet`: 16 sınama
  geçti (biri `ignore`): yayın, ön denetim, teslim, güncelleme ve pafta setleri.
- **Masaüstü:**
  - `cargo test -p kentos-desktop --lib -- style::manager::tests kitapligim` geçti.
  - Yeni sınamalar: `the_project_keeps_the_symbols_of_kitapligim_it_uses` (yuvaya ve nesnelere projenin kopyası,
    aynı sembole tek kopya, eski başvurunun yanına alınması ve iletisi) ve
    `the_print_preview_draws_the_symbols_of_kitapligim` (baskı önizlemesi sembolü projeye alır, söyler, sayfada
    dolgu var).
  - `katman_stili_picks_from_the_library_and_keeps_its_own_symbols` yeni kurala göre güncellendi: "Kitaplığıma
    kaydet"ten sonra yuva projenin kopyasını gösterir.
  - `-- module_tests ribbon_tests history::text catalog settings saving sheet_book sheets:: layer_style exchange::
    style::`: 516 sınamadan 515'i geçti. Kırmızı tek sınama `the_modules_keep_to_their_dependency_baseline`'dır; render
    motoru birleşmesinin üç bağı (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`) önceden
    de vardı, bu dilim yeni bağ eklemedi (`scripts/dev/module_deps.py --check`).
- **Python:** yerel modül yeniden derlendi. `test_missing_symbols` (2 sınama: eksik sembol `export_pdf`'in
  bulgularında bir kez, sistem sembolü söylenmez) ve PDF'e dokunan `test_plot_document`, `test_page_setups`,
  `test_sheet_sets`, `test_plot_ink`, `test_sheet_clips`, `test_sheet_annotations`, `test_sheet_view_links`,
  `test_point_names` geçti (16 sınama).
- **Boyut:** `scripts/dev/source_size.py` bu dilimin dosyalarında bir şey söylemez.
- **Resimler** (`.run/gui-evidence/kullanim-9317/`, `kitapligim_symbols_screens`, büyütülerek bakıldı):
  - `1-cizim-alani.png`: Kitaplığım sembolüyle çizilen alan pembe dolu;
  - `2-pafta.png`: paftada aynı;
  - `3-baski-onizlemesi.png`: baskı önizlemesinde de pembe dolu (önceden yalnız ince kenar çizgisi kalıyordu).
- **Koşulmayan:** gerçek pencerede fareyle deneme (`scripts/usage/real/`) yapılmadı.
