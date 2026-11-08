# 2260 — Senaryo ölçülerinde erişimin çizimde gösterilmesi

Tarih: 2026-11-08. plan/map-cad.md MAP-10, üçüncü tur. Durum: taslak. Plan satırı: “Senaryoları … ölçülerle
karşılaştır; ölçünün yöntemi ve varsayımı görünür olsun.”

İlgili: [ADR 2040](2040-impact-impervious-and-access.md), geçirimsizlik ve erişim. Bu ADR onun “Sonra”sındaki “zarf ve
harita gösterimi” maddesinin harita kısmını erişim ölçüsü için kapatır.

## Dayanak

- **ArcGIS Urban, özel göstergeler**
  ([Custom indicators](https://doc.esri.com/en/arcgis-urban/latest/help/help-custom-indicators.html), 8 Kasım 2026'da
  okundu):
  - gösterge, şehrin plan ve projelerini çözümlemek için kullanılan bir veri öğesidir;
  - yan panelden seçilen gösterge görünümde açılır. Onunla hangi katmanların görüneceği gösterge ayarlarında seçilir.

  Ölçünün sayısının yanında çizimdeki karşılığı da gösterilir.
- **KentOS bugün** (ADR 2040):
  - erişim yalnız iki sayıydı: erişen alan ve payı;
  - hangi konut alanının parka erişmediği çizimden okunamıyordu;
  - komutun cevabı alanları adlandırmıyordu. Python ve MCP de bunu bilemiyordu.

## Karar

1. **Cevapta `reach`.** `access` sorulduğunda `cad.scenarios.impact` her iki taraf için alanları kalıcı kimlikleriyle,
   çizimin sırasıyla adlandırır:
   - `baseReached` / `alternativeReached`: erişen `from` alanları;
   - `baseUnreached` / `alternativeUnreached`: erişmeyen `from` alanları;
   - `baseTargets` / `alternativeTargets`: `to` alanları.

   Kural ADR 2040'ınkidir: ağırlık merkezinden en yakın hedef alana kesin uzaklık. `access` sorulmazsa `reach` yoktur.
   Ek bir alandır: komut sürümü değişmez, erişimsiz durumların cevabı aynı kalır.
2. **Erişimi çizimde göster** (Senaryo ölçüleri penceresi, Raporu kaydet'in yanında; yalnız erişim sorulduysa
   açıktır):
   - pencere kapanır ve görünüm senaryonun bu alanlarına getirilir (her yandan onda bir pay);
   - senaryonun tarafı çizimde kalır:
     - erişen alanlar başarı renginde (yeşil) dolgulu;
     - erişmeyenler tehlike renginde (kırmızı) dolgulu;
     - hedef alanlar vurgu renginde kesikli çerçeveli.
   - Mevcut durum gösterilmez: senaryo onun yerine geçer ve iki taraf aynı yerdedir.
   - İşaretler aynı girdiyle, çizim değiştikçe yeniden hesaplanır: hedefe yaklaştırılan alan hemen yeşile döner.
     Senaryo kaldırılırsa işaret kalmaz.
   - **İşareti kaldır** (`data.unmark`) ya da başka bir çizim işaretleri kaldırır. Çizime bir şey yazılmaz.
   - Durum satırı sayıları ve renkleri söyler.
3. **Kapsam işareti:** ADR 1440'taki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/scenarios_impact_cases.py`, KentOS kodu yok. Erişimli dört durumun cevabı
  `reach`'i elle taşır: dikdörtgenlerin ortası ve kutuya uzaklık. Öbür 26 durumun cevabı değişmedi. Rust ve Python
  geçer.
- **MCP** (`crates/native/mcp/tests/scenario_reach.rs`): durumun çizimi `.kcad` olarak açılır. `reach` durumunkiyle
  aynıdır; erişim sorulmayınca yoktur.
- **Masaüstü** (`impact_window_tests.rs`, `the_access_stays_on_the_drawing_until_unmarked`):
  - Erişimi çizimde göster pencereyi kapatır;
  - erişen, erişmeyen ve hedef alanların sayıları durumunkidir;
  - hedefler silinince işaretler yeniden hesaplanır, hiçbir alan erişmez;
  - İşareti kaldır işaretleri siler.

  Çapraz masaüstü denetimleri 138 geçer. Resimler `.run/gui-evidence/senaryo-erisim-2260/`.

## Sonra

- Mevcut durumun erişimini de göstermek ya da iki tarafı yan yana; erişmeyen alanın en yakın hedefe uzaklığını yazmak.
- Ağ üzerinden yaya erişimi (yol ağıyla): ağ oluşturucusu işlem katmanındadır, uygulama katmanından çağrılamaz.
- Arazi eğimi (yüzeyle), damganın çizimde saklanması, zarf (ADR 2040 Sonra).
