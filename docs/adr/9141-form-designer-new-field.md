# ADR 9141: Form tasarımcısı alanı olmayan katmanı söyler ve alanı yerinde oluşturur

- **Durum:** kabul edildi (2026-11-22); adım VY-40. Plan: [plan/veri.md](../../plan/veri.md) VY-40.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  8. maddesi (P1):
  - alanı olmayan yol katmanında tasarımcı aynı anda hem “katmanın bütün alanları formda” hem “form boş” dedi;
  - `durum` alanını oluşturmak için ayrı Alanlar penceresine gidip dönmek gerekti.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yeni yok (`cad.layers.edit`'in
  alanları).

## §1 Bağlam

- Tasarımcının alan paleti formda olmayan alanları listeliyordu. Liste boşsa, katmanın hiç alanı olmasa da “bütün
  alanları formda” yazıyordu.
- Alan oluşturmak Alanlar penceresindeydi. Kaydet orada pencereyi kapatıyor, kişi forma kendisi dönüyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının katman özelliklerinde “Öznitelik formu” ile “Alanlar” aynı
  pencerenin iki sekmesidir; alan eklenince form düzenleyicisinin alan listesinde hemen görünür.
  Kaynak: [vektör katman özellikleri](https://documentation.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Doğru söz.**
   - Alanı olmayan katmanda palet “Katmanda alan yok: aşağıdan bir alan oluşturun, forma da eklenir.” der.
   - “Katmanın bütün alanları formda.” yalnız alanların hepsi formdayken söylenir.
   - Boş formun ağacı katmanda alan yoksa bunu söyler, varsa eskisi gibi alanlara basmayı önerir.
2. **Yeni alan.** Paletin altında ad, tür ve Oluştur vardır.
   - Oluştur alanı katmanın alanlarına `cad.layers.edit` ile yazar; bu bir geri alma adımıdır.
   - Alan seçili öğenin yanına forma konur; kişi formda kalır.
   - Katmanda olan bir ad reddedilir, listeden eklenmesi söylenir.

## §3 Kanıt

- **Masaüstü** (`gis/forms/new_field_tests.rs`):
  - alanı olmayan “Yol” katmanında yeni form;
  - Yeni alan `durum` katmana yazılıyor, formun öğesi oluyor, tasarımcı açık kalıyor, yazılan ad temizleniyor;
  - aynı ad reddediliyor.
- **Resimler** (`.run/gui-evidence/kullanim-9141/`, Git'e girmez): palet ve ağaç alan olmadığını aynı sözle söylüyor;
  Oluştur'dan sonra `durum` formda.

## §4 Kalan

- **Yayımlama denetimi.** Boş düzenleme formu, zorunlu alan, varsayılan ve geçersiz liste değeri ayrı ayrı
  açıklanmalı; bugün ön denetimin satırlarıdır.
- **Roller.** Tasarımcı, yayımlayan ve yalnız veri giren rollerinin ayrımı bulutta var (ADR 9125); masaüstünde
  yayımlamanın rolü ayrı değil.
- **Sınama.** Yol formunda liste, koşullu alan, alt bakım kayıtları, rol kısıtı ve sürüm yükseltmesinin aynı veride
  sınanması; kaynak destekliyorsa kısıtın veritabanında uygulanması.
- Web (TODOS.md §0.2).
