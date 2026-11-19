# ADR 8159: Çatı kuralları pakette — PAİY 40/4, 40/5, 42/1, BYKHY 58/2, Sivas 40/5, 42/5

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15 (K10: "bugünkü Rust
  kuralları dokunuldukça betiğe taşınır"); ADR [8084](8084-plan-note-layer.md) §6 ("çatı eğimi, mahya ve saçak (40, 42;
  Rust'taki kurallar taşınınca)").
- **Bağlam:** ADR 8044 §9 (çatının mevzuat denetimi, Rust'ta; düzeltmeleri `cad.arch.edit`'in çatı işlemi), 8078 (paket,
  kural seti), 8134 (olguların çatıları: katmanları), 8139 (çatının oturduğu döşeme).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 40/4, 40/5, 42/1 (RG 3.7.2017/30113); Binaların Yangından Korunması
  Hakkında Yönetmelik 58/2 (RG 19.12.2007/26735); Sivas Belediyesi İmar Yönetmeliği 40/5-e, 40/5-g, 42/5 (RG
  10.11.2022/32009).
- **Sözleşme:** yeni komut yok; bulgu kodları değişmedi. `kentos.auditFacts` 1'in `building.roofs[]`'ine `form`
  (`RoofFormFacts`, `RoofPartFacts`, `WindowGapFacts`). `kentos_native_application::roof_audit::measures`. Paketin
  bulgusu başsız denetimde de yerini (`at`) ve düzeltmesini (`fix`: `label`, `command`, `input`) taşır (masaüstü zaten
  okuyordu). `tr.kentos.mimari-mevzuat` 0.46.0 (`cati_olculeri`). **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Çatının mevzuat kuralları Mimari denetimin Rust kodundaydı (ADR 8044 §9). K10 yeni kuralı Rust'a yazdırmaz, dokunulan
Rust kuralını betiğe taşır; planın çatı notları (eğim, mahya, saçak; 42/1: "Uygulama imar planında belirlenmemiş ise")
ancak kurallar paketteyse okunabilir. Rust kurallarının düzeltmeleri vardı (hava bacası, duman bacası, eğim, kar
siperliği); paketin bulgusu başsız denetimde düzeltme taşımıyordu: taşıma onları kaybetmemeliydi.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 40/4, 40/5, 42/1 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin) | Mahya 5,00 m'yi geçmez; hava bacası ve benzerleri çatı örtüsü düzlemini en çok 0,60 m aşar (25.2.2022 ek ibaresi fotovoltaik panelleri ekledi, ölçü değişmedi); plan belirlemediyse saçak en çok 1,00 m | Sürüm tablosunda 1.10.2017 hâlleri (K409) |
| BYKHY 58/2 (mevzuat.gov.tr derlenmiş metni) | Bacalar ait olduğu bina mahyasının en az 0,8 m üzerine çıkarılır; 58'e 2009'da eklenen 12. fıkra 2'yi değiştirmedi | 19.12.2007 hâli (K409) |
| Sivas İmar Yönetmeliği 40/5-e-2, e-3, g; 42/5 (Lexpera'nın metni) | "Çatı yüzeyine paralel pencere ölçüleri (en çok): 1.20 mX1.50 m (en/boy)"; pencereler cephe hatlarına 1 m'den, birbirine 2 m'den fazla yaklaşamaz; "mahya yüksekliği 4.00 m'yi geçmemek kaydıyla çatı yüzeyi en fazla % 40 eğimle"; saçak uçlarında kar siperliği mecburi | 10.11.2022 hâlleri (K409) |
| ADR 8044 §9 | Ölçüler ve iletiler, düzeltmeler (`cad.arch.edit`: parçalar, eğimler, kar siperliği) | Aynı ölçü, aynı ileti (PAİY'ninki hâlin günüyle), aynı düzeltme (K410, K411) |
| `apps/desktop/src/arch/arch_audit/standards.rs` | Paketin bulgusundaki `fix` (label, command, input) ve `at` Denetim'in düzeltmesine ve yerine okunur | Başsız eşleyici de aynısını okur (K411) |

## §3. Karar

- **K409 — Kural seti:** PAİY 40/4 (5,00 m), 40/5 (0,60 m), 42/1 (1,00 m), BYKHY 58/2 (0,80 m), Sivas 40/5-g (4,00 m,
  %40), 40/5-e-2 (1,20 × 1,50 m), 40/5-e-3 (1 m, 2 m), 42/5. Çatı aracının kar siperliği (saçaktan 30 cm) yönetmeliğin
  değeri değildir: paketin sabitidir.
- **K410 — Ölçü Rust'ta, karar pakette:** `roof_audit::measures` her çatının biçimini verir: eğimli mi, mahya (saçak
  seviyesinden, kot farkı dahil), saçak (duvarlarından çizilmişse), kenarların eğimi, kar siperliği, ortası; parçaları
  çizilen değerleriyle, baca tepeleri (kendi yükselişi, yoksa türününki) ve pencerenin cephe hatlarına uzaklığı; iki
  pencere arası (çatının üstünde en kısa). Olgularda `roofs[].form`. Mimari denetimin Rust kuralları ve kodları
  kaldırıldı.
- **K411 — Kural (`cati_olculeri`) ve düzeltme:** mahya PAİY 40/4, Sivas seçiliyse 40/5-g ile iki katman
  (`katman.bulgu`); saçak 42/1; hava bacası 40/5 (düzeltme 0,60 m'ye indirir), duman bacası 58/2 (düzeltme 0,80 m'ye
  çıkarır); Sivas'ta eğim (düzeltme dik kenarları %40'a indirir), kar siperliği (düzeltme verir), pencere ölçüsü ve yeri.
  Düz çatının mahya, eğim, kar siperliği ve pencere yeri kuralı yoktur. Düzeltme `cad.arch.edit`'in çatı üstündeki
  işlemidir; parçalar olgulardaki değerleriyle yazılır, yalnız düzeltilen değişir. Bulgu yerini taşır (çatının ortası,
  parçanın yeri). Satırlar: çatısız, eğimli çatısız, bacasız proje kapsam dışı. Başsız denetim paketin `fix`'ini ve `at`'ini
  okur.

## §4. Uygulama

- `crates/native/application/src/arch/roof_audit.rs` (ölçüler; bulgular kalktı), `regulation_audit.rs` (çağrı ve
  kodlar).
- `crates/shared/extensions/src/audit_facts/building.rs`, `crates/native/processing/src/audit_facts/building.rs`.
- `crates/native/headless/src/regulation.rs` (`fix`, `at`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cati_olculeri.py` (yeni), `mimari.py`, `denetim.py`
  (`cati-olculeri` katkısı), `manifest.json`, `data/surumler.json`.

## §5. Sınamalar

- **Kural** (`test_cati_olculeri.py`): 16 bağımsız durum (mahya içinde ve aşan, Sivas'ta 4,00 m, saçak 1,20 ve tam
  1,00, hava bacası 0,90 düzeltmesiyle, duman bacası 0,50 düzeltmesiyle ve yazılmamış yükselişle 0,80, Sivas eğim
  düzeltmesiyle, kar siperliği düzeltmesiyle, pencere ölçüsü, cepheye ve birbirine uzaklık, düz çatı, çatısız, Sivas'ın
  gününden önce); düzeltmenin parçaları çizilen değerleriyle; eğimin yazılışı.
- **Uygulama** (`roof_sections`): 8044'ün çatısı ölçülür (mahya 4,47, saçak 1,20, eğimler %75, baca tepeleri 0,90 ve
  0,50, pencere cepheye 0,66, iki pencere arası 1,50); Mimari denetimin kendi Mevzuat grubu çatı bulgusu vermez.
- **Masaüstü** (`roof_rules_tests`): aynı çatı Denetim'de paketle: PAİY'de üç bulgu, Sivas'ta dokuz (iletileri 8044'ün
  aynısı, PAİY'ninki hâlin günüyle); kar siperliği ve duman bacası Düzelt'le giderilir, öbür parçalar çizildiği gibi.
- **Resimler:** `.run/gui-evidence/cati-olculeri-8159/`: Denetim'de çatının bulguları, açık ve koyu.

## §6. Kalanlar

- Planın çatı notları (eğim, mahya, saçak; 42/1 "Uygulama imar planında belirlenmemiş ise"): yeni plan notu konuları
  — yapıldı: ADR [8160](8160-plan-roof-notes.md).
- 40/5'in öbür çıkıntıları (merdiven evi, ışıklık, alın ve kalkan duvarları), 42/2–3 (saçak parsel sınırı, çıkmadan
  0,50 m) — 42/2 yapıldı: ADR [8161](8161-eaves-within-the-parcel.md).
