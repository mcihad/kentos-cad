# Yerel GUI kanıtları ve tarihsel kabul kayıtları

27 Ekim 2026 kullanıcı kararı: test ekran görüntüleri Git'e eklenmez. Yeni
gerçek pencere ve otomatik render görüntüleri `.run/gui-evidence/<çalışma>/`
altına yazılır. Bu dizin yok sayılır. Çizim/model fixture'ları, test kaynakları,
küçük metin raporları ve SHA256 manifestleri depoda kalabilir. Ürün ikonları,
fontlar ve `fixtures/` altındaki gerçek test girdileri bu kuralın dışındadır.

Bu değişiklikten önce `docs/baseline/` altında takip edilen 1.139 görüntü,
çalışma ağacındaki aynı yerel yollarında korunup yalnız Git takibinden
çıkarıldı. Yeni bir klonda bu görüntüler bulunmaz. Eski Markdown bağlantıları
ve görüntü doğrulayıcıları, yerel dosyalar varken önceki yollarıyla çalışır.
Yerel dosya yoksa bağlantının görüntü göstermesi veya eski bir kabulün görsel
bölümünün yeniden doğrulanmış sayılması beklenmez.
Git'in resmî [gitignore notları](https://git-scm.com/docs/gitignore#_notes),
zaten takip edilen dosyanın yok sayılabilmesi için önce `git rm --cached`
ile yalnız indeksten çıkarılmasını açıklar; bu temizlik aynı yöntemi kullanır.

## Ortak tarihsel görüntü manifesti

[gui-evidence-manifest.json](gui-evidence-manifest.json) tek metin envanteridir:
eski depo yolu, SHA256, bayt sayısı, Git blob kimliği ve görüntüleri içeren
kaynak `main` commit'i. Görüntü baytları bu manifestin içine gömülmez.

Depo kökünden:

```sh
# Yalnız metin yapısı ve yerel bulunabilirlik; hiçbir görüntü baytı doğrulanmaz.
python3 scripts/qa/gui_evidence.py metadata

# Tüm yerel görüntüleri SHA256/bayt/Git blob karmasıyla gerçekten doğrula.
# Eksik veya değişmiş tek bir dosya bile başarısız sonuç verir.
python3 scripts/qa/gui_evidence.py verify

# Açıkça istenen bir eski görüntüyü mevcut Git geçmişinden geri getir.
python3 scripts/qa/gui_evidence.py restore docs/baseline/2026-10-26-architecture-opening-dimensions/08-dimension-linked-inspector.jpg

# Bütün eski görüntüler gerektiğinde; yaklaşık517MiB yerel dosya oluşturabilir.
python3 scripts/qa/gui_evidence.py restore

# Git indeksinde GUI kanıt görüntüsü bulunmadığını denetle.
python3 scripts/qa/gui_evidence.py check-tracked

# Gerçek Git nesneleri ve geçici dosyalarla küçük araç testleri.
python3 -m unittest discover -s scripts/qa -p 'test_gui_evidence.py'
```

`restore`, manifestteki commit/yol/blob ilişkisini ve blob'un SHA256/bayt
sayısını doğrular. Eksik dosyayı yayımlar; aynı içerikte mevcut dosyayı korur.
Farklı mevcut dosyayı **asla ezmez**, çatışmayı başarısız sonuçla bildirir.
Gerekli geçmiş/blok bulunmuyorsa bunu hata olarak bildirir. Sığ veya nesneleri
filtrelenmiş bir klon, gereken kaynak commit/blob'u ayrıca edinmelidir.

`metadata` sonucu açıkça `metadata-only` ve `verifiedImages: 0` yazar; eksik
dosyaların sayısını bildirir. Bu sonuç, görüntü karşılaştırmasının veya GUI
kabulünün geçtiği anlamına gelmez. `verify` eksik görüntüyü sessizce atlamaz.

## Eski kabul betiklerinin sınırı

Alt dizinlerdeki `verify.py` dosyaları tarihsel kabul anının model, kaynak,
ikili ve görüntü karmalarını denetler. Yalnız görüntüleri geri getirmek, başka
bir kaynak sürümünde bu eski kabulü güncel kabul yapmaz. Bu betikler ilgili
kabul sürümünde ve gereken yerel görüntüler geri getirildikten sonra
çalıştırılmalıdır. Kaynak hash kontrolleri veya beklenen geometriler bu
temizlik için gevşetilmemiştir. Model fixture'ları korunmuştur.

Git geçmişi yeniden yazılmamıştır: eski commit'lerin görüntü blob'ları
geçmişte kalır ve açık `restore` işleminin kaynağıdır. Bu karar yeni commit'lere
görüntü eklenmesini ve normal çalışma ağacına görüntü indirilmesini önler;
mevcut Git geçmişinin depolama boyutunu küçülttüğü iddiası değildir.
