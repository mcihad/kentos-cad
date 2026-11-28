# ADR 8177: Güneş ve gölge: güneşin konumu, binanın zemindeki gölgesi

- **Durum:** kabul edildi ve uygulandı (28 Kasım 2026). 1. bölüm: güneşin konumu, plandaki gölge, Gölge aracı (GL).
  2. bölüm: 3B görünümde güneş ışığı ve gölge haritası, Güneş düğmesi.
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-15. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporunun 5.
  bölümü ([2026-11-27](../research/2026-11-27-mimari-3b-kesit-ve-araclar.md)): konum ve tarihle güneş ve gölge
  piyasadaki araçların hepsinde var, KentOS'ta yok.
- **Sözleşmeler:**
  - Belge değişmedi. Gölge düz nesnelerdir (tarama, kapalı çoklu çizgi, yazı); `.kcad` ve belge şeması aynı. Aracın
    hatırladıkları (konum, saat dilimi, tarih, saat) uygulama belleğindedir, dosyaya yazılmaz.
  - 3B gölgelendiricinin düzen sözleşmesi 5. sürüm (`shaders/wgsl/scene3d.layout.json`): View bloğu 208 → 288 bayt
    (`shadow`, `sun`), grup 0'ın 2. ve 3. bağlamaları (gölge haritası ve karşılaştıran örnekleyici), `shadow` hattı.

## §1 Sorun

- Mimar binanın vaziyetteki gölgesini elle kuruyordu: güneş yüksekliğini bir tablodan okuyup her köşeyi ayrı ayrı
  izdüşürüyordu. Bina değişince gölge baştan çiziliyordu.
- Komşu parsele, bahçeye, oyun alanına ne kadar gölge düştüğü tasarımda sorulan bir soru. KentOS bunu
  cevaplayamıyordu.

## §2 Araştırma

### Piyasa

- **Revit:** güneş ayarları. Konum şehirden ya da enlem ve boylamdan, tarih ve saat; "tek an", "tek gün" (saat
  aralığıyla), "çok gün" güneş çalışmaları. Gölgeler görünümde açılır.
- **Archicad:** konum (enlem, boylam, saat dilimi) ve tarih-saat güneşi kurar; "güneş çalışması" bir gün ya da günler
  boyunca görüntü dizisi üretir.
- **Ortak nokta:** güneşin yeri konumdan, tarihten ve saatten hesaplanır. Proje coğrafi konumdaysa konum ondan gelir;
  coğrafi kuzey plan kuzeyinden ayrı tutulur. Tek saat de saat aralığı da çizilebilir.
- **Gölge çalışmasının günleri:** kış gündönümü (21 Aralık, en uzun gölge), ılım noktaları (21 Mart, 23 Eylül) ve yaz
  gündönümü (21 Haziran, en kısa gölge). Saat aralığı çoğunlukla 09:00–15:00.

### Mevzuat

- **Binalarda Enerji Performansı Yönetmeliği** (RG 5.12.2008/27075; metin 1.4.2010/27539 değişikliğiyle okundu):
  - **7/1:** mimari tasarımda "güneş, nem ve rüzgar etkisi de dikkate alınarak, doğal ısıtma, soğutma, havalandırma
    ve aydınlatma imkanlarından azami derecede yararlanılır".
  - **7/2-a:** binaların ve iç mekânların yönlendirilmesinde güneş verileri dikkate alınır.
  - **7/2-b:** sürekli kullanılan yaşam alanları "güneş ısısı ve ışığı ile doğal havalandırmadan en uygun derecede
    faydalanacak şekilde yerleştirilir".
  - **8/2, son cümle:** "yaz aylarındaki istenmeyen güneş enerjisi kazançları için tasarım sırasında tedbirler
    alınır".
- **Planlı Alanlar İmar Yönetmeliği:**
  - **23/1-ç:** dörtten fazla katlı binada yan ve arka bahçe mesafesi, dördüncü kattan sonraki her kat için 0,50 m
    artar. Bina yükseldikçe komşusundan uzaklaşır.
  - **32/1:** her dairede en az bir oturma odası ve bütün yatak odaları doğrudan dış havadan ışık ve hava alır.
- Ruhsat eklerinde gölge çizimi zorunlu değil. Yukarıdaki maddelerin istediği değerlendirmeyi yapmanın aracıdır.
  Yüksek yapılar için gölge etüdü bazı imar planlarının notlarında istenebilir; ulusal bir hüküm bulunamadı.

### KentOS'ta olan

- **Kesit modeli** (`sections::model`, ADR 8042): duvar, kolon, kiriş, döşeme, çatı ve merdiven cisimleri, her biri
  bir plan halkası ve bir yükseklik kuralıyla (prizma, eğik düzlem, merdiven kolu). Kesit ve görünüş zaten bundan
  çiziliyor.
- **Coğrafi kuzey** (`north::model_north`): projenin koordinat sisteminden (meridyen yakınsaması), yoksa çizimdeki
  kuzey okundan.
- **Koordinat sisteminden enlem ve boylam:** `System::geographic_of`.
- **Alan birleşimi ve farkı:** `union_areas`, `subtract_areas`.

## §3 Karar

### §3.1 Güneşin konumu

- `geom::sun::at(enlem, boylam, saat dilimi, gün, saat)` güneşin azimutunu (kuzeyden saat yönünde) ve yüksekliğini
  verir. Formüller NOAA'nın genel güneş konumu hesabıdır:
  - kesirli yılın Fourier serisinden zaman denklemi ve deklinasyon;
  - gerçek güneş saatinden saat açısı.
- Doğruluk sınamalarla ölçüldü: Greenwich'te yaz gündönümü öğlesi 0,1° içinde, Sivas'ta kış gündönümü öğlesi
  0,15° içinde. Kırılma hesaba katılmaz; 09:00'da (yükseklik yaklaşık 10°) etkisi 0,1°'nin altındadır.
- **Türkiye saati güneşin yarım saat önünde:** Sivas'ta (39,75 K, 37,02 D) güneş öğlesi kışın 12:30, yazın 12:34
  (UTC+3). Saat 12:00'da güneş tam güneyde değildir, 21 Aralık'ta 172°'dedir. Saatler saat dilimiyle yazılır, güneş
  saatiyle değil: mimarın sorusu "saat on ikide".
- **Plandaki yön:** `Sun::in_plan(kuzey)` güneşe doğru birim yönü plan eksenlerinde verir. Coğrafi kuzey bilinmiyorsa
  plan kuzeyi alınır ve bu söylenir.

### §3.2 Gölge

- Kesit modelinin ±0,00'ın üstündeki her cismi güneş ışınları boyunca zemine süpürülür:
  - **prizma** (duvar, kolon, kiriş, döşeme): alt kotundan (en az ±0,00) üst kotuna;
  - **eğik düzlem** (çatı yüzü): her köşe kendi yüksekliğinden;
  - **merdiven kolu**: tabanından son basamağına.
- **Delikli cisim** (avlulu ya da merdiven boşluklu döşeme): halkaları tek-çift kuralıyla okunur; bir halka, köşesi
  öbürlerinin tek sayısının içindeyse deliktir (orta noktası değil: avlulu dış halkanın ortası avludadır). Gölgesi dış
  halkalarının süpürülmesi, eksi her deliğin alt ve üst yüzünün birlikte geçirdiği ışık. İlk sürüm her halkayı dolu
  süpürüyordu: deliğin de gölgesi düşüyordu (10 m'lik döşemede 4 m'lik delik: 101,5 yerine 86,1 m²).
- Bir yükseklik birimi zemine `(−sx/up, −sy/up)` kadar kayar. Süpürülen halkaların birleşiminden binanın ±0,00'da
  oturduğu alan çıkarılır: kalan, binanın zemindeki gölgesidir.
- **Binanın oturduğu alan:** ±0,00'a inen cisimler, binanın oradaki dış çevresi (`site_ground::building_corners`,
  ADR 8174) ve bunların arasında bir cismin (döşeme, çatı) örttüğü her boşluk. İlk sürüm yalnız ±0,00'a inen
  cisimlerin halkalarını çıkarıyordu: duvarlar çıkıyor, **odaların içi taranıyordu** (gerçek pencerede yakın görünümde
  görüldü). Örtülü oda binanın kendisidir, gölgesindeki zemin değil. Açık bir oda (üstünde hiçbir şey yok) gölge
  alır; kolonlara oturan bir döşemenin altı da (sundurma gibi) gölgededir.
- **Dışbükey halka tek parça:** dışbükey bir halkanın süpürülmesi iki ucunun dışbükey zarfıdır, birebir. İlk sürüm her
  halkayı iki uç ve her kenar için bir paralelkenar olarak birleştiriyordu: kesit binasının 168 cisminde 986 parça,
  birleşimi hata ayıklama derlemesinde saatte 1,3–2,1 s. Dışbükey olmayan halka eski yolla süpürülür.
- **Saatler yan yana:** gün boyu çiziminin yedi saati ayrı iş parçacıklarında hesaplanır (`shadows_of`). Ölçüm
  (kesit binası, hata ayıklama derlemesi): gün boyu **10,5 s → 0,46 s**, sonuç aynı (35 alan). İlk sürümde arayüz
  bu sürede donuyor, son seçim yapılmamış gibi görünüyordu (gerçek pencerede görüldü). Enter önizlemenin
  hesapladığını yazar; belge o arada değiştiyse yeniden hesaplar.
- Güneş 0,5°'nin altındaysa gölge yok sayılır: kilometrelerce uzun bir gölge bir şey söylemez.
- **Zemin ±0,00'da düz kabul edilir.** Vaziyetin tesviye zemini (ADR 8174) ve komşu yapılar hesaba girmez (Kalanlar).

### §3.3 Gölge aracı

- **Gölge (GL; GOLGE, GUNESGOLGESI, GOLGEDIYAGRAMI):** Proje ▾ altında, Zemin kotu'nun yanında.
  - **Tarih (T):** 21 Aralık, 21 Mart, 21 Haziran, 23 Eylül.
  - **Saat (S):** 09:00, 12:00, 15:00 ya da 09:00–15:00 saatlik.
  - **Enlem (E), Boylam (B):** proje coğrafi konumda değilse istemde seçenek olarak durur ve istem onları ister.
    Masaüstü bir harfi araca ancak istemin seçeneğiyse verir: ilk sürümde E ve B yalnız aracın kendi okumasındaydı ve
    tuvaldeki harf araca hiç ulaşmıyordu (gerçek pencerede görüldü). Yazılan değer komut satırına gider; ondalık
    virgülle de okunur. **Saat dilimi (Z):** UTC+3, ayar panelinde.
- Gölge ayarlar değiştikçe önizlenir. Önizleme etiketi güneşin yüksekliğini ve azimutunu ya da neden çizilemediğini
  söyler. Enter ya da tıklama gölgeyi yazar ve araç çıkar.
- **Tek saat:** gölge 45° düz çizgi taramasıyla (kâğıtta 3 mm aralık, açık gri #9E9E9E) ve koyu gri çerçevesiyle
  (#616161); altındaki çizim okunur kalır. Uzak görünümde koyu tarama katı bir blok gibi okunuyordu.
  **Birden çok saat:** her biri çerçevesi ve ucunda (ışınların en uzağa düşürdüğü köşe) saatiyle; taranmaz.
- Gölge katmanına (MM-XXXX-GOLGE, EK-4 "Genel" öğesi) tek geri alma adımında yazılır. Her nesne `Gölge` özniteliğini
  (saatiyle) taşır; yeniden çizilince eskisi silinir.
- **Gölge canlı değildir:** düz nesnelerdir, bina değişince yeniden çizilir. Canlı gölge her düzenlemede kesit
  modelini yeniden kurmak demek.
- **Hatırlanır:** yazılan enlem, boylam, saat dilimi, seçilen tarih ve saat uygulama açık kaldıkça bir sonraki
  çalıştırmada gelir.

### §3.4 3B'de güneş ve gölge (2. bölüm)

- **Gölge haritası** (`crates/render/scene3d/src/shadow.rs`): saydam olmayan yüzler güneşten bir kez daha, derinlik
  haritasına çizilir. Harita 4096 × 4096 texel (Hızlı'da 2048), Depth32Float, standart derinlik (güneşe yakın 0).
  - Dik izdüşüm sahnenin kutusuna (sahne ve kesit kapakları) güneşin bakışından sıkı oturur, bir texel payla: 30 m'lik
    bir binanın gölgesi texel başına 7 mm. Kutu köşelerinin tabanına düşen gölgeleri de derinlik aralığına girer:
    zemine düşen uzak gölge ışın boyunca her köşeden derindedir; girmeseydi aydınlık sayılırdı.
  - Kesit düzlemleri gölge düşürenleri de keser: kesitin kaldırdığı parça gölge düşürmez. Saydam parça (cam, mahal)
    gölge düşürmez.
  - Görüntünün dışında kalan döşemeler gölge düşürmez (yalnız çizilenler); tek binada hepsi çizilir.
- **Zemin:** güneş açıkken görüntüleyici sahnenin altına bir zemin koyar (`model-view-ui` `Ground`): sahnenin
  genişliğinin üç katı (en az 40 m) kare, açık sıcak gri. Yükseklikler ±0,00'ı kapsıyorsa 2 cm altında (zemin katın
  döşemesi üstünde görünür), kapsamıyorsa sahnenin 2 cm altında. Zeminsiz bina gökyüzünde duruyor, gölgesi hiçbir
  şeye düşmüyordu (gerçek pencerede görüldü). Piyasadaki güneş çalışmalarının da bir zemin düzlemi var.
- **Yüzlerin ışığı:** doğrudan ışık, haritayla yapılan 25 karşılaştırmanın (5 × 5, her biri dört texelde süzülür)
  aydınlık bulduğu paydır: gölgenin kenarı iki texel kadar yumuşak, basamaksız. Ortam ışığı, gökyüzü ve dolgu ışığı
  gölgelenmez.
- **Çizgilenmeye karşı alıcı düzlem:** her karşılaştırma, yüzün haritadaki derinlik eğimiyle (pikselin
  türevlerinden) kendi yerindeki derinlikte yapılır; örnek noktası normal boyunca güneşe dönük yüzde yarım, sıyıran
  güneşte iki texel kaydırılır, derinlikten bir texel düşülür. İlk sürümde sapma tek texeldi: 45°'lik güneşte dış
  karşılaştırmalar yüzün kendisini gölgede buluyor, zeminde ve kolonun tepesinde yatay bantlar çıkıyordu (ekran
  kartında çekilen resimde görüldü). Alıcı düzlemle bantlar gitti, ama 14°'lik Aralık 15:00 güneşinde aydınlık zemin
  hafif kumlanıyordu; 12°'lik güneşte ölçüldü: 84…91 (255 üzerinden). Sıyıran güneşte kaydırma büyüyünce düzgün.
- **Güneş düğmesi** (3B görünümün şeridi, `apps/desktop/src/shell/scene3d_sun.rs`):
  - Açılınca bina seçilen gün ve saatin güneşiyle aydınlanır, gölgelerini düşürür; Tarih ▾ dört gün, ◀ saat ▶ bir
    saatlik adımlar.
  - Konum, saat dilimi ve ilk gün/saat Gölge aracınındır (`Memory::sun_shadow`; gün boyu seçiliyse 12:00). Konum
    yoksa düğme açılmaz, Gölge (GL) aracında enlem ve boylamın verilmesini söyler.
  - Güneşin ufkun altında (0,5°'den alçak) olduğu saate geçilmez, söylenir. Başka güne geçince o saatte güneş
    doğmamışsa o günün öğlesine gidilir.
  - Durum çubuğu güneşi yazar: "Güneş: 21 Aralık 12:00, yüksekliği 26,5°, azimut 172°".
  - Paftaya koy'un resmi pencere gibi aydınlanır (`Viewer::picture`).

### §3.5 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | Kesit modeli ve vaziyet Mimari'nin. |
| Harita, Yol, CBS | yok | Bina modeli yok. |
| MCP, Python, eklenti, yapay zekâ ajanı | düz nesne | Gölge düz nesnelerdir (`cad.entities.create`, `cad.entities.delete`). Gölgeyi hesaplayan anlamsal bir işlem yok (Kalanlar). 3B'nin Güneş'i pencerenin kendisidir. |

## §4 Kanıt

- **Güneş** (`geom::sun`, 4 sınama): Greenwich ve Sivas'ın gündönümü öğleleri; ılım noktasında güneşin doğudan
  doğup batıdan batması, gece ufkun altında olması; plan kuzeyi dönünce güneşin de dönmesi; gün ve aydan yılın günü.
- **Uygulama** (`crates/native/application/tests/all/shadows.rs`, 5 sınama; deliğin içinden ışık geçiyor:
  `light_passes_through_a_slab_s_hole`):
  - 1 × 1 m, 3 m yüksek kolon, 45° güneş: gölge kuzeye 3 m, alanı 3,00 m² (kolonun oturduğu alan dışında). Alçak
    güneşte daha uzun; güneş batınca yok.
  - Konum: koordinat sistemi yoksa enlem ve boylam istenir; verilince kullanılır, kuzey oku yoksa plan kuzeyi alınır.
    21 Aralık 12:30'da Sivas'ta güneş tam güneyde, 26,8°.
  - Tek saat taramalı, çerçeveli ve saatli; iki saat taramasız, iki çerçeve ve iki saat; yeniden çizilince eskisi
    gidiyor.
  - Binanın oturduğu alan: kutu binada gölge yalnız kuzeyindeki 6 × 3 m (18,00 m²). İkinci binanın odası 1. katın
    döşemesiyle örtülüyse gölge almıyor; açıksa güney duvarının gölgesi içine düşüyor (5,5 × 3 m fazla).
- **Etkileşim** (`crates/native/interaction/tests/all/sun_shadow.rs`, 3 sınama):
  - Konumsuz projede istemde Tarih, Saat, Enlem, Boylam; Enter uyarı veriyor, araç konumu bekliyor.
  - E 39,75 ve B 37,02 ile önizleme çiziliyor; Enter gölgeyi MM-XXXX-GOLGE'ye tek adımda yazıyor: "Gölge çizildi: 21
    Aralık 12:00, güneş yüksekliği 26,…°, azimut 172° (kuzey oku yok, plan kuzeyi coğrafi kuzey sayıldı)." Araç
    çıkıyor, geri alma hepsini götürüyor.
  - 09:00–15:00 saatlik yedi saat, taramasız. Araç yeniden açılınca konum ve saat seçimi geliyor. 21 Haziran
    12:00'da güneş 72,3°, azimut 154°; eski gölge gidiyor.
- **Ekran kartı** (`crates/render/wgpu/tests/gpu/scene3d.rs`, `KENTOS_GPU_TESTS=1`, Intel UHD 770, Vulkan):
  - 10 m'lik zemin üstünde 1 × 1 × 3 m kolon, güneş güneyden 45°: kolonun kuzeyi 61 (yalnız ortam ışığı: 0,8 × 0,3),
    güneyi ve kuzeydoğusu 163 (0,8 × (0,3 + 0,7 × 0,707)). Gölge istenmeyince kuzey güney kadar aydınlık.
  - Kesit kolonun alttaki 1 m'sini bırakınca gölge 1 m: kuzeyde 2 m aydınlık, yarım metre gölgede.
  - 12°'lik güneşte kolonun güneyindeki aydınlık zemin düzgün (91 noktada en çok 2 fark; önce 84…91).
  - Resim: `.run/gui-evidence/3b-kalite-8120/gpu-gunes-golgesi{,-kesit,-alcak}.png`. Alıcı düzlemden önce zeminde
    bantlar vardı, sonra yok.
  - İzdüşüm (`shadow::tests`): kutunun ortası haritanın ortasında, köşesi kenarından bir texel içeride; güneşe yakın
    sığ; tepenin 33 m kuzeydeki zemin gölgesi haritanın içinde ve derinlik aralığında.
  - Düzen sözleşmesi (`styled_contract::the_scene3d_module_validates_and_is_the_contract_s`): naga gölgelendiriciyi
    doğruluyor; View 288 bayt; bağlamalar ve `shadowVs`/`shadowFs` sözleşmede.
- **Masaüstü** (`scene3d_building_tests::the_sun_lights_the_building_by_the_shadow_tool_s_place_and_steps_its_hours`):
  konumsuz projede Güneş açılmıyor ve nerede verileceğini söylüyor; Gölge'nin konumuyla 21 Aralık 12:00 (26,5°);
  13:00; 21 Haziran 13:00 (güneş 0,9'un üstünde yüksekte); Aralık'ta saat saat geri: güneşin doğmadığı saate inilmiyor,
  söyleniyor; kapanıyor.
- **Gerçek pencere, 3B** (`.run/gui-evidence/mimari-bim/sonra/mb15-3b-*`): Güneş kapalıyken bina gökyüzünde, gölge
  yok. Açılınca zemin geliyor; 21 Aralık 12:00'de binanın ve çerçevenin gölgesi kuzeye (kameraya doğru), çerçevenin
  döşeme ve kirişleri ızgara gölge, kirişler arasından ışık; 15:00'te gölge uzayıp kuzeydoğuya dönüyor; 21 Haziran
  15:00'te kısa ve zemin parlak. Kontrast açılınca da zemin pürüzsüz. Şeritte Güneş, 21 Aralık ▾, ◀ 12:00 ▶.
- **Gerçek pencere, plan** (`.run/gui-evidence/mimari-bim/{once,sonra}/mb15-*`; dosya: kesit binası ve çerçevesi,
  koordinat sistemi yok):
  - Önce: GOLGE "birden çok komutu anlatıyor: Gölgeli kabartma…"; gölge çizen araç yok.
  - Sonra: istem "binanın enlemini (E) ve boylamını (B) verin", Enlem ve Boylam düğmeleri görünür. E, 39,75 ve
    B, 37,02 ile önizleme: "21 Aralık 12:00, güneş yüksekliği 26,5°, azimut 172° (kuzey oku yok, plan kuzeyi
    coğrafi kuzey sayıldı)". Gölge kuzeyin biraz batısına düşüyor, batı duvarı boyunca ince bir şerit bırakıyor;
    çerçevenin gölgesinde kirişler arası ışık boşluk.
  - Yakında (1:97): tarama binanın kuzeyinde, odaların içi temiz; akslar ve balonları taramanın altından okunuyor.
  - Saat menüsünden 09:00–15:00 saatlik: yedi çerçeve, taramasız, eski gölgenin yerine. 21 Haziran 12:00: binanın
    kuzeyinde dar bir şerit.

## Kalanlar

- **3B'de görüntü dışındaki döşemelerin gölgesi:** gölge geçişi yalnız çizilen döşemeleri çizer.
- **IFC görüntüleyicide Güneş:** model dosyasının kendi konumundan (IfcSite enlem/boylam).
- **Gölge tesviye zeminde ve komşu yapılarda:** zemin bugün ±0,00'da düz.
- **Gölge süresi:** bir alanın gün boyu kaç saat gölgede kaldığı (saatlik gölgelerin üst üste sayımı).
- **Güneş yörüngesi şeması** (stereografik), vaziyette.
- **Anlamsal işlem** (`arch.shadow`): MCP ve Python'un gölgeyi hesaplatabilmesi.
- **Konum projede:** enlem ve boylam bugün uygulama belleğinde; koordinat sistemi olmayan projede dosyaya yazılmıyor.
- **Binanın içindeki açık avlu:** binanın dış çevresi oturduğu alana girdiği için iç avlu gölge almıyor. Avlu ancak
  dış çevrenin deliği olarak bulunabilir (`building_corners` bugün yalnız dış halkayı veriyor).

## Uygulama

- Güneş: `crates/shared/geometry-core/src/geom/sun.rs` (yeni).
- Gölge: `crates/native/application/src/arch/shadows.rs` (yeni): `place`, `located`, `sun_at`, `caster`, `shadow`,
  `shadow_of`, `shadows_of`, `draw`.
- Araç: `crates/native/interaction/src/arch/shadow_tool.rs` (yeni), `core/session.rs` (kayıt), `core/tool.rs`
  (`Memory::sun_shadow`), `arch/arch_layer.rs` (MM-XXXX-GOLGE).
- Katalog: `tool.sunShadow` ve simgesi, kısa adı GL; Proje ▾.
- 3B çizici: `crates/render/scene3d/src/shadow.rs` (yeni: izdüşüm, harita, hat), `lib.rs` (`Frame::shadows`,
  `render_shadows`, View bloğu), `shaders/wgsl/scene3d/mesh.wgsl` (`sunSlope`, `sunlit`, `shadowVs`, `shadowFs`),
  `scene3d.layout.json` (5. sürüm).
- Görüntüleyici: `crates/model-view-ui/src/{scene.rs, viewer.rs, viewer_view.rs}` (`SceneView::sun`, `Viewer::sun`,
  `Ground`).
- Masaüstü: `apps/desktop/src/shell/scene3d_sun.rs` (yeni), `scene3d_window.rs` (`Event::Sun`), `scene3d_view.rs`
  (şerit), `scene3d_building.rs` (`Building3d::sun`), `scene3d_sheet.rs`.
