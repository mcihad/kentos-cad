# ADR 0585: Veritabanında alanın dışındaki komşular, yetki ve tam geçerlilik

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-12: dış sağlayıcıda görünüm dışı komşularla topolojik yazma.
- **Bağlam belgesi:**
  - ADR 0160: yerel topolojik düzenleme (MEVCUT-01; değişmez);
  - ADR 0201 §6: Geçerliliği denetle;
  - ADR 0531: GeoPackage'ta alanın dışındaki komşular;
  - ADR 0582: PostGIS'te alan ve satır düzeyinde çakışma;
  - ADR 0584: aynı veritabanına tek işlem.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **Sonraya kalan:**
  - satır güvenliğinin SELECT politikasının gizlediği komşu: hesap onu göremez, izleyemez de; bunu söylemenin yolu
    veritabanının politikasını okumaktır, bugün yapılmıyor (§2);
  - eğrili (yaylı) komşu kenarın izlemesi ADR 0160'ın kuralıyla düzleştirilir; kaynağın eğri türü korunmaz.

## Bağlam

ADR 0531'den beri alan sınırlı bir parçanın yazması, ortak köşeleri değişen ve çizimde olmayan komşu satırları da
yazıyordu. Bu yalnız GeoPackage'ta çalışıyordu: `follow_outside` dosyayı yeniden okuyordu, PostGIS kaynağında hiçbir komşu
izlemiyordu. Parçalı PostGIS katmanında (ADR 0582) ortak sınır sessizce açılıyordu.

Üç eksik daha vardı:

1. Komşu ancak köşe sayısı üçün altına düşerse “geçersiz” sayılıyordu. Kendini kesen ya da alanı sıfıra inen komşu
   yazılıyordu.
2. Veritabanında hesabın değiştiremediği satır (yetki ya da satır güvenliği) yazmanın ortasında ortaya çıkıyordu.
3. Başka bir kullanıcının komşuyu, okumayla kilit arasında değiştirmesi denetlenmiyordu.

## Karar

### 1. PostGIS'te komşular

Parçanın yazmasında `topology` verilince komşular kaynaktan okunur:

- **GeoPackage:** dosya değişikliklerin kutusuyla aynı dosya durumundan yeniden okunur (ADR 0531, değişmedi).
- **PostGIS:** değişen köşelerin kutusu tablonun sistemine çevrilir, satırlar sağlayıcının kutu sorgusuyla (`&&`, dizinle)
  okunur. Parçanın alanında okunmamış her satır aday komşudur.

İzleme mevcut çekirdeğin işidir (`topology_edit::changes`, `apply`); yeni geometri kuralı yazılmadı. İzleyen satırlar
katmanın kendi değişiklikleriyle aynı işlemde, ADR 0584'ün tek işleminde yazılır.

Okunan her komşunun anahtarı, kalıcı kimliği ve özeti saklanır (`Followed.expect`). Tablo kilitlendikten sonra komşunun
satırları yeniden okunur ve özetle karşılaştırılır (`neighbours_changed`). Arada başkası değiştirdiyse ya da sildiyse
yazma `source_changed` çakışmasıyla döner:

- yol `topology`, `uids` komşunun kalıcı kimliği;
- hiçbir şey yazılmaz;
- yeniden yazmak komşuyu yeni hâliyle okur.

Bu kural iki profilde de geçer: sürüm engeli kayıp yazma demektir, gevşek profil bunu da atlamaz.

Sağlayıcının sözleşmesi buna göre genişledi. `check` verilince alanın sürümü de denetlenir, yeter ki `revision` boş
olmasın. Tabanı olan katman boş verir (satır satır, ADR 0582 §2). Tabanı olmayan katman alanın sürümünü verir ve komşular
için yine satır denetimini kullanır.

Masaüstünde `topology` yollu `source_changed` Kaynakla uzlaştır penceresini açmaz. Uyuşmazlık katmanın değil komşunun
olduğu için ileti söylenir.

### 2. Yetki ve satır güvenliği

PostGIS sağlayıcısı, yazmadan önce güncellenecek ve silinecek satırları iki kez sorar:

- düz `SELECT` ile: görünenler;
- `SELECT … FOR UPDATE` ile: değiştirilebilenler. Bu sorgu UPDATE yetkisini ve satır güvenliğinin UPDATE politikalarını
  uygular.

Görünüp değiştirilemeyen satır varsa yazma yeni `rows_denied` kodlu retle döner (`ProviderError::RowsDenied`), hiçbir
şey yazılmaz:

- yetki hiç yoksa (SQLSTATE 42501) bütün anahtarlar;
- yazma sırasında 42501 gelirse o satır.

Reddedilen satırlar komşuysa yol `topology` olur, ileti komşuları izlemeden yazmayı önerir; katmanın kendi satırıysa
yol `layer` ya da `with` olur. `uids` her durumda satırların kalıcı kimlikleridir.

Hesabın hiç göremediği satır (satır güvenliğinin SELECT politikası) okunmaz. İzlenmez, söylenemez de (“Sonraya kalan”).

### 3. Komşunun tam geçerliliği

Bir komşu, izledikten sonra iki durumda geçersiz sayılır:

- köşe sayısı halkayı kuramıyorsa (ADR 0531);
- Geçerliliği denetle'nin (ADR 0201 §6) bulduğu bir sorun yeni ortaya çıkıyorsa: kendini kesen halka, alanı sıfır olan
  halka, dış halkanın dışına taşan ya da örtüşen delik.

Yinelenen köşe sayılmaz: GEOS da onu geçerli sayar.

İzlemeden önce de geçersiz olan komşu engel değildir, köşeyi izler. Yerel çizimin önceden bozuk verisi yazmayı kilitlemez.

- **Sıkı profil:** geçersiz kalacak bir komşu yazmayı durdurur (`outside_neighbour_invalid`).
- **Gevşek profil:** o komşu olduğu gibi kalır ve söylenir (`neighbours_left`).

Bu kural GeoPackage'ta da geçer.

## Doğrulama

- `scripts/fixtures/source_neighbours_cases.py`: 24 ortak durum (`fixtures/commands/v1/cad.sources.sync.neighbours.json`),
  KentOS kodu olmadan.
  - Dosya `fixtures/geopackage/v1/komsu-gecerlilik.gpkg`'dir. GDAL'ın OGR'ı yazar: alanın içinde A; dışında A'nın sağ
    kenarının altında B, üstünde C üçgeni, köşesinde önceden kendini kesen D papyonu.
  - İzleyen satırlar ADR 0160'ın kuralıyla yazılır. Geçerlilik önce ve sonra GEOS'un `IsValid`'iyle (OGR üzerinden)
    denetlenir.
  - Yazan durumlar geçici kopyada koşar; sonuç `cad.sources.query` ile WKT olarak geri okunur.
- `crates/native/postgis/tests/all.rs`'in `neighbours_outside_the_area_follow_in_the_database`'i,
  `scripts/fixtures/postgis_cases.py`'nin geçici PostGIS'inde (`komsu` şeması):
  - B, A'nın taşınan köşesini aynı işlemde izler;
  - kendini kesecek B sıkı yazmayı durdurur;
  - başka bir `psql` oturumu C'yi, yazma onu okuduktan sonra ve kilitten önce değiştirir; yazma kilidi bekler,
    `source_changed` (`topology`, C'nin kalıcı kimliği) ile döner, yeniden yazınca C'yi yeni hâliyle izler;
  - satır güvenliği B'yi değiştirmeyi yasaklayan `kisitli` hesabıyla komşulu yazma `rows_denied` (`topology`, B'nin
    kalıcı kimliği) ile döner, hiçbir şey yazılmaz; komşusuz yazmada A yazılır.
  - Betik veritabanının son hâlini psycopg2'yle ayrıca denetler.
- Masaüstünde `exchange::gpkg_tests::a_neighbour_left_invalid_stops_a_strict_write_and_says_so` ve
  `validity_screens` (`.run/shots/gpkg-*-11-gecerlilik-*`).
- ADR 0531'in GeoPackage durumları ve yerel topolojik düzenlemenin izleri değişmeden geçer.
