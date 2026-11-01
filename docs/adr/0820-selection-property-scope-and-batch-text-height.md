# 0820 — Seçimden güvenli toplu özellik düzenlemeye geçiş

Tarih: 2026-11-01. Kapsam: CAD-ADV-03; özellik kapsamı, sorgu, güvenli tek/toplu yazı yüksekliği ve geçici nesne yalıtımı.

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

## Geçici nesne yalıtımı

`cad.entities.isolate` kaydedilmeyen, belge oturumuna ait bir görünüm komutudur.
`action.kind` değerleri `isolate`, `end` ve `query` olur; `isolate` geçerli,
boş olmayan bir `uids` listesi ister. Bütün kimlikler değiştirmeden önce
kontrol edilir; geçersiz/bilinmeyen hedef isteği atomik reddeder. `plan` salt
okunurdur. Sonuçtaki `hidden` ve `retained` yalnız bu maskeyi sayar;
katman, evre veya başka görünürlük koşulları ayrıca uygulanır.

Maske kalıcı UID'lerle tutulur. Yeniden yalıtma önceki maskeyi değiştirir;
yeni çizilen nesneler görünürdür, silinip Undo ile geri gelen gizli nesne
aynı UID ile gizli kalır. Katman gözleri, kilitleri, kalıcı süzgeçleri ve
evreleri değiştirilmez. Gizli katmandaki bir nesne yalıtılarak zorla açılmaz.
Çizim, etiket, yakalama/seçim ve Processing'in görünür kaynakları aynı
maskeyi kullanır. Processing modelinin açık `Ids` girdisi önceki sözleşmesi
gereği görünürlükten bağımsızdır; bu kaynak sessizce yeniden yorumlanmaz.

Yalıtım dosyaya, otomatik kurtarma kopyasına veya geometrik Undo geçmişine
yazılmaz; belgeyi kirletmez, geometri revizyonunu artırmaz, Redo'yu silmez.
Yeniden açılan belge yalıtımsızdır. Komut anlamsal günlüğe ve Python yankısına
normal komut yolu ile gider. PDF/pafta/atlas/preflight ve pafta önizlemesi
geçici editör maskesini dikkate almaz; kalıcı proje görünüm ayarları korunur.

```python
from kentos import cad

cad.entities.isolate(doc, action=cad.IsolateIsolationAction(uids=selected_uids))
state = cad.entities.isolate(doc, action=cad.QueryIsolationAction())
cad.entities.isolate(doc, action=cad.EndIsolationAction())
```

MCP aynı `cad.entities.isolate` şemasını kullanır. Masaüstünde paletin yalıtım
simgesi, sahne sağ tuş menüsü ve ISOLATEOBJECTS/UNISOLATEOBJECTS komutları aynı
yola gider. AutoCAD'in resmî palet ekranındaki üst satır incelenerek tür/sayı
kutusu, özelliklerle seçim simgesi ve yalıtım simgesi tek satırda toplandı.
Ayrı metinli araç satırı, ikinci bir “…” menüsü ve tekrarlayan tür/katman
başlığı yoktur. Simgelerin açıklamaları vardır. Yalıtım açıkken simge etkin
vurgulu olur ve **Nesne yalıtımını bitir** işlevine döner; seçim boşalsa da
erişilebilir kalır. Yalnız açık yalıtımda kısa durum satırı, kilitli hedef
varsa düzenlenebilir/kilitli sayısı gösterilir. Karma tür seçimi yalnız
özellik kapsamını daraltır; sahne seçimi korunur. Nesne adı/katman ve tam
tür özeti seçici açıklamasında korunur.

## Kalan kabulün doğrulanması — 2026-11-01

- Native iki ek test: salt okunur plan, dosya/revizyon/geometri geçmişinin
  değişmemesi, yeni nesne, silme/Undo, kalıcı UID, hatalı hedefin atomik reddi,
  yeniden açma ve Redo'nun korunması.
- Masaüstü: 1566 test geçti (256 ignored); çizim/picking/etiket görünürlüğü
  ve çıkışın maskeden bağımsızlığı ayrıca sınandı. Beş etkilenen kitaplıkta
  Clippy `-D warnings` geçti.
- Python: beş test geçti. 50.000 nesnede 5.000 hedef yalıtılıp 45.000 gizli
  sayıldı; aynı hedefler sorgu ve Processing ile bulundu. Yükseklik değişimi,
  Undo/Redo, maskeyi bitirme ve kaydedilmeyen görünüm doğrulandı.
- MCP'nin gerçek Server testi: plan/isolate/query/end, Processing hedefleri,
  belge değişmezliği ve toplu düzenleme aynı komutlarla geçti.
- Gerçek PDF üretimi regresyon testi: geçici maskeyle bütün editör nesneleri
  gizliyken bile, önceki PDF'nin baytları ve preflight bulguları birebir aynı.
- Gerçek uygulamada A yazısının açık hücresine **77 cm** girilip B seçildi.
  Odaktan çıkan taslak yalnız A'ya **0,77 m** yazıldı; B ve kilitli C **0,50 m**
  kaldı. B'de **88 cm** taslağı Escape ile bırakıldı. Kaydedilen dosyanın
  bağımsız karşılaştırması yalnız A yüksekliğinin değiştiğini doğruladı.
- Bu tekrar tekli yazı yüksekliği alanının birim ekini eskiden atabildiğini
  ortaya çıkardı. CAD tekli alanı da toplu alanla aynı fiziksel birim okuyucusuna
  geçirildi; cm/mm/ft ve atomik hatalı giriş testleri eklendi.
- A yalıtıldı; gizli B'nin yerinde gerçek tıklama seçim üretmedi. Seçim boşken
  bitirme eylemi erişilebilir kaldı. Yalıtım sırasında kaydedilen dosya,
  yalıtım öncesi belgeyle bütün baytlarıyla eşit çıktı. Bitirince B, kilitli C
  ve çizgi döndü; başlangıçta gizli katman gizli kaldı.
- ISOLATEOBJECTS komut aramasıyla A tekrar yalıtıldı; **100 cm** Enter ile
  **1,000 m** oldu. Undo yüksekliği **0,770 m** yaptı ve yalıtımı korudu.
  Yalıtımı bitirdikten sonra Redo tekrar **1,000 m** yaptı. Son iki Undo ve
  kayıt sonunda bütün belge, başlangıçla birebir eşit bulundu.
- Son palet düzeni gerçek uygulamada tekrarlandı: dört sahne nesnesi korunurken
  yazı türüyle üç hedefe daraldı, iki düzenlenebilir/bir kilitli sayıldı.
  Küçük sorgu simgesi mevcut seçili/yazı kapsamını açtı. Yalıtım simgesi
  çalıştırıldı; seçim boşaltılıp panel minimum 240 mantıksal piksele daraltıldı.
  Seçici ve iki simge taşmadı; etkin simge tek tıkla yalıtımı bitirdi.
  Processing paketinin 72 testi geçti (9 ignored).
- Görüntüler, karşılaştırma dosyaları ve günlükler repo dışında
  `/tmp/kentos-cad-advanced-03-finish-2026-11-01/` içindedir.

## Kaynaklar

- [Autodesk Isolate/Hide Objects](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-8F36228C-1CAD-4993-96F6-7FF679878E47.htm): katman yalıtımından ayrı, oturumluk nesne görünürlüğü.
- [Autodesk OBJECTISOLATIONMODE](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT/files/GUID-B4ED98BE-62D0-4982-82A2-87B744C56F99.htm): geçici/kalıcı görünüm ayrımı; bu uygulama geçici olanı seçer.
- [AutoCAD Properties Palette ve ekran örnekleri](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-DidYouKnow/files/GUID-94C065AB-FF9E-4752-B778-23D2FBB87E18.htm): üstte tür/sayı seçici ile küçük seçim araçları; tür kapsamı sahne seçimini değiştirmez.
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
