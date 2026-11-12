# 8021 — Oda aracı

Tarih: 2026-11-08. Durum: kabul (M21, ADV-11; plan S11, S13).

## Sorun

- **Bir oda üç araçla çiziliyordu** ve her biri ayrı bir geri alma adımıydı: Dikdörtgen duvar (iki köşe ya da Boyutlar), Mahal (içine
  bir tık), Etiket (mahale bir tık).
- **ADV-11'in kullanıcı sonucu:** "4,20 × 3,60 bir yatak odası, yanına 1,20'lik koridor fareyle ve birkaç sayıyla çizilir."
  Plan "Oda aracı: dikdörtgen ya da çokgen oda → duvarlar + mahal + etiket, tek adımda" diyor. Büyük düğmeler arasında Oda var
  (ADV-09); M15'te Oda yokken altıncı yeri Ölçü tutmuştu.

## Araştırma (8 Kasım 2026)

- **ADV-11'in örnekleri:**
  - BricsCAD BIMQUICKDRAW [W1 B4, B5]: tek hareketle duvarlı oda.
  - Floorplanner ve Archisketch: oda çizilir, duvarları ve adı birlikte gelir.
- **KentOS'ta parçalar hazırdı:**
  - Dikdörtgen duvar (`cad.walls.rectangle`): boy referansı (İç net, Dış ölçü, Eksen), Boyutlar, dönüş. Ortak duvar
    (`adopt`, ADR 0770 §1): bitişik odanın duvarı yeniden çizilmez.
  - Mahal (`cad.spaces.create`): tohum noktasından sınır bulur.
  - Etiket (`cad.tags.create`): yalnız `of` verilince türün şablonu ve çerçevesi gelir.
  - Tek geri alma: `Document::begin_group`; iç içe gruplar dıştakine katılır.

## Karar

- **Oda (`tool.room`)** Dikdörtgen duvarın aracıdır (`WallRectangle::room`): aynı çizim, aynı seçenekler (Boyutlar, Boy
  referansı, Ortak duvar, Kalınlık, Stil, Açı). Ayrıca **Ad (A)** seçeneği vardır.
- **İkinci tıkla üçü birlikte yazılır, tek geri alma adımı "Oda" olur:**
  - duvarlar (`cad.walls.rectangle`);
  - mahal, dikdörtgenin ortasından tohumlanır (`cad.spaces.create`; net sütun kuralı, Etkin kat), adı seçilen addır;
  - mahalin etiketi (`cad.tags.create`, şablonu ve çerçevesi türünün).
  - Her biri kendi katmanında (ADR 8008): MM-DUVA-…, MM-XXXX-MAHAL, MM-XXXX-ETIKET. Yoksa aynı adımda açılır.
  - Mahal ya da etiket reddedilirse grup iptal edilir, duvarlar da gider: yarım oda kalmaz.
- **Odanın ortası dikdörtgenin kendisinden hesaplanır.** İki köşede ortaları, Boyutlarda köşeden gösterilen yana boyların
  yarısı (dönüşle). Ortak duvar benimsendiğinde yeni duvarlar kapalı halka olmadığından önizlemenin halkasına güvenilmez.
- **Mahal adı** projenin mahal adlarından (Proje ayarları › Mimari) seçilir ya da yazılır: "Adsız" ad yok demektir.
  - Satır açık seçimdir (`Setting::open`): liste önerir, bağlamaz. İmleç kutusunda listede olmayan ad da alınır. Tam yazılan
    kelime, onunla başlayan öbürlerine üstün gelir (Oda, Oda 2).
- **İmleç kutusunda** (ADR 8018) Oda'nın sırası Boyutlar → Mahal adı → Kalınlık → Boy referansı.
- **Şerit:** Duvar panelinde Duvar ▾'nin yanında büyük **Oda**. Ölçü ▾ küçüldü; büyükler artık planın saydığı altı düğmedir:
  Duvar, Oda, Kapı, Pencere, Mahal, Donatı. Görünen düğme 27 → 28.
- **İkon** `room`: dış duvar, iç duvar ve ortada mahalin adı. Dikdörtgen duvardan (köşe tutamaçları) ve Mahal'den (tek
  çizgi) ayrılır; ADR 8016'nın bekçileri geçer.
- **Kat:** Oda Etkin kat'ta çizer (ADR 0713'ün listesine eklendi).
- **Yazılan ad:** KentOS Mimari'de "oda" artık Oda'yı başlatır, önce Mahal'i başlatıyordu (ADR 8002'nin `names-mimari.json`'u).
  O satır kalktı: aracın adı "Oda" bunu karşılıyor. Mahal'in adı MH. Takma adı ODACIZ; "ROOM" Mahal'in takma adı olduğu için
  verilmedi.
- **İleti:** "Oda eklendi: duvarları, mahali (Salon) ve etiketi; boy referansı: İç net. Tek geri almayla gider."

## Ölçüm

- **Test** (`building_context_tests`):
  - `oda_writes_the_walls_the_room_and_its_tag_in_one_undo_step`: Ad "Salon", Boyutlar 4,3, iki nokta.
    1 duvar, 1 mahal (Salon, alanı 12,000 m²), 1 etiket. Bir geri alma: 0, 0, 0.
  - `oda_s_name_beside_the_cursor_takes_the_list_or_any_word`: "Kiler" listede yok, alınır.
  - Şeridin testi büyükleri Duvar, Oda, Kapı, Pencere, Mahal, Donatı sayar.
- **Gerçek pencere** (`fixtures/interaction/real/oda-araci.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Oda, ↓ (Boyutlar) "4.2,3.6", Tab, Mahal adı "Salon", Enter, iki tık. Duvarlar, mahal ve etiket "Salon 15,12 m²" yazıldı;
    MM-DUVA-TUGLA, MM-XXXX-MAHAL, MM-XXXX-ETIKET açıldı.
  - Bir Ctrl+Z üçünü ve katmanlarını kaldırdı, Ctrl+Y geri getirdi.
  - Resimler: `.run/gui-evidence/mimari-m21/m21-once-sonra.png`, `oda-ikonu.png`.

## Sonraki adımlar

- Çokgen oda (plan: "dikdörtgen ya da çokgen"): yol aracının kapanan döngüsüyle (M22'nin "Kendiliğinden mahal"i ile birlikte).
- 2+1 kabul izinin Oda ve imleç yanı seçeneklerle yeniden yazımı (ADV-11'in ilk kabulü: etkileşim 157 → ≤ 80).
- Odanın kenar tutamacıyla büyümesi (ADV-11 "Odayı büyütme").
