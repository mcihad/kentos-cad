# Klasik arayüzün ortak kuralları

Menü çubuğu, araç çubuğu ve kayan araç kutusunun ([DESIGN.md](../../DESIGN.md) §7.1–§7.4) iki platformda aynı kuralları ve sözleri kullanması için. Bir adımın sığıp sığmadığı her platformda kendi ölçüsüyle bulunur; burada her adımın ne yaptığı, araç kutusunun yeri, sütunları ve görünmesi, bir komutun menüdeki satırı vardır. Günlüğün yazılışı (alt panelin Komut geçmişi ve Uyarılar sekmeleri, durum çubuğunun iletisi) `v1/log.json`'da, saklanan yerleşim (paneller, boyutlar, araç kutusu, dok ve şerit durumları) `v1/layout.json`'da, şeridin harf ipuçları, hızlı erişim çubuğunun menüsü, sağ tık menüleri ve bölünmüş düğmenin listesi `v1/ribbon.json`'dadır.

- **Web**: `apps/web/src/ui/shell/shellPlan.test.ts` (Vitest), `ui/shell/shellPlan.ts`, `app/menus.ts` (`menuRowLook`) ve `ui/toolbar/fields.ts`'e karşı. `MenuBar.ts`, `Toolbar.ts` ve `Toolbox.ts` bu kuralları uygular.
- **Masaüstü**: yalnız şeritle çalışır, klasik arayüzü yoktur (sahibin kararı, 27 Eylül); `v1/shell.json` yalnız web'indir. Masaüstü `v1/log.json`, `v1/layout.json` ve `v1/ribbon.json`'u oynatır.
- **Kaydedici**: `python3 scripts/fixtures/shell_cases.py` (`--check` hiçbir şey yazmadan karşılaştırır). Tablolar ve sözler DESIGN.md'den ve web'in çubuklarından elle yazılmıştır; cevaplar koddan ayrı, betikte bulunur.
- **Şerit**: web'de `apps/web/src/ui/ribbon/ribbonPlan.test.ts`, `ui/ribbon/ribbonPlan.ts` ve `keytips.ts`'e (`lettersOf`, `assignKeyTips`, `firstLevelTips`, `keyTipStep`) karşı; sekmelerin adları web'in her çalışma modunda kurduğu şeride karşı. `Ribbon.ts`, `controls.ts` ve `panels.ts` bu kuralları uygular. Kaydedici `python3 scripts/fixtures/ribbon_cases.py` (`--check`). Davranışın bütünü [docs/specs/ribbon.md](../../docs/specs/ribbon.md)'dedir.
- **Yerleşim**: web'de `apps/web/src/app/layoutPlan.test.ts`, `app/layoutPlan.ts`, `app/ribbon.ts` (`quickAccessOf`, `splitCurrent`, `startTab`) ve `app/state.ts`'e (`SAVE_DELAY_MS`) karşı. `createUiState`, `AppShell.ts`, `RightDock.ts`, `BottomPanel.ts`, `Ribbon.ts` ve şeridin bölünmüş düğmeleri bu kuralları uygular. Kaydedici `python3 scripts/fixtures/layout_cases.py` (`--check`).
- **Günlük**: web'de `apps/web/src/ui/bottom/logPlan.test.ts`, `ui/bottom/logPlan.ts`, `ui/bottom/warnings.ts` ve `app/state.ts`'in `MessageLog`'una karşı; görünüş `styles/panels.css` ve `styles/shell.css`'e karşı. `BottomPanel.ts` ve `StatusBar.ts` bu kuralları uygular. Kaydedici `python3 scripts/fixtures/log_cases.py` (`--check`); yerel saatler Python'un kendi saat dilimi veritabanıyla bulunur.

| Dosya | İçerik |
|---|---|
| `v1/shell.json` | Klasik arayüzün kuralları ve sözleri |
| `v1/ribbon.json` | Şerit: harf ipuçları, hızlı erişim çubuğunun menüsü ve değişmesi, sağ tık menüleri, bölünmüş düğmenin listesi ve yüzü |
| `v1/layout.json` | Saklanan yerleşim: alanlar, varsayılanlar, saklananın okunuşu, göç, pencereye göre boyutlar, şeridin sekmesi, hızlı erişimi ve bölünmüş düğmeleri |
| `v1/log.json` | Günlüğün yazılışı: satırın saati ve düzeyi, sekmeler, Uyarılar rozeti, saklanan satır sayısı, yazılanın yankısı, durum çubuğunun iletisi, satırların görünüşü |

## Biçim (`kentos.shell`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.shell"`, `1` |
| `menubar.folds` | Menüler sığmazken sırayla denenen adımlar: 0 bütün çubuk; 1 “KentOS” yazısı gizlenir (`brandWord`); 2 koordinat sisteminin adı da gizlenir (`crsName`; düğmesi ve ipucu kalır); 3 menülerin yan boşluğu 9'dan 6 px'e iner (`menuPadding`). Proje adı hepsinden önce kendiliğinden kısalır. İlk sığan adım kalır |
| `menubar.texts` | Çubuğun erişilebilir adı, kaydedilmemiş noktasının ipucu, koordinat sistemi düğmesinin ipucu (`crsTip`: SRID'den) |
| `toolbar.groups` | Araç çubuğunun grupları sırasıyla: komutlar (`commands`), geçerli özellik listeleri (`fields`: katman, renk, tip, kalınlık) ya da ölçek (`scale`; önünde esnek boşluk). Görünüm grubunun sonunda ⋯ düğmesi durur (katlanınca görünür) |
| `toolbar.viewMore`, `toolbar.widths` | ⋯'e giren komutlar; listelerin genişliği (standart yazıda CSS px, yazı ölçeğiyle çarpılır): tam, dar. Ölçek listesi daralmaz |
| `toolbar.folds` | Araç çubuğunun adımları (0–4): katman listesi dar mı (`layerNarrow`), renk/tip/kalınlık dar mı (`propsNarrow`), üç Görünüm komutu ⋯'de mi (`viewMore`), üç liste tek “Özellikler” listesine katlandı mı (`propsFolded`) |
| `toolbar.texts` | Çubuğun adı, ⋯'nin adı ve ipucu |
| `fields` | Listelerin değerleri: “Katmana göre”, adlarıyla sekiz çizim rengi, çizgi tipleri, kalınlıklar (`0.25 mm` yazılışıyla), ölçekler (`1:1000`) |
| `toolbox.groups`, `toolbox.constants` | Araç kutusunun grupları sırasıyla adlarıyla; kenar payı (8 px), yapışma uzaklığı (14 px), en çok sütun (6), kenardan sürüklenince imlece göre yeri (`undock`: imleç eksi bu) |
| `toolbox.texts` | Araç kutusunun sözleri; grubun katlama ipucu `{ sample, text }` |
| `toolbox.columns` | Saklanan sütun sayısının okunuşu: 3 üçtür, gerisi iki |
| `toolbox.fit` | Sütunlar: seçilen sayıdan başlayıp araçlar yüksekliğe sığana dek birer artar, en çok 6; `fitsFrom`: araçlar bu sütun sayısından itibaren sığar (`null`: hiç sığmaz) |
| `toolbox.place` | Kayan araç kutusunun yeri: istenen `x`, `y`, kutunun `w`, `h`, çizim alanının `hostW`, `hostH` → kenar payı içinde kalan, kenara payı 14 px'ten az kalınca paya yapışan yer (`<` 14; tam 14'te kalır) |
| `toolbox.undock` | Kenara sabitliyken tutamaçtan sürüklenince araç kutusunun yeri: imlecin çizim alanındaki yeri eksi `undock` |
| `toolbox.shown` | Araç kutusu görünür mü: klasik arayüzde `toolboxVisible`, şeritte `ribbonToolbox` |
| `menuRows` | Bir komutun menü satırı: aç/kapa komutunda simge yerine onay; tema (değiştirme dışında), çizim motoru, sembol kipi ve çalışma modu radyodur ve simgesini tutar; araç eylem gibi okunur (simge, onay yok). `checked: null` komutun aç/kapa durumu olmadığı demektir |

## Biçim (`kentos.ribbon`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.ribbon"`, `1` |
| `texts` | Şeridin sözleri: hızlı erişim, özelleştirme ve ipucu, “sabit”, ekle, kaldır, sabit, daralt ve sabitle, bölünmüş düğmenin oku, yöntem adı, panelin ▾'i, başlatılamayan yöntem. Bir değerden yapılan söz `{ sample, text }` |
| `quickAccessFixed`, `quickAccessOffers` | Çubuğun sabit üç komutu; ▾ menüsünün önerdikleri, sırasıyla |
| `tabs` | Web'in her hazır çalışma modunda (`hybrid`, `cad`, `gis`) sekmeleri sırasıyla: kimlik, ad, bağlamsal mı (Seçim) |
| `keyTips.letters` | Bir addan harfler: Türkçe küçük harf, ç ğ ı i ö ş ü â î û düz büyük harfe, A–Z ve 0–9 dışı düşer |
| `keyTips.assign` | Adlar ve ayrılmış ipuçları → ipuçları: ilk harfi yalnız kendinin olan ad o harfi, kalanlar iki harf (baş harfler, ilk harf ve öbür harfleri, ilk harf ve alfabe); hiçbir ipucu tek harflik bir ipucuyla başlamaz; harfi olmayan ad `X` ile |
| `keyTips.firstLevel` | Birinci düzey: çubuğun kullanılabilen düğme sayısı ve görünen sekmelerin adları → ilk dokuz düğmeye 1…9, sekmelere harfler (rakamlar ayrılmış) |
| `keyTips.steps` | Harf ipuçları açıkken bir tuş: düzey (`tabs`, `controls`), yazılan, ipuçları, tuş (`ctrl`: Ctrl ile) → `run` (tamamlanan ipucu), `typed` (yazılanın yeni hâli; öbürleri soluk), `back` (Esc, ikinci düzeyden birinciye), `hide`, `ignore` |
| `quickAccessMenus` | Çubuğun ▾ menüsü: çubuk (sabit üç dahil) ve uygulamadaki komutlar → satırlar: başlık, her komut (`quick`: işaretli mi, sabitse kapalı ve “sabit”, değilse seçilince ne olur: `set`), ayırıcı, daraltma komutunun kendi satırı (`command`) |
| `toggles` | Saklı liste, komut, eklenecek mi → yeni liste (eklenen sona, çıkarılan silinir) |
| `commandMenus`, `ribbonMenu` | Sağ tık: bir komut düğmesinde (sabit, eklenmiş ya da çubukta olmayan) ve şeridin başka bir yerinde |
| `splitMenus`, `splitFaces` | Bölünmüş düğmenin listesi (yöntemlerde aracın adıyla başlık, ailede başlıksız; satırlar komut, seçenek, ad, ipucu) ve üstünün yazısı ve erişilebilir adı |

## Biçim (`kentos.layout`, sürüm 1)

Yerleşim kullanıcıya ve cihaza özgüdür (CLAUDE.md §4.4): proje verisi değildir, geri alınmaz. Web onu localStorage `kentos.ui.v1`'de tek JSON nesnesi olarak tutar; masaüstü kendi dosyasında aynı kurallarla tutar (masaüstünün teması ayarlardadır, `appearance.theme`).

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.layout"`, `1` |
| `key`, `saveMs` | Web'in saklama anahtarı; son değişiklikten bu kadar ms sonra bütün alanlar birlikte yazılır (sürüklemede adım başına değil, bir kez). Yazılamazsa (gizli pencere) sessizce geçilir |
| `defaults` | Alanlar ve varsayılanları. `theme`; `rightVisible` sağ dok; `dockWidth` dokun genişliği; `layersFraction` dokta katman ağacının yükseklik payı; `bottomExpanded`, `bottomHeight`, `bottomTab` alt panel; `toolboxVisible`, `toolboxDocked`, `toolboxX`, `toolboxY`, `toolboxColumns`, `toolboxFolded` klasik arayüzün araç kutusu; `dockTab` dokun üst yuvası (Katmanlar, İşlemler), `processingTab`, `processingFolded` İşlemler sekmesi; `ribbonTab`, `ribbonCollapsed`, `ribbonQuickAccess`, `ribbonSplits`, `ribbonToolbox` şerit |
| `fields` | Saklananın alınması için ne olması gerektiği: `enum` (listedeki metin), `boolean`, `number` (sonlu sayı; sınırları varsa içine çekilir, yuvarlanmaz), `columns` (sayı: 3 üç sütun, başka her sayı iki), `text`, `texts` (liste: metinleri kalır, gerisi atılır), `textMap` (nesne: metin değerli girdileri kalır). Alınamayan alan varsayılanıdır |
| `limits` | Dok genişliği 240–560 px ve pencere genişliğinin yarısı, çift tıkla 312; katman payı 0,15–0,85, çift tıkla 0,5; alt panel en az 96 px, en çok pencere yüksekliğinin 0,6'sı, çift tıkla 190 |
| `reads` | Saklanan metin (`null`: hiçbir şey) → okunan yerleşim. JSON nesnesi değilse (bozuk, liste, sayı, `null`) hepsi varsayılan. Bilinmeyen alanlar atılır. Göç: `toolboxFolded` alanı olmayan nesne araç kutusunun başlıklı gruplarından önceki yerleşimdir, sütun sayısı 3 olur. `1e999` gibi sonsuz sayı alınmaz |
| `dockWidths`, `bottomHeights` | Saklanan boyut kullanıcının isteğidir; gösterilen, pencerenin şimdi izin verdiğidir (JavaScript'in yuvarlamasıyla, yarım yukarı). Pencere daralınca saklanan değişmez, genişleyince geri gelir. Çok dar pencerede üst sınır kazanır. Sürükleme gösterilen boyuttan başlar; sürüklenen boyut da bu kuralla gösterilir ve saklanır |
| `layersDrags` | Katman payı, kenarı `dy` px aşağı sürüklenince (`height` dokun yüksekliği): başlangıç + dy / yükseklik, 0,15–0,85 içinde, yuvarlanmadan |
| `ribbon.quickAccessFixed`, `ribbon.quickAccess` | Hızlı erişim çubuğu: sabit üç komut, ardından kullanıcının eklediklerinden uygulamada olanlar, birer kez, eklendiği sırayla |
| `ribbon.splits` | Bölünmüş düğmenin üstteki girdisi: saklanan seçim (`komut|seçenek`, seçeneksiz girdide `komut|`) hangi girdiyse o; yoksa ya da kalmadıysa ilk girdi (`current`: sırası) |
| `ribbon.startTabs` | Açılışta şeridin sekmesi: saklanan sekme çalışma modunda varsa ve bağlamsal değilse o; yoksa Giriş (`home`) |

Saklananı değiştirenler: `view.rightPanel`, `view.bottomPanel` (F2), `view.toolbox` (klasik arayüzde `toolboxVisible`, şeritte `ribbonToolbox`), `view.toolboxDock`, `view.ribbonCollapse` (Ctrl+F1), `view.theme.dark`, `view.theme.light`, `view.theme.toggle`; dokun, katman ağacının ve alt panelin kenarları; araç kutusunun tutamacı, sütun düğmesi ve grup başlıkları; dokun ve İşlemler'in sekmeleri ve kategorileri; şeridin sekmeleri, hızlı erişim menüsü (ya da şeritte sağ tık) ve bölünmüş düğmenin oku. Saklanmayanlar oturumundur: dokta panellerin katlanması, daraltılmış şeridin açılan sekmesi, listelerin kaydırılması, komut satırının yazılan komutları, çizim yardımcıları (tipli ayarların oturum kapsamı).

## Biçim (`kentos.log`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version`, `timeZone` | `"kentos.log"`, `1`; saatlerin okunduğu saat dilimi (`Europe/Istanbul`): denetleyici kendi yerel saatini buna ayarlar |
| `tabs`, `texts` | Alt panelin sekmeleri sırasıyla (ad, simge), sekme satırının adı, Geçmişi temizle, Paneli kapat, komut satırındaki Komut geçmişini aç düğmesi, panelin kenarı, boş listelerin sözleri |
| `limit` | Günlük en çok bu kadar satır tutar (500); fazlası en eskiden atılır |
| `followWithin` | Yeni satır listeyi sonuna yalnız liste sonuna bu kadar yakınken (24 CSS px) kaydırır; okunan yer yerinde kalır. Liste yeniden kurulunca sonundadır |
| `levels` | Beş düzey sırasıyla (`command`, `info`, `success`, `warn`, `error`): listedeki simgesi (komut ve düz satırda yok) ve hangi sekmelerde listelendiği (Komut geçmişi hepsini, Uyarılar uyarıları ve hataları) |
| `look` | Satırların görünüşü (`panels.css`): eş aralıklı yazı, küçük boy (`xs`), 1,5 satır yüksekliği; sütunlar saat 64 px, simge 18 px, gerisi metin; satır dolgusu 1 × 12 px; saat `text-3`; metin `text-2`, boşlukları korunur ve kaydırılır (`pre-wrap`); komut satırı `text` ve 500 kalınlıkta; başarının simgesi `ok`; uyarının simgesi ve metni `warn`, hatanınki `danger`; simge 14 px, 2 px aşağıda. `flash`: durum çubuğunun iletisi (`shell.css`): 6 px aralık, 0 × 12 px dolgu, 160 ms'de belirir ve kaybolur, simgenin rengi düzeyinden (bilgi kendi rengiyle). Tonlar stil sayfalarının renk jetonlarıdır (`--c-` olmadan) |
| `times` | Satırın saati: yazıldığı andaki yerel duvar saati, saat:dakika:saniye (`09:05:07`), 24 saatlik, iki haneli; saniye saatin gösterdiğidir, yukarı yuvarlanmaz (`at`: Unix ms) |
| `echo` | Kullanıcının yazdığı ya da seçtiği (değer, seçeneğin harfi), işlenmeden önce komut düzeyinde `› ` ile yazılır |
| `flash` | Durum çubuğunun iletisi: komut satırı ve bir öncekini sürdüren girintili satır (iki boşlukla başlar: tıklanan nokta, mesafe) gösterilmez; uyarı ve hata 9 s, gerisi 5 s kalır; simgesi düzeyinden (`info` bilgi) |
| `kept` | `pushed` satır yazıldıktan sonra kalan satır sayısı, ilkinin ve sonuncunun kimliği (kimlikler 1'den başlar) |
| `badge` | Uyarılar rozeti adım adım: bir düzeyde satır yazmak (`{ push, times }` birden çok), `look` (Uyarılar sekmesi ekranda: o ana dek yazılan her satır görüldü; boş günlükte bir şey değişmez), `clear` (Geçmişi temizle; kimlikler sürer) → rozetteki sayı (görülmemiş uyarı ve hata) ve günlükteki satır sayısı |

## Kurallar

- Her şey tam değerle karşılaştırılır.
- Web'in bugünkü kuralları ve sözleri yazılıdır; biri değişince bu dosya, iki çalıştırıcı ve bu belge birlikte değişir.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
