# ADR 0849 — Blok değiştirirken örnek değerlerini koru

Tarih: 2026-11-02. CAD-ADV-06'nın güvenli değiştirme dilimi.

## Dayanak ve hata

[BricsCAD BLOCKREPLACE](https://helpcenter.bricsys.com/en-us/document/command-reference/b/blockreplace-command?version=V26)
aynı ad ve türdeki dinamik parametre değerlerini korur ve belirli örnekleri
seçmeye izin verir. [AutoCAD blok değiştirme](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-5E78781D-AD09-4C6F-A3C0-05BD99CB410F.htm)
yerleştirmenin ölçek, dönüş ve öznitelik değerlerinin korunmasını açıklar.
KentOS'un `cad.blocks.replace` yolu dinamik olmayan hedefte eski parametreleri
tutuyor, başarılı dediği çizimin kaydı `insert/parameters: blok dinamik değil`
ile reddedilebiliyordu. Öznitelikler panelinin ayrı geometri yazma yolu ise
tanım değişince bütün parametreleri sessizce boşaltıyordu.

## Karar

- Panelde Geometri › Blok, tek UID ile mevcut `cad.blocks.replace` komutuna
  gider. Python/MCP/GUI aynı doğrulama ve tek geri alma yolunu kullanır.
- Kullanıcının saklanan girdisi hedefte aynı ad ve parametre türünde kalır.
  Doğrusal sayı açıya, nokta kutupsala dönüşmez. Birimsiz ve fiziksel tanım
  arasında uzunluk girdisi tahminen dönüştürülmez; fiziksel birimler çekirdeğin
  metre sözleşmesine uyar. Yerleştirme ölçeği otomatik değiştirilmez.
- Eski formül sonuçları kullanıcı girdisi sayılmaz; hedefin formülleri yeniden
  hesaplanır. Hedef varsayılanına eşit girdi gereksiz override olarak tutulmaz.
  Hedefin min/max/adım/liste/görünürlük kuralları ortak çözümleyiciyle doğrulanır.
- Korunamayan bir girdi varsa hiçbir açık yerleştirme yazılmaz;
  `invalid_parameter_value`, ilgili UID/parametre yolu ve düzeltme açıklaması
  döner. Kullanıcı uyumsuz girdiyi açıkça sıfırlayabilir veya başka hedef seçer.
  Kilitli yerleştirmeleri atlayan mevcut kapsam ve uyarılar korunur.
- Konum/dönüş/ölçek ve mevcut öznitelik eşleştirme politikası değişmez;
  tanımın içindeki yerleştirmeler güncellenmez. Eski tanımı silmek varsayılan
  değildir. Bu işlem kaynak ailesi tanıyan kitaplık yükseltme arayüzü değildir.

## Kabul

50 ortak değiştirme durumu (28 yeni): uyumlu değerler, varsayılanlar, hedef
formülü, eksik/yeniden adlandırılmış/türü değişmiş parametre, görünürlük,
nokta/çevirme, sınırlar, birimsiz–fiziksel ayrımı, kilit ve atomik toplu ret.
317 application, 1599 masaüstü (256 mevcut ignored), 53 MCP testi geçti;
ayrı native kabulünde bağlı blok alt-geometri ölçüsü, kayıt–aç ve tam tek
Undo geçti. Dokuz Python blok testi, SDK/referans üretim kontrolü, masaüstü
derlemesi ve ilgili native/desktop/MCP hedeflerinde Clippy geçti.

Gerçek GUI'de uyumsuz düz hedef reddedildi, kaydedilen dosya başlangıçla tam
aynı. Uyumlu v2 hedefinde yalnız seçilen yerleştirme değişti; 1050 mm, Sade,
K-1 ve bağlı 1050 mm ölçüsü korundu. GUI dosyası plan kimliğiyle çalıştırılan
Python işlemiyle tam aynı; tek GUI Undo başlangıçla tam aynı; yeni dosya
uygulamada yeniden açıldı. Görüntüler 179–181 ve
`block-replacement-gui-acceptance.json` repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` altında tutulur.

[Kullanım/API rehberi](../cad/block-replacement-parameters.md).
