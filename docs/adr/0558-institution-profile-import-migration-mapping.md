# ADR 0558: Kurumun kendi veri profili: içe alma, sürümler ve göç, kurum eşlemeleri

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-34'ün ikinci dilimi (ilk dilim ADR 0541). Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0541 (veri profilleri; `cad.profiles.apply` ve `.check`);
  - 0199 (katman alanları), 0430 (değer listeleri), 0405 (ilişkiler), 0414 (alanın kullanıldığı yerler: yeniden adlandırma);
  - 0020 (belge: katmanın adı ve bayrakları geri alma adımı değildir);
  - 0253 (komut protokolü: plan, kayıt, tek adım), 0556 (dosyayı geçici adla yazıp geri okuyarak yerine koyma);
  - 0540 (komut türleri `kentos-commands`'ta), 0244 (CAD işlem manifesti).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - profil dosyalarının `.kcad`'in içinde (şema numarası) taşınması ve Paketle'nin `profiller` klasörünü kopyalaması;
  - göçün eşlemeyle birlikte (kurumun adlarıyla yazılı veriyi taşımak) çalışması;
  - değer listesini bölmenin ve birleştirmenin `mapValues`'tan başka biçimleri (liste kimliği değiştirme);
  - adres normalizasyonu, ifraz, tevhit, hisse ve PlanGML (ADR 0541'deki gibi).

## Bağlam

ADR 0541 profili veri yaptı (`catalog/v1/profiles/tucbs-temel.json`) ama yalnız KentOS'un profilini bildi: bir belediyenin
kendi profili (kendi alan adları, kodları, katman yolları) projeye giremiyor, profilin ikinci sürümü çıkınca eski sürümdeki
veriyi taşıyacak yol yok, kurumun “parsel” katmanını “Parseller”, “ParselNo”yu “PARSEL_NO” diye yazması profille
karşılaştırmayı katman yolunun ilk adımında düşürüyordu. Üç şey gerekiyordu:
- kurumun getirdiği profilin ve eşlemenin projeye doğrulanarak alınması, reddi anlaşılır;
- profilin sürümleri ve sürümler arasında verinin kendi adımlarıyla, geri alınabilir, kaybı söyleyen taşınması;
- kurumun adlarıyla profilin adlarının eşlemesi: denetle ve uygula eşlemeden okur, hiçbir ad kendiliğinden yeniden
  adlandırılmaz.

## Karar

1. **Profil ve eşleme dosyadır, çizimin yanındaki `profiller` klasöründe durur.**
   - Profil `<ad>.profil.json`, eşleme `<ad>.eslem.json`; ad küçük harf, rakam ve tire, en çok 64 karakter; dosyanın içindeki
     ad dosyanın adıdır (farklıysa dosya okunmaz, nedeni söylenir).
   - Klasör çizimin dosyasının yanındadır; çizimin dosyası komutlara `drawing` ile verilir (ev sahibi başsız sunucuda, Python'da
     ve MCP'de çizimin kendi dosyasını doldurur, `with_own_file`; masaüstü açık çizimin yolunu verir). Dosyası olmayan çizim
     yalnız KentOS'un profilini görür, içe alma `invalid_file_path` der.
   - **Neden `.kcad`'in içinde değil:** `.kcad` şema numarası başka işlerle çakışır, stil kitaplığı bilmediği türü düşürür ve
     bir profil kurumun dosyasıdır: çizimden ayrı sürümlenir, birden çok projede kullanılır. Dosya bayt bayt kurumun verdiği
     biçimde (sözleşmenin yazımıyla, girintili JSON) durur; çizim dosyasının şeması değişmez. Paketle henüz `profiller`i
     kopyalamaz (sonraya kalan).
2. **Profilin sürümü `release`'tir, biçim sürümü `version` değil.**
   - `DataProfile.release` (varsayılan 1; 1'den küçük `invalid_profile`) ve `migrations`: bir sürümden sonrakine
     (`from`, `to = from + 1`, başlık, 1–64 adım) verinin taşınma adımları. Bir sürümden iki göç, profilin sürümünü aşan göç ve
     adımsız göç geçersizdir (`profile_problem` ad ve başlıktan hemen sonra `migrations_problem`'i çağırır, iletileri Türkçe).
   - `version` 1'dir; başkası `unsupported_profile_version` (biçim sürümü yoksa iletide “yok”).
   - Sürüm bir sayıdır; profil kendi sürümünün verisini nasıl okuduğunu söyler, çizim hangi sürümde olduğunu **saklamaz**:
     göçte `from` açıkça verilir (çizim dosyasının şeması değişmesin diye; Veri profilini denetle verinin hâlâ ayrıldığı yeri
     söyler).
3. **`cad.profiles.import`: getirilen profil ya da eşleme projeye alınır.**
   - Kaynak tam biri: JSON dosyası (`source`: mutlak yol ya da çizimin klasöründen), profilin kendisi (`profile`) ya da eşlemenin
     kendisi (`mapping`).
   - Biçimle tanınır: `kentos.data-profile` ya da `kentos.profile-mapping`; ikisi de değilse `unknown_profile_format`;
     `version` 1'den başkaysa `unsupported_profile_version`; JSON okunmaz, eksik alan ya da tutmayan profil `invalid_profile`;
     tutmayan eşleme `invalid_profile_mapping`.
   - Sıra: `profile_source_missing` (tek kaynak yok ya da birden çok), `invalid_file_path` (kaynak, sonra çizim),
     `file_not_found`, biçim ve sürüm, profil ya da eşleme geçerli mi, `profile_name_taken` (KentOS'un profilinin adı),
     `profile_downgrade`, `file_exists`, `file_unwritable`.
   - Aynı adlı profil dosyası varsa: daha yeni sürümdeyse `profile_downgrade`, aynı sürümdeyse `file_exists`, daha eski
     sürümdeyse değiştirilir ve söylenir (`profile_replaced`, eski sürümü iletide); `overwrite` her birinin üzerine yazar. Eşleme
     varsa `file_exists`, `overwrite` ile değiştirilir (`profile_replaced`).
   - Eşleme profiliyle denetlenir (KentOS'un ya da projenin); profili bulunamayan eşleme yazılır ve `mapping_unchecked` der.
   - Dosya geçici adla (`.kentos-yaziliyor`) yazılır, geri okunur, yerine konur (ADR 0556'nın yolu): bütün ya da hiç, klasör yoksa
     açılır, plan yazmaz. Çizimi değiştirmez, geri alma adımı yoktur.
4. **`cad.profiles.list`: yerleşik ve projenin profilleri, eşlemeler.** Salt okunur. Önce yerleşik profiller, sonra klasörün
   dosyaları adıyla (aynı adda önce profil). Satır: tür, kaynak (`builtin`, `project`), ad, başlık, sürüm, sınıf ve göç sayısı,
   eşlemede profili ve anan sınıf sayısı, yolu. Okunmayan ya da tutmayan dosya düşmez, `problem`le gelir (eşleme profiline
   uymuyorsa nedeni). Penceredeki profil seçici bunu kullanır.
5. **`cad.profiles.migrate`: verinin profilin sürümleri arasında taşınması.**
   - Girdi: profil (`profile` ya da yerleşik/projenin `name`'i), `from` (veri hangi sürümde), `to` (verilmezse profilin sürümü),
     `drawing`, `expectedRevision`. Reddi sırayla: `unknown_profile`, `invalid_profile`, `invalid_migration` (`from` 0 ya da `to`
     `from`'dan büyük değil ya da profilin sürümünü aşıyor), `migration_gap` (`from`'dan `to`'ya kesintisiz göç yolu yok),
     `invalid_revision`, `revision_conflict`, `layer_refused` (belge bir adımı reddetti: hiçbir şey yazılmaz).
   - Adımlar sırasıyla, her biri çizimin o anki hâline göre (öncekinin bıraktığı üzerinde):
     - `renameLayer`: katman yolundaki son ad değişir (Türkçe katlanmış adla bulunur); aynı yerde o ad varsa `layer_refused`;
       katman zaten yeni adıyladır ise sessiz atlanır.
     - `renameField`: alanın adı ve nesnelerin değerleri taşınır (`set_layer_fields`'ın yeniden adlandırması: ifadeler, formlar ve
       ilişkiler izler; bir ilişki ya da form alanı anıyorsa alanın kimliği eski adı olur, bağ bozulmaz). İkisi de varsa
       `layer_refused`.
     - `changeFieldType`: alan yeni tanımını alır; her değer yeni alanın kuralından geçer ve kanonik metniyle yazılır (liste
       etiketi kodu, `3` `3`, `020` `20`, boş boş); geçmeyen `onFail`'e göre (`keep` ya da `clear`) olduğu gibi kalır ya da
       boşaltılır ve değer değer raporlanır. Alanın bağlandığı değer listesi projede yoksa profilin aynı kimlikli listesi aynı
       adımda eklenir.
     - `mapValues`: eski değerler yenisi olur; birkaç eski bir yeniye birleşebilir; bir eski değer aynı katmandaki başka bir
       alanın değerine göre ikiye ayrılabilir (`whenField`, `whenEquals`; koşullu olan önce denenir); çok değerli alanda her kod
       ayrı, yinelenenler tek kalır.
     - `retireCode`: projenin değer listesinde kod `retired` olur (ADR 0418); `replaceWith` varsa o listeye bağlı alanların o
       kodlu değerleri başka kod olur (çok değerli alanın her kodu). Bağlı alanların listeden kopyaladığı değerler aynı adımda
       yazılır (`set_domains`).
   - Katmanı, alanı ya da değer listesi çizimde olmayan adım atlanır ve söylenir (`migration_step_skipped`); zaten uygulanmış
     adım sessiz atlanır (göç yinelenebilir); kilitli katmanın nesnelerine dokunan adım atlanır (`profile_layer_skipped`).
   - Hepsi **tek geri alma adımıdır** (“Profil göçü”: alanlar, değerler ve değer listeleri). **İstisna:** katmanın adı çizimin
     bayrağıdır ve hiçbir katman yeniden adlandırması geri alma adımına girmez (ADR 0020, `cad.layers.set`'in `rename`'i de
     öyle); geri alma alanları, değerleri ve listeleri geri verir, katman yeni adıyla kalır. Yalnız katman adı değişen göçün geri
     alınacak adımı yoktur.
   - Cevap: adım adım ne yapıldığı (`steps`: göç, sıra, tür, katman, uygulandı mı, nesne sayısı, not), yazılan nesne sayısı ve
     uymayan değerler (`losses`: göç, adım, katman, alan, `kept` ya da `cleared`, değer, nesnelerin kalıcı kimlikleri en çok 1000,
     sayı, ileti; değer sırasıyla). Uyarı `values_unfit` ilk değerleri sayar. Plan aynı cevabı yazmadan verir.
   - Göç yeni sürümün getirdiklerini eklemez: **önce göç, sonra Veri profilini uygula** (Uygula eksik alanları ekler; önce
     eklenseydi yeniden adlandırılacak alanla çakışırdı); denetle verinin hâlâ ayrıldığı yeri söyler.
6. **Kurum eşlemesi: kurumun adlarıyla profilin adları.**
   - `kentos.profile-mapping` 1: ad, başlık, profilin adı ve sınıflar; her sınıf profilin kimliğiyle: kurumun katman yolu
     (`layer`), alan adları (`fields`: profildeki ad → kurumdaki ad), kodlar (`codes`: profilin alanının kodları → kurumun
     kodları; alanın değer listesi olmalı).
   - Eşleme profille denetlenir (`mapping_problem`, iletileri ADR'nin iletileri): biçim ve sürüm, ad, başlık, profil, sınıflar profilde
     olmalı ve birer kez, alanlar sınıfta olmalı ve birer kez, kurumdaki adlar dolu ve boşluksuz ve sınıfın başka bir alanının adı
     olmayacak, kodlar listede olmalı ve birer kez, kurumdaki kod dolu, `;`'siz ve listenin başka kodunun adı olmayacak.
   - **Profil eşlemeden okunur** (`through`): eşlenen sınıfların katman yolları kurumun yolları; eşlenen alanlar kurumun adıyla
     (profilin adı alanın kararlı kimliği olur, ilişkiler ve formlar onu izler); kodları eşlenen alan profilin listesine
     bağlanmaz, değerleri eşlenmiş kopyadır (varsayılan da); kimlik alanı, kayıtlı alan, ilişkilerin alanları ve adres verisi
     alanları kurumun adlarıyla yazılır; eşlemenin anmadığı profilin olduğu gibi kalır. Okunan profil de `profile_problem`'den geçer.
   - **Hiçbir ad kendiliğinden değişmez.** Eşleme `cad.profiles.apply` ve `.check`'e `mapping` (eşlemenin kendisi) ya da
     `mappingName` (projenin dosyası, `drawing` ile) olarak verilir; verilmezse profil kendi adlarıyla okunur ve kurumun adları
     eksik görünür. Uygula kurumun adlarıyla bulunur ve ekler (yeni katman kurumun yoluyla, yeni alan kurumun adıyla açılır),
     Denetle kurumun adlarıyla söyler ve raporda `mapping`i yazar. Eşleme göçte kullanılmaz.
   - Reddi: `unknown_mapping` (adı verilen eşleme yok; çizimin dosyası yoksa nedeni söylenir), `invalid_profile_mapping`. Çizimin
     klasöründeki profil `name` olarak da verilir (`unknown_profile` iletisi klasörü söyler).
   - Değer listeleri eşlemeyle de projeye eklenir (kurumun alanı listeye bağlanmasa bile): liste profilindir, kod eşlemesi alanındır.
7. **Bir hata düzeltildi: nesnelerin değerleri alanın adının altındadır, anahtarının altında değil.** ADR 0541'in denetimi
   ve ilişki bağları özniteliği alanın anahtarıyla (kimlik, yoksa ad) okuyordu; `set_layer_fields` yeniden adlandırınca değerleri adın
   altına taşır, anahtar kimlik kalır. Okuma `LayerField.name` ile yapılır; ilişki bir anahtar tutar, değeri alanın adından okunur.
   ADR 0541'in tek durumu (alan adıyla değil anahtarıyla bulunur) değerleri adın altına alır.
8. **Masaüstü: Veri profili penceresi genişledi.**
   - Profil seçici (KentOS'un ve projenin profilleri), Eşleme seçici (profilin eşlemeleri; “Eşleme yok”), İçe aktar… (JSON dosyası,
     “Aynı adlı dosyanın üzerine yaz”), Verinin sürümü seçici (profil birden çok sürümlüyse), göçün planı (adım ve nesne sayıları,
     uymayan değerler) ve Veriyi taşı; göçten sonra uymayan değerler değer değer (ilk dördü, kalanı sayıyla) pencerede durur.
   - Bulgular ve Uygula eşlemeyle çalışır, raporun özeti eşlemeyi söyler. Çizimin dosyası yoksa İçe aktar… kapalıdır ve nedeni yazılıdır.
   - Pencere kayan gövdelidir (düğmeler görünür kalır). Komutların başlıkları komut geçmişinde.
9. **Python ve MCP.** `kentos.cad.profiles.import_`, `.list`, `.migrate` (tipleriyle, `.plan`, `.validate`); `apply` ve `check`'in
   `mapping`, `mapping_name`, `drawing`'i. MCP'de `cad.profiles.import` ve `cad.profiles.migrate` yıkıcıdır (`overwrite` yazar, göç
   değer yazar), `cad.profiles.list` ve `.check` salt okunurdur.

## Doğrulama

- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/profile_files_cases.py`,
  `profile_cases.py`; kurallar sözleşmeden ve bu ADR'den yeniden yazılır, `layer_field_cases.py`'nin `check`'iyle değer kuralı):
  `cad.profiles.import` 40, `.list` 26, `.migrate` 40 durum; `.apply` 22'den 53'e, `.check` 22'den 39'a. Durumlar masaüstünde
  (`KENTOS_CASES_ONLY=cad.profiles cargo test -p kentos-native-application --test all fixtures`) ve Python'da
  (`test_command_cases`) aynı sonucu verir; yazılan dosyalar sonraki adımlarda `cad.profiles.list`, `.check` ve `.apply` ile
  okunarak sınanır; `copyFile` adımı artık hedefinin klasörünü açar.
- Yazılan dosyanın kendisi `tests/all/profile_files.rs`'te: geri okununca verilen profil, göçleriyle sürüm, değiştirilen dosyanın
  bütünlüğü, klasörün yerinde dosya olunca `file_unwritable` ve eski dosyanın olduğu gibi kalması, yarım dosya kalmaması.
- Python'da uçtan uca: `python/tests/test_profile_files.py` (içe al, listele, göç, geri al, eşlemeyle denetle ve uygula, reddi).
- Yetenek testi (`capabilities`): her kod bir durumda ya da gerekçesiyle (`file_unwritable` yukarıdaki testte).
- Süre (`cargo test --release -p kentos-native-application --test all profile_files::migrate_timing -- --ignored --nocapture`,
  Apple M5): 100 000 ağaçlı park çiziminde iki göçün bütün adımları: plan 0,59 s, yazma 0,52 s.
- Resimler (açık ve koyu, 1440×900 ve 1100×650; `KENTOS_SHOTS_ONLY=profil-ice-aktar,profil-esleme,profil-goc,profil-goc-yapildi
  cargo test -p kentos-desktop --bin kentos-cad tools_screens -- --ignored --nocapture`): `.run/shots/arac-profil-*`.

## Sonuçlar ve sınırlar

- Profil dosyaları çizimle birlikte taşınmaz (Paketle kopyalamaz, `.kcad` taşımaz): başka bilgisayara giden çizimin yanına
  `profiller` klasörü de gitmelidir. Bulut projesinde çizimin dosyası yoktur: profil ve eşleme komuta kendisiyle verilir.
- Göç eşlemeyle çalışmaz; kurumun adlarıyla yazılı veriyi taşımak için önce göç profilin adlarıyla yazılmış adımlarla çalışmalı ya da
  profilin göç adımları kurumun adlarıyla (kendi profili olarak) yazılmalıdır.
- Değer listesi başka listeye taşınamaz (liste kimliği değiştirme); `mapValues` ve `retireCode` ile aynı listenin kodları taşınır.
- Katman yeniden adlandırması geri alınmaz (yukarıda): bu `cad.layers.set`'in kuralıdır, göç ona uyar.
- **Gözlem (bu işin dışında):** bu dalda `.kcad` v2'ye yazmak, değer listesine bağlı alanı (`domain`) ya da projenin `relations`'ını
  taşıyan çizimde “bilinmeyen alan” diye geri okuma denetiminde düşüyor (ör. `fixtures/profiles/v1/belediye.json`'u kaydetmek).
  Python testi bu yüzden kaydedilen çizimle yalnız alanları değer listesine bağlı olmayan boş bir projeyi kullanır; kodek ayrı iştir.
