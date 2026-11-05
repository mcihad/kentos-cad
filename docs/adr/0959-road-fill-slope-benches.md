# 0959 — Basamaklı dolgu şevi

Tarih: 2026-11-05. Durum: kabul (Y53). Kapsam ROAD-GRADE-03'ün dolgu kısmı; yarma ADR 0958.

## Sorun

ADR 0958 basamağı yalnız yarma şevine koydu. Yüksek dolgunun şevi de basamakla kesilir:

- yüzey suyunu yavaşlatır;
- şevin bakımına yol verir;
- ortalama eğimi yatırır.

Bu yazılamıyordu. Basamaklı bir dolgu tek eğimli sayılıyordu: şevin ayağı, dolgu alanı ve kübaj yanlıştı.

## Araştırma

- **Civil 3D:** `DaylightBench` alt montajı aynı basamak kuralını yarmada da dolguda da uygular (ADR 0958).
- **Los Angeles Complete Street Design Manual, E-682 "Benches and Terraces":** Yarma ve dolgu şevlerinde en çok 25 ft
  (≈7,6 m) düşey arayla 10 ft (≈3 m) genişliğinde yatay basamak ister.
- **Jackson County (NC) yönetmeliği:** 20 ft'ten yüksek yarma ve dolgu şevinde her 10 ft yükseklik için en az 5 ft
  genişliğinde basamak ister.
- **Karıştırılmaması gereken şey:** Kentucky ve Oregon ulaştırma ayrıntılarındaki "embankment benching", dik
  arazide dolgunun **tabanının** basamaklanmasıdır (Kentucky: arazi %15'ten dik ise; basamak 1–6 ft). Bu, dolgunun
  altındaki diş kazısıdır ve ADR 0906 §6'da ayrı bir hacim olarak vardır. Bu karar dolgunun **şevini** basamaklar.
- **Sonuç:** Sayılar kurumdan kuruma değişiyor (7,6 m'de 3 m, 3 m'de 1,5 m). KentOS öntanımlı sayı vermez.

## Karar

### Model

Tip kesitin yanına `fillBench` {`height`, `width`, `fall`} eklendi. Biçimi ve sınırları yarma basamağıyla
(`bench`) aynı:

- yükseklik (0, 100] m;
- genişlik (0, 50] m;
- düşüş %0–50, dışa doğru.

İkisi ayrı alandır. Aynı yanda yarma ve dolgu farklı basamaklanabilir; kesit hangi durumdaysa (yarma ya da dolgu)
o yanın alanı okunur.

### KCAD

KCAD şema 175'e katıldı; yeni şema açılmadı. Şema 175 aynı gün, henüz yayımlanmamış Y52'de açılmıştı. Ayrı bir 176,
iki alanı ayrı sürümlere bölüp aynı özelliği iki kez denetletirdi. 174 ve öncesi `fillBench`'i de `unknown_field`
ile reddeder.

### Geometri

`bench_path` artık iki yöne yürür. m eğim (y/d), başlangıçtan dışa doğru:

- **Yüz:** yarmada h yükselir, dolguda h iner; yatayda h·m ilerler.
- **Basamak:** w genişliğinde, w·f/100 alçalarak.

Yarmada ve dolguda aynı `walk` yürür: her parçada arazi ayrı aranır ve ilk buluşma şevin ucudur.

**Buluşmanın koşulu:** Basamaklı dolgunun ortalama düşüşü (h + w·f/100) / (h·m + w)'dir.

- Arazi bundan **dik** düşüyorsa şev ona yetişmez ve kesit açık kalır.
- Aynı arazide tek eğimli bir şev buluşabilir. Referansta "Dolgu ortalama eğiminden dik düşen arazi: buluşmaz" ve
  "Aynı arazide basamaksız dolgu buluşur" (ofset 39,62) bunu yan yana sınar.

### Kodlar

Basamak kodlarının adı yöne bağlı olmaktan çıktı:

| Önceki ad (Y52) | Yeni ad | Anlamı |
|---|---|---|
| `bench{k}Top` | `bench{k}In` | Basamağın içi: önceki yüzün bittiği yer |
| `bench{k}End` | `bench{k}Out` | Basamağın dış ucu: sonraki yüzün başladığı yer |

"Top" dolguda yüzün dibini anlatırdı. Y52 yayımlanmadığı için ad değiştirilebildi. Ön denetimdeki Türkçe adlar
("1. basamak içi", "1. basamak ucu") aynı kaldı.

### Pencere

Enkesit → Tip kesit → yanın uç koşulu:

- "Dolgu şevinde basamak (berme)" anahtarı;
- "Dolgu basamağı yüksekliği (m)", "Dolgu basamağı genişliği (m)", "Dolgu basamağı eğimi (%)" alanları.

Önizlemenin örnek arazisi sağda da derinleşir: 5–8 m arasında 7 m düşer, sonra 3,5'te 1 düşmeye devam eder ve 60 m'ye
uzanır. Böylece bir yanda yarma, öbür yanda dolgu basamakları görünür.

## Ölçüm

### Bağımsız referanslar

Referanslar KentOS kodu kullanmaz. Hepsi Rust'ta ve Python'da geçer.

| Referans | Durum |
|---|---|
| `road_template_cases.py` | 36 (8 yeni) |
| `road_daylight_cases.py` | 14 (2 yeni) |
| `road_corridor_command_cases.py` | "Basamaklı dolgu şevli koridor": TK2'nin iki yanına 1 m / 1,5 m / %4 dolgu basamağı. Dolgu km 1+347 – 1+425'te en çok 2,7 m. `left.bench1In/Out`, `right.bench1–2In/Out` kodları çıkar. |
| `road_section_command_cases.py` | 2 yeni tip kesit düzenleme durumu |
| `road_corridor_preflight_command_cases.py` | Kod adları yeni adlarla |

### Tip kesit referansının ölçtükleri

| Durum | Sonuç |
|---|---|
| Derin vadi, iki yanda 3 m / 2 m basamak, 1:1,5 | Dört basamaktan sonra 32,94 m'de buluşur; dolgu 204,28 m² |
| Basamağın üstünde buluşma | Arazi ilk basamağın (96,87 m) üstünü 9,31 m'de keser; dolgu 8,49 m² |
| %4 dışa düşen 4 m / 3 m basamak, 1:2 | 63,84 m'de buluşur |

### Şev tanısı

Basamaklı dolgunun kırık çizgisi, ortalama eğimine yakın düşen araziyi birkaç kez keser.

- **Vadi durumu:** Şev araziyi 3 kez keser. İlk kesişim (şevin ucu) 34,21 m'dedir; sonraki 2,11 m ötede, duyarlılık
  2,55 m/m.
- **Tanının söylediği:** Bu, şevin ucunun arazideki küçük bir değişimle bir basamak ileri sıçrayabileceğini gösterir
  (ADR 0951).

### Pencere ve KCAD

- **Pencere:** Sol yanda 3 m / 2 m / %4 yarma, 3 m / 2 m dolgu basamağı (sağ yan solun aynısı). Örnek arazide yarma
  86,31 m², dolgu 106,78 m². Aynı arazideki tip kesit referansı da dolguyu 106,78 m² bulur.
- **KCAD:** `road-cut-benches` örneğinin yarma basamaklı yanına 5 m / 2 m dolgu basamağı eklendi. Dosya şema 175'te
  geçerli, 174'te reddedilir.

## Açık

- **Şev geçişleri:** Bölgeler arasında şev eğiminin ya da basamağın km boyunca değişmesi yok (ROAD-GRADE-03'ün
  kalanı).
- **Dolgu basamağının hendeği:** Basamağın içine eğimli suyu toplayan hendek ya da şev başı hendeği yok.
