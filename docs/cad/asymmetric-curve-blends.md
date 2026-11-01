# İki mesafeli pah ve asimetrik eğri geçişi

2026-11-02 — CAD-ADV-04 araştırma kararı. Bu belge yeni bir uygulanmış
Blend komutu ilan etmez; mevcut pah/daire yuvarlamasının anlamını korur.

Pah iki bağımsız kenar mesafesidir; mevcut `distance`/`second` ve sahnede
**1. kenar / 2. kenar** bu ihtiyacı karşılar. Dairesel yuvarlamada tek
R vardır. İki kesişen doğruya teğet aynı dairede köşeden teğet noktalarına
uzaklıklar eşittir. İkinci mesafeyi bağımsız yapmak daireyi bozar.

## Geometrik ayrım

Dik köşede mesafeler a,b>0 ise merkez `(a,b)` ve yarı eksenleri a,b olan
çeyrek elips `(a,0)` ile `(0,b)` uçlarını iki doğruya G1 bağlayabilir.
Uç eğrilikleri `a/b²` ve `b/a²` olur. Doğruların eğriliği sıfırdır;
elipsin teğet olması G2 olduğu anlamına gelmez. a=b halinde daireye döner.
Genel iki eğri arasında elips her istenen uç/teğet/eğrilik koşulunu
sağlayamaz. G1 Bézier geçişindeki iki teğet tutamacı da bağımsızdır;
bunlara iki “yarıçap” adı verilmez.

[Rhino BlendCrv](https://docs.mcneel.com/rhino/8mac/help/en-us/commands/blendcrv.htm)
uç şekillerini ayrı düzenler, simetriyi ayrıca seçtirir, süreklilik ve
kırpma/birleştirmeyi önizlemede gösterir.
[RhinoCommon BlendContinuity](https://developer.rhino3d.com/api/rhinocommon/type/rhino.geometry.blendcontinuity/)
konum, teğet ve eğrilik koşullarını ayrı sınıflandırır. KentOS kararı:
G1 yön paralelliği ve ortak konumdur; hız büyüklüğü eşitliğini de isteyen
C1 ile karıştırılmaz. G2 de aynı parametre türevleri demek değildir.

## Ayrı özellik eklenirse sözleşmesi

- Daire yuvarla ve Pah korunur; ayrı **Eğrisel geçiş** yöntemi seçilir.
  İki uç mesafesi, tutamaç/şekil ve süreklilik aynı şey diye sunulmaz.
- İlk kapsam iki düz kenarda eliptik G1; genel eğri çiftinde ayrı NURBS
  geçiş ancak gerçek çözücüsü ve kaynak türü koruması tamamlandığında.
  G2 düğmesi karşılığı çalışan algoritma olmadan gösterilmez.
- Girdi kalıcı nesne/alt parça, tıklanan taraf, iki uç mesafesi ve açık
  kırpma seçimi taşır. Salt okunur plan olası çözümleri, uçları, yönleri,
  hata bütçesini ve çökme/ilmek/kesişim reddini verir. Ekran LOD'u hesap
  toleransı yerine geçmez; ölçü/tarama aktarımları da aynı adıma katılır.
- Görünür iki uç etiketi ve canlı önizleme; geçersiz değerde geometri
  değişmez. Tek onay/Esc/Undo, dar ekran, Python/MCP aynı sözleşme.
- Bağımsız analitik elips eğriliği/G1 testleri, a=b daire sınırı,
  ters yön/dar açı/büyük koordinat ve kaynak kimliği kabul koşuludur.

Bu araştırma mevcut dairesel yuvarlama kabulünden ayrıdır. Uygulama
kapsamına alınırsa yeni komut/şema tasarımı ayrıca gözden geçirilir;
henüz varmış gibi yetenek kataloğuna eklenmez.
