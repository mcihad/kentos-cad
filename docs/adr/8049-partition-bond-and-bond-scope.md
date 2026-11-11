# ADR 8049: Ölçülü bölmede kalıcı bağ ve bağların kapsamı

- **Durum:** 1. ve 2. bölüm yapıldı (11 Kasım 2026); `plan/architecture-advanced.md` ADV-01, M49. Ölçülü bölme
  mesafesini "Bu ölçüyü koru" ile kalıcı bağ olarak yazar (§1); aks bağı (§2). Açık: kolon yüzü, yay kenar ve döşeme
  kenarı bağı (§3).
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

### 3. Bağların kalan kapsamı (açık)

Yay kenar (eş merkezli yayların radyal mesafesi; yayın Kaydır'ı gerekir), kolon yüzü (kolon yüzüyle duvar çizgisi;
kolon aks kesişimindeyse aks bağı bugün de onu taşır) ve döşeme kenarıyla duvar yüzü.

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
