# ADR 1090: Genel 2B detay paftaları, sayfa içerik özeti, yayın durumu ve devam ettirilebilir toplu yayın

- **Durum:** kabul edildi (5 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-16` (“Genel 2B pafta ve yayın işini tek
  güvenilir üretim akışına bağla”) ve ADR 1030 §9'un bıraktığı “tablonun paftalar arası devamı”. İlke “Performance
  First”; madde tek parçada biter.
- **Bağlam belgesi:** ADR 0594 (açık görünüm kaynağı, oran, kilit), 0597 (kırpma), 0598 (pafta görünümünde model
  düzenleme), 0599 (canlı görünüm bağı), 0605 (görünüm katmanı ve açıklama temsili), 0609 (baskı profili), 0615 (sıralı
  yayın), 0617 (kaynaklı teslim), 0618 (ön denetim), 0619 (sabit kaynak sürümü), 0718 (kat planı paftaları), 1030
  (tablo parçaları).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler §10'da.

## Bağlam

Pafta kitabı, analitik kâğıt geometrisi, açık görünüm kaynağı/oran/kilit, kırpma, katman ve açıklama temsili, baskı
profili, atlas, şablon, ön denetim, sıralı ve atomik yayın, sabit kaynak pini mevcut. `cad.sheets.set` bir nesnenin
değişikliğinin hangi görünüm penceresini etkilediğini söylüyor. Eksik olanlar:

1. **Genel 2B detay seti:** aynı modelin 1:1/1:5/1:20/1:100 detaylarını tek komutla, mimari kat verisi olmadan kurmak
   yoktu (`cad.sheets.levels` kat ister). Her detayı `setViewport` ile tek tek kurmak mümkündü ama tarif (kaynak kapsam,
   oran, katman durumu, açıklama temsili, kilit) geri okunup aynı seti yeniden kurmak için tek yapıda verilmiyordu.
2. **Tablonun paftalar arası devamı:** ADR 1030 §7'nin parçaları model tablosunda yan yana duruyordu; bir parçayı ayrı
   paftaya koymanın kapsamı elle bulunuyordu.
3. **Etkilenen çıktı:** `cad.sheets.set` görünümün katman durumunu (tema, liste, ezme) hesaba katmıyor, değişen nesneyi
   okuyan tabloların gösterildiği görünümleri bulmuyordu.
4. **Kaynak ile son yayın:** yayın kaynak pinini `yayin.json`'a yazıyordu, fakat sonradan hangi paftanın güncelliğini
   yitirdiğini söyleyen bir karşılaştırma yoktu. Revizyon çalışma zamanı kimliği, belge SHA'sı her değişiklikte bütün
   seti eski gösterir; tarih alanı her gün bütün paftaları değiştirir.
5. **PDF'in kendisinin denetimi:** ön denetim kaynağı denetliyordu; yazılan sayfanın vektör çizgi, aranabilir metin,
   yazı yüzü, sıfır kalınlıklı çizgi, kırpma, saydamlık, tarama/dolgu ve resme dönmüş görünüm sayımı yoktu.
6. **Toplu yayında devam:** yayın atomikti (ADR 0615): durdurma ya da tek bir sayfa hatası bütün işi siliyordu;
   kapsam özeti yoktu.

### Araştırma

- AutoCAD'in Publish penceresi işi “Plot and Publish Details” penceresine ve günlük dosyasına yazar; başarısız sayfadan
  devam eden resmî bir seçenek bulunmadı, yeniden yayın Sheet Set Manager'dan seçilen paftalarla yapılır
  ([Publish Dialog Box, AutoCAD LT 2026](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-7BCCDFDC-562F-43A4-83F2-CEAE10C0DA64.htm),
  [Sheet set'i yayın için değiştirme](https://help.autodesk.com/cloudhelp/2020/ENG/AutoCAD-Core/files/GUID-64980F13-1BE6-4FD4-A838-35B8837802B1.htm)).
  KentOS kararı: günlük yerine yapılı bir devam kaydı ve sayfa sayfa içerik özeti; yeniden yayın yalnız eski paftalar.
- AutoCAD'de “Display Locked” görünüm penceresinde yakınlaştırma pencerenin ölçeğini değil bütün düzeni değiştirir;
  katman kilidi görünüm kilidi değildir
  ([To Lock the Scale of a Layout Viewport](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-F9A37755-68A8-489D-B508-253EDD88F356.htm),
  [Viewport Lock](https://help.autodesk.com/cloudhelp/2021/ENU/AutoCAD-LT/files/GUID-3E11A784-E4E6-44CA-9F1B-2A84ECB40425.htm)).
  KentOS'ta `viewLocked` (ADR 0594) zaten bu anlamdadır; detaylar varsayılan olarak kilitli kurulur.
- DraftSight'ın pafta seti ve LibreCAD'in baskı önizlemesi (plan [R17], [R23]): model ölçeği ile kâğıt ölçeğinin
  ayrı ve açık olması.

## Karar

### 1. Kapsam

Var: `cad.sheets.details` (detay seti), tablo parçası kaynağı, `cad.sheets.set`'in detay tarifi/kilit alanları ve katman
durumu + tablo etkisi, PDF sayfa denetimi ve içerik özeti, `file.publish.sheets`'in kapsam özeti, sayfa denetimi,
`resumable`/`resume`, yeni `file.publish.status` sorgusu; masaüstü yayın penceresinde kapsam, sayfa denetimi, Durum ve
Devam et. Yeni `.kcad` şeması gerekmez: detay tarifi görünüm penceresinin saklanan alanlarından okunur; yayın kaydı
teslim klasöründedir (AutoCAD'in DST'si gibi çizimin dışında).

Kapsam dışı (sınır, §9): masaüstünde detay seti kuran ayrı bir pencere (komut Python/MCP/komut satırından; sonra ADR
1112), ikili
CTB/STB, dönen detay kapsamı, AutoCAD/Acrobat ile karşı denetim.

### 2. Detay paftaları (`cad.sheets.details` v1, `kentos_sheet::details`)

- Detay: `name`, `bounds` (model metresi) ya da `table {table, part}`, `ratio {paper, model}` (aynı birimde; 1:5 =
  1, 5; 2:1 = 2, 1), isteğe bağlı `layers` (MapLayers: tema/liste), `layerOverrides`, `annotation {fixed, scale}`,
  `locked` (yok: true). Mimari kat verisi istenmez.
- Pencere boyu: `w = round((doğu₂ − doğu₁) · (1e6 · paper / model))` µm, yükseklik aynı biçimde; `f64::round` (yarım
  sıfırdan uzağa). 1 mm'den küçük pencere `detail_too_small`.
- Yerleşim: şablonun ilk sabit görünüm penceresinin iç alanı (varsayılan `sys:cad-teknik`: 370 × 208 mm, 30/20 mm'de).
  Hücre `max(w, 40 mm)` geniş, `h + 6,5 mm` derin (1,5 mm boşluk + 5 mm başlık). Verilen sırayla soldan sağa; satır
  genişliğe sığdıkça (aralık 10 mm), pafta yüksekliğe sığdıkça (aralık 10 mm) dolar; sığmayan detay setin sonraki
  paftasına geçer. Boş alana bile sığmayan detay `detail_does_not_fit` (gereken ve var olan mm'yi söyler).
- Paftalar `detay-1`, `detay-2` … (alınmışlar atlanır), numaralar `first`'ten (yok: D-501; son rakamlar genişliği
  korunarak artar, kullanılan atlanır), ad “D-501 Detaylar” (n > 1 paftada “ (i/n)”), çizim bilgisi alt kümesi
  “Detaylar”. Şablonun penceresi detaylara yer açar (RemoveItems); her detay `<pafta>-detay-k` penceresi ve
  `<pafta>-detay-k-baslik` yazısı (“<ad> — Ölçek [% @olcek %]”, haritası o pencere: yazı görünümün gerçek ölçeğini okur).
- Pencere: `view.fixed {center = kapsamın ortası, ratio, scale = round(payda)}`, `viewLocked`, katman ve ezmeler,
  `annotationScale`. Hepsi tek `cad.sheets.edit` değişikliği, tek undo.
- Retler sırası: `invalid_revision`, `revision_conflict`; her detay için `entity_not_found`, `not_a_table`,
  `table_part_missing`, `ambiguous_source`, katman/temsil okunamazsa `bad_value`; `unknown_template`, `invalid_options`
  (ad/merkez/ölçek verilmez); detay yok `bad_value`; her detay için `bad_value` (ad), `invalid_extent`,
  `invalid_ratio`, açıklama paydası `bad_value`, `detail_too_small`, tekrar eden ad `bad_value`; `not_fixed_map`;
  `bad_value` (başlık); `bad_drawing_info`; `detail_does_not_fit`.
- **Tekrar kurulabilirlik:** `cad.sheets.set` her pencere için `detail` (aynı yapı: merkez ± içerik/2 · payda,
  oran, katman, ezme, temsil, kilit) ve `locked` verir. Bu tarif `cad.sheets.details`'e verilince aynı pencereler
  kurulur (dönmüş ya da başka kaynaklı pencerede `detail` yok).

### 3. Tablonun paftalar arası devamı

`table {table, part}`: parçanın satırlarının derinliği ve tablonun genişliği, sol üst köşeden `x = k·(genişlik +
aralık)` (ADR 1030 §7), çerçeve bandı + yazı yüksekliğinin yarısı her yandan büyütülmüş model kutusu. Her parça ayrı
detay olur; set paftalara dağıtır. Tablo bir kez modeldedir; parçaların gösterildiği paftalar tablonun değişikliğinden
etkilenir (§6).

### 4. Sayfa denetimi ve içerik özeti (`kentos_sheet::pdf::PageAudit`)

PDF yazıcısı her sayfayı yazarken sayar: kontur (`strokes`), dolgu (`fills`, delikli `evenOddFills`), sıfır kalınlık
(`hairlines`, cihazın en ince çizgisi), 0,05 mm'den ince çizgi (`thinLines`, `minStrokeMm`), gerçek metin koşusu
(`texts`; bütün yüzler ToUnicode'lu gömülü alt kümedir, metin aranabilir), kullanılan yüzler (`fonts`), kırpma
(`clips`), alfa < 1 boyama (`transparent`), resim, resme dönmüş görünüm (`rasterMaps`), boş “harita” kutusu
(`placeholders`). Bulgular (hiçbiri tek başına hata değildir; hata ön denetimdedir): `pdf_hairline`, `pdf_thin_line`,
`pdf_transparency`, `pdf_rasterized_view`, `pdf_empty_view`, `viewport_live_layers` (§5).

İçerik özeti: SHA-256 ( “kentos.page-digest/1”, sıkıştırılmamış içerik akışı, sayfa boyu, kullanılan yüz, resim ve
grafik durumu adları; her parça uzunluk önekli). Kaynak adları belge geneli sıralamaya bağlı olduğundan özet her pafta
**tek başına** ve **başlık bağlamı olmadan** (kullanıcı, tarih boş; `Capture::neutral`) üretilir: ertesi gün ya da
başka kullanıcı paftayı eskitmez; içerik değişikliği eskitir.

### 5. Baskı başka kullanıcının katman aç/kapasından etkilenmesin

Baskı paleti sabittir (`paper_style`); arayüz teması PDF'e girmez. Görünümün katmanları `All` ya da `List` ise çizimin
anlık görünürlüğünü izler: aynı projede biri katmanı kapatınca baskı değişir. Katman durumu (tema) bağlı görünüm bunu
izlemez. Yayın her sayfada `drawing` katmanlı sabit görünüm için `viewport_live_layers` uyarısı verir; detaylar
`layers: {type: theme}` alabilir.

### 6. Etkilenen çıktı ve yayın durumu

- `cad.sheets.set {uids}`: `shows` artık görünümün bastığı katmanları (tema, liste, ezme, donma) da sınar; yeni
  `table` nedeni: değişen nesneyi okuyan model tablosu (koordinat/alan/öznitelik tablosunun nesnesi, blok çizelgesinin
  saydığı ekleme, çizelgenin `members` satırı, hücre alanının nesnesi) görünümün zemin kutusuna giriyorsa;
  `AffectedFrame.tables` o tabloları sayar. Neden sırası: shows, table, marked, unknown. Veri çıkarımı tablosu açık
  “Tabloyu güncelle” ile yazıldığından buraya girmez (`cad.extract.run` `stale` söyler).
- `file.publish.status` v1 (sorgu, dosya yazmaz): teslim klasörünün `yayin.json`'u varsa her yayınlanmış pafta için
  şimdiki içerik özetini (yayının kullanıcı/tarih ve dpi bağlamında, özet bağlamsız) yayındakiyle karşılaştırır:
  `current`, `stale`, `failed` (şimdi üretilemiyor, bulgusuyla), `fileChanged` (PDF eksik ya da baytları farklı),
  `removed`, `unpublished`, `unknown` (özetsiz eski yayın). Şimdiki kaynak pini (`current`) ile yayının kaynağı
  (`published`) ayrı verilir. `stale` yeniden üretilecekleri yayın sırasıyla verir; `file.publish.sheets`'in `sheets`
  girdisine olduğu gibi verilir. Devam kaydı varsa `stopped` (neden, tarih, bitmiş/kalan, bulgular, `resumable`).

### 7. Toplu yayın: kapsam, sayfa denetimi, devam

- `SheetsPublished.scope`: her pafta için numara, revizyon, kâğıt mm, baskı profili adı, görünümleri (payda, kilit,
  katman kaynağı, açıklama temsili), tablo sayısı — plan ve doğrulamada da. `pages`: her sayfanın denetimi ve özeti;
  `yayin.json` da `pages`, `scope` ve `user` taşır.
- `resumable` (yalnız ayrı PDF'ler): durdurulan ya da bir sayfası başarısız yayın bitmiş sayfaları ve
  `devam.json`'u (biçim `kentos.publish-resume/1`: yol, kullanıcı, tarih, kaynak pini, seçenekler, seçilen paftalar,
  bitmiş dosyalar ve sayfalar, bulgular, neden) teslim klasörünün yanındaki gizli `.<klasör>.kentos-devam`'da tutar;
  teslim klasörü hiçbir zaman yarım görünmez (ADR 0615 korunur). Kayıt her sayfadan sonra geçici dosya + yeniden
  adlandırma ile yazılır. `resumable` olmadan davranış ADR 0615'inkidir.
- `resume`: kayıttaki kullanıcı/tarih bağlamıyla yeniden yakalar; belge SHA'sı, kaynak ve bağlam özeti, seçenekler ve
  pafta listesi aynı değilse `resume_stale` (bitmiş sayfalar yeni kaynakla karıştırılmaz; çalışma zamanı revizyonu
  karşılaştırılmaz, yeniden açılmış çizim de devam edebilir). Bitmiş dosya diskteki SHA-256'sı kayıttakiyle aynıysa
  tutulur (`resumed`), değilse yeniden yazılır. Kayıt yoksa `no_resume`; tek PDF'te `invalid_input`. Başarıda kayıt
  silinir, klasör tek yeniden adlandırmayla teslim edilir.

### 8. Arayüz

Masaüstü “Paftaları toplu yayınla” penceresi: kapsam satırları (numara · kâğıt · profil · görünümler “Ad 1:20 ·
kilitli”), sayfa denetimi satırları ve bulguları, **Durum** (teslim klasörünü çizimle karşılaştırır, her paftayı
güncel/güncelliğini yitirdi/… gösterir, durdurulan yayını raporlar) ve **Devam et** (devam edilebilir kayıt varken).
Pencere ayrı PDF'te `resumable` ile yayınlar; Durdur sonrası ileti bitmiş sayfaların saklandığını söyler. Yeni ikon
gerekmedi (düğmeler pencerenin içindedir).

### 9. Komutlar ve otomasyon

`cad.sheets.details` (semantic, belge), `file.publish.status` (sorgu, MCP'de salt okunur), `file.publish.sheets`'in
yeni alanları. Python SDK katalogdan üretilir; MCP araçları katalogdandır. Ortak durumlar:
`fixtures/commands/v1/cad.sheets.details.json` (32 durum, `scripts/fixtures/sheet_details_cases.py --check`).

### 10. Performans

Yayın her sayfayı içerik özeti için bir kez daha (bağlamsız, tek başına) çizer: yayın süresi yaklaşık iki katına
çıkar; ilerleme çubuğu bunu ayrı yarı olarak gösterir. Durum sorgusu her yayınlanmış paftayı bir kez çizer, arka plan
işidir ve iptal edilebilir. Denetim sayaçları yazarken toplanır (ek gezinti yok).

## Uygulama

- **Pafta çekirdeği** (`crates/shared/sheet`): `details.rs` (detay, pencere boyu, yerleşim, öğeler, geri okuma,
  `detail_sheets`); `pdf/content.rs`, `pdf/maps.rs` sayaçlar (`Page::note_paint`, kırpma, resim, raster/boş görünüm);
  `pdf/mod.rs` `PageAudit`, `page_digest`, `to_pdf_audited`, `THIN_LINE_MM`; `pdf/write.rs` `document_audited`;
  `pdf/fonts.rs` `base_name`. Yazılan PDF baytları değişmez.
- **Sözleşme/komut** (`crates/shared/contracts`, `crates/shared/commands`): `cad_sheet_details.rs`; `file_publish.rs`
  (`resumable`, `resume`, `scope`, `pages`, `resumed`, `file.publish.status` türleri); `cad_sheet_set.rs` (`recipes`,
  `detail`, `locked`, `AffectReason::Table`, `AffectedFrame.tables`); `codes.rs` altı yeni kod; katalog, yetenekler,
  `FILE_COMMANDS`.
- **Native** (`crates/native`): `application/src/sheets_details.rs` (tablo parçası kutusu, komut), `sheets_set.rs`
  (katman durumu süzgeci, tabloyu okuyan nesneler); `plot/src/book.rs` (`BookPdf.pages`), `provenance.rs`
  (`Capture::neutral`, `context`); `headless/src/publish.rs` (kapsam, sayfa denetimi, özet, devam kaydı),
  `publish_status.rs`, `file_commands.rs`; `mcp` salt okunur `file.publish.status`.
- **Masaüstü** (`apps/desktop/src/exchange/sheet_publish.rs`): kapsam ve sayfa denetimi satırları, Durum, Devam et,
  durdurma iletisi; `sheet_set.rs` `table` nedeninin sözü.
- **Otomasyon:** Python SDK yeniden üretildi (`python/kentos/cad/sheets.py`, `file/publish.py`, `types.py`, …).
- **Web:** yazılmadı (ADR 0228); TS türleri ve katalog JSON'u üretildi.

## Doğrulama

### Bağımsız başvurular

- `scripts/fixtures/sheet_details_cases.py --check`: 32 durum (dört ölçek, 2:1, 1:37,5, satır/pafta taşması, 40 mm
  hücre, ikinci çağrı numarası/kimliği, katman/ezme/temsil, kilitsiz, tablo parçaları, 17 ret). Şablonun sayfası,
  öğeleri ve haritası kendi dosyasından okunur; pencere boyu, yerleşim, numara ve parça kutusu ADR'den yazıldı. Native
  koşucu 32/32 ilk çalıştırmada geçti.
- `crates/native/headless/tests/headless/sheet_sets.rs` (beklenen uzunluklar model uzunluğu × 1000 / payda olarak
  testte hesaplanır, KentOS çıktısından alınmaz):
  1. 1:1/1:5/1:20/1:100 dört pafta ve aynı yerin iki 1:5 görünümü: biri değiştirilince öbürünün JSON'u aynı; `recipes`
     tarifi yeni çizimde aynı çerçeve/oran/merkez/kilidi kurar; ayrı PDF'ler bağımsız okuyucuyla (plot testlerinin
     `files.rs`'i) 420×297 mm, 116,619/23,324/5,831/74,000/18,500 mm doğrular; Ölçü çizgisi ikizlerden yalnız birinde;
     Poppler `pdftotext` “Bulon — Ölçek 1/1”, “… 1/5”, “… 1/20”, “… 1/100” metnini bulur, `pdffonts` bütün yüzleri
     gömülü gösterir; her sayfada kontur, metin, yüz, kırpma > 0, raster görünüm 0, `viewport_live_layers` uyarısı.
  2. Yayın sonrası durum: hepsi `current`; Ölçü çizgisi 1 cm kayınca `cad.sheets.set` etkilenen paftaları =
     `file.publish.status`'un `stale`'i (1:1 paftası güncel kalır; Ölçü'yü donduran görünüm etkilenmez); yalnız eski
     paftalar yeni klasöre yayınlanır, özetleri durumun “şimdi” özetine eşit; elle bozulan PDF `fileChanged`.
  3. İlk sayfa kaydedilince durdurulan yayın: teslim klasörü yok, durum `stopped/cancelled`, bitmiş ≥ 1, kalan ≥ 1,
     `resumable`; `resume` bitmişleri korur (`resumed`), klasörü bütün teslim eder, kayıt silinir; sonra `no_resume`.

### Testler (bu oturumda, hedefli)

Tam takım koşturulmadı; hedefli koşular:

| Komut | Sonuç |
|---|---|
| `kentos-sheet` (birim + bütünleşim, PDF/SVG altınları dahil) | 416 + 80 geçti (4 yok sayılan önceden) |
| `kentos-commands --lib` (katalog yazımı, birimler, kod anlamları, TS) | 1549 geçti |
| `kentos-contracts --lib export_bindings` | 1134 geçti |
| `kentos-native-application --test all` `KENTOS_CASES_ONLY=cad.sheets,cad.navigator` + yetenekler + sheet_tools | 301 ortak durum, 7 test geçti |
| `kentos-headless --test headless sheet_` | 11 geçti, 1 düştü (`sheet_preflight` “substituted-model-font”: önceden var olan, aşağıda) |
| `kentos-plot` | 18 + 70 geçti (3 yok sayılan önceden) |
| `kentos-mcp --no-fail-fast` | hepsi geçti, `desktop_attach_names_the_program_whose_link_it_takes` düştü (worktree yolu Unix soket sınırını aşıyor: “path must be shorter than SUN_LEN”; ortam) |
| `kentos-desktop --lib` yayın penceresi testi + `adv16_detail_set_screens` | 2 geçti |
| `sheet_details_cases.py --check`, `sheet_set_cases.py --check`, `sdk.py --check`, `manifest.py --check` | aynı |

Clippy: `kentos-sheet`, `kentos-contracts`, `kentos-commands`'ta bu maddenin dosyalarında uyarı yok; iki uyarı
(`sheets_details`, `sheets_set`) düzeltildi. `kentos-native-application`'ın önceden var olan `expect_used`
hataları (standards.rs, table_export.rs, road_*) ve `kentos-road`'un 1.96 lintleri `-D warnings`'li koşuyu
durdurduğundan headless/plot/mcp/desktop'ın clippy'si tamamlanamadı.

### Görsel inceleme

`adv16_detail_set_screens` (açık/koyu 13 px, koyu 18 px): 1:5 paftası, iki temsilli 1:5 paftası, tablonun ikinci
paftadaki parçası (1440×900); yayın penceresinde kapsam/sayfa denetimi, güncelliğini yitiren paftalar, durdurulan yayın
ve devam (1100×720). İncelemede bulunup düzeltilenler: detay pencerelerinin çerçevesi çizilmiyordu (ince kenar
eklendi); durum sonucu eski planın raporunun altında kalıyordu (durum sorulunca rapor kalkar, durum bloğu rapordan önce);
kapsam ve sayfa satırları dosya adlarından önceye alındı.

### Çalıştırılmayanlar

Python takımı (`pnpm py:test`, maturin derlemesi), tam Rust takımı, release süre ölçümü, web; AutoCAD/Acrobat karşı
denetimi yapılmadı. `sheet_preflight` başsız testinin “substituted-model-font” durumu bu dalda düştü (beklenen
`missing_font`, bulunan `text_font_substituted`); bu madde font/ön denetim koduna dokunmadı, `main`'de ayrıca
denetlenmedi.
