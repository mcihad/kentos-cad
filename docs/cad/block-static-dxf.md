# Dinamik blokları DXF'e aktarma

KCAD dosyası parametreleri, görünüşleri ve formülleri saklar. DXF çıktısı
her yerleştirmenin o anda çizdiği geometriyi düz bloklara yazar. Dinamik
düzenleme davranışı taşınmaz; bunu kayıp raporunda `Dinamik blok` uyarısı
olarak görürsünüz. İç içe yerleştirmeler de kendi değerleriyle yazılır.

Önce `cad.file.export.dxf.plan(document, path=...)` çağrısıyla raporu
inceleyin; plan dosya oluşturmaz. Ardından aynı yol ile
`cad.file.export.dxf(document, path=...)` aktarımı yapar. Python belgesinin
ve sahnedeki yerleştirmelerin değerleri bu iki çağrıda da değişmez. MCP'de
aynı katalog komutlarının plan/execute yüzeyleri kullanılır.

Varsayılan görünüş durum listesinin ilkidir; bunun gizlediği parçalar,
yerleştirmede özel parametre yokken de gizlidir. Aynı değerleri taşıyan
çocuk referanslar aynı statik tanımı kullanır. Kullanılmayan kitaplık
blokları silinmez. Rapor sayıları dosyaya yazılan referans kayıtlarıdır;
örneğin bir üst blok on kez yerleştirildiyse tanımındaki tek çocuk kaydı
on ayrı kayıt olarak raporlanmaz.

Düz DXF tekrar açılınca geometrisini düzenleyebilirsiniz; önceki parametre
panelini geri getirmek için düzenlenebilir KCAD kaynağını kullanın.
[Karar ve doğrulama](../adr/0851-dynamic-block-default-visibility-and-nested-dxf.md).
