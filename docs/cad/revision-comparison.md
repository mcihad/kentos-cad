# 2B revizyon karşılaştırması

`cad.drawing.compare` salt okunurdur. `against` eski sürümü, açık belge yeni
sürümü belirtir. Model nesneleri kalıcı UUID ile eşlenir. `tolerance` metre
cinsinden yalnız model geometrisinin karşılaştırma toleransıdır.

`changedObjects.fields` içinde `blockDefinition`, `layerStyle`,
`dimensionPresentation`, `dimensionAssociation` ve `hatchAssociation` ilgili
semantik değişikliği belirtir. Yalnız ölçü sunumu veya tarama bağı değişimi
`attributes` olarak bildirilir; geometri uzaklığı üretilmez.

## Pafta ve görünüşler

`projectChanges` model nesnelerinden ayrı bir listedir. Boşsa cevapta yer
almaz; eski istemcilerdeki model sayıları aynı anlamı taşır. Liste öğeleri:

| Alan | Anlam |
| --- | --- |
| `kind` | `sheet`, `viewport`, `namedView`, `viewTemplate`, `sheetBook` |
| `id` | Kalıcı kimlik; viewport için `paftaKimliği/öğeKimliği` |
| `name` | Okunabilir ad; viewport için pafta ve öğe adı |
| `status` | `added`, `removed`, `changed` |
| `fields` | Değişen JSON alan yolları; örneğin `kind.view.scale` |

Viewport kimliğinin iki parçasında `/` ve `~`, JSON Pointer kuralıyla
`~1` ve `~0` olarak kaçırılır. Böylece kimlikler birbirine karışmaz.
Nesneler ve görünüşler listelerdeki sıralarıyla eşlenmez. Ad değiştirmek
kalıcı kimliği değiştirmediğinde tek bir değişikliktir.

Viewport'un ölçeği, oranı, kamera merkezi, dönüşü, kırpması, katman kapsamı,
görünüş kilidi, kâğıt çerçevesi ve yazdırma ayarları kendi kaydında
karşılaştırılır. Pafta; sayfa, kenar boşluğu, antet ve diğer kâğıt öğeleriyle
karşılaştırılır. Öğe sırası `itemOrder`, pafta sırası `book.sheetOrder`
olarak bildirilir. Kitabın ana sayfaları, değişkenleri ve resim içerik
kimlikleri de kapsamdadır; resim baytları karşılaştırma için kopyalanmaz.

Metadata alanları tam değerleriyle karşılaştırılır. Tolerans bir baskı
ölçeği değişimini gizlemez. `limit` model listeleri gibi bu listeyi de
sınırlar; kesilince `truncated=true` olur. `changed`, `added`, `removed` ve
`same` yalnız model nesnelerini sayar.

Python'da `kentos.cad.drawing.compare(...).project_changes`, MCP'de aynı
komutun `projectChanges` alanı okunur. Tipler Rust komut kataloğundan
üretilir; ayrı bir karşılaştırma algoritması yoktur.

Masaüstü bölünmüş görünümünde pafta/görünüş farkları ayrı bir sayaçta
gösterilir. Bunlar model nesnesi vurgusu üretmez. Sayaç değişen pafta veya
görünüşlerin adlarını KentOS UI ipucunda gösterir. Kitap aynı `Arc` ile
tutuluyorsa ve kayıtlı görünüşler/şablonlar değişmediyse metadata raporu
tekrar hesaplanmaz.

## Kapsam ve kaynak

AutoCAD'in resmî [DWG Compare belgesi](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-LT/files/GUID-2D69E78D-5C82-464F-B864-CD29D5720EB9.htm)
karşılaştırmanın model alanıyla sınırlı olduğunu belirtir. KentOS'ta pafta
ve viewport'ların ayrı karşılaştırılması CAD-ADV-15'in üretim gereğidir.
Bu rapor PDF piksel farkı veya paftanın görsel olarak aynı olduğuna dair
kanıt değildir; kaynaktaki yapılandırılmış değişiklikleri gösterir.

Farklı UUID taşıyan dosyalar için aday eşleme, inceleme notlarının yaşam
döngüsü ve seçilmiş farkı alma ayrı açık işlerdir. Bu teslim CAD-ADV-15'in
bütününün tamamlandığı anlamına gelmez.
