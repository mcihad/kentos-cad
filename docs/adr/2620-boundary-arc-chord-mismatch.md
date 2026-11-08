# 2620 — Sınır grafiğinde yay ile kiriş uyuşmazlığı

Tarih: 2026-11-08. plan/map-cad.md MAP-08, dördüncü tur. Durum: taslak. Plan satırı: “Ortak sınır yalnız iki polygonun
tesadüfen çakışan koordinatları olmasın. Mantıksal sınır kimliği üzerinden komşuluk ve kaynağı izlenebilsin”; ADR
2220'nin “Sonra”sındaki “yay ile kiriş uyuşmazlığı”.

İlgili:
- [ADR 2000](2000-parcel-boundary-graph.md): sınır grafiği (köşe, sınır, iki yanın parseli, sorunlar);
- [ADR 2220](2220-boundary-graph-on-the-drawing-and-arc-corners.md): yaylı sınırda eksik köşe, Çizimde göster;
- [ADR 1960](1960-parallel-shift-shared-boundary.md): iki parselin aynı yayı paylaşması.

## Dayanak

- **ArcGIS Pro, parsel ağının öntanımlı topoloji kuralları** (ADR 2220'nin okuması): parsel çizgileri parsel
  sınırlarıyla çakışmalı (“Must be covered by”), parseller birbirini örtmemeli (“Must not overlap”), parseller arasında
  boşluk olmamalı (“Must not have gaps”). Bir yanda yay, öbür yanda kiriş olan sınır bu üç kuralı birden bozar: sınır
  çizgisi öbür parselin sınırını örtmez, aralarında bir şerit ya bindirme ya boşluktur.
- **AutoCAD'in kabarıklık tanımı** (ADR 2220): yay, kirişin uçlarıyla ve kabarıklıkla (bulge = tan(θ/4)) tanımlıdır;
  kabarıklık işaretini kirişin yönü belirler. Aynı iki köşe arasında kabarıklıkları farklı iki kenar aynı sınır değildir.
- **Dairesel segment:** kirişle yayı arasındaki alan r²/2 (θ − sin θ), θ yayın gördüğü açı; kirişin ortasında yayın
  yüksekliği (sagitta) b·c/2.
- **KentOS bugün (ADR 2000):** sınır “iki köşe arasında tek kabarıklıklı kenar”dır; yay ile kiriş iki ayrı sınır olur,
  ikisi de tek yanlı (“dış”) sayılır ve hiçbir sorun söylenmezdi: aradaki şerit görünmüyordu.

## Karar

1. **Yeni sorun `arcChord`** (`cad.parcels.boundaries`): aynı iki köşe arasında kabarıklıkları farklı (1e-9'dan çok) iki
   sınır: bir yanda yay, öbür yanda kiriş; ya da iki farklı yay. Her ardışık çift (sınır sırasıyla) bir sorundur:
   - `boundaries`: iki sınır (S…);
   - `area`: iki eğri arasındaki şeridin alanı: iki segmentin kabarıklık işaretli alanlarının farkı (aynı yana kabaran iki
     yayda fark, karşı yanlara kabaranlarda toplam);
   - `gap`: iki eğrinin ortalarının uzaklığı, |b₁ − b₂|·c/2;
   - `at`: iki ortanın ortası (şeridin içi); `parcels`: iki sınırın parselleri.
   Sorunlar sırasında `overlap`'tan sonra gelir.
2. Sınırlar ve sayılar eskisi gibi: iki sınır yine ayrıdır ve dış sayılır; grafik geometriyi okur, düzeltmez. Hangi
   yanın doğru olduğuna karar verilmez (kaynak belgesi gerekir).
3. **Masaüstü** (Sınır grafiği): sorunun adı “Yay ile kiriş uyuşmuyor”; Sorunlar tablosunda yeni **Fark** sütunu (“S4
   ile S5 · 6.72 m², ortada 1.000 m”); özet sayar; Çizimde göster şeridin içinde halkayla “Yay ile kiriş” yazar.
4. **Kapsam işareti:** ADR 2000'deki gibi **Harita + CBS** (komut Harita + Yol + CBS + Mimari).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/parcel_boundaries_cases.py`, KentOS kodu yok. Segmentin alanı yayın çemberinden
  (uçlarından ve ortasından geçen çember, mpmath 50 basamak), Rust'taki kapalı biçimden bağımsız.
  - 1 m kabaran yay ile kiriş: şerit 6,72 m², ortada 1 m;
  - aynı yana 1 m ve 0,5 m kabaran iki yay: 3,38 m², 0,5 m;
  - karşı yanlara kabaran iki yay: 10,06 m², 1,5 m;
  - üçü bir arada, her biri kendi sorunu.
  - 32 durum (28'den); var olan durumlar değişmedi. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/arc_chord_mismatch.rs`): üç uyuşmazlık `.kcad` olarak açılır, sorunlar durumunkiler.
- **Masaüstü** (`parcel_boundaries_window_tests.rs`): sorunlar, sınırları ve sayıları durumunkiler; Fark sütunu. Çapraz
  denetimler geçer. Resimler `.run/gui-evidence/yay-kiris-2620/`.

## Sonra

- Kalıcı parsel kaydı ve sınır kimliği (MAP-08'in ana açığı).
- Tablodaki satırın çizimde vurgulanması.
- Yayın kirişlerle (birçok köşeyle) yaklaşık çizildiği komşu: sınır grafiğinde “yaklaşık yay” (bugün eksik köşe değil,
  yay ile birden çok kiriş ayrı sınırlar).
- Web (TODOS §0.2).
