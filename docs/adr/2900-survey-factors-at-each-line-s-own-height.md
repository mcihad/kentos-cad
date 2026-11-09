# 2900 — Ölçme çarpanları çizginin kendi yüksekliğinde; `cad.crs.factors`'ta çizgi ve nokta yüksekliği

Tarih: 2026-11-09. plan/map-cad.md MAP-02, beşinci tur. Durum: taslak. ADR 1231'in “Sonra”sındaki “ölçme
pencerelerinin ‘ölçek çarpanı’ satırının bu komuttan okunması; kot farkıyla değişen yükseklik (nokta başına h)”.

İlgili:
- [ADR 0171](0171-ground-ellipsoid-plane.md): zemin, elipsoit, düzlem; hesap pencerelerinin indirgemesi (§4);
- [ADR 0519](0519-vertical-crs-and-epoch.md) ve [ADR 0631](0631-geoid-velocity-and-vertical-statements.md):
  projenin kotları (elipsoit ya da ortometrik), geoit modeli, h = H + N;
- [ADR 1231](1231-scale-summary-and-crs-factors.md): `cad.crs.factors`.

## Dayanak

- **NGS, Procedures for Design and Modification of the State Plane Coordinate System of 2022** (NGS 2023-1214-03-A3,
  [PDF](https://geodesy.noaa.gov/INFO/Policy/files/SPCS2022-Procedures.pdf), §g, 9 Kasım 2026'da okundu):
  - yükseklik çarpanı R/(R + h)'dir; h noktanın elipsoit yüksekliğidir, yani noktaya bağlıdır;
  - birleşik çarpan noktanın ölçeğiyle yükseklik çarpanının çarpımıdır;
  - çarpanlar en az 10 ondalıkla verilir (milyonda 0,0001).

  NGS R'yi noktanın enleminde Gauss ortalama yarıçapı alır. KentOS çizginin doğrultusundaki Euler yarıçapını kullanır
  (ADR 0171 §3); bu ADR onu değiştirmez.
- **KentOS bugün:**
  - hesap pencereleri her çizgiyi projenin tek ortalama elipsoit yüksekliğinde indiriyordu (ADR 0171 §2, §4);
  - ADR 0171 kotları kullanmıyordu, çünkü kotların elipsoit mi ortometrik mi olduğu bilinmiyordu;
  - ADR 0519'dan beri proje bunu söyleyebiliyor (Proje ayarları › Kotlar), ADR 0631'den beri geoit modeliyle
    h = H + N hesaplanıyor;
  - `cad.crs.factors` yalnız noktaların çarpanını tek yükseklikte veriyordu; pencerenin çizgi çarpanını Python ve MCP
    soramıyordu.
- **Büyüklük:** 100 m yükseklik farkı çarpanı 100/6,37·10⁶ ≈ 15,7 ppm değiştirir, km'de 1,6 cm. Dağlık arazide
  ortalama yükseklikle indirilen uzun bir çizgide bu fark milimetrelerin üstündedir.

## Karar

1. **`cad.crs.factors`** (ek alanlar; komut sürümü değişmez):
   - `heights`: her noktanın elipsoit yüksekliği. Noktanın yükseklik çarpanı kendi yüksekliğinde (doğu yönünde,
     ADR 1231'deki gibi);
   - `lines`: noktaların sıralarıyla çizgiler. Her çizginin ölçeği (Simpson), yükseklik çarpanı (jeodeziğin ortasında
     doğrultusundaki Euler yarıçapı, uçlarının yüksekliklerinin ortalamasında), birleşik çarpanı ve kullanılan
     yükseklik. Kutupsal alımın bir çizgiyi indirdiği çarpanlar bunlardır (ADR 0171 §3);
   - bir çizginin iki ucunun yükseklik çarpanlarının ortalaması ile ortalama yükseklikteki çarpan arasındaki fark
     (Δh/2R)² mertebesindedir: 1000 m farkta 6·10⁻⁹.
2. **Çizimin kotları** (`elevations`): her noktanın kotu projenin kotlarıyla elipsoit yüksekliğine çevrilir:
   - elipsoit yüksekliğiyse olduğu gibi;
   - ortometrikse projenin geoit modelinin noktadaki N'i eklenir (ızgaradan, dört düğüm arasında çift doğrusal;
     ADR 0631);
   - cevapta noktanın `height`'ı ve `undulation`'ı, `geoid` modelin adı;
   - geoidin ızgarası dışındaki noktanın kotu çevrilmez, uyarı `points_outside`. O noktanın ve çizgisinin yükseklik
     çarpanı verilmez;
   - tahmin yok: kotları söylenmemiş proje `heights_unknown`, geoit modelsiz ortometrik `geoid_required`, ızgarası
     cihazda olmayan model `geoid_missing` alır. Kotlar sıfır geoit ayrımıyla çevrilmez.

   Kural tektir (`kentos_native_application::survey_heights`): komut, pencere ve `cad.survey.points` aynı yoldan geçer.
3. **Kutupsal alım** (çekirdek `surveyPolar`'ın `undulations`'ı, ek alan): projenin ızgarası açıkken, istasyonun kotu
   ve bir alımın başucu açısı varsa çizgi kendi yüksekliğinde indirilir:
   - yükseklik, istasyonun ve noktanın elipsoit yüksekliklerinin ortalamasıdır: (Zᵢ + Nᵢ + Z + N)/2;
   - N'ler önce ortalama yükseklikle bulunan noktalarda okunur. Son nokta birkaç milimetre ötededir; N orada
     mikrometrenin çok altında aynıdır;
   - başucu açısı olmayan alım ve kotsuz istasyon ortalama yükseklikte kalır (noktanın kotu bilinmez);
   - her noktanın cevabında `height`: indirildiği elipsoit yüksekliği;
   - kotlar elipsoit yüksekliğine çevrilemiyorsa bütün çizgiler ortalama yükseklikte kalır ve pencere nedenini
     söyler;
   - `cad.survey.points` aynı yükseklikle hesaplar: Çizime ekle pencerenin gösterdiği noktayı yazar.
4. **Masaüstü** (Kutupsal alım):
   - sonuç tablosunda **h (m)** sütunu, raporda **Elipsoit yüksekliği** sütunu;
   - özet satırı hangi yüksekliklerin kullanıldığını ya da neden ortalamada kalındığını söyler.

   Poligon hesabı iki boyutludur, Aplikasyon'un hedefleri kotsuzdur; ikisi bugünkü gibi ortalamada kalır (Sonra).
5. **Kapsam işareti:** ADR 1231'deki gibi **CAD + CBS**: `cad.crs.factors` her projede; Kutupsal alım Harita'nın
   ölçme penceresi.

## Kanıt

- **Çekirdek** (`scripts/fixtures/ground_survey_cases.py`, PROJ ve GeographicLib, KentOS kodu yok):
  - üç yeni kutupsal alım: TM30'da elipsoit yükseklikleriyle dik çizgiler; UTM 36N'de ortometrik kotlar ve N'ler,
    başucu açısız çizgi ortalamada; kotsuz istasyonda ayrımlar kullanılmaz;
  - eğik uzunluk ve başucu açısı zemin uzunluğundan ve yükseklik farkından kurulur;
  - her noktanın `height`'ı da sınanır (`crs_ground` testleri); ayrım sayısı yanlışsa ret (çekirdek birim testi);
  - dosya PROJ 9.5.1 ile yeniden yazıldı (önce 9.7.1). Eski durumların sayıları en çok 3·10⁻¹² oranında değişti
    (PROJ'un sayısal türevi), toleranslar aynı.
- **Ortak durumlar** (`scripts/fixtures/crs_factors_cases.py`, PROJ; N betiğin kendi GTX okumasıyla):
  - noktaların kendi yükseklikleri; çizgiler kendi, ortalama, tek ve hiç yükseklikle;
  - elipsoit kotlar; TG-20 (sentetik) geoidiyle ortometrik kotlar; ızgaranın dışındaki nokta;
  - on bir ret;
  - 43 durum (24'ten). Eski durumlar aynı. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/crs_factors_heights.rs`): ızgara cihazda yokken `geoid_missing`; kitaplığa
  eklenince cevap ortak durumunki.
- **Masaüstü** (`survey_heights_tests.rs`):
  - TM36'da ortometrik kotlar ve TG-20: yokuş yukarı ve aşağı iki alımın çarpanları `cad.crs.factors`'ın aynı çizgi
    için verdiğiyle 10⁻¹² içinde aynı;
  - başucu açısız alım 1100 m'de;
  - Çizime ekle pencerenin noktalarını yazar;
  - kotları söylenmemiş projede neden söylenir, sütun yoktur.

  ADR 0171'in pencere testi ve çapraz denetimler geçer. Resimler `.run/gui-evidence/kendi-yuksekligi-2900/`.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("/is/harita/ana.kcad")
f = cad.crs.factors(doc, points=[[486500.0, 4420200.0], [487730.5, 4420200.0]], elevations=[1100.0, 1320.9],
                    lines=[[0, 1]])
print(f.lines[0].scale, f.lines[0].height_factor, f.lines[0].height, f.geoid)
```

MCP: `{"tool": "cad.crs.factors", "arguments": {"points": [...], "elevations": [...], "lines": [[0, 1]]}}`.

## Sonra

- Poligon hesabında ve ~~Aplikasyon'da~~ (ADR 3300) noktaların kotlarıyla çizgi yüksekliği (bugün ortalama).
- Gauss ortalama yarıçapıyla (NGS'nin bölge tasarımındaki gibi) çarpan seçeneği.
- Mesafe ölç ve Alan hesapla'nın satırlarında noktaların kotları.
- Web (TODOS §0.2).
