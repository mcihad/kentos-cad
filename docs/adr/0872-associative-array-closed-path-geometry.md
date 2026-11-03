# ADR 0872 — İlişkili dizinin kapalı yolu gerçek geometrisiyle okuması

Tarih: 3 Kasım 2026. CAD-ADV-07'nin mevcut dizi modelini kullanma önkoşulu.
Örüntü tanıma/benimsetme veya kaynak türü genişletme tamamlanmış sayılmaz.

## Dayanak ve mevcut davranış

[AutoCAD ARRAYEDIT](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-MAC/files/GUID-1D0F6016-F4DC-44BA-A517-B116D8CFC141.htm)
yolda eşit dağıtma ile sabit aralığı ayırır; yol değişince yerleşimin yeniden
hesaplanmasını, öğe istisnalarını ve açık reset eylemini tanımlar.
[BricsCAD V27 ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
örüntü seçimi/dönüşümünü ayrı tutar. KentOS'taki bu dilim yeni tanıma algoritması
eklemez; mevcut ilişkili dizinin geometrik tutarlılığını düzeltir.

`array::path_maps` ve etkileşimli `ArrayPath` önizlemesi, native geometri
okuyucusuyla çoklu çizginin kapalı durumunu biliyordu. Belgenin
`arrays::placements` hesabı ise eski JSON/WASM okuyucusunu kullanıyordu;
bu okuyucunun tarihî varsayılanı `closed` bilgisini bırakıyordu. Sonuç:
4×4 m kapalı karede 8 öğe önizlemede dört kenardayken, ilişkili yazmada
12 m'lik açık yol kabul edilip üç kenara dağılıyordu. Düzenleme ve yol
izleme de aynı hatalı yerleşim hesabını kullanıyordu.

## Karar

Yalnız `kentos_domain::arrays::core` çağrısı `Entity::from_document_json`
kullanır. ADR 0870'te ölçüler için ayrılan native okuyucu burada da geçerlidir.
Kapalı durum, analitik son yay kenarı ve kayıtlı genişlikler korunur.
Eski JSON/WASM sözleşmesinin varsayılanı, `AssocArray`/`ArrayItem` modeli,
yerleştirme UUID'leri ve KCAD şeması değiştirilmez.

İlişkisiz önizleme, ilişkili yazma, `cad.arrays.edit` ve yol değişiminde
izleme aynı kapalı geometrinin uzunluğunu kullanır. Eşit dağılımda kapalı
yolun başlangıcı son öğe olarak yinelenmez. Sabit aralık son kenardaki
istasyonları da kapsar; yay noktası ve hizalama teğeti analitik hesaplanır.
Elle değiştirilmiş öğenin `kept`, silinmiş öğenin boş kimlik semantiği ve
yalnız açık Reset ile geri oluşturma davranışı korunur.

Dosya açılışına toplu bir yeniden yerleştirme/göç eklenmez. Eski hatalı
yerleşimler, kullanıcı dizinin yerleşimini veya bağlı yolu düzenlediğinde
mevcut düzenleme/izleme yolu üzerinden hesaplanır; açmak tek başına yazma değildir.

## Kabul

- Üç yeni domain testi önce eski kodda başarısız oldu: dört kenardaki
  sekiz istasyon, iki yönde kapanış yarım çemberinin noktası/teğeti,
  açılıp kapanan yolun aynı UUID'li öğelerinin tek adımda izlenmesi ve Undo/Redo.
  Düzeltmeyle domain paketinin 140 testi geçti; bir isteğe bağlı test atlandı.
- Bağımsız ortak senaryo, önizlemedeki çemberleri gerçek Insert yerleriyle
  karşılaştırır; yol aç/kapat, aralık değişimi ve Undo/Redo içerir. Toplam
  136 dizi komut durumu native ve Python ortak koşucularında geçti.
- Python'un dört dizi testi; mm projede metre geometrisi, dosya aç/kaydet,
  korunan UUID'ler, iki silinmiş öğe, elle taşınmış istisna, adet değişimi,
  açık Reset ve iki yöndeki kapanış yayını sınar. MCP'nin 70 testi geçti;
  yeni kabul, gerçek protokolde plan/yazma eşitliği, aç/kapat ve kaydedilmiş
  Undo/Redo dosyalarının eşitliğidir.
- Masaüstü derlemesi ve ilgili paketlerin tüm hedeflerinde Clippy geçti.
  Gerçek fare/klavye kabulü ve dosya karşılaştırmaları CAD planındaki kayıtla
  birlikte tutulur; ekranlar ve ham kayıtlar depo dışındadır.
