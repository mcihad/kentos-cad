# ADR 9162: Silinmiş satır yeni satır olarak yalnız ekleme yetkisiyle yazılır

- **Durum:** kabul edildi (2026-11-23); adım VY-61. Plan: [plan/veri.md](../../plan/veri.md) VY-61.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 32. maddesinin
  kalanı (ADR 9152): “Kullanıcı isterse ve ekleme yetkisi varsa **Yeni kayıt olarak oluştur** eylemini ayrıca sun.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Kaynakta silinmiş, burada değişmiş satır kaydedilirken çakışma penceresi iki yol sunuyordu (ADR 9152): “Yeni satır
  olarak ekle” ve “Değişikliğimi bırak”. Bağlantının tabloya satır ekleme yetkisi olup olmadığına bakılmıyordu.
  Yetkisiz hesap ilk yolu seçerse yazım veritabanının reddiyle düşüyordu.
- Veri kaynağının sözleşmesinde ekleme, değer değiştirme, şekil değiştirme ve silme yetenekleri ayrı alanlardır
  (`kentos_data::Capabilities`). Ama PostGIS sürücüsü dördünü de tek bir “yazabilir” bayrağından türetiyordu. Bayrak
  INSERT, UPDATE ya da DELETE'ten birinin verilmesiyle açılıyordu. Yalnız UPDATE yetkisi olan hesap da “ekleyebilir”
  sayılıyordu.
- **Araştırma.** PostgreSQL'de tablo yetkileri ayrı ayrı verilir:
  - INSERT tabloya yeni satır eklemeye izin verir;
  - UPDATE sütunları değiştirmeye izin verir;
  - DELETE satır silmeye izin verir.

  Bir hesabın tablo üzerindeki yetkisi `has_table_privilege` ile tek tek sorulur. Kaynak:
  [PostgreSQL 5.9 Privileges](https://www.postgresql.org/docs/current/ddl-priv.html).

## §2 Karar

1. **Sürücü.** PostGIS tablonun biçimini okurken üç yetkiyi ayrı sorar (`catalog::Rights`).
   - Yetenekler bunlardan gelir: ekleme INSERT'ten; değer ve şekil değiştirme UPDATE'ten (şekil yalnız geometrili
     tabloda); silme DELETE'ten.
   - Satırları hiç yazılamayan tabloda (görünüm, tek anahtarsız tablo) hiçbir yetki sayılmaz.
   - Eşleme saf bir işlevdir (`read::capabilities_of`).
   - “Yazabilir” bayrağının anlamı değişmedi: üçünden biri.
2. **Yazım yolu.** Çakışan her satır, tablonun bu bağlantıdan yeni satır alıp almadığını taşır
   (`data_clash::RowClash::may_add`). Bilgi kaydın kendi okumasındaki yeteneklerden gelir; ayrıca sorgu yapılmaz.
3. **Pencere.** Ekleme yetkisi yoksa kaynakta silinmiş satırın tek yolu vardır: taslak bırakılır.
   - Bu yol seçili gelir. Satır “Bırakılır: bu bağlantının tabloya satır ekleme yetkisi yok.” der.
   - Sayım bunu söyler: “… 0 satırı tek işlemde yazar; 1 satırın taslağı bırakılır.”
   - “Yeni satır olarak ekle” istense de bir şey değişmez. Çözüm adımı da bu satırı yeni satır olarak yazmaz.
   - Ekleme yetkisi olan bağlantıda iki yol eskisi gibidir. Eski anahtar kendiliğinden canlandırılmaz (ADR 9152).

## §3 Kanıt

- **Sürücü** (`kentos-data-postgis`):
  - yalnız UPDATE yetkisi olan hesap satır değiştirebiliyor, ekleyemiyor;
  - üç yetkili geometrisiz tabloda şekil değiştirme yok;
  - yazılamayan tabloda hiçbir yetki sayılmıyor.
- **Veritabanı sınaması:** `KENTOS_DATA_PG` verilince sınama bir tablo kurup yeteneklerini soruyor. Bu makinede
  Docker yanıt vermediği için koşturulmadı; uygunluk takımıyla birlikte atlandı. Atlanan sınama geçmiş sayılmaz.
- **Masaüstü** (`gis/data_conflict_tests.rs`, GeoPackage):
  - dosya kaynağı satır alıyor; aynı çakışma ekleme yetkisiz bağlantıdan geliyormuş gibi kuruluyor;
  - silinmiş satır için “bırak” seçili geliyor, pencere hazır;
  - “Yeni satır olarak ekle” istenince seçim değişmiyor;
  - kaydedince eski anahtar silinmiş kalıyor, yeni satır yazılmıyor, bekleyen değişiklik kalmıyor.
- Uygulamanın veri yazım ve oturum sınamaları ile paylaşılan komut durumları geçiyor (19). Çakışma, silinmiş satır ve
  günlük sınamaları da geçiyor (14).
- **Resim:** `.run/gui-evidence/kullanim-9162/01-ekleme-yetkisi-yok-{light,dark}.png`. “Satır 22 · parseller — Tabloda
  silinmiş; sizde değişmiş — Bırakılır: bu bağlantının tabloya satır ekleme yetkisi yok”, altında sayım.

## §4 Kalan

- Gerçek PostgreSQL'de yalnız SELECT ve UPDATE yetkili ikinci hesapla deneme.
- Sütun düzeyinde verilen INSERT ve UPDATE yetkileri; bu karar tablo düzeyindekini okur.
- Web (TODOS.md §0.2).
