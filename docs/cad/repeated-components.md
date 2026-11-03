# Tekrarlanan 2B bileşenleri bulma

Tek bir bileşenin çizgi, 2B çoklu çizgi, çember ve yaylarını seçip Bloklar panelindeki
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
çizgi, ek parçalar, metin, blok ve NURBS kaynakta açıkça reddedilir;
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

İlk dönüşüm kapsamı tek, veri ve tür şeması olmayan CAD katmanındaki sürekli
çizgili nesnelerdir. Öznitelik/sembol/ek dosya, özel çizgi tipi, parçalara
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
İlk arama dilimi grid/kutupsal/yol örüntülerini içermez.
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
