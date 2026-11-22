# ADR 9152: Kaynakta silinen satırın taslağı bırakılınca sağlam satırlar yazılır

- **Durum:** kabul edildi (2026-11-23); adım VY-51. Plan: [plan/veri.md](../../plan/veri.md) VY-51.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  32. maddesi (P1) ve 26/31 için ek kabul örneği:
  - iki satır değiştirildi, ikinci kullanıcı birini sildi;
  - uyarı silinen FID'yi bilmesine rağmen birincil eylem “Üstüne yaz”dı ve sağlam satır da yazılmadı;
  - önerilen katman geri alma sağlam satırdaki girişi de kaldırdı;
  - bağlantı dönünce çakışan satırın değiştirilmeyen notu tabloda eski değeriyle göründü;
  - silinmiş FID'nin taslağı tabloda görünmedi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- ADR 9148 çakışmaları satır satır taşır. Kaynakta silinen satır için iki yol sunar: “Yeni satır olarak ekle” ve
  “Değişikliğimi bırak”. Sağlam satırlar taslakta kalır, seçimden sonra grup yeniden tek işlemde kaydedilir. Eski anahtar
  kendiliğinden canlandırılmaz.
- Tablo alınmış satırı nesnenin bütün değerleriyle gösteriyordu: burada değişmemiş alan da alındığı anki değerdi.
- Kaynakta silinen satır kaynağın sayfalarında olmadığı için tabloda hiç görünmüyordu.
- Değişiklikleri geri al katmanın bütün bekleyen satırlarını bırakır; ne kadarını bıraktığını söylemiyordu.

## §2 Karar

1. **Ne yazılacağı söylenir.** Çakışma penceresi seçimin altında yazar: “Seçilenleri kaydet 1 satırı tek işlemde yazar;
   1 satırın taslağı bırakılır.”
2. **Silinen satır tabloda.** Bir kaydın kaynakta silindiğini bulduğu satırlar katmanın kaydı, geri alınması ya da
   satırın taslağı bırakılıncaya dek tabloda listelenir. Yeni satırlardan sonra, “22 kaynakta silinmiş” ve yerel
   değerleriyle; anahtar sütunu işaretiyle sığacak kadar genişler.
3. **Değişmeyen alan kaynağınkidir.** Alınmış satırın burada değişmemiş alanı tabloda kaynağın şimdiki değerini gösterir;
   yalnız burada değişen alan taslağındır (`cell_value`).
4. **Geri almanın sayısı.** Değişiklikleri geri al her katmanın bıraktığını söyler: “Katman değişiklikleri geri alındı
   (parseller: 2 değişen satır): bunlar bırakıldı, kaynak olduğu gibi kaldı.”

## §3 Kanıt

- **Masaüstü, GeoPackage'ta** (`gis/data_gone_tests.rs`):
  - 22 ve 23 alınıp değiştirildi, 22 başkasınca silindi; ilk kayıt ikisini de yazmıyor;
  - pencere 22'yi kaynakta silinmiş gösteriyor, “2 satırı tek işlemde yazar” diyor; tablo 22'yi listeliyor;
  - Değişikliğimi bırak seçilince “1 satırı … 1 satırın taslağı bırakılır”;
  - kaydedilince 23 “Pazar”, 22 ne yeniden ne yeni satır olarak yazılıyor; bekleyen yok.
  - Değişmeyen alan için kaynağın değeri, değişen için taslağınki geliyor.
  - Geri alma “parseller: 2 değişen satır” diyor.
- Veri katmanı, öznitelik tablosu, şerit, modül ve geçmiş sınamaları (132) geçiyor.
- **Resimler** (`.run/gui-evidence/kullanim-9152/`, Git'e girmez):
  - pencere, silinen satır ve sayı;
  - tabloda “22 kaynakta silinmiş”.

## §4 Kalan

- **Gerçek PostgreSQL.** Çevrimdışı açılış, başka personelin silmesi ve UPDATE'in etkilediği satır sayısı gerçek
  sunucuda koşturulmadı (Docker). Sağlayıcı silinmiş satırda etkilenen satır sayısını denetliyor (ADR 9104).
- **Yeni kayıt yetkisi.** “Yeni satır olarak ekle” ekleme yetkisini ayrıca sormuyor; yetkisizse kaynak yazımı reddeder.
- Web (TODOS.md §0.2).
