# Parametrik 2B tasarım — doğrulanan destek ve sınırlar

2026-11-02. CAD-ADV-05'in aşamalı 2B kapsamı ve birleşik kabulü
**tamamlandı** ([ADR 0843](../adr/0843-combined-parametric-plate-and-readable-bars.md)).
Bu belge uygulanan adaptörlerin sınırını açıklar; bütün eğrilerde sınırsız
parametrik çözüm iddiası değildir.

| Geometri / yetenek | Kodda mevcut davranış | Kapsam sınırı / kabul |
|---|---|---|
| Nokta / çizgi | Konum, düz kenar, uzaklık, yön ve geometrik bağlar | Büyük koordinat birleşik plaka, kilit/gevşetme/Undo kabulü geçti |
| Daire / bağımsız yay | Merkez/yarıçap, yay uç açıları; teğet/eşitlik/ölçüler | Büyük koordinat teğet/eşit delikler, üst teğet ve sağ/sol dalı, kilitli referans birleşik kabulü geçti |
| Çoklu çizgi / alan | Her halka/parçanın köşeleri bilinmeyen; düz kenarlar Segment | ArcEdge ile açık anılan dairesel kenarda değişken bulge/yarıçap, teğet ve mevcut daire kuralları; uç/genişlik/dal korunur. Genel eğri serbest nokta/süpürme açısı açık |
| Elips | Merkez ve mevcut Start/End (tam elipste dikiş); gerçek yarı eksen/açı bilinmeyenleri, dual türevler | Genel teğet ve kalıcı ayrı eksen başvuruları açık; tam elips eksen grip hedefi geçici niyet olarak uygulanır; kısmi yay Start/End grip hedefi, izleyen çizgi, plan/önizleme ve Undo kabulü geçti |
| NURBS | Açık/periyodik olmayan ve iki ucu tam clamped eğride Start/End; dört uç koordinatı, iç kontrol noktaları sabit | Genel kontrol/teğet/G2 ve diğer uç yapıları bu adaptörde yok |
| Blok yerleştirmesi / iç nesne | Yerleştirme noktası Node(0), yalnız iki konum bilinmeyeni; ayna/açı/ölçek ve kaynak korunur | İç parça, dönme ve ölçek bilinmeyenleri bu adaptörde yok; birleşik GUI/Python/MCP ve grip/Undo kabulü geçti |
| Parametre ifadeleri | Uzunluk/açı/sayı, ad değiştirme, token bağımlılığı, döngü, sıfıra bölme; yeni ifadede boyut cebiri, değişken üs bağımlısı, eski dosyada salt okunur birim uyarısı | Eski pi yarım tur anlamındadır; açık onarım dışında değer değiştirilmez. Birleşik teknik detay kabulü geçti |
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

## Araştırma ve birleşik kabul

[AutoCAD dimensional constraints](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-F7ED2587-8F25-4770-BD13-C7C95456E707.htm)
sürücü ile yalnız ölçen referansı ayırır.
[BricsCAD Parameters Manager](https://helpcenter.bricsys.com/en-us/document/panels/parameters-manager-panel?version=V25)
tanımlayan/etkilenen parametreleri inceleme, ad değiştirme ve ilgili
geometriyi seçme örneğidir. KentOS kendi mevcut komut ve çözücüsünü
kullanır; bu kaynakların yeteneklerinin tamamı varmış gibi gösterilmez.

Yaylı kenar/elips kapsamı, ölçü boyutları, aynalı blok ve geçici sürükleme
adaptörlerinin kabulü aşağıdaki ADR'lerdedir. Birleşik teknik plakada
ölçü/teğet/eşitlik/konum, çözüm dalı, çelişki, kilit, açık gevşetme ve Undo
UI/Python/MCP ile doğrulandı: [ADR 0843](../adr/0843-combined-parametric-plate-and-readable-bars.md),
[tekrar çalışma rehberi](technical-plate-constraints.md). Güvenli ret bir
adaptörün uygulanması yerine sayılmaz; üst tablodaki genel eğri sınırları
hâlâ geçerlidir.

Elips nokta adaptörünün sınırları ve kabulü: [ADR 0829](../adr/0829-native-ellipse-point-constraints.md).

Başarı sonrası durağan imleç ve birimli sonuç kabulü: [ADR 0830](../adr/0830-constraint-success-feedback-and-parameter-units.md).

Elipsin kendi merkez–uç yönünde ekseni çökertmeden çeyrek dönüşü, kilit/çelişki ve UI/Python/MCP/Undo kabulü: [ADR 0831](../adr/0831-ellipse-ray-direction-without-axis-collapse.md).

Açık NURBS uçlarının 6→8 m, bağlı çizgi, kapsam notu, dosya/Undo ve UI/Python/MCP kabulü: [ADR 0832](../adr/0832-bounded-nurbs-endpoint-constraints.md), [Python/MCP kullanımı](nurbs-constraints-python.md).

Aynalı/dönük blok yerleştirme noktası ve gerçek grip kabulü: [ADR 0833](../adr/0833-block-insertion-anchor-constraints.md), [Python/MCP kullanımı](block-anchor-constraints-python.md). İlk seçilen sabit nesnenin geçici hold altında gereksiz raporlanması [ADR 0834](../adr/0834-permanent-diagnosis-after-temporary-holds.md) ile düzeltildi; geçici tutuş başarılı hareketi seçer, teşhis kalıcı bağları gösterir.

Elips yayı uç grip hedefi, çözülmüş önizleme ve büyük koordinatta saklanan uç/izleyen nokta kabulü: [ADR 0835](../adr/0835-elliptical-endpoint-grip-targets.md). Tam elips eksen grip'leri bu kabulün kapsamında değildir.

Sahne kısıt etiketlerinin panel sınırına sığması, pencereyle ortak birimler ve çizim oturumuna bağlı önbellek: [ADR 0836](../adr/0836-constraint-label-viewport-and-units.md).

Çoklu çizgi/alan dairesel kenarı, altı bağla büyük koordinat ve UI/Python/MCP/dosya/Undo kabulü: [ADR 0837](../adr/0837-variable-circular-path-edge-constraints.md). Saklanan f64 temsilde sahte çelişkiyi tolerans büyütmeden doğrulama: [ADR 0838](../adr/0838-stored-constraint-roundtrip-certificate.md).

Sabit yay uçlarında imkânsız yarıçap/çap için birimli kiriş ve alt sınır açıklaması: [ADR 0839](../adr/0839-actionable-fixed-arc-radius-refusals.md). Uçlardan biri serbestse kirişin kısalmasına izin verilir.

Boyut cebiri, yeni/değişen ifadelerin atomik denetimi ve eski pi'nin sessizce yeniden yorumlanmadan denetlenmesi: [ADR 0840](../adr/0840-parameter-dimensions-without-legacy-reinterpretation.md), [Python/MCP birim rehberi](parameter-units-python.md). Gerçek GUI'de altı bağlı mm çizimde yanlış tür reddi, karekök ifadesiyle 2500→3000 mm, tek Undo, dar pencere, açık pi onarımı ve yeniden açma doğrulandı; UI/Python dosyaları tam aynı.

Tam elipsin sıfır olmayan dikişini eksen tutamaçlarında ve eksen sırası değişiminde koruma; gerçek büyük koordinat GUI/Python dosya eşitliği, tek Undo ve yeniden açma kabulü: [ADR 0841](../adr/0841-full-ellipse-seam-through-axis-grips.md). Bu düzeltme kalıcı bağımsız eksen başvurusu eklemez.

Tam elipste açık kaynak eksen hedefi, serbest diğer yarıçapın uyumu, eksen sırası değişimi, gerçek GUI/Python dosya eşitliği ve kalıcı Fix çatışmasında atomik ret: [ADR 0842](../adr/0842-explicit-full-ellipse-axis-grip-targets.md), [Python/MCP](ellipse-axis-grips-python.md). Geçici hedef kalıcı teşhisin serbestlik sayısını değiştirmez.
