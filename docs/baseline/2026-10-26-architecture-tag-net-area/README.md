# Ayrı net mahal etiketi — 26 Ekim 2026

ARCH-SPACE-02'nin `{netArea}` dilimi; [ADR 0659](../../adr/0659-room-tag-net-area.md).
Bu dilimin aşağıdaki otomatik ve gerçek GUI kabulü tamamlandı.
Genel mimari yol haritası ve tam görünüm matrisi açık kalır.

## Anlam ve korunacak davranış

`{area}` mahalın seçilmiş Net/duvar ekseni/brüt sınırının alanı olarak kalır.
`{netArea}` aynı kat/kapsam/kaynak ve kolon tercihleriyle iç yüzlerin alanıdır.
Net yönteminde bile seed varsa mevcut Metraj'ın kaynak hesabı kullanılır;
dış güncelleme yalnız kaynağı getirmişse eski kayıtlı sınır net sanılmaz.
Elle çizilmiş geçerli Net sınır kendi delikleri ve parçalarıyla okunur.
Elle çizilmiş brüt/eksen veya kopuk sınır için net alan boş kalır. Birim ve
basamak mevcut alan ayarıdır; uzunluk birimi mm seçilince alan büyümez.

Bu fiziksel modelin hesabıdır; kuruma özgü/yasal net alan profili, tüm BIM
katmanlarının düşümü veya ISO uyumluluk iddiası değildir. UI'ya geometri
kopyalanmaz. Yeni komut, KCAD alanı veya TS/TSX dosyası yoktur; mevcut
`cad.tags.create/edit`, Python ve MCP kullanılır. `TagRead` tek immutable
belge sorgusu boyunca kaynak ağlarını paylaşır; düzenlemeler arasında cache
yaşamaz. Net-only kolon değişimi brüt mahalı değiştirmese de yeni etiketin
metni aynı undo'ya katılır. Elle konmuş etiket yeri ve ankrajı korunur.

## Otomatik kontroller ve erken denemeler

`test-runs.json` gerçek komutları, zamanları, çıkışları ve ham günlük
hash'lerini kaydeder. Erken hatalar son kabulün yerine sayılmaz:

- `python-red`: eski SDK/host yeni `{netArea}` yerini reddeder.
- `cases-first`: bağımsız kurulumda 17/18 slotları eklendiği için eski
  prelude artık 12'den başlamıyordu; manual örneğin line sözleşmesi de
  yanlışlıkla pts kullanmıştı. Yeni örnekler bağımsız iki nesneli kurulum
  aldı ve line a/b'ye düzeltildi. Ürün hesabı değişmedi.
- `cases-second`: çok parçalı/delikli örnek için eski dikdörtgen ankraj
  referansı uygun değildi. Delik küçük ikinci parçaya kondu; büyük parça
  simetrik kaldı ve ankraj matematiksel olarak merkezdir. Undo/redo'ya da
  diğer adımlarla aynı 1e-9 geometri karşılaştırması eklendi. Ürün geometrisi,
  algoritma veya toleransı değiştirilmedi; alan beklentisi aynı kaldı.
- `mcp`: duvar değişiminde mevcut anchor takibinin 5.000000000000001
  sonucu için yeni testin kesin JSON eşitliği uygun değildi. Elle konumun
  1e-9 geometri kontrolüne düzeltildi; undo/redo tam snapshot eşitliği sürer.
  Ürünün aynı eski aritmetiği ve toleransı korunur.
- `python-focused`: testte mevcut 0–4 sınırı yerine altı alan basamağı
  istenmişti; dört basamağa ve bağımsız Decimal biçimine düzeltildi.

Başarılı sonuçlar:

- `cases-final`: **225** ortak durum, **30 yeni** net alan durumu.
  Üç yöntem × dört kalınlık × ters yön/büyük koordinat; altı çizilmiş/
  kopuk/delik/çok parça durumu. Validate/plan yazmaz, tek undo/redo ve
  bütün kaynak nesneler karşılaştırılır. Önceki 77 create durumu yalnız
  izin verilen tokenları sayan hata metni dışında birebir korunur.
- `python-focused2`: **yedi test**, 27 yöntem/kolon/alan birimi birleşimi;
  gerçek KCAD kayıt-aç, açık kat ve kaynak alt kümesi, net-only kolon
  değişimi, tek undo/redo, iptal, güncel ayarda geçmiş, kaynak kaybı,
  analitik daire/büyük koordinat, sanal ayırıcı, tam kaplanma ve kilitli
  katmanda validate/plan/execute reddi.
- `domain`: **68**; gerçek dış kolon güncellemesiyle Net ve brüt kaynak
  sınırı/revision/dirty değişmeden yeni net yazı; reddedilmiş ayar adımının
  tam geri dönmesi. Son dört basamaklı test kaynağı `native-final` içinde tekrar geçti.

- `mcp-final`: **iki gerçek host testi**; yeni net etiketin plan/execute/
  kaynak kalınlığı/tek undo/redo akışı ve önceki kot/numaralandırma akışı geçti.

- `native-final`: **68 domain + 697 interaction + 219 application** başarılı;
  interaction'da bir ve application'da 22 mevcut atlanan test vardır.
  Ortak 225 durum bu koşunun içinde de geçti. Önceki duvar/açıklık/mahal,
  tutamaç, kot etiketi, tek onay/iptal ve Python geçmişi kontrolleri dahildir.

- `python-all`: **318 test** geçti; dokuz yeni testin son ikisi atanmamış
  kapsamın başka kattaki kaynakları dışlamasını ve net/kot karışık şablonun
  elle yerini korumasını kapsar. Python 3.14 sqlite test yardımcılarının
  kapatılmamış bağlantı ResourceWarning çıktıları ham günlükte korunmuştur;
  bunlar test başarısızlığı değildir, çözülmüş sayılmaz.
- `clippy-final`: contracts/domain/application/interaction/MCP/Python/desktop,
  tüm hedefler `-D warnings` geçti. `block 0.1.6` gelecek uyumluluk
  bildirimi günlüktedir; bu dilimde bağımlılık değiştirilmedi.
- `build-final`: gerçek masaüstü ikilisi derlendi; `wasm-contracts`:
  contracts WASM/no-default-features geçti. Web uygulama derlemesi yapılmadı.
- `kcad-reference`: önceki **1170** bağımsız dosya geçti. Katalog üç test,
  referans ve Python SDK üretim sapma kontrolü geçti.
- `independent-final2` ve son tüm günlükleri kapsayan `independent-final3`: **15 tam GUI kaydı / 21 ham ekran** ve ilgili
  paket/kaynak/günlük hash'leri geçti. İlk `independent-final` yalnız silinen
  ilk satırdan sonra okuyucunun yeniden verdiği gösterim ID'lerini eski
  değerlerde bekliyordu. Okuyucu mevcut `id=len(uids)` kuralıyla yeniden
  numaralar; beklenen silinmiş belge buna düzeltildi. Kalıcı UUID'ler,
  kaynak bağı, metin ve geometri kontrolü gevşetilmedi; ürün değiştirilmedi.

## Gerçek GUI

01 eski sürümde aynı dört nesneli sentetik belgeyi; 02 görünür yeni etiket
alanında `{netArea}` girdisinin reddini gösterir. Eski sürümde Kaydet sonrası
`before-rejected.kcad` bağımsız okuyucuyla `before.kcad`'e tam eşittir.
Ham ekranlara müdahale edilmez. `gui-events.json` zaman/hash/paket bağlantısı
ve değişmez test paketlerinin hash'leri tutulur; kullanıcı profili ayrı kalır.

Yeni sürümde gerçek fare/klavye ve Kaydet akışı:

- 03–04 yeni paket ve görünür şablon/kol/3,5 mm ayarları. Ayar kaydı dört
  kaynak nesneyi aynen korur. 05–06 **tanısal** otomasyon odağı hatalarıdır:
  yüzen koordinat kutusu metin alanı sanıldı; ilk yapıştırma odak almadı,
  sonraki tıklama etiketi yanlış yere koydu. Yanlış yerleşim undo ile
  kaldırıldı; kabul sonucu olarak kullanılmaz. 07 gerçek alt komut metin
  alanındaki `9,2` taslağıdır; 08 tek Enter ile bu noktaya yerleşimdir.
- 08–12 ayrı etiket brüt26,04/net21,79; oluşturma undo/redo; kolon `100 cm`
  tek Enter ile 1m ve net21,04; tek undo21,79 / redo21,04. Kaynak brüt mahal
  parçaları birebir aynı; `at=(9,2)` ve eski ankraj korunur.
- 13 kolon eni0 reddi görünür; Escape/Kaydet modeli değiştirmez. 14 duvar
  `40 cm` tek Enter ile brüt28,16/net19,16. 15 kaynak duvar silinince son
  sınır `lostEnclosure` uyarısı taşır, net token boşalır; 16 tek undo bütün
  kaynak UUID'leri ve net21,04'ü geri getirir.
- 17 uygulamanın **ayrı test profilinde**, gerçek arayüzden açık tema18px,
  2200×1366 ham piksel (1100×683 mantıksal) pencere; kolon `75 cm` tek Enter
  ile net21,48. 18–20 yeni etiket önizlemesi: ilk Escape hedefi bırakır,
  ikinci Escape araçtan çıkar; yeni nesne yazılmaz. 21 uygulama gerçekten
  kapatılıp yeni süreçte dosya açılır, net21,04 ve elle etiket yeri korunur.

Koyu13 yeni geniş pencere2880×1720 ham piksel, eski2880×1726'dır; yeniden
başlatma açık18 geniştir. `screenshot-dimensions.json` ölçülen boyutları,
`dark13-profile.json` / `light18-profile.json` gerçek test profilini kaydeder.
Kullanıcının asıl çizim ve profili kullanılmadı. Görüntüler kırpılmadı veya
üretilmedi; yalnız ham çıktı kaydedildi. Tam sekizli mimari kabul matrisi
bu iki profille kapanmaz. Dar büyük yazıda genel panel başlıkları/kısaltmaları
aynen görünür; bu dilim genel docking/taşma iyileştirmesi iddiası taşımaz.

`verify.py` tam belgeyi geometri, ayarlar ve kalıcı UUID'lerle karşılaştırır.
Değişmeyen eski kaynaklarda kesin eşitlik, yeni geometrik noktalarda1e-9
kullanılır; her model `models.json` hash'iyle bağlanır. Bağımsız referans:
6×4 eksen/0,2 duvar → brüt6,2×4,2=26,04; net5,8×3,8−0,5²=21,79.
Kolon1m→21,04; duvar0,4→6,4×4,4=28,16 ve5,6×3,6−1=19,16;
kolon0,75→22,04−0,75²=21,4775→21,48. `before.kcad` ve `seed.json`
kurulumu yeniden incelemeye açıktır.

## Ölçülen süre ve açık sınırlar

`measure.py` herkese açık Python komutlarını debug yerel ikiliyle ölçer;
her on kaynak hareketinden sonra bütün nesneler/UUID'ler/metinler/yerler
bağımsız beklenen belgeyle karşılaştırılır. Ölçüm sırasında derleme/test
ve test penceresi çalışmıyordu.

| Belge | Plan | Oluşturma | On kolon hareketinin ortancası / en uzunu |
| --- | ---: | ---: | ---: |
| 200 mahal +200 net etiket +400 kaynak, kolon Net | 35,050ms | 66,220ms | 26,854 /27,879ms |
| Aynısı +20.000 ilgisiz çizgi, kolon Düşme | 359,251ms | 700,158ms | 348,204 /364,214ms |

İkinci durum mevcut legacy kaynak yolunu da ölçer; yoğun belgede bu maliyet
hâlâ vardır. Bu bir önceki sürümle hızlanma karşılaştırması değildir;
release/p95/bellek/GPU/kare süresi veya her mimari aracın kabulü sayılmaz.
Net alan isteğe bağlıdır; eski varsayılan şablon yeni hesabı çağırmaz.
Kat/isim/kullanım listeleri, Koru/Yenile ve tam mimari matris açık kalır.
