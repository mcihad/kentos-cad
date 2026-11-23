# ADR 9194: Formların rolleri bulutta üç hesapla kabul edildi; proje sayfası rolü ve kayıtları gösterir

- **Durum:** kabul edildi (2026-11-23); adım VY-93. Plan: [plan/veri.md](../../plan/veri.md) VY-93.
- **İstek:** sahibin sözü (23 Kasım 2026): “Madde 25/29 yerelde deneme hesaplarıyla yap, web arayüzünde de görüntüler
  gönder”. Değerlendirmenin 25. maddesinin kalanı ([ADR 9125](9125-records-written-by-role-not-by-design-right.md) §4):
  “iki gerçek hesapla bulutta kabul”. 29. maddenin kalanı ([ADR 9150](9150-fields-the-role-does-not-read.md),
  [ADR 9188](9188-null-empty-text-and-codes-told-apart.md)): “bulutta üç hesapla kabul”.
- **Web:** buradaki web arayüzü bulut portalıdır (`apps/portal`, [ADR 9000](9000-cloud-portal.md)). Duran web
  uygulaması ([ADR 0228](0228-web-pause-desktop-and-cloud-first.md)) için TODOS.md §0.2.
- **Komut:** yeni komut yok. Sunucunun `GET …/features` yanıtı (`FeaturePage`) ilk sayfada `layers` taşır. Sözleşme,
  TypeScript türleri ve `docs/api/openapi.json` yeniden üretildi. **`.kcad`:** değişmedi.

## §1 Bağlam

- **25. madde.** Veri girişi rolünün yayımlanmış formla yazması ADR 9125'te masaüstünde düzeltildi. Kalan, gerçek
  sunucuda gerçek hesaplarla kabuldü: sunucu sınamaları o gün veritabanı bulamamıştı.
- **29. madde.** Rolün okumadığı alan tabloda, kartta ve dışa aktarımda boş sütun değil; adıyla söyleniyor (ADR 9150).
  NULL ile boş metin ayrı (ADR 9188). Kalan, bulutta üç hesapla kabuldü.
- **Proje sayfası.** Bulut portalının proje sayfası çizimi, ekleri ve künyeyi gösteriyordu; projedeki rolü ve kayıtları
  göstermiyordu.
  - Sunucu kayıtları rolün okuduğu gibi kesiyordu (`form_guard::projected`). Ama tarayıcıya katmanların adını ve hangi
    alanların okunmadığını söylemiyordu.
  - Katman ağacı olmayan bir istemci, gönderilmeyen alanı boş sanabilirdi. 29. maddenin söylediği kusur buydu.
- **Araştırma.**
  - OGC API – Features, Part 5: Schemas ([23-058r2](https://docs.ogc.org/is/23-058r2/23-058r2.html)):
    - Bir koleksiyonun şeması `/collections/{collectionId}/schema`'da yayımlanır.
    - İstemcinin alabileceği özellikler (returnables) ve gönderebileceği özellikler (receivables) ayrı sınıflardır.
  - Barındırılan bir katmanın görünümünde alan gizlemek
    ([Esri destek makalesi 000026741](https://support.esri.com/en-us/knowledge-base/how-to-hide-fields-in-a-hosted-feature-layer-in-arcgis-000026741)):
    görünümün kullanıcısı gizlenen alanı hiç görmez, adını da bilmez.
  - **KentOS kararı:**
    - Şema ayrı bir uçta değil; kayıtların ilk sayfasında, çağıranın okuduğu biçimde gelir. Sayfadaki kayıtlar ve
      katmanların sözü tek kararla verilir.
    - Gizlenen alan adıyla söylenir (29. madde, ADR 9150). Veri girişinin çiziminde katmanın alanları zaten durur, ve
      saha çalışanı okumadığı alanı “kayıtta not yok” diye anlamamalıdır.

## §2 Karar

1. **Kayıtların ilk sayfası katmanları taşır** (`FeaturePage.layers`, `kentos_contracts::FeatureLayer`).
   - İmleçsiz ve `ids`'siz istenen ilk sayfa, alanı olan her katmanı ağacın sırasıyla verir:
     - **okunan alanlar** (`fields`): katmanın sırasıyla ve tanımlarıyla (`LayerField`: takma ad, tür, değer listesi);
     - **rolün okumadığı alanlar** (`withheld`): adlarıyla; Veri girişinin canlı formlarının göstermedikleri, başkası
       için boş;
     - **kapsam** (`narrowed`): kayıtlar daralıyorsa yazılır (Veri girişinin form kapsamı, ya da bütün formları geri
       çekilmiş katman).
   - Kural, kayıtları kesen ortak çözümleyicidir (`kentos_forms::reach`; sunucuda `form_guard::layers_read`).
   - Sonraki sayfalar ve `ids` ile okuma katman taşımaz.
2. **Bulut portalının proje sayfası** (`apps/portal/src/pages/project-records.ts`, `lib/records.ts`).
   - **Erişiminiz** (Künye'de): rol, nereden geldiği (projenin sahibi, paylaşımla, kurumun kuralıyla), bir satırda ne
     yapabildiği. Sözler masaüstünün paylaşım penceresinin sözleridir.
   - **Kayıtlar** (veritabanı projesinde): her katman için okunan alanların tablosu.
     - Kodlu değer etiketiyle, evet/hayır sözle yazılır.
     - Hiç yazılmamış değer soluk “NULL”, boş metin boş.
     - Kayıtlar etiketlerine göre doğal sırayla dizilir (D-2, D-10'dan önce). Katman başına en çok 100 satır gösterilir;
       okunan ilk 500 kayıttır.
   - **Tablonun altı:**
     - Kapsam kayıtları daraltıyorsa: “Formun kapsamı bu katmanın yalnız bazı kayıtlarını size açıyor; öbürleri burada
       yok.”
     - Rolün okumadığı alanlar varsa, masaüstünün sözüyle: “Rolünüz bu alanları okumuyor, gösterilmiyor: Not, Ekip”.
   - **Reddedilen çizim** sunucunun gerekçesini yazar: “Çizim size kapalı · Veri girişi projenin bütün çizimini görmez;
     kayıtlarını formla okur.” Önceden “Çizim şu an gösterilemiyor” deniyordu. Veri girişine “Çizimi indir (SVG)”
     düğmesi gösterilmez.
3. **Masaüstünün Bilgi kartı kodlu değeri etiketiyle yazar** (`object_info::object_card`, `pending_rows::shown_value`).
   - Örnek: “kotu” değil “Kötü”, tablonun ve veri katmanının kartının yazdığı gibi.
   - Kabulde tablo ile kart aynı değeri ayrı yazıyordu.
4. **Kabul düzeni** (`apps/desktop/scripts/forms-roles-live.sh`). Betik sırayla:
   - **Veritabanı ve hesaplar:** bu derlemenin göçleriyle geçici veritabanı ve geliştirme hesapları kurulur
     (`apps/api/examples/e2e_database.rs`). Elif, kurumun yönetici olmayan düzenleyicisi olarak eklendi.
   - **Sunucu:** boş bir kapıda `kentosd` açılır.
   - **Masaüstü:** canlı sınama koşar (`shell/cloud/live_forms.rs`, `#[ignore]`). Sınama, `live_run.rs`'in modül içinde
     açılan koşucusunu kullanır (`sign_in_as`, `shot_to`).
   - **Portal:** aynı sunucuya bağlanır; resim betiği koşar (`apps/portal/scripts/forms-shots.mjs`).
   - **Kapanış:** sunucu ve portal durur, veritabanı silinir.
   - **Parola:** betikte üretilir, `.run/cloud-forms/password`'ta (600) durur, yazdırılmaz.

## §3 Kanıt

- **Kurulum.** Gerçek HTTP, gerçek sunucu, geçici veritabanı, üç hesap. Ayşe (proje yöneticisi) “Saha denetimleri”ni
  veritabanı projesi olarak yükler:
  - Durum zorunlu ve düzenlenebilir, Bolge salt okunur, Not ve Ekip formun dışında. Kapsam `Ekip = 'A'`; form
    yayımlanmış ve uygulanıyor.
  - Dört denetim:
    - D-1 (A ekibi): not “Teknik ekip notu”;
    - D-2 (B ekibi);
    - D-3 (A ekibi): notu hiç yazılmamış;
    - D-4 (B ekibi): notu boş metin.
  - Projeyi Mehmet'le Veri girişi, Elif'le Görüntüleyici olarak paylaşır.
- **Mehmet (Veri girişi)** projeyi Benimle paylaşılanlar'dan açar.
  - Rolü Veri girişi. Çiziminde yalnız D-1 ve D-3 var.
  - Not ve Ekip ne kayıtta, ne tabloda, ne Bilgi kartında, ne kopyasında; kopyanın sütunları Durum ve Bolge.
  - Kart “Rolünüz okumuyor: Not, Ekip” der, Durum'u “Kötü” yazar.
  - Kayıt yazar, form tasarlamaz.
  - Boş zorunlu Durum reddedilir: “Kayıt formun kurallarına uymuyor: “Durum” zorunlu; doldurun.” Durum “iyi” kaydedilir.
  - **Sunucu**, programın ne yaptığından bağımsız olarak:
    - salt okunur Bolge'ye yazmayı 403 ile reddeder: “Saha denetimi” formu: “Bolge” bu kayıtta salt okunur;
    - kapsam dışındaki D-2'ye yazmayı 404 ile reddeder;
    - Mehmet'e notu hiç göndermez.
- **Elif (Görüntüleyici)**
  - Kayıtlar onun için salt okunur; neden olarak rolü söylenir.
  - Notu okur. D-3'ün notu NULL, D-4'ünki boş metin; çiziminde ve tabloda ayrı görünür.
  - Formu açar ama değiştiremez.
- **Ayşe (sahip)**
  - Mehmet'in değeri ona gelir; not ve Bolge korunmuştur.
  - İkinci sürümü yayımlar: Durum'un etiketi “Saha durumu” olur.
  - Kendi formunda yeni etiket ve salt okunur Bolge görünür; Mehmet'in formunda da öyle.
  - Mehmet'in ikinci yazımında not yine korunur.
- **Portal.** Üç hesap giriş sayfasından klavyeyle girer. Her sayfanın söyledikleri betikte denetlenir.
  - **Mehmet:** Erişiminiz “Veri girişi · paylaşımla”.
    - Kayıtlar'da yalnız D-1 ve D-3; sütunlar Durum ve Bolge.
    - Kapsam ve okunmayan alanlar tablonun altında söylenir.
    - Çizim gerekçesiyle kapalıdır; SVG düğmesi yoktur.
  - **Elif ve Ayşe:** Erişiminiz “Görüntüleyici · paylaşımla” ve “Sahip · projenin sahibi”.
    - Dört kayıt, bütün alanlar.
    - D-3'ün notu soluk NULL, D-4'ünki boş.
    - Çizim “4 nesne çizildi” der.
- **Sunucu sınamaları** (geçici PostGIS'te) geçiyor:
  - `project_forms`: Veri girişinin ilk sayfası Not'u `withheld` sayar; düzenleyicininki bütün alanları verir; sonraki
    sayfa katman taşımaz.
  - `project_form_scope`: kapsam Veri girişinin katmanını `narrowed` yapar, düzenleyicininkini yapmaz.
  - OpenAPI denetimi geçiyor.
- **Portal sınamaları:** tür denetimi ve birim sınamaları (9) geçiyor. `lib/records.test.ts` NULL, boş metin, kod,
  evet/hayır ve çok değerli alanı; katmana göre ayırmayı ve etikete göre sırayı sınar.
- **Masaüstü sınamaları** geçiyor:
  - kart, rol alanları, NULL, ekler, veri katmanı ve GeoPackage durumu: 32;
  - ortak, form, bulut ve tablo: 418.
- **Resim.** `.run/gui-evidence/kullanim-9194` içinde 16 resim var: 13'ü masaüstünden, 3'ü portaldan. Resimler Git'e
  girmez.

## §4 Kalan

- **Portalın kayıt tablosu yalnız okur.** Sayfalama, süzgeç ve formla yazma yok. Veri katmanlarının (dış kaynak)
  satırları da yok.
- **Önizleme.** Sunucu noktaları çok küçük çiziyor: yarıçapı 1,6 kıl çizgisi, portalda bir iki piksel. Nokta
  etiketlerini de çizmiyor.
- **Gerçek pencere.** Gerçek fare ve klavyeyle deneme ([ADR 8000](8000-real-window-acceptance.md)) yapılmadı. Bugünkü
  kabul, uygulamayı süreç içinde süren canlı sınamadır.
- **Duran web uygulaması:** TODOS.md §0.2.
