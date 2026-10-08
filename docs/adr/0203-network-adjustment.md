# ADR 0203: Ağ dengelemesi ve kot ağı

- **Durum:** kabul edildi (2026-10-08). Kapsam sahibin kararlarıdır (8 Ekim): en küçük kareler yatay ağ dengelemesi ve kot ağı
  (nivelman); gözlemlerin önsel doğrulukları Proje ayarları › Ölçme'de, değerlerini sahip verir; dengelenmiş noktalar aynı adlı
  noktaları günceller, yenileri eklenir. Prizmatik aplikasyon ve kanava çizimi eski teknikler olarak iptal edildi. Simgeler sahibin
  seçtikleridir (8 Ekim, ikisi de önerilen seçenek). Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-05` (ve araştırma notu), ADR 0070 ve 0071 (Hesap pencereleri), ADR 0169 (saha verisi, Karne
  editörü, Ölçme'nin toleransları), ADR 0171 (zemin, elipsoit ve düzlem; uzunlukların projeksiyona indirilmesi), ADR 0153 (nokta
  editörü: bağlı çizgilerin izlemesi), ADR 0156 (Vektör oturtma: artıklar ve m0), ADR 0025 (`.kcad` v2).

## Bağlam

Hesap pencereleri tek bir yolu hesaplar: Poligon hesabı kapanmayı pusula kuralıyla dağıtır, Kutupsal alım ve kestirmeler fazla ölçüyü
kullanmaz. Birden çok poligonun düğümlendiği, aynı noktaya iki yönden bakılan, kenarın iki ucundan ölçüldüğü ağlarda bütün ölçüler en
küçük kareler ile birlikte dengelenir; sonuçla birlikte noktaların doğrulukları (hata elipsleri), ölçülerin artıkları ve uyuşumsuz ölçünün
testi verilir. Netcad'in Ağ dengeleme modülü, Star*Net, Columbus ve ArcGIS'in Least Squares Adjustment'ı bu işi yapar. Kot ağı
(nivelman ve trigonometrik kot farkları) aynı yöntemin bir boyutlusudur.

## Karar

### 1. Önsel doğruluklar (Proje ayarları › Ölçme)

Projenin ölçme ayarına altı değer eklenir (`SurveySettings`, `.kcad` şema 28); yazılmayan değerin yerine varsayılan kullanılır:

| Alan | Anlamı | Birim (saklanan) | Formda | Varsayılan |
|---|---|---|---|---|
| `sigmaDirection` | doğrultu ölçüsünün standart sapması | radyan | cc (grad projede), ″ (derece) | 10 cc (π/200 000 rad, ≈ 3,24″) |
| `sigmaDistance` | kenarın sabit payı | m | mm | 2 mm |
| `sigmaPpm` | kenarın uzunlukla artan payı | ppm | ppm | 2 |
| `sigmaCentering` | alet ve hedefin her birinin merkezlemesi | m | mm | 1 mm |
| `sigmaZenith` | başucu açısı | radyan | cc, ″ | 10 cc |
| `sigmaLevelling` | geometrik nivelmanın kilometre başına standart sapması | m/√km | mm/√km | 2 mm/√km |

`sigmaDirection`, `sigmaDistance`, `sigmaZenith` ve `sigmaLevelling` sonlu ve sıfırdan büyük, `sigmaPpm` ve `sigmaCentering` sonlu ve
sıfırdan küçük olmayan sayıdır; başkası reddedilir. Varsayılanlar 3″ sınıfı total station ve mühendislik nivelmanıdır; projenin
değerlerini sahip verir. Formda boş alan varsayılanı yer tutucu olarak gösterir.

### 2. Yatay ağ: noktalar, gözlemler, ağırlıklar

- **Noktalar.** Bilinen noktalar sabit ya da ağırlıklıdır: ağırlıklı noktanın σ'sı (Y ve X'te aynı) verilir. Öbür bütün noktalar yeni
  noktadır. Nokta adları baştaki ve sondaki boşluklar atılarak, harfler Türkçe katlanarak (i/İ, ı/I) karşılaştırılır.
- **Gözlemler.** Her satır bir durulan ve bir bakılan noktayla bir doğrultu, bir kenar ya da ikisini verir. Doğrultu projenin açı
  biriminde yatay okumadır; bir istasyonun bütün doğrultuları bir seridir ve bir yöneltme bilinmeyeni alır. Kenar yatay uzunluktur (m).
  Proje uzunlukları projeksiyona indiriyorsa (ADR 0171 §4) her kenar, hattının o anki koordinatlarda hesaplanan ölçek ve yükseklik
  çarpanıyla düzleme indirilir; yoksa yazıldığı gibidir. Doğrultulara yay-kiriş düzeltmesi yapılmaz (birkaç kilometreden kısa hatlar).
- **Ağırlıklar.** σ₀ = 1 (birimsiz); her gözlemin ağırlığı p = 1/σ². Hattın o anki uzunluğu s ile doğrultunun σ² = σ_d² + 2(σ_m/s)²,
  kenarın σ = σ_k + σ_ppm·10⁻⁶·s ve σ² = σ² + 2σ_m² (σ_d doğrultu, σ_k kenarın sabit payı, σ_m merkezleme). Ağırlıklı nokta Y ve X için
  birer sözde gözlemdir (σ'sıyla). Ağırlıklar her yinelemede o anki koordinatlarla yeniden hesaplanır; sonuç yaklaşık değerlerden
  bağımsızdır.

### 3. Yaklaşık koordinatlar

1. Bilinen noktalar kendi koordinatlarıyla; pencerenin verdiği, çizimde aynı adlı noktası olan yeni noktalar çizimdeki yerleriyle başlar.
2. Kalanlar turlarla, hiçbir şey değişmeyene kadar hesaplanır; her tur sırasıyla:
   a. **Yöneltme:** yeri bilinen ve yöneltmesi olmayan istasyon, yeri bilinen hedeflerine doğrultularından yöneltilir: z = t − r'lerin
      ilkine göre (−π, π]'ye katlanmış farklarının ortalaması, ilkine eklenerek (t semt, r okuma).
   b. **Kutupsal:** yöneltilmiş istasyondan doğrultusu ve kenarı olan, yeri bilinmeyen hedef: P = S + d·(sin(r+z), cos(r+z)) (satır sırasıyla).
   c. **Serbest istasyon:** yeri bilinmeyen istasyonun yeri bilinen, birbirinden ayrı iki hedefine doğrultusu ve kenarı varsa (satır
      sırasıyla ilk ikisi): yerel kutupsal noktalarından iki noktalı dönüklük θ = semt(K₁K₂) − semt(l₁l₂), S = K₁ − d₁·(sin(r₁+θ), cos(r₁+θ)),
      yöneltmesi θ.
   d. **Önden kestirme:** yeri bilinmeyen hedefe iki yöneltilmiş, yeri bilinen istasyondan yalnız doğrultu varsa (satır sırasıyla, kesişme
      açısı 1 gon'dan büyük ilk çift) iki ışının kesişimi.
3. Yine de yeri bulunamayan nokta (yalnız kenarlarla bağlı nokta gibi) reddedilir, adıyla: çizimde yaklaşık yeriyle bulunması ya da
   gözlem eklenmesi istenir.

### 4. Dengeleme ve istatistikler

- **Bilinmeyenler:** önce ağırlıklı noktaların (bilinenler tablosunun sırasıyla), sonra yeni noktaların (gözlemlerde ilk görünme
  sırasıyla: satır satır durulan, sonra bakılan) Y ve X'i, ardından istasyonların yöneltmeleri (ilk görünme sırasıyla).
- **Düzeltme denklemleri** (Y doğu, X kuzey, semt kuzeyden saat yönünde): doğrultu r + v = t(S, H) − z; kenar s + v = √(ΔY² + ΔX²);
  sözde gözlem y + v = Y. Gauss–Newton yinelemesi: N = AᵀPA, n = AᵀPℓ (ℓ = ölçü − hesap, doğrultuda (−π, π]'ye katlanmış), Δ = N⁻¹n
  Cholesky ile. Koordinat düzeltmelerinin en büyüğü 10⁻⁷ m'den, yöneltmelerinki 10⁻¹⁰ rad'dan küçük olunca durulur; 30 yinelemede
  durmazsa ret.
- **Dayanak:** N tekilse (Cholesky'nin bir köşegeni sütununun ilk köşegeninin 10⁻¹⁰ katından küçük) ret, belirlenemeyen bilinmeyenin
  adıyla (“P7'nin Y'si”, “S3'ün yöneltmesi”): en az iki sabit ya da ağırlıklı nokta ve noktayı belirleyen gözlemler gerekir.
- **Artıklar** son koordinatlarda: doğrultuda v = katla(t − z − r), kenarda v = s_hesap − s_ölçü (düzleme indirilmiş), sözde gözlemde
  v = Ŷ − y. Ω = vᵀPv, n gözlem sayısı (sözde gözlemler dahil), u bilinmeyen sayısı, f = n − u; f > 0 ise m₀ = √(Ω/f).
- **Doğruluklar:** Q = N⁻¹ son koordinatlarda; σ̂ = m₀ (f = 0 ise 1, söylenir). Noktanın σY = σ̂√q_YY, σX = σ̂√q_XX,
  konum hatası σP = √(σY² + σX²); hata elipsi (1σ) λ₁,₂ = (q_YY + q_XX)/2 ± √(((q_YY − q_XX)/2)² + q_YX²), a = σ̂√λ₁, b = σ̂√λ₂, büyük
  eksenin semti θ = ½·atan2(2q_YX, q_XX − q_YY), [0, π)'ye katlanmış.
- **Uyuşumsuz ölçü testi (Baarda):** q_vv = 1/p − aQaᵀ, katkı (redundancy) r = p·q_vv; r < 10⁻⁴ olan gözlem denetlenemez (testi yok);
  öbürlerinde w = |v|/√q_vv (σ₀ = 1, önsel) ve w > 3,29 (α₀ = 0,001, iki yanlı) “uyuşumsuz”. Küçük ağda bir kaba hata komşu ölçüleri
  de işaretletir; uyuşumsuzların en büyük w'lusu ayrıca söylenir (önce o denetlenir, veri yoklaması).
- **Model testi:** Ω ≤ χ²₀,₉₅(f) ise geçer (α = 0,05); χ² sınırı düzenli eksik gama işlevinin ikiye bölmeyle tersi (f > 0).

### 5. Kot ağı

- **Noktalar** yatay ağdaki gibi sabit ya da ağırlıklı (σ, m); **gözlemler** başlangıç, bitiş, kot farkı ΔH (m) ve uzunluk L (m, > 0).
- **Ölçü türü** pencerenin seçimidir: geometrik nivelmanda σ = σ_n·√(L/1000); trigonometrikte σ² = (L·σ_z)² + ((ΔH/L)·(σ_k +
  σ_ppm·10⁻⁶·L))² (L yatay uzunluk).
- **Yaklaşık kotlar** bilinenlerden satır sırasıyla yayılır (ucu bilinen satır öbür ucu verir); bilinenlere bağlı olmayan nokta reddedilir.
- **Düzeltme denklemi** ΔH + v = H_bitiş − H_başlangıç; doğrusal, tek çözüm. Artıklar, m₀, σH, katkı, w ve model testi §4'teki gibi.

### 6. Karne editöründen aktarma

- **Ağ dengelemesine aktar:** kullanılan bütün istasyonların indirgenmiş satırları (ADR 0169 §3): durulan, bakılan, doğrultu (iki durumun
  ortalaması, projenin biriminde) ve yatay uzunluk (eğik uzunluk ve başucu açısı varsa). Bir kenarın iki ucundan ölçüsü iki gözlemdir.
- **Kot ağına aktar:** kot farkı olan satırlar: durulan, bakılan, kot farkı (yer eğriliği ve refraksiyonla), yatay uzunluk; ölçü türü
  trigonometrik.
- Bilinen noktalar tablosu değişmez; pencere açılır.

### 7. Pencereler

Hesap pencereleri ailesinde iki pencere, Hesap pencerelerinin tablo, özet ve rapor kurallarıyla (ADR 0070, 0071):

- **Yatay ağ dengelemesi:** Bilinen noktalar tablosu (Ad, Y, X, σ (mm); Y ve X boşsa çizimdeki aynı adlı nokta; σ boşsa sabit), Gözlemler
  tablosu (Durulan, Bakılan, Doğrultu, Kenar (m)), önsel doğrulukların satırı (Ölçme ayarlarından), en çok altı sorunluk özet; sonra
  dengeleme satırı (n, u, f, yineleme, m₀, model testi) ve iki sonuç tablosu: noktalar (Ad, Y, X, σY, σX, σP, a, b, θ, çizimdeki yerinden
  kayma) ve gözlemler (Durulan, Bakılan, Tür, Ölçü, v, σ, r, w; uyuşumsuz satırlar tehlike renginde). Altta Raporu kopyala, Katman (yeni
  noktalar için), Çizime yaz, Kapat.
- **Kot ağı dengelemesi:** Ölçü türü (Geometrik nivelman, Trigonometrik), Bilinen noktalar (Ad, Kot (m), σ (mm); kot boşsa çizimdeki
  aynı adlı noktanın kotu), Gözlemler (Başlangıç, Bitiş, Kot farkı (m), Uzunluk (m)); sonuçta noktalar (Ad, Kot, σH, çizimdeki kotundan
  fark) ve gözlemler (Başlangıç, Bitiş, Ölçü, v, σ, r, w).
- Hesap her değişiklikte yeniden yapılır; açılar projenin açı biriminde, artıklar cc ya da ″ ve mm ile gösterilir.

### 8. Çizime yazma

- Yalnız yeni noktalar yazılır (bilinen sabit ve ağırlıklı noktalara dokunulmaz).
- **Yatay:** çizimde aynı adlı nokta (etiketi ya da `Ad` özniteliği, §2'nin karşılaştırmasıyla) varsa yeri dengelenmiş yere taşınır;
  çizgilerin, çoklu çizgilerin ve alanların o noktada (1 µm içinde) duran köşeleri onunla gider (nokta editörünün kuralı, ADR 0153 §3),
  kotlar kalır. Çizimde yoksa seçilen katmana yeni nokta eklenir (etiketi adı; `Ad` ve `Tür`: “Ağ noktası” öznitelikleriyle).
- **Kot:** çizimde aynı adlı noktanın kotu dengelenmiş kot olur, `Z (m)` özniteliği varsa o da (üç basamak); o noktadaki köşelerin
  kotları da. Çizimde olmayan nokta yazılmaz, söylenir.
- Hepsi pencerenin adında tek geri alma adımıdır: `cad.entities.edit`'in `networkAdjust` ve `levelAdjust` işlemleri (güncellemeler),
  `cad.entities.create`'in `networkAdjust`'ı (yeni noktalar), `cad.entities.set` (`Z (m)`) bir belge grubunda. Kilitli katmandaki nesneye
  dokunan yazım bütünüyle reddedilir.

### 9. Komutlar ve şerit

- `calc.network` **Yatay ağ dengelemesi…** (AGDENGELE, AĞDENGELE, YATAYAG, DENGELEME, NETWORK) ve `calc.levelNetwork` **Kot ağı
  dengelemesi…** (KOTAGI, KOTAĞI, NIVELMAN, NİVELMAN, LEVELNETWORK).
- Ölçme menüsünün yeni bölümü Dengeleme; CBS şeridinin Ölçme sekmesinde Dengeleme paneli, CAD'de Ölçme ▾'in altında.
- Kapsam dışı: 3B (birleşik yatay ve düşey) dengeleme, GNSS baz vektörleri, serbest ağ (iç dayanak), yönetmeliğin sınırları (sahibin
  tarifiyle, `GIS-07` ile birlikte), hata elipslerinin çizime yazılması.

## Uygulama

- **Sözleşme** `crates/shared/contracts/src/document.rs`: `SurveySettings`'in altı önsel doğruluğu (`sigma_holds`, `sigma_part_holds`,
  `has_sigmas`, `sigmas`, ret iletileri, `sanitized`), `SurveySigmas`, `SIGMA_DEFAULTS`, `ProjectSettings::sigmas`; `cad_edit.rs`'in
  `EditOperation::NetworkAdjust` ve `LevelAdjust`'ı, `cad_create.rs`'in `CreateOperation::NetworkAdjust`'ı; `FORMATS_VERSION` 38.
- **Çekirdek** `crates/shared/geometry-core/src/survey/adjust/`: `mod.rs` (önsel doğruluklar, adların karşılaştırması, gözlemlerin
  istatistikleri: Ω, f, m₀, katkı, w, uyuşumsuzların en kötüsü, model testi; en çok 1 500 bilinmeyen, 30 yineleme), `linalg.rs` (normal
  denklemler, Cholesky ve tekilliğin bilinmeyeni, ters), `chi2.rs` (χ² sınırı), `horizontal.rs` (yaklaşık koordinatların turları,
  düzeltme denklemleri, Gauss–Newton, hata elipsleri; işlem `networkAdjust`), `levelling.rs` (işlem `levelAdjust`); Karne editörünün
  satırları `survey/fieldbook.rs`'in `network_rows` ve `level_rows`'u (işlemler `fieldNetwork`, `fieldLevels`).
- **`.kcad`** şema 28: kodek `crates/shared/kcad`, `docs/specs/kcad-v2.md`, bağımsız okuyucu `tools/kcad/kcad.py` ve yazıcı
  `scripts/fixtures/kcad_v2_reference.py`; örnek `fixtures/kcad/v2/survey-sigmas.kcad`, üç bozuk dosya (`broken/survey-sigma-*.kcad`).
- **Proje ayarları › Ölçme** Ağ dengelemesi grubu: form kuralları `crates/native/project/src/survey_form.rs` ve web
  `model/surveyForm.ts` (14 alan, boşta varsayılan yer tutucu), pencereler masaüstünde `project/survey.rs`, web'de
  `ui/settings/ProjectSettingsDialog.ts`; web `model/projectSettings.ts`'in `surveySigmas`, `hasSigmas`'ı.
- **Web**: çekirdeğin çağrıları `model/geom/networkAdjust.ts`; iki pencere `ui/calc/NetworkDialog.ts` (tablolar, okuma kuralları ve
  iletiler masaüstününkiyle aynı, özet, sonuç tabloları, rapor, Çizime yaz), Karne editörünün iki düğmesi `ui/calc/FieldBookDialog.ts`;
  komutlar `app/calc.ts`, Ölçme menüsünün Dengeleme bölümü `app/menus.ts`, CAD'in Ölçme ▾'i `app/ribbon.ts`, simgeler `ui/icons.ts`
  (`surveyNetwork`, `surveyLevel`), görünüş `styles/calc.css`.
- **Masaüstü** `apps/desktop/src/calc/network/`: `mod.rs` (tablolar, okuma, çözüm, olaylar, rapor), `view.rs` (iki pencere), `write.rs`
  (Çizime yaz); işaretli satırlı sonuç tablosu `calc/parts.rs`'in `result_table_marked`'ı; Karne editörünün aktarmaları
  `calc/fieldbook/`; katalog `catalog.rs`.
- **Ürün komutları** iki platformda: `cad.entities.edit`'in `networkAdjust` ve `levelAdjust`'ı, `cad.entities.create`'in `networkAdjust`'ı;
  adımlar pencerelerin adıyla. Python SDK'sının tipleri (`python/kentos/cad/types.py`) katalogdan.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/network_adjust_cases.py` (ADR'den, KentOS kodu olmadan; gözlemler gerçek yerlerden sabit
  sözde rastgele hatayla; yaklaşık değerler §3'ün adımlarıyla; mpmath ile 50 basamakta Gauss–Newton, normal denklemler LU ile çözülür,
  çekirdek Cholesky kullanır; istatistikler mpmath'in tersinden, χ² sınırı düzenli eksik gamanın ikiye bölmesiyle; projeksiyona indirmenin
  çarpanları `ground_survey_cases.py`'nin PROJ ve GeographicLib'inden): `fixtures/network-adjust/v1/cases.json`,
  18 durum: bağlı poligon, çaprazlı dörtgen, ağırlıklı kontrol, uyuşumsuz kenar, serbest istasyon (küçük harfli ve boşluklu ad), önden
  kestirme, yaklaşık yerleri çizimden trilaterasyon ve onun reddi, tek bilinen noktanın reddi, derece, denetimsiz (f = 0), projeksiyona
  indirme (TM30, 850 m), durulan ve bakılan aynı reddi; nivelman halkaları, ağırlıklı reper, uyuşumsuz kot farkı, trigonometrik, bağlı
  olmayan noktanın reddi; ayrıca χ² sınırları ve varsayılanlar. Çekirdeğin `tests/all/network_adjust.rs`'i ve web'in
  `model/geom/networkAdjust.test.ts`'i (WASM) bütün durumları oynatır; ikisi de geçer. Başvuruya bilerek konan iki hata (merkezlemenin
  çarpanı, θ'nın işareti) yakalandı; yaklaşık değerlerin bir değişikliği sonucu değiştirmedi (sonuç yaklaşık değerlerden bağımsız).
- Proje ayarları › Ölçme'nin form kuralları `scripts/fixtures/survey_form_cases.py` (`fixtures/project/v1/survey-form.json`) iki
  platformda; `.kcad` şema 28: `crates/shared/kcad/tests/all/survey_sigmas.rs` ve bağımsız Python okuyucusu ile yazıcısı, örnek dosyalar
  bayt bayt.
- Komutlar: `fixtures/commands/v1/cad.entities.edit.json`'da `networkAdjust` ve `levelAdjust` (yer, sonra kot; adımlar ve geri alma),
  `cad.entities.create.json`'da `networkAdjust` (adlı noktalar, “Ağ noktası”, geri alma ve yineleme); iki platformda geçer.
- Masaüstünün testleri (`calc/network/tests.rs`, sahne `fixtures/interaction/v1/network-adjust.kcad`): K1 ve K2 adlarıyla çizimden,
  sonuç başvurunun; uyuşumsuz kenarın adı, işareti ve özeti; okuma iletileri ve çekirdeğin reddi; Çizime yaz'da Y1 ve üstündeki köşe
  taşınır, Y2 Poligon'a eklenir, tek geri alma adımı; Kot ağında yükseklikler, `Z (m)` ve köşenin kotu, çizimde olmayan N3'ün iletisi;
  hücre değişince yeniden çözüm; Karne editörünün iki aktarması. Web'de aynı sahne `shots.mjs network`'te tablolar elektronik tablodan
  yapıştırılarak doldurulur; özetler, işaretli satırlar, iki pencerenin Çizime yaz'ı ve geri alma adları sahnelerde denetlenir.
- Resimler `ag-*` (masaüstünde `calc::network::tests::screens`, web'de `shots.mjs network`; 1440 × 900 ve 1100 × 650, iki tema) ve CBS
  şeridinin Ölçme sekmesi (`serit-cbs-survey-*`).
- Süre (`network_adjust::timing`, release): ilk satırı bilinen ızgara ağ, birkaç cc ve mm hatayla: 12 × 12 (408 bilinmeyen, 792 gözlem,
  üç yineleme) 0,07 s; 20 × 20 (1 160 bilinmeyen, 2 280 gözlem) 1,6 s. Pencere her değişiklikte yeniden hesapladığından bin
  bilinmeyene yaklaşan ağda yazmak gecikir; olağan ağlar (birkaç yüz bilinmeyen) anında çözülür.
