# ADR 0632: Birleşik görünümde kaynak tablosu, toplama sütunları ve dışa aktarma

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-09'un kalanı. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0527 (birleşik görünüm), 0581 ve 0592 (kaynağı sorgulamak, kaynaktan okumak: `sources_query::stage`),
  0582 (PostGIS'te view ve sorgu katmanı), 0505 ve 0623 (PostgreSQL adresi, parola cihazda), 0200 §6 (anahtarın kuralı),
  0199 §4 ve 0223 §3 (Öznitelik tablosu, Dışa aktar), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** görünümün çizimde katman olarak gösterilmesi (ana katmanın stili ve etiketi birleşen alanları okusun; QGIS'in
  katman birleştirmesi gibi ilk eşleşen ve toplama sütunları), birleşimin sağlayıcının SQL'inde yapılması (bugün iki taraf ayrı
  okunur, bellekte birleşir), görünümün okunuşunun arka plandaki iş parçacığında olması.

## Bağlam

ADR 0527 bir katmanın kayıtlarını başka bir katmanınkilerle canlı gösterdi. Plan iki şey daha ister: birleşen taraf çizimin
dışındaki bir kaynak olabilsin (bağlı GeoPackage ya da PostGIS tablosu; PostgreSQL view'u ve salt okunur sorgu katmanı ADR
0582'de yeniden açılabilen katmanlardır), ve aynı parselin üç maliki toplamı bozmasın. Malik sayısı ve pay toplamı gibi
**kayıt başına** özetler bugün ancak İşlemler'in İçindekinden bilgi al'ıyla kopya olarak yazılabiliyordu; kopya canlı değildir.

## Karar

### 1. Birleşen kayıtlar bir kaynak tablosu olabilir

Görünümün `source`'u `{path, table}`: bir GeoPackage dosyasının yolu ya da PostgreSQL veritabanının parolasız adresi
(`postgresql://kullanıcı@sunucu/veritabanı`; parola ADR 0623'ün sırasıyla cihazdan) ve tablonun, view'un ya da salt okunur
sorgunun adı. `with` o zaman boştur; ikisi birlikte yazılmaz (`invalid_join_view`). Adreste parola reddedilir: proje parola
taşımaz.

- Tablo görünüm her okunduğunda sağlayıcıdan **bütünüyle** okunur: `cad.sources.query`'nin ve İşlemler'in kaynaktan okumasının
  yolu (`sources_query::stage`, en çok 1 000 000 kayıt), kayıtlar kalıcı kimlikleriyle (GeoPackage'ta `kentos_uids` ya da
  `gpkg:<tablo>:<fid>`'in UUIDv5'i; PostGIS'te ADR 0505'in kuralı). Kopya saklanmaz.
- Okunamayan dosya ya da tablo görünümü **kopuk** yapar: nedeni söylenir, satır verilmez, eski bir okuma gösterilmez. Anahtar
  sütunu tabloda yoksa da öyle.
- Anahtar bir sütundur ya da `$uid` (kaydın kalıcı kimliği). Anahtarlar ADR 0527 §2'nin kuralıyla eşleşir (`007` ile `7`).
- Sütunlar tablonunkiler sırasıyla (`fields` verilmişse yalnız onlar). Kaynağın hücresi görünümden **yazılmaz**: neden “Kaynağın
  kaydı (…): görünümden yazılmaz; tabloyu katman olarak bağlayıp orada düzenleyin.” Yazmanın yolu bağlı katmandır (ADR 0503).
- Rapor kaynak dosyasının o anki işaretini verir (`sourceStamp`: boyu ve değişme zamanı, `cad.sources.query`'ninki;
  veritabanında yoktur): okuyan, satırların hangi dosyadan geldiğini bilir.
- `cad.joins.edit` kaynağı okumaz: görünüm şekliyle yazılır, okunamayan kaynağın görünümü okununca kopuktur.

### 2. Toplama sütunları

Görünümün `aggregates`'i en çok 20 sütun: `{name, how, field?}`. Her ana kayıt için eşleşen kayıtlarından:

| `how` | Değer |
|---|---|
| `count` | alansız: eşleşen kayıt sayısı; alanla: alanı boş olmayanların sayısı |
| `sum` | alanın sayılarının kesin toplamı (`kentos.statistics/1`, en büyük ondalık basamakta) |
| `min`, `max` | en küçük ve en büyük sayı |
| `mean` | toplam / sayı, en büyük basamak + 2'de, yarıda çifte |

Sayı olmayan değer atlanır. Eşleşmeyen kayıt `0` sayar, toplamı yoktur. Sütunlar birleşen alanlardan sonra gelir, salt
okunurdur (neden: “Toplama sütunu: birleşen kayıtlardan her okuyuşta hesaplanır.”). Kaydın bütün satırları aynı değeri gösterir;
alttaki toplamlar ADR 0527 §3'teki gibi her kaydı **bir kez** sayar: üç malikli parselin pay toplamı bir kez girer. Alan
birleşen katmanın alanının anahtarıyla ya da kaynağın sütun adıyla verilir; `cad.joins.edit` birleşen **katmanda** olmayan alanı
reddeder (`field_not_found`, `edit.view.aggregates.<i>.field`), kaynağınkini denetlemez (§1).

### 3. Dışa aktarma

Öznitelik tablosunda görünümün çubuğunda Dışa aktar ▾ (XLSX, ODS, CSV; ADR 0223'ün yazıcısı): görünümün bütün satırları, her
sütun başlığıyla (aynı başlık sayıyla ayrılır), toplama sütunları sayı, ötekiler metin. Kuralı uygulama katmanında
(`joins_query::sheet`); Python ve MCP `cad.joins.query`'nin satırlarını alır.

### 4. `.kcad` şema 105

Görünümün `source`'u ve `aggregates`'i şema 105'tir; yazıcı 105'i yalnız kaynaklı ya da toplama sütunlu görünüm varken yazar,
başka her çizim eskisiyle bayt bayt aynıdır. Şema 2–104'te iki alan bilinmeyen alandır. Örnek `fixtures/kcad/v2/join-sources.kcad`,
bozukları `broken/join-sources-*.kcad` ve `broken/schema-version-106.kcad`; bağımsız Python okuyucusu (`tools/kcad/kcad.py`)
aynı kuralı okur (`docs/specs/kcad-v2.md`).

### 5. Masaüstü

Birleşik görünümler penceresinde Birleşen: Katman | Kaynak tablosu. Kaynakta yol ya da adres, Dosya seç… (GeoPackage), Tabloları
oku (tablolar `cad.sources.describe` ile, arayüzün iş parçacığının dışında), tablo ve anahtar sütunu listeden. Toplama sütunları
satır satır: ad, ne yapacağı (Sayı, Toplam, En küçük, En büyük, Ortalama) ve alanı (Eşleşen kayıtlar ya da birleşen alanlardan
biri), Kaldır, Toplama sütunu ekle. Öznitelik tablosunda görünüm, kaynak tablosunun adıyla raporlanır.

## Doğrulama

- Ortak durumlar `fixtures/commands/v1/cad.joins.query.sources.json` (23 durum): GDAL'ın yazdığı `fixtures/joins/v1/tapu.gpkg`
  (tapu kayıtları; bir pay sütunu metin tutar), satırlar sqlite3'le okunur, kalıcı kimlikler UUIDv5'ten, istatistikler
  `spatial_query_cases.py`'nin kesin başvurusundan; kaynaklı görünüm, yalnız eşleşenler, seçilen sütun, altı toplama, katman
  görünümünde toplama, olmayan alan, sayfalar, kopukluklar (anahtar sütunu, tablo, dosya), şekil retleri, düzenlemeyle kurulan,
  güncellenen, reddedilen ve silinen görünüm. `python3 scripts/fixtures/join_source_cases.py --check`; Rust'ta
  `KENTOS_CASES_ONLY=cad.joins cargo test -p kentos-native-application --test all every_command_case` (ADR 0527'nin durumlarıyla
  76 durum). KentOS kodu yok.
- KCAD: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Masaüstü: `cargo test -p kentos-desktop --bin kentos-cad joins_` (pencereden kaynaklı görünüm, üç malikli parselin satırları,
  bir kez sayılan toplam, yazılmayan kaynak hücresi, sayfa); resimler `features::joins_tests::screens`
  (`.run/shots/birlesik-kaynak-*`).
- Süreler (`join_source_timing`, release, bu makinede; 100 000 parsel, 300 000 tapu kaydı):

  | Görünüm | Satır | Süre |
  |---|---:|---:|
  | kaynak tablosu (GeoPackage), toplamasız | 300 000 | 980 ms |
  | kaynak tablosu, üç toplama sütunu | 300 000 | 1 236 ms |
  | katman, toplamasız | 300 000 | 522 ms |
  | katman, üç toplama sütunu | 300 000 | 739 ms |

  İlk ölçüm her yolda 11 s'ydi: ADR 0527'nin okuyuşu birleşen kaydın daha önce görülüp görülmediğini listede arıyordu (satır
  sayısının karesi). Kayıt başına sayaç dizisiyle 10–20 kat hızlandı; ADR 0527'nin görünümleri de bundan yararlanır. Kaynağın
  okunması (300 000 kaydın nesneye çevrilmesi) farkın geri kalanıdır. Görünüm masaüstünde arayüzün iş parçacığında, çizim
  değişince bir kez okunur; yüz binlerce satırlık görünümün arka planda okunması sonraya kalır.
