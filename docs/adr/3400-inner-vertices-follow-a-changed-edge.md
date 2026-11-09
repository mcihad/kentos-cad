# 3400 — Topolojik düzenlemede değişen kenarın içindeki komşu köşeler

Tarih: 2026-11-09. plan/map-cad.md MAP-07, altıncı tur. Durum: taslak. ADR 2600'ün “Sonra”sındaki “Tutamaç, Köşe tablosu
ve Esnet'in köşe taşımasında da kenar içindeki komşu köşe (kenar döner; köşenin kenar üstündeki yeri oranla
korunmalı)” ve ADR 3000'in aynı satırı.

İlgili:
- [ADR 0160](0160-topological-editing.md): Topolojik düzenleme (ortak köşe ve kenar birlikte);
- [ADR 2600](2600-shared-edge-junctions-and-inner-vertices.md): Paralel kaydır'da ortak kenarın ortasındaki köşeler;
- [ADR 1261](1261-topological-editing-stops-at-a-locked-neighbour.md): Kilitli komşuda durdur.

## Dayanak

- **ArcGIS Pro, Change settings for editing geometry**
  ([belge](https://pro.arcgis.com/en/pro-app/3.1/help/editing/geometry-options-for-editing.htm), 9 Kasım 2026'da okundu):
  - “Stretch topology proportionately when moving a topology element”: bir kenar ya da ucu taşınınca bağlı
    topoloji öğelerinin genel biçimi kenarlarının göreli boyları ölçeklenerek korunur;
  - “Stretch geometry proportionately when moving a vertex”: aynısı nesnenin kendi parçaları için.
- **ArcGIS Pro 3.4, Edit topological vertices**
  ([belge](https://pro.arcgis.com/en/pro-app/3.4/help/editing/edit-topology-vertices.htm), 9 Kasım 2026'da okundu):
  harita topolojisi açıkken Köşeleri düzenle ortak sınırların bağlılığını korur.
- **ArcGIS Pro 3.4, Move topological edges or features**
  ([belge](https://pro.arcgis.com/en/pro-app/3.4/help/editing/move-topology-edges-and-nodes.htm)): “Topologically
  connected edges stretch and remain connected”.
- **ArcMap 10.4, Moving a vertex without changing a feature's general shape**
  ([belge](https://desktop.arcgis.com/en/arcmap/10.4/manage-data/editing-existing-features/moving-a-vertex-without-changing-a-feature-s-general-shape.htm)):
  orantılı esnetmede öbür köşeler parçaların oranlarını korumak için birlikte taşınır.

  Belgeler oranın nasıl hesaplandığını yazmıyor. KentOS kenar boyunu (yayda açıyı) kullanır.
- **KentOS bugün:** ADR 2600 kenar içindeki köşeleri yalnız Paralel kaydır'da taşıyordu. Tutamaç, Köşe tablosu, Esnet
  ve köşe komutu (Yaya dönüştür, Düz kenar yap) kenarın uçlarındaki ortak köşeleri taşıyordu ama kenarın ortasındakileri
  bırakıyordu:
  - iki parselin kenara değen ortak köşesi (üçlü birleşim) yerinde kalıyordu; kenar dönünce aralarında boşluk ya da
    bindirme oluyordu;
  - kenara T ile bağlanan yolun ucu kenardan kopuyordu;
  - komşunun kenar boyunca ölçülmüş ara köşesi V biçimini alıyordu.

## Karar

1. **Kural** (`topology_edit::changed_edges` ve `carried`; `topology_follow::neighbours`):
   - Değişen kenar: düzenlenen nesnenin köşe sayısı aynı kalan yolunda, ucu taşınan ya da yayı değişen her kenar.
   - Görünen komşuların kenarın içinde (kenara 1 µm içinde, iki uçtan 1 µm'den uzak) duran her köşesi, kenarın yeni
     hâlinin aynı kesrine gider. Düz kenarda kesir boyun, yayda açının kesridir.
   - Bunlar üçlü birleşimin köşesi, kenara bağlanan çizginin ucu, komşunun ara köşesi ve deliğin kenarındaki köşedir.
     Nokta yalnız Noktalar da açıkken gider.
   - Oranın korunması kenar ötelenince köşeyi aynı yer değiştirmeyle taşır. Kenar kendi doğrusunda uzayınca köşe de
     orantılı kayar (göreli boylar korunur).
2. **Yayı değişen kenar** (Yaya dönüştür, Düz kenar yap, yayın tutamacı): komşunun kenar boyunca duran bir parçası, aynı
   kesirler arasındaki yeni kenar parçasının yayını alır (tan(θ·(t₂ − t₁)/4)). Örneğin iki parsel düz kenarın yarılarını
   paylaşıyorsa, kenar yay olunca ikisi de yayın yarısı olur.

   Ucu taşınan yay bulge'ını korur, açısı değişmez. Bu yüzden komşunun yay parçası da bulge'ını korur ve yeni yayın
   üstünde kalır.
3. **Esnet'te esnetilen öbür nesneler:** seçimsiz pencere iki nesneyi birden esnetebilir. Örneğin A ve B2'nin köşesi
   aynı yerde pencerede kalabilir. O zaman B2'nin A'nın kenarındaki T köşesi de taşınır (`inside_edges` öbür esnetilenlerle,
   `carry_edited`), sınır ayrılmaz.

   Tutamaç ve Köşe tablosu esnetilen öbür nesneyi düzeltmez. Onlarda esnetilen bir nesnenin köşesinin olduğu yer
   taşınmaz: sınır yine ayrılmaz, eskisi gibi yerinde kalır.
4. **Kilitli komşu:** köşesi gitmez. ADR 2600'deki gibi sınırın ayrıldığı söylenir (`neighbour_locked`); Kilitli komşuda
   durdur açıkken hiçbir şey yazılmaz (`shared_boundary_locked`). Gizli komşu katılmaz.
5. **Köşe sayısı değişen düzenleme** (Köşe ekle, Köşe sil): kural uygulanmaz.
6. **İleti** (tutamaç, Köşe tablosu): “Topolojik düzenleme: 3 komşu nesne de değişti; kenarın ortasındaki 2 ortak köşe
   de kenarla gitti.” (`Neighbours.carried`). Esnet'in cevabında izleyenler `followed`'dadır.
7. **Paralel kaydır** kendi kuralıyla kalır (ADR 2600: kenara dik aynı uzaklık, `neighbours_with`). Komut sözleşmeleri
   değişmez.
8. **Kapsam işareti:** **CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi**, ADR 2600'deki gibi.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/topology_inner_cases.py`; yay merkezi, kesirler ve parça yayları betikte,
  KentOS kodu yok; koordinatlar 10⁻⁶ m içinde):
  - `cad.entities.stretch.inner.json`, 20 durum:
    - dönen, kısalan, ötelenen, kendi doğrusunda uzayan kenar;
    - T ile bağlanan yol ve çizgi;
    - komşunun ara köşesi;
    - kilitli (uyarı, durdur), gizli, 1 mm uzaktaki ve uzantıdaki köşe;
    - Noktalar da;
    - açık çoklu çizgi ve çizgi;
    - deliğin kenarı;
    - yayın ucu, başı, iki ucu;
    - seçimsiz pencerede iki esnetilen nesne;
    - Topolojik düzenleme kapalı.
  - `cad.entities.vertex.inner.json`, 20 durum:
    - Yaya dönüştür dört kenarda, noktadan (içe ve dışa) ve yarıçapla, parça yaylarıyla;
    - Düz kenar yap;
    - Noktalar da açık ve kapalı;
    - kilitliyle durdur;
    - Topolojik düzenleme kapalı;
    - ortasında köşe olmayan kenarlar;
    - Köşe ekle'de kural uygulanmaz.
  - Rust ve Python geçer; Esnet'in, köşe komutunun ve Paralel kaydır'ın eski durumları değişmedi.
- **Çekirdek** (`topology_edit::tests`):
  - dönen kenar;
  - açısı korunan yay;
  - yaya dönen kenar ve komşunun parça yayı.
- **Python** (`python/tests/test_topology_inner.py`): seçimsiz Esnet'te ortak T köşesi, Yaya dönüştür'de parça yayları,
  Topolojik düzenleme kapalı.
- **MCP** (`crates/native/mcp/tests/topology_inner.rs`): seçimsiz Esnet'in cevabı durumunki; kaydedilen dosyada T köşesi
  ve J'nin ucu dönen kenarda.
- **Masaüstü** (`topology_inner_scenes.rs`):
  - tutamaçla sürüklenen köşe T köşesini ve çizgi ucunu taşır, ileti ikisini sayar, tek geri alma alır;
  - Köşe tablosunda yazılan koordinat da aynısını yapar;
  - resimler `.run/shots/arac-tutamac-ic-kose-*`, `arac-kose-tablosu-ic-kose-*`.

  Etkileşim izleri (tutamaç ve Esnet) ve çapraz denetimler geçer.

## Sonra

- Köşe sil'de birleşen iki kenarın içindeki komşu köşeler (bugün kalır).
- Tutamaçla birden çok nesne birlikte taşınınca öbür taşınanın kenar içindeki köşesi (bugün yerinde kalır).
- Paralel kaydır'da yaylı ortak kenarda içteki köşe (ADR 2600 Sonra; bugün ret).
- ~~Topoloji denetiminin veri formu yolu (MAP-07'nin öbür maddesi).~~ ADR 3800.
- Web (TODOS §0.2): çekirdeğin `changed_edges`/`carried`'ı WASM'a.
