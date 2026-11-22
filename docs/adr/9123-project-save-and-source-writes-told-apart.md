# ADR 9123: Proje kaydı ile kaynağa yazım ayrı görünür; Katman değişiklikleri penceresi

- **Durum:** kabul edildi (2026-11-22); adım VY-22. Plan: [plan/veri.md](../../plan/veri.md) VY-22.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  2. maddesi (P0) ve 3. saha turundaki ek kabul örneği:
  - veri katmanında bir hücre değiştirildi, üstteki disketle proje kaydedildi;
  - proje sekmesinin değişiklik noktası kalktı ama satır bekliyordu, ayrı bir SQLite okuyucusu eski değeri gördü;
  - yeni değer ancak katman menüsündeki “Değişiklikleri kaydet”ten sonra diske ulaştı;
  - formdan kaydedilen değer de katmanda bekliyordu, ileti ise “yazıldı” diyordu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok; kaynağa yazım ve geri alma
  ADR 9103'teki düzenleme oturumunun işleridir.

## §1 Bağlam ve araştırma

- **Masaüstü CBS'ler** (22 Kasım 2026'da okundu):
  - *ArcGIS Pro 3.5, Enable and disable editing* ile *3.3, Save or discard edits*: düzenlemeler kaydedilene ya da
    atılana kadar bekler. Kaydetme ve atma düzenleme oturumunun kendi komutlarıdır.
  - *QGIS 4.2* (`QgsVectorLayerEditBuffer`, `QgsVectorLayer::commitChanges`): katmanın düzenleme tamponu sağlayıcıya
    ayrıca gönderilir. Proje dosyasını yazmak tamponu kaynağa yazmaz.
  - Değerlendirmede QGIS'in ikinci istemcisi geri alma yolunda eski değeri, commit yolunda yenisini gördü. Kabulün
    ölçütü de budur: fiziksel kaynaktaki görünürlük.
- **KentOS'ta durum:**
  - kaynağa yazım zaten ayrıydı (ADR 9103 §5, 9104);
  - kaydedilen projenin bekleyen satırları bu cihazda bir taslak günlüğünde tutuluyordu (ADR 9111 §3).
  - Ama tek görünür iz, katman ağacında uyarı renginde bir kalem ve ipucuydu. Proje kaydedilince değişiklik noktası
    da kalkıyordu.
- **Bulunan hata (günlüğün yeri):**
  - günlüğün yeri `.kcad` dosyasının bağlantıları çözülmüş yoludur (`file_place`);
  - ilk kayıtta dosya henüz olmadığından yol çözülmüyor, açılışta çözülüyordu;
  - bağlantılı bir klasörde (macOS'ta `/var`) bu yüzden iki ayrı yer çıkıyor, bekleyen düzenlemeler proje açılınca
    gelmiyordu;
  - `data_journal_tests` bu makinede bunu gösterdi.

## §2 Karar

1. **İki ayrı iz.**
   - **Projenin noktası** yalnız projeyi (katmanlar, görünüşler, ayarlar) söyler. İpucu: “Proje kaydedilmedi”.
   - **Kaynağa yazılmayan satırlar** ayrıca sayılır:
     - adın yanında amber bir sayı rozeti;
     - durum çubuğunda “Kaynağa yazılmadı: 2 katmanda 3 satır” hücresi. Çubuk sıkıştıkça “3 satır bekliyor”, sonra
       yalnız simge ve sayı kalır; ipucu bütün sözleri taşır.
   - Kaynağın reddettiği son yazımdan sonra hücre tehlike rengindedir: “Kaynağa yazılamadı …”. İpucu nedeni söyler.
   - Rozet ve hücre tıklanınca Katman değişiklikleri açılır.
2. **Katman değişiklikleri penceresi** (`gis/data_pending.rs`, liste ve ayrıntı).
   - **Solda** bekleyen katmanlar: adı, kaynağı (dosya ya da veritabanı › tablo), sayıları, yazılamadıysa “yazılamadı”.
   - **Sağda** seçilen katmanın satırları bir veri tablosunda: Durum, Anahtar, Alan, Kaynakta, Bu cihazda.
     - değişen satır alan alan gösterilir, şekli değiştiyse bir satırla;
     - yeni satır bütün değerleriyle, silinen satır kaynaktaki hâliyle gösterilir.
   - **Kaynağa yaz** o katmanı yazar. **Değişiklikleri geri al…** ikinci basışta, kaç satırın bırakılacağını söyleyerek
     bırakır. Birden çok katman bekliyorsa **Hepsini kaynağa yaz**.
   - Üst satır bekleyenlerin nerede durduğunu söyler:
     - proje kaydedildiyse bu cihazda taslak olarak; proje dosyasını başka bir bilgisayara kopyalamak onları taşımaz;
     - proje kaydedilmediyse yalnız bu oturumda;
     - bulut projesinde cihazda saklanmaz.
   - Katmanın menüsünde **Değişiklikleri incele…** pencereyi o katmanda açar.
3. **İletiler sayıyı ve yeri söyler.**
   - Proje kaydı: “Proje kaydedildi; katmanlardaki 2 satır kaynağa yazılmadı (parseller-b: 1, parseller: 1). Bu
     satırlar bu cihazda taslak olarak saklandı …”. Sayı yeni satırları da içerir: onlar proje dosyasında durur ama
     kaynakta yoktur.
   - Kaynağa yazım: “… kaynağa yazıldı”.
   - Geri alma: “… kaynak olduğu gibi kaldı”.
   - Formun Kaydet'i bir veri katmanının kaydını değiştirince ileti başarı değil bilgidir: “Değer “bakim” katmanında
     bekliyor, kaynağa yazılmadı: 1 değişen satır. …”. Başarı iletisi ancak kaynağa yazım bitince gelir.
4. **Kapatma ve proje değiştirme sorusu** her seçeneğin kaynağa ve bu cihaza etkisini söyler:
   - **Kaynağa yaz:** başka kullanıcılar ve programlar satırları o zaman görür.
   - **Taslak olarak tut** (önceki adı “Projede tut”): kaynağa yazmaz. Proje kaydedilince bu bilgisayarda saklanır,
     proje burada açılınca geri gelir; dosyayı başka bilgisayara kopyalamak onları taşımaz.
   - **Değişiklikleri geri al:** satırları bırakır, kaynak olduğu gibi kalır.
   - **Vazgeç.**
5. **Yazım başarısız olunca** değişiklikler olduğu gibi kalır.
   - Neden katman katman tutulur (`DataEdits::failed`); hücre, pencere ve listenin satırı söyler.
   - Kaynağa yaz yeniden dener. Yazım gidince, satırlar geri alınınca ya da düzenleme bitince neden silinir.
6. **Günlüğün yeri:** henüz yazılmamış bir dosya, çözülmüş klasörü ve adıyla yerlenir. Kayıtta ve açılışta aynı yer
   çıkar.
7. **Sayım** çizimin her değişiminde bir kez yapılır (oturum ve kuşak anahtarıyla saklanır). Durum çubuğu her karede
   satır karşılaştırmaz (CLAUDE.md §6.2).

## §3 Kanıt

- **Masaüstü** (`gis/data_pending_tests.rs`), iki GeoPackage, iki veri katmanı ve bir yerel katman değişikliği; her
  adımda sayılar kaynağı kendi bağlantısıyla okuyan ikinci okuyucuyla karşılaştırıldı:
  - proje kaydı: proje temiz, “2 katmanda 2 satır”, ikinci okuyucu iki kaynakta da eski değerleri görüyor;
  - pencereden tek katman yazıldı: “1 katmanda 1 satır”, okuyucu yalnız o satırın yenisini görüyor;
  - proje dosyadan yeniden açıldı: bekleyen satır bu cihazdan geri geliyor, sayı aynı;
  - pencereden geri alındı: ilk basış yalnız soruyor, ikincisi bırakıyor; bekleyen yok, kaynak eski hâlinde;
  - Hepsini kaynağa yaz, salt okunur kaynakta: o katmanın satırı kalıyor, hücre “Kaynağa yazılamadı” diyor, neden
    satırda; izin dönünce yeniden deneme yazıyor, neden siliniyor;
  - veri katmanındaki kaydın beklediğini söyleyen ileti.
- `data_journal_tests` (bağlantılı geçici klasörde ilk kayıttan sonra günlük bulunuyor) ve `data_edits_tests` geçiyor.
  `ribbon_tests` ile `app_menu` da geçiyor.
- **Resimler** (`.run/gui-evidence/kullanim-9123/`): durum hücresi ve ad rozeti; Katman değişiklikleri penceresi. Açık
  tema 1440 × 900, koyu tema 1100 × 760.

## §4 Kalan

- 1100 piksellik CBS penceresinde durum çubuğu zaten taşıyordu, bulut hücresi kenarda kesiliyor. Çubuğun CBS'deki
  içeriği raporun 12. maddesinde ele alınacak; bu hücre sıkışınca simge ve sayıya iner.
- Çevrimdışı açılışta kaynak sayfası alınamazken bekleyenlerin tabloda görünmesi raporun 31. maddesidir. Bu pencere
  onların ilk görünümüdür.
- Gerçek pencerede fare ve klavyeyle deneme (`scripts/usage/real/`). Web (TODOS.md §0.2).
