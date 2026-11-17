# ADR 8112: Mimari denetimin koridor kuralları en dar yerden (PAİY 29/3, BYKHY 33/1, 33/2)

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (B15'in kalanı); ADR
  [8110](8110-health-buildings-sleeping-floors-corridors.md) §6'nın ilk sıradaki işi.
- **Bağlam:** ADR [8033](8033-regulation-rules-rooms-storeys-doors.md) (29/3: hol ve koridor net eni 1,20 m; "L biçimli
  koridorda iç daire köşede biraz büyür, ilk sürümde böyle kabul edildi"), [8035](8035-fire-escape-routes-distances-load-exits.md)
  (33/1 kaçış yolu olan koridor 1,10 m, 33/2 yüksek binada 1,20 m), [8110](8110-health-buildings-sleeping-floors-corridors.md)
  (koridorun en dar yeri: `geom::neck`, `room_width::least_of`).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`, olgular, kural paketi:** değişmedi. **Komut:** `cad.arch.audit`'in
  `paiy_corridor_narrow` ve `bykhy_corridor_narrow` bulgularının ölçüsü, iletisi ve yeri değişti; kodlar aynı.

## §1. Sorun

Her iki kural koridorun genişliğini en büyük iç dairesinin çapından okuyordu: bir odanın dar kenarı için doğru, koridor için
değil. En büyük daire koridorun en geniş yerini ölçer: 1,10 m'lik kolları olan L hol 1,29 m okunur (29/3'ün 1,20 m'sinden
geçer), T'de %25, artıda %41 fazla; bir hole ya da lobiye bağlanan 0,90 m'lik koridor holün genişliğini okur. ADR 8033'ün
"ilk sürümde kabul" notu yalnız L'nin %17'sini görmüştü.

## §2. Karar

- **K208 — Ölçü:** hol ve koridor (alanın adla ya da kullanımla okuduğu geçiş) en dar yerinden ölçülür: iki yanında kendi
  genişliğinde daire kalan en kısa kesit, yoksa en büyük daire (ADR 8110 K200). Öbür mahallerin dar kenarı en büyük daire
  kalır (29/1).
- **K209 — Bulgu:** ileti "… en dar yeri 1.10 m < 1.20 m"; bulgunun yeri en dar kesitin ortası (sorunun olduğu yer;
  eskiden dairenin merkeziydi). 33/1 ve 33/2'de koridorun öbür bulguları (kaçış uzaklığı) eski yerinde kalır.
- **K210 — Kodların anlamı:** `paiy_corridor_narrow` "hol ya da koridorun en dar yeri …". `cad.arch.audit`'in
  açıklaması ölçüyü söyler.

## §3. Uygulama

`crates/native/application/src/arch/regulation_audit.rs` (`neck_of`: en dar yer ve ortası; koridor türü mahallerde),
`fire_audit.rs` (koridorun `width`'i en dar yer, `neck` bulgunun yeri), `crates/shared/contracts/src/core/codes.rs`,
`crates/shared/commands/src/catalog/arch_project.rs`; üretilen katalog ve Python SDK'sı.

## §4. Sınamalar

- `regulation_audit_tests::an_l_shaped_hall_is_as_wide_as_its_arms`: konut dairesinde kolları 1,10 m L hol kalır ("Hol en
  dar yeri 1.10 m < 1.20 m"), bulgu kolun köşeye bağlandığı kesitin ortasında; 1,30 m kollarla geçer.
- Mevcut sınamalar yeni iletiyle geçti: dar koridorlu küçük konut (29/3), büro katının 0,85 m koridoru (33/1), yüksek
  binanın 1,15 m koridoru (33/2), kaçış yolları; uygulamanın denetim ve yangın sınamaları, `cad.arch.audit`'in paylaşılan
  durumları.
- Resim: `.run/gui-evidence/koridor-en-dar-8112/` (L holün bulgusu Denetim'de).

## §5. Sıradaki adımlar

- Çıkmaza daralan koridorun dar ucu ve serbest kolonun yanındaki aralık (ADR 8110 §1'in görmediği yerler).
- Bulguyu seçince çizimin en dar kesite yaklaşması (bugün mahal seçilir).
