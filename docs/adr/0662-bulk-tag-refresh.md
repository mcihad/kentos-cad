# ADR 0662: Toplu etikette açık Koru / Yenile

- Durum: kabul edildi (26 Ekim 2026); yalnız bu işlev dilimi, genel mimari kabulü değildir.
- Taban: `30e8a7dd5`, aynı mimari dal. ARCH-SPACE-02'nin eski etiket yönetimi.

## İncelenen davranış ve kaynaklar

`cad.tags.create` açık `tags` girdisiyle aynı kaynağa birden çok etiket
yerleştirebilir. `all` ise herhangi bir katmanda etiketi bulunan kaynağı
atlar. Etiket kaynaktan ayrıdır; metin/ankraj zaten kaynak değişikliklerini
izler. Toplu araçta önizleme ve görünür ayar bulunmaz.

[Revit Rooms and Room Tags](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-ArchDesign/files/GUID-65668F88-4429-4992-AA0D-1C4FBDCFA650.htm)
etiketsiz kaynakların topluca etiketlenmesini, etiket silinince mahalın
yaşamaya devam etmesini tanımlar.
[Archicad Label Zones](https://help.graphisoft.com/AC/25/INT/_AC25_Help/005_NewFeatures/005_NewFeatures-21.htm)
aynı mahalın farklı görünüş ve ölçekler için birden çok etiketini destekler.
Bu nedenle kaynağın tüm etiketlerini silip yeniden oluşturmak uygun değildir.

## Karar

- Yeni komut veya kalıcı veri alanı eklenmez. `TagAll.refresh` isteğe bağlı
  boolean'dır; yok/null/false bugünkü Koru davranışını ve cevap baytlarını
  korur. Açık `tags` girdisinin çoğul etiket davranışı değişmez.
- `refresh=true`, `all` tarafından seçilen görünür kaynakların yalnız
  `layerId` katmanındaki mevcut etiketlerini yeniler. Başka katmanlardaki
  etiketler değişmez; orada etiketi bulunan kaynak için kendiliğinden ikinci
  etiket yaratılmaz. Hiç etiketi olmayan uygun kaynağa mevcut kuralla yeni
  etiket eklenir. Aynı kaynağın hedef katmandaki birden çok etiketi ayrı
  kimlikleriyle korunur. Açık `tags` ile zaten verilen kaynak `all` dışında
  kalır; aynı çağrıdaki gizli çifte işlem önlenir.
- Yenile yalnız açıkça verilen `all.content/shape/size` alanlarını uygular.
  Boş ayar mevcut etikette o alanı korur, yeni etikette tür varsayılanıdır.
  Konum, UUID/slot, katman, renk, çizgi, kol, öznitelik ve kaynak korunur.
  Varsayılanı açıkça geri yükleme mevcut `cad.tags.edit` nullable alanlarıyla
  yapılabilir. Yenile yeniden numaralandırma veya yeni alan hesabı değildir.
- Plan yeni etiketleri `tags`, değişen mevcut etiketleri ayrı isteğe bağlı
  `refreshed` listesinde gösterir. Execute eski `uids/ids` alanlarında
  yalnız yeni etiketleri, `refreshed` içinde yalnız yenilenen UUID'leri
  döndürür. Boş `refreshed` serileştirilmez. Değişmeyen etiket yazılmaz.
- Doğrula/plan salt okunur; yeni+yenilenen etiketler birlikte en çok10.000.
  Kilitli hedef, yanlış şablon, eski revision veya slot kapasitesi tüm
  çağrıyı reddeder. Yazım tek işlem/undo; ret sonrası kısmi yazım yoktur.
- Araç görünür Koru/Yenile, tür, şablon/çerçeve/kağıt mm boyu alanları,
  sayılı plan ve gerçek önizleme sunar. Önizleme çizimi ilk200 etiketle
  sınırlıysa bunu açıklar; toplam işlem sayısını gizlemez. Sonuç tek Enter
  ile yazılır, ret araçtan çıkmaz; Escape taslağı bırakır.
- Etkin katman önizlemeden sonra değişmişse ilk onay yalnız yeni hedefin
  planını gösterir ve bunu açıkça bildirir; sonraki onay o planı uygular.
  Etkin katman değişimi belge revision'ını ilerletmediğinden yalnız revision
  kontrolü yeterli değildir. Normal akışta tek onay değişmez.
- Kaynak kapsamı eski kuralla görünür katmanlardaki tüm mimari katlardır;
  bu açıkça gösterilir. Çoklu pafta/ölçek bağlamı, ayrı görünüşte otomatik
  etiket ve mimari kat süzgeci bu dilimin kabulü değildir.

## Kabul kapıları

- Eski ortak durum/şema varsayılanları; yeni plan/execute, çoğul etiket,
  başka katman, gizli kaynak, kilitli hedef, karma yeni+yenile, boş tekrar,
  revision ve tek undo/redo; bağımsız beklenen tam nesneler.
- Gerçek Python ve MCP aynı alanlarla aynı sonuç; güncel SDK/katalog/TS.
- Gerçek eski/yeni native GUI, elle taşınmış etiket, ayarlar, ret, tek
  Enter, iptal, undo/redo, kaydet/aç; ham ekran ve bağımsız tam KCAD modeli.
- İlgili native takımlar, clippy/build, WASM/SDK ve bağımlılık kapıları.
  Belgede yalnız doğrulanan alt madde işaretlenir.

## Kabul sonucu

[Ham ekranlar, tam bağımsız KCAD ve çağrı günlükleri](../baseline/2026-10-26-architecture-tag-refresh/README.md).
31 yeni bağımsız durumla ilgili264 durum; eski107 create durumu aynen.
809 commands, 51+221 application (22 atlanan), son63+702 interaction
(bir entegrasyon/beş perf atlanan), 335 Python ve yeni gerçek MCP testi geçti.
İlk MCP takımının yardımcı beklenti hataları ve kalan ikilinin ayrı koşumu
kabul kaydında ayrıdır; tek yeşil tam MCP çağrısı iddia edilmez.
Clippy/build, ortak WASM, SDK/referans ve bağımlılık yönü geçti.
35 ham ekran, 24 tam beklenen native model ve bir tanısal eski disk kopyası
bağımsız okuyucuyla doğrulandı. İki tema/iki yazı/iki gerçek pencere paneli
incelendi; cihazın dış pencere yükseklik sınırı ve kapsam kayıtta belirtilir.
Gerçek GUI önce açıklama yerleşimini, ardından klavye ile katman değişiminde
önizlenmeyen hedefe yazmayı yakaladı; son ikilide ikincisi tekrar doğrulandı.
1756 önceki takipli KCAD dosyası bayt özdeş, 268 komut kimliği aynı; üç mevcut
TS tipi ve Python SDK güncel, yeni TS/TSX veya web arayüzü yoktur.
Çoklu pafta/ölçek bağlamı, mimari kat süzgeci ve tam mimari matris açıktır.
