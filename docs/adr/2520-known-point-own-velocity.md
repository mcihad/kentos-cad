# 2520 — Bilinen noktanın kendi hızı

Tarih: 2026-11-08. plan/map-cad.md MAP-03, dördüncü tur. Durum: taslak. Plan satırı: “Kampanya … koordinat/epok … kalıcı
modelde”; ADR 2120'nin “Sonra”sındaki “Bilinen noktanın kendi hızı (TUSAGA-Aktif istasyonlarının yayımlanan hızları),
ızgarasız” maddesi.

İlgili:
- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı, bilinen nokta `bn-N`;
- [ADR 2120](2120-network-at-the-measurement-epoch.md): ağ ölçme epokunda, bilinen noktalar hız ızgarasıyla oraya;
- [ADR 1230](1230-coordinate-chain-epoch-grids-and-budget.md) §7: resmî TUREF hız alanı sahibin kaynağını bekliyor.

## Neden bu parça

Bugün arazi ölçümünün hız ızgarası yoksa 2005,0'da girilmiş TUSAGA-Aktif koordinatlarıyla 2026'da ölçülmüş bazlar hiç
dengelenmiyordu (`epoch_unsupported`). Resmî TUREF hız alanı da henüz yok (ADR 1230 §7). İstasyonların hızları ise
yayımlanır; bu dilim onları kullanmanın yolunu açar.

Sıradaki öbür iş, GVX oturumlarının çapraz korelasyonları, bu turda yapılmadı. Bazın uyuşumsuzluk testi bugün her bazı
kendi kovaryansıyla beyazlatıp üç bileşenini ayrı sınıyor. Oturumun tam kovaryansıyla beyazlatılınca bir bazın satırları
oturumdaki öbür bazları da taşır, testin hangi bazı işaretleyeceği değişir. Bu, ayrı bir araştırma ve kararla yapılacak
(Sonra).

## Dayanak

- **BÖHHBÜY Madde 15(1)(a)** (ADR 1740'ın okuması): TUTGA/TUSAGA-Aktif ve eski C1 koordinatları ölçme epokuna taşınır.
  Nasıl taşınacağını söylemez.
- **IERS Conventions (2010)**, IERS Technical Note 36, Bölüm 4 “Terrestrial reference systems and frames” (1 Nisan
  2019 tam bölüm güncellemesi; [belge](https://iers-conventions.obspm.fr/content/chapter4/icc4.pdf), 8 Kasım 2026'da
  okundu):
  - §4.1.2: ITRF istasyonlarının düzenlenmiş koordinatları zamanın parametrik işlevleridir;
  - ITRF2008'e dek bir referans epokundaki konum ve istasyon hızından oluşurlar. ITRF2014'te bazı istasyonlara
    deprem sonrası deformasyon (PSD) işlevleri de eklenir;
  - denklem (4.13) konumu X(t) = X(t₀) + (t − t₀)·Ẋ biçiminde taşır.
- **TKGM'nin TUSAGA-Aktif koordinat ve hız belgesi:** akademik bir çalışma “TUSAGA-Aktif istasyonları koordinat ve
  hızları (Aralık 2015 – Temmuz 2017)” belgesine atıf yapıyor. Bağlantısı artık sitenin ana sayfasına yönleniyor;
  biçimi (çerçeve, epok, birim) okunamadı. Bu yüzden KentOS hızın birimini ve eksenlerini açıkça ister.
- **NGS'nin GVX 1.0 belgesi** (ADR 1740), §8 SESSION: oturumun `CROSS_CORRELATION_MATRIX`'inin `CCM_BLOCK`'ları
  vektör çiftleri arasındaki 3×3 korelasyonlardır:
  - satırları `VECTOR_ID_ROW`'un, sütunları `VECTOR_ID_COL`'un X, Y, Z'sidir;
  - dokuz değer satır sırasıyladır (Ek A, Şekil A1 ve A3 karşılaştırıldı);
  - n vektörde n(n − 1)/2 blok gerekir.

  Bu dilimde kullanılmadı; Sonra'daki iş için okundu.

## Karar

1. **Bilinen noktanın hızı** (`SurveyKnown.velocity`, `map.survey.edit known`'un `velocity`'si):
   - çerçevenin yer merkezli VX, VY, VZ'si, m/yıl (yeni birim `m/yr`). Yayımlanan istasyon hızları böyle verilir;
   - noktayı kendi `epoch`'undan taşır; hızı olan noktanın epoku olmalıdır;
   - her bileşen en çok 1 m/yıldır. Bu KentOS'un sınırıdır, kural değeri değildir: istasyonlar yılda birkaç santimetre
     kayar, 1 m/yıldan büyük değer çoğu zaman m/yıl yerine yazılmış mm/yıldır;
   - aykırısı `invalid_survey_value`, alanı `operation.velocity`.
2. **Ağda taşıma** (`map.survey.network`): ölçme epoku noktanın epokundan farklıysa nokta:
   - kendi hızı varsa onunla taşınır: X(t) = X(t₀) + V·(t − t₀), çerçevenin elipsoidinde yer merkezli X, Y, Z'ye ve geri;
   - yoksa eskisi gibi arazi ölçümünün hız ızgarasıyla;
   - ikisi de yoksa `epoch_unsupported`. İleti artık üç yolu da sayar: kendi hızı, ızgara, ölçme epokunda girmek.
3. **Cevapta** `moved[].by`: `velocity` ya da `grid`. Taşınma doğu, kuzey, yukarı olarak eskisi gibi verilir.
4. **Yeni noktalar** arazi ölçümünün epok kararına (2005,0 gibi) hız ızgarasıyla döner. Ağ bilinen noktaların hızıyla
   uyuşsa da ızgara yoksa yeni noktalar karara taşınamaz, ağ kabul edilemez ve neden söylenir. Epok kararı olmayan arazi
   ölçümünde yeni noktalar ölçme epokunda projeye yazılır.
5. **`.kcad` şema 207:**
   - yazıcı 207'yi bir bilinen noktanın hızı varken seçer; 206 ve öncesi `velocity`'yi `unknown_field` sayar;
   - Rust okuyucu-yazıcı, bağımsız okuyucu (`tools/kcad/kcad.py`) ve başvuru yazıcısı;
   - örnek `map-survey-velocity`, üç bozuk dosya, “daha yeni” dosya artık `schema-version-208`.
6. **Masaüstü** (GNSS ağ dengelemesi):
   - bilinen nokta formunda **Hız (m/yıl)** satırı: VX, VY, VZ, üçü birden ya da hiçbiri;
   - özetin epok satırı yolu söyler: kendi hızıyla, hız ızgarasıyla ya da ikisi birden;
   - Bilinen noktalar'ın Taşınma sütunu “hız …” ya da “ızgara …” der.
7. **Kapsam işareti:** ADR 2120'deki gibi **Harita + CBS** (pencere ve kabul Harita'nın).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_survey_network_cases.py`, KentOS kodu yok. X, Y, Z'ye ve geri PROJ'la
  (TUREF 3B ↔ yer merkezli). Bağımsız denetim: BN1 ve BN2'nin hızı, hız ızgarasının kendi yerlerindeki hızıdır. Hız,
  PROJ'un deformation adımının bir yıllık taşıması olarak X, Y, Z'de yazıldı, 0,01 mm/yıla yuvarlandı. Kendi hızıyla
  taşıma ızgaralı taşımayla 0,2 mm içinde aynıdır: bn-1 (−516,6; 169,8; −42,6) mm, ızgarayla (−516,7; 169,8; −42,6).
- **Yeni ağ durumları** (41, 38'den):
  - kendi hızlarıyla ızgarasız: ağ uyuşur (m0 0,377, model testi geçer). Yeni noktalar 2005,0 kararına hız modelsiz
    taşınamaz, ağ kabul edilemez;
  - epok kararı olmayan arazi ölçümünde aynı ağ kabul edilebilir;
  - biri hızıyla, öbürü ızgarayla.
- **Var olan taşımalar** `by: grid` aldı; ızgarasız ret iletisi yeni yolu da sayar.
- **Arşiv düzenleme:** `map.survey.edit`'e üç yeni durum: hızla bilinen nokta, epoksuz hız, 1 m/yıldan büyük hız.
  Toplam 135.
- **Sonuç:** Rust ve Python geçer.
- **`.kcad`:** codec testleri 242 geçer; bağımsız okuyucu 1693 dosyada `expected.json` ile aynı sonucu verir.
- **MCP** (`crates/native/mcp/tests/known_velocity.rs`):
  - şema 207 örneği açılır, ölçme epokunda dengelenir;
  - `moved` `velocity` der, taşınma V·Δt'nin doğu, kuzey, yukarısıdır (testte dosyanın sayılarından hesaplanır);
  - epoksuz hız reddedilir.
- **Masaüstü** (`survey_velocity_tests.rs`):
  - iki nokta da kendi hızıyla taşınır, sayılar durumunkiyle 0,1 mm içinde aynıdır, özet ve Taşınma sütunu yolu söyler;
  - karışık durumda özet ikisini sayar;
  - formdan hız girilir, iki bileşenle ret.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/kendi-hizi-2520/`.

## Sonra

- GVX oturumlarının çapraz korelasyonları: oturumun tam kovaryansı ve korelasyonlu gözlemde bazın uyuşumsuzluk testi
  (araştırma ve karar).
- Yeni noktaların hızı: istasyon hızlarından enterpolasyon (idarenin yöntemi) ya da resmî hız alanı. Akademik bir
  çalışmaya göre HGM 2022'de ulusal bir hız alanı yayımladı; okunmadı, sahibin kaynağı bekleniyor (ADR 1230 §7).
- Doğu, kuzey, yukarı olarak verilen hızlar; deprem sonrası deformasyon (PSD) işlevleri.
- Arşiv panelinde bazlar ve dengelemeler. Yapıldı: [ADR 2920](2920-archive-panel-baselines-and-adjustments.md).
- Web (TODOS §0.2).
