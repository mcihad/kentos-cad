# 0958 — Basamaklı yarma şevi (berme)

Tarih: 2026-11-05. Durum: kabul (Y52). Kapsam ROAD-GRADE-03'ün yarma kısmı.

## Sorun

Tip kesitin yarma şevi tek bir eğimle araziye uzanıyordu (ADR 0903). Yüksek yarmada şev her birkaç metrede bir
yatay basamakla (berme) kesilir:

- düşen taşı ve akan suyu tutar;
- şevin bakımına yol verir;
- şevin ortalama eğimini yatırır.

Basamak yazılamadığı için şevin buluştuğu yer, yarma alanı, kübaj ve üst yüzey yanlıştı. Basamaklı bir yarma tek
eğimli sayılıyordu.

## Araştırma

- **Civil 3D:**
  - `DaylightBench` alt montajı yarma ya da dolgu şevini gerekirse tekrarlanan basamaklarla kurar.
  - Araziye düşey uzaklık "en büyük yükseklik"ten fazlaysa basamak yüksekliği, eğimi ve genişliğiyle basamak koyar;
    değilse tek şev çizer.
  - `MultipleBench_Depth` dört basamağa kadar ayrı eğim ve genişlik alır; dördüncüden sonra şevle devam eder.
  - Basamak eğimi ± yüzdedir; artı, ofsetin arttığı yöne yükselir.
  - Kaynak: Autodesk Civil 3D Subassembly Reference, DaylightBench ve MultipleBench sayfaları (2022–2025).
- **Iowa DOT Design Manual:**
  - 200F-8 kaya yarmalarında: 35 ft'ten (≈10,7 m) yüksek şevde basamaklar en çok 35 ft arayla, genellikle en az
    10 ft (≈3 m) genişlikte olur; formasyon sınırlarına da konabilir.
  - 3J-1 berme ve basamakları zemin bölümüyle birlikte belirlenecek bir tasarım kararı sayar.
  - 200F-11 zemin yarmasında basamak genişliğini tipik olarak 15 ft verir.
- **WVDOT Cut Slopes Guidelines:** Basamak aralığı ve genişliği kaya türüne göre değişir; daha yüksek aralıkta daha
  geniş basamak istenir.
- **Sonuç:** Hepsi aralığı, genişliği ve eğimi zemin raporuna bırakıyor. KentOS öntanımlı sayı vermez: üçü de
  kullanıcıdan gelir.

## Karar

### Model

Tip kesitin her yanına isteğe bağlı `bench` {`height`, `width`, `fall`} yazılır (KCAD şema 177):

- `height`: bir yüzün düşey yüksekliği, (0, 100] m;
- `width`: basamağın yatay genişliği, (0, 50] m;
- `fall`: basamağın dışa doğru düşüşü, %0–50; öntanımı 0, sıfırsa yazılmaz.

Basamak yalnız **yarmada** uygulanır. Dolguda basamak ve tipler arası şev geçişi açık kalır (aşağıda).

### Geometri

Şev, hendeğin (varsa) ya da platformun ucundan başlayan bir kırık çizgidir (`bench_path`). m yarma şevi (y/d),
h yükseklik, w genişlik, f düşüş:

1. Yüz: yatayda h/m ilerler, düşeyde h yükselir.
2. Basamak: yatayda w ilerler, düşeyde w·f/100 alçalır.
3. Bu ikili, arazinin erişiminin ötesine geçilene kadar tekrar eder (en çok 200 basamak).

Arazi her parçada ayrı ayrı aranır (`meet_within`: parçanın sonunda kırpılmış arazi). İlk buluşma şevin ucudur.
Geçilen köşeler kesite girer.

- **Buluşmanın koşulu:** Basamaklı şevin ortalama eğimi (h − w·f/100) / (h/m + w)'dir. Arazi bundan dik yükseliyorsa
  şev ona hiç yetişmez ve kesit "açık" kalır; tek eğimli bir şevle buluşacak bir arazi basamakla buluşmayabilir.
  Referansın "Ortalama eğim araziden yatık: buluşmaz" durumu bunu sınar.
- **Basamağın üstünde buluşma:** Arazi basamağın üstünü kesebilir. O zaman şev basamakta biter, yüz başlamaz
  ("Basamağın üstünde buluşur", ofset 8,409).

### Kodlar ve tanı

- Koridor her basamak için iki öznitelik kodu yazar: `bench{k}In` (basamağın içi, önceki yüzün bittiği yer) ve
  `bench{k}Out` (basamağın dış ucu). Y52'de `bench{k}Top`/`bench{k}End` idi; dolgu basamağı (ADR 0959) gelince yöne
  bağlı olmayan adlarla değişti. Şevin ucundan önce gelen köşeler yazılır. Üst yüzey, kübaj ve çizim bunları sıradan noktalar
  gibi taşır.
- Şev tanısı (ADR 0951) basamaklı yolu aynı kırık çizgiyle okur; köşeler aday yerlere eklenir.
- Ön denetimin iletileri (ADR 0956) kodları Türkçe adlarıyla yazar: `bench1In` "1. basamak içi", `bench1Out`
  "1. basamak ucu".

### Pencere

Enkesit → Tip kesit → yanın uç koşulu:

- "Yarma şevinde basamak (berme)" anahtarı;
- "Basamak yüksekliği (m)", "Basamak genişliği (m)", "Basamak eğimi (%)" alanları.

Tip kesitin önizlemesi, yanlardan biri basamaklıysa daha yüksek bir örnek yarmaya geçer. Örnek arazi 5 ile 8 m
arasında 7 m tırmanır, sonra 3,5'te 1 yükselir. Basamaklar böylece görünür.

## Ölçüm

### Bağımsız referanslar

Referanslar KentOS kodu kullanmaz. Hepsi Rust'ta ve Python'da geçer.

| Referans | Durum |
|---|---|
| `road_template_cases.py` | 28 (8 yeni) |
| `road_daylight_cases.py` | 12 (2 yeni) |
| `road_corridor_command_cases.py` | Basamaklı koridor sorgusu; `bench1In`, `bench1Out`, `bench2In`, `bench2Out` kodları |
| `road_section_command_cases.py` | 3 tip kesit düzenleme durumu: basamaklı yeni tip, genişliksiz basamak, dik basamak eğimi |
| `road_corridor_preflight_command_cases.py` | 25 (1 yeni): km 1+300 dikişinde "yalnız öncekinde sol 1. basamak içi, sol 1. basamak ucu"; sol şev ucu planda 3,911 m, kotta 0,141 m kayıyor |

### Pencere testi

`İki şerit` tipine sol basamak yazıldı: 3 m yükseklik, 2 m genişlik, %4 düşüş, sağ yan solun aynısı. Ana aksta
km 1+040 kesitinin sol yanı:

| Nokta | Ofset (m) | Kot (m) |
|---|---|---|
| Hendek tabanı | −5,50 | 101,255 |
| Yüz (1:1, 3 m) | −8,50 | 104,255 |
| Basamak (2 m, −0,08 m) | −10,50 | 104,175 |
| Şev ucu | −12,12 | 105,794 |

Örnek yarmada tip kesitin önizlemesi dört basamak gösterir; yarma 86,31 m².

### KCAD

Şema 177 `road-cut-benches` dosyası. Aynı içerik şema 176'da reddedilir. 192 geçerli dosya.

## Açık

- **Dolgu basamakları:** Y53'te yapıldı (ADR 0959).
- **Şev geçişleri:** Bölgeler arasında şev eğiminin ya da basamağın km boyunca değişmesi (ROAD-GRADE-03'ün kalanı)
  yok. Şimdilik bölge sınırında sıçrar; ön denetim (ADR 0956) bunu dikiş olarak söyler.
- **En büyük yükseklik eşiği:** Civil 3D'de basamak ancak şev bir eşikten yüksekse başlar. Burada eşik basamak
  yüksekliğinin kendisidir: şev ilk yüzde buluşursa basamak çıkmaz.
- **Basamak hendeği:** Basamağın içine akan suyu toplayan hendek (MultipleBench'in swale'ı) yok.
