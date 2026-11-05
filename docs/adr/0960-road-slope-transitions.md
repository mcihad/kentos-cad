# 0960 — Koridor bölgesinde şev geçişi

Tarih: 2026-11-05. Durum: kabul (Y54). Kapsam ROAD-GRADE-03'ün kalanı (basamaklar ADR 0958–0959).

## Sorun

Koridor her bölgede tek bir tip kesit kullanır (ADR 0905). İki bölge aynı km'de buluşunca şevler orada sıçrar:

- yarma 1:1'den 2:1'e bir kesitten öbürüne geçer;
- şev ucu planda metrelerce kayar;
- üst yüzeyde bir basamak kalır.

Ön denetim (ADR 0956) bunu dikiş olarak söylüyordu ama düzeltmenin bir yolu yoktu. Sahada şev böyle sıçramaz; eğim
bir boy boyunca değişir.

## Araştırma

- **Civil 3D:**
  - Bölgeleri bitişik koymak dikişte keskin bir kenar bırakır. Forumdaki "%2'den %3'e birden geçiyor" şikâyeti
    bunu anlatır.
  - Boşluk bırakmak geçiş yapmaz.
  - 2023.2'den beri **Corridor Transitions** var: km'ye göre eğim değerleri yazılır ve değer o boyca dağıtılır. Örnekte
    %2'lik değişim 40 birime eşit yayılıyor. Belgesi bunu açıkça "daylighting to transition between slopes" için de
    söyler.
  - Daha önce ANZ Country Kit'in `BatterTransition` alt montajı vardı: şev iki km arasında 2:1'den 5:1'e geçer.
  - Kaynaklar: Autodesk Civil 3D forumu ("gradual transition of cross slope in corridor", "transition in corridor for
    one side"), IMAGINiT "Corridor Transitions, Civil 3D 2023.2 new feature", Autodesk "From the Ground Up" blogu
    (BatterTransition).
- **Ortak yan:** Geçiş bir km aralığında değerin doğrusal değişmesidir. Değer şevin kendi yazılışıyla (2:1 → 5:1)
  verilir.

## Karar

### Model (KCAD şema 178)

Koridor bölgesine isteğe bağlı `transition` (m) yazılır. Bölgenin başından o kadar metre boyunca her yanın yarma ve
dolgu şevi, önceki bölgenin tip kesitininkinden bu bölgeninkine doğrusal geçer:

m(s) = m₀ + (m₁ − m₀)·(s − baş)/L, 0 ≤ s − baş ≤ L

m bir düşeye yatay (y/d) yazılır, tip kesitteki gibi. Eğim (1/m) değil y/d'nin kendisi doğrusal gider; Civil 3D'nin
"2:1'den 5:1'e" yazılışı da budur.

**Sınırlar:**

- geçiş 0'dan uzun, bölgeden kısa olur;
- ilk bölgede olmaz;
- önceki bölgenin bittiği yerde (1 mm içinde) başlamayan bölgede olmaz.

Sözleşme, KCAD okuyucusu ve komut üçü de aynı iletiyle reddeder. Boşluktan sonra "önceki şev" yoktur.

### Neyi geçirir, neyi geçirmez

- Yalnız şevler (yarma ve dolgu) geçer.
- Bileşenler, hendek, basamaklar ve üstyapı bölgenin kendi tip kesitinindir. Genişliğin ve hendeğin geçişi şevinkinden
  ayrı bir iştir; bileşen genişliği zaten dever çizelgesinin genişletmesiyle değişir (ADR 0904, 0932).
- Bu yüzden şev geçişi dikişi yalnız iki tip kesitin bileşenleri aynıysa kapatır. Ölçüm aşağıda.

### Örnekleme

- Geçişin sonu koridorun örnekleme olaylarına girer (`why: "transition"`): yüzey o km'de kırılır.
- Öncelik sırasında en sondadır; aynı km'de başka bir sebep varsa o yazılır.

### Tek kapı

Bölgenin tip kesitini bir km'de okuyan her yer şu iki işlevden geçer:

- `road_corridors::region_template`;
- `Made::template_at(ri, s)`.

Bunları kullananlar:

- koridorun kendi kesitleri ve örnekleme denetimi (ADR 0939);
- kübajın ara kesitleri (prizmoidal) ve işlenmiş kesitleri (sıyırma, diş kazısı, üstyapı);
- şev tanısı;
- aplikasyon;
- istinat duvarı;
- km'de kesit satırı.

Biri unutulsa kübaj ile koridor iki ayrı yüzey hesaplardı.

### Pencere

- **Koridor → Bölgeler:** tabloya "Şev geçişi (m)" sütunu eklendi.
- **Ön denetim:** Artık **yazıldığı hâliyle** koridoru okur (`preview_of`). Önceden kayıtlı koridoru okuyordu: geçişi
  yazan kişi, etkisini görmek için önce kaydetmek zorundaydı. Şev tanısı görünüşü zaten yazılanı okuyordu.

## Ölçüm

### Bağımsız referanslar

Referanslar KentOS kodu kullanmaz. Hepsi Rust'ta ve Python'da geçer.

| Referans | Durum |
|---|---|
| `road_corridor_command_cases.py` | `template_at` kübaj referansıyla ortak; sorgu 4, düzenleme 4 yeni durum (biri geçerli, üçü ret: ilk bölge, bölgeden uzun, boşluktan sonra) |
| `road_volume_command_cases.py` | 3 yeni durum: ortalama alan, prizmoidal, sıyırma + diş kazısı |
| `road_corridor_preflight_command_cases.py` | 3 yeni durum |

### Dikiş

Ana koridor, km 1+300'de tk-1'den ikizine geçer. İkiz aynı bileşenlerle, yatık şevlidir: yarma 2, dolgu 3.

| Durum | Sol şev ucu sıçraması | Sağ şev ucu sıçraması | Ön denetim |
|---|---|---|---|
| Geçişsiz | planda 2,450 m, kotta 0,088 m | planda 1,669 m, kotta 0,059 m | buluşmuyor |
| 40 m geçiş | 0 | 0 | buluşuyor |

- **Üst yüzey:** Geçişsiz 12 916,3 m². 40 m geçişte 12 868,1 m², 120 m geçişte 12 672,8 m²; şevler daha uzun boy
  dik kalır.
- **Bileşenleri farklı tip kesit (tk-1 → tk-2):** 40 m geçişle de buluşmaz. En büyük sıçrama sağ şev başında
  planda 3,5 m'dir (tk-2'nin fazladan şeridi). Sol şev ucunun sıçraması 1,19 m'den 2,08 m'ye **büyür**: tk-1'in şevi
  tk-2'nin hendeksiz, başka genişlikteki platformundan başlayınca araziyi başka yerde keser. Pencere ve ön denetim bunu
  söyler.

### Kübaj

Ana koridorun tk-2 bölgesine 40 m geçiş verildiğinde:

| Yöntem | Geçişsiz kazı | 40 m geçişle kazı | Dolgu |
|---|---|---|---|
| Ortalama alan | 25 878,1 m³ | 25 873,2 m³ | 3 507,4 → 3 507,3 m³ |
| Prizmoidal | 25 564,7 m³ | 25 560,4 m³ | — |

### KCAD

Şema 178 `road-slope-transitions` dosyası. Aynı içerik şema 177'de reddedilir; 193 geçerli dosya.

## Açık

- **Bileşen geçişi:** Genişlik ve hendek geçişi yok. Bileşenleri farklı iki tip kesitin dikişi ancak bir ara bölgeyle
  ya da dever genişletmesiyle yumuşar.
- **Doğrusal olmayan geçiş:** Civil 3D'nin km'ye göre değer çizelgesi gibi doğrusal olmayan bir geçiş yok.
- **Planda görünmüyor:** Geçiş aralığı koridorun planında ayrıca çizilmiyor.
