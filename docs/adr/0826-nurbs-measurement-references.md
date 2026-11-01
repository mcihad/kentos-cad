# 0826 — NURBS ölçü başvurularını gerçek eğri üzerinde izlet

Tarih: 2026-11-02. Durum: bu dilim doğrulandı; birleşik alan kabulü nedeniyle CAD-ADV-04 açık.

## Bulgu ve karar

Eğri düzenleme komutları çalışırken NURBS, `SubPlace`'in gerçek geometri
hesabında yoktu. Başlangıç, bitiş ve eğri üzerindeki başvurular üretilemiyor;
bölme sonrası parça araması da NURBS'i bulamıyordu. İkinci sorun, native
sözleşmenin yazmadığı `closed=false` ve `periodic=false` alanlarının ortak
JSON okuyucuda zorunlu olmasıydı. Bu yüzden geçerli açık eğri ölçü/tarama
izleyicisince okunamayabiliyordu.

- NURBS ayrı geometrik ailedir. Başlangıç/bitiş, gerçek knot aralığının
  uçlarında değerlendirilir; ilk/son denetim noktası olduğu varsayılmaz.
- `Along`'un bütün indeksleri sıfırdır; 0–1 kesri `u=lo+t*(hi-lo)` ile
  gerçek parametreye gider. Parametre kesri, uzunluk kesri değildir.
- Şekli koruyan knot eklemesinde aralık başvurusu aynı yerde kalır.
  Denetim noktası düzenlemesinde aynı parametrenin yeni konumunu izler.
  Kır/Buda mevcut kalıcı kimlik/parça taşıma politikasını kullanır.
- Yakın başvuru doğrudan NURBS üzerinde en yakın noktayı değerlendirir;
  ekran veya hesap kirişinin noktasına bağlanmaz. Mevcut en yakın nokta
  çözücüsü knot aralıklarında örnekten Newton iyileştirmesidir; tüm olası
  çoklu köklerin sertifikalı izolasyonu diye sunulmaz.
- Aralık uzunluğu mevcut adaptif hız integraliyle ölçülür. Kapalı eğride
  aralık sarması vardır; açık eğride ters sıralı aralık reddedilir.
- Ortak **Entity** okuyucusu yalnız eksik NURBS bayraklarını false okur.
  Açıkça yazılmış null, yanlış tür veya geçersiz geometri kabul edilmez.
  Böylece blok içindeki eğriler de aynı native sözleşmeyi okuyabilir.

Yeni dosya şeması, komut veya Python/MCP türü eklenmez; mevcut `SubPlace`
yapısı kullanılır. Kaynak geometri türünü değiştiren açık dönüşümde,
çözülemeyen başvuru son koordinatını koruyup `kind_changed` olur.
Varsayılan eğri polyline'a çevrilmez. Web arayüzü ADR 0228 gereği ertelidir.

## Araştırma

[Rhino Essential Mathematics: Parametric Curves](https://developer.rhino3d.com/guides/general/essential-mathematics/parametric-curves-surfaces/)
gerçek domain, yeniden parametreleme ve parametre ile uzunluk kesrinin
ayrımını açıklar. KentOS'ta normalize başvuru gerçek knot aralığına
eşlenir; aralık uzunluğu ayrıca integre edilir. Kaynak açıklaması kendi
sayısal doğrulamamızın yerine geçmez.

## Kabul kaydı

- Ortak geometride ağırlıklı analitik çeyrek daire, `[2,6]` knot aralığı,
  denetim/kiriş dışında gerçek yakın nokta, düğüm eklemede şekil/başvuru,
  aralık uzunluğu, geçersiz indeks/kesir ve yanlış bayrak türleri sınandı.
- Native komutlarda denetim noktası taşıma, düğüm ekleme, Kır'da gerçek
  parçaya kimlik aktarımı, Buda'da yeni uç ve silinen iç bağın açıklanması,
  açık polyline dönüşümünde kopuk bağ ve tek Undo sınandı.
- Kır aracının native seçim filtresi NURBS'i dışlıyordu; filtre artık
  anlamsal komutun `breakable` denetimini kullanır. Arayüz testi bu açığı
  ortaya çıkardı; etkileşim testi aynı seçimi ve tek Undo'yu korur.
- Python 2, MCP 1, native bağ 5 ve ortak subref 14 test geçti.
  Etkileşim 71 + 858, masaüstü 1572 test geçti; ortak komut fixture'ları,
  bağımlılık yönü ve ilgili crate'lerin all-targets clippy kontrolü temiz.
- Gerçek GUI'de başlangıç denetim noktası `(1,0) → (2,0)` taşındı:
  uç ölçüsü `√2 → √5`, ara ölçü `0,765367… → √1,5` oldu. Kaydedilmiş
  tüm nesneler ve UID'ler aynı Python komutuyla birebir karşılaştırıldı.
- GUI Kır + tek Enter, çeyrek daireyi iki ağırlıklı NURBS parçasına
  ayırdı; bitiş bağı yeni parçaya, ortadaki bağ ilk parçanın bitişine
  taşındı. İki ölçü canlı ve aynı koordinatta kaldı. Bağımsız Python
  De Boor hesabında 20.002 örneğin yarıçap hatası en çok `3,33e-16 m`,
  kiriş toplamının `π/2` uzunluğundan farkı `4,06e-10 m` çıktı.
  Tek Undo sonrası bütün nesneler ve UID'ler başlangıçla birebir aynı.
- Görseller 50–54 ve kaydedilmiş kabul dosyaları repo dışında
  `/tmp/kentos-cad-adv04-2026-11-01/` altında tutuldu.

[Python/MCP kullanım kılavuzu](../cad/curve-references-python.md).
