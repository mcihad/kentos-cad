# 5260 — İkiden çok görüntünün birleşme çizgisi ağı; NoData'dan ayak izi

Tarih: 2026-11-11. plan/map-cad.md MAP-12, onuncu tur. Durum: taslak. ADR 4700'ün “Sonra”sındaki iki iş: ikiden çok
görüntünün birleşme çizgisi ağı (üçlü düğüm); NoData'dan ayak izi. Kenar algılama ve yüksek nesnelerden uzaklaştırma
açık kalır. Türeyen nesnenin kaynağa izi sahibin kararını bekler.

İlgili:
- [ADR 4700](4700-orthophoto-seamline-polygons-built.md): iki görüntünün birleşme çizgisi ve mozaik elemanları;
- ADR 4460 (mozaik elemanlarından kayıklık ölçümü), 1780 (ortofoto kontrolü).

## Dayanak

11 Kasım 2026'da okundu; not `.run/research/map-cad/uygulama/MAP-12.10.md`.

- **ArcGIS Pro, Build Seamlines**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/build-seamlines.html)):
  - her ayak izine bir birleşme çizgisi düşer;
  - örtüşmede hangi görüntünün kullanılacağını sıralama kuralı seçer: kuzeybatı, bakış noktasına en yakın (görüntünün
    nadir yeri) ya da öznitelik;
  - hesap yöntemleri:
    - Geometry: ayak izlerinin kesişimine göre, örtüşmeyen yerlerde ayak izleri birleşir;
    - Radiometry;
    - Copy footprint;
    - Edge detection;
    - Voronoi: alan Voronoi diyagramı;
    - Disparity: binaları kesmemek için stereo farkı.
- **ArcGIS Pro, Build Footprints** ve GDAL'ın `gdal_footprint`'i (ADR 4700'ün notu): ayak izi geçerli piksellerin (NoData
  olmayan) sınırıdır.
- **Pan, Wang, Li, Li (2009)**: alan Voronoi diyagramı (ADR 4700'ün iki görüntülü hâli).
- **Mevzuat:** BÖHHBÜY birleşme çizgisinin yerine kural koymaz. Kayıklık m. 67(6) ile denetlenir. TUCBS Ortogörüntü'de
  ayak izi ve mozaik elemanı kavramları vardır.
- **KentOS bugün:** birleşme çizgisi yalnız iki görüntü arasında kuruluyor. Ayak izi rasterin dört köşesi ya da elle
  çizilen bir alan.

## Karar

1. **`cad.orthophoto.network` v1, Birleşme çizgisi ağı** (yeni, belgeyi değiştirir, tek geri alma adımı “Mozaik
   elemanları ağı”):
   - **Girdi:** `images` (2–50 ortofoto, projenin sisteminde); `footprints` (aynı sırayla; alan ya da kapalı çoklu çizgi,
     düz kenarlı, dışbükey; verilmezse rasterlerin dört köşesi); `layer`.
   - **Kural:** ayak izlerinin birleşiminin her yeri, onu kaplayan görüntülerden ayak izinin merkezi (alan ağırlık
     merkezi) en yakın olana gider; eşitse önce verilene. Bu, bakış noktasına en yakın kuralının merkezle hâlidir:
     merkezlerin Voronoi diyagramının ayak izleriyle kesilmişidir.
   - **Hesap** (`ops::seam_network`, ortak çekirdek):
     - görüntünün elemanı, ayak izinden her öbür görüntünün ayak izinin ona daha yakın kısmının çıkarılmasıdır. O kısım
       eşit uzaklık doğrusuyla kesilen dışbükey çokgendir;
     - fark ortak çekirdeğin alan cebiriyledir;
     - hesap birinci ayak izinin ilk köşesi çevresinde yapılır.
   - **Halkalar:** dış halka saat yönünün tersine, delik saat yönünde. Kapanış tekrarı ve doğrusal köşeler atılır (dönüşü
     koordinat boyunun 10⁻¹²'sinden küçük). Halka en güneydeki (eşitse en batıdaki) köşeden başlar. Parçalar ve
     delikler başlangıçlarına göre sıralanır.
   - **Düğüm:** üç ya da daha çok elemanın paylaştığı köşe. Üç merkezin hücrelerinin buluştuğu yerde bu, merkezlerin
     çevrel merkezidir; dört merkez aynı çemberdeyse dört eleman bir düğümde buluşur.
   - **Cevap:** elemanlar girdinin sırasıyla (parçaları ve delikleri, alanı, merkez, yazılınca kimlik), düğümler
     (güneyden kuzeye, batıdan doğuya; yeri ve görüntüleri), kaplanan alan, katman, notlar.
   - **Yazma:** her eleman bir alan; çok parçalıysa parçalarıyla, delikliyse delikleriyle. Öznitelikler `Görüntü` ve
     `Yöntem` (“merkeze en yakın”). Katman verilmezse `mozaik-elemanlari`; yoksa aynı adımda açılır.
   - **Retler:**
     - `invalid_input`: ikiden az ya da 50'den çok görüntü; bir görüntü iki kez; ayak izleri her görüntüye bir değil;
       dışbükey olmayan ya da alan kaplamayan ayak izi; her yeri başka görüntüye daha yakın olan görüntü (aynı yerdeki
       ikinci görüntü);
     - `invalid_uid`, `entity_not_found`, `not_a_raster`, `crs_mismatch`;
     - `layer_not_found`, `layer_locked`, `invalid_revision`, `revision_conflict`.
2. **`cad.orthophoto.footprint` v1, Ayak izi** (yeni, belgeyi değiştirir, tek geri alma adımı “Ayak izi”):
   - **Geçerli piksel:** bantlarından biri NoData değilse (ve NaN değilse). NoData'sı olmayan rasterde her piksel
     geçerlidir; ayak izi rasterin sınırıdır.
   - **Sınır:** geçerli piksellerin piksel kenarlarından sınırı, 4 komşulukla. Köşeden değen pikseller ayrı parçadır.
     İzleme yönünde geçerli piksel sağda kalır; iki yol olan köşede sağa dönülür. Delik, sol yanındaki geçersiz pikselin
     merkezini içeren en küçük dış halkaya bağlanır.
   - **Zarf (`hull`):** geçerli piksellerin köşelerinin dışbükey zarfı (Andrew'un monoton zinciri, tamsayılarla). Birleşme
     çizgisi ve ağ bunu ayak izi olarak alır.
   - **Yere çevirme:** köşeler rasterin afiniyle yere çevrilir (döndürülmüş raster de); halkalar ağınki gibi düzenlenir.
   - **Cevap:** parçalar, alan, okunan ve geçerli piksel sayısı, NoData değeri, katman, yazılınca kimlik, notlar.
   - **Yazma:** bir alan (parçaları ve delikleriyle) `ayak-izleri` katmanına (yoksa aynı adımda açılır). Öznitelikler
     `Görüntü` ve `Yöntem` (“piksel sınırı” ya da “dışbükey zarf”).
   - **Retler:**
     - `invalid_input`: 25 milyondan çok piksel; her piksel NoData;
     - görüntünün dosyası: `file_not_found`, `invalid_file_path`, `import_unreadable`;
     - `invalid_uid`, `entity_not_found`, `not_a_raster`, `crs_mismatch`;
     - `layer_not_found`, `layer_locked`, `invalid_revision`, `revision_conflict`.
3. **Masaüstü:** Raster sekmesinin **Mozaik birleşme çizgisi** penceresinin üstünde **İki görüntü | Ağ | Ayak izi**
   seçimi; yeni şerit düğmesi yok.
   - **Ağ:** çizimin ortofotoları işaretli liste (hepsi işaretli açılır); plan her değişiklikte koşar. Özet; elemanlar
     tablosu (görüntü, parça, delik, alan); elemanlar ayrı renklerle ve düğümler noktayla çizilir. **Ağın elemanlarını
     çiz** yazar ve seçer.
   - **Ayak izi:** raster ve dışbükey zarf kutusu; özet, parçalar tablosu, plan. **Ayak izini çiz**.
   - Çizimde tek raster varsa pencere Ayak izi'yle açılır.
4. **Kapsam işareti:** ADR 4700 gibi **CAD + CBS** (raster bağlam sekmesi).

## Sınırlar

- Ağ ayak izlerini dışbükey ister: içbükey ayak izi parçalara bölünmeli ya da Ayak izi'nin zarfı kullanılmalı.
- Ağın kuralı merkeze en yakın olandır. Radyometrik fark, kenar algılama ve yüksek nesnelerden uzaklaştırma ağda yok;
  iki görüntü arasında radyometrik fark ADR 4700'dedir.
- Merkez ayak izinin alan ağırlık merkezidir; çekimin nadir yeri (dış yöneltme) kayıtlı değildir.
- Ayak izi tek bantlı ve çok bantlı rasterde NoData değeriyle çalışır; alfa bandı ve maske dosyası okunmaz.
- Ayak izi en çok 25 milyon piksel okur; büyük rasterde küçültülmüş kopya gerekir.
- Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/orthophoto_network_cases.py`; KentOS kodu yok):
  - **elemanlar:** shapely'nin farkıyla; yarım düzlem büyük bir çokgenle;
  - **düğümler:** üç görüntülü durumda merkezlerin çevrel merkezine eşit olduğu betikte denetlenir (70; 38,33);
  - **ayak izi:** geçerli piksellerin karelerinin shapely birleşimi. Görüntüler betikte bayt bayt yazılır (8 bitlik TIFF,
    GeoTIFF anahtarları, NoData etiketi 42113).
  - **`cad.orthophoto.network`, 25 durum:**
    - üç görüntü (plan, yazma ve geri alma, doğrulama, sırası değişmiş);
    - iki görüntü; 2 × 2 dört görüntü (dört elemanlı düğüm; yazma);
    - örtüşmeyen görüntü; büyüğün içindeki küçük görüntü (büyüğün elemanı delikli);
    - verilen dörtgen ayak izleri; verilen katman; Yol projesi;
    - 13 ret.
  - **`cad.orthophoto.footprint`, 21 durum:**
    - delik, kenar çentiği ve köşeden değen parça (plan, yazma, doğrulama);
    - zarf (plan, yazma); NoData'sız raster;
    - 20° döndürülmüş raster ve zarfı; verilen katman; Yol projesi;
    - 11 ret.

  Bozma denemesi: düğümün kuzeyi 1 cm ve ayak izinin bir köşesi değiştirilince koşucu tam o ikisini yakalar (46'da 2).
- **Çekirdek** (`ops::seam_network::tests`): üç kare tek düğümde (elle 87/12), alanların toplamı birleşim; delik ve
  köşeden değen parça, zarf.
- **Python** (`python/tests/test_orthophoto_network.py`), testin kendi hesabıyla:
  - üç dikdörtgenin içerme–dışlamayla birleşim alanı ve merkezlerin çevrel merkezi;
  - TIFF'in şeridinden okunan geçerli piksellerle alan;
  - Andrew zinciriyle zarfın alanı.

  Python ortak durumları geçer.
- **MCP** (`crates/native/mcp/tests/orthophoto_network.rs`): düğüm, alanlar, tek görüntünün reddi; ayak izinin alanı ve
  parçaları.
- **Masaüstü** (`ortho_network_tests`, 2 test ve resim):
  - Ağ durumun cevabını verir;
  - işareti kalkan görüntüyle iki eleman kalır, düğüm kalmaz;
  - yazma üç alan ekler ve seçer;
  - Ayak izi tek rasterle açılır, sınır ve zarf durumunkiler.

  Çapraz denetimler 161 test. Resimler `.run/gui-evidence/mozaik-agi-5260/`.

## Sonra

- Ağda radyometrik fark ve kenar algılama, yüksek nesnelerden uzaklaştırma.
- Çekimin nadir yerinin (dış yöneltme) merkez olarak kullanılması.
- Alfa bandı ve maske dosyasından ayak izi; büyük rasterde piramit düzeyinden ayak izi.
- Görüntüden türeyen nesnenin kaynağa izi (MAP-01'in sahip kararı).
- Web (TODOS §0.2).
