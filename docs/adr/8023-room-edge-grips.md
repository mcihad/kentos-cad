# 3023 — Odayı kenarından büyütme

Tarih: 2026-11-08. Durum: kabul (M23, ADV-11 ve ADV-01; plan S13).

## Sorun

- **Oda kenarından büyütülemiyordu.** Seçili mahalin tek tutamacı etiketinin yerindeydi; sürüklenince bütün mahal yer
  değiştiriyordu (ADR 0342 §3). Odayı büyütmek için duvar seçilip Duvarı kaydır'a (ADR 0790 §1) ya da duvarın geçici
  ölçüsüne (ADR 0710) gidiliyordu.
- **Oda seçilemiyordu.** Mahalin parçaları sınırı ve adıdır.
  - Döşemesine tıklamak hiçbir şey seçmiyordu.
  - Oda aracının odasında (ADR 8021) ortaya tıklamak etiketi seçiyordu.
  - Sınır duvarın yüzünde olduğu için kenara tıklamak duvarı seçiyordu. Mahal yalnız tıkın "1/2" listesinden seçilebiliyordu.
- **ADV-11 "Odayı büyütme":** "mahalin kenar tutamacı odayı büyütür; Duvarı kaydır'ın planını bağlarla kullanır".
  ADV-01'in kullanıcı sonucu: "Bu odanın iç eni 3,20 m olsun".

## Araştırma (8 Kasım 2026)

- **Floorplanner** (Editor manual, s. 13–14):
  - Duvar parçası sürüklenir. Mavi ok karşı duvara uzaklığı gösterir.
  - Sürüklerken bu uzaklık yazılıp Enter'a basılır ("wall to wall distance").
  - Odanın iç ölçüsüne tıklanıp değer yazılır, oklar hangi duvarın kayacağını seçer.
- **Chief Architect Room Planner** ("Modifying Walls"):
  - Seçili duvar boyuna dik taşınır; ortasında taşıma tutamacı vardır.
  - Odalar ölçüleriyle yazılıdır ("Room 16'-0" x 16'-0"").
  - Seçili duvarın yakın duvarlara uzaklığı görünür.
- **Planner 5D** (yardım: "How to resize a room"): 2B'de duvar seçilip sürüklenir ya da uzunluğu yazılır.
- **KentOS'ta parçalar hazırdı:**
  - Duvarı kaydır'ın planı (`cad.walls.edit`, `shift`): bağlar, izleyen duvarlar, açıklıklar, mahallerin alanı, iki yanın
    net mesafesi (ADR 0770 §3).
  - Mahal tohumundan sınırını duvarlardan yeniden bulur.
  - Seçmede duvarın gövdesi (ADR 0776 §3) ve açıklığın boşluğu (ADR 0798 §3) alan olarak bulunuyor.

## Karar

- **Oda döşemesinden seçilir** (`store/pick.rs`, `score`):
  - Mahal, içine tıklanınca net alanıyla bir alan olarak bulunur.
  - Kenar isabetleri önce gelir; içindeki daha küçük alanlar da (tarama, kapı boşluğu). Odanın içindeki bir çizgi
    odadan önce seçilir.
  - Döşeme (slab) böyle bulunmaz: bütün katı kaplar.
  - Basılı sürükleme yine seçim kutusudur.
- **Kenar tutamacı** (`room_grip.rs`):
  - Seçili mahalin her düz kenarında, bir duvarın yüzüne oturuyorsa, kenarın ortasında bir tutamaç vardır. Kenara dik
    çift ok olarak çizilir; dinamik bloğun doğrusal parametre tutamacının şekli (ADR 0319 §1).
  - Duvar kilitsiz bir katmanda, odanın katında olmalı; kenar düz, duvar parçası düz olmalı.
  - Sınırı elle çizilmiş mahalde (tohumu yok) tutamaç yoktur: duvarları izlemez.
  - Tutamacın üstünde durulunca: "Odayı büyüt: kenarı sürükleyin · İç ölçü 7.885 m".
- **Sürükleme ya da tıklayıp yerine tıklama:**
  - Kenarın arkasındaki duvar parçası imleçle kendine dik kayar. Duvarı kaydır'ın planıdır (`walls_edit::plan`): bağlar,
    bitişik duvarlar, açıklıklar, mahaller izler.
  - Bağ geçirmezse kart bağı yerinde işaretler (ADR 0790 §1).
  - Önizleme: kenarın kendisi, gideceği yer, oda boyunca kesik bir çizgi (Floorplanner'ın oku) ve duvarların yeni hâli.
  - Kart sırası:
    - iç ölçü önce → sonra;
    - bu mahalin alanı, sonra öbür mahaller;
    - kenarın öbür yanındaki net mesafe;
    - izleyenlerin sayısı.
  - Tek geri alma adımıdır ("Duvarı düzenle").
  - İleti: "Oda büyütüldü: Mahal 101 Salon 12,00 m² → 15,00 m²; kenarı +1,000 m kaydı." Küçülünce "küçültüldü".
- **Yazılan sayı:**
  - Odanın o kenara dik yeni iç ölçüsüdür. Enter'a gerek yok; yazılıp onaylanınca uygulanır.
  - Önünde + ya da − varsa ne kadar büyüyeceği ya da küçüleceğidir.
  - Karşısında duvar yoksa sayı kenarın ne kadar dışarı gideceğidir.

## Ölçüm

- **Testler:**
  - `geometry-core` `wall_pick.rs::a_click_on_a_rooms_floor_picks_the_room_and_a_line_in_it_first`:
    - döşemedeki tık [oda];
    - odadaki çizginin üstü [çizgi, oda];
    - mahalsiz yer: boş.
  - `building_context_tests::a_room_s_edge_grip_moves_its_wall_and_the_room_grows`, Oda 4 × 3 üzerinde:
    - döşemeye tık odayı seçer, 4 tutamaç;
    - sağdaki 1 m dışarı sürüklenince 12 → 15 m², kart "İç ölçü" ile başlar ve "Salon"u söyler;
    - bir geri alma 12 m²;
    - üstteki tutamaca 3.5: 14 m²;
    - sağdakine −0.5: 12,25 m², "Oda küçültüldü".
  - `building_context_tests::a_room_s_edge_grips_need_an_unlocked_wall_and_a_room_that_follows_it`:
    - duvar katmanı kilitliyken 0 tutamaç, açılınca 4;
    - aynı sınır elle çizilince 0.
- **Gerçek pencere** (`fixtures/interaction/real/odayi-buyut.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Oda ile oda çizildi; döşemesine tıklanınca Mahal sekmesi açıldı, kartta "Alan 51.81 m²", dört kenarda tutamaç.
  - Sağ tutamaçta ipucu "İç ölçü 7.885 m".
  - Tıklayıp götürünce kart "İç ölçü: 7.885 m → 9.988 m · Mahal: 51.81 m² → 65.63 m²"; tıklanınca duvar kaydı, oda büyüdü.
  - Ctrl+Z geri aldı; üst tutamaca "+1" yazılınca oda 1 m yukarı büyüdü.
  - Resim: `.run/gui-evidence/mimari-m23/m23-once-sonra.png`; başvurular `referans/`.

## Sonraki adımlar

- Yay kenarlı oda, bölücüyle kapanan kenar (mahal ayırıcı), kolon yüzü: bugün tutamaç yok.
- Sınırı çizilmiş mahalin kenarını Paralel kaydır'la (ADR 0191) büyütmek.
- Birden çok kenarı birlikte (Shift ile) kaydırmak; iki oda arasındaki duvarı ortak tutamaçla kaydırmak.
