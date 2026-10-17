# Formlar: sahibin vereceği kararlar (FORM-DEC açıkları)

Hazırlandı: 17 Ekim 2026. `plan/form.md` §17'de açık kalan sekiz karar için seçenekler ve öneriler. Masaüstünde yapılanlar bu
kararları önceden bağlamamak için yalnız belgenin ve komutların sınırında durur (ADR 0401–0446). Her karar verildiğinde bir ADR'ye
dönüşür ve maddesi işaretlenir. Önerilerin hepsi geri alınabilir başlangıçlardır.

## FORM-DEC-04: Formun sahipliği, taşınması ve yerel kopya

Bugün: form projenin ayarıdır (`.kcad` şema 72 `forms`); bulutta veritabanı projesinin ayarlarıyla gider ve gelir; yayın taslağın
gövdesini ve özetini tutar (ADR 0401, 0444 §2). Ayrı bir “dağıtım” nesnesi yok.

| Seçenek | Ne demek | Bedeli |
|---|---|---|
| A. Form projede kalır, dağıtım projenin meta verisi | Bugünkü model; dağıtım (hedef kitle, başlangıç, geri alma) projeye eklenen bir kayıt | En az göç; projeler arası ortak form yok |
| B. Form kurumun kitaplığında, proje sürüme bağlanır | Yayın kurumun nesnesi, proje `form@sürüm`'ü anar | Birden çok projede aynı form; göç ve bağ çözümü gerekir |
| C. İkisi birden | Projede yerel form, kurumda yayınlanan form; içe alınca kimlik yeniden eşlenir | En esnek, en karmaşık |

**Öneri:** A ile başla, B'ye açık bırak: formun kimliği ve yayının özeti zaten projeden bağımsız taşınabilir (paket `cad.forms.export`,
ADR 0435). Yerel kopya `.kcad` v2'nin kendisi; ayrı biçim icat edilmez.

## FORM-DEC-07: Yetkilerin kesişimi ve ProjectAccess göçü

Bugün: `forms.design`, `forms.publish`, `forms.use` izinleri ve Veri girişi rolü (ADR 0411); sunucunun yazma bekçisi `form_guard.rs`.
Satır ve alan düzeyi yok.

| Seçenek | Ne demek |
|---|---|
| A. Proje düzeyi kalır, alan düzeyi yalnız formun gizlemesi | Basit; alan gizliliği güvenlik sınırı değildir |
| B. Etkin yetki = proje rolü ∩ formun dağıtımı ∩ katmanın satır kuralı (RLS) | Gerçek sınır; her yazma yolu (HTTP, Python, MCP, içe aktarma) aynı kesişimden |

**Öneri:** B, güvenli varsayılanla: kesişimde bir parça eksikse **reddet**. Göç: bugünkü roller aynı izinleri verir; satır kuralı
olmayan katman “bütün satırlar”dır. Önce yazma yolu, sonra okuma (sorgu, sayım, otomatik tamamlama, AI bağlamı).

## FORM-DEC-08: Kişisel form ile kurum yayınının kimliği

Bugün: cihazdaki bölüm kitaplığı ve kalan taslak kişiseldir, projenin formlarından ayrı listelenir (ADR 0441 §1, §5).

**Öneri:** kişisel form kurum yayınına yalnız **içe alınarak** girer: içe alma yeni kimlik verir, kaynağı (`imported from`) kaydeder,
alanları yeniden eşler (ADR 0435'in kuralı), bağlantı ve parola taşımaz, yetki vermez. Dosyadaki hiçbir iddia yeni yetki yaratmaz
(FORM-PUB-15 ile aynı ilke).

## FORM-DEC-11: Birden çok kaynağa dokunan düzenleme

**Öneri:** otomatik dağıtık atomiklik **yok**. Her kaynak kendi yeteneğini bildirir: `read-only`, `transactional` (aynı veritabanı)
ya da `workflow` (adım adım, her adım ayrı onay ve geri alma kaydıyla). Formun birleştirilmiş görünümü yalnız kanonik kaynağın
kaydına yazar; öbürleri salt okunur gösterilir. Bugün masaüstünde zaten böyle: ilişkili kayıtlar kendi katmanlarına ayrı yazılır, tek
gönderim aynı belgede tek adımdır (ADR 0440).

## FORM-DEC-12: Yayın onayı ile veri onayının rolleri

| Seçenek | Ne demek |
|---|---|
| A. Tasarlayan yayınlar (`forms.publish`) | Küçük ekipler için yeterli |
| B. İsteğe bağlı gözden geçiren: kurum açarsa yayın ikinci kişinin onayını bekler | Dört göz ilkesi |

**Öneri:** varsayılan A, kurum ayarıyla B. Veri onayı (iş akışı durumları, ADR 0428) ayrı izindir, yayın izni vermez; eklenti
sürümünün onayı ayrı kalır.

## FORM-DEC-13: Mobil yığın

Bugün: kod modüler; hesap `crates/shared`'te, uygulama katmanı `crates/native`'te Iced ve pencere bilmez (ADR 0228).

| Seçenek | Ne demek | Bedeli |
|---|---|---|
| A. Rust çekirdek + yerel kabuk (Swift/Kotlin), çekirdek FFI ile | Kural motoru, form oturumu ve KCAD aynı kod | İki yerel arayüz |
| B. Rust çekirdek WASM + web arayüzü (PWA ya da sarmalayıcı) | Web'in TS arayüzü yeniden kullanılır | Web duruyor (ADR 0228); çevrimdışı depo tarayıcıya bağlı |
| C. Iced'in mobil hedefi | Masaüstü arayüzü | Olgunluk riski |

**Öneri:** önce görüntüleme için B ya da A'yı küçük bir denemeyle karşılaştır; karar ölçümle. Hangisi seçilirse seçilsin kural motoru
`kentos-forms` olur, ortak durumlar (`fixtures/forms/v1`) aynen geçer.

## FORM-DEC-14: Çevrimdışı politika

**Öneri:** cihaz bir “görev paketi” alır: izinli veri alt kümesi, formun yayını, son geçerlilik zamanı, imzalı makbuz. Süre dolunca yeni
kayıt taslak kalır, gönderilmez; yetki geri alınınca eşitleme reddeder, taslak cihazda kalır ama görev kapanır. Hesap değişince
önceki hesabın taslakları ve önbelleği ayrı klasörde kilitlenir. Çakışmada sunucunun kaydı kazanmaz, iki sürüm de kullanıcıya
gösterilir (masaüstünün bugünkü çakışma sorusu gibi, ADR 0043). İndirilmiş verinin geri silinebileceği vaat edilmez.

## FORM-DEC-15: Python ve İşlemler eylemleri

Bugün: eylem kayıtlı işlem aracının kimliğiyle bağlanır, olmayan araç ön denetimde `form_action_unknown` (ADR 0441 §6, 0442).

**Öneri:** eylemin tanımı aracın kimliği, sürümü ve gereken çalışma ortamını (`requires`, ADR 0441) taşır; yürütücü yerelse masaüstü,
değilse sunucu. Mobil ve web yalnız sunucunun izin verdiği yürütücüyü çağırır. Çevrimdışıyken eylem düğmesi kapalıdır ve nedenini
söyler; kayıt eylemsiz yazılabilir.

## Kararın biçimi

Her madde için bir harf ya da “öneri” yeterli; ben ADR'sini yazar, maddeyi işaretler, gereken kodu sırasına koyarım.
