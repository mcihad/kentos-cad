# ADR 0025: `.kcad` v2: KentOS CBOR profili, kap düzeni, kalıcı kimlikler ve v1 göçü

- **Durum:** kabul edildi (2026-09-26). Yön [ADR 0011](0011-kcad-binary-snapshot.md)'den gelir. Kodlama sahibin 26 Eylül kararıdır: kendi CBOR profilimiz, paylaşılan Rust'ta, yeni bağımlılık olmadan; yayımlanmış bir spesifikasyon ve aynı profili anlatan küçük bir Python okuyucusu. Bayt düzeni, profil ve şema [docs/specs/kcad-v2.md](../specs/kcad-v2.md)'dedir; bu ADR kararları ve gerekçelerini tutar.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md §9 (`FILE-01..08`, `FILE-12..23`), §22.1 adım 8, `DOM-03/04/06`, `NUM-01`, `SYNC-14`; ADR 0002, 0008, 0009, 0010, 0011, 0014 (dilim 4), 0020

## Bağlam

- Bugüne kadar `.kcad` v1 JSON'du (`DocumentSnapshotV1`). Kalıcı kimlik yazmıyordu: açılışta içerikten türetiliyordu (ADR 0014, dilim 2). Düzenlenip v1 olarak kaydedilen dosya başka bir anlık görüntüydü ve bütün nesneler yeni kimlik alıyordu.
- ADR 0011 yönü koydu: yalnız kaydetme ve yükleme için sürümlü bir binary anlık görüntü; v1 okunmaya devam eder; tür içerikten koklanır; açık spesifikasyon, bayt düzeyinde örnekler, bağımsız okuyucu.
- Sahibin kararı (26 Eylül): CBOR sandığı, sıkıştırma ya da özet sandığı eklenmez. SHA-256 zaten kilitli `sha2`'den gelir (wasm32 için de onaylı). v2.0'da sıkıştırma yoktur; başlık kodek alanını ayırır, tanımlı tek değeri “yok”tur (`FILE-10/11`).

## Karar

### Kap

| Parça | Karar | Neden |
|---|---|---|
| İmza | `89 4B 43 41 44 0D 0A 1A 0A` (`\x89KCAD\r\n\x1a\n`) | PNG'nin düzeni: 7 bit aktarımı, satır sonu çevirisini ve JSON sanılmayı ilk baytlarda yakalar; ilk beş bayt tutup kalanı tutmazsa hata “metin olarak aktarılmış” der |
| Başlık | 36 bayt sabit + zorunlu uzantı listesi; tam sayılar küçük sonlu | `major`, `minor`, `minReaderMinor`, `headerLength`, `encoding`, `codec`, `payloadLength`, `decodedLength`, `flags`, `extensionCount` (spec §3) |
| Bütünlük | Dosyanın son 32 baytı, önceki **bütün** baytların SHA-256'sı | Başlık da kapsanır; akarak yazılır; eksik dosya ve fazla bayt boy denetiminde, bozuk bayt özette yakalanır. Özet bozulmayı yakalar, **imza değildir** (`FILE-12`) |
| Sürümler | Kap sürümü (2.0) ile belge şeması sürümü (2) ayrı; en az okuyucu sürümü başlıkta | `FILE-02`. Yazıcı `minReaderMinor`'ı yalnız gerçekten kullandığı yeni özelliğe göre yükseltir; 2.0 okuyucusu daha yeni bir yazıcının yalnız 2.0 özellikleri kullanan dosyasını açar |
| En az yazıcı | Ayrı alan yok | 2.0 okuyucusu anlamadığı hiçbir şeyi açmaz, bu yüzden açabildiğini kayıpsız yazar; eski istemci yeni bir anlık görüntüyü veri kaybederek yazamaz (`SYNC-14`) |
| Sıkıştırma | `codec = 0` (yok); başka değer `unknown_codec` | `FILE-10/11`: codec seçimi ölçümle ayrı karar. Özet sıkıştırılmış baytları kapsar: bozuk veri çözücüye hiç girmez |

**Bilinmeyen zorunlu uzantı: açık hata, salt okunur açılış değil** (`FILE-02`'nin iki seçeneğinden biri). Zorunlu uzantı içeriğin anlamını değiştirir (yeni bir nesne türü, şifreleme). Onu bilmeden açmak çizimi eksik ya da yanlış gösterir; salt okunur olsa da kullanıcı yanlış bir çizime bakar ve ondan ölçü alabilir (`DOM-06`: tanınmayan kritik geometri için sessiz düşürme yok). Hata uzantının adını ve “KentOS'u güncelleyin”i söyler. İsteğe bağlı (atlanabilir) uzantılar 2.0'da tanımlı değildir; gelirlerse “korunur ya da raporlanır” kuralıyla ve en az yazıcı sürümüyle gelirler.

### KentOS CBOR profili 1

- **İzin verilenler:** işaretsiz ve negatif tam sayı, bayt dizgisi (yalnız şemanın bayt istediği yerde), metin, dizi, metin anahtarlı harita, `false`, `true`, `null` ve binary64. Belirsiz uzunluk, etiket, float16/32, undefined ve diğer basit değerler yasaktır.
- **Belirlenimli:** en kısa tam sayı ve uzunluk; anahtarlar RFC 8949 §4.2.1 sırasında (kodlanmış bayta göre: önce kısa); yinelenen anahtar hata; yok olan değer yazılmaz; dosyada zaman damgası ya da rastgele sayı yoktur. Aynı çizim her yazıcıda aynı baytları verir. Bu, testin de temelidir: Rust, bağımsız Python yazıcısının yazdığı dosyaları bayt bayt aynı yazar.
- **Okuyucu katıdır:** profile uymayan yük geçerli CBOR olsa da reddedilir. Böylece “belirlenimli” iddiası okunan her dosya için doğrudur ve iki okuyucu (Rust, Python) aynı dosyada aynı kararı verir.
- **Sayılar (`FILE-06`, `NUM-01`):** float her zaman binary64'tür, lossless kısaltma yapılmaz (`1.0` de 8 bayt). NaN ve ±∞ hiçbir yerde yazılamaz (geometri sonludur; JSON istemcileri taşıyamaz). −0 yazılabilir ve bitiyle korunur. Şemalı float alanına tam sayı, tam sayı alanına float yazılamaz. Kesin ondalık ve hisse değeri float olmaz; şemada bugün yoktur, geldiğinde sözleşmedeki metin biçimleriyle (`DecimalString`, `ShareValue`) gelir.
- **Etiket yok:** ADR 0014 UUID etiketini (37) önermişti. Kullanılmadı: şema `uid`'in 16 baytlık UUID olduğunu zaten söyler; etiket her nesneye 2 bayt ve profile “etiket nerede geçerli” kuralı ekler; genel CBOR araçları etiketsiz dosyayı da çözer.
- **Sınırlar (`FILE-22`):** yük 2³⁰ bayt; iç içe derinlik 64; dizi öğesi ve harita çifti 2²⁴; dizgi 2²⁴ bayt. Beyan edilen her uzunluk önce sınırla, sonra yükte kalan baytla karşılaştırılır; okuyucu bir liste için en çok 4096 öğelik yer ayırır, gerisi okundukça büyür. Böylece 5 baytlık bir başlık gigabaytlık ayırma yaptıramaz.
- **Kendi yazımımız:** yalnız anlık görüntünün gerektirdiği türler (spec §5). Yazıcı ve okuyucu `crates/shared/kcad/src/cbor.rs`'te, yaklaşık 600 satırdır.

### Belge şeması 2

- **Kök** `{format, version, document}`: kodlanmış sıraya göre `format` ve `version` `document`'ten önce gelir, okuyucu belgeye girmeden şema sürümünü bilir.
- **Anahtar adları v1 sözleşmesinin adlarıdır** (`layerId`, `pts`, `bulges` …); dosya kendini anlatır, Python okuyucusu ve spesifikasyon okunaklı kalır. Nokta `[x, y]`, sınırlar `[minX, minY, maxX, maxY]` dizisidir.
- **Nesne tek anahtarlı haritadır:** `{"polygon": {…}}`. Tür önce okunur, alanlar türe göre denetlenir; ayrı `kind` anahtarı da gerekmez (nesne başına 4 bayt az). Tür sürümü ayrı yazılmaz; şema sürümü hepsini sürümler.
- **Katı:** bilinmeyen alan, bilinmeyen tür, eksik zorunlu alan, yanlış tür ve aralık dışı değer açık hatadır (`DOM-06`). v1 okuyucusu bilmediğini sessizce atıyordu; v2 hiçbir şeyi sessizce atmaz.
- **Belge kuralları** (katman başvurusu, bilinen SRID, köşe sayıları) dosya biçiminin değil çizimin kurallarıdır: uygulamaların okuyucuları v1'deki gibi denetler (spec §6.10). Kodek biçimi, tipleri ve kimlikleri denetler.

### Kimlik (ADR 0014, dilim 4)

- Sözleşmede `EntityId` ve `ProjectId` (16 bayt; JSON'da ve telde küçük harfli, tireli metin; okurken başka yazım reddedilir) ve `DocumentSnapshotV2 { …, uids, projectId?, migratedFrom? }` var (`crates/shared/contracts/src/{identity,document}.rs`). `DocumentSnapshotV2` dosyanın değil sözleşmenin JSON biçimidir: tarayıcı ile WASM modülü onu alıp verir.
- Dosyada her nesnenin `uid`'i 16 bayttır, dosyada **benzersiz** ve boş olmayan. Çalışma yuvası yazılmaz (`FILE-05`): okuyucu nesnelere dosya sırasıyla 1, 2, 3 … verir.
- v1 açılınca kimlikler (UUIDv5), projenin türetilen kimliği ve **göç kaynağı** `{format: kentos.document, version: 1, sourceSha256}` belgeye girer (`migrate_v1`; web'de biçim işçisinin `v1Identities`'i). Kaynak sonraki bütün v2 kayıtlarında korunur: kimliklerin nereden geldiğini söyler. Nesnelerin eski yerel kimlikleri ayrıca yazılmaz; aynı v1 dosyasından türetilen kimlikler zaten dosyadaki `uid`'lerdir.
- Web belgesi (`CadDocument.projectId`, `migratedFrom`) ve masaüstü belgesi (`kentos_domain::Document::project_id()`, `migrated_from()`) ikisini de tutar; yalnız bütün çizim değişince (`replaceWith`, `from_snapshot*`) değişir, düzenleme sayılmaz.
- **Yeni proje kimlik almaz** (2.0'da). Bulut projesinin kimliği sunucunundur (ADR 0015); yerel yeni projeye kimlik vermek bulut bağlantısı kararıyla birlikte gelir.
- Sonuç: bir v2 dosyası her nesnenin kimliğini kaydetme ve açma boyunca, düzenlenmiş olsa da, iki platformda korur. v1'in “düzenlenip kaydedilen dosya yeni kimlik alır” sınırı kalktı.

### Kod yerleşimi

| Parça | Yer | Not |
|---|---|---|
| Kodek | `crates/shared/kcad` (`kentos-kcad`) | Kap, profil, şema, koklama, `encode_verified`. Bağımlılıkları sözleşmeler, `serde`, `serde_json` (opak kısımlar `Value`), `sha2`: `Cargo.lock`'a yeni paket gelmedi. `scripts/arch/deps.mjs`'te `shared` grubundadır |
| Neden ayrı sandık | `crates/shared/formats` modülü değil | Proje dosyası değişim biçimi değildir (ADR 0009'u etkilemez); masaüstü DXF ve geometri çekirdeğini çekmeden yalnız bunu alır |
| Tarayıcı | `crates/wasm/formats-wasm`: `encodeKcad`, `decodeKcad` (`FORMATS_VERSION` 5) | Biçim modülü zaten tembel yüklenir ve işçide çalışır (ADR 0014'ün gerekçesi) |
| Masaüstü | `apps/desktop/src/document.rs` | Yerel olarak `kentos-kcad` |
| Geliştirici aracı | `cargo run -q -p kentos-kcad --bin kcad -- inspect | validate | sniff | migrate` | `kentosd` alt komutu yapılmadı: sunucu yığınını derlemeden çalışır; `migrate` var olan dosyanın üzerine asla yazmaz ve yalnız doğrulanmış baytları yazar |
| Bağımsız okuyucu | `tools/kcad/kcad.py` | Yalnız Python standart kütüphanesi: `inspect`, `validate`, `dump`, `sniff` |
| Bağımsız yazıcı | `scripts/fixtures/kcad_v2_reference.py` | Örnek dosyaları elle yazılmış çizimlerden üretir, `--check` diskle karşılaştırır ve okuyucuyla `expected.json`'a göre okur |

**Biçim modülünün boyu** (`--profile wasm`, wasm-bindgen 0.2.128; brotli 11 ve gzip 9 Node'un zlib'iyle): 935 464 → 1 263 396 bayt; brotli 255 464 → 309 977 (+54,5 KB), gzip 326 380 → 411 363. Yazma yaklaşık +26 KB, okuma +28 KB brotli. Modül başlangıçta yüklenmez; artık ilk kayıtta da yüklenir.

### Kaydet ve Farklı kaydet (`FILE-14..18`, `FILE-21`)

- **Kaydet her zaman v2 yazar** (`application/octet-stream`). Yalnız dosyanın gerçekten yazıldığı sürüm kaydedilmiş sayılır; kayıt sürerken yapılan değişiklik kirli kalır (`markSaved(revision)`, değişmedi). İndirme yedeği de v2 indirir ve çizimi kaydedilmemiş sayar (değişmedi).
- **v1'den açılan çizim salt okunur uyumluluk okuyucusundan gelir:** Kaydet eski dosyaya yazmaz, v2 dosyasının yerini sorar (önerilen ad dosyanın adıdır). Açılışta bunu söyleyen bir bilgi satırı çıkar. Kullanıcı sorulan pencerede eski dosyayı seçerse o dosya v2 olur: bu onun açık kararıdır ve yazılan baytlar önce doğrulanmıştır. Sahibin kuralı (“Orijinali göç doğrulanmadan ezme”) böylece iki kez tutar: kendiliğinden ezilmez, ezilirse doğrulanmış baytlarla ezilir.
- **Farklı kaydet v1 dışa aktarımı sunmaz.** Web her açılışta güncel uygulamayı yükler, masaüstü v2 okur; v1'e yazmak kalıcı kimlikleri, proje kimliğini ve kaynağı atmak demektir ve bir kayıp raporu gerektirirdi. Gereksinim çıkarsa ayrı bir “Dışa aktar → KentOS v1” komutu olarak eklenir (Kaydet'in yerine geçmez).
- **Doğrulama:** masaüstü `encode_verified` ile yazar: baytlar çözülür ve çizimle bit bit (JSON metniyle, −0 ayrı) karşılaştırılır; sonra aynı dizinde yeni bir geçici dosyaya yazılır, `fsync` edilir, geri okunup karşılaştırılır, tek adımda hedefin üzerine taşınır ve dizin `fsync` edilir (`FILE-16`). Herhangi bir adım bozulursa önceki dosya yerinde kalır, geçici dosya silinir. Web'de biçim işçisi aynı işi uçtan uca yapar: çizim sözleşmeye indirgenir, JSON'a yazılır (−0 korunur), kodlanır, çözülür ve gönderilen çizimle `Object.is` ile karşılaştırılır; yalnız tutan baytlar sayfaya döner.
- **Sessiz düşürme yok:** v1 okuyucusu bilmediği alanları nesnede tutuyordu (örn. `"note"`). v2 onları taşıyamaz; web kaydı onları yazmaz ve hangilerini yazmadığını uyarı olarak söyler (`polyline.note`). Masaüstünün v1 okuyucusu (serde) bu alanları zaten okumuyordu.

### Worker kararı (`FILE-15`)

- **Web'de kodlama, çözme ve doğrulama biçim işçisindedir.** Sayfa kayıtta çizimin sığ bir kopyasını yapar ve onu işçiye yollar (yapılandırılmış kopya, aynı turda: kopya o anın sürümüdür); açılışta çözülmüş çizimi yapılandırılmış kopyayla alır, v1'deki gibi denetler ve belgeye koyar. Sayfada dev bir JSON metni kurulmaz; v1 kaydının nesne başına `structuredClone` ve `JSON.stringify`'ı kalktı.
- **Ölçüm** (100 000 alan × 20 köşe, 3 öznitelik ve etiket; i5-11300H):
  - Boy: KCAD v2 48 274 445 bayt; aynı çizim v1 JSON 98 042 368 bayt.
  - Masaüstü (native, `--release`): yazma 97 ms, doğrulamalı yazma ~650 ms, okuma ~197 ms; v1 JSON yazma 148 ms, okuma 652 ms. `cargo test --release -p kentos-kcad --test measure -- --ignored --nocapture`.
  - Web (Node, aynı WASM ve `io/kcad.ts`): işçide kayıt ~5,5 s, açma ~3 s. Darboğaz JSON sınırıdır: JSON'un WASM'da ayrıştırılması ~2,2 s, çözülen çizimin JSON'a yazılıp okunması ~1,4 s, yapılandırılmış kopya ~1,2 s. Sayfa donmaz ama büyük çizimde bekleme uzundur. 2 120 nesnelik duman çiziminde fark hissedilmez.
- **Kalan:** sayı yoğun veride JSON yerine tipli dizili (Float64Array) bir sınır; `FILE-24`'ün tam ölçümüyle birlikte.

### Tür koklama (`FILE-03`)

- `kentos_kcad::sniff` ve web'de aynı kuralın küçük bir eşi (`io/kcad.ts` `sniffDrawing`: ilk baytlar): `kcad`, `kcad-damaged`, `json`, `empty`, `foreign`. Sayfa v2'yi işçiye, v1 JSON'u eski okuyucusuna yollar; boş ve yabancı dosya nedenini söyler (DXF “İçe aktar ile açılır”). Web eşi aynı `expected.json`'la sınanır. Masaüstü aynı kararı Rust'ta verir.

### Doğrulama

- **Örnek dosyalar** (`fixtures/kcad/v2`): 4 geçerli (en küçük, bütün türler ve uç değerler, v1'den göç, daha yeni yazıcının 2.0 dosyası), 53 bozuk (her biri tek kuralı bozar: imza, satır sonu, eksik, fazla, özet, sürüm, bayrak, kodlama, kodek, uzantı, boy; yinelenen ve sırasız anahtar, derinlik, uzunluk, en kısa olmayan tam sayı ve uzunluk, dar float, NaN, sonsuz, belirsiz uzunluk, etiket, undefined, ayrılmış ek bilgi, UTF-8, metin olmayan anahtar; şema: biçim, sürüm, bilinmeyen ve eksik alan, tür, kimlik boyu, nil ve yinelenen kimlik, bilinmeyen tür, iki anahtarlı nesne, üç sayılı nokta, null çizici, aralık dışı tam sayı, bilinmeyen değer, bozuk göç kaynağı), v1 JSON ve DXF (koklama), 2 değişim dosyası.
- **Beklenenler elle yazıldı** (`expected.json`, spesifikasyondan); geçerli dosyaları bağımsız Python yazıcısı elle yazılmış çizimlerden üretti. Rust aynı çizimleri **bayt bayt aynı** yazar; Rust, WASM ve Python okuyucusu her dosyada aynı hata kodunu ya da aynı çizimi verir (CLAUDE.md §9.4).
- **v1 göçünün referansı:** `migrated.json` Python'da v1 örneğinden ve ADR 0014'ün bağımsız kimlik referansından türetildi. Web'in ve masaüstünün kendi Kaydet'i v1 örneğini bu dosyanın baytlarına yazar.
- **Değişim (TODOS.md §9 kabulü):** masaüstünün kaydettiği çizim web'de açılır, düzenlenir, kaydedilir (`exchange/web-edited.kcad`); masaüstü onu okur, web'in düzenlemelerini ve başka hiçbir değişiklik olmadığını bulur, kendi düzenlemelerini yapar, kaydeder (`exchange/desktop-edited.kcad`); web onu aynı biçimde okur. “Kayıpsız” bit bittir: dosya, önceki çizime beklenen düzenlemeler elle uygulanmış hâliyle karşılaştırılır, kimlikler dahil. Dosyaları iki platformun kendi testleri `KENTOS_WRITE_EXCHANGE=1` ile yazar; sonra karşılaştırır.
- **Güvenilmeyen girdi:** rastgele baytlar, geçerli bir kabın içinde rastgele ve CBOR'a benzer yükler, her örnek dosyanın her öneki ve yükünün her baytının değiştirilmesi: hiçbiri çökmez, hiçbir rastgele girdi çizim diye okunmaz. 600 rastgele çizim (−0, alt normal ve en büyük sayılar, Unicode, derin katman ağacı, opak değerler) bit bit geri gelir ve ikinci yazımda aynı baytları verir.

## Sonuçlar

- CLAUDE.md §4.8'in “Mevcut `.kcad` JSON `DocumentSnapshotV1`'dir” cümlesi artık doğru değildir; önerilen metin teslim raporundadır. ADR 0011'in “binary sürüm kodda çalışana kadar” sonucu doldu.
- **Ertelenenler:** sıkıştırma kodeki ve ölçümü (`FILE-10/11`), gömülü varlıklar (`FILE-09`), akışlı/bloklu okuma (`FILE-10`), yerel kurtarma kopyası (`FILE-19`), aşamalı ve iptal edilebilir açılış (`FILE-20`; bugün açılış bütün dosya denetlenince tek adımda olur, yarım dosya belgeye girmez), tam ölçüm (`FILE-24`), bulut dosya revizyonları (`SYNC-02..06`), isteğe bağlı uzantılar ve en az yazıcı sürümü, tipli öznitelik ve kesin ondalık alanları (`DOM-09/10`), web'in JSON dışı sınırı, v1 dışa aktarımı.
- **Bilinen sınırlar:**
  - JavaScript tam sayı değerli float'ı tam sayıdan ayıramaz: opak kısımdaki `2.0` web'den geçince `2` yazılır; 2⁵³'ü aşan opak tam sayılar yuvarlanır (v1'de de böyleydi). Stil motoru ikisini aynı okur. Şemalı alanlarda fark yoktur.
  - Çalışma modu ve çizim yazı tipi olmayan bir dosyayı web açıp kaydederse varsayılanları (`hybrid`, `barlow`) açıkça yazar (v1'deki gibi); masaüstü yokluğu korur.
  - Web kaydı artık biçim modülüne bağlıdır: modül hiç yüklenmemişken ve ağ yokken kayıt nedenini söyleyerek düşer, çizim kirli kalır (v1'in JSON kaydı modülsüz çalışıyordu).
  - Masaüstünün yeni dosyası sistemin varsayılan izinleriyle oluşur; üzerine yazılan dosyanın izinleri kopyalanmaz.
  - Masaüstünde doğrulamalı yazma büyük çizimde doğrulamasızın yaklaşık altı katıdır (JSON karşılaştırması); arka planda çalışır.
