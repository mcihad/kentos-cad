# ADR 8139: Çatının oturduğu döşeme — BYKHY 28/3-a

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8134](8134-roof-covering-fire-class.md) §6 ("kalan 28/3-a").
- **Bağlam:** ADR [8134](8134-roof-covering-fire-class.md) (28/2, 28/3-b, `catilar`), [8136](8136-protected-shafts-and-fire-stops.md)
  (yangın kesici, Ek-3/B 10 EI 30), [8082](8082-fire-resistance-of-elements.md) (döşemenin Ek-3/B 3 dayanımı, `dayanim`),
  [8044](8044-roof-and-roof-plan.md) (çatının saçağı katının üstünde). Kurallar Python'dadır, Rust olguları verir (bim-plan K10).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 28/3-a (10.8.2009-2009/15316 ile değişik), 4/1-pp,
  Ek-3/B 10.
- **Sözleşme:** belge ve komut değişmedi (`cad.arch.audit`'in açıklaması yeni kodu sayar). `kentos.auditFacts` 1'e
  `building.roofs[].seat`. **`.kcad`:** değişmedi. **Web:** değişmedi.

## §1. Sorun

28/3'ün b bendi (8134) okunuyordu; a bendi ("Çatıların oturdukları döşemelerin yatay yangın kesici niteliğinde") 8134'te
döşemenin kendi Ek-3/B dayanımına bırakılmıştı. Ama altında döşeme çizilmemiş bir çatı (merteği duvara oturan ahşap
çatı) hiç okunmuyordu: Ek-3 olmayan döşemeyi denetleyemez.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 28/3 | "Yüksek binalarda ve bitişik nizam yapılarda; a) Çatıların oturdukları döşemelerin yatay yangın kesici niteliğinde, b) Çatı taşıyıcı sistemi ve çatı kaplamalarının yanmaz malzemeden, olması gerekir." | a) çatının altında döşeme olmalı ve yatay yangın kesici niteliğinde olmalı (K346, K347) |
| BYKHY 4/1-pp | "Yangın kesici: Bina içinde, yangının ve dumanın ilerlemesini ve yayılmasını belirlenmiş bir süre için durduran, yatay veya düşey konumlu elemanı" | Yatay yangın kesici: yangını ve dumanı durduran döşeme |
| BYKHY Ek-3/B 10 | Yangın kesici EI 30, ayrı ayrı her bir yüzey | "Niteliğinde": sınıfı en az EI 30 (K347); yangın kesici beyanı (8136) aranmaz, nitelik sınıftan okunur |
| ADR 8044, `geometry::roof_shape` | Çatının saçağı katının üstünde (`storey_top`) ve kendi kot farkında | Çatının oturduğu yer katının üstüdür; döşemenin üstü kat kotu ve kendi kot farkıdır (8071) |
| 8134 §3 (28/3-b) | Nizam çizimde yok: yüksek olmayan binada tutmayan çatı uzman incelemesi | a bendi de aynı (K347) |

## §3. Karar

- **K346 — Oturduğu döşeme:** bir çatının oturduğu döşemeler, rolü kat döşemesi ya da teras olan (balkon değil),
  kaplama ve eğim olmayan, üstü (katının kotu ve kot farkı) çatının katının üstünde (1 cm içinde) ve dış çizgisi
  çatının dış çizgisinin ortasını tutan döşemelerdir (olguların `roofs[].seat`'i). 1 cm, aynı kotta çizilmiş iki
  değerin çizim payıdır; hesap toleransı değildir.
- **K347 — 28/3-a (`catilar._oturdugu`, `bykhy_roof_seat_fire_stop`):** yüksek binada (4/ccc) çatının oturduğu döşeme
  yoksa ya da sınıfı beyan edilmemişse eksik veri; sınıfı E ve I ile en az 30 değilse (E 30, R 60) kaldı. Yüksek
  olmayan binada bunlar uzman incelemesidir (bitişik nizamda uygulanır, nizam çizimde yok). Değer kural setinde
  (`seat` 30). Döşemenin kendi dayanımı (Ek-3/B 3) `dayanim`'da kalır.
- **K348 — Satır:** 28/3'ün satırı iki bendin bulgularından; başlığı "Yüksek ve bitişik nizam binada çatının döşemesi ve
  yanmazlığı". Önce her çatının a bendi, sonra b bendi.

## §4. Uygulama

- `crates/shared/extensions/src/audit_facts/building.rs` (`RoofFacts.seat`); `crates/native/processing/src/audit_facts/
  building.rs` (`seat_candidates`, `seat_slabs`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/catilar.py` (`_oturdugu`), `data/surumler.json` (28/3'ün başlığı,
  `seat`); paket 0.31.0.
- `scripts/fixtures/roof_fire_cases.py` (her çatının REI 60 döşemesi; 28/3-a durumları).

## §5. Sınamalar

- **Python** (`test_catilar.py`, 29 durum; 6'sı yeni, 28/3 ve Ek-3/B 10'dan elle): yüksek binada döşemesiz çatı eksik
  veri, E 30 kalır, sınıfsız eksik veri, tam EI 30 geçer; yüksek olmayan binada döşemesiz ve R 60 uzman incelemesi. Eski
  23 durumun çatıları REI 60 döşemeye oturur: sonuçları değişmedi.
- **Masaüstü** (`roof_fire_tests`): 8134'ün binasında çatı 1. katın üstünde REI 60 döşemeye oturur (sınaması
  değişmedi); sekiz katlı kulede (24 m) döşemesiz çatı eksik veri ("7. kat: çatının altında, katının üstünde döşeme
  çizilmemiş"), altına çizilen döşeme olgularda `seat`, E 30 ile kalır, EI 60 ile geçer. Ortak süzgeçlerle (modül,
  şerit, geçmiş, katalog, ayarlar, Öznitelikler, mevzuat, kaplamalar) 251 sınama yeşil; olguların altını değişmedi.
- **Resimler:** `.run/gui-evidence/cati-dosemesi-8139/` (`roof_fire_tests::seat_screens`): kulenin 7. katı ve Denetim'de
  28/3-a'nın bulgusu; açık ve koyu.

## §6. Kalanlar

- Bitişik nizam: binanın nizamı İmar durumu'na yazılırsa yüksek olmayan binanın uzman incelemesi kalkar.
- 28/4 (orman alanları, 21/5): binanın orman alanında olduğu çizimde yok.
- Eğimli döşemeye oturan çatı (çatı katı): eğimli döşeme bugün oturduğu döşeme sayılmaz.
