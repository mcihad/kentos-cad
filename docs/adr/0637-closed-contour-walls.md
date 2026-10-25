# 0637 — Kapalı konturdan mimari duvar

25 Ekim 2026 · ARCH-DRAW-03 · İlk uygulama dilimi; kabul çalışması sürüyor.

## Karar ve sınır

Mevcut `cad.walls.convert` v1 komutuna isteğe bağlı `mode=contours` eklenir.
Varsayılan `each` ve eski `pairs` geometrisi, kaynak atlama ve kısmi eşleme
politikası korunur. Yeni komut, KCAD nesnesi veya dosya sürümü yoktur.

Kapalı çoklu çizgi, alanın bütün parçaları/delikleri ve analitik daire kabul
edilir. Dış halka saat yönünün tersine, delikler saat yönüne düzenlenir;
başlangıç köşesi ve yay bulgeleri korunur. Her halka normal, açıklık
barındırabilen kapalı bir duvar olur. İç net / Dış ölçü / Eksen referansı,
mevcut duvarın Left / Right / Center değerlerini kullanır; çizim metreyle
hesaplanır, gösterim birimi geometriyi yeniden ölçeklemez.

Kontur modunda bütün kaynak kümesi doğrulanır. Açık/uygunsuz kaynak, kilitli
katman, uyumsuz hedef katman türü, geçersiz halka/delik, kalınlığın kapattığı
oda ve yinelenen kontur hiçbir duvar yazmadan durur. Kot veya M değeri
kaynakları sessizce 2B'ye indirilmez. Farklı katların semantik eşlemesi bu
dilime dahil değildir. Önizleme aynı native uygulamanın planıdır; kaynak
koru/sil sayıları görünür. Yazma tek işlem/undo'dur. Nitelikler, renk ve
mevcut görsel özellikler taşınır; bağımsız etiket bağları ve ekler yeni
duvarlara çoğaltılmaz.

Birden çok konturun aynı katmandaki değen/kesişen sınırları **ortak duvar
çözümü değildir**: mevcut 1e-9 m analitik toleransıyla açıkça reddedilir.
Aynı analitik halka aynı katmanda mevcutsa tekrar üretim reddedilir.
Genel kenar bölme, yaklaşık/yeniden örneklenmiş eş halka tanıma, ortak duvarı
tekilleştirme ve farklı kat entegrasyonu açık kalır. ARCH-DRAW-03 bu nedenle
ve tam gerçek pencere matrisi tamamlanana kadar [~] tutulur.

## Görünür ayarlar

Mevcut Çizgiden duvar aracında Yöntem, Stil, Genişlik, Referans ve Kaynak
ayarları gösterilir. Kapalı kontur ilk seçildiğinde İç net başlar; aynı
seçeneği tekrar seçmek referansı sıfırlamaz. Eski Ç/C çiftler kısayolu korunur.
Araç ayarı ve iptal belgeye yazmaz. Kontur onayı planın revision'ıyla yapılır;
belge değişmişse ilk onay reddedilir, yeni plan için tekrar açık onay gerekir.

Hesap arayüze taşınmaz; Python SDK, MCP keşfi ve plan/execute aynı komutu
kullanır. Web arayüzü ADR 0228 gereği ertelenir. Ortak TS üretimi geçici
`.run` dizinindedir; yalnız mevcut WallsFromMode enum farkı depoya alınır.
ADR 0626'nın birim/1:100, FP64, yazı, import ve kamera/render kararları korunur.

## Kanıt

Bağımsız başvuruda eski 38 dönüşüm vakası aynı kalır, 31 kontur vakası eklenir.
Eski çeyrek daire tanjantının platforma bağlı son biti bağımsız Decimal
sqrt(2)-1 ile sabitlenir; ürün geometri hesabı değiştirilmez.
L oda, avlu/delik, ters yön, daire, büyük koordinat, üç referans ve farklı
gösterim birimleri ayrı kabul kapsamındadır. Çalıştırılmış kontroller ve gerçek
GUI sonuçları [kabul kaydında](../baseline/2026-10-25-architecture-wall-contours/README.md)
raporlanır. Eksik kapılar tamamlanmış sayılmaz.
