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
  - [x] Uzak düzeyler eğri ve uzun çizgi düzeylerinin ardına da eklendi (ADR 9207 §6). Tekerlek karesinde GPU
    sivas.kcad'de 84,9 → 69,9 ms, NCZ'de 87,5 → 81,5 ms; çizilen öğe −%11 ve −%8.
  - [x] Kalan öğelerin dağılımı (`KENTOS_DIAG_ITEMS`, ADR 9207 §6). NCZ'nin 831 bin öğesinin 536 bini, sivas.kcad'in
    1,38 M öğesinin 624 bin köşesi uzak düzey **almamış** büyük toplu çizimlerde. Izgara kümelemesi uzun parçaları ve
    alan bırakan üçgenleri azaltamıyor, bellek kuralı düzeyi reddediyor.
  - [x] Uzak düzeyde yollar ızgaradan önce sadeleştiriliyor (Douglas–Peucker, ADR 9207 §7). Tekerlek karesinde GPU
    NCZ'de 81,5 → 74,9 ms, sivas.kcad'de 69,9 → 67,3 ms. Yarım piksel kuralı içinde kazanç küçük.
  - [x] MSAA'nın payı ölçüldü (ADR 9207 §7): 4× MSAA uzak görünümün işinin üçte biri (NCZ 81 → 55 ms, sivas.kcad
    70 → 42 ms kapalıyken). Çizgiler kenarı gölgelendiricide yumuşattığı için MSAA'ya ihtiyaçları yok.
  - [x] Hareket ederken tek örnek (sahibin kararı, ADR 9207 §8). Durulan kare ayarın örnek sayısıyla çizilir. Öbür
    sayının hedefleri yedekte. Release ölçümü (24 Kasım): tekerlek karesi NCZ'de 47–55 ms, sivas.kcad'de 42 ms;
    yazılar GPU'dayken (RM-13) NCZ 40 ms.
  - **Sıradaki:** düzeyleri 2 kat aralıkla sıklaştırmak ve parsel dolgularının uzak düzeyi.
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
  - [x] Tarama dolguları (`hatchFill`, BÖHHBÜY'de 8 simge; 1202 Resmî bina 45°) denetlendi (24 Kasım):
    gölgelendiricide hesaplanıyorlar. Döşeme kökeninden 14 km uzaktaki alanda bile 13 çentikte (~700 px/m) çizginin
    sapması ortalama 0,025 px; testere yok.
  - [ ] Desen dolguları (`patternFill`, BÖHHBÜY'de 30 simge, örneğin 0633 bataklık) atlastaki karo bitmap'iyle
    çiziliyor ve yakında büyütülür. Vektöre geçmek, alanla kırpma ister (maske ya da kalıp).
  - [ ] Yazı işaretleri RM-13'ün glif atlasına (SDF, ADR 9212) geçecek.
  - [ ] Asıl madde: İşaret ve simgeler GPU'da örnekli (instanced) çizilir; yol başına 50 000 işaret sınırı kalkar.
- [ ] **RM-12 — Veri katmanları aynı döşemelere geçer.**
  - PostGIS ve GeoPackage döşeme başına sorgulanır.
  - Sadeleştirme sunucuda yapılır, sonuç önbellekte tutulur.

### F4 — Bütün çizim içeriği motorda

- [x] **RM-13 — CAD yazıları GPU glif atlasında.** TTF ve SHX yazılar atlastan çizilir; hiçbir çizim yazısı
  çarpışma yüzünden düşmez. Sahibin seçimi (24 Kasım). Tasarım: [ADR 9212](../docs/adr/9212-drawing-text-on-the-gpu.md).
  Tek kanallı SDF glif atlası; ekranda 512 px'i geçen glif vektör. Yerleşim CPU'da kalıyor, yalnız çizim GPU'ya
  geçiyor.
  - [x] Aşama 1: atlas ve gölgelendirici (ADR 9212 §5).
    - Alan yapımı 0,24 ms; 22 yüzün Latin ve Türkçe takımı 553 ms, 3 sayfa.
    - Döndürülmüş, haleli yazı CPU'da bütünüyle çizilmiş referanstan ortalama 0,24 / 255 farklı.
    - Piksel birimi yakınlaşınca boyunu koruyor; kaydırma float32 adımının altında da kayıpsız.
  - [x] Aşama 2: nesne yazıları, blok yazıları ve maskesiz çok satırlı yazılar GPU'da (ADR 9212 §6).
    - Glifler kanvasınkiyle aynı (Iced'in şekillendirmesi); yazılar aynı karede sahnenin üstüne çiziliyor.
    - Ev projesinde kanvasla fark piksellerin %0,04–0,35'i, yalnız kenar yumuşatmasında.
    - Kare süreleri toplu ölçümde.
  - [~] Aşama 3: ölçü değerleri, etiketler, eğri boyunca harfler, hücreler, çağrı kutuları.
    - [x] Birinci adım (ADR 9212 §7): GPU'da düz dörtgenler (maske, alt çizgi, dolgu); maskeli yazılar, ölçü değerleri,
      maskeli paragraflar ve tablo hücreleri GPU'da. Denetim çiziminde kanvasla yakın görünümde dörtte birden fazla
      sapan piksel yok.
    - [x] İkinci adım (ADR 9212 §8): etiket motorunun etiketleri, eğri boyunca harfleri, gölgeleri ve dikdörtgen
      zeminleri GPU'da. Yuvarlak zeminli etiket ile çerçeve ve çağrı çizgileri kanvasta (aşama 4).
  - [x] Aşama 4 (ADR 9212 §9): yazıların kanvası boşaldı. Çizgiler, halkalar (üçgenleme ve yumuşatılmış dış çizgi),
    bantlar, Unicode yazılar ve önizleme GPU'da; yazılar canlı kamerayla çiziliyor. Kanvasta yalnız ekran işaretleri
    (RM-14) ve sistem yazı tipine düşen parça kaldı.
- [x] **RM-14 — iced katmanları kalkar.** Etiketler, servis etiketleri, arazi, hat, onaylı katman ve işaretler motora
  taşınır. (ADR 9214)
  - Katmanlar ortak bir boyama yüzeyiyle (`Paint`: kanvas ya da `GpuFrame`) çiziliyor. Sıralı yuvalarla glif
    katmanına gidiyorlar ve canlı kamerayla taşınıyorlar.
  - Ekran işaretleri ekrana çapalı. 0,0 işareti haritaya çapalı.
  - Delikli dolgular ve köşe birleşimleri GPU'da. Arayüz yazı tipleri atlasta; tanınmayan yazı tipinin harfleri dış
    hatlarıyla dolduruluyor.
  - Önce/sonra gerçek pencerede: farklar harf kenarlarında.
  - Kanvasta kalanlar: tutamaçlar, kenet işaretleri, artı imleç, araç önizlemesi ve kartlar. Hareket boyunca gizliler;
    çizim iş parçacığıyla (RM-07) ele alınacak.

### F5 — CAD görünümü

- [ ] **RM-15 — Çizgi ve dolgu gösterimi.**
  - Çizgi kalınlığı ekranda: en az 1 piksel, kenar yumuşatmalı.
  - Çizgi tipleri ve taramalar gölgelendiricide.
  - Maskeler.
- [ ] **RM-16 — Görsel uygunluk paketi.**
  - Küçük fikstürler sabit kameralarda çizilip altın görüntülerle karşılaştırılır.
  - Büyük dosyalar yalnız yerelde koşar.

### F6 — NCZ alımı

- [x] **RM-17 — Alınmayan 18 nesne** ([ADR 9209](../docs/adr/9209-ncz-records-not-taken.md)).
  - Uçları çakışan tam tur yay artık daire. Dosyadaki “tam tur” yayı aslında açıklığı boş bir yaymış.
  - Sıfır yarıçaplı daireler ve geçersiz yaylar ayrı nedenleri ve ilklerinin yeriyle raporlanıyor.
  - 0 ve 14 türlerinden üç kayıt boş çıktı; “Bilgi” olarak sayılıyor. İçi dolu üç 0 türü kaydı hiçbir okuyucu
    tanımıyor.
  - “Alınmadı” 18 → 15.
- [x] **RM-18 — NCZ ↔ DXF denkliği** ([ADR 9216](../docs/adr/9216-ncz-and-its-dxf-compared.md)). Aynı planın NCZ'si
  (72 MB) ile DXF'i (657 MB) katman katman, yazı yazı ve hücre hücre karşılaştırıldı.
  - Parsel sınırları ve KADASTRO yazıları aynı (5,6 milyon hücrede 2'ye karşı 26 hücre fark; 52 018 yazı bire bir).
  - NCZ okuyucusu Netcad'in DXF aktarımından fazlasını alıyor: yapılaşma koşulu sembolleri (40 211 yazı), @ROL.
    DXF'te 148 yazı 0,0'da.
  - [x] Parsel numaraları ([ADR 9217](../docs/adr/9217-ncz-parcel-numbers-from-labels.md), sahibin kararı: yalnız
    eksiklere).
    - Durum: 2 396 parselin numarası yalnız etiketindeydi.
    - Yazılan: ağırlık merkezi tek parselde kalan 1 117'sine, Netcad'in kuralıyla (ağırlık merkezine ortalı, 3 m).
      1 106'sı Netcad'in etiketiyle aynı yerde ve aynı metinde, farklı numara yok.
    - Atlanan: üst üste parsellerdeki 1 199'u. Orada Netcad başka bir alandan numara yazıyor, o alan okunamıyor.
  - [x] Netcad sembol kodları lejant işaretiyle ([ADR 9218](../docs/adr/9218-ncz-symbols-as-legend-marks.md), sahibin
    kararı: yönetmelik lejantından).
    - Stil çekirdeği: noktaya verilen alan sembolü ağırlık merkezi işaretini noktada çiziyor (önce hiçbir şey).
    - NCZ okuyucusu 47 kodu MPYY uygulama imar planı sembollerine eşliyor: 4 533 nokta lejant işaretiyle.
    - Bilerek dışarıda: S101 (parklar; Netcad boş çiziyor), özel tesislerin “Ö” işaretleri (lejantta yok).
  - [x] Bindirme katmanları MPYY lejantının sembolüyle ([ADR 9219](../docs/adr/9219-ncz-overlays-with-legend-symbols.md)).
    Sahibin kararları: stil kayıtları çözülsün; bindirmeler MPYY sembolüyle, plan alanları Netcad'in renginde;
    bindirme alanları yarı saydam.
    - Katmanın stil kaydından çizgi tipi ve sembol numarası okunuyor; 13 alan ve 11 çizgi sembolü eşlendi.
    - Sivas planında 69 142 nesne: 471 yarı saydam alan (kuşaklar, sitler, tescilli yapılar ve parseller), 68 671 çizgi
      (plan onama sınırı dahil).
    - Nesnenin saydamlığı artık kendi sembolünü de saydam çiziyor (stil çekirdeği, masaüstü).
    - Saydamlık anahtarı kapalıyken (sahibin ayarı) kuşaklar opak çizilir (ADR 0195).
  - [x] KA_KORUNMA_ALANI (sınır boyunca iri noktalar, çizgi tipi 31): sahibin kararı (24 Kasım) düz çizgi kalsın;
    anlamı belli olunca eşlenir. Çizgi tipi 4 (demiryolu kuşağı ile rezerv yapı ortak) ve 22 (jeolojik formasyon)
    bilerek düz.
- [ ] **RM-19 — Bellek.** Büyük NCZ açılırken en yüksek kullanım 3,8 GB; 1,5 GB'ın altına inecek.

### F7 — DWG ve DXF alımı

- [ ] **RM-20 — Akışlı DXF okuyucu.** Bellek sınırlı kalır; 256 MiB sınırı kalkar.
  - Ölçüm (24 Kasım, release, geçici bir örnekle): bugünkü okuyucu 657 MB'lık imar planı DXF'ini (USB) 1 GiB sınırla
    okuyor. 1 milyon nesne sınırında 7,7 s ve en çok 2,1 GB; sınırsız 3 199 462 nesne, 9,4 s ve 5,2 GB.
    Ayrıştırma darboğaz değil. Darboğaz nesne sayısı: NCZ'deki 453 534 nesne, DXF'e aktarılırken parçalanmış.
  - DWG köprüsü 1 GiB'lik DXF metnine izin veriyor, kullanıcının seçtiği DXF ise 256 MiB'te reddediliyor.
  - [x] Sahibin kararı (24 Kasım): ilk 1 milyonla raporlu aç ([ADR 9211](../docs/adr/9211-big-dxf-opens-with-its-first-million.md)).
    Aç'ın sınırı 1 GiB oldu; rapor alınmayanların sayısını veriyor (657 MB'lık DXF: kalan 2 199 464). Sınırdan
    sonraki nesne okuyucunun ziyaret bütçesini artık harcamıyor.
  - [ ] Bölge seçerek açmak (akışlı ve uzamsal süzgeçli): asıl hedef, açık.
- [ ] **RM-21 — DWG doğrudan eşleme.** DWG kod çözücüsünün nesneleri doğrudan KentOS nesnelerine çevrilir. Bellekteki
  DXF metin köprüsü (270–463 MB) kalkar.
  - [x] Ön adım ([ADR 9215](../docs/adr/9215-dwg-bridge-written-faster.md)): okunmayan nesneler atılınca nesne haritası
    küçültülüyor. SİVAS AVM'nin köprü metni 11,3 s yerine 4,9 s'de yazılıyor, metin bayt bayt aynı. Kalan süre
    kodeğin uzantı sözlüğü aramasında.
  - [~] Kodek düzeltmesi yazara önerildi (sahibin kararı): hakanaktt/acadrust#651, PR #652. Yazarın ana dalında aynı
    DWG'nin DXF'i 112,3 s yerine 2,95 s'de yazılıyor. Birleşince sabit sürüm yükseltilecek.
- [~] **RM-22 — Alınmayanlar.** SİVAS AVM'de 8 051 öğe alınmıyor:
  - [x] “Görünmez” bayrağı doğrulandı (24 Kasım), doğru okunuyor:
    - **2 katlı ev:** DWG ve DXF'inde düz nesnelerden hiçbiri görünmez sayılmıyor.
    - **İLBANK İÇMESUYU:** DWG ve DXF birebir aynı (3 472 nesne).
    - **2 katlı evin 20 nesnelik farkı:** DXF'te fazladan iki adsız blok yerleştirmesi var; içleri ölçü kalıntısı
      (12 çizgi, 2 yazı, 6 tanım noktası) ve çizimin uzağında. Kod çözücü 133 ölçünün hepsini okuyor, köprü DXF'i de
      hepsini taşıyor; köprü yalnız 70. gruba 32 bitini ekliyor, tür okumasını etkilemiyor.
    - **AVM'nin görünmezleri:** 6 328 LWPOLYLINE ile 513 LINE “0” katmanında; ATTDEF ve ATTRIB mülk no ile oda alanı
      katmanlarında (A_ANNO_TXT_PRPRTY_NO, A_ANNO_TXT_RM_AREA). Bilinçli gizlenmiş yardımcı nesnelere benziyorlar.
  - [ ] WIPEOUT maskesi: “paper” renginde dolgu olabilir. Ama katman içinde dolgular çizgilerden önce çizildiği için
    çizim sırası kararı ister (RM-15).
  - WIPEOUT, IMAGE ve REGION;
  - okunamayan tarama sınırları;
  - boş ATTRIB'ler.
- [x] **RM-23 — Birim** ([ADR 9210](../docs/adr/9210-cad-unit-from-the-survey-grid.md)).
  - Sayıları Türkiye'nin ölçme ızgarasında duran CAD dosyası, ister milimetre bildirsin ister bildirmesin, Aç'ta
    metre okunuyor ve birim soruluyor (metre seçili).
  - Suşehri DXF'i (milimetre bildiriyordu, 12 m açılıyordu) ve 6 örnek daha artık gerçek boyunda.
  - Mimari çizimler değişmedi. Birim bildirmeyen ve orijinde duran çizim eskisi gibi soruluyor.
  - [x] İkinci adım: yazı boyundan birim ([ADR 9213](../docs/adr/9213-cad-unit-from-letter-heights.md)). Sahibin sözü
    (24 Kasım): “bu proje bu kadar büyütülüyor … ölçü sistemi ile mi alakalı”, “Ama Autocad kusursuz büyütebiliyor”.
    - Milimetre bildiren iki katlı ev projesi 5 cm'lik pafta açılıyor, en derin ölçek (0,1 mm/piksel) paftanın
      tamamında duruyordu.
    - Yazılarının ortancası okunduğu birimde 1 mm'den küçük, metre okununca 2 cm – 2 m olan dosya metre okunuyor ve
      birim soruluyor (metre seçili).
    - 23 örnekten 4'ü (ev ×2, küçük apartman, palazzina) metreye geçti; milimetre projeleri (harfleri 12–30 mm)
      değişmedi.
- [ ] **RM-24 — Derin yakınlaşma** ([ADR 9213](../docs/adr/9213-cad-unit-from-letter-heights.md) §2). AutoCAD gibi:
  yakınlaştırma kesinlik bitene dek sürer.
  - Bugünkü en derin ölçek 10 000 px/m (0,1 mm/piksel), uzun çizgilerin 128 m'lik parçalarının GPU kesinliğine
    bağlı: 100 000 px/m'de ~0,5 px, 1 000 000 px/m'de ~5 px sapma.
  - Yol: parçalar görünüme göre yeniden kökenlenir ya da kesilir (RM-08 ve RM-09'un döşemeleriyle); sonra sınır
    kalkar ya da kesinliğe bağlanır.
  - Tasarım (24 Kasım, kodu okuyarak):
    - Kesinliği bozan, kenarın kameradan uzak ucudur. Hata, ucun kameraya uzaklığı × 2⁻²⁴ kadar.
    - GPU'da çözülemez: uzak uçtan hesaplanan her değer aynı hatayı taşır. WGSL'de iki-float hilesi sürücüye göre
      bozuluyor.
    - Stilli yol uzun kenarları hiç bölmüyor; ham CAD yolu 128 m'de bölüyor.
    - **Derin sahne:** ölçek 5 000 px/m'yi geçince (birimle çarpılır) görünüm kutusuyla (görünüm ±3 genişlik, yardımcı
      çizgilerin `construction_clip`'i) kesişen nesnelerden ayrı bir sahne kurulur. Kenarlar kutunun sınırında ve
      içinde ek noktalarla **bölünür, kesilmez**: yol uzunluğu, kesik evresi, işaret aralığı ve dolgu aynı kalır. Bu
      sahne düzenli sahnenin yerine çizilir; kutu bayatlayınca (görünüm çıktı ya da 20 kat içeri) yeniden kurulur.
    - **Eğriler:** kutuyla kesişen yayın yalnız kutudaki açı aralığı, 0,25 piksellik kirişle parçalanır (RM-09'un ilk
      adımı).
    - **Sınır:** `MAX_SCALE` 10⁴ → 10⁷ px/m. Kutu içinde uçlar en çok yarım kutu ötede, 10⁷'de hata ~4·10⁻⁴ px.
    - Uygulamada bütün katmanlar stil motorundan geçiyor (`cad/style/scene.rs`); ham CAD sahnesi (`build_fixed`,
      `build_curves`) yalnız stil kitaplığı yokken. Derin sahne önce stilli yolda kurulur, ham yol ve seçim vurgusu
      aynı kutuyla ardından gelir. Derin sahne, döşemeler gelince (RM-08) en ince döşeme düzeyi olur.
  - [x] Adım 1, çekirdek: `geom::deep::split_at_box` (kenar kutu sınırında bölünür; noktalar kenarın üstünde) ve stil
    çekirdeğinin `View::deep`'i (yollar ve halkalar bölünür; bloklar açılarak çizilir, uzak düzeyler kapanır).
    Kutuyu kesmeyen yol olduğu gibi kalır. Sınamalar: geometri 6, stil çekirdeği 2.
  - [ ] Adım 2, masaüstü: derin sahne (kutudaki nesneler, ayrı önbellek), sınır 10⁷ px/m, gerçek pencere kanıtı.
  - [ ] Adım 3: yayların yalnız kutudaki açı aralığı 0,25 piksellik kirişle.

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
