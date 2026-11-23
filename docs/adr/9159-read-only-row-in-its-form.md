# ADR 9159: Salt okunur bağlantının satırı formda yalnız görüntülenir

- **Durum:** kabul edildi (2026-11-23); adım VY-58. Plan: [plan/veri.md](../../plan/veri.md) VY-58.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin iki maddesinin
  kalanı:
  - 27. madde (ADR 9149): “Salt okunur bağlantıda form yalnız görüntülenmeli.”
  - 28. madde (ADR 9129): “Salt okunur durumu satırda ve form başlığında göster.” İki PostGIS bağlantısı ve bir
    GeoPackage arasında on kez geçildiğinde kişi hedefi kaydetmeden önce ayırt edebilmeli; her kayıtta yalnız seçilen
    hedefteki değer değişmeli.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- ADR 9149'da tablodan seçilen satır Formla düzenle'ye ulaştı. Salt okunur bağlantının katmanında form açılmıyor,
  yalnız “kayıtları formla yazılmaz” deniyordu. Kişi kaydın değerlerini formun düzeninde göremiyordu.
- Formun kaydı çizimdeki bir nesnedir. Düzenlenen katmanın satırı oturuma alınır (ADR 9101). Salt okunur katmanın
  düzenleme oturumu yoktur; satırı çizime almak, yazılamayacak bir kopya bırakırdı.
- Form başlığı salt okunurluğu söylemiyordu. Aynı adlı iki katmandan biri salt okunur olunca kişi bunu yalnız katman
  seçicide görüyordu (ADR 9129).
- **Araştırma.** Masaüstü CBS'lerde öznitelik tablosunun form görünümü seçilen kaydın değerlerini formun alan
  düzeninde gösterir. Değer değiştirmek için önce katman düzenlemeye açılır; açılmayan katmanda form yalnız gösterir.
  Kaynak: [QGIS öznitelik tablosu, “Table view vs Form view” ve “Editing attribute values”](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/attribute_table.html).

## §2 Karar

1. **Okunur, alınmaz.** Salt okunur katmanın satırı Formla düzenle'de kaynağından okunur. Okuma arayüzün iş
   parçacığında yapılmaz. Satır çizime alınmaz, düzenleme oturumu gerekmez.
   - Form kaydı çizimin dışında tutar (`Entry.row`: anahtar sütunu ve anahtar).
   - Form ekranını uygulama katmanı kurar (`forms::row_screen`). Kurallar satırın şeklini ve değerlerini görür;
     yeni kaydın “son değerleri” konmaz.
2. **Yalnız görüntülenir.**
   - Denetimler yazılamaz; Kaydet kapalıdır.
   - Klavyeyle gelen Kaydet de yazmaz, nedenini söyler.
   - Vazgeç sormadan kapatır, çünkü saklanacak bir şey yoktur.
3. **Başlık ve neden.**
   - Başlık kaydı anahtarıyla adlandırır ve sonuna “salt okunur” ekler: “Kapalı alan · parseller · fid 22 · salt okunur”.
   - Altında bilgi satırı nedeni söyler: “… salt okunur bağlantıdan geliyor: kayıt görüntülenir, değiştirilmez ve
     kaynağa yazılmaz.”
   - Aynı söz, alınmış ya da çizimdeki bir kayıt salt okunur katmandaysa da geçerlidir (kilit sözlerinin ilki).

## §3 Kanıt

- **Masaüstü** (`gis/forms/row_target_tests.rs`):
  - salt okunur bağlantının 22. satırı formda açılıyor: “Park” gösteriliyor, başlık “· fid 22 · salt okunur”;
  - çizime nesne alınmıyor;
  - değer yazılıp Kaydet gönderilince hiçbir şey yazılmıyor: dosyada hâlâ “Park”, bekleyen değişiklik yok;
  - **on geçiş:** iki GeoPackage dosyasının aynı adlı “parseller” tabloları ve birincinin bir de salt okunur
    bağlantısı var. Hedef on kez değişiyor. Her turda başlık salt okunur olanı kaydetmeden önce ayırıyor. Salt okunur
    bağlantı birinci dosyanın o anki değerini gösteriyor. Her kayıt yalnız seçilen dosyanın satırını değiştiriyor;
    sonunda “Deneme-9” ve “Deneme-7”, bekleyen değişiklik yok.
- Form, veri, rol, ilişkili kayıt ve ortak masaüstü sınamaları (361) geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9159/01-salt-okunur-form-{light,dark}.png`. Başlıkta “fid 22 · salt
  okunur”, bilgi satırı, gri kutuda “Park”, kapalı Kaydet.

## §4 Kalan

- On geçişin iki PostGIS bağlantısıyla gerçek sunucuda sınanması.
  **Yapıldı (23 Kasım 2026):** gerçek PostgreSQL'de sınandı, [ADR 9182](9182-postgresql-remainders-on-a-real-server.md).
- Salt okunur satırın formunda ilişkili kayıtlar (veri katmanı formunun ilişkili listesi, 22. maddenin kalanı).
- Web (TODOS.md §0.2).
