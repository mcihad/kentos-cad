# 4480 — Onaydan sonra geometrisi değişen nesnenin onaylanan şeklinin çizimde gösterilmesi

Tarih: 2026-11-10. plan/map-cad.md MAP-01, dokuzuncu tur. Durum: taslak. ADR 4080'in “Sonra”sındaki ve MAP-01'in
açığındaki “geometri farkının çizimde gösterilmesi”.

İlgili:
- [ADR 3280](3280-map-work-changed-object-parts.md): değişen nesnenin parçaları (geometri, öznitelik, katman, görünüş);
- [ADR 3680](3680-map-work-approved-values.md): onayın nesnelerin değerlerini tutması;
- [ADR 4080](4080-map-work-restore-as-approved.md): Onaylı hâline getir.

## Dayanak

- **ArcGIS Pro, dal sürümünün değişiklikleri**
  ([View branch version changes](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/version-changes-branch.html),
  10 Kasım 2026'da okundu):
  - farklar görünüşünün altındaki **Change Display** görüntüleyicisi seçili nesnenin geometrisine yapılan değişikliği
    haritada karşılaştırmaya yarar;
  - görüntüleyici nesneyi haritalarda gösterir; gösterilecek hâl **Current**, **Target** ya da **Common ancestor**
    (nesnenin düzenlenmeden önceki hâli) olarak seçilir;
  - haritalar ayrı ayrı yakınlaştırılır, seçili nesneye gidilir.
- **ADR 3280'in okuması** (ArcGIS'in sürüm farkları): öznitelik farkı tabloda, geometri farkı haritada.
- **KentOS bugün** (ADR 3280, 3680): onay nesnelerin değerlerini tutar; Değişenler değişen nesneleri seçer ve hangi
  parçalarının değiştiğini söyler, öznitelikleri adıyla ve iki değeriyle verir. Geometrisi değişen nesnenin eski şekli
  ne söyleniyor ne gösteriliyordu: ADR 3680'in sınırı.

## Karar

1. **`map.work.query`'nin `changes.objects[]`'ine `approved`** (komut sürümü değişmez): geometrisi değişen nesnenin
   onaylanan hâli (anlık görüntü biçimi, kimliksiz), onay nesnelerin değerlerini tuttuysa (şema 213). Geometrisi
   değişmeyen nesnede yazılmaz (öznitelikleri adıyla zaten söylenir).
2. **Masaüstü** (Harita işi › Değişenler): değişen ve eklenen nesneleri seçmenin yanında geometrisi değişenlerin
   onaylanan şekli çizimin üstünde **kesik çizgiyle**, temanın uyarı rengiyle çizilir (ArcGIS'in “Common ancestor”
   gösteriminin karşılığı; şimdiki şekil seçim olarak vurgulanır):
   - şekil çekirdeğin kenarlarıdır: yay kısa kirişlerle (bir resmin toleransı: yarıçapın binde biri; ölçü değil), nokta
     küçük bir halkayla;
   - çizim Değişenler'in yaptığı seçim durdukça kalır; başka seçim onu kaldırır;
   - durum çubuğu: “… Geometrisi değişen 1 nesnenin onaylanan şekli kesik çizgiyle gösteriliyor.”.
3. **Uygulama katmanı:** onaylanan değeri nesneye çeviren işlev (`map_work_restore::approved_entity`) Onaylı hâline
   getir'inkiyle aynıdır.
4. **Kapsam işareti:** ADR 2880, 3280 ve 3680'deki gibi **yalnız Harita (CAD)**.

## Sınırlar

- Hangi köşenin ne kadar kaydığı söylenmez (sayısal fark yok); iki şekil yan yana gösterilir.
- Silinen nesnenin onaylanan şekli çizilmez (ADR 3680: durum çubuğu ve ipucu onu söyler; Onaylı hâline getir geri
  getirir).
- Şekiller seçimle birlikte yaşar; ayrı bir “göster” anahtarı yoktur.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/map_work_cases.py`; KentOS kodu yok), 2 yeni query durumu (38):
  - onaydan sonra taşınan çizgi: parçası geometri, onaylanan hâli onayın değeri;
  - hem taşınan hem kotu değişen çizgi: öznitelik adıyla, şekil de.

  Eski durumlar (query 36, edit 57, restore 28) değişmedi. Rust ve Python geçer. Bozma denemesi: şekil her değişen
  nesnede verilince yalnız özniteliği değişen durumun beklenmeyen alanını koşucu yakalar.
- **Python** (`python/tests/test_map_work_approved_shape.py`), bağımsız `.kcad` okuyucusuyla: taşınan çizginin şekli
  okuyucunun okuduğu onay değeri ve taşınmadan önceki uçları; yalnız kot değişince şekil yok; ikisi birden.
- **MCP** (`crates/native/mcp/tests/map_work_approved_shape.rs`): taşınan çizginin şeklinin uçları örnek dosyanınki;
  yalnız kotta şekil yok.
- **Masaüstü** (`map_work_geometry_tests`): Değişenler iki değişen çizgiyi seçer; yalnız taşınanın onaylanan şekli
  çizilecek, uçları onaydaki; durum çubuğu söyler; başka seçim şekli kaldırır. Harita işi testleri ve çapraz denetimler
  geçer. Resimler `.run/gui-evidence/onayli-sekil-4480/`.

## Sonra

- Köşe köşe kayma (en büyük kayma, Hausdorff uzaklığı) ve tabloda gösterilmesi.
- Silinen nesnenin onaylanan şeklinin de çizilmesi.
- Tipli katman profili (MAP-01'in öbür açığı).
- Web (TODOS §0.2).
