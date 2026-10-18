# ADR 0512: Bağlı katmanda geniş değerlerin anlamı

- **Durum:** kabul edildi (2026-10-18). `plan/gis.md` CBS-06'nın bir dilimi: GeoPackage'a bağlı katmanın gidiş-dönüşü
  2⁵³'ü aşan tam sayıyı, kısa ondalığı olmayan sayıyı, boş metinle NULL'u, 0 ile false'u ve saat dilimli zamanı bozmaz;
  dosya, sağlayıcı, tablo ve Python yolunda anlam aynıdır.
- **Bağlam belgesi:** ADR 0502 (GeoPackage), 0503 (bağlı katman), 0199 (katman alanları).
- **Sonraya kalan:**
  - öznitelik modelinin kendi türleri (int64, kesin ondalık, UUID, saat dilimli an, JSON ve dizi, ikili başvuru): bugün
    öznitelik metindir;
  - PostGIS sağlayıcısında aynı denetim;
  - alan adı değişiminin kararlı alan kimliğiyle stil, ifade ve birleştirmeye kadar izlenmesi (formlarda var, ADR 0414).

## Bağlam

Okuma kuralları ADR 0502'dedir:

- NULL özniteliğin yokluğu, boş metin boş metindir.
- 2⁵³'ü aşan tam sayı her basamağıyla metin alanına alınır, söylenir.
- Zamanın dilimi alınmaz, söylenir.

Ama eşitleme bir satırın değişip değişmediğine hücreleri sayıya çevirerek bakıyordu (`as f64`). Bundan iki sonuç
çıkıyordu:

- `9007199254740993` ile `9007199254740992` aynı sayılıyordu: elle değiştirilen tam sayı yazılmıyordu.
- Dilimli zaman (`…+03:00`) çizimde dilimsiz durduğu için hiç dokunulmamış satır değişmiş sayılıyor ve geri
  yazılıyordu. Dosyaya ne gideceği açılıştan hemen sonraki eşitleme planında görünüyordu.

## Karar

1. Eşitleme hücreleri **sütunun okuma kuralıyla** karşılaştırır (`geopackage::same_cell`). İki hücre, çizime aynı
   öznitelik metniyle okunuyorsa aynıdır:
   - tam sayı basamaklarıyla (2⁵³'ün ötesinde de);
   - ondalık sayı okunduğu en kısa metinle;
   - zaman tarih ve günün saatiyle: çizimin tutmadığı dilim değişiklik değildir, satır onun için yeniden yazılmaz;
   - NULL yalnız NULL'la.
2. Sütunu bilinmeyen hücre (tablonun okunmayan sütunu) yalnız aynısıyla aynıdır.
3. Kayıp bağlamadan önce söylenir (`import_losses`): zamanın dilimi, metin alanına alınan büyük tam sayı. Python'da bu
   uyarı `CommandWarning`'dir.

## Doğrulama

- `fixtures/geopackage/v1/genis.gpkg`'yi GDAL'ın OGR'ı yazar (`wide_values_cases.py --files`). İçinde:
  - 2⁵³ + 1 ve en küçük 64 bitlik tam sayı;
  - 0 ve false;
  - 0,1 ve kısa ondalığı olmayan bir sayı;
  - boş metin ve NULL;
  - +03:00 dilimli ve UTC zaman;
  - tırnaklı metin.
- Uygulamanın testi (`sources.rs`'in `wide_values_…`'u) dosyayı bağlar:
  - bağlamadan hemen sonra plan hiçbir satırı değişmiş saymaz;
  - bir satırın metni değişir ve geri yazılır, yalnız o satır yazılır;
  - 2⁵³ + 1 → 2⁵³ değişikliği bir değişiklik sayılır.
- `KENTOS_WRITE_GPKG=1` ile yazılan `kentos-genis.gpkg`'yi `wide_values_cases.py` sqlite3'le kaynakla hücre hücre
  karşılaştırır: değişen metin dışında her hücre değeri ve SQLite türüyle aynıdır.
- `python/tests/test_wide_values.py` aynı yolu SDK'dan yürür: değerlerin metinleri, NULL'un yokluğu, kayıp uyarısı,
  gidiş-dönüşte dosyanın hücreleri.
