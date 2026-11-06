# ADR 1126: Adlandırılmış tablo stili ve ofis standardının tablo stilleri

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` maddesinin “paket … tablo stili …
  taşısın” satırı ve docs/specs/drawing-standard-package.md'nin açık bıraktığı “table style content”.
- **Bağlam belgesi:** ADR 0184 (tablo nesnesi), 0327 (hücre stilleri), 0183 (yazı stili ve yüz), 1002 (standart
  kaynaklarını kur), 1100 (blok ve pafta şablonu satırları), 0306 (denetle ve temizle).

## 1. Araştırma

- Autodesk, [New and Modify Table Style Dialog Boxes](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT/files/GUID-4DCAAF7A-27BF-40A8-B363-41C2ED4E4ABE.htm)
  ve [About Table Styles](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT-MAC/files/GUID-CC7A6033-995B-411B-9F2E-4E6A8B5A95BF.htm):
  Standard tablo stilinin üç hücre stili vardır: Title, Header (Label türü) ve Data. Tablo yönü Down/Up (başlık ve
  sütun başı üstte ya da altta). Hücre stilinin General sekmesi: dolgu rengi (varsayılan yok), hizalama (üst/orta/alt
  × sol/orta/sağ), biçim, tür, yatay ve dikey kenar boşluğu (metrik varsayılan 1,5), satır/sütun eklerken birleştir.
  Text sekmesi: yazı stili, yükseklik (veri ve sütun başı 0,18, başlık 0,25 inç), renk, açı. Borders sekmesi: kenar
  kalınlığı, çizgi tipi, renk, çift çizgi ve aralığı.
- Autodesk, [DXF TABLESTYLE](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-0DBCA057-9F6C-4DEB-A66F-8A9B3C62FB1A.htm):
  3 açıklama, 70 akış yönü (0 aşağı, 1 yukarı), 71 bayraklar, 40/41 yatay/dikey hücre kenar boşluğu, 280/281 başlığın
  ve sütun başının bastırılması; hücre başına tekrar eden 7 yazı stili adı, 140 yazı yüksekliği, 170 hizalama, 62 yazı
  rengi, 63/283 dolgu rengi ve açık mı, 90/91 veri ve birim türü, 274–279 kenarların kalınlığı, 284–289 görünürlüğü,
  64–69 renkleri.
- KentOS'ta bugün: tablonun tek yazı yüksekliği (`height`, metre), tek yüzü (`TextFace`, yazı stili kimliği ile), sütun
  hizaları, ilk satırın başlık (kalın, ortalı) olması, aralık başına hücre stilleri (hizalama, kalın, renk, dolgu,
  ondalık; ADR 0327) ve sabit hücre iç boşluğu (`PAD · h`) var; adlandırılmış tablo stili yok. `TableStyle` adı
  öznitelik tablosunun koşullu biçimine (ADR 0413) ait; yeni tür `TableStyleDef`'tir.

## 2. Karar: model

- `ProjectSettings.tableStyles: TableStyleDef[]` — `id`, `name` (yazı stillerinin ad kuralı: boş değil, kenarı
  boşluksuz, en çok 64 harf, denetim karakteri yok, Standart'ın değil, harf büyüklüğü dışında bir kez), `title`,
  `header`, `data` (her biri isteğe bağlı `TableCellStyleDef`).
- `TableCellStyleDef` (hepsi isteğe bağlı, kâğıt mm): `textStyle` (yazı stilinin **adı**, grup 7; 1–255 bayt),
  `height` (0 < h ≤ 100, grup 140), `align` (sol/orta/sağ; grup 170'in yatay kısmı), `color` ve `fill` (`#rrggbb`,
  62/63), `horizontalMargin`, `verticalMargin` (0–100, 40/41), `borderWeight` (0–100, 274–279), `borderColor`
  (64–69). Bilinmeyen alan reddedilir (`deny_unknown_fields`), sessizce atılmaz.
- `TableEntity.tableStyle`: tablonun izlediği stilin kimliği.
- Satır türleri (`table_style_rows`): ilk satır bütün sütunları kaplayan tek birleşmiş aralıksa (birden çok sütunda)
  **başlık**; ardından tablonun `header`'ı varsa ilk satır **sütun başı**; gerisi **veri**.
- `.kcad` şema **185** (`SCHEMA_WITH_TABLE_STYLES`, geçici numara, birleştirmede yeniden numaralanır):
  `settings.tableStyles` (JSON olarak, boş liste yazılmaz, `table_style_defs_problem` ile denetlenir; anahtar
  `sourceBases`'tan sonra, kurallı sırada) ve tablonun `tableStyle`'ı (boş olmayan metin). 184 ikisini de
  `unknown_field` sayar. Bağımsız `tools/kcad/kcad.py` ve `scripts/fixtures/kcad_v2_reference.py` aynı kuralla;
  örnekler `table-styles.json/.kcad`, `broken/table-styles-{in-schema-184,unknown-field,duplicate,height,colour,
  empty-ref}.kcad`; en yeni bozuk şema örneği `schema-version-186.kcad`. Sütun aktarımı (`columns.rs`) `tableStyle`'ı
  tablonun JSON yan kanalında taşır. Geri al/yinele: `StyleTables` tablo stillerini de stil stil tutar.

## 3. Karar: çizim

`kentos_contracts::styled(t, settings)` tabloyu stilinin çizdiği gibi verir; tablonun kendi belirlediği her şey kalır:

- Her satır türü için stilin hücre stilinden hizalama, yazı rengi ve dolgu, tablonun **kendi hücre stillerinin
  altında** (önce gelir; ADR 0327'de sonraki öncekinin üstündedir). Veri satırlarının hizası yalnız tablonun sütun
  hizası yoksa; başlık ve sütun başı hizası tablonun örtük “başlık ortalı” kuralının yerine geçer.
- Tablonun yazı stili ve yüzü yoksa, veri hücresinin yazı stili (adıyla, harf büyüklüğü dışında) tablonun yüzü olur
  (yazı stilinin kimliği, yazı tipi, kalın, eğik, eğiklik).
- Uygulandığı yer: depo kaydı (`viewport_scope::record_at` → `geometry::core_table`; ekran, seçim, GPU stil derlemesi
  ve PDF vektörü buradan), ekranın ve paftanın hücre yazıları ve dolguları (`labels.rs`). Stil değişince masaüstü
  hücre bandı önbelleği stili de anahtar sayar.
- Stili projede olmayan tablo kendisidir (bulgu yok; standart denetimi stili söyler).

## 4. Kapsam dışı (açık)

- Hücre türüne göre yazı yüksekliği, kenar boşlukları, kenar çizgisi kalınlığı ve rengi **saklanır, aktarılır,
  standartla karşılaştırılır ama çizilmez**: tablonun tek `height`'ı ve sabit `PAD`'i sürer, çizgiler katmanın
  görünüşündedir.
- Tablo yönü (Up), hücre türü/biçimi (90/91), dikey hizalama, yazı açısı, kenar görünürlüğü ve çizgi tipi, çift çizgi,
  “oluştururken birleştir” modelde yok.
- DXF/DWG TABLESTYLE okuma ve yazma yok; tablo stili DXF'e aktarılmaz.
- Tablo stili penceresi (oluştur/düzenle) ve `cad.styles.edit`'e tablo türü yok; stil bugün `.kcad`, standart kurulumu
  ve Python/MCP'nin `cad.standards.install`'ı ile gelir. Tabloya stil atamak `cad.entities.edit`/`set` ile değil, dosya
  ya da kurulumla; ayrı komut sonraki iştir.

## 5. Temizle

`cad.drawing.audit`'in `purge`'ı `tableStyles` alır (`AuditPurge::TableStyles`, `ResourceKind::TableStyle`): hiçbir
tablonun (çizimde ya da blok tanımında) izlemediği tablo stilleri `unused` listesindedir ve istenince silinir; bir
tablonun izlediği stil hiçbir zaman. Bir tablo stilinin hücresinin adlandırdığı yazı stili kullanılıyor sayılır,
yazı stili temizliği onu silmez.

## 6. Ofis standardı

- Paket v2 `resources.tableStyleDefinitions` (spec'in alan listesi; her stil ve hücre stili alanı da izin listesinde).
- `cad.standards.resolve`: bağlama sırasında adın ilk tanımı, `provenance.tableStyleDefinitions` (boşken yazılmaz).
- `cad.standards.conflicts`: aynı adlı farklı stil `tableStyleDefinition`; alanlar değişen hücre stilleri (`title`,
  `header`, `data`), değerleri, olmayan `null`; sayılar 1e-9 içinde eşit.
- `cad.standards.check`: eksik stil `missingTableStyleDefinition`, farklı stil `tableStyleDefinition` (düzeltilemez;
  `expected`/`found` görünüşün JSON'u; onu izleyen tablolar `objects`'te, blok parçaları iletide).
- `cad.standards.install`: satır türü `tableStyle` (`missing/same/differs`, `add/replace/keepLocal/renameIncoming`);
  `replace` yerel ad ve kimliği korur, tablolar izler; eklenen stil kimliğini korur ya da `-2`, `-3` alır. Hiçbir şey
  silinmez. Tek geri alma adımı. Masaüstü kurulum penceresi “Tablo stili” satırlarını gösterir.
- Paylaşılan durumlar (bağımsız başvuru `scripts/fixtures/standards_table_styles_cases.py`, KentOS kodu yok):
  `cad.standards.install.tables.json` 26, `cad.standards.check.tables.json` 10, `cad.standards.conflicts.tables.json` 4.

## 7. Kanıt

- `kentos-contracts` `table_style_defs`: satır türleri (başlık birleşmiş ilk satır, sütun başı, veri), stilin tablonun
  kendisinin altında kalması (kendi hücre stili, sütun hizaları, yüzü), alan alan doğrulama ve bilinmeyen alan reddi.
- `kentos-kcad` `--test all`: 242/242 (geçerli örnek 201 → 202; şema 185 örneği ve 184'ün reddi; sütun aktarımı
  baytları). `kcad_v2_reference.py --check`: 1603 dosya tutarlı (bağımsız okuyucu `tools/kcad/kcad.py`).
- `kentos-domain` `audit::table_style_tests`: kullanılan tablo stili ve onun adlandırdığı yazı stili temizlenmez.
- Komut durumları (`KENTOS_CASES_ONLY=cad.standards.install,check,conflicts,resolve,cad.drawing.audit`): eski ve yeni
  hepsi geçti; yeni 71 durum (kurulum 26, denetim 25, fark 20) `standards_table_styles_cases.py`'den.
- `kentos-plot` `table_styles`: paftada stilin başlık/sütun başı dolgusu ve yazı renkleri, tablonun kendi hücre stili
  üstte; stilsiz tablo eskisi gibi; PDF üretilir.
- Python (`python/tests/test_standard_table_styles.py`, yeniden derlenmiş SDK): kurulum planı ve yürütme, yerinde
  değiştirme sonrası tablonun stil kimliği, `ofis-baslik-2`, tek geri alma; temizlikte yalnız kullanılmayan YEREL.
- Masaüstü resimleri (`.run/shots/arac-standart-tablo-{kur,kuruldu}-*.png`, repoya alınmaz): kurulum penceresinde dört
  “Tablo stili” satırı (farklı CETVEL, eksik BASLIK ve PLAN-LISTE, aynı sade; kaynakları ofis-a/ofis-b); kurulumdan sonra
  CETVEL'i izleyen tablonun sütun başı dolgulu, veri hücreleri sağa hizalı, stilsiz tablo değişmemiş (piksel ölçümüyle:
  başlık hücresi 15,20,25 → 80,84,89).
