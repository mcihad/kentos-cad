# 3300 — Aplikasyon'da çizginin kendi yüksekliği

Tarih: 2026-11-09. plan/map-cad.md MAP-02, altıncı tur. Durum: taslak. ADR 2900'ün “Sonra”sındaki “Poligon hesabında ve
Aplikasyon'da noktaların kotlarıyla çizgi yüksekliği (bugün ortalama)”nın Aplikasyon yarısı.

İlgili:
- [ADR 0171](0171-ground-ellipsoid-plane.md) §4: ölçme pencerelerinde zemin ve projeksiyon uzunlukları;
- [ADR 1231](1231-scale-summary-and-crs-factors.md): noktanın kendi yüksekliğindeki çarpan;
- [ADR 2900](2900-survey-factors-at-each-line-s-own-height.md): çizginin kendi yüksekliği (`cad.crs.factors`,
  Kutupsal alım; `survey_heights`).

## Dayanak

- **ADR 2900'ün dayanağı** (NGS'nin ölçek ve yükseklik çarpanı yönergeleri, ADR 1231'in okuması): yükseklik çarpanı
  R/(R + h) çizginin bulunduğu elipsoit yüksekliğiyle alınır. Projenin ortalama yüksekliği yalnız bir yaklaşımdır:
  - 100 m yükseklik farkı 1,6·10⁻⁵ çarpan farkı, yani kilometrede 1,6 cm verir;
  - dağlık arazide aplike edilen uzunluk bu kadar kayar.
- **Aplikasyon bugün:** hedefler çizimin noktalarıdır, kotları da çizimde vardır. Ama zemin uzunluğu projenin ortalama
  yüksekliğiyle verilir. Kutupsal alım ADR 2900'den beri her çizgiyi kendi yüksekliğinde indiriyordu, Aplikasyon
  indirmiyordu: aynı istasyondan ölçülen ve aplike edilen aynı çizgi farklı çarpan alıyordu.

## Karar

1. **Çekirdek** (`surveyStakeout`'un girdisine ek alanlar):
   - `stationZ`: istasyonun kotu;
   - `targetZs`: her hedefin kotu (yoksa boş), hedeflerle bire bir;
   - `undulations`: istasyonda ve her hedefte geoit ayrımı (N), hedeflerden bir fazla.
2. **Kural:** projenin ızgarası açıkken, istasyonun ve hedefin kotu ve ayrımları varsa çizgi kendi yüksekliğinde
   alınır: (Zᵢ + Nᵢ + Zₕ + Nₕ)/2, uçlarının elipsoit yüksekliklerinin ortalaması (ADR 2900 §3'ün kutupsal kuralı).
   - Kotu olmayan hedefin çizgisi, kotsuz istasyonun bütün çizgileri ve ayrımlar verilmemişse hepsi ortalama
     yükseklikte kalır;
   - her çizginin cevabında `height`: alındığı elipsoit yüksekliği;
   - sayısı tutmayan kot ya da ayrım listesi ret.
3. **Kotlar ve ayrımlar** ADR 2900'ün kuralından gelir (`survey_heights::stakeout_undulations`): elipsoit kotta 0,
   ortometrikte projenin geoit modelinin N'i. Tahmin yok: kotları söylenmemiş proje, modelsiz ortometrik kot, cihazda
   olmayan ızgara ya da ızgaranın dışındaki nokta ortalamada kalır ve pencere nedenini söyler.
   - Hedefler bilinen noktalardır, ilk geçiş gerekmez. Kutupsal alımın noktalarıysa önce bulunur.
4. **Masaüstü** (Aplikasyon):
   - durulan nokta ve hedefler çizimin noktalarıysa kotları onlardan alınır. Y,X yazılan noktanın kotu yoktur;
   - sonuç tablosunda **h (m)** sütunu, raporda **Elipsoit yüksekliği** sütunu;
   - özet hangi yüksekliklerin kullanıldığını ya da neden ortalamada kalındığını söyler.
5. **Poligon** bu ADR'de yoktur: yeni noktaların kotu hesapta yoktur (iki boyutlu), dolayısıyla bacakların uç
   yükseklikleri bilinmez. Trigonometrik kotla birlikte gelir (Sonra).
6. **Kapsam işareti:** ADR 2900'deki gibi **CAD + CBS**: Aplikasyon her projenin hesap penceresidir.

## Kanıt

- **Çekirdek** (`scripts/fixtures/ground_survey_cases.py`, PROJ 9.5.1 (pyproj 3.7.1) ve GeographicLib, KentOS kodu yok):
  - üç yeni aplikasyon: TM30'da elipsoit yükseklikleriyle; UTM 36N'de ortometrik kotlar ve N'lerle, kotu olmayan hedef
    ortalamada; kotsuz istasyonda ayrımlar kullanılmaz;
  - her çizginin `height`'ı da sınanır (`crs_ground` testleri);
  - aynı PROJ sürümüyle yazıldı: eski durumların sayıları değişmedi, yalnız `height` eklendi.
- **Masaüstü** (`survey_heights_tests.rs`):
  - TM36'da ortometrik kotlar ve TG-20: çizimin S1 noktasından T1'e çizginin çarpanları `cad.crs.factors`'ın aynı
    çizgi için verdiğiyle 10⁻¹² içinde aynı;
  - kotsuz T2 ve Y,X yazılan hedef 1100 m'de;
  - raporda Elipsoit yüksekliği sütunu;
  - resimler `.run/gui-evidence/aplikasyon-yuksekligi-3300/`.

  Hesap pencereleri ve çapraz denetimler geçer.

## Sonra

- Poligon hesabında trigonometrik kotla bacakların kendi yüksekliği.
- Gauss ortalama yarıçapıyla çarpan seçeneği; Mesafe ölç ve Alan hesapla'nın satırlarında noktaların kotları
  (ADR 2900 Sonra).
- Web (TODOS §0.2): Aplikasyon'un web penceresi aynı çekirdeği çağırır.
