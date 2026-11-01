# 0828 — Parametre penceresinde teşhis, bağımlılık ve tür

2026-11-02. Durum: bu arayüz dilimi doğrulandı; CAD-ADV-05 açık.

Mevcut pencere yalnız tutuyor/tutmuyor diyordu; çözücünün serbestlik,
tekrar/kopuk/tekil bilgisi kullanıcıya ulaşmıyordu. Her görünüşte değer
ve tutarlılık hesaplanıyordu. Yeni kullanıcı parametresi de her zaman
uzunluktu; mevcut ad değiştirme komutu görünür değildi.

- Aynı `cad.constraints.query` teşhisi kullanılır; ayrı teşhis motoru yok.
  Sorgu sonucu ve ortak parser'dan çıkarılan ters bağımlılık grafiği
  `(çizim oturumu, revision, generation)` için saklanır. Yalnız taslağın
  yazılması sorguyu yeniden çalıştırmaz; yazma/Undo sonucu yenilenir.
- Parametre adı seçilince kaynakları ve dolaylı etkileri; aynı yerde
  yeniden adlandırma ve ilgili geometriyi gösterme vardır. Ad değiştirme
  mevcut komutun token bazlı ifadesini günceller. Geometri gezintisi
  yalnız seçim/kamerayı değiştirir; aynı pencereye dönünce taslak kalır.
- Yeni parametrede uzunluk/açı/sayı seçilir. Referans yalnız ölçer;
  düzenlenebilir sürücüyle karışmaz. Sonuç/hata üstte görünür. GUI testi
  altta gizlenen hata mesajını buldu; yerleşim buna göre düzeltildi.
- Yeni şema/komut/bağımlılık yok. Python/MCP zaten aynı query'den
  parametre ifadelerini, sürücü/referans durumunu ve teşhisi alır;
  aynı edit komutlarıyla rename/set/add yapar. Seçim Python konsolunun
  mevcut seçim izleyicisince, belge işlemleri komut günlüğünce izlenir.

[Resmî araştırma ve kod destek matrisi](../cad/parametric-support.md).

## Kanıt

Beş yeni test: query teşhisiyle birebir eşitlik; tuşlarda aynı önbellek,
yazma/Undo'da yenileme; token/dolaylı/çevrimli bağımlılık; rename ve
geometri gezintisinde taslak; açı/sayı türü; hatalı ifadede atomik ret
(aynı testler bazı koşulları birlikte denetler).

Gerçek GUI'de 17 kısıtlı iki delikli plakada `taban → modul`, bağlı
`genislik = modul * 2`, `6 → 7` ile `12 → 14 m`; referans köşegen
`√260` ve tam kısıtlı/0 serbestlik. Kaydedilmiş nesne/UID/parametre ve
teşhis aynı typed Python rename/set sonucuyla birebir eşleşir.
`1 / 0` ret halinde son geçerli değer 7 ve geometri kalır. Geometriyi
gösterip tekrar açınca taslak korunur. Açı türünde `donus = 30deg`
GUI'den eklendi; açık/koyu ve 1100 mantıksal piksel pencerede listeler,
hata ve Kapat görünür. Açı seçildiğinde örnek ifade de `30deg` olur. Görseller 58–65 repo dışı
`/tmp/kentos-cad-adv04-2026-11-01/` altındadır.

Desktop testlerinde 1577 başarılı sonuç; son düzenleme sonrasında beş
yeni test ve desktop tüm hedefler Clippy (`-D warnings`) başarılı.
Bağımlılık/mimari denetimi 51 crate ve 67 crate/hedef için temiz.

Bu dilim eğrisel kısıt adaptörlerini, genel blok kısıtlarını veya tam
birim boyut cebirini tamamlamaz; CAD-ADV-05 bunlar için açık kalır.
