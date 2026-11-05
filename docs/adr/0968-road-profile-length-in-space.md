# 0968 — Kırmızı kotun uzaydaki boyu, km'den ayrı

Tarih: 2026-11-05. Durum: kabul (Y62). Kapsam ROAD-ALIGN-06.

## Sorun

ROAD-ALIGN-06: "Station, başlangıçtan analitik yatay uzunluk olsun; 3D uzunluk ve kullanıcıya gösterilen km label'ı ayrı
query olsun."

Durum şuydu:

- **Km:** Aksın başından yatay analitik boydur (ADR 0901). Km eşitlikleri ve başlangıç km'si yalnız km yazısını değiştirir
  (`km`, `kmText`).
- **Eksik:** Yolun uzaydaki boyu hiçbir sorguda yoktu. Aks düzlemde bir eğridir; uzaydaki boy ancak boykesitle, yani
  kırmızı kotla tanımlıdır.

## Araştırma

- **Yay boyu (diferansiyel geometri):** Düşey düzlemdeki z(s) eğrisinin boyu ∫ √(1 + z′(s)²) ds'dir. Yeni bir tanım
  değildir.
- **Parçalar kapalı biçimde:**
  - Eğimde z′ = g sabittir: boy Δs·√(1 + g²).
  - Parabolde z′ = u = g₀ + r·t doğrusaldır. Bilinen integral ∫ √(1 + u²) du = ½·(u·√(1 + u²) + asinh u) ile boy
    [F(u)]/r'dir.
  - Dairede boy R·|Δφ|'dir; φ merkezden düşeyle açıdır, sin φ = (s − s_c)/R.
- **CAD yazılımlarında:** Civil 3D aks boyunu yatay verir; 2B ve 3B boyu ayrı gösterdiği yer, kotlu çizgilerdir
  (feature line). KentOS'ta 3B boy da km'nin yerine geçmez, yanında ayrı bir alan olarak okunur.

## Karar

### Çekirdek

`Piece::length_to` ve `Vertical::length_to`/`length` kapalı biçimi uygular:

- **Parabol, eğim neredeyse değişmiyorsa:** Parçada eğim değişimi |Δu| ≤ 10⁻³ ise F(u₁) − F(u₀) farkı basamak
  kaybettirir. Bu durumda beş noktalı Gauss–Legendre kullanılır; integrand √(1 + u²) olduğundan hata Δu¹⁰ düzeyindedir.
- **Dairede:** Açı `asin`'den okunur. Eğimin `atan`'ı h → 0 yakınında d/h bölmesiyle bozulurdu.

### Komut

`road.profiles.query` iki alan verir:

- **`length3d`** (boykesit raporunda): kırmızı kotun ilk PVI'dan son PVI'ya uzaydaki boyu. Rapor `create` ve `edit`
  yanıtlarında da aynıdır.
- **`along3d`** (her km satırında): ilk PVI'dan o km'ye uzaydaki boy. Boykesitin dışında yoktur. İki satırın farkı, iki
  km arasındaki uzaydaki boydur.

Km (`station`, `km`, `kmText`) değişmez. Üçü ayrı alandır: yatay boy, kullanıcının gördüğü km yazısı ve uzaydaki boy.

### Pencereler

- **Boykesit:** Durum satırında "yatay 669,000 m · 3B 669,380 m" yazar. İkisi de çekirdeğin `Vertical`'ından okunur;
  komutun raporu da odur.
- **Görünüşler:** Alt satırda "ilk PVI'dan 3B 340,199 m" yazar. `road_profiles::point_at`'ten, yani sorgunun
  işlevinden okunur.
- **Taşma:** Satır uzayınca alt çubuktaki Kapat pencerenin dışına itiliyordu (1440 px'te "Ka" görünüyordu). Düzeltme
  pencerede değil, ortak bileşende yapıldı: `kentos_ui::widget::Dialog::status`. Durum yazısı düğmelerin bıraktığı yeri
  alır ve orada kırılır; düğmeler kendi genişliğinde kalır. Bu, CSS Flexible Box Layout'un `flex: 1 1 0; min-width: 0`
  ile `flex: none` kuralıdır (DESIGN.md §7.19 "Alt çubuk").

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_profile_command_cases.py` boyu kapalı biçimle değil, mpmath'in tanh–sinh integraliyle bulur.
Integral, referansın kendi eğimleri üstünden (`road_vertical_cases.py`, kesirler ve mpmath, 40 basamak) her parçada
alınır.

Yeni durum: "3B boy: dik eğimde km'den uzun, kurplarda integral". Profilde eğimler %15, −%10 ve %15'tir; L 100 parabol ve
R 500 daire vardır.

- %15 eğimde 150 m km'nin boyu 150·√1,0225 = 151,678 m'dir; referans bunu kendisi de sınar.
- Kısmi kot (%2): 300·√1,0004 = 300,05999 m.

Bütün sorgu durumlarında 57 değerde Rust ile referans arasındaki en büyük fark **1,1·10⁻¹³ m**'dir (durumların toleransı
10⁻⁶). Rust'ta ve Python'da geçer.

### Çekirdek testi

`verticals::the_length_in_space_is_the_arc_length` şunları sınar:

- eğimde √(1 + g²), 10⁻¹²;
- dairede R·Δθ, 10⁻⁹;
- her parça sonunda 20 000 aralıklı Simpson, 10⁻⁹;
- parabolün iki yolu (Δg 0,0004'te Gauss–Legendre, 0,002'de kapalı biçim) Simpson'a karşı, 10⁻¹⁰.

### Pencere testleri

- `the_status_says_the_length_in_space_apart_from_the_km_s`: Boykesit durum satırı sorgunun `length3d`'sini yazar.
- `the_red_line_at_the_km_is_the_query_s`: Görünüşler 21 km'de `along3d`'yi de sorguyla aynı okur.

## Açık

- **Koridorun 3B çizgileri:** Kenar, şev ucu gibi kotlu koridor çizgilerinin 3B boyu ayrı bir sorgu değildir.
- **Dever:** Platform kenarının uzaydaki boyu deveri de içerir; bu, kırmızı kotun boyu değildir.
