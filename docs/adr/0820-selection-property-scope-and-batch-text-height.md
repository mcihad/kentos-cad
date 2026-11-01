# 0820 — Seçimden güvenli toplu özellik düzenlemeye geçiş

Tarih: 2026-11-01. Kapsam: CAD-ADV-03'ün özellik kapsamı, sorgu ve toplu yazı yüksekliği bölümleri. Geçici nesne yalıtımı kabulü henüz tamamlanmadığından planın bütünü kapatılmaz.

## Karar

Özellik panelindeki tür seçimi yalnız paneli daraltır. Sahne seçimi, seçimin sırası ve kimlikleri korunur. Panel, toplam seçimi, özellik kapsamını, düzenlenebilir ve kilitli hedef sayılarını ayrı gösterir. Panelden **Özelliklerle seç…**, mevcut İfadeyle seç aracını aynı kapsam ve türlerle açar; yeni seçim motoru veya ayrı ifade dili yoktur.

Panelin önbellek ve hücre anahtarı belge oturumu, belge değişim sayacı, seçim sürümü ve özellik kapsamı sürümüdür. Hedef değiştiğinde aynı değer gösterilse bile eski hücre taslağı yeniden kullanılmaz. Önceden üretilmiş bir düzenleme olayı anahtarı güncel değilse yazmadan reddedilir. Geometri ve ortak değerler her tuşta yeniden ölçülmez.

Çoklu yazı seçiminde **Yükseklik** alanı, mevcut EditCell ve CAD uzunluk okuyucusunu kullanır. `35 cm` gibi birim açıkça yazılabilir. Enter veya odaktan ayrılma mevcut hücre sözleşmesindeki gibi işler; Escape taslağı bırakır. Boş/pozitif olmayan/geçersiz değer çizime yazılmaz. Panel kilitli hedefleri önceden sayar ve düzenlenebilir UID'leri tek `cad.texts.edit` isteğine verir.

## Komut ve otomasyon

`cad.texts.edit` yeni `height` işlemini alır: `uids`, sonlu ve pozitif `height` (gerçek model metresi). Metin, konum, açı ve diğer özellikler korunur. Bir yazı stiline bağlı metinde `height` yerel geçersiz kılması kaydedilir. Komuta doğrudan kilitli hedef verilirse tüm istek reddedilir; kısmi yazma yoktur. Validate/plan salt okunurdur; execute bir Undo adımıdır. Mevcut revizyon ve plan korumaları uygulanır.

`cad.entities.select` mevcut geometrik/liste kaynağını `properties` ile daraltabilir:

```json
{
  "region": {
    "kind": "properties",
    "source": {"kind": "all"},
    "query": {"textHeightMin": 0.3, "textHeightMax": 0.3}
  }
}
```

Geometrik kaynaklar ve model/pafta görünüm kapsamının mevcut görünürlük kuralları korunur. `properties` kaynakları iç içe geçirilmez. Aralık uçları dahildir. Kaynakta var olan UID sırası korunur. Boş sonuç bir yazma değildir. `leftOut` mevcut tür/katman/gizli sayaçlarıdır; yeni özellik koşulları için ayrı bir sayaç eklenmemiştir.

`assignedColor`, nesnenin atanan CAD rengi veya `byLayer` değeridir. `displayedColor`, katman mirası çözülmüş CAD boya değeridir; seçili nesnenin vurgu rengi, tema sonrası ekran pikseli veya CBS tematik sembol rengi değildir. Tematik çizimde bu sorgu şu anda açıkça reddedilir; yanlış renk sonucu verilmez.

`blockPath`, `cad.entities.pick` çıktısındaki sıfır tabanlı tanım indekslerini alır; örneğin `[0, 0]`. Her iç içe yerleştirmede kendi/ByLayer/ByBlock mirası ortak `resolve_look` ile çözülür. Çözülmüş düzleştirilmiş renkten kaynak tahmin edilmez. Çekirdeğin `piece_chain` okuyucusu `piece` ile aynı yerleştirilmiş/dinamik tanımları okur ve kaynak görünüş verisini korur. Bu yolda sonuç **sahnedeki sahibi olan yerleştirmenin UID'sidir**; tanım üyesi düzenlenmez. Parça yolu yalnız renk koşulları içindir; yükseklik/ifade ile birlikte kullanımı reddedilir. Yol bulunamazsa o yerleştirme eşleşmez. Bu bir tanım parçasının CAD boya sorgusudur, kırpma/örtüşme sonrası piksel görünürlüğü sorgusu değildir.

Alt nesne seçilmişken panel parça yolunu, düzenleme hedefini, atanan ve görünür renkleri gösterir. Bloğu düzenle, var olan tanım düzenleyicisidir. Xref tanımına salt okunur parça incelemesi bir kaynak dosyayı yazma yetkisi vermez. Yüklenmemiş veya bulunamayan parça uydurulmaz.

Python'ın üretilen `PropertiesSelectRegion`, `SelectProperties` ve `HeightTextEdit` tipleri ve MCP katalog şeması aynı komutlara gider. Örnek:

```python
from kentos import cad

query = cad.PropertiesSelectRegion(
    source=cad.AllSelectRegion(),
    query=cad.SelectProperties(text_height_min=.3, text_height_max=.3),
)
hits = cad.entities.select(doc, region=query)
# Kilitli UID'leri uygulamanın önceden gösterdiği gibi ayırın; doğrudan API bunları atomik reddeder.
cad.texts.edit(doc, edit=cad.HeightTextEdit(uids=editable_uids, height=.35))
```

İfadeyle seç'in gelişmiş CAD koşulları aynı `refine_properties` okuyucusunu kullanır. Tüm/görünen/katman/seçili kaynakları ve yeni/ekle/çıkar/kesiştir davranışı mevcut aracınkidir. Arayüzle tam aynı çağrı `cad.processing.run(tool="selection.byExpression", parameters=...)` ile yapılır. Processing'in mevcut geometri ifadeleri metre okur; `SelectProperties.expression`, katman süzgeci gibi projenin çizim birimini okur. Birinin sayısal ifadesi diğerine birim dönüşümü olmadan kopyalanmış eşdeğer sorgu sayılmaz.

Önizlemedeki ifade sayacı ile çalıştırıldığında uygulanan CAD özellik koşulları ayrılır. 20.000 nesnelik örneklem sınırı toplam kapsam sayısı gibi gösterilmez. Salt seçim araçlarında çıktı katmanı/çıktı yeri kontrolleri gösterilmez.

## Renk girişi

Renkler kod yazma alanı değildir. Ortak KentOS ColorPicker bileşeni, adlandırılmış
CAD renkleri ve renk örnekleriyle birlikte sunulur. **Renk koşulu yok** sorguyu
renkle sınırlamaz. Atanan renk için **Katmana göre** ve **Bloğa göre** ayrı
seçeneklerdir. **Özel renk…** doygunluk/parlaklık düzlemi, ton şeridi ve hazır
renk paletini açar; hex alanı burada isteğe bağlı ileri giriş olarak kalır.

Processing parametresinin `color_picker` sunum bilgisi sıradan string API değerini
değiştirmez. Veriyle tanımlanan parametrelerde `widget: "color"` ve
`colorInheritance: true/false` okunur. Araç adımından yeni model girdisi oluşturulup
JSON ile paylaşılırken bu bilgi korunur. Araç formu, adımın sabit değeri, model
girdisinin varsayılanı ve model çalıştırma formu aynı renk bileşenini kullanır.

## Seçim tarifleri

İfadeyle seç ve Konuma göre seç pencerelerindeki **Seçim tarifini model olarak kaydet…**, mevcut model tasarımcısını tek adımlı bir taslakla açar. Koşul, CAD sınırları, kapsam ve seçim birleştirme biçimi korunur. Modelin var olan kaydet/içe aktar/dışa aktar yapısı kullanılır. Çizim veya katman oluşturulmaz. Sabit UID veya katman kimliği kullanan tarifin kapsamı başka çizime uygun hâle getirilmelidir; bunun yerine dinamik `all`/`selection` kapsamı seçilebilir.

## Doğrulama

- Masaüstü: 1564 geçti, 256 açıkça ignored; Processing kitaplığı: 10 geçti. macOS test soketi yol uzunluğu için testler `TMPDIR=/tmp` ile çalıştırıldı.
- Native uygulama: 3 test; normal ve iki düzey iç içe blokta atanan/görünür renk farkı, aralık/ifade denetimi, salt okunur plan, atomik kilit reddi, tam Undo.
- Python: 4 test; 50.000 nesnede 5.000 hedef, Processing/komut UID eşliği, dört seçim birleştirme biçimi, 4705 düzenlenebilir yazı, save/open, Undo/Redo, kilit/geçersiz yükseklik/revizyon reddi.
- MCP: gerçek Server tools/call üzerinden aynı özellik sorgusu ve Processing seçimi; salt okunur plan, toplu yükseklik ve Undo.
- Clippy: etkilenen dört kitaplıkta `-D warnings` geçti. Python SDK `--check` ve hedef TS tip dışa aktarımları geçti.
- Gerçek macOS uygulamasında 50.000 nesne seçildi; panel yalnız 10.000 yazıya daraltıldı ve sahnedeki 50.000 seçim korundu. 0,30 m yüksekliğinde 5000 yazı seçildi; 35 cm girişi 4705 kilitsiz yazıyı değiştirdi, 295 kilitli yazı korundu. Kaydedilen dosyanın bağımsız UID/özellik karşılaştırması yalnız yükseklik değiştiğini doğruladı. Tek arayüz Undo'su sonrası kaydedilen dosyanın 50.000 nesnesi bütün özellikleriyle önceki hâliyle eşit bulundu.
- Gerçek Enter/Escape, negatif giriş, birimin ilk karakterleri, tür kapsamı, kayıtlı seçim tarifini çalıştırma, sonuçtan sahneye yakınlaştırma ve üç iç içe blok renk kaynağı kullanılarak sınandı. Hedef değişiminde kuyrukta kalmış olaylar otomasyonla ayrıca sınandı.
- Test çizimleri, ekran görüntüleri ve günlükler repo dışında `/tmp/kentos-cad-advanced-03-2026-11-01/` içindedir. Depoya ekran görüntüsü eklenmez.

## Henüz kapanmayan kabul

Kalıcı katman süzgecinden ayrı geçici **nesne** yalıtımı ve yalıtımı bitirme akışı tamamlanmalıdır. Var olan Katmanı yalıt/Yalıtımı kaldır bunun yerine tamamlanmış nesne yalıtımı sayılmaz. 1100 mantıksal piksel genişliğindeki gerçek pencerede panel ve blok renk sorgusu sınandı; atanan renk 1/3, aynı görünür renk 2/3 hedef buldu. Seçim değiştirirken açık hücre taslağının gerçek etkileşim tekrarı da plan kapatılmadan tamamlanır. Bu kayıt bu kalan işleri tamamlanmış göstermez.

## Kaynaklar

- [QCAD Property Editor](https://qcad.org/en/manual/scripts/widgets/propertyeditor/doc/propertyeditor/): karma seçimde türle özellik kapsamı.
- [QCAD Selection Filter](https://www.qcad.org/en/manual/scripts/pro/widgets/selectionfilter/doc/selectionfilter/): kaynak/görünen renk ayrımı ve seçim birleştirme biçimleri.

Renk seçici eki: Processing paketinin 72 testi geçti (9 ignored), masaüstünün
Processing bölümünde 98 test geçti (18 ignored). Görsel palet, adlandırılmış
renkler ve Katmana göre seçimi gerçek uygulamada açılıp kullanıldı. Görsel
olarak seçilen kırmızı, üç iç içe bloktan doğru ikisini; Katmana göre
koşuluyla doğru birini buldu. 1100 mantıksal piksel genişliğinde renk
alanları ve açılır palet taşmadan kullanıldı. Siyah hazır renk, tuvalin
adaptif `ink` belirteci yerine gerçek `#000000` kullanır. Model
renk girdisinin JSON dönüşünde picker bilgisinin korunması ayrıca sınandı.
