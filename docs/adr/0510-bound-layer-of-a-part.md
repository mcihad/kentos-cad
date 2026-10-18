# ADR 0510: Bağlı katmanın parçası: alan sınırlı çalışma kopyası

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-02 ve CBS-42'nin dilimi: büyük bir GeoPackage tablosunun
  yalnız bir alanı çalışma kopyası olur. Bütün tablo okunmaz; eşitleme alanın dışına dokunmaz.
- **Bağlam belgesi:** ADR 0503 (bağlı katman), 0504 (kaynağın sistemi ve kutu dönüşümü), 0509 (ölçüm tabanı).
- **Sonraya kalan:**
  - alanı sonradan değiştirmek (bugün bağı çözüp yeniden bağlanır);
  - görünüme göre kendiliğinden okuma (kaydırdıkça sayfa sayfa);
  - PostGIS tablosunda alan (sağlayıcının okumasına kutu);
  - dosyanın SHA-256'sı bütün dosyadan hesaplanır (1 milyon parsellik dosyada sürenin çoğu): sürümü daha ucuz bir
    damga (SQLite'ın değişiklik sayacı, tablo başına) ile karşılaştırmak.

## Bağlam

ADR 0509'un ölçümü: 1 milyon parsellik tabloyu bağlamak 13,7 s ve 3 GB sürer. Sağlayıcının kendi kutu sorgusu ise
0,6 ms'dir. Kurum verisinde çoğu iş bir mahallenin, bir paftanın alanında yapılır; bütün tablonun çalışma kopyası
gereksizdir.

## Karar

1. **Bağın alanı:**
   - `SourceLink.extent` projenin sisteminde `[minX, minY, maxX, maxY]`; isteğe bağlı, `.kcad`'de bağla birlikte saklanır
     (şema 73 içinde isteğe bağlı alan).
   - `cad.sources.edit`'in `link`'i `extent` alır, yalnız dosyadan yeni katmana bağlamada.
   - `invalid_extent`: ters ya da sonlu olmayan alan, var olan katman, geometrisiz tablo ya da veritabanı.
2. **Okuma:**
   - Alan tablonun sistemine kutu dönüşümüyle çevrilir (ADR 0504'ün `transform_bounds`'u, kenarlar sıklaştırılarak).
   - Satırların kutusu o alana değenler okunur: mekânsal dizinle, dizin yoksa her geometrinin kendi kutusuyla.
     GeoPackage crate'inin kutu sorgusu dizinsiz tabloda bütün satırları sessizce döndürüyordu; artık süzer.
   - Geometrisiz satır alanda değildir.
3. **Eşitleme:**
   - Okuma ve yazma aynı alanla yapılır. Alanın dışındaki satırlar silinmiş sayılmaz.
   - Alanın dışına taşınan nesne yeni satır değil, kendi satırına yazılır. Dışarıdaki satırların kalıcı kimlikleri
     `fids` ve `kentos_uids` ile bilinir.
   - Tablonun sürümü bugünkü gibi bütün dosyanın SHA-256'sıdır: alanın dışında bir değişiklik de çakışmadır. Güvenli
     yön budur.
4. Python ve MCP katalogdan gelir (`extent`).
5. **Masaüstü:**
   - Kaynağa bağla penceresinde, Bağlı katman olarak aç işaretliyken **Yalnız görünen alanın satırları** seçeneği
     çıkar. Görünen alan ve boyu yazılır.
   - Seçilince geometrili her tablo görünen alanla bağlanır; geometrisiz tablo bütün bağlanır.

## Doğrulama

- Ortak durumlar `bound_sources_cases.py`'den. Satırlar OGR'ın zarflarıyla seçilir; KentOS kodu kullanılmaz.
  - Alan sınırlı bağlama: yalnız alandaki satırlar, alan bağda.
  - Dört ret.
  - Eşitlemenin planı: yazılacak bir şey yok, silinen yalnız kendisi silinecek, alanın dışına taşınan bir değişiklik.
- Masaüstü (`exchange::gpkg_tests`): görünüm ilk parsele getirilir ve seçenekle bağlanır; katmanda yalnız o parsel,
  bağda görünen alan. Resimler `.run/shots/gpkg-*-8-alan-*`, `gpkg-*-9-alan-bagli-*`.
- KCAD: bağımsız okuyucunun kuralı, örnek dosyada alan, iki bozuk dosya (ters alan, veritabanında alan).
- 1 milyon parsellik dosyada (`provider_timing`):
  - 400 × 300 m'lik alanın bağlanması 372 ms ve 24 MB, 374 nesne (bütün tablo 13,7 s ve 3 GB);
  - yazma planı 419 ms, tepe 109 MB.
