# ADR 0030: `.kcad` v2'de büyük çizim: tipli işçi sınırı, aşamalı açılış, kayıt hataları ve kurtarma kopyası

- **Durum:** kabul edildi (2026-09-26). Yön [ADR 0025](0025-kcad-v2-encoding.md)'in ertelenenlerinden ve TODOS.md §9'dan (`FILE-15`, `FILE-18..20`, `FILE-24`) gelir. Sınırın düzeni, açılışın aşamaları ve kuralları, kayıt hatalarının dili, kurtarma kopyasının yeri ve ömrü ve ölçüm yöntemi bu dilimin kararıdır. Dosya biçimi (spesifikasyon, örnek dosyalar, şema) değişmedi. İki soru sahibindedir: kopyaların ömrü ve tarayıcının dosya sınırı (Sonuçlar).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md §9 (`FILE-15..20`, `FILE-22`, `FILE-24`), §20; CLAUDE.md §4.8, §6.2, §20, §21.2–21.3; ADR 0005 (taslak), 0011, 0014, 0025, 0026

## Bağlam

- ADR 0025'te web kaydı çizimi biçim işçisine JSON olarak veriyor, açılış çözülen çizimi yapılandırılmış kopyayla alıyordu. Bu dilimin başında tarayıcıda ölçüldü (`82dcbb9`, Chrome 154, i5-11300H; [kcad-web-before](../perf/kcad-web-before-2026-09-26.md)): 200 000 parsellik, 96,6 MB'lık çizimde Kaydet 14,4 s sürdü, sayfa 1,2 s kesintisiz dondu, Chrome 2,2 GB daha aldı; Aç 10,9 s, en uzun donma 3,3 s, +1,7 GB.
- Açılış iki platformda da tek parçaydı: ilerleme ve Vazgeç yoktu; açılış sürerken yapılan düzenleme dosya gelince sorusuz gidiyordu. Masaüstünde önce başlayıp sonra biten açılış yenisinin üstüne gelebiliyordu.
- Kayıt hatalarının çoğu tek bir “yazılamadı: …” iletisiydi; izni unutulmuş dosyada izin yazmadan önce yeniden istenmiyordu. Masaüstünde kayıt durdurulamıyordu ve anlık görüntü arayüz iş parçacığında alınıyordu (200 000 parselde 176 ms).
- Yerel çizimin kaydedilmemiş işinin kopyası yoktu; yalnız bulut projelerinin cihaz taslağı vardı (ADR 0026).

## Karar

### Tipli sınır (`FILE-15`)

Nesneler altı tipli dizide geçer. Nesneler dışındaki her şey (ad, ayarlar, köken, katmanlar, stiller) sözleşmenin JSON'u olarak kalır (“baş”): küçüktür ve stil motorunun parçaları zaten JSON'dur.

| Dizi | Tür | İçerik |
|---|---|---|
| `kinds` | u8 | nesne başına tür numarası (sözleşmenin 13 türü) |
| `uids` | 16 bayt | nesne başına kalıcı kimlik |
| `ints` | u32 | önce katman tablosunun boyu; nesne başına katman (tablodaki sırası), bayraklar, öznitelik sayısı ve türün sayıları (köşe, yay, delik …) |
| `floats` | f64 | türün sayıları, bit bit (−0 −0 kalır) |
| `text` + `text_lengths` | UTF-16 kod birimi + u32 | önce katman tablosu; nesne başına renk, etiket, sembol (varsa), öznitelikler anahtar sırasıyla (UTF-8 bayt sırası, sözleşmenin haritası gibi), türün metinleri |

- Düzen `crates/shared/kcad/src/columns.rs`'in başındaki tablodadır; sayfa tarafı `io/columns.ts` aynı düzeni yazar ve okur. İki taraf her örnek dosyada aynı dizileri üretir.
- **UTF-16:** JavaScript metni UTF-16 tutar; sayfa bütün metin dizisini bir kez `TextDecoder('utf-16le')` ile çözer. Eşi olmayan vekil (UTF-8'e yazılamaz) yeri söylenerek reddedilir (`invalid_utf8`, `entities/{i} ({tür}) › alan`); önce JSON ayrıştırıcısı anlaşılmaz bir hatayla düşüyordu.
- **Sahiplik:** tamponlar `postMessage` ile aktarılır, kopyalanmaz; gönderen onları bir daha okumaz.
- **Kayıt:** sayfa çizimi tek turda (o anın sürümü) sütunlara yazar (`packDrawing`) ve işçiye verir. İşçi (`encode_columns`) sütunlardan çizimi kurar, kodlar, baytları çözer ve çözüleni gelen sütunlarla akarak bit bit karşılaştırır (`columns::differs`); sayfa başı JSON metniyle karşılaştırır. Tutmayan baytlar sayfaya hiç dönmez. Sözleşmenin bilmediği alanlar yazılmaz ve söylenir (ADR 0025'teki gibi).
- **Açılış:** işçi baytları çözer, sütunlara ayırır (`split`) ve baytları bırakır; sayfa sütunları okur ve v1'deki gibi denetler.
- **Masaüstü:** `encode_verified` JSON metni yerine aynı sütun karşılaştırmasını (`first_difference`) kullanır: 100 000 parselde doğrulamalı yazma ~650 → 392 ms (`cargo test --release -p kentos-kcad --test measure -- --ignored --nocapture`).
- `FORMATS_VERSION` 5 → 6: işçi ile sayfanın sözleşmesi değişti; önbellekte kalmış eski işçi yeni sayfayla konuşmaz.
- **WebAssembly'nin katmanlı derlemesi:** V8 çalışmakta olan bir WebAssembly çağrısını üst derleme katmanına taşımaz: saniyelerce süren tek bir çağrı temel derleyicinin kodunda kalır. Nesne başına işler bu yüzden ayrı, satır içine alınmayan (`#[inline(never)]`) işlevlerdir ve özet 1 MB'lık parçalarla hesaplanır; sık çağrılan küçük işlevler üst katmana geçer. İşçide 200 000 parselin kodlaması 2,8 → 0,64 s.
- **Biçim modülü** (ADR 0025'in yöntemiyle): 1 263 396 → 1 348 503 bayt; brotli 309 977 → 323 659 (+13,7 KB), gzip 411 363 → 428 852. Modül yine tembel yüklenir.

### İşçinin ömrü

- Vazgeç işçiyi sonlandırır: WebAssembly'deki uzun döngü mesaj beklemez; sonlandırmak hem durdurur hem belleği geri verir. Bekleyen istekler `cancelled` ile düşer; sonraki işlem yeni işçi açar.
- 16 MB'tan büyük bir işlemden sonra işçi boşta kalır kalmaz durdurulur (öbür durumda 30 s sonra): WebAssembly belleği küçülmez ve büyük bir açılıştan sonra yüzlerce MB tutardı. Yeniden başlaması geliştirme sunucusunda ~0,45 s.

### Aşamalı açılış (`FILE-20`)

- **Aşamalar:** dosya okunur (8 MB üstünde parça parça, ilerlemeyle) → bütünlük özeti denetlenir (yüzdeyle) → proje bilinir (ad, nesne ve üst katman sayısı; nesnelerden önce) → nesneler okunur (her 2 048 nesnede bir haber) → sayfa nesneleri denetler (katman, SRID, köşe; 12 ms'lik dilimlerle, arada sayfa boyanır) → çizim tek adımda değişir → pencere ilk kare çizilince kapanır. Masaüstünde aynı aşamalar kendi iş parçacığındadır (`iced_runtime::task::blocking`); son aşama belgenin kurulmasıdır.
- **Pencere:** baştan kiplidir (modal); 250 ms'den kısa açılışta görünmez. Web'de `ui/io/OpeningDialog.ts`, masaüstünde KentOS UI `Dialog` ve `overlay::modal`. Vazgeç, Esc, × ve arka plan durdurur.
- **Açılış sürerken çalışan komut yoktur** (`FILE-20`'nin sorusu). Web'de kipli pencere fareyi ve kısayolları keser, dosya komutları `busy` iken kapalıdır. Masaüstünde komut, tuş (Esc dışında), katman, ayar ve çizime tıklama alınmaz; görünüm kaydırılabilir ve yakınlaştırılabilir (düzenleme değildir).
- **Kurallar** (CLAUDE.md §21.2):
  - Yarım okunmuş dosya çizim olmaz: belge yalnız bütün dosya okunup denetlendikten sonra, tek adımda değişir.
  - Durdurulan ya da ardından başka açılış başlatılan açılış hiçbir şeyi değiştirmez; işçiden ya da iş parçacığından geç gelen cevap da.
  - Açılış sürerken ekrana başka çizim geldiyse ya da çizim hiçbir yerin tutmadığı biçimde değiştiyse (yerel çizimde bir işlemin sonucu) açılış geri çekilir ve bunu söyler. Bulut projesinin değişiklikleri (proje onları tutar; örneğin bir ortağın düzenlemesi) açılışı durdurmaz. Projeden yalnız açılış kesinleşince çıkılır; geri çekilen açılış projeyi bağlı bırakır.
  - Bellek: masaüstünde dosyanın baytları belge kurulmadan bırakılır (96,6 MB'lık açılışın bellek artışı 448 → 367 MB); web'de baytlar işçiye aktarılır, sayfada kalmaz.
- **Tarayıcının dosya sınırı 256 MB:** daha büyüğü okunmadan, nedeni ve masaüstü önerisiyle reddedilir. 96,6 MB'lık açılış sayfaya ve işçiye ~0,85 GB ekledi; doğrusal olarak 256 MB ~2,3 GB, biçimin 1 GiB'ı ~9 GB eder. Spesifikasyon (§3.4) platforma daha küçük bir sınır izni verir. Masaüstü biçimin sınırını uygular.

### Kayıt ve hataları (`FILE-18`)

- **Web:** izin, kullanıcının tıklaması geçerliyken ve kodlamadan önce yeniden istenir. Tarayıcı yazıcının kopyasına yazar; dosya yalnız `close()` başarıyla bitince değişir, hata olursa yazıcı iptal edilir (`abort`). Yalnız yazılan sürüm kaydedilmiş sayılır (`markSaved(revision)`, değişmedi).
- **Masaüstü:** arayüz iş parçacığı yalnız belgenin nesneleri paylaşan kopyasını alır (200 000 parselde 176 → 18 ms). Anlık görüntü, doğrulamalı kodlama, geçici dosyaya 8 MB'lık parçalarla yazma, `fsync`, geri okuma, yer değiştirme ve dizin `fsync`'i kaydın iş parçacığındadır. 300 ms'den uzun kayıtta sağ altta aşamasıyla bir panel ve durdurma düğmesi çıkar; durdurma yer değiştirmeden önce geçerlidir.
- Kullanıcının gördüğü. Her durumda çizim kaydedilmemiş kalır, önceki dosya olduğu gibi durur ve ileti bunu söyler:

| Durum | Platform | İletinin özü | Yapabileceği |
|---|---|---|---|
| İzin verilmedi ya da unutuldu | web | “… yazma izni verilmedi; çizim kaydedilmedi” | Kaydet'e yeniden basıp izin vermek ya da Farklı kaydet |
| İzin yazarken geri alındı | web | “… için yazma izni yok ya da geri alındı” | aynı |
| Disk ya da depolama dolu | web, masaüstü | “… diskte (ya da tarayıcının depolama alanında) yer kalmadı” | yer açıp yeniden kaydetmek ya da başka diske Farklı kaydet |
| Yazma izni yok, disk salt okunur | masaüstü | “… yazma izniniz yok”, “disk salt okunur” | izni denetlemek ya da Farklı kaydet |
| Klasör ya da dosya artık yok | web, masaüstü | “… taşınmış ya da silinmiş” | Farklı kaydet ile yeni yer |
| Dosya başka yerde kullanılıyor | web | “… başka bir program ya da sekme tarafından kullanılıyor” | biraz sonra yeniden ya da Farklı kaydet |
| Kayıt durduruldu | masaüstü | “… kaydı durduruldu; dosyaya dokunulmadı” | yeniden kaydetmek |
| Kayıt sürerken pencere kapatıldı | masaüstü | “… kaydediliyor; kayıt bitince pencereyi yeniden kapatın” | beklemek |
| Sekme kapandı, atıldı ya da tarayıcı çöktü | web | (sayfa yok) | sonraki açılışta kurtarma kopyası sorulur |

- **Hata taklidi:** masaüstünde `saving::Faults` her adımda (oluşturma, yazma, `fsync`, geri okuma, yer değiştirme) seçilen işletim sistemi hatasını üretir; web testleri `DOMException` adlarını yazıcının her adımında üretir. Testler kullanıcının dosyalarına dokunmaz (geçici klasör, bellek).

### Kurtarma kopyası (`FILE-19`)

- **Ne:** kaydedilmemiş çizimin KCAD v2 baytları (kaydın doğrulanmış kodeği; kalıcı kimlikler dahil) ve birkaç bilgi (ad, dosyanın adı, zaman, nesne sayısı). Hiçbir `.kcad` dosyasının içinde ya da yanında değildir.
- **Nerede:** web IndexedDB `kentos.recovery/copies`. Bulut taslaklarının deposu (`kentos.cloud/drafts`) kullanılmadı: ömürleri ve biçimleri ayrıdır; bulut projesinin değişikliklerini cihaz taslağı tutar, ona kopya yazılmaz. Masaüstü `$XDG_DATA_HOME/kentos-cad/kurtarma` (yoksa `~/.local/share/…`): her çalışan KentOS'un kendi klasörü ve çalıştıkça kilitli tuttuğu `kilit` dosyası; kopya `<n>.kurtarma` ve `<n>.json`, geçici dosyaya yazılıp yer değiştirilir. Klasör kullanılamazsa açılışta komut satırında söylenir.
- **Ne zaman:** çizim değiştikten 3 s sonra, değişmeye devam ederse en geç 30 s'de bir; web'de sekme gizlenince hemen. Açılış sürerken yazılmaz. Kayıt dosyayı yazarken sekme gizlenirse kaydın doğrulanmış baytları kopya olur (ikinci kodlama yok). Kopya biçim işçisinde kodlanırken açılışın Vazgeç'i işçiyi sonlandırırsa kopya uyarısız, çizim durulunca yeniden yazılır.
- **Silinir:** kayıt çizimi temizlediğinde ya da değişiklikler bilerek bırakıldığında (Kaydetmeden devam et, aç, çık). Sorusuz değiştirilen çizimin (üstüne bulut projesi açıldı) kopyası kalır ve sonra sorulur: kaydedilmemiş iş kendiliğinden silinmez.
- **Sorulur:** uygulama açılınca, sahibi artık çalışmayan kopyalar (web: sekmenin `navigator.locks` ile tuttuğu kilit; masaüstü: klasör kilidi) en yeniden başlayarak birer birer sorulur. **Geri yükle** kopyayı aşamalı açılışla açar; çizim kaydedilmemiş ve dosyasız olur (Kaydet yer sorar, hiçbir dosyanın üzerine kendiliğinden yazılmaz); kopya silinir ve yerine hemen yenisi yazılır. **Sonra** kopyayı sonraki açılışa bırakır, **Sil** siler. Web'de `ui/widgets/confirm.ts`, masaüstünde KentOS UI `Dialog`.
- Web Locks olmayan tarayıcıda başka bir sekmenin açık olup olmadığı bilinemez; bugünkü tarayıcıların hepsinde vardır.

### Ölçüm (`FILE-24`)

- **Çizim:** parsel başına 20 köşeli alan, üç öznitelik ve etiket (`crates/shared/kcad/tests/measure.rs` ile aynı, belirlenimli), 1–200 000 parsel (96,6 MB). **Web:** `pnpm perf:kcad`; Vite geliştirme sunucusu, başsız Chrome, makinenin GPU'su; uygulamanın kendi Farklı kaydet'i ve Aç'ı; en uzun görev Long Tasks API'den, bellek Chrome'un bütün süreçlerinin PSS toplamından. **Masaüstü:** `apps/desktop/src/perf.rs` (`--release`), her işlem kendi sürecinde, bellek VmHWM. 3 koşunun ortancası; dosyalar `.run/`'a yazılır.
- 200 000 parsel, 96,6 MB, i5-11300H, Chrome 154:

| Ölçüt | Önce | Sonra |
|---|---|---|
| Web Kaydet | 14 417 ms | 2 168 ms |
| Web kayıtta en uzun donma | 1 227 ms | 361 ms |
| Web kayıtta bellek artışı | 2 192 MB | 663 MB |
| Web Aç, çizim ekranda | 10 878 ms | 3 704 ms |
| Web açışta en uzun donma | 3 263 ms | 716 ms |
| Web açışta bellek artışı | 1 684 MB | 850 MB |
| Masaüstü kaydın arayüz iş parçacığındaki payı | 176 ms | 18 ms |
| Masaüstü yazma (kaydın iş parçacığında) | 1 418 ms | 1 049 ms |
| Masaüstü açma (aşamalı, durdurulabilir) | 635 ms | 627 ms |
| Masaüstü açışta bellek artışı | 448 MB | 367 MB |

- Bütün boylar: [kcad-web-before](../perf/kcad-web-before-2026-09-26.md), [kcad-web-after](../perf/kcad-web-after-2026-09-26.md), [kcad-desktop-before](../perf/kcad-desktop-before-2026-09-26.md), [kcad-desktop-after](../perf/kcad-desktop-after-2026-09-26.md).
- **Tarayıcıda 200 000 parselin aşamaları** (tek koşu, yaklaşık). Kayıt: sayfada paketleme 0,37 s (tek görev; dosya bir anın çizimidir), işçide sütunlardan çizim 0,25 s, kodlama 0,64 s, özet 0,37 s, doğrulamanın çözmesi 0,52 s. Açılış: işçinin başlaması 0,45 s, özet 0,39 s, çözme ve sütunlar 0,77 s, sayfada denetim 0,74 s (dilimli), belgenin değişmesi 0,22 s, ilk kare ~0,7 s. Açıştaki en uzun donma artık çizicinin ilk karesidir.
- Sıkıştırma maliyeti ölçülmedi: v2.0'da kodek yoktur (`FILE-10/11`).
- **Denetim koşusu** (`9f018d7`, aynı makine, 1 koşu, 200 000 parsel): web Kaydet 2 188 ms (en uzun görev 360 ms, +731 MB), Aç çizildi 3 782 ms (en uzun görev 827 ms, +947 MB); yerel kodek 100 000 parselde doğrulamalı yazma 376 ms, 200 000 parselde `encode_columns` 907 ms. Tablodaki ortancalarla aynı düzeyde; tek koşu olduğu için tabloya girmedi.

### Kod yerleşimi

| Parça | Yer |
|---|---|
| Sütunlar; ilerleme ve durdurma | `crates/shared/kcad/src/columns.rs`, `watch.rs`; `encode_watched`, `decode_watched`, `split`, `encode_columns` |
| WASM bağlayıcısı | `crates/wasm/formats-wasm`: `encodeKcad(baş, altı dizi, ilerleme)`, `decodeKcad(baytlar, ilerleme)` → `Kcad` |
| Sayfa | `io/columns.ts`, `io/kcad.ts`, `io/client.ts`, `app/drawingFile.ts`, `app/fileIO.ts`, `app/fileAccess.ts`, `app/recovery.ts`, `ui/io/OpeningDialog.ts` |
| Masaüstü | `apps/desktop/src/opening.rs`, `saving.rs`, `recovery.rs`; ölçüm `perf.rs` ve pencere resimleri `screens.rs` (ikisi de elle çalıştırılır) |

### Doğrulama

- **Rust:** sütunların birim testleri; her geçerli örnek dosya sütunlardan geçip kodeğin kendi baytlarını verir (`tests/columns.rs`); 600 rastgele çizim sütunlardan geçip değişmeden döner (`tests/robustness.rs`); ilerlemenin sırası, her adımda durdurma, sütunların ret nedenleri; geri okununca gönderilen sütunları vermeyen baytlar (UTF-16 sırasındaki öznitelikler) `verify_failed` ile reddedilir.
- **Web:** JS ve Rust her örnek dosyada aynı sütunları üretir; −0, uç değerler, her tür ve isteğe bağlı alan yazılıp okunur (`io/columns.test.ts`, `io/kcad.wasm.test.ts`). Açılışın sırası, her aşamada Vazgeç, geç gelen cevap, değişen ya da değiştirilen çizim, bulut projesi, 256 MB (`app/fileIO.opening.test.ts`); kayıt hataları ve kayıt sürerken kapanan sekme (`app/fileIO.failures.test.ts`); kurtarma kopyaları, değişmeye devam eden çizimin en geç yarım dakikada bir kopyası, geri yüklenen kopyanın açık dosyaya gitmemesi ve işçinin sonlanmasıyla duran kopyanın yeniden yazılması (`app/recovery.test.ts`); işçinin sayfanın gönderdiği başla karşılaştırması (`io/kcad.wasm.test.ts`); tamponların işçiye aktarılması ve büyük çizimden sonra işçinin hemen durması (`io/client.test.ts`).
- **Masaüstü:** her adımda disk hataları ve geri okunan baytları değiştiren disk (`Faults::garbled`), her aşamada durdurma, açılışın aşamaları, Vazgeç ve Esc, geride kalan açılış (iki sırada da), değişen çizim, açılışta tıklama, bozuk dosya, kurtarma ve en geç yarım dakikada bir kopya (`opening.rs`, `saving.rs`, `recovery.rs` testleri).
- **Bilerek bozma** (26 Eylül): her kural kaynakta bilerek bozuldu, adı geçen test düştü, kaynak geri alındı ve ağaç temiz kaldı. İlk kayıttaki iki bozma da geçerlidir: açılışın bayat denetimi kaldırılınca iki web testi, açılış sırasında tıklamayı kesen satır kaldırılınca bir masaüstü testi düştü. \* işaretli kuralları dilimin ilk testleri yakalamıyordu: bozma `9f018d7`'nin testleriyle bütün pakette denendi ve hiçbir test düşmedi; test eklendi (`7110077`, `8edc4bc`), bozma yeniden denendi ve düştü.

| Yer | Kural | Bozma | Düşen test |
|---|---|---|---|
| web | 256 MB sınırı dosya okunmadan uygulanır | `tooLarge` hiç reddetmez | `refuses a file larger than the browser opens…` |
| web | unutulan izin kodlamadan önce yeniden istenir | izin sorulmadan yazılır | `asks for a forgotten permission again…` |
| web | hata veren yazıcı iptal edilir | `abort` çağrılmaz | `a permission taken back while the file is written…`, `a full disk…` |
| web | hiçbir yerin tutmadığı değişiklik açılışı geri çeker | yalnız başka çizim açılması denetlenir | `a change nothing keeps stops it…` |
| web | bulut projesinin kendi değişiklikleri açılışı durdurmaz | her değişiklik durdurur | `a change nothing keeps stops it…` |
| web | ekrana başka çizim gelirse açılış geri çekilir | `reset` dinlenmez | `gives way when the drawing on screen changed or another was opened…` |
| web | buluttan yalnız açılış kesinleşince çıkılır | proje açılışın başında bırakılır | `a change nothing keeps stops it…` |
| web | \* geri yüklenen kopya dosyasız açılır | ekrandaki çizimin dosyası kalır | `offers what gone tabs left…` |
| web | geri yüklenen kopya kaydedilmemiş sayılır | `markUnsaved` yok | `offers what gone tabs left…` |
| web | yalnız sahibi çalışmayan sekmelerin kopyaları sorulur | Web Locks denetimi yok | `offers what gone tabs left…` |
| web | kayıt çizimi temizleyince kopya silinir | silinmez | `keeps a copy of unsaved work; a save removes it`, `a save that fails keeps the recovery copy…` |
| web | bilerek bırakılan değişikliklerin kopyası silinir | silinmez | `dropping the changes on purpose removes the copy…` |
| web | sorusuz değiştirilen çizimin kopyası kalır | çizim değişince silinir | `dropping the changes on purpose removes the copy…` |
| web | açık bulut projesine kopya yazılmaz | yazılır | `keeps no copy of an open cloud project…` |
| web | kayıt sürerken gizlenen sekmede yalnız o anın baytları kopya olur | eski sürümün baytları da kopya olur | `the tab hidden while a save writes an older revision…` |
| web | \* değişmeye devam eden çizimin kopyası en geç 30 s'de yazılır | yalnız 3 s sessizlikten sonra | `writes a copy at least every half minute…` |
| web | \* çizimin tamponları işçiye aktarılır, kopyalanmaz | aktarım listesi boş | `hands a drawing and a file over without a copy…` |
| web | \* büyük çizimden sonra işçi boşta kalır kalmaz durur | 30 s beklenir | `hands a drawing and a file over without a copy…` |
| web | \* işçi dosyanın başını sayfanın gönderdiğiyle karşılaştırır | karşılaştırma yok | `refuses bytes whose head reads back otherwise…` |
| web | işçinin sonlanmasıyla duran kopya uyarısız yeniden yazılır (`1bf2eaa`'daki düzeltme) | yeniden yazılmaz, uyarı çıkar | `a copy stopped with the worker…` |
| kodek | \* geri okunan nesneler gönderilen sütunlarla karşılaştırılır | `columns::differs` yok sayılır | `columns_that_do_not_read_back_as_sent_are_refused_as_unverified` |
| kodek | eşi olmayan UTF-16 vekili yeri söylenerek reddedilir | kayıplı dönüştürülür (U+FFFD) | `unpacking_gives_back_the_objects…`, `columns_the_codec_cannot_take…` |
| kodek | durdurulan okuma çizim, durdurulan yazma bayt vermez | izleyicinin `false`'u yok sayılır | `a_read_reports_the_project_before_its_objects…`, `a_write_stopped_at_any_step_gives_no_bytes` |
| masaüstü | kayıt geçici dosyaya yazılıp yer değiştirmeyle konur | doğrudan hedefe yazılır | `a_failing_disk_leaves_the_previous_file_and_says_what_to_do` |
| masaüstü | \* diskten geri okunan baytlar yazılanlarla karşılaştırılır | karşılaştırma yok | `a_failing_disk_leaves_the_previous_file_and_says_what_to_do` |
| masaüstü | durdurma yer değiştirmeden önceki son ana dek geçerlidir | son denetim yok | `a_save_stopped_at_any_stage_before_the_rename_leaves_the_previous_file` |
| masaüstü | kayıt sürerken pencere kapanmaz | kapanır | `a_window_closing_while_a_save_runs_waits_for_it` |
| masaüstü | \* önce başlayıp sonra biten açılış hiçbir şeyi değiştirmez | açılışın kimliği denetlenmez | `a_later_open_overtakes_an_earlier_one` |
| masaüstü | açılış sürerken çizim değiştiyse açılış geri çekilir | denetlenmez | `an_open_gives_way_when_the_drawing_on_screen_changed_meanwhile` |
| masaüstü | durdurulan açılış son aşamada da hiçbir şey vermez | belge kurulmadan önceki denetim yok | `a_read_stopped_at_any_stage_gives_nothing` |
| masaüstü | yalnız çalışmayan KentOS'ların kopyaları sorulur | klasör kilidi denetlenmez | `a_gone_kentos_copy_is_offered_and_restored…` |
| masaüstü | geri yüklenen kopya dosyasız açılır | kopyanın yolu kalır | `a_gone_kentos_copy_is_offered_and_restored…` |
| masaüstü | kayıt çizimi temizleyince kopya silinir | silinmez | `unsaved_work_gets_a_copy_that_a_save_removes` |
| masaüstü | bilerek bırakılan değişikliklerin kopyası silinir | silinmez | `dropping_the_changes_on_purpose_removes_the_copy` |
| masaüstü | \* değişmeye devam eden çizimin kopyası en geç 30 s'de yazılır | yalnız 3 s sessizlikten sonra | `a_copy_is_written_at_least_every_half_minute…` |

## Sonuçlar

- CLAUDE.md §2 (depolar, ölçüm komutu) ve §4.8 için önerilen metin teslim raporundadır.
- **Kalanlar:**
  - Web kaydının ilerleme paneli ve durdurması yok (masaüstünde var); 200 000 parselde kayıt 2,2 s.
  - Kaydın ve kurtarma kopyasının sayfadaki paketlemesi tek görevdir (200 000 parselde ~0,36 s donma), çünkü dosya bir anın çizimidir. Belgenin yazınca kopyalanan (copy-on-write) bir anlık görüntüsü onu dilimlere bölebilirdi.
  - Doğrulamanın çözmesi az önce hesaplanan özeti yeniden hesaplar (~0,37 s); özetsiz bir çözme yolu açılışla aynı kodu ikiye bölerdi.
  - Açıştaki en uzun donma çizicinin ilk karesidir (~0,7 s; çizim hattının işi).
  - v1 dosyası `JSON.parse` ile tek adımda okunur (yalnız eski dosyalar).
  - Kurtarma kopyası her seferinde bütün çizimdir; çok büyük çizimde yazımı pahalıdır (işçide ~2 s ve IndexedDB'ye ~100 MB). Değişiklik günlüğü ayrı bir tasarımdır.
  - Masaüstünde kaydın bellek artışı dosya boyunun ~6,6 katıdır (anlık görüntü, baytlar, geri okuma); akarak yazma ayrı iştir.
  - Masaüstünde kayıt sürerken çöken uygulama, kayıttan önceki son 3 saniyenin değişikliklerini kopyada bulamayabilir (kopya kayıt sürerken yazılmaz).
  - Web ölçümü geliştirme sunucusundadır; üretim derlemesinde işçinin başlaması daha kısadır.
- **Sahibe sorular:** kopyaların ömrü (geri yüklenene ya da silinene dek / 7 gün / 30 gün); tarayıcının dosya sınırı (256 MB / 512 MB / biçimin 1 GiB'ı).
- **Varsayılanlar (26 Eylül, birleştirmede):** sahibin sorularında ve dilimin iki açık noktasında önerilen seçenekler uygulandı. Sahip başka birini seçerse değişir.
  - kopyalar geri yüklenene ya da silinene dek kalır: kaydedilmemiş iş sorulmadan silinmez;
  - tarayıcının dosya sınırı 256 MB;
  - web kaydının ilerleme paneli ve Durdur'u sonraya kaldı (`FILE-25`);
  - `FILE-24` sıkıştırma ölçümü olmadan kapandı. v2.0'da sıkıştırma yok; ölçümü `FILE-10/11` ile yapılacak.
