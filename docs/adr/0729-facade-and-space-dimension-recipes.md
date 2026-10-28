# ADR 0729: Cephe, duvar grubu ve mahal ölçü dizileri; kalıcı kaynak ve üyelik

- **Durum:** kabul edildi (28 Ekim 2026); ARCH-DIM-01'in kalan bütün alt maddeleri.
- **Bağlam:** `plan/architecture.md` ARCH-DIM-01; ADR 0695 (kalıcı tek duvar dizisi), ADR 0700 (kenar ve bütün kenarlar),
  ADR 0711 (kesen çizgiyle iç zincir), ADR 0694 (açıklığın yerleri), ADR 0725 (aksın yerleri), ADR 0342 (mahal).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0695/0700'ün ölçü dizisi **tek bir duvarın tek bir düz kenarını** ölçer: kenarın kendi yüz köşeleri arasında açıklıklar ve
toplam. Dört ayrı duvarla çizilmiş bir binada güney duvarının dizisi bu yüzden duvarın kendi yüzünü (eksenden eksene, 10,000)
ölçer, köşedeki dış yüze (10,250) inmez; bölmeler, akslar ve duvar kalınlıkları hiç ölçülmez; mahal seçilemez. Planın açık
maddeleri: cephe/duvar grubu/mahal kaynağı ve kalıcı tarifi (seçilen kimlikler, kat, yüz/merkez/uç politikası), kaynak
değişince üyeliğin güncellenmesi (sonradan kapı, açıklık kaldırma, duvar bölme/rehost, ters yön; aynı kaynak çiftinin
kimliği, kullanıcının taşıdığı sıra yeri), sıranın önizlemesinde kaynak/yüz/sayı ve yazı sığması, desteklenen geometrinin
açıkça daraltılması ve atomik ret, Python/MCP yaşam döngüsü.

Araştırma (üreticilerin güncel belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Automatic Exterior Dimensioning](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-49.htm):
  seçilen duvarlar; yön bir elemanın kenarına tıklanarak ya da iki tıklamayla; dört zincire kadar: **Overall Dimension**,
  **Dimension External Geometry** (cephenin çıkıntıları), **Dimension Structures** (duvar kalınlığı), **Dimension Openings**
  (Opening Center ya da Opening Endpoints); **Dimension Walls by: Outer Faces / Core Faces**; zincirler arası uzaklık; **Place
  Dimensions on Four Sides**; gerekmeyen zincir (açıklık yokken açıklık zinciri gibi) atlanır.
- [Revit 2026 — Add Aligned Dimensions to Walls Automatically](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-C26D95E4-F272-4C67-8C1D-BC26C0493B75.htm):
  Entire Walls; seçenekler **Openings** (Centers / Widths), **Intersecting Walls**, **Intersecting Grids** (duvara dik akslar;
  başka bir noktayla çakışan aks ikinci kez ölçülmez).
- [Vectorworks 2026 — Dimensioning exterior walls](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Dimensions/Dimensioning_exterior_walls.htm):
  **Dimension T-joins in exterior walls** (dış duvara saplanan iç duvarlar), duvar ya da çekirdek bileşen, dış yüzler ya da
  eksen, açıklıklarda merkez ya da kenarlar; küçük ölçüleri işaretleme; ilişkili ölçüler duvarla güncellenir.

KentOS kararı: Archicad'in akışı (kaynaklar, yön, yer, sıralar, dört yan), Revit'in kesişen duvar ve aks sıraları,
Vectorworks'ün saplanan iç duvarları; sıralar üyeleri sıradan, seçilebilir Doğrusal ölçüler olarak tutan ADR 0695 tarifinde
kalır. Eksene (duvar ortası) ölçü bu dilimde yoktur: planın istediği iç yüz, dış yüz, çekirdek ve açıklık merkezi/uçlarıdır;
duvarın ekseni kalıcı bir alt nesne yeri değildir (ADR 0728 yalnız geçici ölçüde kullanır).

## Karar

### 1. Model: tarifin `span`'ı

`DimensionSet` isteğe bağlı `span` alır; yoksa tarif ADR 0695/0700'ün tek duvar kenarı dizisidir, değişmeden.

- `walls`: cephe (duvar grubu) tarifinde `wall`'dan sonraki kaynak duvarlar, kullanıcının sırasıyla. `wall` ilk kaynaktır.
- `space`: mahal tarifi; `wall` o zaman **mahalin** kimliğidir, `walls` boştur.
- `direction`: ölçünün doğrultusu, birim vektör (dünya düzleminde).
- `reference`: `faces` (dış yüzler: duvarın ilk ve son çizgisi) ya da `core` (çekirdek: en güçlü önceliğin bileşen bandının iki
  sınırı; bileşensiz ya da hepsi aynı öncelikte duvarda yüzler).
- `level`: kaynakların ortak mimari katı (hepsi katsızsa yok). Saplanan duvarlar ve akslar bu kattan (ya da katsızdan).
- Sıralar `rows` yeni iki değer alır: `walls` (duvarlar: kalınlıklar ve saplanan/kesişen duvarlar) ve `grids` (akslar). Bunlar
  yalnız `span`'lı tarifte geçerlidir; `span`'lı tarifte en çok dört sıra, yinelenmeden.
- `face` ve `edge` `span`'lı tarifte kullanılmaz (`face` varsayılan, `edge` yok).
- Kopma nedenine `mixedSources` eklenir: kaynaklar sonradan farklı katlara düştü.
- Dosya: `span`'lı bir tarif KCAD **şema 128** ister; yoksa yazıcı eski şemayı seçer, eski dosyaların baytları değişmez.

### 2. Planlayıcı: `ops::dimension_span` (ortak çekirdek, saf)

Gösterim: `d` doğrultu, `n = (−d.y, d.x)`; bir noktanın `t = p·d` (boyunca) ve `o = p·n` (enine) değeri. Yan `s`: `at`'ın
`o`'su kaynakların gövdelerinin `o` aralığının ortasından büyük ya da eşitse `+1`, değilse `−1`; sıralar `s` yönünde dışarı.

- **Kenarlar:** bir yolun her kenarı, büküm sıfırsa ve `|u·n| ≤ 1e−6` ise *paralel*, `|u·d| ≤ 1e−6` ise *dik*, değilse
  *eğik*. Büküm sıfır değilse *yay*.
- **Duvarın çizgileri** `geom::wall::lines`'tır (0 sol yüz, son sağ yüz); referans çizgileri §1'in `reference`'ı. Bir çizginin
  köşeleri yolun köşeleriyle birebir olmalıdır (pahlanmış köşe desteklenmez).
- **Dik kenar noktası:** dik bir kenarın her referans çizgisi bir noktadır; yeri çizginin o kenarın iki ucundaki köşelerinden
  `s` yönünde ilerdekidir (`s·o` büyük olan; eşitse küçük köşe numarası), başvurusu `Vertex{part: çizgi, ring: 0, index: köşe}`.
- **Serbest uç:** kaynak açık duvarın yol ucu, son kenarı paralelse ve uç **başka bir kaynağa** değmiyorsa (başka kaynağın
  yol ucu `JOIN_TOLERANCE` içinde, ya da uç onun gövde bandının içinde) bir noktadır: referans çizgilerinin o köşelerinden
  `s` yönünde ilerdeki. Kaynak olmayan duvara değen uç da serbesttir: seçilmeyen duvar ölçülmez.
- **Saplanan duvar:** kaynak olmayan, kaynağın katındaki, görünür ve çözülmüş katmandaki duvar; yol uçlarından biri bir
  kaynağın yol ucuna `JOIN_TOLERANCE` içinde ya da gövde bandının içinde, ya da yolu bir kaynağın yolunu kesiyor. Bütün
  kenarları paralel ya da dikse dik kenarlarının referans çizgileri `walls` sırasına katılır; eğik ya da yay kenarı varsa
  ölçülmez ve `skipped`'da söylenir.
- **Açıklık:** kaynak bir duvarın paralel kenarındaki (pervazlarının ortası o kenarın yüzleri arasında) açıklık; baca hiç,
  niş yalnız `s` yanındaki yüzdeyse. Yakın yüzün pervaz köşeleri (`s·o` büyük olan çift: 0,2 ya da 1,3; ADR 0694) ya da merkez.
- **Aks:** düz (büküm yok) aks, katında ya da katsız, görünür katmanda, doğrultusu `d`'ye dik, doğrusu kaynakların `o`
  aralığını kesen ve `t`'si en dıştaki iki noktanın arasında kalan. Yeri iki ucundan `s` yönünde ilerdeki (`Start`/`End`).
- **Mahal:** mahalin parçalarının halkaları (`Vertex{part, ring, index}`); yay ya da eğik kenar desteklenmez. Dış halkaların dik
  kenarları dış noktalardır; deliklerin (kolon, boşluk) dik kenarları yalnız `walls` sırasına girer. Açıklık: pervaz
  çiftlerinden biri mahalin `s`'e bakan (dışa normali `s` yönünde) paralel bir kenarının üstünde olan açıklık.
- **Görünürlük (yalnız cephenin açıklık sırası):** bakan `s` yönünde sonsuzdadır. Bir nokta, ait olduğu kenarın bandı dışındaki
  bir kaynak duvar bandı `t`'sini kesin içinde tutuyor ve `s` yönünde ondan ileri uzanıyorsa (`> 1e−9`) gizlidir. Böylece
  kuzey duvarın pencereleri güney zincirine, iç yüzler dış zincire girmez; cephenin girinti çıkıntısının yüzleri girer.

Sıralar (kaynaktan dışarı, verilen sırayla):

| Sıra | Noktalar |
|---|---|
| `openings` | cephede: görünür dik kenar noktaları ve serbest uçlar ile görünür açıklıklar; mahalde: dış halkanın dik kenar noktaları ve `s`'e bakan kenarın açıklıkları |
| `walls` | bütün dik kenar noktaları (kaynaklar, saplanan duvarlar; mahalde delikler de) ve serbest uçlar |
| `grids` | en dıştaki iki nokta ve akslar |
| `overall` | en dıştaki iki nokta (kaynakların dik kenar noktaları ve serbest uçları; mahalde dış halkalar) |

Her sıranın noktaları `t`'ye göre sıralanır (eşitte duvar/mahal, açıklık, aks; sonra kaynak sırası ve yer); `|Δt| ≤ 1e−9`
olanlardan ilki kalır. İki noktası olmayan sıra yazılmaz. ADR 0695'in eki gibi, önceki bir sırada çizilen aynı iki yeri ölçen
ölçü tekrar çizilmez. `span`'lı tarifte **boş kalan sıra yer kaplamaz**: sonraki sıra bir aralık içeri gelir. Her ölçü
Doğrusal, açısı `d`'nin; sıra çizgisi dünyada yerindedir (ADR 0695 §11), elle taşınan sıra yeri korunur.

### 3. Daraltılmış geometri ve atomik ret

`cad.dimensionsets.create` yazmadan önce (planda da) `invalid_dimension_set` ile reddeder: kaynak yok ya da duvar değil;
kaynakların bir kenarı eğik ya da yay (“Kaynağın (kimlik) bir kenarı ölçü doğrultusuna ne paralel ne dik …”; hatanın
`uids`'i kaynağı adlandırır); kaynaklar farklı katlarda; mahal yok ya da yay/eğik kenarlı; kaynağın bir açıklığı
yerleşemiyor (`broken`; açıklık kutunun dışında olsa da okunur, dışarıda bırakılmaz); doğrultu sıfır ya da sonlu değil;
`walls`/`grids` sırası `span`'sız tarifte; `edge`, `direction` ya da `reference` yanlış türde tarifte; yinelenen kaynak;
mahalle birlikte duvar; hiçbir sırada ölçülecek aralık yok. Kısmen yazılmış dizi kalmaz: bütün diziler tek geri alma
adımıdır. Yazıldıktan sonra kaynak eğikleşir ya da katlar karışırsa dizi `unsupportedWall`/`mixedSources` ile kopar, son
geometrisi uyarı üçgeniyle kalır (sessiz yanlış ölçü yok).

### 4. Üyelik ve izleme

- Tarif, adımda kaynaklarına, kaynaklarındaki açıklıklara (öncesi ve sonrası: rehost), etki kutusuna (kaynakların ve
  mahalin gövdesinin kutusu, en geniş kaynağın kalınlığı ve 1 m payla) değen duvar, aks, kolon ve mahale dokunulunca yeniden
  planlanır: sonradan kapı ya da bölme eklemek üye ekler, kaldırmak çıkarır.
- **Kimlik:** aynı sıra ve aynı iki kaynak yeri aynı ölçü kimliğini korur (ADR 0695 §5); yerin adı değişse de (ters
  çevrilen duvarın açıklığının pervaz adları yer değiştirir) aynı sıra, aynı iki kaynak nesne ve nanometreye yuvarlanmış aynı
  iki nokta olan ölçü kimliğini korur.
- **Bölme:** adımda kaynak bir duvarın eski yolunun üstünde (doğrusal, uçları eski yolun içinde) yeni bir duvar eklenirse
  kaynaklara katılır; parçaların ortak ucu iki kaynağın değdiği uçtur, ölçüler ve kimlikleri değişmez. Tek duvar tarifinin
  duvarı bölünürse tarif cephe tarifine döner (kaynaklar iki parça, doğrultu kenarın, yer aynı) ve cephe kuralıyla ölçer:
  köşelerinde duvar varsa ölçü onların dış yüzlerine iner (tek duvarın kendi yüzündeki 10,000, köşe duvarlarıyla 10,250
  olur); üyeler yeniden üretilir.
- **Birleştirme:** kaynak bir duvar silinip aynı adımda başka bir duvarın yeni yolu onun eski yolunu kapsıyorsa onun yerini
  alır (tek duvar tarifinde de: tarif cephe tarifine döner).
- **Silme:** silinen kaynak kaynaklardan çıkar; kaynak kalmazsa (ya da mahal silinirse) dizi `hostGone` ile kopar.
- **Ters yön:** cephe ve mahal tarifi yüzü yandan seçtiği için etkilenmez. Tek duvar tarifinin duvarı ters çevrilirse `face`
  sol ↔ sağ ve `edge` yeni numarasıyla izler; aynı fiziksel yüz ölçülür.

### 5. Komutlar

- `cad.dimensionsets.create` v1'e yeni isteğe bağlı alanlar: `walls` (birden çok kaynak duvar: cephe), `space` (mahal),
  `direction` (yoksa ilk kaynağın en uzun düz kenarının, mahalde dış halkanın en uzun düz kenarının doğrultusu), `reference`.
  `walls` ya da `space` verilince tarif `span`'lıdır; `rows` varsayılanı her tarifte olduğu gibi `openings, overall`
  (araç son seçilen sıraları hatırlar). Doğrultu okunur yöne çevrilir: sola (ya da düz aşağı) bakan doğrultu tersine döner,
  sıralar soldan sağa ya da aşağıdan yukarı yazılır; ölçülen değerler değişmez. `around` `span`'lı tarifte **dört yan**
  demektir: `d` ve `n` doğrultusunda, kaynakların dört yanına birer dizi, her biri en dıştaki gövdeden `at`'ın uzaklığında;
  tek adımda.
- `cad.dimensionsets.edit`: `Set`'e `reference` (yalnız `span`'lı tarifte; `face` `span`'lı tarifte ret); yeni
  `Sources { walls }` cephe tarifinin kaynaklarını değiştirir (aynı denetimlerle; mahal tarifinde ret). `rowOffset`
  `span`'lı tarifte sıranın kaynakların en dış yüzünden (sıraların yanında) uzaklığıdır, dışa doğru artı; sıra boyundaki
  yeri korunur. Düzenlemenin yazması da planın uyarılarını söyler.
- `cad.dimensionsets.query`: `wall` süzgeci cephe tarifinin bütün kaynaklarını ve mahal tarifinin mahalini de eşler.
- Plan çıktısı `DimensionSetsPlan`'a `rows` (her sıra: `row`, nokta sayısı `points`, kaynak sayısı `sources`, çizilen
  ölçü sayısı `dimensions`, ölçülen `reference`), `skipped` (ölçülmeyen saplanan duvarlar) ve `tight` (yazısı sığmayan
  ölçülerin `entities`'teki yerleri) eklenir. Uyarılar: `dimension_text_tight` (yazısı iki uzatma çizgisinin arasına
  sığmayan ölçüler; gizlenmez, söylenir) ve `dimension_set_wall_skipped`.

### 6. Yazı sığması ve sıra aralığı

Bir ölçünün değeri belgenin biçimiyle (`Format::dimension_value`) yazılır; genişliği çizim yazı tipinin ölçülmüş
ilerlemeleriyle (`text::paragraph::advance`) ve ölçünün yazı yüksekliğiyle. Genişlik artı bir yazı yüksekliği ölçülen
aralıktan büyükse ölçü *sığmıyor* sayılır. Sıra aralığı (`gapMm × plotScale / 1000`) yazı yüksekliğinin 1,5 katından küçükse
plan “sıralar sık” notunu taşır. Kağıt aralığı çizim ölçeğiyle bir kez çevrilir; proje birimi ya da çizim ölçeği değişmez.

### 7. Masaüstü

Duvar ölçüleri aracı (Mimari › Belgele › Ölçü ▾): **Kaynak türü**: Duvar (ADR 0695/0700), Cephe (seçili duvarlar ya da
tıklanan duvar; köşe duvarları uçlarda ölçülür), Mahal (tıklanan mahal); birden çok seçili duvar cephe, seçili mahal mahal
olarak başlar. **Doğrultu**: kaynak boyunca (en uzun düz kenarın) ya da kaynağa dik; sıralar: Açıklık sırası, Duvarlar
sırası, Akslar sırası, Toplam boy; **Ölçülen çizgi**: Dış yüzler / Çekirdek; **Yanlar**: İmleç yanı / Dört yan. Önizleme
komutun planındaki gerçek ölçüleri çizer; etikette ve panelde her sıranın ölçü ve kaynak sayısı yazar; sığmayan değerler
tehlike renginde çizilir ve sayılır, ölçülmeyen duvarlar söylenir. Öznitelikler'de dizinin kaynağı (cephe: n duvar, mahal)
ve doğrultusu gösterilir; sıraları (her biri aç/kapa), açıklık uçları/merkezleri, ölçülen çizgisi ve sıra aralığı
düzenlenir, seçili ölçünün “Sıra uzaklığı” en dış yüzden sayılır; “Kaynak duvarları göster” bütün kaynakları seçer.

### 8. Bulut

Dosya olarak saklanan bulut projesi tarifi KCAD 128 içinde olduğu gibi taşır. Veritabanında saklanan projede
`project.changes` v2 `span`'lı tarifi masaüstünün kuralıyla, aynı işlemde ve proje kilidi altında denetler
(`changes::check_span_rows`): tarifin kaynakları, bu işlemin bıraktığı hâliyle bütün akslar, tarifin kutusundaki duvar ve
açıklıklar (PostGIS `geom && st_makeenvelope(...)`) ve kaynakların barındırdığı açıklıklar okunur; katmanın görünürlüğü
masaüstünün kuralıdır (kendisi ve bütün üst grupları açık ve dondurulmamış; ağaçta olmayan gösterilir); plan ortak
modülle kurulur (`kentos_formats::dimension_spans::plan`: aynı kutu, aynı kat, aynı sıralama). Her planlanan aralığın
bir üyesi olmalı; değişmeyen kaynak çiftinin ölçü kimliği korunur; aynı aralık iki üyeyle yinelenmez; üyenin ölçüsü
planın ölçtüğünü ölçer (iki nokta, sıra uzaklığı, açı, doğrusal biçim); kaynakta artık olmayan aralık aynı işlemde
kalkar. Aksi atomik rettir, yol `project.dimensionSets[i].members…` ya da `features[w].entity.<alan>`, hiçbir şey
yazılmaz. Planlanamayan tarif (eğik ya da silinmiş kaynak, kapanmayan mahal) son geometrisini korur (kopuk dizi, §5) ve
üyeliği denetlenmez.

## Sınırlar

- Eğik ve yay duvar, eğik aks, yay kenarlı mahal desteklenmez (atomik ret); kolon cephe sırasında ölçülmez (ADR 0711'in iç
  zinciri ölçer); duvar ekseni noktası yoktur.
- Sıralar dış cephe kuralıdır: kaynakların dışında, sıraların yanındaki uzak bir bakıcının gördüğü noktalar. Sıra binanın
  içine konursa köşe duvarlarının o yandaki uçları ölçülür (değerler doğrudur, uzatma çizgileri uzun olur); iç ölçü için
  ADR 0711'in kesen çizgisi kullanılır.
- DXF/DWG'ye ölçülerin geometrisi gider, tarif ve üyelik gitmez (ADR 0695'in dışa aktarma sınırı aynen).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/dimension_spans_cases.py` (KentOS kodu çalıştırmaz; duvarın çizgileri yolun
  ötelenmesi, sıralar §2'nin kurallarından, işaretler kesirlerle kesin, sığma çizim yazı tipinin ölçülmüş
  ilerlemeleriyle) `fixtures/commands/v1/cad.dimensionsets.create.span.json`'a 45 (21 plan, 3 yazma/geri alma/yineleme,
  16 ret, 5 izleme: kapı ekleme, bölme silme, kaynak bölme, ters çevirme, kaynak silme) ve `….edit.span.json`'a 25
  (sıralar, merkezler, aralık, taşıma, sıra uzaklığı, çekirdek, kaynak çıkarma ve ekleme, mahal, eğik bölme; 9 ret)
  durum yazar. Rust koşucusu (`KENTOS_CASES_ONLY=cad.dimensionsets`, eski ADR 0695/0700 durumlarıyla 181 durum) ve
  Python'un tipli katmanı (kurulum KCAD'den açılarak) hepsini geçer; yetenek testi yeni iki uyarıyı iki komutta da bulur.
  Koşucunun yanlış beklentiyi yakaladığı ayrıca denendi. İlk koşuda bir ayrılık bulundu: kaynağın kutu dışına düşmüş
  yerleşemeyen açıklığı okunmuyordu; kural düzeltildi (açıklık kaynağındaysa her yerde okunur).
- **Testler:** çekirdek `tests/all/dimension_span.rs` (5), uygulama `tests/all/dimension_spans.rs` (9: cephe, bölme ve
  birleştirme, tek duvarın cepheye dönmesi, ters çevirme, kaynak silme, karışık kat, dört yan, mahal, KCAD örneği),
  belge `dimension_sets` (11), araç `wall_dimensions` (16; yeni dört: tıklanan duvarın cephesi, seçili duvarlar ve dört
  yan, sığmayan değerlerin tehlike rengi, mahal), masaüstü Öznitelikler (47), MCP takımı, Python
  `test_wall_dimension_sets`, `test_dimension_levels`, `test_carriage`.
- **KCAD 128:** `fixtures/kcad/v2/dimension-set-spans.{json,kcad}` ve beş bozuk örnek; bağımsız okuyucu
  (`kcad_v2_reference.py --check`, 1319 dosya) ve kodek takımı; eski dosyaların baytları aynı.
- **Kapılar:** değişen 13 crate'te `clippy --all-targets -D warnings`, ortak crate'lerin `wasm32` derlemesi, SDK ve CAD
  manifesti güncel.
- **Gerçek arayüz:** `facade_dimension_scenes.rs` (8 sahne, iki tema, 1440×900 ve 1100×650; `.run/shots/arac-cephe-olcu-*`,
  `arac-mahal-olcu-*`): güney cephesi dört sırayla (açıklıklar 1,175 / 0,900 / 4,100 / 0,900 / 3,175; duvarlar
  0,250 / 3,800 / 0,150 / 5,800 / 0,250; akslar 0,125 / 4,000 / 6,000 / 0,125; toplam 10,250), sığmayan beş değer
  kırmızı ve sayılı; tek Enter ile yazılan dizi ve üyesinin Öznitelikler'i; sonradan eklenen kapının aynı adımda sıraya
  girmesi; dört duvar seçili Dört yan (önizleme ve yazılan); Çekirdek (tuğla yüzleri: toplam 10,090); batı odası Mahal
  olarak (0,925 / 0,900 / 1,975 ve net 3,800).
