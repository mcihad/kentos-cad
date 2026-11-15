# ADR 8059: Eklenti kataloğunda sürüm akışı — onaylı yayının durum makinesi

- **Durum:** uygulandı, ilk dilim (15 Kasım 2026). Yalnız saf Rust mantığı: `crates/shared/extensions/src/release.rs`
  ve `release/`. Sunucu, depo, imza ve arayüz sonraki dilimlerin (§9).
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

Kişisel katalogdaki sürümü her durumda yalnız sahibi görür. Kataloğun (kiracının) üyeliğini sunucu önce denetler;
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

1. **Sunucu ve API:** uçlar, kayıtların PostgreSQL'de saklanması, eşzamanlı yazmada beklenen sürüm, inceleme kuyruğu
   ve inceleyici kapasitesi; MCP ve Python yayın araçları.
2. **Depolama:** paket ve SBOM baytları nesne deposunda; SBOM üretimi.
3. **İmza ve kimlik:** onay kaydının ve geri çekme listesinin imzası; askıdaki ve geri çekilen sürümlerden ADIM 17'nin
   `kentos.extension-revocations` listesinin üretimi; kısa ömürlü yayın kimliği (OIDC), önekli ve süreli jetonlar, ad
   alanının doğrulanmış alan adına bağlanması (D13).
4. **Platform tekerlek deposu:** onaylı kilitten platform başına tekerlek üretimi, ikili taraması, özetlerin onay
   kaydına bağlanması (D3).
5. **Arayüz:** katalog penceresi, inceleyicinin fark görünümü, karne (D17).
6. **Dönemsel yeniden tarama ve kural kaydı** (yeni kural önce yalnız sahibine, D11): şimdilik onaydan sonra kapılar
   kapalı, bulunan sorun askıya almayla ele alınır.
7. Yazar ve yayımcı devri, yerine geçen eklenti (D9); itiraz yolu (D15); kurum ilkesiyle bağ (izin ve engel listesi,
   D14).

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
