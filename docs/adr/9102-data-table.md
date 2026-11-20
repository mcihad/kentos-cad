# ADR 9102: Veri katmanının tablosu kaynakta — Türkçe sıra, satır komutu, sayfalı tablo

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-04.
- **İstek:** sahibin sözü (20 Kasım 2026): “alttaki panellerde de datalar çok yavaş filtreleniyor aranıyor aynı
  sebepten olduğunu düşünüyorum bunlar da doğrudan veritabanından sorgulanmalı.”
- **Önceki kararla ilişkisi:** ADR 0591'in “Kaynağın tümü”sü bağlı katmanlar için durur (`cad.sources.query`,
  imleçle sonraki sayfa). Veri katmanının tablosu bu ADR'nin yolundan gelir: satırlar
  [ADR 9100](9100-data-drivers.md)'ün sürücülerinden, [ADR 9101](9101-data-layers.md)'in veri katmanı için.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** değişmez. **Komut:** `cad.layers.rows` v1 (§4).
- **Bağımlılık:** yeni dış kitaplık yok. Uygulama crate'i `kentos-data`'yı kullanır; testleri GeoPackage sürücüsünü
  geliştirme bağımlılığı olarak alır.

## §1 Bağlam

Öznitelik tablosu bugün katmanın çizimdeki nesnelerini okur. Veri katmanının nesnesi çizimde yok, satırları
kaynakta (ADR 9101). Bir milyon satırlık tablo belleğe alınamaz: sayfa, sıra, arama, süzgeç ve sayı kaynakta
yapılmalı.

## §2 Araştırma

**Masaüstü CBS'nin tablosu** (QGIS 4.2.3; belgesi ve ekranları `.run/gui-evidence/veri/qgis/05*.png`):

- Sol alttaki seçim: Bütün nesneleri göster, Seçilenleri göster, Haritada görünenleri göster, Kısıtı tutmayanlar,
  Düzenlenen ve yeni nesneler, Alan süzgeci, Gelişmiş süzgeç (ifade), Kayıtlı süzgeçler.
- Başlığa basmak o sütunla sıralar; birden çok sütun ifadeyle (`concat(...)`) sıralanır. Sıralama kaynağın sırasını
  değiştirmez.
- Seçenekler › Veri kaynakları: açılışta hangi süzgeç, varsayılan görünüm (tablo ya da form), satır önbelleği
  (“son okunan N satır saklanır”). Büyük katman için öğüt: açarken haritada görünenlerle sınırlamak.
- Tablo açılırken nesnelerin hepsi istenir ve önbelleğe alınır; büyük tabloda yavaşlığın kaynağı budur.

**Türkçe sıralama:**

- PostgreSQL 18'de ICU'nun Türkçe harmanlaması `tr-x-icu` vardır. Yalıtılmış sunucuda ölçüldü: “ağaç Ağaç cami
  Ceylan Çınar ırmak ilçe İnci Okul Öğretmen Su Şehir Uzun Ünal Zafer”.
- SQLite yalnız bayt sırasını bilir; “Ç” “Z”nin ardına düşer.

**Türkiye mevzuatı:**

- 1353 sayılı Türk Harflerinin Kabul ve Tatbiki Hakkında Kanun (RG 3.11.1928/1030) Türk alfabesini koyar. Sıranın
  dayanağı budur: `a b c ç d e f g ğ h ı i j k l m n o ö p r s ş t u ü v y z`.
- Kurumların CBS'deki öznitelik tablosunun nasıl sayfalanacağı ya da aranacağı hakkında hüküm yoktur. 6698 sayılı
  KVKK md. 12'nin tedbir yükümlülüğü değişmez: tablo satırları kaynakta okur, dosyaya yazmaz.

## §3 Karar: metin Türk alfabesinin sırasıyla

`kentos_data::turkish` tek kuraldır:

- harfler Türk alfabesinin sırasıyla; `q`, `w`, `x` Latin alfabesindeki yerlerinde;
- rakamlar harflerden önce;
- şapkalı ünlü düz olanıyla birlikte (`â` `a` ile);
- küçük harf büyüğünden önce (`ağaç`, `Ağaç`);
- başka bir karakter harflerden sonra, koduyla.

Bu kural metin sütununun her sıralamasında uygulanır, kaynağın harmanlaması ne olursa olsun:

- **PostGIS:** veritabanında `tr-x-icu` varsa `COLLATE "tr-x-icu"` kullanılır. Yoksa her harf yerinin baytına
  çevrilir (`translate`) ve `COLLATE "C"` ile bayt bayt karşılaştırılır.
- **GeoPackage:** SQLite'a kaydedilen `kentos_order(metin)` işlevi aynı kuralla bir sıralama anahtarı üretir.
  rusqlite'a yeni özellik açmak gerekmedi.
- **Uygunluk takımı:** mahalleye göre sıralama denetimi eklendi (“Esentepe, İnönü, Kılavuz, Mevlana, Yenişehir”).
  İki sürücü de bu denetimden geçer.

## §4 Karar: `cad.layers.rows` v1

Bir veri katmanının satırlarını kaynağından okur; hiçbir şey yazmaz.

- **Girdi:**
  - `bbox`: Görünümdekiler (projenin sisteminde; tablonun sistemine çevrilir);
  - `keys`: Seçililer;
  - `search`: Ara; alanlardan birinde, Türkçe katlanarak (`İ`/`i`, `I`/`ı` ayrı çiftler);
  - `filter`: İfade süzgeci;
  - `order`: alan alan sıra, sonra anahtar; boş değerler iki yönde de sonda;
  - `offset`, `limit` (0–1 000; verilmezse 100), `count`, `allKeys`, `fields`, `geometry` (projenin sisteminde WKT).
- **Süzgeç yoksa** kutu, anahtarlar, arama, sıra, sayfa ve sayı kaynakta yapılır (`checked: source`).
- **Süzgeç varsa** ifade dilinin kendi anlamıyla sorulur. Kaynağın tuttuğu her satır okunur: anahtarı, süzgecin andığı
  alanlar, süzgeç ölçüyorsa (`$alan` …) geometrisi. Süzgeç sırayla sorulur; sayfa ve sayı tuttuğu satırlardandır
  (`checked: here`).
  - Böylece dilin boş değer, sayı penceresi ve büyük–küçük harf kuralları kaynağın kurallarına bırakılmaz.
  - Başka nesnelere bakan süzgeç (`world` çağrıları) reddedilir.
- **Ret sırası:** `no_layer`, `invalid_count`, `invalid_extent`, `layer_not_found`, `not_a_data_layer`,
  `field_not_found`, `invalid_expression`, `source_unreachable`; durdurulan iş `cancelled`.
- **Ortak durumlar:** `scripts/fixtures/layer_rows_command_cases.py` GeoPackage'ı sqlite3 ile kendisi yazar
  (`fixtures/data/v1/satirlar.gpkg`). Beklentileri sözleşmenin kurallarıyla hesaplar, KentOS kodu kullanmaz. 28 durum
  kapsar: Türkçe sıra, katlanan arama (“isik” Işıklar'ı bulmaz), sayılarda arama, anahtarlar, kutu, WKT, sayı,
  bütün anahtarlar, üç süzgeç (biri geometri ölçer) ve dokuz ret.
- **Python ve MCP:** komut `kentos.cad.layers.rows`'tur, MCP katalogdan çağırır. Bilgi aracının Python ve MCP'deki
  karşılığı da budur (ADR 9101 §10).

## §5 Karar: masaüstünde tablo (`apps/desktop/src/gis/features/data.rs`)

Katman ▾'da veri katmanı seçilince Öznitelik tablosu satırları kaynaktan okur. Katman listede “(veri)” diye görünür.

- **Sayfalar:** satırlar 200'lük sayfalar hâlinde okunur. Sanal tablo görünen satır aralığını bildirir; bunun için
  `kentos_ui`'nin `VirtualList::on_range` ve `Table::on_range` olayları eklendi. Yalnız görünen sayfalar istenir.
  En çok 40 sayfa tutulur; görünümden en uzaktakiler bırakılır.
- **Okuma** `layers_rows::read` ile arayüzün dışında yapılır. Komutun aynı denetimlerini katmanın ve ayarların
  kopyasıyla çalıştırır; çizim kopyalanmaz.
- **Başlığa basmak** o alanla sıralar; ikinci basış ters, üçüncüsü anahtar sırası.
- **Ara ve İfade süzgeci Enter'la sorulur.** Bir milyon satırı her harfte aramak yerine. Süzgeçli sorunun ilk
  sayfası bütün anahtarları getirir; sonraki sayfalar anahtarla okunur, tarama bir kez yapılır.
- **Göster:**
  - Görünümdekiler görünümün kutusunu okur; görünüm değişince yeniden okunur.
  - Seçililer, tabloda basılan satırlardır.
- **İptal:** yeni soru, eskisinin süren okumasını durdurur (taramanın bir sonraki parçasında).
- **Satıra basmak** satırı seçer; Ctrl ile seçimi tersine çevirir.
- **Salt okunur:** tablo bu maddede yalnız okur. Hücre düzenlemesi VY-05'in oturumunda.

Tablonun satır önbelleği, katman seçilmemişken etkin katmanın değişmesini görmüyordu (önceden de böyleydi). Anahtara
tablonun çözdüğü katman eklendi.

## §6 Planda olmayan kararlarım

1. **Metin her zaman Türk alfabesiyle sıralanır**; kaynağın kendi harmanlaması kullanılmaz (`tr-x-icu` dışında).
2. **Süzgeç her zaman burada sorulur.** Satır taranır, ifade SQL'e çevrilmez. Doğruluk önce geldi; SQL'e çeviri
   ileride tarama hızlandırması olarak gelebilir.
3. **Arama bütün alanlarda yapılır**; metin olmayan alanlar da metin olarak okunur.
4. **Sayfa en çok 1 000 satırdır.** Masaüstü 200'lük sayfa okur, en çok 40 sayfa tutar.
5. **Ara ve süzgeç Enter'la uygulanır.** Yazarken uygulanmaz.

## §7 Kanıt

Gerçek pencerede, PostGIS'te 1 000 000 parselle denendi (`.run/gui-evidence/veri/vy04/`, `vy04b/`).

| Adım | Sonuç |
|---|---|
| Tabloyu açmak | 1.000.000 satır sayıldı; ilk sayfa geldi |
| Mahalleye göre sıralayıp ortalara kaydırmak | Satırlar anında geldi |
| Ara “Mahalle 9” | 50.000 satır; veritabanı da 50.000 sayıyor |
| İfade süzgeci “alan > 650 ve kullanim = 'Park'” | 19.986 satır; veritabanı da 19.986 sayıyor |
| Görünümdekiler (1:9 994) | 3.400 satır |
| Bir satır basılıp Seçililer | 1 satır |

## §8 Kalan

- **Haritada seçim:** tabloda seçilen satırın haritada vurgulanması, Seç aracının veri katmanında satır seçmesi ve
  Seçime yakınlaş.
- **Yazma:** düzenleme VY-05'te.

## Uygulama

- **Türkçe sıra:** `crates/native/data/src/turkish.rs` (`compare`, `sort_key`, `sort_key_translation`).
  Sürücülerde `data-gpkg/src/sql.rs` (`kentos_order`) ve `data-postgis/src/{catalog,sql}.rs` (`Shape::turkish`).
- **Komut:** `crates/shared/commands/src/gis/cad_layers_rows.rs`, `crates/native/application/src/gis/layers_rows.rs`.
- **Ortak yardımcılar:** `sources::TableSystem::moved` ve `geopackage::attribute_text` uygulama crate'ine taşındı;
  masaüstünün canlı çizimi ve Bilgi aracı da bunları kullanır.

## Doğrulama

- `cargo test -p kentos-data`: Türkçe sıra, anahtar ve çeviri denetimleri (3 yeni test).
- `cargo test -p kentos-data-gpkg --test conformance`, `KENTOS_DATA_PG=… cargo test -p kentos-data-postgis --test
  conformance`: mahalleye göre Türkçe sıra dahil geçti.
- `KENTOS_CASES_ONLY=cad.layers.rows … every_command_case_matches`: 28/28.
- `cargo test -p kentos-commands`: katalog ve birim denetimleri geçti.
- `cargo test -p kentos-desktop --lib data_layers_tests`: tablo, gösterdiği sayfayı kaynaktan okur, göstermediğini
  okumaz; sona kaydırınca son sayfayı okur; başlık Türkçe sıralar ve tersine çevirir; arama Enter'la sorulur
  (450 satırdan 225'i); eski yanıt bayattır.
