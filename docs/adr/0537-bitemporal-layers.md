# ADR 0537: İki zamanlı katmanlar: geçerlilik zamanı ve bilinen gün

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-33'ün ilk dilimi: iki zamanlı model ve zamanda sorgu. Kapsamı
  ben belirledim.
- **Bağlam belgesi:** ADR 0210 (zamansal katmanlar, Zaman sürgüsü, anların okunuşu), 0265 (yeni şemanın kuralı), 0503
  (bağlı katman).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - zaman süzgecinin sağlayıcıya (GeoPackage ve PostGIS sorgusuna) taşınması (CBS-02);
  - kayıt zamanının değişmez sürüm kaydına (projenin revizyonları, `project.changes`) bağlanması: bugün kayıt zamanı
    nesnenin öznitelikleridir, KentOS onları kendiliğinden yazmaz;
  - sensör ve gözlem akışı: varlık kimliği, olay ve alım zamanı, kalite, geç gelen ve yinelenen kaydın politikası,
    akış kopunca son değerin “güncel” gösterilmemesi, OGC SensorThings;
  - aynı tarih süzgecinin İşlemler'in girdilerine, öznitelik tablosuna ve dışa aktarmaya kendiliğinden uygulanması;
  - takvim dışı birimler (iş günü), yerel saat diliminin proje ayarı (bugün bölgesiz saat UTC'dir).

## Bağlam

Zamansal katman (ADR 0210) nesnenin gerçek dünyada geçerli olduğu zamanı tutar: Zaman sürgüsü “şu tarihte geçerli
imar durumu”nu gösterir. Ama kadastro ve imar verisi sonradan düzeltilir. 2022'den beri geçerli bir parsel sisteme ancak
2025'te girer. Bu yüzden “1 Haziran 2024'te geçerli olan” ile “1 Ocak 2025'te sistemde bildiğimiz 1 Haziran 2024” farklı
cevaplardır. Plan bu ikisinin ayrı modellenmesini, tarihsel sorgunun kayıt zamanına bağlanmasını, zaman dilimini, eşit
başlangıç ve bitişi, açık aralığı ve geç gelen kaydı ister.

## Karar

1. **Kayıt zamanı katmanın ayarında.** `LayerTime`'ın `recordedStart` ve `recordedEnd`'i (`.kcad` şema 90) nesnelerin
   hangi özniteliklerinin sürümün kaydedildiği ve yerine başkasının geçtiği anı tuttuğunu söyler.
   - Kayıt bitişi kayıt başlangıcı ister.
   - İkisi geçerlilik zamanının ve kimliğin alanlarından ayrıdır, birbirinden de.
   - Yazıcı şema 90'ı yalnız kayıt zamanı tutan katmanı olan çizimde yazar.
2. **Kural çekirdekte.** Sürümün kayıt aralığı [kayıt başlangıcı, kayıt bitişi)'dir; boş bitiş hâlâ geçerli kayıttır
   (+∞). Boş ya da okunmayan başlangıç hiçbir gün bilinmez. Sürüm, kaydedildiği andan yerine başkasının geçtiği ana dek
   bilinir; bitiş anı dahil değildir (`time::recorded_interval`, `time::known`). Anlar ADR 0210'un kuralıyla okunur: tarih
   ya da tarih ve saat, isteğe bağlı bölgeyle (`Z`, `+03:00`); bölgesiz saat UTC'dir.
3. **`cad.time.query` (salt okunur)** zamansal bir katmanın gösterilen nesnelerini söyler.
   - Geçerlilik zamanı bir an (`at`) ya da [`from`, `to`) aralığıdır; ADR 0210 §4'ün tablosuyla değerlendirilir
     (aralıklı, anlık, birikimli; zamansız nesne her zaman gösterilir).
   - `knownAt` o gün bilinmeyen sürümleri bırakır.
   - Cevap gösterilenlerin kalıcı kimliklerini çizimin sırasıyla ve öbürlerinin nedenlerini sayar: `notValid`, `notKnown`,
     `timeless`, `unrecorded`, `unreadable`; anları UTC yazar.
   - Retler: `not_temporal`, `not_recorded` (kayıt zamanı tutmayan katmanda bilinen gün), `invalid_moment`,
     `invalid_period` (an ve aralık birlikte, yalnız bir uç, bitişi başlangıcından önce ya da ona eşit).
4. **Masaüstü.**
   - Zaman ayarları'nda Kayıt başlangıcı ve Kayıt bitişi.
   - Zaman sürgüsünde, kayıt zamanı tutan katman varken Bilinen gün. Çizim, seçim ve kenet o gün bilinen sürümlerle
     yapılır: mağazanın `set_recorded` ve `set_known_at`'i, `time_shown`.
   - Sürgü kapanınca bilinen gün kalkar.
5. **`cad.layers.time`** kayıt zamanı alanlarını aynı kurallarla yazar; Python ve MCP iki komutu katalogdan alır.

## Doğrulama

- `python3 scripts/fixtures/time_query_cases.py --check`: `cad.time.query`'nin 33 durumu, KentOS kodu olmadan. Anların
  okunuşu ve pencerenin kuralı `temporal_cases.py`'nin ADR 0210'dan yazılmış ikizinden, kayıt zamanı bu ADR'nin
  kuralından. Durumların kapsadıkları:
  - geç gelen düzeltme ve bilinmeye başladığı an (bölgeyle);
  - sürüm sınırı (bitiş dahil değil), yerine geçilen gün;
  - boş geçerlilik aralığı, bölgeli aralık ve an, gün.ay.yıl;
  - anlık ve birikimli katman, kayıt zamanı tutmayan katman;
  - on bir ret.
- `python3 scripts/fixtures/temporal_command_cases.py --check`: `cad.layers.time`'ın kayıt zamanı durumları (beş yeni).
- `cargo test -p kentos-geometry-core --lib store::tests::the_day_known_leaves_out_the_versions_not_known_then`.
- `.kcad` şema 90: `fixtures/kcad/v2/recorded-time.kcad` bağımsız Python yazıcısıyla, üç bozuk örnek;
  `kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Resimler: `KENTOS_SHOTS_ONLY=zaman-bilinen,zaman-ayarlari-kayit cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture` (`.run/shots/arac-zaman-bilinen-*`, `arac-zaman-ayarlari-kayit-*`).
