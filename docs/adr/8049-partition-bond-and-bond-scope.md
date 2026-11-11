# ADR 8049: Ölçülü bölmede kalıcı bağ ve bağların kapsamı

- **Durum:** yapıldı (11–12 Kasım 2026); `plan/architecture-advanced.md` ADV-01, M49. Ölçülü bölme mesafesini "Bu
  ölçüyü koru" ile kalıcı bağ olarak yazar (§1); aks bağı (§2); kolon yanı bağı (§3); yay kenarın Kaydır'ı, geçici
  ölçüsü, Mesafe'si ve bağı (§4); döşeme kenarı bağı bilerek yazılmadı (§5).
- **Bağlam belgesi:** ADR 0770 (bağlantılı odalar ve kalıcı mimari bağlar), ADR 0657 ve 0734 (ölçülü bölme, yay kenar),
  ADR 0710 (duvarın mesafesi, Kaydır), ADR 0780 ve 0790 (yoldaki bağlar), ADR 0334–0338 (kısıtlar).

## Bağlam

Plan ADV-01'in 7 Kasım notu: "ölçülü bölme oluştururken kalıcı bağ yazmıyor (`walls_partition.rs`; ARCH-DRAW-02'nin
`[ ]`'i) ve bağların yay kenar, kolon, aks ve döşeme kapsamı (ADR 0770 sınırları) → M49". İlk kabulü: "1,20 m koridoru
koruyan bağı etkinleştirip dış duvarı değiştir … alanlar, kapılar, etiketler ve ölçüler izlesin."

ADR 0770 "Bu ölçüyü koru"yu Duvarın mesafesi'ne (`cad.walls.edit` Mesafe'nin `keep`'i) koydu; ama aynı ölçüyü ilk
çizerken veren araç, ölçülü bölme, ölçüyü yalnız bir kez uyguluyordu: bölme yerleşiyor, kaynak duvar sonra kayınca
bölme yerinde kalıyordu. Kullanıcı ölçüyü iki kez vermek zorundaydı (önce bölme, sonra Mesafe ile koru).

Başka programlar: Revit'te ölçü konulup kilitlenir ("lock it in place"), Chief Architect'te ölçü uçlarından hangi duvarın
kayacağı seçilir, Archicad'in mesafe kılavuzları kalıcı ilişki tutmaz (ADR 0770'in araştırması). Mevzuat açısından
korunan mesafe PAİY 29'un en küçük net ölçülerini (Mimari denetim › Mevzuat, ADR 8033) dış duvar değişse de tutmanın
yoludur.

## Karar

### 1. Ölçülü bölmede "Bu ölçüyü koru"

- `cad.walls.partition` v1'in yeni `keep` alanı (öntanımlı `false`; eski çağrılar bayt bayt aynı). `true` iken bölme
  yazıldıktan sonra **aynı adımda** bir mesafe bağı ve parametresi yazılır (ADR 0770 §4, `constraints_edit::add_bond_given`):
  - **Net yüzler arası** (`clear`): kaynağın bölme tarafındaki yüzü (`side` sol ise sol yüz) ile bölmenin kaynağa bakan
    yüzü (yolunun kaynak yüzün bulunduğu yanındaki yüz);
  - **Eksenler arası** (`axis`): kaynağın ekseni ile bölmenin ekseni;
  - değer verilen `distance` (parametre `d1`, `d2` …; ölçülen değer değil, verilen), bağın yönü çizimden okunur.
- Bağ ADR 0770'in çözücüsüne girer: kaynak duvar Kaydır, Mesafe, Boy ya da tutamaçla kayınca bölme aynı mesafede izler;
  bölmenin kendisi kaydırılırsa kaynak izler (en az değişiklik). Tek geri alma adımı "Duvar" bölmeyi ve bağını birlikte
  geri alır.
- **Yay kenar** kaynağıyla `keep` reddedilir (`invalid_constraint`, `keep`): mimari bağ düz duvar kenarları arasında
  kurulur; yay kenarın bağı §2'de.
- `WallsCreated`'ın yeni `bonds` alanı: yazılan bağın kimliği (yalnız bölmede; boşken yazılmaz).
- **Araç:** Mimari › Duvar ▾ › Ölçülü bölme'de "Bu ölçüyü koru" (K) aç/kapa satırı Araç ayarlarında ve komut satırında;
  K tek basışta değiştirir.

### 2. Aks bağı

Revit'te ölçünün kilidine basınca ölçülen öğeler birbirine bağlanır ve "kısıt kurulduktan sonra model değişse de
korunur" (Autodesk Revit yardımı, *Understanding Constraints*; BIM Pure, *17 Tips to Master Dimensions*, 16–17: "Entire
Walls" ölçüsü duvarı kesen aksları da alır, kilit onları birlikte tutar). Archicad duvarı aksa bağlamaz; yalnız kolonu aks
kesişimine oturtur ("Associative Grid Lines", duvar yüzü aksla gitsin isteği Graphisoft'un istek listesinde bekliyor).

Mevzuat: Mimari Proje Çizim ve Sunuş Standartları A.1.1.3 planlarda taşıyıcı aks sisteminin "statik projeye uygun harf
ve sayılarla" gösterilmesini, dış ölçülerin 3. çizgisinin taşıyıcı akslar olmasını ve "inşai akslar ve kesişme
noktaları"nın belirtilmesini ister; statik projede düşey taşıyıcılar akslarla tariflenir ve "seçilen aks sistemi mimari
projeleri ile uyumlu olacaktır" (AB Bakanlığı, *İnşaat Rehberi*, 2015, aynen aktarır). Statik proje değişip aks
kaydığında mimari plan aynı kalırsa iki proje ayrışır; dijital proje yönetmeliğinin Md. 9'u (RG 5.8.2026/33331) da model
ile çizimin uyuşmasını ister. Ofislerde dış duvar aksına göre ölçülendirilir; aks kayınca duvarın elle izlemesi ölçü
hatası doğuruyordu.

- **Sözleşme:** kısıtın parçasında yeni tür `grid` (`CadElement::Grid`, alanı yok): düz bir aksın çizgisi (iki nokta,
  bükümsüz), ilk noktasından ikincisine yönlü. Mimari parçadır: duvar çizgisi gibi uzaklık (duvar çizgisine, başka
  aksa; pervazdan ona), paralel, simetrik (eşit aralık) ve sabit alır; nokta kuralları ve harita bağı almaz (`bad_value`
  / `invalid_constraint`).
- **Çözücü:** ADR 0770'in bağdaştırıcısında aks bir kayma bilinmeyenidir (`Var::Grid`); kayınca iki noktası sol
  normali boyunca aynı kadar gider. Düzenlemenin taşıdığı ya da döndürdüğü aks tutulur (yalnız kendi boyunca uzayan
  tutulmaz), kilitli katmandaki aks kımıldamaz. Aksın kesişimindeki kolonlar aksla aynı adımda gider (ADR 0343 §3'ün
  izlemesi bağdaştırıcının yazdığı aksı da görür): duvar → aks → kolon zinciri tek geri alma adımıdır.
- **Kopma:** aks silinirse ya da yaya çevrilirse bağı aynı adımda kalkar (ADR 0334'ün kuralı).
- **Mesafe'de koru:** `cad.walls.edit` Mesafe `to: grid` + `keep` artık reddedilmez: o yandaki aks (`a`) ile duvarın
  ekseni (`b`) arasında mesafe bağı ve verilen değerle parametresi yazılır. Masaüstünde geçici ölçünün aks yanındaki
  kilidi aynı bağı yazar ya da kaldırır; mesaj "Mesafe bağlandı (d1 = 2.000 m): aks kayınca duvar, duvar kayınca aks
  izler."
- **Dosya:** `.kcad` şema 243 (`SCHEMA_WITH_GRID_BONDS`); yazıcı bir kısıt aks anınca seçer, 242 ve öncesi `bad_value`
  sayar. Örnek `arch-grid-bond.json/.kcad`, bozuklar `arch-grid-bond-in-schema-242`, `-coincident`, `-boundary`; en
  yeni olmayan şema örneği `schema-version-244.kcad`.

### 3. Kolon yanı bağı

Dış duvar çoğu zaman kolon yüzüyle aynı hizada ya da ondan belirli bir pay içeride/dışarıda durur; kolon kayınca duvarın,
duvar kayınca kolonun izlemesi gerekir. Revit'te kilitli ölçü kolonu da taşır; Archicad'de kolon ile duvar arasında
kalıcı ölçü yoktur.

- **Sözleşme:** kısıtın parçasında yeni tür `columnSide` (`side` 0–3): kolon kutusunun yanı, ilk köşesinden saat
  yönünün tersine (`cad.columns.distances`'ın numaraları: 0 alt, 1 sağ, 2 üst, 3 sol, kolonun dönüşüyle). Duvar çizgisi
  gibi uzaklık, paralel, simetrik ve sabit alır; 3'ten büyük yan `bad_value`.
- **Çözücü:** serbest kolonun karşılıklı iki yanı tek kayma bilinmeyenidir (`Var::Column`, 0 ile 2'nin ya da 1 ile
  3'ün): kolon o yanlara dik kayar (`at` taşınır). Akslara bağlı kolonun yanı, o yana paralel aksının bilinmeyenidir:
  bağ aksı kaydırır, kolon aksların yeni kesişimine oturur. Hiçbir aksı yana paralel olmayan (döndürülmüş) bağlı kolon
  yerinde tutulur. Düzenlemenin taşıdığı, döndürdüğü ya da yanlarını oynattığı (genişlik) kolon tutulur.
- **Kolonun oturması:** aks bir yazımla kayınca kolon ancak adım kapanırken kesişime oturuyordu; bağ ise yazımın hemen
  ardından çözülür ve kolonu eski yerinde ölçerdi (bağ sessizce bozulurdu). Bağdaştırıcı okumaya başlamadan bağların
  andığı bağlı kolonları güncel aksların kesişimine oturtur; yazımdan sonra izlenecek nesnelere bu kolonların aksları da
  katılır (`implied_parts`).
- **Mesafe'de koru:** `cad.arch.edit` Mesafe'nin `keep`'i: kolon kaydırılır ve aynı adımda karşıdaki yüz (duvar yüzü ya
  da başka kolonun yanı, `a`) ile kolonun yanı (`b`) arasında bağ ve verilen değerle parametresi yazılır; kolon zaten o
  mesafedeyse yalnız bağ yazılır. Aksa olan mesafede `keep` reddedilir (`invalid_column`, `operation.keep`): kolon aksa
  kesişime oturarak bağlanır (Akslara bağla). Masaüstünde kolonun geçici ölçüsündeki kilit aynı bağı yazar ya da kaldırır.
- **Dosya:** `.kcad` şema 244 (`SCHEMA_WITH_COLUMN_SIDE_BONDS`); örnek `arch-column-bond.json/.kcad`, bozuklar
  `arch-column-bond-in-schema-243`, `-side-4`, `-coincident`; en yeni olmayan şema örneği `schema-version-245.kcad`.

### 4. Yay kenarın Kaydır'ı

Yay kenar bağı (eş merkezli iki yay arasındaki radyal mesafe, ör. eğri koridorun 1,20 m'si) yayın kendisinin
kaydırılabilmesini ister; Kaydır yay kenarı reddediyordu ("geçici ölçü ve kaydırma düz kenardadır"), yayın yanındaki
düz kenarı da.

- **Yay kenar:** çemberi merkezinde büyür ya da küçülür; `by` yayın soluna (sola dönen, bükümü artı yayda merkeze
  doğru; sağa dönende dışa). Yay kendi çemberinde kalır, bükümü yeni açıklığının (`tan(açıklık/4)`).
- **Uçlar düz kenardaki gibi, doğrusu boyunca yeni çembere** (eski köşeye yakın kesişim): düz komşunun doğrusu, uçtaki
  tek duvarın doğrusu (o duvarın ucu izler), aynı doğrultudaki iki duvarın doğrusu, saplandığı duvarın doğrusu;
  serbest uç ve `none` birleşim yarıçap boyunca.
- **Yaya saplanan duvarlar** kendi son kenarları boyunca, uçlarının durduğu çemberin aynı kadar büyüyen/küçülen
  hâline kayar (duvarın içindeki yerleri korunur).
- **Yayın yanındaki düz kenar** kayınca köşe, kayan doğrunun yayın çemberiyle buluştuğu yere gider; yay çemberini ve
  uzak ucunu korur, bükümü yeni açıklığının (eskiden reddediliyordu).
- **Reddedilir, hiçbir şey kımıldamaz:** komşusu da yay olan yay; yaya teğet düz komşu (yuvarlatılmış köşe: merkezin
  kayması gerekirdi); yayla birleşen köşede başka duvar ucu; merkezine çöken yay; yeni çemberle buluşmayan doğru;
  ters dönen yay (yeni açıklık eskisinden yarım turdan çok uzak).
- **Araç:** Duvarı kaydır yay kenarı çemberinden seçer (açıklığı içinde, değilse yakın ucundan); imlecin çembere
  uzaklığı kaydırmadır; seçilen yay kısa kirişlerle işaretlenir.
- **Geçici ölçü ve Mesafe yayda:** yay kenarın iki yanında, merkezi 1 µm içinde aynı (eş merkezli), aynı kattaki ve
  açıca örtüşen en yakın yayın bakan yüzüne açık mesafe; yarıçap boyunca, örtüşmenin ortasında ölçülür (eğri koridorun
  net genişliği). Mesafe aynı formülle yayı merkezinde kaydırır. Düz kenar yaya hiç paralel sayılmaz; yayda aks ve Boy
  satırı yoktur. Boy ve aksa Mesafe düz kenarda kalır; ileti "Duvarın bu kenarı yay; geçici ölçü, Mesafe ve Boy düz
  kenarda çalışır." (yalnız Boy ve aks için çıkar).
- Birleşim uyarıları: bağımsız birleşim çözücüsü (`wall_command_cases.join_warnings`) yalnız düz duvar aldığından yay
  içeren yazımların ortak durumları uyarıyı açık bırakır (`$any`); geometri birebir denetlenir.

- **Yay kenar bağı:** duvarın yay kenarındaki çizgi (yüz, eksen, çekirdek yüzü) bağın parçasıdır; mevcut `wall`
  parçası (`edge`, `line`) yay kenarı da anar, dosya biçimi değişmedi. Çözücüde düz çizgi gibi bir kayma
  bilinmeyenidir, çizgisi çemberi: kayınca yarıçapı `−t·Δ` değişir. İki yay çizgisi eş merkezliyse (merkezleri 1 µm
  içinde) "paralel" sayılır; uzaklık (ör. eğri koridorun net 1,20 m'si), paralel (eş merkezli kal), simetrik ve sabit
  alır. Düz çizgi yaya, pervaz yay çizgisine bağlanmaz ("bağlanan yay çizgileri eş merkezli olmalı; düz çizgi yaya
  bağlanmaz"). Yay düzenlemede çemberi (merkez, yarıçap, dönüş) değişince tutulur; kenar düzden yaya ya da yaydan düze
  dönerse bağı kalkar (domain).
- **Bu ölçüyü koru verilen değeri yazar:** Mesafe'nin `keep`'i bağın parametresini ölçülen değerden değil verilen
  mesafeden yazar (Mesafe duvarı tam o mesafeye getirir; ölçülen değer başlangıçtan uzakta 1.200000000011 gibi
  çıkıyordu). Geçici ölçünün kilidi değeri nanometreye yuvarlar. Yay kenarda kalıcı ölçü "+" düğmesi gösterilmez (iç
  ölçü düz kesit boyunca çalışır).

### 5. Bağların kalan kapsamı

- **Döşeme kenarı bağı yazılmadı:** "Kaynağa bağlı sınır"lı döşeme (ADR 0771 §4) sınırını duvarların dış ya da iç
  yüzlerinden payıyla her adımda yeniden buluyor; duvar kayınca döşeme kenarı zaten izliyor. Ayrı bir kenar bağı aynı
  ilişkiye ikinci bir yetke olurdu (ADR 0338 §2'nin tek yetke kuralı).

## Doğrulama

- Uygulama: 8 × 6 m odada 3 m net bölme `keep` ile: bir bağ, parametre `3`, bölmenin ekseni y = 3,075; tek geri alma
  ikisini alır; güney yüz Kaydır ile 0,5 m dışa: bölmenin ekseni 2,575 (3 m korunur).
- 92 ortak durum (`walls_partition_cases.py`): bağ ve `d1 = 3` aynı adımda; yay kaynaktan `keep` reddedilir, hiçbir şey
  yazılmaz; masaüstü işleyicisinde ve Python'da.
- Araç: K satırı Kapalı → Açık, komut satırında "Bu ölçüyü koru", yerleşince bağ ve parametre, geri alma ikisini alır.
- Aks (§2): 43 ortak durum (`arch_bond_cases.py`, 30'dan; aks 0,5 m güneye → güney duvarı, kapıları ve T bağlı bölme
  izler; duvar kaydırılınca aks izler; Mesafe 1,50 → 2,00 m korunur, aks taşınınca duvar döner; sabit aks reddeder;
  zincir aks → güney duvarı → koridor duvarı; yay aks, dik aks ve aks olmayan nesne reddedilir) masaüstü işleyicisinde ve
  Python'da. Uygulama: duvar 0,5 m güneye → aks −1,5'ten −2,0'a, A-1 kolonu (−1, −1,5)'ten (−1, −2,0)'a, tek geri
  alma "Duvarı düzenle" üçünü alır; aks silinince ya da yaya çevrilince bağ kalkar. Etkileşim: aks yanındaki kilit
  bağı yazar (d1 = 2), aks 0,5 m kuzeye → duvar izler, "Taşı" tek adım; kilit yeniden bağı kaldırır. `.kcad` 244 test,
  bağımsız okuyucu 1796 dosya.
- Gerçek pencere (§2; `fixtures/interaction/real/aks-bagi.txt`, Xvfb 1920×1080, gerçek fare/klavye): eski sürümde aks
  tarafındaki 1,400 m'de kilit yok ve aks 0,5 m güneye taşınınca duvar yerinde kalıyor (kolon izliyor); yeni sürümde
  kilit bağı yazıyor ("d1 = 1.4 m"), aks taşınınca güney duvarı ve A-1 kolonu birlikte gidiyor, Ctrl+Z tek adımda
  ("Geri alındı: Taşı") üçünü geri alıyor. Görüntüler `.run/gui-evidence/mimari-m49/`.
- Kolon yanı (§3): 52 ortak durum (43'ten; serbest kolon: güney duvarı 0,5 m dışa → kolon 0,5 m izler; kolon 0,3 m
  kuzeye → duvar izler; Mesafe 0,50 m korunur ve duvar kayınca kolon y 0,8 → 0,3; mesafe aynıyken yalnız bağ; aksa
  koru reddedilir; akslara bağlı kolon: batı duvarı 0,3 m batıya → 1 aksı ve kolon (−1,3; −1,5); 1 aksı 0,3 m batıya →
  kolon ve batı duvarı izler; kolon olmayan nesne ve paralel olmayan yan reddedilir) masaüstü işleyicisinde ve
  Python'da. Uygulama: Mesafe+koru tek geri alma "Mimari nesneyi düzenle" kaymayı ve bağı birlikte alır; aks taşınınca
  kolon ve duvar tek "Taşı" adımı. Etkileşim: seçili kolonun dört yanında kilit; güney kilidi d1 = 1.2 yazar (büyük
  koordinatlarda 1.199999999814 çıkıyordu; kilit değeri nanometreye yuvarlar), duvar 0,5 m dışa → kolon izler; kilit
  yeniden bağı kaldırır. `.kcad` 244 test, bağımsız okuyucu 1800 dosya.
- Yay Kaydır'ı (§4): `cad.walls.edit.shift` 53 durum (12'si yayın; bağımsız model `wall_shift_cases.py`, yayın
  merkezi ve yarıçapı bükümden): tek yay 0,3 m dışa → r 2,9 → 3,2, uçlar yarıçap boyunca (−0,207; 219,783), büküm
  0,4 kalır; düz-yay-düz yolda yay 0,3 m → köşeler y = 230 doğrusu boyunca (2,618 ve 5,382), saplanan duvar 0,3 m
  izler, büküm 0,3 → 0,217; uçtaki köşe duvarı x = 4 boyunca (249,646); kapalı odanın yay kenarı 0,25 m dışa; doğu
  kenarı 0,3 m dışa → yay köşesi çember boyunca (5,3; 260,173), büküm 0,25 → 0,267; teğet komşu, yuvarlatılmış köşe ve
  merkeze çöken yay reddedilir; eski "yay duvar reddedilir" durumu artık yayı küçültür. Rust ve Python'da aynı; 748
  duvar ve bağ durumu geçer. Etkileşim: yay çemberinden seçilir, −0,3 yazılınca uçlar 3,2/2,9 oranında, büküm aynı,
  tek geri alma.
- Yay geçici ölçüsü (§4): eğri koridor (r 3 ve 4,4, (2; 290) merkezli çeyrek yaylar, aralarında net 1,20 m; içeride r
  1,5 yarım örtüşen yay): iç yay dışta 1,20 m (270°'de), içte 1,30 m (örtüşmenin ortası 292,5°'de); Mesafe 1,50 m → iç
  yay r 2,7; dış yay içe 1,00 m → r 4,2; eski "yay duvar reddedilir" ölçüsü artık iki yanı boş döner. 35 ölçü ve 55
  Kaydır durumu Rust ve Python'da. Etkileşim: iç yay seçilince 1,200 görünür, sayısına 1.5 yazılınca yarıçap 2,7,
  tek geri alma.
- Yay bağı (§4): 59 bağ durumu (52'den; bağımsız model `arch_bond_cases.py`'nin çember çizgisi): eğri koridor
  (merkez (20; 10), r 3 ve 4,4) 1,20 m ölçülür; iç yay 0,3 m içe → dış yay r 4,4 → 4,1; dış yay 0,5 m dışa → iç yay r 3
  → 3,5; Mesafe 1,50 korunur, dış yay r 4,8'e → iç yay r 3,1; eş merkezli (paralel) bağda yalnız kayan kımıldar; düz
  çizgi–yay ve eş merkezli olmayan yaylar reddedilir. 1146 duvar, kısıt, mimari, harita bağı ve parametre durumu Rust
  ve Python'da. Uygulama: yay düzleşince ya da düz kenar bükülünce bağ kalkar. Etkileşim: koridorun iç yayında tek
  kilit, d1 = 1.2 (yuvarlamadan önce 1.200000000011), dış yay 0,5 m dışa → iç yay r 3,5. Çekirdek: eş merkezli iki
  yay çizgisinde 1,5 m bağ dış yayı r 4,3 → 4,6 yapar; düz–yay paralel çelişir.
- Gerçek pencere (§4; `fixtures/interaction/real/yay-bagi.txt`): eski sürümde seçili yayın ölçüsü yok, Duvarı kaydır
  yaya tıklanınca "kaydırılacak duvar kenarı yok" diyor; yeni sürümde iç yayın 1,200 m'si yarıçap boyunca ve kilidi
  ("+" yok), kilit bağı yazıyor, dış yay 0,5 m dışa kayınca iç yay izliyor (koridor 1,200), Ctrl+Z ikisini birlikte
  geri alıyor.
- Gerçek pencere (§3; `fixtures/interaction/real/kolon-bagi.txt`): eski sürümde seçili kolonun ölçülerinde kilit yok;
  yeni sürümde dört yanda kilit, güney kilidi "d1 = 0.8 m" yazıyor (kolon kımıldamıyor), duvarın güney kenarı A aksına
  1,40'tan 0,90'a alınınca kolon da 0,5 m güneye gidiyor, Ctrl+Z ("Geri alındı: Duvarı…") ikisini birlikte geri alıyor.
