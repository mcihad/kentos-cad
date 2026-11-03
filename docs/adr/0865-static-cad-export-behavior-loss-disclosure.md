# ADR 0865 — Statik CAD aktarımında davranış kaybını önceden bildir

Tarih: 2026-11-03. Kapsam: CAD-ADV-06, native masaüstü, ortak DXF yazıcısı,
Python ve MCP. Web arayüzü bu kabulün kapsamında değildir.

## Bulgular ve karar

Birleşik gerçek arayüz kabulünde iç içe, görünürlük/lookup/formül içeren
bileşenin seçilmiş yayını 1100 mm’den 1200 mm’ye yükseltildi. KCAD kaydet–aç
ve tek Undo/Redo hem değerleri hem nesneye bağlı ölçüyü korudu. Statik DXF
yeniden açılınca güncel 1200 mm değeri ve her yerleştirmenin görünüşü
korundu; parametrik düzenlenebilirlik ve ölçünün kaynak nesneye ilişkisi
korunmadı. İlk kayıp yazım sonrası raporda vardı fakat yazım öncesi
pencerede görünmüyordu; ikinci kayıp ayrıca raporlanmıyordu.

DXF/DWG dışa aktarım penceresi, kapsam seçicisinin hemen altında ve dosya
yolu sorulmadan önce, dinamik tanımların statikleşeceğini açıklar.
Parametre, formül, görünüş seçimi ve katalog davranışı kaybını, güncel boyut
ve görünüşün yazılmasından ayırır. Kaynağın KCAD olarak saklanmasını söyler.
Yazıcı kullanılmayan kitaplık tanımlarını da yazdığı için sadece seçili kök
nesnelerin `dynamic` alanına bakılmaz: bütün tanım metadata’sı sayılır.
Uyarı için pahalı varyant üretimi veya dosyaya yazma yapılmaz.

Seçili köklerde veya yazılan kitaplık tanımlarında `Dimension.refs` varsa
pencere ayrıca ölçünün güncel değerinin/çiziminin korunacağını, fakat yeniden
açılan dosyada geometri değişiminin kendiliğinden izlenmeyeceğini söyler.
Yazıcı aynı metni `Ölçü bağı`, `severity=warning` rapor notuyla verir. Bu
not her iki ölçü formunda ve KentOS metadata açık/kapalıyken üretilir;
ayrı bir Python/MCP uygulaması yoktur. Plan dosya yaratmadan aynı raporu
verir; yürütme aynı raporu döndürür. Bağsız ölçü için bu not üretilmez.

KentOS metadata seçeneğinin ve ölçü raporunun “bütün alanlar” vaadi kaldırılır:
korunan kapsam stil/görünüş seçenekleridir. Kaynak KCAD’ye ve bağlı ölçüye
hiçbir yazma yapılmaz. Rapor değişikliği DXF geometri baytlarını değiştirmez.

Bu karar DXF biçiminin ilişki taşıyamadığı iddiası değildir. KentOS’un mevcut
yazıcı/okuyucu yolunun sınırıdır: native DIMENSION’da ilişki yazılmıyor,
blok biçiminden geri okurken `refs` bırakılıyor. İlişkili DXF nesneleri ve kaynak kimliği
eşlemesiyle tam uyumluluk bu dilimde uygulanmış sayılmaz.

## Kod ve doğrulama

- `apps/desktop/src/exchange/dxf_export.rs`: kapsamın üstündeki görünür
  kayıp bildirimi; seçilmiş kök ve kitaplık ölçü ilişkisi kontrolü.
- `crates/shared/formats/src/dxf/mod.rs`: arayüz/rapor ortak ilişki kaybı metni.
- `crates/shared/formats/src/dxf/writer/entities.rs`: başarıyla yazılmış
  bağlı ölçünün raporu; ölçü metadata açıklamasının doğruluğu.
- `crates/shared/formats/tests/dxf_write.rs`: native/blok × metadata açık/kapalı,
  kaynağın değişmezliği ve geri okunan ölçünün bağsızlığı.
- Masaüstü testleri yalnız iç içe kullanımı ve kullanılmayan tanımın
  uyarısını; Python/MCP birleşik bileşenin plan/yürütme raporunu, yazısız
  planı, değişmeyen KCAD’yi ve geri okunan 1100 mm ölçüyü sınar.

Son kaynakla 51 DXF yazıcı, 1624 masaüstü, 66 MCP, 12 başsız DXF ve 56 Python
blok testi, Clippy ve derleme geçti. Gerçek fareyle büyük/en küçük pencere,
kaydırma ve sabit eylemler, iç içe üst blok seçimi/Tümü geçişi, DXF/DWG
uyarıları, Vazgeç, DXF yazımı ve yeniden açma doğrulandı. GUI/Python çıktısı
bayt bayt aynı; kaynak KCAD aktarım ve iptallerden sonra başlangıçla aynı.
Yeniden açılan GUI kaydını bağımsız KCAD okuyucusu da kabul etti. Birleşik
1100→1200 mm yayın yükseltme ve Undo/Redo kabulü ADR0864’teki kaynak
kayıtlarıyla tamamlandı. Ham görüntüler ve test dosyaları Git’e eklenmez.

## Resmî dayanak

[AutoCAD 2026: Associative Dimensions](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-DidYouKnow/files/GUID-7C7B6F58-2E17-4A77-9579-D42E205800BF.htm)
ilişkili ölçünün geometriyi izlemesini, bağsız ölçünün düzenlenebilir olmasına
rağmen bunu yapmamasından ayırır. KentOS’un uyarısı da bu kullanıcıya görünür
farkı anlatır; çizimin doğru görünmesini ilişki uyumluluğu saymaz.
