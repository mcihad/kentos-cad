# 2000 — Sınır grafiği: parsellerin ortak sınırı bir sınır olarak (`cad.parcels.boundaries`)

Tarih: 2026-11-07. plan/map-cad.md MAP-08, ikinci tur. Durum: taslak. Plan satırı: “Ortak sınır yalnız iki polygonun
tesadüfen çakışan koordinatları olmasın. Mantıksal sınır kimliği üzerinden komşuluk ve kaynağı izlenebilsin; CAD/GIS
geometrisi bunun denetlenebilir temsili olsun.” İlgili:

- [ADR 1400](1400-parcel-area-check-and-identity.md): Parsel alan denetimi, parsel kimliği;
- [ADR 0160](0160-topological-editing.md) ve [ADR 1960](1960-parallel-shift-shared-boundary.md): ortak köşe ve kenarın
  birlikte düzenlenmesi.

## Dayanak

- **ArcGIS parsel ağı (parcel fabric):**
  - Parseller, sınırları (lines) ve köşeleri (points) ayrı öğelerdir.
  - Kayıtlı ölçü ile koordinat ayrı tutulur.
  - En küçük kareler analizi ve uygulaması ayrıdır (plan [S17], [S18]).
- **Türk kadastrosu:** parsel köşeleri numaralı noktalardır. Ortak sınır iki parselin aynı köşeler arasındaki sınırıdır
  (ölçü krokisi). Bu ADR resmî bir kural uygulamaz; teknik bir denetimdir.
- **KentOS bugün:**
  - Parseller birbirinden bağımsız alanlardı. Ortak sınır yalnız koordinatların çakışmasından anlaşılıyordu.
  - Topoloji kuralları çakışma ve boşluğu buluyordu, ama sınırı ve iki yanındaki parseli göstermiyordu.
  - Topolojik düzenleme ortak köşeyi birlikte taşıyordu.

## Karar

1. **`cad.parcels.boundaries` v1** salt okunurdur; Harita, Yol, CBS ve Mimari'de parsel katmanı olan her projede çalışır.
   - **Girdi:** `uids`; yoksa parsel katmanının (`parsel`) bütün alanları. Retler `cad.parcels.check`'inkilerdir.
   - **Köşe:** bir halkanın köşesinin olduğu yer; koordinatları saklandığı gibi aynı olan yer bir köşedir. Doğu sonra
     kuzey sırasıyla `K1`, `K2` … diye numaralanır. Her köşe orada köşesi olan parselleri taşır.
   - **Sınır:** iki köşe arasında, aynı kabarıklıkla bir kenar. Kabarıklık küçük kimlikli köşeden büyüğe doğru alınır
     (1e-9 içinde aynı). Halka kenarı aynı sınırın üstündeyse aynı sınırdır. `S1` … diye köşelerine göre
     numaralanır.
     - Boy: kirişin, yayda c·θ / (2 sin(θ/2)).
     - Solu ve sağı, halkanın yönünden bulunur: saat yönünün tersindeki dış halkanın parseli kenarlarının solundadır;
       delikte tersi.
   - **Parsel:** kimliği (İl/İlçe/Mahalle/Ada/Parsel), geometrik net alanı, sınırları halka sırasıyla, komşuları.
   - **Sorunlar:**
     - `missingVertex`: bir köşe başka bir düz sınırın içinde 1 µm'de ve o sınırın orada köşesi yok. Sınır yalnız
       kısmen ortaktır; tipik örnek T birleşimidir;
     - `splitCorner`: 1 µm'den yakın iki ayrı köşe, yani tesadüfen çakışan koordinat;
     - `overlap`: üç ya da daha çok parselin kenarı, ya da bir yanda iki parsel.
2. **Kimlikler geometriden türetilir.** Aynı geometri aynı kimlikleri verir. Kalıcı bir kayıt değildir: kalıcı sınır
   kimliği ve sınırın kaynağı sonraki adımdır.
3. **Alanlar:** grafik geometrik alanı verir. Kayıtlı alanın üstüne bir şey yazılmaz (ADR 1400).
4. **Masaüstü:** Parsel alan denetimi penceresinde **Sınır grafiği…** düğmesi aynı parseller için **Sınır grafiği**
   penceresini açar:
   - özet: köşe ve sınır sayıları, ortak ve dış sınırlar, sorunlar türleriyle;
   - Sınırlar / Köşeler / Sorunlar seçimi ve kitin veri tablosu;
   - **Sorunluları seç**.

   Şerit yeri Parsel alan denetimininkidir: Plan › Ada ve parsel, CBS'de Veri › Tablo.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/parcel_boundaries_cases.py`, 24 durum; KentOS kodu yok.
  - Kurallar elle uygulanır. Shoelace saklanan değerler üzerinde kesirle hesaplanır.
  - İlk koşuda başvurunun harita koordinatlarında float shoelace'i 0,4 µm genişliğin farkını yitirmişti (100,0 yerine
    99,999996 m²). KentOS doğruydu. Başvuru kesire çevrildi; tolerans büyütülmedi.
  - Durumlar:
    - temiz ada: köşede birleşen üç parsel, yaylı dış sınır, delikli parsel ve içindeki parsel, saat yönünde çizilmiş
      parsel, iki parçalı parsel, yaylı ortak sınır;
    - alt kümeler: tek parsel, iki parsel, seçilmeyen komşu, seçim sırası;
    - parsel katmanındaki çizgi; CBS projesi; boş katman; plan ve doğrulama;
    - sorunlu ada: T birleşimi, 0,4 µm ayrık köşeler, bir yanda iki parsel; her biri tek başına da;
    - üç ret.
  - Kasıtlı bozulan bir sorun adı üç durumu düşürdü.

  Rust ve Python geçer.
- **Masaüstü** (`parcel_boundaries_window_tests`):
  - temiz adanın grafiği ortak durumun sayılarını verir;
  - sorunlu adanın altı sorunu sırasıyla listelenir;
  - Sorunluları seç yedi parseli seçer;
  - çizim değişmez.

  Resimler `.run/gui-evidence/sinir-grafigi-2000/` altındadır.

## Otomasyon

```python
from kentos import cad
g = cad.parcels.boundaries(doc)
for b in g.boundaries:
    print(b.id, b.from_, b.to, b.length, b.left, b.right)
print([(p.kind, p.at) for p in g.problems])
```

MCP: `{"tool": "cad.parcels.boundaries", "arguments": {"drawing": "…"}}`.

## Sonra

- Kalıcı parsel kaydı: kaynak belge, geçerlilik zamanı, önceki/sonraki parsel (şema).
- Kalıcı sınır kimliği ve sınırın kaynağı (ölçü, belge).
- Erişim denetimli hak, pay ve yükümlülük kayıtları.
- Sınır grafiğinin sahnede gösterilmesi (köşe numaraları, sorunların işaretleri).
- Yaylı sınırda eksik köşe denetimi.
