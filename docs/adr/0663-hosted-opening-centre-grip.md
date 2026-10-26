# 0663 — İlişkili açıklığın merkez tutamacı

25 Ekim 2026 · ARCH-OPEN-01 takip · Durum: uygulandı; aşağıdaki kapsamda doğrulandı.

## Bulgu

ADR 0662'ün birleşik kabul testi kapının merkez tutamacıyla taşındığında
önizlemenin değiştiğini, bırakınca kayıtlı `at` değerinin aynı kaldığını yakaladı.
Eski GUI derlemesinde de fareyle tekrarlandı: `27` önizlemesinden sonra `28`
görüntüsünde kapı 5,000 m'de kaldı ve belge değişmedi.

Genel tutamaç yolu `cad.entities.edit` için `EntityGeometry` üretir. Açıklık
bu sözleşmede bilinçli olarak yoktur; kendi `cad.openings.edit` komutu vardır.
Eski yol, açıklık geometrisi çevrilemeyince nesneyi sessizce atlıyordu.

## Değişiklik

- Mevcut merkez tutamacı ve tıklama/klavye/iptal düzeni korunur.
- Tek açıklığın Esnet modu aynı host'a hedef noktasıyla `Rehost` gönderir.
  İzdüşüm, ankraj, yay boyu, köşe/taşma/çakışma, katman ve revizyon kontrolleri
  mevcut native komutta kalır. Yeni geometri yazma veya dosya modeli eklenmez.
- Önizleme aynı komutun planıdır. Kopya modunda planın host koordinatından
  mevcut `Copy` uzaklığı hazırlanır; kaynağa çakışma native Copy planında da
  denetlenir. Kaynak yerinde kalır; kopya tutamacı sıcak kalır.
- Açıklığın başka nesnelerle birlikte birden fazla sıcak tutamacı seçilmişse
  işlem açık uyarıyla bütünüyle reddedilir. Açıklığı atlayıp diğer nesneleri
  kısmen yazmak kabul edilmez. Farklı hostlarda çoklu anlamsal tutamaç taşıması
  bu düzeltmede desteklenmiş sayılmaz.
- Diğer nesnelerin Esnet yolu ve Taşı/Döndür/Ölçekle/Aynala modları değişmez.
- Yalnız mevcut komut çağrıldığı için Python/AI aynı `Rehost`/`Copy` girdisini
  kullanabilir; yeni şema, TS üretimi veya SDK uç noktası gerekmiyor.

## Kabul

- Etkileşim: yeni hassas açıklığın tutamacı, yazmasız önizleme, gerçek konum,
  journal komutu, tek undo/redo; uçtan taşmanın atomik reddi; kopyanın kaynağı
  koruması; karışık sıcak seçimde kısmi yazmama.
- Native: iki pervaz ve menteşe mesafesi × düz/ters duvar; sayısal en,
  menteşe/yan çevirme, kalınlık, bölme, başka host ve undo/redo.
  Yeniden parametrelenen 10 m host için f64 aritmetik sınırı ayrı sınanır;
  üretim toleransı değiştirilmez.
- Tam interaction regresyonu, ilgili desktop testleri, GPU boyut matrisi,
  clippy ve gerçek yeni derleme fare/klavye + kaydet/aç kanıtı gerekir.

Yeni derleme gerçek pencerede taşıma, tek undo/redo ve kaydet/aç ile doğrulandı.
715 interaction ve 1274 desktop testi geçti. Sekiz boyut/font/tema durumu
GPU üzerinde ayrıca çalıştırıldı; bunlar gerçek pencere yerine geçmez.
Kanıt: [kabul kaydı](../baseline/2026-10-25-architecture-grips/README.md).
ARCH-OPEN-01’in tam native pencere matrisi ve gerçek Kaydır kabulü açıktır.
