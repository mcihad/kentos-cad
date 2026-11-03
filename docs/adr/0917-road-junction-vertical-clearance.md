# 0917 — KentOS Yol: ayrı seviyeli kavşakta düşey açıklık

Tarih: 2026-11-04. plan/road.md Y17.4 (ROAD-JUNC-09). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** Analyze › Inquiry › Minimum Vertical Distance between Entities iki öğe (profil, 3B çoklu çizgi, koridora
  gömülü özellik çizgisi) arasındaki düşey uzaklığı, iki öğenin kotunu ve uzaklığın ölçüldüğü X,Y yerini verir; aksların
  kesiştiği yerde tasarım profilleri arasındaki uzaklığı bulmak için anlatılır; gerekli açıklıkla karşılaştırma
  kullanıcınındır
  ([To Identify the Vertical Distance Between Two Entities](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-5BBC3642-1511-4E75-BF7A-E66672E8A030.htm),
  4 Kasım 2026'da okundu).
- **Trimble Novapoint (Bridge › Tools):** Calculate Minimum Vertical Distance yol yüzeyinin ve köprü yüzeyinin çoklu
  çizgilerini alır; uzaklığı planda her kesişim noktasında hesaplar, aralarında kullanıcının verdiği aralıkla doğrusal
  ara değer alır (düşey geometri doğrusal değilse küçük hata, aralık küçük tutulmalı); en küçük uzaklığı yazar ve
  yerini işaretler
  ([Calculate Minimum Vertical Distance](https://novapointhelp.trimble.com/Bridge/Tools/Calculate-Minimum-Vertical-Distance)).
- **OpenRoads Designer:** Clash Detection seçilen öğe kümeleri arasında fiziksel çakışmaları ve açıklık sorunlarını
  bulur; OpenBridge Modeler yapı ile yol arasında gereken en küçük açıklığı denetler
  ([OpenRoads Designer Help: Analyze](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20Help-v19/en/analyze.html),
  [WSDOT/Bentley sunumu](https://wsdot.wa.gov/eesc/bridge/WBES/2017/Session4/4C3-XinLou.pdf)).
- **Ölçme kuralı:** TxDOT en küçük düşey açıklığı bütün trafik yolu genişliğinde (şeritler ve kullanılabilir banket)
  ve yapının derinliği boyunca yeterince ölçüyle arar
  ([How to Measure Minimum Vertical Clearance](https://www.txdot.gov/manuals/trf/smk/warning_signs/vertical_clearance-i1000022/how_to_measure_minimum_vertical_clearance-i1000147.html)).
- **Netcad:** NETPRO için kavşakta düşey açıklık denetimini anlatan çevrimiçi resmî belge bulunamadı.

## Karar

### §1 Komut

`road.junctions.clearance` v1, yalnız okur. Girdi:

- `junction`: ayrı seviyeli kavşağın kimliği ya da adı;
- `profiles`: kolların sırasıyla iki boykesit (kimlik ya da ad); yoksa her kolun aksının tek boykesiti;
- `depth`: üst yolun tabliye derinliği, boykesit kotundan alt yüzüne (m); kullanıcınındır, varsayılanı yok;
- `step`: örnekleme aralığı (m);
- `minimum`: isteğe bağlı gerekli açıklık; az kalırsa `junction_clearance_short` uyarısı. KentOS bir sayı önermez.

### §2 Hesap

Kesişimde boykesit kotu yüksek olan kol üst yoldur. Alt yolun genişliği (`−left … right`) ve km'si kesişimin iki
yanında `(üst ve alt yolun sol ve sağ kenarlarının toplamı) / sin(açı) + step` kadar, ikisi de `step`'ten büyük
olmayan eşit aralıklarla bölünür; ızgaranın her noktası üst yolun aksına dik izdüşürülür (aksın gerçek geometrisi,
`Alignment::locate`), üst yolun genişliğine düşenler örnektir. Alt yolun her boy çizgisinde ve her en çizgisinde üst
yolun kenarı ikiye bölmeyle (1e-12 m) bulunur ve örneğe katılır: örtüşme bölgesinin köşeleri kesin, kenarları her
çizgide yer alır. Her örnekte açıklık = üst boykesit kotu − `depth` − alt boykesit kotu. Cevap en küçük ve en büyük
açıklığı yerleriyle (x, y, iki km ve iki öteleme, iki kot), iki aksın kesişimindeki açıklığı ve örnek sayısını verir.

### §3 Ret ve uyarılar

Ret sırası: `not_a_road_project`, `unknown_junction`, `invalid_junction` (okunamayan ya da hemzemin kavşak),
`unknown_road_profile`, `invalid_road_profile` (kurulamayan boykesit), `invalid_clearance` (iki boykesit değil,
boykesit kolun aksında değil, bir kolun boykesiti yok ya da birden çok, `depth` eksi, `step` 0'dan büyük değil,
`minimum` eksi, en çok 1 000 000 ızgara noktası, iki kol kesişimde aynı kotta, boykesit örtüşme bölgesini örtmüyor,
yollar örtüşmüyor). Uyarılar: `junction_stale`, `profile_stale`, `junction_clearance_short`.

### §4 Masaüstü

Kavşaklar penceresinde ayrı seviyeli kavşağın altında Düşey açıklık bölümü: iki boykesit, tabliye derinliği, aralık,
gerekli açıklık; en küçük, en büyük ve kesişimdeki açıklık, planda örtüşme bölgesi ve en küçüğün yeri.

## Açıklar

- Boykesit kotu yolun aks kotudur: enine eğim ve dever, kaplama kalınlığı alınmaz; koridor yüzeyinden açıklık açık.
- Tabliye düz kabul edilir (sabit derinlik); kiriş alt kotu, mesnet ve ayak yerleri (ROAD-JUNC-08) açık.
- Alt yoldaki araç zarfı (Y17'nin araç izi) ve yatay açıklık açık.
