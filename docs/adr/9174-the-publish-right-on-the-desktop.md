# ADR 9174: Masaüstü formu yayımlama iznini tasarlama izninden ayırır

- **Durum:** kabul edildi (2026-11-23); adım VY-73. Plan: [plan/veri.md](../../plan/veri.md) VY-73.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 8. maddesinin
  kalanı (ADR 9141): “Tasarımcı, yayımlayan ve yalnız veri giren rollerini ayır.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Bulut projesinin izinleri formu tasarlamayı (`forms.design`), yayımlamayı (`forms.publish`) ve formla kayıt
  girmeyi (`forms.use`) ayırır.
- Sunucu da ayırır (`form_guard::settings_needs`):
  - bir sürümün eklenmesi ya da düşmesi `forms.publish` ister;
  - taslak, yayından kaldırma, kuralları uygulama ve ilişkiler `forms.design` ister.
- Masaüstü yalnız proje düzenleme kilidine bakıyordu. Bugünkü rollerde (Yönetici, Sahip) iki izin hep birlikte
  geliyor. Ama tasarlayıp yayımlamayan bir izin kümesinde Yayınla açık görünüyor ve sunucunun reddiyle düşüyordu.
- **Araştırma.** Proje rolleri ve izinleri KentOS'un kendi tasarımıdır (ADR 0411). Sunucunun kuralı kaynaktır:
  `crates/server/application/src/form_guard.rs`, `access.rs`. Dış bir ürünün belgesine dayanmaz.

## §2 Karar

1. **Yayımlama kilidi** (`forms_publish_locked`). Önce tasarımın kendi kilidine bakılır (proje kilidi, ADR 9125).
   Sonra bulut projesinin rolünde `forms.publish` yoksa Yayınla kilitlidir. Neden rolün adıyla söylenir: “Bu projedeki
   rolünüz (…) formu yayınlamaz: taslağı kaydedebilirsiniz, yayını form yayımlama izni olan bir proje yöneticisi
   yapar.”
2. **Tasarımcı.**
   - Yayınla kapalıdır; alt satır “… · rolünüz yayınlamaz” der.
   - Kaydet, Yayından kaldır ve Kuralları uygula açık kalır; sunucu bunları tasarım iznine bağlar.
   - Yayınla yine de gönderilirse (kısayol, otomasyon) hiçbir şey yazılmaz ve neden söylenir.
3. **Yerel proje** (bulut kaynağı olmayan) kilitlenmez; izinler bulut projesine aittir.

## §3 Kanıt

- **Masaüstü** (`tests/form_roles_tests.rs`): yayınlanmış binalar formu; rol Yönetici, izinler okuma, düzenleme,
  `forms.design` ve `forms.use`.
  - Tasarım açık; yayımlama kilidi rolün adıyla “formu yayınlamaz … taslağı kaydedebilirsiniz” diyor.
  - Yayınla gönderilince yayın sayısı değişmiyor; neden söyleniyor.
  - `forms.publish` eklenince kilit kalkıyor.
- Form, bulut ve ortak masaüstü sınamaları geçiyor (466).
  - `typing_tests::project_settings_scale_and_heights_draw_each_key_at_once` toplu koşuda bir kez düştü, tek başına
    geçti. Konuyla ilgisi yok; paralel koşu kararsızlığı.
- **Resim:** `.run/gui-evidence/kullanim-9174/01-yayinlamayan-rol-{light,dark}.png`. Formlar penceresinde alt satır
  “Taslak 3. sürüm · son yayın 1 · rolünüz yayınlamaz”, Yayınla kapalı, Kaydet açık.

## §4 Kalan

- 8. maddenin öbür kalanı: yol formunda değer listesi, koşullu alan, alt bakım kayıtları, rol kısıtı ve sürüm
  yükseltmesinin aynı veride bütün sınaması.
- Tasarlayıp yayımlamayan hazır bir rol yoktur; rol tablosunda böyle bir rol açılması sahibin kararıdır.
- Web (TODOS.md §0.2).
