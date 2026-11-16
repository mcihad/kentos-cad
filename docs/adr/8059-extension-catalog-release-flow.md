# ADR 8059: Eklenti kataloğunda sürüm akışı — onaylı yayının durum makinesi

- **Durum:** uygulandı, üç dilim (15 Kasım 2026). İlki saf Rust mantığı: `crates/shared/extensions/src/release.rs`
  ve `release/` (§1–§8). İkincisi sunucu uçları, bellekteki depoyla (`apps/api`, §10). Üçüncüsü geri çekme listesi:
  paylaşılan biçim, akıştan üretim ve uç (§12). Kalıcı depo bir sonraki dilim (§11); imza, tekerlek deposu ve arayüz
  daha sonra (§9).
- **Plan:** `plan/eklenti.md` ADIM 22A (PY-CLOUD-01…18; dersler D11, D12, D13).
- **Bağlam belgeleri:** ADR 8057 §1 (paket, içerik özeti, saf crate) ve §2 (kurum kataloğu, geri çekme listesi);
  `docs/research/2026-11-13-eklenti-ekosistemleri.md` (Open VSX'in yük altında taranmamış paketi "PASSED" geçirmesi,
  Obsidian'ın yalnız ilk sürümü inceleyen kuyruğunun 2.300 başvuruyu aşması).

## Bağlam

Bulut kataloğunda bir sürümü sunucu, katalog penceresi ve programlı çağrılar (Python, MCP, ajan) hareket ettirecek.
Hepsi aynı kurala bağlanmazsa programlı bir çağrı incelemeyi atlar (PY-CLOUD-18). Kural bir sunucu ucuna değil, paket
denetleyicisinin yanındaki saf crate'e (`kentos-extensions`; dosya sistemi, ağ ve saat yok) yazıldı: sunucu, MCP ve
Python aynı tipleri ve aynı hareketleri kullanır, kendi durum mantığını yazmaz.

## Karar

### §1 Durumlar ve hareketler

Her hareket `Release`'in bir yöntemidir; tablo kodda veridir (`Action::sources`) ve `the_table_of_moves` testi onu
birebir doğrular. Yükleme (`Release::upload`) yeni kaydı Taslak olarak açar.

| Durum | Kod | Bu durumdan yapılabilecekler (→ hedef) |
|---|---|---|
| Taslak | `draft` | İçeriği yenile · İncelemeye gönder → Gönderildi · Denetim sonucu · Arşivle → Arşivlendi |
| Gönderildi | `submitted` | Denetim sonucu (engellerse → Değişiklik isteniyor) · İncelemeye al → Denetleniyor · Arşivle |
| Denetleniyor | `inReview` | Denetim sonucu (engellerse → Değişiklik isteniyor) · Değişiklik iste · Engeli gerekçeyle kaldır · Onayla → Onaylandı · Arşivle |
| Değişiklik isteniyor | `changesRequested` | İçeriği yenile · İncelemeye gönder → Gönderildi · Denetim sonucu · Arşivle |
| Onaylandı | `approved` | Değişiklik iste (onay düşer) · Yayımla → Yayımlandı · Arşivle |
| Yayımlandı | `published` | Askıya al → Askıda · Geri çek → Geri çekildi · Arşivle → Arşivlendi |
| Askıda | `suspended` | Askıdan al → Yayımlandı · Geri çek → Geri çekildi |
| Geri çekildi | `withdrawn` | — (son durum) |
| Arşivlendi | `archived` | — (son durum) |

| Durum | Listede | Yeni kurulum | Çalışır (bulut işi, eşitleyen KentOS) | Görenler |
|---|---|---|---|---|
| Taslak … Onaylandı | hayır | hayır | hayır | yazar, yayımcı; kataloğun inceleyici, yönetici ve denetim hizmeti |
| Yayımlandı | evet | yalnız onaylanan içerik | evet | herkes |
| Askıda, Geri çekildi | hayır | hayır | hayır | herkes, nedeniyle |
| Arşivlendi, yayımlandıktan sonra | hayır | hayır | evet | herkes |
| Arşivlendi, yayımlanmadan | hayır | hayır | hayır | yazar, yayımcı, katalog |

Kişisel katalogdaki sürümü her durumda yalnız sahibi ve kataloğun denetim hizmeti görür (kapıları orada da taranır;
hizmet karar vermez). Kataloğun (kiracının) üyeliğini sunucu önce denetler;
sayılar ve aramalar bu süzgeçten sonra hesaplanır (PY-CLOUD-17).

### §2 Sürüm, içeriğidir (PY-CLOUD-04, 08, 10)

`ReleaseContent` = paketin içerik özeti (`integrity.json`'ın kanonik biçimi) + `uv.lock`'un SHA-256'sı + SBOM özeti
(katalog üretince) + istenen yetenekler kümesi. Kimlik paketin baytlarından hesaplanır (`ReleaseFacts::of` →
`content()`), yükleyenin beyanından değil.

- İçerik yalnız Taslak ve Değişiklik isteniyor durumunda yenilenir ve yenilenince bütün kapılar baştan başlar;
  gönderildikten sonra `content_frozen`.
- Onay içeriğe bağlıdır (`Approval.content`). Yayımlarken ve askıdan alırken onay–içerik eşitliği yeniden denetlenir;
  kurulum yalnız onaylanan içeriğe izin verir (`allows_install` → `content_mismatch`).
- Bir sürüm numarası bir katalogda bir kez yüklenir; yapı üst verisi (`1.1.0+x`) ayrı sürüm sayılmaz
  (`version_taken`, kabul P14).

### §3 Kim ne yapar (PY-CLOUD-07, 09, 18)

Yetkiler veridir; roller onların adlandırılmış kümeleridir ve sunucu kendi kümelerini de verebilir.

| Rol | Yetkiler |
|---|---|
| Yayımcı | Yükleme, İncelemeye gönderme, Yayımlama (kendi sürümünü geri çekme ve arşivleme dahil) |
| İnceleyici | İnceleme (incelemeye alma, değişiklik isteme, engeli gerekçeyle kaldırma), Onaylama |
| Katalog yöneticisi | Katalog yönetimi (askıya alma, askıdan alma, her sürümü geri çekme ve arşivleme) |
| Denetim hizmeti | Denetim sonucu bildirme |

1. **Onay ile yayın ayrı yetkidir** ve hiçbir varsayılan rolde birlikte bulunmaz.
2. **Dört göz:** kurum ve genel katalogda yazar kendi sürümünü incelemeye alamaz, onaylayamaz, engelini kaldıramaz,
   askıdan alamaz (`self_review`); genel katalogda yayımcının hiçbir üyesi alamaz. Kişisel katalogda sahibi kendi
   sürümünü inceler (onu başka kimse kurmaz); kapılar yine geçmek zorunda.
3. **Karar kişinindir:** onay, değişiklik isteme, engel kaldırma ve askıdan alma yalnız katalog penceresinden
   (`Channel::Window`). Program (`Channel::Program`), hangi hesabın jetonunu taşırsa taşısın, `decision_by_program` alır.
   Program yükler, gönderir ve onaylanmış sürümü yayımlar; onaysız yayın denemesi `not_approved`.
4. Yazar ve yayımcı kendi sürümünün denetim sonucunu bildiremez (`self_scan`).
5. `Actor` sunucunun kimlik doğrulamasından kurulur; istek gövdesinden okunamasın diye `Deserialize`'ı yoktur.

### §4 Kapılar açık kalmaz (PY-CLOUD-05, 06; D11)

Yedi kapı, her katalogda ve her sürümde: paket (`kentos kek verify`'in aynısı), bağımlılıklar, lisanslar, uyumluluk,
sırlar (AI araç yapılandırmaları dahil), test ve duman testi (yalıtılmış işçide), yayımcı (ad alanı, taklit).

- Durumlar üç: doğrulanıyor / engellendi / doğrulandı. Tarayıcının cevabı `ScanOutcome`: `started`, `passed`,
  `blocked`, `errored`. Yalnız sonuna kadar çalışmış temiz `passed` doğrular; taranmadı, taranıyor ve hata verdi
  "doğrulanıyor"dur ve onay `gates_open` ile durur. "Tarayıcı yok" ile "kuyruğa alınamadı" ayrı cevap değil, ikisi de
  `errored`: hiçbiri geçti okunmaz.
- Bulgusuz engel (`scan_unexplained`) ve başka bir içerik için gelen geç sonuç (`scan_stale`) reddedilir: eski
  içeriğin geç cevabı yeni içeriği doğrulamaz.
- Gönderilmiş ya da incelenen sürümü engelleyen kapı onu bulgularıyla yazara geri gönderir; inceleyici kuyruğu
  engelli sürümle dolmaz (D12).
- Bağımlılık, lisans, sır ve test bulgusunu inceleyici gerekçeyle kaldırabilir (`waive`). Paket, uyumluluk ve yayımcı
  kuralları kesindir. Hata veren ya da taranmamış kapı kaldırılamaz, yeniden taranır. Kaldırma, aynı bulgu yeniden
  bulunsa da sürer; başka bulguda düşer.
- Onaydan sonra kapılar kapanır (`gates_closed`).

### §5 Ret ve geri dönüş yolu (PY-CLOUD-11)

Ret, değişiklik isteğidir. Hazır gerekçe kodları: `capability_unused`, `capability_too_wide`, `undeclared_behaviour`,
`dependency_unneeded`, `dependency_unknown`, `code_unreadable`, `listing_misleading`, `broken`, `other` (not ister).
Her gerekçe dosyası ya da bağımlılığı ve Türkçe çözümüyle kaydedilir; ajan tek adımda düzeltir. Geri dönüş:
düzeltilmiş içerik aynı sürüm numarasıyla yüklenir ve yeniden gönderilir. İçeriği değiştirmeden göndermek yazarın
notunu ister (`unchanged_resubmit`, `gate_blocked`); yanlış pozitif yolu budur, notu inceleyici okur.

### §6 Askı, geri çekme, arşiv (PY-CLOUD-15)

Askıya alma şüphedir, geri alınabilir (yönetici, gerekçeyle). Geri çekme kesindir (yönetici ya da yayımcının
kendisi). İkisi de yeni kurulumu ve çalıştırmayı durdurur, nedeni gösterir ve dürüsttür: "İndirilmiş kopyalar silinemez
ve geri alınamaz; KentOS geri çekme listesini eşitlediğinde onları durdurur (verileri kalır), eşitlemeyen bilgisayar bunu
bilmez." Arşiv, yayımlanmış sürümde emekliliktir (yeni kurulum yok, kurulu kopyalar çalışır), yayımlanmamışta
vazgeçmektir.

### §7 Her hareketin kaydı (PY-CLOUD-16)

`AuditRecord`: zaman (çağıranın verdiği, 1970'ten beri milisaniye, UTC), hesap, kanal, hareket, önceki ve sonraki
durum, içerik özeti, kapı, gerekçe ya da not. Reddedilen hareket kaydı değiştirmez. Depodan okunan kayıt aynı
kurallardan geçer (`Release::read` ve `Deserialize`): onaysız yayımlanmış, yazarınca onaylanmış, onaydan sonra kapısı
açık ya da hareketi eksik kayıt `record_inconsistent`; bozuk ya da bilinmeyen alanlı kayıt `record_invalid`.

### §8 İnceleme farka ve riske odaklı (D12)

`classify(önceki, sonraki, güvenilir)` derinliği, en riskliden başlayan farkları ve kuyruk için tek satırı verir.
`start_review` odağı kendisi hesaplar, dışarıdan odak almaz; karşılaştırılan sürüm bu eklentinin bu sürümden önce
onaylanmış, askıya alınmamış ve geri çekilmemiş bir sürümü ve onun onaylanan paketi olmalıdır (`Release::baseline`,
aksi `baseline_invalid`). Onay, incelemenin derinliğini kaydeder.

| Fark | Derinlik |
|---|---|
| İlk onaylı sürüm | Derin inceleme |
| Yeni yetenek; genişleyen yetenek (kapsam, yeni ağ adresi, dış yapay zekâya veri); yeni bağımlılık (eklenti ya da Python paketi); kaynağı değişen bağımlılık | Derin inceleme, güvenilir yayımcıda da |
| Daralan ya da kalkan yetenek; sürümü, aralığı ya da dosyaları değişen bağımlılık; kalkan bağımlılık; yalnız kilit | Fark incelemesi; güvenilir yayımcının hızlı yolunu kapatır |
| Kod: Python, manifestin davranış alanları, form, panel, araç tanımı, şema, veri | Fark incelemesi; güvenilir yayımcıda hızlı yol |
| Metin ve simge: değişiklik notu, lisans, çeviri, tanıtım metinleri, simge, `.md` ve `.txt` | Hızlı yol |

Her sürümün değiştirdiği karşılaştırmaya girmez: manifestin, `pyproject.toml`'ın ve `uv.lock`'taki projenin kendi sürüm
numarası, kilidin biçim revizyonu. Ne olduğu bilinmeyen dosya koddur. **Ölçüm:** `ornek-imar` örneğinde yalnız sürüm
(1.1.0 → 1.1.1, üç dosyada) ve değişiklik notu değişince paket ve kilit özeti değişti, sınıflama "Hızlı yol — Metin ve
simge: CHANGELOG.md" oldu (`facts_come_from_the_package_and_a_version_bump_is_not_code`).

## Sonuçlar

- Kod: `release.rs` (hareketler ve sorgular) ve `release/`: `state` (tablo), `actor` (yetki, rol, kanal, katalog
  kapsamı), `admit` (kim, nasıl, nereden), `gate`, `record` (gerekçe, onay, duyuru, kayıt), `facts` (paketin söyledikleri,
  kimlik), `review` (sınıflama), `stored` (depodan okuma), `tests` (21 test). `lib.rs`'e tek satır; yeni bağımlılık yok.
- Sunucu, MCP ve Python bu hareketleri çağırır; reddi (`Refusal`: kalıcı kod + Türkçe neden ve çözüm) olduğu gibi
  iletir.

## §9 Bilerek sonraya bırakılanlar

1. **Sunucu ve API:** uçlar ve beklenen sürümle yazma ikinci dilimde, bellekteki depoyla (§10). Kalanı: kalıcı depo
   (§11); kararlar için taze kimlik doğrulama (kurumun kimlik sağlayıcısında yeniden giriş ya da cihazda kullanıcı
   doğrulaması), katalog çalışan sunucuda bundan önce açılmaz; inceleme kuyruğu ve inceleyici kapasitesi;
   inceleyicinin ve tarayıcının taslağın paketini indirmesi (bugün yalnız onaylı paket iner); MCP ve Python yayın
   araçları.
2. **Depolama:** paket ve SBOM baytları nesne deposunda (§11); SBOM üretimi.
3. **İmza ve kimlik:** onay kaydının ve geri çekme listesinin imzası (liste üçüncü dilimde üretiliyor, §12; imzalanana
   dek masaüstü onu uygulamaz); kısa ömürlü yayın kimliği (OIDC), önekli ve süreli jetonlar, ad alanının doğrulanmış
   alan adına bağlanması (D13).
4. **Platform tekerlek deposu:** onaylı kilitten platform başına tekerlek üretimi, ikili taraması, özetlerin onay
   kaydına bağlanması (D3).
5. **Arayüz:** katalog penceresi, inceleyicinin fark görünümü, karne (D17).
6. **Dönemsel yeniden tarama ve kural kaydı** (yeni kural önce yalnız sahibine, D11): şimdilik onaydan sonra kapılar
   kapalı, bulunan sorun askıya almayla ele alınır.
7. Yazar ve yayımcı devri, yerine geçen eklenti (D9); itiraz yolu (D15); kurum ilkesiyle bağ (izin ve engel listesi,
   D14).

## §10 İkinci dilim: sunucu uçları (`apps/api`)

Sunucu akışı yalnız `kentos_extensions::release` ile yürütür: sürümü bulur, çağıranın `Actor`'ünü kataloğun
yetkilerinden kurar, bir hareketin istediği paketleri okur, hareketin sonucunu yazar ve reddi akışın kendi koduyla
iletir. Kod: `apps/api/src/extension_catalog.rs` (hizmet), `extension_catalog/model.rs` (katalog, sürüm anahtarı,
hesap, hareket, hata), `extension_catalog/store.rs` (depo trait'i), `extension_catalog/memory.rs` (testlerin bellekteki
deposu), `http/extension_catalog.rs` (uçlar), `http/extension_catalog/views.rs` (yanıtlar),
`http/extension_catalog_tests.rs`.

| Uç | İş |
|---|---|
| `GET /v1/extension-catalogs/{katalog}/releases` | çağıranın görebildiği sürümler; arama ve sayı yalnız onlar üstünde (PY-CLOUD-17) |
| `POST …/releases` | `.kek` baytları yüklenir: taslak (201, `Location`) |
| `GET …/releases/{eklenti}/{sürüm}` | içeridekilere bütün kayıt (hareketler dahil); ötekilere hesapsız, notsuz, bulgusuz görünüm |
| `PUT …/releases/{eklenti}/{sürüm}` | taslağa ya da geri gönderilen sürüme yeni içerik |
| `POST …/releases/{eklenti}/{sürüm}/moves` | bir hareket: `{"action": "approve", …}`; adlar `Action`'ınkiler |
| `GET …/releases/{eklenti}/{sürüm}/install` | kurulabilecek olan: yayımlanmış sürümün onaylanan içeriği |
| `GET …/releases/{eklenti}/{sürüm}/package` | onaylanan baytlar; çıkmadan önce onaya karşı yeniden denetlenir |
| `GET …/releases/{eklenti}/{sürüm}/sbom` | aynı baytlardan yazılım malzeme listesi (CycloneDX 1.6; ADR 8057 §91); Python'suz pakette `sbom_empty` (422) |

`{katalog}`: `public`, `tenant:{kısa ad}` ya da `personal:{hesap}`. Kurum kısa adı `public` ya da `personal` olabildiği
ve API'de "catalog" proje kataloğunun adı olduğu için yol `/v1/extension-catalogs/…`. Paket baytlarını taşıyan uçlar,
paketin en büyük boyutu (128 MB) ve yükleme süresiyle ayrı katmandadır.

**Kanal (PY-CLOUD-18).** Yalnız erişim anahtarı taşımayan ve `x-kentos-client: web` diyen istek, yani web uygulamasının
tarayıcı oturumu, pencere sayılır. Erişim anahtarı (`Authorization: Bearer`), masaüstü, Python SDK, MCP ve başlık
söylemeyen istek programdır: yükler, gönderir, tarama bildirir, onaylananı yayımlar; onay, değişiklik isteme, engel
kaldırma ve askıdan alma ona `decision_by_program` (403) döner. **Sınır:** başlık uygulamanın beyanıdır, kanıt değil;
bir kişinin tarayıcı oturumunu ele geçiren program onu taklit edebilir. Bu yüzden katalog bu dilimde çalışan sunucuda
kapalıdır (`AppState::extension_catalog` boş, uçlar 404) ve açılmadan önce kararlar taze bir kimlik doğrulama ister (§9).

**Yetkiler.** Varsayılan yok: rol katalog başına verilir (genel katalogda sunucu yöneticisi, kurum kataloğunda kurum
yöneticisi verir; verme aracı kalıcı depoyla gelir) ve kurumun kendi rolleri (sahip, düzenleyen …) katalogda bir şey
vermez. Yayımcı üyeliği hesaba bağlıdır; genel ve kurum kataloğunda yükleyen manifestin `publisherId`'sindeki
yayımcının üyesi olmalıdır (yoksa `publisher_missing`). Kişisel katalogda sahibi yayımcı, inceleyici ve yönetici
rollerini tutar; orada kataloğun hizmetlerinden başka kimse yoktur. Tarayıcı, her katalogda denetim hizmeti rolünü
tutan hizmet hesabıdır; erişim anahtarıyla gelir, yani programdır ve karar vermez.

**Görünürlük.** Katalogda olmayan hesap, göremediği sürüm ve olmayan sürüm aynı 404'ü aynı sözle alır. Liste önce
görünürlükle süzülür, arama ve sayı ondan sonra. Başkasının taslağıyla çakışan sürüm numarası, o taslağın durumunu
söylemeden reddedilir. Bir eklenti kimliğinin bir katalogdaki sürümleri tek yayımcınındır (`extension_owned`): ad alanı
doğrulaması (D13) gelene dek başka bir yayımcı o kimlikte sürüm numarası kapatamaz.

**Ret gövdesi.** API'nin `ApiError`'ı: `error` akışın kodu, `message` Türkçe neden ve çözüm, `requestId`. Durum kodu:
yetki yok 403 (`permission_missing`, `not_owner`, `self_review`, `self_scan`, `decision_by_program`); istek yanlış 422
(`reason_missing`, `changes_missing`, `package_invalid`, `identity_mismatch`, `publisher_mismatch`,
`publisher_missing`, `scan_unexplained`, `waiver_not_allowed`); sunucunun kaydı bozuk 500 (`package_mismatch`,
`baseline_invalid`, `record_invalid`, `record_inconsistent`); geri kalanı, sürümün durumu ya da başka bir hareket engel
olduğu için 409. Yalnız `release_changed` aynen yeniden gönderilebilir (`retryable`). Paket reddi denetleyicinin
sorunlarını (ilk 10'unu) dosyası ve yeriyle söyler.

**Depo.** Küçük bir trait (`ReleaseStore`): kataloğun sürümleri, bir sürüm, yazma (yeni kayıt ya da yalnız okunan
kaydın üstüne: hareket sayısı beklenen değilse `Changed`), içerik özetiyle bir kez yazılan paket baytları, hesabın
yetkileri, güvenilen yayımcı. Hareket okunan kaydın üstüne yazılır; araya başka bir hareket girdiyse yeni kaydın
üstünde yeniden yapılır (en çok 4 kez) ve akış onu yeniden denetler: yedi kapının aynı anda gelen raporları birbirini
ezmez. İçerik yenileme önce denenir; reddedilecekse paket baytları hiç saklanmaz.

**Ölçüm.** `cargo test -p kentos-api --bin kentosd extension_catalog`: 12 test, veritabanısız. Yüklemeden kuruluma uçtan
uca; anahtarla, python, desktop ve mcp ile gelen kararın reddi; yazarın ve yayımcı üyesinin kendi sürümünü
inceleyememesi; yeni içeriğin onayı düşürmesi ve eski içeriğin geç taramasının reddi; göremeyene 404 ve sayılmama;
yalnız onaylı içeriğin kurulması ve askıdaki sürümün dürüst nedeni; farka göre odak ve güvenilir yayımcının hızlı yolu;
ret gövdesi ve durum tablosu; kişisel katalog; yarışı kaybeden hareketin tek kayıtla yeniden yapılması; katalog
kapalıyken 404, veritabanısız oturumda 503.

**Bu dilimde verilen kararlar.**

1. Kanal kuralı ve kataloğun çalışan sunucuda kapalı kalması (yukarıda).
2. Katalog yetkileri varsayılan yok ve katalog başına; kurum rolleri katalog rolü değildir.
3. Katalog adları `public`, `tenant:…`, `personal:…`; yol `/v1/extension-catalogs/…`.
4. Ötekilere görünüm: hesap kimliği, not, bulgu ve değişiklik isteği yok; geçmiş yalnız durumu değiştiren hareketler.
5. Eklenti kimliği sahipliği (`extension_owned`) ve başkasının taslağını ele vermeyen `version_taken` sözü.
6. Testlerde hesap `x-kentos-test-account` başlığından gelir (`Accounts::Named`), yalnız test derlemesinde; sunucunun
   derlemesinde bu yol yoktur.
7. Kişisel katalogda tarayıcı: `Release::visible_to` kişisel kataloğun sürümünü önce yalnız sahibine gösteriyordu;
   tarayıcı onu göremeyince kapıları hiç taranamaz, sürüm onaylanamazdı. Kural akış modülünde düzeltildi (sahibi ve
   denetim hizmeti, `who_sees_a_release`); sunucu görünürlüğü yalnız modülden alır.

## §11 Sonraki dilim: kalıcı depo (tasarım)

`ReleaseStore`'un PostgreSQL ve nesne deposu uygulaması, `kentos` şemasında:

| Tablo | Ne tutar |
|---|---|
| `extension_release` | anahtar (`catalog_kind`, `catalog_ref`, `extension_id`, `version`; tekil), `publisher`, `author`, `state`, içerik sütunları (`package_digest`, `lock_digest`, `sbom_digest`, `capabilities`), kaydın kendisi (`record jsonb`: akışın `Release` JSON'u), `moves` (hareket sayısı), oluşturma ve değişme zamanı |
| `extension_release_move` | hareket kaydı, yalnız eklenir: (`release_id`, `seq`) birincil anahtar, `at`, `account`, `via`, `action`, `from_state`, `to_state`, `content`, `gate`, `reason` |
| `extension_package` | `digest` birincil anahtar, `size`, nesne deposundaki anahtar (`extension-packages/{digest}`); bir kez yazılır |
| `extension_catalog_grant` | (`catalog_kind`, `catalog_ref`, `account`, `role`); hizmet hesapları için `catalog_kind = every` |
| `extension_publisher`, `extension_publisher_member` | yayımcı (kimlik, ad, doğrulanmış alan adı, güvenilir mi) ve üyeleri |

- **Gönderilen içerik değişmez.** `extension_release` üstündeki bir tetikleyici, durum `draft` ya da `changesRequested`
  değilken içerik sütunlarının değişmesini reddeder; akış da reddeder (`content_frozen`). Paket satırı ve nesnesi hiç
  güncellenmez, yalnız eklenir: bir özet hep aynı içeriktir.
- **Hareket kaydı yalnız eklenir.** Uygulama rolünün `extension_release_move` üstünde UPDATE ve DELETE izni yoktur; yeni
  hareketler kayıtla aynı işlemde eklenir ve `moves` onların sayısıdır. Yazma `UPDATE … WHERE moves = $beklenen` ile
  yapılır; satır değişmezse `Changed`.
- **Okurken akışın kuralları.** Kayıt `Release::read` ile okunur, bozuk ya da akıştan geçmemiş kayıt yüklenmez; hareket
  tablosu kaydın hareketleriyle aynı olmalıdır.
- **Kurum kataloğu.** Üyelik var olan kiracı üyeliğinden ve kataloğun yetki tablosundan gelir; satır düzeyi güvenlik
  projelerdeki gibi uygulanır.
- **Verme.** `kentosd extension-catalog grant|revoke|trust` (sunucu yöneticisi), sonra yönetim uçları.
- Göç bu dilimde yazılmadı; veritabanı testleri kalıcı depoyla gelir.

## §12 Üçüncü dilim: geri çekme listesi

Katalog, ADIM 17'de masaüstünün eklenti barındırıcısının okuduğu listeyi (`kentos.extension-revocations` v1) kendi
sürümlerinden üretir.

- **Tek biçim.** `Revoked`, `Revocations`, `REVOCATIONS_FORMAT`, `Revocations::read` ve `Revocations::reason`
  barındırıcının `policy.rs`'inden paylaşılan crate'e taşındı (`crates/shared/extensions/src/revocations.rs`). Kodlar
  ve Türkçe sözler aynı (`revocations_invalid`). Barındırıcı aynı adları `policy`'den yeniden verir; çağıranları ve
  testleri değişmedi. Yazan ve okuyan aynı tipleri kullandığı için birbirinden ayrılamaz.
- **Üretici** (`release::revocation_list`, `release/revoked.rs`). Katalogun bütün sürümleri verilir. Yayımlanmış ve
  şimdi askıda ya da geri çekilmiş her sürüm bir girdidir: `id`, `versions: "=X.Y.Z"` (anlamsal biçimde: önsürüm eki
  kalır, yapı üst verisi atılır) ve `reason`.
  - Askıdan alınan sürüm, durumuyla birlikte listeden düşer.
  - Arşivlenen sürüm listelenmez: yayımlanmışsa çalışmaya devam eder (§1).
  - Yayımlanmamış hiçbir sürüm listelenmez. Durum yetmez; kayıtta yayımlama hareketi aranır.
  - Sıra önce kimliğe, sonra anlamsal sürüme göredir (1.2.0, 1.10.0'dan önce); aynı sürümler hep aynı baytları verir.
- **Gerekçe.** Duyurunun gerekçesi, `notice_text`'in sözleriyle ve sondaki noktası olmadan yazılır, çünkü barındırıcı
  onu kendi cümlesine koyar: "1.0.0 sürümü geri çekildi: {gerekçe}.". Barındırıcı her girdiye "geri çekildi" der;
  askıdaki sürümün geri dönebileceğini bu yüzden gerekçe söyler: "… (askıya alındı; inceleme bitince askıdan
  alınabilir ya da geri çekilir)". Duyuru ve girdi bu sözü aynı sabitten (`UNDER_REVIEW`), gerekçeyi aynı yardımcıdan
  alır.
- **Yeni alan yok.** v1 okuyucusu bilinmeyen alanı reddeder; `generatedAt` gibi tek bir anahtar, bütün kurulu
  KentOS'ların listeyi reddetmesi demektir. Liste yalnız `format`, `schemaVersion`, `entries`, girdi yalnız `id`,
  `versions`, `reason` yazar. Bir test bunu ve fazla anahtarın reddedildiğini kilitler. İmza da alan olmayacak:
  kataloğun güven kökü listeyi okumadan önce dışarıdan denetler.
- **Uç** (`GET /v1/extension-catalogs/{katalog}/revocations`). Katalogun her üyesine aynı liste gider; kişisel
  katalogda yalnız sahibine, kataloğun tarayıcısına bile gitmez. Ötekiler öteki uçlardaki 404'ü aynı sözle alır.
  Liste bütün sürümlerden yapılır, ama içinde yalnız herkesin zaten gördüğü geri alınmış yayınlar ve gerekçeleri
  vardır: hesap kimliği, not, bulgu, içerik özeti ve taslak yoktur. İçeriden ve dışarıdan bakan aynı listeyi alır.
- **Önbellek.** `Cache-Control: private, no-cache` ve gövdenin SHA-256'sı `ETag` olarak gider. Geri çekme bir sonraki
  eşitlemede görülmeli; bu yüzden hiçbir kopya sorulmadan kullanılmaz (`max-age` yok). Değişmeyen liste
  `If-None-Match` ile 304 döner, sormak ucuz kalır. `private`, çünkü yanıt oturum açmış bir üyenindir; paylaşılan bir
  önbellek kurum ya da kişisel kataloğun listesini başkasına vermemeli.
- **İmzasız.** Bu dilimde liste imzasızdır ve **masaüstü onu imzalanana dek uygulamamalıdır**: barındırıcı listeyi
  ancak kataloğun güven kökünün imzası denetlendikten sonra uygular (§9.3).

**Ölçüm.**

- `cargo test -p kentos-extensions --lib -- release:: revocations::`: 28 test geçti; 5'i üreticinin (askı ve
  askıdan alma, geri çekme, arşiv ve yayımlanmamışlar, sıra, fazla alan yok), 2'si biçimin.
- `cargo test -p kentos-extension-host`: 30 + 1. Yenisi sözleşme testidir: barındırıcının `Revocations::read`'i
  üreticinin JSON'unu okur ve `reason` gerekçeyi verir. Barındırıcının cümlesi şöyle çıkar: "1.0.0 sürümü geri çekildi:
  Kullanıcı verisini dışarı gönderiyor.".
- `cargo test -p kentos-api --bin kentosd extension_catalog`: 15 test; 3'ü yeni (üyeye aynı liste ve içeriden ve
  dışarıdan aynı olması, dışarıdakine 404, 304 ile yeni geri çekmede yeni etiket). 976 satırlık test dosyası dört
  gruba ayrıldı: akış, erişim, retler, geri çekme listesi.

**Bu dilimde verilen kararlar.**

1. Askıdaki sürüm de listelenir, çünkü yeni kurulumu ve çalıştırmayı durdurur (§6); gerekçesine askı sözü eklenir,
   askıdan alınınca listeden düşer.
2. Arşivlenen sürüm listelenmez; yayımlanmamış hiçbir sürüm listelenmez (kayıtta yayımlama hareketi aranır).
3. Sürüm aralığı tam sürümdür (`=X.Y.Z`) ve anlamsal biçimde yazılır.
4. Kişisel katalogun listesini yalnız sahibi alır.
5. Önbellek `private, no-cache` ve `ETag`'dir.
6. Listede yeni alan yoktur; imza listenin dışından gelecek.

**Kalan.** Listenin imzası ve masaüstünün imzalı listeyi eşitlemesi (§9.3). D15'in kalanı: kamuya açık kaldırma kaydı,
"şüpheli" ile "kesin" ayrımının biçimde gösterilmesi (v1 tek tür girdi bilir; ayrım bugün gerekçenin sözünde) ve itiraz
yolu. Kalıcı depo (§11) geldiğinde liste aynı üreticiden çıkar.

## §13 Dördüncü dilim: kalıcı depo ve çalışan sunucuda katalog

`ReleaseStore`'un PostgreSQL ve nesne deposu uygulaması (`extension_catalog/postgres.rs` ve `postgres/` altında
`moves`, `packages`, `access`), göç `0042_extension_catalog.sql`, yönetici komutları (`extension_catalog/grants.rs`,
`cli/extension_catalog.rs`) ve sunucunun bağlantısı (`main.rs`). Hareket yapma `extension_catalog/acting.rs`'e taşındı.
Katalog yalnız `KENTOS_EXTENSION_CATALOG=on` ile açılır; varsayılan `off`, `on|off` dışındaki değer hatadır. Açıkken
veritabanı ister, yoksa sunucu başlamaz.

- **Tablolar (§11'deki gibi).** `extension_release` (yer, yayımcı, yazar, durum, içerik sütunları, `record jsonb`,
  `moves`, zamanlar), `extension_release_move` (yalnız eklenir), `extension_package`, `extension_catalog_grant`,
  `extension_publisher`, `extension_publisher_member`. Sütunlar kaydın kendisidir: bir `check` kısıtı kimliği,
  durumu, içerik özetlerini, yetenekleri ve hareket sayısını `record` ile karşılaştırır; kaydından başka bir şey
  söyleyen satır yazılamaz.
- **Veritabanının kendi bekçisi.** Tetikleyici (`extension_release_kept`) gönderilmiş içeriği (`content_frozen`),
  sürümün kimliğini (yer, eklenti, sürüm, yayımcı, yazar) ve kayıttaki hareketleri korur: hareket azalmaz, eski
  hareket değişmez. Uygulama rolünün `extension_release_move` ve `extension_package` üstünde yalnız SELECT ve INSERT'ü
  vardır, `extension_release` üstünde DELETE'i yoktur; yetkileri, yayımcıları ve güvenleri yalnız okur.
- **Yazma.** Yeni sürüm `insert … on conflict do nothing` (satır yoksa `Exists`); hareket `update … where moves =
  <okunan>` (satır değişmezse `Changed`). Yeni hareket satırları aynı işlemde, tablodaki eski hareketlerin kayıtla aynı
  olduğu denetlendikten sonra eklenir.
- **Okuma.** Kayıt `Release::read` ile okunur ve hareket tablosu kaydın hareketleriyle satır satır aynı olmalıdır.
  Tutmayan kayıt depo hatasıdır (500, ayrıntı günlükte); sürüm hiç yüklenmez.
- **Kapsam ve satır düzeyi güvenlik.** Her çağrı, sorulan kataloğun kapsamında bir işlemdir: kurum kataloğu kurumun
  kimliğiyle (`app.tenant_id`), kişisel katalog hesabın kimliğiyle (`app.user_id`), genel katalog kapsamsız.
  Politika projelerdeki gibidir: bir kurumun sürümleri ve hareketleri yalnız onun kapsamında görünür ve yazılır. Kimin
  hangi sürümü gördüğü yine akışın kararıdır (`visible_to`); satır düzeyi güvenlik, kataloğu süzmeyi unutan bir
  sorguyu başka kurumun satırlarından korur.
- **Paketler.** Baytlar nesne deposunda içerik özetinin anahtarıyla bir kez yazılır: özet üstünde kilit, kendi
  anahtarına yazma, diske indirme, yerine taşıma, sonra satır. Var olan pakete ikinci yazma bir şey değiştirmez.
  Okurken boyut satırla karşılaştırılır; içerik yine akışın onay denetiminden geçer (§10).
- **Yedek.** Paket nesneleri yedeğin adlandırdığı nesnelerdendir (`backup::named`): dökümle birlikte alınır, geri
  yüklemede denetlenir.

**§11'den sapmalar (kanıt neyi söylediyse).**

1. Güven katalog başınadır: `extension_publisher` üstünde "güvenilir mi" sütunu yerine `extension_catalog_trust`
   tablosu. Depo "bu katalog bu yayımcıya güveniyor mu" diye sorar; tek bir sütun, bir kurumun güvendiği yayımcıyı
   genel katalogda da güvenilir yapardı.
2. Nesne anahtarı `extension-packages/sha256-<hex>`: nesne deposu anahtar parçasında yalnız harf, rakam ve tire kabul
   eder, `sha256:` içindeki iki nokta kabul edilmez. Yazılırken `extension-packages/staging/<kimlik>`.
3. `extension_package` baytların SHA-256'sını da tutar: yedek nesneleri baytlarının özetiyle denetler, içerik özeti
   ise dosyaların yolunun, boyutunun ve özetinin özetidir.
4. Kurum kataloğunun `catalog_ref`'i kısa ad değil kurumun kimliğidir: kapsam kurumu kimliğiyle taşır, politika onu
   projelerdeki gibi `kentos.current_tenant()` ile karşılaştırır. Kısa addan kimliğe `kentos.extension_catalog_tenant`
   (security definer) çevirir: kurum satırı yalnız üyelerine görünür, tarayıcı ve denetim işçisi üye değildir.
5. Kişisel katalogda yetki yazılmaz (`catalog_kind` `every`, `public` ya da `tenant`): sahibi rollerini zaten tutar.
6. Komutlar `grant|revoke|trust` ile bitmedi, `publisher` (yayımcı açma ve üyelik) ve `list` de var. Genel ve kurum
   kataloğuna yükleyen yayımcının üyesi olmalıdır (§10, `publisher_missing`); üyelik veremeyen bir araçla o kataloglar
   kullanılamazdı.
7. Var olmayan kurumun kataloğu kimse için yoktur (bellekteki depoda her kısa adın kataloğu vardı); hizmet hesabı da
   orada üye sayılmaz. Kurum kataloğunun üyesi, etkin kurumda etkin üyeliği olan hesaptır (koltuk gerekmez).

**Çalışan sunucuda ne açık.** §9.1 "taze kimlik doğrulama gelmeden katalog çalışan sunucuda açılmaz" diyordu;
gerekçesi (§10) pencere başlığının taklit edilebilmesi, yani bir kişinin kararının bir programca verilebilmesiydi.
Gerekçe kararlarla ilgili; bu yüzden kararlar kapalı, geri kalanı açık:

- **Kapalı:** onay, değişiklik isteme, engel kaldırma ve askıdan alma (`Action::needs_person`). Sunucunun
  oturumlarından gelen hesapta `Account::fresh_sign_in` hep `false`; pencereden gelen karar, akış izin verseydi bile
  `fresh_sign_in_required` (403, `retryable: false`) alır ve hiçbir şey yazılmaz. Akışın kendi reddi önce gelir:
  program `decision_by_program`, yetkisiz `permission_missing`, görünmeyen sürüm 404. Gerçek oturumla sınandı.
- **Açık:** yükleme ve içerik yenileme (taslak), liste ve okuma, incelemeye gönderme, tarama raporu, incelemeye alma,
  askıya alma (karar değil, güvenlik hareketi), geri çekme, arşivleme, yayımlama, kurulum bileti, onaylı paket, geri
  çekme listesi. `project.audit` işleri aynı katalogdan okur (`CatalogAudits`, §66'nın eşlemesiyle).
- **Sonuç:** yeni bir sunucuda onaylanan, dolayısıyla yayımlanan sürüm olmaz. Kurulum bileti, onaylı paket ve denetim
  işi taze kimlik doğrulama gelene dek bir şey bulamaz (denetim işi "yayımlanmış bir sürümü yok" der); yayımcı yükler
  ve gönderir, tarayıcı raporlar, inceleyici incelemeye alır.
- **Sınır:** her oturum açmış hesap kendi kataloğuna paket yükleyebilir (en çok 128 MB) ve yüklenen her paket kalır;
  kota yok. Kataloğu açan yönetici bunu bilmeli (Kalan).

**Yayımlamada ortam (§66'nın çağrı yeri).** Bir hareket sürümü başka bir durumdan yayımlanmışa geçirince (yayımlama ya
da askıdan alma) katalog sürümü ve onaylı baytlarını `OnPublish`'e verir. Sunucununki (`PrepareEnvironments`) ortamı
ayrı bir iş parçacığında `prepare_release` ile hazırlar, sonucu günlüğe yazar ve isteği bekletmez. Denetimi ya da
Python projesi olmayan sürümde hazırlanacak bir şey yoktur; ağ isteyen denetimin reddi günlüğe yazılır. Çalışan
sunucuda bu yola kararlar açılınca girilir; testler ve uçtan uca sınama onu kullanır.

**Ölçüm.**

- `KENTOS_TEST_DB=required cargo test -p kentos-api --bin kentosd extension_catalog`: 48 test geçti (47'si katalogun,
  1'i ayarın). 18 durum iki depoda birden koşar (`on_both_stores!`: bellekte ve kendi test veritabanında, kopya yok);
  9'u yalnız kalıcı deponun: içeriği, kimliği ve hareketleri veritabanı korur; uygulama rolü hareket ve paketi
  değiştiremez, silemez; eski kayıt üstüne yazma `Changed`, ikinci ilk yazma `Exists`; kurumlar arası satır düzeyi
  güvenlik ve aynı 404; paket bir kez yazılır; hareketleri tutmayan ya da adım atlamış kayıt yüklenmez; yönetici
  yetkileri; çalışan sunucuda oturumla karar reddi; süre. 2'si depodan bağımsız.
- Depo süreleri (yerel PostgreSQL 18.4, hata ayıklama derlemesi; 10 sürüm × 12 hareket): yazma ortanca 3,53 ms, %95
  4,73 ms; okuma ortanca 0,41 ms, %95 0,75 ms; en çok 10 sürümlü katalog listesi ortanca 1,43 ms, %95 2,22 ms.
- Uçtan uca (`cargo test -p kentos-api --test job_run_db audit_catalog`): kurum kataloğunda yükleme, 7 kapı,
  gönderme, inceleme, onay, yayımlama; yayımlamadan ortamın hazır olmasına 4,67 sn (çevrimdışı, tekerlek klasöründen);
  `project.audit` işi gerçek `kentos-jobrun` ile 2,04 sn, kural tekerleğin modülünü içe aktarıp 5 bulgu verdi.
- `cargo test -p kentos-application --test backup`: paketli yedek alınır, geri yüklenir, denetlenir (5 nesne).
- Komutlar geçici bir test veritabanında elle sınandı: yetki, yayımcı ve üyelik, güven, liste, geri alma ve hatalı
  girdilerin Türkçe sözleri; katalog açık ama veritabanı yokken sunucu başlamaz.

**Bu dilimde verilen kararlar.**

1. Katalog `KENTOS_EXTENSION_CATALOG=on` ile açılır, varsayılan kapalı.
2. Çalışan sunucuda kişinin kararları taze kimlik doğrulaması gelene dek kapalı (`fresh_sign_in_required`, 403),
   gerisi açık; ret, akışın kendi reddinden sonra gelir.
3. Kapsam çağrının sorduğu katalogdan kurulur: depo trait'i çağıranı bilmez, görünürlük akışındır.
4. Güven katalog başına, ayrı tabloda.
5. Paket anahtarı `sha256-<hex>`; baytların SHA-256'sı ayrıca tutulur; paketler yedektedir.
6. Yönetim komutlarına `publisher` ve `list` eklendi.
7. Sürüm satırı silinmez: uygulama rolünün DELETE izni yok.
8. Yayımlanan (ya da askıdan alınıp yeniden yayımlanan) sürümün ortamı yayımlamada hazırlanır; hazırlık isteği
   bekletmez.

**Kalan.** Taze kimlik doğrulama (kurumun kimlik sağlayıcısında yeniden giriş ya da cihazda doğrulama) ve kararların
açılması; yetki ve yayımcı yönetim uçları (bugün yalnız komut); paket kotası, hiçbir sürümün göstermediği paketlerin ve
yarım kalmış `staging` nesnelerinin temizliği; S3 uyumlu nesne deposu; tarayıcının hizmet kimliği; liste ve aramanın
veritabanında yapılması (bugün kataloğun bütün kayıtları okunup süzülür); geri çekilen sürümün ortamının kaldırılması.

## §14 Taklit denetimi: ad, kimlik ve simge (D13)

**16 Kasım.** Kod: paylaşılan `resemblance.rs` (yeni: `skeleton`, `resemblances`, `imitations`, `sentences`); akışta
`Listing` (sürüm kaydının adı ve simge özeti) ve `Release::imitations`; sunucu `acting.rs` (`imitation_gate`); kayıt
defteri `imitation.rs` (yeni), `Review::resemblances`, `Registry::review_beside`; masaüstünün kurulum incelemesi.

- **Sorun.** 2026'da bir düzenleyicinin pazarından kaldırılan 2.140 eklentinin 1.109'u taklitti; görünen adı benzersiz
  olmayan bir biçimlendirici kopyası iki günde bine yakın kuruluma ulaştı
  (`docs/research/2026-11-13-eklenti-ekosistemleri.md`, V14, V21). Yayımcı kapısı "ad, simge ve kimlik başkasınınkini
  taklit etmez" diyordu (§4) ama onu denetleyen bir kural yoktu; masaüstüne e-postayla gelen `.kek`'i ise hiçbir kapı
  görmez.
- **Karar: okunan yüz.** İki metin baytlarıyla değil, okuyanın gördüğüyle karşılaştırılır (iskelet): küçük harf;
  Türkçe harfler tabanlarıyla (ı, İ, i; ş, s …); Latin'e benzeyen Kiril ve Yunan harfleri o harf (а, е, о, р, с, і …);
  harf yerine geçen rakam ve çiftler (0 → o; 1, l, I → i; rn → m; vv → w); harf ve rakam dışı her şey (boşluk, tire,
  nokta, görünmez karakter) atılır. Aynı iskelet **taklittir**: göz ayırt edemez. Bir harf ya da yer değiştirmiş iki
  harf uzaklık **benzerliktir**: çoğu zaman dürüst bir addır (iki belediyenin "Ula" ve "Urla"sı); söylenir, hiçbir
  şeyi engellemez (D11: engelleyen kural az ve kesin). Benzerlikte 6 harften kısa ad ve 10 harften kısa kimlik
  karşılaştırılmaz. Simgenin baytları aynıysa taklittir; KentOS iskeletinin her yeni eklentiye verdiği simge kimsenin
  değildir.
- **Katalogda: yayımcı kapısı, gönderimde.** Sürüm kaydı yüklemede paketinden adını ve simge özetini tutar
  (`listing`; içerik yenilenince yenilenir). Gönderilen sürüm aynı katalogda başka bir yayımcının yayımlanmış
  eklentisini taklit ediyorsa katalog kendisi (`kentos-katalog`, tarayıcı rolüyle) yayımcı kapısını bulgularıyla
  engeller ve sürüm yazarına hemen döner: tarayıcı ya da inceleyici ona zaman harcamaz. Yayımcı kapısının kuralı
  kesindir (kaldırılamaz): yazar adı ya da simgeyi değiştirip içeriği yeniler. Taklit bulunmaması kapıyı geçirmez (ad
  alanının sahipliği tarayıcınındır), o yüzden temiz sonuç bildirilmez. Yayımcının kendi eklentileri birbiriyle
  karşılaştırılmaz. Kayıt biçimi geriye uyumlu: `listing`'i olmayan eski kayıt okunur, karşılaştırılmaz.
- **Masaüstünde: kurulum incelemesi.** Kurulacak paket kurulu eklentilerle ve okunan kataloğun girdileriyle
  karşılaştırılır. E-postayla gelen `.kek`'in yayımcısını kimse denetlemediği için aynı ad alanı da karşılaştırılır;
  yalnız güncellediği eklentinin kendisi karşılaştırılmaz. Taklit incelemenin en üstünde uyarıdır ve Kur, kişi
  "Benzerliği okudum, yine de kur"u işaretleyene dek kapalıdır; benzerlik bilgi olarak söylenir. Bulgular öteki
  eklenti başına tek cümle: "Adı, kimliği (tr.ornek.vltrin) ve simgesi “Eklenti Vitrini” (tr.ornek.vitrin)
  eklentisininkilerle aynı okunuyor." İlk denemede aynı eklenti üç ayrı cümlede üç kez yazılıyordu.
- **Kurulduktan sonra da işaretli (aynı gün).** Uyarıya rağmen kurulan taklit ve taklit edilen, ikisi de kurulu
  kaldıkça ayrıntılarında ve kurulu eklentilerin görünümünde (`ExtensionView::resemblances`; MCP'nin
  `kentos://extensions/installed`'ı da) birbirleriyle söylenir: "Kurulu başka bir eklentiyle karışabilir. Adı ve
  kimliği … aynı okunuyor. Hangisinin hangisi olduğundan emin değilseniz birini kaldırın." Burada ad ve kimlik
  karşılaştırılır; simge Kur'da karşılaştırıldı (liste her çizildiğinde her simgeyi okumak söylediğinden pahalıydı).
- **Gerçek pencere (KentOS GIS, yalıtılmış profil, Vitrin kurulu):** Vitrin'in kimliği `tr.ornek.vltrin`, adı
  "EKLENTİ VİTRİNİ" yapılmış kopyası ayrı bir katalogdan incelendi: üç bulgu tek cümlede, Kur kapalı; kutu
  işaretlenince açıldı; kurulunca listede ve şeritte iki "Vitrin" göründü; kopyanın ayrıntısı uyarıyla açıldı.
- **Sınamalar:** paylaşılan 5 (iskelet, taklit, benzerlik, simge, cümleler) ve akış 27; kayıt defteri 1 (31); masaüstü
  1 (eklenti sınamaları 99); katalog API 1 (50; bellek deposunda; veritabanı türü yerel test veritabanının yönetici
  adresi olmadığından atlandı).
- **Kalan:** ad alanının doğrulanmış alan adına bağlanması (D13'ün öteki yarısı) ve tarayıcının yayımcı kapısında aynı
  kuralı çalıştırması; inceleyicinin benzerlikleri görmesi (inceleme penceresi henüz yok); kurulu taklidin
  ayrıntısında kalıcı bir işaret.

## §15 Sır taraması: dosyanın içine yazılmış anahtar (PY-CLOUD-05, D11)

**16 Kasım.** Kod: paylaşılan `secrets.rs` (yeni: `found`, `scan`, `gate_findings`); paket denetimi `archive.rs`
(her metin dosyası); sunucu `acting.rs` (`secrets_gate`); `verifyCodes`'a `secret_in_text`.

- **Sorun.** Paket denetimi gizli bilgi taşıyan dosyaları adından dışlıyordu (`content_secret`: `.env`, `.pem`, `id_rsa`,
  `credentials.json`; yapay zekâ araçlarının ayarları), ama izinli bir dosyanın içine yazılmış anahtarı görmüyordu:
  `src/arac.py`'de `ANAHTAR = "sk-…"`. Eklentileri yapay zekâ ajanları yazıyor (planın 1.1'i) ve ajana kullanıcının
  anahtarı verilmiş olabilir; kataloğa giden paket anahtarı herkese verir. Sır kapısı vardı (§3), onu besleyen kural yoktu.
- **Karar: yalnız biçiminden anahtar olan.** Sağlayıcının öneki, ardından gelen harflerin türü ve sayısı: Anthropic
  (`sk-ant-`), OpenAI (`sk-proj-`, `sk-svcacct-`), Stripe (`sk_live_`, `rk_live_`), harf ve rakam karışık 32+ karakterli
  `sk-` (DeepSeek ve benzerleri), GitHub (`ghp_`… 36, `github_pat_`), GitLab (`glpat-`), Slack (`xoxb-`…), Google
  (`AIza` + 35), AWS (`AKIA`/`ASIA` + 16), Hugging Face (`hf_`), PyPI (`pypi-AgE`), npm (`npm_` + 36); ve özel anahtar
  zırhı (`-----BEGIN … PRIVATE KEY-----`). Önek daha uzun bir sözcüğün içindeyse (`task-…`) sayılmaz. Örnek değer sayılmaz
  (tek harfin tekrarı, "xxxx", "example", "your", "dummy", "örnek" …: `AKIA…EXAMPLE`). **Ad yargılanmaz**
  (`api_key = …`): engelleyen kural az ve kesin olmalı (D11); tahminle uyaran kural kimseye yaramaz.
- **Değer hiçbir yerde söylenmez:** bulgu türü ve satırı yazar ("Anthropic API anahtarı gibi görünen bir değer var",
  "satır 5"); ileti, rapor ve kapı kaydı anahtarı taşımaz (sınamalar denetler). Depo da anahtar biçiminde bir dizgi
  taşımaz: sınamalar örnek anahtarı çalışırken üretir.
- **Paketlemede ve kurulumda uyarı** (`secret_in_text`): kurumun kendi iç hizmeti için yazdığı paket anahtarını bilerek
  taşıyabilir; kişinin bilgisayarında karar onundur. `kek verify`, ajanın `extension.verify`'ı ve kurulum incelemesi
  aynı uyarıyı satırıyla görür (ajan tek adımda düzeltir).
- **Katalogda engel:** gönderilen sürümün paketi taranır; bulgu varsa katalog kendisi (`kentos-katalog`, tarayıcı rolüyle)
  sır kapısını bulgularıyla engeller (`secret_found`, dosya ve satır) ve sürüm yazarına döner. Taklit kapısı (§14) gibi:
  hiçbir şey bulunmaması kapıyı geçirmez (daha geniş kurallar tarayıcınındır), temiz sonuç bildirilmez; başka bir kapının
  geri gönderdiği sürüme dokunulmaz. Sır kapısı gerekçeyle kaldırılabilen kapılardandır (karar 5): yanlış bir bulgu
  inceleyicinin gerekçesiyle aşılır.
- **Sınamalar:** paylaşılan 3 (sekiz sağlayıcının biçimi ve zırh satırıyla; örnek değer, kısa/uzun dizi, `sk-`'den sonra
  sözcük, sözcük içi önek, ad, sertifika bulgu değil; paketleme uyarısı ve kapı bulgusu değeri söylemeden), paylaşılan
  crate'in hepsi (161) geçti; katalog API 1 (52; README'sinde anahtar olan sürüm `changesRequested`, kapı `blocked`,
  bulgu `secret_found` · `README.md` · `satır 5`, yanıtın hiçbir yerinde anahtar yok; anahtarsız sürüm `submitted`).
  Mevcut paketler: mevzuat ve Vitrin 0 uyarı.
- **Atölye kullanıcının kendi anahtarını tanır (aynı gün).** Asistan'ın `extension.workshop.write`'ı, yazacağı
  dosyalardan biri kullanıcının yapay zekâ ajanlarından birinin anahtarını (saklanan, oturumun ya da ortam değişkeninin)
  birebir içeriyorsa hiçbir şeyi yazmaz ve ajana kimin anahtarı olduğunu söyler: "“src/anahtar.py” kullanıcının bir
  anahtarını içeriyor (“Deneme ajanı” ajanının API anahtarı): anahtar pakete yazılmaz …" (`secret_in_text`). Biçimden
  bağımsızdır ve yanlış bulgu vermez: aynı dizgi. Evsahibi bildirir (`Host::own_secrets`); MCP'nin evsahibi bilmez.
  Hizmet bağlantılarının sırları da (aynı gün): API anahtarları, parolalar, jetonlar ve istemci sırları
  (`services::secrets::Secrets::known`; kullanıcı adı ve istemci kimliği sır değil), bağlantının adresiyle söylenir
  ("https://harita.ornek.bel.tr bağlantısının parolası"). 8 karakterden kısa değer aranmaz (her metinde geçer).
  Sınama: masaüstü `limits_tests::the_workshop_refuses_the_users_own_key` (ajan anahtarı ve bağlantı parolası ret,
  değer söylenmez, dosya yazılmadı; anahtarı ortamdan okuyan aynı dosya yazıldı).
- **Kalan:** tarayıcının daha geniş sır kuralları (ad + yüksek düzensizlik, geçmişte sızmış anahtar listeleri),
  yalıtılmış duman testi işçisi.

## Uygularken verilen kararlar (planda yazılı değildi)

1. Kişisel katalogda sahibi kendi sürümünü inceleyip onaylar; kurum kataloğunda yazar onaylayamaz, yayımcının başka bir
   üyesi onaylayabilir; genel katalogda yayımcının hiçbir üyesi onaylayamaz.
2. Onay, değişiklik isteme, engel kaldırma ve askıdan alma yalnız pencereden; kanalı sunucu kimlik doğrulama yolundan
   belirler.
3. Yayımlamayı yayımcı yapar (onay inceleyicinin, zamanlama sahibin kararı); yayımcı kendi sürümünü geri de çekebilir.
4. Engelleyen kapı incelemedeki sürümü kendiliğinden geri gönderir; aynı içerikle yeniden gönderme not ister.
5. Gerekçeyle kaldırılabilen kapılar: bağımlılık, lisans, sır, test.
6. Onaylanmış ama yayımlanmamış sürümde de değişiklik istenebilir; onay düşer.
7. Sürüm numarası katalog başına bir kez; yapı üst verisi ayrı sürüm sayılmaz.
8. Ret gerekçelerinin ilk sözlüğü (dokuz kod, §5).
9. Zaman damgası çağıranın verdiği milisaniyedir; akış saat okumaz.
10. Yayımlandıktan sonra arşivlenen sürüm çalışmayı sürdürür, yayımlanmadan arşivlenen hiç çalışmaz.
11. Bağımlılık sürümünün değişmesi derin değil, fark incelemesidir (yalnız yeni bağımlılık derindir); kaynağı değişen
    bağımlılık yeni sayılır.
