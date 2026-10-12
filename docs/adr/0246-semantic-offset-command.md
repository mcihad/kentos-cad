# ADR 0246: Anlamsal Ötele komutu (`cad.entities.offset`)

- **Durum:** kabul edildi (2026-10-12). plan/cad.md §24.1'in 4. maddesi; CAD-CMD-01, 02, 06, 07, 09, 10 ve 19'un Ötele dilimi.
  Paralel ajanın dalında (`cad-offset`) yapıldı; ADR numarasını ana ajan ayırdı.
- **Bağlam belgesi:** plan/cad.md §5 ve §5.1 (önerilen `cad.entities.offset`), ADR 0013 ve 0022 (ürün komutları ve katalog), ADR 0047
  (Ötele ve `cad.entities.edit`), ADR 0140 (İki yana, Kaynağı sil), ADR 0142 (köşe kotu), ADR 0165 §2 (çizim birimi), ADR 0228 (web'e ara).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler TODOS.md §0.2'de (`CAD-CMD-02`).

## Bağlam

Ötele aracı bugüne dek kopyayı kendisi hesaplıyor (`ops::offset::offset_entity`), sonra hazır geometriyi `cad.entities.edit`'in `offset`
işlemiyle yazıyordu. Python ve MCP de yalnız bu yolu bulabiliyordu: bir ajan “bu yol kenarını 0,2 m doğuya ötele” diyemez, kopyanın
köşelerini kendisi hesaplayıp vermek zorundaydı. plan/cad.md §5 bunu açık söyler: bir arayüz işleminin API girdisi yalnız “uygulanacak son
koordinatlar” ise yüksek düzeyli işlem eksiktir; hesap arayüzde ve Python'da ayrı ayrı yapılmamalıdır.

AutoCAD'in OFFSET'i, QGIS'in Offset Line ve Buffer'ı ve ArcGIS'in Copy Parallel'i aynı girdiyi alır: nesne, mesafe ya da geçilecek nokta,
taraf, iki yana ve kaynağı silme, hedef katman.

## Karar

1. **Yeni ürün komutu `cad.entities.offset` v1** (`kentos_contracts::cad_offset`). Girdi sonucu değil isteği taşır:
   - `uids`: ötelenecek nesnelerin kalıcı kimlikleri (en az bir; tekrarlanan bir kez);
   - `by`: `{ kind: "distance", distance, side }` (mesafe metre, sıfırdan büyük; taraf noktası) ya da `{ kind: "through", point }`
     (Noktadan geç: her nesnenin mesafesi noktanın ona uzaklığı);
   - `bothSides` (İki yana), `deleteSource` (Kaynağı sil), `layerId` (kopyaların katmanı; yoksa kaynağınki), `expectedRevision`.
   - Birim: tel üzerinde mesafe ve koordinatlar **metredir**, projenin gösterdiği birim ne olursa olsun (ADR 0165 §2); araç yazılanı
     çevirip verir.
   - Alt nesne başvurusu v1'de yoktur: araç nesneyi seçer, kenarı değil (plan/cad.md CAD-DATA-13 ayrı iştir).
2. **Çıktı** `created`, `removed`, `distances` (kaynak başına kullanılan mesafe; Noktadan geç'te bulunanı) ve `revision`. **Plan**
   yazmadan kopyaları (`id` 0), silinecekleri, mesafeleri ve sürümü verir; planın sürümüyle yazma tam o planı yazar.
3. **Tek hesap.** İşleyici (`kentos_native_application::offset`) kopyayı çekirdeğin `offset_entity` ve `through_distance`'ıyla bulur
   ve `cad.entities.edit`'in `offset` işlemiyle yazar: kopyanın katmanı, rengi, çizgi kalınlığı, kotu ve ölçüsü (M) bugünkü kuralla
   gelir, adım “Ötele”dir. İki yana'nın öbür yanının noktası (`across`) araçtan işleyiciye taşındı. Önizleme ve yazma aynı hesaptır
   (CAD-CMD-10): plan `edit::plan_in`'in, yazma `edit::execute`'un aynı girdisidir.
4. **Ötele aracı** artık komutu çağırır: imleç hareket ettikçe komutun planı kesik çizgiyle çizilir, tıklama komutu yazar; aracın
   kendi geometri hesabı kalmadı. Görünen davranış değişmedi; iki yana ikisi de yapılamayınca bugün tek uyarı söylenir (eskiden iki).
5. **`cad.entities.edit`'in `offset` işlemi** uyumluluk için kalır: hazır geometri yazan istemciler bozulmaz.
6. **Ret ve uyarılar** (CAD-CMD-19), sırasıyla: `no_entities`, `invalid_uid`, `not_finite` (mesafe, sonra noktanın x'i ve y'si),
   `invalid_distance`, `invalid_revision`, `revision_conflict`, `entity_not_found`, nesne nesne `layer_locked` ve `not_offsettable`
   (`uids[i]`), hedef katmanın `layer_not_found`, `not_a_layer`, `service_layer`, `layer_locked` (`layerId`; gizliyse `layer_hidden`
   uyarısı), `offset_failed`. Yapılamayan kopya (merkezini geçen daire, içe dönen yay, iki yanada nesnenin üstündeki nokta)
   `offset_failed` uyarısıyla atlanır; hiçbiri yapılamazsa hiçbir şey yazılmaz. Kaynağı sil yalnız kopyası yapılan kaynakları siler.
7. **Katalog:** `hosts` yalnız `Desktop` (web duruyor), `headless`, `undo: step`. MCP'de `deleteSource` nesne sildiği için
   `destructiveHint`. Python'da `kentos.cad.entities.offset` katalogdan üretilir.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/cad_offset.rs`; katalog kaydı `catalog.rs`; TS tipleri `apps/web/src/contracts/generated/`
  (`EntitiesOffset`, `OffsetBy`, `EntitiesOffsetMade`, `EntitiesOffsetPlan`, `commandCatalog.json`).
- İşleyici: `crates/native/application/src/offset.rs`; kodlar `codes.rs`'te (`invalid_distance`, `not_offsettable`, `offset_failed`);
  `edit::plan_in` belgeye yazma bağlamı olmadan planlar.
- Araç: `crates/native/interaction/src/offset.rs` (`request`, plan ile önizleme, `execute` ile yazma).
- Başsız sunucu `crates/native/headless/src/dispatch.rs`, MCP `crates/native/mcp/src/tools.rs`, Python `python/kentos/cad/`
  (`scripts/python/sdk.py` ile üretildi).
- Resimler: `apps/desktop/src/offset_scenes.rs` (`otele-onizleme`, `otele-iki-yana`, `otele-sonuc`).

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/offset_command_cases.py` (KentOS kodu olmadan, ADR 0047'nin tanımından): düz kenarlar kesirlerle
  kaydırılır, ardışık kenarların doğrularının kesişimi köşedir; yay aynı merkezle r ∓ d; daire ve yay nokta dışarıdaysa büyür. Çizim,
  her değer tam bir double olacak biçimde seçildi (eksenlere paralel, boyu ikinin kuvveti kenarlar; yarım ve çeyrek mesafeler) ve betik
  bunu doğrular; tek aşkın değer yayın yeni bükümüdür, tam double π'nin tan(π/4)'ü (Python'un libm'i ile çekirdeğinki aynı:
  0,9999999999999999). 30 durum `fixtures/commands/v1/cad.entities.offset.json`'da: çizgi, çoklu çizgi (kotlarıyla), kapalı alan, yaylı
  çoklu çizgi, yay, daire, Noktadan geç, iki yana, Kaynağı sil, hedef katman, birden çok nesne, kilitli katman, ötelenmeyen tür,
  yapılamayan kopya, retlerin sırası, sürüm, plan. Masaüstü (`kentos-native-application`'ın `fixtures.rs`'i) ve Python
  (`test_command_cases.py`) bit bit geçer.
- **Araç = komut:** `crates/native/interaction/tests/all/edits.rs` (`the_offset_tool_writes_what_the_semantic_command_writes`) aynı
  çizimde (`edits.kcad`) araçla ve komutla 0,2 m öteler (çizgi; çoklu çizgi iki yana; kapalı alan Kaynağı sil ile): bütün nesneler
  eşittir. **Python** (`python/tests/test_offset.py`) ve **MCP** (`crates/native/mcp/tests/mcp.rs`,
  `an_offset_by_its_meaning_gives_the_tools_copy`) aynı çizgiyi aynı istekle öteler ve aracın yazdığı sayıları bulur
  (x = 486979,8; y 4420006'dan 4420018'e).
- **İzler:** `edge-tools.json` masaüstünde üç çeşitte geçer (`KENTOS_TRACES_ONLY=edge-tools cargo test -p kentos-desktop
  every_trace_passes`).
- **Resimler:** `KENTOS_SHOTS_ONLY=otele-onizleme,otele-iki-yana,otele-sonuc cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture` → `.run/shots/arac-otele-*` (iki tema, 1440×900 ve 1100×650).

## Sonuçlar

- Ötele'nin ne yaptığı artık katalogda bir sözleşmedir; Python ve ajan aracın kopyasını ister, hesaplamaz.
- Aynı kalıp Buda, Uzat, Köşe yuvarla ve Pah'a uygulanacak (plan/cad.md §24.1 madde 5).
- Açık: alt kenar başvurusu (CAD-DATA-13), birimli miktar girdisi (CAD-DATA-01), web'in işleyicisi ve aracı (TODOS.md §0.2).
