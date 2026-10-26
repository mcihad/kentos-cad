# Sorgu ömründe ortak mahal metrajı — ADR0680

25–26 Ekim 2026. `codex/architecture-2d-workflows`; önceki kabul commit'i
`cbc6b9b2a`. Bu sorgu/yerleşim dilimi doğrulandı;
ana mimari maddeler ve tam GUI/performance matrisi açık kalır.

## Neden ve korunan davranış

Her mahalın net/eksen/brüt sonucu için ayrı `SpaceSearch` kaynak ağları
kuruluyordu. Şimdi tek metraj sorgusunun immutable belge ödüncü ortak aramayı
sahiplenir. Kat/kapsam/UUID kümesi ayrı, kaynak slotları **belge sırasında**
ve kutularıyla tutulur. Tam seçili küme, yakın kutuya indirgemeden önce
kontrol edilir. Aynı sıralı kaynak/oda politikası aynı analitik ağı paylaşır.
Sorgu sonrasında listeler ve ağlar bırakılır; kalıcı/global cache yoktur.
Legacy mahalın eski Document arama yolu, yakın kutusu, tam fallback,
tolerans, birim, son sınır ve rapor alanları değiştirilmez.

## Birebir eski/yeni karşılaştırması

`capture.py after`: 20 açık çizim × 5 rapor süzgeci = **100 tam rapor**.
Beş çizim birimi, iki konum (yerel/500000–4400000), gerçek ayırıcı veya
sıradan kesik çizgi, aynı sorguda net/all/ignore kolon tercihleri, katlar,
atanmamış kapsam, geçerli/eksik açık kaynak kümeleri ve çizilmiş mahal vardır.
Dosya sıra/slot sırası farklı tutulur. JSON girişler native testte açık
UUID'leriyle V2 model olarak; aynı girişlerin kanonik KCAD kopyaları public
Python'da açılır. İlk V1 denemesi UUID'leri türettiğinden açık kaynak
karşılaştırması olarak kullanılmadı; dondurulmuş kabul V2/KCAD'dir.

`answers-before.json` optimizasyon öncesi native uzantıdan dondurulmuş tam
cevaptır; geometri için bağımsız oracle diye sunulmaz. Native uygulama testi
100 cevabı, plan/execute salt okunurluğunu ve tersten/tekrarlı yöntemlerle
paylaşılan aramanın tek çağrılık aramaya eşitliğini kontrol eder. Bağımsız
fiziksel beklentiler ADR0679'nin 105 ortak komut durumunda ayrıca sınanır.
`frozen-before-sha256.json` eski giriş/cevap/iş yükü/ölçüm baytlarını korur;
`before` modu var olan kayıtları yeniden üretmez.

## Aynı büyük belge ve ölçüm sınırı

`workload.kcad`: 200 duvar, 200 analitik kolon, 200 ayırıcı, 400 mahal,
20.000 ilgisiz çizgi; toplam 21.000 nesne. Her yapıda bağımsız beklenen alanlar
34,22 + 38,22 m², toplam **14.488 m²**. Kaynak hareketiyle 42,02/30,42,
400 bağlı etiket ve çizelge, tam Undo ilk workload scriptinde doğrulanır.
`benchmark.py after` aynı KCAD'yi public Python'dan açar; beş tam raporu eski
400 satır/duvar/toplam/uyarı cevabıyla birebir karşılaştırır, belge baytları
her sorgudan sonra aynıdır. Bu, sorgu sonucunun saklanması değildir; her
tekrar yeni arama/ağ ömrüdür.

Önceki debug uzantının beş tekrar ortancası **8646,943 ms**, en yükseği
8801,436 ms. Yeni debug uzantıda beş sorgu **815,623 / 83,014 / 83,765 /
81,862 / 84,027 ms**; ortanca **83,765 ms**. İlk sorgu maliyeti ayrıca gösterilir.
Eski ölçüm workload üretimi/ön sorgular sonrasındadır; yeni ölçüm yeni Python
sürecindedir. Soğuk süreçler eşlenik değildir; sıcak tam sorgu karşılaştırmasıdır.
Apple M5/32 GiB/macOS 26.6.2, aynı iş yükü ve derleme yokken ölçüldü.
`benchmark-before.json`/`benchmark-after.json` uzantı ve iş yükü SHA'larını,
ham süreleri, tam cevap eşitliğini ve değişmeyen belge baytlarını kaydeder;
`environment.json` donanımın kimliksiz özetidir. Bunlar release, p95, GPU
karesi veya kullanıcı girişinden sunuma süre garantisi değildir. Bellek/tahsis
ve uçtan uca mimari hız matrisi bu ölçümle kapanmaz.

## Gerçek GUI

01–02: önceki doğrulanmış paketle Dosya Aç →21.000 nesne →Metraj,
400 mahal/14488 m² ve 200 duvar/7200 m. Derleme eşzamanlı olduğu için eski
GUI denemesinin araç çağrı süresi benchmark değildir. 02'de dikey kaydırma
çubuğu Brüt sütununu keserken sağda kullanılmayan alan kalıyordu. İç tablo
kaydırıcısı kullanılabilir eni dolduracak şekilde düzeltildi. 06–11'de
400 satırın kaydırılması, koyu/açık ve 13px gerçek pencere tekrarları vardır.
03–05'te aynı ayırıcı gerçek fare/sayısal girişle 1m taşındı: 34,22→42,02 m²;
tek Undo ile bütün küçük model birebir geri geldi. Büyük dosyada rapor/kayıt
sonrası bütün model eskiyle eşittir.

08'de gerçek CSV dışa aktarımı bağımsız okuyucuyla **400 veri satırı, 12 sütun,
200×34,22 +200×38,22 =14488 m²** doğrulandı. Çıktı bildirimi 600 satır diyor:
bütün çalışma kitabının satırlarını sayması ayrı ARCH-DOC-02 takip işidir;
CSV'nin gerçek satır sayısı olarak sunulmaz.

12 yalnız başarısız pencere boyutlandırma denemesidir (Metraj kapalı); 14'te
yatay çubuk sürüklemesi konumu değiştirmedi. 15 ve 18 sağ uçta kırpılan son
rakamın kırmızı kanıtıdır; başarı değildir. 16–18 paketinde verilen
`Scrollbar::spacing` değeri iced_widget 0.14.2'nin `Direction::Both` layout
yolunda kullanılmadığı için nihai çözüm değildir. Son çözüm çubuk enine bağlı
sağ/alt içerik boşluğudur: 19 geniş, 20 dar/sağ uç, 21 dar/sağ-alt uçta
Brüt 37,82/41,82 tam görünür; 22 son kayıt. Son paketin gerçek CSV baytları
önceki doğrulanmış CSV ile aynıdır, kaydedilen bütün büyük model korunur.

Ham piksel boyutları 2880×1740/1742 ve 2200/2248×1742 civarındadır; Retina 2x
mantıksal en 1440 ve 1100/1124, yükseklik yaklaşık 839'dur. Bunlar plandaki
1440×900 /1100×650 tam matrisinin yerine geçmez. 18px dar görünümde yatay
kaydırma gerekir; düğmeler erişilebilir kalır.

Ham JPEG/event/SHA kayıtları `gui-events.json` içindedir; resimler yeniden
çizilmez. `before-binary.json` önceki paket; `binary.json` genişlik düzeltmesi;
`final-binary.json` reddedilen iki yönlü spacing denemesi;
`accepted-binary.json` son kenar boşluğu paketinin kaynak/executable SHA'larını
ayırır. Test profili ve dosyaları `.run/architecture-quantity-cache` altındadır;
kullanıcının projesi değildir. `verify.py` eski/yeni raporlar, dondurulmuş
hash'ler, gerçek KCAD kayıtları ve gerçek CSV'yi bağımsız kontrol eder.

## Kontroller

- application1: 51 kütüphane +217 entegrasyon başarılı, 22 atlanan.
- domain1: 10+66+22+5 =103 başarılı; 0 testli doc-test başarı sayısına eklenmez.
- clippy1 yeni testte `= *method` boşluğunu yakaladı. Yalnız test yazımı düzeltildi.
- mcp2: gerçek MCP host'u üzerinden 6 başarılı test.
- python2: **300 test**, tüm `scripts/python/test.sh` ve son clippy başarılı.
- Son UI değişiklikleri clippy4/desktop-build4 başarılı; bütün hedefler
  uyarılar hata sayılarak kontrol edildi. Ham loglar ayrıca arşivlendi. Çekirdek/API testleri yalnız yerleşim satırları için yeniden
  sayılmaz; `test-runs.json` tam komut, süre, çıkış ve log SHA'larını taşır.

Web/ortak geometri/katalog/SDK/şema bu optimizasyonda değişmez. Önceki WASM
kontrolü ADR0679'de kayıtlıdır; bu oturumda tekrar çalıştırılmış sayılmaz.
Ana ARCH-SPACE-01/ARCH-PERF-01 ve genel mimari plan açık kalır.
