# 2220 — Sınır grafiği çizimde ve yaylı sınırda eksik köşe

Tarih: 2026-11-08. plan/map-cad.md MAP-08, üçüncü tur. Durum: taslak. Plan satırı: “Ortak sınır yalnız iki polygonun
tesadüfen çakışan koordinatları olmasın. Mantıksal sınır kimliği üzerinden komşuluk ve kaynağı izlenebilsin; CAD/GIS
geometrisi bunun denetlenebilir temsili olsun.”

İlgili: [ADR 2000](2000-parcel-boundary-graph.md), sınır grafiği. Bu ADR onun “Sonra”sındaki iki maddeyi kapatır:

- “Sınır grafiğinin sahnede gösterilmesi (köşe numaraları, sorunların işaretleri)”;
- “Yaylı sınırda eksik köşe denetimi”.

## Dayanak

- **QGIS, Topology Checker**
  ([eklenti belgesi](https://docs.qgis.org/latest/en/docs/user_manual/plugins/core_plugins/plugins_topology_checker.html);
  8 Kasım 2026'da okundu):
  - “Show errors on the canvas” kutusu hataların yerini harita tuvalinde gösterir;
  - tablodaki bir satıra tıklamak haritayı ilgili nesneye yaklaştırır.
- **ArcGIS Pro, parsel ağının topolojisi**
  ([Work with the geodatabase topology](https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/aboutparcelfabrictopology.html);
  8 Kasım 2026'da okundu):
  - parsel çizgileri ve noktaları ayrı öğelerdir;
  - öntanımlı kurallar arasında “Lines - Endpoint Must Be Covered By - Points” ve “Polygon - Boundary Must Be Covered
    By - Lines” vardır.

  Sınır, köşeleri olan bir çizgidir; KentOS'un grafiği aynı ayrımı geometriden çıkarır (ADR 2000).
- **Kabarıklık** (Autodesk, AutoCAD ActiveX
  [GetBulge](https://help.autodesk.com/cloudhelp/2016/ENU/AutoCAD-ActiveX/files/GUID-6AC9CF99-7230-4333-859A-1CBACB57B5BA.htm);
  8 Kasım 2026'da okundu):
  - kabarıklık, köşeden sonrakine giden yayın merkez açısının dörtte birinin tanjantıdır;
  - eksi değer yayın saat yönünde gittiğini söyler;
  - 0 doğrudur, 1 yarım çemberdir.
- **Geometri:** bir kiriş çemberi iki yaya böler ve bu yaylar kirişin doğrusunun iki ayrı yanındadır. Çembere yakın bir
  nokta, ancak yayın ortasıyla kirişin aynı yanındaysa o yayın üstündedir.
- **KentOS bugün** (ADR 2000):
  - `missingVertex` yalnız düz sınırlarda aranıyordu. Yaylı bir sınırın ortasında köşesi olmayan T birleşimi
    görünmüyordu;
  - grafik yalnız penceredeki tabloda okunuyordu. Bir köşenin ya da sorunun çizimde nerede olduğu Y, X'ten
    bulunmalıydı.

## Karar

1. **Yaylı sınırda eksik köşe.** Bir köşe, ucu olmadığı yaylı bir sınır için şu iki koşulu sağlıyorsa `missingVertex`
   olur:
   - yayın çemberine 1 µm'den yakındır;
   - kirişin yayın yanındadır, kesin olarak (kirişin üstünde değil).

   Ayrıntılar:
   - Çember kabarıklıktan bulunur: merkez kirişin ortasından, kirişe dik yönde (1 − b²)/(4b) · c uzaklıktadır;
     yarıçap c (1 + b²)/(4|b|)'dir. Hesap sayı kaybı olmasın diye yayın başlangıcına göre yapılır.
   - Bağımsız başvuru çemberi başka yoldan bulur: yayın iki ucundan ve ortasından geçen çember (çevrel merkez,
     mpmath'te 50 basamak). Yan sınaması yayın ortasına göredir.
   - Sorunun yeri köşedir. Parselleri köşenin parselleri ile sınırın parselleridir, ADR 2000'deki gibi.
   - Aynı köşede birden çok sorun varsa sınırların sırasıyla dizilir.
   - Izgara aday seçimi yayın kutusunu kullanır:
     - en çok yarım çemberlik yayda kirişin kutusu, yayın yüksekliği kadar her yandan büyütülür (yay kirişinden daha
       uzağa gitmez, uçlarını da geçmez);
     - daha büyük yayda çemberin kutusu alınır.

     Cevap ızgaradan bağımsızdır.
2. **Çizimde göster** (Sınır grafiği penceresi, Sorunluları seç'in yanında):
   - pencere kapanır ve görünüm köşelere getirilir (her yandan onda bir pay);
   - çizimin üstünde işaretler kalır:
     - her sorunda tehlike renginde bir halka ve sorunun adı;
     - her köşede bir nokta ve numarası (K…, vurgu renginde);
     - her sınırın numarası (S…, eğrisinin ortasında, çizimin mürekkebiyle).
   - Ekranda birbirini sıkıştıracak numaralar yazılmaz. Sorun adları hiç atlanmaz: her biri dört yerden (sağ üst,
     sağ alt, sol üst, sol alt) başka bir halkaya ve yazıya değmeyenine, yoksa en az değenine konur. Ekranda en çok
     600 numara yazılır.
   - İşaretler çizim değiştikçe yeniden hesaplanır: düzeltilen köşenin halkası hemen gider. İstenen parsellerden biri
     artık okunmuyorsa (silindiyse) işaret kalmaz.
   - **İşareti kaldır** (`data.unmark`) ya da başka bir çizim işaretleri kaldırır.
   - Çizime bir şey yazılmaz. Penceredeki bilgi satırı bunu söyler.
3. **Komut:** `cad.parcels.boundaries`'ın sözleşmesi değişmez, aynı alanlar. Katalogdaki açıklama yaylı sınırı da
   söyler. Python ve MCP aynı denetimi alır.
4. **Kapsam işareti:** ADR 2000'deki gibi:
   - komut Harita, Yol, CBS ve Mimari'de;
   - pencere Plan › Ada ve parsel'de, CBS'de Veri › Tablo'da.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/parcel_boundaries_cases.py`, KentOS kodu yok. 28 durum (24'ten). Yeni durumların
  hepsi yarıçapı 26 m, yüksekliği 2 m olan bir yayın üstündedir:
  - yayın tepesi üstteki iki parselin köşesidir, alttaki parselin değil: eksik köşe, üç parsel;
  - köşe yayın çemberinde ama kirişin öbür yanında: sorun yok;
  - köşe yaydan 0,4 µm uzakta: eksik köşe;
  - köşe 3 µm uzakta: sorun yok.

  Eski 24 durumun cevabı değişmedi. Rust ve Python geçer. Python koşucusu yerel modül yeniden derlenmeden önce eski
  kodla çalıştı: tam olarak iki eksik köşe durumu düştü. Bu, durumların kuralı yakaladığını gösterir.
- **MCP** (`crates/native/mcp/tests/parcel_arc_corners.rs`): durumun çizimi `.kcad` olarak açılır. Yayın tepesindeki
  sorun ve kirişin öbür yanındaki köşe durumunkilerle aynıdır.
- **Masaüstü** (`parcel_boundaries_window_tests.rs`, `the_graph_stays_on_the_drawing_until_unmarked`):
  - Çizimde göster pencereyi kapatır;
  - işaretler grafiğin köşe ve sınır sayılarıyla altı sorunun adlarını taşır;
  - Q7 silinince işaretler yeniden hesaplanır: çakışma ve iki eksik köşe gider;
  - İşareti kaldır işaretleri siler.

  Çapraz masaüstü denetimleri 134 geçer. Resimler `.run/gui-evidence/sinir-grafigi-cizimde-2220/`: pencere, çizimde
  sorunlu ada ve yayın tepesindeki eksik köşe.

## Sonra

- Kalıcı parsel kaydı ve kalıcı sınır kimliği, sınırın kaynağı (ADR 2000 Sonra).
- Tablodaki satırın (sınır, köşe, sorun) çizimde vurgulanması ve satıra tıklayınca yaklaşma (QGIS'teki gibi).
- Aynı köşeler arasında yay ile kirişin uyuşmaması (iki parselin aynı sınırı farklı kabarıklıkla çizmesi).
