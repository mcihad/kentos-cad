# 0662 — Mimari açıklığı hassas yerleştirme

25 Ekim 2026 · ARCH-OPEN-01 · Durum: yerleştirme dilimi uygulandı ve doğrulandı;
ana maddenin kalan kabul koşulları açık.

## Kapsam ve korunan davranış

Kapı, Pencere ve Boşluk aynı mevcut araç/`cad.openings.create` komutunda kalır.
Tek tık, komut satırındaki köşeden yakın pervaz mesafesi, ortala, tür, yan ve menteşe davranışı
korunur. Yeni alanlar verilmemiş eski v1 girdisi aynı sonucu üretir. Yeni nesne
veya dosya modeli, çizim birimi, render hesabı ve genel tuş sistemi eklenmez.

## Anlamsal yerleştirme

- Var olan `from=near/start/end` araçta **Duvar ucu** seçimiyle görünür.
- Yeni isteğe bağlı `distanceReference=jamb/center/hinge` yalnız `distance` ile
  kullanılır. Varsayılan jamb eski yakın pervaz davranışıdır. Center hedefin
  açıklık merkezi olduğunu, hinge `hand` ile seçilen pervaz olduğunu söyler.
  Duvar ucu ve menteşe yönü birbirinden bağımsızdır. Saklanan ankraj merkez
  olarak kalır; sonraki en/host düzenleme sözleşmesi değişmez.
- Yeni isteğe bağlı `span=[a,b]`: iki pervaz noktası host yoluna izdüşürülür;
  yol boyunca ayrım genişlik, yol üzerindeki ortası merkezdir. Yayda kiriş
  uzunluğu kullanılmaz. `at`, `distance`, `middle`, `width` ile karıştırılmaz;
  `point` açılma yanını seçebilir. Duvar ucunun dışına verilen nokta kırpılarak
  sessiz kabul edilmez. Köşe geçen aralık mevcut tek kenar kuralıyla reddedilir.
- Bütün üretim plan/execute, sonluluk, çakışma, revizyon, katman, atomiklik ve
  undo kontrolleri mevcut native komuttadır; Python/AI aynı girdiyi kullanır.

## Etkileşim

**Yöntem**: Tek tık / İki pervaz / Örnekten al. İki pervazda ilk nokta ve host
kilitlenir; ikinci nokta aynı duvarda gerekir. Snap'in gerçek noktası korunur;
koordinat ve göreli ikinci nokta ortak okuyucudan gelir. Sıfır en, taşma ve
çakışma yazmaz; ilk nokta düzeltilmek için kalır. Escape/araç içi undo ilk
noktayı geri alır; Bitir yarım açıklık üretmez. Başarılı ikinci tık tek normal
ilişkili açıklık yazar ve araç sonraki çift için açık kalır.

Örnekten al aynı biçimdeki mevcut açıklığın **tür ve genişliğini** alır; host,
numara, menteşe veya açılma yanını gizlice kopyalamaz. Başka biçimdeki nesne
ve çözülemeyen kayıtlı tür mevcut ayarları değiştirmez. Yapılan işlem yalnız
geçici araç ayarıdır; yazma bir sonraki yerleştirmede olur.

Gerçek ilk GUI denemesinde mesafe etkin olduğu hâlde Yerleştirme alanı
Serbest diyordu; üç açık seçenekle düzeltildi: Serbest / Uçtan uzaklık / Ortala.
Serbest açıkça mesafeyi kaldırır; Uçtan uzaklık sıfırı da gerçek net mesafe olarak
kullanır. Görünür mesafe alanında 0 değeri uçla sıfır paydır. Eski komut satırı
`U` → `0` mesafeyi kaldırmayı sürdürür. Büyük yazıda kesilen yeni alan adı
**Ölçü noktası** olarak kısaltıldı; genel özellik hücresi değiştirilmedi.
Ayrıca araçtaki Ortala seçimi, hatırlanan eski mesafeyi artık isteğe koymaz;
komutun eski distance-önce önceliği yüzünden ortalamanın bastırılması giderildi.
Komut satırındaki eski Orta aç/kapa, mesafeyi geçici bellekte koruyup geri getirebilir.
Semantik komutun eski öncelik sırası değişmez.

Seçilen host çizgileri önizlemede kesikli vurgulanır. Komut planındaki açıklık
üzerinden native ölçüm yardımcısı eni ve host kenarının iki ucuna net pervaz
payını verir; kapıda yan ve menteşe de yazılır. Kabuk ikinci bir geometri
hesabı yapmaz. Bu helper bir render veya bağımsız mutasyon yolu değildir.

İki pervaz/örnek alma sırasında kullanılmayan en/mesafe/referans alanları
gösterilmez; Ortala eski mesafe alanlarını gizler. İki pervazın gerçek eni
önizlemede yazılır. Uzun ret metni sözcük sınırlarında satıra bölünür;
hata açıklaması kesilmez. Yayda en analitik host boyunca ölçülür; mevcut
açıklık sembolünün kiriş gösterimi değiştirilmez. Eğri doğrama profili bu
yerleştirme seçeneğinin sonucu olarak sunulmaz.

## Kabul kapıları

Bağımsız düz/ters/yay aritmetiği, iki noktanın yönü, taşma/köşe/çakışma,
sonluluk ve çoklu isteğin atomik reddi; eski fixture'lar; araç ayar/taslak
korunumu; örnek tür sınırı; gerçek açık/koyu pencere ve kaydet/aç/undo;
Python/MCP/headless yüzeyi ve sözleşme/SDK tutarlılığı doğrulanmadan ana
ARCH-OPEN-01 tamamlandı işaretlenmez.

## Bu dilimin kanıtı ve açık takip

- Bağımsız başvuruya 38 durum eklendi: 24 uç/referans/menteşe birleşimi,
  4 iki pervaz yönü ve 10 ret/katman durumu. Önceki 41 oluşturma durumu
  değişmeden korunur; 79 create + 45 edit durumu native ve Python'da geçti.
- Yeni native, etkileşim, Python ve gerçek MCP/headless testleri aynı komutu
  kullanır. Katalog, SDK, CAD manifesti, bağımlılık yönü, clippy ve ortak WASM geçti.
- Gerçek fare/klavye denemeleri: menteşeden mesafe, iki pervazla düz/ters/yaylı
  host, örnekten tür/en, tek undo/redo, tek sağ-tuş Onayla ile yarım çizimi
  kapatma, Enter ile özellik enini değiştirme ve undo, Escape ile alan taslağı
  iptali, tekerlek yakınlaştırması. Açık/koyu 18 px geniş ve 13 px dar
  pencere görüntüleri gözle incelendi.
- Bağımsız KCAD okuyucusu: başlangıçtaki beş nesne ve proje ayarları tamamen
  aynı; dört yeni ilişkili açıklık kaydedildi. `drawingUnit=m`, `plotScale=100`.
- Kaydırma aracına gönderilen gerçek sürüklemelerde kamera hareketi
  doğrulanamadı. Kaydırma kabulü geçmiş sayılmaz; ölçek/render/input altyapısı
  varsayımla değiştirilmez. İlk bakışta çizim özetindeki ölçek farklı sanıldı;
  görüntü büyütülünce **1:100** olduğu doğrulandı (`scale-summary-crop.png`).
  Noktalı sıfırın yanlış okunmasıydı; ölçek hatası bulunmadı.
- Yeni yöntemle yaratılan açıklıkta tutamaç, host değiştirme, duvar kalınlığı
  ve bölme sonrası bağ korunumu için özel birleşik kabul henüz tamamlanmadı.
  Bütün 2 tema × 2 boyut × 2 yazı boyutu matrisi de tamamlanmış sayılmaz.
  Bu nedenle ana ARCH-OPEN-01 `[~]` kalır.

Ekranlar, dosyalar, test sonuçları ve gözlem ayrımı:
[`2026-10-25-architecture-openings`](../baseline/2026-10-25-architecture-openings/README.md).
