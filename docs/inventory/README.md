# Web özellik envanteri

TODOS.md `BASE-04`: web uygulamasının komutları, araçları, ayarları, dosya alanları, işlem araçları ve ekranları makinece okunabilir tek bir listede. Masaüstü kabuğu (`UI-11`), komut sözleşmesi (`CMD-01`) ve AI kapsam manifesti (`AI-01`) bu listeden başlar.

| Dosya | İçerik |
|---|---|
| [web.json](web.json) | Tam envanter; **üretilir**, elle düzenlenmez |
| [web.md](web.md) | Özet: sayılar, kısmi ve bekleyen öğeler, arayüzde yeri görünmeyen komutlar; **üretilir** |
| [annotations.json](annotations.json) | Elle yazılan notlar, durum düzeltmeleri ve kabul senaryoları |

```bash
pnpm inventory          # uygulamayı başsız Chrome'da açar, web.json ve web.md'yi yazar
pnpm inventory:check    # hiçbir şey yazmaz; dosyalar güncel değilse düşer ve değişen öğeleri sayar
```

Bir komut, araç, işlem aracı, ayar, depo, pencere ya da `.kcad` alanı eklenince, silinince ya da değişince `pnpm inventory` çalıştırılır. Fark okunur ve değişiklikle aynı commit'e girer. Betik Vite ve başsız Chrome açar; ağır bir iştir, başka ağır işle aynı anda çalıştırılmaz.

## Nereden gelir

| Bölüm | Kaynak |
|---|---|
| `commands` | Çalışan uygulamanın komut kaydı (`CommandRegistry.all()`) |
| `tools` | Araç kataloğu (`tools/catalog.ts`, `ToolManager.list()`) |
| `processing`, `models` | İşlem kaydı ve model kitaplığı. Taze tarayıcı profilinde yalnız hazır modeller bulunur |
| `workspaces` | `app/workspaces.ts` |
| `settings` | Kullanıcı tercihleri (`kentos.prefs.v1`), yerleşim (`kentos.ui.v1`), proje ayarları (`PROJECT_SETTINGS_DEFAULTS`, `.kcad`'e yazılır) ve oturum yardımcıları (kalıcı değil). `default` taze profilin değeridir |
| `storage` | Kaynak taraması: `persistedSignals('…')` localStorage anahtarları; `indexedDB.open` yanındaki `const DB`/`STORE` |
| `fileFields` | `.kcad` v1: `DocumentSnapshotV1`'den erişilen bütün sözleşmeler. Rust'tan üretilen TS tiplerinden okunur, tanımlandıkları Rust dosyasıyla birlikte |
| `screens` | Kaynak taraması (`src/ui`): `export function open…` pencereleri, `Component`/`Panel`'den türeyen paneller |

Komut kayıtlarının yanında menü ve şerit yerleri de hesaplanır: menü yolu, şerit sekmesi ve paneli, hızlı erişim, araç kutusu. Kısayollar tuş eşleminden gelir. `hiddenIn`, komutu hangi hazır çalışma modunun gizlediğini söyler.

## Alanlar

- **`status`:** `implemented` | `partial` | `pending`.
  - Komutta `pending(...)` ile, araçta `ready: false` ile, çalışma modunda “Yakında” ile gelen durum `pending`'dir.
  - Öbür öğeler `implemented` başlar.
  - `partial` yalnız `annotations.json`'dan gelir ve nedenini `note` alanında taşır.
- **`platforms`:** `{ web, desktop }`.
  - `web` `status`'la aynıdır.
  - `desktop` masaüstü uygulaması yokken `none`'dır. Bir özellik masaüstünde anlamsızsa `n/a` yazılır.
- **`tests`:** kimliğin geçtiği test dosyaları ve e2e betikleri.
  - Komutta kimlik tırnak içinde aranır. Araçta `tool.<id>` ya da `activate('<id>')`, işlem araçlarında kimlik ya da komut kimliği aranır.
  - Kimliğin geçmesi davranışın sınandığını göstermez. Boş liste de sınanmadığı anlamına gelmez: test komutu başka bir yoldan çalıştırıyor olabilir.
- **`uiSources`** (komutlar): kimliğin tırnak içinde geçtiği `src/ui` dosyaları. Panel düğmeleri, durum çubuğu, uygulama menüsü ve pencereler böyle bulunur.
- **`note`, `acceptance`:** `annotations.json`'dan. `acceptance`, TODOS.md'deki kabul izidir. Otomatik testin yerine geçmez.

## Notlar

- `annotations.json`'da anahtar `bölüm:kimlik` biçimindedir (`command:file.save`, `tool:polygon`, `screen:<yol>#<ad>`, `setting:user.snapAperture`).
- Envanterde bulunmayan bir öğeye yazılmış not betiği durdurur. Notlar böylece fark edilmeden eskimez.
- Yalnız kodda, testte ya da bir ADR'de doğrulanmış bilgi yazılır.

## Sınırlar

- Tarama kalıpları kodun bugünkü alışkanlıklarıdır. Başka biçimde yazılmış bir pencere ya da depo kaçırılır, yanlış okunmaz.
- Envanter web uygulamasınındır. Sunucu uçları (`apps/api`) ve Rust hesap çağrıları ayrı envanterdir. Masaüstü, Python ve AI sütunları kendi fazlarında eklenir (`AI-01`).
