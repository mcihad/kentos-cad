# ADR 8142: Bitişik kenar — komşu binaya bitişik duvar; PAİY 39/4, BYKHY 25/1 ve 25/2

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8141](8141-building-layout-nizam.md) §6 ("PAİY 39/4 … bir sonraki dilim").
- **Bağlam:** ADR [8032](8032-site-plan-parcel-setbacks-and-datum.md) (vaziyet planı, kenar türleri, bahçe mesafeleri),
  8141 (yapı nizamı), [8081](8081-fire-compartments.md) (mahaller arası ayrımın okunması),
  [8082](8082-fire-resistance-of-elements.md) (Ek-3 dayanımları), [8090](8090-declared-load-bearing-and-escape-roof.md)
  ve [8137](8137-wall-style-load-bearing.md) (taşıyıcılık beyanı). Kurallar Python'dadır, Rust olguları verir (bim-plan
  K10).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-pppp, 23/1, 39/4; Binaların Yangından Korunması Hakkında Yönetmelik
  25/1, 25/2.
- **Sözleşme:** `SiteEdgeKind`'e `party`; `SitePlan`'ın denetimi bitişik kenarın bahçe mesafesini reddeder
  (`cad.settings.set`'in `arch.site`'ı; yeni komut yok). `kentos.auditFacts` 1'e `building.party`.
  `tr.kentos.mimari-mevzuat` 0.33.0 (yeni denetim `bitisik`). **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 29
  (`settings.arch.site.edges[].kind` `party`). **Web:** dokunulmadı (ADR 0228); TS türü yeniden üretildi
  (`SiteEdgeKind`), web'in Parsel aracına gerekenler TODOS.md §0.2'de.

## §1. Sorun

Vaziyet planının kenarları yalnız bahçeliydi (yan, ön, arka, kamusal alan): bitişik nizamda komşuya bitişik yanın bahçesi
yoktur ama plan orada 3 m isteyip "dar" diyordu; bahçe mesafesi sıfır yazılamaz (sıfırdan büyük olmalı). Komşu parsel
sınırındaki duvarın kuralları — PAİY 39/4 (pencere ve kapı yok) ve BYKHY 25 (yangın duvarı) — hiç okunmuyordu.

## §2. Araştırma (18 Kasım 2026)

PAİY ve BYKHY derlenmiş resmî metinlerinden (mevzuat.gov.tr PDF'leri, `pdftotext`; 8098 ve 8081'in kopyaları) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-pppp-2, -3 | Bitişik nizam bir ya da birden çok komşu parseldeki binaya bitişik olan, ortak alandan arka bahçeye çıkış sağlanan nizam; ikili ve ikiz nizamlar blok sayılır | Bitişik kenar: binanın komşu parseldeki binaya bitişik durduğu kenar; bahçe istemez (K361) |
| PAİY 4/1-pppp-1 | Ayrık nizam hiçbir yanından komşu parseldeki binalara bitişik olmayan nizamdır | Ayrık nizamda bitişik kenar yan bahçe okunur: plan "dar" der, çelişki görünür (K362) |
| PAİY 23/1-b | Yan bahçe mesafesi en az 3.00 m | Bitişik kenar yan bahçe değildir |
| PAİY 39/4 (derlenmiş metinde değişiklik işareti yok; yönetmelik 1.10.2017'de yürürlükte, 71 inci madde) | Bitişik ve blok nizama tabi binalarda komşu parsel sınırı üzerindeki bitişik duvarlarda pencere ve kapı açılamaz | K364 |
| BYKHY 25/1 (değişmedi, 19.12.2007) | Bitişik nizam yapılarını ayıran yangın duvarları en az 90 dakika | EI 90; taşıyıcı beyanlıysa REI 90 (K365) |
| BYKHY 25/2 (değişmedi) | Yangın duvarında delik ve boşluk bulunamaz; kaçınılamayan kapı ve sabit ışık penceresi en az duvarın yarı süresi; kapı kendiliğinden kapanır ve duman sızdırmaz | K366 |
| TS EN 13501-2 | R taşıyıcılık, E bütünlük, I yalıtım; S duman sızdırmazlık, C kendiliğinden kapanma | Paketin `yangin_sinifi.okuma`'sı (8078) |
| ADR 8081 | Ayrım 25 cm adımla, köşeden 30 cm uzakta okunur; değişim 0,1 mm'ye bulunur | Bitişik kenar aynı yolla okunur (K363) |

## §3. Karar

- **K361 — Bitişik kenar:** `SiteEdgeKind::Party`, "Bitişik (komşu bina)": binanın komşu parseldeki binaya bitişik durduğu
  kenar (bitişik nizam, blok ya da ikiz nizamın bitişik yanı). Bahçe istemez (0); bahçe mesafesi yazılamaz ("2. kenar
  bitişik (komşu bina): bahçe mesafesi olmaz"). Parsel aracında kenara tıklama sırası Yan → Ön → Arka → Kamusal →
  Bitişik → Yan. IFC'nin künyedeki ön, yan, arka mesafelerine katılmaz. Mimari verisi 29.
- **K362 — Ayrık nizamda:** yapı nizamı ayrıksa bitişik kenar yan bahçe okunur (ayrık hiçbir yanından bitişik olamaz):
  vaziyet planı "4. kenar (Bitişik (komşu bina)) 0,00 < 3,00 m" der, rozet kırmızıdır; kenar boyunca bir şey okunmaz.
- **K363 — Okuma (`Document::party_walls`):** bitişik kenar boyunca, parselin 5 cm içinde, 25 cm adımla, köşelerden 30 cm
  uzakta, her katta: açıklık (kapı, pencere, boşluk; niş ve baca değil), yoksa duvar, yoksa açık; değişim 0,1 mm'ye
  (8081'in yolu). Parselin içi köşelerinin çizildiği yönden (işaretli alanı) bulunur; kenarın numarası planınki. Bir
  şey duran katlar olgularda (`building.party`: kenar, kat, duvarlar ve açıklıklar beyan edilen sınıflarıyla ve
  uzunluklarıyla, açık uzunluk).
- **K364 — PAİY 39/4 (`bitisik`, `paiy_party_wall_opening`):** bitişik kenardaki pencere ve kapı kaldı (kenar ve kat
  başına bir bulgu, sayılarıyla). Bitişik kenar binanın bitişik ya da blok nizamda olduğunu gösterir: yapı nizamı
  yazılmamış olsa da uygulanır. Boşluk pencere ya da kapı değildir (25/2'nindir).
- **K365 — BYKHY 25/1 (`bykhy_party_fire_wall`):** bitişik kenardaki duvar en az EI 90; taşıyıcı beyanlıysa REI 90;
  sınıfsız eksik veri, azsa kaldı. Değer kural setinde (`minutes` 90).
- **K366 — BYKHY 25/2 (`bykhy_party_wall_opening`):** boşluk kaldı; kapı ve pencerenin sınıfı en az E 45 (25/1'in yarısı;
  `fraction` 0,5) değilse kaldı, beyan edilmemişse eksik veri; kapının sınıfında kendiliğinden kapanma (C) ya da duman
  sızdırmazlık (S) yoksa eksik veri; tutan kapının ve pencerenin kaçınılmazlığı (pencerenin sabit ışık penceresi olduğu)
  uzman incelemesi.

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`SiteEdgeKind::Party`, `has_party_edges`, `SitePlan::problem`);
  `crates/shared/kcad/src/lib.rs`, `decode.rs` (`ARCH_WITH_PARTY_EDGES`, `uses_party_edges`, eski sürümün reddi).
- `crates/native/domain/src/site.rs` (ayrık nizamda yan bahçe), `party_walls.rs` (yeni; `separations.rs`'in okuma
  yardımcıları ortak); `crates/native/interaction/src/map/parcel_tool.rs` (sıra);
  `crates/native/application/src/arch/arch_ifc.rs`.
- `crates/shared/extensions/src/audit_facts/building.rs` (`PartyFacts`), `crates/native/processing/src/audit_facts/
  building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/bitisik.py` (yeni), `data/surumler.json` (PAİY 39/4, BYKHY 25/1,
  25/2), `denetim.py`, `mimari.py`, `manifest.json`; `scripts/fixtures/party_wall_cases.py`.

## §5. Sınamalar

- **Kural** (`test_bitisik.py`, 16 durum, 39/4 ve 25'ten elle; `scripts/fixtures/party_wall_cases.py`): bitişik kenarı
  olmayan bina kapsam dışı; EI 90 duvarlar, açıklık yok: üçü geçer; sınıfsız pencere 39/4 kaldı, 25/2 eksik veri;
  EI2 60-S200C5 kapı 25/2 uzman incelemesi, EI 60 kapı (C, S yok) eksik veri, E 30 kapı kaldı; boşluk 39/4'te değil,
  25/2'de kaldı; E 60 pencere uzman incelemesi (sabitliği); iki pencere bir kapı tek 39/4 bulgusu; sınıfsız duvar eksik
  veri, EI 60 kaldı; taşıyıcı EI 90 (R yok) kaldı, REI 120 geçer; iki bitişik kenarın duvarları ayrı; 1.6.2017
  başvurusunda 39/4, 1.6.2006'da üçü de uzman incelemesi. Bütün `mimari-mevzuat` sınamaları yeşil.
- **Ortak durumlar:** `cad.settings.set` 139 (2'si yeni: bitişik kenar yazılır; bahçe mesafeli bitişik kenar
  `arch.site.edges[1].setback`'te reddedilir).
- **`.kcad`:** `a_party_edge_round_trips_under_arch_data_29` (yazılır, okunur; 28 diyen aynı dosya reddedilir); `arch_`
  sınamaları (30) yeşil. Olguların altı değişmedi (örnek binanın vaziyet planı yok).
- **Masaüstü** (`party_wall_tests`): iki katlı binanın batı duvarlarının dış yüzü parselin batı kenarında; Parsel batı
  kenarını dört tıkla Bitişik yapar: plan orada bahçe istemez (gereken 0, mahallerin çizgisi 0,20 m); olgularda 4. kenarda
  zemin katın duvarı, 1. katın duvarı ve penceresi. Denetim: 39/4 kaldı ("4. kenar (bitişik), 1. kat: … 1 pencere"),
  25/1 ve 25/2 eksik veri; duvarlar EI 90 beyan edilip pencere silinince üçü geçer; ayrık nizamda kenar dar ("4. kenar
  (Bitişik (komşu bina)) 0,20 < 3,00 m"), olgular boş, üç madde kapsam dışı. `regulation_dates_tests`'in kural seti
  satırında üç madde kapsam dışı (33). Ortak süzgeçlerle (modül, şerit, geçmiş, katalog, ayarlar, Öznitelikler, durum
  çubuğu, mevzuat, vaziyet planı, ruhsat, çatı, kaplama, kazan dairesi, kompartıman) sınamalar yeşil.
- **Resimler:** `.run/gui-evidence/bitisik-8142/` (`party_wall_tests::screens`): Parsel açıkken batı kenarı Bitişik
  (araçta "Bitişik (komşu bina) 0,00 m · binada 0,20 m", yaklaşma sınırı kenarın üstünde); Denetim'de 39/4'ün bulgusu;
  açık ve koyu.

## §6. Kalanlar

- Bitişik nizamda ortak alandan arka bahçeye çıkış (4/1-pppp-2): binanın ortak alanı ve arka bahçesi çizimde yok.
- BYKHY 25/2'nin tesisat geçişleri; 27/2-c (farklı yükseklikteki bitişik binalarda yüksek binanın alçak binanın çatı
  hizasındaki cephe kaplaması); 28/1-f (çatıdan komşu çatıya yangın).
- PAİY 41: bitişik nizamda çıkma bitişik olduğu komşu sınırına 2,00 m'den fazla yaklaşamaz.
- Bitişik kenar boyunca açık kalan kısım (bina komşu sınırına bir kısmıyla dayanıyor) bulgu değildir; olgularda uzunluğu
  var.
