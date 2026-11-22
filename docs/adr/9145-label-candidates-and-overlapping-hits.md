# ADR 9145: Etiket denetimi adayları ve nedenleri söyler; Bilgi örtüşen nesneleri sayar

- **Durum:** kabul edildi (2026-11-22); adım VY-44. Plan: [plan/veri.md](../../plan/veri.md) VY-44.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  13. maddesi (P1):
  - yol ekseninin tamponla örtüştüğü yerde Bilgi kartı “Çoklu çizgi · durum planlı” dedi; nesnenin hangi katmana ya da
    kayda ait olduğu görünmedi;
  - Etiketler penceresinde şablona `{durum}` yazılınca Denetle “0 etiket yerleşti, 0 yerleşemedi, 0 gizli” dedi;
    nedeni söylenmedi;
  - istenen: kartta katman ve anahtar, örtüşmede kaç kayıt bulunduğu ve öbürüne geçiş, şablon için alan seçici, ilk
    satırdan örnek metin, ölçek/görünürlük/boş değer/en küçük nesne/çakışmanın ayrı nedenleri.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Bilgi kartının başlığı katmanın kart şablonundan gelir (ADR 0225 §5). Şablon yoksa nesnenin türüdür. Bağlı tablo
  satırının kartı tablonun yerini ve anahtarını söylüyordu; çizimin öbür nesnelerinin kartı katmanı söylemiyordu.
- Bilgi aracı tıklamanın ilk adayını gösteriyordu (ADR 0187 §1); altta kalan nesneler sayılmıyordu.
- Etiket şablonu `{Ad}` biçimindeki alanları nesnenin değerleriyle doldurur (ADR 9110 §5). Katmanın alanı olmayan bir ad
  yazıldığı gibi kalır; yanlış yazılan alan kaybolmasın diye böyledir. Bildirilmiş ama boş alan boş metin verir; boş
  metinli nesneye etiket konmaz.
- Denetle (`cad.labels.report`, ADR 0533 §6) motorun yerleştirdiğini, yerleştiremediğini ve gizlediğini sayıyordu.
  Motora hiç ulaşmayan adayları ve nedenlerini saymıyordu: gizli katman, ölçek aralığı, en küçük nesne, boş metin.
- **Araştırma.** Başvuru alınan CBS uygulamasının belgeleri (22 Kasım 2026'da okundu):
  - nesne belirleme sonucu ağaçtır: ilk düzey katmanın adı, altında belirlenen nesneler. Kip, sonucun hangi katmanlardan
    geleceğini seçer: etkin katman, yukarıdan ilk isabet, yukarıdan hepsi;
  - ifade oluşturucusu alan listesini sunar; çıktı önizlemesi varsayılan olarak katmanın ilk nesnesinde hesaplanır;
  - etiketin görünmesini ayrı ayarlar belirler: İşleme sekmesinde ölçeğe ve piksel boyuna bağlı görünürlük, nesne
    seçeneklerinde “şundan küçük nesneleri etiketleme”.

  Kaynaklar:
  - [nesne belirleme](https://documentation.qgis.org/3.44/en/docs/user_manual/introduction/general_tools.html#identify);
  - [ifadeler](https://documentation.qgis.org/3.44/en/docs/user_manual/expressions/expression.html);
  - [etiket ayarları](https://documentation.qgis.org/3.44/en/docs/user_manual/style_library/label_settings.html).

## §2 Karar

1. **Kartın altında katman ve kimlik.**
   - Çizim nesnesinin kartı başlığın altında “Bina katmanı · kimlik …0000002a” der: kalıcı kimliğin son sekiz hanesi.
     İlk haneler nesnenin yapıldığı zamandır; birlikte yapılan nesnelerde aynıdır.
   - Bağlı tablonun nesnesi tablonun satırını söylemeyi sürdürür.
   - Veri katmanı satırının kartı katmanın adını da söyler: “parseller katmanı · … › parseller · fid 22”.
   - Başlık değişmedi: izler kartı başlığıyla tanır.
2. **Örtüşen nesneler.**
   - Bilgi'nin tıklaması altındaki bütün nesneleri de verir (`ViewChange::InfoHits`): ilki kartın nesnesidir, sonra
     yakalama sırasıyla öbürleri.
   - Kart “Bu noktada 3 nesne · 1/3” der; Sonraki bir sonrakine geçer, sondan ilkine döner, çizimden silinmişi atlar.
     Araç halkayı geçilen nesneye taşır (`Tool::info_shown`).
   - Boşa tıklama ve kapatma bunları unutur.
3. **Etiket penceresinin örneği.**
   - Şablon satırının yanında “Alan ekle”: önce katmanın bildirilmiş alanları, sonra ilk 500 nesnenin öznitelikleri.
     Seçilen alan şablonun sonuna `{ad}` olarak eklenir; izler alanın adıyla erişir.
   - Pencerenin özeti yazıldıkça katmanın ilk nesnelerinin aldığı metni gösterir: “Örnek metin (ilk nesne): “101/7”.”
     İlk nesnelerin metni boşsa bunu söyler.
   - Katmanın alanı olmayan şablon adı uyarıdır: “Şablondaki {durum} katmanın alanı değil: etikette yazıldığı gibi çıkar.
     Katmanın alanları: Ada, Parsel, Tapu alanı.”
4. **Denetle'nin adayları ve nedenleri.** Motorun sayılarının altında:
   - “Adaylar: görünümde N nesne, etiket metni olan M, metni boş K; örnek metin “…”.”
   - Nedenler ayrı satırlardır:
     - etiketleme Yok;
     - katmanın nesnesi yok;
     - katman gizli;
     - görünür alanda nesne yok;
     - şablonun alanı her nesnede boş, ya da metin boş;
     - ölçek aralığı dışında: görünümün ve aralığın uçları 1:N olarak;
     - en küçük nesne sınırından küçük;
     - çakışma yüzünden yer bulamayan.
   - Hiçbir şey sayılmadıysa motorun satırı başarı işareti taşımaz.
   - Ölçek ve boyut kuralları motorunkidir: aralık `scale::visible_at` ile, en küçük nesne sınır kutusunun kısa kenarının
     ekrandaki boyuyla. Metinler motorun kodundan gelir (`label_texts::object_texts`).
   - Paylaşılan rapor sözleşmesi değişmedi; nedenler masaüstünde hesaplanır.

## §3 Kanıt

- **Masaüstü, etiketler** (`gis/labelling/no_label_tests.rs`):
  - parselin `{durum}` şablonu: örnek “{durum}”, alanın katmanda olmadığı ve katmanın alanları; Denetle'de bir aday.
  - Bildirilmiş boş `durum`: 0 yerleşti, “şablondaki {durum} … hepsinde boş”.
  - `{Ada}/{Parsel}`: örnek “101/7”.
  - En uzak 1:1: “1 aday bu ölçekte gösterilmez … en uzak 1:1, en yakın sınırsız”.
  - En küçük nesne 100000 px: ayrı satır, ölçek satırı yok.
  - Görünüm uzakta: “hiçbiri görünür alanda değil”.
  - Alan ekle: `{Ada} {Parsel}`, örnek “101 7”; izden alan adıyla.
- **Masaüstü, Bilgi** (`cad/info_card_tests.rs`):
  - kartın kaynağı “Parsel katmanı · kimlik …”;
  - binanın dairesi ile yayının arasına tıklama: en az iki bina nesnesi, kart ilkinde;
  - Sonraki ikinciye geçiyor, turun sonunda ilke dönüyor;
  - boşa tıklama kartı ve sayıyı kaldırıyor.
- Veri katmanı satır kartı sınaması katmanın adını da bekliyor (`gis/data_layers_tests.rs`).
- Etiket, Bilgi, veri katmanı, iz, şerit, modül ve geçmiş sınamaları (87) geçiyor.
  - `traces::layer_keys_tests` toplu koşuda bir kez kaydedip açmada takıldı; tek başına geçiyor. Bilinen, bu işten
    bağımsız bir yarış.
- **Resimler** (`.run/gui-evidence/kullanim-9145/`, Git'e girmez):
  - Alan ekle ve örnek metin;
  - Denetle'nin adayları ve boş alan nedeni;
  - Bilgi'nin örtüşen nesneleri (“Bu noktada 3 nesne · 1/3”, Sonraki).

## §4 Kalan

- **Görüntüleme alanı.** Kart başlığında katmanın görüntüleme alanının değeri, şablonsuz katmanda da.
- **Veri katmanı satırları.** Çizimin nesnesi ile veri katmanının satırı aynı tıklamada birlikte sayılmıyor: veri
  katmanı yalnız çizimde nesne yokken soruluyor.
- **Etiket önizlemesi.** Örnek metnin pencerede etiketin kendi biçimiyle çizilmesi; boş ve uzun ad örnekleri.
- **Veri katmanında etiket.** Kaynakta duran satırların etiketi ve denetimi.
- **Uçtan uca.** Planlı → kontrol değişiminde etiketin, tablonun ve haritanın aynı değeri gösterdiği sınama.
- Web (TODOS.md §0.2).
