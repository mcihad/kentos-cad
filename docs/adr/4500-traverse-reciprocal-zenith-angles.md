# 4500 — Poligon hesabında karşılıklı başucu açıları

Tarih: 2026-11-10. plan/map-cad.md MAP-02, dokuzuncu tur. Durum: taslak. ADR 4100'ün “Sonra”sındaki “karşılıklı başucu
açıları”.

İlgili:
- [ADR 0169](0169-field-data.md) §3: proje k katsayısı ve yer eğriliği–refraksiyon kuralı;
- [ADR 4100](4100-traverse-trigonometric-heights.md): Poligon hesabında trigonometrik kotlar.

## Dayanak

- **İTÜ Topografya, Bölüm 8** (ADR 4100'ün okuması; metin 10 Kasım 2026'da yeniden çıkarıldı): tek yönlü kural
  h = S·cot z + (1 − k)·S²/2r ve K_B = K_A + i − t + S·cot z + (1 − k)·S²/2r. Karşılıklı gözlem notta yoktur.
- **Baselga, García-Asenjo ve Garrigues, “Practical Formulas for the Refraction Coefficient”**, Journal of Surveying
  Engineering 140(2), 2014, doi:10.1061/(ASCE)SU.1943-5428.0000124; yazarların deposundaki metin
  [riunet.upv.es](https://riunet.upv.es/bitstream/10251/61536/3/Refraction.pdf) (PDF'in metni çıkarılıp denklem sayfaları
  resim olarak okunarak 10 Kasım 2026'da okundu):
  - kırılma katsayısını belirlemenin en kolay ve en hassas yolu eş zamanlı karşılıklı başucu gözlemleridir;
  - Z_A′ ve Z_B′ gözlenen eş zamanlı karşılıklı başucu açıları, R yerin ortalama yarıçapı, s görüş uçları arasındaki
    doğru boyunca uzaklık (kiriş) olmak üzere kapalı biçim (denklem 11):
    k = ½·sin Z_A′ + ½·sin Z_B′ + (2R/s)·cos((Z_A′ + Z_B′)/2);
  - kesin çözümle (Tsoulis vd. 2008) uygulamada çakışır;
  - eş zamanlı karşılıklı gözlem yapılamıyorsa birinci derece refraksiyon etkisini gideren yöntemlere başvurulur
    (ortadan geometrik nivelman, atlamalı trigonometrik nivelman).
- **KentOS bugün** (ADR 4100): her kenar tek yönlü, proje k'sıyla hesaplanıyordu. Kenarın öbür ucundan geri nişanı
  girilemiyordu; havanın gerçek k'sı proje k'sından farklıysa her kenarın kot farkı (k farkı)·S²/2R kadar şaşıyordu.

## Karar

1. **Çekirdek** (`survey::traverse`): `heights.back`, her kenar için isteğe bağlı geri nişan: kenarın uzak ucundan
   istasyonuna başucu açısı z′, oradaki aletin yüksekliği i′ ve istasyondaki işaretin yüksekliği t′.
   - Geri nişanlı kenarın kot farkı iki tek yönlü farkın ortalamasıdır:
     Δh = ((S·cot z + i − t) − (S·cot z′ + i′ − t′))/2. Tek yönlü kuralın (ADR 4100) eğrilik–refraksiyon terimi iki
     yönde aynı k ile aynıdır, ortalamada düşer; sonuç proje k'sından bağımsızdır. Bu, tek yönlü kuralın iki yöne
     uygulanmasının cebirsel sonucudur;
   - iki nişan aynı iki uç arasındaysa (her uçta aletin yüksekliği öbür uçtan nişanlanan işaretinkine eşit: i = t′,
     t = i′; Baselga vd.'nin kirişi), kenar çiftin kırılma katsayısını da verir: denklem 11, s = S / sin z (ileri
     nişanın kirişi), R = 6 371 000 m (ADR 0169 §3). Uçlar farklıysa k verilmez (kirişler aynı değildir);
   - kot kapanması ve dağıtılması ADR 4100'deki gibidir; tek yönlü kenarlar proje k'sıyla kalır.
2. **Komut** `cad.survey.points` (sürüm aynı, ek alanlar): `work.heights.back` (`zenith`, `instrument`, `target`; her
   kenara bir, geri nişansız kenar `null`); çıktıda `refractions`: her kenarın çift k'sı (yoksa `null`), en az bir
   kenar geri nişanlıysa.
   - Retler: `not_finite` (`work.heights.back[i].zenith`, `instrument`, `target`); `survey_refused`: geri nişan sayısı
     kenar sayısına uymuyor; geri başucu açısı 0 ile 200 g/180° arasında değil.
3. **Masaüstü** (Poligon hesabı):
   - Kotlar açıkken **Karşılıklı: Geri başucu açılarıyla** anahtarı; açılınca tabloya **Geri başucu**, **Geri alet** ve
     **Geri işaret** sütunları gelir. Geri başucu boşsa kenar tek yönlüdür; yükseklikler boşsa ileri nişanla aynı kiriş
     sayılır (geri alet = ileri işaret, geri işaret = ileri alet);
   - özet: “Karşılıklı nişanlı N kenarda kot farkı iki yönün ortalaması: yer eğriliği ve refraksiyon düşer. Aynı iki uç
     arasındaki çiftlerin k'sı: …”; raporda **k** sütunu;
   - Çizime ekle komutu `heights.back` ile çağırır.
4. **Kapsam işareti:** ADR 4100 gibi **CAD + CBS**.

## Sınırlar

- Eş zamanlılık denetlenmez: iki nişanın aynı havada yapıldığı kullanıcının sorumluluğudur; ortalama k'nın iki yönde
  aynı olduğunu varsayar.
- Çiftin k'sı yalnız bilgi olarak verilir; tek yönlü kenarlar yine proje k'sını kullanır (ölçülen k'nın projeye
  yazılması ayrı karar).
- Kot kapanması toleransı yoktur (kaynak bekler; ADR 4100'ün sınırı).

## Kanıt

- **Çekirdek** (`survey::tests`): iki 1000 m kenarlı açık poligonda birincisi geri nişanlı; açılar 10 m'lik farkın k =
  0,13 ile verdiği açılar: kot farkı 10 m (proje k'sı 0,13 de 0,2 de olsa), çiftin k'sı 0,13 (10⁻³ içinde); ikinci
  kenar tek yönlü proje k'sıyla; başka işaret yüksekliğinde k yok ama ortalama doğru; üç ret.
- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/geometry_call_reference.py` (yüksek duyarlıklı Decimal): bağlı poligonun birinci ve üçüncü
    kenarı geri nişanlı, 120. çağrı. Eski 119 çağrı aynı. Bozma denemesi: denklem 11'in kosinüs teriminin işareti
    değişince çağrı yakalar;
  - `scripts/fixtures/survey_points_cases.py`: 5 yeni durum (40): havanın k'sı 0,2, projeninki 0,13 iken bütün kenarları
    geri nişanlı poligonun kotları yolun kotları ve çiftlerin k'sı; ortadaki kenar tek yönlü, sonuncunun işareti başka
    yükseklikte (k yok); üç ret. Eski 35 durum değişmedi.
- **Python** (`python/tests/test_traverse_reciprocal.py`), testin kendi hesabıyla: geri nişanlı kenarların kotları proje
  k'sı 0,13 de 0,25 de olsa testin kotları; çiftlerin k'sı testin denklem 11 hesabı ve havanın 0,2'si; tek yönlü kenar
  proje k'sının farkı kadar şaşar.
- **MCP** (`crates/native/mcp/tests/traverse_reciprocal.rs`): durumun cevabı (kot kapanması yok, çiftlerin k'sı);
  eksik geri nişan reddedilir.
- **Masaüstü** (`calc::traverse_reciprocal_tests`): anahtarla dokuz sütun; birinci kenarın kot farkı iki yönün
  ortalaması, ikincisi proje k'sıyla; raporda k; Çizime ekle'nin girdisinde geri nişan; yüksekliksiz geri nişan aynı
  kiriş. Hesap testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/karsilikli-basucu-4500/`.

## Sonra

- Ölçülen k'nın (çiftlerin ortalaması) projeye önerilmesi; atlamalı (ortadan) trigonometrik nivelman.
- Kot kapanması toleransı (kaynak bekler).
- EPSG'nin çokgen kullanım alanları (MAP-02'nin öbür açığı).
- Web (TODOS §0.2).
