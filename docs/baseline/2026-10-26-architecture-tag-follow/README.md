# Etiket tutamacı ve kaynak izleme — ADR 0655

26 Ekim 2026. **Etiket konumu ve kaynak izleme dilimi doğrulandı.**
ARCH-SPACE-02 bütünü ve tam mimari ekran matrisi açık kalır.

## Kusur ve dar düzeltme

`gui-binary.json`, ADR0654'ün değişmeden tekrar kullanılan test paketidir;
derleme kaydı yeniymiş gibi sunulmaz. 01'de mahalın dışarıdaki etiketi fareyle
seçildi; 02'de ortasından yukarı sürüklendi ve bırakıldı. Konum değişmedi,
“Yapılacak değişiklik verilmedi” uyarısı çıktı. `failed-drag.kcad`, bağımsız
KCAD okuyucusunda başlangıcın tamamıyla eşittir; başarılı taşıma değildir.

Ortak geometri tutamacı Tag.at konumunu taşıyor, fakat genel EntityGeometry
bilerek Tag temsil etmiyor. Select'in geometri yazım döngüsü etiketi atlayıp
boş değişiklik komutu yolluyordu. Düzeltme yalnız native seçimin etiket konum
rotasıdır: mevcut `cad.entities.transform` Move komutuna yalnız sıcak etiket
UUID'leri gider. Seçilmiş ama sıcak olmayan nesneler değişmez. Kopya aynı
komutu kullanır; karışık sıcak türler ve sonradan kilitlenen etiket katmanı
kısmi yazmadan reddedilir. Önizleme mevcut ortak geometri tutamacıdır.

Yeni şema, ürün komutu, SDK, TS/TSX veya renderer/birim/ölçek değişikliği
yoktur. Python ve MCP mevcut anlamsal taşıma yüzeyini kullanır. Genel diğer
mimari tutamaçlar ve tam klavye matrisi bu dilimin kabulü değildir.

## Otomatik kontroller

Komut, gerçek çıkış kodu, zaman ve günlük SHA256 `test-runs.json` içindedir.

- `interaction-final`: **692 başarılı, bir atlanan**. Altı yeni regresyonda
  20 açık hedef/büyük dünya koordinatı, gerçek pointer_down/move/up,
  tam model/UUID ve tek undo/redo; sıcak alt küme/çoklu etiket, kopya,
  karışık tür, katmanı sonradan kilitleme, önizleme ve Escape sınanır.
- `application-final`: **219 başarılı, 22 atlanan** entegrasyon testi.
- `python-final`: **303 başarılı**; FFI/SDK kontrolü ve extension clippy dahil.
  Üç yeni test 20 yer değiştirme plan/yazma/kayıt-aç, 20 duvar kalınlığında
  `(6-w)*(4-w)` Decimal net alanı, etiket/çizelge ve kaynak silme/undo'yu sınar.
- `mcp-final`: mevcut gerçek Server mahal numaralama/etiket/kalıcılık testi
  **bir başarılı**. Yeni etiket tutamacına özgü MCP GUI testi değildir.
- `tag-reference`: **165 bağımsız ortak durum**; `transform-reference`:
  **64 durum** aynı. Sistem Python'unda mpmath bulunmadığından dönüşüm
  başvurusundaki ince plakanın 50 basamaklı karşılaştırması **atlandı**;
  bütün yüksek hassasiyet kapsamı tamamlandı denmez.
- `clippy-final`: interaction/desktop bütün hedeflerde `-D warnings` geçti;
  `build` desktop derlemesi geçti. Python ham günlüğündeki SQLite
  ResourceWarning ile mevcut ts-rs/future-compat bildirimleri saklandı;
  bütün uyarıların sıfırlandığı iddiası yoktur.

`red-grip.log` eski kaynakta gerçek oturum yerleştirmesini başarısız gösterir;
önizleme/Escape geçer, konumu yazma geçmez. `green-grips.log` yeni testteki
Entity/&Entity kıyas yazım hatası nedeniyle derlenmedi; kabul değildir.
İlk `python.log` yeni test düzeneğinin generator'ı liste gibi kullanmasıyla
hata verdi. `python-focused.log` centroid FP64 ulp'si için tam sayı eşitliği,
aynı duvar kalınlığında gereksiz undo ve kayıt sonrası sayısal slot eşitliği
varsayımlarını açığa çıkardı. Yalnız test düzeneği düzeltildi; ardından
`python-focused2` ve tam `python-final` geçti. Başarısız ham günlükler korunur,
son başarılı sayılara eklenmez; ürün kusuru gibi sunulmaz.

## Son değişmez pakette gerçek Mac etkileşimi

`after-binary.json` son kaynakların ve çalıştırılabilirin SHA256 kaydıdır.
03–22 bu paketle çekildi. Son derleme/paketlemeden sonra ürün/test kaynakları
değişmedi. Ham JPEG'ler düzenlenmedi, kırpılmadı, yeniden boyutlanmadı.
`gui-events.json`, `models.json` ve `screenshot-dimensions.json` kayıtları ayırır.

- **03–06:** dışarıdaki etiketi gerçek fareyle sürükle, bırak, Cmd-S;
  bir Cmd-Z ve bir redo. Yalnız Tag.at değişir; mahal/sınır/ankraj/kaynak/
  tablo/belge ayarları değişmez. Görünür 6×4 m duvardan kalibre edilen hedef
  yaklaşık (−1,5) m; kayıtta (−1.0083115915697682,4.989020167151163) m.
- **07–09:** duvar kalınlığına `40 cm`, tek alan Enter'ı ve Kaydet.
  Net mahal/etiket/çizelge 22.04→**20.16 m²**, çevre 19.200→**18.400 m**;
  elle taşınmış etiket yerinde kalır. Bir undo ve redo bütün zinciri geri getirir.
- **10–11:** görünür Sil ile kaynak mahal silinir; etiket son yazı/konumunu
  ve `targetGone` uyarısını korur, çizelge satırı kalkar. Bir undo bağı düzeltir.
- **12–13:** kaynak duvar silinir; mahal son geçerli sınırını `lostEnclosure`
  uyarısıyla korur, etiketin mahal bağı sürer. Bir undo uyarıyı kaldırır.
  13'te fare konumundan doğan araç ipucu vardır; tüm panelin örtüsüz olduğu
  iddia edilmez. Daha önce kalan etiketlerin mahal yazısına yakın konumları
  test dosyasının başlangıç düzenidir, yeni otomatik yerleşim kabulü değildir.
- **14–16:** mevcut Şablon alanı tek Enter ile `{number}`, `{name}`,
  `Net {area}` satırlarına çevrilir. Aynı UUID/yer/kaynak ve türetilmiş
  `U-099 / Salon / Net 20.16 m²`; bir undo/redo şablonu geri getirir.
- **17–21:** izole profil ayarına açıkça **18 px** yazılıp açık tema kaydedilir;
  gerçek dar pencerede sıcak etiket tutamacı + teker zoom önizlemesi.
  Önizleme aktifken Cmd-S yalnız işlenmiş modeli yazar; sağ tuş Tutamaç
  menüsünü açar, Bırak iptal eder. Ayrı tekrar tek Escape ile iptal olur.
  Her üç dosya işlenmiş şablonlu modelle tamamen aynıdır.
- **22:** uygulama kapatılır, aynı paket yeni süreçte KCAD'yi açar;
  taşınmış etiket/40 cm duvar/mahal/çizelge görünür. Yeni süreçte Cmd-S ile
  yeniden yazılmış dosya önceki modele tamamen eşittir.

**22 ham JPEG ve 18 gerçek disk kopyası** vardır; ilk iki görüntü ve başarısız
sürükleme dosyası tanısaldır. Gerçek ölçülen pencere boyları koyu/13 px için
1512×872 ve 1440×870 pt, açık/18 px için 1440×870 ve 1100×682 pt'dir.
`dark13-profile.json`/`light18-profile.json` gerçek kayıtlı ayarlardır.
1440×900/1100×650 × iki tema × 13/18 px × bütün iş akışlarının tam matrisi
hâlâ açıktır; tek birleşik senaryo bütün mimari kabulü sayılmaz.

Önceki ADR0654 açık tema görüntülerinin 18 px etiketi yanlıştı: kaynak
profil 15 px idi. O kabulün notları 15 px olarak düzeltildi, profil saklandı,
ham JPEG'ler değişmedi. Bu kayıttaki 18 px ayrıca girilip doğrulanmıştır.

## Bağımsız dosya karşılaştırması

`verify.py`, yalnız bağımsız KCAD okuyucusunu ve Decimal hesabı kullanır;
KentOS ürün/komut/geometri kodunu çağırmaz. 18 dosyanın tamamında açık beklenen
modelle karşılaştırma, UUID/sıra/bağlar, kaynak kaybı uyarısı ve tablo yapılır.
Sürükleme yalnız etiket yerini değiştirebilir. Yeniden hesaplanan mahal konturunda
**1e-9 m**, etiket yer/ankrajında **1e-10 m** sınırı vardır; diğer alanlar tam
eşittir. Silinip yeniden açılmış iki modelde yalnız sayısal depolama `id`'si
normalleştirilir; kalıcı UUID ve kaynak kimliği asla atılmaz. Undo/redo,
önizleme/iptal ve yeni süreçte açılış kendi karşılıklarına **tam eşit** olmalıdır.
Ham görüntü/çalıştırılabilir/kaynak/günlük SHA'ları da denetlenir.
`independent-final.log` gerçek başarılı sonucu içerir.

Eski etiketi açık Koru/Yenile aracıyla yönetme, daha zengin döşeme/tavan
şablon alanları, kat/pafta oranları ve bütün mimari matris ayrı açık işlerdir.
