# ADR 9232: Blok tanımının eğrileri en büyük yerleştirmesine göre çizilir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-09, 2. adım.
- **İstek:**
  - planın RM-09 maddesi: “Bloklardaki eğriler (tanımın toleransı en büyük örneğin ölçeğinden)”;
  - sahibin “yakında daire köşeli görünmez” ve “kaliteden ödün verilmez” sözleri
    ([ADR 9222](9222-quality-is-not-traded-for-speed.md));
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. `.kcad`, komut, ayar ve stil topluluklarının biçimi aynı.

## §1 Bulgu

[ADR 9223](9223-screen-curves-round-at-every-zoom.md), ekranın eğrilerini derin görünümün eşiğindeki çeyrek piksele
yuvarlıyor (metrede 0,5 mm). Derin görünümde ([ADR 9220](9220-deep-view-scene.md)) bloklar açılarak
çizildiği için onların eğrileri de yuvarlak.

**Eşiğin altında örnekli bloklar başka yoldan çiziliyor**
([ADR 1063](1063-instanced-blocks-and-curve-detail-levels.md)):

- Tanım bir kez, kendi biriminde kaydediliyor; her yerleştirme onu kendi ölçeğiyle yerleştiriyor.
- Dünya toleransı tanımın biriminde yanlış olacağından kayıtta kaldırılıyordu. Tanımın eğrileri tur başına 72 kirişte
  kalıyordu.
- **72 kirişin sapması yarıçapın binde biri.** Büyük ölçekle yerleştirilen bir bloğun eğrisi derin eşiğin altında
  köşeli görünüyordu:
  - 20 m'lik bir çember 460 px/m'de kenarında 9 piksellik bir köşe yapıyor;
  - bağlanmış bir dış referansın 30 m'lik eğri cephesi 100 px/m'de yaklaşık 3 piksel.

## §2 Karar

Örnekleyici, bir katmanın yerleştirmelerini görünüşlerine göre sayarken her görünüşün en büyük düzgün ölçeğini de
tutar (`Instancer::scales`). Tanım kaydedilirken eğrilerin toleransı dünya toleransının bu ölçeğe bölümüdür:

- **En büyük yerleştirmede** eğri, blok dışındaki bir eğri gibi çeyrek piksele yuvarlak; daha küçüklerde daha da ince.
- **Toleransın olmadığı yerde** (kâğıt ve PDF, harita projeksiyonlu görünüm) tanımın kendi 72 kirişi kalır.
- **Bedeli:** büyük ölçekle yerleştirilen tanımın eğrisi daha çok kiriş, tanım başına bir kez. Örneğin 20 m'lik
  çember tanımda 72 yerine en az 444 kiriş.

## §3 Kanıt

**Sınama:** `kentos-style-core`
`style::build::instances::tests::a_definition_s_curves_are_drawn_for_its_largest_insert`.

- Girdi: yarıçapı 1 olan bir çemberin bloğu 1, 20 ve 1 ölçekle yerleştiriliyor.
- 0,5 mm toleransla, örnekli tanımın kiriş sayısı 20 m'lik çemberin gerektirdiği kadar.
- Toleranssız 72.

Stil çekirdeğinin bütün sınamaları geçti (86 + 9); yerel stil (52 + 52 + 2 + 2).

**Altın görüntüler** ([ADR 9225](9225-golden-pictures-and-grid-bound.md)):

- `bloklar` izni aştı: 517 piksel farklı, 20'sinin eşi yok. Farklar blok çemberlerinin kenarlarındaki kenar
  yumuşatması.
- Görüntü yeniden yazıldı. Öbür 14 görüntü bayt bayt aynı.

**Gerçek pencere** (`.run/gui-evidence/render/rm09/once-sonra-blok-egri.png`):

- Kurulum: yarıçapı 1 olan bir bloğun 1, 20 ve 1 ölçekli üç yerleştirmesi (`.run/rm09/blok/blok-egri.dxf`); 20 m'lik
  çemberin kenarına tekerlekle 460 px/m'ye yakınlaşıldı.
- **Önce:** kenar, ortada köşe yapan iki düz kiriş.
- **Sonra:** düzgün bir yay.

## Uygulama

- `crates/shared/style-core/src/style/build/instances.rs`: `Instancer::scales`, tanımın kaydında toleransın ölçeğe
  bölünmesi; sınama.
- `fixtures/render/golden/bloklar.png`: yeniden yazıldı.
- [ADR 9223](9223-screen-curves-round-at-every-zoom.md) §3'ün “bloklardaki eğriler” maddesi bununla kapandı.
