# ADR 8051: Büyük projede akıcılık — mahal çizelgesi, döşeme dizini, deneme adımı

- **Durum:** yapıldı (12 Kasım 2026); `plan/architecture-advanced.md` ADV-07, M51 ("163 → < 100 ms"): 100 daire ve 200
  çizelgede tasarım değişikliğinin karesi 562 → 85 ms, Kapı önizlemesi 450 → 1,9 ms.
- **Bağlam belgesi:** ADR 0776 (100 daire ölçümü), ADR 0782 (adım yalnız ulaştığını okur; aynı çizelgeler bir kez),
  ADR 0786 (yazı katmanı nesne nesne; kare 163 ms), ADR 8040 §1–3 (duvarın ve mahalin yüksekliği), ADR 8041 §1 (kiriş),
  ADR 0770 §3 (Mesafe'nin etkileri).
- **`.kcad`:** şema değişmez. Komutların girdisi ve cevabı değişmez; önizlemenin izleyenleri artık çizelge tablosu
  taşımaz (§3).
- **Makine:** Intel Core i5-13500 (20 iş parçacığı), 24 GB, Linux 7.0; ölçümler release. ADR 0782 ve 0786'nın
  sayıları Apple M5'teydi; karşılaştırma bu makinede önce/sonra yapıldı.
- **Plan dışı:** planın adı koyduğu "etiket motorunun artımlı yerleşimi" yapılmadı: ölçüldü, karenin 3 ms'si (§Ölçümler).
  Onun yerine sahnenin parçaları kısaldı (§4). Önizlemelerin ve komut planlarının izleyenleri artık çizelge tablosu
  taşımaz (§3). `flat_timing` çizimi dosyaya yazabilir (`KENTOS_PERF_SAVE`); gerçek pencere sürücüsü fareyi bir doğru
  boyunca gezdirir, uygulama boşa çıkana dek bekler ve işlemci süresini ölçer (`glide`, `settle`, `cpu`).

## Bağlam

M51'in başında aynı ölçüm (`flat_timing`, `KENTOS_PERF_SCHEDULES=each`: 100 daire, 4 400 nesne, 200 çizelge) Mesafe'yi
**479 ms**, Planı 530 ms, Kaydır'ı 488 ms ölçtü; ADR 0786'da (M5) 77,9 / 98,2 / 78,0 ms'ydi. Masaüstünde tasarım
değişikliğinin karesi 562 ms (komut 479), Kapı'nın önizlemesi fare her kıpırdadığında 450 ms'ydi (ADR 0786'da 1,25 ms).

Adımın izleme aşamaları tek tek ölçüldü (`follow_into`'nun 24 aşaması, geçici sayaçlarla): çizelgeler (`follow_tables`)
adım başına ~215 ms, geri kalan her aşama 10 ms'nin altında. ADR 0782 §2'nin tekilleştirmesi çalışıyordu — adımda yalnız
iki çizelge çıkarılıyordu — ama **mahal çizelgesinin çıkarılması tek başına 197 ms**'ydi (500 mahal; kapı-pencere
çizelgesi 1 000 satırla 3,7 ms). Mesafe'de adım iki kez koştuğundan (etkiler için okuma kopyasında, ADR 0770 §3; sonra
gerçekten) iki kez ödeniyordu.

Sebep M40'ın yükseklik kuralı (ADR 8040 §3, `9009cd783`): `arch_value` bir mahal için hangi sütun sorulursa sorulsun
(numara, ad, kullanım…) önce mahalin yüksekliğini hesaplıyordu; yükseklik altındaki ve üstündeki döşemeyi bulmak için
çizimin **bütün nesnelerine** bakıyordu (`slabs_holding`, iki kez). 500 mahal × 7 sütun × 2 tarama × 4 400 nesne =
30,8 milyon bakış; mahal sayısıyla ve çizimin büyüklüğüyle birlikte büyüdüğünden karesel. Duvarın yüksekliği (ADR 8040
§1) ve kirişin üstü (ADR 8041 §1) de aynı taramayı kullanıyordu.

Başka programlar:

- **Archicad**'de büyük projede mahal (zone) çizelgelerinin yavaşlığı bilinen bir sorun; Graphisoft'un Archicad 23
  güncellemeleri listesinde "zone bilgisi taşıyan çizelgelerin yavaş hesaplandığı" bir durumun düzeltildiği yazıyor
  (<https://graphisoft.com/downloads/archicad/updates/AC23/ImprovementsList>, arama dizininden; sayfa 12 Kasım 2026'da
  açılmadı). Graphisoft'un "Troubleshooting slow schedule performance" destek makalesi yavaş çizelge için ölçütü
  daraltmayı (öğe türünü "bütün türler"de bırakmamayı, katlara bölmeyi) önerir: aksi hâlde Archicad modelin her öğesine
  bakar (<https://support.graphisoft.com/hc/en-us/articles/24521190308113>; sayfa erişime kapalıydı, arama özetinden).
- **Revit** modeli yeniden ürettiğinde bunu açık olan her görünümde yapar; önerilen, kullanılmayan görünümleri kapatmak
  (Close Inactive). Yani iki program da yavaşlığı kullanıcıya kapsamı daraltarak çözdürüyor.

Bizde kullanıcıdan çizelgeyi kapatması ya da "elle güncelle"ye alması istenmez: Mevzuat bunu da istemez.

- **Planlı Alanlar İmar Yönetmeliği 57(29)** (Ek: RG 25.2.2022/31761): mahal listesi projenin parçasıdır, her mekân
  kendi koduyla; liste mahallerden okunur (ADR 8038).
- **Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik** (RG 5.8.2026/33331) **Md.
  9**: "BIM modeli ile CAD çizim dosyası PDF/A nüshasının birbiriyle uyumlu olması zorunludur". Eski kalmış bir çizelge
  modelle uyuşmaz; bu yüzden çizelgeler her adımda yazılmaya devam eder, yalnız hızlı yazılır.

## Karar

### 1. Değer, onu soran yer için hesaplanır

`Document::arch_value` (etiketin ve çizelgenin alanı) bir mahalin alanını ve yüksekliğini, bir açıklığın duvarını, bir
duvarın yüz alanını yalnız onları okuyan alan sorulduğunda hesaplar (kapanış ile, sorulunca). Değerler aynı; mahal
çizelgesinin "Yükseklik" sütunu yüksekliği yine hesaplar, "Ad" sütunu hesaplamaz. Açıklığın her sütunu için duvarın bütün
alanlarının kopyalanması da kalktı.

### 2. Döşeme ve kiriş dizini

`kentos_domain::structure_index` (`Store::structure`): çizimin döşemeleri ve kirişleri belge sırasıyla, her birinin
halkaları (kirişin bandı) ve kutusuyla. Duvar adaylarının dizini (ADR 0782 sonrası `wall_candidates`) gibi türetilir:
kaydedilmez, ilk sorulduğunda kurulur, bir döşeme ya da kiriş yazıldığında ya da silindiğinde düşer; okuma kopyaları onu
paylaşır, önizleme geri alındığında eskisi döner. Kararı vermez: mahalin tabanı ve tavanı, duvarın ayağı ve üstü, kirişin
üstü eskisi gibi her döşemeyi ve kirişi deponun tuttuğu hâliyle okuyup kendi kuralını uygular (`slabs_at`, `beams_at`).

Kutunun dışındaki nokta hiçbir halkanın içinde değildir: `point_in_polygon` tam yönelim ile tek-çift sayar, noktanın
sağındaki kesişim sayısı kutunun dışında sıfır ya da çifttir. Kutu yalnız sınamayı atlatır; sayı olmayan bir koordinat
eskisi gibi halkanın sınamasına gider.

### 3. Deneme adımı ve önizleme çizelge yazmaz

- **Mesafe'nin etkileri** (ADR 0770 §3) okuma kopyasında adımı koşar; etkiler duvarların, mahallerin, açıklıkların,
  ölçülerin ve donatıların değişimidir. Kopya artık `Document::without_tables` ile açılır: çizelgeler o kopyada yazılmaz,
  çünkü kopya ne kaydedilir ne gösterilir. Etkilerin karşılaştırması (`differs`, nesnenin JSON'u) da yalnız bu türlere
  yapılır; önce 100 çizelgenin her biri JSON'a çevrilip karşılaştırılıyor, sonra sonuç zaten atlanıyordu.
- **Önizlemenin izleyenleri** (`Document::preview_followers`, Kapı'nın, taşımanın, tutamağın, Uzat'ın hayaleti)
  çizelgeleri hesaplamaz. Kapı duvar boyunca gezdirilirken 100 kapı-pencere çizelgesi (her biri 1 000 satır) her
  kıpırtıda yeniden çıkarılıyor, kopyalanıyor ve ızgarası kesik çizgiyle hayalete çiziliyordu. Önizlemenin gösterdiği
  izleyendir (ölçü dizisi, etiket, mahal); çizelge adım yazılınca yazılır. Ortak komut durumlarının hiçbirinde izleyenler
  arasında tablo yoktu.

### 4. Çizim alanının sahnesi: kısa parçalar, yalnız yazısı değişen tablo

Komut hızlanınca karenin en büyük payı çizim alanının sahnesi oldu (`viewport.view`, 37–39 ms; boyama 17,5 ms).
Stil motoru bir katmanı ADR 0121'in parçalarıyla kurar, ama yalnız 8 192 nesneden büyük katmanı: 4 400 nesnelik kat
tek parçaydı ve bir duvarın değişmesi 200 çizelgenin ızgarasıyla birlikte bütün katı yeniden stilliyordu. Binanın yalnız
iki çizelgesiyle aynı ölçüm sahneyi 7,5 ms ölçtü: farkın tamamı tabloların.

- **Kısa parçalar** (`apps/desktop/src/style/scene.rs`, `part_run`): 1 024 nesneden büyük ve nesneleri belgede en çok
  16 384 yerlik aralıkta duran katman 512'lik yer aralıklarında kurulur (en çok 32 parça); yarısının altına inince ya da
  aralık iki katını geçince yeniden bütün. Büyük katman ADR 0121'deki gibi 4 096. Belgede dağınık küçük katman (başka
  katmanların nesneleri arasına serpilmiş) bütün kalır: 512'lik aralıklar onu yüzlerce küçük parçaya bölerdi. Parçalar
  ADR 0121'in kuralıyla bütün katmanın sırasında çizilir; çizilenin bit bit aynı olduğu sınanır.
- **Yalnız yazısı değişen tablo parçasını kirletmez** (`drawn_print`, `drawn_same`): stil motoru bir tablodan
  çizgilerini ve hücre stillerinin kenarlarını çizer, sözcüklerini yazı katmanı çizer. Tablonun çizgileri hücreler,
  çizelge tanımı, formüller ve alanlar dışındaki her şeyinden çıkar; parçası kurulurken bunların izi (`DefaultHasher`)
  tutulur. Değişen tablonun izi aynıysa, aynı katmanın aynı parçasındaysa ve katmanı düz görünüşle (yorumlayıcısız)
  çiziliyorsa parçası kurulmaz. Tablo stili olan tablo (proje ayarları da çizer) ve yorumlayıcılı katman her zaman
  yeniden kurulur.

## Ölçümler

100 daire, 4 400 nesne, 200 çizelge (`KENTOS_PERF_SCHEDULES=each`), ms, medyan; aynı makinede art arda.

| Penceresiz (`flat_timing`) | Önce | §1–2 | §1–3 |
|---|---|---|---|
| Mesafe (Bu ölçüyü koru) | 479,0 | 100,4 | **32,5** |
| Planı | 529,8 | 130,7 | **3,0** |
| Kaydır | 488,4 | 95,7 | **32,0** |
| Öznitelik | 9,9 | 9,3 | 9,0 |
| Mahal çizelgesinin çıkarılması (adımda) | 197 | 2,0 | 2,0 |
| Bellek tepesi, Mesafe (MB) | 441 | 441 | 439 |

Masaüstü (`perf::flat`, gerçek Iced olayları, arayüz iş parçacığının payı p50):

| Masaüstü | Önce | §1–3 | §1–4 |
|---|---|---|---|
| **Tasarım değişikliğinin karesi** | 562 | 115,8 | **84,8** |
| — uygulama (komut ve izleme) | 502 | 56,8 | 57,0 |
| — görünüm | 40,6 | 40,8 | **9,1** |
| — çizim | 18,2 | 17,5 | 17,8 |
| Komut | 479 | 34,2 | 34,3 |
| Kapı önizlemesi (60 konum) | 450 | 1,8 | 1,9 |
| Metraj (`cad.spaces.quantities`) | 272 | 198 | 202 |
| Bütün planı kaydırma / bir daire / seçme | — | 1,9 / 0,7 / 0,8 | 2,0 / 0,7 / 0,9 |
| Değişiklik başına bellek (MB) | — | — | +0,2 |

§1–4'ün sütunu sayaçsız son ölçümdür. Karenin dökümü (§1–4, ayrı bir koşuda geçici sayaçlarla): komut 34 ms (çizelgelerin yazılması 21), uzamsal dizinin eşitlenmesi 22 ms
(değişen 100 tablonun kaydı), sahne 5–7,5 ms, yazı katmanının yerleşimi 2,6–2,9 ms, çizim 18 ms. **Etiket motorunun
yerleşimi karenin 3 ms'si**: ADR 0786'nın açık bıraktığı artımlı yerleşim bu hedef için gerekmedi.

Binanın yalnız iki çizelgesiyle (gerçek kullanım, `KENTOS_PERF_SCHEDULES` yok) aynı değişikliğin karesi §1–3'ten sonra
28 ms (komut 13,3, görünüm 9,3, çizim 2,8).

**Gerçek pencere** (`fixtures/interaction/real/buyuk-proje-akicilik.txt`; Xvfb 1920×1080, GPU yazılımla, aynı dosya iki
ikiliyle): fare Duvarı kaydır ile Bölme 3'ün yüzünü tutup 40 adımda (2,0 s) gezdirildiğinde önizlemenin işi gezinme
bittikten **16,1 s** sonra bitiyordu, şimdi **0,3 s**; Kapı Bölme 1 boyunca 60 adımda (3,0 s): **10,1 → 0,8 s**; 0,05 yazılıp
uygulanan değişiklik ekrana 1,2 → 1,0 s'de çıkar (çizimi yazılım GPU'su yapıyor). İki ikili aynı sonuca varır: Yatak odası
1 15,98 m², Yatak odası 2 11,93 m². Uygulamanın işlemci toplamı burada ölçü değildir: yeni ikili her kareyi çiziyor ve
yazılım GPU'su da işlemcide çiziyor.

## Sınırlar

- 200 çizelgenin 198'i birbirinin aynısıdır ve her biri kendi hücrelerini taşır; hücreleri paylaşmak sözleşmenin
  değişmesini ister. Kalan pay bu yüzden tablolarda: adımda 21 ms (100 kopya), uzamsal dizinde 22 ms (100 kaydın
  hücreleri), çizimde ~15 ms (görünüşteki tabloların yazısı yeniden boyanır). Bu dilimde yapılmadı.
- Metraj (198 ms) bu dilimin konusu değildi; mahal yüksekliği ondan da payını aldı (272 → 198).

## Doğrulama

- `cargo test -p kentos-domain --lib structure_index`: 60 döşeme (delikli, iki parçalı, üst üste) ve 40 kiriş (bir ile
  üç doğru, sola/ortaya/sağa yaslı), 4 000 nokta (dörtte biri 0,25 m ızgarada: köşelerde, kenarlarda, kutuların
  sınırında): dizin her nesneye bakan eski taramanın bulduğu döşemeleri ve kirişleri aynı sırada bulur; döşeme eklenir,
  taşınır, adımı geri alınır ve yinelenir, silinir, okuma kopyasında silinir: dizin her seferinde çizimi izler.
- `cargo test -p kentos-domain`: 159 test. `cargo test -p kentos-native-application`: 581 test geçti; iki test bu dilimden
  önce de kırıktı (M45'in "Asansör" ve "Otopark yeri" blokları şablon durumlarına yazılmamıştı; M48/M49'un iki komutunun
  hata kodları katalogda eksikti) ve ayrı işlemle düzeltildi.
- `cargo test -p kentos-desktop --lib style::scene_tests`: 13 test; yeniler: 512'lik parçalar bütün katmanın
  çizdiğini bit bit aynı sırada çizer; 4 400 nesnelik katman 9 parçada, bir düzenleme bir parçayı kurar, 600 nesnede 2
  parça, 100'de bütün; başka katmanın nesneleri arasına serpilmiş 1 152 nesnelik katman bütün; yalnız yazısı değişen
  tablo (bütün ve parçalı katmanda) hiçbir parçayı kurmaz, geri alınınca da; satırı uzayan tablo yalnız kendi parçasını.
- Ölçümler: `KENTOS_PERF_FLATS=100 KENTOS_PERF_SCHEDULES=each cargo test --release -p kentos-headless --test headless
  flat_timing -- --ignored --nocapture` ve `KENTOS_PERF_PARTS=1 KENTOS_PERF_FLATS=100 KENTOS_PERF_SCHEDULES=each cargo
  test --release -p kentos-desktop --lib perf::flat -- --ignored --nocapture --test-threads=1`.
- Gerçek pencere: `KENTOS_PERF_SAVE=DİZİN` ile yazılan `flats-100.kcad`, `KENTOS_REAL_SIZE=1920x1080
  KENTOS_BIN=target/release/kentos-cad KENTOS_REAL_START=20 scripts/usage/real/session.sh
  fixtures/interaction/real/buyuk-proje-akicilik.txt ÇIKTI flats-100.kcad`.
- `cargo clippy -p kentos-domain -p kentos-native-application -p kentos-desktop --tests`: değişen satırlarda uyarı yok.
