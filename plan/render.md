# KentOS — Render motoru planı (arayüzden bağımsız tuval, LOD, kayıpsız görüntü; NCZ ve DWG alımı)

Başlangıç: **23 Kasım 2026**. Dal: `render-motoru`; main ile birleştirme yalnız sahip isterse yapılır.

## Sahibin isteği

Özetle:

- Render motorunda çok ciddi sorunlar var. Çizim alanı saf wgpu olacak.
- Arayüz ve çizim tuvali birbirinden tamamen bağımsız olacak. Bugün sahnede nesne seçince arayüz de yenileniyor.
- Sahne tam bir CAD gibi görünmüyor; tuhaflıklar ve veri kaybolmaları var; LOD sistemi yok.
- Büyük projelerde yakınlaştırıp uzaklaştırırken bazı katmanlar ekrana yapışıyor, nesneler ekranda sabit kalıyor.
- Önce bir CAD'de nasıl olması gerektiği iyice araştırılacak.
- NCZ ve DWG alımı da açık kaynak projelere bakılarak kusursuz hâle getirilecek. Örnek dosyalar USB bellekte ve
  İndirilenler'de.

## Çalışma biçimi

Sıra mantık sırasıdır. Her madde şu adımlarla biter:

1. araştırma;
2. kod ve hedefli sınamalar;
3. gerçek pencerede önce/sonra görüntüleri;
4. ADR ve bu planın güncellenmesi;
5. commit ve `render-motoru` dalına push;
6. kısa bir Türkçe rapor, ardından sonraki madde.

Görüntüler `.run/gui-evidence/render/` altındadır (Git'e girmez). Büyük örnek dosyalar depoya girmez. ADR'ler
**9200**'den başlar.

Araştırma, bulgular ve hedef mimari: [ADR 9200](../docs/adr/9200-render-engine-research.md).

## Maddeler

### F0 — Ölçüm, doğruluk denetçisi, hızlı düzeltmeler

- [x] **RM-01 — Ölçüm koşumu.** ([ADR 9201](../docs/adr/9201-render-measurement.md),
  [taban](../docs/perf/files-desktop-once-2026-11-23.md))
  - `perf::files`: gerçek dosyalar uygulamanın kendi açılışıyla, kendi sürecinde açılır. Ölçülenler:
    - `perf::large`'ın izi;
    - on dört çentiklik patlama;
    - “Tümünü göster”in kutusu;
    - bir tıkla seçim;
    - bellek.

    Ölçüm gerçek GPU'da yapılır ve pencere açmaz.
  - Gerçek pencere sürücüsüne `burst` eklendi: çentikler beklemeden verilir, ekran ilk çentikten önce izlenmeye
    başlar. İlk ve son değişimin süresi yazılır.
  - **Taban çizgisi** (UHD 770, release):
    - Tekerlek çentiği sivas.kcad'de 95 ms UI + 108 ms GPU, NCZ'de 71–180 ms UI. Sürenin çoğu `App::view`'da: sahne
      ve yazılar her yakınlık bandında yeniden üretiliyor.
    - İmleç ve kaydırma 1 ms UI.
    - DWG'de bir tık seçim 106 ms UI tutuyor, bunun 65 ms'i görünüm.
    - “Tümünü göster” NCZ'de x'i −4,1 M ile +4,7 M arasına açıyor.
    - DWG açılışı 22,6 s.
  - **Planda olmayan:** GPU süresi zaman damgası sorgusuyla değil, geri okumayla ölçüldü. Sorgu RM-07'ye kaldı.
- [x] **RM-02 — Doğruluk denetçisi; durmuş görünümde eski yazılar.**
  ([ADR 9202](../docs/adr/9202-truth-check-and-settled-labels.md))
  - **Denetçi (`perf::oracle`):**
    - Aynı dosya iki uygulamada açılır; biri her kareyi taze çizer (`Viewport::fresh`). İkisi aynı girdiyi alır:
      imleç, tekerlek, patlama, kaydırma, “Tümünü göster”, dönük görünüm.
    - Her kareden sonra çizim alanları bir piksel komşulukla karşılaştırılır. Farklı karenin üç resmi kaydedilir:
      kısayollu, taze ve fark.
  - **Bulgu, yapışmanın bir kökü:**
    - sivas.kcad'de görünüm durduktan sonra yazılar, tekerleğin başladığı görünümün yerleşiminde ve yazı boylarında
      kalıyordu (dört dinlenme karesinin dördü).
    - Sebep: yazı resminin anahtarı yerleşimi içermiyordu. Durunca aynı kamerada yeniden yerleşen yazılar
      çizilmiyordu.
    - Düzeltme: yerleşim sayacı ve hareket bayrağı anahtara katıldı.
    - Ölçü: durmuş görünümde fark 4/4 → 0/4. DXF, NCZ ve DWG'de 174 karenin hiçbiri farklı değil.
  - **Kalan:** tekerlek dönerken yazıların eski yerleşimi (0597, tasarım gereği; sivas.kcad'de 32 kare). Yazılar GPU'ya
    geçince (RM-13) kalkar.
- [x] **RM-03 — Tek kapsam kuralı.** ([ADR 9203](../docs/adr/9203-show-everything-without-far-objects.md))
  - “Tümünü göster” ile saklı görünümü olmayan bir çizimin açılışı aynı kutuyu kullanır (`extents_box`).
  - Kapsam denetiminin kuralıyla çizimin geri kalanından çok uzakta kalan nesneler kutuya girmez ve sayıları söylenir:
    “… görünüme alınmadı. Kapsam denetimi onları seçer.”
  - Bugüne kadar büyük NCZ'yi 8 800 km'ye, DWG'yi 0,0'a kadar açan nesneler artık görünümü bozmuyor.
- [x] **RM-04 — Gezinme kartı.** ([ADR 9204](../docs/adr/9204-hover-card-follows-the-view.md))
  - Görünüm imlecin altında kımıldayınca (tekerlek, kaydırma, “Tümünü göster”) vurgu ve kart gider.
  - İmleç kıpırdayınca her zamanki gibi geri gelir.

### F1 — Tuval arayüzden ayrılır

- [x] **RM-05 — Kamera ve jestler tuvale geçer.** ([ADR 9205](../docs/adr/9205-live-camera-in-the-canvas.md),
  [ölçü](../docs/perf/files-desktop-rm05-2026-11-23.md))
  - Tekerlek ve orta tuşla kaydırma, çizim alanının canlı kamerasını widget'ın içinde değiştirir ve yalnız kare ister;
    uygulamanın `update`'i ve `view`'ı çalışmaz.
  - Görünüm 150 ms durunca uygulamaya tek ileti gider (`ViewChanged`); ağır iş o zaman, jest başına bir kez yapılır.
  - Üst katmanlar canlı görünümü izler (`Follow`):
    - yazılar ve veri katmanlarının işaretleri taşınıp ölçeklenir;
    - ekran boyutlu işaretler, kartlar ve düzenleyiciler görünüm durana kadar gizlenir.
  - Jest sürerken imleç hareketi dünya noktasını canlı kameradan okur; basma jesti önce alır.
  - **Ölçü (UHD 770):**
    - tekerlek karesinde UI 23,6–92,8 ms → 0,2 ms;
    - görünüm 32 karenin 32'sinde → 1'inde;
    - kaydırmada 40 → 0;
    - bütçeyi aşan tekerlek karesi 16–32 → 0–1.
  - **Gerçek pencerede** NCZ'de ilk değişim 1 074 → 330 ms (içeri) ve 260 → 127 ms (dışarı).
  - **Denetçi:** durmuş görünümde dört dosyada fark 0.
  - **Bedel:** resmin son hâline varış uzadı (59–313 → 294–591 ms). Dinlenmedeki ağır kare RM-07 ve RM-08..10'da
    arayüz iş parçacığından çıkacak.
  - **Planda olmayan:**
    - `Message::Viewport` kalkmadı, çünkü imleç ve tıklama araçların iletisi olarak kalıyor;
    - durum çubuğundaki ölçek görünüm durunca değişiyor.
- [x] **RM-06 — Seçim arayüzü yeniden kurmanın bedelini taşımaz.**
  ([ADR 9206](../docs/adr/9206-selection-without-whole-drawing-scans.md))
  - Profil (`perf`, simgeli release, 150 tık): DWG'de tıklamanın görünüm payının çoğu Öznitelikler'in “Bu değer nereden
    geliyor?” sayımındaydı. Sayım, belgenin bütün nesnelerini üç kez tarıyordu.
  - Sayım artık nesnenin katmanında ve tek geçişte (`Document::by_layer`).
  - **Ölçü:** tıklama karesi UI 103,4 → 40,7 ms, görünüm 64,9 → 2,3 ms.
  - Seçim ve gezinme vurgusu zaten GPU parçası; arayüzün geri kalanına `lazy` gerekmedi.
  - **Kalan:** en uzak görünümde isabet sorgusu 37 ms → RM-08.

### F2 — Çizim iş parçacığı

> **Sıra değişti (23 Kasım, ölçüye göre):** F3, F2'den önce yapılır.
>
> - RM-05'ten sonra jest karelerinde arayüz 0,2 ms. Kalan bedel uzak görünümün GPU'su (sivas.kcad 105 ms, NCZ
>   87 ms) ve dinlenme karesinin yeniden üretimi (25–193 ms).
> - wgpu'da kuyruk tek: ayrı bir çizim iş parçacığı GPU'nun sırasını değiştirmez, süresini düşürmez.
> - Yeniden üretimi arayüz iş parçacığından çıkarmak döşeme ve düzeylerin (RM-08..10) kurulduğu arka plan işinin
>   parçasıdır.
> - RM-07, F3'ten sonra ölçülerek yeniden değerlendirilecek.

- [ ] **RM-07 — Ayrı iş parçacığında çizim.**
  - Tuval kendi dokularına üçlü tamponla çizer. Her doku, çizildiği kamerayla damgalanır.
  - Yeni kare hazır değilse son doku, damgadaki kameradan şimdiki kameraya dönüştürülerek gösterilir.
  - Bugünkü beş yeniden kullanma yolu kalkar.

### F3 — Döşeme, ayıklama, LOD

- [ ] **RM-08 — Dünya döşemeleri ve görünüm ayıklaması.** Çizim sırası korunur.
  - En uzak görünümde tıklamanın isabet sorgusu (RM-06'dan) en yakını önce arayacak. Bugün aday kutusu nesnelerin
    sekizde birini geçince bütün nesneler belge sırasıyla geziliyor: 601 619 nesnede 37 ms.
- [ ] **RM-09 — Eğriler görünüme göre çizilir.** Daireler, yaylar ve elipsler GPU'da ya da döşeme başına görünüme göre
  çizilir. Bütün çizime yayılan 1 M kiriş bütçesi kalkar; yakında daire köşeli görünmez.
- [~] **RM-10 — Düzey sadeleştirme.** İlk adım yapıldı:
  [ADR 9207](../docs/adr/9207-far-levels-of-strokes-and-fills.md).
  - Model ekranında düz çizgilerin ve düz dolguların uzak düzeyleri (8/32/128 m ızgara, piksel kümeleme). Hiçbir nesne
    pikselinden çıkmıyor.
  - **sivas.kcad:** çizilen öğe 3,19 M → 1,56 M, tekerlek karesinde GPU 104 → 85 ms.
  - **NCZ:** öğe 4,17 M → 0,90 M, ama süre değişmedi (87 ms).
  - Düzey zorlama ölçümü: GPU süresi çizilen örnekle doğru orantılı (örnek başına ~90 ns). NCZ'nin kalan 0,9 M örneği
    uzak düzey almayan eğri ve uzun çizgi düzeylerinde: binlerce küçük daire en kaba eğri düzeyinde bile kirişlerle
    çiziliyor.
  - **Sıradaki:** uzak düzeyler eğri ve uzun çizgi düzeylerinin ardına da eklenecek (`Lod`'un tavanları genişler).
  - Kalan, aşağıdaki asıl madde:
  - Her düzey ekran toleransıyla sadeleştirilir.
  - 1 pikselden küçük nesne düşmez, bir piksel olarak çizilir.
  - Hazır olmayan döşemenin yerinde ebeveyni gösterilir; ekran boş kalmaz.
- [~] **RM-11 — Örnekli işaret ve simgeler.** **Öne alındı (23 Kasım, sahibin bildirimi):** “SVG çizgileri, stil
  çizgileri çok zoom yapınca testere gibi.”
  - Neden: SVG semboller ve desen karoları 2048×2048'lik tek bir atlasa en çok 512 px'lik bitmap olarak
    rasterleştiriliyor. Sayfa dolunca bu sınır yarıya iniyor. Yakında büyütülen bitmap'in çapraz kenarları
    basamaklanıyor.
  - [x] İlk adım ([ADR 9208](../docs/adr/9208-svg-symbols-as-vectors.md)): model ekranında yere ölçülü SVG işaretleri
    bir kez üçgenleniyor. Simge ekranda 128 pikseli geçince her yerleşimde vektör olarak çiziliyor
    (`kentos_native_style::svg_marks`, `SolidInstanced`); daha küçükken eski bitmap'i. Kenar her yakınlıkta keskin;
    yer, çizim sırası ve maske kutuları işaretinkiyle aynı.
  - [ ] Desen karoları (tarama ve dolgu desenleri) aynı yoldan: karonun SVG'si tanım ağı, dolgu içinde örnekleme.
  - [ ] Yazı işaretleri RM-13'ün glif atlasına (MSDF) geçecek.
  - [ ] Asıl madde: İşaret ve simgeler GPU'da örnekli (instanced) çizilir; yol başına 50 000 işaret sınırı kalkar.
- [ ] **RM-12 — Veri katmanları aynı döşemelere geçer.**
  - PostGIS ve GeoPackage döşeme başına sorgulanır.
  - Sadeleştirme sunucuda yapılır, sonuç önbellekte tutulur.

### F4 — Bütün çizim içeriği motorda

- [ ] **RM-13 — CAD yazıları GPU glif atlasında.** TTF ve SHX yazılar atlastan çizilir; hiçbir çizim yazısı
  çarpışma yüzünden düşmez.
- [ ] **RM-14 — iced katmanları kalkar.** Etiketler, servis etiketleri, arazi, hat, onaylı katman ve işaretler motora
  taşınır.

### F5 — CAD görünümü

- [ ] **RM-15 — Çizgi ve dolgu gösterimi.**
  - Çizgi kalınlığı ekranda: en az 1 piksel, kenar yumuşatmalı.
  - Çizgi tipleri ve taramalar gölgelendiricide.
  - Maskeler.
- [ ] **RM-16 — Görsel uygunluk paketi.**
  - Küçük fikstürler sabit kameralarda çizilip altın görüntülerle karşılaştırılır.
  - Büyük dosyalar yalnız yerelde koşar.

### F6 — NCZ alımı

- [ ] **RM-17 — Alınmayan 18 nesne.** Bugün alınmayanlar ve yapılacak iş:
  - Tam tur yay: daire olarak alınacak.
  - Sıfır yarıçaplı daireler ve geçersiz yaylar.
  - Tanınmayan 0 ve 14 türleri; 8 türü de okunmuyor.
- [ ] **RM-18 — NCZ ↔ DXF denkliği.** Aynı planın NCZ'si (72 MB) ile DXF'i (657 MB) karşılaştırılır:
  - katman başına nesne sayısı;
  - kapsam;
  - yazılar.
- [ ] **RM-19 — Bellek.** Büyük NCZ açılırken en yüksek kullanım 3,8 GB; 1,5 GB'ın altına inecek.

### F7 — DWG ve DXF alımı

- [ ] **RM-20 — Akışlı DXF okuyucu.** Bellek sınırlı kalır; 256 MiB sınırı kalkar.
- [ ] **RM-21 — DWG doğrudan eşleme.** DWG kod çözücüsünün nesneleri doğrudan KentOS nesnelerine çevrilir. Bellekteki
  DXF metin köprüsü (270–463 MB) kalkar.
- [ ] **RM-22 — Alınmayanlar.** SİVAS AVM'de 8 051 öğe alınmıyor:
  - “görünmez” bayrağı doğrulanacak (6 328 LWPOLYLINE, 513 LINE, 437 ATTDEF, 437 ATTRIB);
  - WIPEOUT, IMAGE ve REGION;
  - okunamayan tarama sınırları;
  - boş ATTRIB'ler.
- [ ] **RM-23 — Birim.** Birim yazılmamışsa kapsamdan ve yazı boylarından çıkarılır; emin olunamazsa sorulur.

## Kabul ölçütleri

| Ölçüt | Hedef |
|---|---|
| Kamera değişikliğinden onu gösteren kareye (gerçek GPU, p95) | ≤ 16 ms; sivas.kcad, NCZ, DWG ve 1 M parselde |
| Kamera olayı başına arayüzün yeniden kurulması | 0 |
| Doğruluk denetçisi, jest senaryolarında | Taze çizimden fark yok; yapışma yok |
| Yakında dairenin kirişten sapması | ≤ 0,25 piksel, her çizim boyutunda |
| Görüntüde sessizce düşen nesne | 0 |
| Alımda düşen nesne | Her birinin raporda bir nedeni var |
| Açılış ve “Tümünü göster” | Aynı kutu |
