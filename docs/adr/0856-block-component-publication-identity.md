# ADR 0856 — Blok ailesi, tipi ve yayınını yerel sayaçtan ayır

Tarih: 2026-11-02. CAD-ADV-06'nın kimlik dilimi; seçilmiş yükseltme ve
kalıcı tip–örnek parametre politikası ayrıca açıktır.

## Dayanak ve mevcut durum

[Revit 2026: Element Properties](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-GetStarted/files/GUID-1B5B5C2E-072F-4349-9C70-D88204F9145D.htm)
aile tipi ile o tipin yerleştirilmiş örneklerini ayrı ele alır. Bu kavram
2B bileşen yönetimine dayanak olur; KentOS'a Revit dosya veya BIM davranışı
uygulandığı anlamına gelmez. Mevcut `BlockCard.version` geometri/tanım
çizimi değiştikçe artan yerel sayaçtır. Kitaplık yayınının numarası olarak
kullanmak kaynak yayını ile yerel düzenlemeyi karıştırır. Kodda kalıcı aile,
tip ve yayın kimliği yoktu; yalnız blok kimliği/adı ve yerel sayaç vardı.

## Karar

- `BlockCard.publication` isteğe bağlı `BlockPublication` taşır:
  `familyId` ve `typeId` kalıcı sıfır olmayan UUID; `typeName` 1–128 karakterlik tip adı;
  `release` yayıncının verdiği pozitif uint32 numarasıdır. Tip adında kenar
  boşluğu ve kontrol karakteri yoktur. Aynı aile farklı tip/yayınlarda aynı
  aile kimliğini korur; tip adı değişse de `typeId` korunur. Çizimin blok kimliği ve adı bundan bağımsızdır.
- `cad.blocks.edit` işlemi `card` için `publication` verilirse kimliği
  günceller; verilmezse korur. Açık `clearPublication=true` kimliği kaldırır.
  İkisi aynı anda verilirse `invalid_block_card`, yol `publication` ile
  atomik reddedilir. Bozuk kimlik ortak kart kurallarıyla reddedilir.
- Tanım/örnek geometrisi bu kart işlemiyle değiştirilmez. Yerel `version`
  yayın numarası verilince artmaz; sonraki normal geometri değişimi yerel
  sayacı artırır ve yayın kimliğini yeniden yazmaz. Bu metaveri içeriğin
  değiştirilmemiş olduğuna dair kriptografik doğrulama iddiası taşımaz.
- Blok kartı penceresinde isteğe bağlı bileşen alanları vardır. Yeni aile
  ve tip kimliği otomatik hazırlanır; kimlik alanları açılabilir ayrıntı
  bölümündedir. Kopyalama/yapıştırma ile aynı aileye bağlanır.
  İptal hiçbir şey yazmaz. Geçersiz UUID/yayın numarası kayıttan önce anlaşılır
  mesaj verir. Kaydetmek çevrimiçi yayın değildir.
- Çizim listesi/kart ipucu ve dosya kitaplığı tip/yayını gösterebilir;
  arama tip adı ve aile kimliğini de bulur. Uzun metin yerleştirme sayısını
  veya yan düğmeleri örtmemelidir. Yerel sayaç yayın numarası diye gösterilmez.
- KCAD şema 158, metaveri varsa seçilir; eski çizimlerin baytları değişmez.
  Rust okuyucu/yazıcı ve bağımsız Python okuyucu/yazıcı aynı kuralları uygular.
  Yeni özellik eski şemada sessizce düşürülmez. Kimlikli blok kitaplığı
  sürüm 2 yazar; kimliksiz kitaplık sürüm 1 kalır. Her iki kitaplık sürümü
  okunur; sürüm 1 dosyada yayın kimliği reddedilir. Böylece eski uygulama
  sürüm 2 dosyadan yeni kimliği sessizce atamaz. Python tipleri, katalog ve MCP
  aynı sözleşmeyi sunar; yeni bir ayrı otomasyon algoritması yoktur.

Dosya açılışında güncelleme yoktur. Aile/yayın karşılaştırması, seçilmiş
kapsamda yükseltme, yerel değişiklik/çatışma politikası ve yayın içeriğinin
bütünlük takibi bu kimlik dilimiyle tamamlanmış sayılmaz. Salt adı eşleştiren
eski kitaplık import'u henüz sürüm yöneten yükseltme değildir. Tip adı da bir
parametreyi otomatik olarak tip kapsamına taşımaz.

## Doğrulama

- 1163 contracts (1 ignored), 234 KCAD, 319 application (37 ignored),
  58 MCP, 1605 masaüstü (256 ignored) ve 448 Python testi geçti. Katalog
  1057 komutu üretti; Clippy ve masaüstü derlemesi geçti. Bağımsız fixture
  üretim/karşılaştırmasında 1489 dosya; blok komutlarında 27+49 vaka geçti.
- Aile/tip UUID, yayın sıfırı, boş/uzun/kontrol karakterli tip adı ve
  yayın/kaldırma çakışması plan/uygula yollarında kısmi yazmadan reddedildi.
  Kimliksiz sürüm 1 kitaplık ve kimlikli sürüm 2 kitaplık sınırı denetlendi.
  Yerel geometri düzenlemesinin sayacı artırıp yayın kimliğini koruması,
  kayıt/açma, kimlik kaldırma ve tek Undo test edildi.
- Gerçek uygulamada kart fareyle açıldı; tip adı ve yayın 2→3 klavyeyle
  değiştirildi. Kaydedilen KCAD bağımsız Python işleminin dosyasıyla tam
  aynı (SHA-256 `e21a82208954b4550e41e2dd3bcbaba5d13841373351d47e110ef93d4adb7f68`).
  Gerçek tek Undo kaydı başlangıçla; Redo ve değiştirilmiş taslaktan
  Vazgeç kaydı yeni dosyayla bayt düzeyinde aynı. Yeniden başlatılıp aynı
  dosyada yayın 3 / yerel sayaç 7 ayrı korundu. Sıfır numara gerçek Kaydet
  ile reddedildi; son mesaj ekranında alanlar ve düğmeler içeride kaldı.
- Ayrıntılar açılınca alt düğmelerin kaybolduğu gerçek GUI'de bulundu;
  yalnız gövde kaydırılarak alt düğmeler görünür tutuldu. 2200×1364 fiziksel
  piksel / minimum pencere ve büyük yazı ayarında kimlikler, kaydırma,
  Vazgeç ve Kaydet kontrol edildi. Son düzeltmeden sonra masaüstü testleri,
  Clippy ve derleme; son mesaj düzeltmesinden sonra ilgili test, Clippy ve
  derleme tekrar geçti. Bu kayıt koyu tema veya web GUI kabulü iddiası taşımaz.
- Dosya kitaplığına gerçek klasör seçicisiyle kaynak eklendi. Blok adında
  bulunmayan “tek kanat” tip metni ve aile UUID öneki gerçek klavyeyle
  arandı; yalnız ilgili blok kaldı, tip adı/yayın 2 listede gösterildi.

Kanıtlar `/tmp/kentos-cad-adv04-2026-11-01/` altında: 214–223 numaralı
PNG'ler, `block-publication-gui-acceptance.json`, gerçek GUI ve Python
KCAD kayıtları, `block-publication-results.json` ve test günlükleri. Ekranlar
depo dışında tutulur. Seçilmiş yayın yükseltmesi ve kalıcı tip–örnek
parametre kapsamı bu kabulün dışında, CAD-ADV-06 açık kalır.
