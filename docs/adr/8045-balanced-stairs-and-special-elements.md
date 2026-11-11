# ADR 8045: Merdiven ve özel elemanlar — balansmanlı merdiven, korkuluk, asansör, otopark yeri

- **Durum:** bitti (11 Kasım 2026); `plan/architecture-advanced.md` ADV-21, M45. 1. bölüm yapıldı: balansmanlı
  (dengelenmiş) merdiven — çekirdek geometri, plan, model, Merdiven aracı, Öznitelikler ve PAİY 31/2-c denetimi (§1),
  kesit ve görünüşte tek tek dengelenmiş basamaklar (§2). 2. bölümde bağımsız korkuluk — model, `.kcad` 242, komut,
  plan, Korkuluk aracı (Boşluktan ile galeri boşluğu), Öznitelikler, kesit ve görünüş (§3) ve Denetim › Mevzuat'ta
  PAİY 38 (§4). 3. bölümde asansör: kitaplığın değiştirilebilir Asansör bloğu ve Denetim › Mevzuat'ta PAİY 34/2, 34/5
  (§5). 4. bölümde otopark yeri: kitaplığın Otopark yeri bloğu, Otopark hesabı'nda sayımı ve Otopark Yönetmeliği 4/1-ç,
  4/1-ı, 5/1-h, 5/1-j denetimi (§6).
- **Bağlam belgesi:** ADR 0343 (merdivenin yürüme çizgisi, sahanlıklar, basamak çizgileri), ADR 0704 (yükseklikten
  merdiven), ADR 0709 (döner merdiven), ADR 0724 (kata bağlı merdiven, üst kat planı), ADR 8034 (PAİY 31 denetimi).

## Bağlam

KentOS'un L ve U merdiveni dönüşte her zaman sahanlık kurar. Türkiye'de konutta sık görülen balansmanlı merdivende
dönüşte sahanlık yoktur: basamaklar dönüşün iki yanında "dengelenir", yürüme çizgisinde hepsi aynı derinlikte kalır, iç
kenarda köşeye doğru daralır. PAİY bu merdiveni adıyla düzenler; KentOS onu ne çizebiliyor ne denetleyebiliyordu.

## Araştırma

### Türkiye mevzuatı

- **PAİY Md. 31/2** (RG 3.7.2017/30113, resmî metin): (a) basamak yüksekliği asansörsüz binada 0.16, asansörlüde
  0.18 m'yi geçemez; (b) "Basamak genişliği 2a+b= 60 ila 64 formülüne göre hesaplanır … Ancak bu genişlik 0.27 metreden
  az olamaz."; (c) "Balansmanlı (dengelenmiş) merdivenlerde basamak genişliği en dar kenarda 0.10 metre, basamak
  ortasında 0.27 metreden az olamaz."; (ç) basamak uçları damlalıksız.
- **PAİY Md. 31/1-a:** ortak merdiven kolu ve sahanlık konutta 1.20 m, diğer yapılarda 1.50 m, bağımsız bölüm içinde
  1.00 m (genişlik denetimi ADR 8034'te var; balansmanlı merdiven kolunu aynı ölçüyle okur).
- **PAİY Md. 38** (resmî metin): (1) "Binalarda; balkon, teras, galeri boşluğu, sahanlıkların boş kenarları 1'den
  fazla basamağı bulunan açık merdivenlerde ve rampalarda, istinat duvarı üstünde, … bulunduğu katın taban
  döşemesinden itibaren kotu 0.90 metreden az olan pencere boşluklarında, döşeme kotundan itibaren en az 1.10 metre
  yüksekliğe kadar teknik gereklere ve standartlara uygun olarak korkuluk yapılması mecburidir." (2) Son kat açık
  teraslarındaki korkuluğun ve parapetin 1.10 m'si bina yüksekliğinden sayılmaz. (3) Kırılmaz ya da kırıldığında
  dağılmayan malzeme, insan çarpması dahil tasarım yükleri. (4) "Boşluklarda, yük altındaki deformasyonlar da dâhil,
  en fazla 0.10 metre çapında geçişe izin verilir." Md. 40/6: kullanılan teras çatıda 1.10 m parapet ya da korkuluk.
  Sivas İmar Yönetmeliği 38/1 PAİY ile aynıdır (ADR 8034).
- **PAİY Md. 34** (resmî metin; 34/1 RG 1.7.2026/33297 değişikliğiyle): (1) bodrumlar dahil 3 katta asansör yeri,
  4 ve daha fazlada asansör (ADR 8034). (2) "Tek asansörlü binalarda; asansör kabininin dar kenarı 1.20 metre ve alanı
  1.80 m²'den, kapı net geçiş genişliği ise 0.90 metreden az olamaz. Asansör kapısının açıldığı sahanlıkların
  genişliği, asansör kapısı sürgülü ise en az 1.20 metre, asansör kapısı dışa açılan kapı ise en az 1.50 metre olmak
  zorundadır. Birden fazla asansör bulunan binalarda, asansör sayısının yarısı kadar asansörün bu fıkrada belirtilen
  ölçülerde yapılması şarttır. Tek sayıda asansör bulunması durumunda sayı bir alta yuvarlanır." Taban alanında
  yapılaşma hakkı 120 m²'nin altındaki parselde ve tek bağımsız bölümlü müstakil konutta TSE ölçülerine uyulmasına
  idare yetkilidir. (4) umumi binalarda, plana göre 10 ve daha fazla katlı ya da zemin üstünde 20'den çok konutlu
  binada en az iki asansör. (5) "10 kat ve üzeri binalarda asansörlerden en az bir tanesi yük, eşya ve sedye taşıma
  amacına uygun olarak dar kenarı 1.20 metre ve alanı 2.52 m2'den, kapı genişliği ise net 1.10 metreden az olmayacak
  şekilde yapılır." TS EN 81-70'in Tip 2 kabini 1,10 × 1,40 m (kapı 0,90 m), PAİY'nin dar kenarından küçüktür.
- **Otopark Yönetmeliği** (RG 22.2.2018/30340, resmî metin): 4/1-ç "Binek otoları için birim park alanı, manevra
  alanı dâhil en az 20 m²'dir … Manevra alanı ve şekli dâhil park yerlerinin vaziyet veya kat planında gösterilmesi
  zorunludur."; 4/1-ı umumi bina, bölge ve genel otoparklarda "giriş-çıkış ve asansörlerine en yakın yerlerinde birden
  az olmamak şartıyla, her 20 park yerinden birinin" engellilere ayrılması; 5/1-h-8 "manevra alanı hariç, binek
  otoları için en az 2,40x4,90 metre, kamyonet ve panelvan için 3,00x6,00 metre"; 5/1-h-9 "uzun kenarı en az 4,90
  metre, kısa kenarı ise engelliler için en az 3,50 metre diğerleri için en az 2,40 metre"; 5/1-j (RG 25.3.2021/31434)
  zorunlu otoparkı 20 ve üzeri yeni yapılarda "1 adetten az olmamak üzere en az %5'inin" şarj ünitesi dahil elektrikli
  araçlara uygun düzenlenmesi. PAİY 4/jjjj: umumi bina "… otel, yurt, iş hanı, büro, pasaj, çarşı, alışveriş merkezi
  gibi ticari yapılar …".

### Başka programlar (yalnız görünüş ve kurgu için)

- **Archicad, Stair Turning Type** (help.graphisoft.com, Archicad 25): dönüş türleri Automatic Landing, Winder with
  Equal Angles ve Winder with Equal Goings. "Equal Goings means that all goings in the Stair are equal, both on the
  straight and the winding steps."; düzenlenebilen tek değer "the number of winders": orta rıhtın ya da basamağın
  altında ve üstünde kaç basamağın döneceği, zincirle iki yan eşit
  (`.run/gui-evidence/mimari-m45/referans/archicad-winder-equal-goings.png`).
- Revit'in winder kolları da "number of straight treads" ile dönen bölgenin uzunluğunu verir; KentOS aynı kurguyu
  tek sayıyla alır: dönüşün iki yanında dengelenen basamak.
- **Archicad, Railing Tool** (help.graphisoft.com, Archicad 29–30): "Railing input is similar to polyline input";
  merdivene, döşemeye, duvara, çatıya bağlanabilir; segment altında küpeşte (toprail), en çok iki tutamak, dikmeler,
  çubuklar (balusters) ve paneller ayrı bileşen; planda küpeştenin sembolü çift çizgi (eksenli ya da eksensiz) ya da
  yalnız eksen (`.run/gui-evidence/mimari-m45/referans/archicad-railing-*.png`).
- **Revit, Railings:** yol çizilerek kurulur, yükseklik ve çubuk yerleşimi korkuluk türünde
  (`revit-railing-sketch-path.png`). KentOS planı balkon korkuluğunun çift çizgisini (ADR 0722 §4) sürdürür; dikme ve
  çubuk ayrıntısı planda yok, kesit ve görünüşte gelecek.
- **Archicad, Elevator** (Archicad Library; Model View Options › Elevator: Show Entire Door, Show Opening Only, Hide
  on Floor Plan): planda kabin çift çizgi, arkasında karşı ağırlık, önünde teleskopik kapı kanatları ve açılma oku;
  kuyu duvarları ayrı çizilir (`.run/gui-evidence/mimari-m45/referans/archicad-elevator-*.png`). Revit'te de asansör
  bir aile, yani kitaplık nesnesidir.
- **Otopark yeri:** Archicad'in otopark nesneleri (Parking Place, Parking Stall, Parking Places Polyline) yer ölçüsü,
  numara ve isteğe bağlı engelli yeri taşır; Revit'te otopark yeri “Parking Space” ailesi, kendi kategorisinde
  (Parkplatz). Bimpedia'nın plan görüntüsü: dik sıralanan yerler, engelli yeri daha geniş ve tekerlekli sandalye
  işaretli (`.run/gui-evidence/mimari-m45/otopark/`).

## Karar

### 1. Balansmanlı merdiven (1. bölüm)

- **Model (`StairFields.winders`, `.kcad` 241):** dönüşün iki yanında dengelenen basamak sayısı; yoksa her dönüşte
  sahanlık. Şema 240 dosya yazamaz; okuyan "merdivenin balansmanı (winders) şema 241 ister" der. Alan düzeyinde ret:
  sıfır, döner merdiven ("basamakları zaten yay boyunca dengeli"), düz merdiven ("dönüşü yok"), sahanlıkla birlikte.
- **Geometri (`geom::arch`, `winder_turns`, `riser_ends`, `winder_treads`):** sahanlık yok; rıhtlar yürüme çizgisinde
  basamak aralıkla dönüşü dolaşır. Her dönüşün bölgesi köşenin iki yanında `winders × basamak`. İç kenar (saat yönü
  tersi dönüşte sol) dönüşte `genişlik × tan(dönüş/2)` kısa (90°'de genişlik kadar); bu kayıp bölge boyunca köşeye
  doğru doğrusal artan payla dağıtılır: köşedeki basamak en dar, bölgenin uçlarındaki basamak yürüme çizgisine dik.
  Bölgedeki rıht iç kenardaki noktasından yürüme çizgisindeki noktasından geçip dış kenara uzanır. 1,00 m'lik sola dönen
  L merdivende (2,80 + 2,24 m, 0,28 m basamak, 5 balansman) iç basamaklar 0,26 → 0,22 → 0,18 → 0,14 → 0,10 → 0,10 →
  0,14 … m; köşedeki rıht iç köşeden dış köşeye iner.
- **Ret (geometri, `winder_problem`):** bölge iç kenarın kaybından kısa ("en az ⌊kayıp / basamak⌋ + 1 basamak
  gerekir"), bölge kollarına sığmıyor, iki dönüşün bölgesi çakışıyor (kısa ara kollu U merdiven: şimdilik sahanlık).
- **PAİY 31/2-c'ye yeten en az balansman (`winders_for`):** iç kenarda hiçbir basamak 0,10 m'den dar kalmayan en küçük
  sayı. 1,00 m'lik L'de 5, 1,20 m'lik L'de 7, kolları 4 m'lik 1,00 m'lik U'da 6.
- **Plan:** basamak çizgileri `riser_ends` ile; kesme çizgisi kesme rıhtının sol ucundan bir basamak ötekinin sağ ucuna;
  üst kat planı aynı. Korkuluk ve band değişmedi.
- **Merdiven aracı:** Balansman (A): boş ya da "yok" sahanlık, tam sayı dönüşün iki yanında dengelenecek basamak; düz
  ve döner merdivende yok, iki noktalı (düz) yolda komuta verilmez. Tasarım satırı en dar iç basamağı yazar; 0,10 m'den
  darsa "PAİY 31/2-c'nin 0,10 m'sinden dar; balansman basamağını artırın" der.
- **Öznitelikler:** dönen merdivende Dönüş (Sahanlık / Balansmanlı; Balansmanlı'ya geçiş PAİY'ye yeten en az sayıyla),
  Balansman basamağı (boş: sahanlık) ve En dar iç basamak.
- **Denetim › Mevzuat (`paiy_stair_step`, PAİY 31/2-c):** en dar iç basamak 0,10 m'den darsa "Balansmanlı merdivenin
  basamağı dar uçta 0.090 m < 0.10 m — PAİY 31/2-c"; düzeltmesi PAİY'ye yeten en az balansman (`cad.arch.edit`).
  Basamağın ortası yürüme çizgisidir; oradaki 0,27 m mevcut basamak denetimi.
- **Sonraya:** kısa ara kollu U merdivende birleşik bölge, iç köşesi yuvarlatılmış (gözlü) merdiven.

### 2. Kesitte dengelenmiş basamaklar (1. bölüm, ikinci adım)

Kesit, merdiveni kollar (eğik plak üstünde dik basamaklar) ve sahanlıklarla kuruyordu. Balansmanlı merdivende sahanlık
olmadığı için ilk kol yürüme çizgisinin köşesinde bitiyor, dönüşün dış yarısı boş kalıyor, ikinci kol havada duruyordu;
plandaki dengelenmiş basamakların kesitte karşılığı yoktu.

- **Kollar dönüş bölgesinde durur (`sections::stair`, `stops`):** her dönüş, bölgesinden önceki son dik rıhttan
  bölgesinden sonraki ilk dik rıhta (yoksa merdivenin son rıhtına) uzanır; kol orada sahanlıktaki gibi biter.
- **Her dengelenmiş basamak ayrı bir prizma:** tabanı iki rıhtının uçları ve aralarında kalan band köşeleri
  (`geom::arch::tread_ring`, saat yönünün tersi); köşedeki iki basamak iç köşeyi tutar ve dış köşeye uzanır. Üstü kendi
  yüksekliği (taban + rıht × sırası), altı bir rıht ve merdivenin kalınlığı aşağısı, döşemenin altına inmez; merdiveni
  tutan döşeme yoksa (kalınlık bilinmiyor) döşemeye kadar dolu, kollarla aynı.
- **Alt yüz basamaklı (karar):** betonarme balansmanlı merdivenin alt yüzü burulmuş bir yüzeydir; onu çizmek kesit
  motoruna yeni bir yüzey türü ister. Ruhsat ölçeğinde (1/50, 1/100) basamak basamak inen alt yüz okunuyor; burulmuş
  alt yüz sonraya.
- Görünüşler aynı katı modelden çıkarıldığı için dengelenmiş basamaklar görünüşte de görünür; kesit düzleminin
  ardında kalan basamakların rıhtları görünüş çizgisiyle çizilir.

### 3. Bağımsız korkuluk (2. bölüm, ilk adım)

Korkuluk yalnız merdivenin ve rampanın yanında (ADR 0705) ve balkon ile terasın serbest kenarında (döşeme kenarı,
ADR 0722 §4) vardı. PAİY 38/1'in saydığı galeri boşluğu, sahanlığın boş kenarı, istinat duvarı ya da asma kat kenarı
için çizilebilen bir korkuluk yoktu.

- **Model (`RailingFields`, `Entity::Railing`, `.kcad` 242):** çizgisi (küpeştenin ortası; açıkken en az 2, kapalıyken
  3 nokta, art arda aynı nokta ve kapalıda sonda yinelenen ilk nokta yok), `closed`, yükseklik (döşemesinden
  küpeştenin üstüne; komut verilmezse 1,10 m, PAİY 38/1), küpeşte eni (verilmezse 0,05 m, balkon korkuluğununki),
  dolgu (`balusters` dikey çubuk — yoksa —, `glass` cam, `solid` dolu), çubuk aralığı (`gap`: çubuklar arası net
  boşluk; yoksa PAİY 38/4'ün 0,10 m'si, yalnız dikey çubukta okunur), kot farkı (döşemesinin katın döşemesinden farkı).
  Sınırlar: yükseklik 3 m, küpeşte 0,5 m, aralık 1 m, kot farkı ±10 m. Blok tanımında olmaz (kata oturur). Şema 241
  türü tanımaz (`unknown_kind`); sunucu `feature` tablosu `railing` türünü alır (0040).
- **Plan (`geom::railing`):** küpeştenin iki yüzü, orta çizginin iki yanında enin yarısı kadar (iç köşeler gönye),
  açık korkuluğun uçları kapalı — balkon korkuluğunun iki ince çizgisi; 1/200 sadeleştirilmiş görünüşte yalnız orta
  çizgi. Seçim ve sınır için açık korkuluk bant, kapalı korkuluk delikli halka.
- **Komut:** `cad.railings.create` v1 (“Korkuluk”, tek geri alma adımı; ret sırası `no_objects`, `not_finite`,
  `invalid_railing`, `level_not_found`, katman ve öznitelik retleri). `cad.arch.edit` korkuluğun `height`, `width`,
  `infill`, `gap` (null: 0,10 m) ve `offset` alanlarını değiştirir. Kat ver, Denetim'in katsız nesnesi, seçim süzgeci
  (Korkuluk, son sırada; kayıtlı süzgeçler bitlerini korur), CBS türevi (çizgi), DXF, GeoJSON korkuluğu tanır.
- **Korkuluk aracı (Mimari › Merdiven ▾ › Korkuluk, KO; Revit'in dolaşım paneli gibi merdiven ve rampanın yanında):**
  Çizgi (S) Çiz — noktalar, Enter yazar, Kapat (K) ya da ilk noktaya tıklamak kapalı yazar; Boşluktan — Etkin kattaki
  döşemenin boşluğunun içine tek tıklama. Yükseklik (Y), Dolgu (D), Aralık (A, yalnız dikey çubukta), Küpeşte (E),
  Kot farkı (F). Katmanı dolgusuna göre MM-KORK-CUBUK, MM-KORK-CAM, MM-KORK-DOLU (EK-2 Korkuluk, KORK). 1,10 m'den
  alçak ya da aralığı 0,10 m'den geniş korkuluk yazılır; önizleme etiketi ve ileti kırmızıyla PAİY 38'i söyler.
- **Boşluktan'ın kararları:**
  - Yalnız serbest kenarlar: duvar yüzünün boyunca uzandığı kenara korkuluk konmaz (balkon korkuluğunun
    `free_runs` kuralı, boşluk kendi döşemesi gibi okunur). İlk denemede boşluğun bütün çevresi kapalı yazılıyordu;
    görünüş binasının merdiven boşluğunda batı ve doğu kenarlar duvarın yüzündeydi ve korkuluk duvarın içine
    giriyordu.
  - Merdivenin çıkışı açık: alt kattaki düz bitişli merdivenin son rıht çizgisi (yürüme çizgisinin son noktasında
    bandın eni) boşluğun kenarından 2 cm içindeyse o parça kesilir; yoksa korkuluk merdivenin çıkışını kapatırdı.
  - 0,10 m'den kısa parça yazılmaz: PAİY 38/4'ün 0,10 m'lik cismi o aralıktan geçmez. Görünüş binasında merdiven bandı
    boşluktan 2,5 cm dar kalıyordu; 2,5 cm'lik bir korkuluk parçası çıkıyordu.
  - Bütün çevresi serbest boşluk tek kapalı korkuluk; parçalar ayrı korkuluk, hepsi tek geri alma adımı. Yay kenarlı
    boşluk reddedilir (noktalarla çizilir).
- **Öznitelikler:** Yükseklik, Dolgu, Çubuk aralığı (dikey çubukta), Küpeşte eni, Kot farkı (düzenlenebilir), Çizgisi
  (Açık/Kapalı), Uzunluk, Kat, Tadilat durumu ve Denetim: Uyuyor ya da “1,10 m'den alçak” (PAİY 38/1), “Aralık 0,10
  m'den geniş” (PAİY 38/4).
- **Kesit ve görünüş (`sections::railing`):** döşemesinden (katın döşemesi + kot farkı) yüksekliğine; katmanı dolgusunun
  (MM-KORK-CUBUK, -CAM, -DOLU), kesiti ince ve taramasız. Dikey çubuklu ve camlı korkulukta yalnız küpeşte katıdır
  (5 cm, kesitte küçük bir kare); çubuklar (aralık + 2 cm'lik çubuk, ilki yarım adım içeride) ya da camın ayağı ve
  uçları ayak izi olmayan bir katının çizgileridir: önündeki onları gizler, onlar arkasındakini gizlemez — kapının
  duvar yüzündeki çizgileri gibi (ADR 8043). İlk sürüm korkuluğu 1,10 m'lik opak bir bant yaptı; A-A'da merdiven
  holünün penceresi korkuluğun arkasında yarım kaldı (denizliği +3,90'da, küpeştenin altı görünmüyordu). Dolu
  korkuluk tam yüksekliğinde katıdır, küpeştenin altı iki yüzünde çizgi.
- **Sonraya:** metrajda korkuluk boyu, yay kenarlı boşluk, döşemeye bağlı (döşeme değişince izleyen) korkuluk, dikme
  ve tutamak ayrıntısı.

### 4. Korkuluğun mevzuat denetimi (2. bölüm, son adım)

`railing_audit` Denetim › Mevzuat'ta, bulgusu maddeyi ve Resmî Gazete'yi taşır:

- **Alçak korkuluk (`paiy_railing_low`, 38/1):** “Korkuluk 0.90 m < 1.10 m — PAİY 38/1”; düzeltmesi “Yüksekliği 1,10 m
  yap” (`cad.arch.edit`).
- **Geniş aralık (`paiy_railing_gap`, 38/4):** dikey çubuklu korkulukta “Korkuluğun çubuk aralığı 0.12 m > 0.10 m —
  PAİY 38/4”; düzeltmesi “Aralığı 0,10 m yap” (aralık alanı kalkar, PAİY'nin 0,10'u okunur).
- **Korkuluksuz döşeme boşluğu (`paiy_railing_missing`, 38/1 “galeri boşluğu”):** boşluğun korkuluk isteyen kenarları
  (duvarsız, alt kattan gelen merdivenin çıkışı dışında, 0,10 m'den kısa parça yok) Boşluktan'ın kendi hesabıdır
  (`railing_voids`, araç ve denetim aynı fonksiyonu okur). Kenar 5 cm'de bir yürünür; katının bir korkuluğunun çizgisi
  küpeştenin yarısı ve 5 cm içindeyse o nokta tutulmuştur. Tutulmayan boy 0,10 m ve fazlaysa “Döşeme boşluğunun 4.47
  m'lik kenarında korkuluk yok — PAİY 38/1”. Kendiliğinden düzeltmesi yok: düzeltme tek komut adımıdır ve korkuluğun
  EK-4 katmanını (MM-KORK-CUBUK) açmak aracın işidir; onarım metni Boşluktan'ı söyler.
- **Korkuluksuz balkon ve teras (`paiy_railing_missing`, 38/1 “balkon, teras”):** kenarı (korkuluk ya da parapet)
  olmayan balkonun, terasın serbest kenarlarında tutulmayan boy; düzeltmesi “Serbest kenarlarına korkuluk koy”: döşemenin
  kenarı korkuluk (`cad.slabs.edit`). Bunun için bulgunun düzeltmesi artık komutunu taşır (`Fix.command`; yoksa
  `cad.arch.edit`).
- Döşemenin kendi kenarı (korkuluk ya da parapet) bütün serbest kenarlarını, boşluklarınınkileri de tutar: o döşemede
  bulgu aranmaz.

### 5. Asansör (3. bölüm)

Asansör yeni bir nesne türü değil, KentOS kitaplığının değiştirilebilir bloğudur (`scripts/blocks/library.py`, 39.
blok; Archicad'de ve Revit'te de kitaplık nesnesi). Kuyu duvarlarla ve “Asansör” adlı boşluk mahalle çizilir (ADR
8034); blok kuyunun içini çizer:

- **Çizim:** kabin çift çizgi (3 cm kabin duvarı) ve içinde çapraz; arkada karşı ağırlık; önde iki sürme kanat (kat
  ve kabin kapısı) ve açılma oku. Taban noktası kuyunun kapı duvarındaki sol iç köşesi, kuyu +y yönünde. Görünüş
  Kabin (varsayılan) ya da Kuyu ile: duvarsız çizimde kuyunun iç yüzü de çizilir.
- **Parametreler PAİY'nin ölçtükleri:** Kabin eni (0,80–2,60 m) ve derinliği (1,00–3,00 m) iç ölçü, Kapı genişliği net
  geçiş (0,70–1,40 m), 5 cm adımla. Varsayılan 1,20 × 1,50 m ve 0,90 m: 34/2'nin en azı. Tip tablosu: Konut 120 × 150,
  Sedye 120 × 210 (34/5: 2,52 m², kapı 1,10 m), TSE Tip 2 110 × 140 (34/2'nin TSE istisnası, idarenin yetkisinde).
- **Kapı kabinin sol iç kenarına bağlı:** dinamik bloğun tutamağı kendi tanım noktasında durur, başka bir parametrenin
  kaydırdığı nesneyle kaymaz (ADR 0319); kapı kabin eniyle kaysaydı tutamağı kapının ucundan ayrılırdı. Karşı ağırlık
  kabinle genişler, ortada kalır.
- **Katman:** EK-2 Tablo 2.3'ün Asansör'ü (ASAN): MM-ASAN-GENEL. Donatı araması (ADR 8024) “asansör” ve “lift” ile
  bulur.
- **Denetim › Mevzuat:** blok asansördür (34/1'in asansörü var, 31/2-a'nın rıhtı 0,18 m). Kuyu net ölçüsüyle, içindeki
  blok kabini ve kapısıyla ölçülür; kabini taşıyamayan kuyuda bloğun yazdığı ölçü bir şey değiştirmez. Kuyusuz blok
  kendi başına bir asansördür. Tek asansör ya da yarısı (34/2) tutmazsa “Asansör kabininin dar kenarı 1.10 m < 1.20 m
  — PAİY 34/2”, “Asansör kabininin alanı 1.54 m² < 1.80 m²”, “Asansör kapısının net geçiş genişliği 0.80 m < 0.90 m”
  (`paiy_elevator_small`). Düzeltme bloğun parametreleridir (`cad.blocks.parameters`; `Fix.operation` değerleri
  taşır): “Kabini 1,20 × 1,50 m yap” dar kenarı 1,20'ye, öbürünü alanın istediğine kuralın 5 cm adımıyla yukarı
  çıkarır; “Kapıyı 0,90 m yap”.
- **Sedye asansörü (34/5, `paiy_elevator_stretcher`):** 10 ve daha fazla katlı binada hiçbir asansör dar kenar
  1,20 m, 2,52 m² ve kapı 1,10 m değilse “Sedye asansörü yok (10 kat): …”; en büyük kabinde söylenir, düzeltmesi
  “Kabini 1,20 × 2,10 m, kapıyı 1,10 m yap”. Katlar ±0,00'dan sayılır, planın kat adedi gibi: 34/1 bodrumu açıkça
  sayar, 34/5 saymaz.
- Kalanlar: 34/4'ün iki asansörü (umumi bina, 10 kat, 20 konut), bloğun kesit ve görünüşte kabini, kapı açıklığının
  duvarı kendiliğinden kesmesi.

### 6. Otopark yeri (4. bölüm)

Otopark yeri de kitaplığın değiştirilebilir bloğudur (40.): bir yer, önündeki manevra alanı ayrı (5/1-h-8 “manevra
alanı hariç”). Taban noktası koridor tarafındaki sol köşe, yer +y yönünde.

- **Ölçü ve tür:** En (2,00–4,00 m) ve Boy (4,00–7,00 m), 5 cm adımla; Tür görünürlük durumu: Binek, Engelli
  (tekerlekli sandalye işareti), Şarjlı (şimşek), Kamyonet. Ölçü tablosu türü ve en az ölçüsünü birlikte verir (Binek
  240 × 490, Engelli 350 × 490, Şarjlı 240 × 490, Kamyonet 300 × 600) ve ilk listedir: Blok ekle'nin Varyant'ı (V) onu
  çevirir. Numara özniteliği NO. İşaretler En ile kaymaz (tutamak yalnız kendi parametresinin nesnelerini taşır):
  engelli işareti 3,50 m'lik yerin, şimşek 2,40 m'lik yerin ortasındadır.
- **Katman:** yönetmeliğin kendi sözü “otoparkların tefrişi”: MM-TEFR-OTOPARK. Donatı araması “otopark”, “park yeri”.
- **Otopark hesabı:** çizilen yerler çizimin yerleridir (4/1-ç gösterilmesini ister); hiç yer çizilmemişse eskisi gibi
  Otopark ya da Garaj mahallerinin net alanı ÷ 20 m². Çizelgede “Çizilen otopark yeri”, umumi binada “Engelli park yeri
  (çizimde n)”, 20 ve üstünde “Elektrikli araç şarj yeri (çizimde n)”; eksik satır uyarı renginde. Yer çizilmemiş
  çizimin çizelgesi ve bulguları değişmedi.
- **Umumi bina:** programda konut bölümü yok ve büro ya da dükkân bölümü var. Konutlu karma bina umumi sayılmaz:
  dükkânlı apartmanı umumi saymak yanlış uyarı verirdi; idare başka okursa engelli yeri elle eklenir.
- **Denetim › Mevzuat (`parking_audit`):** kısa kenar 2,40 m'den (engellide 3,50, kamyonette 3,00), uzun kenar
  4,90 m'den (kamyonette 6,00) küçükse `parking_space_small` (5/1-h-9, kamyonette h-8), düzeltmesi “Yeri 2,40 × 4,90 m
  yap” (`cad.blocks.parameters`). Otopark ya da Garaj adlı mahalde, katında ve ortası içinde kalan yer sayısı net alan
  ÷ 20 m²'yi aşarsa `parking_area_short` (4/1-ç). Umumi binada engelli yeri her 20 yerde 1 ve en az 1 değilse
  `parking_disabled_short` (4/1-ı; kesir bir üste, 8/4 gibi); düzeltmesi yok, geniş yer sırayı kaydırır. Zorunlu yer 20
  ve üstündeyse şarjlı yer %5 ve en az 1 değilse `parking_charging_short` (5/1-j), düzeltmesi “Bu yeri şarjlı yap”
  (bir binek yerinin Tür'ü). `parking_short`'un iletisi yer çizildiyse “çizimde 19 otopark yeri” der.
- `block_values`: bir düzeltmenin blok parametresi için kuralın verdiği en küçük değer, asansör ve otopark ortak.
- Kalanlar: bir sıra yeri tek seferde çizen araç, manevra yolu genişliği (5/1-h-7: 6,00/6,50 m, 15 araçtan azda tek
  yönlü 4,90 m), rampa (5/1-h-5, 6), engelli yerinin girişe ve asansöre yakınlığı.

## Doğrulama

- Bağımsız başvuru (`scripts/fixtures/arch_cases.py`, kesirlerle tam): 6 balansman durumu (sola dönen 1,00 m L, sağa
  dönen 1,20 m L, U, üç ret); çekirdek testi her rıhtın iki ucunu, her dengelenen basamağın iç ve orta genişliğini,
  reddi ve en az sayıyı 1e-9 içinde karşılaştırır.
- KCAD şema 241 örneği (`stair-winders.kcad`), şema 240'ta ret, şema 242'nin reddi; KCAD testleri 250; başvuru
  `--check` 1790 dosya. Ortak komut durumları: `cad.stairs.create` (2 geçerli: L ve U; 7 ret: düz, döner, sıfır,
  sahanlıkla birlikte, dönüşün kaybından kısa, kola sığmayan, iki dönüşte çakışan), `cad.arch.edit` (balansman verilir,
  sahanlık kalkar, null sahanlığa döner; kola sığmayan ve düz merdivende ret). Etkileşim: Merdiven aracında A 5 ile
  1,20 m'lik L'nin tasarım satırı PAİY uyarısı, A 7 ile "(PAİY 31/2-c: en az 0,10 m)", düz yolda balansman yazılmaz,
  A "yok" sahanlığa döner. Masaüstü 66 (seçenek harfleri, Öznitelikler, Mevzuat).
- Gerçek pencere (Xvfb 1920 × 1080, gerçek fare ve klavye; `fixtures/interaction/real/merdiven-balansman.txt`):
  görünüş binasının sağına 14,5,1 → 17,1 → 17,4 yazılarak 1,20 m'lik L merdiven, Balansman 5: tasarım satırı "en dar
  iç basamak 0.062 m: PAİY 31/2-c'nin 0,10 m'sinden dar"; planda köşede dönen basamaklar; Denetim › Mevzuat'ta
  "Balansmanlı merdivenin basamağı dar uçta 0.061 m < 0.10 m — PAİY 31/2-c" (29 bulgu, 5 düzeltilebilir); Düzelt
  "Balansmanı 7 basamak yap", bulgu düştü (28, 4). Balansman 7 ile çizilen merdivenin Öznitelikler'i: Dönüş
  Balansmanlı, Balansman basamağı 7, En dar iç basamak 0.119 m, Denetim Uyuyor. Eski ikili aynı yolu dönüşte
  sahanlıkla çiziyor.
- Kesit (§2): çekirdek testi düz basamağın dikdörtgenini ve 1,00 m'lik L'nin köşesindeki iki dengelenmiş basamağın
  rıht uçlarıyla birlikte iç (2,3; 0,5) ve dış (3,3; −0,5) köşeyi tuttuğunu, saat yönünün tersine döndüğünü denetler.
  Kesit modeli testi: 1,00 m'lik L, 5 balansman, 18 rıht × 0,17 m: 2 kol ve 10 dengelenmiş basamak (6. ile 15. rıhtların
  üstü, üstleri 1,02 … 2,55 m), sahanlık yok, köşedeki ikisi iç ve dış köşeyi tutuyor, kalınlık olmadan hepsi döşemede.
- Gerçek pencere (`fixtures/interaction/real/merdiven-balansman-kesit.txt`): 1,20 m'lik L, Balansman 7, 18 rıht ×
  0,167 m; A-A kesiti ilk kolu ve dönüşü 0,10 m yürüme çizgisinin dışından kesiyor. Eski ikili: ilk kolun 9 basamağı
  yürüme çizgisinin köşesinde (x = 17,00) bitiyor, dönüşün dış 0,60 m'si boş, ikinci kol +1,50'de havada. Yeni ikili:
  gövde merdivenin dış kenarına (17,60) kadar dolu, dengelenmiş basamaklar tek tek kesiliyor, ardındakilerin rıhtları
  görünüyor; ±0,00'dan son basamağın +2,83'üne kesintisiz.
- Korkuluk (§3): sözleşme testi (uzunluk, aralık, alan retleri, varsayılanların yazılmadığı JSON); çekirdek testi
  (açık korkuluğun iki yüzü ±0,025 m, 1/200'de tek çizgi, kapalı korkuluğun delikli halkası); KCAD şema 242 örneği
  (`railing-objects.kcad`: açık dikey çubuklu ve kapalı cam), şema 241'de `unknown_kind`, şema 243'ün reddi; KCAD
  testleri 250, başvuru `--check` 1792 dosya. Ortak komut durumları: `cad.railings.create` 35 (bağımsız başvuru
  `scripts/fixtures/arch_command_cases.py`), `cad.arch.edit` 75 → 83 (korkuluğun alanları, null aralık, retler,
  türünde olmayan alan). Etkileşim: Korkuluk aracı 4 test (Çiz ve kapatma, katmanlar MM-KORK-CUBUK/CAM, Boşluktan
  serbest çevre, duvarlı kenarlar ve merdiven çıkışı: üç parça tek adım, alçak korkuluğun uyarısı).
- Gerçek pencere (`fixtures/interaction/real/korkuluk-galeri.txt`): görünüş binasının 1. katında merdiven boşluğu
  (2,85 × 3,94 m), Boşluktan ile tek tıklama: batı ve doğu kenar duvar yüzünde, korkuluk yok; kuzey kenarı duvardan
  duvara 2,850 m; güney kenarı batı duvarından merdivenin çıktığı yere (6,20 m'de biter, merdivenin 1,20 m'lik çıkışı
  açık; üstüne gelince kart “Uzunluk 1.625 m”, “Alan 0.08 m²”), 2,5 cm'lik artık yazılmadı. Öznitelikler: Yükseklik 1.100, Dolgu Dikey çubuk, Çubuk aralığı 0.100,
  Küpeşte eni 0.050, Çizgisi Açık, Uzunluk 2.850, Denetim Uyuyor. Eski ikilide menü araması “Korkuluk menülerde yok”
  diyor.
- Korkuluk kesitte (§3): kesit modeli testi (bölüm binası, 1. kat, 2,40 m'lik üç korkuluk): çubuklu olan küpeşte katısı
  +4,05–+4,10 ve 20 çubuk (+3,00'ten +4,05'e, ayak izsiz katının çizgileri), camlı olan küpeşte ve camın ayağı ile iki
  ucu, dolu olan +3,00–+4,10 tek katı; A-A'nın bloğunda üç katman. Gerçek pencere (`korkuluk-kesit.txt`,
  `korkuluk-kesit-a.txt`): görünüş binasında Boşluktan'dan sonra B-B'de iki korkuluğun küpeştesi kesilen küçük birer
  kare, çubukları +3,00'e inen tek çizgi; A-A'da merdiven holünde küpeşte bandı ve 0,12 m'de bir çubuk, arkadaki
  pencere bütün. Önceki ikilide (korkuluk var, kesitte yok) iki kesitte de korkuluk görünmüyor.
- Korkuluk denetimi (§4): bölüm binasında (`railing_audit` testi) merdiven boşluğu “Döşeme boşluğunun 4.47 m'lik
  kenarında korkuluk yok” (2,850 + 1,625 m; boşluğun kendi hesabı 4,475 m), Boşluktan'ın iki korkuluğuyla düştü;
  0,90 m'ye alçaltılıp aralığı 0,12 m yapılan korkuluğun iki bulgusu `cad.arch.edit` düzeltmeleriyle düştü; duvarın
  önündeki 3,00 × 1,50 m kenarsız balkon “Balkonun 6.00 m'lik serbest kenarında korkuluk yok”, düzeltmesi
  `cad.slabs.edit` ile düştü. Mevzuat ortak durumları değişmedi. Gerçek pencere (`korkuluk-denetim.txt`): görünüş
  binasının 1. katında Mevzuat 10 bulgu, 8.'si boşluk; Boşluktan'dan sonra Denetle 9 bulgu, boşluğunki yok. Önceki
  ikilide aynı çizimde bu bulgu yoktu.
- Asansör (§5): kitaplık 38 → 39 blok (`library.py --check`); bloğun esneme kuralları ayrıca çizildi
  (`.run/gui-evidence/mimari-m45/asansor/varyantlar.png`: sedyede karşı ağırlık kabinle geri kayar, 2,00 m'lik kabinde
  genişleyip ortada kalır). `elevator_audit` testi (bölüm binası, 1. kat): varsayılan blokta bulgu yok; Tip TSE Tip 2
  110 × 140 ve 0,80 m kapı “dar kenarı 1.10 m < 1.20 m”, “alanı 1.54 m² < 1.80 m²”, “kapısının net geçiş genişliği
  0.80 m < 0.90 m”; “Kabini 1,20 × 1,50 m yap” girdisi `{"Kabin eni": 1.2, "Kabin derinliği": 1.5}`, iki düzeltmeyle
  bulgu kalmaz; 1,60 × 1,10 m'lik geniş kabinde derinlik 1,20'ye çıkar; aynı katta ikinci küçük asansör yarı kuralıyla
  söylenmez. 1,60 × 2,10 m kuyu bulgusuz, içindeki 1,10 × 1,40 m blok iki bulguyla (kuyu bulgusu yok). 10 katta
  “Sedye asansörü yok (10 kat)”, düzeltmesi “Kabini 1,20 × 2,10 m, kapıyı 1,10 m yap”, Tip Sedye 120 × 210 da yeter.
  EK-4 adı MM-ASAN-GENEL; donatı araması “asansör” ve “lift”.
- Gerçek pencere (`fixtures/interaction/real/asansor-denetim.txt`, görünüş binası, 1. kat): önceki ikilide menü
  araması “asansor menülerde yok”, Mevzuat 26 bulgu ve 6.'sı “Merdivenin rıhtı 0.167 m > 0.16 m (asansörsüz bina)”.
  Yeni ikilide arama “Asansör — Donatı › Asansör › Yolcu”, Enter ile Blok ekle: katman MM-ASAN-GENEL, Kabin eni 1.200,
  Kabin derinliği 1.500, Kapı genişliği 0.900; 5.825,6.7 yazılınca holün kuzeyinde (nesne 81 → 82). Denetle: rıht
  bulgusu yok; sayı yine 26, çünkü kapılar grubu yeni bir doğru bulgu verdi: “K12 kapısının kanadı açılırken bir
  donatıya çarpıyor” (mutfak kapısı asansörün köşesini süpürüyor; başsız denetimle aynı). Öznitelikler › Katalog ›
  Tip: TSE Tip 2 110 × 140 → 28 bulgu, 8. ve 9. “Asansör kabininin dar kenarı 1.10 m < 1.20 m — PAİY 34/2”, “…alanı
  1.54 m² < 1.80 m² — PAİY 34/2”; Düzelt “Düzeltildi: Kabini 1,20 × 1,50 m yap”, 26 bulgu, Tip yine Konut 120 × 150.
- Otopark yeri (§6): kitaplık 39 → 40 blok; dört türün çizimi `.run/gui-evidence/mimari-m45/otopark/kentos-otopark-
  turleri.png`. `parking_audit` testi: iki 100 m²'lik büro (Ek-1: 4 yer, umumi bina) ve 15 × 10 m otopark (alanla 7);
  yer çizilmeden bulgu yok. Dört yer (biri 2,30 m): “Otopark yerinin kısa kenarı 2.30 m < 2.40 m — Otopark Yön.
  5/1-h-9” (düzeltmesi `{"En": 2.4}`) ve “Engelli otopark yeri 0 < 1 (4 yerde her 20'de 1, en az 1; umumi bina)”;
  Ölçü › Engelli 350 × 490 ile bulgu kalmaz, 3,00'e daraltılan engelli yer söylenir ve düzelir; sekiz yer “Otopark:
  8 otopark yeri, 150.00 m² ancak 7 yere yeter”; sıranın ortasındaki yer Engelli yapılınca 2,40 m söylenir ama
  düzeltmesi yok (komşusuna binerdi); çizelgede “Çizilen otopark yeri 8”, “Engelli park yeri (çizimde 2)”. Yirmi
  150 m²'lik dairede 20 yer: “Elektrikli araç şarj yeri 0 < 1 (20 zorunlu yerin %5'i, en az 1) — Otopark Yön. 5/1-j
  (25.3.2021)”, “Bu yeri şarjlı yap” ile düşer; bir yer silinince “gereken 20 yer, çizimde 19 otopark yeri”.
- Gerçek pencere (`fixtures/interaction/real/otopark-yeri.txt`; çizim `fixtures/arch/v1/parking-building.kcad`,
  `scripts/fixtures/parking_building.py` ile bağımsız yazıcıdan: 10 daire, 1 dükkân, 10 × 10 m otopark): önceki ikilide
  arama “otopark yeri menülerde yok”, Mevzuat'ın 11.'si “Otopark: gereken 7 yer, çizimde 5 (100.00 m² ÷ 20 m²)”.
  Yeni ikilide arama “Otopark yeri — Donatı › Otopark › Park yeri” (“otopark” tek başına önce Otopark hesabı'nı
  verir), Blok ekle katmanı MM-TEFR-OTOPARK, her yer numarasını sorar; altı yer yazılınca “gereken 7 yer, çizimde 6
  otopark yeri” ve “6 otopark yeri, 100.00 m² ancak 5 yere yeter (araç başına manevra dahil 20 m²) — Otopark Yön.
  4/1-ç”; P6'nın Tür'ü Engelli: planda işaret, Ölçü “—”, “Engelli otopark yerinin kısa kenarı 2.40 m < 3.50 m”;
  Düzelt “Yeri 3,50 × 4,90 m yap”, Ölçü Engelli 350 × 490, bulgu yok. M36'nın eski deneme çizimi şema 211 ile
  kaydedilmiş (Mimari şemaları 217–237'ye taşınmadan önce; bina programı bugün 221'den sonra) ve bugünkü ikili onu
  açmıyor; bu yüzden çizim depoya bağımsız yazıcıyla yeniden yazıldı (şema 223).
- Ortak komut durumu `cad.blocks.edit` › “Blokları al: KentOS'un temel kitaplığı adıyla” kitaplığın bütün kimliklerini
  bekler (`scripts/fixtures/blocks_command_cases.py` kitaplık dosyasından üretir): asansör adımında 38'de kalmıştı, 40
  kimlikle yeniden üretildi; blok, revizyon ve denetim durumları (349) geçiyor.
