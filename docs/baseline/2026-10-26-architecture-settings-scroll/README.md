# Yeni araçta görünür ayarlar — ADR 0657

26 Ekim 2026. **Bu dar düzeltme kabul edildi.**

ADR0656 denemesinde önceki nesne özelliklerinde aşağı kaydırılmış panelin yeni
mimari araçta aynı yerde kalabildiği görüldü. 02 ham görüntüde eski değişmez
pakette gerçek Mimari > Mahal tıklaması sonrası yöntem/kat ve ayar başlığı
üstte gizlidir; alt numara/ad/kullanım görünür. 01 tanısaldır: Etiket etkinken
komut satırından yeni araca geçme girişimi aracı bitirdi; Mahal etkin değildir.
Bu görüntü başarılı geçiş kanıtı sayılmaz; gerçek şerit yolu 02'dir.

Masaüstü Öznitelikler gövdesi mevcut tek scroll'u aynı stil/ölçüyle kendisi
kurar; özel widget kimliğiyle adreslenir. Başarılı ve görünür ayarlı yeni araç
paneli gösterdikten sonra yalnız bu alanı başlangıca getirir. Genel docking,
araç içindeki her tuş/değer/önizleme veya diğer paneller sıfırlanmaz. Eski
şeffaf gezinme/ölçü erken dönüş rotası korunur. Üç ürün dosyası ve bir ayrı
widget regresyon testi değişti. Yeni TS/komut/şema/SDK/birim/render yoktur.

## Otomatik kontroller

Komut/zaman/çıkış kodu/log SHA256 `test-runs.json` içindedir. Tüm Cargo
çalışmalarında `TS_RS_EXPORT_DIR` geçici `.run` dizinine yönlendirildi.
Web/envanter ADR0228 gereği donmuş olarak kaldı.

- `red-scroll`: eski kodda yeni araç sonrası gerçek widget ötelemesi
  **508 px**, beklenen 0; test bu davranış nedeniyle başarısız.
- `green-scroll`: aynı geçiş ve ayar düzenleme koruması geçti. Son teste ayrıca
  boş olmayan mevcut seçim koruması eklendi; kabul sonraki `settings-final`.
- `settings-final`: **4 başarılı, 2 atlanan**; yeni widget regresyonu gerçek
  scroll state'ini ve uygulamanın döndürdüğü widget görevini sürer. Etiket→Mahal
  ve Mahal→Etiket başlangıcı 0; arada geçerli yazı boyu düzenlemesi eski
  ötelemeyi korur. Tam belge ve mevcut seçim aynı kalır. Diğer üç mevcut ayar/
  eskimiş olay/snap testi de geçer. İki atlanan GPU matrisi bu dilimde çalışmadı.
- `properties-final`: **42 başarılı, 4 atlanan**; mevcut özellik semantiği,
  birimli uzunluklar, atomik ret ve ilişkiler. Görüntü/performans için atlanan
  dört kontrol başarı sayılmaz.
- `saving-final`: **6**, `keys-final`: **10** başarılı mevcut host testi.
  Son dört kümenin toplamı **62 başarılı, 6 atlanan**; önceki tekrarlar eklenmez.

`clippy-final` (desktop bütün hedefler, `-D warnings`) ve `build-final` geçti.
Mevcut ts-rs/future-compat bildirimleri günlüklerde korunur.
Python/MCP veya interaction katmanı değişmedi; önceki dilimde geçen testler
bu dilimde yeniden çalışmış gibi sunulmaz.

## Gerçek native pencere

Son paket `after-binary.json` ile dört kaynak hash'ine ve değişmez executable'a
bağlıdır. 03–19 son paketten ham JPEG'dir; düzenleme/kırpma/ölçekleme yoktur.
Uygulama, normal kullanıcı tercihleri yerine ayrı QA profiliyle çalışır;
koyu13/açık18 JSON'ları gerçek görünüm seçimlerini doğrular.

- 03: gerçek Salon seçili, nesne özellikleri aşağı kaydırılmış.
- 04 tanısaldır: seçim bağlamsal şeridi açtığından önceki Mimari düğmesinin
  boş kalan koordinatına tıklama araç başlatmadı; başarı kanıtı değildir.
- 05: görünür **Mahal bağlamı > Etiket** düğmesiyle başlatma ayar başlığı ve
  dört alanı üste getirir. Kaydırılmış özelliklerin altında kalmaz; seçim #5.
- 06–08: aynı Etiket çalışırken nesne özelliklerine aşağı kaydırılır; gerçek
  Ad hücresinde `Oturma`/tek Enter, iki bağlı etiket ve çizelge güncellenir.
  Panel yerinde kalır. Bir Cmd-Z Salon'u ve bütün takipçileri geri getirir;
  her aşama Cmd-S ile gerçek diske yazılır. Ayar/önizleme nesne üretmez.
- 09/10: aşağı kaydırılmış önceki araçtan gerçek **Mimari > Mahal > Kapı**
  geçişleri ilk yöntem/kat veya açıklık alanlarını üste getirir; seçim korunur.
- 11/12: Özellikler kapatılır, Etiket aracı başlayınca tekrar açılır ve baştan
  gösterilir. Üst Katmanlar yığını mevcut davranışla korunur.
- 13–15: **1100×683 pt açık18**; üst yığın elle daraltılır. Kaydırılmış
  özelliklerden Mahal→Kapı→dar Belgele menüsü > Etiket; ilk ayarlar görünür.
  Uzun listelerin alttaki alanları mevcut kaydırma ile erişilir; tüm alanların
  her küçük panelde aynı anda sığdığı iddia edilmez.
- 16/17: elle biraz aşağı kaydırılmış yazı boyunda `3,5 mm`/tek Enter ve gerçek
  Salon tıklama/tekerlek önizlemesi panel konumunu değiştirmez.
- 18: hedefi bırakıp araçtan çıkmak için iki Escape, Cmd-S; yeni nesne yok.
- 19: Cmd-Q sonrasında aynı paket yeni süreçte açılır; Home/Cmd-S; 14 nesne ve
  etiket/çizelge ilişkileri korunur. 03–12 **1440×863 pt koyu13**, 19 geniş
  açık18; bütün ham boyutlar `screenshot-dimensions.json` içindedir.

## Bağımsız disk kabulü ve sınır

`verify.py` uygulamanın komut/geometri hesabını kullanmadan beş gerçek kaydı
beklenen tam modele eşitler. Bütün UUID sırası, 14 nesne, katman ve ayarlar
karşılaştırılır. Yalnız gerçek Ad işleminin kaynak/iki tag/table farkına izin
verilir; undo/iptal/yeniden açma ilk modele tam eşittir. Resim/günlük/paket/son
kaynak SHA256'ları ve iki görünüm profili de denetlenir. `independent-final`
geçti. Önceki 697 interaction/303 Python bu dilimin yeni çalışma sayısı değildir.

Bu dar düzeltme genel docking tasarımını, yerleşim kalıcılığını veya bütün
tema/font/pencere/araç matrisini tamamlamaz. Model/ölçek/render ve anlamsal
hesaplar değişmedi; diğer mimari geliştirmeler plan içinde açık kalır.
