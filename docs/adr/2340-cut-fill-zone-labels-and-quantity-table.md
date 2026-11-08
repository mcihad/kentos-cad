# 2340 — Kazı ve dolgu bölgelerinin yazısı ve miktar tablosu

Tarih: 2026-11-08. plan/map-cad.md MAP-15, üçüncü tur. Durum: taslak. Plan satırı: “Tesviye … hacim, kesit ve rapor
birlikte güncellensin.”

İlgili: [ADR 1840](1840-cut-and-fill-zones.md), kazı ve dolgu bölgeleri. Bu ADR onun “Sonra”sındaki “bölgelerin yazısı ve
bir miktar tablosu” maddesini kapatır; derinlik yazısı sonraya kalır.

## Dayanak

- **Autodesk Civil 3D, Volumes Dashboard**
  ([Insert Cut/Fill Summary](https://help.autodesk.com/cloudhelp/2017/ENU/Civil3D-UserGuide/files/GUID-C2939F9C-F81A-4DC9-AC08-5305BAECA4A8.htm),
  8 Kasım 2026'da okundu):
  - “Insert Cut/Fill Summary” çizime bir tablo koyar; yeri kullanıcı seçer;
  - özet bir blok olarak oluşur, hacim bilgisi değişince kendiliğinden güncellenmez.
- **GEOS, InteriorPointArea:** shapely'nin `representative_point`'i, alanın kutusunun ortasından köşelere değmeyen tarama
  çizgisinde içerideki en geniş aralığın ortasıdır. KentOS'un çekirdeğindeki `interior_point` aynı kuraldır (yaylar
  dahil; ADR 0218 §2).
- **KentOS bugün** (ADR 1840):
  - bölgelerin türü, alanı ve hacmi öznitelikte ve cevaptaydı;
  - çizimde yazı ve tablo yoktu: “Yazı (etiket) bu dilimde yazılmaz”.

## Karar

1. **`labels`** (`road.terrains.cutfill`, isteğe bağlı, ilk değer kapalı): her bölgenin içine türü ve hacmi yazılır, ör.
   “Kazı 14.27 m³”.
   - Yer: bölgenin halkalarının çekirdekteki iç noktası (GEOS'un kuralı). Çukurlu ya da delikli bölgede de içeridedir.
   - Yazı yüksekliği projenin yazı yüksekliği × ölçektir (ADR 0205; ilk değer 2,5 mm). Yazı ortadan hizalı ve
     maskelidir.
   - Cevapta `labels`: bölgelerin sırasıyla yer ve sözcükler.
2. **`table`** (isteğe bağlı, ilk değer kapalı): bölgelerin yanına bir miktar tablosu konur.
   - Başlık: Bölge, Tür, bantlıysa Alt (m) ve Üst (m), Alan (m²), Hacim (m³). Bölgeler sırasıyla gelir; sonra Kazı,
     Dolgu ve Net (dolgu − kazı) toplamları.
   - Sayılar iki ondalıklıdır. Yazıdır; ölçünün kendisi cevapta ve öznitelikte kesin değeriyle kalır.
   - Tablonun sol üst köşesi, bölgelerin kutusunun sağına iki tablo yüksekliği, üstüyle aynı hizadadır. Yükseklik
     projenin tablo yüksekliği × ölçektir. Sütunlar Tablo aracının sığdırmasıyla genişler (`take_rows`); sayı sütunları
     sağa hizalıdır.
   - Cevapta `table`: hücreler, başlık önce.
3. **Güncellik:** yazı ve tablo bölgelerle aynı geri alma adımındadır (“Kazı ve dolgu bölgeleri”) ve tutulan çizimin
   parçasıdır. Yüzey değişince `road.project.rebuild` aynı girdiyle hepsini yeniden çizer. Civil 3D'nin özeti blok olarak
   güncellenmeden kalır.
4. **Uyum:** girdi alanları isteğe bağlıdır. Tutulan çizimin girdisi `.kcad`'de açık bir eşlemdir; şema değişmez.
   Girdinin sözleşmesi bilinmeyen alanı reddetmez, eski bir KentOS bu alanları yok sayar.
5. **Masaüstü** (Yüzey analizi › Fark, Çizime yaz): **Bölgelere** satırında **Yazı (tür, hacim)** ve **Miktar tablosu**
   kutuları. Kazı ve dolgu bölgelerini çiz onları birlikte yazar.
6. **Kapsam işareti:** ADR 1840'taki gibi **Harita + Yol**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_terrain_cutfill_cases.py` (uv ile `shapely==2.0.6`), KentOS kodu yok.
  - Yazının yeri shapely'nin `representative_point`'idir; tablo hücreleri bölgelerin sayılarından yazılır.
  - 38 durum (33'ten), eski durumların cevabı aynı. Yeni durumlar:
    - Tümsek düzün üstünde yazılarla: delikli kazı karesinde ve dolgu dörtgeninde;
    - miktar tablosu;
    - bantlı miktar tablosu (11 bölge, alt ve üst sınırlar);
    - yazı ve tablo birlikte, plan yazmaz;
    - proje koordinatlarında (x 512 000, y 4 512 000) yazılar.
  - Rust ve Python geçer. KentOS'un iç noktası GEOS'unkiyle 10⁻⁹ içinde aynıdır.
- **MCP** (`crates/native/mcp/tests/cutfill_labels.rs`): durumun çizimi `.kcad` olarak açılır. Planın yazıları ve
  tablosu üç durumunkidir.
- **Masaüstü** (`surface_analysis_window_tests.rs`, `fark_writes_the_zones_labels_and_quantity_table`):
  - iki kutu işaretlenince bölgelerle birlikte iki yazı ve tablo yazılır;
  - yazıların sözcükleri ve yerleri, tablonun hücreleri durumunkidir;
  - tek geri alma hepsini kaldırır.

  Çapraz masaüstü denetimleri 141 geçer. Resimler `.run/gui-evidence/bolge-yazisi-2340/` (1/100 ölçekte; fikstürün
  1/1000'i 10 m'lik bölgelere 2,5 m'lik yazı verirdi).

## Sonra

- Bölgenin en derin yeri ve derinliği (çekirdeğin parça köşelerinden).
- Malzeme bölgesi ve stok sahası; kesit yöntemiyle hacim karşılaştırması (ADR 1840 Sonra).
- Tablonun yerini kullanıcının seçmesi.
