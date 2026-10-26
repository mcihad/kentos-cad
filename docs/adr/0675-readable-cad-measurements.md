# 0675 — Dar CAD Öznitelikler panelinde eksiksiz ölçü okuma

25 Ekim 2026. **Bu dar dilim kabul edildi.** ARCH-BASE-01'in gerçek GUI
kontrolünde 18 px / 312 px panelde 33,44 m² mahal alanının mevcut çizim
birimindeki `33440000.00 mm²` değeri `33440000.… mm²` olarak kısalıyor.

Karar: yalnız CAD Öznitelikler'deki salt okunur, birimli sayısal ölçüler için
yerleşim uyarlansın. Değer ve birim mevcut hücreye gerçek yazı genişliğiyle
sığıyorsa iki sütun aynı kalsın; sığmıyorsa başlık ve tam ölçü alt alta,
satır yüksekliği içeriğe göre olsun. Sayıyı yuvarlama, bilimsel gösterime
çevirme, birim/ölçek değiştirme veya eksik basamakları tooltip'e saklama yok.
Normal düzenlenebilir hücreler, seçim menüleri ve CBS yerleşimi korunur.

- [x] Gerçek önce görüntüsü ve kayıt kimliği.
- [x] Dar/geniş, 13/18 px, iki temada eksiksiz alan/çevre görünümü.
- [x] Aynı kaydın bağımsız okuyucuda bütün model eşitliği.
- [x] Gerçek hücre düzenleme/Enter/Escape ve undo davranışı korunumu.
- [x] Ölçülü UI yerleşim regresyonu, masaüstü derleme ve clippy.

Bu dilim tüm mimari araç matrisi veya genel Öznitelikler tasarımı kabulü değildir.

## Gerçek düzenleme testinde bulunan birim eki

İlk aday GUI'de CAD mahal yüksekliğine `3 m` yazılıp Enter verilince mevcut
`web_number` / parseFloat yolu eki atıp millimetre projede 0,003 m sakladı;
hacim 0,10 m³ oldu. İlk kayıt ve kırmızı test saklanır, kabul sayılmaz.
CAD mahal yüksekliği ortak `point_text::parse_length` / `Len::metres` yoluna
bağlanır. Birimsiz sayı mevcut çizim birimindedir; açık metrik/imperial ek
kendi fiziksel değeridir. Boş değer temizler, geçersiz ek açık mesajla atomik
reddedilir. Diğer özelliklerin ve CBS'nin legacy sayı okuyucusu değiştirilmez;
bu dar düzeltme bütün mimari alanların birim denetimi yerine geçmez.

- [x] 5 çizim biriminde metrik/imperial/ondalık virgül, birimsiz değer, boşaltma,
  hatalı ek/sıfır/negatif ret ve tam model undo regresyonu.
- [x] Son GUI binary'sinde `3 m` → 3000.000 mm / 100,32 m³; Escape/ret/undo/redo.

Kabul: [22 ham görüntü, 11 KCAD ve bağımsız kontrol](../baseline/2026-10-25-architecture-measurements/README.md).
Son masaüstü özellik testleri 39 başarılı / 4 atlanan, yerleşim 3 başarılı,
derleme ve clippy başarılı. Önceki tam UI koşusu 280 başarılı / 72 atlanan;
son eklenen resize testi ayrı son koşuda geçti. Atlananlar başarılı sayılmaz.
Diğer uzun özellik adları ve diğer mimari giriş alanlarının birim eki denetimi
açıktır; bu dilim genel ölçek sistemi veya bütün mimari GUI matrisi kabulü değildir.
