# ADR 0264: İleri CAD nesnelerinin kaynak modeli ve düzenleme anlamı

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-20`: “dolu alan, bölge, wipeout, merkez işareti ve
  eksen çizgisi, çok kollu kılavuz ve ileri nesneler için **önce** kaynak model ve düzenleme anlamı”. Kapsamı ben belirledim (sahibin
  sözleri: “CAD data bölümüne geç ve orayı tamamla”, “durmadan devam et”). Bu ADR modeli ve anlamı bağlar; araçları ve çizimleri
  planın kendi maddeleri yapar (`CAD-EDIT-23`, `CAD-HATCH-09`, `CAD-ANNO-11`, `CAD-ANNO-2x`).
- **Bağlam belgesi:** ADR 0062 ve 0186 (tarama), 0065 ve 0143 (alan ve çok parçalı alan), 0146 (kılavuz), 0249 ve 0263 (alt nesne
  başvuruları), 0258 (kapalı çoklu çizgi), 0260 (görünüş ve çizim sırası), 0244 (işlem manifesti).
- **Web:** ertelendi (ADR 0228); bu ADR'de kod yok, web'in yapacağı bir şey yoktur.

## Bağlam

AutoCAD'den gelen çizimlerde ve yeni CAD işlerinde KentOS'un henüz kendi adıyla taşımadığı nesneler var: dolu üçgen ve dörtgen
(SOLID, TRACE), bölge (REGION), wipeout (WIPEOUT), merkez işareti ve eksen çizgisi (CENTERMARK, CENTERLINE), çok kollu kılavuz
(MULTILEADER), çoklu çizgi (MLINE), geometrik tolerans (TOLERANCE). Bugün bir kısmı bir KentOS nesnesine çevrilir, bir kısmı DXF'te
gerekçesiyle atlanır (`dxf/emit.rs`). Her birine ayrı nesne türü eklemek her türün yazıcıda, okuyucuda, depoda, çizimde, komutlarda
ve Python'da bir kez daha yazılması demektir; eklemeden önce **var olan bir nesnenin anlamı yetiyor mu** sorusu cevaplanmalıdır.

Kural (bu ADR'nin özü): **yeni nesne türü ancak var olan türlerin hiçbiri nesnenin düzenleme anlamını taşıyamıyorsa eklenir.** Taşıyorsa
nesne o türün bir alanı ya da görünüşüdür; içe ve dışa aktarma o alana çevirir ve kaybı söyler.

## Karar

### 1. Dolu alan (SOLID, TRACE)

- **Model:** kapalı alan (`polygon`) ve dolgusu. Katı dolgu ayrı bir tür değildir: `hatch`'in `solid` deseni (ADR 0062) ya da alanın
  katmanının dolgusu (stil). DXF'in SOLID ve TRACE'i bugün alan olarak gelir, dolgusu katman stilinden (“Dolu alan … kapalı alan olarak
  alındı” notu); dışa aktarmada `solid` desenli tarama `HATCH` SOLID'dir.
- **Düzenleme:** alanın bütün işlemleri (taşı, döndür, ölçekle, aynala, tutamaç, alan işlemleri); dolgu nesneyle gider.
- **Sonraki:** SOLID'in kendi rengi alanın `color`'ına ve “dolgu”suna gelir: `CAD-HATCH-09` (dolgunun saydamlığı ve sırası).

### 2. Bölge (REGION)

- **Model:** KentOS'un bölgesi **alandır**: `polygon` dış halka, `holes` delikler, `parts` ayrık parçalar (ADR 0143), kenarları düz ya da
  yay (`bulges`). Bölgenin Boole işlemleri alan işlemleridir (birleştir, kesiştir, çıkar; ADR 0065). Kapalı çoklu çizgi bölge **değildir**
  (ADR 0258 §1).
- **DXF:** REGION'ın sınırı ACIS verisindedir; ACIS okuyucumuz yoktur: atlanır ve söylenir (bugünkü gibi). Bir gün ACIS'in düzlemsel
  bölgesi okunursa hedef türü alandır, yeni tür değil.

### 3. Wipeout

- **Model:** alanın (`polygon`) görünüşü: **maske**. Yeni alan `mask` (`{ frame: bool }`): alan çizim zemininin rengiyle dolar,
  altındakileri örter; `frame` sınırını çizer. Örtme **çizim sırasıyla** (ADR 0260 §3) olur: aynı katmanda kendinden önce çizilenleri,
  katmanlar arasında altındaki katmanları. Ayrı tür değildir: bütün alan işlemleri ve tutamaçlar geçerlidir.
- **Seçme:** içinden değil, sınırından seçilir (resim gibi, ADR 0192); `frame` kapalıyken de.
- **Yazının zemini** (ADR 0145, 0182) ve ölçü değerinin zemini (ADR 0147) bu maskenin özel hâlleridir; onlar kendi nesnelerinin alanı
  olarak kalır.
- **DXF:** WIPEOUT'un sınırı (14 ve 24'lerin noktaları, `IMAGEDEF` olmadan) maskeli alana gelir, maskeli alan WIPEOUT'a gider.
- **Yeri:** `CAD-EDIT-23` (çizim sırası, öne ve arkaya, wipeout ve sınırın görünürlüğü).

### 4. Merkez işareti ve eksen çizgisi

- **Model:** yeni **açıklama türleri**, çünkü ikisi de kendi geometrisini kaynak nesneden **türetir** ve onu izler; hiçbir var olan tür bu
  ilişkiyi taşımaz:
  - `centermark { of: SubRef (dairenin ya da yayın `center`'ı), size, extension, lines: bool }`: merkezdeki artı, isteğe bağlı eksen
    uzantıları dairenin dışına `extension` kadar.
  - `centerline { a: SubRange, b: SubRange, extension }`: iki düz kenarın (ya da iki yayın) ortası boyunca eksen; uçlar iki kenarın
    izdüşümlerinin ortaklarından.
- **Başvuru:** ADR 0249 ve 0263'ün başvuruları (blok içindekiler de); kaynak değişince belge kayıttan önce izler (ölçünün `dimension_ties`
  kuralıyla); kopan bağ nedeniyle kalır.
- **Düzenleme:** kendi başına taşınmaz; kaynağı taşınınca gelir. `extension`, `size` ve `lines` Öznitelikler'den. Patlat çizgilere
  çevirir ve bağı düşürür.
- **DXF:** AutoCAD'in CENTERMARK ve CENTERLINE'ı blok ve ilişki nesneleridir; okunurken türe, yazılırken çizgiler ve KENTOS verisine.
- **Yeri:** `CAD-ANNO-11`.

### 5. Çok kollu kılavuz (MULTILEADER)

- **Model:** var olan `leader` (ADR 0146): ok, kırık çizgi, kol ve not tek nesne. Çok kollu kılavuz `leader`'ın **kollarının listesi**
  olur (`arms: [{ pts, arrow }]`, ilki bugünkü `pts`'tir); not ve stil ortaktır. Blok içerikli kılavuz notun yerine bir yerleştirme
  (`content: block`) taşır. Bugün MULTILEADER'ın ilk kolu ve MTEXT notu gelir, öteki kollar söylenir.
- **Düzenleme:** kol ekle, kol sil, kolları hizala, kolları topla (AutoCAD'in MLEADERALIGN, MLEADERCOLLECT'i) `cad.leaders.edit`'in
  işlemleri olur; notun yerinde düzenlenmesi bugünkü gibidir.
- **Yeri:** `CAD-ANNO-2x` (§11.3).

### 6. Öteki ileri nesneler

| Nesne | Karar |
|---|---|
| MLINE (çoklu çizgi) | Yeni tür değil: içe aktarmada her öğesi kendi ötelemesiyle çoklu çizgi olur (bugün atlanıyor; “patlatıp kaydedin” notu). Duvarlar `CAD-ARCH`'ın `wall` kaynağıdır. |
| TOLERANCE (geometrik tolerans) | Çok satırlı yazının (ADR 0182) çerçeveli biçimi olarak; bugün atlanır. |
| IMAGE | Resim nesnesi (ADR 0192); DXF'in IMAGE'ı `IMAGEDEF` yoluyla bağlı resme gelir (`CAD-IO`). |
| 3DSOLID, BODY, SURFACE, MESH | 2B CAD'in dışında; atlanır ve söylenir. 3B kapsamı TODOS.md §17–19'dadır. |
| OLE, PDF/DWF/DGN altlıkları | Dış başvuru ve altlık kapsamı (`cad.references.*`), ayrı karar. |

### 7. Ortak kurallar

- Her yeni alan ya da tür **önce** sözleşmede ve `.kcad` şemasında (eski okuyucu açıkça reddeder; `CAD-DATA-23`), bağımsız Python okuyucu
  ve yazıcısında, tipli sütunlarda, komutların ortak durumlarında ve Python ile MCP'de birlikte gelir (`CAD-DATA-22`).
- İçe aktarmada türe çevrilemeyen her nesne gerekçesiyle söylenir; sessizce düşmez.
- Türetilmiş geometri (merkez işareti, eksen, maskenin dolgusu) kaynak olarak saklanmaz; kaynak ilişkidir, çizim ondan üretilir.

## Sonuçlar

- Yeni nesne türü yalnız iki tanedir (merkez işareti, eksen çizgisi); öbürleri var olan türlerin alanı ya da görünüşüdür.
- Wipeout ve dolgu çizim sırasına bağlanır; sıra ADR 0260'tan gelir.
- Plan maddeleri (`CAD-EDIT-23`, `CAD-HATCH-09`, `CAD-ANNO-11`, `CAD-ANNO-2x`) bu modeli uygular.

## Doğrulama

- Karar belgesi; kod değişmedi. Bugünkü davranışın dayanakları: DXF okuyucusunun SOLID ve TRACE'i alana çevirmesi ve atladıklarını
  gerekçesiyle söylemesi (`crates/shared/formats/src/dxf/emit.rs`), alanın delik ve parçaları (ADR 0143), başvurular (ADR 0249, 0263).
