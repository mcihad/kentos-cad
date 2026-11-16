# ADR 8067: Binanın 3B görünümü — IFC'nin gövdeleri ortak sahnede, tıkla ve planda seç; örtüşen mahal Python'da

- **Durum:** uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-06; uygulama sırasında B6. İlk
  kabulün dört ölçütü sağlandı; gerçek pencerede (işletim sisteminin faresiyle) kabul bu Mac'te koşturulmadı (§6, §8).
- **Bağlam:** ADR [0910](0910-preliminary-route-junction-and-3d.md) (yolun 3B görünümü: ortak sahne çekirdeği `kentos-scene3d`, çizici
  `kentos_render_wgpu::scene3d`), [8047](8047-ifc-building-export.md) (binanın IFC'si: `kentos_ifc::building::Building`),
  [8061](8061-ifc-viewer-and-reading-engine.md) (IFC görüntüleyici: gevşek bağ, BIM-06 ile ortak parçalar),
  [8062](8062-digital-project-rule-catalog.md) §14 ve bim-plan K10 (kararı Python verir, Rust ölçer),
  [8064](8064-ek3-ek4-2d-checks-python.md) (`cad` olguları).
- **Dayanak:** Hazırlama Yönetmeliği Md. 14 (gelişim seviyesi), Md. 15 ve EK-9 Tablo 9.1 satır 13 ("Mahaller diğer
  mahallerle örtüşmeyecek şekilde ve doğru katta modellendi mi?"), 14, 15; bim-plan Z1.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `cad` bölümüne isteğe
  bağlı `roomOverlaps` (yalnız örtüşme varken yazılır; eski olgular aynı bayt); SDK şeması yeniden üretildi. **Komut:**
  yeni komut yok; `scene3d.view` yol projesi dışında binayı açar. Görünümdür, çizimi değiştirmez (araç = komut kuralı
  çizimi değiştiren işler içindir). **Paket:** `dijital-proje` 0.5.0 (`kalite`).

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Solibri, Space Validation (kural 202) ve General Intersection (SOL/1) ([kural şablonları](https://help.solibri.com/hc/en-us/sections/1500000754682-Rule-Templates), [kural seçimi](https://help.solibri.com/hc/en-us/articles/1500006958582-How-to-Choose-the-Appropriate-Rule), [önem tablosu](https://society.solibri.com/assets/uploads/files/1647586669178-smc_severity_table.pdf)) | Mahal doğrulamasında "space intersection" ayrı bir sonuç türü; kabul edilen kesişim için bir tolerans parametresi; genel kesişim kuralı kesişen hacmi hesaplar, küçük kesişimi hacim toleransıyla yok sayar | Ortak alan olgudur (Rust ölçer, 0,0001 m²); "ne kadarı fazla" kuraldadır (Python, 0,01 m²) |
| BIMcollab Zoom ([mahal ve açıklıklar](https://helpcenter.bimcollab.com/portal/en/kb/articles/viewing-openings-and-spaces), [renk ve saydamlık](https://helpcenter.bimcollab.com/portal/en/kb/articles/set-up-smart-views-to-both-color-and-set-transparency-of-components), [otomatik renk ve lejant](https://helpcenter.bimcollab.com/portal/en/kb/articles/classify-components-with-auto-color), [bulgudaki bileşeni boyamak](https://helpcenter.bimcollab.com/portal/en/kb/articles/color-components-and-annotate-viewpoints-in-issues)) | Mahal ve açıklık "sanal bileşen", açılıp kapanır; kurallı görünümler bileşeni boyar ve saydam yapar; bağlam beyaz saydam; renklerin lejantı; bulgunun bakışı bileşenlerini boyar | Mahaller yarı saydam hacim, düğmeyle gizlenir; bulgunun nesneleri kırmızı, seçilen turuncu; lejant yalnız sahnede olan parçaları sayar |
| Revit, Selection Box ([yardım](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-DocumentPresent/files/GUID-0BBD875D-0E36-4FA4-A997-D6E3573CA3FF.htm)) | Herhangi bir görünümde seçilen nesneler 3B görünümde kesit kutusuyla yalıtılır; kutu tutamaklarla büyütülür; Section Box işareti kaldırılınca kutu gider | Kesit kutusu sonraki adım: çizicide kırpma düzlemi yeni iştir ve IFC görüntüleyiciyle ortaktır (§8) |

Resmî sayfaların ikisi bu bilgisayardan okunamadı (403, 404); satırlar arama sonuçlarının yardım metinlerinden alıntılanan
kısımlarına dayanır. Hiçbir değer bu kaynaklardan alınmadı: eşik ve renkler KentOS'un kararıdır.

## §2. Karar

1. **Kaynak dosyanın modelidir.** 3B görünüm IFC dışa aktarımının kurduğu binayı (`arch_ifc::model` →
   `kentos_ifc::building::Building`) gösterir; gövdeler `kentos_ifc::mesh` ile üçgenlenir: prizmanın halkaları (çift-tek
   kuralı, dosyanın yazdığı gibi) kapaklar ve kenar başına bir yan dörtgen; içinden açıklık geçen prizma açıklığın denizlik
   ve lento kotlarında bantlara bölünür ve her bandın tabanı halkalardan o bantta duran açıklıkların izi çıkarılarak
   bulunur (geometri çekirdeğinin overlay'i), böylece pencere bir deliktir; yüzlerle tanımlı gövde (çatı) yüzleriyle;
   mahal hacmi. Kotlar katın kotu artı gövdenin kendi kotu. Gösterilen, dosyanın tuttuğudur: ayrı bir 3B model kurulmaz.
2. **İkinci motor yok.** Yolun penceresi genelleşti: sahne [`Source`] listesinden kurulur (parça, ad, nesnenin kalıcı
   kimliği, köşeler, üçgenler, yumuşak gölge, içinden tıklanır); yol ve bina aynı çekirdeği (`chunk`, `select`, `pick`) ve
   çiziciyi kullanır. Sahne her kurulduğunda içerik kimlikleri yeni bir bloktan (2³²) başlar: çizici yüklediğini kimliğiyle
   tuttuğu için yeniden kurulan sahne eskisinin karolarını çizmez. Bu, yolun Araziyi kes ve Düz yüzler düğmelerinde de
   vardı (her sahne 1'den başlıyordu); giderildi.
3. **Görünüş:** sınıfa göre renk (duvar, döşeme ve kaplama, kolon, kiriş ve dikme, çatı ve baca, merdiven ve rampa,
   korkuluk, kapı, pencere, öbürü); pencere camı ve mahal yarı saydam (0,55 ve 0,16); planda seçili nesne turuncu;
   Denetim bulgusunun nesnesi kırmızı, mahalse yarı saydam kırmızı. Işık bakanın sol omzunun üstünden gelir ve bakışla
   döner: hangi yandan bakılsa görünen iki yüz farklı aydınlanır. Binada düşey abartma yoktur; sığdırma binanın kutusunu
   bütün gösterir.
4. **Etkileşim:** hazır bakışlar; yörünge, kaydırma, yaklaşma (yolunki); kat süzgeci (adı çizimin kat adı: Zemin kat,
   1. kat); mahaller aç ve kapa. Tıklama mahal hacminin içinden geçer, arkasındaki nesneyi bulur; mahal ancak arkasında
   bir şey yoksa seçilir. Tıklanan nesne planda seçilir; boşluğa tıklamak planın seçimini boşaltır (planda boşluğa tıklamak
   gibi); planın seçimi pencere açılırken turuncudur. Model değişince (pencere açıkken geri alma) sahne yeniden kurulur,
   kat ve mahal işaretleri adlarıyla korunur; gösterilecek gövde kalmazsa pencere sebebini söyleyip kapanır.
5. **Bulgunun rengi:** Denetim'de seçili satırların nesneleri; seçili satır yoksa listelenen bütün bulguların. Kesit
   binasında 50 bulgunun çoğu çizim kuralıdır (katman adı, mahal numarası) ve neredeyse her nesneyi anar; hepsini boyamak
   binayı kırmızıya boğuyordu (ilk resimde görüldü). Seçili satır neye bakılacağını söyler. Satır seçmek zaten nesneyi
   planda seçip görünümü ona sığdırıyordu ([ADR 0703](0703-architectural-project-audit.md)); 3B görünüm aynı seçimi okur.
6. **Örtüşen mahal (EK-9/13):** Rust ölçer: aynı kattaki (ya da ikisi de katsız) iki mahalin çizilen alanlarının kesişimi
   (`intersect_area_sets`; yalnız kutuları kesişen çiftler), alanı (0,0001 m²'ye) ve merkezi `cad.roomOverlaps`'ta. Karar
   Python'da: `dijital_proje.kalite` ortak alanı 0,01 m² ve üstü olan çifti hata sayar, iki mahali nesnesiyle; altı
   çizimin yuvarlamasıdır. Bulgu Mimari denetimin Mevzuat grubunda (`digital_quality_room_overlap`); `mimari.denetle` üç
   kuralı (`ek34`, `turetim`, `kalite`) birlikte verir; paketin denetim katkısı `kalite`.
7. **EK-9 formu:** 13. soru hâlâ mimara sorulur; cevap metni yeni bulguyu ve 3B görünümü gösterir.
8. **Şerit:** Mimari › Proje › 3B görünüm (Denetim'in yanında, küçük düğme, adıyla). Katalog açıklaması binayı da anlatır.

## §3. Gövde sayısı

| | Kesit binası (`fixtures/arch/v1/section-building.kcad`) |
|---|---|
| Duvar, kapı, pencere, döşeme, kolon, kiriş, merdiven | 16, 5, 9, 3, 16, 8, 1 = **58 eleman** |
| Mahal | **10** |
| IFC dosyasında temsili olan yapı elemanı ve mahal (dosyanın metninden sayıldı) | **68** |

ADR 8047'nin bağımsız okuyucusundaki "68 gövde" 58 eleman ile 10 mahaldir. 14 açıklık bir kesimdir, gövde değildir;
3B'de duvardaki delik olarak görünür.

## §4. Masaüstü

```text
┌ 3B görünüm ────────────────────────────────────────────────────────────────────────────────────┐
│ [Kuzeydoğu] [Güneybatı] [Kuzeybatı] [Üst] [Sığdır]       Katlar [x] Zemin kat [x] 1. kat [x] Mahaller │
│ ┌ gövdeler: sürükle döndür, sağ tuş ya da Shift kaydır, tekerlek yaklaş; tıkla planda seç ┐   │
│ └─────────────────────────────────────────────────────────────────────────────────────────┘   │
│ ■ Duvar ■ Döşeme ■ Kolon ■ Kiriş ■ Merdiven ■ Kapı ■ Pencere ■ Mahal ■ Seçili ■ Bulgulu mahal   │
│ 58 yapı elemanı, 11 mahal · 2 nesnede Denetim bulgusu · 5208 üçgen, 1 karo …           [Kapat] │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Bu yüz 16 Kasım'da değişti; bugünkü yüz §9'dadır. Seçilen noktanın satırı nesneyi adıyla ve katıyla söyler: "Duvar ·
MM-DVR-Dış_duvar_25 · Zemin kat" (§9'dan beri "Duvar MM-DVR-Dış_duvar_25, Zemin kat"). Kod
`apps/desktop/src/shell/scene3d_window.rs` (pencere, sahne, çizici bağı) ve `scene3d_building.rs` (bina, parçalar, seçim,
yenileme).

## §5. Sınamalar ve kanıt

- **`kentos-ifc`** (`mesh`, 2): her eleman ve mahal kapalı bir ağ, hacmi kendi hacmi; pencere duvarda bir delik (denizlik ve
  lento yüzleri çizilir).
- **`kentos-extensions`:** `roomOverlaps` çifti kimlik sırasıyla, alanı 0,0001 m²'ye; bunun altı atılır.
- **`kentos-processing`:** `mimari.kcad`'de örtüşme yok; Salon'un merkezinden çizilen 1 m'lik kare 1,00 m² ve merkeziyle
  çift; aynı kare üst katta Salon'la karşılaştırılmaz.
- **Paket** (`tests/test_kalite.py`, 5): temiz durum; iki mahal hata, ikisi nesnesiyle, kat kodu ve satır 13 ile; eşiğin
  altı; adsız mahal kimliğiyle; `mimari` kuralı içerir. Öbür dört sınama dosyası geçiyor. `kentos kek verify`: 0 uyarı,
  4 katkı.
- **Masaüstü** (`scene3d_building_tests`, 3): (a) 58 eleman ve 10 mahal sahnede, dosyadaki 68 ile aynı; bir kat ve mahaller
  gizlenip geri gelir; abartma 1. (b) 3B'de bir duvara tıklanır, planda o duvar seçili ve 3B'de turuncu; boşluk seçimi
  boşaltır; planda seçilen duvar açılışta turuncu; duvar silinince sahne yeniden kurulur, duvar 3B'de yok. (c) Çizildiği
  gibi bina temiz; birinci dikdörtgen mahalin ortasına 1 m'lik kare mahal eklenince Denetim tek bulgu verir ("1.00 m²
  örtüşüyor", iki mahalin kimliği); satırı seçilince 3B'de iki mahal yarı saydam kırmızı, başka nesne kırmızı değil.
- **Yolun 3B sınamaları** genelleşmeden sonra da geçiyor (`scene3d_window_tests`).
- **Ortak süzgeçler:** `ribbon_tests`, `module_tests`, `history::text`, `catalog`, `settings`, `ek_rules`, `permit_`: 155
  geçti. `kentos-native-application` `arch_ifc`, `kentos-ifc` `quality` geçti.
- **Bilinen:** `kentos-processing`'in `the_architecture_facts_are_the_fixture_s` sınaması ana dalda da düşüyor (kaçış
  yolu 11.55 ↔ 4.45); bu işin farkı değil, altın dosya yenilenmedi (CLAUDE.md §23.4).
- **Resimler** `.run/gui-evidence/bina-3b-8067/` (açık 1440×900, koyu 1100×700; uygulamanın kendi görünüm ağacı, wgpu
  çizicisiyle): 1 şeritteki yer, 2 kuzeydoğu, 3 güneybatı, 4 duvar seçili, 5 planda seçili duvar, 6 yalnız zemin kat,
  7 örtüşen mahaller kırmızı, 8 Denetim'deki bulgu.

## §6. Kabul (plan: BIM-06 ilk kabul)

| Ölçüt | Durum |
|---|---|
| Bölüm binasının 68 gövdesi 3B'de | var: 58 eleman ve 10 mahal |
| Gövde sayısı dışa aktarılan dosyadakiyle eşit | eşit: dosyanın metninden 68 |
| 3B'de tıklanan duvar planda seçili | seçili; 3B'de turuncu |
| Örtüşen iki mahal kırmızı | Denetim'in bulgusu; satırı seçilince iki mahal kırmızı |
| Gerçek pencerede resim | **açık:** resimler uygulamanın görünüm ağacından gerçek çiziciyle; işletim sisteminin faresiyle kabul koşturulmadı (§8) |

## §7. IFC görüntüleyiciyle ortak

`kentos_ifc::mesh` (binadan üçgen), pencerenin `Source` girdisi, kat süzgeci, mahalleri aç ve kapa, tıkla ve planda seç bu
işle kuruldu. IFC görüntüleyici (IFC-12) okunan dosyayı aynı girdiye dolduracak (çekirdek kaynağını bilmez: burada kesit
modeli, orada dosya). ADR 8061'in saf görüntüleme çekirdeği kurulduğunda sahnenin kurulması ve seçimi oraya taşınabilir;
`mesh` modülü bugün yeni bağımlılık getirmediği için `kentos-ifc`'te ayrı bir özellik (feature) değil.

## §8. Eksik kalanlar

- **Gerçek pencerede kabul:** `scripts/usage/real/` düzeneği Xvfb'ye (Linux) bağlı; bu Mac'te sahibin masaüstü kullanımdaydı,
  fare ele geçirilmedi. Sahibin onayıyla gerçek pencerede tıklama ve resim ayrı adımdır.
- **Kesit kutusu:** çizicide kırpma düzlemleri yeni iş; IFC-12 ile ortak kurulacak.
- **Tadilat durumuna göre renk; gizle ve yalnız bırak; yapı ağacı ve özellik paneli** (IFC-12 ile ortak parçalar).
- **Katlar arası örtüşme:** iki kata uzanan mahal (galeri, dubleks) ve katlar arası 3B kesişim ölçülmüyor; olgu aynı katın
  2B kesişimidir. EK-9/14 (kapalı hacim ve üst sınır), /15 (doğru kot) ve /16 (yinelenen) henüz kural değil (BIM-08,
  BIM-10).
- **Büyük model:** sahne her seçimde ve kat süzgecinde yeniden kurulur; kesit binasında fark edilmez, büyük binada
  ölçülmedi. Gerekirse seçim, karoları yeniden kesmeden parçanın rengini değiştirecek biçimde ayrılır.
- **Python bulgusunun istisnası:** `cad.arch.exceptions` yalnız Rust bulgularını tanıyor (BIM-03'ten beri açık).

## §9. Pencerenin yeni yüzü (16 Kasım 2026)

Sahibin isteği: “mimari projeyi 3d olarak görüntüleyen pencereyi biraz düzenleyelim, özellikle toolbar, alt taraf; şu
anda tasarım hoş değil”. Eski yüzde beş çerçeveli düğme yan yana duruyordu. Altında renk kutuları vardı, sonra “A · B · C”
biçimli uzun bir bilgi satırı (sayılar, kamera açıları, fare yardımı) ve Kapat. Sahne kutu içinde kutuydu, gökyüzü
sabit renkti. Pencere artık kendiliğinden işletim sisteminin ayrı penceresinde açılır (ADR 8060 §9). Yüzü DESIGN.md
§7.20'dir:

```text
┌ 3B görünüm — KentOS CAD ──────────────────────────────────────────────────────────────────────┐
│ ◈ Kuzeydoğu ▾  ⤢ Sığdır │ ▤ Katlar 1/2 ▾  ▢ Mahaller                                      ⇱ │
├───────────────────────────────────────────────────────────────────────────────────────────────┤
│                                   gövdeler, kenardan kenara                              (K)  │
│ ┌ Lejant ⌄ ┐                                                                                  │
│ │ ▣ Duvar  │                                                                                  │
│ └──────────┘                                                                                  │
├───────────────────────────────────────────────────────────────────────────────────────────────┤
│ ⌖ Duvar MM-DVR-Dış_duvar_25, Zemin kat  X 12.125  Y 2.049  Z 0.268   58 yapı elemanı │ ⚠ │ ? │
└───────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Araç şeridi** stil pencerelerinin şerididir (`style::strip`). Dört bakış tek menüde toplandı (“Görünüş ▾”). Menünün
  yüzü o anki bakışın adıdır, menüde “Kuzeye bak” da vardır. Katlar artık yan yana onay kutuları değil. Onların yerinde
  menü açıkken art arda işaretlenen bir liste (`Menu::stay`) ve gizli kat varken “Bütün katları göster” var. Çok katlı
  binada eski satır pencereye sığmıyordu. Şeride iki yardımcı eklendi: `labelled` (simge ve ad) ve `toggle` (adlı
  anahtar).
- **Kuzey oku** (kitin `Compass`'ı, DESIGN.md §8 sağ üst): kamerayla döner (`north_on_screen`), tıklanınca kamera kuzeye
  bakar (`Event::North`, yükseklik ve uzaklık kalır). Kamera açıları satırdan kalktı. Yerlerini okun ipucu aldı (“Model
  kuzeydoğudan görülüyor, 30° yukarıdan”). Açı sayısı ölçmecilik semtiyle karışabiliyordu (CLAUDE.md §5).
- **Lejant** sahnenin sol altında, harita lejantının kutusundadır (`Legend`, `legend::frame`) ve başlığı daraltır.
  **Renk düzeltmesi:** iced 0.14 varsayılan olarak web renkleriyle çizer (`web-colors`: gamma düzeltmesi yok, yüzey sRGB
  değil). Çizici de parçanın rengini ışıkla çarpıp ekrana olduğu gibi yazar. Eski lejant rengi gamma ile açıyordu
  (`powf(1/2.2)`), bu yüzden duvar kutusu açık zeminde görünmüyordu. Lejant artık ışığa dönük yüzün gösterdiği rengi
  çizer: parçanın rengi, kenarı bir ton koyu.
- **Durum çubuğu** ana pencerenin `StatusBar`'ıdır. Hücreleri: seçilen nokta, miktar (üçgen ve karo ipucunda), bulgu,
  Fare. `Building3d::counted` artık bulguyu söylemez, bulgu ayrı hücredir (`found_shown`). Kaynak adı “A · B · C” değil:
  “Duvar MM-DVR-Dış_duvar_25, Zemin kat”. Adı sınıfın sözcüğüyle başlıyorsa sınıf iki kez yazılmaz. Yolda “Şerit, Ana
  koridor”.
- **Gökyüzü** sabit renk değil, çizim alanının zeminidir (`viewport::palette(canvas)`). Tepeye doğru o zeminin mavisiyle
  (`mark_colors(canvas).window`) koyulaşır: açık zeminde tepede %16, ayakta %3; koyu zeminde tepede %3, ayakta %12. Yüksek
  karşıtlık zemini düz siyahtır. CAD projesinde çizim zemini açık temada da koyudur (ADR 0165 §5). 3B de planın zeminini
  alır. Oranlar KentOS'un seçimidir, bir kaynaktan gelmez.
- **Kapat düğmesi kalktı:** ayrı pencerede kapatma işletim sisteminin düğmesidir, çizimin üstünde başlıktaki ×'tir.
  İz oynatıcının “Kapat”ı ikisine de gider.
- **Sınamalar ve resimler:** `scene3d_view::tests` (kuzey okunun yönü, “hangi yandan”), `scene3d_window_tests` (Kuzeye
  bak, lejant), `scene3d_building_tests` (açık pencerede seçim, bulgu hücresi). Resimler: `.run/gui-evidence/3b-pencere/`
  (yol: dört bakış, saydam, kesilmemiş, seçilen nokta, ana pencerede, tam ekran; koyu ve dar) ve
  `.run/gui-evidence/bina-3b-8067/` (Katlar menüsü açık, seçili duvar, örtüşen mahaller).
