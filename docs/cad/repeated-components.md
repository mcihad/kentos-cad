# Tekrarlanan 2B bileşenleri bulma

Tek bir bileşenin çizgi, 2B çoklu çizgi, çember, yay, düz yazı ve statik bloklarını seçip Bloklar panelindeki
**Tekrarlanan bileşenleri bul…** düğmesine basın. Bul, adayları panelde açar;
çizimi kullanmaya devam edebilirsiniz. Göster bir adayı seçip yakınlaştırır.
İstediğiniz adayları işaretleyip **… adayı sahnede seç** ile seçimi alın.
Arama hiçbir nesneyi bloğa dönüştürmez, geometriyi veya bağları değiştirmez.

Döndürülmüşler aranır. Aynalanmışları ve farklı boyuttaki ölçekli eşleri
aramak için ilgili kutuları açıkça seçin. Katman, görünüş ve öznitelik eşitliği
varsayılan korunur. Bu kutuyu kapatınca sonuç yalnız geometrik eşleşmedir;
farklı verilerin birleştirilebilir olduğu anlamına gelmez. Tolerans alanı mm
ile görünür, örneğin `0,001 mm` veya `0.000001 m` yazılabilir. Tolerans proje
ölçeğini ve nesnelerin gerçek boyutunu değiştirmez.

Tek parçalı, deliksiz çoklu çizgide açık/kapalı durum, yaylı kenarlar ve
başlangıç/bitiş genişlikleri karşılaştırılır. Kapalı çizginin başlangıç
köşesi veya çizim yönü farklı olabilir. Köşe sayısını değiştiren yeniden
parçalama/sadeleştirme yapılmaz. Z/M verisi veya mahal ayırıcı rolü taşıyan
çizgi, ek parçalar ve desteklenmeyen NURBS gibi türler kaynakta açıkça reddedilir;
bu öğeler kaynaktan sessizce çıkarılmaz. Kaynak en çok 128, arama kapsamı
50 000 nesne; her çoklu çizgi en çok 4 096, kaynak ve kapsam birlikte
200 000 çoklu çizgi köşesidir. Kısmi sonuç başlığı varsa kapsamı daraltın. Durdur veya
Bloklara dön aramayı iptal eder. Çizim değişmişse Yeni kaynak ile tekrar başlayın.

```python
from kentos import cad

# Kaynağı çiziminizdeki bir bileşenin gerçek UUID'leriyle açıkça verin.
source = [first_line_uid, second_line_uid, circle_uid, arc_uid]
before = doc.to_bytes()
found = cad.blocks.find(doc, source=source, mirrors=True)
assert doc.to_bytes() == before
for candidate in found.matches:
    # members[i], source[i] nesnesinin bu aday içindeki karşılığıdır.
    print(candidate.members, candidate.scale, candidate.mirror)
if found.truncated:
    print("Kapsamı daraltıp yeniden arayın.")
# Bir adayı seçmek ayrı karardır.
if found.matches:
    doc.select(found.matches[0].members)
```

API uzunlukları metre, dönüşü radyan, ölçeği birimsiz çarpanla alır.
`uids` yoksa bütün belge, `[]` ise boş kapsam aranır. `plan` aynı yazısız
çıktıyı verir. `expected_revision` ile eski çizim durumuna ait sorgu reddedilir.
Arka plan işi: `doc.start("cad.blocks.find", cad.BlocksFind(source=source,
mirrors=True))`; `Job.cancel()`/`Job.wait()` mevcut iş arayüzüdür. MCP aynı
`cad.blocks.find` komutunu, `op=plan`, `drawing.job.start/status/cancel`
yüzeylerini kullanır. [Karar ve kapsam](../adr/0866-repeated-2d-candidate-recognition.md).
[Çoklu çizgi eşlemesi](../adr/0867-analytic-polyline-repeat-recognition.md).

## Seçilmiş adayları dönüştürme

İşaretlenmiş adaylar için **Bloğa dönüştür…** düğmesi ayrı inceleme açar.
Adı, kök→yerleştirme sayısını ve korunacak ölçü noktalarını kontrol edin.
İnceleme çizimi değiştirmez; **Dönüştür** tek geri alma adımıdır. Ad değişince
yeniden inceleme gerekir. Durdur/Esc kabul edilmemiş işi bırakır.

Dönüşüm kapsamı tek, veri ve tür şeması olmayan CAD katmanındaki sürekli
çizgili analitik nesneler, düz yazı ve statik iç bloklardır. Genel nesne verisi/sembol/ek dosya, özel çizgi tipi, parçalara
karışan çizim sırası, tablo veya taşınamayan ilişki varsa gerekçe gösterilir;
bağlar silinmez. Aramada geometriye göre eşleşmiş olmak bu denetimi kaldırmaz.
İlişkili diziye dönüşüm bu düğmenin kapsamı değildir.

```python
# Grupları kullanıcı incelemesiyle açıkça seçin; bütün eşleşmeleri kendiliğinden dönüştürmeyin.
chosen = [found.matches[0].members, found.matches[1].members]
args = dict(name="Teknik sembol", source=source, groups=chosen, mirrors=True,
            expected_revision=found.revision)
review = cad.blocks.convert.plan(doc, **args)
for point in review.dimensions:
    print(point.uid, point.before, point.after, point.deviation)  # sapma metre
# Kabul verildiğinde aynı girdiler yeniden doğrulanır.
done = cad.blocks.convert(doc, **args)
```

`mapping` eski kök UUID'sini korunacak yerleştirme UUID'sine ve 0 tabanlı
tanım parçasına bağlar. Her adayın ilk nesnesi yerleştirmede kimliğini korur.
Ters/döngüsel kenara bağlı ölçünün yeri ve oranı da açıkça eşlenir; belirsiz
köşe numarası rastgele seçilmez. Uzun işler için `doc.start("cad.blocks.convert",
cad.BlocksConvert(**args))` kullanılabilir. MCP aynı komutta `op=plan`,
korunan plan kimliği ve `drawing.job.*` iş yüzeylerini sunar.
[Karar, sınırlar ve ilişki denetimi](../adr/0871-selected-repeat-conversion-and-topological-maps.md).

## Mevcut yerleştirmeleri diziye bağlama

İncelenmiş bir yerleşim ve açık öğe eşlemesi `cad.arrays.adopt` ile mevcut
dizi modeline bağlanabilir. Bu komut örüntüyü kendisi tahmin etmez;
doğrusal örüntü inceleme akışı aşağıdaki masaüstü panelindedir.

```python
# Sekiz yerin 2 ve 6 numaraları boş, 1 numaralı öğe elle taşınmış.
# existing_by_place kullanıcı tarafından incelenmiş 0 tabanlı yer → mevcut UUID eşlemesidir.
items = [cad.ArrayItem(uid=existing_by_place[k], kept=(k == 1))
         if k in existing_by_place else cad.ArrayItem() for k in range(8)]
args = dict(source=existing_by_place[0], items=items,
            layout=cad.LineArrayLayout(count=8, dx=2, dy=0, fit=False))
review = cad.arrays.adopt.plan(doc, **args)
print(review.missing, review.kept, review.deviation)  # sapma metre
# Kullanıcının inceleyip kabul ettiği aynı yerleşim:
done = cad.arrays.adopt(doc, **args, expected_revision=review.revision)
cad.arrays.edit(doc, array=done.array,
    edit=cad.LayoutArrayEdit(layout=cad.LineArrayLayout(count=10, dx=3, dy=0, fit=False)))
```

Bağlama geometriyi taşımaz; UUID, çizim sırası, blok tanımı ve mevcut ölçü
referansları aynıdır. Eksik yerler boş, korunan öğe yerinde kalır; Reset
açıkça istenirse eksikler tamamlanır ve istisnalar kaldırılır. Ortak
şablona veya yerleşime uymayan öğe veri atılmadan reddedilir.
Yerinde kalan üye UUID'sini korur; silinmiş yeri Reset ile doldurmak yeni
bir nesne ve yeni UUID oluşturur. Silinen üyeye ait eski kimlik yeniden
kullanılmaz; bu, silme işlemini Undo ile geri almaktan farklıdır.
`doc.start("cad.arrays.adopt", cad.ArraysAdopt(**args))` durdurulabilir
iştir. MCP aynı komutta `op=plan` ve plan beklentisini kullanır.
[Kapsam ve atomiklik](../adr/0873-existing-insert-array-adoption.md).

## Mevcut bloklarda doğrusal örüntü arama

`cad.arrays.find` aynı statik bloğun aynı görünüş, yön, ayna ve ölçekteki
yerleştirmeleri arasında doğrusal adaylar üretir. En az üç dolu yer ve
gözlenmiş bir komşu çift gerekir. Kaynak sıranın ortasında olabilir;
önerinin `source` alanı ilk dolu uca eşlenir. Arama çizimi değiştirmez.

```python
before = doc.to_bytes()
found = cad.arrays.find(doc, source=seed_insert_uid)
assert doc.to_bytes() == before
for candidate in found.matches:
    print(candidate.present, candidate.missing, candidate.deviation,
          candidate.overlaps, candidate.adoption.to_json())
print(found.excluded)  # UUID → neden; eligible listesinde olup adayda olmayanlar eşleşmemiştir.
if found.truncated:
    print("Kısmi sonuç; kapsamı daraltın.")
# Kullanıcı tek bir adayı incelediğinde:
proposal = found.matches[reviewed_index].adoption
args = dict(source=proposal.source, layout=proposal.layout, items=proposal.items,
            tolerance=proposal.tolerance, expected_revision=proposal.expected_revision)
review = cad.arrays.adopt.plan(doc, **args)
# Kabul edilen aynı öneri ayrı bir yazma işlemidir.
done = cad.arrays.adopt(doc, **args)
```

Bir öğenin dışarıda kalması, onun hangi eksik yere ait olduğunu ispatlamaz.
Elle taşınan öğe ancak kullanıcı belirli yere açıkça `ArrayItem(uid=..., kept=True)`
atayınca korunmuş sayılır. Her ikinci yer doluysa aradaki boşluk kendiliğinden
varsayılmaz. Aynı yere iki ayrı UUID düşerse rastgele biri seçilmez.

`uids=[]` yalnız kaynağı arar; `uids` verilmezse belge taranır. Toplam kapsam
2 048 nesnedir; tolerans metreyle varsayılan 0,000001, en çok 0,001'dir.
`doc.start("cad.arrays.find", cad.ArraysFind(source=seed_insert_uid))`
durdurulabilir sorgudur. MCP aynı komutu ve `drawing.job.*` yüzeylerini kullanır.
Diğer aileler açık `kind` seçimiyle aşağıda açıklanır.
Bağlanan doğru düzenini **Diziyi düzenle → Aralık** ile değiştirebilirsiniz.
Pozitif birimli uzunluk iki komşu yer arasındaki gerçek uzaklıktır; yön
korunur. Adet değişimi ayrı seçenektir. Mevcut `fit` düzeninin toplam boyu
aralıkla birlikte yeniden hesaplanır; yerinde tutulan öğeler taşınmaz.
[Yöntem, belirsizlik ve iş sınırları](../adr/0874-read-only-linear-array-patterns.md).


## Masaüstünde doğrusal dizileri inceleme

1. Çizimde tek bir blok yerleştirmesini seçin. Bloklar panelinde **Dizileri bul…**
   veya komut satırında `DIZIBUL` açın. Mevcut Blok grubunun ek menüsünde de
   **Doğrusal dizileri bul…** vardır.
2. Varsayılan **0,001 mm** toleransı ve kapsamı gözden geçirip **Bul** seçin.
   Arama sırasında çizimi kullanabilir, **Durdur** ile işi kesebilirsiniz.
3. Panelde dolu/boş yer sayısı ve aralık gösterilir. **Göster** mevcut blokları
   seçip yakınlaştırır; **İncele…** bağlama koşullarını denetler. Yer şeması
   geometrik önizleme değildir: ● mevcut, ○ boş, ◇ yerinde korunandır.
4. Kaymış bir bloğu da bağlayacaksanız **Elle taşınmış öğeyi koru…** bölümünde
   eşleşmeyen bloğu ve boş yerin numarasını seçip **Bu yere koru** kullanın.
   Yerler 1'den başlar; dolu yerin üzerine yazılmaz. Atama taslağı kaldırılabilir.
   Bu işlem bloğu taşımaz ve henüz çizime yazmaz.
5. **Diziye bağla** ile onaylayın. Mevcut kökler ve ölçü bağları korunur;
   eksik yerler boş kalır. Bağlama tek Undo/Redo adımıdır. Vazgeç çizime yazmaz.

Çizim aramadan sonra değişirse eski aday yazılamaz. **Yeni kaynak** ile
kaynağı yeniden seçip arayın. Kısmi sonuç, ortak öğe, ret ve iptal görünürdür.
Python işlem çıktısı açıksa arama aynı `cad.arrays.find` girdileriyle yazılır;
uygulama aynı native `cad.arrays.adopt` işini kullanır. Yerleştirmeleri
bağladıktan sonra mevcut **Diziyi düzenle → Aralık/Adet** kullanılabilir.
[Arayüz, iptal ve kabul kararı](../adr/0875-linear-array-review-panel.md).


## Dünya X/Y dikdörtgen örüntüleri

Aynı **Dizileri bul… / DIZIBUL** penceresinde **Düzen türü → Dikdörtgen (X/Y)**
seçin. Kaynak köşede olmak zorunda değildir. En az iki satır, iki sütun, dört
dolu yer ve her yönde gözlenen bir komşu çift gerekir. Dolu bir sınır köşesi
kaynak olur; eksik köşe doldurulmaz. Kart iki aralığı ayrı, inceleme şeması
yerleri satır satır gösterir. Kaymış bloğu korumak için boş yerin **Satır**
ve **Sütun** numaralarını seçin; kaynak köşeden 1 ile başlar. Dolu yere
atama reddedilir; gerekçe onay düğmelerinin üstünde görünür.

```python
# Eski çağrıda kind yoksa doğrusal arama aynen sürer.
found = cad.arrays.find(doc, source=seed_insert_uid, kind=cad.ArrayFindKind.GRID)
# Aynı şema ve inceleme/yazma sözleşmesi kullanılır.
proposal = found.matches[reviewed_index].adoption
print(proposal.layout.to_json(), found.truncated)
args = dict(source=proposal.source, layout=proposal.layout, items=proposal.items,
            tolerance=proposal.tolerance, expected_revision=proposal.expected_revision)
review = cad.arrays.adopt.plan(doc, **args)
# Kullanıcının onayladığı bu öneriyi ayrı çağrıyla bağlayın.
done = cad.arrays.adopt(doc, **args)
```

`doc.start("cad.arrays.find", cad.ArraysFind(source=seed_insert_uid,
kind=cad.ArrayFindKind.GRID))` ve MCP aynı durdurulabilir sorgudur. İki
milyonluk iş bütçesine ek olarak grid aralık keşfi 500 000 çift veya bir
eksende 128 aralık adayını aşarsa sonuç **kısmidir**. Uyarıyı tam tarama
gibi yorumlamayın. Dönük/eğik grid ve yol tanıma bu kapsamda bulunmaz. Kutupsal tanıma
aşağıdaki ayrı açık aile seçimidir. [Karar ve sınırlar](../adr/0876-axis-aligned-grid-pattern-review.md).

Büyük koordinatlarda kısa aralıkların tek komşu farkındaki yuvarlaması bütün
sıraya çoğaltılmaz: toplam gözlenen açıklık da denenir ve uç üyelik yeniden
taranır. Aday yine aynı gerçek yerleşim doğrulayıcısından, verdiğiniz mutlak
toleransla geçer; bloklar taşınmaz. Koordinatın temsil aralığının altındaki
toleranslarda bütün desenleri bulma garantisi yoktur; tolerans otomatik
büyütülmez. [Sayısal düzeltme ve ölçülen sınırlar](../adr/0877-full-span-pattern-spacing-validation.md).


## Tam çemberde kutupsal tekrarlar

**Düzen türü → Kutupsal (tam çember)** seçin. Bu, açık 360° yorumudur;
kısmi yay niyeti kendiliğinden çıkarılmaz. En az dört dolu yer ve gözlenen
komşu çift gerekir. Kaynak 1'dir; yerler dünya düzleminde saat yönünün
tersine sıralanır. Kart **Yön korunur** veya **Öğeler döner** davranışını
gösterir. İncelemede merkez X/Y, yarıçap, tam açı, açısal aralık ve boş
yerler ayrı görünür. Yarıçap blok taban noktasından ölçülür.

```python
found = cad.arrays.find(doc, source=seed_insert_uid, kind=cad.ArrayFindKind.POLAR)
proposal = found.matches[reviewed_index].adoption
assert isinstance(proposal.layout, cad.PolarArrayLayout)
print(proposal.layout.center, proposal.layout.fill, proposal.layout.rotate)
# Kullanıcı incelemesiyle aynı native plan/adopt akışına verilir.
args = dict(source=proposal.source, layout=proposal.layout, items=proposal.items,
            tolerance=proposal.tolerance, expected_revision=proposal.expected_revision)
review = cad.arrays.adopt.plan(doc, **args)
done = cad.arrays.adopt(doc, **args)
```

Kaymış öğe kendiliğinden hangi boş yere ait sayılmaz. Açık atama, boş yere
`kept=True` bağlar; kök taşınmaz. Dolu yerin üzerine yazılmaz. Adet değişince
boş/korunan yer ve mevcut UUID'ler mevcut dizi kurallarınca korunur;
Sıfırla ayrı kullanıcı kararıdır. Python Job ve MCP aynı komutu kullanır.
[Yöntem, sayısal sınırlar ve gerçek kabul](../adr/0879-full-circle-pattern-recognition.md).


## Açık kılavuz boyunca sabit aralıklı tekrarlar

Kaynak bloğu ve bir kılavuzu birlikte seçip **Dizileri bul…** açın;
**Yol boyunca (sabit aralık)** hazır gelir. Yalnız blok seçmişseniz düzeni
ve **Kılavuz yol** listesinden yolu açıkça seçin. Kaynak blok taban noktası
kılavuzun kayıtlı başlangıcında olmalıdır. Ters yön veya başlangıç ofseti
tahmin edilmez; uygun değilse nesneler taşınmadan gerekçe gösterilir.

En az üç dolu yer ve gözlenen komşu aralık gerekir. Kılavuz tek parçalı 2B
çizgi, yay, çember veya en çok 4096 köşeli çoklu çizgidir. Eksikler boş,
kaymış öğeler dışarıda kalır; açık koruma ataması yapılabilir. Kartta sabit
aralık ve teğeti izleme/yönü koruma ayrıdır. Son dolu öğeden sonra görünmeyen
yerler veya gizli yarım aralık varsayılmaz. Kılavuz korunur; sonradan
düzenlenince normal üyeler kimliklerini koruyarak izler.

```python
found = cad.arrays.find(doc, source=start_insert_uid,
                        kind=cad.ArrayFindKind.PATH, path=guide_uid)
proposal = found.matches[reviewed_index].adoption
assert isinstance(proposal.layout, cad.PathArrayLayout)
print(proposal.layout.path, proposal.layout.spacing, proposal.layout.align)
# Kullanıcı incelemesinden sonra aynı plan/adopt sözleşmesi kullanılır.
args = dict(source=proposal.source, layout=proposal.layout, items=proposal.items,
            tolerance=proposal.tolerance, expected_revision=proposal.expected_revision)
review = cad.arrays.adopt.plan(doc, **args)
done = cad.arrays.adopt(doc, **args)
```

Bu arama sabit aralık önerir; yolu adede göre bölme veya uzayınca yeni öğe
ekleme niyetini kendiliğinden çıkarmaz. Mevcut **Diziyi düzenle → Aralık/Adet**
ayrı işlemdir. Tek Undo/Redo, Python Job ve MCP aynı native uygulamadır.
[Karar ve gerçek kabul](../adr/0880-explicit-guide-pattern-recognition.md).


## Yazı çeşitleri ve iç bloklar

Aramada **Yazı değerleri farklı olabilir** ve **Blok öznitelik değerleri
farklı olabilir** ayrı seçeneklerdir. Yalnız değer karşılaştırmasını
gevşetir; yazı biçimi, blok tanımı ve diğer nesne verileri korunur.
Dönüşümde **Metin değerlerini blok öznitelikleri olarak koru** ayrıca
seçilmelidir; seçeneği değiştirince yeniden inceleyin. Her yerleştirme
kendi metnini `TEXT_n` alanında tutar. Son sıradaki düz tek satırlı yazıların
temsil edilemeyen font/maske/biçimi veya ilişkisi varsa ret gerekçesi görünür.
İç bloklar mevcut tanımı ve aynı öznitelik değerlerini korur; farklı iç
öznitelik değerlerini bir değere indirgeme yapılmaz.

```python
found = cad.blocks.find(doc, source=source, text_variants=True)
args = dict(name="Numaralı parçalar", source=source,
            groups=[found.matches[i].members for i in reviewed_indices],
            text_to_attributes=True)
review = cad.blocks.convert.plan(doc, **args)
for m in review.mapping:
    print(m.from_, m.insert, m.attribute if m.attribute else m.path)
# Kullanıcı kabul ettiğinde:
done = cad.blocks.convert(doc, **args, expected_revision=review.revision)
```

Metin eşlemesinde `attribute="TEXT_1", path=[]` vardır: öznitelik değeri
normal SubRef yaprağı değildir. Özniteliğe dönüştürülen yazıya bağlı ölçü
bu nedenle reddedilir. Mevcut iç geometrinin ölçü yollarına yeni parça yolu
eklenir ve dünya noktası tekrar denetlenir. Aynı örnek `doc.start` ile iş,
MCP ile `op=plan`/korunan `expected.plan` olarak kullanılabilir.
[Karar ve kesin sınırlar](../adr/0881-repeat-text-values-and-nested-blocks.md).
