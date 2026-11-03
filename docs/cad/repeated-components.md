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
