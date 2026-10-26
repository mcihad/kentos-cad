# 0676 — Duvar ve açıklık özelliklerinde açık uzunluk birimleri

25 Ekim 2026. **Dar dilim uygulandı ve doğrulandı.** ADR0675 gerçek mahal yüksekliği
denemesinden sonra duvar Genişlik hücresinde `25 cm` → `25 mm` hatası da
gerçek pencerede yeniden üretildi. Önce kaydı saklandı, tek Undo ile başa dönüldü.

Kapsam: yalnız CAD duvar genişlik/öteleme/yükseklik ve açıklık genişlik/konum/
kasa/denizlik/başlık fiziksel uzunluk hücreleri ortak uzunluk okuyucusunu
kullansın. Birimsiz sayı mevcut çizim biriminde; açık mm/cm/m/in/ft fiziksel
değerinde. Mahal yüksekliği aynı yolu paylaşabilir. Açıklık açısı, metin,
kağıt boyu, oran, koordinat, proje birimi/ölçeği, CBS legacy okuyucusu korunur.
Model komutları, follower, doğrulama ve undo yeniden uygulanmaz.

- [x] Eski GUI hatası ve kırmızı App regresyonu.
- [x] Beş çizim biriminde bağımsız fiziksel beklenenler; negatif öteleme,
  geçersiz ek/alan sınırı atomik ret, opsiyonel boşaltma, legacy CBS korunumu.
- [x] Son GUI'de 25 cm → 250 mm duvar ve 120 cm → 1200 mm kapı; tek Enter,
  ret/Escape/undo/redo, kaydet ve bağımsız model kontrolü.
- [x] Özellikler regresyonu, derleme/clippy ve kanıt dosyaları.

Yeni model alanı/sözleşme yoktur. Yalnız masaüstü özellik adapteri mevcut
`parse_length` ve model komutlarını kullanır. Ortak regresyon 120 açık birimli
düzenlemeyi beş çizim biriminde fiziksel sabitlerle karşılaştırır; her biri tek
undo ile tam model anlık görüntüsüne döner. Hatalı ekler, alan sınırları,
opsiyonel boşaltma, negatif/sıfır öteleme, açı ve CBS legacy davranışı ayrıca
sınanır. Son özellikler: 40 başarılı, 4 atlanan; derleme ve clippy başarılı.

[Kabul kaydı](../baseline/2026-10-25-architecture-property-units/README.md):
29 gerçek ham görüntü, 26 bağımsız KCAD karşılaştırması; duvar follower alanı ve
kapı söveleri Decimal ile ayrı hesaplandı. Eski 80 cm denemesi kasa doğrulaması
tarafından reddedildi; ayrı saklanan eski 120 cm denemesi gerçek 120 mm hatasını
gösterir. Başarısız/reddedilen deneme başarılı düzenleme diye sayılmadı.

Diğer mimari nesne uzunluk alanları ve tüm mimari araç matrisi açık kalır.
