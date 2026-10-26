# Toplu etikette Koru / Yenile — 26 Ekim 2026

[ADR0662](../../adr/0662-bulk-tag-refresh.md), ARCH-SPACE-02.
Durum: Koru/Yenile işlev dilimi kabul edildi; genel mimari kabulü değildir.

## Davranış

Hepsini etiketle, görünür Koru/Yenile, şablon/çerçeve/kağıt mm boyu,
sekiz kaynak türü ve sayılı plan sunar. Koru eski davranıştır. Yenile
uygun görünür kaynakların yalnız etkin etiket katmanındaki etiketlerini
kimlikleriyle değiştirir; elle konum, kol, kaynak ve belirtilmeyen alanlar
korunur. Başka etiket katmanlarına dokunulmaz; orada etiketi bulunan bir
kaynak için kendiliğinden ikinci etiket yaratılmaz. Yeni+yenilenen birlikte
tek undo'dur. Gizli kaynağın etiketi değişmez. Kaynak kapsamı görünür
katmanlardaki **bütün mimari katlardır**; ayrı kat/görünüş/pafta süzgeci değildir.

Mevcut `cad.tags.create` / `TagAll.refresh` ortaktır. Yok/null/false eski
çıktıyı korur. Açık `tags` girdisinin bir kaynağa birden çok etiket koyması
değişmez ve bu kaynak aynı çağrının `all` işlemine tekrar girmez. Planın
`tags` alanı yalnız yeni, isteğe bağlı `refreshed` alanı değişecek mevcut
nesnelerdir. Execute `uids/ids` yalnız yeni etiketler, `refreshed` mevcut
UUID'lerdir; boş liste cevapta yoktur. Şablon/boy boşken mevcut değer
korunur, yeni etikette tür varsayılanı kullanılır. Açık sıfırlama mevcut
`cad.tags.edit` nullable alanlarıyla yapılabilir. Değişmeyenlere yazılmaz.

En çok10.000 yeni+yenilenen; hata/kapasite/kilit/revision hepsi ya da hiçbiri.
Önizleme çizimi ilk200 ile sınırlıysa açıklanır; toplam sayı gizlenmez.
Ret araçtan çıkmaz, Escape taslağı bırakır. Yeni kalıcı KCAD alanı veya
sürüm yoktur; birim/ölçek/renderer değiştirilmez.

## Bağımsız ve otomatik kanıt

`test-runs.json` gerçek çağrı, çıkış, süre ve ham günlük SHA256'sını taşır.
Başarısız denemeler de korunur ve başarı sayılmaz:

- İlk check'te metnin `.into()` türü belirsizdi; açık String düzeltildi.
- `python-red` güncellenmemiş SDK'nın yeni alanı tanımadığını gösterir.
- İlk ortak durumda başvuru, çerçeve Daire'ye dönünce JSON'dan kaldırılan
  `shape` alanını eski nesneyle birleştirirken geri getiriyordu. Ürün
  beklenen kanonik JSON'u yazdı; bağımsız tam nesne kurucusu düzeltildi.
- İlk MCP beklentisi etiket içeriğiyle değişen host `stamp` belirtecini
  eski değerle karşılaştırdı. Karşılaştırma tam UUID/entity modelinde
  yapıldı; yeniden açılan cevapta kalan aynı test yardımcı hatası da
  giderildi. Ham iki başarısız takım ve başarılı odak testi saklanır.
- İlk Python tekrar/Undo beklentisi istek makbuzunun da kaybolacağını
  varsaydı. Mevcut [ADR0253 §6](../../adr/0253-command-protocol.md) protokolü makbuzu `undone` olarak saklar; gerçek
  bağımsız okumada yalnız `requests` değişiyordu. Test bütün modelin eski
  halini, makbuzun tek kayıt kalmasını ve yalnız durum değişmesini tam
  karşılaştırır; Redo yürütülmüş dosyanın tamamına bayt özdeş döner.
- GUI doğrulama betiğinin ilk denemesi Undo sonrası yeni slotun13
  olacağını varsaydı; gerçek masaüstü kayıt12'yi verdi. Beklenti mevcut
  masaüstü tahsisine düzeltildi; eski UUID, konum veya geometri gevşetilmedi.

31 yeni bağımsız durumda Koru yok/null/false, Yenile, yalnız açık alanı
değiştirme, iki elle konmuş etiket, karma yeni+yenile, başka etiket katmanı,
kaynak katman/evre, geçersiz şablon/boy, kilit ve revision, açık `tags`, boş
tekrar, doğrula/plan ve tek undo/redo yer alır. Eski107 create durumu ve
öteki dört etiket/çizelge fixture dosyası aynıdır. İlgili264 durum geçti.

`audit.py`268 komut kimliğinin aynı kaldığını, yalnız `cad.tags.create`
sözleşmesinin genişlediğini ve1756 önceki takipli KCAD dosyasının bayt
özdeşliğini doğrular. Üç **mevcut** üretilmiş TS tipi, katalog ve Python SDK
günceldir; yeni TS/TSX dosyası veya web arayüzü eklenmez. Web ADR0228 ile bekler.

## Koşulmuş takımlar

| Kapı | Sonuç |
|---|---|
| Ortak komutlar | 809 başarılı |
| Native uygulama | 51 birim +221 entegrasyon başarılı; 22 atlanan |
| Etkileşim | 63 birim +702 entegrasyon başarılı; bir entegrasyon ve beş performans testi atlanan |
| Python | 335 başarılı; yeni altı senaryo ayrıca odaklı geçti |
| Yeni MCP | Gerçek sunucuda plan/tek undo/redo/kayıt-aç başarılı |
| Derleme ve clippy | Altı pakette all-targets/-D warnings; son katman korumasından sonra interaction/desktop tekrar başarılı |
| Ortak kapılar | WASM, SDK, bağımsız referans ve 47 crate/61 hedef bağımlılık yönü başarılı |
| Dosya/uyum | 107 eski durum ve 1756 önceki KCAD baytı korunuyor; 268 komut kimliği aynı |
| Son GUI dosyaları | 24 tam beklenen model +bir tanısal disk kopyası, 35 ham ekran; SHA256/kaynak ikilisi de doğrulandı |

İlk iki tam MCP çağrısı yeni test yardımcı beklentisinde durdu. Önceki test
ikilileri o çağrılarda geçti; düzeltme sonrası yeni etiket testi ayrı, kalan
`wall_contours` ikilisi dört testle ayrı geçti. Bunlar tek yeşil tam MCP
çağrısı olarak sunulmaz. Ham sonuçlar ve gerçek komutlar `test-runs.json`'dadır.
Atlanan testler başarı sayılmaz. SQLite ResourceWarning ve ts-rs'nin mevcut
serde uyarıları günlükte tutulur. Yeni bir performans ölçümü yapılmadı.

## Gerçek native arayüz ve tam kayıtlar

`before.kcad`, ADR0661'in final dosyasının bayt özdeş kopyasıdır: altı
mahal/beş etiket, iki mimari kat ve gizli kaynak. Kullanıcı çizimleri ve
asıl uygulama profili kullanılmaz. QA uygulamaları değişmez ayrı ikililerdir;
manifestler executable ve kaynak hash'lerini, `gui-events.json` ham JPEG
hash/zaman/notunu, `models.json` gerçek Save dosyalarının hash'lerini tutar.
Ham görüntüler kırpılmadı, yeniden boyutlanmadı veya boyanmadı.

- 01 eski araçta ayar/önizleme bulunmadığını gösterir.
- 02 ilk yeni panelde uzun açıklamaların büyük yazıda ayarları aşağı
  ittiğini yakaladı. Kontroller üste, açıklamalar alta alınır; etiket
  katmanı tek satırdır, uzun varsayılan çerçeve adı kısaltılır. Son ikilide
  aynı durum iki tema/iki yazı/iki pencere boyunda tekrar doğrulandı.
- 03 gerçek Move ile yalnız etiket#6 `(2,1.5)→(-1,2.5)`. Görüntüde eski
  araç ipucu bir kısmını örter;04–10 elle konumu açık gösterir. Kaynak
  mahalın kendi yazısı da sahnede durur; bu eski çift gösterim bu dilimde
  değiştirilmez ve çözülmüş sayılmaz.
- 04–06 Koru+şablon+3mm, tek Enter→bir yeni etiket, eski beşi aynı, tek
  undo/redo. Alan Enter'ı ayarı işler; araç onayı sahne odaklandıktan sonra
  bir kez verilir. Bu ayrım eski panel davranışıdır.
- 07 salt okunur plan bir yeni/dört yenilenecek; manüel yer aynı. Save
  çizimi değiştirmez.08 gerçek sağ tuş menüsü,09 tek Onayla/bitir ve Save.
  Dört mevcut etikete şablon/yuvarlak köşe/3.5mm; gizli kaynak etiketi aynı.
- 10 tek undo tüm beş değişikliği birlikte döndürür; tekrar Redo aynı
  UUID'leri ve bütün modeli geri getirir.
- 11–13 son yerleşim ikilisinde açık18 geniş, boş tekrar ve yanlış şablon:
  ret araçtan çıkmaz; geçersiz alan son geçerli değere döner ve kayıt aynı.
- 14–15 açık18 dar; geçmiş paneli mevcut düğmesiyle kapatılıp sahne büyütülür.
  4 mm Yenile taslağı beş değişen etiketi önizler; tek Escape ve Save tam
  önceki modele döner. Docking veya geçmiş paneli kodu değiştirilmez.
- 16–18 açık13 geniş/dar ve gerçek Yenile/4 mm girişi; 0 yeni/beş yenilenecek,
  tek Escape ve Save. 19–22 koyu13 geniş/dar, sekiz tür ve alt kapsam açıklaması.
- 23–25 koyu18 geniş/dar; kısa çerçeve adı ve tek satır hedef taşmıyor,
  kaydırmayla boy alanına erişilip 4 mm taslağı giriliyor; iptal sonrası
  final Save bütün kaynak/etiket/ayarları koruyor.

26–30 hedef değişimi takip denemeleridir. Fareyle seçimde (26–28) imleç
sahneye döndüğünden hedef zaten Çizim olarak önizlendi ve tek onay doğru
yazdı; ilk olay notlarının yanlış yorumu `gui-events.json` düzeltme alanında
açıkça kaydedildi. Gerçek açık 29–30'da klavye ile tekrarlandı: şerit Çizim,
araç Etiket hedefini gösterirken Enter yeni katmana yazdı. Etkin katman
revision'ı ilerletmez. `target-change-red` otomasyon testi aynı kusurda
başarısız oldu; son koruma katman farkında önce planı yenileyip yazmadan
kalır. Normal, hedefi değişmemiş akışın tek onayı korunur.

`target-layout-undo.kcad` dosyası tanısaldır: arşivleme asenkron Save
bitmeden yapıldığı için önceki 12 nesneli kayıttır. Bu dosya başarılı Undo
kayıt kanıtı sayılmaz; tam eski disk durumu olarak bağımsız denetlenir.
Son tekrarın kayıtları Save sonrası arayüz sonucu beklenerek alınır.
31–35 aynı klavye akışının son ikilide tekrarını gösterir: ilk onayda
hiçbir etiket yazılmadı, Çizim hedefi ve açık uyarı gösterildi; sonraki
onay bir etiket yazıp aracı kapattı. Tek Undo/Redo ve yeni süreçte açılış
bütün model/UUID eşitliğiyle doğrulandı. Katman değişmeden normal tek
Enter/sağ tuş onayı 04–09'da ve güncel etkileşim takımında korunur.

Son panel için açık/koyu ×13/18 px ×geniş/dar sekiz birleşim gözle incelendi.
Ham pencereler 2880×1720 ve2200×1364'tür (2× ekran; 1440×860 ve1100×682
başlık dahil). Bu cihazda daha kısa dış pencere sistemin asgari yüksekliğine
oturdu; 1100×650 gerçek dış pencere ölçüldüğü iddia edilmez. Büyük yazı/dar
pencerede tüm alanların aynı anda sığması beklenmez; panelin kendi scroll'u
ile alt ayarlar erişilir. Son model doğrulaması **24 tam model +bir tanısal disk kopyası/35 ham ekran**
için geçti; görsel test FPS, titreme veya genel docking kabulü değildir.

`verify.py` bağımsız KCAD okuyucusuyla **bütün** modelin tam eşitliğini
kontrol eder: her kaynak, UUID, ayar, katman, etiket bağı/yeri/kolu/stili,
yeni kesin etiket ve gizli nesne. Eşdeğer durumlar ayrıca bayt özdeştir.
`seed.json` yalnız yeni rastgele UUID'leri verir; beklenen geometriler ve
etiket alanları üretim sonucundan kopyalanmaz.

Çoklu pafta/ölçek bağlamı, mimari kat süzgeci, genel etiket yerleşimi ve
bütün mimari iş akışlarının tema/boyut/yazı matrisi açıktır. Bu dilimin
başarısı tüm mimari araçların üretim kabulü anlamına gelmez.
