# ADR 0855 — Kitaplık değişince hesaplanan blok değerlerini yenile

Tarih: 2026-11-02. CAD-ADV-06.

## Sorun ve dayanak

[AutoCAD 2026 Parameters Manager](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-A2AE7503-7EFF-4C67-B283-EC0898BE8C59.htm)
parametrenin ifadesi ile sonucunu ayrı gösterir; sonuç alanı doğrudan
düzenlenmez. Kaynak KentOS'un iç önbellek algoritmasını tarif etmez.
Bizdeki sorun kendi kodumuz, eski Python bağı ve gerçek uygulamayla üretildi:
kitaplıkta `@Genişlik / 2` yerine `/ 3` alındığında üç örneğin hesaplanan
değerleri 450/500/450 mm kalıyordu. İşlem başarılı görünse de değerler ve
bunları kullanan geometri yeni tanıma uymuyordu. Dosyadan yeniden tanımlamada
ve kitaplıkta yer almayan bir ana tanımın iç çocuğunda aynı hata vardı.

Üstelik varsayılan 0,9 m için geçerli olan
`@Genişlik / abs(@Genişlik - 1)` ifadesi, 1 m'lik mevcut örnekte tanımsızdır.
Eski doğrulama bunu kabul ediyordu. Dört yeni Python regresyon testi eski
bağda başarısız oldu; kanıtlar repo dışındadır.

## Karar

- Kitaplık ve dosyadan blok alma/yeniden tanımlama, son tanımları geçici
  kopyada hazırlar. Değişen tanımın formül varsayılanları mevcut ortak
  ifade değerlendirme yordamıyla denetlenir.
- Mevcut doğrudan yerleştirmeler ve değişmeyen ana tanımların etkilenen iç
  yerleştirmeleri eski hesaplanan önbelleği girdi saymaz. Kullanıcının
  literal girdileri korunur; yeni formüller her örneğin girdileriyle hesaplanır.
- Kitaplıktan açıkça gelen ana tanımın iç yerleştirmesinde kitaplığın
  literal girdileri kullanılır. Yeni tanımın hesaplanan alanları önbellektir;
  açık kullanıcı girdisi gibi yeniden uygulanmaz.
- Son tanımlar, yenilenmiş kök yerleştirmeler ve bütün blok kuralları yazmadan
  önce birlikte doğrulanır. Tek bir örneğin hesabı geçersizse kısmi yazma yoktur.
  Kitaplık hatasının yolu `library`; dosyadan alma hatası önceki `block_refused`
  ve `path` sözleşmesini korur. Mesaj etkilenen yerleştirmeyi belirtir.
- Tanımlar, değişmeyen ana tanımlardaki gerekli önbellek düzeltmeleri ve kök
  yerleştirme değerleri aynı Undo adımında yazılır. Plan salt okunurdur;
  Python ve MCP aynı native uygulama yolunu kullanır. Yeni şema/komut yoktur.
- Formülsüz örnekler gereksiz normalizasyondan geçirilmez. Uyumsuz literal
  değerler sessizce silinmez. Önceki atomik blok grafiği ve öznitelik konumu
  korumaları sürer.

Bu dilim ifade motorunun bağımlılık/döngü algoritmasını, birim dönüşüm
politikasını veya aile/yayın sürümü modelini değiştirmez. Sadece kitaplık ve
`cad.file.import.block` yollarının formül yenilemesini tamamlar; bütün çizimler
arası alışveriş yollarına kapsam genişletildiği iddia edilmez.

## Doğrulama

318 application (37 ignored), 57 MCP, 1604 masaüstü testi (256 ignored),
Clippy ve masaüstü derlemesi geçti. Dört Python regresyon testi doğrudan
örnekleri, kitaplıkta bulunmayan ana tanımdaki iç örneği, tek örnekte tanımsız
ifadenin validate/plan/execute reddini ve dosyadan yeniden tanımlamayı kapsar.
Tam Python paketinde 444 test geçti (340,916 s).

Gerçek eski GUI'de 450/500/450 mm önbelleği üretildi (210). Yeni GUI'de aynı
başlangıç ve kitaplık, gerçek konsoldan plan/expected-plan ile uygulandı;
sonuç 300/333,333…/300 mm oldu ve geometri yer değiştirdi (211). Kaydedilen
GUI KCAD baytları bağımsız native Python çalıştırmasıyla tam aynı. Gerçek
klavye Undo'sunun kaydı başlangıçla tam aynı. Üç aşamada reddedilen tanımsız
hesaptan sonraki gerçek GUI kaydı da başlangıçla tam aynı (212). Kaydedilen
doğru dosya gerçek Aç menüsünden yeniden açıldı; geometri ve örnek değerleri
korundu (213). Ekranlar, ham test günlükleri ve
`block-library-formula-gui-acceptance.json` repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` dizinindedir.
