# ADR 9158: Özet istatistik tablosu dosyaya sayı olarak yazılır

- **Durum:** kabul edildi (2026-11-23); adım VY-57. Plan: [plan/veri.md](../../plan/veri.md) VY-57.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 23. maddesinin
  kalanı (ADR 9136 §4): öbür tablo veren araçlar türlerini söylemiyor; Özet istatistik'in sonucu GeoPackage'da
  hâlâ metin.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `cad.processing.run`'ın `statistics.summary` sonucuna ADR 9136'nın `kinds` ve `values` alanları eklendi;
  `rows` aynı kaldı.

## §1 Bağlam

- ADR 9136 işlem tablolarına sütun türlerini ve düz değerleri getirdi; ilk araç Histogram'dı. Özet istatistik
  (`statistics.summary`) türsüz kaldı. Dosyaya yazılınca Nesne, Değer, Toplam, Ortalama, En az, En çok ve Std. sapma
  metin sütunlarıydı; `"Toplam" > 100` sayıyla süzmüyordu.
- Özet istatistik'in hücreleri gösterim için yuvarlanmış metin değildir: toplam, en az ve en çok kaynağın ondalık
  değerlerinden kesin hesaplanır; ortalama ve standart sapma kaynağın en büyük ölçeğinin iki basamak fazlasıyla yazılır
  (`kentos_geometry_core::ops::statistics`). Hücrenin metni aracın sonucudur.
- **Araştırma.** GeoPackage öznitelik sütunlarının türleri SQLite'ın INTEGER, REAL ve TEXT'idir; GDAL'ın GeoPackage
  sürücüsü onları Integer64, Real ve String okur. Değeri olmayan hücre NULL'dur; boş metin bir değerdir ve sayı
  sütununda karşılaştırmayı bozar. Kaynak: [GDAL GeoPackage sürücüsü](https://gdal.org/en/stable/drivers/vector/gpkg.html).

## §2 Karar

1. **Türler.**
   - Grup sütunu (gruplanınca) metin.
   - Nesne ve Değer sayıları INTEGER.
   - Toplam, Ortalama, En az, En çok ve Std. sapma REAL.
2. **Düz değerler.** Her hücrenin düz değeri kendi metninden okunur, çünkü metin aracın kesin sonucudur:
   - sayı sütununda boş hücre (tek değerli grubun standart sapması, hiç değer okunmamış grubun toplamı) NULL'dur;
   - REAL, kesin ondalık metnin en yakın çift duyarlıklı sayısıdır. Kesin metin pencerede, Panoya kopyala'da ve
     komutun `rows`'unda kalır.
3. **Yazıcı değişmedi.** ADR 9136'nın kuralı geçerli: türü söylenmiş ve her satırının düz değeri olan sütun sayı olur.

## §3 Kanıt

- **Uygulama** (`tests/all/processing_typed_tables.rs`): ortak sorgu çiziminin `yapi` katmanı “Taban alanı”
  alanıyla, “Kat”a göre gruplanıp GeoPackage'a yazılıyor. SQLite'ın kendisine soruluyor:
  - Grup metin, Toplam satırının Nesne'si integer, Toplam'ı real;
  - tek yapılı 1. kat grubunun Std. sapma'sı null;
  - Toplam satırının Toplam'ı 231,5.
- **Ortak durumlar:** `fixtures/processing/v1/queries.json`'ın Özet istatistik durumları türleri ve düz değerleri
  taşıyor (`scripts/fixtures/spatial_query_cases.py`, `--check` tutarlı). `fixtures/commands/v1/cad.processing.run.json`
  yeniden üretildi (`scripts/fixtures/processing_command_cases.py`, `--check` tutarlı: 89 + 25 durum).
- İşlem çekirdeği sınamaları, komut durumlarının masaüstü işleyicileriyle eşleşmesi ve türlü tablo sınamaları (3)
  geçiyor.
- Görünür arayüz değişmedi; resim çekilmedi.

## §4 Kalan

- Öbür tablo veren araçlar (bölgesel istatistik, uzaklık ve kaynak-hedef matrisleri, komşular ve diğerleri) hâlâ
  türsüz; her biri `kinds` ve `values` ekledikçe dosyaya sayı yazar.
- Kaydet ▾ (XLSX/ODS/CSV) hâlâ gösterilen metni yazar.
- PostGIS sınamasının gerçek veritabanında koşturulması.
- Web (TODOS.md §0.2).
