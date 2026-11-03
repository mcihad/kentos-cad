# Dinamik blokları DXF'e aktarma

KCAD dosyası parametreleri, görünüşleri ve formülleri saklar. DXF çıktısı
her yerleştirmenin o anda çizdiği geometriyi düz bloklara yazar. Dinamik
düzenleme davranışı taşınmaz; bunu kayıp raporunda `Dinamik blok` uyarısı
olarak görürsünüz. İç içe yerleştirmeler de kendi değerleriyle yazılır.
Masaüstündeki DXF/DWG aktarım penceresi bu kaybı dosya yolu seçilmeden
önce gösterir. Yalnız statik nesneler seçilmiş olsa da yazılan kullanılmayan
dinamik kitaplık tanımları için uyarı verilir.

Bağlı ölçünün mevcut değeri ve çizimi yazılır; kaynak geometriyi izleyen
ilişkisi aktarılmaz. Geri açılan dosyada geometri değişince ölçü kendiliğinden
güncellenmez. Pencere bunu önceden, plan/yürütme raporu `Ölçü bağı` uyarısıyla
bildirir. KentOS verisi seçeneği stili ve görünüşü korur; bu ilişkiyi koruduğu
anlamına gelmez. Kaynağı KCAD olarak saklayın.

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
[Yazım öncesi uyarı ve ilişki sınırı](../adr/0865-static-cad-export-behavior-loss-disclosure.md).
