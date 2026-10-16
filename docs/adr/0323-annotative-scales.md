# ADR 0323: Ölçekli açıklamalar

- **Durum:** kabul edildi (2026-10-16). [plan/cad.md](../../plan/cad.md) §11 `CAD-ANNO-12`, `13`, `14`, `15` (sahibin sözü: “Pull yap
  anno bitene kadar durma”; 3–4 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0205 (açıklamaların yükseklikleri ve çizim ölçeğinin izlenmesi), 0164 ve 0252 (pafta ve haritaları, PDF), 0183
  ve 0322 (stiller ve stilden ayrılan alanlar), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Açıklamanın (yazı, ölçü, kılavuz) yüksekliği çizim biriminde tek bir sayıdır: projenin çizim ölçeğinde kâğıtta 2,5 mm olsun diye
`2,5 / 1000 × ölçek` yazılır (ADR 0205). Aynı çizim iki paftada iki ölçekte (1:50 kat planı, 1:100 vaziyet) basılınca yazı birinde yarı
boyda kalır, öbüründe iki kat olur. Paftanın haritası küçük yazıyı yazmaz bile. AutoCAD bunu ölçekli (annotative) nesnelerle çözer: nesne
katıldığı her ölçekte bir temsil taşır; görünüm alanı kendi ölçeğindeki temsili çizer.

## Karar

### 1. Model

Yazının, ölçünün ve kılavuzun yeni `annotative` alanı (`kentos_contracts::Annotative`):

- `at`: nesnenin kendi geometrisinin ölçeği (1:N'nin N'si). Nesnenin kendisi o ölçeğin temsilidir; çizim alanı onu çizer.
- `scales`: öbür ölçekler, küçükten büyüğe, birer kez, `at` olmadan, en çok 32. Her öğe ölçek ve o ölçeğin ayrı değerleri: yazı ve
  kılavuzda `point` (yazının noktası, kılavuzun kolunun başı) ve `rotation`; ölçüde `offset` (çizginin uzaklığı) ve `textShift`
  (değerin yeri). Değerler mutlaktır: bir ölçeğin değeri değişince öbür ölçeklerin temsilleri değişmez.
- Eğri boyunca yazı (harfleri eğrisinin çerçevesinde) ve nesneye bağlı yazı (kendi etiket ölçeğini izler) ölçekli olmaz; tablo,
  koordinat yazısı ve km yazısı ADR 0205'in kuralında kalır.

Ölçekli olmayan nesne yoktur sayılmaz: eski nesneler ve ölçekli yapılmayanlar eskisi gibi tek yükseklikle çizilir (§3).

### 2. Temsil

`at_scale(e, s)` (sözleşmede, bağımsız başvurusu `scripts/fixtures/annotative_cases.py`):

- `s` nesnenin kendi ölçeğiyse nesnenin kendisi; katılmadığı ölçekte hiç çizilmez.
- Başka ölçeğinde `f = s / at` kadar büyür: yazının yüksekliği ve kutu genişliği, kılavuzun yüksekliği (ok ve kol onunla), ölçünün
  yüksekliği (oklar, uzatma çizgileri ve değer boşluğu yüksekliğin katlarıdır) ve ayrı değeri yoksa değerinin kayması. Konumlar,
  ölçülen noktalar, ölçünün değeri değişmez.
- Ayrı değerler bu ölçekte yerini alır; kılavuzun `point`'i okun ucu dışındaki köşeleri birlikte kaydırır.

Paftanın haritası her ölçekli nesneyi kendi ölçeğinde çizer: `Document::at_scale(s)` çizimin okuma kopyasıdır (geri alma geçmişi yok,
aynı sürüm), ölçekli nesneler temsilleriyle, katılmadıkları ölçekte yok; kopya yoksa çizimin kendisi. Masaüstünün pafta kipi
(`sheets.rs`: stilli katmanlar, yazılar, PDF vektörleri) ölçek başına bir kopya ve geometri deposu tutar (çizim değişince yenilenir,
dört ölçek); başsız kitap PDF'i (`kentos_plot::book_pdf`) ölçek başına bir kopya kurar. Ölçekli nesnesi olmayan çizimde kopya kurulmaz.

### 3. Çizim ölçeği değişince (`CAD-ANNO-14`)

ADR 0205'in izlemesi ölçekli nesnede yüksekliği ayrıca değiştirmez; nesnenin kendisi yeni ölçeğin temsili olur
(`rebase_annotative`): yeni ölçek nesnenin ölçeklerinde yoksa eklenir, eski kendi ölçeği öbür ölçeklerine geçer ve her ölçek eskisi gibi
çizilir (eski temsilin değerleri, yenisinden farklıysa, o ölçeğin ayrı değerleri olur). Tür yüksekliği (Yazı, Ölçü, Kılavuz) aynı adımda
değiştiyse ve nesne o yükseklikteyse yeni yüksekliği alır. Yazılması ADR 0205'in “Yazı yüksekliklerini uydur” adımıdır
(`cad.entities.edit`'in `annotationScale`'i); `EntityGeometry`'nin yeni `annotative`'i ölçekleri taşır.

**Göç açıktır:** eski nesneler ölçekli olmaz, boyları değişmez; kullanıcı Ölçekli yap'ı seçtiği nesnelerde çalıştırır. Bir düzenleme
(`cad.entities.edit`'in güncellemesi) ölçekleri vermezse nesne kendininkini korur; ölçekleri yalnız `cad.annotations.scales` kaldırır.
Taşı, Kopyala, Döndür, Ölçekle, Aynala ve Hizala ayrı noktaları nesneyle birlikte dönüştürür, doğrultuları nesnenin dönüşü kadar döner,
ölçünün ayrı uzaklığı ve kayması dönüşümün uzunluk ölçeğiyle büyür; kauçuk levha ve eğen dönüşüm onları olduğu gibi bırakır.

### 4. Komut ve arayüz (`CAD-ANNO-12`)

`cad.annotations.scales` (anlamsal, tek geri alma adımı): `make` (Ölçekli yap; kendi ölçeği projenin çizim ölçeği, `scales`
eklenir; ölçekli nesnede yalnız ölçekleri ekler), `unmake`, `add`, `remove` (kendi ölçeği kaldırılmaz: `own_scale`), `set` (`scale`'de
`point`, `rotation`, `offset`, `textShift`; kendi ölçeği `own_scale`, katılmadığı ölçek `scale_not_taken`, türün almadığı değer
`invalid_annotative`), `reset` (verilen ya da bütün öbür ölçeklerde ayrı değer kalmaz). Olamayan ya da (make dışında) ölçekli olmayan
nesne `not_annotative` uyarısıyla atlanır, hiçbiri kalmazsa ret; kilitli katmandakiler `layer_locked` uyarısıyla. Python
`kentos.cad.annotations.scales`, MCP aracı katalogdan (yıkıcı listede: ölçekleri kaldırır).

Masaüstünde Öznitelikler'in **Ölçekli** (Evet/Hayır), **Kendi ölçeği**, **Öbür ölçekler** (yazılarak eklenir ve kaldırılır, tek adım;
ayrı değeri olan ölçek yıldızlı) ve **Ayrı değerler** › Sıfırla satırları; CAD › Açıklama'nın **Ölçek** paneli: Ölçekli yap, Ölçekteki
yeri, Ölçeği sıfırla, Ölçekliliği kaldır. **Ölçekteki yeri** aracı ölçekli nesneye tıklanınca, Ölçek (Ö) ile seçilen ölçekte yeni yerin
tıklanmasını bekler (ölçüde çizginin geçeceği yer) ve önizlemesi o ölçeğin temsilidir.

### 5. Kâğıttaki ölçü (`CAD-ANNO-15`)

Pafta çekirdeğinde kâğıt üstünde ölçü öğesi yoktur; bu karar kuralını koyar, öğe gelince ona uyar:

- Ölçekli modeldeki açıklamanın ölçeği **bir kez** uygulanır: modeldeki temsil `kâğıt mm × ölçek / 1000` yüksekliğindedir, harita onu
  `1 / ölçek` ile kâğıda indirir. Kâğıt yüksekliği haritanın ölçeğiyle ikinci kez çarpılmaz; ölçünün değeri (ölçülen uzunluk) ölçekle
  hiç çarpılmaz.
- Kâğıt üstündeki ölçü bir model nesnesine haritadan bağlanacaksa (AutoCAD'in DIMASSOC 2'si gibi), bağı haritanın değil model nesnesinin
  yeridir (ADR 0249): değeri modeldeki uzunluk, yüksekliği kâğıt mm'si, çizgisi haritanın dönüşümüyle kâğıda; çizim ölçeği ve haritanın
  ölçeği ona uygulanmaz. Bu, kâğıttaki ölçüyle ölçekli model ölçüsünün aynı nesneyi iki kez boylandırmasını önler.
- DXF'e ölçekli nesne kendi ölçeğindeki hâliyle gider; dışa aktarma raporu söyler (AutoCAD'in ölçek bağlamları yazılmaz).

`.kcad` şema 64, `FORMATS_VERSION` 73: yazının, ölçünün ve kılavuzun `annotative`'i (JSON'u gibi); sütunlarda yazının 15., kılavuzun
4. seçeneği ve ölçünün “more” JSON'u. Sunucu nesnenin tanımında taşır, kuralını denetler; göç yoktur.

## Sonuçlar

- Bir yazı, ölçü ya da kılavuz 1:50 ve 1:100 paftada aynı kâğıt yüksekliğinde basılır; bir ölçekteki yeri öbürünü bozmaz.
- Çizim ölçeği değişince ölçekli nesne yeni ölçekte kendisi olur; eski ölçeği kaybolmaz.
- Eski çizimler değişmez; ölçekli yapmak açık bir adımdır.
- Çizim alanı yalnız nesnenin kendi ölçeğini gösterir; başka ölçeğin temsili pafta kipinde ve Ölçekteki yeri'nin önizlemesinde görülür.
- Paftanın kopyası çizimin her değişikliğinde ölçek başına yeniden kurulur; büyük çizimde ölçekli nesne varken pafta kipinin ilk karesi
  bir geometri deposu kurmak kadar uzar.

## Doğrulama

- Bağımsız başvuru (KentOS kodu olmadan): `scripts/fixtures/annotative_cases.py` — 36 komut durumu (`fixtures/commands/v1/cad.annotations.scales.json`:
  altı işlem, retler, çizim ölçeğinin değişmesi `cad.settings.set` ile, taşımada ayrı nokta `cad.entities.transform` ile) ve 30 temsil
  durumu (`fixtures/annotative/v1/cases.json`: her ölçekte çizilen, gizlenen, ölçeğe taşınan; sözleşmenin testi gidiş dönüşte aynı
  çizimi de denetler).
- KCAD: `annotative.kcad`, beş bozuk dosya; bağımsız yazıcı ve okuyucu.
- Pafta: `apps/desktop/src/annotative_sheet_tests.rs` kabul odasının iki A3 paftasını masaüstünün PDF'iyle basar ve poppler'ın
  `pdftotext -bbox`'ıyla ölçer: tek yükseklikli ölçünün değeri 1:100'de yazılmaz ya da yarı boydadır; ölçekli ölçünün değeri iki
  paftada aynı yükseklikte ve aynı sayıdır; 1:100 yerini 1 m kaydırmak yalnız ikinci sayfada değeri 10 mm kaydırır.
- Araç testleri `kentos-interaction`'ın `annotative`'i.
- Resimler: `KENTOS_SHOTS_ONLY=olcekli-oznitelik,olcekli-yeri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
