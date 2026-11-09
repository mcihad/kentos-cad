# 3680 — Onayın nesnelerin değerlerini tutması: değişen özniteliğin adı, silinenin onaylı hâli; `.kcad` şema 213

Tarih: 2026-11-09. plan/map-cad.md MAP-01, yedinci tur. Durum: taslak. ADR 3280'in “Sonra”sındaki “değişen özniteliğin
adı ve onaylanan değeri; silinen nesnenin onaylanan hâli (onaylanan değerlerin saklanması)”.

İlgili:
- [ADR 1320](1320-sheet-sources-and-update-publication.md): nesne özeti (`kentos.object-digest/1`);
- [ADR 2880](2880-map-work-objects-changed-since-approval.md): onaydan sonra değişen nesneler (`approvedObjects`);
- [ADR 3280](3280-map-work-changed-object-parts.md): değişen nesnenin parçaları (`approvedParts`, şema 211).

## Dayanak

- **ArcGIS Pro**, [Archive history](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/archive-history.html)
  (9 Kasım 2026'da okundu):
  - arşivleme nesne düzeyindeki değişikliklerin kaydını tutar. Her ekleme, güncelleme ve silme izini bırakır, nesnenin
    geçmişi okunabilir kalır;
  - `GDB_FROM_DATE` nesnenin o hâlinin başladığı, `GDB_TO_DATE` bittiği andır. 9999-12-31 güncel hâldir.

  Yani eski hâl saklanmadıkça “ne idi” söylenemez; özet yalnız “değişti”yi söyler.
- **ArcGIS Pro'nun sürüm farkları** (ADR 3280'in okuması): güncellenen nesnede iki tarafın öznitelik değerleri ayrı
  gösterilir.
- **KentOS bugün:**
  - onay her nesnenin özetini (ADR 2880) ve dört parçasının özetini (ADR 3280) tutar, değerini tutmaz;
  - “öznitelikleri değişti” denir ama hangi özniteliğin, neyden neye değiştiği söylenmez;
  - silinen nesne çizimde yoktur, ne olduğu da söylenemez.

## Karar

1. **`approvedValues`** (`WorkItem`, `.kcad` şema 213):
   - onay `approvedObjects` ve `approvedParts`'ı yazarken her nesnenin değerini de yazar: nesne özetinin alındığı metnin
     kendisi, yani anlık görüntü biçiminin `id`'siz, anahtarları sıralı, boşluksuz JSON'u;
   - liste `approvedObjects` ile bire birdir. Her metnin SHA-256'sı listedeki özettir: değer kendi özetiyle doğrulanır,
     okuyucu metni yeniden yazmadan denetler;
   - değerler birlikte en çok 4 MiB tutulur (`MAX_APPROVED_VALUES_BYTES`). Daha büyük öğe yalnız özetleri ve parçaları
     tutar. Bu sınır KentOS'un seçimidir, bir kuralın değeri değildir: dosya ve iş ayarlarının her geri alma adımı küçük
     kalsın;
   - onaydan çıkan öğenin değerleri de kalkar. Onay damgası değişmez.
2. **Doğrulama** (`mapWork.items[i].approvedValues`): sayısı `approvedObjects`'inkinden farklı, birlikte 4 MiB'tan uzun
   ya da SHA-256'sı nesnenin özeti olmayan değer belgeyi reddettirir.
3. **Cevap** (`map.work.query`, `changes`'e ek alanlar; komut sürümü değişmez). Değerleri tutulmuş onayda:
   - her değişen nesnede (`objects[]`) `attributes`: `attrs`'tan değişen her öznitelik, adına göre, onaylanan (`approved`)
     ve şimdiki (`now`) değeriyle. Yoksa o taraf yazılmaz. Ekleri değiştiyse `attachments: true`;
   - `removedObjects`: her silinen nesne, kimliğiyle ve onaylanan hâliyle (`approved`, anlık görüntü biçiminde), kimliğe
     göre, en çok 1000.

   Değerleri tutulmamış onayda (şema 211 ve öncesi) bu alanlar yoktur; yeniden onaylanınca yazılır.
4. **Masaüstü** (Harita işi paneli):
   - Değişenler'e basınca durum çubuğu: “… değişen: 1 nesnenin öznitelikleri (Kot: yok → “1060”); silinen 1 nesne
     çizimde yok, onaylanan hâli: Çizgi (tasarim).”;
   - satırın ipucu: “Değişen öznitelikler: …” ve “Silinenlerin onaylanan hâli: …”. İlk üçü yazılır, kalanın sayısı
     söylenir.
5. **`.kcad` şema 213:**
   - yazıcı bir öğe değerleri taşıyınca 213'ü seçer;
   - 212 ve öncesinde alan bilinmez (`unknown_field`);
   - bire bir olmayan liste, özeti tutmayan değer ve 4 MiB'ı aşan liste `bad_value`;
   - Rust okuyucusu ve bağımsız okuyucu aynı. “Daha yeni şema” dosyası `schema-version-214.kcad` oldu.
6. **Kapsam işareti:** ADR 2880 ve 3280'deki gibi **yalnız Harita (CAD)**.

## Sınırlar

- Silinen nesneyi onaylı hâline geri getiren bir komut yoktur; cevap onu söyler, yazmaz.
- Geometrinin neyinin değiştiği (hangi köşe) söylenmez; değer ikisinin karşılaştırılmasına yeter, bu ADR yalnız
  öznitelikleri adlar.

## Kanıt

- **Dosya biçimi:**
  - bağımsız yazıcı ve okuyucu 1717 dosyada aynı sonucu verir (`kcad_v2_reference.py --check`). Örnek
    `map-work-approved-values.json/.kcad`, değeri çizimin kendi nesnesinin metninden üretilir ve özeti tutar;
  - üç bozuk dosya: şema 212'de alan, bire bir olmayan liste, özeti tutmayan değer;
  - Rust `kentos-kcad` 242 test geçer, geçerli dosya sayısı 244.
- **Sözleşme testleri** (`crates/shared/contracts/tests/all/approved_values.rs`): tutarlı değerler geçer; bire bir
  olmayan, özeti tutmayan ve 4 MiB'ı aşan reddedilir.
- **Ortak durumlar** (`scripts/fixtures/map_work_cases.py`; KentOS kodu yok; değer, özniteliklerin farkı ve silinenin
  hâli elle):
  - edit 57 durum, query 36 durum;
  - onay değerleri de yazar. Değerleri tutulmadan (şema 211) onaylanmış öğe yeniden onaylanınca değerlerini kazanır.
    Onaydan çıkınca değerler kalkar;
  - “Kot” “1060”tan “1061”e değişince adı ve iki değeri söylenir; silinen çizgi onaylanan hâliyle söylenir.

  Rust ve Python geçer.
- **Bağımsız Python testi** (`python/tests/test_map_work_approved_values.py`):
  - şema 211 örneği yeniden onaylanıp kaydedilir. Bağımsız okuyucuyla (tools/kcad/kcad.py) okunan dosya şema 213'tür;
  - her değerin SHA-256'sı özettir ve JSON olarak okunduğunda okuyucunun okuduğu nesnenin kendisidir;
  - değişen özniteliğin farkı ve silinenin hâli kaydedilen değerden burada hesaplanır.
- **MCP** (`crates/native/mcp/tests/map_work_changes.rs`):
  - şema 213 örneğinde değişen öznitelik ve silinen çizgi söylenir;
  - yeniden onaydan sonra kaydedilen dosya şema 213'tür (önceki test 211 bekliyordu).
- **Masaüstü** (`map_work_changes_tests`): beklenen değişiklikte öznitelik ve silinenin hâli; durum çubuğunun cümlesi;
  kaydedilen dosya şema 213. Harita işi testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/onaylanan-degerler-3680/`.

## Sonra

- Silinen nesneyi onaylanan hâline geri getirme komutu (değerden; komut, Python ve MCP bütünlüğüyle):
  [ADR 4080](4080-map-work-restore-as-approved.md) (`map.work.restore`).
- Geometri farkının çizimde gösterilmesi (onaylanan hâlin hayalet çizimi).
- Dosyası olan öğede nesne başına fark; tipli katman profili (MAP-01'in kalanı).
- Web (TODOS §0.2).
