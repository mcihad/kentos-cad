# 3700 — Koordinat dönüştür'de nokta başına karar: kullanım alanları

Tarih: 2026-11-09. plan/map-cad.md MAP-02, yedinci tur. Durum: taslak. ADR 2500'ün “Sonra”sındaki “Koordinat dönüştür'ün
kendisinde nokta başına karar (ADR 1230 §7'nin öbür yarısı)”.

İlgili:
- [ADR 0167](0167-second-crs-and-converter.md) §4: Koordinat dönüştür penceresi;
- [ADR 0519](0519-vertical-crs-and-epoch.md): `cad.crs.transform`;
- [ADR 1230](1230-coordinate-chain-epoch-grids-and-budget.md) §3 ve §7: koordinat zinciri ve kalan işleri;
- [ADR 2500](2500-reproject-preview-shifts-and-reasons.md) §3: katmanın dönüştürülmesinde nesne başına notlar.

## Dayanak

- **IOGP, Guidance Note 7-1** (ADR 2500'ün okuması, 8 Kasım 2026'da okundu):
  - §5.7: sistemlerin ve dönüşümlerin kullanım alanı vardır. Kutu yaklaşıktır, kayıt seçmek içindir;
  - §6.5: dönüşümün kullanım alanının verinin alanıyla tutarlı olması seçim ölçütlerinden biridir.
- **PROJ, Computation of coordinate operations between two CRS** (ADR 2500'ün okuması):
  - `proj_trans()` noktayı işlemlerin kullanım alanıyla karşılaştırabilir;
  - kutunun hesabı yaklaşıktır.
- **KentOS bugün:**
  - katmanı dönüştürmede her nesne kaynak sistemin, datum dönüşümünün ve projenin sisteminin kutusuna karşı okunuyordu
    (ADR 2500 §3);
  - Koordinat dönüştür (komut ve pencere) kararı çağrı başına veriyordu. Zincir tamdı ya da sınırlıydı; bir noktanın
    kendi alanının dışında olduğu söylenmiyordu. TM30'da yazılmış bir Ankara noktası ya da TM33'e çevrilen bir Bursa
    noktası hiçbir uyarı almıyordu.
- **Bulunan:** TM30'un orta meridyenindeki nokta (30°D) UTM 36N'in kutusunun batı kenarındadır. PROJ boylamı
  29,999999999…°, KentOS 30,0° buldu; not kayan noktanın gürültüsüyle dönüyordu.

## Karar

1. **Nokta başına notlar** (`cad.crs.transform`, `flagged`; komut sürümü değişmez). Her dönüştürülen nokta üç kutuya
   karşı okunur. Notlar şu sırayla yazılır (`CRS_POINT_NOTES`):
   - `outside_source_area`: kaynak sistemin kutusu, noktanın kaynağın kendi datumundaki enlem ve boylamında;
   - `outside_operation_area`: datum dönüşümünün EPSG işleminin kutusu, aynı yerde. Datum aynıysa ya da yol projenin
     kendi dönüşümüyse yoktur;
   - `outside_target_area`: hedef sistemin kutusu, noktanın vardığı yerde.

   Kutular katmanınkilerle aynıdır (`fixtures/crs/v1/areas.json`, PROJ'un veritabanı). Projenin kendi tanımının kutusu
   yoktur. Çevrilemeyen nokta `unreached`'tedir, not almaz.
2. **Not bir uyarıdır, ret değildir:** nokta yine dönüştürülür. `outside_area` uyarısı sayıları nota göre söyler: “2
   nokta EPSG'nin kullanım alanlarından birinin dışında: 1 nokta kaynak sistemin…; 2 nokta hedef sistemin…. Dönüştürüldüler;
   bu noktaları denetleyin.” Zincirin kararı çağrınınki olarak kalır.
3. **Kenar:** kenara 10⁻⁹° (yaklaşık 0,1 mm) yakın nokta kenarda sayılır (`kentos_project::crs::EDGE`,
   `box_holds_near`). Bu, EPSG kutusunun kendi kesinliğinin (iki ondalık) çok altındadır ve iki motorun ters izdüşümünün
   kenardaki gürültüsünü (Bulunan) kapatır. Aynı kural katmanın dönüştürülmesinin notlarına da geçti (ADR 2500 §3); onun
   durumları değişmedi.
4. **Masaüstü** (Koordinat dönüştür penceresi; çekirdeği doğrudan çağırır, notu aynı fonksiyondan alır):
   - tek nokta: “Nokta hedef sistemin kullanım alanının dışında; dönüştürüldü, sonucu denetleyin.”;
   - liste: sonuç tablosunda **Alan dışı** sütunu (kaynak, dönüşüm, hedef), özet satırında sayı. CSV ve pano aynı
     sütunu taşır.
5. **Kapsam işareti:** Koordinat dönüştür gibi **CAD + CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/crs_vertical_cases.py`, PROJ; KentOS kodu yok):
  - noktanın enlem ve boylamı PROJ'un kaynak ve hedef sistemin kendi coğrafi sistemine çevirisiyle bulunur. Kutular
    PROJ'un `area_of_use`'u, fixture ile aynı olduğu denetlenir;
  - dört yeni durum:
    - TM30'da yazılmış Ankara WGS 84'e (kaynak);
    - Bursa ve Eskişehir TM33'e (hedef);
    - WGS 84 / UTM 35N'de Filibe TUREF'e (dönüşüm ve hedef);
    - üç noktanın ayrı kararları: biri içeride, biri hedefin, biri kaynağın ve hedefin dışında;
  - var olan durumlardan hedef alanı dışına düşenler notlarını aldı (gerçek). `crs_chain_cases.py` ve `geoid_cases.py`
    aynı kuralı paylaşır;
  - 78 durum (32 + 21 + 25) Rust'ta ve Python'da geçer. Katmanın dönüştürülmesinin durumları kenar kuralından sonra
    aynıdır.
- **Bağımsız Python testi** (`python/tests/test_crs_point_notes.py`): dört yerin notu, enlemleri, boylamları ve EPSG'nin
  kutularından Python'un kendi kutu sınamasıyla bulunur. TM33'e kaynak ve hedef notları, WGS 84'e yalnız kaynağınkiler
  çıkar; her nokta dönüştürülür.
- **MCP** (`crates/native/mcp/tests/crs_point_notes.rs`): ayrı kararların durumu `.kcad` olarak açılan projede aynı
  notları ve uyarıyı verir.
- **Masaüstü** (`calc::convert::tests::a_point_beyond_an_area_of_use_is_said`):
  - TM30'dan TM33'e Bursa'nın sütunu “hedef”, Ankara'nınki “kaynak”;
  - tek noktanın notu; WGS 84'e Bursa içeride.

  Pencere testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/kullanim-alani-3700/`.

## Sonra

- Zamana bağlı Helmert (ITRF çerçeveleri arası): resmî parametre kaynağı ve kullanım hakkı bekleniyor.
- ~~Poligon'da trigonometrik kotla bacakların kendi yüksekliği (ADR 3300 Sonra)~~: [ADR 4100](4100-traverse-trigonometric-heights.md).
- EPSG'nin çokgen kullanım alanları (bugün kutu; ADR 2500 Sonra).
- Web (TODOS §0.2).
