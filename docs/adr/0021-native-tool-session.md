# ADR 0021: Masaüstünün araç oturumu, değer alanı ve etkileşim izlerinin native oynatıcısı

- **Durum:** kabul edildi (2026-09-25). Yön TODOS.md §2.1'den (`native/interaction`), `UX-01`, `UX-04`, `UX-06` ve ADR 0018'dendir. Crate, istemin veri olarak tutulması, Iced'de girdinin yolu, önizlemenin nasıl çizileceği ve oynatıcı bu dilimin kararıdır.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md `UX-01`, `UX-04`, `UX-05`, `UX-06`, `UI-11`, §22 F2 kapısı; ADR 0010 (platform sınırı), 0017 (masaüstü kabuğu), 0018 (araç oturumu ve izler), 0019 (wgpu çizim alanı), 0020 (native belge)

## Bağlam

- ADR 0018 web'in araç davranışını iki platformun ortak hedefi yaptı. Davranışı dört iz tutuyor: `fixtures/interaction/v1` içinde `polygon-accept`, `polygon-close`, `polygon-keys`, `polygon-signs`. Web onları gerçek tarayıcıda üç varyantta oynatıyor: US klavye, Türkçe Q, 2× ekran.
- Masaüstünün çizim alanı (ADR 0019) ve belgesi (ADR 0020) vardı; aracı yoktu. F2 kapısı “tıkla → yaz → Enter → geri al/yinele”yi masaüstünde de ister.
- Kural: izler değiştirilmeden oynatılır. İzde yazılı olanı masaüstü yapamıyorsa iz uyarlanmaz; durup açıklanır.

## Karar

### Araç oturumu: `crates/native/interaction`

- Paket `kentos-interaction`, crate `kentos_interaction`. Saf Rust, yalnız native. Bağımlılıkları `kentos-contracts`, `kentos-geometry-core` ve `kentos-domain`'dir.
  - `scripts/arch/deps.mjs`'te kendi grubu vardır: `interaction`. `shared` ve `domain`'i kullanır. Iced, winit, wgpu, naga, sunucu çalışma zamanları, tarayıcı bağları, PyO3, GDAL ve PROJ altında bulunamaz. `desktop` grubu onu kullanabilir.
  - `default-members`'tadır: `pnpm rust:test` onu da sınar.
- **`Session`** ADR 0018'in durumlarını tutar:

  | Durum | Oturumda |
  |---|---|
  | Boşta | araç yok; izler aracı `select` okur |
  | Çalışıyor | araç nokta, sayı ya da seçenek bekler (`prompt`) |
  | Önizleme | imleç hareketi yalnız `preview`'ı değiştirir, belgeyi değil |
  | Onay | araç tek geri alma adımı yazar ve yeni nesne için açık kalır; hiçbir şeyi yoksa çıkar |
  | İptal | `exit`: taslak bırakılır, belgeye ve geçmişe bir şey yazılmaz |
  | Askıda | yok: nokta hesaplayıcı masaüstünde yok (`UX-07`) |

  “Son komutu yinele” için son başlatılan aracı saklar (`last`). Hangi tuşun ne yapacağı oturumun değil, masaüstünün işidir (aşağıda).
- **`Tool`** arayüzü web'in `Tool`'unun DOM'suz karşılığıdır: `pointer_move`, `pointer_down`, `input`, `confirm` (kal ya da çık), `undo_step`, `preview`. Araç her çağrıda bir `Context` alır:
  - belge (`kentos_domain::Document`): bir şeyi değiştirmenin tek yolu;
  - görünüm: dünya → ekran eşlemesi ve piksel başına dünya uzunluğu;
  - çizim yardımları: ortho, kutupsal izleme adımı, kenet açıklığı (web'in varsayılanı 11 px);
  - iletiler: web'in düzeyleriyle (`command`, `info`, `success`, `warn`, `error`). İzler en yeni iletinin düzeyini okur.
- **Kapalı alan aracı** (`polygon.rs`) web'in `PathTool`'udur (`closed: true`), adım adım:
  - Düz kip; Yay (Y) ve Düz (D); Uzunluk (U); Geri (G). İstem bütün yay seçeneklerini gösterdiği için hepsi çalışır: Açı (A), Merkez (M), Yarıçap (R), İkinci nokta (İ, I), Doğrultu (T).
  - İlk köşeyi yeniden vermek alanı kapatır ve bitirir: tam yazarak, ya da üç köşe varken kenet açıklığı içinde tıklayarak. Yay kipinde kapanış kenarı çizilen yaydır.
  - Üç köşeden azla onay uyarır, yazmaz, baştan başlatır. Aynı yere ikinci tıklama nokta eklemez.
  - Ctrl+Z önce taslağın son noktasını geri alır, taslak boşken belgeyi.
  - Onay tek belge işlemidir: `Document::add`, geri alma adı “Ekle”.
  - Kilitli katmana yazmaz, nasıl düzeltileceğini söyler; gizli katmana yazarken uyarır (ADR 0020'nin açık maddesi).
  - Web'in iletileri kelimesi kelimesine korunur. Hesabın tamamı `kentos-geometry-core`'undur: bulge, teğet, yarıçap, merkez, ortho ve kutupsal imleç, alan. Yeni geometri algoritması yazılmadı.
- **`Format`** web'in `Formatter`'ıdır: uzunluk, alan birimi, semt, `Y … X …`. Sayılar web'in `toFixed`'i gibi yuvarlanır. Tam yarım sıfırdan uzağa gider; Rust'ın biçimlemesi çifte yuvarlardı: `487012.0625` web'de `…063`, düz Rust'ta `…062` olurdu. Yalnız gösterimdir (CLAUDE.md §23.2).

### İstem veri olarak

- `Prompt { tool, step, options: [{ label, key }] }`.
  - `text()` web'in metnini tam olarak yazar: `Kapalı alan: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G) / Bitir (Enter)]`.
  - `keys()` izlerin karşılaştırdığı seçenek tuşlarıdır.
  - `option_for_key` web'in `optionForKey`'idir: önce tuşun kendisi, Türkçe büyük harfle (i → İ, ı → I); sonra işaretsiz hâli (`Ç` → `C`).
- **Neden veri:** masaüstü seçenek düğmelerini metni ayrıştırmadan kurar; komut satırı ve durum çubuğu aynı veriyi okur. Web kendi metnini ayrıştırmaya devam eder. İki tarafın tipli `PromptSpec`'e geçişi `UX-02`'dir. Her durumun metni bir testte web'in metniyle karşılaştırılır.

### Yazılan değerin dilbilgisi

- Dilbilgisi ortak Rust'tadır: `crates/shared/geometry-core/src/tools/point_text.rs`. Çözdüğü noktayı hesaplayan işlevlerin yanındadır (`relative_point`, `polar_offset`, `toward_point`). Okuduğu biçimler: mutlak `Y,X`, göreli `@dY,dX`, kutupsal `@mesafe<açı` ve yalın mesafe, web'in sırasıyla. `parse_number` ve `looks_like_coordinate` de vardır.
- Web kendi düzenli ifadelerini korur (`coordinateInput.ts`). İki okuyucuyu ortak dosya bağlar: `fixtures/point-input/v1/cases.json`, 68 durum.
  - Web onu `coordinateInput.test.ts`'te, Rust `geometry-core/tests/point_text.rs`'te okur.
  - Durumlar çevirinin kolay yanlış yapacağı yerleri tutar: ECMAScript'in boşlukları (U+FEFF kırpılır, U+0085 kırpılmaz), yalnız ASCII rakam, `,` `;` ya da boşluktan tek ayırıcı, üs ve yalın nokta yok, en yakın double'a yuvarlama, son nokta yokken göreli ve kutupsal yazımın başka biçim denemeden “nokta yok” demesi.
- Dilbilgisinin kendisi değişmedi. Birim, grad, ifade ve ondalık virgül `UX-05`'tir.

### Iced'de girdinin yolu

Iced bir tuşa basışı şöyle verir: değiştiricisiz tuş, düzenin ürettiği tuş, fiziksel konum, değiştiriciler ve yazdığı metin. Odaktaki yazı kutusu yazdığını kendine alır (`Captured`); gerisi aboneliğe gelir. Masaüstü tek abonelikle (`keys::key_event`) her alınmamış basışı yönlendiriciye verir (`input.rs`). Sıra ADR 0018'inkidir:

1. **Açık metin alanı ya da pencere.**
   - Değer alanı açıksa her tuşu o alır; yalnız web'in `allowInInput` akorları geçer (Ctrl+S, F tuşları …).
   - Komut satırı odaktaysa yazdığını kendisi alır. Ona gelmeyenler (Tab, akorlar) yalnız aynı akorlarsa çalışır. Böylece komut satırındayken Ctrl+Z artık çizimi geri almaz; ADR 0020'nin açık maddesiydi.
   - Pencere açıksa Esc onu kapatır.
2. **IME birleştirmesi:** masaüstünde henüz yok.
3. **Çalışan komutun seçenek harfi** kısayoldan önce gelir. Poligonda G “Geri”dir.
4. **Kısayollar** web envanterinden gelir.
   - Akorlar (Ctrl, Alt, F tuşları) her komuta ulaşır: taşınmamışı “web'de var” der.
   - Tek tuşlar yalnız taşınan komutlara ulaşır (ADR 0017).
   - `+` ve `−` yalnız komut çalışmıyorken yakınlaştırır.
   - Akor, web'in `chordFromEvent` kuralıyla yazılır: harf ve `+ - ,` ürettiği karakterle, gerisi konumuyla.
5. **Değer başlatan karakter:** rakam, `.`, `@`; komut çalışırken `-` ve `+` da.
   - İmleç çizim alanındaysa, komut çalışıyorsa ve tercih açıksa değer alanı açılır; değilse komut satırı.
   - İlk karakter tam bir kez girer.
   - Karakter olayın metninden alınır, tuşun konumundan değil. AltGr ile yazılan simge metindir: Windows onu Ctrl+Alt diye bildirir, harf ya da rakam değilse akor sayılmaz.
6. **Başka metin** komut satırını açar (ADR 0017, CAD alışkanlığı).

- **Tuş anlamları:**
  - Enter ve Boşluk onaylar. Komut yokken son komutu yeniden başlatırlar.
  - Esc önce değer alanını, sonra komutu kapatır.
  - Backspace siler. Tab değeri korur, odağı taşımaz.
- **Değer alanı uygulamanın durumudur, Iced yazı kutusu değildir.** Nedenleri:
  - ilk karakter, odak görevlerinin sırasına bağlı kalmadan alana tam bir kez girer;
  - Iced'in yazı kutusu Boşluk'u yazar, Esc'i yutup odağı bırakır, Tab'e dokunmaz; ADR 0018'in anlamları için davranışını baştan değiştirmek gerekirdi.

  Alan bugün yalnız sona ekler ve Backspace'le siler. İçinde imleç gezdirme, seçme ve yapıştırma yok (aşağıda).
- **Komut satırı** KentOS UI'ın bileşenidir. Üç seçime bağlı yetenek eklendi; vitrin eskisi gibi kalır:
  - `space_submits`: Boşluk Enter'dır.
  - `on_cancel`: boş satırda Esc komuttan çıkar ve odağı bırakır.
  - `on_focus`: bileşen odağı alıp bıraktığını söyler.

  Uyarılar geçmişte uyarı rengiyle görünür (`Entry::Warning`).
- **Fare** (`viewport::gesture`):
  - Sol basış bir noktadır: float64 kamerayla dünyaya çevrilir.
  - 300 ms'den kısa sağ tık Enter'dır. Basılı sağ tığın açacağı menü henüz yok.
  - Orta tuşla kaydırma ve çift tıkla tümünü gösterme olduğu gibi kaldı.
  - Çizime tıklamak açık değer alanını uygulamadan kapatır: web'de alan odağı kaybeder.
  - Alan penceredeki yerini de bildirir; oynatıcı ve görüntü aracı onu kullanır.
- **Shift** bir tıklamada ortho'yu tersine çevirir; değiştirici durumu abonelikten gelir.
- **Taşınan komutlar:** `tool.polygon` (G, şerit düğmesi, `KA`/`ALAN`/`POLYGON`), `tool.confirm`, `tool.cancel`, `view.zoomIn`, `view.zoomOut`, `view.zoomExtents` (Home). Masaüstünde 19 / 163 komut çalışıyor.
- **İstem:** durum çubuğu ve komut satırı adımı ve seçenekleri düğme olarak gösterir. Web bunları çiziminin üstündeki şeritte de gösterir; masaüstünde o şerit yok.

### Önizleme nasıl çizilir

- **Karar:** taslak, wgpu çizim alanının üstünde Iced'in canvas katmanında çizilir. Ölçü etiketi imlecin sağ altına, değer alanı sağ üstüne yerleştirilen bileşenlerdir (`preview.rs`). Katman her zaman vardır, boşken boştur; çizim alanının bileşen ağacındaki yeri, dolayısıyla hareket durumu değişmez.
- **Neden wgpu sahnesinin bir parçası değil:**
  - Taslak her imleç hareketinde değişir, birkaç noktadır. wgpu sahnesi belgenin sürümüne göre önbelleğe alınır, parça kimliğiyle bir kez yüklenir (ADR 0019). Her harekette yeni parça yüklemek bu sözleşmeyi bozardı.
  - Önizlemenin kesik çizgisi ve yazısı var. Hat henüz kesik çizgi (`REN-11`) ve yazı (`REN-12`) çizmiyor.
  - Web de araç önizlemesini belgenin GPU katmanlarında değil, ayrı bir 2B katmanda çizer.
- **Hassasiyet:** dünya → ekran dönüşümü float64 kamerayla yapılır. Canvas yalnız ekran farklarını float32 alır; mutlak koordinat GPU'ya gitmez.
- **Renk:** temanın vurgu jetonu. Kapalı şekil %8 dolgulu ve kesik çizgilidir; yardımcı çizgiler kesiktir. Web'deki gibi.
- `REN-11` ve `REN-12` gelince taslağın wgpu katmanına taşınması ölçümle yeniden değerlendirilebilir.

### İzlerin native oynatıcısı

- `apps/desktop/src/traces.rs`, `fixtures/interaction/v1/*.json`'ı değiştirmeden okur. Bilmediği bir alan onu durdurur; yeni alan sessizce atlanamaz. Gerçek `App`'i pencere açmadan, penceresinin göndereceğiyle sürer:
  - **Tuşlar:** US ya da Türkçe Q klavyenin ürettiği Iced tuş olaylarıdır. Web oynatıcısının tablosu kullanılır: Türkçe Q'da `+` Shift+4, `-` Equal tuşu, `@` AltGr+Q (Ctrl+Alt). Olaylar uygulamanın kendi aboneliğinden geçer.
  - **Fare:** Iced fare olaylarıdır ve çizim alanının kendi `gesture` kodundan geçer. Konum, dünya noktasının aynı kamerayla düştüğü pencere pikselidir, ekranın aygıt pikseline yuvarlanır: 2× ekranda yarım mantıksal piksel.
    - Oynatıcının penceresinde alanın boyu tek sayıdır ve üstü yarım piksele düşer. Böylece yuvarlama gerçekten sınanır.
    - Alanın dışına düşen nokta izi durdurur.
  - **Komut satırı:** odaktayken tuşlar bileşenin bir modeline gider. Bir test modeli gerçek bileşene tuş tuş bağlar: mesajlar ve bileşenin olayı alıp almadığı. Test, KentOS UI'ın ekransız çizicisiyle çalışır (`Snapshot::software`, `Snapshot::deliver`).
  - **`run`:** komutun mesajıdır, şerit düğmesinin gönderdiği.
  - **`saveAndReopen`:** Ctrl+S ve Ctrl+O. Dosya seçici geçici bir dosyayla yanıtlanır (`Picker::File`; web oynatıcısının `picker`'ı). Uygulamanın kayıt ve açma görevleri sonuna dek çalışır (`iced_runtime::task::into_stream`).
  - **Odak:** görevdeki bir odak işlemi, oynatıcının izleyemediği bir değişikliktir; izi durdurur. Bu dört izde yoktur.
- **Karşılaştırma** web oynatıcısının kurallarıyladır:
  - tıklanan noktalar `clickTolerance` içinde;
  - yazılan değerlerin kenar farkları tam eşit;
  - ölçek göreli 1e-9 içinde;
  - `dynamicInput: null` ile alanın hiç yazılmaması ayrıdır.
- **Sonuç:** 4 iz × 3 varyantın hepsi geçer.
- **Ters deneme:** üç davranış bilerek bozuldu; izler tam ADR 0018'in adlandırdığı adımlarda düştü.
  - Komut çalışırken `-` yakınlaştırınca `polygon-signs` 4. adımda düştü. `-4` yazılınca nokta `[4, 0]`'a gitti.
  - Tab alanı kapatınca `polygon-keys` 20. adımda düştü.
  - AltGr simgesi akor sayılınca Türkçe Q'da `polygon-accept` 7. adımda düştü. `@` kayboldu, köşe koordinat başlangıcına, yüzlerce kilometre öteye kondu. US ve 2× varyantları geçti, çünkü orada `@` Shift+2'dir.
- **Görüntü:** `kentos-cad snapshot çıktı.png --iz <iz> [--adim <n>] [--varyant us|tr-q|hidpi]` izi görüntüye oynatır. Uygulama çizimin ortasında görünür: taslak, değer alanı, istem. 2× varyant 2× çizilir.

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| Araç kodu | `PathTool`, `ToolManager` (TS) | `kentos_interaction` (Rust) | — |
| Hesap | WASM | native | `kentos-geometry-core` |
| Yazılan değerin okunması | `coordinateInput.ts` | `point_text.rs` | `fixtures/point-input/v1` |
| Davranış | tarayıcıda oynatıcı | native oynatıcı | `fixtures/interaction/v1` |
| İstem | metin, ayrıştırılır | veri, metne yazılır | metin (testte karşılaştırılır) |
| Değer alanı | DOM `<input>` | uygulama durumu | ADR 0018'in anlamları |

## Sonuçlar

- **Testler:**
  - `kentos-interaction`: 16 test, bir kural bir test.
  - `geometry-core`: dilbilgisinin 6 birim testi ve ortak dosya.
  - Masaüstü: 27 test. Oynatıcı, komut satırı modeli ve tuş yönlendirmesi bunlara dahil.
  - Web: aynı ortak dosyadan 69 vitest.
- **Bağımlılıklar:** yeni paket yok. `Cargo.lock`'a yalnız yeni crate'in satırları girdi.
  - `iced_runtime` masaüstünün doğrudan bağımlılığı oldu. Oynatıcı uygulamanın görevlerini onunla çalıştırır. Crate Iced ve KentOS UI'ın `snapshot` özelliğiyle zaten ikilideydi.
  - Iced'in `canvas` özelliği masaüstünde adıyla istendi; KentOS UI üzerinden zaten açıktı.
- **Web'de davranış değişmedi.** Web tarafında yalnız ortak dilbilgisi dosyasını okuyan vitest eklendi.
- **Masaüstünde** kilitli katmana yazmayan ilk araç geldi (ADR 0020'nin açık maddesi). Komut satırındaki Ctrl+Z maddesi de kapandı.

## Ertelenenler

- Öbür araçlar: çizgi, çoklu çizgi, `PathTool`'un ölçme ve parsel biçimleri, dikdörtgen …
- Masaüstünde nesne keneti ve nesne izleme. İzlerin hepsinde kenet kapalıdır; kenet açık bir iz oynatıcıyı açık bir iletiyle durdurur.
- Nokta hesaplayıcı ve askıdaki araç (`UX-07`).
- IME (`UX-03`, `UX-12`) ve Türkçe F klavye varyantı.
- Basılı sağ tığın komut menüsü, Shift + sağ tığın kenet menüsü.
- Değer alanında düzenleme: imleç gezdirme, seçme, yapıştırma. Alanın ekran kenarına göre yerleşimi `UX-08`'dir.
- Ortho, kutupsal izleme, kenet açıklığı ve `cursorInput` için ayar penceresi ve kalıcılık (`SET-*`). Bugün web'in varsayılanlarıyla çalışır.
- Tipli `PromptSpec` (`UX-02`) ve dilbilgisinin kararı (`UX-05`).
- Web ile bilinçli farklar:
  1. Komut satırında Ctrl+Z masaüstünde hiçbir şey yapmaz; web'de alanın kendi geri almasıdır.
  2. Değer alanı yalnız sona ekler (yukarıda).
- Kapanan fark (25 Eylül): seçenek ya da kısayol olmayan harf artık web'de de komut satırını açar (ADR 0018, 6. adım). `pnpm e2e` denetliyor.
  - İzlere bu adım henüz eklenmedi. Native oynatıcı, uygulamanın odak işlemlerini (`focus`, `unfocus`) ve komut satırının öneri listesini izleyemiyor; öneri listesi açıkken Enter öneriyi çalıştırır. Oynatıcı ikisini izleyince komut adı yazan bir iz eklenecek.

## Doğrulama (25 Eylül 2026, Linux; main `2c659c3` üstünde)

- `pnpm rust:test`: 429 test geçti, clippy temiz, bağımlılık yönü temiz (17 crate, 22 crate × hedef). Veritabanı testleri bu çalışma ağacında yerel sunucu olmadan kendini atlar; bu değişiklik sunucuya dokunmaz.
- `pnpm rust:test:desktop`: masaüstü 27, KentOS UI 170, vitrin 55, render 27 test geçti; clippy temiz. GPU testi `KENTOS_GPU_TESTS` olmadan atlandığını söyler.
- `cargo test -p kentos-interaction`: 16 test geçti.
- Native oynatıcı: 4 iz × 3 varyant geçti. Ters deneme yukarıda.
- `pnpm typecheck` temiz. `pnpm test`: 889 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm e2e:interaction`: web 4 iz × 3 varyant geçti.
- `pnpm inventory:check` güncel. `cargo fmt --all --check` temiz.
- `KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot --iz polygon-accept`:
  - 5. adımda: değer alanında `12`, ölçü etiketi, taslak ve iki yerde istem;
  - 8. adımda: kesik kapanış kenarı ve alan;
  - 10. adımda: wgpu'nun çizdiği bitmiş üçgen.
- Pencere (`make desktop`) açıldı, günlükte hata yok; `make stop-desktop` ile kapatıldı. Pencerede elle klavye ve fare denemesi bu çalışmada yapılmadı.
