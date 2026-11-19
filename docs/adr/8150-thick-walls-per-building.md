# ADR 8150: Kalın taş duvarlar bina bina — 5/8 ve 20/8-j her binanın kendi katlarıyla

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8145](8145-structure-per-building.md) §6 ("Kalın taş duvarların bina bina toplamı (olgularda binaya göre): birden çok
  geleneksel bina uzman incelemesinden çıkar").
- **Bağlam:** ADR [8095](8095-thick-stone-walls-out-of-emsal.md) (kalın taş duvarlar 5/8, 20/8-j), 8145 (binanın kendi
  taşıyıcı sistemi; K373: duvarların toplamı bina bina ayrılmadığından birden çok geleneksel binanın biri 3 katı aşıp
  öbürü aşmıyorsa uzman incelemesi).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 5/8, 20/8-j (zemin dahil 3 katı geçmeyen geleneksel kesme taş yığma
  bina).
- **Sözleşme:** yeni komut yok. `kentos_domain::zoning::ZoningLevel.masonry`, `masonry_footprint`.
  `kentos.auditFacts` 1'in `building.masonry`'sine `storeys` (`MasonryStorey`: kat, duvar payı, taban alanı payı, kalın
  duvarlar). `tr.kentos.mimari-mevzuat` 0.38.0. **`.kcad`:** değişmedi. **Web:** dokunulmadı (web'de emsal tablosunun
  kalın duvarları yok; TODOS.md §0.2'de ADR 8095'in maddesi).

## §1. Sorun

Olgular kalın taş duvarların payını projenin toplamı olarak veriyordu. İki geleneksel bloğun biri zemin dahil 3 katı
aşıp öbürü aşmıyorsa kural hangi duvarın hangi bloğa ait olduğunu bilemiyor, bulguyu uzman incelemesine bırakıyordu
(8145 K373).

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 5/8, 20/8-j (8095) | "Taşıyıcı sistemi kesme taş yığma bina olarak tasarlanan zemin dâhil 3 katı geçmeyen … binalarda" duvarların kalın kısmı | Koşul binanındır: her binanın duvarları kendi kat sayısıyla (K387) |
| ADR 8072, 8145 | Bir katın binası bina programının binasıdır; katın taşıyıcı sistemi binasınınki | Bir duvarın binası katının binasıdır (K387) |

## §3. Karar

- **K387 — Duvarlar katlarına göre:** emsal tablosunun her katı kendi payını taşır (`ZoningLevel.masonry`: odalarının
  dış hatları içindeki kalın duvar payı; `masonry_footprint`: sayılan zemin katta dış duvarların taban alanı dışı payı).
  Olgular `masonry.storeys`'te her katın payını ve kalın taşıyıcı duvarlarını (duvarın kendi katıyla) verir; payı ve
  duvarı olmayan kat yazılmaz.
- **K388 — Kural bina bina (`emsal`):** bir binası kendi taşıyıcı sistemini söylüyorsa `_binalar` her binanın katlarının
  payını toplar; 20/8-j ve 5/8 her geleneksel binanın duvarlarını kendi zemin dahil kat sayısıyla okur: 3 katı aşan
  binanınkiler kalır (bulgu onun duvarlarını ve paylarını adlandırır), öbürlerininkiler geçer; satırlar birleşir
  (durumun önceliği, notlar). Beyansız duvarların eksik veri bulgusu projenin bütünüyle bir kez. Duvarları hiçbir
  binanın katına düşmüyorsa (katsız duvar) projenin bütünü okunur. 8145'in K373'ündeki uzman incelemesi kalktı.

## §4. Uygulama

- `crates/native/domain/src/zoning.rs` (`ZoningLevel.masonry`, `masonry_footprint`).
- `crates/shared/extensions/src/audit_facts/building.rs` (`MasonryFacts.storeys`, `MasonryStorey`),
  `crates/native/processing/src/audit_facts/building.rs` (`masonry_storeys`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/emsal.py` (`_duvar_payi`, `duvarlar`, `_bina_duvarlari`).

## §5. Sınamalar

- **Kural** (`test_emsal.py`; 20/8-j ve 5/8'in metninden elle): emsal 52 durum, 2'si yeni: iki geleneksel blok, A 2
  katlı (duvar payı 3 m², taban 1,5 m²), B 4 katlı (2 m², 1 m²) → B'nin duvarları kaldı, bulgu "(2.00 m²)", "(1.00 m²)",
  "zemin dahil 4 katlı"; A 2, B 3 katlı → ikisi de geçti, satırlar birleşti. Eski 50 durum değişmedi.
- **Masaüstü** (`stone_wall_tests`, 8095'in iki katlı taş evi): olgularda Zemin kat'ın payı 9,125 m² (iki odanın
  3,875 + 0,6875'i), taban alanı payı 7,75 m², beş kalın duvar; 1. kat'ın 7,75 m²'si, dört duvar.
- **Ortak süzgeçler:** taş duvar, kesme taş, emsal, bodrum listesi, bina programı sahneleri, zon, kural seti, modül,
  şerit, geçmiş, katalog ve ayar sınamalarıyla 177 masaüstü sınaması yeşil; olguların altı değişmedi (örnek bina
  geleneksel değil); SDK şeması yeniden üretildi; iş çalıştırıcının ulusal kural sınamaları yeşil.
- **Resim:** yok (arayüz değişmedi; olgular ve kuralın sözü).

## §6. Kalanlar

- Katsız duvarlar (katı olmayan kalın duvar) bina bina okunmaz.
