# 0617 — Kaynaklarıyla pafta teslimi

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-27.

## Karar

`file.publish.sheets` v1'in `resources: true` seçeneği PDF'lerle birlikte
`kaynaklar/cizim.kcad`, yerel bağımlılıklar, 22 tam TrueType font dosyası ve 7 OFL
lisansı, baskı profilleri ve sayfa düzenlerini tek atomik teslim klasörüne koyar.
Varsayılan PDF yayını aynı kalır; kaynak paketi seçilmediyse kaynaklarıyla tam
bir çizim teslimi üretildiği söylenmez.

Bağımlılık toplama ikinci bir uygulama değildir: mevcut `file.package` planlayıcısı
xref, bağlı veri kaynağı, resim, raster/yan dosya, nokta bulutu, tablo, ek ve proje
profillerini toplar, kopyaları tekilleştirir ve linkleri taşınabilir hâle getirir.
Yeni nested plan hedefin son `kaynaklar/` konumuna göre kurulur; henüz oluşmamış
üst teslim klasörü için plan diske yazmaz. Yazma dış yayının benzersiz geçici
klasöründe yapılır; ana teslim kesinleşene kadar kaynak paketi görünmez.

Bu teslim genel Paketle'den daha sıkıdır: bulunmayan/okunamayan kaynak, ağ veya
veritabanı kaynağı, harita servisi, yerine başka fontla çizilen stil ya da başka
sisteme taşınamayan dosya adı ön denetimde hata olur. Dolu hedefin üzerine
geçilmez. Sistem tarafından taşınamayan kaynak açıkça raporlanır; eksik teslim
başarılı gösterilmez. Yerel bir kopya hazırlanabilir veya kullanıcı kaynak paketi
seçimini kapatarak açıkça yalnız PDF üretir.

Font dosyalarının sadece adları yazılmaz; tam byte'ları ve lisansları taşınır.
Baskı kitaplığı ile paftada yakalanmış profil aynı kimlikle farklı içerik taşıyorsa
ikisi ayrı hash'li `.kplot` olur. Adlı sayfa düzenlerinin yakalanmış profilleri de
korunur. Geçerli ayarlar `.kcad` içinde, ayrıca `baski/sayfa-duzenleri.json` ve
standart `.kplot` alışveriş dosyalarında bulunur. Binary CTB/STB codec'i bu dilimde
eklenmez (CAD-SHEET-19'un yerel profil stratejisi sürer).

## Bütünlük, ilerleme ve tekrar istek

Kök `yayin.json` kaynak revision/hash'ini, pafta kimlik/revizyonlarını, PDF'leri ve
bütün kaynakların göreli yolu/rolü/boyutu/SHA-256'sını birlikte verir. Proje
paketinin eski `paket.json` manifesti de korunur; font ve baskı dosyaları dahil
çizim setinin tam envanteri kök manifesttedir. Her kopya ve üretilen dosya byte
boyu/hash'iyle geri okunur; yazma sırasında değişmiş kaynakta teslim kesinleşmez.

Sayfa ve kaynak kopyalama aşamaları aynı ilerleme sayacına bağlıdır. `Progress`
alt aralıkları aynı stop bayrağını paylaşır; PDF bitince kaynak aşamasında yüzde
sıfıra düşmez. İptal/hata geçici pafta ve kaynak klasörlerini kaldırır.

Aynı requestId ile tekrar gelen başarılı yayın yanıtı, sadece manifestin varlığına
güvenmez: bütün PDF ve kaynak dosyalarını doğrular. Dosya eksilmiş, değişmiş veya
sembolik bağla yer değiştirmişse `integrity_failed` döner; hiçbir şey yeniden
üzerine yazılmaz. Göreli yollar normal ve taşınabilir olmalı; klasör dışına çıkışa
izin verilmez. Bu SHA kontrolü içerik bütünlüğüdür; imza/onay yetkisi değildir.

İşlem günlüğünün recipe replay'i geçmiş render bağlamını yeniden üretmediği için
bu çağrıyı `same` diye etiketlemez. Mevcut teslimin hash doğrulamasını yapar,
`skipped` notunda bunu açıklar; bozuk teslimde yapılandırılmış hata da verir.
Böylece manifest hatalı `overwrite` alanıyla tekrar planlanmaz ve geçmiş çıktı
yeniden üretilmiş gibi sunulmaz. Kaynak snapshot/provenance CAD-SHEET-29'da
bütün çıktı yollarına tamamlanır.

## Doğrulama

Bağımsız Python üreticisinin 35 olgusu yerel/göreli/tekrarlı/aynı adlı kaynaklar,
raster yan dosyaları, tablo, ek, nokta bulutu, GeoPackage, xref, veri profilleri,
font/lisans, ayrı baskı snapshot'ları, eksik/ağ/servis/font kaynakları, taşınamayan
ad, hedef koruma ve on tür dosya bozma/silme/bağ değiştirme durumunu kapsar.
Teslim başka yere taşınır, eski kaynaklar kaldırılır ve teslimin çizimi açılır.
Python koşucusu bağımsız KCAD okuyucusu ve hashlib ile de linkleri/byte'ları okur.
Hazırlık sonrası değişen kaynak ve iptal temizliği ayrıca sınanır. Masaüstü ve
MCP aynı yayın komutunu kullanır; arayüz kaynak paketi seçimini ve envanterini gösterir.

Çalıştırılan kabul kontrolleri:

- Native dosya koşucusunda 35 bağımsız olgu; kaynak mutasyonu ve iptal için iki
  hazırlık/yazma testi; aşama sayacı ve ortak iptal bayrağı testi geçti.
- Eski `file.package` için iki dosya/taşınabilirlik regresyonu, masaüstünden gerçek
  teslim üretme ve MCP üzerinden font teslimi/bozuk tekrar isteği testleri geçti.
- Açık 13, koyu 13 ve koyu 18 ekranlarında seçenekler ve hazır raporu açılarak
  incelendi. Kaynak seçimi ve Denetle ayrıca gerçek fare girdisiyle sınanır.
- Katalogdaki üç kontrol, SDK/manifest tutarlılığı ve 47 crate/61 hedef bağımlılık
  yönü kontrolü geçti. Ortak WASM derlendi; mevcut smoke korpusunun tamamı geçti.

Python tam takımında 245 test ve sözleşmelerin 873 TS tür üretim testi geçti.
İlgili native/masaüstü/MCP/komut hedeflerinin clippy kontrolü uyarısız geçti.
Son bütünlük regresyonu sağlıklı request/recipe tekrarını ve on tür bozma
reddini de doğruladı (35 olgu). Gerçek kaynak anahtarı + Denetle fare testi ve
mevcut yayın ekranı regresyonu üç tema/yazı kombinasyonunda geçti; son ekranlar
tekrar açılarak incelendi.
Web UI ADR 0228 gereği ertelidir.
