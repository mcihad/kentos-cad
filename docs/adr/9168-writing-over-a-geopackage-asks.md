# ADR 9168: Var olan GeoPackage'ın üstüne yazmadan önce silinen ve yazılan tablolar söylenir

- **Durum:** kabul edildi (2026-11-23); adım VY-67. Plan: [plan/veri.md](../../plan/veri.md) VY-67.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 6. maddesinin
  kalanı (ADR 9139 §4): “Son onayda hedef dosya, yeni/var olan tablo, kapsam, geometri sayısı ve taşınan
  stil/form/ilişki bilgileri listelensin.” Dosya işletim sisteminin penceresinde seçildiği için özet ondan önceydi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok; `file.export.gpkg`'nin
  `overwrite`'ı aynı (dosyanın tümü).

## §1 Bağlam

- GeoPackage dışa aktar'ın penceresi dosya sorulmadan önce kapsamı, katmanları sayılarıyla, giden stil ve ilişkileri
  ve gitmeyenleri söylüyor (ADR 1073, 9139).
- Dosya seçilince yazım hemen yapılıyordu (`overwrite: true`). Var olan dosyanın yerine yenisi yazılıyordu. Dosyada
  bu dışa aktarmanın yazmadığı tablolar varsa kayboluyordu ve kişi bunu dosyayı seçtikten sonra göremiyordu.
- **Araştırma.** Masaüstü CBS'lerde vektör katman var olan bir dosyaya kaydedilirken, biçimin yeteneklerine göre
  (GeoPackage, SpatiaLite…) kişi şunlardan birini seçer: bütün dosyanın üstüne yazmak, yalnız hedef katmanın üstüne
  yazmak, var olan katmana nesne eklemek ya da nesneyle birlikte yeni alanları da eklemek. Kaynak:
  [QGIS “Creating Layers”, var olan dosyaya kaydetme](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html).

## §2 Karar

1. **Var olan dosyada önce sorulur.** Seçilen dosya varsa yazmadan önce pencere bir onay gösterir. Dosyanın
   tabloları okunur (`GeoPackage::layers`) ve onay şunları söyler:
   - “… zaten var; Üstüne yaz dosyanın yerine yenisini yazar.”;
   - **Dosyadan silinen tablolar**: bu dışa aktarmanın yazmadığı tablolar, sayılarıyla;
   - **Yeniden yazılan tablolar**: aynı adla yeniden yazılanlar;
   - **Yeni tablolar**;
   - katman stilleri tablosu dosyada varsa yeniden yazıldığı (her dışa aktarma yazar; kaybolmaz);
   - **Yazılan**: her tablo nesne sayısıyla.

   Dosya GeoPackage olarak okunamıyorsa bu da söylenir.
2. **Düğmeler.**
   - “Üstüne yaz” yazar. Salt okunur dosya yine ezilmez, nedeni söylenir (ADR 9139).
   - “Geri” hiçbir şey yazmadan kapsam özetine döner.
3. **Yeni dosya** sorulmaz. Kaybolan bir şey yoktur ve neyin gittiği dosya seçilmeden önce söylenmiştir.
4. **Kapsam dışı.** Yalnız hedef tablonun üstüne yazmak ya da var olan tabloya eklemek bu kararın konusu değildir
   (§4). Bugün dışa aktarma her zaman dosyanın tümünü yazar ve onay bunun sonucunu gösterir.

## §3 Kanıt

- **Masaüstü** (`shell/exchange/gpkg_tests.rs`):
  - kendi `eski_tablo`su olan bir dosya seçilince onay şunları söylüyor: “Dosyadan silinen tablolar (1): eski_tablo.”,
    yeni tablolar, yazılanlar;
  - Geri dosyayı olduğu gibi bırakıyor;
  - Üstüne yaz yeni dosyayı eski tablosuz yazıyor.
- **Teslim sınaması** (`gpkg_tests/delivery.rs`): aynı dosyaya ikinci dışa aktarma şunları gösteriyor:
  - onay Türkçe adlı tabloyu “yeniden yazılan” sayıyor, silinen tablo yok, stil tablosu ayrı satırda;
  - salt okunur dosyada Üstüne yaz sonrası dosya olduğu gibi kalıyor.
- Dışa aktarma, GeoPackage ve ortak masaüstü sınamaları geçiyor (345).
- **Resim:** `.run/gui-evidence/kullanim-9168/01-uzerine-yaz-{light,dark}.png`. Onayda “teslim.gpkg zaten var”,
  silinen “eski_tablo”, yeni tablolar ve yazılanlar; Geri ve Üstüne yaz.

## §4 Kalan

- Var olan dosyada yalnız hedef tabloların üstüne yazmak ya da tabloya eklemek.
- Yarıda kesilen dışa aktarmanın sınaması (`file.export.gpkg`'nin durdurulması yok).
- Web (TODOS.md §0.2).
