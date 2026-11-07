# 1032 — Bulutun kotları geoitle projenin kotlarına

Tarih: 2026-11-07. Durum: kabul (Y97). Kapsam ROAD-NUM-03 (yolun bulut zemini), ADR 1029 §Açık ("bulutu
çevirmek"), ADR 0928 (buluttan arazi), ADR 0631 (geoit ızgaraları), ADR 0519 (projenin kotları).

## Sorun

İHA LiDAR'ı ve GNSS'li tarayıcı çoğu kez elipsoit yüksekliği verir. LAS dosyası da çoğu kez düşey sistemini
söylemez. ADR 1029'dan beri böyle bir bulutun zemini karşılaştırmaya bilinmeyen sistemle girer; dosya projeninkinden
başka bir sistem söylüyorsa hiç alınmaz. Kotları projenin sistemine çevirmenin yolu yoktu.

KentOS geoit modeliyle çevirmeyi biliyor (`cad.crs.transform`, ADR 0631), ama yol komutları onu kullanmıyordu.

## Araştırma (7 Kasım 2026)

- **BÖHHBÜY** (26.06.2018) Madde 1(1)(b): yükseklikler TUDKA99'a dayalı Helmert ortometrik yüksekliklerdir (ADR
  1031'in kaynağı). GNSS'in elipsoit yüksekliğinden ortometriğe geçiş bir jeoit modeli ister; TG-20 resmî modeldir
  (ADR 1029'un kaynakları).
- **H = h − N:** ortometrik yükseklik, elipsoit yüksekliğinden noktanın enlem ve boylamındaki jeoit ondülasyonu
  çıkarılarak bulunur. PROJ'un `vgridshift` adımı ve KentOS'un `cad.crs.transform`'u ızgarayı çift doğrusal okur
  (ADR 0631).
- **LAS 1.4:** düşey sistem WKT'nin `VERT_CS`'i ya da GeoTIFF anahtarı 4096'dır (ADR 1029); söylenmeyebilir.
- **Civil 3D:** yüzeyin düşey datumunu tutmaz; Autodesk forumunda önerilen yollar dışarıda çevirmek ya da sabit bir
  kaydırmadır (ADR 1029'un kaynağı). Sabit kaydırma TG-20'nin ondülasyonu km'de santimetrelerce değiştiği için
  doğru değildir.
- **Matematik:** her kalan nokta için N, noktanın TUREF enlem ve boylamında ızgaranın çift doğrusal değeridir.
  - Seyreltme bulutun kendi kotlarıyla yapılır: hücreler ve dik hücrenin en alçağı ile en yükseği bir kot farkına
    bakar. N'in bir hücre içindeki değişimi (≈ 2 m'de 0,1 mm'den az) o kararları değiştirmez.
  - Çevrilen kalanlar ve dışarıda bırakılanlar hatayı yine aynı sistemde ölçer.

## Karar

- **`road.terrains.cloud`'a `heights`:** bulutun kotlarının ne olduğu (projenin `heights`'ı gibi bir yükseklik
  tanımı).
  - Verilmezse eskisi gibidir (ADR 1029).
  - Verilirse kalan noktaların kotları `cad.crs.transform`'un yoluyla projenin kotlarına çevrilir. Yatay sistem iki
    yanda da projeninkidir. Elipsoit yüksekliğinden ortometriğe H = h − N; aynı sistemde değişmez. Dışarıda
    bırakılanlar da hata için çevrilir.
- **Retler,** buluttan önce (ızgara bulutun ortasında bir noktayla denenir):
  - kuralına uymayan `heights` (`invalid_heights`);
  - projenin kotları söylenmemiş (`heights_unknown`);
  - geoidin özeti yok (`geoid_required`) ya da ızgarası cihazda yok (`geoid_missing`);
  - geoitsiz iki ayrı datum (`vertical_unsupported`);
  - dosya başka bir sistem söylüyor (`cloud_heights_differ`).
  Ret iletileri `cad.crs.transform`'un iletileridir; "to tarafı" yerine "projenin" denir.
- **Bulut okunduktan sonra:** ızgaranın dışında ya da boşluğunda kalan nokta varsa `invalid_cloud_terrain`, kaç
  noktanın kaldığıyla. Sıfır ayrım varsayılmaz.
- **Kayıt** (KCAD şema 203; Dalda 196 olarak yazıldı; main'le birleşirken 193–199 Harita işlerine verilmişti, 203 oldu.): bulut kaydına `heights` (`from`: çağrının söylediği, `to`: noktaların yazıldığı projenin
  kotları). Karşılaştırma (ADR 1029) ve makine kontrolü (ADR 1031) bu kaydın noktalarını `to`'nun sisteminde sayar.
- **Masaüstü:** Buluttan arazi'de "Bulutun kotları" seçimi (Dosyanın söylediği, Elipsoit yüksekliği (geoitle çevir),
  Projeninki); önizlemede "Kotlar: elipsoit yüksekliği → ortometrik, …; geoitle çevrildi".

## Ölçüm

- **Bağımsız başvuru** `road_cloud_terrain_cases.py` (KentOS kodu yok):
  - `road-ground-tm36.las` aynı noktaları TUREF / TM36'ya (38,75° K, 35,47° D) taşır; tamsayılar aynı, yalnız
    başlığın ötelemesi.
  - N, `geoid_cases.py`'nin sentetik TG geoidini (`fixtures/geoid/v1/tg.gtx`) kendi çift doğrusal okumasıyla, enlem ve
    boylam PROJ'un ters TM36'sıyla bulunur.
  - Yeni yüzeyin köşeleri, üçgenleri, alanı, en düşük ve en yüksek kotu da başvurunun kendi Delaunay'ındandır.
- **`road.terrains.cloud`** 57 durum, 9'u yeni:
  - Elipsoitten çevrilen bulutun en düşük kotu 71,926 m (çevrilmeden 100,089; N ≈ 28,16 m), 562 üçgen, 1078,023 m²
    başvuruyla 1e-6 içinde aynı.
  - En düşük kot 1 mm değiştirilince durum düşüyor.
  - Projeninki denen bulut çevrilmez.
  - Yedi ret: projenin kotları yok, geoit özeti yok, ızgara cihazda yok, kurala uymayan kotlar, geoitsiz başka datum,
    dosya başka sistem söylüyor, ızgaranın dışındaki 291 nokta.
- **Karşılaştırma ve makine kontrolü:** `road.terrains.compare` (28) ve `file.export.landxml` (30) birer yeni
  durumla çevrilmiş bulut zeminini projenin sisteminde sayıyor.
- **KCAD:** referans ile okuyucu 1652 dosyada aynı (`road-cloud-turned`, `broken/road-cloud-turned-in-schema-202`).
- **Resimler** `.run/gui-evidence/yol-y97/`: Buluttan arazi'de seçim ve önizlemenin "Kotlar" satırı (1440 açık, 1100
  koyu). İlk resimde satır "(sentetik) (geoitle çevrildi)" diye iki ayraç yan yana yazıyordu; noktalı virgülle
  ayrıldı.

## Açık

- **Çizime aktarılan noktalar:** LandXML, CSV ve GIS'ten gelen noktaların kotları da aynı yolla çevrilebilmeli
  (ADR 0631 §4'ün uyarısı söylüyor ama çevirmiyor). ROAD-NUM-03 bunun için açık kalır.
- **Aynı bulutta çevirme ve seyreltme sırası:** çok büyük alanlarda (onlarca km) N'in hücre içi değişimi yine küçük,
  ama ölçülmedi.
