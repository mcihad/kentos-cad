# ADR 0687: Kaynağa bağlı mahal kotu etiketleri

- Durum: bu sınırlı dilim kabul edildi (26 Ekim 2026); ARCH-SPACE-02 bütünü açık.
- Kapsam: plan/architecture.md ARCH-SPACE-02; mevcut etiket komutları.
- Web uygulaması ADR 0228 uyarınca ertelenmiştir; yeni TS dosyası gerekmez.

## Karar

Mevcut `content` metnine iki yer eklenir: `{floorElevation}` mahalın bilinen
katının `elevation` değeri; `{ceilingElevation}` buna yalnız mahalın açık
`height` değerinin eklenmesi. Katın floor-to-floor yüksekliği tavan yüksekliği
olarak varsayılmaz. Kat/mahal yüksekliği bilinmiyorsa ilgili yer boş yazılır;
bilinen sıfır `0.000`'dır. Mahalın kendi düşey offset'i ve katmanlı döşeme
modeli henüz yoktur; bitmiş döşeme/asma tavan geometrisi iddiası değildir.

Hesap mevcut SI/f64 modelinde, yazım diğer etiket uzunlukları gibi çizimin
birimi ve basamağındadır. Varsayılan şablon, genel yükseklik/hacim fallback'i,
birimler, ölçek ve renderer değişmez. KCAD alanları ve kodlama değişmez;
bağımsız okuyucunun izin verdiği şablon yerleri güncellenir. Eski bir KentOS
sürümü yeni yer adlarını bilmediğinden bu yeni şablonu reddedebilir. Kat/numara/ad/alan aynı şablonda
kullanılabilir. Yeni kot alanları yalnız mahalde anlamlıdır.

Kaynak düzenlemesi mevcut follower/tek undo yolunu kullanır. Proje ayarı
normalde model undo'suna ait olmadığından kot şablonlarının türetilmiş yazısı
ayar/dış değişiklik aktarımında ve undo/redo/iptal sonrasında güncel kaynaktan yerinde
yenilenir. Bu yenileme etiketin elle konmuş yerini, kolunu, UUID'sini veya
geometrisini değiştirmez. Başka şablonların geçmiş davranışı korunur.

## Gerçek GUI'nin bulduğu yükseklik satırı

Açık18 görünümünde `3.000 (katın)` düzenlenebilir sayısal hücreye sığmadı.
Yalnız CAD mahalında sayı hücresinde kalır; kaynağın katlar arası yüksekliği olduğu
ayrı, kısa `Katlar arası` notunda söylenir. Uzun tavan kuralı etiket aracının
yardımında kalır. GIS satırı, model ve edit komutu korunur.

## Araştırma ve sınır

[Graphisoft mahal oluşturma](https://helpcenter.graphisoft.com/user-guide/76551/)
mahal tabanının katla bağlantısını, üst kat bağlantısını ve sabit yüksekliği
ayrı modeller. [Autodesk Room Instance Properties](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-21326970-0037-41C6-A996-980C24EE019F.htm)
kat, taban offset'i ve üst sınırı ayırır. Bunlar ürün karşılaştırmasıdır;
KentOS'un bugünkü iki alanını daha zengin BIM modeliyle eşitlemez.

## Kabul adımları

1. KentOS hesabını çağırmayan Decimal referans: sıfır/negatif/üst kot,
   eksik mahal yüksekliği, atanmamış/bilinmeyen kat × beş çizim birimi;
   validate/plan/execute/undo/redo ve bütün nesne korunumu.
2. Python üzerinden kaynak yüksekliği/kat/ayar, taşıma, tek undo/redo,
   sonradan değişmiş ayar altında geçmiş, iptal ve KCAD yeniden açma.
3. İlgili application/domain/interaction/SDK/MCP/katalog drift ve clippy/build.
4. Değişmez test paketinde gerçek GUI: şablon, açık yükseklik, yerinde takip,
   ret/Escape/tek onay/undo/redo/kaydet-aç; ham ekran ve bağımsız disk modeli.

[Kabul kaydı](../baseline/2026-10-26-architecture-tag-elevations/README.md):
195 ortak durum, 309 Python, 697 interaction/219 application/67 domain,
bir gerçek MCP testi; özellikler 42 başarılı/4 mevcut atlanan, clippy/build
ve contracts WASM geçti. Gerçek GUI 19 ham görüntü/on bağımsız tam kayıt:
tek Enter/ret/undo/redo/iptal/kaydet-aç, açık18 geniş ve koyu13 geniş-dar.
Tam mimari matris, ayrı net alan ve daha zengin düşey model açık kalır.
