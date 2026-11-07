# 1740 — Arşivden GNSS ağı: bazlar, bilinen noktalar, iki aşamalı dengeleme ve kabul; `.kcad` şema 199

Tarih: 2026-11-07. plan/map-cad.md MAP-03 ve MAP-05, ikinci dilim. Durum: taslak.

İlgili ADR'ler:
- [ADR 1340](1340-gnss-campaign-archive.md): GNSS arşivi. Bu ADR oradaki “Sonra: baz vektörü ve tam kovaryanslı ağ
  dengelemesi (`adjust` bugün işarettir)” maddesini kapatır.
- [ADR 1380](1380-gnss-baseline-network-adjustment.md): baz ağı dengelemesi (`cad.survey.baselines`). Bu ADR oradaki
  “Sonra: arşivin işlenmiş bazlarından doğrudan ağ kurma; kabul edilen çözümün yazılması” maddesini kapatır.
- [ADR 0203](0203-network-adjustment.md): ortak istatistik çekirdeği (vᵀPv, m0, χ², Baarda, Cholesky pivot kuralı).

## Dayanak (7 Kasım 2026'da okundu)

- **GVX 1.0** (NGS, “GNSS Vector Exchange File Format, Narrative Description”, sürüm 1.0, 4 Şubat 2021; D. Gillins,
  I. Sellars; [belge](https://geodesy.noaa.gov/data/formats/GVX/gvx_documentation.pdf),
  [örnek](https://geodesy.noaa.gov/data/formats/GVX/sample_gvx.xml)):
  - XML'dir, kökü `GVX VERSION="1.0"`.
  - `POINT` her koordinat takımıdır: `ID`, `NAME` (işaretin adı), `ARP_HEIGHT` (işaretten ARP'ye düşey) ve `POINT_TYPE`
    (Keyed-in, Code, Float, Fixed, PartialFixed, Adjusted, PPP Solution).
  - `GNSS_VECTOR`:
    - uçlar `INITIAL_POINT_ID`, `TERMINAL_POINT_ID`; nasıl yapıldığı `SURVEY_SETUP_ID`;
    - `OBSERVATION_TIME` `START`/`END`, “yalnız GPS zamanı”;
    - `ECEF_DELTAS` `DX`, `DY`, `DZ`: “işaretten işarete” (anten yükseklikleri uygulanmış) bileşenler, metre;
    - `CORRELATION_MATRIX`: standart sapmalar `SDX`, `SDY`, `SDZ` ve korelasyonlar `PXY`, `PXZ`, `PYZ`.
  - `SURVEY_SETUP`: `SOLUTION_TYPE` (Single-baseRTK, NetworkRTK, Post-processed) ve `PROCESSING_SOFTWARE`.
  - Uzunluk birimi yalnız `meters`'tır.
  - İsteğe bağlı `SESSION` aynı oturumda işlenen vektörler arasındaki çapraz korelasyonları `CCM_BLOCK`'larla taşır.
- **Trimble Business Center** ([Understanding Network Adjustment](https://help.fieldsystems.trimble.com/tbc/1934.htm)):
  dengelenmiş değerler projeye güncel koordinat olarak kaydedilir, kalitesi “Adjusted” ya da “Fixed” olur. Her nokta
  için ek bir koordinat kaydı yapılır, dengelenmiş olan son değer olur; Point Comparison Report ile karşılaştırılır.
  ([ADR 1380](1380-gnss-baseline-network-adjustment.md) TBC'nin önce minimal, sonra tam kısıtlı akışını anlatır.)
- **Leica Infinity**
  ([How to perform a network adjustment](https://leica-geosystems.com/products/gnss-systems/software/leica-infinity/network-adjustment)):
  - önce iç kısıtlı dengeleme ölçülerin kalitesini sınar;
  - sonra kontrol noktalarıyla kısıtlı dengeleme yapılır;
  - sonuç ayrıca “saklanınca” noktalar “adjusted least squares” rolünü alır.
- **BÖHHBÜY** (RG 26.06.2018/30460 Mükerrer). Metin, Çanakkale Onsekiz Mart Üniversitesi Harita Mühendisliği'nin
  yayımladığı kopyadan okundu (Yönetmelikler, Nisan 2018, s. 6966–6969):
  - **Madde 15(1)(a):** TUTGA/TUSAGA-Aktif ve eski C1 koordinatları ölçme epokuna taşınır.
  - **Madde 15(1)(b):** C1 ölçüleri, ölçme anındaki koordinatları bilinen ve sabit alınan **bir** noktaya dayalı
    serbest dengelenir. Uyuşumsuz baz istatistikle sınanır, varsa yeniden hesaplanır ya da ölçülür. Her bağımsız bazın
    σΔX, σΔY, σΔZ ≤ ±(10 mm + 1 ppm).
  - **Madde 15(1)(c):** TUTGA/TUSAGA-Aktif'le 3B benzerlik dönüşümü, ölçek |1 − λ| ≤ ±3 ppm.
  - **Madde 15(1)(ç):** ağ TUTGA/TUSAGA-Aktif koordinatları değişmez alınarak dengelenir; σφ, σλ ≤ ±3,0 cm, σh ≤ ±5,0 cm.
  - **Madde 15(1)(d):** istatistik güven 1 − α = 0,95.
  - **Madde 20(1)(a), (b):** C2: baz bileşenleri σ ≤ ±(10 mm + 1,5 ppm) “tekli veya oturum baz çözümü ile”;
    σφ, σλ ≤ ±4,0 cm, σh ≤ ±5,0 cm.
  - **Madde 22(1)(ğ), (h):** C3: σ ≤ ±(10 mm + 1,5 ppm); σφ, σλ ≤ ±5,0 cm, σh ≤ ±6,0 cm.
- **Netcad:** GNSS ağ dengelemesinin herkese açık belgesi bu oturumda bulunamadı (ADR 1340'taki durum sürüyor).
- **KentOS'ta bugün:**
  - arşiv oturumları, çözümleri ve kabul edilen noktaları tutar, ama baz vektörü yoktur: `adjust` yalnız işarettir;
  - `cad.survey.baselines` ağı metinden dengeler, hiçbir şey yazmaz;
  - ikisi bağlı değildir.

## Karar

### 1. Arşivin yeni kayıtları (`.kcad` şema 199)

- **Baz** `bz-N` (`SurveyBaseline`). Alanları:
  - arazi ölçümü; başlangıç ve bitiş işareti (iki ayrı ad);
  - ECEF ΔX, ΔY, ΔZ (m, işaretten işarete) ve kovaryans xx, xy, xz, yy, yz, zz (m², pozitif tanımlı);
  - gözlem zamanları (GPS), çözüm türü (`postProcessed`, `singleBaseRtk`, `networkRtk`), yazılım, epok ve uydu sayısı;
  - dosyası (GVX ya da baz metni, SHA-256 revizyonuyla) ve dosyadaki kimliği;
  - yeniden işleme tarifi, dışlama nedeni, `adjust`.

  Silinmez, dışlanır.
- **Bilinen nokta** `bn-N` (`SurveyKnown`):
  - bir işaretin verilen koordinatı: enlem, boylam, elipsoit yüksekliği, coğrafi çerçeve (EPSG), epok;
  - σ kuzey/doğu/yukarı verilmişse ağırlıklı, yoksa sabit;
  - kaynak ve dışlama nedeni.

  Bir işaretin dışlanmamış tek bilinen noktası olur.
- **Dengeleme** `da-N` (`SurveyAdjustment`). Kabul edilen ağın işlem kaydıdır:
  - bazları ve bilinen noktaları; serbest aşamada tutulan nokta; varyans çarpanı; sınıf ve kural paketi
    (`bohhbuy-2018-gnss@1`) ve sonucu;
  - iki aşamanın m0'ı ve serbestliği; çerçeve ve epok.

  Kabul edilen noktanın sonucu `method: network` ve `adjustment: da-N` taşır, çözümü olmaz.
- Kurallar `map_survey_problem`'dedir; bağımsız okuyucu (`tools/kcad/kcad.py`) aynı kuralları kendisi yazar. Yazıcı 199'u
  bunlardan biri varken seçer. 198 ve öncesi `bad_value` sayar.
- Örnek ve bozuk dosyalar:
  - örnek `map-survey-network.json/.kcad`;
  - bozuklar: 198'de ağ, iki ucu aynı baz, pozitif tanımlı olmayan kovaryans, bir işarette iki bilinen nokta, arşivde
    olmayan bazı adlandıran dengeleme, çözümü olan ağ noktası;
  - “daha yeni” örneği artık `schema-version-200.kcad`.

### 2. Bazlar dosyadan ya da elle (`map.survey.edit`)

- **`baselines`:** bir dosyanın vektörlerini bir arazi ölçümüne alır.
  - **GVX 1.0:** `formats::gnss::gvx`.
    - Kovaryans SD ve P'den yeniden kurulur: xx = SDX², xy = PXY·SDX·SDY …
    - Zamanlara `Z` eklenir.
    - `SESSION`'ların çapraz korelasyonları okunmaz; uyarı `gvx_correlations_ignored`. Vektörler bağımsız
      dengelenir.
    - Sürümü 1.0 olmayan, birimi metre olmayan, korelasyonu [−1, 1] dışında olan ya da kovaryansı pozitif tanımlı
      olmayan dosya `unreadable_gnss_file` ile, satırı söylenerek reddedilir.
  - **KentOS baz metni:** `B;başlangıç;bitiş;dX;dY;dZ;xx;xy;xz;yy;yz;zz` satırları (ADR 1380 §5). K/N satırları arşive
    alınmaz, çünkü ECEF koordinatın çerçevesi ve epoku yoktur; uyarı `baseline_text_stations`.
- **`baseline`:** elle tek baz.
- **`known`:** bilinen nokta. Çerçeve kayıttaki coğrafi sistem olmalıdır (`unknown_frame`).
- **`exclude`, `include`, `adjust`:** `bz-` ve `bn-` kimliklerini de alır. Bilinen nokta dengelemeden çıkarılmaz,
  dışlanır.
- **`remove`:** bir dengelemenin kullandığı baz ve bilinen nokta, ve noktaların bugün taşıdığı dengeleme çıkmaz
  (`survey_in_use`).

### 3. Ağın çözümlemesi (`map.survey.network`, salt okunur)

- **Ağın kurulması:**
  - Girer: `adjust` olan ve dışlanmamış bazlar (`campaign` verilirse yalnız onun bazları).
  - İşaretlerin sırası, bazların adlandırdığı sıradır.
  - Bilinen noktalar o işaretlerdeki dışlanmamışlardır; hepsi aynı coğrafi çerçeve ve epokta olur (`known_differ`).
    Bu epok ölçme epokudur: Madde 15(1)(a) uyarınca bilinen koordinatı ölçme epokuna taşımak kullanıcının işidir.
  - Bilinen noktanın X, Y, Z'si çerçevenin elipsoidindedir (TUREF: GRS80); `gnss::to_ecef` ve `to_geodetic`.
- **Serbest aşama** (Madde 15(1)(b)): yalnız bir bilinen nokta (`hold`, verilmezse ilki) sabittir, ötekiler
  bilinmeyendir. Uyuşumsuz bazları ve ağın kendi duyarlığını gösterir.
- **Dayalı aşama** (Madde 15(1)(ç)):
  - σ'sız bilinen noktalar sabittir.
  - σ'lı bilinen noktalar ağırlıklıdır. Kuzey/doğu/yukarı σ'ları, noktanın enlem ve boylamında Rᵀ·diag(σe², σn², σu²)·R
    ile ECEF'te **ilişkili** bir kovaryansa döner. Çekirdek bunun için tam kovaryanslı ağırlıklı istasyonu öğrendi:
    Cholesky ile beyazlatılmış üç sözde ölçü.
- **Testler:**
  - Bazın ve ağırlıklı noktanın bileşenleri kovaryansının alt Cholesky çarpanıyla ilişkisizleştirilir (ADR 1380 §2).
  - Sınır, sınıfın güveniyle iki yanlı normal nicemdir: √χ²₁(p); C1 için p = 0,95, w > 1,96. Sınıfsız ya da güveni
    söylenmeyen sınıfta Baarda'nın α₀ = 0,001'idir (w > 3,29).
  - Model testi χ²(%95)'tir.
- **Baz satırı:**
  - başlangıcının doğu/kuzey/yukarısında artık;
  - işlenmiş σX σY σZ (varyans çarpanıyla);
  - serbest aşamada dengelenmiş vektörün σ'sı: √diag(Qtt + Qff − Qtf − Qft)·m0. Çekirdeğin `BaselineResidual.adjusted`
    alanıdır.
- **İşaret satırı:**
  - enlem, boylam, h ve σ kuzey/doğu/yukarı;
  - bilinen nokta olmayanın projedeki yeri, koordinat zincirinin kendisiyle (ADR 1340 §3'ün `place`'i);
  - kabul edilmiş noktasından farkı (dx, dy, dz): TBC'nin karşılaştırma raporu.
- **Kural paketi:** sınıflar `catalog/v1/rules/gnss-ag-bohhbuy-2018.json`'dadır. Hiçbir sayı kodda değildir; `rules` ile
  başka paket verilebilir. Denetlenenler:
  - bazın σ'sı ≤ `constant` + `ppm`·10⁻⁶·uzunluk; C1'de serbest dengelemenin, C2/C3'te işleyicinin σ'sı;
  - yeni işaretin σN, σE ≤ `horizontal`, σU ≤ `height`.

  Tutmazsa uyarı `network_rules_failed` verilir. Kabulü durdurmaz: teknik denetimdir, kurum onayı değildir.
- **`acceptable` ve `why`:** kabul şu durumlarda olmaz:
  - iki aşamadan birinde uyuşumsuz ölçü var;
  - model testi düştü;
  - bir işaret projeye taşınamıyor (ızgarasız iki epok …);
  - yeni işaret yok;
  - **aynı ağ zaten kabul edilmiş** (`current`): aynı bazlar, bilinen noktalar, tutulan, çarpan ve sınıf, ve
    noktalar hâlâ o dengelemeyi taşıyor.
- **Retler:** `not_a_map_project`, `unknown_survey_record`, `invalid_survey_value`, `unknown_rule_class`, `no_baselines`,
  `solutions_differ`, `datum_missing`, `known_differ`, `unknown_frame` ve çekirdeğinkiler (`duplicate_station`,
  `network_too_large`, `network_disconnected`: ADR 0203'ün pivot kuralıyla ilk belirsiz bilinmeyen).
  - Arşivin içeriğinden doğan ret yolsuzdur; düzenlemede yolu `operation`'dır.
  - Fazla ölçü yoksa uyarı `network_unchecked` verilir.

### 4. Kabul (`map.survey.edit network`)

- **Ret sırası:** önce aynı ağın kabulü (`nothing_to_set`); sonra uyuşumsuz ölçü (`network_blunder`; ileti en
  kötüsünü ve “dışlayın ya da yeniden işleyin”i söyler, BÖHHBÜY 15(1)(b)); sonra model testi (`network_test_failed`;
  ileti m0²·çarpanı önerir, TBC'nin skaleri); sonra zincirin kendi reddi.
- **Tek adımda yazılanlar** (“Ağ dengelemesini kabul et”):
  - `da-N`;
  - bilinen nokta olmayan her işaret: kabul edilmiş noktası varsa yeni revizyon. Eski sonuç geçmişte kalır, nesne yeni
    yerine gider, nedeni “da-N ağ dengelemesiyle üretildi”. Yoksa yeni `kn-N` ve nokta nesnesi `layer`'a yazılır.
  - nesnenin Çözüm özniteliği `da-N`, Yöntem'i “Ağ dengelemesi”.
- **Güncellik:** ona dayanan kayıtlar (`kb-N`) güncel değildir. Ağ noktası dengelemesinin bir bazı dışlanınca,
  dengelemeden çıkarılınca ya da silinince, bilinen noktası dışlanınca güncel değildir; nedeni yazılıdır.
- **Yeniden üretme:** ağ noktası çözümsüz yeniden üretilmez (ağı yeniden dengeleyip kabul edin); çözüm verilirse
  çözümden yeniden üretilir.

### 5. Masaüstü: Arşivden ağ dengelemesi (`map.survey.adjustment`; yalnız KentOS Harita)

- **Yer:** Ölçme › GNSS arşivi › Ağı dengele (simge `gnssNetworkAccept`: ağ, dolu üçgen bilinen nokta, kabul işareti;
  seçenek sayfası `.run/gui-evidence/ag-dengelemesi-1740/secenekler.png`).
- **Form:** Baz içe aktar… (son arazi ölçümüne); Sınıf (Sınıf yok, C1, C2, C3); Varyans çarpanı.
- **Özet:** iki aşamanın m0'ı, χ² testi ve uyuşumsuzları; sınıfın sonucu; kabul edilebilirlik ya da nedenler;
  “teknik denetimdir, kurum onayı değildir”.
- **Tablolar:**
  - Noktalar: rol (sabit, ağırlıklı, yeni), σ, ΔY/ΔX/ΔZ (mm), sınıf, kabul edilen;
  - Bazlar: arşivin bütün bazları. Dengeye girenin testi gösterilir. Dışla nedeni testin sözüdür; Geri al geri alır.
  - Bilinen noktalar ve eşit kutulu ekleme formu.
- **Kabul et:** yalnız kabul edilebilirken açıktır; tek geri alma adımıdır.
- **CBS:** pencereyi sunmaz; ağı Python ve MCP'den `map.survey.network` ile okur.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/map_survey_network_cases.py`, KentOS kodu yok. Araçlar: numpy'nin tam
  kovaryanslı genelleştirilmiş en küçük kareleri (P = Σ⁻¹), scipy'nin χ² ve normal nicemleri, PROJ'un TUREF
  coğrafi ↔ yer merkezli dönüşümü, TM30 için crs_vertical_cases.py, GVX için Python'un ElementTree'si.
  - **`map.survey.network`, 30 durum:**
    - Ankara çevresinde (N1'in kabul edilmiş yerinin çevresinde) iki bilinen nokta, üç işaret, sekiz baz;
    - C1, C2, C3; çarpan 100 ile sınıfın tutmaması;
    - çarpan 0,07 ile model testinin düşmesi, uyuşumsuz baz yokken;
    - 8 cm'lik kaba hata; ağırlıklı bilinen nokta; `hold`;
    - dışlanan ve dengeden çıkan baz; iki arazi ölçümü; fazlalıksız ağ;
    - ölçme epokunda bilinen noktalar (zincir taşıyamaz);
    - kabul edilmiş ağ ve kabulden sonra değişen ağ;
    - plan, doğrulama ve 13 ret.
  - **`map.survey.edit`, 33 yeni durum** (dosya 109 durum):
    - GVX'ten ve baz metninden bazlar, GVX 2.0 reddi, elle baz ve retleri;
    - bilinen nokta ve retleri; dışla, dengeden çıkar;
    - ağın kabulü (C2 ile, çarpan 100 ile, fazlalıksız);
    - kabulden sonra dışlanan baz ve güncel olmayan üç nokta; çıkarılamayan kayıtlar; çözümsüz yeniden üretme;
    - aynı ağın ikinci kabulü; uyuşumsuz baz, model testi, kilitli katman, zincir, yeni işaretsiz ağ, bilinen
      noktasız, bazsız, farklı epoklu, bilinmeyen sınıf, parçalı ağ, katlanınca aynı ad.
  - Sayılar 1e-6 içindedir.
  - Rust'ta bütün komutların ortak durumları geçer (75 s). Python'da `map.survey.*` ve `cad.survey.*` `.kcad` şema 199
    gidiş-dönüşüyle geçer.
- **.kcad:** bağımsız okuyucu 1673 başvuru dosyasında `expected.json` ile aynıdır. Rust `kentos-kcad` 242 test geçer;
  geçerli dosya sayısı 230'dur ve Rust yazıcısı yeni örneği başvuruyla bayt bayt aynı yazar.
- **GVX okuyucusu:** `formats::gnss::gvx` birim testleri (kovaryans, işaretler, zamanlar, yazılım; sürüm, birim,
  korelasyon, kayıp nokta, aynı uç, çapraz korelasyon uyarısı).
- **Masaüstü** (`survey_network_window_tests`, 5 test):
  - ortak durumun ağı; C2; Kabul et tek adımda ve `.kcad` 199 olarak yazılıp geri okunur;
  - Dışla ve Geri al;
  - boş arşive GVX ve bilinen nokta formu;
  - eksik σ reddi;
  - Harita sunar, CBS ve Mimari sunmaz.

  Resimler `.run/gui-evidence/ag-dengelemesi-1740/` altındadır. Ortak masaüstü denetimleri geçer: `module_tests`,
  `ribbon_tests`, `history::text`, `catalog`, `settings`, arşiv paneli ve baz ağı penceresi (142 test).
- **MCP:** `kentos-mcp` testleri geçer.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("ag.kcad")
cad.map.survey.edit(doc, operation={"op": "baselines", "campaign": "kp-1", "file": "/veri/ag-2026-11.gvx"})
cad.map.survey.edit(doc, operation={"op": "known", "mark": "ANKR", "lat": 39.887450, "lon": 32.758565,
                                    "height": 976.012, "srid": 5252, "epoch": 2026.84, "source": "TUSAGA-Aktif"})
r = cad.map.survey.network(doc, class_="C2")
if r.acceptable:
    cad.map.survey.edit(doc, operation={"op": "network", "class": "C2", "layer": "gnss"})
```

MCP: `{"tool": "map.survey.network", "arguments": {"class": "C2"}}` salt okunurdur. Kabul `map.survey.edit`'in
`network` işlemidir.

## Sonra

- **Çapraz korelasyon:** GVX `SESSION`'larının vektörler arası korelasyonları; oturum başına tam kovaryansla
  beyazlatma.
- **Madde 15(1)(c):** TUTGA/TUSAGA-Aktif'le 3B benzerlik dönüşümü ve ölçek testi.
- **Epok:** bilinen noktaların hız ızgarasıyla ölçme epokuna kendiliğinden taşınması; farklı epoklu bilinen noktalar.
- **Ölçek:** seyrek çözüm ve büyük ağ ölçümü (MAP-05'in kalanı); robust kestirim.
- **Okuyucular:** SINEX ve RTK denetleyicisinin vektörlerinin okunması (JobXML …).
- **Arşiv paneli:** bazların ve dengelemelerin panelde gösterilmesi.
- **Kabul:** gerçek kullanıcı projesinde GUI kabulü; web.
