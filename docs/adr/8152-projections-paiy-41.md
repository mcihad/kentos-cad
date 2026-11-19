# ADR 8152: Çıkmalar — PAİY 41/1 ve 41/2

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15 ("kalan PAİY 41,
  6"); ADR [8032](8032-site-plan-parcel-setbacks-and-datum.md) §6 ("Açık çıkmaların bahçe mesafesine taşma kuralları"),
  [8142](8142-party-edge-and-fire-wall.md) §6.
- **Bağlam:** ADR 8032 (vaziyet planı: parsel, kenarların bahçe mesafeleri, yapı yaklaşma sınırı, taban alanının izi),
  8142 (bitişik kenar; ayrık nizamda yan bahçe), 8069 (dış mekân: kullanımın kartının `outdoor`'u), 8040 (döşemenin
  katmanları), 8072 (binanın zemin katı).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 41/1, 41/2 (RG 3.7.2017/30113; 41/1-b-5 RG 30.9.2017/30196 ile mülga),
  4/1-şşşş (yapı yaklaşma mesafesi), 23 (bahçe mesafeleri), 57/3 (çıkma izdüşümleri projede).
- **Sözleşme:** yeni komut yok. `kentos_domain::Document::projections` (`Projection`, `ProjectionEdge`).
  `kentos.auditFacts` 1'in `building`'ine `projections` (`ProjectionFacts`, `ProjectionEdgeFacts`).
  `tr.kentos.mimari-mevzuat` 0.40.0 (`cikmalar`). **`.kcad`:** değişmedi. **Web:** dokunulmadı; TODOS.md §0.2'de.

## §1. Sorun

Vaziyet planı taban alanının bahçe mesafelerine yaklaşmasını okuyordu (8032), üst katların taban alanı dışına taşan
balkonları ve kapalı çıkmaları okumuyordu: ön bahçeye 2 m taşan balkon, yan sınıra 2,5 m yaklaşan kapalı çıkma, altı
2,40 m'den alçak çıkma denetimden geçiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 41/1 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin) | Taban alanı dışında, kendi bahçe hudutları dışına taşmamak şartıyla her cephede açık ve kapalı çıkma; yol cephesinde parsel içinde yapı yaklaşma sınırından en çok 1,50 m (a-1, b-1); yan ve arka bahçede parsel sınırına 3,00 m'den fazla yaklaşmamak kaydıyla 1,50 m (a-2, b-2); taban sınırdan içeri çekilirse bu mesafeleri aşmadan istenen ölçüde (a-3, b-3); bitişik nizamda açık çıkma komşu sınırına 2,00 m'den fazla yaklaşamaz (b-4) | K392, K393 |
| PAİY 41/2 | Tabii ya da tesviye edilmiş zeminden çıkma altına en az 2,40 m; ön bahçesi 3 m ve daha az parselde yol kotundan | K394 |
| PAİY 41/3 | 0,20 m'ye kadar motif çıkma, bahçedeki üstü açık teras, zemin kat giriş merdiveni ve 2,50 m'ye kadar giriş saçağı çıkma sayılmaz | Zemin kat mahalleri çıkma değildir; motif çıkma ve saçak çizimde mahal değildir (K392) |
| ADR 8032 | Kenarın istediği bahçe mesafesi (23/1; kenarın kendi değeri önce), yapı yaklaşma sınırı | Taşma kenarın istediği mesafeden ölçülür (K393) |

## §3. Karar

- **K392 — Çıkma:** zemin katından (binasının zemin katı, 8072) yüksek bir katta mahalin dış hattının taban alanının izi
  dışındaki parçası (1 cm²'den küçüğü değil). Mahal dış mekânsa (kullanımının kartı `outdoor`: balkon, teras) açık çıkma,
  değilse kapalı çıkma. Her parsel kenarına en yakın uzaklığı, kenarın bahçeye göre türü (ayrık nizamda bitişik kenar yan
  bahçe, 8142) ve istediği mesafe; altı, ortasının altındaki döşemenin taşıyıcısının alt yüzü eksi altındaki katmanlar
  (8040), ±0,00'dan; döşeme çizilmemişse yok.
- **K393 — 41/1:** taşma = kenarın istediği mesafe − uzaklık (yapı yaklaşma sınırından ölçülür; taban içeri çekilmişse
  çıkma o kadar uzun olabilir, a-3). Ön bahçede taşma en çok 1,50 m; yan, arka ve kamusal alan bahçesinde (park, meydan
  yol cephesi değildir: sıkı taraf) taşma en çok 1,50 m ve parsel sınırına en az 3,00 m; bitişik kenarda açık çıkma komşu
  sınırına en az 2,00 m. Aşan çıkma kaldı (`paiy_projection_over`, kenarları ve nedenleriyle).
- **K394 — 41/2:** çıkma altı zeminden (modelin ±0,00'ı; ön bahçesi 3 m ve daha az parselde yol kotu da ±0,00 sayılır)
  en az 2,40 m; alçak kaldı (`paiy_projection_low`). Altında döşeme yoksa katın kotu 2,40 m'den azsa kaldı (çıkma altı
  ondan da alçak), değilse eksik veri.

## §4. Uygulama

- `crates/native/domain/src/projections.rs` (yeni).
- `crates/shared/extensions/src/audit_facts/building.rs` (`ProjectionFacts`, `ProjectionEdgeFacts`),
  `crates/native/processing/src/audit_facts/building.rs` (`projections`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/cikmalar.py` (yeni), `mimari.py`, `denetim.py`
  (`cikma_denetimi`), `manifest.json`, `data/surumler.json` (41/1, 41/2'nin 1.10.2017 hâli).

## §5. Sınamalar

- **Kural** (`test_cikmalar.py`; 41/1 ve 41/2'nin metninden elle): 12 bağımsız durum: ön bahçede 1,20 m taşan balkon
  geçer, 2,00 m kalır; yan sınıra 2,50 m kapalı çıkma kalır (3,00 m); 4,50 m'lik arka bahçede sınıra 3,00 m, 1,50 m
  taşan geçer; arka sınıra 1,00 m iki nedenle kalır; bitişik komşuya 1,50 m açık çıkma kalır, kapalı çıkma geçer;
  kamusal alan kenarı yan bahçe gibi kalır; altı 2,30 m kalır; döşemesiz eksik veri, katı 2,20 m'de kalır; çıkma yok
  kapsam dışı.
- **Masaüstü** (`projection_tests`, elle çözülen iki katlı konut): 1. katın 2 m derin balkonu ön sınıra 3,00 m, yapı
  yaklaşma sınırından 2,00 m taşıyor: 41/1 kaldı, 41/2 eksik veri (döşeme yok); balkon 1,20 m ve altında 15 cm döşeme:
  altı 2,85 m, ikisi geçti. Kural seti satırında kapsam dışı maddeler 37 (`regulation_dates_tests`).
- **Ortak süzgeçler:** çıkma, bitişik kenar, vaziyet planı, kural seti, iç yalıtım, emsal, bodrum listesi, zon, modül,
  şerit, geçmiş, katalog ve ayar sınamalarıyla masaüstü sınamaları yeşil (174); domain'in bütün sınamaları, olguların
  altı (örnek binanın vaziyet planı yok), uygulamanın `blocks_areas`, `arch_ifc`, `protocol`, `site` sınamaları ve iş
  çalıştırıcının ulusal kural sınamaları yeşil; SDK şeması yeniden üretildi.
- **Resimler:** `.run/gui-evidence/cikma-8152/`: 1. katın planı (parsel, yapı yaklaşma sınırı, balkon) ve Denetim'in
  41/1 bulgusu, açık ve koyu.

## §6. Kalanlar

- Çıkmanın vaziyet planında izdüşümüyle gösterilmesi (57/3).
- Ön bahçesi 3 m ve daha az parselde yol kotu (bugün ±0,00).
- PAİY 6 (8142 §6'nın öbür kalanı).
