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
| `settings` | Tercihler: tipli ayarlar ([ADR 0023](../adr/0023-typed-settings.md); `localStorage kentos.settings.v1`, kapsamı `user` ya da `device`, `default`'u şemadan). Yerleşim (`kentos.ui.v1`), proje ayarları (`PROJECT_SETTINGS_DEFAULTS`, `.kcad`'e yazılır) ve oturum yardımcıları (kalıcı değil). `setting`, tipli ayarın anahtarıdır. Yerleşimde ve oturumda `default` taze profilin değeridir |
| `storage` | Kaynak taraması: `persistedSignals('…')` localStorage anahtarları; yerleşimin `LAYOUT_KEY`'i (`app/layoutPlan.ts`); tipli ayarların `SETTINGS_STORAGE`, `SETTINGS_BACKUP` ve eski `LEGACY_PREFS` sabitleri; `indexedDB.open` yanındaki `const DB`/`STORE`; sessionStorage kullanan modülün `export const …_KEY` sabitleri (sekmeye özgü depo) |
| `fileFields` | `.kcad`: `DocumentSnapshotV1`'den (okunan v1 JSON) ve `DocumentSnapshotV2`'den (yazılan binary v2, [docs/specs/kcad-v2.md](../specs/kcad-v2.md)) erişilen bütün sözleşmeler. Rust'tan üretilen TS tiplerinden okunur, tanımlandıkları Rust dosyasıyla birlikte |
| `screens` | Kaynak taraması (`src/ui`): `export function open…` pencereleri, `Component`/`Panel`'den türeyen paneller |
| `layout` | Menü çubuğu, şerit (`ribbon`: türü sorulmamış projenin, CBS'ninki; `ribbonByMode`: her proje türünün, ADR 0165) ve hızlı erişim, web'in sırasıyla; masaüstü menüsünü ve şeridini bundan kurar ([ADR 0017](../adr/0017-desktop-shell.md)) |
| `icons` | İkon seti (`ui/icons.ts`, `ICONS`): ad → 20×20'lik çizgi ikonun SVG metni (tutamaçlar içinde). Masaüstü komutların ikonlarını bundan, web'in çizdiği gibi çizer ([ADR 0054](../adr/0054-desktop-draws-the-web-icons.md)) |

Komut kayıtlarının yanında menü ve şerit yerleri de hesaplanır: menü yolu, her proje türünün şeridindeki yeri (“Tür › Sekme › Panel”; panelin ▾ listesi de), hızlı erişim. Kısayollar tuş eşleminden gelir. `hiddenIn`, komutu hangi hazır proje türünün şeridinde olmadığını söyler.

## Alanlar

- **`status`:** `implemented` | `partial` | `pending`.
  - Komutta `pending(...)` ile, araçta `ready: false` ile, proje türünde “Yakında” ile gelen durum `pending`'dir.
  - Öbür öğeler `implemented` başlar.
  - `partial` yalnız `annotations.json`'dan gelir ve nedenini `note` alanında taşır.
- **`platforms`:** `{ web, desktop }`.
  - `web` `status`'la aynıdır.
  - `desktop`: `implemented` | `partial` | `pending` | `none` | `n/a`; aşağıdaki kaynaklardan, her biri öncekinin üstüne, bu sırayla gelir. Hiçbiri bir şey demezse `none`'dır.
    1. `apps/desktop/equivalents.json`'ın `sections`'ı: bütün bir bölüm için tek söz (ör. `.kcad` alanlarının hepsi, ortak kodek).
    2. `apps/desktop/ported.json`: masaüstü kabuğunun çalıştırdığı komutlar (masaüstü testi onu `catalog::PORTED` ile eşit tutar) `implemented`'dır; onlarla araçlar (`tool.<id>`), işlem araçları ve modeller (`processing.run.<id>`, `processing.model.<id>`) ve proje türleri (`workspace.<id>`).
    3. Ayarlarda tipli ayarın `hosts`'unda masaüstü varsa `implemented` (`settingsSchema.json`).
    4. `apps/desktop/equivalents.json`'ın öğe öğe dedikleri; öğeye `desktopWhere` (masaüstündeki yeri) ve `desktopNote` (neden; `n/a`'da zorunlu) da gelir.
    5. `annotations.json`'daki `desktop`.
- **`tests`:** kimliğin geçtiği test dosyaları, e2e betikleri ve etkileşim izleri (`fixtures/interaction`, [ADR 0018](../adr/0018-tool-session-and-input.md)). Yalnız resim çeken betikler (`…shots.mjs`) sayılmaz.
  - Komutta kimlik tırnak içinde aranır. Araçta `tool.<id>` ya da `activate('<id>')`, işlem araçlarında kimlik ya da komut kimliği aranır.
  - Kimliğin geçmesi davranışın sınandığını göstermez. Boş liste de sınanmadığı anlamına gelmez: test komutu başka bir yoldan çalıştırıyor olabilir.
- **`uiSources`** (komutlar): kimliğin tırnak içinde geçtiği `src/ui` dosyaları. Panel düğmeleri, durum çubuğu, uygulama menüsü ve pencereler böyle bulunur.
- **`shortcutsInInput`** (komutlar): `shortcuts`'tan, bir metin alanı klavyedeyken de çalışanlar (tuş eşleminde `allowInInput`, `app/keybindings.ts`: Ctrl+S, F tuşları …). Yalnız böyle bir akoru olan komutlarda bulunur. Masaüstü metin alanlarında hangi akorun geçeceğini buradan okur (`apps/desktop/src/catalog.rs`); elle kopyalanmış liste yoktur.
- **`productCommand`** (araçlar): aracın onayında çalışan ürün komutu, katalogdaki kimliğiyle (ADR 0013, 0022). Araç kataloğundan (`tools/catalog.ts`) gelir; yalnız bir ürün komutundan yazan araçlarda bulunur. Arayüz komutu ile ürün komutu arasındaki eşleme budur (`AI-01`).
- **`note`, `acceptance`:** `annotations.json`'dan. `acceptance`, TODOS.md'deki kabul izidir. Otomatik testin yerine geçmez.

## Masaüstünün tablosu (`apps/desktop/equivalents.json`)

Masaüstünün elle tuttuğu dosyadır (`kentos.desktop-equivalents`, sürüm 1); `ported.json`'ın yanında durur. Yoksa boş sayılır.

```json
{
  "format": "kentos.desktop-equivalents",
  "version": 1,
  "commands": { "view.renderer.webgl2": { "desktop": "n/a", "reason": "Masaüstü yalnız wgpu ile çizer (ADR 0019)." } },
  "screens": { "apps/web/src/ui/processing/ToolDialog.ts#openToolDialog": { "desktop": "implemented", "where": "apps/desktop/src/processing/window.rs" } },
  "storage": { "kentos.processing.v1": { "desktop": "implemented", "where": "$XDG_STATE_HOME/kentos-cad/islemler.json" } },
  "sections": { "fileFields": { "desktop": "implemented", "reason": "Masaüstü KCAD v2'yi ortak kodekle okur ve yazar." } }
}
```

- Anahtarlar envanterin bölümleri (`commands`, `tools`, `processing`, `models`, `workspaces`, `settings`, `storage`, `fileFields`, `screens`) ve `sections`'dır; öğe anahtarları `web.json`'daki `id`'lerdir.
- Bir öğe: `desktop` (yukarıdaki sözler), `where` (masaüstündeki yeri: modül, dosya, ayar anahtarı), `reason` (Türkçe neden; `n/a`'da zorunlu, `partial`'da beklenir). Başka alan yazılmaz.
- Envanterde olmayan bir bölüm ya da kimlik, bilinmeyen bir söz ya da alan, nedeni yazılmamış `n/a` betiği durdurur: tablo fark edilmeden eskimez.
- `summary.desktop` her bölümde masaüstünde olan, kısmi, olmayan, bekleyen ve anlamsız öğeleri sayar; `summary.desktopSections` tablonun bütün bölümler için dediğidir. `web.md` bunları bölüm bölüm yazar ve web'de olup masaüstünde olmayanları sıralar.
- `pending` (Bekliyor) web'de de yapılmamış öğedir: masaüstü onu web'in notuyla soluk gösterir. `web.md` onu olmayanlardan ayrı sayar ve listede “(iki platformda da bekliyor)” diye yazar.

## Notlar

- `annotations.json`'da anahtar `bölüm:kimlik` biçimindedir (`command:file.save`, `tool:polygon`, `screen:<yol>#<ad>`, `setting:user.snapAperture`).
- Envanterde bulunmayan bir öğeye yazılmış not betiği durdurur. Notlar böylece fark edilmeden eskimez.
- Yalnız kodda, testte ya da bir ADR'de doğrulanmış bilgi yazılır.

## Sınırlar

- Tarama kalıpları kodun bugünkü alışkanlıklarıdır. Başka biçimde yazılmış bir pencere ya da depo kaçırılır, yanlış okunmaz.
- Envanter web uygulamasınındır. Sunucu uçları (`apps/api`) ve Rust hesap çağrıları ayrı envanterdir. Python ve AI sütunları kendi fazlarında eklenir (`AI-01`).
