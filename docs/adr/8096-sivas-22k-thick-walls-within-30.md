# ADR 8096: Sivas 22/k — kalın taş duvarlar %30'un içinde; olgulara emsalin %30'u

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8094](8094-structural-system-traditional-cut-stone.md) §6 ("Sivas 22/k"), [8095](8095-thick-stone-walls-out-of-emsal.md) §6.
- **Bağlam:** ADR [8095](8095-thick-stone-walls-out-of-emsal.md) (taş duvarların kalın kısmı, tablo %30 dışında sayar),
  [8034](8034-regulation-stairs-shafts-elevators-local-rules.md) (yerel yönetmelik katmanı: sıkı olan bağlar),
  [8030](8030-emsal-taks-zoning-badge-and-table.md) (5/8'in %30'u).
- **Dayanak:** Sivas Belediyesi İmar Yönetmeliği 22/k (RG 10.11.2022/32009); Planlı Alanlar İmar Yönetmeliği 5/8 (RG
  31.12.2022/32060), 69/3.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  `exemptCap` (5/8'in %30'u ve tablonun içinde saydığı emsal harici).

## §1. Araştırma (17 Kasım 2026)

Sivas'ın metni Resmî Gazete'nin eski sayısından (WebFetch, ADR 8034'ün kaynağı) okundu; araç metni uzun alıntılamadığı
için okunamayan yerler kural setinde "…" ile yazıldı.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Sivas 22/k | Taşıyıcı sistemi kesme taş yığma, zemin dahil 3 katı geçmeyen geleneksel binada iç taş duvarların 12,50 cm'den, dış taş duvarların 25 cm'den fazla kısımları, %20'lik balkonları ve taşlık, dış sofa, eyvan "toplamı %30'u geçmemek üzere" katlar alanına dahil edilmez | Duvarların kalın kısmı %30'un içindedir (K139) |
| Sivas 22 (öbür bentler) | Balkon %20, depo eklentisi %50, kapıcı 70 m², kütüphane 50 m² | PAİY'ninkilerle aynı: ayrı kural yok |
| PAİY 5/8 (31.12.2022'den) | Kalın taş duvarlar "bu hesaba dâhil edilmeksizin emsal haricidir" | PAİY'de %30'un dışında; Sivas daha sıkı |
| PAİY 69/3, ADR 8034 | Yerel yönetmelik planın söylemediği yerde uygulanır; sıkı olan bağlar | Sivas'ın %30'u bağlar; ileti PAİY'nin okumasını da söyler |

- **"Toplamı %30'u geçmemek üzere":** bendin kalemleri 5/8'in %30'unun içinde sayılır (22. maddenin öbür kalemleri gibi).
  KentOS, %30'un içindeki emsal haricine duvarların kalın kısmını ekleyip sınırla karşılaştırır.

## §2. Karar

- **K139 — Sivas 22/k pakette** (`emsal.sivas_duvarlar`, paket 0.20.0): yerel yönetmelik Sivas, bina beyanlı ve zemin dahil
  en çok 3 katlıysa (çok katlıda bulgu PAİY 20/8-j ve 5/8'in), tablonun %30'un içinde saydığına duvarların kalın kısmı
  eklenir; sınırı aşarsa kaldı (`local_masonry_walls_over`: aşan kısım emsale girer, PAİY 5/8'in okuması iletide);
  imar durumunda emsal yoksa eksik veri; kalın taş duvar yoksa kapsam dışı. Satır "Sivas 22/k", 10.11.2022'den.
- **K140 — Olgulara %30** (`building.exemptCap`): 5/8'in sınırı (parselin emsal alanının %30'u) ve tablonun içinde
  saydığı emsal harici; imar durumunda emsal yoksa yoktur.
- **K141 — Tablo tek sayımla okunur** (`Document::zoning_counted`): olgular sınırlı kullanımları, mahallerin brüt alanını,
  taş duvarları ve %30'u aynı sayımdan alır (önceden üç ayrı hesap).

## §3. Uygulama

- **Alan hesabı:** `Document::zoning_counted`; ADR 8095'in yalnız olguların kullandığı erişimi kaldırıldı.
- **Olgular:** `ExemptCap` (`cap`, `exempt`); SDK'nın şeması yeniden yazıldı.
- **Paket:** `emsal.sivas_duvarlar`, `YEREL`; `data/surumler.json`'a Sivas 22/k; Denetim'in `emsal` katkısı
  `local_masonry_` kodunu da alır.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/emsal.json`, 4 yeni, 40 durum; metinden elle): 60 m² sınırda 16,875 m² geçer; 15 m²
  sınırda 2 m² haricle 18,875 m² kalır (aşan 3,875 m²); emsalsiz eksik veri; Sivas'tan önceki başvuruda satır yok.
- **Masaüstü** (`stone_wall_tests::sivas_counts_the_thick_walls_within_the_30_percent`, ADR 8095'in evi): olgularda %30
  60 m²; Sivas 22/k geçer; 50 m² parselde sınır 15 m², 16,875 m² ile kalır (aşan 1,875 m²), PAİY 5/8 geçer.

## §5. Arayüz

- Yeni pencere yok. Denetim: "Sivas 22/k taş duvarların kalın kısmını (16.88 m²) %30'un içinde sayar: emsal harici
  16.88 m², sınır 15.00 m²; aşan 1.88 m² emsale girer (PAİY 5/8 onları %30'un dışında sayar; sıkı olan bağlar) — Sivas
  22/k".

## §6. Sıradaki adımlar

- **Sivas 22/k'nin metni:** "…" ile yazılan yerler Resmî Gazete'nin PDF'inden tamamlanmalı.
- **Tablonun Sivas katmanı:** Emsal ve TAKS tablosu ulusaldır; Sivas'ın %30 okuması tabloda ayrı sütun değildir.
