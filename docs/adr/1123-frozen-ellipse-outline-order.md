# ADR 1123: Elipsin çizilen konturunun toplama sırası dondurulmuş yanıtlarla aynı kalır

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11` sırasında `main`'de zaten kırmızı
  olan iki style-core testi (`tests/all` `whole_layers_as_frozen`, `one_symbol_on_one_object_as_frozen`).
- **Bağlam belgesi:** ADR 0835 (elips uç noktası tutamağı), `fixtures/style/v1/cases.json` (stil derleyicisinin
  dondurulmuş yanıtları; WASM ile aynı bitler).

## 1. Neden

İki test son basamakta farklı sayı veriyordu (ör. sembol 22'de `-44.47798031369822` yerine `…21`; katman 4).
İkisi de elips nesnesi içerir. `git log` ile bulunan değişiklik 31c296103'tür (2 Kasım 2026, ADR 0835):
`ellipse_point` `c.x + major.x·cos t + minor.x·sin t` sırasından `c.x + (major.x·cos t + minor.x·sin t)`
sırasına geçti. Kısıtlama çözücüsü harita koordinatında doğru yuvarlanmış uç nokta için bu sırayı ister
(`a_real_mouse_drag_endpoint_uses_the_independently_rounded_map_coordinate`, mpmath referanslı). Aynı fonksiyon
`tessellate_ellipse` üzerinden çizilen konturu da üretiyordu; dondurulmuş yanıtlar eski sırayla kaydedilmişti.
Bu gerçek bir aritmetik sıra değişikliğidir, tolerans ya da beklenen değer sorunu değildir.

Deneme: `ellipse_point` eski sıraya alınınca iki test geçti, kısıtlama testi kırmızı oldu. Yani iki sözleşme
birbirinden ayrılmalı.

## 2. Karar

- `tessellate_ellipse` artık özel `drawn_point` kullanır: merkez ve eksen terimleri soldan sağa toplanır
  (dondurulmuş yanıtların sırası). Stil derleyicisinin, renderer'ın ve PDF'in gördüğü kontur bu fonksiyondan gelir.
- `ellipse_point` ADR 0835'teki tek orijin toplamasını korur: snap, tutamak, kısıtlama, budama ve işaret
  yerleştirici doğru yuvarlanmış uç noktayı kullanmayı sürdürür.
- Beklenen değerler yenilenmedi, tolerans eklenmedi.

## 3. Kanıt

- `cargo test -p kentos-style-core --test all`: 9/9 geçti (iki dondurulmuş test dahil).
- `cargo test -p kentos-geometry-core --lib -- ellipse`: 25/25 geçti (mpmath referanslı kısıtlama testi dahil);
  `--test all -- ellipse`: 2/2.

## 4. Sınır

Açık elips yayında çizilen konturun son noktası ile `ellipse_point(t1)` arasında harita koordinatında en çok bir
ULP fark olabilir (31c296103'ten önce de çizilen kontur bu sıradaydı). Bu, çizim ile snap arasında yeni bir
gösterim farkı değildir; snap analitik uç noktayı kullanır.
