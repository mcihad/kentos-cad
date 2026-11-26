# ADR 9313: Bütün yazı tipleri `fonts` klasöründe, listeler oradan; yazılar dik

- **Durum:** kabul edildi (2026-11-26). Sahibin sözleri (26 Kasım 2026):
  - “Ayrıca fonts altında olmalı tüm fontlar ve listelenebilmeli oradan”;
  - “Yazılar italik yazılmasın özellikle nokta yazıları”.
- **İlgili:**
  - [ADR 0055](0055-desktop-drawing-text.md) (çizim yazı tipleri);
  - [ADR 0175](0175-labels-to-text.md) §1 (tek satırlık yazı eğikti);
  - [ADR 0183](0183-text-and-dimension-styles.md) (yazı stili);
  - [ADR 0324](0324-paragraph-layout-and-fonts.md) §5 (ad kuralı, `cad.fonts.report`);
  - [ADR 9311](9311-arial-times-point-names-labels-off.md) (Arial, Times New Roman, nokta adları);
  - [ADR 9312](9312-install-layout.md) (kurulumun dizinleri).
- **Web:** iptal edildi.
- **`.kcad`:** şema değişmedi. Yazı tipi alanı KentOS'un yüzlerinin adlarının yanında klasördeki bir ailenin adını da
  alır (§3).
- **Bağımlılık:** yok. Yüz bilgisi var olan `rustybuzz`'ın `ttf_parser`'ıyla okunur.

## Bağlam

- **Dosyalar üç yerdeydi:**
  - çizimin yüzleri `apps/desktop/assets/fonts/drawing/`;
  - arayüzünkiler `crates/ui/assets/fonts/`;
  - yedek yüzler (Arapça, İbranice, matematik) `crates/shared/geometry-core/data/text-fonts/`.
  Kurulumun `fonts` klasörü (ADR 9312) yalnız Arial ve Times New Roman'ın lisanslı kopyası için okunuyordu.
- **Listeler sabitti:** Ayarlar, Proje ayarları, yazı stili ve ölçü stili dokuz yüzlük sabit bir liste gösteriyordu.
  Klasöre konan yazı tipi listede görünmüyordu ve seçilemiyordu.
- **Yazılar eğikti:** kendi yüzü olmayan tek satırlık yazı eğik çiziliyordu (ADR 0175 §1). Bu kural masaüstünün
  etiket boyacısında ve paftanın çizicisindeydi. Köşe adları (ADR 9311 §6) tam böyle yazılardır; eğik görünüyorlardı.

### Araştırma (26 Kasım 2026'da okundu)

- **QGIS 3.28 ve sonrası:**
  - kullanıcı profilinin `fonts` alt klasörüne konan TTF ve OTF dosyaları program açılırken yüklenir
    ([changelog](https://changelog.qgis.org/en/entry/2357));
  - `QgsFontManager::installUserFonts` bu klasörü okur, `addUserFontDirectory` başka klasör ekler
    ([API](https://api.qgis.org/api/classQgsFontManager.html)).
- **AutoCAD:**
  - çizimin adlandırdığı yazı tipi bulunmazsa yerine başkası çizilir;
  - TrueType'ta sıra eşleme tablosu, sonra yazı stilinin yüzü, sonra sistemin benzer yüzüdür;
  - SHX'te `FONTALT` (öntanımı `simplex.shx`) devreye girer
    ([About Substitute Fonts](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-928DF015-1E04-4CC2-AF1B-0037548DFBAE.htm),
    [FONTALT](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-LT/files/GUID-858A8D84-5104-4A35-9C82-2094DF6F411D.htm)).
- **OpenType belirtimi:**
  - `name` tablosunun 1 ve 2 numaralı adları aile ve biçemdir, 16 ve 17 numaralıları tipografik aile ve biçemdir;
  - `OS/2` tablosunun `fsType` alanı gömme hakkını söyler. Kısıtlı lisanslı (0x0002) yüz izin alınmadan belgeye
    gömülemez. Gömen uygulama gömmeye izin vermeyen yüzü gömmez
    ([OS/2 — fsType](https://learn.microsoft.com/typography/opentype/spec/os2)).
- **KentOS kararı:**
  - bütün yazı tipleri tek klasördedir;
  - her program bu klasörü ve kullanıcının klasörünü açılışta okur;
  - listeler bu okumadan kurulur.
  - Bulunmayan aile Arial'la çizilir ve söylenir. Bu, AutoCAD'in yerine koymasıyla ve KentOS'un `font_substituted`
    uyarısıyla aynı yoldur.
  - Kısıtlı lisanslı yüz okunmaz: paftanın PDF'i kullandığı harfleri gömer.

## Karar

### 1. Tek klasör: `paket/fonts` (kurulumda `<kaynak>/fonts`)

| Klasör | İçerik |
|---|---|
| `paket/fonts/cizim/` | çizimin yüzleri (Barlow, Arimo, Overpass, Quicksand, Architects Daughter, Courier Prime, IBM Plex Mono) ve Times New Roman'ın ikizi Liberation Serif, lisanslarıyla |
| `paket/fonts/arayuz/` | arayüzün yüzleri (Inter, IBM Plex Sans ve Mono, Noto Sans, Roboto, Source Sans 3, Plus Jakarta Sans, JetBrains Mono), lisanslarıyla |
| `paket/fonts/yedek/` | yazı tiplerinde olmayan harfler için yedek yüzler (Noto Sans, Arapça, İbranice, matematik), lisanslarıyla |
| `<kullanıcı>/fonts/` | kullanıcının koyduğu yazı tipleri; alt klasörler de okunur |

- **Gömülü kopyalar:** programlar yüzleri bu dosyalardan derleme anında içlerine alır (`include_bytes!`). Sınama,
  başsız çalışma ve WASM klasöre bakmadan aynı harfleri çizer.
- **Arial ve Times New Roman:** dosyaları depoya ve pakete konmaz (ADR 9311). Lisanslı kopya `<kullanıcı>/fonts`'a ya
  da `<kaynak>/fonts`'a konabilir. İkisinde de yoksa işletim sisteminin dosyası okunur.
- **Yollar:** dört üretici (`scripts/fonts/*.py`) ve dört örnek üreticisi yeni yolları kullanır. Yedek yüzlerin
  bildirgesi yerinde kaldı (`crates/shared/geometry-core/data/text-fonts/manifest.json`); dosyaları `yedek/`'tedir.

### 2. Listeler klasörden

- **Okuma:** her program açılırken `<kaynak>/fonts`'u ve `<kullanıcı>/fonts`'u okur
  (`kentos_native_application::font_folders::load`). Okuyanlar: masaüstü, `kentos`, `kentos-mcp`, Python modülü.
  - Dosyalar yollarına göre sıralanır (`kentos_launch::install::font_files`).
  - `.ttf`, `.otf`, `.ttc` ve `.otc` okunur, dört düzey alt klasöre kadar.
  - Her yüzün ailesi, biçemi, kalınlığı, eğikliği, Türkçe harfleri ve gömme hakkı kendi tablosundan okunur
    (`kentos_geometry_core::text::face_info`).
- **Kullanım:** her yüz şunlardan biridir.

| Kullanım | Ne zaman | Ne olur |
|---|---|---|
| KentOS'un yüzü | baytları gömülü bir çizim yüzününkiyle aynı | program kendi kopyasıyla çizer |
| Eşdeğer | Liberation Serif | Times New Roman kurulu değilse onunla çizilir |
| Asıl yüz | lisanslı Arial ya da Times New Roman, ya da işletim sisteminin dosyası | ikizin yerine çizer |
| Arayüz | `arayuz/`'da, ailesi KentOS'un bir çizim yüzü olan dosya (IBM Plex Mono) | çizim KentOS'unkini kullanır |
| Yedek | baytları gömülü bir yedek yüzünkiyle aynı | yüzde olmayan harfleri çizer |
| Seçilebilir | başka her aile (Roboto, Inter, kurumun yazı tipi) | çekirdeğe verilir; yazılar adıyla alır |
| Kullanılmıyor | koleksiyon, okunamayan, adı olmayan, kısıtlı lisanslı, ikinci kopya | nedeni not olarak yazılır |

- **Çekirdeğe verme:** seçilebilir aileler ilk yazı çizilmeden önce çekirdeğe verilir (`font_data::install`).
  Çekirdeğin kendi yüzlerinin numaraları değişmez; aileler onların ardından numaralanır (`text::families`).
- **Listeler:** listeler önce KentOS'un dokuz yüzünü gösterir (Arial, Times New Roman ve öbürleri), sonra
  klasördeki aileleri adlarına göre (`font_folders::choices`). Seçili değer listede yoksa (başka bilgisayarda yapılmış
  proje) sona eklenir. Listeyi kullananlar:
  - Ayarlar'ın “Çizim yazı tipi”;
  - Proje ayarları;
  - yazı stili;
  - ölçü stili.
- **Yazı tipleri penceresi (`text.fonts`):** pencerede “Yazı tipi klasörleri” tablosu vardır. Her dosyanın ailesini,
  biçemini, yerini, kullanımını, Türkçe harflerini ve notunu gösterir. Kullanıcının klasörünün yolunu da söyler.
- **Notlar:** değişken yazı tipi varsayılan kalınlığıyla çizilir. Türkçe harfleri eksik yüzde o harfler yedek yüzle
  çizilir.

### 3. Sözleşme: `DrawingFont::Installed(FontFamily)`

- **Biçim:** yazı tipi tek bir metindir.
  - KentOS'un yüzleri kimlikleriyle yazılır (`courier-prime`).
  - Klasördeki aile adıyla yazılır (`Roboto`): kırpılmış, 1–64 harf, denetim karakteri yok.
  - KentOS'un bir yüzünün adı (`Times New Roman`, `arial`) o yüzdür.
- **Okuma:** JSON şeması iki seçeneklidir (`anyOf`). `.kcad` dosyası aileyi adıyla yazar ve okur. Sütunlar ailenin
  yerini dokuzuncu sıra olarak, adını ardından metin olarak tutar.
- **Bellek sınırı:** adlar program boyunca birer kez tutulur. Bir programın okuyabileceği ad sayısı 65 536 ile
  sınırlıdır (`MAX_FONT_FAMILIES`): uydurulmuş adlarla bellek sonsuz büyütülemez.
- **Ayarlar:** `newProjects.drawingFont` ve `project.drawingFont` klasördeki bir ailenin adını da alır
  (`SettingDescriptor::font`).
- **Bozuk örnek:** `style-face-font-unknown.kcad` artık hiçbir ailenin alamayacağı 65 harflik bir ad yazar.

### 4. Klasörde olmayan aile

- **Çizim:** proje, ailesi bu bilgisayarın klasörlerinde olmayan bir yazı tipini adlandırabilir; çekirdek adı korur.
  Yazılar Arial'la çizilir ve ölçülür (`Font::drawn`).
- **Uyarı:** `cad.fonts.report` her eksik aile için `font_substituted` uyarısı verir. Değişiklik hiçbir zaman sessiz
  değildir.
- **Dosya:** ad dosyada kalır. Aileyi tutan bilgisayar yazıları kendi yüzüyle çizer.

### 5. Çıktılar

- **PDF:** pafta PDF'i klasör ailelerinin kullanılan harflerini gömer (`plot::fonts::pdf_fonts`).
- **SVG ve resim:** klasörün yüzleri SVG'ye gömülür, resimde çizilir (`bundled_faces`). İşletim sisteminin Arial'ı
  hiçbir zaman gömülmez (ADR 9311).
- **DXF:** STYLE kaydı ailenin adını ve `<Aile>.ttf` dosya adını yazar. DXF'in adlandırdığı bir yazı tipi
  klasördeki aileye uyuyorsa ona çözülür (`FontMatch::Folder`).

### 6. Yazılar dik

- **Eski kural kalktı:** “kendi yüzü olmayan tek satırlık yazı eğik çizilir” kuralı masaüstünde (`labels::face_font`)
  ve paftanın çizicisinde (`plot::labels::face`) kaldırıldı.
- **Yeni kural:** yazı, kendi yüzü eğik demedikçe diktir. Nokta adları, kenar uzunlukları ve köşe yazıları dik
  çizilir.
- **Değişmeyenler:** sahne (GPU) ve DXF zaten dik çiziyordu. Yazı stilinin “İtalik”i ve eğim açısı yerindedir.

### 7. Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Harita | var | Yazılar, ölçüler, etiketler, köşe adları yazı tipini kullanır |
| Yol | var | Aynı |
| Mimari | var | Aynı (mahal ve kapı yazıları, ölçüler) |
| CBS | var | Açıklama katmanının yazıları ve etiketler aynı yazı tipini kullanır |

### 8. Kanallar

| Kanal | Var / yok | Nasıl |
|---|---|---|
| Python | var | Modül yüklenirken klasörleri okur. `cad.fonts.report`'un `files` ve `choices`'ı aynı listeyi verir. Yazı ve ayar komutları ailenin adını alır. |
| MCP | var | `kentos-mcp` açılırken okur; aynı komutlar. |
| Yapay zekâ ajanı | var | MCP ile aynı. |
| Eklenti | var | Aynı komutlar; `cad.fonts.report` salt okunurdur. |

Tam uyum: her program klasörleri aynı işlevle okur (`font_folders::load`). Listeyi aynı komut verir, doğrulamayı
aynı sözleşme yapar. Kanal kendi listesini kurmaz.

## Uygulama

- **Sözleşme:**
  - `crates/shared/contracts/src/core/drawing_font.rs` (yeni);
  - `document.rs` ve `annotation.rs`'teki eski tanım taşındı;
  - `settings/{mod,rules,schema}.rs`;
  - `crates/shared/commands/src/core/cad_fonts.rs` (`FontFile`, `FontFolder`, `FontRole`, `choices`, `files`) ve
    `cad_standards.rs`.
- **Çekirdek:** `crates/shared/geometry-core/src/text/{families,face_info}.rs` (yeni); `font_data.rs`, `shaping.rs`,
  `fonts.rs`, `mod.rs` (`Font(u16)`, `drawn`, `missing`), `entity.rs`, `shaping/placed.rs`.
- **Klasörler:**
  - `crates/native/launch/src/install.rs` (`font_files`);
  - `crates/native/application/src/core/font_folders.rs` (yeni) ve `fonts_report.rs`;
  - `crates/native/headless/src/lib.rs` (`load_fonts`) ve onu çağıranlar: `apps/cli`, `kentos-mcp`, Python modülü.
- **Dosya ve çıktılar:**
  - `crates/shared/kcad/src/{decode,columns}.rs`, `decode/{styles,objects/object}.rs`, `encode/names.rs`;
  - `crates/native/plot/src/{fonts,labels}.rs`, `crates/render/wgpu/src/scene.rs`;
  - `crates/shared/formats/src/dxf/{styles,writer/styles}.rs`, `blocks.rs`;
  - `crates/native/scene-query/src/geometry.rs`, `crates/native/style/src/line_types/text.rs`,
    `crates/native/processing/src/audit_facts/cad.rs`.
- **Masaüstü:**
  - `apps/desktop/src/cad/drawing_fonts.rs` (`load_font_folders`), `launch.rs`;
  - `cad/font_report.rs`, `cad/annotation_styles.rs`, `cad/labels/{mod,paint,ink}.rs`;
  - `shell/project/{settings,mod}.rs`, `shell/settings_view.rs`.
- **Taşınan dosyalar:** `paket/fonts/{cizim,arayuz,yedek}/` (70 dosya, `git mv`). Yollarını okuyan kod, üreticiler ve
  belgeler güncellendi.

## Doğrulama (26 Kasım 2026)

- **Sözleşme:** `cargo test -p kentos-contracts --lib` 1 679 sınama geçti. Bunlar arasında yazı tipinin dört
  sınaması var: kimlik ve ad, kendi yüzün adı, alınamayacak ad, adın bir kez tutulması.
- **Çekirdek:** yazı sınamaları (`--lib text` 67, `--test all text_shaping` 28) geçti.
  - Yedek yüzün adını “seçilebilir yüz değil” diye atlayan sınama yeni kurala göre güncellendi: bilinmeyen ad artık
    bu bilgisayarda olmayan bir klasör ailesidir. Tolerans değişmedi.
  - `face_info`'nun sınamaları gömülü yüzlerin ailesini, kalınlığını ve eğikliğini okur.
- **Dosya:** `cargo test -p kentos-kcad` 284 sınama geçti. Yeni `font_families`: aile `.kcad`'e adıyla yazılıp aynı
  okunur, sütunlar da taşır, kendi yüzlerin dosyası baytı baytına aynı kalır. `kcad_v2_reference.py --check`: 1 893
  dosya tutarlı.
- **Kurulum:** `cargo test -p kentos-launch` 20 sınama geçti. `every_font_is_under_the_fonts_folder`: üç klasör ve
  sıralı dosya listesi.
- **Uygulama:**
  - `font_folders`'ın üç sınaması geçti: depodaki klasörün kullanımları, kullanıcı klasörünün ailesi ve
    kullanılmayanların nedeni, listenin sırası.
  - `fonts_report`'un iki sınaması geçti: seçimlerin sırası, eksik ailenin uyarısı.
  - `cad.fonts.report`'un ortak durumları (`every_command_case`, `capabilities`) geçti.
- **Çıktılar:**
  - `kentos-plot`: 97 sınama geçti. `bands::a_part_s_band_holds_room_in_the_book…` bu işten önce de kırmızıydı.
  - `kentos-formats` DXF: 104 sınama geçti. `dxf_write_reference.py --check`: 11 çıktı doğru.
  - `kentos-sheet`: 523 sınama geçti.
  - `kentos-headless`: 35 sınama geçti.
- **Masaüstü:** etiketler, yazı tipleri, stil penceresi, ayarlar, modül, şerit, katalog, geçmiş ve pafta süzgeçleriyle
  223 sınama geçti. `the_modules_keep_to_their_dependency_baseline` bu işten önce de kırmızıydı: nedeni
  `shell/approved_layer.rs`, `shell/gpu_frame.rs` ve `gis/data_selection.rs`'teki bağlardır, bu iş onlara dokunmadı.
- **Python:**
  - `test_font_folders` (üç sınama) ve `test_shx_texts` geçti;
  - `cad.fonts.report`'un ortak durumları geçti;
  - `sdk.py --check` temiz.
- **Üreticiler:** `font_report_cases.py --check` (50 durum), `text_fallbacks.py --check`, `drawing_fonts.py --check`,
  `sheet_metrics.py --check` ve `drawing_metrics.py --check` temiz.
- **Resimler** (`.run/gui-evidence/kullanim-9313/`, `font_folders_screens`):
  - `kose-adlari-arial.png`, `kose-adlari-roboto.png`: köşe adları P00001–P00006 dik, Arial'la ve klasördeki
    Roboto'yla;
  - `yazi-tipleri-penceresi.png`: klasörlerde 55 dosya, 7 seçilebilir aile;
  - `proje-ayarlari-yazi-tipi.png`: projenin yazı tipi Roboto.
- **Koşulmayanlar:**
  - gerçek pencerede fare ve klavyeyle kabul;
  - Windows ve Linux'ta okuma;
  - `kentos-ui`'nin kendi sınamaları (sınamada `snapshot` özelliğini ister; yazı tipi yolları masaüstü derlemesinde
    derlendi);
  - donmuş web'in `sheet-wasm` sınaması.

## Açık kalanlar

- **Paftanın kendi yazıları:** başlık bloğu ve pafta yazıları paftanın kendi yüz listesini kullanır
  (`crates/sheet-ui`, `data/font-metrics.json`). Klasör aileleri orada henüz sunulmaz.
- **Koleksiyonlar:** `.ttc` ve `.otc` dosyaları listelenir ama kullanılmaz; yüzleri ayrı dosya olarak konmalıdır.
- **Değişken yazı tipleri:** varsayılan kalınlıkla çizilir; kalın yazı için ayrı kalın dosya gerekir.
- **Yeniden okuma:** klasöre konan dosya programın yeniden açılışında okunur. Çalışırken yeniden okuma yoktur.
- **Python'un yeni projesi:** `kentos.cad.Document.new`'un öntanımı hâlâ `barlow`'dur; masaüstünün sihirbazı ve
  `newProjects.drawingFont` Arial'dır (ADR 9311). Öntanımı Arial'a çevirmek yeni proje açan 59 Python sınamasının
  ölçülerini değiştirir; ayrı işte yapılır.
