# 3280 — Onaydan sonra değişen nesnenin neyinin değiştiği

Tarih: 2026-11-09. plan/map-cad.md MAP-01, altıncı tur. Durum: taslak. ADR 2880'in “Sonra”sındaki “değişen nesnenin
neyinin değiştiği (geometri mi, öznitelik mi): bugün yalnız özeti farklıdır”.

İlgili:
- [ADR 2080](2080-map-work-content-revision-and-approval-stamp.md): içerik revizyonu ve onay damgası;
- [ADR 2880](2880-map-work-objects-changed-since-approval.md): onaydan sonra değişen nesneler (`approvedObjects`).

## Dayanak

- **ArcGIS Pro, sürüm farkları**
  ([View traditional version changes](https://pro.arcgis.com/en/pro-app/latest/help/data/geodatabases/overview/using-the-version-changes-command.htm),
  [View branch version changes](https://pro.arcgis.com/en/pro-app/3.4/help/data/geodatabases/overview/version-changes-branch.htm),
  9 Kasım 2026'da okundu):
  - değişiklikler eklenen, güncellenen ve silinen olarak ayrılır;
  - güncellenen nesnede farklı öznitelik değerleri ayrı gösterilir; geometri değişikliği haritada ayrıca
    karşılaştırılır;
  - kullanıcı neyin değiştiğini nesne nesne görür: öznitelik mi, şekil mi.
- **KentOS bugün** (ADR 2880): onay her nesnenin tek özetini tutar. Değişen nesnenin özeti farklıdır ama neyin
  değiştiği söylenmez. Bir yazım düzeltmesi ile köşe kayması aynı “değişti”dir.

## Karar

1. **Nesnenin dört parçası:** anlık görüntü biçiminin (`id`'siz) alanları adlarıyla dörde ayrılır:
   - **öznitelik:** `attrs`, `attachments`;
   - **katman:** `layerId`;
   - **görünüş:** `color`, `label`, `symbol`, `lineWeight`, `labelPins`, `lineType`, `linePattern`, `lineTypeScale`,
     `transparency`, `order`, `byLayer`, `wipeout`;
   - **geometri:** geri kalan her alan (`kind` ve türün kendi alanları: yer, köşeler, yaylar, kot, yazının metni…).

   Her parçanın özeti, alanlarından oluşan nesnenin boşluksuz, anahtarları sıralı JSON'unun SHA-256'sının ilk 16 küçük
   onaltılık hanesidir (`object_parts`). Alanları olmayan parça boş nesnedir (`{}`).
2. **`approvedParts`** (`WorkItem`, `.kcad` şema 211):
   - onay `approvedObjects`'i yazarken her nesnenin dört parçasını da yazar. Liste onunla bire birdir: aynı sayı, aynı
     sıra;
   - onay damgası değişmez: damga `approvedObjects`'in özetidir, parçalar onun yanında durur;
   - onaydan çıkan öğenin parçaları da kalkar.
3. **Doğrulama** (`mapWork.items[i].approvedParts`): sayısı `approvedObjects`'inkinden farklıysa ya da bir özet 16 küçük
   onaltılık haneden başkaysa belge reddedilir.
4. **Cevap** (`map.work.query`, `changes`'e ek alanlar; komut sürümü değişmez). Parçaları tutulmuş onayda:
   - `parts`: değişen nesnelerden kaçının geometrisi, öznitelikleri, katmanı, görünüşü değişti. Bir nesne birden çok
     parçada sayılabilir;
   - `objects`: her değişen nesne ve değişen parçaları (`geometry`, `attributes`, `layer`, `look` sırasıyla), kimliğe
     göre, en çok 1000.

   Parçaları tutulmamış onayda (şema 210) bu alanlar yoktur; yeniden onaylanınca yazılır.
5. **Masaüstü** (Harita işi paneli):
   - Değişenler'in ipucu: “Onaydan sonra 1 nesne değişti (öznitelik 1), 1 nesne eklendi, 1 nesne silindi.”;
   - satıra basınca durum çubuğu: “… 2 nesne seçildi; değişen: 1 nesnenin öznitelikleri; silinen 1 nesne çizimde yok.”
6. **`.kcad` şema 211:**
   - yazıcı bir öğe parçaları taşıyınca 211'i seçer;
   - 210 ve öncesinde alan bilinmez (`unknown_field`);
   - bire bir olmayan liste ve 16 haneden başka özet `bad_value`;
   - Rust okuyucusu ve bağımsız okuyucu aynı;
   - “daha yeni şema” dosyası `schema-version-212.kcad` oldu.
7. **Kapsam işareti:** ADR 2880'deki gibi **yalnız Harita (CAD)**.

## Sınırlar

- Silinen nesnenin onaylanan hâli saklanmaz; yalnız sayısı söylenir (ADR 2880 Sonra).
- Parça değişen alanın kendisini söylemez (hangi öznitelik). Bunun için onaylanan değerlerin kendisi gerekir.

## Kanıt

- **Sözleşme** (`kentos-contracts`, `tests/all/approved_objects.rs`):
  - bire bir liste geçer; eksik ya da fazla liste, büyük harfli, 15 ve 17 haneli özet, listesiz parçalar reddedilir;
  - yuvanın kimliği parça değildir; köşe geometri, ad öznitelik, katman katman, renk görünüş;
  - geometrinin özeti elle yazılan JSON'un SHA-256'sıdır.
- **`.kcad`:**
  - bağımsız başvuru yazıcısı (`scripts/fixtures/kcad_v2_reference.py`) ve bağımsız okuyucu (`tools/kcad/kcad.py`):
    1710 dosya, `expected.json`'daki her dosya aynı sonucu verir;
  - örnek `map-work-approved-parts.json/.kcad`, parçaları Python'da hesaplanmış; üç bozuk dosya (210'da alan, bire bir
    olmayan, 15 haneli);
  - Rust okuyucusu ve yazıcısı aynı (`kentos-kcad`, 242 örnek).
- **Ortak durumlar** (`scripts/fixtures/map_work_cases.py`, KentOS kodu yok; parçalar üreteçte ayrılır ve özetlenir):
  - değişen nesnenin geometrisi ve görünüşü;
  - onaydan sonra özniteliği değişen nesne (yalnız öznitelik);
  - onay parçaları yazar; parçasız onaylanmış değişmemiş öğe yeniden onaylanınca parçalarını kazanır; onaydan çıkış
    kaldırır;
  - `map.work.query` ve `map.work.edit` 90 durum. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/map_work_changes.rs`): şema 211 örneğinde çizginin özniteliği değişince
  `parts` ve `objects` yalnız özniteliği söyler; ADR 2880'in testi yeniden onaydan sonra 211'i kaydeder.
- **Masaüstü** (`map_work_changes_tests.rs`):
  - değişen çizginin parçası öznitelik; ipucunun cümlesi; durum çubuğu;
  - kaydedilen dosya şema 211;
  - resimler `.run/gui-evidence/onaydan-sonra-parcalar-3280/`.

  `map_work` testleri ve çapraz denetimler geçer.

## Sonra

- Değişen özniteliğin adı ve onaylanan değeri; silinen nesnenin onaylanan hâli (onaylanan değerlerin saklanması).
- Dosyası olan öğede nesne başına fark.
- Tipli katman profili (MAP-01'in kalanı).
- Web (TODOS §0.2).
