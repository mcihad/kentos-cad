# Parametrik 2B tasarım — mevcut destek ve CAD-ADV-05 işleri

2026-11-02. CAD-ADV-05 **tamamlanmadı**. Bu belge kod sınırını,
doğrulanan ilk arayüz dilimini ve kalan gerçek geometri işlerini ayırır.

| Geometri / yetenek | Kodda mevcut davranış | Açık iş |
|---|---|---|
| Nokta / çizgi | Konum, düz kenar, uzaklık, yön ve geometrik bağlar | Birleşik sürükleme/dal kabulünü genişlet |
| Daire / bağımsız yay | Merkez/yarıçap, yay uç açıları; teğet/eşitlik/ölçüler | Büyük koordinat ve kilitli referans birleşik kabulü |
| Çoklu çizgi / alan | Her halka/parçanın köşeleri bilinmeyen; düz kenarlar Segment | Yaylı kenarın bulge/yarıçapı sabit, henüz değişken değil |
| Elips | Merkez ve mevcut Start/End (tam elipste dikiş); gerçek yarı eksen/açı bilinmeyenleri, dual türevler | Genel teğet, ayrı eksen noktaları, gripte kısmi tutuş kabulü |
| NURBS | Açık/periyodik olmayan ve iki ucu tam clamped eğride Start/End; dört uç koordinatı, iç kontrol noktaları sabit | Genel kontrol/teğet/G2 ve diğer uç yapıları bu adaptörde yok |
| Blok yerleştirmesi / iç nesne | Yerleştirme noktası Node(0), yalnız iki konum bilinmeyeni; ayna/açı/ölçek ve kaynak korunur | İç parça, dönme ve ölçek bilinmeyenleri bu adaptörde yok; birleşik GUI/Python/MCP ve grip/Undo kabulü geçti |
| Parametre ifadeleri | Uzunluk/açı/sayı, ad değiştirme, token bağımlılığı, döngü, sıfıra bölme | Boyut cebiri ve uygunsuz birim karışımı ayrıca denetlenmeli |
| Teşhis | `cad.constraints.query`: serbestlik, hareket edebilenler, fazla/çelişen/tekil/kopuk | Sahnedeki serbest yönlerin gösterimi ve büyük problemde durdurma |

## Doğrulanan ilk arayüz dilimi

- Parametreler aynı query'nin teşhisini gösterir. Serbest geometriye gitme,
  kısıt adından ilgili nesneyi seçme ve yakınlaştırma belgeyi değiştirmez.
  Serbestlik yalnız çözücüye katılan geometri içindir; projedeki bütün
  nesnelerin global serbestlik derecesi olduğu iddia edilmez.
- Adı seçince tanımlayan ve dolaylı etkilenen parametreler görünür;
  geometriyi göster eylemi bağlı ölçülerin kaynaklarını seçer. İfadeler
  ortak parser ile okunur; ad alt dizisi araması yapılmaz. Çevrimli
  dış veri gelse de grafik gezintisi durur.
- Yeniden adlandır, mevcut `cad.parameters.edit rename` komutunu kullanır.
  Geometriyi gösterip aynı çizimin penceresine dönünce taslaklar kalır.
- Yeni kullanıcı parametresinde uzunluk/açı/sayı açık seçilir. Sonuçta gerçek
  proje birimi (mm/cm/m/in/ft, °/gon) yazılır; çok küçük değer sıfır
  gibi gösterilmez. Gösterim yuvarlaması saklanan değeri değiştirmez.
  Başvuru parametresi yalnız ölçer ve ifade alanı düzenlenemez.
- Teşhis ve ayrıştırılmış bağımlılıklar çizim oturumu/sürümü/generation
  için önbelleğe alınır. Tuş başına çözücü yeniden çalışmaz. Undo ve
  yeniden açılan başka çizim eski teşhisi kullanmaz.
- Hata/sonuç açıklaması üsttedir, seçilen ayrıntı tablonun altındadır.
  Sadece değişen satırda Uygula çıkar; Sil seçilen parametreye aittir.

## Kod kanıtı

- `geometry-core/src/ops/constraints/model.rs`: bilinmeyen düzeni;
  `element_vars` yaylı Segment'i reddeder; elips 5/7, sınırlı NURBS uç modeli 4, Insert yerleşimi 2 bilinmeyen alır.
- Aynı dizindeki `rules.rs`, `solve.rs`: dual türevler, bağımsız gruplar,
  teşhis ve en az değişim çözümü. Yeni bir motor kurulmayacak.
- `native/domain/src/constraints.rs`: kalıcı başvuruların adaptörü;
  `core_point` Along/Mid'i çözücü noktasına çevirmiyor.
- `native/application/src/constraint_solve.rs`, `constraints_query.rs`:
  birim dönüşümü, gerçek ölçü ve aynı teşhis; kilitli nesneler tutulur.
- `native/application/src/parameters_edit.rs`: atomik rename/set,
  geçersiz ifade ve çelişkide son geçerli model.
- `desktop/src/parameters_window.rs` ve `parameters_window/analysis.rs`:
  yukarıdaki arayüz, salt okunur ortak query ve önbellek.

## Araştırma ve sonraki kabul

[AutoCAD dimensional constraints](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-F7ED2587-8F25-4770-BD13-C7C95456E707.htm)
sürücü ile yalnız ölçen referansı ayırır.
[BricsCAD Parameters Manager](https://helpcenter.bricsys.com/en-us/document/panels/parameters-manager-panel?version=V25)
tanımlayan/etkilenen parametreleri inceleme, ad değiştirme ve ilgili
geometriyi seçme örneğidir. KentOS kendi mevcut komut ve çözücüsünü
kullanır; bu kaynakların yeteneklerinin tamamı varmış gibi gösterilmez.

CAD-ADV-05 kapanmadan yaylı kenar/elips kapsamı, ölçü boyutları, aynalı
blok ve geçici sürükleme/çözüm dalı davranışı tamamlanmalı; teknik
plakada çelişki, kilit, gevşetme, Undo ile UI/Python/MCP aynı sonucu
vermelidir. Güvenli ret bir adaptörün uygulanması yerine sayılmaz.

Elips nokta adaptörünün sınırları ve kabulü: [ADR 0829](../adr/0829-native-ellipse-point-constraints.md).

Başarı sonrası durağan imleç ve birimli sonuç kabulü: [ADR 0830](../adr/0830-constraint-success-feedback-and-parameter-units.md).

Elipsin kendi merkez–uç yönünde ekseni çökertmeden çeyrek dönüşü, kilit/çelişki ve UI/Python/MCP/Undo kabulü: [ADR 0831](../adr/0831-ellipse-ray-direction-without-axis-collapse.md).

Açık NURBS uçlarının 6→8 m, bağlı çizgi, kapsam notu, dosya/Undo ve UI/Python/MCP kabulü: [ADR 0832](../adr/0832-bounded-nurbs-endpoint-constraints.md), [Python/MCP kullanımı](nurbs-constraints-python.md).

Aynalı/dönük blok yerleştirme noktası ve gerçek grip kabulü: [ADR 0833](../adr/0833-block-insertion-anchor-constraints.md), [Python/MCP kullanımı](block-anchor-constraints-python.md). İlk seçilen sabit nesnenin geçici hold altında gereksiz raporlanması [ADR 0834](../adr/0834-permanent-diagnosis-after-temporary-holds.md) ile düzeltildi; geçici tutuş başarılı hareketi seçer, teşhis kalıcı bağları gösterir.
