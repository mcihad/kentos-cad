# ADR 9171: Topoloji sonucu veri katmanının kaynağı dışarıda değişince eskir

- **Durum:** kabul edildi (2026-11-23); adım VY-70. Plan: [plan/veri.md](../../plan/veri.md) VY-70.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 14. maddesinin
  kalanı (ADR 9146 §4): “Veri katmanının kaynağında dışarıdan yapılan değişiklik ve katmanın başka tabloya bağlanması
  eskimeyi tetiklemiyor.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Topoloji denetimi bir veri katmanının tablosunu bütün olarak kaynağından okur (ADR 9124).
- Sonucun eskidiğini çizimin değişiklik günlüğü söyler (ADR 9146). Bağlı katmanların kaynak sürümü de eskitir.
- Veri katmanının kaynağında çizimin dışında yapılan yazım (başka bir program, başka bir oturum) çizimi değiştirmez.
  Katmanın başka bir dosyaya bağlanması da değiştirmez (Yeniden bağla, ADR 9155). Bu iki durumda sonuç güncel
  görünüyordu.
- **Araştırma.** Masaüstü CBS'lerin topolojisinde doğrulamadan sonraki her düzenleme, yeniden doğrulanması gereken
  yer olarak işaretlenir; yalnız o yerler yeniden işlenir. Kaynak:
  [ArcGIS Pro, Validate a topology](https://doc.esri.com/en/arcgis-pro/latest/help/data/topologies/validating-a-topology.html).

## §2 Karar

1. **Okunan kaynak kaydedilir.** Denetim bir veri katmanının tablosunu okurken şunları saklar (`SourceRows`):
   - katmanın kaynağını ve tablosunu;
   - dosyanın parmak izini (`topology_check::data_fingerprint`): GeoPackage dosyasının boyu ve son yazılma anı, varsa
     yazım günlüğü dosyasınınki (`-wal`) de. Henüz ana dosyaya katılmamış yazımlar da böylece görülür.
2. **Eskime.** Sonuç şu durumlarda eskidir:
   - katman başka bir kaynağa ya da tabloya bağlandı;
   - katman kalmadı;
   - dosyanın parmak izi değişti.

   Bu denetim çizimin değişmesine bağlı önbelleğe girmez. Dışarıdaki yazım çizimi değiştirmez; dosya bilgisini okumak
   ucuzdur. Kendi değişikliklerini kaynağa yazmak da dosyayı değiştirir; sonuç temkinli olarak eskir.
3. **Söz ve yol.** Sekmenin notu şöyle der: “… katmanının kaynağı son denetimden sonra çizimin dışında değişti (yazıldı
   ya da başka tabloya bağlandı); sonuçlar eski olabilir. Denetle ile yenileyin.” Altında Değişenleri denetle değil
   **Denetle** durur, çünkü değişenleri denetlemek çizimin günlüğüne bakar ve kaynaktaki değişikliği görmez.
4. **Veritabanı.** PostGIS gibi değişikliğini bu yolla söylemeyen kaynağın parmak izi yoktur. Onun dışarıdaki yazımı
   bugün eskitmez (§4).

## §3 Kanıt

- **Masaüstü** (`gis/topology/source_tests.rs`): parsellerin GeoPackage tablosu veri katmanı, “Çakışmamalı” kuralı
  var.
  - Denetimden hemen sonra sonuç güncel.
  - Başka bir program dosyaya dördüncü parseli yazınca sonuç eskiyor; değişen kaynak “parseller” diye söyleniyor.
  - Yeniden denetlenince güncel, değişen kaynak yok.
  - Katman başka bir dosyaya yeniden bağlanınca yine eskiyor.
- Topoloji sınamaları (39), uygulamanın topoloji sınamaları ve paylaşılan komut durumları geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9171/01-disaridan-degisti-{light,dark}.png`. Topoloji sekmesinde
  “parseller katmanının kaynağı … çizimin dışında değişti …” notu ve altında Denetle.

## §4 Kalan

- PostGIS kaynağının dışarıdaki yazımı: veritabanının değişiklik sayacıyla (örneğin tablonun istatistikleri) gerçek
  sunucuda.
  **Yapıldı (23 Kasım 2026):** `Source::change_mark` (PostGIS'te `pg_stat_all_tables` sayaçları), sekme açıkken
  arka planda sorulur; gerçek sunucuda sınandı, [ADR 9182](9182-postgresql-remainders-on-a-real-server.md) §2.4.
- Kendi kaynağa yazımlarını dışarıdakinden ayırıp sonucu gereksiz eskitmemek.
- Kurumun gerçek eşik değerleri (ADR 9146 §4).
- Web (TODOS.md §0.2).
