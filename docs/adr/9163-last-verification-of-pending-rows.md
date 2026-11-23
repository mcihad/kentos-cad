# ADR 9163: Bekleyen satırlar kaynağın son doğrulama zamanını söyler

- **Durum:** kabul edildi (2026-11-23); adım VY-62. Plan: [plan/veri.md](../../plan/veri.md) VY-62.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 31. maddesinin
  kalanı (ADR 9151): “Güncellenen satır için FID, önceki ve yerel değer ile veri kaynağından son doğrulama durumunu
  göster.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.
- **`.kcad`:** değişmedi. Projenin bu cihazdaki veri günlüğüne isteğe bağlı `verified` eklendi; eski günlükler okunur.
- **Komut:** yok.

## §1 Bağlam

- Kaynak okunamazken tablo oturumun bekleyen satırlarını listeliyor (ADR 9151):
  - değişen satır anahtarıyla ve “Okul (önce: Park)” biçiminde;
  - durum satırı “kaynak okunamadığı için doğrulanmadı” diyor.
- Ama kişi kaynağın en son ne zaman doğrulandığını göremiyordu. Bir saat önce okunmuş kaynakla üç gün önce okunmuş
  kaynak aynı görünüyordu. Proje çevrimdışı yeniden açılınca bu bilgi hiç yoktu.
- **Araştırma.** Saha uygulamalarında indirilmiş haritadaki değişiklikler eşitlenene kadar yalnız cihazda kalır.
  Başkalarının değişiklikleri de cihaza yalnız eşitlemede gelir; bağlantı yokken eşitleme olmaz. Bu yüzden kişinin
  yerel kopyanın kaynağa göre ne kadar eski olduğunu bilmesi gerekir. Kaynak:
  [ArcGIS Field Maps, Sync](https://doc.esri.com/en/arcgis-field-maps/latest/use-maps/sync.html?pivots=mobile-ios).
  Belge bir “son eşitleme” zamanından söz etmiyor; zamanı göstermek KentOS'un kararıdır.

## §2 Karar

1. **Ne zaman doğrulanmış sayılır.** Katmanın kaynağı her yanıt verdiğinde zaman tutulur (`DataEdits::verified`,
   1970'ten beri milisaniye):
   - tablonun bir sayfası okunduğunda;
   - değişiklikler kaynağa yazıldığında.
2. **Günlükle gider.** Proje kaydedilirken, bekleyen satırı olan katmanların zamanı veri günlüğüne yazılır
   (`verified`). Kaynağa yazım sonrası güncellenen günlük de onu taşır.
   - Proje yeniden açılınca zaman günlükten geri gelir.
   - Bekleyen satırı olmayan katmanın zamanı günlüğe yazılmaz. Yalnız bu bilgi günlük dosyası bırakmaz.
3. **Söz.** Kaynak okunamazken durum satırı şöyle der: “Bekleyen değişiklikler: 1 değişen satır; kaynak
   okunamadığı için doğrulanmadı (son doğrulama: 22.11.2026 14:32)”.
   - Zaman cihazın saat diliminde, kısa tarih ve saatle yazılır (`local_time::stamp`).
   - Bilinmiyorsa “(son doğrulama: bilinmiyor)” der.
   - Satır başına “önce” değeri, satırın alındığı andaki kaynak değeridir (ADR 9151).

## §3 Kanıt

- **Masaüstü** (`gis/features/pending_rows_tests.rs`):
  - sözün iki biçimi (bilinmiyor ve zamanlı) kelimesi kelimesine sınanıyor;
  - tablonun sayfası okununca katmanın zamanı “şimdi” oluyor;
  - proje kaydedilince günlükte `"verified":{"<katman>":<zaman>}` var;
  - proje taslakla bırakılıp uygulamanın belleğindeki zaman silinince, yeniden açılışta zaman günlükten aynen geliyor
    ve söz onu söylüyor.
- Veri, tablo, kayıt, günlük ve ortak masaüstü sınamaları (272) geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9163/01-son-dogrulama-{light,dark}.png`. Kaynağı okunamayan tabloda “22
  değişen · Okul (önce: Park)” ve satırın sonunda “(son doğrulama: 22.11.2026 14:32)”.
- Resimdeki kaynak hatası SQLite'ın kendi İngilizce sözüdür (“no such table”); bu kararın konusu değil.

## §4 Kalan

- Gerçek PostgreSQL kesintisi (ADR 9151 §4).
- Satır alma anının satır başına gösterilmesi; şimdilik satırın “önce” değeri alındığı andakidir.
- Kaynak hatalarının Türkçe söylenmesi.
- Web (TODOS.md §0.2).
