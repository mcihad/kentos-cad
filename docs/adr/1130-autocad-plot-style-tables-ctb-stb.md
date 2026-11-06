# 1130 — AutoCAD baskı stili tabloları (CTB/STB): okuma, açık eşleme, kayıp raporu ve yayında kullanım

Tarih: 2026-11-06. Durum: kabul (CAD-ADV-16'nın “ikili CTB/STB” açığı). Kapsam: plan/cad-advanced.md `CAD-ADV-16`;
ADR 0609'un ertelediği ikili codec (§“Alışveriş stratejisi”), ADR 0610'un mürekkep anlamları, ADR 1090/1112'nin yayın
akışı. Web: ertelendi (ADR 0228); web'de gerekenler §8'de.

## Sorun

ADR 0609 ikili `.ctb/.stb` dosyalarını bilerek reddetti (`unsupported_plot_profile`) ve bir codec eklendiğinde uyulacak
beş kuralı yazdı: kaynağın SHA-256'sı ve biçimi tutulur, açılmış boy sınırlanır; CTB'nin gerçek indeksleri ve STB'nin
gerçek adları kullanılır, RGB'den kalem tahmin edilmez; her özellik açıkça eşlenir, eşlenmeyen için adlandırılmış kayıp
ya da ret vardır, sessiz kırpma yoktur; bilinmeyen alan uyduğu iddia edilmeden raporlanır; doğrulanamayan biçim
yazılmaz. Ofislerin elinde AutoCAD'den gelen CTB/STB'ler var; KentOS'ta bunlar elle yeniden kurulmak zorundaydı.

## Araştırma

- **Autodesk Plot Style Table Editor** ([AutoCAD LT 2015 yardımı](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT-MAC/files/GUID-2770BBA8-AF39-4278-8B65-C3FF84A66C12.htm)):
  renk (varsayılan “Use object color”), titreşim (dither), gri tonlama, kalem no (1–32, Automatic), sanal kalem no
  (1–255; sanal kalemde öteki ayarlar yok sayılır), tarama (0–100; **titreşim açık değilse uygulanmaz**), çizgi tipi,
  uyarlanır çizgi tipi, kalınlık, uç, birleşim, dolgu; renge bağlı tabloda stil eklenip silinemez; adlı tabloda
  **Normal** nesnenin özelliklerini kullanır ve değiştirilmez.
- **Biçim** ([ezdxf `addons/acadctb` belgesi](https://ezdxf.readthedocs.io/en/stable/addons/acadctb.html) ve kaynağı,
  MIT): 48 baytlık `PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec` (STB'de `STBVER1`), ardından üç küçük-sonlu u32
  (sıkıştırılmış gövdenin Adler-32'si, NUL dahil açılmış uzunluk, sıkıştırılmış uzunluk), sonra zlib akışı; metin
  `ad=değer` satırları ve `ad{ … }` blokları; dizgi `"` ile açılır, kapanmaz; bir değerin ` (…)` eki atılır. Renk
  `0xTTRRGGBB`'nin işaretli 32 bit değeri (TT 0xC2 RGB, 0xC3 ACI; `-1` nesnenin), `color_policy` bitleri (1 titreşim,
  2 gri, 4 adlı renk), çizgi tipi 0–30 + 31 (nesnenin), uç 0–4, birleşim 0–3 + 5, dolgu 64–73, `lineweight` kalınlık
  tablosunun sırası (mm). Bu yalnız bağımsız biçim başvurusudur; ürüne Python/ezdxf bağımlılığı eklenmedi.
- **Doğrulanamayanlar:** bu makinede AutoCAD yok, bağımsız gerçek CTB/STB korpusu da indirilmedi (güvenilmeyen dosya
  indirilmez). Bu yüzden (1) üst bilgideki Adler-32'nin ve uzunluğun AutoCAD'in yazdığıyla aynı anlamda olduğu, (2)
  “Use object lineweight”ın dosyadaki değeri (ezdxf kalınlık tablosunda 0,00 mm'yi nesnenin kalınlığı okur; tablo dışı
  bir sıranın, örneğin 255'in, ne olduğu belgelenmemiş), (3) `mode_color`'ın anlamı, (4) metnin kod sayfası doğrulanmadı.
  Karar her birini adıyla raporlamak ya da reddetmektir (§3).

## Karar

### 1. Kapsam

Var: ikili `.ctb/.stb` okuyucusu (`kentos_sheet::plot_profile::acad`), `PlotProfile.source` (biçim, SHA-256, dosya adı),
anlamsal komut `cad.plotstyles.import` (dosyayı okur, profili kitaba yazar, kalemleri katmanlara/kâğıt öğelerine açıkça
verir, paftalara uygular; tek değişiklik), masaüstü **Baskı stili tablosu…** penceresi (`sheet.plotStyles`), yayının bu
profille basması, bağımsız Python okuyucusu ve 37 ortak durum, MCP uçtan uca PDF sınaması.

Yok (sınır, §7): CTB/STB **yazıcısı** (biçim AutoCAD'le doğrulanmadı; ADR 0609 §5), aynı tablonun güncellenerek yeniden
alınması (`replace`), UTF-8 dışı kod sayfası seçimi, AutoCAD'in 1–30 baskı çizgi tiplerinin ve dolgu desenlerinin
KentOS desenine eşlenmesi, paketin özgün `.ctb`'yi de taşıması.

### 2. Dosya okuma

- Boy ≤ 4 MiB (`plot_style_file_too_large`), açılmış metin ≤ 8 MiB; üst bilgi CTB/STB değilse
  `unsupported_plot_style_file`; üst bilgi kısa, sıkıştırılmış uzunluk dosyayla tutmuyor, zlib akışı bozuk, satır
  `ad=değer`/blok değil, blok kapanmıyor ya da açılmamış, stil sıraları 0'dan birer artmıyor, CTB'de 255 stil yok, alan
  aralık dışı (tarama 0–100, çizgi tipi 0–31, uç 0–4, birleşim 0–3/5, dolgu 64–73, kalınlık 0–100 mm, kalem 0–255, renk
  32 bit) → `bad_plot_style_file`; UTF-8 olmayan metin `plot_style_encoding`; CTB üst bilgisi ile
  `aci_table_available=FALSE` (ya da tersi) `plot_style_kind_mismatch`; STB'de aynı ad iki kez `duplicate_plot_style`;
  4096'dan çok stil `plot_profile_too_large`.
- Üst bilgideki Adler-32 sıkıştırılmış gövdeye, uzunluk açılmış metne uymazsa metin yine zlib'in kendi Adler-32
  denetimiyle okunur ve bu bir **not**tur (`checksum`, `length`); ret değildir, çünkü üst bilginin AutoCAD'deki anlamı
  doğrulanmadı.
- Profil kimliği `ctb-`/`stb-` + dosya SHA-256'sının ilk 12 hanesi; adı verilen ya da dosya adının gövdesi.

### 3. Alanların eşlenmesi

Her satır üç ağırlıktan biridir: **lost** (KentOS nesnenin görünüşünü basar), **approximate** (KentOS'un en yakın kuralı),
**note** (basılan aynıdır, kayıt için söylenir). Rapor satırları türlerin sırasıyla; her satırın stilleri (CTB'de ACI
numarası, STB'de ad) ve dosyadaki değerleri ilk görülüş sırasıyla, birer kez.

| Dosya alanı | KentOS | Rapor |
|---|---|---|
| `color = -1` ya da `0xC3FFFFFF` | `object` | — |
| `color` türü 0xC2/0xC3 | `color #rrggbb` (dosyanın RGB'si) | — |
| başka tür (ByLayer 0xC0, ByBlock 0xC1, 0xC4 …) | `object` | `colorKind` lost (tür iki onaltılık hane) |
| `mode_color ≠ color` | kullanılmaz | `modeColor` lost |
| gri biti, nesnenin rengi | `grayscale` (0,2126 R + 0,7152 G + 0,0722 B, ADR 0610) | `grayscale` approximate |
| gri biti, açık renk | `color` gri: `(2126 R + 7152 G + 722 B + 5000) / 10000` tamsayıyla | `grayscaleColor` approximate |
| 3'ten büyük policy bitleri | yok sayılır | `policyBits` lost |
| `screen < 100`, titreşim açık | `screening` | `dither` approximate (düz karışım rengi) |
| `screen < 100`, titreşim kapalı | `screening 100` (AutoCAD taramaz) | `screenWithoutDither` note |
| fiziksel / sanal kalem ≠ 0 | kullanılmaz | `physicalPen` / `virtualPen` lost |
| çizgi tipi 0 | `dash: []` (sürekli) | — |
| çizgi tipi 1–30 | nesnenin çizgi tipi | `linetype` lost (`n: Ad`) |
| çizgi tipi 31 | nesnenin | — |
| kalınlık: tablonun mm değeri | µm; üç basamağa kadar kesin, sonrası yarım sıfırdan uzağa | `lineweightRounded` approximate |
| kalınlık değeri 0 µm | nesnenin kalınlığı (biçim başvurusunun okuyuşu) | — |
| tablo dışı sıra (örn. 255) | nesnenin kalınlığı | `lineweightOutside` lost (doğrulanmadı) |
| uç 0/1/2, birleşim 0/1/2 | düz/kare/yuvarlak, köşeli/pahlı/yuvarlak | — |
| uç 3, birleşim 3 (elmas) | nesnenin | `endStyle` / `joinStyle` lost |
| dolgu 64–72 | nesnenin dolgusu | `fillStyle` lost |
| stil açıklaması | taşınmaz | `description` lost |
| STB `Normal` | profilde stil değildir (nesnenin görünüşü, ADR 0609) | değişikse `normalChanged` lost |
| bilinmeyen alan (üstte ya da stilde) | okunmaz | `unknownField` lost (alan adı) |
| `apply_factor = TRUE` | uygulanmaz (çizgi tipi/dolgu eşlenmediği için) | `scaleFactor` lost |
| `adaptive_linetype`, `linepattern_size`, `localized_name`, `custom_lineweight_display_units` | yalnız biçimi denetlenir | — |

Bir stil **aynen** sayılır, onu adlandıran lost ya da approximate satırı yoksa. CTB'nin 255 kalemi profile ACI 1–255
anahtarıyla girer; atanmayan kalem hiçbir şeyi değiştirmez (ADR 0609: atanmayan renk bağımlı kaynak nesne görünüşünü
korur). **Kalem katmanın RGB'sinden tahmin edilmez:** ACI 1 ve 10'un aynı kırmızıyı vermesi gibi belirsizlikler yüzünden
`assignments` açıkça verilir.

### 4. Komut `cad.plotstyles.import` v1

Girdi: `path` (mutlak ya da `base`'den), `name?`, `assignments[]` (`{target:{type:modelLayer|paperItem,id}, key}`),
`sheets[]`, `expectedRevision?`. Sıra: sürüm; boş yol ya da geçersiz ad `invalid_input`; dosya yok ya da klasör
`file_not_found`; okunamıyor `file_unreadable`; okuyucunun retleri (§2); kitap okunamıyor `book_unreadable`; aynı ad ya da
aynı dosya (kimlik) kitapta `duplicate_plot_profile`; atama okunamıyor `invalid_input`, hedef katman yok
`layer_not_found`, kâğıt öğesi yok `unknown_item`; aynı hedefe iki atama ya da tabloda olmayan kalem/stil
`bad_plot_assignment`; olmayan pafta `unknown_sheet` (yinelenen pafta bir kez). Yazma `cad.sheets.edit`'in yoludur:
`savePlotProfiles` (var olanlar + yeni) ve her pafta için `setSheetPlot` (profil kimliği ve kopyası,
`profileSnapshot`; sayfa düzeninin öteki ayarları korunur). Çıktı: `SheetsEdited` + `profile`, `format`,
`description`, `styles`, `exact`, `losses`, `applied`. Lost/approximate satır varsa `plot_style_losses` uyarısı. Plan
yazmaz; masaüstünde pencere komutu bir çizim grubunda çalıştırır (tek geri alma, ADR 0590).

### 5. Yayında kullanım

Profil paftaya uygulanınca mevcut yayın yolu (`plot_profile::for_sheet` → `apply::map`/`paper`, ADR 0610) onu basar; yeni
bir yayın kodu yoktur. MCP sınaması: ADR 0610'un 17. ink durumunun çizimi ve paftası, profilsiz sayfa düzeni; çizginin
PDF kalınlığı önce katmanınki; `cad.plotstyles.import` `siyah.ctb`'nin 1. kalemini (0,50 mm, siyah) `cizim` katmanına
verip paftaya uygulayınca aynı çizgi PDF'te **0,50 mm, siyah** (bağımsız PDF okuyucu, `tests/support/files.rs`).

### 6. Arayüz

Çıktı › Pafta › **Baskı stili tablosu…** (`sheet.plotStyles`, ikon `colorModeMono`, takma adlar `BASKISTILI`, `CTB`,
`STB`), `RoadDialog` iskeletinde: Tablo (dosya, Dosya seç…, profilin adı), Kalemler (her katman için kalem/stil seçimi,
“—” verilmez), Paftalar (uygula anahtarları). Sağda planın rakamları (Biçim, Stil, Aynen, Kayıp, Yaklaşık), kaç alan türünün
basılmadığını söyleyen tek bant ve iki tablo arasında seçim: **Rapor** (ağırlık, konu, stiller, değerler, açıklama) ve
**Stiller** (renk, tarama, kalınlık, uç/birleşim, verildiği katman). İçe aktar tek adım; Kapat yazmaz. Kalem seçimi
tablo değişince sıfırlanır. İnceleme: ilk resimde 18 rapor satırı ayrı bantlar olarak stillerin tablosunu pencereden
taşırıyordu; rapor tabloya alındı.

### 7. Sınırlar

- Yazıcı yok: KentOS profili `.kplot` olarak yazılır; `.ctb/.stb` yazmak AutoCAD'le karşılaştırmalı doğrulama ister.
- AutoCAD'in 30 baskı çizgi tipi ve 9 dolgu deseni KentOS'ta yok; eşlenmeleri desen tanımlarının kaynağını ister.
- Kod sayfası: UTF-8 dışındaki (ör. CP1254 Türkçe adlı STB) reddedilir; kullanıcı seçimi sonraki iştir.
- Aynı dosyanın güncellenmiş sürümünü mevcut profilin yerine, atamalarını koruyarak koymak (`replace`) yok.
- Paketleme profili `.kplot` olarak taşır (ADR 1111); özgün `.ctb` baytları pakete konmaz, profil SHA-256'yı taşır.
- Gerçek AutoCAD ile karşı denetim (aynı CTB ile AutoCAD ve KentOS PDF'i) yapılmadı.

### 8. Web'e ertelenenler (TODOS.md §0.2)

`cad.plotstyles.import` işleyicisi (WASM'da `kentos_sheet::plot_profile::acad::read` aynı çekirdek), `PlotProfile.source`
için `io/` ve tasarımcının profil penceresi, üretilmiş `PlotStylesImport.ts`, `PlotStylesImported.ts`,
`sheet/PlotSource.ts`, `sheet/PlotLoss*.ts`, `sheet/PlotStyle{Format,Read}.ts`; ortak durumların web koşucusu.

## Uygulama

- Çekirdek: `crates/shared/sheet/src/plot_profile/acad.rs` (okuyucu, eşleme, rapor; 2 birim testi),
  `plot_profile.rs` (`source`, doğrulaması).
- Sözleşme ve komut: `crates/shared/commands/src/cad_plot_styles.rs`, `catalog.rs`, `capabilities.rs`;
  `crates/shared/contracts/src/codes.rs` (10 kod); `crates/native/application/src/plot_styles_import.rs`, `registry.rs`,
  `lib.rs` (`DESKTOP_COMMANDS`); MCP yıkıcı listesi (`tools.rs`: paftaların baskı ayarını değiştirir).
- Masaüstü: `apps/desktop/src/plot_styles_window.rs` (+ testleri), `plot_style_scenes.rs`, `app.rs`, `view.rs`,
  `traces/answers.rs`; `catalog/v1/{additions,operations,ribbon}.json`, `scripts/ui/ribbon.py`.
- Otomasyon: `python/kentos/cad/plotstyles.py` (üretildi), `types.py`; `docs/cad/manifest.*`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/plot_style_cases.py` (Python standart kütüphanesi; KentOS kodu yok): 17 test
  tablosunu belgelenen biçimden kendisi yazar (`fixtures/commands/v1/plotstyles/`), kendi okuyucusuyla §2–§4'ün
  kurallarını yeniden uygular ve `fixtures/commands/v1/cad.plotstyles.import.json`'un **37 durumunu** yazar: siyah CTB
  (255/255 aynen), katmana ve kâğıt öğesine kalem, iki paftaya uygulama, plan/doğrula yazmaz, kitapsız proje, bütün
  kayıp türleriyle `kalemler.ctb` (255 stilden 239'u aynen; 18 rapor satırı), adlı STB (Normal stil değil, Türkçe adlar),
  STB stilinin katmana verilmesi, değişik Normal, sağlaması/uzunluğu tutmayan üst bilgi (not), aynı ad ve aynı dosya,
  olmayan katman/öğe/pafta, iki atama, olmayan stil, okunamayan atama, boş yol/ad, olmayan dosya, klasör, 12 bozuk
  dosya reddi, sürüm. `--read DOSYA` herhangi bir tabloyu bu okuyucuyla JSON olarak yazar; `--check` hem JSON'u hem
  tabloları karşılaştırır.
- Native koşucu: 37/37 (ilk çalıştırmada 36; sürüm çakışmasının `revision` alanı başvuruya eklendi). Yetenekler (4
  test; `plot_style_file_too_large`, `file_unreadable` ve `book_unreadable` UNREACHED'de gerekçeli, ilk ikisi
  `plot_style_files_beyond_the_limit_and_unreadable_files_are_refused` testinde), katalog (`KENTOS_WRITE_CATALOG=1`),
  `kentos-sheet` `plot_profile` (17), masaüstü pencere testleri (2: önizleme yazmaz, kalem katmana, A-101'e uygulanır,
  tek Undo geri alır; Kapat yazmaz, tablo olmayan dosya söylenir), MCP `an_agent_imports_a_ctb_and_publishes_with_its_pen`.
- Çalıştırılmayanlar: `pnpm py:test` (paylaşılan `.run/py` ortamına `maturin develop` ile yerel modül kurmak ana çalışma
  ağacını değiştirir; `sdk.py --check` geçti); AutoCAD ile karşı denetim (ürün yok); gerçek AutoCAD CTB korpusu (indirme
  yapılmadı).
