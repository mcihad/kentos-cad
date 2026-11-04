# 0927 — KentOS Yol: platform ve saha tesviyesi

Tarih: 2026-11-03. plan/road.md Y14 (ROAD-GRADE-01, 02, 03, 06, 07, 08, 09, 10). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** tesviye bir ayak izinden (feature line, parsel çizgisi ya da başka tesviyenin günışığı çizgisi) bir
  hedefe yapılır: yüzey, yükseklik, göreli yükseklik ya da uzaklık; tesviye ölçütleri (ör. yüzeye 4:1 şev) bölgelere
  göre değişebilir. Tesviye grubu tek bir grup yüzeyi kurar, hacim araçları kazı ve dolguyu söyler, grup kazı ve
  dolgu dengelensin diye yükseltilip alçaltılır; Auto-Balance ayak izini hedef hacme yaklaşana dek oynatır
  ([About Grading](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-3E75D600-B442-4C78-8DCC-C649E1ED1F87.htm),
  [To Create Grading From a Footprint](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-30D7E7C2-4361-47A2-9F28-F5915EB713AB.htm),
  [Exercise 3: Creating a Grading](https://help.autodesk.com/cloudhelp/2016/ENU/Civil3D-Tutorials/files/GUID-CC753396-5BFA-4FEE-9122-968B3ED5F9B5.htm),
  3 Kasım 2026'da okundu). Forumdaki bir kullanıcının gözlemi: dengeleme istenen kazıyı tutturur ama göletin taşma
  kotunu bilmez; KentOS dengelemeyi yalnız öneri olarak verir.
- **OpenRoads Designer:** arazi sekmesinin Analysis grubunda Create Cut Fill Volumes iki yüzey (mevcut arazi ve tasarım)
  arasındaki kazı ve dolguyu hacim öznitelikli 3B katı olarak verir, kabarma ve sıkışma çarpanlarıyla; Analyze Trace
  Slope yüzeyde bir eğimle ya da en dik yoldan iz sürer; pad ve otopark tesviyesi kurallı (rule-based) tasarımla ve
  elemanlardan önerilen arazi modeliyle yapılır
  ([OpenRoads Designer CONNECT Edition](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20CONNECT-v12/en/GUID-274B514F-6C32-4E34-8837-B560AB116928.html),
  [FHWA ORD kılavuzu, Proposed Model](https://highways.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13271/ord-manual22-proposed-model_1.pdf)).
- **Softree RoadEng (karşılaştırma):** Grading platform özelliği kullanıcının kazı ve dolgu açılarıyla bir hedef
  yüzeye izdüşürür; şev çizgileri özelliğin her noktasında ve aralarında bir aralıkla kurulur, günışığı çizgisi şevlerin
  hedef yüzeyle buluştuğu yerdedir; TIN kurulup hacim hesaplanır, dengelemek için pad 1 m adımlarla oynatılıp tesviye
  yinelenir ([Grading Platform](https://support.softree.com/knowledge-base/terrain-module/engineering-functions/grading-platform)).
- **Netcad:** Netpro karayolu, havalimanı, gölet ve kazı işlerinde tesviyeyi kapsar
  ([Netpro](https://en.netcad.com/en/products/netpro)); kardeş modülü Excanet hazır şev ve platform şablonları ve
  toprak hacmi karşılaştırması sunar ([Excanet](https://en.netcad.com/en/products/excanet)); Netcad 8 Netpro ders
  kitabı şevli kotlu planı ve kübajı anlatır
  ([Mersin Üniversitesi](https://publish.mersin.edu.tr/index.php/geoengbooks/article/download/722/620)). Resmî kılavuz
  çevrimiçi değil.
- **Matematik:** platform bir düzlemdir: z = z₀ + g·((x − x₀) cos θ + (y − y₀) sin θ) + öteleme. Şev çizgisi bir
  doğrultuda yükselir ya da alçalır (kazıda 1/k₁, dolguda 1/k₂ eğimle; k yatay bölü düşey). Arazi bir TIN'dir: bir
  doğru boyunca üçgen kenarlarını kestiği yerler arasında doğrusaldır ([`Surface::section`]); şev de doğrusal, öyleyse
  aradaki fark her parçada doğrusaldır ve ilk sıfırı kesin bulunur (enkesitin `meet` kuralı, ADR 0903). Dışbükey
  köşede şev bir koni yüzeyidir; çokgen yelpaze onu en çok `fan` derece aralıklı doğrularla yaklaştırır (Civil 3D'nin
  köşe izdüşümü); içbükey köşede iki kenarın dik doğruları kesişeceğinden tek doğru açıortaydan. Hacim iki TIN
  arasındadır (ADR 0906'nın `between_surfaces`'i). Kazı ile dolgu farkı öteleme arttıkça azalır (platform yükseldikçe
  kazı küçülür, dolgu büyür), öyleyse işaret değiştiren bir aralıkta ikiye bölme kökü bulur; işaret değişmiyorsa denge
  yoktur ve söylenir (en iyi tahmin uydurulmaz, ROAD-GRADE-09).

## Karar

1. **Çekirdek `kentos_road::grading`.** `Platform`: sınırın köşeleri (kapalı, her iki yönde de olur; kendini kesen,
   aynı yerde iki köşe ve alansız sınır reddedilir), düzlemin taban noktası, eğimi (%) ve yönü (derece, +x'ten saat
   yönü tersine), kazı ve dolgu şevi (yatay/düşey, sıfırdan büyük), şev çizgisi aralığı, köşe yelpazesinin en geniş
   açısı (0–90°) ve şevin en uzun erimi. `grade` her köşeden ve kenar boyunca aralıkla şev çizgilerini kurar, her
   birinin yanını (kazı, dolgu, açık) ve günışığını bulur; açık çizgi yoksa platformun halkası, günışığı halkası ve her
   şev çizgisi kırık hat, günışığı halkası sınır olan tasarım yüzeyini kurar ve araziye karşı kazı ve dolguyu,
   alanlarını verir. `balance` verilen aralıkta kazı ile dolgu toleransla eşitlenene dek ötelemeyi ikiye böler (en çok
   60 adım; yakınsadı mı, adım sayısı ve kalan fark söylenir).
2. **Proje ayarı ve komutlar (Y14.2).** Yol ayarlarının `platforms[]`'ı (kimlik, ad, sürüm, köşeler, taban, eğim, yön,
   şevler, aralık, yelpaze, erim, arazi yüzeyi); `road.platforms.edit` (set, remove; tek geri alma adımı),
   `road.platforms.query` (şev çizgileri, günışığı halkası, açıklar, alanlar, kazı ve dolgu; isteğe bağlı denge
   aralığıyla önerilen öteleme, yazılmaz) ve `road.platforms.draw` (platformun sınırı, günışığı çizgisi, şev
   çizgileri ve kazı ile dolgu yazıları; yol çizimlerinin kaydına girer, ADR 0911 §1). Şevin değeri kullanıcınındır:
   KentOS zemin türüne göre öntanımlı şev uydurmaz (ROAD-GRADE-02; geoteknik profil sahibin kaynağını bekler).
3. **Masaüstü (Y14.3).** Arazi sekmesinde Platform penceresi: köşeler tablosu ya da çizimden kapalı çizgi, taban kotu,
   eğim ve yönü, şevler, yüzey; planda platform, kazı (kırmızı) ve dolgu (yeşil) şevleri, günışığı çizgisi ve açık
   çizgiler; hacimler ve Dengele (önerilen kot, onaylanınca yazılır).

Açık: basamaklı şev (bench) ve hendek (ROAD-GRADE-03'ün kalanı), korunan ya da kazı yapılmayacak bölgeler
(ROAD-GRADE-06), platform ile yol yüzeyinin birleşmesi (ROAD-GRADE-04), drenaj havzası ve göllenme
(ROAD-GRADE-05), maliyet ve arazi kullanımı kısıtlı eniyileme (ROAD-GRADE-08'in kalanı).

## Aşamalar

Y14.1 (yapıldı, çekirdek): §1. `crates/shared/road/src/grading.rs`, testleri `tests/all/grading.rs`: düz araziye 2 m
dolgu ve kazı kapalı biçimle (kutu, dört yan prizma, altı dilimli dört köşe piramidi), köşeler ters sırada aynı,
eğimli arazide doğu kazı ve batı dolgunun günışığı elle çözümle (5/9 m, 1,25 m), eğimli platformun düzlemi, arazinin
dışı ve erimin ötesi açık, sınırın ve değerlerin retleri, eğimli arazide ve düz arazide (eşit şevlerle öteleme 0)
denge, işaret değiştirmeyen aralık.

Y14.2 (yapıldı, proje ayarı ve komutlar): §2. `RoadSettings.platforms` (`PlatformDef`, `.kcad` şema 162'de; en çok
1000; yüzeyi projede olmalı, `broken/road-platform-unknown-surface.kcad`), geri alma adımı `Op::RoadPlatforms`;
`road.platforms.edit` 28, `.query` 24, `.draw` 21 ortak durum (`scripts/fixtures/road_platform_cases.py`: arazi iki
düzlem; günışığı kapalı biçimde, u = (zₜ − z₀)/(m − g); tek yanlı platformda iki komşu şev çizgisi arasındaki her
parça düzlem olduğundan hacim parçanın alanı ile ağırlık merkezindeki kot farkının toplamı, düzlem olmayan parçada (içbükey
köşe) ve kazı-dolgu karışık platformda hacim `$any`; denge ikiye bölmenin aynısı; KentOS kodu yok). Platform çizimi yol
çizimlerinin kaydına girer (kaynağı platform ve onun yüzeyi), proje ağacında yüzeyin altında, kurarak denetlemede açık
şev çizgisi “çözülmeyen”; Yeniden üret onu da çizer. Durumların bulduğu: üçgen kenarı boyunca giden (yönü yuvarlamayla
kenardan sapan) bir doğruda `Surface::section` kesişimi sıfıra yakın paydadan buluyor ve başlangıcın kotunu yanlış
okuyordu (96,5 yerine 100); yaklaşık paralel kenar artık paralel sayılır (sinüs 1e-12 altı), aynı doğrudaysa yalnız
uçları yer olur. Boykesit, enkesit ve koridorun durumları bu düzeltmeyle değişmedi. Python `kentos.cad.road.platforms`,
MCP.

Y14.3 (yapıldı, masaüstü): §3. Arazi sekmesinde Tesviye paneli, `platform.editor` (Platform…,
`apps/desktop/src/platform_window.rs`): platform listesi, ad, arazi yüzeyi, köşeler “Y,X; Y,X; …” olarak yazılır ya da
seçili kapalı çoklu çizgiden ya da alandan alınır (taban noktası, yazılmamışsa ilk köşe); taban Y, X ve kotu, eğim,
doğrultu, yarma ve dolgu şevi, aralık, yelpaze, erim. Yazıldıkça `road.platforms.edit`'in okuma kopyasındaki
önizlemesi: planda platform, yarma (kırmızı), dolgu (yeşil) ve açık (turuncu nokta) şev çizgileri, günışığı halkası,
köşe kotları; platform ve taban alanı, kazı, dolgu, fark, çizgilerin yanları. Dengele yazılan platformu okuma kopyasına
yazıp `road.platforms.query`'nin dengesini sorar (dengeli taban kotu, kalan fark, adım, yakınsadı mı); Uygula yalnız
taban kotu alanına yazar, Kaydet tek geri alma adımıdır (ROAD-GRADE-08: öneri kullanıcının onayıyla). Planda çiz
`road.platforms.draw`. Proje ağacında platform satırı bu pencereyi açar. Testler `platform_window_tests.rs` (önizleme,
dolgu şevi değişince dolgu artar, denge ve uygula, kaydet, çiz ve geri al, yeni platform, seçimden köşeler); resimler
`.run/gui-evidence/yol-y14-platform-*` (açık geniş, koyu dar). İlk resimde dar pencerede Uygula kesiliyor ve Uygula'dan
sonra denge sözü kayboluyordu: denge düğmeleri kendi satırında, söz uygulamadan sonra kalır.
