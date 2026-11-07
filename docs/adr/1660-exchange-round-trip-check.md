# 1660 — Dışa aktarma kaybı denetimi: değişim biçiminden gidiş-dönüş, sayısal sapma ve anlamsal kayıp ayrı (`file.exchange.check`)

Tarih: 2026-11-07. plan/map-cad.md MAP-17, ilk dilim. Kabul satırı: “KentOS→dış biçim→KentOS karşılaştırması yap.
Sayısal hata ve semantik kayıplar ayrı raporlansın”. Durum: taslak. İlgili ADR'ler:
- [ADR 0179](0179-data-compare.md): veri karşılaştırma kuralı;
- [ADR 0253](0253-command-protocol.md) §10: dosya yazan komutlar;
- [ADR 0627](0627-geopackage-tiles-coverages-links-and-profiles.md): GeoPackage dışa aktarma.

**Ad (7 Kasım 2026):** önce “Devir denetimi” adıyla yazıldı (plandaki “veri devri”). Sahibi adın ne anlama geldiğini
sordu; “devir” Türkçede dönme ve dönem anlamına da gelir. Pencerenin ve komutun başlığı **Dışa aktarma kaybı
denetimi** oldu; komutun kimliği (`file.exchange.check`) ve davranışı değişmedi.

## Dayanak

- **Plan:** Dönüşen, korunan, yaklaşıklaştırılan ve kaybolan bilgi her aktarımda açıklanmalı. Bir okuyucunun bulunması
  aynı biçimde kusursuz yazıcı ya da gidiş-dönüş garantisi sayılmamalı.
- **KentOS bugün:**
  - Her dışa aktarma kendi kaybını söyler: yazılmayanlar türlerine göre, kirişlenen eğriler, M'nin düşmesi (GML,
    GeoParquet).
  - Bu sözler yazıcının kendi iddiasıdır. Dosyanın geri okununca ne verdiğini ölçen bir şey yoktu.
- **Bağımsız okuyucu:** GDAL'ın `ogrinfo`'su dosyada neyin bulunduğunu KentOS'tan bağımsız söyler. KentOS'un bağımsız
  denetimi onu kullanır (aşağıda).

## Karar

1. **`file.exchange.check` v1** (headless dosya komutu, salt okunur):
   - **Girdi:** `format` (gpkg, parquet, fgb, gml, shp, dxf), `layers` (boş: yazıcının varsayılanı), `tolerance`
     (metre, varsayılan 0,001).
   - **Gidiş:** çizim, biçimin `file.export.*`'i ile çizimin bir kopyasından sistemin geçici klasöründeki kendi
     klasörüne yazılır.
   - **Dönüş:** dosya, biçimin `file.import.*`'i ile çizimin boş bir kopyasına (ayarlar ve katmanlar, nesnesiz) okunur.
   - Klasör her sonuçta silinir. Açık çizim değişmez.
2. **Eşleme:**
   - Önce kalıcı kimlikle.
   - Sonra sınıfa (nokta, çizgi, alan; yazı, blok kendi türüyle) göre, kutu merkezi en yakın özgünle.
   - En son kalan çizgi ve alanlar sınıfsız eşlenir: Shapefile daireyi çizgi olarak verir, DXF deliği ayrı çizgi
     yapar.
   - Sıra ve katman tutulmaz, çünkü biçimler onları korumaz:
     - FlatGeobuf Hilbert sırasıyla yazar;
     - Shapefile katmanı geometri türüne göre böler;
     - GML türleri XML adına çevirir.
3. **Sayısal sonuç (`numeric`):**
   - Geri okunan her köşenin özgün geometriye kesin uzaklığı: çizgi, çoklu çizginin ve alanın yaylı kenarı, daire, yay.
     Elips, eğri ve NURBS 1024 parçalık çevreyle ölçülür.
   - Köşe sayısı aynıysa kot ve ölçü farkı.
   - Uzunluk ve alan farkı.
   - En büyükleri, en büyük kaymanın nesnesi ve toleransı aşanların sayısı.
4. **Anlamsal sonuç (`losses`):**
   - `notWritten` (türüne göre);
   - `kindChanged` (`circle → polygon`);
   - `chorded`;
   - `zLost`, `zAdded`, `mLost`;
   - `attrLost`, `attrChanged`, `attrAdded` (özniteliğin adıyla; `12.50` ile `12.5` aynı sayı sayılır);
   - `styleLost` (renk, kalınlık, sembol);
   - `identityLost`;
   - `extra`.

   Her biri sayısı ve özgün nesnelerin kimlikleriyle (en çok 100). Yazıcının ve okuyucunun uyarıları ayrıca verilir.
5. **Retler:** `invalid_tolerance`, `layer_not_found`, yazıcının kendi reddi (`export.<alan>`), `exchange_unreadable`
   (yeni: KentOS'un okuyucusu KentOS'un dosyasını reddetti), `file_unwritable`.
6. **Masaüstü, Dışa aktarma kaybı denetimi penceresi** (Dosya › Alışveriş › Dışa aktar › Denetim; Harita, CBS, Yol, Mimari):
   - Biçim ve Tolerans; Denetle.
   - Özet: eşlenen ve yazılmayanlar; Sayısal satırı; yazıcının sözleri.
   - Anlamsal kayıplar tablosu: türler ve ayrıntılar Türkçe.
   - Kayıplıları seç.
7. **Sınır:** sonuç KentOS'un yazıcısıyla okuyucusunun birlikte yaptığıdır. Başka programın okuduğu onun kendisinindir
   ve bağımsız denetimle ayrıca sınanır.

## Bulgu ve düzeltme

- **GeoParquet'te küçük dairelerin kirişleri** 0,1 mm denmesine karşın çemberden 0,41 mm sapıyordu.
  - Örnek: TM36 koordinatlarında 0,6 m'lik baca.
  - Neden: `parquet_write::circle` üç noktadan geçen çemberin merkezini harita koordinatlarının karesiyle (≈10¹³)
    hesaplıyordu; sayısal iptal oluşuyordu (CLAUDE.md §23: büyük koordinatta küçük ayrıntı).
  - Düzeltme: merkez ilk noktaya göreli hesaplanır.
  - Sınama: `a_small_arc_at_map_coordinates_keeps_its_corners_on_the_circle`. Gidiş-dönüşte en büyük kayma artık
    6·10⁻¹⁰ m.

## Kanıt

- **Çizim** (`scripts/fixtures/exchange_drawing.py` → `fixtures/exchange/v1/devir.json`), gerçek koordinatlı (EPSG:5256):
  - Z'li noktalar;
  - Z ve M'li güzergâh;
  - yaylı yol kenarı;
  - baca dairesi;
  - delikli parsel;
  - yazı;
  - metin ve sayı öznitelikleri;
  - bir nesnenin kendi rengi.
- **Bağımsız denetim** (`python/tests/test_exchange_check.py`):
  - Her biçimin dosyası KentOS'la yazılır ve GDAL'ın `ogrinfo`'su ile (DXF: ezdxf) okunur; KentOS kodu kullanılmaz.
  - Komutun raporu dosyanın içeriğiyle şu konularda tutar: yazılmayanlar, kirişe dönen eğriler, Z ve M kaybı,
    öznitelik kaybı ve en büyük köşe kayması (10⁻⁶ m içinde).
  - DXF'te: delik ayrı kapalı çizgidir, yaylar bulge olarak kalır, M yoktur.
  - Altı biçim de geçer.
- **Rust** (`exchange_check::tests`):
  - GeoPackage kimliği, yayı, Z'yi ve M'yi korur.
  - DXF M'yi ve kimliği kaybeder, deliği ayırır.
  - GeoParquet kirişler; kirişler çemberin üstündedir.
  - Retler sınanır; açık çizim değişmez.
- **Masaüstü** (`exchange_check_window_tests`): GeoPackage ve DXF gidiş-dönüşü, sayı olmayan tolerans, Kayıplıları
  seç, her modülde.
  Resimler `.run/gui-evidence/aktarma-kaybi-1660/` altındadır.
- **Biçimlerin bugünkü tablosu** (bu çizimle):

  | Biçim | Yazılmayan | Tür | Kiriş | M | Kimlik | Ek öznitelik |
  |---|---|---|---|---|---|---|
  | GeoPackage | yazı | daire → alan | — | korunur | korunur | — |
  | GeoParquet | yazı | daire → alan | daire, yaylı yol | kaybolur | korunur | `katman` |
  | FlatGeobuf | yazı | daire → alan | — | korunur | kaybolur | `katman` |
  | GML | yazı | daire → alan | — | kaybolur | kaybolur | — |
  | Shapefile | yazı | daire → çoklu çizgi | daire, yaylı yol | korunur | kaybolur | — |
  | DXF | — | alan → çoklu çizgi (delik ayrı) | — | kaybolur | kaybolur | — |

## Sonra

- Blok, ilişki ve tablo nesnelerinin aynı denetimle sınanması.
- Teslim paketinin (çizim, bağlı kaynaklar, profil/ızgara bilgisi, kayıp raporu) bu raporu taşıması.
- Kaynak–türev çatışmasının geri almada denetimi.
- LandXML ve IFC alt modelleri.
- Kurum profillerine göre biçim önerisi.
