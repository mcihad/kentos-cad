# ADR 9309: Form tasarımcısı, harita servisi ve veritabanı bağlantısı pencereleri sadeleşti

- **Durum:** kabul edildi (2026-11-25).
- **İstek** (sahip, 25 Kasım): “Form tasarım dialog ekranı ve web servis ekleme dialog dış kaynak ekleme formlarının
  tasarımları çok kötü, özellikle form tasarımı o kadar karmaşık ki anlatamam.”
- **Sözleşmeler:** komut, kayıt biçimi ve ayar değişmedi. Pencerelerin düzeni ve sözleri değişti.
  - Tasarımcının iki pencere olayı eklendi: `FormSettings`, `InspectorSection`.
  - İzlerin bastığı düğme sözlerine “Form ayarları” ve “İlişkili kayıtlar” eklendi; eski “İlişkili” de geçer.

## §1 Bulgu

Sahibin ayarlarıyla (açık tema, 14 px Plus Jakarta Sans, kenar yumuşatma kapalı) gerçek pencerede ve resim sınamasında
çekildi.

### Formlar (form tasarımcısı)

Bütün yetenekler aynı anda ekrandaydı:

- **Sol sütun iki işi birden yapıyordu:** hangi formun düzenleneceği (liste) ve forma ne konacağı (palet, yeni alan).
- **Ekleme araçları:** dokuz düğmelik bir sıra; altısı yalnız simgeli, ne koyduklarını söyleyen yoktu.
- **Formun kendi ayarları** (ad, kapsam, Adım adım, Gizlenenleri sil, Kuralları uygula) sütunların altında, tam
  genişlikte iki satırlık bir şeritti.
- **Alt çubuk:** altı ayrı eylem düğmesiyle birlikte dokuz düğme.
- **Alanın özellikleri** tek sütunda yirmi beşe yakın denetimdi.
- **Formu olmayan projede** boş bir liste kutusu, ortada bir cümle ve basılacak bir şeyi olmayan dört sekme vardı.
- **Başlık ve satırlarda** tasarım sisteminin yasakladığı “A · B” kalıbı vardı (DESIGN.md §13).

### Harita servisi

- **Pencere her türde başka boydaydı:** tür değişince küçülüp yeniden ortalanıyordu; tıklanacak öğe imlecin altından
  kayıyordu. Sekmeli pencere sabit boyludur (DESIGN.md §7.9).
- **Hazır altlıklar:** kaydırma alanı içeriği kadar dardı, çubuğu ortada duruyor, sağda ~170 px boş kalıyordu. Kartlar
  yalnız adı gösteriyordu; katalogdaki açıklama (ör. “eş yükselti eğrili, gölgeli topoğrafik harita; en çok 17. kat”)
  hiçbir yerde görünmüyordu.
- **XYZ sayfası** WMS'in yönlendirmesini taşıyordu: “servisin katmanları listelenir.”
- **“Bağlantı” etiketi** aslında kimliği (anahtar, kullanıcı) soruyordu.
- **“Saydamlık %100”**, katmanın tam göründüğü öntanımlı değerdi. Değer opaklıktı; Türkçede tam saydam, yani görünmez
  demek. Servis katmanının sağ tık menüsü de aynıydı. Raster ve nokta bulutu pencereleri “Saydamlık”ı doğru anlamıyla
  kullanıyor.

### Yeni PostgreSQL bağlantısı

- **Uzunluk:** 1440 × 900 ekranda pencere 48–850 piksel arasını kaplıyordu. Beş alanın altında açıklama, dört bölüm
  başlığı vardı.
- **Yeni bağlantıda gerekmeyenler en öndeydi:** SSL ve iki listeleme seçeneği.
- **“Bağlantıyı dene”** formun dibinde, iki satırlık bir açıklamayla duruyordu.

## §2 Karar

### §2.1 Formlar

- **Sol sütun:** formların listesi (form sayısı kadar uzun, en çok dört satır), “Yeni form”, “Forma ekle” başlığı
  altında palet ve yeni alan. Satırda “Bina katmanı, 1. yayın”.
- **Orta sütun, tek araç sırası:**
  - “Ekle ▾” menüsü, iki başlık altında: Düzen (sekme, grup, bölücü, not) ve Kayıt (ilişkili kayıtlar, başvuru, ekler,
    konum, eylem). Her satırda simge ve bir satırlık açıklama.
  - Seçili öğenin işleri: yukarı, aşağı, kopyala, yapıştır, sil.
  - “Form ayarları”, geri al, yinele. Sıra sığmazsa “Form ayarları” simgesine iner (`fallback`).
- **Sağ sütun:**
  - Öğe seçili değilken formun ayarları, her biri ne yaptığını söyleyen bir cümleyle: ad, kapsam (örnek açıklamada),
    Adım adım, Gizlenenleri sil, Kuralları uygula. Altında formun gerektirdikleri, çeviriler, yorumlar.
  - Alanın özellikleri beş katlanır bölümde (`Form::foldable_section`):
    - Genel (açık): etiket, öğe türü, değer listesi, yardım, yer tutucu.
    - Değer: varsayılan, varsayılan ifade, hesap, değiştirilebilir, son değeri kullan, öneri, tekil.
    - Kurallar: zorunlu, salt okunur, görünürlük, zorunluluk, salt okunurluk, seçenek süzgeci, alt türler.
    - Denetimler.
    - Diğer: örneklerde gizle, yansıma, bölüm kitaplığı, yorumlar, bağlar.

    Katlı bölümün başlığı içinde kaç ayarın dolu olduğunu söyler (“Kurallar (2)”). Hatası olan bölüm kendiliğinden
    açılır. Onay kutularının sözleri ne yaptıklarını da söyler (“Tekil: aynı değer iki kayıtta olamaz”).
- **Alt çubuk:** “Form ▾” menüsü (kayıtları denetle, hesapları yenile, içe aktar, dışa aktar, yayından kaldır, formu
  sil; silen satır kırmızı), formun durumunu söyleyen cümle, Vazgeç, Yayınla, Kaydet. Başlık “Formlar”, kaydedilmemişse
  yanında nokta.
- **Bölmeler 390 yerine 460 yüksek** (12 px yazıya göre): alttaki iki satır gitti, pencerenin boyu aynı kaldı.
- **Formu olmayan proje:** ortada tek bir davet. Formun ne olduğunu söyleyen bir paragraf ve “Yeni form”. Basılınca ad
  ve katman alanları etiketleriyle aynı yerde açılır.

### §2.2 Harita servisi

- **Sabit boy:** liste ve sayfa her türde aynı boyda; sayfa sığmazsa kendi içinde kayar.
- **Hazır altlıklar:** grup başlıkları altında iki sütunlu kartlar. Her kartta simge, ad, açıklamanın ilk satırı;
  tamamı ipucunda.
- **Sözler:**
  - “Bağlantı” yerine “Kimlik”;
  - XYZ'de “Şablonu yazıp Bağlan'a basın: bir karo denenir, karo ayarları açılır.”;
  - “Saydamlık” görünmeyen pay: tam görünen katman %0. Saklanan değer değişmedi (opaklık); sağ tık menüsü de aynı.
- “ArcGIS REST” servis türünün adı olarak kaldı; başka uygulama adı denetiminin izinli listesinde (`other_names_tests`).

### §2.3 Yeni PostgreSQL bağlantısı

- Sunucu ve bağlantı noktası tek satırda; yer tutucunun zaten söylediği iki açıklama kalktı.
- Parola notu tek cümle.
- SSL ve listeleme seçenekleri katlı bir “Gelişmiş” bölümünde. Yeni bağlantıda kapalı; kayıtlı bağlantı bunlardan
  birini öntanımlı dışında tutuyorsa açık gelir. Açıp kapamak deneme sonucunu silmez.
- “Bağlantıyı dene” alt çubuğun solunda, sonucu yanında (DESIGN.md §7.9). Ne yaptığı ipucunda.

### §2.4 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| CBS | var | Formlar, harita servisi ve veritabanı bağlantısı onun pencereleri; resimler CBS projesinde çekildi. |
| Harita, Yol, Mimari | pencereyi açan her modülde aynı | Değişen yalnız pencerenin düzeni; açılış yolları değişmedi. |
| MCP, Python, eklenti, yapay zekâ ajanı | değişmedi | Pencereyi görmezler; aynı komutları (`cad.forms.*`, servis ve bağlantı komutları) kullanırlar. |

## §3 Kanıt

**Resimler:**

- Form tasarımcısı `form_fit_screens`'te (sahibin görünümü, 1100 × 650 / 1440 × 900 / 1920 × 1080). Yeni sahne:
  `tasarim-form-ayarlari`.
- Öbür ikisi gerçek pencerede, sahibin ayarlarının kopyasıyla (`scripts/usage/real/session.sh`): hazır altlıklar, XYZ,
  WMS, yeni bağlantı, boş Formlar, Yeni form, tasarımcı, “Ekle” menüsü.

**Ölçüler** (1440 × 900):

- Bağlantı penceresi 802 piksel yerine 504 piksel yüksek.
- Servis penceresi XYZ ve WMS'te yerinde kaldı; WMS tıklaması bu kez doğru öğeye düştü.
- Tasarımcının alt çubuğunda dokuz düğme yerine üç düğme ve bir menü.
- Alan özellikleri kaydırmasız (Genel açık, dört bölüm katlı).

**Sınamalar:** masaüstünde 200 sınama geçti: formlar, servisler, bağlantılar, kaynaklar, PostGIS, pencere boyları ve
başka uygulama adı denetimi. Ayrıca `kentos-cad`'in 37 ve `kentos-desktop-cbs`'nin 57 sınaması geçti.

## Uygulama

- `apps/desktop/src/gis/forms/designer.rs`: `FormSettings`, `InspectorSection`, bölüm adları, yeni düğme sözleri.
- `apps/desktop/src/gis/forms/designer/step.rs`: iki olayın işlenmesi.
- `apps/desktop/src/gis/forms/designer_view.rs`: başlık, liste, “Ekle ▾”, tek araç sırası, “Form ▾”, boş proje daveti.
- `apps/desktop/src/gis/forms/designer_view/inspector.rs`: alanın beş bölümü.
- `apps/desktop/src/gis/forms/designer_view/palettes.rs`: formun ayarları.
- `apps/desktop/src/shell/services/window.rs`: sabit boy, kartlar, sözler. `app.rs`: sağ tık menüsünün saydamlığı.
- `apps/desktop/src/gis/db_connections.rs`: tek satır sunucu, “Gelişmiş”, alt çubukta deneme.
- `apps/desktop/src/tests/form_fit_screens.rs`: Form ayarları sahnesi.
