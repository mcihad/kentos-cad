# ADR 0504: Kaynağın koordinat sistemi korunarak bağlı katman ve kutu dönüşümü

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-07'nin ilk dilimi: kaynak sistemi ile projenin
  (gösterim ve işlem) sistemi ayrılır, kaynağa kendi sisteminde yazılır, dönüştürülmüş kutu sorguya hazırdır.
- **Bağlam belgesi:** ADR 0503 (bağlı katman), 0222 (içe aktarmada açık dönüşüm), 0167 ve 0168 (dönüşümler,
  datum seçimleri).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - her katmanın kendi sistemiyle çizimde anında dönüşüm: belge katmanın sistemini taşımıyor;
  - katmanın sistemine göre ters dönüşümlü tıklama ve kenet;
  - WKT2 ve kullanım alanının saklanması;
  - genel PROJ bağdaştırıcısı: kayıtta olmayan sistemler bugün reddedilir;
  - düşey sistem ve epoch (CBS-08).

## Bağlam

ADR 0503'te bağlı katman tablonun sisteminde olmak zorundaydı (`crs_mismatch`). Belediyenin GeoPackage'ı çoğu zaman
WGS 84'te ya da başka bir dilimde gelir. İçe aktarmanın çevirmesi (ADR 0222) kopyadır: tablo da çevrilmiş olur, ya
da geri yazılamaz. Plan üç sistemi ayırmak ister:

- **kaynak sistemi:** tablonun kendi sistemi, değişmez;
- **gösterim sistemi** ve **işlem/ölçüm sistemi:** bugün ikisi de projenin sistemi.

Kaynağa kendi sisteminde yazılmalı. Kutu sorgusu dönüştürülürken yalnız iki köşe yetmez.

## Karar

### 1. Bağlı katmanda iki sistem

- `cad.sources.edit`'in `link`'i `reproject` alır. Başka sistemdeki tablo yalnız onunla bağlanır; yoksa
  `crs_mismatch` döner, sessiz dönüşüm yoktur.
- Bağ tablonun EPSG'sini (`srid`) tutar. Katmanın nesneleri projenin sistemindedir.
- Her eşitleme iki yönde çevirir (`sources::Mover`): projenin datum seçimleri ve ADR 0167–0168'in yolları geçerlidir.
  - **Karşılaştırma ve yazma:** katmanın nesnesi tablonun sistemine çevrilir, satır olarak orada karşılaştırılır ve
    yazılır (`kentos_formats::reproject::move_entity`).
  - **Okuma:** tablonun satırları projenin sistemine çevrilir.
- Gidip gelen koordinat aynı sayılır. Hoşgörü tablonun biriminde: düzlemde 10⁻⁶ m, derecede 10⁻¹¹ (yaklaşık
  1 mm).
- Yaylı nesne ve hedefin ulaşmadığı köşe kesin çevrilemez:
  - bağlanırken satır katmana alınmaz;
  - karşılaştırmada o satır yok sayılır, yazma onu silmez;
  - katmana çizilen yaylı nesne yazılmaz ve söylenir (`export_losses`).
- Projenin sistemi sonradan değişirse yön yeniden bulunur. Sistemi olmayan (yerel) projede ve kayıtta olmayan
  sistemde reddedilir: `crs_mismatch`, nedeniyle.

### 2. Kutu dönüşümü

Çekirdeğin `crs::transform_bounds` işlevi bir sistemin kutusunu başka bir sistemin kutusu olarak verir:

- her kenar köşelerinin yanında `densify` iç noktada örneklenir; PROJ'un `proj_trans_bounds`'u ile aynı kural;
- hedefin ulaşmadığı noktalar düşer;
- derecedeki hedefte boylamlar [−180°, 180°)'ye katlanır. Antimeridyeni geçen kutu `min x > max x` ile sarar, PROJ
  gibi.

Bir TM diliminin tamamında köşeler kutuyu 2 km eksik verir (42,4168° yerine 42,4354°); sıklaştırma bunu giderir.
İlk kullanıcısı PostGIS sağlayıcısının görünüm kutusu sorgusu olacak (CBS-04).

### 3. Masaüstü

GeoPackage penceresinde Bağlı katman olarak aç, başka sistemdeki dosyada da açıktır. "Projenin sistemine çevir"
işaretlenmeden bağlanmaz. Açıklama, tablonun kendi sisteminde kalacağını söyler.

## Doğrulama

- **Kutu dönüşümü:** `fixtures/geodesy/v1/bounds.json`, PROJ'un kendi `transform_bounds`'undan
  (`scripts/fixtures/crs_bounds_cases.py`; KentOS kodu olmadan). On durumu var:
  - Ankara paftası;
  - bütün dilimin köşeli ve sık kutuları;
  - WGS 84, Web Mercator ve ED50 arası dönüşümler;
  - antimeridyende saran kutu.

  Çekirdek düzlemde 10⁻⁶ m, derecede 10⁻¹¹ içinde aynı kutuyu verir.
- **Ortak durumlar:** dokuz yeni durum (`bound_sources_cases.py`). Kapsadıkları:
  - reproject'siz ret;
  - WGS 84 tablosunun bağlanması;
  - gidip gelen koordinatın aynı sayılması;
  - taşınan noktanın farkı, yazma planı ve geri okunması.
- **Geçici kopyada yazma:** WGS 84 tablosu TM36 çizimine bağlanır, nokta 10 m doğuya taşınıp yazılır. Yazılan
  `kentos-synced-wgs84.gpkg`'nin noktasını PROJ TM36'ya çevirir; nokta 10 m doğuda, mikrometre içinde. Dosya GDAL'ın
  doğrulayıcısından geçer.
- **Masaüstü:** başka sistemdeki dosya yalnız çevirme onayıyla bağlanır, bağ tablonun sistemini tutar.
