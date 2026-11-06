# ADR 1127: Tablo stilinin çizilmesi, düzenlenmesi ve tablonun stili

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` tablo stilinin kalanı; ADR 1126'nın
  §4'te “saklanır, henüz çizilmez” dediği yükseklik, kenar boşlukları ve kenar çizgileri; `cad.styles.edit`'in tablo
  stili türü; tablonun stilinin `cad.entities.edit` ile verilmesi; masaüstü penceresi ve özellik satırı.
- **Bağlam belgesi:** ADR 1126 (adlandırılmış tablo stili), 0184 (tablo nesnesi), 0327 (hücre stilleri), 0248 §4
  (`cad.styles.edit`), 0183 (yazı stilleri), 0047 (`cad.entities.edit`), DESIGN.md §7.19, ADR 0938 (kitin veri tablosu).

## 1. Araştırma

- Autodesk, [New and Modify Table Style Dialog Boxes](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT/files/GUID-4DCAAF7A-27BF-40A8-B363-41C2ED4E4ABE.htm):
  yatay kenar boşluğu yazıyla hücrenin sol ve sağ kenarı, dikey kenar boşluğu üst ve alt kenarı arasındaki
  uzaklıktır (metrik varsayılan 1,5); yükseklik hücre stilinin yazı yüksekliğidir; Borders sekmesinde kalınlık ve renk
  seçilen kenarlara (All, Outside, Inside, Bottom, Left, Top, Right) uygulanır; dikey hizalama üst/orta/alt.
- Autodesk, [DXF TABLESTYLE](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-0DBCA057-9F6C-4DEB-A66F-8A9B3C62FB1A.htm)
  (ADR 1126 §1): hücre başına 140 yükseklik, 40/41 kenar boşlukları, 274–279 kenar kalınlıkları, 64–69 renkleri.
- KentOS'ta: satırların derinliği tablonundur (`rows`), yazılar hücrenin dikey ortasında durur (ADR 0184 §2); hücre
  stilinin yalnız yatay hizası vardır (ADR 1126 §2). AutoCAD'in kenar başına (üst, alt, sol, sağ, iç, dış) ayrı
  özelliği KentOS'ta hücre türü başına tek bakıştır.

## 2. Karar: `cad.styles.edit` tablo stili türü

- `AnnotationStyleKind::Table` (`kind: "table"`), tanım `StylesEdit.table: TableStyleDef`. create, update, rename,
  remove, export, import öbür türler gibidir; denetim `table_style_defs_problem` (ad kuralı, sonra her stilin hücre
  stilleri başlık, sütun başı, veri sırasıyla). İleti öneki “Tablo stilleri:”.
- Tablo stilin çizdiğini kendi taşımaz: update ve rename tablo yazmaz (`followed` 0). remove, stili izleyen çizim
  tablolarının bağını (`tableStyle`) tek adımda (“Tablo stili”) kaldırır; kilitli katmandakiler kalır ve `locked`'ta
  sayılır. Blok tanımındaki tablo silinen stili anarsa kendisi olarak çizilir, blok değişmez.
- Yazı stili yeniden adlandırılınca tablo stillerinin hücre stillerindeki `textStyle` adı (harf büyüklüğü dışında
  eşleşen) yeni adı alır; çizgi tiplerinin yazılarındaki gibi.
- Ortak durumlar: `fixtures/commands/v1/cad.styles.edit.tables.json`, 37 durum; bağımsız başvuru
  `scripts/fixtures/table_style_edit_cases.py` (ad ve değer kuralları ADR'den elle yazılır).

## 3. Karar: tablonun stili `cad.entities.edit` ile

- Tablo geometrisine `tableStyle` (`Option<Option<String>>`, `nullable`): yok, tablonunki kalır; null, kaldırılır;
  kimlik, projenin tablo stili. Projede olmayan kimlik `unknown_style`, yolu `changes[i].geometry.tableStyle`
  (oluşturmada da). Kilitli katman bütün düzenlemeyi reddeder; tek adım işlemin adıyla (Öznitelikler: Değiştir).
- Gidiş-dönüş: `edit_geometry_of` tablonun stilini geometriye yazar; `edit_geometry` (çekirdeğin şeklinden) yazmaz ve
  güncelleme tablonunkini korur. Eskiden Tabloyu düzenle, Yazı stili takibi ve her `set_geometries` yolu tablonun
  stilini düşürüyordu.
- Ortak durumlar: `fixtures/commands/v1/cad.entities.edit.tablestyle.json`, 20 durum (aynı başvuru). Nesneyi
  değiştirmeyen düzenleme adım yazmaz; plan nesneleri olacakları gibi verir.

## 4. Karar: çizim (yükseklik, kenar boşlukları, kenar çizgileri)

- `geometry::styled_table(t, settings)`: `styled`'ın tablosu, üstüne her satır türüne (`table_style_rows`) hücre
  stilinin yüksekliği, yatay ve dikey boşluğu (kâğıt mm × proje ölçeği / 1000 metre) ve kenar kalınlığı (kâğıt mm) ile
  rengi, tablonun kendi hücre stillerinin altında. Depo kaydı (`viewport_scope::record_at`), masaüstü ve PDF hücre
  yazıları bunu kullanır.
- Çekirdek (`geom/table.rs`, `CellLook`'a `height`, `margin`, `vmargin`, `borderWeight`, `borderColor`): hücrenin
  yazısı kendi yüksekliğinde, `PAD · h` yerine yatay boşluk kadar içeride; taban çizgisi yine kutunun ortasının 0,35
  yükseklik altında (KentOS dikeyde ortalar; dikey boşluk yerleşimi değiştirmez, Sığmayan hücreler denetiminde
  üstte ve altta sayılır). Çizgi, üstündeki hücrenin (yatay; üst çizgi ilk satırın) ya da solundakinin (dikey; sol
  çizgi ilk sütunun) bakışını alır; bir ızgara çizgisindeki kenarlar bakış değişince ayrı çizgi olur. Bakışlı
  çizgiler `borders()`, öbürleri `plain_lines()`; `lines()` ikisi (kenet ve kenarlar için).
- Depo kaydı tablonun düz çizgilerini taşır; stil derlemesi (`style-core build.rs` `table_borders`) kenarları
  tablonun çizgi sembolüyle, rengini ve kâğıttaki kalınlığını değiştirerek çizer (ekran GPU'su, PDF vektörü, CPU
  resmi aynı yoldan). Rengi verilmeyen kenar tablonun rengi, kalınlığı verilmeyen tablonun kalınlığıdır; hücre
  stilinin yazı rengi gibi stilin kenarı nesnenin rengini ezer (ADR 1126 §3).
- Stili olmayan tablo, ya da hücre stillerinden hiçbiri bunları vermeyen tablo, çizgi çizgi ve bit bit eskisi
  gibidir (bakış hesaplanmaz; `fixtures/table/v1` çekirdek durumları değişmeden geçer).
- Kanıt: `crates/native/plot/tests/all/table_styles.rs` `a_table_prints_its_styles_heights_margins_and_borders_at_two_scales`,
  1:100 ve 1:500'de: yazıların yeri ve kâğıttaki boyu, her düz çizgi ve her kenarın kalınlığı ile rengi, başka çizgi
  olmaması, CPU resminde kenar renkleri ve stilsiz aynı tablonun eski hâli. Bağımsız başvuru
  `scripts/fixtures/table_style_plot_cases.py` → `fixtures/table/v1/styled-plot.json` (KentOS kodu yok).

## 5. Karar: masaüstü

- `table_style_window.rs` — Tablo stilleri (komut `style.tableStyles`, takma adlar TABLOSTILI, TABLESTYLE, TS;
  Açıklama sekmesinin Tablo grubu), DESIGN.md §7.19 iskeleti (`RoadDialog`): formun üstünde kitin tablosunda
  stiller ve izleyen tablo sayısı; formda ad, hücre türü seçimi (Başlık | Sütun başı | Veri) ve türün alanları
  (yazı stili, hizalama, yükseklik, boşluklar, kenar kalınlığı ve rengi, yazı rengi, dolgu; boş: tablonun); sağda
  örnek tablo çizimin çizdiği gibi ve üç hücre stili yan yana. Kaydet `cad.styles.edit` çağrılarını tek adımda
  (“Tablo stilleri”) yazar.
- Özellikler panelinde tablonun **Tablo stili** satırı: Yok, projenin stilleri ve “Tablo stilleri…”; seçim
  `apply_table_style` → `cad.entities.edit` (properties).
- Kurulum penceresinin Kullanım sütunu tablo stili satırında “N tablo” der; `StandardInstallRow.uses` çizimin ve blok
  tanımlarının o stili izleyen tablolarını sayar.

## 6. Açık kalanlar

- DXF TABLESTYLE alma ve verme (grup kodları §1'de); bu pakette yapılmadı.
- Dikey hizalama (üst/alt) ve buna bağlı dikey boşluğun yerleşime etkisi; satırın yazıya göre büyümesi.
- Kenar başına (dış, iç, alt …) ayrı bakış; çift çizgi ve kenar çizgi tipi.
- Web (ADR 0228): çekirdeğin `CellLook` alanları ve `plain_lines`/`borders` WASM'a açılmadı (TODOS.md §0.2).
