# 0935 — KentOS Yol: düşey kurp boyunun görüşten, konfordan ve bisektristen önerilmesi

Tarih: 2026-11-04. plan/road.md Y31 (ROAD-PROFILE-13). Durum: Y31.1–Y31.3 kabul.

## Dayanak

- **AASHTO Green Book (metrik):** durma görüş mesafesi tepki yolu ile fren yolunun toplamıdır:
  `0,278·V·t + V²/(254·(a/9,81 ± G))`.
  - t = 2,5 s, sürücülerin yaklaşık %90'ını kapsar. a = 3,4 m/s² için sürücülerin yaklaşık %90'ı bundan sert yavaşlar.
    İkisi de yüzdeliktir, ortalama değildir.
  - Göz 1,08 m (eski baskılarda 1,07 m), engel 0,60 m yüksekliktedir (1994 öncesi 0,15 m). Tepe için
    `L = A·S²/658` (S ≤ L), 658 = 200(√1,08 + √0,6)².
  - Çukurda far yüksekliği 0,60 m, ışın 1°'dir: `L = A·S²/(120 + 3,5·S)`. Konfor için `L = A·V²/395`.
  - Kaynaklar: [Wikipedia, Stopping sight distance](https://en.wikipedia.org/wiki/Stopping_sight_distance), NYSDOT
    çalışma notları, [FHWA'nın durma görüş mesafesi notu](https://webarchive.library.unt.edu/eot2008/20080916011951/http:/safety.fhwa.dot.gov/geometric/mitigationstrategies/chapter3/3_stopdistance.htm).
    Green Book'un metni çevrimiçi okunamadı.
- **Civil 3D:** ölçüte dayalı tasarımda tasarım hızına göre en küçük K tablosu kullanılır (tepe için durma görüşü, çukur
  için kurp boyu). Uymayan kurp K değiştirilerek ya da boy ve eğim ayarlanarak düzeltilir
  ([yardım](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-FFE978DE-6282-4D40-8C16-7A1D6288D7A1.htm)).
- **Novapoint:** çukurda durma görüşü ile üst engel için ayrı K değerleri, tepede geçme görüşü K'sı vardır. Küçük eğim
  farklarında K'yı alt sınırın epey üstünde tutmayı önerir.
- **NETPRO ve KGM:** bisektris, açık ve kapalı kurp katsayısı, DGM ve GGM ölçütleri kullanılır (ROAD-PROFILE-13'ün dili).
  Tebliğin tabloları bu oturumda çevrimiçi bulunamadı.

## Karar

### §1 Çekirdek (`kentos_road::profile::lengths`, Y31.1)

- **`stopping(V, t, a, G)`:** `v·t + v²/(2(a + g·G))`, v = V/3,6, g = 9,80665 (Y27 ile aynı). AASHTO'nun 254'ü 2·9,81·3,6²'nin
  yuvarlanmışıdır; fark %0,1'dir. Eğim frenin tümünü alıyorsa (`a + g·G ≤ 0`) mesafe yoktur.
- **`crest(A, S, h₁, h₂)`:** parabol teğetinden `A·x²/(200L)` düşer, görüş çizgisi parabolü teğet keser. S ≤ L için
  `L = A·S²/(200(√h₁ + √h₂)²)`, S > L için `L = 2S − 200(√h₁ + √h₂)²/A`. Eğimler görüşü kendi başına veriyorsa sonuç 0'dır:
  kurp istenmez.
- **`headlight(A, S, h, β)`:** S ≤ L için `L = A·S²/(200(h + S·tan β))`, S > L için `L = 2S − 200(h + S·tan β)/A`.
- **`comfort(A, V, a_v)`:** `R = 100L/A` ve `v²/R = a_v` olduğundan `L = A·V²/(1296·a_v)`. AASHTO'nun 395'i
  a_v ≈ 0,3'e karşılık gelir.
- **`bisector(A, E)`:** `L = 800E/A`.
- Hiçbir değer KentOS'tan gelmez: hız, tepki süresi, yavaşlama, yükseklikler, ışın açısı, ivme ve bisektris
  standarttan ya da kullanıcıdan gelir.

**Fizik sınaması:** önerilen boydaki tepede, Y20'nin bağımsız görüş hesabı (`Vertical::sight`) bütün göz yerlerinde en
kısa görüşü ölçer. Sonuç istenen DGM'ye 5 cm içinde eşittir. Örnekler: 80 km/sa ve A %6 için DGM 128,18 m, L 149,81 m,
en kısa görüş 128,18 m. 100 km/sa ve A %3 için DGM 182,92 m, L 146,50 m, en kısa görüş 182,92 m. A %1 ve A %4 (50 km/sa)
için kurp istenmez; eğimler 384,5 m ve 82,3 m görüş verir.

### §2 Komut (`road.profiles.lengths` v1, Y31.2)

Salt okunur sorgudur. Girdi:

- hız;
- durma görüşü için tepki süresi ve yavaşlama; `grades` ile yaklaşma eğimleri. İki yönün uzunu alınır: ileride gelen
  eğim, geride giden eğimin tersi.
- tepe için göz ve engel; `passing` ile geçme görüşü (nesne göz yüksekliğindedir);
- çukur için far ve ışın açısı, konfor ivmesi;
- bisektris ve yuvarlama adımı.

Kuralları:

- Değerlerin hiçbiri KentOS'tan gelmez. Hiç ölçüt verilmezse ya da eksik çift verilirse (göz verilip engel
  verilmezse, tepe ölçütü süre ve yavaşlama olmadan verilirse) `invalid_profile_lengths` ile reddedilir.
- **PVI'ler üzerinden yürür.** Eğri raporu kurpsuz PVI'yi (kırığı) atlıyordu, oysa öneri en çok kırığa gerekir. Eğimler
  ardışık PVI'lerden okunur.
- **Şimdiki boy:** simetrik parabolün boyu, asimetrikte L₁ + L₂, dairede aynı K'li parabol boyu `R·|A|/100`.
- **Cevap:** düz yoldaki DGM ve her PVI için tepe ya da çukur oluşu, A, şimdiki boy, ölçütlerin boyları, belirleyen
  ölçüt, gereken boy, öneri ve "kısa mı". Eksik kalan kurp `vertical_curve_short` ile uyarılır.

21 ortak durum vardır (`road_design_check_cases.py`'ye eklendi). Kırıklı ve asimetrik kurplu yeni bir boykesit (b-4)
eklendi.

**Ölçüm** (ortak çizim, AASHTO'nun örnek değerleri, 80 km/sa, A −%8 tepe ve +%5,71 çukur):

| Ölçüt | DGM | Gereken boy | K |
|---|---|---|---|
| Tepe, durma görüşü | 128,18 m | 199,8 m | 25,0 |
| Çukur, far ışığı | 128,18 m | 165,4 m | 29,0 |

AASHTO'nun tablosunda 80 km/sa için tepe K 26, çukur K 30'dur. Fark tablonun DGM'yi 130 m'ye ve K'yi yukarı
yuvarlamasından gelir; formül aynıdır. 100 km/sa'te tepe 406,8 m ister, 5 m'ye yuvarlanınca 410 m önerilir.

### §3 Boykesit penceresi: Düşey kurp boyu (Y31.3)

- Boykesit'in alt çubuğunda **Kurp boyu öner…** düğmesi vardır. Seçili boykesitin yazıldığı hâliyle (kaydedilmeden)
  yardımcı pencereyi açar; Dever'in Genişletme önerisiyle aynı kalıptır.
- **Solda değerler:** hız, durma görüşü (süre, yavaşlama, eğim anahtarı), tepe (göz, engel, geçme), çukur (far, ışın,
  ivme), öneri (bisektris, yuvarlama). Hepsi boş açılır; KentOS sayı vermez. Geri ile kapanınca değerler program
  açık kaldıkça tutulur; ikinci açılışta yeniden yazılmaz.
- **Sağda:** her PVI için şimdiki boy çubuk, gereken boy çizgi. Şimdiki boy çubuğun solunda, gereken boy çizginin
  sağında yazılır; ilk görüntüde ikisi yan yana düşünce üst üste biniyordu (v1: 40,0 ve 44,2 m). Altında özetler
  (kurp, kısa, DGM, en uzun gereken) ve tablo vardır: PVI, tür, A, şimdi, belirleyen, gereken, öneri.
- **Kısaları uzat:** yalnız kısa kurplara dokunur ve türünü korur. Kırığa parabol konur, parabolün boyu yazılır.
  Dairede aynı boyu veren yarıçap yazılır (`100·L/|A|`). Asimetrikte iki kol oranıyla büyütülür. Boylar milimetrede
  yukarı yuvarlanır; böylece öneri kıl payı kısa kalmaz. Sonra Boykesit'e dönülür, Kaydet yazar.
- **Sığmayan öneri.** İlk denemede iki komşu PVI 140 m aralıktaydı ve önerilen boylar (260 ve 220 m) 240 m kol
  istedi. Kısaları uzat boykesiti kurulamaz bıraktı ("kolları 99,023 m uzun"). Artık her aralıkta uzatılan kollar
  kalan yeri oranlarıyla paylaşır, hiçbiri eski boyunun altına inmez. Boykesit kurulur ve PVI'ların izin verdiği kadar
  öneriye yaklaşır; ölçümde 151,666 m parabol ve R 1695,247 m çıktı, toplam kol tam 140 m. Pencere bunu basmadan önce
  söyler: tabloda "260,0 sığmaz" yazar, ayrıca bir uyarı vardır. Basınca da sığmayanların adı uyarı olarak verilir.
  Civil 3D bu durumda yalnızca çakışmayı uyarır; çözümü kullanıcıya bırakır.
- Dar pencerede (1100 px) öneri sütunu kesiliyordu. Form 360 px'e, sütunlar 508 px'e indirildi.

Bekçi: `profile_lengths_window_tests` (3).

## Doğrulama

33 ortak durum vardır (`scripts/fixtures/road_vertical_length_cases.py`, 30 basamak). Referans kapalı formülleri
kullanmaz; her ölçütü ileri yönde yazar ve L'yi çözer:

- Tepe: her göz yerinden görüş çizgisinin yola teğet olduğu nokta ve nesnenin çizginin altına indiği yer ikiye bölmeyle
  bulunur. En kötü göz yeri altın oranla, L de ikiye bölmeyle aranır.
- Çukur: ışının yolu kestiği yer bulunur.
- Durma, konfor ve bisektris ileri formülleriyle çözülür.

Referansın ilk sürümü nesnenin göründüğü yanı ters ikiye böldü ve 95,5 m yerine 57,4 m buldu. Y20'nin görüş hesabı
çekirdeği doğruladığı için hata referansta arandı.

Bekçi: `tests/all/vertical_lengths.rs`.
