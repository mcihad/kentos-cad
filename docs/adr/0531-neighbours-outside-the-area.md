# ADR 0531: Alanın dışındaki komşularla topolojik yazma

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-12'nin ilk dilimi: dosya kaynağının (GeoPackage) alan sınırlı
  parçası. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0160 (topolojik düzenleme: `topology_edit::changes`, `apply`), 0503 (bağlı katman), 0510 (alan
  sınırlı parça, `Stored.outside`), 0504 (kaynağın sistemi), 0254 (`topology_follow`).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - PostGIS kaynağı: sağlayıcının kutu sorgusu alan sınırı almıyor (veritabanı her zaman bütün tablo), alan sınırlı PostGIS
    parçası ve onun komşuları CBS-02 ile gelir;
  - komşunun kendi geçerliliğinin (kendini kesme, halkaların dönüşü) denetlenmesi: bugün ADR 0160'ın kuralı, yani yalnız köşesi
    azalıp çöken halka geçersiz sayılır;
  - kilitli komşu (dosyada kilit yok) ve izinler (CBS-38); komşunun öznitelik kuralları (formlar, CBS-15);
  - masaüstünde yazmadan önce izleyecek komşuların çizimde gösterilmesi (önizleme).

## Bağlam

Alan sınırlı bağlı katman (ADR 0510) tablonun yalnız görünen parçasını taşır; dışarıda kalan satırlar çizimde yoktur
(`Stored.outside`: kalıcı kimlik → fid). Topoloji açıkken çizimdeki komşular ortak köşe ve kenarları izler (ADR 0160), ama
alanın dışındaki komşu izleyemez: kaynağa yazılınca ortak sınır açılır ya da örtüşür. Plan iki tuzak söyler: görünüm dışındaki
komşunun sessizce bozulması, yarım yazma.

## Karar

1. **Seçenek komutun girdisinde:** `cad.sources.sync`'in `topology`'si (`strict`, `loose`; yoksa izlenmez, eski davranış).
   Okumada verilirse `topology_on_read` reddi.
2. **Değişiklikler katmandan:** yazılacak nesnelerin kaynaktaki hâliyle çizimdeki hâli arasındaki köşe değişiklikleri
   `topology_edit::changes` ile çıkarılır (çizgi, çoklu çizgi, alan).
3. **Komşular dosyanın aynı sürümünden:** değişikliklerin bağ noktalarının kutusu (SAME payıyla), gerekiyorsa tablonun
   sistemine çevrilip (`table_area`) tablodan yeniden okunur. Dosyanın sha256'sı bağın sürümünden farklıysa
   `source_changed` çakışması olur. Adaylar yalnız `outside`'da olup çizimde olmayan satırlardır.
4. **Aynı kural:** komşular `topology_edit::apply` ile izler; köşesi azalınca çöken komşu geçersizdir.
   - `strict`: geçersiz komşu varsa `outside_neighbour_invalid` reddi verilir, hiçbir şey yazılmaz.
   - `loose`: geçersiz kalacak komşu olduğu gibi kalır, `neighbours_left` uyarısı verilir.
5. **Tek işlem:** izleyen komşular kendi fid'leriyle, tablonun sisteminde, katmanın değişiklikleriyle aynı SQLite işleminde
   güncellenir. Cevapta `followed` (izleyen) ve `left` (bırakılan) sayılır; plan da sayar, yazmaz.
6. **Masaüstü:**
   - Katmanlar'ın Kaynak ▸'sindeki Kaynağa yaz, alan sınırlı parçada durum çubuğunun Topoloji'si açıkken `strict` yazar.
   - Parçada ayrıca Kaynağa yaz, komşular gevşek (`loose`) seçeneği vardır.
   - İleti izleyen komşuları sayar.

## Doğrulama

- `python3 scripts/fixtures/bound_sources_cases.py --check`:
  - sekiz yeni ortak durum: izlemesiz yazma, `strict` ve `loose` taşıma, `loose`'da bırakılan üçgen, `strict` reddi, değişiklik
    yok, okumada ret, bütün tablo;
  - örnek dosya `fixtures/geopackage/v1/komsu.gpkg`'yi GDAL yazar (`--files`);
  - komşu kuralı KentOS kodu olmadan yazılmıştır;
  - yazılan `kentos-komsu.gpkg`'nin halkaları OGR'la denetlenir.
- `KENTOS_CASES_ONLY=cad.sources cargo test -p kentos-native-application --test all every_command_case`.
- `KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all a_part_writes_its_neighbours`: geçici kopyaya
  yazar. Planın yazmadığı, üç parselin çıkıntısının bir metre doğuda olduğu denetlenir.
- `cargo test -p kentos-desktop a_part_writes_its_neighbours_while_topology_is_on`. Resimler:
  `cargo test -p kentos-desktop exchange::gpkg_tests::neighbour_screens -- --ignored --nocapture`
  (`.run/shots/gpkg-*-10-komsular-*`).
