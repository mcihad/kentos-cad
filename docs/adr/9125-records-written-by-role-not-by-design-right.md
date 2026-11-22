# ADR 9125: Formla kayıt yazmak tasarım iznine değil rolün kayıt iznine bağlı

- **Durum:** kabul edildi (2026-11-22); adım VY-24. Plan: [plan/veri.md](../../plan/veri.md) VY-24.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  25. maddesi (P0):
  - bulut CBS projesi bir kişiyle **Veri girişi** rolüyle paylaşıldı;
  - o hesapla Formla düzenle açılınca her iki alan ve Kaydet kapalıydı;
  - sunucu aynı hesabın `project.changes` ile gönderdiği geçerli değişikliği kabul ediyordu (erişim yanıtında
    `forms.use` vardı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** değişmedi.

## §1 Bağlam

- **Masaüstündeki neden:** giriş formu, kaydetme, iş akışı adımı ve çoklu kayıt `forms_locked()`'a bakıyordu. Bu da
  katman ağacının kilidine (`tree_locked`, `project.edit`) bağlıydı. Bu izin formun tasarımı ve yayını içindir.
- **Rollerin izinleri** (ADR 0411 §1, sözleşmedeki `ProjectPermission`):
  - formu tasarlamak `forms.design`, yayınlamak `forms.publish` ister;
  - yayımlanmış formla kayıt yazmak `forms.use` ister, nesne yazmak `feature.write`.
  - Veri girişi rolünde `forms.use` vardır, `project.edit` yoktur.
- **Sunucu** kapsamı ve alan izinlerini ayrıca denetler (`form_guard`). Bu karar ona dokunmaz.
- **Masaüstü CBS:** QGIS 4.2 API'sinde form alanlarının düzenlenebilirliği (`QgsEditFormConfig`) ile katmandaki
  değişiklikleri sağlayıcıya gönderme işlemi ayrı tanımlıdır (değerlendirmede okunan belge).

## §2 Karar

1. **İki ayrı karar.**
   - `forms_locked()` yalnız formların tasarımı ve yayını içindir; değişmedi.
   - Yeni `records_locked()` yayımlanmış formla kayıt yazmanın kararıdır.
2. **Kayıt yazılmaz:**
   - bulut projesi arşivdeyse: “Proje arşivde: kayıtlar yalnız görüntülenir …”;
   - hesabın rolünde `forms.use` da `feature.write` da yoksa, rolün adıyla: “Bu projedeki rolünüz (Görüntüleyici) kayıt
     yazmaz; form yalnız görüntülenir. Veri girmek için proje sahibinden Veri girişi rolünü isteyin.”

   Yerel çizim ve dosya projesi eskisi gibi yazar.
3. **Formun kendi denetimleri sürer:**
   - yalnız okumaya açılan kayıt;
   - katmanın kilidi: ““Bina” katmanı kilitli; kayıt yalnız görüntülenir. Yazmak için katmanın kilidini açın.”;
   - alanların salt okunurluğu, zorunlu alan ve iş akışı durumu formun kurallarıdır, değişmedi.
4. **Neden görünür.** Form salt okunur açıldığında üstteki uyarı ve “Bu form” ipucu nedeni söyler: rol, arşiv ya da
   katman kilidi. Önceki ortak cümle (“Katman kilitli ya da yetkiniz yok”) kalktı.
5. **Aynı karar** formla kaydetme, iş akışı adımı, birçok kayda birden yazma ve alt kayıtlar için kullanılır.

## §3 Kanıt

- **Masaüstü** (`tests/form_roles_tests.rs`, form_scenes'in binaları ve formu):
  - Veri girişi (`project.read`, `forms.use`): kayıt yazılıyor (kat sayısı 2 → 5), form kapanıyor. Formların tasarımı
    ve yayını kapalı kalıyor;
  - Görüntüleyici: form açılıyor, uyarı rolün adını söylüyor, Kaydet hiçbir şey yazmıyor;
  - arşivdeki proje: rol ne olursa olsun yalnız görüntüleme;
  - yerel çizim eskisi gibi yazıyor.
- Formların masaüstü sınamaları (40) geçiyor.
- **Resimler** (`.run/gui-evidence/kullanim-9125/`): aynı form Veri girişi ve Görüntüleyici hesabıyla. Açık tema
  1440 × 900, koyu tema 1100 × 760.

## §4 Kalan

- Değerlendirmenin kabulü iki gerçek hesapla bulut sunucusunda yeniden sınanmalı:
  - sahip v2'yi yayınlar;
  - Veri girişi yeni etiketi görür ve yalnız `Durum`'u yazar;
  - başka oturum yeni değeri okur.

  Sunucunun form sınamaları (`project_forms`, `project_form_scope`) veritabanı ister. Bu oturumda Docker yanıt
  vermedi, koşturulmadı.
- Saha kullanıcısına verilmeyen alanın tabloda boş görünmesi raporun 29. maddesidir. Web (TODOS.md §0.2).
