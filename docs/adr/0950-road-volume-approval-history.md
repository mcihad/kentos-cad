# 0950 — Onaylı kübajın değişmez geçmişi

Tarih: 2026-11-05. Durum: kabul (Y44). Kapsam ROAD-VOL-17.

## Sorun

ADR 0906 §5 kübajı onaylatıyor: onay günü, notu, toplamlar ve her aralığın kazısı ile dolgusu kaydediliyor. Kaynak
değişince sorgu bugünkü hesabın onaylıdan nasıl ayrıldığını söylüyor. İki boşluk vardı:

- **"Onayı kaldır" onayı siliyordu.** Ne zaman, neden kaldırıldığı ve kaldırılan onayın miktarı kayboluyordu.
- **Yeniden onay eskisinin üstüne yazıyordu.** Birinci hakedişin metrajı ikinci onayla siliniyordu.

ROAD-VOL-17: "Approved quantity revision immutable olsun; kaynak değişince yeni run ve diff üret, önceki hesap/audit
silinmesin."

## Dayanak

- **Hakediş pratiği:** her hakedişin metrajı donar. Sonraki hakediş bir öncekinin üstüne kurulur ve farkıyla okunur.
- KentOS'ta onaylı kübajın tanımı zaten değişmiyor (`volume_approved` reddi). Eksik olan bitmiş onayların kaydıydı.

## Karar

- **Onay numaralı:** `VolumeApprovalDef.number`, 1'den başlar. Numarasız eski onay (şema 173 öncesi) bir onay sayılır;
  yerine gelen 2 olur.
- **Biten onay geçmişe geçer:** `VolumeDef.history`, eskiden yeniye, onaylandığı hâliyle ve bitişiyle
  (`VolumeApprovalEnd`: `at` gün, `revoked`, `note` neden).
  - `road.volumes.approve` onaylı bir kübajı yeniden onaylayınca eski onay, yeni onayın günüyle "yerine yenisi geldi"
    diye geçmişe yazılır.
  - `revoke` ile kaldırılan onay, verilen gün ve nedenle "kaldırıldı" diye geçmişe yazılır. Bunun için `at` ve `note`
    kaldırmada da alınır; ikisi de isteğe bağlıdır.
  - Geçmiş hiçbir komutla değişmez. Kübaj düzenlemesi geçmişi olduğu gibi taşır (`road.volumes.edit` tanımı klonlar).
  - **Geçmişi olan kübaj silinmez** (`volume_approved`): silmek denetim izini silmek olurdu.
- **Kurallar** (`contracts::road`): geçerli onayın bitişi olmaz; geçmişteki her onayın olur; numaralar artarak gider;
  geçerli onayın numarası geçmiştekilerden büyüktür; geçmiş en çok 200 onaydır.
- **KCAD şema 173:** `number`, `ended` ve `history`. Yazıcı 173'ü yalnız bunlardan biri varken seçer; 172 ve öncesi
  `unknown_field` ile reddeder. Örnek `road-volume-history.kcad` (iki biten onay, biri kaldırılmış), eski şemada
  `broken/road-volume-history-in-schema-172.kcad`. Bağımsız okuyucu `tools/kcad/kcad.py` aynı kuralları denetler.
- **Fark tek yerde:** `road_volumes::change(önceki, sonraki)`. Aralıklar km'leriyle eşlenir (1e-6 m); değişen
  (1e-6 m³'ten çok), yeni ve kalkan aralıklar ile toplamların farkı çıkar. Onayın bugüne göre karşılaştırması
  (`compare`) da bunu kullanır.

## Komut: `road.volumes.history`

- **Çıktı:**
  - onaylar eskiden yeniye: onay olduğu gibi, geçerli mi, bir öncekine göre farkı;
  - bugünkü toplamlar;
  - bugün son onaya (geçerli ya da biten) göre.
- Bugün hesaplanamayan kübajın geçmişi yine verilir; `problem` nedenini söyler.
- **Retler:** `not_a_road_project`, `unknown_volume`.

## Pencere: Kübaj → Onay geçmişi

- Formda "Onay notu": Onayla ve Onayı kaldır bu notla ve bugünün günüyle yazar (kaldırma da artık günlü).
- Onaylı ya da geçmişi olan kübajda "Onay geçmişi" görünümü:
  - **tablo:** #, gün, durum (geçerli · yerine yenisi geldi · kaldırıldı, gün ve neden), kazı, dolgu, öncekine göre
    farklar, değişen aralık sayısı, not; son satır "Bugün";
  - **resim:** kazı ve dolgu iki ayrı şeritte, her biri kendi aralığında. İlk sürüm ikisini sıfırdan başlayan tek
    eksende çiziyordu; 25.878 m³ üstüne +96 m³ düz bir çizgi gibi görünüyordu.

## Ortak durumlar ve ölçüm

- `road.volumes.approve`: 24 durum. Kaldırma geçmişe yazılır; kaldırma günü ve nedeniyle; yeniden onay; kaldırılmış
  onaydan sonra ikinci onay; üçüncü onay; numarasız eski onay. `road.volumes.edit`: "Onay geçmişi olan kübaj
  silinmez".
- `road.volumes.history`: 20 durum (`scripts/fixtures/road_volume_history_command_cases.py`, kübajlar
  `road_volume_command_cases.py`'nin bağımsız başvurusundan). Rust ve Python aynı bulur.
- Ana kübajda koridor 20 m'ye sıklaşınca ikinci onay birinciye göre kazıyı +96,0 m³, dolguyu +15,0 m³ değiştirir;
  41 aralık değişir. Beş hakedişli durumda numaralar 1–5. Bölge kısalınca 37 aralık değişir ya da kalkar.
- Pencere bekçisi `approvals_are_kept_in_the_history`: iki onay ve bir kaldırma; geçmiş [1, 2]; ikincinin bitişi
  "kaldırıldı" ve notuyla; pencerenin okuduğu geçmiş komutunkiyle aynı; geçmişi olan kübaj silinmiyor.

## Açık

- Onayın kaynak özetleri (koridorun, arazinin, boykesitin sürümleri). Bugün yalnız kübaj tanımının sürümü
  (`revision`) yazılıyor. ROAD-VOL-01 ve ROAD-SURVEY-09 ile birlikte.
- Taşıma planının onayı ve geçmişi (ROAD-HAUL-08).
