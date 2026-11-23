# ADR 9205: Kamera ve jestler tuvalde; arayüz her tekerlek çentiğinde yeniden kurulmaz

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-05.
- **İstek:** sahibin sözü (23 Kasım 2026): “arayüz ve çizim tuvali tamamen birbirinden bağımsız olacak şu anda mesela
  sahnede obje seçtiğimde bazen arayüzün de yenilendiği fark ediyorum”. Bulgu: [ADR 9200](9200-render-engine-research.md)
  §1.1, ölçü [ADR 9201](9201-render-measurement.md) §3.
- **Yerine geçtiği:** [ADR 0019](0019-native-wgpu-viewport.md)'un “kamera uygulamanın durumudur, her olay uygulamaya
  ileti olur” kararı, tekerlek ve orta tuşla kaydırma için.
- **Sözleşmeler:** çizim alanının iletisine `Event::ViewChanged { from, to }` eklendi. Komut, ayar ve `.kcad` değişmedi.

## §1 Sorun

Çizim alanının her olayı bir uygulama iletisiydi (`Message::Viewport`). Tekerleğin her çentiği şu yoldan geçiyordu:

1. `App::update`;
2. bütün `App::view`: şerit, paneller, üst katmanlar;
3. kamera değiştiği için yazıların yeniden yerleşimi ve eğrilerin yakınlık bandı.

Gerçek GPU'da ölçülen (9201 §3):

| Dosya | Tekerlek çentiği başına görünüm |
|---|---|
| sivas.kcad | 99,5 ms |
| NCZ | 116 ms |

Hızlı bir el on çentiği arka arkaya çevirdiğinde bu bedel on kez ödeniyordu.

## §2 Araştırma

- **iced 0.14.2:** shader widget'ı bir olaydan sonra ileti yayınlamadan yalnız kare isteyebilir
  (`shader::Action::request_redraw`, kaynak kodundan doğrulandı). Böyle bir karede çalışma zamanı uygulamanın
  `update`'ini de `view`'ını da çalıştırmaz, var olan arayüzü yeniden çizer.
- **CAD programlarında** kamera değişince sahne yeniden çizilir, yeniden üretilmez. Ağır işler, örneğin yazıların
  yerleşimi ve eğri çözünürlüğü, görünüm durunca yapılır.

## §3 Karar

1. **Canlı kamera** (`viewport/live.rs`, `LiveView`):
   - Uygulamanın görünüm durumuyla çizim alanının widget'ı arasında paylaşılır.
   - Tekerlek ve orta tuşla kaydırma bu kamerayı widget'ın içinde değiştirir ve yalnız kare ister. Hesap
     `Camera::zoom_at` ve `pan_by`'dır, uygulamanın kullandığının aynısı.
   - Sahne çizilirken bu kamerayı okur.
2. **Görünüm 150 ms durunca** (yazıların dinlenmesiyle aynı süre, 0597) uygulamaya bir kez söylenir:
   `ViewChanged { from, to }`.
   - Uygulama kamerayı alır ve görünümü bir kez kurar; yazılar ve semboller o an yerleşir.
   - Görünüm geçmişi bunu bir tekerlek dizisi sayar (`record_wheel`).
   - Gezinme vurgusu gider (9204).
3. **Üst katmanlar canlı görünümü izler** (`viewport/follow.rs`, `Follow`):
   - **Taşınanlar:** yazılar, veri katmanlarının etiketleri, seçimi ve düzenlemeleri, arazi, hat ve onaylı katman.
     Kuruldukları kameradan canlı kameraya bir ölçek ve bir kaydırmayla çizilirler.
   - **Ölçek ve kaydırmanın doğruluğu:** aynı dönüklük ve aynı harita sisteminde iki kamera arasındaki ilişki tam olarak
     budur; her noktası sahnenin aynı dünya noktasının üstüne düşer.
   - **Gizlenenler, görünüm durana kadar:**
     - imlecin altındakini söyleyen kartlar ve yazı düzenleyicileri;
     - ekranda sabit boyutlu çizilen işaretler, tutamaçlar, artı imleci ve aracın önizlemesi. Bunlar taşınıp
       ölçeklenseydi, hızlı bir tekerlekte yedi kat büyürlerdi (denetçinin ilk koşumunda görüldü).
4. **Uygulamanın kendi değişikliği** (sığdırma, önceki görünüm, boyut) bir sonraki görünümde canlı kameraya yazılır
   ve süren bir jesti bitirir.
5. **İmleç:**
   - Jest sürerken bir **imleç hareketi** jesti almaz. İmlecin dünya noktası ve kenetin erimi canlı kameradan okunur
     (`Viewport::world`, `shown`).
   - **Basma, sağ tık ve “Tümünü göster”** jesti önce alır; ekranın gösterdiği kamerayla çalışırlar.

## §4 Planda olmayan kararlarım

1. **`Message::Viewport` kalkmadı.** Planda “kalkar” yazıyordu. İmleç, basma ve sağ tık araçlara gitmeli, yani
   uygulamanın iletisi olarak kalmalı. Kalkan, tekerlek ve kaydırmanın her adımının ileti olması.
2. **Durum çubuğundaki ölçek görünüm durunca değişir.** Kendi küçük bileşeni RM-06'ya kaldı.
3. **Ölçüm donanımı çalışma zamanına uyduruldu** (5a5d99dc9). Görünüm yalnız bir iletiden sonra kurulur ve
   sayılır. Ölçümün önce ve sonra hâli aynı donanımla alındı.

## §5 Kanıt

**Sınamalar:**

- `viewport::live::tests` (2) ve `viewport::follow::tests` (2).
- `viewport::tests::a_wheel_turn_moves_the_live_camera_and_reaches_the_app_once_settled`: gerçek widget'lar üstünden
  iki tekerlek çentiği; uygulamanın kamerası bekler, canlı kamera yakınlaşır; dinlenince uygulama tek iletiyle o
  kamerayı alır ve geçmişe bir kayıt düşer.
- `viewport::tests::a_move_reads_the_live_camera_and_a_press_takes_it`.
- **Masaüstünün bütün kitaplık sınamaları:** 3 054 geçti. Kalan 3 hata bu daldan önce de vardı; main'de b9b8e592d
  onardı.

**Ölçüm:** UHD 770, release, `perf::files`; önce = 5a5d99dc9, sonra = bu değişiklik. Ayrıntı
[docs/perf/files-desktop-rm05-2026-11-23.md](../perf/files-desktop-rm05-2026-11-23.md).

| Ölçü | Önce | Sonra |
|---|---|---|
| Tekerlek karesinde UI | sivas.kcad 92,8 ms · NCZ 64,9 ms · DWG 40,8 ms · DXF 23,6 ms | dördünde 0,2 ms |
| Görünüm kurulan tekerlek karesi | 32 / 32 | 1 / 32 (dinlenince) |
| Kaydırma karesinde UI | 0,8–2,0 ms | 0,2 ms |
| Görünüm kurulan kaydırma karesi | 40 / 40 | 0 / 40 |
| Bütçeyi aşan tekerlek karesi | 16–32 / 32 | 0–1 / 32 |
| Resmin son hâline varış (patlama) | 59–313 ms | 294–591 ms |

Son hâle varış uzadı. Ağır iş (yazıların yerleşimi, eğrilerin bandı) artık görünüm 150 ms durunca, bir kez yapılıyor;
o kare 25–193 ms UI tutuyor. Bu işi arayüz iş parçacığından çıkarmak RM-07 ve RM-08..10'un işi.

GPU süresi değişmedi: sahne aynı, yalnız kamerası canlı. Uzak görünümün GPU süresi LOD'un işi (F3).

## Uygulama

- `apps/desktop/src/shell/viewport/live.rs` (yeni): `LiveView`, `SETTLE`.
- `apps/desktop/src/shell/viewport/follow.rs` (yeni): `Follow`, `Follows`, `moved`.
- `apps/desktop/src/shell/viewport/mod.rs`:
  - `Event::ViewChanged`;
  - `Program::live` ve `update`: tekerlek, kaydırma, dinlenme;
  - `draw`: canlı kamera;
  - `Viewport::{live, live_sent, pointer_px}`.
- `apps/desktop/src/shell/viewport/state.rs`: `live_for_view`, `take_live`, `shown`, `world`, `ViewChanged`.
- `apps/desktop/src/shell/input.rs`: jestin alınması, kenetin canlı kamerası.
- `apps/desktop/src/shell/navigation.rs`: geçmiş.
- `apps/desktop/src/shell/view/area.rs`: üst katmanların `Follow`'u.
