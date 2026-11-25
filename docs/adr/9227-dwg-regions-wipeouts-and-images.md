# ADR 9227: DWG'nin bölgeleri delikleriyle alan, maskeleri maske, görüntüleri çerçeve olur

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-22, 1. adım.
- **İstek:**
  - planın RM-22 maddesi: “WIPEOUT, IMAGE, REGION, tarama sınırları, boş ATTRIB ve vekil grafikler ele alınır”;
  - sahibin 23 Kasım isteği: “NCZ ve DWG alımı kusursuz olsun, veri kaybolmasın”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. Bölge alan (`polygon`) ve delikleri, maske alanın görünüşü
  ([ADR 0310](0310-draw-order-and-wipeout.md)), görüntü kapalı çoklu çizgi olarak gelir; `.kcad`, komut ve ayar aynı.

## §1 Bulgu

SİVAS AVM DWG'sinin (32 MB, 601 619 nesne) açılış raporunda 8 051 öğe “alınmadı” diyordu. Görünmez işaretlilerin
doğru okunduğu 24 Kasım'da doğrulandı (plan, RM-22). Geri kalanlar:

| Öğe | Sayı | Neden alınmıyordu |
|---|---|---|
| REGION | 33 | Şekli ACIS verisinde. DXF köprüsü bu veriyi yalnız veri bölümüne (ACDSDATA) yazıyor, DXF okuyucusu oraya bakmıyor: “3B katı, yüzey ve bölge nesneleri alınmaz”. |
| WIPEOUT | 4 | KentOS'ta maske var (ADR 0310), ama DXF okuyucusu WIPEOUT'u desteklenmeyenler listesinde tutuyordu. |
| IMAGE | 3 | Dosyaları çizimin dışında (`..\2015.05.27\İmar Durum Belgesi.jpg` gibi). Ne görüntü ne yeri geliyordu. |
| Boş ATTRIB | 196 | Değeri boş; çizilecek yazısı yok, ama “alınmadı” sayılıyordu. |

**AVM'nin 33 bölgesi alan paftaları:** brüt alan (A_ANNO_AREA_GROS), bina alanı (A_ANNO_AREA_BULDBL) ve net alan
(A_ANNO_AREA_NET). Her katın alanı, kolonları ve boşlukları delik olarak, bir bölgedir. Camide (`cami.dwg`) 20 bölge
var, kenarları spline.

**Kod çözücünün bir kusuru:** acadrust'ın `SatTransform`'u ikili ACIS'te (SAB) dönüşümün sayılarını bir sıra kaydırarak
okuyor. Bölgeler düzleşip bir çizgiye iniyordu. SAB'da koordinat üçlüleri ayrı belirteç (0x13/0x14) ve kayıtta önde
işaretçi yok; metin ACIS'te (SAT) ise önde bir işaretçi var.

## §2 Karar

### §2.1 Bölge, sınırıyla alan olur

Köprü, DXF metnini yazmadan önce her REGION'ı (`crates/native/dwg/src/regions.rs`, `outlines`) kapalı LWPOLYLINE'lara
çevirir:

- **Her düzlemsel yüz bir alan.** Dış döngüsü REGION'ın yerini alır: aynı tanıtıcı, katman, renk ve sahip.
- **Öbür döngüler deliktir.** Her biri bir çoklu çizgi daha, KentOS'un kendi genişletilmiş verisiyle (`hole`, KentOS'un
  DXF yazıcısının alan deliklerini işaretlediği gibi) o alanın deliği olarak işaretlenir. Okuyucu ikisini tek alan yapar.
- **Kenarlar:**
  - doğru düz kalır;
  - daire yayı tümseğiyle (bulge) gelir, tam daire dört çeyrek yay;
  - elips, yarı büyük ekseninin binde biri içinde noktalarla;
  - spline (ACIS `intcurve`), kontrol noktası başına 8 noktayla, de Boor ile;
  - başka bir eğri kirişiyle.

  Noktayla ve kirişle çizilenler sayılır, raporda söylenir.
- **Dönüşüm:** `transform` kaydı, dünya = ölçek · (p · M) + T; M'nin satırları birim vektörlerin görüntüleri. Sayılar
  sırayla okunur: önde işaretçi varsa geçilir, üçlüler açılır. Böylece SAT de SAB de aynı 13 sayıyı verir.
- **Alınmayan:** alanı olmayan döngü atlanır ve sayılır (“düz”); yatay bir düzlemde durmayan bölge “bölge yatay bir
  düzlemde değil” nedeniyle alınmaz.

**Rapor:** “Bölge (REGION) · 33 · sınırlarıyla kapalı alan olarak alındı (bölge kenarlarıyla çizilir); 264 iç sınır
alanın deliği oldu; 14 elips ve spline kenarı üzerindeki noktalarla”.

### §2.2 Delikler blok içinde de birleşir

Caminin bölgeleri bir bloğun içinde. Delik işaretli halkaları alana katan adım (`merge_holes`) yalnız çizimin kendi
nesnelerine bakıyordu. Artık her blok yerleştirmesinin hücresi kendi tanıtıcılarıyla birleşir (`merge_holes_from`):
bir halka yalnız aynı hücredeki sahibine katılır.

### §2.3 WIPEOUT maske olur

Maske, alanın görünüşüdür (ADR 0310): zeminin rengiyle opak dolar, kendinden önce çizileni örter.

- **Sınır:** maskenin kırpma sınırı görüntü uzayındadır. Nokta (x, y) dünyada `ekleme + (x + ½)·U + (H − y − ½)·V`'dir;
  maskede H = 1.
  - Kırpılmamış maske (280 = 0) bütün dikdörtgendir.
  - İki noktalı sınır dikdörtgenin iki köşesidir.
- **Çerçeve:** çizimin WIPEOUTVARIABLES'ının 70. grubu söyler. Yoksa çerçeve görünür (biçimin varsayılanı).
- **Rapor:** “Maske (WIPEOUT) · 4 · çizimin zemin rengiyle dolan maske alanı olarak alındı; kendinden önce çizileni
  örter”.

Planın açık sorusu (“katmanda dolgular çizgilerden önce çiziliyor, çizim sırası kararı ister”) ADR 0310 §2'nin
bantlarıyla zaten çözülmüş: maskeli katmanda her maske kendi bandıdır, kendinden önceki çizgileri de örter.

### §2.4 IMAGE, yerini gösteren çerçeve olur

Raster görüntünün dosyası çizimin içinde değil. Görüntü yerine sınırı gelir:

- **Çerçeve:** kırpma sınırı (kırpılmamışsa bütün dikdörtgen), görüntü uzayından dünyaya maskedeki gibi çevrilir, kapalı
  çoklu çizgi olur.
- **Rapor dosyayı adlandırır** (IMAGEDEF'in 1. grubu): “Görüntü (IMAGE) · “..\2015.05.27\İmar Durum Belgesi.jpg”
  dosyası çizimin içinde değil; görüntünün yerini gösteren çerçevesi alındı. Dosyayı Altlık ile ekleyin”.
- **Adı olmayan** görüntü alınmaz, nedeni söylenir: “raster görüntünün dosyası adlandırılmamış; görüntü alınmadı”.
  Kod çözücünün kendi yazdığı varsayılanlarla dolan, boyu ve dosyası olmayan bir IMAGE de böyle düşer.

### §2.5 Boş ATTRIB bir bilgi notudur

Değeri boş öznitelik “alınmadı” sayılmaz: “Blok özniteliği (ATTRIB) · değeri boş; çizilecek yazısı yok”.

## §3 Kanıt

**SİVAS AVM DWG** (son kod, açılış raporu):

- “Alınmadı” 8 051 → **7 815**. Kalanların hepsinin nedeni doğru:
  - görünmez işaretli: 6 328 LWPOLYLINE, 513 LINE, 437 + 57 ATTDEF, 437 + 38 ATTRIB;
  - sınırı okunamayan ya da alanı olmayan 3 tarama;
  - iki köşesi olmayan 2 çoklu çizgi.
- **33 bölge → alan, 264 delik:** A_ANNO_AREA_GROS 192, A_ANNO_AREA_BULDBL 44, A_ANNO_AREA_NET 28. 14 eğri kenar noktalarla.
- **4 maske**, **3 görüntü çerçevesi** dosya adlarıyla, **196 boş öznitelik** notta.

**Cami DWG:** 20 bölge alan oldu. Spline kenarlarının 20'si önce kirişle geliyordu, şimdi noktalarla.

**Sınamalar:**

- `kentos-dwg` `regions`:
  - `a_region_s_loops_are_its_rings_moved_by_its_transform`: SAT üreteci, kaydırılmış ve delikli bölge;
  - `a_circle_hole_is_four_quarter_arcs_turning_its_own_way`;
  - `fewer_than_three_points_on_arcs_are_given_a_middle`;
  - `a_region_becomes_its_outer_polyline_and_a_hole_marked_as_its`: köprünün DXF'i KentOS'un okuyucusuyla okunur, 1
    alan ve 4 tümsekli bir delik.

  Bütün sınamalar geçti (7 + 7).
- `kentos-formats`:
  - `a_wipeout_is_a_mask_where_its_clip_says`: AVM'nin ızgara kutusu gibi döndürülmüş ve milyonlarca birim uzunluğunda
    U, V; kırpılmamış ve iki noktalı sınır; WIPEOUTVARIABLES'sız çerçeve;
  - `an_image_is_its_frame_and_its_file_is_named`.

  Bütün sınamalar geçti (221 + 140 + 63 + 6).

**Gerçek pencere** (`.run/gui-evidence/render/rm22/`, sahibin ayarlarının kopyası):

- AVM, alan paftalarının üst sırası. Brüt ve net alan katmanları Katmanlar panelinden açıldı; o katmanlar dosyada kapalı.
- `once-sonra-avm.png`:
  - **önce** yalnız taramalar;
  - **sonra** bölgelerin sınırları ve kolonların, boşlukların delikleri.

## §4 Bu işte görülen, ayrı işler

- **Taramada üçgen boşluklar.** Aynı resimde, iki derlemede de, çok delikli taramalarda büyük üçgen boşluklar var;
  bölgenin sınırı taramanın neresinin eksik kaldığını gösteriyor. RM-22'nin 2. adımı.
- **“486 531 DWG nesnesi okunmadı”** (DGN çizgi stili bileşenleri ve vekil nesneler). Görünen bir şey kaybolup
  kaybolmadığı ölçülecek. RM-22'nin 3. adımı.
- acadrust'ın `SatTransform`'unun SAB kayması kod çözücüye bildirilecek (RM-21'in acadrust işleriyle).

## Uygulama

- `crates/native/dwg/src/regions.rs`: yeni; `outlines`, `Regions`, ACIS yüzlerinden halkalar, dönüşüm, sınamalar.
- `crates/native/dwg/src/lib.rs`: köprü bölgeleri yazar; rapor notları.
- `crates/shared/formats/src/dxf/emit.rs`: `merge_holes_from`, blok hücresinin kapsamı; WIPEOUT maskesi; IMAGE çerçevesi
  ve notları; boş ATTRIB notu.
- `crates/shared/formats/src/dxf/entity.rs`: `Kind::Wipeout`, `Kind::Image`, sınırın görüntü uzayından dünyaya çevrilmesi.
- `crates/shared/formats/src/dxf/ordering.rs`: `Objects`: WIPEOUTVARIABLES ve IMAGEDEF dosya adları.
- `crates/shared/formats/src/dxf/mod.rs`: maskenin çerçevesi, görüntülerin dosyaları.
- `crates/shared/formats/tests/all/dxf.rs`: sınamalar.
