# 2880 — Harita işi: onaydan sonra değişen nesneler

Tarih: 2026-11-08. plan/map-cad.md MAP-01, beşinci tur. Durum: taslak. ADR 2080'in ve ADR 2480'in “Sonra”sındaki
“onaydan sonra hangi nesnelerin değiştiği (`.kcad`: onay damgasında nesne başına özet)”.

İlgili:
- [ADR 1200](1200-map-work-structure-and-provenance.md): Harita işi;
- [ADR 2080](2080-map-work-content-revision-and-approval-stamp.md): içerik revizyonu (`kentos.work-content/1`) ve onay
  damgası;
- [ADR 1320](1320-sheet-sources-and-update-publication.md) §2: nesne özeti `kentos.object-digest/1`.

## Dayanak

- **ArcGIS Pro, View branch version changes**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/version-changes-branch.html),
  8 Kasım 2026'da okundu):
  - Differences listesi katman katmandır, her katmanın yanında fark sayısı yazar;
  - her katmanın altında farklar üç türdür: eklenen, silinen, güncellenen nesne;
  - bir nesne kimliğinin menüsünden nesne haritada parlatılır, yakınlaştırılır ya da seçilir;
  - farklar iki sürümün aynı olduğu andan beri yapılan değişikliklerdir. Ortak ata, nesnenin düzenlenmeden önceki
    hâlidir.
- **AutoCAD, DWG Compare**
  ([To Compare Drawings](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-LT-MAC/files/GUID-65A932C1-0599-47BB-863A-76EA496573F6.htm),
  8 Kasım 2026'da okundu):
  - yalnız güncel çizimde olan, yalnız karşılaştırılan çizimde olan ve ikisinde ortak nesneler ayrı renklerle
    gösterilir;
  - farklar yakınlıklarına göre değişiklik kümelerine toplanır, oklarla birinden ötekine gidilir;
  - belgede ayrı bir “özelliği değişmiş” türü geçmiyor.

  KentOS üç türü ArcGIS'teki gibi ayırır: eklenen, silinen, değişen. Değişen nesne de seçilir.
- **KentOS bugün** (ADR 2080):
  - onaylanan dosyasız öğe içerik revizyonunu damgalar. Revizyon, öğenin nesnelerinin
    `[[kalıcı kimlik, nesne özeti], …]` listesinin özetidir;
  - içerik sonra değişince öğe “Onay eskidi” olur;
  - hangi nesnelerin değiştiği söylenemiyordu: damga tek özettir, liste saklanmıyordu.

## Karar

1. **`approvedObjects`** (`WorkItem`, isteğe bağlı): onayın damgaladığı liste, `[[kalıcı kimlik, nesne özeti], …]`,
   kimliğe göre sıralı.
   - Dosyası olmayan öğe onaylanınca yazılır. Listenin boşluksuz JSON'unun SHA-256'sı damganın kendisidir
     (`approved`), yani liste damgayla doğrulanır.
   - En çok 100 000 çift (`MAX_APPROVED_OBJECTS`). Daha büyük öğede liste yazılmaz, yalnız damga (ADR 2080'deki gibi).
   - Yeniden onay listeyi yeniler. Onaydan çıkan öğenin damgası ve listesi birlikte kalkar.
   - Dosyası olan öğede liste yoktur: onayı dosyanın revizyonudur.
   - Onay bir düzenleme adımıdır, geri alınır.
2. **Doğrulama** (`mapWork.items[i].approvedObjects`): liste yalnız damgalı öğede olur; dosyası olan öğede olmaz; en
   çok 100 000 çifttir; kimlik boş değildir, özet 64 küçük onaltılık hanedir; kimlikler kesin artandır (tekrarsız);
   listenin özeti damgadır. Değilse belge reddedilir.
3. **Cevap** (`map.work.query`, ek alan; komut sürümü değişmez): onayı eskimiş (`revised`), dosyasız ve listeli öğenin
   satırında `changes`:
   - bugünkü liste onaylanan listeyle kimlikle karşılaştırılır:
     - yalnız bugünkünde olan `added`;
     - yalnız onaylananda olan `removed`;
     - ikisinde olup özeti farklı olan `changed`;
   - `uids`: bugün çizimde olanların (eklenen ve değişen) kimlikleri, kimliğe göre, en çok 1000
     (`WORK_CHANGES_LISTED`);
   - listesi olmayan öğede `changes` yoktur: şema 210'dan önce onaylanmış ya da 100 000'den büyük öğe. Yeniden
     onaylanınca liste yazılır;
   - bugünkü liste içerik revizyonu için zaten çıkarılıyor; karşılaştırma tek geçişlik birleştirmedir.
4. **Masaüstü** (Harita işi paneli):
   - onayı eskimiş öğenin altında **Değişenler** satırı. Hapı nesne sayısıdır, bilgi tonunda: öğenin kendi satırı
     zaten bir sorun sayılır, ikinci kez sayılmaz;
   - ipucu: “Onaydan sonra 1 nesne değişti, 1 nesne eklendi, 1 nesne silindi.” Öğenin ipucunda da aynı cümle;
   - satıra basınca değişen ve eklenen nesneler seçilir ve gösterilir. Silinenlerin çizimde olmadığı söylenir;
   - ad kısadır: dar panelde öğenin altında kesilmeden okunur (ilk denemede “Onaydan sonra değişenler” kesildi).
5. **`.kcad` şema 210:**
   - yazıcı bir öğe listeyi taşıyınca 210'u seçer;
   - 209 ve öncesinde alan bilinmez (`unknown_field`);
   - damgayı vermeyen liste, dosyası olan öğedeki liste ve tekrarlı liste `bad_value`;
   - Rust okuyucusu ve bağımsız okuyucu aynı;
   - “daha yeni şema” dosyası `schema-version-211.kcad` oldu.
6. **Kapsam işareti:** ADR 2080'deki gibi **yalnız Harita (CAD)**: Harita işi `cadWork: map` projesinin ayarıdır.

## Kanıt

- **Sözleşme** (`kentos-contracts`, `tests/all/approved_objects.rs`): geçerli liste ve her ret: damgasız, dosyalı,
  sırasız, tekrarlı, boş kimlik, büyük harfli ya da kısa özet, damgayı vermeyen, 100 001 çift.
- **`.kcad`:**
  - bağımsız başvuru yazıcısı (`scripts/fixtures/kcad_v2_reference.py`) ve bağımsız okuyucu (`tools/kcad/kcad.py`):
    1706 dosya, `expected.json`'daki her dosya aynı sonucu verir;
  - örnek `map-work-approved-objects.json/.kcad`; dört bozuk dosya (209'da alan, damgayı vermeyen, dosyalı, tekrarlı);
  - Rust okuyucusu ve yazıcısı aynı (`kentos-kcad`, 242 test).
- **Ortak durumlar** (`scripts/fixtures/map_work_cases.py`, KentOS kodu yok):
  - üreteç listeyi ve karşılaştırmayı kendisi yapar;
  - onaydan sonra değişen, eklenen ve silinen nesneler; listesi aynı onaylı öğe;
  - onay listeyi yazar, yeniden onay yeniler, onaydan çıkış kaldırır;
  - listesiz onaylanmış değişmemiş öğe yeniden onaylanınca listesini kazanır;
  - `map.work.query` 32 durum (30'dan), `map.work.edit` 55 durum (54'ten). Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/map_work_changes.rs`):
  - şema 210 örneği açılınca tasarım güncel, değişen yok;
  - çizgisine kot verilip katmanına bir çizgi çizilince bir değişen, bir eklenen ve iki kimlik;
  - kaydedilen dosya şema 210'dur ve onayın tek çiftini tutar;
  - yeniden onaydan sonra güncel, dosya iki çifti kimliğe göre tutar.

  ADR 2080'in MCP testi aynen geçer.
- **Masaüstü** (`map_work_changes_tests.rs`):
  - panelden onay iki çifti yazar;
  - değişen, eklenen ve silinen çizgiden sonra sayılar ve kimlikler;
  - satıra basınca iki çizgi seçilir, durum çubuğu silineni söyler;
  - yeniden onay sayıyı kaldırır, geri alma döndürür; taslağa dönüş listeyi kaldırır;
  - kaydet–aç aynı sayıları verir.

  `map_work_stamps_tests` ve çapraz denetimler geçer. Resimler `.run/gui-evidence/onaydan-sonra-2880/`.

## Sonra

- Değişen nesnenin neyinin değiştiği (geometri mi, öznitelik mi): bugün yalnız özeti farklıdır.
- Silinen nesnenin onaylanan hâlinin gösterilmesi (ArcGIS'in ortak atası gibi): bugün yalnız sayısı; onaylanan hâl
  saklanmaz.
- Dosyası olan öğede nesne başına fark (dosyanın içeriğinin kendisi).
- Tipli katman profili (MAP-01'in kalanı).
- Web (TODOS §0.2).
