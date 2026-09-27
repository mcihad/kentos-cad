# ADR 0090: Masaüstünde stilli çizim

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.9, §14; docs/STYLE.md §6; DESIGN.md §8; ADR 0008 (“Stil derleyicisi”), 0019 (wgpu çizim hattı), 0029 (seçim deposu), 0055 (çizimin yazıları)
- **Sahibin yönü (27 Eylül):**
  - web'in stil pencereleri ve SVG düzenleyicisi masaüstüne gelir;
  - MPYY sembolojisinin tamamı kusursuz çizilir;
  - performans kırmızı çizgidir.
- **Sahibin onayı (27 Eylül):**
  - tiny-skia 0.11.4 `kentos-render-wgpu`'ya;
  - roxmltree 0.20.0 masaüstüne.

## Bağlam

- **Web:** gösterilen her katmanı stil motorundan çizer (`render/styledLayer.ts`):
  - her nesne kendi sembolüyle;
  - yoksa katmanın işleyicisiyle;
  - yoksa katmanın düz görünüşüyle.
  Düz görünüş de sembollere çevrilir: kalınlık ve kesik kâğıt milimetresidir, nokta simgesi ekran pikselidir.
- **Masaüstü:** katmanları kendi düz sahnesinden çiziyordu. Kalınlık yoktu; kesikler ekran pikseliydi. Nesnenin sembolünü, katmanın işleyicisini ve sistem kitaplığını (770 MPYY öğesi) hiç çizmiyordu.
- **Paylaşılan parçalar:**
  - hesabın hepsi ortak Rust'tadır (`kentos-style-core`: çözümleme, derleme, yerleşim, GPU toplulukları);
  - gölgelendiriciler `shaders/wgsl/styled` ve sözleşmesi `styled.layout.json`'dadır.
- **Web'e kalanlar:** sayfanın payı TypeScript'teydi:
  - programı kurmak;
  - ifade tablosu;
  - toplulukların renkleri ve atlas görüntüleri;
  - atlasın kendisi.

## Karar

### Sayfanın payı: `kentos-native-style` (`crates/native/style`)

- **Kitaplık** (`library.rs`): sistem, kullanıcı ve proje olmak üzere üç kaynak.
  - Bir kimlik önce projede, sonra kullanıcıda, sonra sistemde aranır.
  - Öğeler kaydedildikleri JSON'la tutulur, okunmayan alanlar gidip gelirken kaybolmaz.
  - Açık çizimin `ProjectStyles`'ı çizim her değiştiğinde projeye yüklenir.
- **Sistem kitaplığı** (`system.rs`): web'in TypeScript'inden yazılan `.kstil` dosyasıdır (`crates/native/style/assets/system-library.kstil`).
  - 695 sembol, 81 çizim, 138 kategori; 800 KB, masaüstüne gömülü.
  - İlk kullanımda bir kez okunur.
  - `apps/web/scripts/style/system-library.test.ts` dosyayı yazar (`KENTOS_WRITE_SYSTEM_STYLES=1`). TypeScript'ten geri kalınca `pnpm test`'te düşer.
- **Düz görünüş** (`simple.rs`): `fromLayer.ts`'in karşılığı; çizgi tiplerinin mm kesikleri, nokta simgesi, taramanın kendi deseni.
- **Katman kurulumu** (`program.rs`, `table.rs`): `buildStyledLayer`'ın sayfa yarısı.
  - Her nesnenin dört sayısı: kip, küme ya da sembol, düz görünüşün kümesi, renk.
  - Kümeler, renkler ve kendi sembolleri ilk gerektikleri sırayla, birer kez yazılır.
  - Kullanılan semboller ve döşenen görüntülerin boyları programa girer.
  - İfade tablosu `exprTable`'ın düzenindedir.
  - Sembol ölçeği (`symbol_scale_of`) web'in çeyrek oktav adımıdır.
- **Topluluklar** (`batches.rs`): `styledBatches.ts`'in karşılığı.
  - Tema jetonları paletten çözülür.
  - SVG renkleri (`param(fill)`, `param(stroke)`, `currentColor`) sembolden verilir.
  - Görüntü anahtarları web'inkilerle bayt bayt aynıdır, döşeme deseninin JSON'u dahil.
  - Hesaplananlar: erişim, desenin uzak tonu, yazı ve görüntü genişliği.
- **Ortak durumlar:** `fixtures/style/v1/batches.json`'ın 8 durumunu da geçer (`crates/native/style/tests/batches.rs`). Denetlenenler:
  - sembol ölçeği;
  - her nesnenin kararı;
  - program;
  - nesnelerin sayıları;
  - tablo;
  - float32'ye kadar toplulukların kendisi.

### GPU'da: `kentos-render-wgpu::styled`

- **Boru hatları:** sözleşmenin altı boru hattı (vuruş, düz, tarama, desen, döşeme, işaret) web'in karıştırma kurallarıyla, her örnek sayısı için bir kez.
- **Katmanın arabellekleri** üç parçadır; bir topluluk bir bağlama ve bir çizim çağrısıdır:
  - toplulukların sayıları tek köşe arabelleğinde;
  - her topluluğun 144 baytlık stil bloğu tek bir uniform arabellekte, dinamik ofsetle;
  - bağlama grubu katman başına bir tane.
- **Bağlama grupları:** Iced'in aygıtı en çok iki bağlama grubu alır (`max_bind_groups: 2`, iced_wgpu). Sözleşme üç grup kullanır.
  - Masaüstü atlası 2. gruptan 0. gruba, çerçevenin yanına alır (bağlama 1 ve 2). Değişen yalnız bu iki sayıdır (`styled::shader::ATLAS_REMAP`), gölgelendiricinin kendisi ortaktır.
  - `tests/styled_contract.rs` ortak modülü ve masaüstününkini naga ile doğrular: bağlamalar, yapıların yerleşimi, giriş noktaları, köşe düzenleri ve karıştırmalar sözleşmenindir.
  - Web ajanına öneri: sözleşmenin 2. sürümünde atlas çerçevenin grubuna geçsin, bu çeviri kalksın.
- **Atlas:** 2048² tek sayfa.
  - Görüntü gösterildiği boyda çizilir, ikinin kuvveti adımlarla (8–512 px); raf yerleşimi, yerine duran boy, sayfa dolunca baştan (`render/atlas.ts`'in kuralları).
  - Görüntüler işlemcide tiny-skia ile çizilir: SVG ve raster işaretler, yazılar, desen döşemeleri, şekiller (`canvasShapes.ts`'in karşılığı).
  - Bir kare yeni görüntülere en çok 12 ms harcar. Kalanlar sonraki karelere bırakılır ve çizim alanı kendisi yeni bir kare ister; çok sembollü bir çizim açılışta tek bir kareyi dondurmaz.
- **Renkler:** Iced yüzeyi sRGB olmayan biçimde açar (`web-colors`, tarayıcının tuvali gibi).
  - Renkler yazıldıkları gibi gider, karışım web'deki gibi sRGB uzayındadır; yarı saydam dolgular iki platformda aynı tonu verir.
  - sRGB bir yüzey çıkarsa stil blokları doğrusallaştırılır.

### Resimler: `apps/desktop/src/style`

- **SVG** (`svg.rs`): roxmltree ile okunur. Renkler, dönüşümler, uzunluklar, viewBox ve yol verisi SVG çekirdeğinin okuyucularıdır.
  - `<use>` ve `<symbol>` okunur.
  - Parlaklık `<mask>`'ı okunur; sistemin 7 piktogramı bununla çizilir.
  - Yazılar sistemin harf ana hatlarıyla çizilir.
  - Kalıtım, `opacity`, dolgu kuralı, kesik, uç ve birleşim işlenir.
  - Stil sayfaları, degradeler ve gömülü görüntüler bu dilimde yok; sistemin çizimleri kullanmaz. 81 sistem çiziminin hepsi okunur (test).
- **Yazı işaretleri** (`images.rs`): Iced'in yazı sisteminden harf ana hatları olarak çizilir.
  - Web'in yazı yığınlarındaki ilk bulunan yüzle: Linux'ta Arial Liberation Sans, Times Liberation Serif'tir (tarayıcının fontconfig takma adları gibi).
  - Hiçbiri yoksa KentOS'la gelen Arimo kullanılır.
- **Raster:** PNG çalışma alanındaki `png` ile çözülür. JPEG çözücü kilitte yok; JPEG varlığı masaüstünde henüz çizilmez.

### Çizim alanı

- **Hangi katmanlar:** gösterilen her katman stilli çizilir (`style/scene.rs`, `Viewport::styled_scene`).
  - Düz sahne yalnız ızgarayı (altta) ve seçimle üzerine gelmeyi (üstte) çizer.
  - Yazılar yazı katmanında kalır (ADR 0055); ölçüler kendi ince çizgileriyle çizilir.
- **Ne zaman yeniden kurulur:** bir katman yalnız çizdiği bir şey değişince kurulur.
  - Nesneleri değişince: belgenin günlüğü hangi yuvaların değiştiğini söyler. Silinen nesnenin katmanı son kurulumdan bilinir.
  - Stili ya da adı değişince.
  - Palet, sembol ölçeği ya da kitaplık değişince: hepsi.
  - Yardımcı çizgisi olan katman, kırpma kutusu kayınca.
  - Hiçbiri değişmediyse kare bir karşılaştırmaya mal olur.
- **Paralel kurulum:** kurulacak katmanlar makinenin çekirdeklerinde yan yana kurulur (en çok 8 iş parçacığı, büyükler önce). Depo, kitaplık ve görünüş yalnız okunur.
- **Ekranda sabit semboller:** tekerlek dönerken son kurulan ölçekte kalır, durunca (150 ms) yeniden kurulur (web'in kuralı).
- **Ayarlar:** `graphics.symbolSize` ve `graphics.lineWeights` masaüstünde de ayardır.
  - Uygulama ayarları'nda “Semboller ve çizgiler” başlığı altındadır.
  - Durum çubuğundaki Kalınlık ile `view.symbols.plot` ve `view.symbols.screen` komutları çalışır.

## Ölçüm (27 Eylül 2026, Linux, release, bu makine)

`cargo test --release -p kentos-desktop style::perf -- --ignored --nocapture --test-threads=1`: parseller (20 köşe), noktalar ve kırık çizgiler üç düz görünüşlü katmanda.

| Nesne | Düz sahne (önceki) | Stilli kurulum | GPU belleği (düz → stilli) | Bir nesne değişince | Değişmeyen kare |
|---|---|---|---|---|---|
| 25 000 | 14,0 ms | 15,7 ms | 7,8 → 5,2 MB | 7,0 ms | 0,001 ms |
| 125 000 | 80,6 ms | 91,7 ms | 39,2 → 26,2 MB | 32,3 ms | 0,001 ms |
| 250 000 | 172,8 ms | 180,1 ms | 78,4 → 52,4 MB | 67,6 ms | 0,001 ms |

- **İlk kurulum** düz sahneyle başa baştır (%4). İlk yazımda paralel kurulum yoktu ve 250 000 nesnede fark %44'tü (246,9 ms).
- **GPU belleği** üçte bir azdır.
- **Bir düzenleme** yalnız kendi katmanını kurar. Düz sahne her düzenlemede bütün çizimi kuruyordu: 250 000 nesnede 172,8 ms yerine 67,6 ms.
- **Web'in gösterim kataloğu** (2049 nesne, 124 katman, 1920 topluluk) 9,6 ms'de kurulur. Paralel kurulum olmadan 33,3 ms'ydi.
- **Çizim çağrısı:** düz görünüşlü bir katman üç topluluktur, yani üç çağrı.

## Doğrulama

- **Rust:**
  - `cargo test -p kentos-native-style`: `batches.json`'ın 8 durumu, renk okuma, sistem kitaplığı.
  - `cargo test -p kentos-render-wgpu`: `styled_contract` (naga, bağlamalar, yerleşim, boru hatları), atlas adımları, çizim testleri (maskeli grup, şekillerin ana hatları, viewBox'a sığdırma).
  - `cargo test -p kentos-desktop`: 269 test, SVG okuyucusu, yazı yığınları ve base64 dahil.
  - `cargo clippy … -D warnings` üç paket için.
- **Web:** `system-library.test.ts`, sistem kitaplığı dosyasının güncel olduğunu denetler.
- **Görüntüler:**
  - Web'in gösterim kataloğu masaüstünde ve web'de aynı görünümlerde çekildi (`apps/desktop/src/style/screens.rs`, `apps/web/scripts/style/showcase-shots.mjs`): 1:1000, bölümün sol üstü görünümde.
  - Görünümler: Temel çizgi tipleri, işaretler ve alanlar; UİP sınırlar, konut ve çalışma, yapı düzeni, sosyal altyapı, karayolları.
  - Koyu ve açık tema, 1440×900 ve 1100×650.
  - Web ile masaüstü piksel ölçeğinde aynıdır: kesikler, kalınlıklar, taramalar, desenler, dişliler, piktogramlar, yazı işaretleri ve yollar.
  - Web'in çizimi `KENTOS_DEMO_OUT=… pnpm -C apps/web exec vitest run scripts/style/demo-drawing.test.ts` ile dosyaya yazılır.

## Bu dilimde olmayanlar

- JPEG görüntü varlıkları, SVG'lerde stil sayfaları ve degradeler.
- Bağlama grubu çevirisinin kalkması: sözleşmenin 2. sürümü, web ajanıyla birlikte.
- float32 hassasiyeti web'deki gibidir. Topluluklar çizimin yerel orijine göre float32'dir; masaüstünün düz sahnesindeki yüksek/alçak bölme stilli yolda yoktur (ADR 0019). Büyük koordinatta çok yakın görünümde kıpırtı ikisinde de aynı sınırdadır.
- Stil pencereleri: sonraki dilimler (Katman stili, Stil yöneticisi, Lejant, Sembol tasarımcısı, SVG düzenleyicisi).
