# ADR 0569: Olaylar rotalarını kendiliğinden izler

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-27'nin dilimi; [ADR 0526](0526-event-drift.md)'nın “sonraya kalan” listesindeki
  “rota değişince olayların kendiliğinden izlemesi (belgenin adımında, ilişkili ölçü gibi)”. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0224 (doğrusal referanslama: Olayları rotada göster), 0526 (Olay kaymasını denetle), 0562 (`keepCurrent`: alt ağ adları
  aynı adımda yeniden yazılır; bu ADR'nin örneği), 0337 (belgenin izleyicileri), 0265 (`.kcad` şemasının altı adımı), 0508 (İşlemler araçları komut
  olarak), 0244 (CAD işlem manifesti).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Yalnız dokunulan rotaların olaylarının yerleştirilmesi: bugün bağın katmanlarına dokunan her yazma bağın bütün olaylarını yeniden
    yerleştirir (§3, maliyet §6); artımlısı graf için ADR 0562 §6'daki gibi “tam kurmayla aynı” ispatı ister.
  - İşlemler penceresinin kendi çalıştırıcısı bir komutun adımı değildir, olayları izlemez; `cad.processing.run` bir komuttur ve izler.
  - Olay katmanının süzgeci (ADR 0211): izleme süzgeçten geçmeyen olayları da yerleştirir (süzgeç çizimi ve seçimi süzer, veriyi değil).
  - Alan olaylar (rotaya göre konumlanan alanlar) ve Z/M'nin olaya yazılması: olay nesnesi nokta, çizgi ya da çoklu çizgidir.

## Bağlam

Olay nesnesi rota kimliğini ve ölçüsünü kendi özniteliklerinde taşır (ADR 0224 §5, 0526 §1). Rota değişince (eksen düzeltilir, rota
yeniden kalibre edilir) olay eski yerinde kalır; ADR 0526 bunu raporlayıp isteyene taşıyordu. ArcGIS'in olay davranışları (Event Behavior)
ve QGIS'in ilişkili geometrisi gibi, izlemenin kendiliğinden olması istenir: kaza noktaları ve kaplama kesimleri eksenin taşındığı adımda
eksenle birlikte gider, geri alma ikisini birlikte geri alır. Aynı adımda yazma, ilişkili ölçü (ADR 0249), ilişkili tarama ve alt ağ
adlarının güncel tutulması (ADR 0562 §3) ile aynı mekaniktir: belgenin izleyicisi komutun adımının içinde çalışır.

## Karar

### 1. Kapsam ve veri modeli: projenin olay bağları

Güncel tutma bir **bağ**dır, katmanın tek bir özelliği değil: bağın bir olay katmanı, en çok 16 rota katmanı ve olayın hangi
özniteliklerinin rotayı, ölçüyü ve ötelemeyi söylediği vardır. Bunlar ADR 0562'nin `keepCurrent`'ı gibi ağın bir alanına da,
katmanın bir alanına da sığmaz (rota katmanları başka katmanlardır); ağın tanımı gibi **proje ayarı**dır: `settings.linearEvents`, bir
`EventLink` listesi:

| Alan | Anlam |
|---|---|
| `id`, `name` | Formun kimlik ve ad kuralı (1–64); kimlik projede tekil |
| `events` | Olay katmanının kimliği; bir katman en çok bir bağın olay katmanıdır |
| `kind` | `point` (bir ölçü) ya da `line` (başlangıç ve bitiş ölçüsü; çizgi ya da çoklu çizgi nesnesi) |
| `routes` | 1–16 rota katmanı; birbirinden ve olay katmanından ayrı |
| `routeField` | Rotaların kimlik alanı: rota, kimliğin ilk nesnesidir (rota katmanlarının sırasıyla) |
| `eventRoute` | Olayın rota kimliğini söyleyen özniteliği |
| `measure` / `from` + `to` | `point`: ölçü alanı zorunlu, `from` ve `to` olmaz; `line`: ikisi zorunlu, `measure` olmaz |
| `offset`, `side` | İsteğe bağlı öteleme alanı (m, rotaya dik) ve artı ötelemenin yanı (`right` varsayılan, `left`) |
| `tolerance` | Olayın rotanın verdiği yerden en çok uzaklığı (m), eksiksiz ya da sıfır; yok: 0,001 |

Kurallar sözleşmenin `event_link_problem` ve `event_links_problem`'idir (Türkçe iletilerle; her yerde aynı). Listede ek kurallar: en çok 50
bağ; bir bağın olay katmanı başka bağın rota katmanı olamaz (bir bağın yazması ötekini beslerdi: zincir ve döngü yok). Katman ve alanın
varlığı kurala girmez (kopuk bağ bir şey yapmaz, tanımda söylenir).

Karar gerekçesi (bağ ya da katman özelliği): ağın alt ağ adları aynı örüntüdür (ADR 0562) ama o tek ağın tek bir yazısıdır; olayın
rotası başka katmanlardadır ve her olay katmanının alanları farklıdır. Bağ listesi, `.kcad`'de bir ayar olarak, katman ağacına dokunmaz.
Sunucuda ayrı bir doğrulama yazılmadı (birleşik görünümler gibi: ayar sözleşmenin türüyle taşınır; kuralları komut ve `.kcad` okuyucusu uygular).

**`.kcad` şema 97** (`SCHEMA_WITH_LINEAR_EVENTS`; ADR 0265'in altı adımı): ayarların `linearEvents`'i, listenin JSON'u, yalnız bağı olan
projede yazılır; şema 2–96 yükünde bilinmeyen alandır. `FORMATS_VERSION` aynı (sütunlar değişmedi). Altı adım: kodek
(`crates/shared/kcad`: `lib.rs`, `encode.rs` (12 harfli anahtarlar arasında), `decode.rs`), bağımsız Python okuyucu ve yazıcı
(`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`), örnek dosya `fixtures/kcad/v2/linear-events.{kcad,json}`, on bozuk örnek
(`broken/linear-events-*`: şema 96'da, bilinmeyen alan, varsayılan yazılmış, boş liste, noktada başlangıç alanı, çizgide bitiş yok, rota
olay katmanı, iki bağ aynı kimlik, aynı olay katmanı, olay katmanı başka bağın rotası, eksi tolerans), `expected.json`, şartname §5 ve
şema listesi, taşıma testi (`carriage`'in ayar alanları). `broken/schema-version-97.kcad` 98 oldu.

### 2. Komut: `cad.linearref.define`

`cad.network.define`'ın biçiminde: `set` (aynı kimlikli bağın yerini alır, yoksa sona eklenir) ve `remove`. Proje ayarıdır: çizimi kirli
yapar, geri alma adımı yazmaz ve **olayları kendiliğinden taşımaz**; bağdan sonraki yazmalar taşır (§3; tanımlamanın kendisi izlenmez).
Redler sırasıyla: `no_event_link`, `no_id`, `invalid_event_link`; sonra `invalid_revision`, `revision_conflict`, `unknown_event_link`.
Çizimde olmayan katman `unknown_layer` uyarısıyla (olay katmanı, sonra rotalar). `events_dirty` her yazan komutun olası uyarısıdır
(`PROTOCOL_CODES`). Python `kentos.cad.linearref.define`; MCP aracı katalogdan, silen olduğu için yıkıcı listesinde; manifestte
`linearRef.eventLinks` `semantic`, `via` `cad.linearref.define`.

### 3. İzleme: aynı adımda, olayların yeniden yerleştirilmesi

`call::recorded` bir komutun yazmasından ve kısıtların izlemesinden, alt ağ adlarının izlemesinden sonra, **aynı adımda**
`event_follow::follow`'u çalıştırır:

1. Bağın katmanlarından (olay katmanı ya da rota katmanlarından biri) birinin nesnesine dokunan yazma (eklenen, silinen, değişen; değişen
   nesnenin önceki ve sonraki katmanı `Document::step_layers`) o bağı izletir. Ayar nesne sayılmaz; `cad.linearref.define` izlenmez.
2. Olay nesneleri (`kind` `point`: nokta; `line`: çizgi ya da çoklu çizgi; öbür türler ve olay katmanında başka nesneler **atlanır**, uyarı yok)
   kendi özniteliklerinden okunur: rota kimliği, ölçüler, öteleme. Rota, rota katmanlarının sırasıyla kimliğin ilk nesnesidir.
3. Yerleştirme **tek kuraldır**: Olay kaymasını denetle'nin kuralı, çekirdekte `ops::lrs::events_run` (nokta: ölçüdeki yer ve öteleme;
   çizgi: iki ölçü arasındaki parçalar, rotanın boşluğunda ayrı parçalar, yaylar yay). Okuma, kayma ve taşıma
   `kentos_native_application::lrs_events`'tedir (okuma `read`, kayma `drift_of` = `point_drift` ve `line_drift`, taşıma `moved`);
   İşlemler'in Olay kaymasını denetle'si de buradan geçer, ikinci bir yerleştirme kuralı yazılmadı. (Rotaların kimlik dizini, ölçü
   hücresinin sayı kuralı ve çoklu çizgi yazımı da oraya taşındı; İşlemler onları buradan alır.)
4. Rotanın verdiği yerden `tolerance`'tan fazla uzak olay oraya taşınır: nokta noktaya, çoklu çizgi rotanın parçalarına (M'leriyle),
   `Line` nesnesi yalnız yol düz tek kenarsa (kırıklı yola taşınamaz). Nesnenin öteki alanları (katman, öznitelikler, görünüş, kimlik) kendisinin
   kalır. Elle taşınan olay bu yüzden hemen rotanın verdiği yere döner: elle yer vermek için ölçü ya da öteleme özniteliği değişir.
5. Taşınanlar `Document::update_many` ile, açık adımın içinde yazılır: komutun kaydı, geri alma adımı, `expectedRevision` ve planı tek adımdır;
   geri alma yazmayı ve olayların yerlerini birlikte geri alır.
6. **Bir yazma bunun yüzünden hiçbir zaman reddedilmez.** Yerleştirilemeyen olay (rotası yok, rota kimliği ya da ölçü hücresi boş ya da sayı
   değil, ölçü rotanın dışında ya da rotanın ölçüsü yok, çizgi kırıklı yola taşınamaz) ve kilitli olay katmanında taşınacak olaylar olduğu yerde
   kalır; yazmanın cevabına bağ başına bir `events_dirty` uyarısı girer: “… bağı olayları rotalarında tutuyor ama bu yazma n olayı yerine
   koyamadı: #3: …”. İlk neden söylenir (kilit nedeni yalnız başka neden yoksa); Olay kaymasını denetle her olayı nedeniyle söyler. Uyarı
   bağın katmanlarına dokunan her yazmada tekrarlanır (bir olayın yeri hâlâ kurulamıyorsa).
7. Bağlar listenin sırasıyla izlenir; bir bağın olay katmanı başka bağın rota katmanı olamadığı için sıranın sonucu etkilemez.

### 4. Arayüz

- **Olay bağları penceresi** (`linearRef.eventLinks`; CBS › Analiz › Ağ analizi ▾'de Olay kaymasını denetle'nin altında,
  `catalog/v1/additions.json`'un `overflow`'una eklendi), ikon `routeEventLink` (eğri bir rota, üstünde iki olay ve olaylardan rotaya
  kesik dikmeler, yanında ok; ikonu ben seçtim, resimde görünür): sol listede bağlar (Yeni bağ, Bağı sil), sağda Ad, Olay katmanı, Olay türü (Nokta, Çizgi), Rota katmanları
  (işaretler), Rotaların kimlik alanı ve Olayın rota alanı (katmanların alan adları), Ölçü alanı ya da Başlangıç ve Bitiş ölçüsü alanları,
  Öteleme alanı (yok ya da alan), “Artı öteleme solda”, Tolerans (m). Kaydet yazılanı `cad.linearref.define` ile (önce silinenler, sonra
  değişen ve yeniler) yazar; komutun reddi olduğu gibi söylenir ve pencere açık kalır.
- Pencere projedeki ayarı düzenler; olayların kendisini Olay kaymasını denetle aracı (Yerine taşı) tek adımda toplu olarak düzeltir. Bağ
  kurulunca bağdaki olaylar hemen taşınmaz (ayar adım değildir): ilk izleme sonraki yazmada olur ya da bir kez Olay kaymasını denetle çalıştırılır.

### 5. Performans

Bir yazmanın ödediği: bağın rota katmanlarındaki nesnelerden rota dizini (kimliklere göre) ve olay katmanındaki her olay için bir
`events_run` yerleştirmesi ve kayma ölçümü (nokta için bir uzaklık; çizgi için köşelerin rotanın yoluna uzaklığı). Bütçe: 10 000 olay ve
1 000 rotada bir yazmanın izlemesi 100 ms altında (bir tutamaç sürüklemesi her adımda ödemez: bağın katmanlarına dokunmayan yazma hiçbir şey
ödemez). Release ölçümü **yapılmadı** (bu Mac'te derleme dakikalar sürer): sayı Doğrulama'da “ölçülmeyen” olarak duruyor; çok büyük olay
katmanları için artımlı yerleştirme sonraya kalanda.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/linear_events.rs` (`EventLink`, `EventLinkKind`, `EventSide`, `event_link_problem`,
  `event_links_problem`), `ProjectSettings.linear_events`, kodlar `no_event_link`, `invalid_event_link`, `unknown_event_link`,
  `events_dirty` (`codes.rs`); komut türleri `crates/shared/commands/src/cad_linearref.rs`, katalog ve yetenekler.
- **`.kcad`:** `crates/shared/kcad` (şema 97), `tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`, `docs/specs/kcad-v2.md`.
- **Uygulama katmanı:** `kentos-native-application`: `lrs_events.rs` (ortak yerleştirme), `event_follow.rs` (izleyici, `call::recorded`'ın
  içinde), `linearref_define.rs` (komut).
- **İşlemler:** `crates/native/processing/src/builtin/lrs/drift.rs` ortak okuma, kayma ve taşımayı kullanır; `mod.rs` rota dizinini
  ve hücre kurallarını uygulama katmanından alır (davranış değişmedi: `lrs.json`'un üç `drift-` durumu aynı).
- **Masaüstü:** `apps/desktop/src/event_links_window.rs` (Olay bağları penceresi); `catalog/v1/additions.json`.
- **Otomasyon:** Python `kentos.cad.linearref` (üretildi), `python/tests/test_linearref.py`; MCP yıkıcı listesi; manifest.

## Doğrulama

- **Bağımsız başvurular (KentOS kodu olmadan):**
  - `scripts/fixtures/linearref_define_command_cases.py --check`: `cad.linearref.define`'ın **34 durumu** (set, değiştir, sil, plan, doğrula,
    çizimde olmayan katmanlar ve on altı rota katmanı, bağın her kuralı ve sözü, listenin iki kuralı, sürüm çakışması).
  - `scripts/fixtures/linearref_follow_cases.py --check` (`uv run --no-project --with mpmath`): olayların izlenmesinin **24 durumu**: rota taşınır
    (iki bağ, çoklu çizgi rota, geri alma ve yineleme), ölçü, rota ve öteleme öznitelikleri değişir, elle taşınan nokta ve çizgi geri döner,
    bağın katmanına dokunmayan yazma hiçbir şey yerleştirmez, dokunan yazma bütün olayları yerleştirir, rota silinir ya da kimliği değişir,
    ölçü dışarıda, sayı değil ya da boş, öteleme sayı değil, kilitli olay katmanı, yeni olay (geri alma), rotasız yeni olay, tolerans, kırıklı
    yola çizgi, olay silinir, bağı tanımlamak ve silmek, bağsız çizim. Rota kuralı `lrs_cases.py`'nin 50 basamaklı başvurusu (`point_at`,
    `stretch`), okuma ve kayma `lrs_processing_cases.py`'nin; beklenen koordinatlar ±1e-6 içinde. Masaüstü işleyici ilk koşuda iki dosyanın 58 durumundan 57'sini geçti;
    düşen tek durum başvurunun bağı yanlış sıraya koymasıydı (set yerinde değiştirir), başvuru düzeldi.
  - `kcad_v2_reference.py --check` (1102 dosya) ve `tools/kcad/kcad.py validate`: `linear-events.kcad` ve on bozuk örnek.
- **Rust:** `KENTOS_CASES_ONLY=cad.linearref cargo test -p kentos-native-application --test all every_command_case` (58 durum); uygulama
  paketinin bütün testleri (121 + 40); `capabilities` (her ret ve uyarı kodu bir durumda); `kentos-kcad` (204: şema 97, taşıma matrisi,
  şemaların altı adımı); `kentos-contracts` ve `kentos-commands` (katalog ve TS türleri güncel; `EventLink.tolerance` için `x-unit`);
  `kentos-processing`'in `the_linear_referencing_cases_do_what_they_say`'i (Olay kaymasını denetle'nin üç `drift-` durumu ortak koddan
  geçince aynı); `kentos-mcp`; clippy `-D warnings` dokunulan on crate'te temiz (masaüstü hariç: aşağıda).
- **Python:** `python/tests/test_linearref.py` (3 test: taşınan rotanın olayları ve tek geri alma, silinen rota ve `events_dirty`, SDK ile
  set ve remove); `test_command_cases.py` bu iki dosya için; `sdk.py --check`; `scripts/gis/coverage.py --check`; `scripts/cad/manifest.py --check`.
- **Masaüstü:** `event_links_window` testleri (pencerenin yazdığı bağ ve sonraki yazmanın izlemesi, geri alma); resimler
  `.run/shots/olay-baglari-pencere-{1440x900,1100x650}{,-acik}.png` (koyu ve açık; 1100×650'de taşma ve kesik düğme yok, düzenleyici kaydırılır).
- **Ölçülmeyenler:** §5'in 10 000 olay ve 1 000 rotadaki izleme süresi (release derlemesi bu makinede dakikalar sürer; sayı yok); Olay bağları
  penceresinin gerçek kullanıcı eliyle denenmesi; sunucuda (PostGIS) bağlı proje; İşlemler penceresinin kendi çalıştırıcısının izlememesi (bilinçli).
- **Bulunan, bu maddenin olmayan:** `apps/desktop/src/networks/window.rs`'in `Window::form`'u yalnız testte kullanılıyor; `cargo clippy -p
  kentos-desktop -- -D warnings` (test olmayan ikili) `dead_code` ile düşüyor. Bu madde dokunmadı.
