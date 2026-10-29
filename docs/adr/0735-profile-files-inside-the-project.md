# ADR 0735: Veri profillerinin ve eşlemelerinin projenin kendi dosyasında taşınması

- **Durum:** kabul edildi (2026-10-29). `plan/gis.md` CBS-34'ün beşinci dilimi. Kapsamı ben belirledim.
- **Kalan iş:** ADR 0558 ve 0647'den kalan “profil dosyalarının `.kcad`'de taşınması”.
- **Bağlam belgesi:**
  - ADR 0541 (veri profili, `cad.profiles.apply`, `.check`);
  - ADR 0558 (kurumun profili ve eşlemesi, `profiller` klasörü, `cad.profiles.import`, `.list`, `.migrate`);
  - ADR 0647 (eşlemeyle göç);
  - ADR 0596 (Paketle);
  - ADR 0251 (pafta kitabının projenin dosyasında opak taşınması);
  - ADR 0265 §2 (yeni şema eklemenin kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 131 (opt-in; yalnız profil ya da eşleme taşıyan projede).
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - adres kodunun UAVT gibi bir dış kayıtla karşılaştırılması;
  - ifraz, tevhit, hisse ve PlanGML (mevzuat işi, sahibin kararıyla sona kalır);
  - Paketle'nin manifestinde çizimin taşıdığı profillerin ayrıca sayılması (bugün yalnız klasördekiler sayılır;
    çizimin taşıdıkları `.kcad`'in içinde zaten gider);
  - gerçek kullanıcılarla günlük kullanım kabulü.

## Bağlam

Kurumun profili ve eşlemesi (ADR 0558) çizimin dosyasının yanındaki `profiller` klasöründe dosyadır. Bu üç yerde
yetmiyor:

- **Henüz kaydedilmemiş çizim:** dosyası yok, yanında klasör de yok. İçe aktarma “önce çizimi kaydedin” diye reddeder.
- **Bulut projesi:** veritabanı ya da dosya projesinin yanında klasör yoktur. Kurumun profili ancak her cihazda ayrı
  bir klasörde tutulabilir.
- **Tek dosya paylaşmak:** e-postayla gönderilen `.kcad` profilini götürmez. Paketle (ADR 0596) klasörü kopyalar, ama
  kullanıcı çoğu zaman yalnız çizimi gönderir.

Projenin taşınabilirliği (CBS-36) de profilin proje ile birlikte gitmesini ister.

## Karar

### §1 Ayarların `profileFiles`'ı

Projenin ayarlarına isteğe bağlı bir liste eklenir: `ProjectProfileFile { kind, name, text }`. Alanlar:

- `kind`: `profile` ya da `mapping`;
- `name`: profilin ya da eşlemenin adı. Klasördeki dosyanın adı da budur. Küçük harf, rakam ve tire, 1–64 karakter.
- `text`: belgenin kendisi, JSON metni olarak. Bu, içe aktarmanın okuduğu belgenin girintili JSON'udur; klasöre
  yazılan dosyanın aynısıdır.

Listenin kuralları sözleşmede tek yerdedir (`kentos_contracts::profile_files_problem`):

- metin boş ya da yalnız boşluk olamaz; UTF-8'de en çok 4 MiB olur;
- en çok 256 öğe olur;
- liste adların sırasıyla tutulur; aynı adda önce profil gelir;
- aynı ad ve tür bir kez geçer.

`sanitized_profile_files` uymayanı düşürüp sıraya koyar. `with_profile_file` bir öğeyi yerine koyar ya da sırasına
ekler.

Metin opaktır. Dosya biçimi `kentos.data-profile` ya da `kentos.profile-mapping` belgesini okumaz. Bunun üç nedeni var:

- profil belgesinin kendi sürümü (bugün 1) değişince `.kcad`'in şeması değişmek zorunda kalmaz;
- bağımsız Python okuyucusu profilin bütün şemasını bilmek zorunda kalmaz;
- pafta kitabı (ADR 0251) aynı yolu izler.

Belgeyi profil komutları okur ve denetler. Okunamayan bir kopya düşmez; listede nedeniyle görünür.

### §2 Hangi kopya okunur

Bir projenin profilleri iki yerdedir: çizimin taşıdıkları ve klasördeki dosyalar. Uygulama katmanında ikisini birlikte
tutan yapı `profile_files::Shelf`'tir. Bir ad ve tür iki yerde de varsa **çizimin içindeki okunur**. Profil adıyla şu
sırayla aranır:

1. KentOS'un yerleşik profili (`tucbs-temel`);
2. çizimin kopyası;
3. klasördeki dosya.

Eşleme aynı sırayla aranır; yerleşik eşleme yoktur.

Çizimin kopyası okunamıyorsa klasördekine geçilmez; neden söylenir. Okunan kopya, adını ve türünü klasördeki dosya gibi
tutmalıdır. Kendi adı farklı olan profil ya da eşleme diye taşınan profil nedeniyle reddedilir.

`cad.profiles.list` şunları sırayla verir:

- yerleşik profili;
- çizimin taşıdıklarını, adlarıyla (`source: drawing`, yolsuz);
- klasördekileri, adlarıyla (`source: project`).

Çizimin de taşıdığı ad ve türdeki klasör dosyası `shadowed: true` ile listelenir. Okunmadığı için Veri profili
penceresinin seçicilerinde görünmez.

`apply`, `check` ve `migrate`'in bilinmeyen ad retleri çizimin taşıdıklarını da sayar. Örnek: “… KentOS'un profili:
tucbs-temel; çizimin içindeki profiller: park-bahce.” Çizim hiçbir şey taşımıyorsa eski iletiler değişmez.

### §3 `cad.profiles.import`'un `store`'u

İçe aktarma artık nereye yazacağını sorar:

- **`folder`:** varsayılandır. ADR 0558'in davranışıdır, API geriye uyumludur.
- **`drawing`:** çizimin ayarlarına yazar. Çizimin dosyası gerekmez; bulut projesinde ve kaydedilmemiş çizimde çalışır.
  Proje ayarı olduğu için geri alma adımı değildir; `cad.settings.set` gibi çizimi değişmiş yapar, sürümü artırır.

Değişmeyenler: sürüm kuralları (`profile_downgrade`, `profile_replaced`), `overwrite`, KentOS'un adının alınmaması ve
eşlemenin profiliyle denetlenmesi. Eşlemenin profili çizimde, klasörde ya da KentOS'ta aranır (§2).

Çizime yazarken değişenler:

- aynı sürüm ya da aynı eşleme `overwrite` olmadan `profile_exists` ile reddedilir; klasördeki karşılığı `file_exists`;
- ileti “Çizimin içindeki …” diye başlar; okunamayan kopyanın yerine yazmak “okunamayan kopyası” der;
- metin 4 MiB'ı ya da liste 256'yı aşacaksa `invalid_profile` ile reddedilir ve profiller klasörü önerilir.

Bir ad iki yerde birden olacaksa `profile_shadowed` uyarısı verilir:

- çizime alınan profilin klasörde de dosyası varsa, komutların çizimdekini okuyacağı söylenir;
- klasöre alınan profilin çizimde de kopyası varsa, klasördeki dosyanın okunmayacağı söylenir.

Girdiye `expectedRevision` eklendi. Çıktının `store`'u her zaman yazılır; `path` yalnız klasörde olur. Kataloğun
etkisi `query`'den `document`'e geçti.

Komut her iki yerde de komut kaydına kendi satırını yazar (`call::recorded`). Kayıt komutu yeniden oynatabilir.

### §4 `cad.profiles.remove`

Yeni komut, çizimin taşıdığı bir profili ya da eşlemeyi çıkarır. Girdisi: `kind`, `name`, isteğe bağlı `drawing` ve
`expectedRevision`. Yalnız çizimdeki kopyaya dokunur. Klasördeki dosya kullanıcınındır ve bu komutla silinmez. Ret bunu
dosyanın yoluyla söyler: “… Klasördeki … bu komutla silinmez; dosyayı klasörden kaldırın.”

Çıktı:

- `bytes`: çıkan metnin uzunluğu;
- `folderPath`: klasörde aynı adlı dosya varsa, bundan sonra okunacak olan dosyanın yolu.

Profil çıkıyorsa ve klasörde aynı adlı dosyası yoksa, onu adlandıran her eşleme `profile_still_named` ile söylenir:
önce çizimdekiler, sonra klasördekiler, adlarıyla. Bu eşlemeler bundan sonra profille denetlenemez.

Retler `unknown_profile`, `unknown_mapping`, `invalid_revision`, `revision_conflict`. Komut geri alma adımı değildir,
proje ayarıdır. MCP'de yıkıcı araçtır.

### §5 `.kcad` şema 131

Ayarlar `profileFiles`'ı `linearEvents`'ten sonra, kanonik anahtar sırasıyla, sözleşmenin JSON'u olarak yazar.

- Yazıcı 131'i yalnız liste boş değilken seçer; öteki bütün dosyaların baytları değişmez.
- Okuyucu listeyi sözleşmenin kurallarıyla denetler. Boş liste `bad_value` olur.
- Şema 130 ve öncesinde `profileFiles` bilinmeyen alandır (`unknown_field`).
- Desteklenmeyen sürüm örneği `broken/schema-version-132.kcad` oldu.

Bağımsız okuyucu `tools/kcad/kcad.py` aynı kuralı KentOS kodu olmadan okur. Boşluğun tanımı orada Unicode White_Space
kümesiyle açıkça yazılıdır.

### §6 Sunucu

Sunucu profil komutlarını çalıştırmaz. Ama veritabanı projesinin ayarlarını saklar ve projenin `.kcad` görüntüsünü
yazar. Kurala uymayan bir liste o görüntüyü yazılamaz kılardı. Bu yüzden proje oluşturma ve `project.changes`'in ayarları
listeyi aynı kuralla denetler (`check_profile_files`). Yol `settings.profileFiles` ve `project.settings.profileFiles`'tır.
Bulut eşitlemesi listeyi ayarlarla birlikte taşır.

### §7 Masaüstü

Veri profili penceresi:

- **Nereye:** “Çizimin içine” ya da “Profiller klasörüne”. Varsayılan çizimin içidir. İçe aktar… çizimin dosyası
  olmadan da çalışır.
- **Seçiciler:** profiller “(çizimde, sürüm n)” ya da “(klasörde, sürüm n)” diye yazılır, eşlemeler “(çizimde)”.
  Gölgedeki klasör dosyaları seçicide yoktur.
- **Kaldırma:** çizim bir şey taşıyorsa “Profili çizimden kaldır” ve “Eşlemeyi çizimden kaldır” görünür. Düğme yalnız
  seçili profil ya da eşleme çizimdeyse etkindir. Kaldırma geri alınamaz; önce sorulur. Soru, klasörde aynı adlı dosya
  varsa onun okunacağını, yoksa yeniden içe aktarmak gerektiğini söyler.
- **Profilin eşlemeleri:** kaldırılan profilin başka kopyası kalmıyorsa, onu adlandıran çizimdeki eşlemeler soruda
  adlarıyla söylenir ve birlikte kaldırılır. Kalsalardı pencerede seçilemez, kaldırılamazlardı: eşleme seçicisi yalnız
  seçili profilin eşlemelerini gösterir. Komut tek tek çıkarır; pencere önce eşlemeleri, sonra profili çıkarır. Python
  ve MCP'de profil tek başına çıkarılabilir; o zaman `profile_still_named` uyarısı kalan eşlemeleri söyler.
- **Onay kutusu:** “Aynı adlının üzerine yaz”. İki yerde de geçerli olduğu için “dosya” demez.

### §8 Paketle

Değişmez. Çizimin taşıdığı profiller `.kcad`'in içinde gider. `profiller/` klasörünün dosyaları ADR 0558 §10'daki gibi
bayt bayt kopyalanır.

## Doğrulama

- **`.kcad`:**
  - `scripts/fixtures/kcad_v2_reference.py` `profile-files.kcad`'i yazar: bir profil, aynı adda bir eşleme ve bir başka
    eşleme;
  - on bozuk örnek: şema 130, boş liste, bilinmeyen alan, tür, ad, uzun ad, boş metin, iki kez, ters sıra, eşleme önce;
  - `--check` 1343 dosyanın hepsini bağımsız okuyucuyla aynı bulur;
  - `cargo test -p kentos-kcad` 234 testle geçer: taşıma matrisi, örnekler ve şemaların kuralı.
- **Ortak durumlar:** `scripts/fixtures/profile_files_cases.py` KentOS kodu olmadan yazar:
  - `cad.profiles.import` 64 durum: 24'ü çizimin içine, gölge uyarıları, beklenen sürüm, 256 sınırı;
  - `cad.profiles.list` 38 durum: 12'si çizimin taşıdıklarıyla;
  - `cad.profiles.migrate` 56 durum: 6'sı çizimdeki profil ve eşlemeyle;
  - yeni `cad.profiles.remove` 24 durum.

  `KENTOS_CASES_ONLY=cad.profiles cargo test -p kentos-native-application --test all every_command_case` profil
  komutlarının 274 durumunu oynatır. Yetenek testi yeni kodları (`profile_exists`, `profile_shadowed`,
  `profile_still_named`) durumlarda bulur.
- **Sözleşme ve sunucu:** `kentos-contracts`'ın birim testi listenin kurallarını, `kentos-application`'ın testi sunucunun
  reddini sınar.
- **Masaüstü:** `data_profile_tests` şunları sınar:
  - dosyası olmayan çizime profil ve eşleme alınır, ikisi seçilir, çizim değişmiş olur;
  - kaldırma önce sorulur; Vazgeç bir şey çıkarmaz;
  - Kaldır profili ve onu adlandıran çizimdeki eşlemeyi birlikte çıkarır;
  - yalnız eşleme çıkarılınca profil kalır ve seçili kalır;
  - klasöre almak dosyasız çizimde hiçbir şeyi değiştirmez.
- **Python ve MCP:** SDK `scripts/python/sdk.py` ile yeniden üretildi; `pnpm py:test` 375 testle geçti (ortak durumlar
  ve `.kcad` taşıma matrisi de). `cad.profiles.remove` MCP'de yıkıcı araçtır; `cargo test -p kentos-mcp` geçti.
  Manifest, komut geçmişinin başlığı ve CBS kapsam matrisi (301 komut) yeniden yazıldı.
- **Resimler:** `KENTOS_SHOTS_ONLY=profil-ice-aktar,profil-cizimden-kaldir,profil-kaldirildi cargo test -p
  kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-profil-*`). 1440×900 ve 1100×650, iki
  temada.

## Ölçümler

Release, bu makinede (`cargo test --release -p kentos-native-application --test all profile_files::carried_timing --
--ignored --nocapture`). Çizim 127 profil ve 127 eşleme taşır, toplam 673 KB metin. Her profil park ve bahçelerin üç
sürümlü profilinin bir kopyası, her eşleme kendi profilini adlandırır.

| İş | Süre |
|---|---|
| `cad.profiles.list`: 254 kopyanın hepsi okunur, her eşleme profiliyle denetlenir | 7,4 ms |
| profili adıyla bulmak (`find_profile`) | 26 µs |
| çizime bir profil daha almak (255.) | 0,84 ms |
| `.kcad` yazmak (geri okuyarak doğrulamayla) | 4,2 ms |
| `.kcad` okumak | 0,74 ms |

Dosya 687 KB'tır: metin opak olduğu için olduğu gibi yazılır. Liste her açılışta metinleri yeniden okur; 254 kopyada bu
bir karenin altında kalır, önbellek gerekmedi.
