# ADR 0290: Dikdörtgen ve düzgün çokgen komutları

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-04`: “Rectangle üzerinde boyut/alan/rotation/fillet/chamfer
  seçenekleri; polygon üzerinde inscribed/circumscribed ve kenar yöntemlerini tamamla.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0032 (dikdörtgen ve düzgün çokgen araçları), 0287 (yöntemleri: Boyutlarla, Alanla, çember içinde, dışında,
  kenardan), 0162 (Çakışma ve Topoloji'nin yeni alanı), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Araçlarda bütün seçenekler vardı: Dikdörtgen'in iki köşesi, Boyutlar (B), Alan (A), Döndür (D), Köşe yuvarla (Y) ve Pah (P);
Döndürülmüş dikdörtgen'in kenarı ve eni; Düzgün çokgen'in çember içinde, dışında, kenardan yöntemleri ve yazılan yarıçapı. Ama
halkayı araç hesaplıyor, hazır köşelerle `cad.polygon.create`'e veriyordu (manifestte `geometry`); Python ve ajan bir dikdörtgeni
köşelerini kendileri hesaplamadan çizemiyordu.

## Karar

1. **`cad.rectangle.create` v1:** çerçeve `corners` (iki karşı köşe), `size` (köşe, dönüşün doğrultusunda boy, karşısında en, `toward`'ın
   çeyreğine), `area` (köşe, alan, boy; en alan bölü boy) ya da `edge` (kenar ve solundaki en, eksi sağında); `rotation` kenarların
   doğrultusu (radyan, `edge`'de okunmaz); `corners` keskin, `fillet` (yarıçap) ya da `chamfer` (iki kenarda aynı uzaklık). Halka paylaşılan
   çekirdeğin (`rect_from_corners`, `rect_from_size`, `rect_side_for_area`, `rect_from_edge`, `corners_of_ring`). Retler sırasıyla:
   `not_finite`, `invalid_distance` (boy, en, alan, yarıçap, pah sıfırdan büyük; kenarın eni sıfır değil), `degenerate_shape`,
   `invalid_corners` (çekirdeğin sözüyle), sonra `cad.polygon.create`'inkiler.
2. **`cad.polygon.regular` v1:** kenar sayısı (3–1024) ve çerçeve `inscribed` (merkez, ilk köşe), `circumscribed` (merkez, ilk kenarın
   ortası) ya da `edge` (kenar, çokgen solunda). Halka çekirdeğin (`regular_polygon`, `regular_polygon_on_edge`). Retler: `not_finite`,
   `invalid_count`, `degenerate_shape`, sonra `cad.polygon.create`'inkiler.
3. **Yazma `cad.polygon.create`'in yolundan:** iki komut halkayı hesaplar, çokgenin denetimlerini ve yazmasını kullanır (`polygon::write`),
   kayıtta kendi adlarıyla görünür. Plan çokgenin planıdır. Yeni kodlar `degenerate_shape`, `invalid_corners`.
4. **Araçlar komutla:** Dikdörtgen, Döndürülmüş dikdörtgen ve Düzgün çokgen girdiyi kurar (verilen köşeler, boyut, dönüş, köşe biçimi; yazılan
   yarıçap için çekirdeğin `regular_radius_point`'i), önizlemeyi komutun hesabından (`shapes::rectangle_ring`, `regular_ring`) alır ve
   komutla yazar. Köşeler sığmazsa araç eskisi gibi söyler ve keskin yazar. Çakışma denetimi ya da Topoloji yeni alanı değiştirirse alan,
   ADR 0162'deki gibi alanın yolundan yazılır (`points::write_area_by`): komutun girdisi bu kipleri bilmez.
5. **Python ve MCP:** `kentos.cad.rectangle.create`, `kentos.cad.polygon.regular`; MCP'de araç (yeni nesne yazar, yıkıcı değil).
   Manifestte araçlar ve yöntemleri `semantic`.

## Sonuçlar

- Bir ajan “20 × 12,5 m, köşeleri 1,5 m yuvarlak oda” isteğini köşe hesaplamadan tek çağrıyla yazar.
- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan: dikdörtgende 35, düzgün çokgende 27 (trigonometri 60 basamaklı serilerle,
  çekirdeğin işlem sırasıyla, en yakın double'a).

## Doğrulama

- `python3 scripts/fixtures/shape_command_cases.py --check`; `cargo test -p kentos-native-application --test all` (ortak durumlar ve
  yetenekler); `cargo test -p kentos-interaction --test all the_shape_tools_write_through_their_commands` ve bütün etkileşim testleri;
  `pnpm py:test`; `cargo test -p kentos-mcp`; `python3 scripts/cad/manifest.py --check`; bütün izler değişmeden geçer.
- Resim: `KENTOS_SHOTS_ONLY=dikdortgen-komut cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
