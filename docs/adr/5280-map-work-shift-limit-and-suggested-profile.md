# 5280 — Harita işinde kaymanın seçilen resmî değerle karşılaştırılması; öğe türüne göre önerilen profil

Tarih: 2026-11-12. plan/map-cad.md MAP-01, on birinci tur. Durum: taslak. ADR 4880'in “Sonra”sındaki iki iş:
- kaymanın kullanıcının seçtiği resmî değerle (yanılma sınırı) karşılaştırılması;
- öğe türüne göre önerilen profil (kurum profiliyle).

İlgili:
- [ADR 4880](4880-map-work-typed-layer-profile-shift-and-removed-shape.md): tipli katman profili, köşe köşe kayma ve
  Hausdorff uzaklığı (`shape_shift`);
- [ADR 0541](0541-data-profiles.md) (veri profili), ADR 0558 ve 0735 (projenin klasöründeki ve çizimin içindeki
  profiller);
- [ADR 4520](4520-known-place-tolerance.md): bilinen noktanın kabul farkı sınırı (aynı “sınır içinde / dışında” dili).

## Dayanak

12 Kasım 2026'da okundu; notlar `.run/research/map-cad/uygulama/MAP-01.11.md` (resimler `MAP-01.11-resim/`) ve
`.run/research/map-cad/mevzuat/MAP-01.10.md`.

- **Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik** (RG 27.09.2022/31966) m. 9(1):
  - üretim yöntemine ve ölçeğe göre nokta konum doğruluğu tablosu, metre;
  - tablo resim olarak yayımlandı; büyütülerek okundu ve not edildi (`01-yanilma-siniri-m9-tablo.jpg`);
  - ölçekler 1/200–1/10.000, sütunlar:
    - prizmatik (ölçü değerlerinden; paftadan okunan);
    - takeometrik (poligona uzaklık U ≤ 100 m; U > 100 m; paftadan okunan);
    - fotogrametrik, sayısal fotogrametrik, fotoplan, sayısal;
  - fotogrametrik ve fotoplan 1/2000'den, sayısal fotogrametrik 1/1000'den başlar; küçük ölçeklerin hücreleri boştur;
  - m. 9(2): birden çok yöntemle üretilen geometride en büyük değer;
  - m. 9(3): tabloda olmayan ölçeğin değerini Genel Müdürlük belirler.
- **BÖHHBÜY:** m. 46(1) detay konumu ±7 cm; m. 78(1)(h) çizimde nokta konumu ±0,2 mm.
- Teknik hata kararı kadastro müdürlüğünündür (TKGM 2025/4 m. 12(10)). Sınır içinde kalmak bir ölçüdür; “teknik hata
  değildir” denmez.
- **Öğe türüne göre profil için resmî kural yok:**
  - BÖHHBÜY katman tanımlamaz;
  - TKGM ve ÇŞB iş türüne göre şablon vermez.
- **Benzer uygulamalar:**
  - ArcGIS Pro'da katman şablonu (layer template) şemadan ibaret bir pakettir. Pro onu belli bir yerde arar; kurum
    şablonları paylaşılan klasörde ya da portalda durur. Hangi şablonun seçileceği kullanıcınındır.
  - Detect Feature Changes'in değişim toleransı tek sayıdır, kaynağı söylenmez (ADR 4880).
  - Data Reviewer'ın konumsal doğruluk aracı Pro'ya taşınmadı.

## Karar

1. **Kayma sınırı** (`map.work.query`'nin `limit`'i; tipler `kentos_commands::map_work_limit`):
   - **Esaslar** (`basis`):
     - `own`: kullanıcının değeri (`metres`, 0 < m ≤ 1000);
     - `detail`: BÖHHBÜY m. 46(1);
     - `drawing`: m. 78(1)(h), `scale`'le (0,2 mm × payda / 1000);
     - `method`: Yanılma Sınırı m. 9(1) tablosundan `method` ve `scale`.
   - **Değerler veridir:** `catalog/v1/rules/konum-dogrulugu-2022.json` (değer, kaynak cümlesi, tablonun ölçekleri ve
     yöntemlerin satırları; boş hücre `null`). Kodda değer yok.
   - **Retler** (`invalid_input`, yolu `limit.metres`, `limit.scale` ya da `limit.method`):
     - kendi değerinde değer yok ya da aralık dışı;
     - ölçek yok ya da 1–1 000 000 dışı;
     - yöntem verilmemiş ya da tabloda yok (yöntemler sayılır);
     - tabloda olmayan ölçek (ölçekler sayılır, m. 9(3) söylenir); tablonun boş hücresi.

     Ret sırası: `not_a_map_project`, `invalid_file_path`, `invalid_input`, `invalid_uid`, `entity_not_found`.
   - **Karşılaştırma** (geometrisi değişen ve onay değerleri tutulan nesnede, ADR 4880'in `shift`'i):
     - köşeleri eşleniyorsa en büyük köşe kayması, eşlenmiyorsa Hausdorff uzaklığı sınırla karşılaştırılır;
     - tam değer karşılaştırılır, yuvarlanmış gösterim değil; eşit değer sınır içindedir;
     - `shift.beyond`; eşlenen köşelerde `shift.over` (sınırı aşan köşe sayısı, çekirdeğin `vertices_beyond`'u);
     - `changes.beyond`: listelenen nesnelerden sınırı aşanların sayısı;
     - ağacın `limit`'i: değer ve kaynak;
     - öğenin `why`'ına “k nesne seçilen sınırın dışına kaydı: … m (kaynak).” eklenir. Durum değişmez (zaten
       `revised`).
   - **Sınır projede saklanmaz:** sorgunun girdisidir. Masaüstü oturum boyunca tutar.
2. **Önerilen profil:**
   - **Veri:** profilin sınıfı hangi iş öğesi türlerine uyduğunu `work` alanında söyler (yeni, isteğe bağlı; eski
     profiller değişmeden okunur).
   - **Sorgu** (`suggest: true`): profili olmayan öğenin `suggested`'ı, türüne uyan sınıfları olan profillerdir (adı,
     başlığı, sınıfları profilin sırasıyla). Sıra:
     - KentOS'un profilleri;
     - çizimin içindekiler;
     - proje klasöründekiler (aynı adı içerideki gölgeler; okunmayan profil atlanır, profil listesi nedenini söyler).
   - **Kurum profili:** kurumun profili çizimin içinde ya da klasörde durur, aynı alanla kendi sınıflarını önerir.
     Öğe türü–sınıf eşlemesi kodda yok.
   - **KentOS'un profillerinin işareti** (KentOS'un seçimi, kural değil):
     - Harita varlıkları: arazi ölçümü ve çalışma verisi;
     - İmar planı: tasarım;
     - TUCBS temel: teslim.

     Altlık ve denetim için KentOS'un önerisi yok.
3. **Masaüstü** (Harita işi paneli; yeni şerit düğmesi yok):
   - **Önerilen:** Tipli katman'da seçili öğenin profili yoksa ve önerisi varsa **Önerilen** satırı görünür (profilin
     başlığı ve sınıf sayısı). Düğmesi önerilen sınıfları tek adımda verir (`map.work.edit profile`) ve söyler.
   - **Kayma sınırı** (Değişenler'in kayma tablosunun üstünde): esas seçilir (Sınır yok, Kendi değerim, Detay konumu,
     Çizim doğruluğu, Üretim yöntemi); yöntem ve ölçek kural dosyasının listelerinden.
   - Altında tek satırda değer ve yöntem/ölçek; kaynağın tamamı ipucunda.
   - Tabloda **Sınır** sütunu nesnenin hemen yanında (dar yuvada görünsün diye): “Dışında (k köşe)” ya da “İçinde”.
   - Tutmayan seçim (boş hücre) satırın altında kırmızıyla söylenir; sorgu sınırsız sürer, panel kapanmaz.
   - Değişenler'in ipucu sınırı aşan nesne sayısını söyler.
4. **Kapsam:** yalnız Harita (CAD), ADR 2880–4880 gibi.

## Sınırlar

- Üretim yöntemi nesnede kayıtlı değil:
  - sınırı kullanıcı seçer;
  - m. 9(2)'nin “birden çok yöntemde en büyük değer” kuralı kendiliğinden uygulanmaz.
- Ek-1'in URETIM_YONTEMI kodları tablonun sütunlarıyla eşlenmedi (tam örtüşmüyor).
- m. 11 (aplikasyon yanılma sınırı: iki değerin karesel ortalaması) ve m. 8(5) (hata çemberiyle alan yanılma sınırı)
  yok.
- Sınır projede saklanmaz.
- Listelenmeyen nesneler (1000'den sonrası) sayılmaz.
- Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_work_cases.py`; KentOS kodu yok):
  - **değerler:** tablo ve BÖHHBÜY değerleri yönetmeliklerden betiğe ayrıca yazıldı. Kural dosyası okunmaz, durumlar
    onu denetler;
  - **query 41'den 71 duruma (30 yeni):**
    - 10 sınır (her esas; 0,269 m kayan çizgi iki yanda: 0,09, 0,2, 0,25, 0,07 dışında; 0,3, 0,35, 0,4, 0,5 içinde);
    - eşlenmeyen çizgi Hausdorff'la; değişen nesne yok; plan;
    - 10 ret; kimlikten önce sınır; harita projesi değil;
    - 5 öneri (türlere göre, profili olan öğe, çizimin içindeki kurum profili, önerisiz sorgu).
  - Bozma denemesi: bir `beyond` ve bir önerinin sınıfı değiştirilince koşucu tam o ikisini yakalar (71'de 2).
- **Yerleşik profillere `work` eklenince** profil kullanan komutların ortak durumları değişmeden geçer:
  `cad.profiles.*`, `cad.zoning.*`, `map.features.*`, `map.work.*`.
- **Çekirdek ve komutlar:** `map_work_limit` testleri (tablonun hücreleri, boş hücre ve tabloda olmayan ölçek reddi,
  çizimin değeri).
- **Python** (`python/tests/test_map_work_limit_suggest.py`, bağımsız `.kcad` okuyucusuyla): sayısal 1/1000'de iki uç
  dışında, takeometrik tam değerle; çizim değeri; boş hücre reddi; tasarıma imar planı. Python ortak durumları geçer.
- **MCP** (`crates/native/mcp/tests/map_work_limit_suggest.rs`).
- **Masaüstü** (`map_work_limit_tests`):
  - öneriyi verme;
  - sınır sütunu (dışında 2 köşe, içinde);
  - boş hücre.

  Çapraz denetimler 156 test. Resimler `.run/gui-evidence/kayma-siniri-5280/`.

## Sonra

- Nesnenin üretim yönteminin kaydı (Ek-1 URETIM_YONTEMI ile eşleme) ve m. 9(2)'nin en büyük değeri.
- Aplikasyon yanılma sınırı (m. 11).
- Sınırın projede saklanması.
- Eklenenleri de silen tam onaylı hâl (sahip kararı).
- Web (TODOS §0.2).
