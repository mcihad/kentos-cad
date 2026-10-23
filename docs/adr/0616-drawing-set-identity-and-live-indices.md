# 0616 — Pafta seti kimliği, antet alanları ve canlı dizin

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-26.

## Karar

Paftanın `drawingInfo` kaydı numara, başlık, revizyon, durum ve alt küme düzeylerini
taşır. DTO `kentos-contracts` içindedir; ortak pafta modeli ve komut sorgusu aynı
tipi kullanır. Sırf sorgu metadata'sı için native komutların şema/TS bağımlılığı
zorunlu yapılmaz; `--no-default-features` derlemesi ayrıca denetlenir. Eski kitaplarda alan yoktur; sekme adı ve önceki değişkenler korunur.
Numara/başlık/revizyon/durum ayrı alanlardır; belge revision sayacı ile paftanın
kullanıcının verdiği revizyon etiketi aynı anlamda değildir. Durum serbest proje
etiketidir; yetki veya yayın onayı kuralı yerine geçmez.

`setDrawingInfo` tek paftayı, `setDrawingSet` birden çok paftanın bilgisini atomik
olarak değiştirir. İkinci işlem iki mevcut numaranın karşılıklı değiştirilmesine
izin verir. Her iki işlemin tam tersi vardır; normal `cad.sheets.edit`, Python,
MCP, grup/changeset ve pafta tasarımcısının aynı geçmişi üzerinden çalışır.
Aynı batch'te tekrar kimlik, eksik pafta veya geçersiz son durum bütün işlemi
reddeder. Boş numaralar taslaklarda tekrarlanabilir; dolu numara set içinde
büyük/küçük harften bağımsız tekildir. Çoğaltma ve şablondan yeni pafta üretme,
yeni paftanın numarasını boşaltır; eski yayımlanmış kimliği kopyalamaz.

Alt küme adları hiyerarşinin ayrı düzeyleridir; önek denetimi metne değil
segmentlere uygulanır. `Mimari` alt kümesi `Mimari / Katlar`ı kapsar,
`MimariX`i kapsamaz. Arayüzde `/` ayırıcıdır; kayıtta bir String dizisidir.
Sınırlar ve geçersiz kontrol karakterleri ortak çekirdekte denetlenir.

Antetlerin `@pafta_no`, `@pafta_basligi`, `@revizyon`, `@pafta_durumu`, `@alt_kume`
alanları standart kaydı okur; kayıt yokken önceki değişken davranışı sürer.
Standart kayıt varsa aynı isimli serbest değişken ikinci bir otorite oluşturmaz.
Boş başlık sekme adını izler. Değer içindeki ifade işaretleri yeniden çalıştırılmaz.

`TableSource::DrawingSet` altı sütunlu canlı tablodur: numara, başlık, revizyon,
durum, alt küme, görünüm. Pafta dizininde kitap sırasıyla bir satır/pafta; görünüm
dizininde görünür ve basılan viewport başına bir satır vardır. Ortak ana sayfanın
görünümü her sahibi için ayrı satırdır. Adsız çizim görünümünün adı öğenin adıdır;
adlı görünüm bağlantısı güncel adı, snapshot ise yakalanmış adı taşır. Eksik
bağlantı satırda belirtilir. Hidden/nonprintable/yerleştirilmemiş görünüm ve
overview dizine alınmaz. Tabloya başlıklar elle kopyalanmaz; çıktı her seferinde
aynı kitaptan yeniden kurulur. Normal tablo taşma/preflight/continuation kuralları
geçerlidir.

Masaüstündeki **Pafta seti** penceresinde pafta değiştirirken taslak alanlar
kaybolmaz. Kaydet tek metadata batch'i, Dizini paftaya ekle aynı adıma canlı tablo
ekler. Python/AI aynı işlemleri ve tablo kaynağını kullanır. `SheetInfo` sorguları
standart kimliği de verir; `.kcad` ve `.kpafta` aynı kaydı taşır.

Toplu yayın ad şablonu `{number}`, `{title}`, `{revision}`, `{status}`, `{subset}`
alanlarını da kabul eder. Alanın içindeki `{id}` gibi metin tekrar yer tutucu
olarak yorumlanmaz. `yayin.json` seçilen paftaların standart kimliklerini taşır;
PDF hash'i ile hangi revizyonun teslim edildiği birlikte okunur.

## Doğrulama

Bağımsız Python referansının 35 olgusu alan sınırları, tekillik, alt küme üyeliği,
batch numara değişimi, atomik ret, tam ters işlem, kayıt/açma ve görünüm dizinini
sınar. Ortak çekirdek, native protokol ve tipli Python aynı dosyayı tüketir.
Native PDF ayrıca antet alanı ile dizindeki revizyonun birlikte değiştiğini,
eski başlığın çıktıda kalmadığını ve 100 mm çizginin boyunu denetler. MCP,
masaüstü undo, açık/koyu/büyük yazı ve yayın adları ayrıca sınanır.

Web UI ADR 0228 nedeniyle ertelidir. Desteklenmeyen bir çıktı kaynağını tam
olarak çizdiği iddiası bu metadata diliminin kapsamına girmez; CAD-SHEET-28
native kaynak ön denetimini tamamlar. Kabul: 35 ortak/native protokol ve WASM olgusu; exact inverse ve yeniden açma;
2 tipli Python metadata testi (35 olgu ve birleşik undo), 244 Python testinin
tamamı; native PDF antet/dizin/fiziksel boy, yayın adları/manifest ve MCP geçti.
3 masaüstü testi modelden pencere açmayı, tek undo'yu ve gerçek alan/klavye/dizin
düğmesini denetledi. Açık/koyu/büyük yazı görüntüleri açıldı; kaybolan alan
etiketleri ve scrollbar boşluğu düzeltilip yeniden görüntülendi. Gerçek PDF
ayrıca rasterleştirilerek okundu. Katalog/TS/SDK/manifest, clippy, minimal komut
özellikleri ve bağımlılık yönü denetimleri geçti. Web UI çalıştırılmadı (ADR 0228).
