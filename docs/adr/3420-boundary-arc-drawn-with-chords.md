# 3420 — Sınır grafiğinde kirişlerle çizilmiş yay

Tarih: 2026-11-09. plan/map-cad.md MAP-08, altıncı tur. Durum: taslak. ADR 2620'nin ve ADR 3020'nin “Sonra”sındaki
“yayın kirişlerle (birçok köşeyle) yaklaşık çizildiği komşu”.

İlgili:
- [ADR 2000](2000-parcel-boundary-graph.md): sınır grafiği;
- [ADR 2220](2220-boundary-graph-on-the-drawing-and-arc-corners.md): yaylı sınırda eksik köşe;
- [ADR 2620](2620-boundary-arc-chord-mismatch.md): yay ile kiriş uyuşmazlığı.

## Dayanak

- **ArcGIS Pro, Densify** ([Densify (Editing)](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/editing/densify.html),
  ADR 3340'ın okuması):
  - gerçek eğriler (yaylar) ara köşelerle çoklu çizgiye çevrilebilir;
  - Offset yöntemi eğriden en büyük sapmaya göre köşe koyar. Kirişlerle çizilen yay, yayın kendisi değildir; aradaki
    sapma ölçülebilir bir büyüklüktür.
- **ArcGIS Pro parsel ağının kuralları** (ADR 2620'nin okuması): parseller birbirini örtmemeli ve aralarında boşluk
  olmamalı. Bir yanda yay, öbür yanda o yayı kirişlerle çizen sınır bu kuralları bozar: kirişlerle yay arasında bir şerit
  kalır.
- **Dairesel segment:** kiriş ile yayı arasındaki alan r²/2 (θ − sin θ); kirişin ortasında yayın yüksekliği
  r (1 − cos(θ/2)).
- **KentOS bugün:**
  - ADR 2620 aynı iki köşe arasındaki yay ile tek kirişi söyler;
  - ADR 2220 yayın üstündeki köşeyi eksik köşe sayar.

  Yayı birçok köşeyle çizen komşuda iki şey oluyordu. Köşeler tam yayın üstündeyse her biri ayrı bir eksik köşe
  oluyordu, şerit söylenmiyordu. Köşeler yuvarlanmışsa (santimetreye yazılmış koordinat) hiçbir şey söylenmiyordu: yay
  ve kirişler ayrı tek yanlı sınırlardı.

## Karar

1. **Yeni sorun `approxArc`** (`cad.parcels.boundaries`; sorunlar sırasında `arcChord`'dan sonra):
   - Her yay sınırı için yayın köşeleri bulunur: uçları, ve çemberinden uzaklığı sınırı geçmeyen, kirişin yay yanındaki
     köşeler. Uzaklık sınırı `arcTolerance`, en az 1 µm.
   - Her köşenin yeri, yayın açısının alt uçtan kesridir.
   - Alt uçtan zincir kurulur: her adımda (yayın kendisi dışında) bir sınırla, yayın ilerideki köşelerinin en yakınına
     gidilir. Eşitlikte küçük sınır seçilir.
   - Zincir en az bir köşeden geçip üst uca varırsa yay kirişlerle çizilmiştir.
2. **Yayın kendi parçaları sayılmaz:** zincirin her sınırı yayın kendi parçasıysa (bulge'ı yayın iki kesri arasındaki
   parçanınki, 10⁻⁹ içinde) yay köşelerinde bölünmüştür. Şerit yoktur, köşeler eksik köşe kalır. ADR 2220'nin
   durumu (iki komşu yayı tepesinde ikiye bölmüş) böyle kalır.
3. **Sorunun alanları:**
   - `boundaries`: yay, sonra zincirin sınırları;
   - `area`: iki kapalı eğrinin alanlarının farkı. Yay ve kirişi; zincir ve kiriş. İkisi de alt uca göre: köşelerin
     shoelace'i ve parçaların segmentleri;
   - `gap`: zincirin her sınırının ortası ile yayın o sınırın iki kesrinin ortasındaki noktası arasındaki uzaklıkların
     en büyüğü;
   - `at`: o en geniş yerin iki noktasının ortası;
   - `parcels`: yayın ve zincirin parselleri.
4. **Eksik köşe tekrarlanmaz:** zincirin köşeleri o yayın `missingVertex`'i sayılmaz; tek sorun onları anlatır. Öbür
   yaylardaki ya da düz sınırlardaki eksik köşeleri kalır.
5. **`arcTolerance`** (girdi, metre; ek alan, komut sürümü 1 kalır):
   - eksi ya da sonlu olmayan değer `invalid_input`'tır;
   - yalnız bu zincir için kullanılır; eksik köşe, ayrık köşe ve öbür kurallar 1 µm'de kalır;
   - değeri kullanıcı verir, KentOS önermez. Komşunun koordinatlarının yuvarlanma payı kullanıcının bildiğidir.
6. **Masaüstü** (Sınır grafiği):
   - **Kirişli yayda köşenin yaya uzaklığı (m)** kutusu (0: 1 µm), yardım satırıyla;
   - sorunun adı “Yay kirişlerle çizilmiş”;
   - Fark sütunu: “S5 ile S4–S9 (4 kiriş) · 1.68 m², en geniş 0.124 m”;
   - özet sayar; Çizimde göster en geniş yeri halkalar (“Kirişli yay”).
7. **Hangi yanın doğru olduğuna karar verilmez** (ADR 2620 gibi): grafik geometriyi okur, düzeltmez. Kaynak belgesi
   gerekir.
8. **Kapsam işareti:** ADR 2000'deki gibi **Harita + CBS** (komut Harita + Yol + CBS + Mimari).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/parcel_boundaries_cases.py`; yayın çemberi uçlarından ve ortasından, kesirler
  çemberin açılarıyla, alanlar ve açıklıklar mpmath'le; KentOS kodu yok):
  - 6 yeni durum (38 durum):
    - üç köşe tam yayda: tek sorun;
    - santimetreye yuvarlanmış köşeler uzaklıksız (yalnız tam tepedeki köşe eksik köşe) ve 1 cm'lik uzaklıkla;
    - kopuk zincir: köşeler eksik köşe kalır;
    - eşit olmayan kirişler ve zincirdeki T birleşimi;
    - eksi uzaklıkla ret.
  - Betik yayın ortasını artık mpmath'le hesaplıyor (harita koordinatında float ortası nanometreye yuvarlıyordu). Eski
    yay–kiriş alanları on altıncı basamakta değişti. Tolerans aynı (10⁻⁹).
  - Eski durumlar, ADR 2220'nin yayı ikiye bölen komşusu dahil, aynı. Rust ve Python geçer.
- **Python** (`python/tests/test_approx_arc.py`): SDK'nın `arc_tolerance`'ı; tek sorun, yuvarlanmış köşeler ve ret.
- **MCP** (`crates/native/mcp/tests/approx_arc.rs`): uzaklıksız ve 1 cm'lik uzaklıkla cevaplar durumunki.
- **Masaüstü** (`parcel_boundaries_window_tests::an_arc_drawn_with_chords_is_one_problem_with_a_distance`):
  - uzaklıksız tek eksik köşe; 0,01 m ile tek kirişli yay;
  - sınırlar, alan ve açıklık durumunki; Fark sütununun sözü;
  - resimler `.run/gui-evidence/kirisli-yay-3420/` (pencere ve çizim, açık geniş ve koyu dar).

  Sınır grafiği'nin öbür testleri ve çapraz denetimler geçer.

## Sonra

- ~~Çizimde zincirin ve yayın vurgulanması (bugün en geniş yer halkalanır).~~ ADR 3820.
- ~~Aynı yerdeki sorun sözlerinin tek sözde birleşmesi (ADR 3020 Sonra).~~ ADR 3820.
- Kalıcı parsel kaydı ve sınır kimliği (MAP-08'in ana açığı).
- Web (TODOS §0.2).
