# 1340 — KentOS Harita: GNSS arazi ölçümü ve ölçü veri arşivi

Tarih: 2026-11-06. plan/map-cad.md MAP-03. Durum: taslak; ilk dilim (MAP-03.1) aşağıda, kalanlar “Sonra”da. Madde
`[ ]` kalır. Canlı alıcı (HYB-24) ertelendi; bu ADR canlı aygıt özelliği teslim etmez.

## Dayanak (6 Kasım 2026'da okundu)

- **Ölçü verisi çizimden ayrıdır.** Civil 3D'nin ölçüm veritabanı çizime bağlı değildir, çizimin dışındadır; kontrol
  noktalarını, gözlemleri, poligon tanımlarını ve ekipman verisine dayanan standart sapmaları tutar; çizime elle ya
  da veritabanı güncellenince eklenir ([About Survey Databases](https://help.autodesk.com/cloudhelp/2022/ENG/Civil3D-UserGuide/files/GUID-E0B6AAE2-A5EC-438F-8687-B5E023F85643.htm)).
  Ağ (network) kontrol noktalarını, kurulumları (setup: arazide bir alet oturumu) ve gözlemleri taşır; ekipman
  veritabanı aletin ölçme standart sapmalarıdır ([Survey database fundamentals, WisDOT](https://c3dkb.dot.wi.gov/Content/c3d/srvy/srvy-db-fndmntl.htm)).
  İçe aktarma olayı (import event) içe aktarılan ağı, şekilleri ve noktaları adlandırır; yeniden içe aktarılabilir,
  silinince verisi de gider ([About Survey Import Events](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-5A134AA3-563C-446C-8E51-3656F760AFB7.htm)).
- **Gözlem silinmez, devre dışı bırakılır.** Trimble Business Center'da baz devre dışı bırakılabilir; devre dışı baz
  işlenmez, işlenmiş bazın vektörü temizlenir ([Enable and Disable Baselines](https://help.fieldsystems.trimble.com/tbc/4062.htm),
  [Process Baselines](https://help.fieldsystems.trimble.com/tbc/3252.htm)). Noktanın koordinatı ona bağlı ham
  gözlemlerden hesap motoruyla türetilir; Point Derivation Report noktaya giren bütün gözlemleri gösterir; gözlem açılıp
  kapatılır, F4 ile yeniden hesaplanır ([TBC tip 34: comp engine ve point derivation report](https://community.trimble.com/blogs/joe-blecha1/2020/11/18/tip-of-the-week-34-the-power-of-the-comp-engine-and-the-point-derivation-report)).
  Oturum düzenleyicisi uyduları ve kötü veri aralıklarını maskeler; baz işleme ayarları işlemeden önce seçilir
  ([TBC tip 237](https://community.trimble.com/blogs/anna-zapel/2025/02/24/tbc-tip-237-baseline-processing-and-session-editor)).
- **Aynı ada birden çok ölçü, ortalama ve dışlama.** Trimble Access aynı adlı konumları ortalar (öntanımlı ağırlıklı);
  Ayrıntılar'da tek tek konum ortalamaya alınır ya da dışarıda bırakılır, her konumun ortalamaya göre artığı
  gösterilir; yeni gözlem eklenince ortalama kendiliğinden güncellenmez, yeniden hesaplanır
  ([Compute average](https://help.fieldsystems.trimble.com/trimble-access/2024.10/en/cogo-compute-average.htm)).
  Carlson SurvPC'de aynı nokta adıyla saklanan her ölçü yeni ölçüdür; ortalamaya etkin ölçülerin hepsi girer
  ([Average GNSS](https://help.carlsonsw.com/en/survpc/main-menu/equip/configure/average-gnss/average-gnss.html)).
  Leica Infinity'de “Averaged Point” iki ya da daha çok ölçülmüş noktadan türetilen noktadır; nokta rolü ölçünün
  türünü ve hassasiyetini söyler (Leica Infinity User Guide v3.1.1).
- **RINEX başlığı.** RINEX 3.05 (IGS/RTCM, 1 Aralık 2020) gözlem başlığında `ANTENNA: DELTA H/E/N` işaretten anten
  referans noktasına (ARP) düşey yüksekliği ve doğu/kuzey dış merkezliği metre olarak verir; `TIME OF FIRST OBS` ve
  `TIME OF LAST OBS` zamanı ve zaman sistemini (GPS, GLO, GAL, BDT, QZS, IRN) taşır; etiket 61–80. sütunlardadır
  ([RINEX 3.05](https://files.igs.org/pub/data/format/rinex305.pdf), [RINEX 4.00](https://files.igs.org/pub/data/format/rinex_4.00.pdf)).
- **NMEA GGA.** Kalite 4 RTK sabit (fixed), 5 RTK kayan (float) çözümdür; bazı üreticiler 5'i başka kiplere de verir;
  düzeltme yaşı saniyedir ([NMEA-0183 GGA, Trimble](https://help.fieldsystems.trimble.com/sps/nmea0183-messages-gga.htm)).
  GGA'nın yüksekliği antenin ortalama deniz yüzeyinden yüksekliğidir. HDOP birimsiz geometri ölçüsüdür, metre
  değildir; GGA hata kestirimi taşımaz.
- **Netcad.** Netcad'in GNSS/ölçü modüllerinin herkese açık belgesi bu oturumda bulunamadı (wiki.netcad.com.tr arama
  sonuçlarında çıkmadı). Bir Netcad komutunun ya da ayarının adı buraya yazılmadı; karar yukarıdaki belgelere dayanır.
- **KentOS'ta bugün:** GPX/NMEA okuyucusu (`formats::gnss`, ADR 0169) konumları fix, uydu, HDOP ve yükseklikleriyle
  okur; GNSS içe aktarma (ADR 0169 §6) onları noktaya çevirir ama oturum, anten, çözüm ve kabul ayrımı yoktur; aynı
  işaretin ikinci günkü ölçüsü yeni bir nokta olur ya da eskisinin üstüne çizilir. Harita işi (ADR 1200) bir GNSS
  arazi ölçümünü öğe olarak adlandırır, içini tutmaz. Koordinat zinciri (`cad.crs.transform`, ADR 1230) epokları hız
  ızgarasıyla taşır, ızgarasız iki epoku reddeder.

## Karar

### 1. Ne CAD'in, ne GIS'in (modül işareti)

| Özellik | KentOS Harita (CAD) | KentOS GIS | Neden |
|---|---|---|---|
| GNSS arşivi paneli (arazi ölçümü, ölçüm oturumu, çözüm düzenleme, dışlama, kabul, yeniden üret) | var | yok | Ölçü defteri işi; plan: “GIS'e çizim karnesi penceresi taşımak zorunlu değildir”. |
| `map.survey` (paneli aç), `map.survey.import` (oturum içe aktar) | var | yok | Düzenleme Harita'nındır (`modules.only: ["harita"]`). |
| `map.survey.select` (kabul edilen ölçü noktalarını seç) | var | var | Kabul edilen noktalar GIS'te okunur (`modules.only: ["harita", "cbs"]`). |
| Öznitelikler › GNSS ölçümü (kabul edilen noktanın kalitesi, çözümleri, durumu; salt okunur) | var | var | Plan: kalite ve kaynak ilişkisi GIS'te okunsun (`PropertyRows::Survey`). |
| `map.survey.query` (Python, MCP) | var | var | Salt okunur; salt okunur kullanıcı da okur. |
| `map.survey.edit` (Python, MCP) | var | — | Düzenleme komutu; harita projesinin ayarını değiştirir. |

Kabul edilen nokta sıradan nokta nesnesidir; öznitelikleri (Ad, Kabul, Revizyon, Yöntem, Çözüm, Arazi ölçümü) GIS'in
tablosunda ve dışa aktarmasında görünür. Arşiv yalnız `cadWork: map` projesinin ayarıdır; GIS onu harita projesinden okur.

### 2. Veri modeli: `settings.mapSurvey` (`MapSurvey`), Harita işinden ayrı

Harita işinin (`mapWork`, ADR 1200) anlamına dokunulmaz: arazi ölçümü bir harita işi öğesine yalnız kimliğiyle (`is-N`,
türü `campaign`) bağlanır. Beş liste, her kayıt kendi kimliğiyle (`önek-N`, N baştaki sıfırsız pozitif tamsayı; yeni
kimlik listedeki en büyüğün bir fazlası):

- **Arazi ölçümü** (`campaign`) `kp-N`: adı; bağlı harita işi öğesi (`work`, isteğe bağlı); operatörü; **epok kararı** (`epoch`,
  ondalık yıl: kabul edilen noktaların koordinat epoku; yoksa projenin `coordinateEpoch`'u) ve epoklar farklıysa
  çözümleri taşıyacak **hız ızgarası** (`velocity`: adı ve SHA-256'sı; ADR 0631).
- **Ölçüm oturumu** (`session`) `os-N`: arazi ölçümü; **işaret** (`mark`: arazideki noktanın adı; aynı işaretin her günkü oturumu ayrı
  kayıttır, üstüne yazılmaz); başlangıç ve bitiş (`start`, `end`: `YYYY-AA-GGTss:dd:sn[.kesir]Z`); zaman sistemi
  (`gps`, `utc`, `glo`, `gal`, `bdt`, `qzs`, `irn`); alıcı (tür, seri no, yazılım sürümü); **anten** (tür, seri no,
  ölçülen yükseklik, ölçüldüğü yer: `arp` düşey ARP'ye ya da `slant` eğik; eğikte üreticinin yarıçapı `radius` ve ölçü
  noktasının ARP'den yüksekliği `offset`; dış merkezlik `east`, `north`); operatör; **orijinal dosya** (mutlak yol,
  SHA-256 revizyonu, biçimi `rinex`/`nmea`/`gpx`); RINEX başlığının başka yerde tutulmayan bilgileri (sürüm, işaret
  no, aralık, yaklaşık konum XYZ); ve NMEA/GPX'ten okunan **ham ölçümler** (`observations`: okuyucunun `GnssPoint`'i
  aynen, satır numarasıyla; oturum başına en çok 86 400).
- **Çözüm** `cz-N` (sağlayıcının çözümü): oturumu; türü (`receiver`: alıcının kendi gerçek zamanlı çözümü, `processed`:
  ofis yazılımında işlenmiş baz/PPK/statik, `ppp`: PPP servisi); enlem, boylam (derece), elipsoit yüksekliği (m);
  **çerçeve** (`srid`: kayıttaki coğrafi bir sistem, ör. 4326 WGS 84, 5252 TUREF) ve **epoku**; **uygulanmış anten
  yüksekliği** (`antennaApplied`, m: çözümü hesaplayanın çıkardığı düşey ARP yüksekliği; 0: konum ARP'nin);
  kalite: fix (alıcının adıyla), epok sayısı, en az uydu, en büyük HDOP (birimsiz), σ kuzey/doğu/yukarı (m), **tam
  kovaryans** (KK, KD, KY, DD, DY, YY; m²), düzeltme kaynağı, en büyük düzeltme yaşı (s); işleme raporu dosyası (yol +
  SHA-256); **yeniden işleme tarifi** (`recipe`, serbest metin: yazılım, efemeris, maske, anten modeli …); alıcı
  çözümünde hangi ham ölçümlerden (`from`, `to`, `fixes`) yapıldığı; **dışlama nedeni** (`excluded`; varsa çözüm
  dışlanmıştır, silinmez); **dengelemeye girer** (`adjust`).
- **Kabul edilen nokta** `kn-N`: işaret; çizimdeki nokta nesnesinin kalıcı kimliği (`uid`); **revizyonu** (1, 2 …);
  bugünkü **sonucu** (kullandığı çözümler, yöntem, işaretin enlem/boylam/yüksekliği, çerçeve, çözümlerin epoku, epok
  kararı, her oturumun kullanılan düşey anten yüksekliği, σ, projedeki x/y/z, koordinat zincirinin `complete`/`limited`
  kararı, yeniden üretilmişse nedeni) ve **eski sonuçları** (`history`, en eskisi önce: eski rapor korunur).
- **Dayanan** `kb-N`: kabul edilen noktalardan yapılmış nesneler (ör. yüzey): nesnelerin kimlikleri, işlem adı ve
  her noktanın o anki revizyonu.

**Üç kimlik ayrıdır:** ham gözlem (oturumun dosyası ve ham ölçümleri), sağlayıcının çözümü (`cz-N`), projeye kabul
edilen nokta (`kn-N` ve nesnesi). Kurallar (`MapSurvey::problem`, okuyucu ve yazıcı aynı): kimlikler biçiminde ve
tekil; oturumun arazi ölçümü, çözümün oturumu, noktanın çözümleri var; noktanın çözümleri dışlanmamış ve hepsi noktanın
işaretinin; bir işaretin tek kabul edilen noktası; metinler boşlukla başlayıp bitmeyen 1–200 karakter (tarif 1–2000,
dışlama nedeni 1–500); sayılar sonlu; enlem [−90, 90], boylam [−180, 180]; anten yüksekliği [0, 100] m, eğikte
0 < yarıçap < yükseklik; σ > 0; kovaryans pozitif tanımlı (köşegen, 2×2 ve 3×3 öncü küçükler > 0); yaş ≥ 0; epok
ADR 0519'un aralığında; bitiş başlangıçtan önce değil. En çok 1000 arazi ölçümü, 10 000 oturum, 100 000 çözüm, 10 000
nokta, 10 000 dayanan, nokta başına 100 eski sonuç.

### 3. Kurallar ve hesaplar

- **Düşey anten yüksekliği** H: `arp` → ölçülen; `slant` → √(S² − R²) − offset.
- **İşaretin yüksekliği:** h_işaret = h_çözüm + antennaApplied − H (oturumun bugünkü anteni). Dış merkezli anten
  (east/north ≠ 0) bu dilimde indirgenmez (`antenna_eccentric`).
- **Alıcı çözümü** (`solve`): oturumun ham ölçümlerinden `from`..`to` aralığındaki (0'dan sayılı, ikisi dahil;
  öntanımlı hepsi) ve `fixes` verildiyse fix'i onlardan biri olanlar; elipsoit yüksekliği olmayanlar dışarıda kalır, hiçbiri kalmazsa ret (GPX'in `ele`'si datumu bilinmediği için elipsoit yüksekliği değildir, ADR 0169); enlem, boylam, yükseklik sırayla toplanıp
  sayıya bölünür (soldan sağa toplama; Python'un `sum`'ı değil); fix hepsinde aynıysa o, değilse yok; uydu en azı,
  HDOP en büyüğü (varsa). Çerçeve ve uygulanmış anten yüksekliği kullanıcının söylediğidir; KentOS tahmin etmez.
- **Kabul** (`accept`) ve **yeniden üret** (`regenerate`): yöntem `single` (tek çözüm), `mean` (≥ 2, aritmetik, verilen
  sırayla) ya da `weighted` (≥ 2, her bileşen 1/σ² ağırlıklı, σ = √(1/Σw); hepsinde σ olmalı). Çözümler aynı çerçeve ve
  epokta olmalı; oturumlarının arazi ölçümleri aynı epok kararını ve hız ızgarasını taşımalı. Projedeki yer **koordinat
  zincirinin kendisiyle** hesaplanır: `cad.crs.transform` (kaynak: çözümün çerçevesi, elipsoit yüksekliği, epoku;
  hedef: projenin sistemi ve yükseklikleri, epok kararı; hız ızgarası arazi ölçümününki). İki epok ızgarasız ya da ızgara
  bu cihazda yoksa zincirin reddi aynen döner (`epoch_unsupported` …); başarılı hassas dönüşüm uydurulmaz.
- **“Son ölçülen”, “kabul edilen”, “dengelemeye giren”:** bir işaretin çözümlerinden oturumunun bitişi (yoksa
  başlangıcı) en geç olanı son ölçülendir (eşitlikte kimlik numarası büyük olan; zamanı olmayan sıralanmaz); kabul
  edilen noktanın bugünkü sonucundakiler kabul edilendir; `adjust` olup dışlanmamış olanlar dengelemeye girer.
- **Güncel değil (nokta):** bir oturumunun bugünkü düşey anten yüksekliği sonuçtakinden farklı; epok kararı değişti;
  çözümlerinden biri sonradan dışlandı. Nesnesi silindiyse `missing`. **Dayanan güncel değil:** bir noktanın revizyonu
  kaydındakinden büyük; nesnesi yoksa `missing`.
- **Oturumun oynatma özeti** (kayıtlı NMEA/GPX'in replay denetimi): fix türlerinin sayıları (ilk görülme sırasıyla);
  **RTK sabitten düşüş** (RTK sabit konumdan hemen sonra başka fix'li konum); **zaman atlaması**: zamanı olan ardışık
  iki konumun farkı (sn) ≤ 0 ya da adımların en sık değerinin (eşitlikte küçüğü) iki katından büyük. Okunmayan satırlar
  (kesilen veri) içe aktarmada uyarıdır, ham dosya revizyonuyla kalır.

### 4. Komutlar

- `map.survey.query` (salt okunur): arazi ölçümleri (oturum sayısı, bağlı harita işi öğesinin olup olmadığı), oturumlar
  (anten ve düşey yüksekliği, dosyası `found`/`missing`, `deep` ile `current`/`changed`; ham ölçüm sayısı; oynatma
  özeti), işaretler (oturumları, çözümleri “son ölçülen / kabul edilen / dengelemede / dışlandı” işaretleriyle, işaret
  yüksekliği), kabul edilen noktalar (durumu ve nedenleri, sonucu, eski sonuç sayısı), dayananlar; `uid` ile o nesnenin
  ölçüsü. Retler: `not_a_map_project`, `invalid_uid`, `entity_not_found`.
- `map.survey.edit` (her işlem tek geri alma adımı, ya hep ya hiç): `campaign`, `setCampaign`, `session` (dosyayla:
  RINEX gözlem başlığından işaret, alıcı, anten ve zamanlar; NMEA/GPX'ten ham ölçümler; elle verilen alan dosyanınkini
  yener, farkı `rinex_differs` uyarısıdır; okunmayan satırlar `gnss_lines_skipped`), `setSession`, `solve`, `solution`
  (işlenmiş/PPP/elle çözüm, kovaryans ve raporuyla), `exclude` (nedenle), `include`, `adjust`, `accept` (nokta nesnesi
  ve `kn-N` aynı adımda), `regenerate` (eski sonuç `history`'ye, nesne yeni yerine, revizyon +1), `use`, `remove`
  (kayıt; kabul edilen noktanın kullandığı çözüm, oturum ya da arazi ölçümü ve bir dayananın adlandırdığı nokta
  silinmez: `survey_in_use`). Kilitli katman `layer_locked`; plan ve doğrulama hiçbir şey yazmaz.
- **Python ve MCP** aynı katalogdan; `map.survey.edit` MCP'nin yıkıcı listesindedir (`remove`, `exclude`,
  `regenerate` var olanı değiştirir).

### 5. Kalıcılık: `.kcad` şema 194 (geçici numara)

`mapSurvey` ayarların 9 harfli anahtarları arasında `lineTypes`'tan sonra JSON olarak; yalnız harita projesinin; boş
yapı yazılmaz; okuyucu ve yazıcı §2'nin kurallarını denetler. Ham ölçümler ve çözümler dosyadadır: kaynak dosya
kaybolsa, taşınsa ya da içe aktarma iptal edilse proje ham veriyi yitirmez (dosyanın yokluğu `missing` görünür).

### 6. Masaüstü

Yalnız KentOS Harita'da (ve kentos-cad'de) sağ dokta **GNSS arşivi** paneli (Harita işi panelinin durum ağacı kalıbı).
Üstte tek araç çubuğu satırı, ikon düğmeler ve sözleri ipuçlarında: Arazi ölçümü ekle, Ölçüm oturumu ekle… (RINEX,
NMEA ya da GPX seçilir), Dosyaları oku (SHA-256) ve tek metin kutusu (dosyanın söylemediği işaret ya da dışlama
nedeni). Ağaçta üç başlık: **Arazi ölçümleri** (altında ölçüm oturumları: işaret, düşey anten yüksekliği, dosya
biçimi ya da sorunu, ipucunda alıcı, anten, zamanlar ve oynatma özeti; kayıtlı bir NMEA'nın düğmesi alıcı çözümünü
yapar: RTK sabit ölçümlerin ortalaması, NMEA 0183'ün WGS 84'ü, GGA'nın anten yüksekliği çıkarılmamış), **İşaretler**
(kabul edilen noktanın durumu hapında; düğme Kabul et — dışlanmamış ve son ölçülenle aynı çerçevedeki çözümlerin
ortalaması — ya da güncel değilse Yeniden üret; altında çözümler: Son ölçülen, Kabul edilen, Dengelemede, Dışlandı
hapı, işaret yüksekliği ve σ, düğme Dışla / Dışlamayı kaldır) ve **Dayananlar**. Şerit: Ölçme sekmesinde (yalnız
`map` işinde) **GNSS arşivi** grubu (GNSS arşivi, Ölçüm oturumu ekle…, Kabul edilen noktalar); GIS'in Ölçme
sekmesinin Noktalar grubunda Kabul edilen noktalar. Öznitelikler'de **GNSS ölçümü** bölümü Harita ve GIS'te.

## Sonra

- RTCM/NTRIP ve canlı alıcı (HYB-24, ertelendi); PPK/PPP çözücü adaptörü (ayrı doğruluk ve lisans değerlendirmesi).
- Baz vektörü ve tam kovaryanslı ağ dengelemesi (bugün `adjust` yalnız işarettir); dış merkezli anten indirgemesi;
  ANTEX faz merkezi modeli; RINEX gözlem verisinin kendisi (bu dilim başlığı okur).
- GGA'nın düzeltme yaşı ve istasyon kimliği (bugünkü okuyucu taşımaz; çözüme elle yazılır).
- Dayanan kaydının yüzey/eğri komutlarınca kendiliğinden yazılması; web (ADR 0228).

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/map_survey.rs` (`MapSurvey` ve kayıtları, `problem()`, zaman ve kovaryans
  kuralları, `RinexHeader`), `ProjectSettings.map_survey`, 19 yeni kod (`codes.rs`), birim `m2` (`units.rs`);
  `GnssPoint`'in sayılarına birim. `crates/shared/commands/src/map_survey.rs` (sorgu ve düzenleme türleri), katalog,
  yetenekler.
- **Biçim:** `crates/shared/formats/src/gnss/rinex.rs` (RINEX gözlem başlığı); NMEA/GPX okuyucusu değişmedi.
- **`.kcad`:** şema 194 (`SCHEMA_WITH_MAP_SURVEY`, geçici), yazıcı ve okuyucu; `tools/kcad/kcad.py`'nin bağımsız
  denetimi; `scripts/fixtures/kcad_v2_reference.py`'nin örnek ve bozuk dosyaları; `docs/specs/kcad-v2.md`.
- **Belge:** `crates/native/domain/src/map_survey.rs` (geri alma adımı `Op::MapSurvey`).
- **Komutlar:** `crates/native/application/src/map_survey.rs` (sorgu, durum ve neden kuralları, oynatma, kabul
  hesabı ve koordinat zinciri `crs_transform::answer` ile), `map_survey_edit.rs` (13 işlem, tek adım; kabulde nokta
  nesnesi ve kayıt aynı işlemde, yeniden üretmede nesne taşınır).
- **Masaüstü:** `apps/desktop/src/map_survey_panel.rs` (GNSS arşivi paneli), `properties/mod.rs` (GNSS ölçümü bölümü),
  `module_surfaces.rs` (`Panel::MapSurvey` Harita'da; `PropertyRows::Survey` Harita ve GIS'te), `app.rs` (komutlar),
  `catalog/v1/additions.json` (üç komut, dokuz ikon), `catalog/v1/ribbon.json` (Ölçme › GNSS arşivi grubu, yalnız
  `map`; `modules.only`), `catalog/v1/operations.json`.
- **İkonlar** (her satırın kendi ikonu): `gnssArchive` (GNSS arşivi: işaretin üstünde uydu), `gnssSession` (Ölçüm
  oturumu ekle: dosya ve artı işareti), `gnssSelect` (Kabul edilen noktalar: çerçeveli işaret), panelde
  `gnssCampaign` (Arazi ölçümü ekle: bayrak), `gnssFiles` (Dosyaları oku), `gnssAccepted` (Kabul et), `gnssExclude`
  (Dışla), `gnssRegenerate` (Yeniden üret), `gnssSolve` (Alıcı çözümü).
- **Otomasyon:** Python `kentos.cad.map.survey` (`edit`, `query`), MCP (`map.survey.edit` yıkıcı listede), manifest.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/map_survey_cases.py` (KentOS kodu yok; kurallar ve cümleler bu ADR'den,
  RINEX başlığı sütunlarıyla, NMEA `gnss_nmea_cases.py` ile, projeye taşıma PROJ ile: `crs_vertical_cases.py`,
  `geoid_cases.py`'nin hız ızgarası ve `crs_chain_rules.py`'nin zincir kararı). `map.survey.query` 22, `map.survey.edit`
  73 durum; masaüstü işleyicisi 95'ini geçer (`KENTOS_CASES_ONLY=map.survey cargo test -p kentos-native-application
  --test all every_command_case`). Projelenmiş x, y ve z sonuçlarda ve nesnelerde 1e-6 m içinde (ADR 1230 §4).
- **`.kcad`:** `python3 scripts/fixtures/kcad_v2_reference.py --check` ve `tools/kcad/kcad.py validate`: örnek geçerli;
  şema 192'de, mimari projede, iki noktalı işaret, pozitif tanımsız kovaryans ve atlayan revizyon reddedilir;
  `cargo test -p kentos-kcad`.
- **Masaüstü** (`map_survey_panel_tests`): iki günlük N1'in kabulü ve rolleri; anten düzeltilince güncel değil ve
  nedeni; panelden yeniden üretme (2. revizyon, eski sonuç, nesne 0,1 m alçak, yüzey güncel değil), geri alma; nedensiz
  dışlama yazmaz, nedenle dışlama ve geri alma; kapatılan dosya penceresi yazmaz; NMEA oturumu, oynatma özeti, alıcı
  çözümü ve kabul (WGS 84 zinciri sınırlı); `.kcad` kaydet–aç, dosyası kaybolan oturumun ham ölçümleri yerinde, okuma
  yazmaz; Harita düzenler, GIS kabul edilen noktayı Öznitelikler'de okur, Mimari ve Yol göstermez. Resimler
  `.run/gui-evidence/gnss-arsivi-*`.
- **Ölçülmeyenler:** gerçek alıcı ve canlı bağlantı (HYB-24 ertelendi); büyük arşivde süre (86 400 ham ölçümlük
  oturum); Python takımı (`pnpm py:test`) bu dalda koşmadı.
