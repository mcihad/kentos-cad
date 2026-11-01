# 0827 — Yaylı alan bölmede tarama ve ölçünün parçasını koru

2026-11-02. Durum: masaüstü, ortak komut ve Python/MCP kabulü doğrulandı.

## Bulgu

Yaylı ve delikli kapsül alanında, tohumları solda ve sağda olan iki
ilişkili tarama oluşturuldu. `cad.areas.edit split` ilk parçada kaynak
UID'sini tutuyordu. Her iki tarama da o UID'yi izleyince sağdaki tarama
sol parçaya sıçrıyordu. `region_cut`'ın tohum dışarıda kalınca en büyük
bölgeye dönmesi normal yerinde düzenlemeye yarar; açık parça aktarımında
başka parçayı seçme kararı yerine kullanılamaz. Ölçünün eski köşe indeksi
de yeni bölünmüş konturda aynı fiziksel köşeyi garanti etmiyordu.

## Karar

- Alan böl her kaynak için yalnız kendi parçalarından oluşan geçici
  aktarım listesini mevcut `Document::relocate_ties` yoluyla verir.
  `EntityEdit::Add.from` ile yeni UID'ler aynı sırada eşlenir; birden çok
  kaynak işlenirken bütün çıktılar tek ortak havuza atılmaz.
- Ölçüler mevcut fiziksel nokta/parça arama kuralını kullanır; eski
  indeksin anlamsızlaşması nesnenin kimliğinin kaybolması değildir.
- Kapalı nesneye bağlı tarama açık bir aktarım listesi varsa o listede
  tohumunu gerçekten içeren kapalı alanı seçer. Delik içi dahil hiçbir
  aday yoksa son geometri kalır ve bağ kopar; en büyük parçaya sıçramaz.
  Normal yerinde taşıma/düzenlemenin mevcut fallback kuralı değiştirilmez.
- Aynı dış adımda izleyiciler çalışır; tek Undo bütün alanları, ölçüleri,
  taramaları ve soy kaydını geri getirir. Yeni kalıcı şema/komut yoktur.
  Python ve MCP aynı native komutu çağırır; web ADR 0228 gereği ertelidir.
- `cad.areas.edit plan` mevcut sözleşmesi gereği doğrudan alan değişikliği
  ve yeni alanları döndürür; tüm takipçi nesnelerin son durumunun listesi
  olduğu iddia edilmez. İzleyicilerin son durumu execute/Undo kabulünde
  ayrıca denetlenir.

## Resmî kaynak ve ürün kararı

[AutoCAD: To Reshape a Hatch or Fill](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-MAC-Core/files/GUID-A8015804-ADDB-4721-92F6-7774F3C493F1.htm)
kapalı sınır değiştiğinde ilişkili taramanın güncellenmesini, sınır
açıldığında son şeklin korunup bağın kopmasını açıklar.
[QCAD Boolean Operations](https://qcad.org/en/manual/scripts/pro/draw/polyline/polylineboolean/doc/polylineboolean/)
alan cebirini polyline işlemi olarak sunar. KentOS'un parça/tohum/UID
aktarımı kendi ürün kararıdır; bu kaynakların aynı iç algoritmayı
kullandığı iddia edilmez.

## Kabul kanıtı

- Kapsül: 4 × 2 dikdörtgen + iki R1 yarım daire − 1 × 1 delik.
  Bağımsız alan `7+π`; x=2 bölmesinin parçaları `3+π/2`, `4+π/2`.
  Kaydedilmiş GUI sonucuna bağımsız Green integrali/bulge formülü
  uygulanınca toplam `10,141592653589793 m²`; delik solda, yaylar bulge
  olarak kalır. Parça çevreleri toplamı `16+2π` (iki yeni kesit dahil).
- GUI'de Alt+Q → Alan böl, alan seçimi, `2,-1` ve `2,3`, tek bitirme;
  iki tarama doğru tarafta kalır. Kaydedilmiş UID/assoc'lar bağımsız dosya
  okuyucusuyla kontrol edildi. Tek Undo sonrası bütün nesne ve UID'ler
  başlangıçla birebir aynı. Görseller 55–57 ve `.kcad` kabul dosyaları
  `/tmp/kentos-cad-adv04-2026-11-01/` içinde, Git dışında tutulur.
- Native birleşik testte sağ kenara bağlı iki ölçü ucu yeni parçanın
  gerçek köşelerine canlı aktarılır, öznitelikler/tek Undo korunur.
  İki kaynağı birlikte bölme testi başka kaynağın parçasına bağlanmaz.
- Python typed API ve MCP bölme/tarama/roundtrip/Undo kabulü geçti.
  Alanı olmayan açık parçalarda güvenli kopma ve ortak komut fixture'ları
  mevcut izleyici regresyonlarıyla denetlenir.

[Asimetrik geçiş araştırması](../cad/asymmetric-curve-blends.md),
[destek matrisi](../cad/curve-editing-support.md).

- Çok parçalı yaylı çizgi bölmesinde diğer parça birebir korunur; iki
  parçaya ayrılan ilk parça nedeniyle indeksi 1'den 2'ye geçen sonraki
  parçanın iki ölçü başvurusu canlı kalır. Toplam uzunluk `π+4` korunur.
- Son kontroller: 3 yeni native alan/çoklu parça testi; MCP 1; Python
  yeni alan testi + 2 NURBS testi; domain 18+87+22+5; bütün native ortak
  komut fixture'ları; masaüstü 1572 test geçti. İlgili native crate'lerin
  all-targets clippy ve bağımlılık yönü kontrolü temiz.
