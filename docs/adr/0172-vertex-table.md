# ADR 0172: Köşe tablosu

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın on beşinci işi `HYB-15`. Ayrıntılar bu ADR'nin
  varsayılanlarıdır.
- **Bağlam belgesi:** TODOS.md `HYB-15` (ilgili: `HYB-06`, `HYB-04`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md);
  ADR 0058 ve 0114 (alt panel, Koordinat listesi), ADR 0153 (nokta editörü: yerinde düzenleme, taslak satır, iletiler), ADR 0068 ve 0074
  (tutamaç, Ortasına köşe ekle, Köşeyi sil, Yaya dönüştür), ADR 0047 (Köşe ekle/sil, `cad.entities.edit`), ADR 0069 (yaylı kenar,
  bükümün işareti), ADR 0142 (köşe kotu), ADR 0143 (çok parçalı alan), ADR 0160 (topolojik düzenleme), ADR 0149 (gösterim kuralı);
  QGIS Vertex Editor, ArcGIS Pro Edit Vertices ve Geometry properties.

## Bağlam

Alt paneldeki Koordinat listesi seçili nesnenin köşelerini Y, X, kenar uzunluğu ve semtle gösterir, salt okunur. Bir köşenin yerini ya
da kotunu yazarak düzeltmenin yolu yoktur: tutamaç sürüklenir (yazılan nokta tutamaçla da verilebilir, ama köşe köşe ve tabloyu
görmeden), kot için Kot ver bütün nesneye çalışır, yayın yarıçapı hiç yazılmaz (Yaya dönüştür bükümü sürükleyerek verir).

QGIS'in Vertex Editor paneli seçilen nesnenin köşelerini x, y (z, m ve yaylı köşede r) sütunlarıyla gösterir: satır seçimi haritada
köşeyi vurgular, hücreye yazılan değer köşeyi taşır, seçili satırlar silinir. ArcGIS Pro'nun Edit Vertices ve Geometry properties'i
köşeleri parça parça tabloda verir; köşe eklenir, silinir, x, y, z yazılır, kenarın türü (doğru, yay) ve yarıçapı görülür. Netcad'de
karşılığı yoktur.

Nokta editörü (ADR 0153) alt panelde aynı işi noktalar için yapar: yerinde düzenleme, taslak satır, tek adımlık yazma, iletiler. Köşe
tablosu onun kurallarını izler.

## Karar

### 1. Yer

Köşe tablosu, Koordinat listesi sekmesinin düzenlenen biçimidir; ayrı bir sekme ya da pencere değildir.

- Seçimde yalnız bir nesne varsa ve o çizgi, çoklu çizgi ya da kapalı alansa tablo düzenlenir.
- Birden çok nesne seçiliyken liste bugünkü gibi ilk nesneyi gösterir ve düzenlenmez; altında: “(ilk nesne gösteriliyor; düzenlemek
  için tek nesne seçin)”.
- Nesne kilitli katmandaysa tablo düzenlenmez; altında: “(katman kilitli)”. Gizli katmandaki nesne seçili kalmışsa yine düzenlenir.
- Öbür türler (daire, yay, elips, eğri, yazı, ölçü, …) ve yalnız noktalardan oluşan seçim bugünkü gibi salt okunur gösterilir.
- `view.coords` (“Koordinat listesi”) sekmeyi açar. Takma adlarına KÖŞETABLOSU ve KT eklenir; komut aramasında “Köşe tablosu” da bulur.

### 2. Tablo

- **Satırlar:** nesnenin köşeleri, çekirdeğin sırasıyla: çizginin iki ucu; çoklu çizginin köşeleri; alanın dış halkası, delikleri,
  sonra her parçanın halkası ve delikleri (ADR 0143). Bütün halkalar tek tabloda, art arda.
- **Sütunlar:** Köşe, Halka, Y, X, Z, Yarıçap, Kenar, Semt.
  - Köşe satırın tablodaki sırasıdır (1, 2, 3…), halkadan halkaya sürer.
  - Halka yalnız nesnenin birden çok halkası varsa gösterilir: “Dış”, “Delik 1”, “Parça 2”, “Parça 2, delik 1”.
  - Y ve X köşenin yeri, Z kotudur (kotsuz köşede boş). Eksenlerin adları proje türünündür (ADR 0165 §4).
  - Yarıçap köşeden çıkan kenarındır (sonraki köşeye; kapalı halkanın son köşesinden ilkine): düz kenarda boş, yayda işaretli yarıçap.
    Artı, yay köşeden sonrakine giderken sola döner (saat yönünün tersi; bükümün işareti, ADR 0069); eksi, sağa döner.
  - Kenar köşeden sonrakine kiriş uzunluğu, Semt kirişin semtidir (bugünkü liste gibi). Çoklu çizginin ve çizginin son köşesinde
    Yarıçap, Kenar ve Semt boştur.
- **Sayılar** projenin ondalıklarıyla, ADR 0149'un gösterim kuralıyla yazılır; düzenlemeye başlanınca hücre değerin tamamını gösterir.
- **Altta** bugünkü özet: nesnenin adı, alanı ve çevresi ya da uzunluğu (nesnenin kendi ölçüleri: yaylar ve delikler sayılmış).

### 3. Seçim ve vurgu

- Satıra tık satırı seçer; Ctrl ekler ya da çıkarır, Shift aralığı seçer (nokta editörünün `click_pick` kuralı). Satır seçimi
  tablonundur; çizimin seçimini değiştirmez.
- Seçili satırların köşeleri çizimde vurgulanır: tutamacın üstünde vurgu renginde bir halka. Vurgu nesnenin tutamaçlarıyla birlikte
  çizilir.
- Köşe hücresine çift tık o köşeye görünümü taşır (ölçek değişmez); satırın menüsünde ve araç çubuğunda Göster de aynı işi yapar.
- Seçim başka nesneye geçince ya da nesnenin köşe sayısı değişince satır seçimi temizlenir; aynı nesnede yazma sonrası seçim korunur.

### 4. Yerinde düzenleme

Y, X, Z ve Yarıçap düzenlenir; Köşe, Halka, Kenar ve Semt düzenlenmez.

- **Başlatma, bitirme, yanlış değer, değişmeyen değer:** nokta editörünün kuralları (ADR 0153 §3): çift tık; Enter yazar ve aynı
  sütunda alttaki satıra geçer; Tab sağdakine, Shift+Tab soldakine (Yarıçap'tan sonra alttaki satırın Y'si); başka yere tık yazar ve
  bitirir; Esc vazgeçer. Sayılar nokta girişinin dilbilgisiyle okunur (`parse_number`), yerel projede projenin biriminde (ADR 0165 §2).
- **Her hücre bir geri alma adımıdır: “Köşe düzenle”** (`cad.entities.edit`, `properties` işlemi; nesnenin bütün geometrisi, kotlarıyla).
- **Y, X:** köşe taşınır. Yaylı kenarların bükümü korunur (tutamacın kuralı, ADR 0068): yarıçap kirişle birlikte değişir. Köşe komşu
  köşesinin yerine (1 nm) taşınamaz: “Köşe komşu köşesinin yerine taşınamaz; köşeyi kaldırmak için satırı silin.”
- **Z:** köşenin kotu; boş Z kotu kaldırır.
- **Yarıçap:** köşeden çıkan kenarı yay yapar ya da yayı değiştirir.
  - Boş ya da 0 kenarı düz yapar.
  - |R| kirişin yarısından küçük olamaz: “Yarıçap kirişin yarısından küçük olamaz: en az 12.346 m.” En az değer projenin
    ondalıklarına yukarı yuvarlanır; yazılan değer kirişin yarısından en çok bir gösterim biriminin yarısı kadar küçükse kenar yarım
    daire olur (görülen en az değer yazılınca yarım daire kabul edilir).
  - İşaret yayın yönüdür (§2). Yayın büyüklüğü korunur: yarım daireden büyük yay büyük kalır; düz kenar küçük yay olur.
  - Çizginin (line) kenarı yay olamaz; hücresi düzenlenmez. Yaylı yol için önce köşe eklenir: çizgi çoklu çizgi olur (§5).
- **Kilitli katman** tabloyu baştan kapatır (§1); komutun reddi yine de söylenir.

### 5. Satır ekleme ve silme

- **Satır ekle:** seçili son satırın (tablonun sırasıyla) altında taslak satır açılır; seçili satır yoksa tablonun sonunda.
  - Taslağın Y, X ve Z'si yazılır; Tab ve Shift+Tab taslağın hücreleri arasında yazmadan gezer.
  - Enter, Y ve X geçerliyse köşeyi o satırın köşesi ile halkadaki sonraki köşe arasına ekler (çoklu çizginin ve çizginin son
    köşesinden sonra yolun sonuna). Adım “Köşe ekle” (`cad.entities.edit`, `vertexAdd` işlemi). Sonra yeni köşenin altında yeni bir
    taslak açılır: köşeler sırayla yazılır.
  - Bölünen kenar yaylıysa iki yeni kenar düzdür. Çizgiye köşe eklenince çizgi çoklu çizgi olur (Köşe ekle aracının kuralı, ADR 0047).
  - Z boşsa yeni köşe kotsuzdur.
  - Y ya da X eksikse Enter eksik olanı söyler (“Y ve X yazılmalı.”, “X yazılmalı.”) ve taslak kalır. Yeni köşe komşusunun yerindeyse
    §4'ün iletisiyle reddedilir.
  - Esc taslağı bırakır.
- **Sil:** seçili satırların köşeleri tek adımda silinir: “Köşe sil” (`cad.entities.edit`, `vertexRemove` işlemi). Delete tuşu,
  tablonun klavyesi tablodayken, aynı işi yapar.
  - Silinen köşenin iki kenarı tek düz kenar olur (Köşe sil aracının kuralı).
  - Her halkada en az üç köşe, çoklu çizgide en az iki köşe kalmalıdır: “Her halkada en az üç köşe kalmalı.”, “Çoklu çizgide en az iki
    köşe kalmalı.”; çizginin köşesi silinmez: “Çizginin iki ucu silinemez.” Reddedilen silmede hiçbir köşe silinmez.

### 6. Topolojik düzenleme

Durum çubuğundaki Topoloji açıkken (ADR 0160) tablonun yazmaları, tutamaç gibi, görünen ve kilitsiz komşuların ortak köşe ve kenarlarını
da aynı adımda değiştirir: köşe taşıma, yarıçap (ortak kenarın bükümü), köşe ekleme (ortak düz kenara) ve silme. Çekirdeğin
`topology_edit::changes` ve `apply`'ı kullanılır; ileti tutamacınkidir. Noktalar da seçeneği noktaları da taşır.

### 7. Hesap (ortak çekirdek)

`ops::vertex_table` (WASM `vertexTable…` çağrıları), nesnenin kotlu yolları üzerinde (`ops::elevation::Elevated`: yollar çekirdeğin
`paths` sırasıyla):

- **Satırlar:** her köşenin yolu ve yoldaki sırası, yeri, kotu; çıkan kenarın kirişi ve işaretli yarıçapı.
- **Yazmalar:** köşe taşıma, kot, yarıçap (büyüklüğü koruyarak, en az değerin payıyla), köşe ekleme (yaylı kenar ikiye düz), köşe silme
  (çoklu: halka başına en az), her biri yeni yolları ya da reddin nedenini verir (`missing`, `ontoNeighbour`, `noEdge`, `lineArc`,
  `noChord`, `radiusBelow` ve en az yarıçap, `lineEnds`, `pathMin`, `ringMin`). Nedeni tablo kendi sözleriyle söyler: en az yarıçap
  projenin biçimiyle yazılır (yerel projenin birimi, ADR 0165 §2), iki platformun sözleri ortak durumlarla denetlenir.
- **Yarıçaptan büküme** yalnız karekökle: s = c/(2|R|), küçük yay s/(1 + √(1 − s²)), büyük yay (1 + √(1 − s²))/s; iki platform aynı
  bitleri verir.
- **Bağımsız başvuru:** `scripts/fixtures/vertex_table_cases.py`: taşıma, ekleme ve silme kesirlerle, yarıçap ile büküm arası mpmath'le
  (50 basamak); `fixtures/vertex-table/v1/cases.json`. Arayüzün yazmaları (adım adları, iletiler, çizim) `edits.json`'da, iki platform
  aynı sonuçları verir.

### 8. Komutlar

Yeni sözleşme yoktur:

| İş | Komut |
|---|---|
| Y, X, Z, Yarıçap | `cad.entities.edit` (`properties`), adım “Köşe düzenle” |
| Satır ekle | `cad.entities.edit` (`vertexAdd`), adım “Köşe ekle” |
| Sil | `cad.entities.edit` (`vertexRemove`), adım “Köşe sil” |

Python ve MCP aynısını nesnenin geometrisini yazarak yapar.

### 9. Kapsam dışı

- Başka türlerin tanım noktaları (dairenin merkezi ve yarıçapı, elipsin eksenleri, eğrinin kontrol noktaları): Öznitelikler'dedir.
- Halka ekleme ve silme, delik doldurma (`HYB-16`), parça ekleme ve ayırma (`HYB-17`).
- Kayıtlı ölçüler ve COGO sütunları (`HYB-23`).
- M değeri (KentOS'ta yoktur).

## Adımlar

1. Çekirdek: `ops::vertex_table` (satırlar, taşıma, kot, yarıçap, ekleme, silme, retler), WASM çağrıları ve web'in cephesi
   (`model/ops/vertexTable.ts`); bağımsız başvuru `vertex_table_cases.py` (`fixtures/vertex-table/v1/cases.json`, 5 nesnenin satırları,
   48 yazma). Bitti (5 Ekim).
2. Tablo iki platformda: düzenlenen Koordinat listesi, Halka ve Yarıçap sütunları, satır seçimi ve çizimdeki vurgu, Göster, yerinde
   düzenleme (Y, X, Z, Yarıçap; “Köşe düzenle”), Satır ekle (“Köşe ekle”) ve Sil (Delete; “Köşe sil”); web `ui/bottom/VertexTable.ts`,
   `vertexEdit.ts`, masaüstü `vertices/`; ortak durumlar `edits.json` (`vertex_edit_cases.py`, 53 durum); resimler `kose-tablosu-*`
   (masaüstü `vertices::tests::screens`, web `shots.mjs vertextable`), sahne `fixtures/interaction/v1/vertex-table.kcad`. Bitti (5 Ekim).
3. Topoloji açıkken komşular (§6; masaüstü `kentos_interaction::neighbours::neighbours_in`, web `neighboursOf`; sahneye 12'nin ilk
   kenarını paylaşan 13 numaralı parsel), `view.coords`'un takma adları (KOSETABLOSU, KT) ve açıklaması. Kullanım senaryosu
   yerine: izlerin dilbilgisi alt paneli sürmez; tablo iki platformda resimlerle (web'de gerçek fare ve klavyeyle) sınanır. Bitti
   (5 Ekim).

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Çekirdek: bağımsız başvurunun durumları iki platformda (Rust ve WASM aynı bitler); yarıçap ile büküm arası 1e-12 göreli.
- Arayüz: ortak durumlar iki platformda; masaüstünde ve web'de fareyle ve klavyeyle kullanılarak resimlenir (1440×900 ve 1100×650, iki
  tema).
