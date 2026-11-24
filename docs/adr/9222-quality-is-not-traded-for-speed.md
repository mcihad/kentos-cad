# ADR 9222: Kalite hıza feda edilmez: uzak düzeyler ve hareketteki tek örnek kalktı

- **Durum:** kabul edildi (2026-11-24). [ADR 9207](9207-far-levels-of-strokes-and-fills.md)'nin §1–§8'ini geri alır. Plan:
  [plan/render.md](../../plan/render.md) RM-10.
- **İstek:** sahibin sözleri (24 Kasım 2026):
  - “Performansı arttırırken kaliteden ödün veriyorsun.”
  - “Bu yazılımlar kaliteden ödün verilmemesi gereken yazılımlar.”
  - İki ödün sorulunca seçimi: “İkisini de kaldır”.
- **Sözleşmeler:** `graphics.msaa` ayarının açıklamasından hareket cümlesi çıktı. Ayarın türü ve değerleri aynı.
  `settingsSchema.json`'u sözleşmenin kendi sınaması yeniden yazdı (`KENTOS_WRITE_SETTINGS=1`).

## §1 Bulgu

ADR 9207 hız için iki ödün verdi:

- **Uzak düzeyler (§1–§7):** model ekranında 16, 64 ve 256 m/piksel'in altında düz çizgiler ve dolgular, düzeyin yarım
  pikseli olan bir ızgaraya oturtuluyor ve önce sadeleştiriliyordu. Kural “en çok yarım piksel, hiçbir nesne pikselinden
  çıkmaz” idi.
- **Hareketteki tek örnek (§8):** görünüm kayarken ve yakınlaşırken kare, ayardan bağımsız olarak tek örnekle
  çiziliyordu.

**Gerçek pencerede ölçüm** (Xvfb, sahibin ayarları, Sivas imar planı NCZ'sinin açılış görünümü, ~32 m/piksel):

- Düzeyli ve düzeysiz resimler 1 128 × 660 alanın **86 002 pikselinde** ayrılıyor; **6 940'ı** çeyrekten fazla.
- Düzeyli resimde parsel çizgileri ızgara hücrelerine kayıp kalınlaşıyor, kümeleniyor; yollar ve pafta çizgileri
  oynuyor (`.run/gui-evidence/render/kalite/once-sonra-40-uzak-yakin.png`).
- Dört çentik uzakta 7 449 piksel ayrı, 650'si çeyrekten fazla.
- ADR 9207'nin ölçtüğü %0,09–0,5'ten çok fazla. O ölçüm 20'lik farkla ve başka ayarlarla yapılmıştı.

**Sahibin ayarı:** cihazında `graphics.msaa` 1 (Kapalı). Hareketteki tek örneğin onun ekranında görünür bir payı yoktu;
4× seçen bir kullanıcının ekranında vardı.

## §2 Karar

- **Uzak düzey kalktı.** Model ekranı her uzaklıkta her kenarı ve dolgu üçgenini, kâğıt ve PDF gibi çizer.
  `kentos_native_style::far_levels` silindi; `batches::decode` artık düzey eklemiyor. Tanıdaki adları (`uzak`,
  `eğri+uzak`) ve `KENTOS_DIAG_LEVEL=print` de kalktı.
- **Hareketteki tek örnek kalktı.** Çizim alanı her karede ayarın örnek sayısıyla çizer (`viewport::drawn_with`
  silindi). Çizicinin örnek sayısı değişince öbür hedefleri yedekte tutması (`ViewState::spare`) yerinde: ayar
  değişince işe yarıyor.
- **Hız artık kaliteyi bozmayan yollarla aranır:**
  - döşemeler ve görünüm ayıklaması (RM-08);
  - ayrı çizim iş parçacığı (RM-07);
  - örnekli çizim (RM-11).
  
  Bunlar görünen resmi değiştirmez; her önbellek ve düzey, baştan çizilenin aynı resmini verir.

### §2.1 Kalanlar ve bedeli

- **Daha eski düzeyler.** Eğri düzeyleri (ADR 1063), uzun çizgi düzeyleri (1064), küçük blok vekili (1065) ve okunamaz
  yazı bandı (0786) bu kararın dışında kaldı. Her biri baştan çizimle karşılaştırılıp ölçülecek; görünür fark veren
  kalkar (plan RM-10).
- **Bedel ölçülmedi, tahmin:** ADR 9207'nin release ölçümlerine göre (UHD 770) NCZ'nin uzak görünüm karesi GPU'da
  ~47–55 ms'den, düzeylerden önceki ~87–105 ms'ye döner. Sahibin msaa=1 ayarıyla §8'in payı yok.

## §3 Kanıt

**Sınamalar** (yeşil olanlar):

| Paket | Sonuç |
|---|---|
| `kentos-native-style` (lib) | 52 (uzak düzeyin 5 sınaması modülle birlikte gitti) |
| `kentos-render-wgpu` | bütün hedefler; GPU sınamaları gerçek aygıtta: 32 + 8 + 10 + 16 + 8 + 5 + 7 |
| `kentos-contracts` `settings` | 13 |
| `kentos-desktop` | görünüm, stilli sahne, kararlılık ve bölünmüş görünüm sınamalarından 57 |

**Kırmızı (bu işle ilgisiz):** `kentos-native-style`'ın `bound_legend` sınaması. Main'deki a489301e7'nin (CBS projesinde
bağlı katman kuralı) getirdiği kırmızı, öncesinde de vardı.

**Gerçek pencere:** `.run/gui-evidence/render/kalite/`. Önce: main'in ucu, uzak düzeyli. Sonra: bu karar.

- `once-sonra-40-uzak.png`, `once-sonra-40-uzak-yakin.png`;
- `fark-40-uzak.png`, `fark-41-daha-uzak.png`.

## Uygulama

- `crates/native/style/src/far_levels.rs`: silindi.
- `crates/native/style/src/lib.rs`: modül kaydı silindi.
- `crates/native/style/src/batches.rs`: `decode` düzey eklemiyor; `Lod`'un açıklaması güncellendi.
- `crates/render/wgpu/src/styled/gpu.rs`: tanıdaki uzak düzey adları ve `print_level` silindi; düz dolgunun düzey yolu
  genel kaldı.
- `crates/render/wgpu/src/renderer.rs`: `spare`'in açıklaması.
- `crates/render/wgpu/tests/gpu.rs`: örnek sayısı değişimi sınamasının açıklaması.
- `apps/desktop/src/shell/viewport/mod.rs` (`drawn_with` silindi) ve `tests.rs` (sınaması silindi).
- `crates/shared/contracts/src/core/settings/schema.rs`, `apps/web/src/contracts/generated/settingsSchema.json`: ayarın
  açıklaması.
