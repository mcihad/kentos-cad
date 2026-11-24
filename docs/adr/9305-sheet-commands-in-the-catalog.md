# ADR 9305: Pafta tasarımcısının işleri masaüstünün kataloğunda

- **Durum:** kabul edildi (2026-11-24). `todos-new.md` A8. [ADR 0247](0247-sheet-commands-on-the-desktop.md)'nin yerlerini
  kataloğa bağlar; [ADR 0164](0164-sheet-layouts.md) (pafta kipi), [ADR 0251](0251-sheet-book-in-the-project-file.md)
  (kitap projenin dosyasında, kâğıdın kendi geri alması), [ADR 0252](0252-first-acceptance-drawing.md) §2
  (`cad.sheets.edit`) ve [ADR 0590](0590-sheet-and-model-transactions.md) (kâğıt ve model tek adımda) geçerlidir.
- **Kural:** CLAUDE.md “Modül ve otomasyon denetimi” (24 Kasım 2026) ve §25 kural 2, 3, 6.
- **Web:** iptal edildi. **`.kcad`:** değişmedi. **Sözleşme ve ürün komutu:** değişmedi; Python, MCP ve eklenti SDK'sı
  yeniden üretilmedi.

## Bağlam

Donmuş web envanterinde (`docs/inventory/web.json`) 59 `sheet.*` komutu vardır. Masaüstü bunlardan yalnız üçünü
çalıştırıyordu: Modele dön, Yeni pafta ve Şablondan pafta. Eklerden gelen Pafta seti, Çizime sığdır, Seçime sığdır, Kat
planı paftaları, Detay paftaları ve Baskı stili tablosu da çalışıyordu.

Kalan 56 komut `Standing::OnTheWeb` idi. Komut ara ve komut satırı “web'de var; masaüstüne henüz taşınmadı”
diyordu. Oysa pafta tasarımcısı (`kentos-sheet-ui`) bu işlerin biri dışında hepsini kendi şeridinden,
paftalar listesinden, öğe ağacından ve tuşlarından yapıyordu. ADR 0247 bu yerleri `apps/desktop/equivalents.json`'a
yazmıştı, ama katalog onları çalıştıramıyordu.

Manifest `sheet.grid`'i bekleyen, `sheet.zoomIn` ve `sheet.zoomOut`'u yarım sayıyordu. Yakınlaştırma yalnız tekerlekle
ve imlecin çevresinde yapılıyordu.

## Karar

1. **Her komut tasarımcının kendi iletisidir** (`apps/desktop/src/cad/sheets/commands.rs`). Komut, düğmenin, listenin,
   ağacın ya da tuşun gönderdiği `kentos_sheet_ui::Message`'ı gönderir. Hesap ve yazma tasarımcıdadır; katalog ikinci
   bir kural yazmaz. 55 kimlik `PORTED`'a ve `apps/desktop/ported.json`'a girdi.
2. **Kapalıyken ne yapılacağı söylenir.** Kâğıt üstündeki bir komut için önde bir pafta gerekir ve kâğıt modele
   verilmemiş olmalıdır. Neden ipucunda ve Komut ara'nın satırında görünür; komut satırından çalıştırılınca aynı söz
   uyarı olarak yazılır. Sözler Komut ara'nın tek satırına sığacak kadar kısadır:
   - “Açık çizim yok.”
   - “Projede pafta yok: Çıktı › Yeni pafta ile bir pafta açın.”
   - “Önde pafta yok: alttaki sekmelerden bir paftayı açın.”
   - “Model düzenleniyor: önce Kâğıda dön'e basın.”
   - Seçim yetmezse: “Önce paftada bir öğe seçin.”, “En az iki öğe seçin.”, “En az üç öğe seçin.” (Dağıt; çekirdek
     üçten azını reddeder), “Tek bir öğe seçin.”, “Önce bir grup seçin.”, “Seçili öğeler kilitli: önce kilitlerini
     açın.”
   - Paftanın sırası: “Pafta zaten en solda.” ve “Pafta zaten en sağda.”
   - Kâğıdın geçmişi: “Geri alınacak pafta işlemi yok.” ve “Yinelenecek pafta işlemi yok.”
   - Atlas: tasarımcının profilindeki Atlas aracının nedeni (ör. “Koordinat sistemi gerekir: …”); profilde Atlas yoksa
     “Bu proje türünün paftalarında atlas yok.”
3. **Kitabın kendi komutları önde pafta istemez.** PAFTA (`sheet.open`) ilk paftayı açar (web'deki gibi).
   `.kpafta dosyasından…` ve kâğıdın Geri al / Yinele'si çizim öndeyken de çalışır; bir pafta silindikten sonra da Geri
   al onu getirir.
4. **Masaüstünün başlıkları.** Web'deki “Sayfaya göre”, “Kenar boşluklarına göre” ve “Seçime göre” öbür hizalamaların
   neye göre yapılacağını seçen düğmelerdi. Tasarımcıda böyle bir seçim yoktur; Hizala ▾ bunları hemen yapar (Sayfanın
   ortasına, Kenar boşluklarının ortasına). Bu yüzden başlıkları yaptıkları işi söyler: “Sayfanın ortasına hizala”,
   “Kenar boşluklarının ortasına hizala”, “Seçimin ortasına hizala” (`DESKTOP_TITLES`, açıklamalarıyla). Üçü de öğeleri
   yatayda ortalar.
5. **Gizle ve Kilitle bütün seçimi alır**, hepsi gizliyse (kilitliyse) hepsini açar. İş tek geri alma adımıdır; adımın
   adını çekirdek koyar (“Gizle: Harita”, `kentos_sheet::ops::describe`). Ağaçtaki göz ve kilit düğmeleri öğe öğe
   çalışmayı sürdürür.
6. **Kaydır** bir yön ister; katalogdan çalıştırılınca ok tuşlarını söyler (1 mm, Shift ile 10 mm, Alt ile 0.1 mm).
   **Paftayı sil** sekmenin × düğmesi gibi soru sormadan siler, silindiğini ve Geri al'ın getireceğini yazar. Paftalar
   listesinin Sil'i de sormaz.
7. **Yakınlaştır ve Uzaklaştır** tasarımcıya yeni iletiler olarak eklendi (`Message::ZoomIn`, `ZoomOut`;
   `View::stepped`). Web'in adımlarından (%5–%1600) bir sonrakine, sahnenin ortası yerinde kalarak geçerler; kâğıt yeniden
   çizilir. Pafta öndeyken + ve − tuşları onlarındır. Tekerlek imlecin çevresinde yakınlaştırmayı sürdürür.
8. **Karelaj bekler** (`sheet.grid`). Tasarımcı bir haritaya karelaj ekleyemez: Karelaj düğmesinin arkasında iş yoktur.
   Şablondan gelen karelaj çizilir, ama denetçide düzenlenemez. Komut “web'de var” durumunda kalır.
9. **Kayıtlar.** `equivalents.json`'da Yakınlaştır ve Uzaklaştır `implemented` oldu; Karelaj'ın yeri düzeltildi.
   Manifestte masaüstünde olmayan CAD tarafı öğeleri 13'ten 11'e indi; Karelaj kaldı.

### Komut → tasarımcının iletisi

“Önde pafta” şu demektir: çizim açık, projede pafta var, bir pafta önde ve kâğıt modele verilmemiş.

| Komut | `kentos_sheet_ui::Message` (ya da iş) | Ne zaman açık |
|---|---|---|
| `sheet.model`, `sheet.new`, `sheet.fromTemplate` | `Model`, `NewSheet`, `Gallery(Open)` (önceden vardı) | ADR 0164, 0622 |
| `sheet.open` | `Tab(1)`: ilk pafta | projede pafta var |
| `sheet.importKpafta` | `ImportKpafta` (dosya penceresi, sonra `ImportText`) | çizim açık |
| `sheet.undo`, `sheet.redo` | `Undo`, `Redo` | kâğıdın geçmişinde adım var |
| `sheet.pageSetup` | `InspectorTab(Page)`: Sayfa sekmesi, şeridin Sayfa'sı | önde pafta |
| `sheet.variables` | `Variables(Open)` | önde pafta |
| `sheet.rename` | `StartRenameSheet(öndeki)`: ad listede yazılır | önde pafta |
| `sheet.duplicate` | `DuplicateSheet(öndeki)` | önde pafta |
| `sheet.delete` | `CloseTab(sekmesi)` | önde pafta |
| `sheet.moveLeft`, `sheet.moveRight` | `MoveTab(i, i − 1)`, `MoveTab(i, i + 1)` | önde pafta, en solda / en sağda değil |
| `sheet.tool.select`, `sheet.tool.hand` | `Tool(Select)`, `Tool(Hand)`; elde olan yanık | önde pafta |
| `sheet.align.left` … `.bottom` (6) | `Align(Left`, `Center`, `Right`, `Top`, `Middle`, `Bottom)` | bir öğe |
| `sheet.alignTo.page`, `.margins` | `AlignTo(Page)`, `AlignTo(Margins)` | bir öğe |
| `sheet.alignTo.selection` | `AlignTo(Selection)` | iki öğe |
| `sheet.distribute.hCenters`, `.hGaps`, `.vCenters`, `.vGaps` | `Distribute(X/Y, Centers/Gaps)` | üç öğe |
| `sheet.matchSize.width`, `.height` | `MatchSize(Width)`, `MatchSize(Height)` | iki öğe |
| `sheet.order.front`, `.forward`, `.backward`, `.back` | `Order(Front`, `Forward`, `Backward`, `Back)` | bir öğe |
| `sheet.group`, `sheet.ungroup` | `Group`, `Ungroup` | iki öğe; seçimde grup |
| `sheet.atlas` | `Atlas` → `file.export.atlas`'ın penceresi | önde pafta, profilde Atlas açık |
| `sheet.renameItem` | `StartRenameItem(seçili)`: ad ağaçta yazılır | tek öğe |
| `sheet.duplicateItems`, `sheet.deleteItems` | `Duplicate`, `Delete` | bir öğe; Sil için hepsi kilitli değil |
| `sheet.hideItems`, `sheet.lockItems` | `Designer::apply`: tek `hide` / `lock` işlemi | bir öğe |
| `sheet.nudge` | ok tuşlarını söyler (tuşlar `Nudge(dx, dy)` gönderir) | bir öğe |
| `sheet.selectAll`, `sheet.escape` | tasarımcının Ctrl+A ve Esc iletisi | önde pafta |
| `sheet.zoomPage`, `.zoomReal`, `.zoomSelection` | `ZoomPage`, `ZoomReal`, `ZoomSelection` | önde pafta; Seçim'de bir öğe |
| `sheet.zoomIn`, `sheet.zoomOut` | yeni `ZoomIn`, `ZoomOut` | önde pafta |
| `sheet.preflight` | `InspectorTab(Preflight)` | önde pafta |
| `sheet.print` | `Print` | önde pafta |
| `sheet.export.pdf`, `.svg`, `.png`, `.kpafta` | `Export(Pdf`, `Svg`, `Png`, `Kpafta)` | önde pafta |
| `sheet.saveTemplate` | `SaveTemplate(Open)` | önde pafta |
| `sheet.grid` | yok: bekliyor (madde 8) | — |

### Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Mimari | var; Atlas yok | Profilin pafta türü vardır (`ModuleProfile::sheet`). Çıktı › Pafta (Yeni pafta, Şablondan, Pafta seti, Detay paftaları, Baskı stili, Kat planı paftaları) komut kümesindedir (`catalog/v1/modules.json`). Atlas, kapsam katmanının her nesnesine bir sayfa veren harita serisidir. `ribbon.json` `modules.only` onu Harita ile CBS'ye verir; tasarımcının CAD profili de koordinat sistemi yokken Atlas'ı gizler. |
| Harita | var; Atlas var | Pafta ve atlas (pafta bölümlemesi, `file.export.atlas`) Harita'nın işidir. |
| Yol | var; Atlas yok | Yol paftaları ve Çıktı › Pafta vardır; atlas Yol'un işi değildir (`modules.only`). |
| CBS | var; Atlas var | Çıktı › Pafta ve Atlası PDF yap CBS şeridindedir. |

Kodda karar şöyle durur:

- Hiçbir şeridin tutmadığı komut kabuğundur, yani her modülündür (`Catalog::module_has`). Atlas `modules.only`'dedir.
- `each_module_offers_the_sheet_commands_and_atlas_where_maps_are` dört modül × 55 komutun her hücresini sınar.
- Komut kümeleri değişmediği için `modules.json` yeniden üretilmedi; `module_catalog` sınaması onun güncel olduğunu
  doğrular.
- `kentos-cad` açık projenin modülünü izler. KentOS Mimari'de Atlas “KentOS Harita'da ve KentOS GIS'te var” der.

### Kanallar

Pafta kitabını değiştiren işlerin kanalları:

- Hizala, Dağıt, Aynı boy, Sıra, Grupla, Grubu çöz.
- Öğe Çoğalt, Sil, Gizle, Kilitle, Kaydır ve Ad ver.
- Paftaya ad ver, Paftayı çoğalt, sil ve taşı.
- Sayfa sekmesinin kâğıt, yön, kenar ve ızgara ayarı.
- Değişkenler.

| Kanal | Var / yok | Nasıl ulaşılır |
|---|---|---|
| Python | var | `kentos.cad.sheets.edit(ops=[…])`. Çekirdeğin aynı işlemi kullanılır: `align`, `distribute`, `matchSize`, `reorder`, `group`, `ungroup`, `addItems`, `removeItems`, `hide`, `lock`, `moveItems`, `renameItem`, `renameSheet`, `duplicateSheet`, `removeSheet`, `moveSheet`, `setPage`, `setSnapGrid`, `applyPageSetup`, `saveVariables`. Geri almak için cevaptaki `inverse` kullanılır; açık grupta model geri almasına girer (ADR 0590). |
| MCP | var | Katalog aracı `cad.sheets.edit`; her modülün kapısından erişilir (`cad.run`; `arch.run`, `map.run`, `gis.run`, `road.run`). `DESTRUCTIVE`'tedir: önce `plan`, sonra `execute`. |
| Yapay zekâ ajanı | var | MCP ile aynı araç katmanı (`kentos_headless::agent_tools`). |
| Eklenti | var | `document.write` izniyle `run` / `call` üzerinden `cad.sheets.edit`. Geri al ve yinele eklentiye kapalıdır (`not_for_extensions`); eklenti kendi yazdığını cevaptaki `inverse` ile geri verir. |

Öbür işler:

| İş | Python, MCP, ajan, eklenti |
|---|---|
| Yeni pafta, Şablondan, Kat planı ve Detay paftaları, Çizime / Seçime sığdır | `cad.sheets.create`, `.levels`, `.details`, `.fit` (manifestte `semantic`) |
| PDF olarak…, Yazdır… | `file.publish.sheets` (Python `kentos.cad.file.publish.sheets`). Ayrıca Python'da `Document.export_pdf`, MCP'de `drawing.exportPdf`. Yazdır'ın görüntüleyicisi yalnız arayüzündür. |
| Atlas | `file.export.atlas` (Python `kentos.cad.file.export.atlas`). Atlasın kuralları `cad.sheets.edit` `setAtlas` ile verilir. |
| Ön denetim | `file.preflight.sheets` (Python `kentos.cad.file.preflight.sheets`). Tasarımcının sekmesindeki çekirdek bulgularının yanında PDF yazıcısının bulgularını da okur. |
| Seç, El, yakınlaştırma, Tümünü seç, Esc, PAFTA, Modele dön | gerekmez: kâğıdın görünümü ve seçimi; çizimi değiştirmez |
| SVG olarak…, PNG olarak…, `.kpafta` olarak…, `.kpafta dosyasından…`, Şablon olarak kaydet | yok (aşağıdaki açıklar) |

### Açıklar (kuralın “tam uyum” maddesi; yeni komut ailesi bu işte yazılmadı)

1. **Tasarımcının düzenlemeleri ürün komutundan geçmiyor.** Tasarımcı çekirdeğin aynı işlemini
   (`kentos_sheet::ops::apply_all`) kendi kopyasına uygular. Ev sahibi kitabın tamamını `Document::set_sheets` ile
   çizime yazar (`sheet_book.rs`). Kanallar ise aynı işlemi `cad.sheets.edit` ile yapar. Sonuç aynıdır, ama komut aynı
   değildir:
   - Kayıtta `cad.sheets.edit` ve girdisi yerine bir parça değişikliği (“Pafta”) görünür.
   - `expectedRevision` denetimi yoktur.
   - Tasarımcının bir düzenlemesi komut olarak yeniden oynatılamaz.
   - Geri alma tasarımcının kendi yığınındadır (ADR 0251 §3).

   Değişen veri `.kcad`'in `sheets` parçasıdır: paftalar, öğeler, çerçeveler, bayraklar, sıra, adlar, sayfa ve
   değişkenler. Bu, madde 1'deki listede kitabı değiştiren bütün komutlar için tek açıktır.
2. **SVG ve PNG** (`sheet.export.svg`, `.png`): masaüstü `kentos_plot::preview::page_captured` ile yazar, ürün komutu
   yoktur. Python, MCP, ajan ve eklenti paftanın resmini yazamaz.
3. **`.kpafta` dışa ve içe aktarma** (`sheet.export.kpafta`, `sheet.importKpafta`): ürün komutu yoktur. İçe aktarma
   kitaba paftalar, değişkenler, sayfa düzenleri ve resim baytları ekler (tasarımcının `import_kpafta`'sı). Bir betik
   aynı işlemleri `cad.sheets.edit` ile verebilir, ama `.kpafta` dosyasını okuyan bir komut yoktur.
4. **Şablon olarak kaydet** (`sheet.saveTemplate`): bu cihazın ya da hesabın şablon kitaplığına yazar, ürün komutu
   yoktur. Projenin verisi değişmez.
5. **Karelaj** (`sheet.grid`): tasarımcı yapamaz. Kanallar ise `cad.sheets.edit` `setItemProps` ile haritanın
   `kind.grids`'ine karelaj yazabilir. Bu tek iş, arayüzün kanaldan geride kaldığı yerdir.
6. **Manifest:** `sheet.preflight`'ın `catalog/v1/operations.json`'da `via`'sı yoktur. Bu yüzden “Okuma, Python'da yok”
   listesinde durur, oysa `file.preflight.sheets` aynı işi görür. `via` bu işin dosyası değildir.
7. **Hızlı erişim ve tuş ipuçları** pafta komutlarına ulaşmaz. Çubuğa ekleme, kataloğun şerit düğmesinin sağ tıkından
   ya da sabit önerilerden yapılır; tuş ipuçları da katalogdan kurulan şeridindir. Pafta sekmesi ise tasarımcının
   kendi gruplarıdır (`ribbon_groups`), katalogdan kurulmaz. Çubuğa bir pafta komutu konmuş olsaydı artık çalışırdı.

## Doğrulama

Sınamalar:

- `cargo test -p kentos-sheet-ui --lib --test designer`: 53 + 54 geçti. Yeni sınamalar
  `view_math::tests::the_zoom_steps_keep_the_stage_s_middle` (%57 → 67, 75, 100; dışa 50; uçlar) ve
  `the_zoom_steps_go_in_and_out` (iletiler, + ve − tuşları).
- `cargo test -p kentos-desktop --lib sheets`: 36 geçti, 14 yok sayıldı. Yeni `sheets::command_tests` 6 sınama taşır:
  - her `sheet.*` komutu çalışır, Karelaj dışında; `PORTED`'daki sıra ve tuşsuzluk;
  - pafta yokken, çizim öndeyken, komut satırında ve model düzenlenirken nedenler;
  - her iletinin işi (geri alma adımının adıyla), Gizle/Kilitle, Grupla, Çoğalt, Sil, kâğıdın geri alması, araçların
    yanıklığı, Esc, yakınlaştırma, paftanın çoğaltılması, taşınması ve silinmesi;
  - pencereler ve sekmeler;
  - Komut ara'nın soluk satırı ve çalıştırması;
  - modüllerin her hücresi.
- `catalog::` 12, `module_tests` 29, `ribbon_tests` 6, `history::text` 5, `typed_names` 10, `ribbon_search` 7,
  `settings` 51, `module_catalog` 3, `shortcuts` 6, `other_names` 4: hepsi geçti.
- `cargo test -p kentos-cad --test programs -- --skip every_trace_passes_in_every_variant`: 36 geçti. Bunlara
  `module_surface_tests` (28) ve `every_ported_command_has_a_handler` dahildir.
- `KENTOS_WRITE_PORTED=1 … ported_file_is_current` ile `ported.json` yeniden yazıldı; sınama geçer.
- `python3 scripts/cad/manifest.py --check`: tutarlı, 1094 satır.
- `python3 scripts/dev/module_deps.py --check`: taban korunuyor.

Resimler `.run/gui-evidence/pafta-komutlari-9305/` altındadır, Git'e eklenmez (açık 1440 ve koyu 1100):

- `1-pafta-onde-hizala`: pafta önde, Komut ara “hizala” için tasarımcının dokuz komutunu açık ve “Pafta” yeriyle
  gösterir;
- `2-cizim-onde-yakinlastir`: çizim önde; kâğıdın Yakınlaştır'ı soluktur ve nedenini söyler, çizimin Yakınlaştır'ı
  açıktır.

Koşulmayanlar:

- `every_trace_passes_in_every_variant` uzun sürer ve izlerde pafta komutu yoktur.
- Gerçek pencerede kullanıcı kabulü (`scripts/usage/real/`) yapılmadı; sahip Mac başındaydı.
- Python takımı koşulmadı; ürün komutu ve SDK değişmedi.
