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
- [ ] **RM-02 — Doğruluk denetçisi.**
  - Sınama kipinde, ekrana konan her kare aynı kamerayla taze çizilen kareyle piksel piksel karşılaştırılır.
  - Fark çıkarsa hangi yeniden kullanma yolunun ve hangi anahtarın yol açtığı yazılır.
  - “Ekrana yapışma” jest senaryolarıyla yeniden üretilir ve kökü bulunur.
- [ ] **RM-03 — Tek kapsam kuralı.**
  - Bugün “Tümünü göster” büyük NCZ'yi ve DWG'yi ekranda bir noktaya indiriyor, çünkü 0,0'ı da içine alıyor.
  - Açılıştaki sığdırma başka bir kutu kullanıyor.
  - Sıfır noktasındaki nesne bulunacak, iki yol tek kurala inecek.
- [ ] **RM-04 — Gezinme kartı.** Kart yakınlaştırma boyunca ekranda sabit kalıyor ve artık imlecin altında olmayan
  nesneyi gösteriyor. Kaydırma ve yakınlaştırmada gizlenecek ya da yenilenecek.

### F1 — Tuval arayüzden ayrılır

- [ ] **RM-05 — Kamera ve jestler tuvale geçer.**
  - `Message::Viewport` kalkar.
  - Durum çubuğundaki koordinat kendi küçük bileşeninden okunur.
  - Görünüm geçmişi, Büyüteç ve bölünmüş görünüm uyarlanır.
- [ ] **RM-06 — Seçim ve gezinme vurgusu motora geçer.** Seçim değişince yalnız ilgili panel değişir.

### F2 — Çizim iş parçacığı

- [ ] **RM-07 — Ayrı iş parçacığında çizim.**
  - Tuval kendi dokularına üçlü tamponla çizer. Her doku, çizildiği kamerayla damgalanır.
  - Yeni kare hazır değilse son doku, damgadaki kameradan şimdiki kameraya dönüştürülerek gösterilir.
  - Bugünkü beş yeniden kullanma yolu kalkar.

### F3 — Döşeme, ayıklama, LOD

- [ ] **RM-08 — Dünya döşemeleri ve görünüm ayıklaması.** Çizim sırası korunur.
- [ ] **RM-09 — Eğriler görünüme göre çizilir.** Daireler, yaylar ve elipsler GPU'da ya da döşeme başına görünüme göre
  çizilir. Bütün çizime yayılan 1 M kiriş bütçesi kalkar; yakında daire köşeli görünmez.
- [ ] **RM-10 — Düzey sadeleştirme.**
  - Her düzey ekran toleransıyla sadeleştirilir.
  - 1 pikselden küçük nesne düşmez, bir piksel olarak çizilir.
  - Hazır olmayan döşemenin yerinde ebeveyni gösterilir; ekran boş kalmaz.
- [ ] **RM-11 — Örnekli işaret ve simgeler.** İşaret ve simgeler GPU'da örnekli (instanced) çizilir; yol başına 50 000
  işaret sınırı kalkar.
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
