# ADR 9325: Form tasarımının adımları görünür, eylemlerin adları ayrı, önizleme kaydı ve rolü söyler

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  6, P3). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 0401](0401-forms-core.md) (formlar), [ADR 0403](0403-form-designer-and-entry-extras.md) §3 (önizleme),
  [ADR 0411](0411-form-permissions-and-server-guard.md) (Veri girişi rolü), [ADR 9103](9103-edit-session.md) (düzenleme oturumu).

## Bağlam

- **Gözlem:** yeni formda “Öğe türü”, “Değer listesi”, “Kapsam” ve “Kuralları uygula” ile üç ayrı kaydetme yüzeyini ilk
  kullanımda birbirine bağlamak zordu:
  - tasarımcının Kaydet'i taslağı yazıp pencereyi kapatıyordu, Yayınla pencereyi açık tutuyordu;
  - Formla düzenle'nin Kaydet'i kaydı katmana yazıyordu;
  - Değişiklikleri kaydet veri katmanının düzenlemelerini kaynağa yazıyordu.
- **İncelemenin istediği:**
  - Akış görünsün: Alanları yerleştir → Örnek kayıtla dene → Taslağı kaydet → Yayınla.
  - Taslağı kaydet, Formu yayınla, Kaydı uygula ve Katman değişikliklerini kaynağa kaydet adları ayrılsın.
  - Önizleme bir test satırını yazmadan açsın, rol görünümlerini göstersin.

## Karar

### 1. Ayrı adlar

| Yer | Önce | Şimdi |
|---|---|---|
| Formlar (tasarımcı) | Kaydet | Taslağı kaydet |
| Formlar (tasarımcı) | Yayınla | Formu yayınla |
| Formla düzenle (kayıt) | Kaydet / Kaydet ve kapat / Kaydetmeden kapat / Kaydedilmedi. | Kaydı uygula / Uygula ve kapat / Uygulamadan kapat / Uygulanmadı. |
| Veri › düzenleme oturumu (`layer.editSave`) | Değişiklikleri kaydet | Katman değişikliklerini kaynağa kaydet (şeritte: Kaynağa kaydet) |

- Bu adları anan iletiler de değişir (“önce Taslağı kaydet’e basın”, “Kaydı uygula ile kayıtla birlikte yazılır”,
  “Kaynağa kaydet'e kadar katmanda bekler”). Etkileşim izi (`form-entry.json`) Kaydı uygula'ya basar.
- **Taslağı kaydet pencereyi açık tutar:** sonraki adım (Formu yayınla) aynı penceredir. Pencere Vazgeç ya da kapatma
  düğmesiyle kapanır; kaydedilmiş taslakta Vazgeç bir şey kaybettirmez.

### 2. Adımlar

- Tasarımcının başında tek satır dört adımı söyler; yapılanın yanında onay işareti, sıradaki adım koyu yazılır:
  1. **Alanları yerleştir:** formda alan öğesi var.
  2. **Örnek kayıtla dene:** önizleme bu pencerede açıldı.
  3. **Taslağı kaydet:** kaydedilmemiş değişiklik yok.
  4. **Formu yayınla:** son yayın kaydedilmiş taslakla aynı.

### 3. Önizleme

- **Başta kayıt söylenir:**
  - “Boş kayıt. Haritada bu katmandan bir nesne seçerseniz önizleme onun değerleriyle açılır.”
  - Ya da “Örnek kayıt: haritada seçili nesne, değerleriyle.”
  - Her ikisinde: “Önizlemede yazılanlar kayda geçmez.”
- **Rol:** Düzenleyen | Veri girişi | Okuyan.
  - **Okuyan:** önizleme salt okunurdur; alanlar gri, neden başta söylenir.
  - **Veri girişi:** kaydın özniteliklerini yalnız yayınlanmış formla değiştirdiği, nesne ekleyip silemediği söylenir.
- Önizleme, kaydın salt okunurluğunu formun sayfalarına da uygular (önceden hep düzenlenebilir çiziliyordu).

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Adımlar, ayrı adlar, önizlemenin kaydı ve rolü | yok: Mimari formsuzdur | var: çizim katmanının formu | yok: Yol formsuzdur | var |
| Kaynağa kaydet adı | yok: veri katmanı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Adlar ve adımlar pencerenin sunumudur. Komutlar aynıdır: `cad.forms.edit` (taslak, yayın), `cad.forms.submit` (kayıt), `cad.layers.session` save. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Yayın özeti:** alan, kural ve liste bağı değişikliklerini kaynak uyumluluğuyla birlikte gösteren bir özet. Bugün
  Değişiklikler sekmesi son yayınla farkı, ön denetim de kaynakta olmayan alanı (ADR 9318) ayrı ayrı söyler.
- Önizlemenin örnek kaydı bugün haritada seçili nesnedir; veri katmanının alınmamış bir satırını seçmek için tablodan
  seçim gerekecek.
- İlişki, alt tür ve değer bileşimi kaydının iki katmanlı gerçek veriyle ayrı kabulü.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- form traces usage`: 193 geçti.
  - `design_steps_tests`:
    - adımlar sırayla tamamlanıyor: önizleme “denedi”yi açıyor, Taslağı kaydet pencereyi açık ve kaydedilmiş bırakıyor,
      Formu yayınla yayını taslakla aynı yapıyor;
    - Okuyan'da önizleme salt okunur ve nedeni söyleniyor; Veri girişi'nde düzenlenebilir.
  - `form_scenes::editing`: Taslağı kaydet'in pencereyi açık bıraktığı beklentisiyle güncellendi.
- **Üreticiler:** `python3 scripts/ui/ribbon.py --check`, `python3 scripts/cad/manifest.py --check` (layer.editSave'in
  yeni başlığı).
- **Resim** (`.run/gui-evidence/kullanim-9325/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-adimlar`: adımlar satırı, alt satırda Vazgeç, Formu yayınla, Taslağı kaydet;
  - `02-onizleme-okuyan`: boş kayıt notu, Rol: Okuyan, gri alan.
