# Katlı duvar dönüşümü — kabul kaydı

25 Ekim 2026, `codex/architecture-2d-workflows`, [ADR 0668](../../adr/0668-architectural-wall-level-isolation.md).
Bu uygulama diliminin kod, otomasyon ve gerçek arayüz kabulü geçti; genel mimari yol haritası açıktır.

## Davranış

- Mevcut Çizgiden duvar aracında Kat (L), kat düzenleme bağlantısı ve açık seçim.
  Yeni oturum Belirtilmedi ile başlar; proje/katman adından kat tahmin edilmez.
- `cad.walls.convert.arch` bütün çıktı duvarlarına gider. Each/Pairs/Contours aynı
  native doğrulamayı kullanır; yeni komut veya dosya şeması yoktur.
- Kontur tekrarı aynı hedef katman ve aynı semantik katta reddedilir. Başka
  kattaki mevcut duvar kaynak dönüşümünü engellemez.
- Ortak FP64 birleşim çözücüsü köşe/uç/T/çaprazları aynı katla sınırlar; katsız
  duvarlar ayrı gruptur. Eşit kotta farklı kimlikler farklı katlardır.
- Bilinmeyen veya taslak sırasında kaldırılmış kat atomik ret; sessiz katsız
  yazma yoktur. Yinelenen adlar hedef tahmin ettirmez; kimlikler harfe duyarlıdır.
- Kat kimliği 2B XY koordinatlarını kaydırmaz. Birim/ölçek/kamera/render hattı
  değiştirilmez; yeni duvarlar normal mevcut mimari duvarlardır.

## Sayısal ve otomasyon kapsamı

- Önceki 93 `cad.walls.convert` durumu birebir korunur; 25 bağımsız vaka
  eklendi. 24'ü m/mm/ft, yerel/uzak koordinat, ters/doğru halka, Koru/Sil ve
  iki hedef katı karşılar; mevcut çakışık duvar diğer kattadır. Sonuncu vaka
  bilinmeyen katı kaynak silinmeden reddeder.
- Core: mevcut ölçülü birleşim referanslarının tamamı ortak açık katta eski
  sonuçla aynıdır. Her duvara ayrı kat verilince sonuçlar bağımsız tek duvar
  çizimleriyle birebirdir; köşe/T/çapraz/çakışma/uç sorunları katlar arası değildir.
  Seçili indisler, analitik yay ve açıklık aralıkları ayrıca sınanır.
- Depo: komşu duvar kat değiştirince iki duvarın kapakları/yüzleri yeniden
  kurulur; eski birleşim kalmaz, aynı kata dönünce birleşim geri gelir.
- Native: beş birim ve iki koordinat başlangıcı; katsız/zemin/üst katı sırayla
  aynı kaynaktan oluşturma, aynı katta tekrar ret, planın yazmaması, tek
  undo/redo ve eski Each/Pairs mimari bilgisi. Bilinmeyen kat kısmi yazmaz.
- Etkileşim: görünür seçim, kısayol döngüsü, yeniden planlama, kaldırılmış
  hedef, yinelenen ad ve büyük/küçük harfi farklı kimlikler.
- MCP: gerçek sunucuda şema, plan/execute, farklı kat, tekrar ve bilinmeyen kat.
- Python: üretilen SDK'da aynı isteğe bağlı alan; bütün komut durumları,
  bağımsız geçmiş ve KCAD kalıcılık denetimi çalıştırılır. Sonuçlar ham loglardadır.

## Gerçek arayüz — CUA, elle fare/klavye

`before.kcad`: iki 4×3 m kaynak oda, ortak sınırları üzerinde 7 m uzunluğunda,
0.4 m genişliğinde Zemin kat duvarı. İki oda 1. kata dönüştürülür; mevcut
Zemin kat duvarı kesilmeden kalır. Gerçek uygulamanın ayrı test kopyası ve
ayrı kullanıcı ayar dizini kullanıldı; kullanıcı çizimi değiştirilmedi.
`gui-binary.json` denenen programın SHA-256'sını kaydeder.

1. Kaynak iki alan seçildi. Seçimin açtığı Alan bağlamsal sekmesi nedeniyle
   ilk şerit tıklaması Döndür aracına geldi; hiçbir nokta/veri yazmadan iptal
   edilip Mimari sekmesinden doğru araç açıldı. Bu giriş hatası kayıtta saklıdır.
2. Kat başlangıcı Belirtilmedi; listede iki proje katı ve Katları düzenle bağlantısı.
3. Kapalı kontur, Eksen, 0.200 m, 1. kat ve Koru seçildi. Önizleme normal
   birleşim yüzlerini ve ortak sınır uyarısını gösterdi; belge yazılmadı.
4. Katları düzenle mevcut Proje ayarları › Mimari'yi açtı. Vazgeç ile taslak,
   seçilen kat ve kaynak seçimi korundu.
5. Sağ tuş › Onayla bir kez tıklandı. 3→6 nesne; yeni üç duvar toplam 25 m.
   Eski Zemin kat duvarının beyaz yüzleri kesilmedi; üst kattakiler kaynağın
   mavi rengini aldı. Kaydedilen dosya bağımsız okuyucuyla karşılaştırıldı.
6. Tek geri al 6→3, tek yinele 3→6. Her durum gerçek arayüzden kaydedildi.
7. Aynı 1. katta tekrar üretim ret; Zemin kattaki mevcut duvarla çakışma da
   ret. Hızlı erişim Kaydet ile ret sonrası dosyalar ayrıca kaydedildi.
8. Uygulama kapatılıp aynı dosyayla yeniden açıldı; altı nesne ve ölçek korundu.
9. Gerçek 1100×650 pencerede açık/13 px ve koyu/18 px Kat satırı, liste ve
   seçim denendi. 18 px listesi yukarı açılıp pencere içinde kalır; alt ayarlara panel kaydırmasıyla erişim de doğrulandı.
   Yakınlaştırma ve Home görsel olarak kontrol edildi.

20 ham JPEG `shots/` içindedir; yeniden boyutlandırılıp üretilmiş arayüz
örnekleri değildir. CUA girdilerinin elle tutulan özeti `gui-events.json`'dır;
uygulamanın kendi olay kaydedicisi değildir. Açık/13 px geniş pencere iç
boyutu 1440×828, dar pencere 1100×650'dir (başlık dahil ham JPEG boyutları
2880×1720 ve 2200×1364). Bu kayıt 1440×900 veya tam tema/font matrisi iddiası taşımaz.

`python3 docs/baseline/2026-10-25-architecture-wall-levels/verify.py`, SDK veya
native geometri kullanmadan bağımsız KCAD okuyucuyla kaynak UUID/nesne/ayar
korunumunu, tam çıktı yollarını, katları, 25 m uzunluğu, undo/redo/ret ve
kaydet/aç dosyalarını denetler. Varsayılan metre KCAD ayarlarında yazılmayabilir;
okuyucunun eksik anahtarı korunur ve şema varsayılanı m olarak değerlendirilir.

## Açık kalan kapsam

`ARCH-DRAW-03` [~] kalır. Etkin/görünür kat yönetimi, mahal katı, geniş karışık
kaynak ve büyük ağ performansı, tam gerçek tema/font/pencere matrisi bu
kayıtla tamamlanmış sayılmaz. Yol haritasının diğer mimari maddeleri de açık kalır.

## Test çağrıları ve ham kayıtlar

Sonuçlar `test-results.json`, ham çıktılar `logs/` içindedir. Toplam 4.093 test
geçti: commands 799, core/native/interaction 1.732, masaüstü 1.280, MCP 3,
Python 279. İlgili ignore/ölçüm kapıları loglarda ayrı görünür. Clippy seçili
paketlerde bütün hedeflerle `-D warnings` geçti; optimize geometry WASM derlendi.
Fixture/SDK/manifest/şerit/bağımlılık yönü kontrolleri ve bağımsız GUI kayıt
denetimi geçti. Tam komutlar, ham log/KCAD/JPEG hash'leri `test-results.json`'dadır.
 İlk genel test çağrılarında bu depoda olmayan `pytest`
ve masaüstünde olmayan `--lib` hedefi seçildi; ürün testleri bu çağrılarda
başlamadı. Doğru `unittest discover` ve masaüstü binary hedefiyle çalıştırıldı;
ilk çağrı çıktıları da saklanır. Test veya alt süreç başarısızsa sonucun geçtiği
iddia edilmez. Otomatik TS dosyaları Git dışı `.run` dizininde üretildi; yalnız
mevcut `WallsFromLines.ts` ve ortak katalog/SDK güncellendi.
