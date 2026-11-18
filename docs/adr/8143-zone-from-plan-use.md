# ADR 8143: Bölge plan kararının kullanımından — Değerlerini al

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8085](8085-zone-storey-heights.md) §6 ("CBS kararından öneri: parselin tek kararının kullanımı bölgeyi önersin, onayla
  yazılsın").
- **Bağlam:** ADR 8085 (Bölge: parselin plandaki kullanımı, PAİY 28/1'in bentleri), [8140](8140-plan-decision-values-into-zoning.md)
  (Değerlerini al), [8141](8141-building-layout-nizam.md) (nizam aynı yolla).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 28/1 (a ticaret alanı, b ticaret + konut alanı, c konut alanı, ç zemin
  katında ticaret yapılabilen konut alanı).
- **Sözleşme:** değişmedi (`Zone::from_plan`, `Zone::word` yalnız okuma). **`.kcad`:** değişmedi. **Web:** dokunulmadı;
  TODOS.md §0.2'de.

## §1. Sorun

Bölge (8085) elle seçiliyordu; planın kararı kullanımını (`Kullanim`) yazıyor ama Değerlerini al onu almıyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| `imar-plani` profili, `plan-kullanim` alanı | konut Konut alanı, ticaret Ticaret alanı, ticaret-konut Ticaret + konut alanı; sanayi, eğitim, sağlık, dini, sosyal, kültür, spor, resmî, teknik, otopark, park, yeşil, meydan, yol | Kod ya da etiket okunur (K367) |
| PAİY 28/1 (8085'in okuması) | a ticaret alanı, b ticaret + konut alanı, c konut alanı, ç zemin katında ticaret yapılabilen konut alanı; öbür kullanımlara kat yüksekliği vermez | konut → c, ticaret → a, ticaret-konut → b, listenin öbürü → "öbür kullanım"; ç'nin profilde kodu yok, plandan alınmaz (K367) |

## §3. Karar

- **K367 — Kullanım → Bölge:** kararın `Kullanim`'ı profilin kodu ya da etiketiyse (Türkçe büyük-küçük harf bir; 8141'in
  katlaması) Bölge'ye: konut Konut alanı, ticaret Ticaret alanı, ticaret-konut Ticaret + konut alanı, listenin öbür
  kodları Öbür kullanım. Başka yazım ("konut+ticaret", "ticari") okunmaz, çevrilmez. Zemin katında ticaret yapılabilen
  konut alanı (28/1-ç) planda yazılmaz: müellif seçer.
- **K368 — Değerlerini al:** söz "…nizam ayrık ve bölge konut alanı İmar durumu'na"; form o bölgeyi tutmuyorsa düğme açık
  kalır; Kaydet yazar.

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`Zone::from_plan`, `Zone::word`); `apps/desktop/src/map/zoning_badge.rs`
  (`PlanTake.zone`), `apps/desktop/src/shell/project/arch.rs` (`take`, `holds`, açıklama).

## §5. Sınamalar

- **Sözleşme:** `the_zone_is_read_from_the_plan_s_uses`: konut, " Ticaret ", ticaret-konut, "TİCARET + KONUT ALANI",
  egitim, "Konut alanı", "Park" okunur; "konut+ticaret", "ticari" ve boş okunmaz.
- **Masaüstü** (`zoning_parcel_tests`): K1'in "konut"u Konut alanı, K2'nin "ticaret-konut"u Ticaret + konut alanı alınır;
  Vazgeç bölgeyi yazmaz, Kaydet yazar; "konut+ticaret" okunmaz.
- **Resimler:** `.run/gui-evidence/plan-degerleri-8140/` yeni sözle yeniden.

## §6. Kalanlar

- Blok başına bölge (birden çok bloklu parsel): bir sonraki dilim (8085 §6, 8094 §6'yla birlikte).
