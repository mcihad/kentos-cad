# ADR 9402: Web uygulaması ve WASM kaldırıldı; TypeScript yalnız Bulut için

- **Durum:** kabul edildi (2026-11-28). [plan/bakim.md](../../plan/bakim.md) Aşama 8, BK-80…BK-88.
- **Kural:** CLAUDE.md “Çalışırken” (masaüstü ve bulut), §3 (portalın teknik kısıtları), §14 (tek hesaplama kaynağı),
  §23 (sayısal doğruluk).
- **İlgili:** [ADR 0228](0228-web-pause-desktop-and-cloud-first.md) (web'in durdurulması), [ADR 0008](0008-shared-core-boundary.md)
  (ortak çekirdek ve WASM), [ADR 9000](9000-cloud-portal.md) (bulut portalı), [ADR 9400](9400-maintenance-plan.md)
  (bakım planı).
- **Ürün:** değişmedi. **`.kcad`:** değişmedi. **Sözleşme:** komut kataloğunun dosyası taşındı; konak listesinden
  `web` çıkar (BK-86).

## Bağlam

Web uygulaması 24 Kasım 2026'da iptal edilip dondurulmuştu (ADR 0228, F3): kodu duruyor, komutları ve kuralları
çıkarılmıştı. Sahip 28 Kasım 2026'da kararı tamamladı:

> Web projesini ve wasm desteğini komple sistemden kaldırıyoruz, ihtiyacımız olursa ileride ekleyebiliriz. Sadece
> KentOS Cloud ile ilgili olanların TypeScript dosyaları üretilsin. JS Math yerine daha ciddi ve CAD için doğru sonuç
> veren bir yapıya geçiyoruz. Tüm dosyaları da sil. Dikkatli şekilde.

Donmuş kod yine de canlı işin içindeydi: üretilen dosyalar web'in klasörüne yazılıyor, portal ve masaüstü oradan
dosya alıyordu. Silmeden önce bağımlılıklar ölçüldü (`bakim` `b00e8afb6`).

## Ölçüm

| Konu | Ölçüm |
|---|---|
| `apps/web` | 5 535 izlenen dosya; 3 899'u üretilen sözleşme türü (`src/contracts/generated`) |
| `crates/wasm` | 9 bağlayıcı crate, 29 dosya |
| Portalın web'den aldığı | 110 tür (içe aldıklarıyla 342 dosya, 276 KB); `ui/dom.ts`, `core/signal.ts`, `core/disposable.ts`; `fonts.css`, `tokens.css`, `accents.css`; 13 yazı tipi ailesi (1,3 MB) |
| Masaüstünün web'den derlediği | yeni proje sihirbazının iki resmi (`include_str!`) |
| Web'in dosyasını okuyan sınama | `tokens.css` (2), `labelDefaults.ts` (1), `favicon.svg` (2), komut kataloğu (1) |
| Komut kataloğunu okuyan | SDK üreteci, CAD manifesti, CBS kapsam betiği, geçmiş çağrıları betiği, iki Python sınaması; yazan: komutların sınaması |
| Web'den okuyan üreteç | yazı tipleri (3 betik: web'in WOFF2 dosyaları), işlem araçları (web'in TypeScript tanımlarından iki alan) |
| Yalnız web'e yazan betik | `scripts/ui/arrow_icons.py`, `hatch_icons.py`; web'in düzeneğini kullanan `scripts/wgsl/browser-check.mjs`, `scripts/wasm/ensure.mjs` |
| `ts` özniteliği | üç crate'te 16 157 satır; 3 890 `ts(export)` |
| WASM'a özgü kod | ortak crate'lerde 13 `cfg(target_arch = "wasm32")` satırı (5 dosya); `wasm-bindgen`; iki derleme profili |
| Yorumlarda `apps/web/…` yolu | Rust 439 satır, Python ~230 satır |
| Katalogda konağı `web` olan komut | 24 (`web` ve `desktop`) |
| `jsmath` | 696 dosya; `js_max` 882, `js_hypot` 702, `js_min` 494, `js_round` 159 çağrı |

Öbür dalların hepsi `bakim`'de birleşmiş durumda (`origin/main`, `arayuz-pencereler`, `mimari-bim`, `render-motoru`):
silinen klasörlere yazan, birleşmemiş bir iş yok.

## Karar

1. **Kaldırılır:** `apps/web`, `crates/wasm`, `wasm-bindgen` bağımlılığı, `wasm` ve `wasm-dev` profilleri, ortak
   crate'lerdeki WASM dalları, yalnız web'e hizmet eden betikler.
2. **Taşınır, silinmez:**
   - Portalın aldıkları portala (`apps/portal/src/lib`, `styles`, `assets/fonts`).
   - Komut kataloğu `catalog/v1/commands.json`'a. İçeriği ve biçimi aynıdır.
   - Yeni proje sihirbazının iki resmi masaüstüne.
   - Sınamaların ve üreteçlerin web kaynağından okuduğu değerler dondurulmuş başvuru dosyalarına. Değerler web'in
     son hâlinden aynen alınır; yeniden yazılmaz.
3. **TypeScript yalnız Bulut için:** `ts-rs` çıktısı portalın klasörüne yazılır ve yalnız portalın kullandığı türler
   üretilir. Kullanılmayan `ts` öznitelikleri ayrı bir dilimde silinir.
4. **Kalır:** sunucunun tarayıcı istemcisi (`x-kentos-client: web`) ve ortak tanımdaki `Client::Web`. Tarayıcı
   istemcisi portaldır. Masaüstü kataloğunun tabanı olan `docs/inventory/web.json` BK-23'e (tek masaüstü kataloğu)
   kadar yerinde durur; `apps/web`'in parçası değildir.
5. **Geri getirme:** web uygulamasının ve WASM bağlayıcılarının son hâli `b00e8afb6` commit'indedir.
6. **Bekçi:** `apps/web` ve `crates/wasm` klasörleri geri gelemez (`make guard`). Eski bir daldan gelen birleştirme
   üretilen dosyaları eski yerine yazarsa bekçi kırmızı olur.
7. **Sayısal temel:** `jsmath` web'in TypeScript çekirdeğinin birebir taşınması için yazılmıştı; amacı tarayıcıyla
   aynı biti vermekti, en doğru sonucu değil. Yerine geçecek temel araştırmayla seçilir ve kendi ADR'sinde karara
   bağlanır (9403). Araştırmanın soruları:
   - Temel işlevler (`sin`, `cos`, `atan2`, `hypot`, `pow`…): doğru yuvarlanan bir kitaplık mı, platformun kendi
     kitaplığı mı, `libm` mi? Sonuç her platformda aynı olmalı (§23.4).
   - Yuvarlama: `js_round` yarımı +∞'a yuvarlar; iş kuralları hangi yönü ister, hangi sürümlü politika altında?
   - `min`, `max` ve NaN: JavaScript NaN'ı taşır; CAD çekirdeğinde NaN denetimli hata olmalı (§23.1).
   - Toplamlar ve alanlar: telafili toplama, çift-çift aritmetik, hatasız dönüşümler.
   - Kararlar: kesin yüklemler (bugün `orientation`), eşik çevresinde aralık aritmetiği (§23.3).
   - Geçiş: beklenen değerler bağımsız yüksek hassasiyetli başvurudan yeniden üretilir; `.kcad` ve komut
     sonuçlarında değişen son basamaklar ADR'de sayılır.

## Sıra

Önce web'den bir şey alan her yer kendi kopyasına ya da yeni yerine bağlanır; klasör en son silinir. Her adım bir
commit'tir ve sonunda derleme, ilgili sınamalar, portalın tür denetimi ve `make guard` koşar. Adımlar
[plan/bakim.md](../../plan/bakim.md) BK-80…BK-88'dedir.

## Modül ve kanal tablosu

| İş | Mimari | Harita | Yol | CBS | MCP | Python | Eklenti | Yapay zekâ ajanı |
|---|---|---|---|---|---|---|---|---|
| Web ve WASM'ın kaldırılması | değişmedi | değişmedi | değişmedi | değişmedi | değişmedi: aynı komutlar; kataloğun dosyası taşındı | değişmedi: SDK aynı katalogdan üretilir | değişmedi | değişmedi |

Hiçbir modül ve kanal bir özellik yitirmez ya da kazanmaz: silinen, donmuş bir istemcinin kodudur.
