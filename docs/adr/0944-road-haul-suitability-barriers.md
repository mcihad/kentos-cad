# 0944 — Taşıma planında malzeme uygunluğu ve engeller

Tarih: 2026-11-05. Durum: kabul (Y38.1 çekirdek, Y38.2 komut ve şema 169, Y38.3 pencereler). Kapsam ROAD-HAUL-04 ve ROAD-HAUL-05.

## Sorun

Y23'ün taşıma planı (ADR 0924) yolu tek malzemeli sayar: her aralığın **net** kütlesi (kazı eksi dolgu) bir yerden
ötekine en küçük taşıma momentiyle gider. Sahada iki şey bunu bozar:

- **Uygunsuz kazı:** yarmadan çıkan malzemenin bir kısmı (organik, aşırı ıslak, şişen kil) dolguya konamaz; döküm
  alanına gider. Net kütle bunu göremez: uygunsuz kazı kendi aralığının dolgusunu bile dolduramaz.
- **Engeller:** köprüsü henüz yapılmamış bir dere, trafiğe kapatılamayan bir kavşak. Taşıma ya dolaşır ya hiç geçemez.

ROAD-HAUL-04: "Geology/material suitability ve contamination/unsuitable malzeme fill'e otomatik transfer edilmesin."
ROAD-HAUL-05: "Barriers, köprü geçişi, temporary road closure ve erişilebilirlik constraints taşıma planını ayırsın."

## Dayanak

- **Araştırma (5 Kasım 2026):**
  - Trimble Business Center'ın koridor kütle taşıması kullanılabilir ve kullanılamaz malzemeyi ayrı sayar; kullanılamaz
    malzeme döküm alanına taşınır.
  - Civil 3D'nin kütle taşıma dengelemesi yalnız ödünç ve döküm alanlarını kapasiteleriyle alır.
  - Uygunsuz malzemenin hangi oranda çıkacağı zemin etüdünün işidir. KentOS oran vermez.
- **Ağ akışı:** malzemenin kimliği yalnız nereye gidebileceğini değiştirir. Döküm alanında iki malzeme aynı kapasiteyi
  paylaşır. Bu, iki katmanlı tek mallı bir en küçük maliyetli akış problemidir; ayrı bir çok mallı problem gerekmez.

## Karar (`kentos_road::haul_plan::plan_sorted`)

- **Aralık:** başı, sonu, kazısı, dolgusu (planın taşıdığı hacimler, katsayılıysa düzeltilmiş) ve kazının uygunsuz
  payı (0–1).
- **İki hat:** yerler (aralık ortaları, alanlar) iki kez dizilir:
  - uygun hat: aralığın uygun kazısı girer, dolgusu çıkar, ödünç alanları verir;
  - uygunsuz hat: aralığın uygunsuz kazısı girer; yalnız döküm alanlarına çıkar.
- **Döküm alanı** bir düğümdür. İki hat ona bağlanır, kapasitesi tek bir yay üzerinden paylaşılır.
- **Aynı aralıkta** uygun kazı kendi dolgusunu sıfır mesafeyle doldurur. Uygunsuz kazı dolduramaz.
- **Engel:** istasyonu ve dolaşma uzunluğu (m), ya da kapalı.
  - Açık engeli geçen yayın uzunluğuna dolaşma eklenir.
  - Kapalı engeli geçen yay yoktur.
  - Bir yerin tam istasyonundaki engel o yerin hemen ilerisinde sayılır.
- **Amaç:** önce en çok yerleştirme, sonra en küçük moment. Yerleştirmenin kazancı yolun ve dolaşmaların toplamının iki
  katından büyüktür, bu yüzden hiçbir rota onu geçemez.
- **Uygunsuz kazı önce yerleşir.** Kazancı iki kattır. Döküm alanları yetmezse dışarıda kalan uygun kazı olur: o bir
  sonraki dolguyu bekleyebilir, uygunsuzun gidecek başka yeri yoktur. Bu kural olmadan iki çözüm eşit sayılıyordu ve
  hangisinin seçileceği belirsizdi (ilk başvuru yazımında ölçüldü).
- **Yöntem:** Y23'ün ardışık en kısa yollar yöntemi (Bellman–Ford, kesin).
- **Uyumluluk:** uygunsuz pay ve engel yoksa sonuç Y23'ün planıdır.

**Rapor:**

- yapılabilirlik;
- eksik dolgu, yerleşmeyen uygun kazı, yerleşmeyen uygunsuz kazı;
- her alanın kullanımı, döküm alanınınkinin uygunsuz kısmı;
- her aralıkta iki hattın akışı;
- moment ve uygunsuz kısmı;
- her engelden geçen hacim.

### Komut ve kayıt (Y38.2)

**Brükner tanımı** iki alan kazanır (KCAD şema 171; 170 ve öncesi `unknown_field` ile reddeder):

- `unsuitable`: km aralıklarıyla kazının dolguya konamayan yüzde payı. Satırlar kesişmez; pay 0–100 arasıdır.
- `barriers`: ad, km ve isteğe bağlı dolaşma (m). Dolaşma yoksa engel kapalıdır.

`road.hauls.edit` ikisini yazar; boş liste kaldırır.

**`road.hauls.plan`:**

- Brüknerde biri varsa aralıkların düzeltilmiş kazı ve dolgusuyla, uygunsuz payı aralık ortasında okunarak
  `plan_sorted` çalışır.
- **Rapora eklenenler:** toplam uygunsuz kazı, dökülemeyen kısmı, momentteki payı, her döküm alanının uygunsuz kısmı,
  her aralıkta akışın uygunsuz kısmı ve engellerden geçen hacim.
- **Yapılamazlık uyarısı** dökülemeyen uygunsuz kazıyı ayrıca söyler.
- **Ekonomik amaç** (ADR 0925) bu alanlarla birlikte henüz çözülmez ve açıkça reddedilir. Sessizce momente dönmez.

**Ortak durumlar:** başvuru (`road_haul_plan_cases.py`) aynı doğrusal programı kurar ve her yerleştirmenin tek
olduğunu denetler: en küçük moment altında en küçük ve en büyük değeri aynı olmalı. Sonra çözücünün toleransını
cilalar:

- sınırına 1e-6 yakın değer sınırdır;
- kapalı engellerin ayırdığı her parçada sınır dışında kalan tek yerleştirme o parçanın dengesinden kesin bulunur.

Bu durumlar bağıl 1e-5 ile karşılaştırılır:

- 10⁷ m³·m'lik momentler farklı sırayla toplanınca 1e-9 kadar ayrılır;
- yapılamaz planlarda birden çok yerleştirme serbest kalır ve HiGHS'in ~1e-5 m³ toleransı orada durur;
- farklı bir plan binlerce m³'ü metrelerce taşıdığı için bu tolerans onu yine yakalar.

48 plan ve 42 düzenleme durumu Rust'ta ve Python tipli katmanında aynıdır.

**Örnek proje** (Ana Brükner; döküm alanı km 30, ödünç alanı km 560):

| Durum | Moment (m³·m) | Not |
|---|---|---|
| Uygunsuz ve engel yok | 9.515.458 | Y23'ün planı |
| 0–300 m arası %40 uygunsuz | 9.515.458 | yol yarma fazlası: fazla zaten dökümde, uygunsuz onun yerini alır |
| Dere km 250, 400 m dolaşma | 16.446.719 | %73 fazla; 17.328 m³ dereyi geçer |
| %30 uygunsuz ve dere km 420, 150 m dolaşma | 12.062.999 | uygunsuz 1.277 m³, momentin 212.459'u; 16.984 m³ geçer |
| Kavşak km 300 kapalı | yapılamaz | doğudaki 14.276 m³ yarma döküme ulaşamaz |
| Döküm 500 m³, %60 uygunsuz | yapılamaz | 18.467 m³ uygunsuz dökülemez |

Uygunsuz malzemenin planı değiştirmediği ikinci satır önemlidir. Yarma fazlası olan bir yolda uygunsuz kazı, zaten
döküme gidecek fazlanın yerini alır. Plan ancak uygunsuz pay dolgu ihtiyacını karşılayan kazıya dokununca değişir. Engel
ise hacmi değil yolu değiştirir.

### Pencereler (Y38.3)

- **Brükner:** "Uygunsuz malzeme" (baş, son, yüzde) ve "Engeller" (ad, km, dolaşma; boşsa kapalı) tabloları.
- **Taşıma planı:**
  - resimde akışın uygunsuz kısmı mor, engeller kesikli çizgi (dolaşmasıyla ya da "kapalı");
  - uygunsuz yarma ve dökülemeyeni rakam olarak;
  - engellerden geçen hacim bir cümlede;
  - alan tablosunda döküm alanlarının uygunsuz kısmı.

Resimler `.run/gui-evidence/yol-y38/` altındadır.

## Ortak durumlar ve ölçüm (çekirdek)

`scripts/fixtures/road_haul_sorted_cases.py` KentOS kodu olmadan çalışır. Problemi doğrusal programlama olarak kurar ve
HiGHS ile çözer:

- **değişkenler:** her yayın iki yönlü akışı, yerleşen kazı ve karşılanan dolgu, alanların kullanımı;
- **birinci aşama:** en çok yerleştirme;
- **ikinci aşama:** onu koruyarak en küçük moment.

16 durum var. Kapalı biçimle sınananlar:

- %30'u uygunsuz yarma: 70 m³ 100 m öteye, 30 m³ ödünçten 150 m, 30 m³ dökümü 50 m geri; toplam 13.000 m³·m;
- aynı aralıkta yarma ve dolgu, yarısı uygunsuz;
- döküm kapasitesini paylaşan iki malzeme (dışarıda kalan 20 m³ uygun kazı);
- kapalı dere;
- 20 m ve 700 m dolaşma: dengelemek 12.000 m³·m, döküm ve ödünç 70.000 m³·m;
- yakın döküm ve ödünç dengelemekten ucuz;
- kapalı engel ve başka yol yok.

Uygunsuz payı ve engeli olmayan durumlar ayrıca Y23'ün net kütle başvurusuyla karşılaştırılır.

Rust 16 durumda da yapılabilirliği, eksik/artan/kalan hacmi ve momenti 1e-6 içinde aynı bulur. 20 rastgele yolda
uygunsuz pay ve engel yokken Y23'ün `plan`ıyla 1e-9 içinde aynıdır (`crates/shared/road/tests/all/haul_sorted.rs`).

## Açık

- Uygunsuz payın km'ye göre değil, katmana göre (üst 1 m uygunsuz) verilmesi.
- Engelin zamanı: köprü üçüncü ayda açılır.
