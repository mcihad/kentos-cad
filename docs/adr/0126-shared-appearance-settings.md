# ADR 0126: Görünüş ayarları iki platformda aynı anahtarlarla

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0023 (tipli ayarlar), ADR 0051 (masaüstünün Görünüm sekmesi); DESIGN.md §3.5, §4; docs/inventory/parity-audit.md A1, A6; TODOS.md UX-13.
- **Sahibin kararları (28 Eylül):** ortak anahtarlarda iki platformun seçeneklerinin birleşimi ("hiçbir platform seçenek kaybetmez"); varsayılan arayüz yazı tipi iki platformda Plus Jakarta Sans.

## Bağlam

Web ile masaüstünün görünüş tercihleri farklı anahtarlardaydı. Ayar dosyası (`kentos.settings`) iki platformda okunuyor ama görünüşü taşımıyordu (A1):

| | Web | Masaüstü |
|---|---|---|
| Tema | yerleşimde (`kentos.ui.v1`): koyu, açık | `appearance.theme`: koyu grafit, açık pafta, gece, yüksek karşıtlık |
| Vurgu | `appearance.accent`: lacivert, amber, petrol yeşili, bordo | `appearance.accentColor`: sekiz Türkçe ad ya da `#RRGGBB` |
| Yazı tipi | `appearance.uiFont` (Plus Jakarta Sans) | `appearance.typeface` (IBM Plex Sans), aynı yedi aile |
| Eş aralıklı | yok (IBM Plex Mono) | `appearance.monoTypeface` |
| Yazı boyu | `appearance.uiScale`: beş adım | `appearance.textSize`: 11–18 px (A6) |

## Karar

### Ortak anahtarlar

Hepsi iki platformun (`hosts: web, desktop`), kullanıcı tercihidir:

- **`appearance.theme`:** `dark` (Koyu grafit, varsayılan), `light` (Açık pafta), `night` (Gece), `highContrast` (Yüksek karşıtlık).
- **`appearance.accent`** (sürüm 2): hazır renklerden biri ya da `#rrggbb`. Hazır renkler renk çemberinin sırasıyla:
  - lacivert (`navy`, varsayılan);
  - mavi (`blue`), petrol yeşili (`teal`), yeşil (`green`), amber (`amber`), turuncu (`orange`), bordo (`bordeaux`), pembe (`pink`), mor (`violet`), gri (`gray`).

  Web'in dört rengi web'in tonlarıyla, masaüstünün öbürleri kendi tonlarıyla gelir. Neredeyse aynı iki çift birleşti: masaüstünün turkuazı petrol yeşili, kehribarı amber oldu.
- **`appearance.uiFont`:** yedi aile. Varsayılanı iki platformda Plus Jakarta Sans'tır (DESIGN.md).
- **`appearance.monoFont`:** IBM Plex Mono (varsayılan), JetBrains Mono.
- **`appearance.textSize`** (sürüm 2): 11–18 piksel, varsayılanı 13.
  - Web'in beş adımı hazır değer olarak kalır: Küçük 12, Standart 13, Büyük 14, Çok büyük 15, En büyük 16.
  - Web'in bütün ölçüleri `--ui-scale = textSize / 13` ile büyür.

Yalnız bir platformun kalanlar: `appearance.drawingBackground` (masaüstü), `appearance.shell` (web'in klasik arayüzü; [ADR 0155](0155-web-ribbon-only.md)'te emekliye ayrıldı).

### Renk kuralı ortak doğrulamada

`SettingDescriptor`'a `color` eklendi: bir metin ayarı, seçenekleri yanında `#rrggbb` de alabilir. Değer küçük harfe iner (`color_of`, TS `colorOf`).

Doğrulama sırası değişmedi. Seçeneklerden biri olmayan değer ancak renkse alınır, değilse `not_allowed`'dır. Böylece `purple` gibi bir değer iki platformda da reddedilir.

### Eski anahtarlar okunurken yenisine çevrilir

Her ayar belgesi okunurken, saklanan da içe aktarılan da, eski anahtarlar bugünkülerine çevrilir (`RENAMED`, `renamed_setting`, TS'te `RENAMED`, `renamedSetting`):

| Eski | Bugünkü | Değer |
|---|---|---|
| `appearance.accentColor` | `appearance.accent` | mavi → blue, turkuaz → teal, yesil → green, kehribar → amber, turuncu → orange, pembe → pink, mor → violet, gri → gray; `#RRGGBB` küçük harfe |
| `appearance.typeface` | `appearance.uiFont` | aynı |
| `appearance.monoTypeface` | `appearance.monoFont` | aynı |
| `appearance.uiScale` | `appearance.textSize` | small 12, standard 13, large 14, xlarge 15, xxlarge 16 |

- Belgede bugünkü anahtar da varsa o geçerlidir, eskisi atılır.
- Karşılığı olmayan eski değer atılır ve eski adıyla söylenir (`not_allowed`, `wrong_type`).
- Çevrilen değer bugünkü anahtarın kuralıyla denetlenir: ör. bilinmeyen bir yazı tipi `appearance.uiFont` adıyla `not_allowed`'dır.

Aynı kural web'in eski `kentos.prefs.v1` göçünde de uygulanır (`legacy.ts`).

### Web

- **Tema** artık ayardır.
  - İlk açılışta ayar saklanmamışsa ve yerleşimde (`kentos.ui.v1`) açık tema varsa bir kez ayara alınır.
  - Yerleşimin `theme` alanı yalnız bu göç için okunur. `fixtures/shell/v1/layout.json` değişmedi.
  - Temayı ve yazı boyunu izleyenler ayarın sinyalini izler (`ctx.prefs.theme`, `ctx.prefs.textSize`).
  - Ayar penceresi, bir komut, içe aktarılan dosya ya da sıfırlama aynı yoldan uygulanır (`createApp`).
- Ayar dosyası bölümü artık görünüşün de taşındığını söyler.
- **Bu dilimde web'in yeni değerleri çizmesi yoktur.** Gece ve yüksek karşıtlık şimdilik koyu tema gibi görünür. Masaüstünün altı vurgusu ve özel renk şimdilik lacivert görünür. Yazı boyu penceresi beş adımı gösterir. Sonraki dilim bunları web'e getirir: temaların jetonları, vurguların hesaplanan jetonları, eş aralıklı yazı seçimi, tema kartları, piksel adımlayıcısı.

### Masaüstü

- Ortak anahtarları okur ve yazar.
- KentOS UI'nin `Accent`'i on hazır renk, İngilizce ayar adları ve eski Türkçe adları da okuyan `parse` ile.
- `Family` ve `Typography::DEFAULT` artık Plus Jakarta Sans'tır. Hiç seçmemiş kullanıcının masaüstündeki yazısı değişir (sahibin kararı).
- Ayar penceresi özel rengi ortak adına çevirerek saklar.
- Web'in ayar dosyasını alırken artık yalnız klasik arayüz ve çizim motoru gibi gerçekten web'e özgü değerleri adlarıyla söyler.
- **Sonraki dilim:** Uygulama ayarları → Görünüm'e web'in tema kartları, vurgu örnekleri, yazı tipi örnekleri ve boyut adımları (28 Eylül'de geldi, ADR 0128).

## Sonuçlar

- Ayar dosyası görünüşü iki platform arasında taşır. İki platform aynı belgeyi aynı kurallarla okur.
- Eski dosyalar kaybolmaz: iki platformun şimdiye dek yazdığı her görünüş değeri bugünkü anahtara çevrilir.
- **Değişen varsayılanlar:**
  - masaüstünde yazı tipi IBM Plex Sans'tan Plus Jakarta Sans'a, vurgu maviden laciverte geçti (DESIGN.md);
  - web'de beş adımın ölçekleri piksele yuvarlandı. Ör. En büyük 1,25 yerine 16/13 ≈ 1,23'tür.
- `fixtures/settings/v1/cases.json`'a eklenen durumlar:
  - değerler: vurgunun hazır adı ve özel rengi, küçük harfe inmesi, reddedilenler; piksel boyu ve aralığı;
  - dosyalar: eski anahtarların çevrilmesi, bugünkü anahtarın önceliği, karşılığı olmayan eski değerin söylenmesi.

## Doğrulama

- `cargo test -p kentos-contracts`: ortak durumlar, şemanın tamlığı (metin yalnız renk yanında seçenek taşır), üretilen `settingsSchema.json` ve `SettingDescriptor.ts`.
- `pnpm -C apps/web exec vitest run src/core/settings src/app/settings`: TS ikizi aynı durumları geçer. `kentos.prefs.v1` göçü eski beş adımı piksele çevirir.
- `cargo test -p kentos-ui -p kentos-ui-showcase -p kentos-desktop`:
  - on vurgunun adları ve her temada okunurluğu;
  - masaüstünün görünüş seçimleri ortak anahtarlara yazılır;
  - eski bir ayar dosyası görünüşünü korur;
  - web'in ayar dosyası görünüşüyle alınır.
