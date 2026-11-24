# ADR 9200: Render motoru — araştırma, ölçülen kusurlar ve hedef mimari

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md).
- **İstek:** sahibin sözü (23 Kasım 2026):

  > render motorunda çok ciddi problemler var … çizim alanı saf wgpu olacak … sahne tam bir CAD gibi gözükmüyor …
  > veri kaybolmaları var lod sistemi yok … büyük veri olan projelerde yakınlaştırma uzaklaştırma yaptığımızda bazı
  > katmanlar ekrana yapışıyor … arayüz ve çizim tuvali tamamen birbirinden bağımsız olacak; şu anda mesela sahnede
  > obje seçtiğimde bazen arayüzün de yenilendiği fark ediyorum.

  Aynı istekte NCZ ve DWG alımının da kusursuz hâle getirilmesi istendi.
- **Yerine geçtiği:** [ADR 0019](0019-native-wgpu-viewport.md)'un “sahne iced'in karesinde, iced'in olaylarıyla çizilir”
  kararı RM-05 ve RM-07 ile bu ADR'nin §3'üne geçer. Ortak WGSL ve kayan nokta hassasiyeti kararları (REN-07) aynen
  kalır.
- **Kapsam dışı:** 3B sahne (kendi bütçeleri var, ADR 0910) ve web'in çizicileri (web duruyor, ADR 0228).

## §1 Ölçülen kusurlar

**Ortam:** 22–23 Kasım 2026, Xvfb'de gerçek pencere ve gerçek fare olayları, lavapipe (yazılım Vulkan), hata ayıklama
derlemesi. Görüntüler `.run/gui-evidence/render/arastirma/` altında. Gerçek GPU ölçümü RM-01'de.

### §1.1 Tuval ve arayüz birbirine bağlı

1. **Her kamera ve seçim olayı bütün arayüzü yeniden kuruyor.**
   - Çizim alanının her olayı bir uygulama iletisi yayınlıyor: `Message::Viewport`
     (`apps/desktop/src/shell/viewport/mod.rs`, `Program::message`).
   - Bu ileti `App::update`'ten ve bütün `App::view`'dan geçiyor: şerit, paneller ve ~15 katman yeniden kuruluyor.
   - Kamera uygulamanın durumunda; tuval kendi başına kare çizemiyor.
   - Sahibin “nesne seçince arayüz de yenileniyor” gözlemi bunun görünen yüzü.
2. **Çizim alanı saf wgpu değil.** `apps/desktop/src/shell/view/area.rs`, wgpu sahnesinin üstüne iced katmanları
   yığıyor:
   - çizim yazıları (`cad/labels`, iced Canvas);
   - veri ve servis etiketleri, arazi, hat, onaylı katman;
   - seçim ve düzenleme işaretleri;
   - kartlar, komut çubuğu.

   Gerçek boyutlu yazıların katmanı yakınlaştırmada her karede baştan çiziliyor (`labels/layer.rs`).
3. **Hızlı yakınlaştırma geç ve sıçrayarak geliyor.** sivas.kcad'de (1 M nesne) on dört çentiklik hızlı bir yakınlaştırma
   yapıldı (`arastirma/sivas-jest`):
   - 1,2 s boyunca ekrana hiç yeni kare gelmedi;
   - ardından kareler büyük sıçramalarla geldi;
   - son görünüme 2–3,7 s'de varıldı.

### §1.2 LOD, ayıklama ve yeniden kullanılan resimler

4. **Eğriler görünüm için değil, bütün çizim için parçalanıyor** (`crates/render/wgpu/src/scene.rs`, `build_curves`).
   - Çizimin bütün kirişleri 1 M'yi aşarsa tolerans her denemede dört kat kabalaşıyor (en çok 12 deneme).
   - Büyük bir çizimde yakınlaşınca daireler köşeli görünüyor.
   - Görünüm ayıklaması yok (REN-10 açık).
5. **LOD parça parça ve yarım.**
   - Var olanlar: eğrilerin yakınlık bandı (1063), uzak çizgi sadeleştirmesi (1064), uzak blok vekili (1065), küçük
     yazının bant olması (0786).
   - Bir yol boyunca 50 000'den sonraki işaret atılıyor (`style-core`, `MAX_MARKERS_PER_PATH`).
   - CAD-ADV-19 ve PERF-11..15 “kısmen” kaldı: 10⁵ nesnelik uzak görünüm bütçesi tutmadı.
6. **Resim beş ayrı yoldan yeniden kullanılıyor; her birinin kendi anahtarı var:**
   - saklanan resim (0120);
   - kaydırmada taşınan resim ve şeritler (1067);
   - tekerlekte ölçeklenen resim (1068, varsayılan kapalı);
   - yazıların hareket boyunca eski yerleşimi (0597, 0626 §7);
   - iced Canvas önbellekleri.

   **Varsayım:** “ekrana yapışma” bu sınıftan bir hata. Anahtarı bir durumu kaçıran resim, kamera değiştiği hâlde
   ekranda kalıyor. lavapipe'ta Suşehri DXF, sivas.kcad ve NCZ ile yeniden üretilemedi; RM-02'nin doğruluk denetçisi
   avlayacak.
7. **Gezinme kartı yapışıyor.** Kart yakınlaştırma boyunca yerinde kalıyor ve artık imlecin altında olmayan nesneyi
   adlandırıyor (`ncz-gezinme-karti.png`: on altı çentik daha yakınlaşınca altında “HAT_DOGALGAZ” ile boş ekran).

### §1.3 Görünüm

8. **“Tümünü göster” çizimi bir noktaya indiriyor** (`ncz-tumunu-goster.png`, `dwg-tumunu-goster.png`).
   - Orta çift tık büyük NCZ'yi de SİVAS AVM DWG'sini de ekranda tek bir noktaya indiriyor; görünüm 0,0'ı da içine
     alıyor.
   - Açılışta sığdırma `scene::extents`'i kullanıyor. “Tümünü göster” ise `App::zoom_extents` → `spatial.extent()` yolunu
     kullanıyor (gizli katmanlar dahil).
   - İki ayrı kutu var ve dizinde sıfır noktasında bir şey duruyor.
9. **DXF'te birim yazılmamışsa mm varsayılıyor.** Suşehri DXF'in koordinatları metre.

### §1.4 NCZ alımı

Dosya: `SIVAS_UIP_2025_06_HAZIRAN.NCZ` (72 MB, 453 534 nesne).

- 3 saniyeden kısa sürede açılıyor, en yüksek bellek kullanımı 3,8 GB.
- Yakın görünüm CAD'e uygun (`ncz-yakin.png`).

10. **18 nesne alınmıyor:**
    - 6 daire: yarıçapı sıfır ya da sayı değil.
    - 1 yay: iki ucu aynı noktada. Tam turdur ama `crates/shared/ncz/src/emit.rs` onu reddediyor.
    - 4 yay: yarıçapı ya da açısı geçersiz.
    - 1 yay: açıklığı boş.
    - Okuyucunun tanımadığı türler: 0 (5 kayıt) ve 14 (1 kayıt). Türlere göre dağıtımda 8 de yok.
11. **NCZ ile DXF karşılaştırılamıyor.** Aynı planın DXF'i 657 MB ve DXF okumasının 256 MiB sınırına takılıyor
    (`crates/native/application/src/core/file_import.rs`).

### §1.5 DWG alımı

Dosya: `SİVAS AVM_2015.06.11.dwg` (32 MB, 601 619 nesne).

12. **8 051 öğe alınmıyor** (`dwg-rapor.png`):
    - “Görünmez” işaretli olduğu için: 6 328 LWPOLYLINE, 513 LINE, 437 ATTDEF, 437 ATTRIB. Bu sayılar şüpheli,
      doğrulanacak.
    - WIPEOUT 4, IMAGE, REGION 33.
    - Sınırı okunamayan 3 tarama, 196 boş ATTRIB.

    Bunlara ek olarak 8 829 öğe değiştirilerek alınıyor.
13. **DWG iki kez ayrıştırılıyor.** Yol: DWG kod çözücüsü → bellekte DXF metni (270–463 MB) → DXF okuyucusu.

## §2 Araştırma: bir CAD'de nasıl olmalı

- **Yerleşik CAD programları (AutoCAD, BricsCAD, Netcad):**
  - Sahne GPU'da tutulur. “Yeniden çiz” (kamera değişti) ile “yeniden üret” (veri değişti) ayrı yollardır.
  - Kamera değişimi veriyi yeniden işlemez.
  - Eğri pürüzsüzlüğü görünüme göredir (VIEWRES, “düzgün” görünüm): yakında daire hep yuvarlaktır.
  - Çizgi kalınlığı ekranda piksel olarak gösterilir (LWDISPLAY) ve en az bir pikseldir.
  - Çizim yazısı başka bir yazıyla çakıştığı için düşmez.
  - Kapsama göre yakınlaştırma donmuş katmanı saymaz, kapalı katmanı sayar. KentOS'ta da böyle (ADR 0311 §1).
- **OpenCADStudio (açık kaynak, Rust):**
  - Çizim alanı yine iced'in shader widget'ında.
  - Yazı, daire, elips ve kesikli çizgiler CPU'da parçalanmıyor, GPU'da çiziliyor.
  - Bloklar için önbellek ve GPU bütçeleri var.
  - DWG kod çözücüsü (acadrust, MPL-2.0) KentOS'ta da kullanılıyor (ADR 0586).
- **CADCraft (açık kaynak):** egui üzerinde görüntüleme listeleri; sahne nesne başına komut listesine iniyor.
- **Harita motorları (MapLibre, QGIS):**
  - Veri döşeme piramidine bölünür.
  - Ekran toleransıyla sadeleştirilir. QGIS'te öntanımlı eşik 1 pikseldir.
  - Çocuk döşeme hazır değilse ebeveyni gösterilir; ekran hiçbir zaman boş kalmaz.
  - Etiket çakışması yalnız harita etiketlerine uygulanır.
- **Teknik zemin (kaynak kodundan doğrulandı):**
  - iced 0.14.2'de shader widget bir olaydan sonra ileti yayınlamadan yalnız yeniden çizim isteyebilir
    (`iced_widget::Action::request_redraw`). Kamera tuvalde yaşayabilir, `App::view` çağrılmaz.
  - wgpu 27.0.1'de `Device` ve `Queue` `Clone + Send + Sync`. Tuval kendi iş parçacığında iced'in aygıtıyla kendi
    dokularına çizebilir; iced yalnız son dokuyu ekrana koyar.

## §3 Karar: hedef mimari

### §3.1 Tuval motoru

Tuval, kendi başına çalışan bir motordur (`crates/render/canvas`, `kentos-render-canvas`). `kentos-render-wgpu` onun
altında kalır.

- **Motorun sahip oldukları:** kamera, jestler (kaydır, yakınlaştır, tümü, gezinme, isabet), kare zamanlaması ve
  bütün çizim içeriği.
- **Uygulama ile tek kanal:**
  - Uygulama → motor: belgenin üretimi ve değişen nesneler, seçim, katman görünürlüğü, ayarlar, araç önizlemesi.
  - Motor → uygulama: tıklanan dünya noktası ve isabet, gezinme, seyreltilmiş “kamera değişti” olayı (durum çubuğu ve
    görünüm geçmişi için).
  - Kamera olayı `App::view`'ı çağırmaz.
- **Çizim iş parçacığı:**
  - Tuval kendi dokularına üçlü tamponla çizer.
  - iced'in shader primitive'i yalnız biten son dokuyu tek bir dörtgenle karesine koyar.
  - Arayüz sahneyi beklemez, tuval de arayüzü beklemez.
- **Tuval pencerenin içinde kalır.** Ayrı bir işletim sistemi yüzeyi olmaz: menüler, kartlar ve komut çubuğu ayrı bir
  yüzeyin üstünde Wayland, X11 ve Windows'ta güvenle çizilemez.
- **Geçici resim için tek kural:**
  - Her doku, çizildiği kamerayla damgalanır.
  - Yeni kare hazır değilse son doku, damgadaki kameradan şimdiki kameraya dönüştürülerek gösterilir. Bu dönüşüm
    geometrik olarak doğrudur; resim ekrana yapışamaz.
  - Taze kare gelince yerini alır.
  - §1.2'deki beş yeniden kullanma yolu kalkar.

### §3.2 Döşemeler ve LOD

- Çizim ve veri katmanları arka planda dünya döşemelerine (dörtlü ağaç) bölünür. Katman ve nesne sırası korunur.
- **Düzeyler:** her düzey ekran toleransıyla sadeleştirilir.
  - 1 pikselden küçük nesne düşmez, bir piksel olarak çizilir.
  - Okunmayacak kadar küçük yazı bant olarak kalır (0786).
- **Eğriler:** daire, yay ve elips görünüme göre çizilir. Bütün çizime yayılan kiriş bütçesi kalkar.
- **İşaret ve simgeler:** örnekli (instanced) çizilir, yol başına sınır kalkar.
- **Eksik döşeme:** hazır değilse yerinde ebeveyni gösterilir.

### §3.3 İçerik ve arayüz

- **Bütün çizim içeriği motorda:**
  - CAD yazıları: GPU glif atlasında, TTF ve SHX.
  - Etiketler, işaretler, seçim ve tutamaçlar.
  - İsabet: kimlik tamponundan.
- **iced'de yalnız arayüz kalır.** Dünya noktasına bağlı kartlar paylaşılan kamerayı çizim anında okur ve hareket
  boyunca gizlenir.
- **Seçim** yalnız ilgili paneli değiştirir; ağır paneller girdilerine göre saklanır.

### §3.4 Alım

- **NCZ:** tam tur yay daire olur; tanınmayan türler ikili inceleme ve açık kaynak okuyucularla çözülür.
- **DXF:** akışlı okunur, 256 MiB sınırı kalkar.
- **DWG:** kod çözücünün nesneleri doğrudan KentOS nesnelerine çevrilir, DXF metin köprüsü kalkar.
- **Görünmez bayrağı** aynı dosyaların DXF hâlleriyle doğrulanır.
- **Birim** yazılmamışsa tahmin edilir; emin olunamazsa sorulur.

## §4 Sonuç

Bu ADR araştırmanın ve kararın kaydıdır. Her madde kendi ADR'sini (9201'den başlayarak) ve kendi ölçümünü getirir.

Kabul ölçütleri [plan/render.md](../../plan/render.md)'dedir. Başlıcaları:

- gerçek GPU'da kamera değişikliğinden onu gösteren kareye p95 ≤ 16 ms;
- kamera olayı başına arayüzün yeniden kurulması 0;
- doğruluk denetçisinde taze çizimden fark yok;
- görüntüde sessizce düşen nesne yok.
