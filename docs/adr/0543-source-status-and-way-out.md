# ADR 0543: Kaynak durumu ve durumun istediği iş

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-40'ın ilk dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0503 (bağlı katmanlar);
  - 0505 (PostGIS);
  - 0506 (taşınabilir proje ve Yeniden bağla);
  - 0510 (tablonun bir alanı);
  - 0504 (kaynağın sistemi);
  - 0538 (kaynakla uzlaştırma).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - aynı bilginin Öznitelik tablosunun başlığında ve bilgi kartında;
  - katman süzgecinin geçen sayısı ve bulut taslağının durumu aynı satırda;
  - eksik sayfa ve bağlantı kopması gibi sağlayıcı hatalarının kendi önerisi;
  - iki referans kullanıcı grubuyla (CBS uzmanı, form kullanan belediye personeli) kullanılabilirlik kabulü: tamamlama oranı,
    süre, yanlış hedef ve kurtarma ihtiyacı. Bu, geliştiricinin yapabileceği bir iş değildir; bu ADR onu tamamlamaz.

## Bağlam

Bağlı katmanın durumu dağınıktı. `cad.sources.describe` dosyanın yerinde olup olmadığını, iki yanın değişip değişmediğini ve
farkların sayısını veriyordu. Masaüstünde Kaynak durumu bunları iletiler olarak yazıyordu. Kullanıcı bundan sonra ne yapacağını
kendisi çıkarmalıydı: Kaynağa yaz mı, Kaynaktan oku mu, Uzlaştır mı, Yeniden bağla mı? Yanlış seçim ya reddediliyor (yazma
`source_changed`, okuma `pending_changes`) ya da başka bir pencereye götürüyordu. Ayrıca şunlar görünmüyordu:
- katmanın tablonun ne kadarını tuttuğu (bütün tablo mu, bir alanı mı; kaç satır);
- tablonun salt okunur olup olmadığı;
- tablonun projeninkinden başka bir sistemde olup olmadığı.

Plan bu bilgilerin var olan yüzeylerde görünmesini ve hataların aynı akıştan çözülmesini istiyor.

## Karar

### 1. Durumun yeni alanları

`cad.sources.describe`'ın her bağ durumu (`SourceState`) üç alan alır:
- `objects`: katmandaki nesneler, her türden;
- `rows`: çalışma kopyasının kapsadığı ve katmanın tutabildiği tablo satırları (alan sınırlı kopyada alanın içindekiler), tablo
  okunamadıysa yok;
- `total`: bütün tablonun satırları, tablo okunamadıysa yok;
- `next`: durumun istediği iş.

### 2. Kural

`SourceNext::of` (sözleşmede, bağımsız başvuruda aynısı):
1. dosya yerinde değil, veritabanı cevap vermedi ya da tablo okunamadı: `relink`;
2. fark yok: `none` (öznitelik alanı olmayan değişiklik de fark değildir);
3. iki yan da değişti: `reconcile`;
4. yalnız katman değişti: `write`;
5. öbür durumlar (yalnız dosya değişti ya da iki yanın işareti olmadan fark var): `read`.

Kural kullanıcının işini götürecek bir şey önermez. Katman değiştiyse okuma önerilmez; iki yan da değiştiyse yalnız uzlaştırma
önerilir.

### 3. Kaynak durumu penceresi

Masaüstünde `source.status` (Katmanlar'ın Kaynak ▸ Kaynak durumu, şerit) bir pencere açar. Her satır bir bağlı katmandır:
- Katman;
- Kaynak: GeoPackage dosyası › tablo ya da PostGIS sunucusu › tablo, salt okunursa söylenir;
- Durum: Aynı, Katmanda yazılmamış, Dosyada yeni, İki yan da değişmiş, Yerinde yok, Tablo okunamadı;
- Kapsam: nesneler ve satırlar (alan sınırlı kopyada “alan 120 / 1 000 000”);
- Sistem: EPSG kodu, başka sistemdeyse “→ proje”;
- durumun istediği işin düğmesi: Kaynağa yaz, Kaynaktan oku, Uzlaştır…, Yeniden bağla… (veritabanında Adresi denetle). Salt
  okunur tabloda yazma düğmesi kapalıdır.

Düğme Kaynak ▸'nın olaylarıyla aynı komutları çalıştırır (`cad.sources.sync`, `.edit`, `.reconcile`); durumlar ardından yeniden
okunur. Uzlaştır… kendi penceresini bunun yerine açar. Satıra basmak katmanı Katmanlar'da seçer ve durumun tamamını tablonun
altına yazar.

### 4. Python ve MCP

Alanlar komutun çıktısındadır; Python (`kentos.cad.sources.describe`) ve MCP aynı öneriyi okur. Arayüzün kuralı ayrıca yazılmaz.

## Doğrulama

- `cad.sources.describe`'ın 30 ortak durumu; bağımsız başvuru (`bound_sources_cases.py`, OGR ve sqlite3) `objects`, `rows`,
  `total` ve `next`'i kendi kuralıyla hesaplar.
- `cargo test -p kentos-desktop the_source_status_says_each_layers_way_out_and_takes_it`: dört bağlı katmanlı sahnede
  `reconcile`, `write`, `none`, `relink`; Kaynağa yaz dosyayı yazar ve satır `none` olur; Uzlaştır… kendi penceresini açar.
- Resimler: `cargo test -p kentos-desktop source_status_screens -- --ignored --nocapture`, `.run/shots/gpkg-kaynak-durumu-*`.
