# ADR 9161: Yayın denetimi formu durduran sorunları türleriyle ayrı söyler

- **Durum:** kabul edildi (2026-11-23); adım VY-60. Plan: [plan/veri.md](../../plan/veri.md) VY-60.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 8. maddesinin
  kalanı (ADR 9141): “Yayımlama denetimi boş düzenleme formu, zorunlu alan, varsayılan ve geçersiz domain değerini
  ayrı açıklasın.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `cad.forms.edit` yayınının `form_preflight` reddi her türü kendi başlığıyla söyler ve boş formu da
  reddeder. Kod ve alan yolu aynı kaldı.

## §1 Bağlam

- Formun ön denetimi (`kentos_forms::preflight`) bu sorunların her birini kendi koduyla verir:
  - formda alan yok (`form_empty`);
  - katmanın zorunlu alanı formda yok ya da gizlenebiliyor (`form_required_hidden`);
  - zorunlu alan varsayılansız salt okunur (`form_required_read_only`);
  - varsayılan değer alanın değeri değil (`form_default_invalid`);
  - liste aracı listesiz alanda (`form_widget_needs_values`), seçenek süzgeci listesiz alanda (`form_choice_unlisted`).
- Ancak üç eksik vardı:
  - tasarımcı en çok sekiz iletiyi türüne bakmadan tek listede sıralıyordu;
  - Yayınla kapalıyken nedenini söyleyen bir söz yoktu;
  - komutun reddi yalnız ilk hatayı söylüyordu (“… yayınlanamıyor: {ilk hata}”). Python ve MCP de bu sözü alır.
- Boş form çizilirken uyarıdır, çünkü boş taslak saklanabilir (ADR 9141). Ama yayınlanabiliyordu. Alanı olmayan
  yayın hiçbir kaydı düzenlemez.
- **Araştırma.** Masaüstü CBS'lerin öznitelik formlarında iki tür kısıt vardır. Zorunlu kılınmamış kısıt sarı işaretle
  uyarır ve kaydetmeyi durdurmaz. Zorunlu kılınmış kısıt turuncu işaretle kaydetmeyi durdurur. Varsayılan değer yeni
  kayda konur. Kaynak: [QGIS vektör katman özellikleri, “Attributes Form Properties”, kısıtlar ve varsayılan
  değerler](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html).

## §2 Karar

1. **Türler** (`kentos_forms::publish_check`; tasarımcı ve komut bunu ortak kullanır). Yayını durduran sorunlar şu
   sırayla söylenir:
   - **Boş form:** `form_empty`;
   - **Zorunlu alan:** `form_required_hidden`, `form_required_read_only`;
   - **Varsayılan değer:** `form_default_invalid`;
   - **Değer listesi:** `form_widget_needs_values`, `form_choice_unlisted`;
   - **Öbür sorunlar:** ön denetimin kalan hataları.
2. **Yayını durduran:** ön denetimin her hatası ve formun boşluğu. Boş form çizilirken uyarı kalır; taslak saklanır,
   yayınlanmaz.
3. **Komut.** Yayın reddi bütün türleri bir arada söyler:
   - biçim: ““Yol formu” formu yayınlanamıyor. Zorunlu alan: …. Varsayılan değer: …. Değer listesi: …. Formu
     tasarımcıda düzeltin.”;
   - bir türün birden çok iletisi “;” ile ayrılır.
4. **Tasarımcı.**
   - Sorun listesi başlıklıdır: önce yayını durduranlar türleriyle (“Zorunlu alan (1)”), sonra “Uyarılar”. Bir iletiye
     basmak öğesine götürür.
   - Alt satır nedeni söyler: “… · yayınlanamaz: 1 zorunlu alan, 1 varsayılan değer, 1 değer listesi” ya da “… ·
     yayınlanamaz: boş form”.
   - Yayınla bunlar varken kapalıdır.

## §3 Kanıt

- **Form çekirdeği** (`kentos-forms`, `publish_check` sınamaları):
  - türler sırasıyla toplanıyor; uyarı yayını durdurmuyor;
  - söz biçimi kelimesi kelimesine sınanıyor;
  - boş form çizilirken hata değil, yayında durduruyor.
- **Masaüstü** (`gis/forms/new_field_tests.rs`, 8. madde):
  - alanı olmayan yol katmanının boş formu “yayınlanamıyor. Boş form: Formda alan yok…” diye reddediliyor;
  - sonra tek redde üç tür sırasıyla söyleniyor: zorunlu “durum” formda yok, “tur”un varsayılanı “Köşk” listede yok,
    “tip” listesiz açılır liste. Hiçbir yayın yazılmıyor.
- Paylaşılan komut durumları masaüstü işleyicileriyle eşleşiyor. Form çekirdeği (59), form, iz ve ortak masaüstü
  sınamaları (347) geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9161/01-yayin-denetimi-{light,dark}.png`. Üç başlık altında birer ileti, alt
  satırda “yayınlanamaz: 1 zorunlu alan, 1 varsayılan değer, 1 değer listesi”, kapalı Yayınla.

## §4 Kalan

- 8. maddenin öbür kalanları: masaüstünde yayımlayan rolün ayrılması, yol formunda alan listesi, koşullu alan, alt
  bakım kayıtları, rol kısıtı ve sürüm yükseltmesinin aynı veride bütün sınaması.
- `form_preflight` kodunun katalog açıklamasına “boş form (yayında)” eklenmesi (üretilen belgeler yenilenince).
- Web (TODOS.md §0.2).
