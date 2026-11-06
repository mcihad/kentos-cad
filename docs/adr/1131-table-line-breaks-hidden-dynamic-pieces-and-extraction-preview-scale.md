# 1131 — Tablo hücresinde satır sonu, dinamik bloğun gizlediği parçaların sayımı ve büyük çizimde önizleme

Tarih: 2026-11-06. Durum: kabul (CAD-ADV-13'ün üç açığı). Kapsam: plan/cad-advanced.md `CAD-ADV-13`; ADR 1030 §9'un
“masaüstü tablo düzenleyicide satır sonu yazma, görünürlük dışı dinamik eylemle gizlenen parça ve büyük çizimde önizleme
ölçümü” sınırı. Web: ertelendi (ADR 0228); TODOS.md §0.2. `.kcad` şeması değişmedi (satır sonlu hücre şema 182'de
vardı).

## Sorun

1. **Satır sonu:** komut, Python ve MCP hücreye `\n` yazabiliyordu (ADR 1030 §7), masaüstünün Tabloyu düzenle
   penceresinin çubuğu ise tek satırlık bir yazı alanıydı; Enter hücreyi yazıyordu, satır sonu girilemiyordu.
2. **Dinamik bloğun gizlediği parça:** görünürlük durumu parçayı yerel kimliğiyle gizliyordu; dizi eyleminin
   kopyaları yeni yerel kimlik alır, bu yüzden **gizli bir parçanın dizi kopyaları görünür kalıyor**, çıkarım onları
   sayıyor ve listeliyordu (`drawn[source]` kopyadan işaretlendiği için gizli asıl parça da gizli sayılmıyordu). Gizli
   sayısı tek bir sayıydı; neden (görünürlük durumu mu, gizli katman mı) söylenmiyordu. Aynı kural çiziciyi de
   ilgilendirir (`block.rs` `vary` ile çizer).
3. **Büyük çizim:** önizleme arayüz iş parçacığında, ölçülmeden çalışıyordu (ADR 1030 §9); ilerleme ve durdurma yoktu.

## Araştırma

- **AutoCAD tablo hücresi:** hücre düzenlerken Enter sonraki hücreye geçer, satır sonu **Alt+Enter**'dır
  ([Autodesk topluluğu, “New Line In Table Cell (Alt+Enter)”](https://forums.autodesk.com/t5/visual-lisp-autolisp-and-general/new-line-in-table-cell-alt-enter/m-p/12328286),
  [CADforum ipucu 7091](https://www.cadforum.cz/de/zeilenumbruche-im-zellen-der-autocad-tabellen-tip7091)); programla
  `\P`. Excel'de de Alt+Enter. KentOS: ADR 1030 §9'un yazdığı Shift+Enter **ve** AutoCAD alışkanlığı Alt+Enter.
- **Dinamik blok görünürlüğü ve veri çıkarımı:** görünürlük durumunun gizlediği nesne o durumda yoktur; topluluk
  raporlarına göre AutoCAD'in veri çıkarımı gizli durumdaki özniteliği de almaz
  ([“Attribute Extraction in Visibility States”](https://forums.autodesk.com/t5/dynamic-blocks-forum/attribute-extraction-in-visibility-states/m-p/6704352/highlight/true),
  [“Dynamic block attributes in visibility states”](https://forums.autodesk.com/t5/dynamic-blocks-forum/dynamic-block-attributes-in-visibility-states/td-p/7107794)).
  Dizi eylemi seçim kümesindeki nesneleri çoğaltır; gizli nesnenin kopyası da o nesnedir. Resmî Autodesk belgesinde
  dizi kopyalarının görünürlüğü ayrıca yazılı bulunamadı; KentOS kararı “kopya, kaynağının görünürlüğünü taşır”dır.
  KentOS'ta öznitelik tanımı parça değildir, görünürlük durumu onu gizleyemez; öznitelik sütunu bu yüzden değişmedi.
- **Uzun iş:** CLAUDE.md §6.2/§18 (uzun hesap arayüz iş parçacığında çalışmaz, iptal/ilerleme), masaüstünün var olan
  deseni (`statistics/source.rs`: okuma kopyası, ayrı iş parçacığı, `AtomicBool` durdurma, oneshot cevap).

## Karar

### 1. Tablo hücresinde satır sonu

Tabloyu düzenle'nin çubuğu iced'in `text_editor`'ıdır (tek satır yüksekliğinde başlar, en çok 4 satır büyür). Tuşlar
(`bar_keys`): **Enter** hücreyi yazar (eski alanın gönderimi gibi), **Shift+Enter** ya da **Alt+Enter** hücrede satır
sonu açar, Tab ve Esc pencerenindir (önceki gibi). Çubuğun sözü ile editör birlikte kurulur (`Editor::set_words`,
imleç sonda); editörün platform satır sonu `\n`'e çevrilir. Izgara hücrenin satırlarını tek satırda `↵` ile ayırarak
gösterir; tablonun kendisi satırları ADR 1030 §7'nin düzeniyle çizer. Kısayol listesine (`shortcuts.rs`
`DRAWING_KEYS`) ve pencerenin durum satırına yazıldı. Kaydet bütün değişiklikleri tek geri alma adımında yazar (ADR 0184).

### 2. Görünürlük durumunun gizlediği parçalar

- `Dynamic::vary_split` eylemleri uygular, sonra parçaları **kaynaklarının** (kopyalandıkları parçanın) yerel kimliğine
  göre çizilen ve görünürlük durumunun gizlediği diye ayırır; `vary_traced` (ve çizicinin `vary`'si) çizilen kısımdır.
  Böylece gizli parçanın dizi kopyaları çizicide de görünmez.
- Çıkarım çizilen parçaları sayar; gizlenenleri `scope.hidden` kapalıyken saymaz, listelemez ve sayar: `notes.hidden`
  (bütün görünmeyenler) ve yeni `notes.hiddenStates` (bunlardan dinamik bloğun görünürlük durumunun gizledikleri,
  kopyalar dahil). `scope.hidden` açıkken gizlenenler **eylemlerin koyduğu yerde** sayılır (önceden tanımdaki yerinde).
  Bir parçanın görünürlükten gizli olduğu bilgisi iç içe bloklara iner (`Frame.state`).
- `extraction_hidden` uyarısının sözü nedeni söyler: “3 parça dinamik bloğun görünürlük durumu gizlediği, 2 nesne gizli
  ya da dondurulmuş katmanda olduğu için sayılmadı ve listelenmedi”.

### 3. Büyük çizimde önizleme

- Çekirdek: `kentos_formats::extract::extract_watched` bir `Watch` alır (`total` çizimlerin üst düzey nesneleri,
  `done` gezilenler, `stop`); `stop` her 256 nesnede bir okunur, durdurulan çıkarım hiçbir şey yarım bırakmadan
  `cancelled` döner. `extract` aynı yolun izlenmeyen biçimidir (sonuç baytı baytına aynı).
- Uygulama: `extract_run::plan_watched` durdurulanı `CommandResult::Cancelled` verir.
- Masaüstü: Veri çıkarımı penceresi **20 000 nesneye kadar** önizlemeyi eskisi gibi hemen yapar (ortak durumlar ve
  küçük çizimler aynı kalır); daha büyük çizimde çizimin okuma kopyası (`reading_copy`) ayrı bir iş parçacığında
  çıkarılır, pencere son satırlarını tutarken “Önizleme okunuyor: n / N nesne (%p)” der ve **Durdur** düğmesini gösterir.
  Yeni bir değişiklik eski önizlemeyi durdurup yenisini başlatır; eski iş parçacığının geç gelen cevabı numarasıyla
  ayıklanır. Biten önizlemenin süresi altta yazar.

### 4. Ölçüm (yayın derlemesi)

`cargo test --release -p kentos-native-application --test all extract_timing -- --ignored --nocapture`; Apple M5,
32 GB, macOS 26 (Darwin 25.6), taban `43555855d` + bu dilimin değişiklikleri, tek iş, başka ağır iş yokken. Çizim: 100 000
üst düzey nesne — 60 000 çizgi, 30 000 Sandalye yerleştirmesi, 10 000 dinamik Sıra (Boy 2 m: dizi eylemiyle 3 Sandalye ve
bir çerçeve çizgisi; her onuncusu Boş durumda). Tarif: blok ve çizgi, iç içe, Blok · Tür · Adet · Uzunluk, birleştir,
toplam; pencerenin önizlemesiyle aynı `plan_watched`.

| Ölçü | Değer |
|---|---|
| Süre (5 kez) | en iyi **121,8 ms**, ortanca **122,4 ms** |
| Sonuç | 3 satır, **137 000** oluşum (60 000 + 30 000 + 27 000 + 10 000 + 10 000), 3 000 gizli (hepsi görünürlük durumundan) |
| Yerleşik bellek (`ps`, 10 ms örnek) | önce 528 MB (çizimin kendisi dahil), en yüksek **609 MB** (+81 MB), sonra 609 MB (ayırıcı geri vermedi) |
| İlerleme | her çalıştırmada `done = total = 100 000` |
| Durdurma | 20 ms'de istendi, **5,5 ms** sonra `Cancelled`; o ana kadar 74 497 / 100 000 nesne gezilmişti |

Okuma: yayın derlemesinde 100 000 nesnelik önizleme bir karenin birkaç katıdır; arayüzü kilitlememesi için büyük çizimde
ayrı iş parçacığına alındı. 20 000 nesnelik eşik bu ölçünün doğrusal payıyla yaklaşık 25 ms'dir (ölçülmedi, tahmin);
masaüstünün debug derlemesinde daha yavaştır. Ölçülmeyenler: masaüstünde `reading_copy`'nin 100 000 nesnelik süresi ve
belleği, başka çizim okuyan (dosyadan) tarifler, dış tabloyla eşleme, Windows/Linux. Bellek örneklemesi `ps` ile 10 ms'de
birdir; kısa tepe noktaları kaçabilir.

## Uygulama

- Satır sonu: `apps/desktop/src/tables/editor.rs` (`bar`, `set_words`, `Event::Bar`, `bar_keys`, ızgaranın `↵`'i),
  `tables/editor_tests.rs`, `shortcuts.rs`.
- Gizlenen parçalar: `crates/shared/geometry-core/src/block_dynamic.rs` (`vary_split`), `crates/shared/formats/src/extract.rs`
  (`Frame.state`, `Notes::hidden_states`), `crates/shared/commands/src/cad_extract.rs` (`ExtractNotes::hidden_states`),
  `crates/native/application/src/extract.rs` (uyarının sözü).
- Önizleme: `crates/shared/formats/src/extract.rs` (`Watch`, `extract_watched`), `crates/native/application/src/extract.rs`
  (`work_out_watched`, `plan_watched`), `apps/desktop/src/tables/extract.rs` (`PREVIEW_INLINE`, `running`, `ticks`,
  `Event::{Previewed, Stop, Tick}`), `app.rs` (abonelik), `crates/native/application/tests/all/extract_timing.rs`.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/extract_command_cases.py` (KentOS kodu yok) dizi eylemini (ADR 0319 §4'ün kuralı:
  `n = ⌊şimdi/aralık + 1e-9⌋ − ⌊varsayılan/aralık + 1e-9⌋ + 1`, kopyalar en büyük yerel kimlikten sonra) ve kaynağın
  görünürlüğünü yeniden yazdı; yeni `Kat 3` çizimi: Boy 2 m'lik dinamik **Sıra** (çerçeve + 1 Sandalye, dizi eylemi 0,5 m)
  iki kez, biri Boş durumda, biri gizli katmanda varsayılan boyda. Yeni iki durum: gizlenenler sayılmaz (Sandalye 3,
  Sıra 2; `hidden` 5, `hiddenStates` 3) ve açıkça seçilince sayılır (Sandalye 7, Sıra 3). Bütün eski durumların notlarına
  `hiddenStates` eklendi (Kapı'nın gizli kanadı 1); başka sonuç değişmedi. `cad.extract.run` 33, `cad.extract.table` 47
  durum, native koşucuda hepsi geçti.
- `extract_timing::a_stopped_extraction_is_cancelled_and_a_whole_one_walks_every_object` (önceden istenen durdurma ilk
  bakışta `Cancelled`; tam çıkarım 1000/1000 nesne; 1370 oluşum, 30 görünürlük gizlisi).
- Masaüstü: `tables::editor::tests::shift_enter_breaks_the_cells_line_and_kaydet_keeps_it` (Shift+Enter ve Alt+Enter
  satır sonu, Enter gönderim, Tab pencerenin; hücre `Zemin\nkat`, düzende iki satır, tek Undo geri alır);
  `extract_window_tests::a_large_drawing_previews_on_its_own_thread_and_durdur_stops_it` (20 001 çizgi: önizleme ayrı
  iş parçacığında, Durdur iş parçacığına durmasını söyler ve pencere bunu yazar, eski cevap ayıklanır, yenisi alınır);
  var olan Veri çıkarımı ve tablo testleri.
