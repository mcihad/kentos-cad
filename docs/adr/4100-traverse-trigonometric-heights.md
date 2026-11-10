# 4100 — Poligon hesabında trigonometrik kotlar: her kenar kendi yüksekliğinde

Tarih: 2026-11-09. plan/map-cad.md MAP-02, sekizinci tur. Durum: taslak. ADR 3300'ün ve 3700'ün “Sonra”sındaki
“Poligon hesabında trigonometrik kotla bacakların kendi yüksekliği”.

İlgili:
- [ADR 0071](0071-desktop-calc-traverse-and-polar.md): Poligon hesabı ve kapanmaların dağıtılması;
- [ADR 0169](0169-field-data.md) §3: proje k katsayısı ve yer eğriliği–refraksiyon kuralı;
- [ADR 0171](0171-ground-ellipsoid-plane.md) §4: uzunlukları projeksiyona indirme;
- [ADR 2900](2900-survey-factors-at-each-line-s-own-height.md): yükseklik çarpanı çizginin kendi elipsoit yüksekliğinde;
- [ADR 3300](3300-stakeout-lines-at-their-own-height.md): Aplikasyon'da aynı kural.

## Dayanak

- **İTÜ İnşaat Mühendisliği, Topografya ders notu, Bölüm 8** (M. Zeki Coşkun),
  [topografya_bolum_8.pdf](https://web.itu.edu.tr/~coskun/contents/lessons/topo/topografya_bolum_8.pdf) (PDF indirilip
  metni çıkarılarak 9 Kasım 2026'da okundu):
  - yakın noktalar arasında (250 m'ye kadar) h = L·cot Z;
  - 250 m'den uzakta yer küreselliği ve atmosferik refraksiyon düzeltilir: h = S·cot z + (1 − k)·S²/2r;
  - A'ya kurulan aletin yüksekliği i, B'de gözlenen hedefin yüksekliği t ile B'nin kotu
    K_B = K_A + i − t + S·cot z + (1 − k)·S²/2r.
- **Trimble Access**, [Instrument corrections](https://help.fieldsystems.trimble.com/trimble-access/latest/en/instrument-corrections.htm)
  (9 Kasım 2026'da okundu):
  - yer eğriliği ve refraksiyon düzeltmeleri düşey açı gözlemlerine uygulanır; sayfa tipik katsayılar olarak 0,13, 0,142
    ve 0,2'yi anar;
  - eğrilik daha büyük etkidir, kilometrede yaklaşık 16"; tipik katsayıda refraksiyon ters yönde ve eğriliğin yaklaşık
    yedide biridir;
  - sayfa formül vermez; alet ve hedef yüksekliği düzeltmelerini ayrı düzeltme türleri olarak sayar.
- **Leica TS11 el kitabının formül sayfası** okunamadı (manualslib, HTTP 403); arama özetindeki değerler dayanak
  sayılmadı.
- **KentOS bugün:**
  - Kutupsal alım başucu açısıyla kot hesaplıyordu: proje k'sı (Proje ayarları › Ölçme, varsayılan 0,13) ve
    `fieldbook::curvature`'ın (1 − k)·D²/(2·6 371 000) kuralı (ADR 0169 §3);
  - Poligon hesabı yalnız konum veriyordu; yeni noktalar kotsuz yazılıyordu;
  - uzunluklar projeksiyona indirildiğinde poligonun kenarları projenin ortalama yüksekliğinde indiriliyordu; kotu
    olmayan bacak için kendi yüksekliği bilinemiyordu (ADR 3300, Sonra).

## Karar

1. **Çekirdek** (`survey::traverse`): isteğe bağlı `heights`: başlangıç kotu, bitiş kotu (yoksa kot kapanması yok),
   her kenarın istasyonunda başucu açısı, alet ve işaret yüksekliği, k ve isteğe bağlı geoit ayrımları (başlangıçta,
   her yeni noktada, bitişte).
   - Kenarın kot farkı ITU notunun kuralıdır, eğrilik terimi KentOS'unki:
     Δh = S·cot z + i − t + (1 − k)·S²/(2·6 371 000). S ölçülen yatay uzunluktur (zeminde).
   - Bitişin kotu biliniyorsa kot kapanma hatası fh = Σ Δh − (H_bitiş − H_başlangıç) kenarlara uzunluklarıyla orantılı
     dağıtılır: vh_i = −fh·S_i/ΣS. Kaynakta okunmadı; koordinat kapanmasının dağıtılmasıyla aynı KentOS kararıdır.
   - Sonuç: yeni noktaların kotları (`heights`), `heightMisclosure`; kenar başına `dh`, `vh`.
   - Kapalı poligonda bitiş kotu başlangıcınkidir; açıkta yoktur.
2. **Kenar kendi yüksekliğinde:** uzunluklar projeksiyona indiriliyorsa her kenarın yükseklik çarpanı uçlarının
   dengelenmiş kotlarının, geoit ayrımları verildiyse H + N'lerinin ortalamasında alınır (ADR 2900'ün kuralı). Kenar
   bu yüksekliği `height` olarak söyler. Ayrımlar, konumların ilk hesabında (ayrımsız) noktaların düştüğü yerlerde
   okunur.
   - Projenin kotları elipsoit yüksekliğiyse kotlar olduğu gibi alınır.
   - Ortometrikse geoit modelinden N alınır; model yoksa ya da bir nokta ızgara dışındaysa kenarlar eskisi gibi
     ortalamada indirilir ve pencere nedenini söyler.
   - Projenin kotlarının ne olduğu söylenmemişse yine ortalamada indirilir; pencere bunu söyler.
3. **Komut** `cad.survey.points` (sürüm aynı, alan eklendi): `work.heights` (`start`, `end`, `zeniths`, `instruments`,
   `targets`); çıktıda `heightMisclosure`. Kotlar noktaların Z'sine ve `Z (m)` özniteliğine yazılır.
   - Başlangıç kotu verilmezse başlangıç noktasının kendi kotu (kotu olan çizim noktası); bağlı poligonda bitiş kotu
     verilmezse bitiş noktasınınki, o da yoksa kot kapanması denetlenmez.
   - Retler: `not_finite` (`work.heights.zeniths[i]`, `instruments`, `targets`, `start`); `survey_refused`: başlangıç
     kotu yok (`work.heights.start`), başucu açısı sayısı kenar sayısına uymuyor, başucu açısı 0 ile 200 g/180° arasında
     değil, geoit ayrımlarının sayısı uymuyor.
4. **Masaüstü** (Poligon hesabı):
   - “Kotlar: Trigonometrik nivelmanla hesapla” anahtarı. Açılınca tabloya Başucu açısı, Alet yüksekliği ve İşaret
     yüksekliği sütunları gelir, kenar sütununun başlığı kısalır (Kenar);
   - Başlangıç kotu ve (bağlıda) Bitiş kotu alanları; boşsa noktanın kendi kotu;
   - açıklama Δh = S·cot z + i − t'yi söyler;
   - özet kot kapanma hatasını, başlangıç ve bitiş kotunun nereden geldiğini, k'yı ve kenarların hangi yükseklikte
     indirildiğini söyler;
   - sonuçta H sütunu; raporda Δh, vh, H ve “Kot kapanma hatası” satırı;
   - Çizime ekle komutu `heights` ile çağırır.
5. **Kapsam işareti:** **CAD + CBS** (ölçme hesabı; noktaların kotları çizimin ve CBS katmanlarının verisidir).

## Sınırlar

- Kenar başına tek yönlü gözlem: karşılıklı (ileri–geri) başucu açılarının ortalaması yok.
- Kot kapanma hatası için tolerans yok. Proje ayarları › Ölçme'de bir kot sınırı alanı yoktur ve değer uydurulmaz;
  kullanıcı ölçü sınıfının sınırıyla karşılaştırır.
- Uzunluk eğik değil, yataydır (Poligon hesabının tablosu yatay kenar alır). Eğik uzunlukla kot Kutupsal alımdadır.
- k proje genelidir; kenar başına k yoktur.

## Kanıt

- **Çekirdek** (`crates/shared/geometry-core/src/survey/tests.rs`):
  - `the_traverse_carries_its_heights_by_trigonometric_heighting`: kot farkları, kapanma ve dağıtım testin kendi
    hesabıyla;
  - `each_leg_s_factor_is_taken_at_its_own_height`: TM30'da kenarın çarpanı kendi yüksekliğinde.
- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/geometry_call_reference.py`: iki yeni çağrı durumu (bağlı poligon kotlarla, grad; açık poligon
    yer küreselliği ve k ile). Eski durumlar aynı;
  - `scripts/fixtures/ground_survey_cases.py`, PROJ 9.5.1 ve GeographicLib ile: UTM 36N'de bağlı poligon
    trigonometrik kotlarla, her kenar kendi yüksekliğinde. Eski iki poligon durumu aynı;
  - `scripts/fixtures/survey_points_cases.py`: altı yeni `cad.survey.points` durumu (35 durum): bağlı, kapalı
    (12 mm kapanma), başlangıç kotu A'nın kendisinden, kotsuz başlangıç, sayısı uymayan ve sonlu olmayan başucu
    açısı. Rust ve Python geçer.
- **Python** (`python/tests/test_traverse_heights.py`), testin kendi açıları, uzunlukları ve kuralıyla: açık poligon
  farkları toplar; kapalı poligon kapanmayı uzunluklarla dağıtır, son nokta başlangıçta kapanır; `Z (m)` üç ondalık.
- **MCP** (`crates/native/mcp/tests/traverse_heights.rs`): ortak durumların çizimi `.kcad` olarak açılır; kapalı
  poligonun ve A'nın kendi kotundan başlayanın noktaları durumların kotlarıyla yazılır; kotsuz başlangıç reddedilir.
- **Masaüstü:**
  - `calc::tests::a_closed_traverse_carries_its_heights`: kare poligonda kotlar testin kendi hesabıyla, rapor, komutun
    `heights`'ı ve yazılan noktaların Z'si;
  - `survey_heights_tests::each_traverse_leg_is_taken_at_its_own_height`: TG-20 ortometrik projede (1100 m) her
    kenarın yüksekliği, ölçek ve yükseklik çarpanı `cad.crs.factors`'ın aynı çizgi için verdiğiyle aynı; başlangıç ve
    bitiş kotu S1'in kendisi;
  - çapraz denetimler (calc, survey_heights_tests, module_tests, ribbon_tests, history::text, catalog, settings,
    fieldbook) geçer.

  Resimler `.run/gui-evidence/poligon-kotlari-4100/`: fh = 5,6 mm (6 mm konmuştu, başucu açıları 0,1 mgon'a
  yuvarlandı), H sütunu ve her kenarın kendi yüksekliğindeki not.

## Sonra

- ~~Karşılıklı başucu açılarıyla kenar başına ortalama kot farkı.~~ ADR 4500.
- Kot kapanma hatası için proje toleransı (resmî ya da kullanıcı değeri; KentOS değer uydurmaz).
- Web (TODOS §0.2): web'in Poligon penceresi (`TraverseDialog`) aynı çekirdeği çağırır; kot sütunları orada yok.
