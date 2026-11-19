# ADR 8165: Çatı örtüsünü aşan duvarlar — PAİY 40/5'in merdiven evi, ışıklık, alın ve kalkan duvarı

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8161](8161-eaves-within-the-parcel.md) §6 ve [8164](8164-eaves-past-projections.md) §6 ("40/5'in öbür çıkıntıları:
  merdiven evi, ışıklık, alın ve kalkan duvarları").
- **Bağlam:** ADR 8044 §6 (çatının parçaları: hava bacası), 8159 (çatı kuralları pakette; 40/5'in hava bacası), 8040
  (duvarın yüksekliği: kendi yüksekliği, yoksa üstündeki döşemeye; ayağı), 8087 (asansör kuyusu: Asansör adlı mahal).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 40/4, 40/5, 40/6 (RG 3.7.2017/30113; mevzuat.gov.tr'nin 1.7.2026'ya kadar
  işlenmiş birleştirilmiş metni).
- **Sözleşme:** yeni komut yok. `roof_walls::walls_over`, `RoofMeasure.walls`; `kentos.auditFacts` 1'in
  `roofs[].form`'una `walls` (`uid`, `kind`, `over`, `at`). `tr.kentos.mimari-mevzuat` 0.52.0 (yeni bulgu kodu
  `paiy_roof_wall_high`). **`.kcad`:** değişmedi. **Web:** dokunulmadı.

## §1. Sorun

40/5 yalnız hava bacasıyla okunuyordu (8159, çatının parçası). Merdiven evinin, ışıklığın ve alın, kalkan duvarlarının
çatı örtüsünü aşması ölçülmüyordu: çatının ortasından 1 m yükselen merdiven evi denetimden geçiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 40/5 (birleştirilmiş metin; 25.2.2022 ek ibaresi fotovoltaik panellerin) | "Merdiven evleri, ışıklıklar, hava bacaları, alın ve kalkan duvarları dördüncü fıkraya göre belirlenen çatı örtüsü düzlemlerini en fazla 0.60 metre aşabilir." | Duvarın üstünün çatı örtüsünden yüksekliği, en çok 0,60 m (K426) |
| PAİY 40/5, son cümle | Asansörlü binalarda TSE standartlarına göre projelendirilecek asansör kulelerinin ve bu bölümlerle birlikte düzenlenen merdiven evlerinin en az ölçülerdeki bölümlerinin örtüyü aşmasına izin verilir | Asansör kulesi muaf; asansörlü binada aşan merdiven evi uzman incelemesi: en az ölçüyü çizim söylemez (K427) |
| PAİY 40/4 | Mahya yüksekliği ayrık binada kırma, iki taraftan bitişikte beşik çatı varsayılarak belirlenir | Örtü düzlemi çizilen çatınınkidir: varsayılan çatının düzlemini KentOS kurmaz (K426; §6) |
| PAİY 40/6 | Teras çatılarda 1,10 m parapet yapılabilir | Düz çatının parapeti 40/5'in değil: düz çatı okunmaz (K426) |
| Ordu Büyükşehir Belediyesi İmar Yönetmeliği'nin tanımı (alomaliye.com, 3.12.2024; arama özeti) | Kalkan duvarı: beşik çatılı yapının mahyaya dik duvarındaki üçgen bölümün alın duvarı | Alın ve kalkan duvarı, merdiven evi, ışıklık ve asansör kuyusu olmayan duvar (K426) |
| ADR 8040 | Duvarın ayağı ve yüksekliği (kendi yüksekliği, yoksa üstündeki döşeme ya da kiriş, katın yüksekliği) | Duvarın üstü = ayağı + yüksekliği (K426) |

## §3. Karar

- **K426 — Ölçü:** eğimli çatının dış hattının içindeki her duvar: üstü (ayağı ve yüksekliği) ile o noktadaki çatı
  örtüsü (saçak kotu ve düzlemlerin yükselişi, `height_at`) arasındaki fark; duvarın iki yüzünün köşelerinde ve yüzün
  saçak çizgisini kestiği yerde (1 cm içeride), çatının altındaki noktalar; en büyüğü ve yeri. Örtü düzlemlerin en
  alçağıdır (içbükey): duvarın taban izinde en büyük fark bu noktalardadır. Örtüyü 1 mm'den çok aşmayan duvar yazılmaz.
  Türü, duvarın ortasından iki yüzüne 30 cm ötede kendi katındaki mahallerden: Asansör (`elevator`), Merdiven …
  (`stair`), Işıklık ya da Aydınlık (`lightWell`), hiçbiri (`other`: alın, kalkan, yangın duvarı). Düz çatı yazılmaz.
- **K427 — Kural:** 0,60 m'yi aşan merdiven evi, ışıklık, alın ya da kalkan duvarı kaldı (`paiy_roof_wall_high`:
  "Merdiven evi duvarının çatı örtüsünü aşan yüksekliği 1.00 m > 0.60 m — PAİY 40/5 (1.10.2017)", nesnesi duvar);
  asansör kulesinin duvarı geçti (izin verilir); asansörlü binada (bir katında asansör) aşan merdiven evi uzman
  incelemesi. 40/5'in satırı hava bacalarını ve duvarları birlikte sayar. 40/5'in sürüm metni asansör cümlesini de
  taşır.

## §4. Uygulama

- `crates/native/application/src/arch/roof_walls.rs` (yeni: `walls_over`, `WallOver`, `WallOverKind`),
  `roof_audit.rs` (`RoofMeasure.walls`).
- `crates/shared/extensions/src/audit_facts/building.rs` (`RoofWallFacts`), `crates/native/processing/src/audit_facts/
  building.rs`.
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cati_olculeri.py`, `data/surumler.json` (40/5'in başlığı ve
  metni).

## §5. Sınamalar

- **Birim** (`roof_walls`): saçak çizgisini kesen yüz 1 cm iki yanında okunur; yolun ortası ve doğrultusu.
- **Kural** (`test_cati_olculeri.py`): 32 bağımsız durum (5'i yeni, 40/5'ten elle: merdiven evi 1,00 m kalır, aynısı
  asansörlü binada uzman incelemesi, kalkan duvarı 0,40 m geçer, asansör kulesi 1,50 m geçer, ışıklık 0,80 m kalır).
  Sınama bulgunun nesnesini ve durumunu da okur.
- **Masaüstü** (`roof_rules_tests`): iki katlı konutun 1. kat çatısı (10 × 8 m, %50 kırma, saçak +6,00) ve Merdiven
  evi'nin güney duvarı (x 6–8, y 5, 20 cm; eğim oklarından uzakta): 5,00 m'lik duvar +8,00'de biter, doğu ucunda örtü
  (doğu saçağından 2,00 m içeride) +7,00: 1,00 m aşar, 40/5 kaldı; 4,50 m'de 0,50 m geçti; binaya Asansör eklenince uzman
  incelemesi.
- **Resimler:** `.run/gui-evidence/cati-duvar-8165/`: Denetim'de merdiven evi duvarının bulgusu, açık ve koyu.

## §6. Kalanlar

- 40/4'ün varsayılan çatısı (ayrıkta kırma, bitişikte beşik) çizilen çatıdan başkaysa örtü düzlemi çizilenindir.
- 40/5'in tesisat hacimleri, fotovoltaik paneller, güneş enerjili su ısıtıcıları ve çatı pencereleri idarenin teknik
  gereğine bırakılmıştır: ölçülmez.
- Düz çatının parapeti (40/6, 1,10 m) ayrı madde.
