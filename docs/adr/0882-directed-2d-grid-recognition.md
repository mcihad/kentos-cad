# ADR 0882 — Dönük/eğik 2B ızgara ve kayıpsız dizi kabulü

Tarih: 3 Kasım 2026. CAD-ADV-07 son düzen ailesi.

## Dayanak ve karar

[AutoCAD 2026 ilişkili dikdörtgen dizi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-181BF535-D6F4-4B90-B04D-EB99C5488DAB.htm)
sayı/aralık önizlemesi, bağlamsal düzenleme ve Axis Angle seçeneği sunar.
KentOS'un mevcut dünya-X/Y `Grid` sözleşmesini değiştirerek eski belgeleri
farklı yorumlamak yerine `Lattice` eklenir. İki bağımsız eksen, düzenli
aralıklar ve kaynak öğe yönü ayrı verilerdir. Bu karar 2B'dir; katlar/Z,
serbest yamulma ve yaklaşık görsel benzerlik bu teslimin kapsamı değildir.

## Veri ve geometri

`rows/cols`, metre `dx/dy`, dünya doğusundan saat tersine derece
`columnAngle/rowAngle`. Her yer ötelemesi `c·dx·u + r·dy·v` olur; kaynak
nesnelerin kendi yönü ve ayna durumu değişmez. Eski `Grid` hesap ve baytları
aynı kalır. Paralel eksen (`abs(sin(Δ)) < 1e-6`), sonlu olmayan değer,
aktif sıfır aralık ve 10 000 yer üzeri reddedilir. KCAD 161 yalnız yeni
layout için yazılır; 160 ve altına gizlice yeni anlam yüklenmez.

## Yazısız tanıma

- Açık `kind=lattice`; varsayılan doğrusal sorgu ve dünya-X/Y seçimi aynı
  kalır. Aynı statik, verisiz, eş yönlü blok yerleştirmeleri incelenir.
- Kaynağa göre yerel FP64 koordinatlar ve gerçekten gözlenen fark
  vektörleri kullanılır. İki eksende komşu çift, en az dört dolu yer,
  iki satır/sütun ve dolu bir sınır köşesi gerekir. Gizli yarım adım ya da
  boş kaynak köşesi üretilmez. Çakışan kökler keyfi UUID ile çözülmez.
- Kısa aralığın büyük WCS koordinatlarında yuvarlanması birikmesin diye
  tam gözlenen açıklıkla iki eksen birlikte uyarlanır. Yalnız aday üretimi
  bu belirsizliği taşır; son kabul mevcut `arrays_adopt::inspect` içinde
  özgün mutlak toleransta ve gerçek yerleştirmelerde tekrar yapılır.
- En çok 32 768 gözlenen çift, 96 eksen vektörü, 2 milyon iş ve kullanıcının
  sonuç sınırı. Sınırda açık `truncated`; iptalde hiçbir sonuç/yazı kalmaz.
  Aynı UUID kümesindeki daha eksiksiz/düşük sapmalı öneri tercih edilir.

## İnceleme, düzenleme ve API

Aday kartı iki ekseni, aralıkları ve eksik yerleri gösterir. İncele/Uygula
ayrıdır. Benimseme yalnız ilişkili dizi kaydı ekler; kök geometri, UUID,
çizim sırası, blok tanımı ve bağlı ölçü değişmez. Eksik/elle korunan yerler
mevcut `ArrayItem`'dır. Düzenlemede satır/sütun adresi kimliği korunur;
korunan öğe yerinde kalır. Sil/reset mevcut komutlardan geçer. Bağlı ölçü
asıl öğe ile hareket eder; her yazı tek Undo'dur.

GUI yön girişi açıkça derecedir; `30`, `30°`, `30deg` kabul edilir,
`30mm` reddedilir. Kullanıcının uzunluk birimi açıya uygulanmaz. Python
`LatticeArrayLayout` ve `ArrayFindKind.LATTICE`, native Job ve MCP aynı
şema/plan/revision kapılarını kullanır; API katalogdan üretilir. Web kopya
komutu aynı çekirdek dönüşümünü kullanır; tanıma masaüstü yeteneği olarak
katalogda açık kalır.

## Doğrulama

Bağımsız Python trigonometrisiyle 23°/81° iki eksen, iki eksik yer,
uyumsuz öğe ve bağlı ölçü hazırlanır. Native kabul; WCS ±4 milyar metre
üzerinde 3/5 mm aralık; tolerans dışı sapma; iptal; kimlik/koru/sil/reset;
komut/Python/iş ve iki okuyuculu KCAD 1513 dosya denetimleriyle sınanır.
Gerçek fare/klavye kabulü, kaydedilen KCAD dosyaları ve ekran kanıtı
ayrıca teslim kaydında belirtilir. Ekran görüntüleri depoya eklenmez.
