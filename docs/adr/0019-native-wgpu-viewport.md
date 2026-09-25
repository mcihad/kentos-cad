# ADR 0019: Masaüstünün çizim alanı: Iced'in aygıtında KentOS'un wgpu hattı

- **Durum:** kabul edildi (2026-09-25). Yön sahibin kararıdır: ana çizim alanı KentOS'un kendi wgpu hattıdır (ADR 0010). Ayrıntılar bu dilimin kararıdır. İki ürün davranışı web'le aynı başladı ve sahibe soruldu: açılışta önce başlangıç görünümü, dolgusu olmayan katmanda poligonun dolgusuz çizilmesi.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** TODOS.md §8.1 (`REN-01..07`), §6 (`UI-11`, Iced shader widget'ı notu), §8.2 (`AA-*`); ADR 0010 (platform sınırı), 0016 (KentOS UI), 0017 (masaüstü kabuğu)

## Bağlam

- ADR 0017'nin kabuğunda çizim alanı bir yer tutucuydu. Katmanlar, özellikler ve kayıt çalışıyordu; çizim görünmüyordu.
- ADR 0010: masaüstünün ana CAD sahnesi KentOS'un wgpu hattıdır. KentOS UI'ın Iced Canvas'lı örnek çizim alanı üretim motoru sayılmaz. WGSL web ile paylaşılabilir, web'in kendi WebGPU/WebGL2 renderer'ları korunur.
- Iced 0.14'ün shader widget'ı, çizimi Iced'in kendi wgpu aygıtı ve karesiyle yapmaya izin verir. ViewCube bunu depoda zaten kullanıyor.

## Karar

### Sorumluluklar (`REN-01`)

- **`crates/render/wgpu`** (paket `kentos-render-wgpu`). Iced'i ve pencere sistemini bilmez:
  - `Camera`: görünüm float64 dünya biriminde (x doğu, y kuzey) ve mantıksal pikselde. Kaydırma, imleçte yakınlaştırma, sığdırma, ekran ↔ dünya. Sınırlar web'inkiyle aynı: 1e-4 … 5000 px/m, sığdırmada 48 px pay.
  - `scene`: çizimin GPU'ya hazır hâli, CPU'da kurulur. `Drawing` özelliğinden okur: bir `.kcad` anlık görüntüsü ya da masaüstünün canlı belgesi, kopyasız. İki parça:
    - sabit parça (noktalar, düz çizgi ve yollar, düz kapalı alanlar ve dolguları, taramalar) yalnız belge ya da tema değişince yeniden kurulur;
    - eğriler (daire, yay, elips, eğri, yaylı yollar) bir de yakınlaştırma bandı değişince.
  - `Renderer`: paylaşılan WGSL'in boru hatları, her görünüm için bir kare uniform'u ve sahne önbelleği. Parça kimliğiyle bir kez yüklenir, kare başına yüklenmez. Görünüm çekilince tamponlar bırakılır.
  - `RenderSettings`, `FrameStats`.
- **`apps/desktop/src/viewport.rs`**: Iced bağlantısı.
  - `shader::Program`: fare hareketleri.
  - `Primitive`: Iced'in aygıtını, kuyruğunu ve render pass'ini renderer'a verir.
  - `Pipeline`: renderer'ı Iced'in ilkel deposunda tutar.
  - Kamera ve sahne önbelleği uygulamanın durumundadır, widget'ın değil. Widget ağacı yeniden kurulunca görünüm atlamaz.
- **Bağımlılık yönü** (`scripts/arch/deps.mjs`, `render` grubu): yalnız `shared` kullanılabilir. Altında Iced, winit, sunucu çalışma zamanları, tarayıcı bağları ve PyO3 bulunamaz. `default-members`'ta değildir; web ve sunucu derlemesi wgpu derlemez.

### Aygıt ve kuyruk paylaşımı (`REN-02`)

- **Aygıt ve kuyruk Iced'indir.** Renderer aygıt açmaz; çizim ve arayüz tek GPU bağlamını ve tek kareyi paylaşır.
- **Render pass Iced'indir.** Renderer doğrudan Iced'in karesindeki pass'e çizer. Ekran dışı doku, kopya ya da CPU'ya geri okuma yoktur (GPU→CPU→GPU döngüsü yok).
- **Viewport ve scissor:** Iced viewport'u widget'ın sınırına (aygıt pikseli), scissor'ı görünen kısmına kurar. Renderer ikisini de değiştirmez; zemini bu viewport'u kaplayan tek üçgenle çizer.
- **DPI:** ölçek katsayısı her karede Iced'in `Viewport`'undan gelir. Kamera mantıksal piksel, uniform aygıt pikselidir. Çizgi kalınlığı ve işaret boyu mantıksal pikseldir.
- **Boyut değişimi:** widget yeni boyutunu bir mesajla bildirir; kamera merkezini ve ölçeğini korur. GPU tarafı her karede gerçek sınırı kullanır.
- **Kompozisyon:** alan, widget'ın katman sırasında çizilir. Iced'in üst katmanları (menü, ipucu, iletişim kutusu) üstüne gelir.
- **Derinlik yok:** 2D sıra: bütün dolgular, sonra bütün çizgiler, sonra işaretler; her birinde alttaki katman önce (web'in sırası).
- **MSAA yok:** Iced'in karesi tek örneklidir. Çizgi ve işaretler shader'da analitik yumuşatılır (kapsama çizgiye uzaklıktan). Dolgu kenarı yumuşatılmaz; üstündeki çerçeve örter. Kendi hedefi ve resolve'uyla MSAA `AA-01..03`'tür.
- **Sınırlar:** Iced aygıtı `max_bind_groups: 2` ve özelliksiz ister. Hat tek bind group kullanır, depolama tamponu kullanmaz; köşe biçimleri WebGPU çekirdeğindedir.
- **Hata:** boru hatları ve yüklemeler wgpu hata kapsamı içinde kurulur. Kurulamazsa uygulama çökmez: alan nedenini ve çözüm yolunu söyler, durum çubuğu da gösterir.

### Paylaşılan WGSL ve sözleşmesi (`REN-04`, `REN-05`)

- **Dosyalar:** `shaders/wgsl/common/{frame,color,marks}.wgsl` ortak matematik ve çizim parçalarıdır. `shaders/wgsl/cad2d/{background,fill,line,marker}.wgsl` boru hatlarının giriş noktalarıdır. WGSL'de include yoktur: modül, dosyaların `cad2d.layout.json` → `sources` sırasıyla birleşimidir.
- **Sözleşme** (`cad2d.layout.json`, sürüm 1):
  - bind group ve `Frame` uniform'u: 64 bayt, alanların ofsetleri;
  - her boru hattının giriş noktaları, topolojisi, çizim başına köşe sayısı ve karışımı;
  - köşe tamponları: adım kipi, adım boyu, öznitelik konumu, biçimi ve ofseti;
  - değerlerin anlamı: sRGB düz alfa renk, hi/lo parçaları, işaret boyu ve türü.

  Bir alan değişirse sürüm artar; Rust, JSON ve WGSL birlikte değişir.
- **Native doğrulama** (`crates/render/wgpu/tests/wgsl_contract.rs`):
  - naga 27.0.3 (wgpu'nun kendi derleyicisi) modülü ayrıştırır ve çekirdek WebGPU'nun ötesinde yetenek istemeden doğrular;
  - Rust yapıları (`offset_of!`, `size_of`), wgpu köşe düzenleri ve boru hattı tanımları JSON'la karşılaştırılır;
  - naga'nın `Frame` yerleşimi ve giriş noktalarının girdi tipleri de JSON'la karşılaştırılır;
  - `LAYOUT_VERSION` Rust, JSON ve WGSL'de aynıdır; her `.wgsl` dosyası modüldedir.
- **Tarayıcı doğrulaması** (`scripts/wgsl/browser-check.mjs`):
  - aynı modül ekransız Chrome'un WebGPU'sunda derlenir (SwiftShader, `apps/web/scripts/e2e/cdp.mjs` bayrakları); `getCompilationInfo` hatası denetimi düşürür;
  - sözleşmedeki boru hatları yalnız JSON'dan, doğrulama hata kapsamı içinde kurulur;
  - TM koordinatlarında JSON ofsetleriyle yazılmış tamponlardan bir kare çizilip geri okunur.
- **Web renderer'ları değişmez** (`REN-03`, `REN-06`). Web WGSL'i ileride aynı JSON'a göre alabilir.

### Hassasiyet (`REN-07`)

- **Koordinatlar CPU'da float64'tür.** GPU'ya her noktanın yerel orijinden (belgenin `origin`'i) farkı iki float32 parça olarak gider: `hi = f32(d)`, `lo = f32(d − hi)`. Kamera merkezi de öyle gider. Vertex shader parçaları ayrı ayrı çıkarır; piksele çevrilen float32, kameradan olan farktır. Mutlak dünya koordinatı GPU'ya gitmez (CLAUDE.md §4.9).
- **Uzun parçalar:** 128 m'den uzun düz parçalar aynı doğru üzerinde parçalara bölünür (yalnız çizim için, belge değişmez). Kameranın altındaki parçanın iki ucu da yakındır; uzak uçlar hassasiyet götürmez.
- **Ölçümler** (E ≈ 487 000, N ≈ 4 420 000 m):
  - shader'ın float32 adımları CPU'da float64'le karşılaştırıldı: her yakınlaştırmada (1e-4 … 5000 px/m) ve üç orijinde (belgenin, kilometrelerce ötesi, (0, 0)) hata 0,001 px'in altında;
  - 0,2 mm/px'te 0,01 mm'lik kaydırmalar: titreme yok, her adım 0,002 px içinde;
  - ekran ↔ dünya gidiş-dönüşü float64'ün kendi sınırında;
  - 20 km'lik çizgiler: bölünmüş hâlde 0,05 px içinde; bölünmeden 0,25 px'ten fazla sapma (bölmenin nedeni);
  - gerçek GPU (Intel Iris Xe, Vulkan, Mesa 26.0.8; `KENTOS_GPU_TESTS=1`): 40 kaydırmada en kötü 0,002 px;
  - tarayıcı (SwiftShader): 0,01 px.
- **Eğriler** paylaşılan çekirdeğin fonksiyonlarıyla parçalanır (`tessellate`: `arc_points`, `circle_ring`, `bulge_path`, `ellipse_points`, `catmull_rom`). Dolgular `triangulate_into` ile üçgenlenir, taramalar `hatch_lines` ile çizilir. Yeni geometri algoritması yazılmadı.
  - Ekrandaki kiriş hatası en çok 0,25 px'tir. Eğriler görünümün gerektirdiğinden bir bant ince kurulur (bant: dünya toleransının ikinin kuvvetine yuvarlanmışı). 4 kat yakınlaşınca ya da 8 kattan fazla uzaklaşınca yeniden kurulur, kare başına değil.
  - Bir milyon kiriş bütçesi aşılırsa tolerans dört katına çıkarılır; bellek tükenmez.
  - Parçalama yalnız çizim içindir; belgeyi, ölçüyü ya da seçimi değiştirmez.

### Çizim kuralları (web'le aynı)

- **Katmanlar:** her atası görünür olan yaprak katmanlar çizilir; listenin üstü en son çizilir.
- **Renkler:** nesnenin kendi rengi, yoksa katmanın rengi. `fg`, `fg-dim`, `ink`, `paper` jetonları DESIGN.md §3.1–3.2'nin tuval renkleridir. Katman rengi veridir; hiçbir şey katman kimliğine göre dallanmaz.
- **Dolgular ve işaretler:**
  - kapalı alan, katmanın dolgusu varsa o renkle dolar;
  - dolu tarama %45 opaklıktadır; çizgili ve çapraz taramalar çekirdeğin tarama çizgileridir;
  - noktalar katmanın nokta biçimiyle çizilir (halka, artı, üçgen; öntanımlı 7 px).
- **Henüz çizilmeyenler:** yazı, ölçü, yardımcı çizgi ve ışın. Sayılır ve durum çubuğunun ipucunda söylenir, tahminle çizilmez.
- **Açılış görünümü:** belgenin başlangıç görünümü, yoksa bütün nesneler (web'in `replaceDrawing`'i). Kapsam, çekirdeğin `entity_bounds_in` kutularının birleşimidir; web deposunun `extent`'iyle eşitliği sınandı.
- **Fare:**
  - orta tuşla sürükleme kaydırır, alanın dışına taşsa da sürer;
  - tekerlek imlecin olduğu yere yakınlaştırır (çentik başına e^0,15, dokunmatik yüzeyde piksel başına e^0,0015);
  - orta tuşa çift tıklamak tümünü gösterir.
- **Çizim isteğe bağlıdır:** Iced bir olaydan sonra yeniden çizer, kendi döngüsü yoktur.
- **Durum çubuğu:**
  - imlecin Y (doğu) ve X (kuzey) değeri, projenin uzunluk basamağıyla;
  - ekran ölçeği “Ekran 1:N”;
  - son karenin sayıları ve henüz çizilmeyen nesneler, çizim motoru ipucunda.

## Sonuçlar

- **Bağımlılıklar:**
  - `wgpu` 27.0.1 artık doğrudan bağımlılık; Iced'in kilitlediği sürüm ve özellikler.
  - `naga` 27.0.3 ve `pollster` 0.4.0 yalnız test için.
  - `Cargo.lock`'a yalnız yeni crate girdi; yeni paket yok.
- **Testler ve görüntü:**
  - `pnpm rust:test:desktop` render crate'ini de kapsar. GPU testi `KENTOS_GPU_TESTS=1` ister; yoksa atlandığını söyler, geçmiş sayılmaz.
  - `kentos-cad snapshot` alanı wgpu çizicisiyle çizer: `--tumu`, `--merkez Y,X`, `--yakinlastir <kat>`. tiny-skia çizicisi alanı çizemez ve bunu uyarır. `make desktop-snapshot` tiny-skia'yı zorladığı için alanı boş gösterir.
- **Belgeyle ilişki:** masaüstünün belgesi native modele geçerken (ADR 0020) sahne onu `Drawing` üzerinden yerinde okur. Belgenin alanlarını bilen tek yer `viewport.rs`'in başındaki küçük bloktur.
- **Sahne ne zaman kurulur:** belgenin revizyonu, açılan çizim ya da tema değişince. Sahne ikinci bir belge değildir: yalnız okunur, hiçbir şey ona yazmaz (`ARCH-04`).

## Ertelenenler

- `REN-08`: kısmi güncelleme. Bugün her revizyon bütün sahneyi yeniden kurar; katman görünürlüğü de öyle.
- `REN-09`, `REN-10`:
  - geçişlerin ayrı bütçesi;
  - görünüm dışı ayıklama;
  - görünüme göre eğri ayrıntısı;
  - arka planda kurma (bugün eğri bandı bütün çizim için ve arayüz iş parçacığında kurulur).
- `REN-11`: çizgi kalınlığı, çizgi türü (kesik çizgi), yuvarlak dışındaki uç ve birleşimler.
- `REN-12`: yazı, etiket ve ölçü.
- `REN-13`: seçim, yakalama, üzerine gelme ve seçim katmanları. Kesin karar CPU'da float64 ile, geometri deposundan verilir.
- `REN-14`, `REN-15`: aygıt kaybından dönüş, küçültme ve askıya alma; isteğe bağlı çizimin ayarları.
- `AA-*`: MSAA, kalite ön ayarları, dolgu kenarının yumuşatılması.
- Yardımcı çizgi ve ışın: görünüme göre kırpma ister.
- Stil motorunun sembolleri ve ızgara.
- Web'in paylaşılan WGSL'i alması (`REN-03`, `REN-06`).

## Doğrulama (25 Eylül 2026, Intel i5-11300H, Iris Xe, Linux)

- `cargo test -p kentos-render-wgpu`, `KENTOS_GPU_TESTS=1` ile: 26 test ve GPU testi geçti.
- `cargo clippy -p kentos-render-wgpu -p kentos-desktop --all-targets -- -D warnings`: temiz.
- `cargo test -p kentos-desktop`: 15 test, 6'sı çizim alanının.
- `node scripts/arch/deps.mjs`: bağımlılık yönü temiz.
- `node scripts/wgsl/browser-check.mjs`: derleyici iletisi yok, boru hatları kuruldu, çizgi 0,01 px içinde.
- `KENTOS_SNAPSHOT_BACKEND=wgpu kentos-cad snapshot`: örnek çizim başlangıç görünümünde, kapsamında, bir parsel köşesinde 700 kat yakınlaştırılmış ve açık temada.
  - Köşede buluşan bütün nesneler aynı piksele düştü.
  - Görüntüden ölçüldü: çizilen köşe (kenarlara oturtulan doğruların kesişimi), float64'ün koyduğu yerden (alanın ortası) 0,001 px ayrıldı. Ölçek yaklaşık 0,22 mm/px, konum E 486 512,34 / N 4 420 187,52.
  - Yakından çekilmiş dairenin kenarı ideal yaydan en çok 0,11 px ayrıldı.
- Pencere (`scripts/dev/svc.sh start desktop`, `make desktop`'un başlattığı servis) örnek çizimle açıldı; günlükte hata yok. `make stop-desktop` ile kapatıldı.
