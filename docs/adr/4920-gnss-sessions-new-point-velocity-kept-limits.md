# 4920 — GNSS baz oturumlarının korelasyonları, yeni noktaların hızı ve kabul farkı sınırının projede saklanması

Tarih: 2026-11-11. plan/map-cad.md MAP-03, onuncu tur. Durum: taslak. ADR 4520'nin “Sonra”sı: GVX oturum korelasyonları
(korelasyonlu gözlemde baz testinin kararıyla), yeni noktaların hızı, sınırların projede saklanması.

İlgili:
- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı, GVX okuyucusu;
- [ADR 2120](2120-network-at-the-measurement-epoch.md), [ADR 2520](2520-known-point-own-velocity.md): ölçme epoku,
  bilinen noktanın kendi hızı;
- [ADR 0631](0631-geoid-velocity-and-vertical-statements.md) §2: hız ızgarası (`cad.crs.transform`'un `velocity`'si);
- [ADR 4520](4520-known-place-tolerance.md): kabul farkı sınırı.

## Dayanak

Araştırma notları: `.run/research/map-cad/mevzuat/MAP-03.10.md`, `.run/research/map-cad/uygulama/MAP-03.10.md` (11
Kasım 2026 eki); resimler `.run/research/map-cad/uygulama/MAP-03.10-resim/`.

- **Netcad (önce bakıldı):** wiki.netcad.com.tr bu makineden açılamadı (sitenin TLS sertifikası doğrulanamadı). Ürün
  sayfasında (Hesap; `netcad-hesap-urun-sayfasi.jpg`) nokta düzenleyici, poligon/nirengi, Afin/Helmert/projeksiyon
  dönüşümü ve tecviz çıktısı var; GNSS baz ağı dengelemesi, oturum korelasyonu ya da hız modeli geçmiyor. Netcad'in
  davranışı okunamadığı için ondan değer alınmadı.
- **NGS GVX 1.0** (4 Şubat 2021, s. 15–20; `gvx-session-tanimi.png`, `gvx-ek-a1-oturum-matrisi.png`,
  `gvx-ek-a3-ccm-block.png`):
  - `SESSION` (ID, TOTAL_VECTORS, SESSION_TIME) birlikte işlenen vektörlerdir;
  - `CROSS_CORRELATION_MATRIX` (ORDER “XYZ”) her vektör çifti için bir `CCM_BLOCK` (VECTOR_ID_ROW, VECTOR_ID_COL; 9
    korelasyon); n vektörde n(n − 1)/2 blok;
  - Şekil A1 ile A3 karşılaştırılınca dokuz sayı **satır satır**: satırlar ROW vektörünün X, Y, Z'si, sütunlar COL'unki;
  - vektörün oturumu GNSS_VECTOR'da yazmaz; oturumun üyeleri blokların andığı vektörlerdir.
- **Korelasyonun etkisi:** “Single Processed Independent and Trivial Vectors in Network Analysis” (J. Surv. Eng.
  126(1), 2000) ve “Impact of Double Differences Correlation on the Adjustment of Small GNSS Networks” (J. Surv. Eng.,
  2019): ikiden çok alıcılı oturumda vektörler arası kovaryans çoğu kez ağ dengelemesine gelmez; ihmali doğruluğu iyimser
  gösterir, küçük ağda koordinatı az etkiler.
- **Korelasyonlu ölçüde uyuşumsuz ölçü testi:** Baarda'nın w-testi korelasyonlu ölçüde
  wᵢ = |eᵢᵀPv| / (σ₀·√(eᵢᵀ·P·Qvv·P·eᵢ)) (Teunissen, Testing Theory, 2006; Baarda 1968). Beyazlatılmış satırın testi
  korelasyonlu oturumda satırı tek bir baza bağlamaz: alt üçgen Cholesky öncekilerin bileşenlerini karıştırır, sonuç
  bazların yazılış sırasına bağlı olur.
- **BÖHHBÜY** (RG 26.06.2018/30460, 04.10.2025 değişikliğiyle):
  - m. 15, 19, 22 “bağımsız baz” ve “tekli veya oturum baz çözümü” der; korelasyonun dengelemeye taşınmasını ne ister
    ne yasaklar: yöntem seçimidir, cevapta yazılır;
  - m. 21 (2025): C derece noktaların hızı HGM'nin güncel Türkiye Hız Alanı Modeli'nden; eski enterpolasyon kalktı.
    KentOS hız uydurmaz; modelin kendisi (sahibin işi, beklemede) bu dilimde yok;
  - m. 103(1)(c): idare yönetmeliğe aykırı olmamak koşuluyla kendi esasını koyabilir, sınır daha sıkı olabilir;
    Geçici Madde 2: işin hangi metin sürümüyle değerlendirildiği kayıtta durmalı. Sınırın **kaynağı** bunun için
    zorunludur.
- **Hız ızgarası:** NGS HTDP ve deformasyon modelleri hızı ızgaradan okur; ADR 0631 §2'nin zinciri hız ızgarasını
  kullanıcının kitaplığından (ad ve SHA-256) alır. Zincir doğrusal: bir yıllık taşınma hızın kendisidir.

## Karar

1. **GVX okuyucusu oturumları okur** (`kentos_formats::gnss::gvx`):
   - her `SESSION`'ın blokları, satır satır; üyeler blokların andığı vektörler, dosyanın sırasıyla;
   - ORDER “XYZ” değilse, blok olmayan bir vektörü, bir vektörü kendisiyle eşliyorsa, aynı çift iki kezse, sayılar dokuz
     değilse ya da [−1, 1] dışındaysa, bir vektör iki oturumdaysa dosya satırıyla reddedilir;
   - iki vektör anmayan ya da TOTAL_VECTORS'ı andığı vektör sayısını tutmayan oturum alınmaz (uyarı
     `gvx_correlations_ignored`); blokları eksik oturum saklanır ve söylenir (`gvx_session_incomplete`).
2. **Arşivde baz oturumu `bo-N`** (`mapSurvey.baselineSessions`): arazi ölçümü, dosyadaki adı, zamanı, bazları (`bz-N`),
   blokları (`row`, `col`, `r` dokuz korelasyon), dosyası. `map.survey.edit`'in `baselines`'ı GVX'in oturumlarını bazlarla
   aynı adımda yazar; `remove` `bo-N`'i çıkarır (bazlar kalır), dengelemedeki oturumu ve oturumdaki bazı çıkarmaz.
3. **Dengeleme `correlations: session | none`** (`map.survey.network` ve `edit`'in `network`'ü, sürüm aynı, ek alan):
   - verilmezse `none`: bugünkü gibi her baz kendi 3×3'üyle; eski cevaplar aynı;
   - `session`: her oturumun dengelemeye giren iki ya da daha çok bazı tam kovaryansıyla (kendi 3×3'leri ve
     Σᵢⱼ = ρᵢⱼ·σᵢ·σⱼ çapraz bloklar, hepsi varyans çarpanıyla) birlikte beyazlatılır (3n×3n Cholesky). Dışlanan baz
     oturumdan düşer (kalanların kovaryansı marjinaldir); tek bazı kalan oturum korelasyonsuz kalır;
   - **baz testi kararı:** `session` ile her bazın bileşenleri kendi ölçüleri üzerinde korelasyonlu w-testiyle sınanır
     (w = |Pv|ᵢ/√(PQvvP)ᵢᵢ, r = (QvvP)ᵢᵢ; sınır sınıfın güveni ya da 3,29); bilinen noktanın sözde ölçüleri bugünkü gibi.
     Test, bazların yazılış sırasına bağlı değildir;
   - retler: kullanılan iki baz arasında blok yoksa `session_blocks_missing` (oturum, iki baz ve “correlations none”
     çıkışı söylenir); oturumun tam kovaryansı pozitif tanımlı değilse `session_covariance_invalid`;
   - cevabın `correlations`'ı (istenince ya da kullanılan bazların oturumu varken): `used`, giren `sessions`, kullanılmayan
     `unused`; `none` ile oturum varken uyarı `session_correlations_unused`. Kabul edilen dengeleme kaydı `sessions`'ı
     tutar; aynı ağın “güncel” sayılması oturumların aynı olmasını da ister.
4. **Yeni noktaların hızı** (`velocities: true`):
   - yalnız arazi ölçümünün hız ızgarasından (kullanıcının dosyası, adı ve SHA-256'sı): koordinat zinciri noktayı ağın
     epokundan (yoksa 2005,0) bir yıl taşır, taşınma noktanın doğu, kuzey, yukarısında okunur (m/yıl);
   - ızgarasız `no_velocity_grid` (KentOS hız uydurmaz); ızgaranın okuyamadığı nokta `why`'da, ağ kabul edilemez;
   - cevapta istasyonun `velocity`'si ve `velocityGrid`; kabul edilen noktanın sonucu `velocity`'yi (`enu`, `grid`)
     tutar. Resmî hız modeli değildir; HGM modelinin dosyası bu ızgara yoluyla verilebilir.
5. **Kabul farkı sınırı projede** (`map.survey.edit` `tolerance`): `horizontal`, `vertical` (m, en az biri, 0'dan büyük)
   ve **kaynağı** (`source`, zorunlu: yönetmelik maddesi ya da idarenin şartnamesi); üçü de verilmezse saklı sınır kalkar.
   `map.survey.query`'nin `places`'i girdide sınır yoksa saklı sınırla değerlendirir; ağaç `tolerance`'ı kaynağıyla verir.
6. **`.kcad` şema 248**: `baselineSessions`, dengelemenin `sessions`'ı, `tolerance`, noktanın `velocity`'si; 247 ve öncesi
   `unknown_field`. Bağımsız okuyucu (`tools/kcad/kcad.py`) ve başvuru yazıcısı da; örnek `map-survey-sessions` ve dört
   bozuk dosya; bilinmeyen sürüm örneği `schema-version-249.kcad`.
7. **Masaüstü:** GNSS ağ dengelemesi penceresinde **Oturum korelasyonları** ve **Yeni noktaların hızı** kutuları, özet
   satırında kullanılan korelasyon, noktalar tablosunda **Hız (mm/yıl)**; GNSS arşivi › Bazlar'da **Oturum** sütunu;
   Kabul farkı sınırı satırında **Kaynak** kutusu ve **Projeye yaz** düğmesi, saklı sınır açılışta okunur.
8. **Kapsam işareti:** ADR 1740 gibi **Harita + CBS** (pencere Harita'nın).

## Sınırlar

- HGM Türkiye Hız Alanı Modeli'nin kendisi, adı ve biçimi (sahibin işi, beklemede); canlı alıcı yok.
- Bağımsız baz seçimi (oturumda n − 1) önerilmez; kullanıcı bazları dışlayarak seçer.
- `session` yalnız arşivdeki oturumlarla çalışır; baz metninin oturumu yoktur.
- Sınırın “yönetmelikten gevşek olamaz” denetimi yapılmaz (kural paketi sınırları bilinen noktanın yer farkını
  kapsamıyor); kaynak kullanıcının sözüdür.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_survey_network_cases.py`, KentOS kodu yok; numpy'nin yoğun GLS'i, PROJ):
  `map.survey.network` 54 durum (13 yeni: iki oturumla `session`, `none`, verilmeyen, C2; sıfır korelasyon, devrik blok,
  dışlanan baz, tek bazı kalan oturum, eksik blok ve pozitif tanımsız retleri; ızgaradan hız ve ızgarasız ret);
  `map.survey.edit` ağ durumları 62 (GVX'ten `bo-1`, eksik bloklu ve tek vektörlü oturumlu dosya, `session` ve hızla
  kabul, oturumlu ağın korelasyonsuz kabulü (uyarı), pozitif tanımsız oturum reddi, sınırın yazılması, aynısı, kaldırılması, üç ret, oturumdaki baz ve dengelemedeki oturum retleri, oturumun
  çıkması); `map.survey.query` üç yeni durum (saklı sınır, girdinin sınırı yener, ağaçta kaynak). Rust geçer.
  **Bozma denemesi:** iki beklenen değer (oturumlu ağda bz-1'in w'su, N1'in doğu hızı) değiştirildi; koşucu tam o iki
  durumu yakaladı, dosya geri yüklendi.
- Çekirdek testleri: sıfır çapraz korelasyonlu oturum korelasyonsuz ağın koordinatı, σ'sı ve vᵀPv'si; testlerin sıraya
  bağlı olmaması; eksik blok ve pozitif tanımsız ret; GVX okuyucusunun oturum testleri.
- Python, MCP, masaüstü ve resimler: plan notunda.

## Sonra

- HGM hız modelinin dosyası geldiğinde aynı yol (sahibin işi).
- Bağımsız baz önerisi (oturum başına n − 1, en kısa ağaç).
- Web (TODOS §0.2).
