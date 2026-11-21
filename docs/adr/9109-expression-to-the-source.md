# ADR 9109: İfadeden kaynağın koşuluna

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-12 (ikinci tur).
- **İstek:** sahibin sözü (20 Kasım 2026): “alttaki panellerde de datalar çok yavaş filtreleniyor aranıyor … bunlar da
  doğrudan veritabanından sorgulanmalı”; 21 Kasım: “Hepsini düzelt eksikleri tamamla.” ADR 9106 §7'deki kalan
  (dışa aktarmanın süzgeci kaynağa gitmiyor) ve ADR 9101 §10'daki kalan (katmanın süzgeci koşul ağacına çevrilmiyor).
- **Sürücü sözleşmesi:** yeni `Condition::Text(field)`. **İfade dili:** `pushdown::sent` ve `Sent`; `push` aynı SQL'i
  yazar. **Komut sözleşmesi:** `RowsChecked::Source` artık “kaynağın tam cevapladığı süzgeç” de demektir. **`.kcad`,
  web:** değişmedi. **Bağımlılık:** yok.

## §1 Bağlam ve araştırma

**Sorun:** alttaki tablonun İfadeyle seç süzgeci, kaynağın tuttuğu her satırı (anahtar ve süzgecin alanları) okuyup
süzgeci burada soruyordu. Bir milyon parsellik PostGIS tablosunda `kullanim = 'Konut'` sayımı 7,55 sn sürdü (bu işten
önce). Dışa aktarmanın süzgeci de kaynağa gitmiyordu: süzgeç ne kadar az satır bıraksa da tablonun hepsi okunuyordu.
Veri katmanının kendi süzgeci (ADR 0211) de görünümün bütün satırları okunduktan sonra uygulanıyordu.

**Masaüstü CBS** (QGIS; 2.10 değişiklik notu, `issues.qgis.org` #14030, qgis-developer listesi Eylül 2025):

- “Execute expressions on server-side if possible” ayarı var. Yalnız desteklenen ifadeler veritabanına gönderilir;
  desteklenmeyen işleç ya da işlev içeren ifade yerelde sorulur.
- 2025'teki tartışmanın konusu iki şey: ifadelerin olabildiğince çoğunu sağlayıcının SQL'ine çevirmek ve kullanıcıya
  ifadenin nerede sorulduğunu göstermek.

**KentOS'un kendi kuralı:** ADR 0581 §3 (`kentos_expression::pushdown`). Sunucu ve formlar bu kuralla zaten SQL
gönderiyordu:

- Kaynağa giden koşul, ifadenin tuttuğu her satırı tutar.
- Tam değilse, kaynağın verdiği satırlar ifadeyle yeniden sorulur.
- SQLite bir sayı sütununda metin de tutabilir. Dil o metni metin olarak sıralar (`'1abc' < 5` doğrudur), SQLite ise
  metni her sayıdan büyük sayar. Bu yüzden SQLite'a giden sayı koşulu sütunun metinlerini de geçirir.

**Türkiye:** mevzuat bir sorgunun nerede sorulduğu hakkında bir şey söylemez. Bu bir uygulama kararıdır.
Uluslararası karşılığı ISO 19143 (OGC Filter Encoding 2.0): süzgeç, verinin bulunduğu servise gönderilir.

## §2 Karar: kural bir yerde, iki yazılışı var

- `pushdown::sent(expr, columns)`, ifadenin gönderilebilir parçasını `Sent` ağacı olarak verir. Ağacın düğümleri:
  ve, veya, sayı sırası, sayı aralığı, metin eşitliği, içinde, boş. Yanında tam olup olmadığını da söyler (sayı
  sütununun yalnız sayı tuttuğu yerde).
- `push` bu ağaçtan SQL yazar; SQL'i değişmedi (dört test aynı metni bekler).
- Uygulama aynı ağacı veri sürücülerinin koşul ağacına yazar (`data_condition::pushed`). Kurallar iki yerde
  yazılmamıştır.
- **Tipli kaynak** (PostGIS): sütun yalnız kendi türünü tutar. Sayı sırası ve aralığı tamdır.
- **SQLite kaynağı** (GeoPackage): sayı koşulu `Condition::Text(sütun)` ile birlikte gider (`typeof(...) = 'text'`) ve
  tam değildir. PostGIS'te tipli bir sayı sütunu için `Text` hiçbir satır tutmaz.
- **Gönderilmeyenler** yerelde sorulur: ölçüler (`$alan`), işlevler, tarihler, metin sırası ve sayı sütununda
  eşitsizlik (`<>`). Böyle bir parça `ve` içindeyse geri kalanı yine gider, ama sonuç tam olmaz. `veya` içindeyse
  koşulun hiçbiri gitmez.
- Sayı eşitliği bir aralıktır (dilin toleransından geniş): asla tam değildir. Metin eşitliği her yerde tamdır.

## §3 Karar: nerede kullanılır

- **Alttaki tablo** (`cad.layers.rows`):
  - Gönderilen parça bütün süzgeçse ve tamsa, kaynak süzgeci tümüyle cevaplar: sayı, sayfa ve anahtarlar SQL'den
    gelir. Yanıt `checked: source` der; durum satırı “süzgeç ve sıra kaynakta” yazar.
  - Öbür durumlarda yalnız gönderilen parçanın tuttuğu satırlar okunur ve süzgeç burada sorulur (`checked: here`).
- **Dışa aktarma:** satırlar ve ilerleme çubuğunun toplamı gönderilen parçayla okunur. Süzgeç yine burada da sorulur.
- **Canlı çizim:** katmanın süzgecinin gönderilen parçası görünümün isteğine eklenir. Çizim, süzgeci çizdiği satırlara
  yine sorar.

## §4 Kanıt

**Ölçüm:** bir milyon parsel, yalıtılmış PostGIS (`kentos-portal-db`), Python SDK ile `cad.layers.rows`, sayım ve ilk
50 satır, üç denemenin en iyisi.

| Süzgeç | Kaynakta | Burada (gönderilemeyen eşdeğeri) |
|---|---|---|
| `kullanim = 'Konut'` (250 000 satır) | 28 ms | 7 550 ms (`kullanim \|\| '' = 'Konut'`) |
| `alan > 600 ve kullanim = 'Park'` (63 357 satır) | 26 ms | 9 008 ms |

**Gerçek pencere** (bir milyon parsel, Tablo, süzgeç `alan > 600 ve kullanim = 'Park'`):

- Önce: Enter'dan 1,5 sn sonra tablo “Kaynaktan okunuyor…” diyordu. Sonunda “63.357 satır · süzgeç burada soruldu”.
- Sonra: 1,5 sn'de satırlar ve “63.357 satır · süzgeç ve sıra kaynakta” görünüyor.
- Görüntüler `.run/gui-evidence/veri/vy12/{once,sonra}`.

**Sınamalar:**

- `kentos-expression`'ın pushdown testleri aynı SQL'le geçti.
- `data_condition`'ın dört testi: metin eşitliği her yerde tam; SQLite'ta sayı sırası metinleri de geçirir; tipli
  kaynakta tam; ölçü `ve`'yi daraltır, `veya`'yı düşürür; boş.
- `cad.layers.rows` 32 durum, beşi yeni: içinde ve boş kaynakta; GeoPackage'ta aralık ve sayılı veya burada.
- Bütün `cad.layers*` durumları Rust'ta ve Python'da geçti. `kentos-forms`, `kentos-data` geçti; sunucu derlendi.
- Masaüstünün veri testleri geçti (177). Dışa aktarma testi artık süzgecin tuttuğu 20 satırı sayıp okuyor,
  tablonun 40 satırını değil.

## §5 Kalan

- Metin sırası (`<`, `>` metin üstünde) gönderilmez: dilin Türk alfabesi sırası ile kaynağınki her yerde aynı değil.
- İşlevler (`benzer`, büyük/küçük harf, tarih işlevleri) gönderilmez.
- Kurallı sınıfların sayımı VY-13'te bu yolla gider.

## Uygulama

- İfade dili: `crates/shared/expression/src/pushdown.rs` (`Sent`, `sent`, SQL yazıcısı).
- Sürücü sözleşmesi: `crates/native/data/src/condition.rs` (`Condition::Text`); `data-gpkg/src/sql.rs`
  (`typeof`), `data-postgis/src/sql.rs`.
- Uygulama: `crates/native/application/src/gis/data_condition.rs`, `layers_rows.rs`, `layer_export.rs`. Masaüstü:
  `apps/desktop/src/gis/data_view.rs`. Komut: `RowsChecked` belgesi.
- Ortak durumlar: `scripts/fixtures/layer_rows_command_cases.py`.
