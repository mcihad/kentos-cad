# ADR 8080: Merdiven altı 1,80 m — net alandan düşülmesi ve kapatılması

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("merdiven altı 1,80 m");
  [architecture-advanced.md](../../plan/architecture-advanced.md) ADV-22'nin kalanı; ADR
  [8029](8029-regulation-areas-of-independent-units.md)'un "Merdiven altında 1,80 m'den alçak yerin net alandan
  düşülmesi (merdivenin kesitinden)" sonraki adımı.
- **Bağlam:** ADR [8029](8029-regulation-areas-of-independent-units.md) (bağımsız bölüm alanları), [8042](8042-section-derived-from-a-section-mark.md)
  §3 (kesitte merdivenin gövdesi), [8045](8045-balanced-stairs-and-special-elements.md) §2 (balansmanlı merdiven),
  [8071](8071-geometry-accuracy.md) §9 (eğri merdiven), [8078](8078-regulation-rule-set-by-application-date.md) (kural seti),
  [8079](8079-emsal-article-22-limits.md) (emsal).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-ğ-3 ve 31/6 (RG 3.7.2017/30113; 1.10.2017'den yürürlükte, 71 inci
  madde; 1.7.2026 derlemesinde ikisinde de değişiklik işareti yok).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  isteğe bağlı `underStairs`.

## §1. Araştırma

Metin yönetmeliğin 1 Temmuz 2026 derlemesinden (`planli-alanlar-01-07-2026.pdf`, pdftotext ile) okundu:

- **4/1-ğ-3 (bağımsız bölüm net alanı):** "… Bu alana; kapı ve pencere eşikleri, 2.5 santimetreyi geçmemek koşuluyla sıva
  payları, kolonlar, … merdivenlerin altlarında 1.80 metre yüksekliğinden az olan yerler, … dâhil edilmez."
- **31/6:** "Yüksekliği 1.80 metreden az olan merdiven altları kapatılır."
- **28/5** (ADR 8040 §3): merdiven altı bir mahal olarak çizilirse iç yüksekliği en az 2,20 m. Bu, kapatılan alçak yerin
  kendisini değil, merdiven altında kurulan mahali söyler; bu ADR'nin konusu değil.
- **Yükseklik nereden:** yönetmelik merdivenin altındaki yerin yüksekliğini söyler; ölçünün yolu merdivenin alt yüzüyle
  altındaki döşeme arasıdır. KentOS'ta merdivenin alt yüzünü kesit modeli çiziyordu (ADR 8042 §3): basamakların kökünden
  dik ölçülen bel kalınlığı (vardığı döşemenin taşıyıcı kalınlığı), sahanlık o kalınlıkta, balansmanlı basamaklar ve
  eğri merdivenin basamakları tek tek.
- **Benzer araçlar:** net alan hesabında merdiven altı düşümünü gösteren araçlar kesit ya da 3B gövdeden okur; KentOS
  da gövdeden okur, ayrı bir çizim istemez.

## §2. Karar

- **K70 — Merdivenin gövdesi tek yerde:** gövdenin geometrisi (kollar, balansmanlı basamaklar, eğri merdivenin
  basamakları ve dikmesi, sahanlıklar) kesit kodundan geometri çekirdeğine taşındı
  (`kentos_geometry_core::geom::stair_body`). Kesit onu çizer, alan hesabı onun alt yüzünü okur. Bel kalınlığını
  `Document::stair_waist` bulur (vardığı döşeme, yoksa başladığı; ±0,5 m içinde), kesit de onu kullanır.
- **K71 — 1,80 m'den alçak yer** (`stair_body::low_under`, `Document::under_stairs`): merdivenin başladığı katın
  döşemesinden 1,80 m'den alçak kalan yerler.
  - **Kol:** başından, alt yüzünün ve basamaklarının 1,80 m'ye ulaştığı yere kadar. Alt yüzün döşemede olduğu baş kısmı
    dolu, gerisi boşluk.
  - **Sahanlık, balansmanlı ve eğri basamak:** altı 1,80 m'nin altındaysa hepsi. Altı döşemenin üstündeyse boşluk.
  - **Bel kalınlığı bilinmiyorsa** (döşeme yok) yalnız kesin olan sayılır: üstü 1,80 m'den alçak basamakların ve
    sahanlıkların altı, dolu sayılır (kesit de gövdeyi döşemesine kadar dolu çizer).
- **K72 — Net alandan düşülür** (4/1-ğ-3): her grubun (bağımsız bölüm, eklenti, ortak alan) net alanından, mahallerinin
  net konturunun bu yerlerin üstünde kalan kısmı düşülür (`PaiyGroup::under_stair`). Metrekare cetveli, Kat irtifakı
  tablosu ve emsalin %20 ve %50 payları aynı neti okur. Mahalin kendi net alanı (oda listesi, 29/1'in piyes alanları)
  değişmez.
- **K73 — Kapatılır** (31/6, paketin kuralı `merdiven_alti`, K10):
  - **Kaldı** (`paiy_under_stair_open`): alçak boşluk adı dolap, depo, kiler, ardiye, vestiyer ya da merdiven altı
    olmayan bir mahalin içinde. O mahale açık sayılır; bulgu merdiveni ve mahali adlandırır.
  - **Geçti:** alçak boşluk böyle kapalı bir mahalde ya da hiçbir mahalin içinde değil (duvarla ayrılmış).
  - **Eksik veri** (madde satırı, bulgu değil): döşemesi olmayan merdivenin alt yüzü okunmuyor.
  - 0,01 m²'den küçük parça açık sayılmaz (sınır gürültüsü). Kural seti 31/6'yı 1.10.2017 hâliyle okur; daha eski
    başvuru günü uzman incelemesidir (K61).
- **Olgular:** `building.underStairs`: merdiven, katı, alçak yerlerin ve boşlukların alanı, bel kalınlığının bilinip
  bilinmediği, boşluğun içinde kaldığı mahaller (kimlik, ad ve numara, alan).
- **Taban:** merdivenin başladığı katın döşemesi (katın kotu). Mahalin döşeme kot farkı (ADR 8040 §3) okunmaz.

## §3. Uygulama

- **Çekirdek:** `geom/stair_body.rs`: `StairPart` (Flight, Prism), `stair_body`, `band_cut`, `clip`, `low_under`,
  `UNDER_STAIR_CLEAR`. Kesitin `curved_stair`, `landing_ring`, `landing_cut` ve `clip`'i kalktı; rampanın sahanlığı da
  çekirdeğin `band_cut`'ını kullanır.
- **Belge:** `crates/native/domain/src/under_stair.rs` (`UnderStair`, `stair_waist`, `under_stairs`,
  `under_stair_rooms`, `low_on`, `taken`); `paiy_areas.rs` ve `zoning.rs` net alanı onunla düşer.
- **Olgular:** `kentos_extensions::audit_facts::building::{UnderStairFacts, UnderStairRoom}`; `factsSchema` yeniden
  yazıldı; `fixtures/audit-facts/v1/mimari.json`'da planın ortak merdiveni (döşemesiz: kesin kısım 3,168 m²) ortak alanın
  netinden düşüldü.
- **Paket:** `tr.kentos.mimari-mevzuat` 0.4.0: `merdiven_alti.py`, `data/surumler.json`'a PAİY 31/6, manifestte
  `merdiven-alti` denetimi.

## §4. Sınamalar

- **Çekirdek** (`stair_body::tests`, elle):
  - 18 × 0,17 m rıht, 0,28 m basamak, 0,15 m bel: alt yüz döşemeden 0,289 m'de ayrılır, 1,80 m'ye 3,254 m'de ulaşır.
  - Bel yoksa 11. basamağa (1,87 m) kadar, 2,80 m.
  - U merdivenin 1,53 m'deki sahanlığı alçak boşluktur.
- **Masaüstü** (`under_stair_tests`, dubleks, elle):
  - Alçak yer (1,80 + 0,175 48)/(0,17/0,28) = 3,253 7 m²; boşluk 1,80 × 0,28/0,17 = 2,964 7 m².
  - Dairenin neti 2 × 42,75 − 3,253 7; zemin katın payı 42,75 − 3,253 7, üst katınki 42,75.
  - Metrekare cetveli zemin katta 39.50 yazar.
  - Denetim: salona açık boşluk `paiy_under_stair_open` (2.96 m², kaldı); aynı mahal Depo adını alınca madde geçer.
- **Paket** (`fixtures/standards/v1/merdiven-alti.json`, 18 durum, 31/6'nın metninden elle;
  `catalog/standards/tests/mimari-mevzuat/test_merdiven_alti.py`):
  - Açık: salon, merdiven evi, iki mahal birden.
  - Kapalı: dolap, depo, kiler (büyük harf), vestiyer, merdiven altı; hiçbir mahalde değil; küçük parça.
  - Eksik veri: döşeme yok. Tarih: tarihsiz; 2016 başvurusu uzman incelemesi.
- **Değişmeyenler:** uygulamanın 843 sınaması (kesit ve IFC'nin merdivenleri ve sabitlenmiş dosyaları) gövde
  taşındıktan sonra da aynı.

## §5. Sıradaki adımlar

- Planda alçak yerin gösterimi (kesik çizgiyle "h < 1,80"), istenirse.
- Mahalin döşeme kot farkının ve merdivenin altındaki döşemenin taban olarak okunması.
- Kapatmanın çizgiyle (mahal ayırıcı, duvar) okunması: bugün mahal adıyla ve mahal sınırıyla okunuyor.
