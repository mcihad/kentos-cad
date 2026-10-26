# 0672 — Mahalın kalıcı kolon düşümü ve kaynak güncelleme sırası

Tarih: 25 Ekim 2026. **Kolon düşümü dilimi uygulandı ve sınandı.**
ARCH-SPACE-01 bütünü açık kalır; sanal sınır, kaynak alt kümesi, uçtan uca
performans ve bütün iş akışlarının tam GUI matrisi bu dilimin kabulü değildir. ADR 0671'nin toplu oda ve legacy sorgu kuralları korunur.

## Üretici araştırması ve KentOS kararı

Autodesk'in [associative space düşüm belgesi](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-C2B86E49-C604-42D7-AAB9-157F5F712652.htm),
kolon/şaft gibi içerideki nesnelerin alanını düşmek için kaynağın sınır rolünün
açık olmasını ister. [Basic alan standardı](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Architecture/files/GUID-3B015299-DED5-4B6E-B799-678087746577.htm)
örneğinde kapalı çizim nesnesinin iç alanı Net'ten düşülür, Brüt'ten düşülmez.
Bu belge bütün kolon tiplerinin veya bütün ülke standartlarının aynı kuralı
kullandığını söylemez. [Revit oda alanı açıklaması](https://help.autodesk.com/cloudhelp/2019/ENU/RevitLT-Model/files/GUID-9348929C-18CA-40AC-BB64-71D01CC52F2B.htm)
kolonun oda sınırlama rolünü ve hesap yüksekliğini ayrı tanımlar.

KentOS çıkarımı: alan düşümü, render dolgusu veya katman/öznitelik adıyla tahmin
edilmemeli; kullanıcı tercihi kalıcı, yönteme bağlı ve görünür olmalı. Mevcut
2B model bir 3B hesap düzlemi veya mevzuat standardı uygulamaz.

## Kalıcı sözleşme

- `SpaceFields.columns`: `ignore` (alanı düşme), `net` (yalnız Net yönteminde),
  `all` (Net/Eksen/Brüt yöntemlerinin hepsinde fiziksel izi düş). Varsayılan ve
  eski dosyada yokluk **ignore**; mevcut mahaller sessizce yeni metraj almaz.
- Aynı seçenek `NewSpace`, `SpacesDiscover` ve `SpaceChange` ile erişilir.
  Mevcut v1 girdisinde yokluk eski davranıştır; yeni GUI aracında varsayılan
  **Yalnız net** seçilir. Öznitelikler paneli mevcut mahalın tercihini
  aynı `cad.spaces.edit` komutuyla değiştirir. Python/MCP aynı sözleşmedir.
- İlişkili mahal (`seed`) gerekir. Elle çizilmiş sınırın kaynak konturu ayrıca
  tutulmadığından, kolon hareket edince önceki deliği geri doldurabilecek bir
  kaynak yoktur. Bu yüzden elle çizilmiş mahalda etkin kolon tercihi açık
  rettir; çizim aracı bu durumu salt okunur açık metinle gösterir. Böl/Birleştir
  duvar bağını kaldırırken kolon ilişkisini de kaldırır; o anki geometri kalır.
- Kalıcı politika sonradan kaybedilmemeli. KCAD **şema 108** (birleştirmede 103’ten kaydı), yalnız etkin kolon
  politikası kullanan mahal varsa yazılır. Eski çizim aynı en eski şemada ve
  aynı baytlarda kalır. 102 ve önceki okuyucu 103'ü açıkça reddeder. Yeni alanın
  eski şema etiketi altına saklanması da reddedilir. Rust ve bağımsız Python
  okuyucu/yazıcıları ile byte fixture'ları birlikte güncellenir.

## Geometri ve kaynak kapsamı

- Duvarın kapattığı yüz önce mevcut yakın/tam arama ve yöntemle bulunur.
  `seed`, bu duvar yüzünün kimlik noktasını seçer; kolon sonradan üstüne
  taşınırsa oda kimliği kaybolmaz. Toplu bulma bölge süzgeci de bu duvar yüzünün temsil noktasını kullanır;
  kolon tercihi bölge üyeliğini değiştirmez. Çıktı etiketi kullanılabilir sınırın içindedir.
- Kaynak yalnız gerçek `Column` nesnesidir. Açık `arch.level` varsa yalnız aynı
  kimliğin kolonları; yoksa mevcut Tüm katlar kapsamıyla bütün kolonlar alınır.
  Kot eşitliği kat kimliği yerine geçmez. Gizli/kilitli nesnenin fiziksel izi
  değişmez. Evre filtresi ve kesit yüksekliği bu 2B dilimde yeni kural değildir.
- Dikdörtgen gerçek en/boy/dönüşüyle; daire iki analitik yarım yayla alınır.
  64 noktalı render dolgusu hesap kaynağı olmaz. Sonlu olmayan, uzunluksuz veya
  FP64'te temsil edilemeyen iz sessizce atlanmaz.
- Aynı oda sınırından bütün etkin izlerin birleşik kapladığı alan çıkarılır:
  örtüşen kolon iki kez sayılmaz; sınırı aşan yalnız kesiştiği kısmı düşer.
  Dışarıdaki kolon etkilemez. Bir oda çok parçalı kalabilir; kolon düşümü yeni
  duvar yüzü/oda kimliği üretmez. Tam kaplanan oda tek-mahal oluşturmasında açık
  rettir; toplu bulmada boş bir öneri üretilmez.
- Kaynak okunamıyorsa yeni yazma atomik ret; mevcut mahal son sınırını ve açık
  kopuk kaynak durumunu korur. Tam kaplanan mevcut mahal da son sınırını kayıp
  kullanılabilir alan durumuyla tutar. Kaynak düzelince aynı UUID yeniden hesaplanır.

## Güncelleme ve undo

- Commit/grup sonu sırası: ilişkiler → açıklıklar → **aksına yerleşen kolonlar**
  → **mahal sınırları** → etiket/görünüş/metin/çizelge. Kolonun eski konumunu
  okuyup bir sonraki işlemi beklemek kabul edilmez.
- Kolon ekleme, silme, taşıma, biçim/en/boy/dönüş/kat değişimi hem eski hem yeni
  etki alanındaki ilişkili mahalleri geçersiz kılar. Aks değişiminin türettiği
  kolon hareketi aynı komutun kaynak işlemleri arasındadır.
- Mahal tercihi/yöntemi/katı değişince plan ve execute aynı sınırı gösterir.
  Bir undo aks+kolon+mahal+etiket/tabloyu önceki hâle döndürür; redo aynı
  kalıcı kimlikleri ve alanı geri getirir. Elle çizilmiş eski mahaller sabittir.
- Toplu bulma ve tek Mahal aynı alan düşümünü kullanır. Yeni öneri sayımı ve
  mevcut oda tespiti farklı yöntem/kolon tercihinden dolayı yinelenen mahal
  üretmez. Kısmi elle çizilmiş çakışma bütün yazmayı engellemeye devam eder.

## Kabul kapıları

- [x] Analitik çekirdek: dik/dönmüş dikdörtgen, daire, örtüşme, sınırda kolon,
  tamamen dışarıda, iki parçaya bölen ve tamamını kaplayan iz; uzak koordinat.
- [x] Native: Net/Eksen/Brüt × üç tercih, aynı kot farklı kat, legacy/ignore
  korunumu, kaynak ekle/sil/taşı/yeniden boyutlandır; plan salt okunur ve atomik ret.
- [x] Aks → kolon → mahal → etiket/çizelge, tek undo/redo ve grup sonu; tam
  kaplanma/kopuk kaynak düzelince aynı UUID'nin geri gelmesi.
- [x] KCAD: 103 çift okuyucu/yazıcı byte eşitliği, eski byte fixture korunumu,
  daha eski schema altında yeni politika reddi; önceki masaüstü/SDK okuyucusunun
  `file_unreadable` yanıtında desteklenmeyen 103 ve son desteklenen 102 bilgisi.
- [x] Python/MCP: aynı komutlar, journaling, miktar, dosya ve kalıcı tercih;
  bağımsız ortak fixture'lar, katalog ve SDK güncelliği.
- [x] Gerçek GUI: araç tercihi/önizleme/kolon hareketi/tek onay/undo/redo,
  Öznitelikler, etiket ve çizelge, kayıt/aç; ham ekran görüntüleri ve bağımsız
  dosya/alan/kaynak korunumu. Boyut/font/tema kapsamı tek tek kaydedilir.

İkinci gerçek build'de araç/önizleme, tek mahal ve toplu tek Enter, boş tekrar,
undo/redo, Öznitelikler'de Net/Brüt ve üç kolon tercihi, aks → kolon → mahal →
etiket/çizelge ve kayıt/aç kabulü sınandı. Dar koyu/18 px pencerede bulunan çizim
kipi etiketi ve tam kaplanma açıklaması kırpılması dördüncü build’de düzeltildi
ve gerçek pencerede yeniden görüldü. Uyarı satırı sabit yükseklik yerine tam
en/içerik yüksekliği kullanır; 240/312/420 px × 13/18 px yerleşim testi ve
gerçek açık/koyu × dar/geniş × 13/18 px sekiz görünümde açıklama bütündür.
Bu sekiz görünüm uyarı panelinin matrisidir; her mahal iş akışının bütün
matriste tekrarlandığı iddia edilmez. Ham görüntüler ve bağımsız
KCAD/Decimal doğrulaması kabul klasöründedir. Tam kaplanmada çizelge son geçerli
sayısını korur; yeni güvenilir sıfır alan iddiası yoktur, mahal kopuk kaynak
uyarısı taşır. Çizelgeye satır bazında kaynak durumu sütunu genel belgeleme
iş akışının ayrıca ele alınacak koşuludur.

Geniş test yeni analitik mahal deliğinin Yarıçap/Yay uzunluğu ölçüsüne mevcut
genel kenar yoluyla zaten girdiğini, fakat yetenek kataloğunun `space` türünü
listelemediğini gösterdi. Katalog iki işlemde analitik yay koşulunu açıkça
söyler; düz mahalın ölçülemediği ret korunur. Native test gerçek yarıçapı
0,5 m / yarım yay açısını π ile, planın salt okunurluğunu ve tek undo'yu
yerel/uzak koordinatta doğrular. Seçili geometri fixture'ının dosya sırasına
bağlı olarak eğri/düz gelmesi genel yetenek testinin sonucunu değiştirmez.

Sanal ayırıcı ve seçili kaynak alt kümesi ayrıca kalıcı kaynak modeli ister;
kolon düşümü bunların tamamlandığı anlamına gelmez. Performans iddiası yeni
kolonlu uçtan uca ölçüm olmadan ADR 0671 çekirdek sorgu oranlarından türetilmez.
