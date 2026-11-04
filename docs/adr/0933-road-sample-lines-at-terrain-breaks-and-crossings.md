# 0933 — KentOS Yol: arazi kırıklarında kesit ve kesişen kesit kolları

Tarih: 2026-11-04. plan/road.md Y29 (ROAD-SEC-13). Durum: Y29.1–Y29.3 kabul.

## Dayanak

- **Civil 3D, Create Sample Lines – By Station Range**
  ([yardım](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-EEECE0FB-8D39-465B-9D52-8794D3648150.htm)):
  - Düzlükte, kurpta ve geçişte ayrı aralık verilir.
  - Aralığın başında ve sonunda, yatay geometri noktalarında ve dever kritik km'lerinde ek kesit alınır.
  - Kol genişliği sabittir ya da her km'de bir ötelenmiş aksa uzatılır (snap).
  - İç kurpta kesişen kollar için belgelenmiş bir çözüm yoktur. Forumların önerisi genişliği küçültmek ya da kolu
    ötelenmiş aksa bağlamaktır.
- **OpenRoads Designer:** kritik yerler olay noktası listesiyle eklenir; yatay ve düşey ana noktalar ile tip kesit
  değişimleri bu listeye girer ([ODOT iş akışı, Temmuz 2024](https://www.oregon.gov/odot/EAST/Documents/CrossSection_CriticalSections.pdf)).
  Düzenli aralıklı kesitler ayrı bir modele düşer; kullanıcılar ikisinin tek dizide birleşmesini istiyor
  (Bentley topluluk forumu).
- **Netcad NETPRO:** ders kitaplarında "Modelden Enkesit Al" vardır; üçgen kenarlarında kesit alma seçeneğinin
  çevrimiçi belgesi bulunamadı. Gereksinim (ROAD-SEC-13) sahibin NETPRO deneyiminden gelir.
- **Matematik:**
  - Düz aks boyunca arazi bir üçgenin içinde doğrusaldır ve üçgen kenarlarında kırılır. Bu yerler boykesitin kesin
    arazisidir (ADR 0902 §1). Kurpta yay üçgenin düzleminde eğildiği için iki kırık arasındaki arazi tam doğrusal değildir.
  - Kare kesit çizgileri normallerin kesiştiği yerde buluşur. Dairede bu nokta merkezdir, iç yanda R uzaklıktadır.
    Klotoidde buluşma evolütün yakınına düşer.

## Karar

### §1 Çekirdek (`kentos_road::section::stations`, Y29.1)

- **`ground_breaks`:** boykesitin üçgen kenarı geçişleri (`profile::terrain`), aralığın başı ve sonu. Kırık bulmak
  için yeni bir algoritma yazılmadı; boykesitinki bağımsız referansıyla zaten doğrulanmıştı.
- **`simplify(profile, tolerance, most)`:** (km, kot) üzerinde Douglas–Peucker uygulanır. Uzaklık düşey okunur, kirişe
  dik okunmaz: önemli olan iki kesit arasında okunan kotun hatasıdır. Tutulan iki km arasında doğrusal arazi kesin
  profilden en çok tolerans kadar ayrılır. Eşit uzaklıkta ilk nokta seçilir.
  - `most`'tan uzun bir boşlukta, başından yürünerek, tutulan son km'ye `most` içinde kalan son kırık eklenir; içerde
    kırık yoksa ilk aşan eklenir. Kotlar yine profilin kendi noktalarıdır.
- **`crossings(al, stations, widths)`:** kolları kendi genişliklerinin içinde ve ikisinde de aynı yanda buluşan bütün
  çiftleri bulur. Aksın kendisini kesmek kesişme sayılmaz. Ucunda değen (kırpılmış) kollar da 1e-9 m payla kesişme
  sayılmaz.
- **`clip`:** buluşmalar yakından uzağa işlenir. Sıralama mikrometreye yuvarlanmış uzaklıkla, sonra kolların sırasıyla
  yapılır. Dairede her çift merkezde buluşur; sırayı yuvarlama değil kolların sırası belirlemelidir. İlk sürümde
  referansla Rust bu yüzden ayrıştı. İki kol da buluşmaya hâlâ ulaşıyorsa ikisi de oraya kısaltılır.
  - Tek geçiş yeter: son genişliklerde buluşan bir çift verilen genişliklerde de buluşuyordu. Sırası geldiğinde ya
    ikisi de ulaştı ve kısaltıldı, ya biri ulaşmıyordu; genişlikler yalnız kısalır.
  - İlk sürüm bütün kesişmeleri aynı anda kırpıyordu. Kısaltılmış bir kolun artık ulaşmadığı buluşmada komşu kol da
    kırpılıyordu ve gereksiz arazi kaybediliyordu.
  - Kırpma kolları aksa dik tutan tek çözümdür. Çarpık kesit başka bir kesit türüdür (ROAD-SEC-05).

### §2 Ölçüm: kırıklar mı, sabit aralık mı

Örnek projenin arazisi kullanıldı (samples/yol_ornegi.py ile aynı fonksiyon, 4 m oynatılmış 20 m ızgara) ve aynı aks
(862,5 m, R 300 m, 60 m klotoid). Aksın 91 kırığı vardır. Kesin profil kırıklar arasında doğrusaldır, bu yüzden en
büyük hata tam olarak kırıklarda okunur:

| Kesitler | Sayı | En büyük kot hatası | KOK |
|---|---|---|---|
| her 20 m | 45 | 0,290 m | 0,050 m |
| her 10 m | 88 | 0,217 m | 0,029 m |
| her 5 m | 174 | 0,078 m | 0,012 m |
| kırıklar, 0,20 m tolerans, en çok 20 m | 64 | 0,102 m | 0,016 m |
| kırıklar, 0,10 m tolerans, en çok 20 m | 67 | 0,044 m | 0,008 m |

0,10 m toleransla 67 kesit, her 5 m'lik 174 kesitten daha az hata verir. Sabit aralık arazinin kırığına denk
gelmediği sürece aradaki tepeyi ya da çukuru keser.

Bekçi: `tests/all/section_stations.rs::the_breaks_carry_the_ground_better_than_an_interval` (seyreltmenin sözü:
en büyük hata ≤ tolerans).

Sınır: ölçüm eksen arazisinindir. Kesit alanı kolun bütün genişliğindeki araziye bağlıdır. Eksenin kırığı yan
arazinin kırığını garanti etmez. Bu ayrım pencerede ve yardımda söylenir.

### §3 Kayıt ve komutlar (Y29.2, KCAD şema 166)

- **`SectionStations::Breaks { from, to, tolerance, most?, geometry }`:** km'ler, takımın yüzeyindeki eksen kırıklarının
  seyreltilmişi, uçlar ve `geometry` ile kurp noktaları ve PVI'lerdir. Takımın yüzeyi olmalıdır; tanımda yoksa kural
  reddeder, kurulamıyorsa km'ler çıkmaz ("yüzeyi kurulabilmeli").
- **`Range.curve`:** kurpta ve klotoidlerde (TS'den ST'ye) her `curve` metrelik yuvarlak km'de de kesit alınır. Civil
  3D'nin kurp ve geçiş aralığının karşılığıdır.
- **`SectionSetDef.clip`:** iç kurpta buluşan kollar §1'deki gibi kırpılır. Kesit satırının uçları kırpılmış koldur,
  arazi de o kolda okunur.
- **Rapor:** takım raporu `crossings`'i (tam genişlikte buluşan çiftler: iki km, yan, iki uzaklık) ve `clip`'i verir.
  İkisi de boşsa yazılmaz, eski cevaplar bayt bayt aynı kalır. Kırpmayan takımın sorgusu `section_lines_cross` uyarısı
  verir; kırpan takımda uyarı yoktur.
- **`road.sections.edit`'in `clip`'i:** `null` dokunmaz. "Değiştirilecek bir şey yok" iletisi de `clip`'i sayar.
- **KCAD şema 166:** `clip`, `breaks` ve `curve`. Yazıcı 166'yı yalnız bunlardan biri varken seçer; 165 ve öncesi
  bunları `unknown_field` ile reddeder (`fixtures/kcad/v2/road-section-breaks.kcad`,
  `broken/road-section-breaks-in-schema-165.kcad`).

13 yeni ortak durum vardır (edit 7, query 6). Kırıklar boykesit referansının yürüyerek bulduğu kenar geçişlerinden
gelir, seyreltme ve kırpma §1'in referansından. Ölçüm (örnek çizim, p1 sola döner, R 150 m, sol kol 180 m, her 20 m):
23 çift kesişir. Kırpılınca daire kısmında sol kollar tam 150,0 m'de, klotoidlerde 153,7 ile 177,4 m arasında biter,
düzlükteki kollar 180 m kalır. Kırıklarla 0–600 m aralığında 0,1 m tolerans ve en çok 20 m aralıkla 32 kesit çıkar.

### §4 Enkesit penceresi (Y29.3)

- **Km yöntemleri:** Aralık, Km'ler, Çizgiler, **Arazi kırıkları**. Aralıkta **Kurpta (m)** alanı vardır. Arazi
  kırıklarında Baş, Son, **Tolerans (m)**, **En çok (m)** ve Kurp noktaları ve PVI'ler alanları vardır.
- Kol genişlikleri ve yöntem bir satırdadır, yöntemin kendi alanları sonraki satırdadır. Tek satırda 1100 px'te yöntem
  seçicisi sıkışıp görünmez oluyordu.
- **Kesişen kolları kırp** onayının yanında nedeni yazar: kesişme varsa uyarının kendisi, yoksa yöntemin kısa açıklaması.
  Arazi kırıklarının açıklaması, eksen kırığının yan arazinin kırığını garanti etmediğini de söyler. Uyarı ilk sürümde
  grafiğin altındaydı ve kaydırmadan görünmüyordu.
- **Plan:** kesit grafiğinin yanında takımın kesit hatları plandan çizilir. Gösterilen kesit kalın çizilir; başka bir
  hatla kesişenler tehlike renginde çizilir. Kırpan takımda hatlar buluşmada biter ve kırmızı hat kalmaz.
- Kaydet `clip`'i yazar. İlk sürümde pencerenin isteği `clip`'i taşımıyordu; bunu pencere testi yakaladı.
- Ölçüm (örnek çizim): Ana takım 0–600 m'de 0,1 m toleransla ve en çok 20 m aralıkla 32 kırıkta kesit verir. Kurp
  noktaları ve PVI'lerle 39'dur. Sol kol 180 m iken, her 20 m'de ve kurp noktalarında 37 çift kesişir; kırpınca sol kol
  daire kısmında 150 m'de biter.

Bekçi: `section_window_tests::breaks_and_crossing_arms_are_typed_said_and_clipped`.

## Doğrulama

28 ortak durum (`scripts/fixtures/road_section_stations_cases.py`):

- 16 seyreltme durumu kesirlerle: düz arazi, sırt, basamak, sabit eğim, tek ve iki nokta, en büyük aralık, 40–300
  kırıklı rastgele araziler.
- 12 kesişme ve kırpma durumu 40 basamakla: R 60 dairede 30 m, 80 m ve uzun dış kol; sağa dönen kurp; klotoidli kurpta
  evolüt boyunca 68 m'den 45 m'ye inen kırpma; R 15 dar kurp, farklı genişlikler; düz aks; tek kesit; uzak kesitler;
  merkeze ulaşmayan kısa kol.
- Referans kırpılmış kolların artık hiç buluşmadığını kendisi de denetler.

## Açıklar

- Kesişme yerine kolu ötelenmiş aksa bağlamak (Civil 3D'nin snap'i).
- Yan arazinin kırıklarında (kolun boyunca) sıklaştırma.
