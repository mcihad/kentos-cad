# ADR 0804: Bileşenlerin tek dili: tek yükseklik, hücre ve çerçeve

- **Durum:** kabul edildi (2026-10-30), dal `launcher-zarif`.
- **İstek:** sahibin sözleri “ui bileşenlerimizde tutarsızlıklar var”, “object inspector bileşenimizi beğenmiyorum”, “edit modunda küçücük
  halbuki mantık olarak arkaplanındaki ızgara hücresini şık bir şekilde doldurması gerekli”, “girdi bileşenlerden tamamını yüksekliği ayrı
  olmamalı aynı olmalı, combo ile input aynı olmalı”, “kapatma düğmesi isteğe bağlı gösterilebilmeli veya gizlenebilir”, “yeniden
  boyutlandırma indikatörü çok sıradan ve bayağı olmuş”, “tüm bileşenleri kısacası gözden geçir”.
- **Bağlam belgesi:** DESIGN.md §3.3, §4, §5.2, §5.3, §7.5, §7.9, §7.13, §11, §13; ADR 0746, 0747, 0748, 0749.
- **Web:** bir şey değişmez (ADR 0228). **`.kcad`, komutlar, Python, MCP:** değişmez.
- **Bağımlılık:** yeni dış kitaplık yok.

## Bağlam

KentOS UI'ın parçaları tek tek işini görüyordu, yan yana gelince dil tutmuyordu. Ölçüldü:

| Bileşen | 13 px'te yükseklik |
|---|---|
| `InputField`, `Select`, `NumberInput`, `DatePicker`, `ColorPicker`, `Segmented`, `ChipInput` | 28 |
| Aynıları `.inline()` ile | 22 |
| `SearchBox`, kendi hâlinde | 26 |
| `SearchBox`, `.height(28.0)` ile (ölçek iki kez) | 30 |
| `Suggest` | 23 |
| Uygulamalardaki 218 ham `text_input` | 23, 25, 27, 29 (`[3, 6]`, `[4, 8]`, `[5, 8]`, `[6, 12]`) |
| Birincil ve ikincil düğmeler (292 çağrı) | 21–29 |
| Tablo, ağaç ve liste satırı | 27, 26, 28 |
| Öznitelikler ve Nesne inceleyici satırı | 30 ve 28 |

Üç ayrı özellik ızgarası vardı: `PropertyGrid` (çizgili, bantlı), `PropertySheet` (sütun çizgisiz, solgun başlıklı), `Inspector` (gerçek ızgara, ikonlu).
Düzenleyiciler hücrenin içinde 22–24 piksellik küçük bir kutuydu: sahibin “küçücük” dediği. Yazı düzenlemeye geçince kayıyordu. Durum dili de
farklıydı: bir girdide üzerine gelme `border_strong`, öbüründe `muted`; odak kimi yerde `accent_line`, kimi yerde `accent` ve hale. Seçili satır
bir yerde dolgu, bir yerde dolgu ve kenar, bir yerde dolgu ve çubuktu. Küçük yazının tonu (`faint`, koyu temada 3,4:1) AA altındaydı. Adlar eş
aralıklıydı (“Parseller”, “sürüyor”, “3 açık”), oysa DESIGN.md §13 bunu yasaklar.

Başvurulan yol (30 Ekim 2026'da okundu):

- Microsoft, *Compact sizing for controls*: yoğunluk iki düzeydir, varsayılan ve sıkışık; `Button`, `CheckBox`, `ComboBox`, `TextBox` aynı sözlükle birlikte
  küçülür, keyfi değer yoktur. <https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/compact-sizing>
- Microsoft, *Content layout and spacing*: “The use of consistently sized spacing and gutters semantically groups an experience into separate
  components”, düğmeler arası 8 epx, içerik alanları arası 12 epx. <https://learn.microsoft.com/en-us/windows/apps/design/basics/content-basics>
- Apple, *Multitask on iPad*: “drag any corner or the handle in the bottom right corner of the window to resize it”: pencere
  köşesinin kendisi tutamaçtır. <https://support.apple.com/en-us/125309>

KentOS'un çıkardığı: yoğunluğu bileşen başına seçtirmek tutarsızlığın kaynağıdır; **bir yoğunluk, bir yükseklik.** Köşe tutamacı pencerenin dışından
yapıştırılan bir işaret olmak zorunda değildir.

## Karar

### 1. Tek yükseklik: 28'lik kareli kâğıt

`metrics::CONTROL` (28, yazı boyutuyla büyür) bütün girdilerin ve eylem düğmelerinin yüksekliğidir; `metrics::ROW` ona eşittir: tablo, ağaç, liste,
ızgara ve menü satırı. Bir paneldeki her şey aynı kareli kâğıda oturur.

- `.inline()` girdilerden kalktı (`InputField`, `Select`, `NumberInput`, `DatePicker`, `TimePicker`, `ColorPicker`); `Select::borderless` ve
  `Dropdown::edge_cell` kalktı. `Segmented::compact` artık yalnız yanları daraltır, yüksekliği aynıdır. `SearchBox::height` kalktı.
- `metrics::INLINE` (22) yalnız satırın içindeki küçük düğmelerindir; girdi bu boyda olmaz.
- Uygulamalar iced'in `text_input`'unu kendi iç boşluğuyla kurmaz: `kentos_ui::widget::text_field` ve kendi çerçevesi olan satır için `bare_field`. Yükseklik
  tek yerde belirlenir (`metrics::field`).
- Düğmeler `metrics::action()` (28) ya da satır içinde `metrics::action_small()` (22). `metrics::BUTTON` (32) kalktı; pencerelerin alt çubuğu da 28'dir.
- Kilit: `crates/ui/tests/control_heights.rs` her girdiyi 11, 13, 15 ve 18 px'te ölçer ve uygulamalarda ham `text_input` olmadığını denetler.

### 2. Hücre

`widget::Cell` ızgaranın değer hücresidir (`crates/ui/src/widget/cell.rs`). İçindeki düzenleyici çerçevesiz ve zeminsizdir, hücreyi çizgiden çizgiye
doldurur; durumları tek yerde çizilir:

| Durum | Görünüş |
|---|---|
| Salt okunur (`Cell::flat`) | Zemin yok, panelin üstünde yazı |
| Yazılabilir, dinleniyor | Gömük zemin (`Tokens::cell`) |
| İmleç üstünde | Biraz aydınlık (`Tokens::cell_hover`) |
| Düzenleniyor ya da listesi açık | Alan zemini (`field`), köşesiz 1 px vurgu kenarı |
| Geçersiz | Kırmızı kenar |

Yapı bilgi taşır: zemini olan değer değişir, olmayan değişmez; bu renkten değil yüzeyden okunur (DESIGN.md §11). Yazı dört durumda da aynı pikselde
kalır; bir test ölçer. Hücre köşesizdir: ızgaranın parçasıdır, ızgarada duran bir kutu değil (DESIGN.md §5.3, dok panelleri köşesiz).

Düzenleyicilerin hücre biçimi `.cell()`'dir: `InputField`, `Select`, `NumberInput`, `DatePicker`, `TimePicker`, `ColorPicker`, `Segmented` (parçalar hücreyi ince
çizgilerle paylaşır). `EditCell` ve `property_grid::choice` kendileri hücredir, `value` düz hücredir; yan yana iki sayı `cell::divider` ile ayrılır.

Düzenleme bildirilmez, sorulur: hücre basışta, klavyede ve yeni karede içindeki odaklı girişe ve açık menüye bakar (`cell::Opened`). Fare hareketinde
sormaz.

### 3. Tek satır, tek başlık

`PropertyGrid`, `PropertySheet` ve `Inspector` aynı satırı (`sheet_row::SheetRow`) ve aynı bölüm başlığını (`property_grid::head_face`) kullanır: 28 piksellik
satır, sütun çizgisi, alt çizgi (`Tokens::rule`), bantlı başlık, 12 piksellik ok, yarı kalın ad. Satır sütunlarını tam piksele kendisi yerleştirir: çizgiler
ve kenarlar hiçbir genişlikte bulanmaz. Düzenlenen satırın 2 piksellik sol çubuğu (ADR 0747) kalktı: vurgu kenarı işaretin kendisidir; satır üzerine
gelince aydınlanır. Inspector'da değişen alanın solundaki çubuk başka bir şey söylediği için kalır.

### 4. Pencere

- **Kapatma düğmesi isteğe bağlıdır.** `Dialog::on_close(message)` başlıkta × gösterir; `Dialog::modal(on_dismiss)` kutuyu ortalar, arkasını karartır ve ×'i
  aynı mesaja bağlar; `Dialog::no_close()` gizler. Verilmezse düğme yoktur. Uygulamalardaki pencereler × ile gelir; kendi cevapları olan `Confirm` gelmez.
- **Köşe tutamacı çerçevenin kendisidir.** Ayrı bir kulakçık (önceki iki kısa çizgi, yüzen pencerede nokta üçgeni) yok: sağ alt köşenin kavisi biraz kalın ve
  belirgindir; imleç bir kenara gelince o kenarın çerçevesi vurgu renginde `]` gibi aydınlanır, köşeye gelince kavis kısa kuyruklarıyla. Köşe yarıçapını
  izler, “Keskin” köşe ayarında da doğrudur (`sizable::frame_mark`). Yüzen araç pencereleri aynısını kullanır.
- Başlığın kısa notu (`hint`) tuş kapağındadır, başlık düğmeleri (tam ekran, kapat) aynı karededir.

### 5. Durum ve seçim dili

- Üzerine gelme `border_strong`, odak `accent_line`: bütün girdilerde. Metin kutularında odak halesi yoktur (DESIGN.md §7.13); `focus_ring` kalktı.
- Seçili satır (tablo, ağaç, liste) yumuşak vurgu zeminidir (DESIGN.md §3.3); kenar yalnız birkaçı seçiliyken birincil satırı söyler (`Row::current`). Açık
  araç düğmesi (`tool`) ile açık anahtar (`toggle`) aynı dille: yumuşak zemin, vurgu yazısı. Sol 2 piksellik çubuk “etkin” demektir (etkin katman).
- Etiket döşemeleri (malzeme, nesne) kenarlı kalır: orada satır çizgisi yoktur.

### 6. Yazı

- `Tokens::soft` okunur en sessiz yazı tonudur (AA); `style::text::faint` onu verir, `t.faint` yalnız çizgi ve işaretlerde kalır (web'in jetonu bir testle bağlı, değişmez).
- Eş aralıklı yazı yalnız sütunda hizalanacak sayılar içindir. Plus Jakarta Sans'ın rakamları orantılıdır ve iced 0.14 `tnum` özelliğini açamaz; bu yüzden
  sayı sütunlarında mono kalır (DESIGN.md §4 ve §13'ün kaydı). Adlar, kısayollar, sayaçlar (“3 açık”) arayüz yazısıyladır. Öne çıkan tek sayı (`label::figure`) arayüz yazısıyladır.
- “A · B · C” kalıpları virgüllü cümleye döndü (Python konsolu, tamamlama, AI özeti).

## Sonuçlar

- Bir panelde yan yana duran her şey aynı boydadır; satır, girdi ve düğme aynı modüldedir.
- Satırlar biraz daha sıkıdır (30 → 29 çizgiyle), düzenleyiciler büyür: aynı yükseklikte daha çok ve daha okunur.
- Ham `text_input` ve `focus_ring` yok; yeni pencere yanlış boyda yazılamaz. `control_heights` testi bunu tutar.
- Web'in `tokens.css`'i değişmedi; yeni jetonlar (`cell`, `cell_hover`, `rule`, `soft`, `border_grip`) web dönünce TODOS.md §0.2'den taşınır.

## Sınırlar

- Şeridin kendi kontrolleri (`ribbon::ROW`, üç satırlık ızgara) ve durum çubuğu bu kuralın dışındadır: yerleri sabit satırlardır.
- Sohbet girişi (AI) ve çizimin üstündeki yazı kutusu (boyu yakınlaştırmaya bağlı) bilerek farklıdır.
- Tablo ve sekmeler sonraki iki kararda: [ADR 0805](0805-data-tables-head-rows-cells.md), [ADR 0806](0806-tabs-keep-their-width.md).
- Elle gerçek pencerede denenmedi; doğrulama gerçek olaylı testler ve ekran resimleriyledir.
