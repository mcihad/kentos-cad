# ADR 0550: Kim hangi kayıtları ve alanları okur ve dışarı alır: tek çözücü

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-38 ("mevcut yetki modelini yeni dataset/satır/alan/iş kapsamına uygula"). Kapsamı ben belirledim.
- **Ek (2026-10-29):** §6'nın olay günlüğü ve sayfa imleci sınırları ile ajanın cevabında `redact` alan
  [ADR 0738](0738-agent-veil-entry-pages-and-unreached-events.md)'de kapandı: hiç erişilmemiş kaydın olayı gitmez,
  sayfanın imleci Veri girişinin kendi kaydıdır, MCP ve masaüstünün ajan bağlantısı gizli değeri vermez.
- **Bağlam belgesi:**
  - ADR 0411 (`forms.design`, `forms.publish`, `forms.use`, Veri girişi rolü, sunucunun yazma bekçisi `form_guard.rs`);
  - 0443 (alanın `redact`'ı: "Örneklerde gizle");
  - 0539 (formun `scope`'u: Veri girişi yalnız kapsamındaki kayıtları okur, sayar ve yazar);
  - 0534 (atlas PDF'i izinli öznitelikleri taşır), 0546 (masaüstü Öznitelik tablosu gizli alanı kopyaya, dışa aktarmaya, İstatistik'e ve Grafik'e sokmaz);
  - 0542 ve 0547 (dayanıklı sunucu işleri, kesinleşmede yetkinin yeniden sorulması);
  - 0015 (proje rolleri ve izinleri), 0445 (formu yayından kaldırma);
  - CLAUDE.md §16 (erişilemeyen proje 404'tür; indirilmiş kopyanın geri alınabileceği iddia edilmez).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - katmanın kendisine ya da tek alanına rol bağlayan ikinci bir izin sistemi (bilerek yapılmadı, §1);
  - satır kapsamının ifadesinin sunucuda SQL'e çevrilip sayfalamayla birlikte veritabanında süzülmesi (bugün kapsamlı katmanın satırları okunup uygulamada yargılanır);
  - olay günlüğünün Veri girişine göre ayrı yazılması (bugün olay kimliği ve zamanı görünür, §6);
  - `redact`'ın kayıtlı kopyalar için de kesilebilmesi (kontrol noktası, dosya revizyonu: değişmez nesneler, bugün verilmez, §2).

## Bağlam

Okuma ve dışa alma yetkisi dört ayrı yerde, dört ayrı biçimde yazılmıştı:

- Veri girişinin okuduğu alanlar (`form_guard::shown_fields`): formu olan her katmanda, **yayından kaldırılmış formun alanları da** sayılırdı;
- Veri girişinin okuduğu kayıtlar (`scopes_of`, `in_scope`): yayından kaldırılmış formu olan katman kapsamsız, yani **bütün kayıtlarıyla açık** sayılırdı;
- `redact`: yalnız yerel yollarda (kayıt örnekleri, tablonun dosyası, İstatistik, Grafik, atlas) ve masaüstünde `python_call::redacted_names` ile bilinirdi;
  **sunucu `redact`'ı hiç bilmezdi**;
- hiçbiri sayılara, kutuya, olaylara, çakışma cevabına, formun liste değerlerine ve kopyalara bakmazdı.

Taranan sunucu yolları ve bulunan sızıntılar:

| Yol | Sızıntı |
|---|---|
| `GET …/details` | nesne sayısı ve `st_extent` sınırı bütün nesneler üstünden (Veri girişi bile) |
| `GET …/events`, WS | kapsam dışı kaydın kimliği, sürümü ve işlemi |
| `project.changes` çakışması | kapsam dışı kaydın gerçek sürümü (`actual`) ve kopyası (`current`) |
| formun liste değeri (`lookup`) | `check_forms` başka katmanın **bütün** satırlarını kurardı: Veri girişi, kapsamında olmayan bir kaydın var olup olmadığını formun kabul ya da reddinden öğrenirdi |
| kontrol noktası ve dosya revizyonu listesi | bütün projenin nesne sayısı (`objects`) |
| `GET …/snapshot`, dışa aktarma işi | `redact` alanlar kopyada olduğu gibi |
| kontrol noktası ve dosya revizyonu indirme, çoğaltma, geri yükleme, dönüştürme | aynı: değişmez kopya `redact` alanlarıyla birlikte |
| işin kesinleşmesi | iş çalışırken formun işaretleri ya da isteyenin form tasarlama hakkı değişirse eski kopya yine verilirdi |

## Karar

### 1. Tek çözücü: `kentos_forms::reach`

Saf, ortak çekirdekte (`crates/shared/forms/src/reach.rs`). Girdisi projenin formları, katmanların alanları, `@` değerleri ve **çağıranın sınıfı**; çıktısı alan düzeyi ve
kayıt sorgusu. Rol ve izin sistemi yerinde kalır (`access.rs`, ADR 0015); çözücü onların üstüne ikinci bir sistem değil, onların söylediğinin **veriye çevrilmesidir**.

- **Çağıran sınıfı** (`Caller`), rolden `projects::caller_of` ile çıkar:
  | Sınıf | Kim | Neden |
  |---|---|---|
  | `through_forms` | Veri girişi (`feature.write` yok, `forms.use` var) | kayıtları yalnız formlarla okur ve yazar |
  | `reader` | Görüntüleyen, yorumcu, düzenleyici | bütün kayıtları ve alanları okur |
  | `whole` (`designs`) | `forms.design` sahibi (Yönetici, Sahip); yerel çizimin sahibi; başsız sunucu | işaretleri kendi değiştirebilir, saklanan kopyayı alabilir |
- **Alan düzeyi** (`Level`): `Hidden` < `Redacted` < `Open`.
  - `Hidden`: Veri girişinin okumadığı alan. Katmanın canlı formları göstermiyorsa ya da katmanın bütün formları yayından kaldırılmışsa.
  - `Redacted`: bir formun `redact` dediği alan. Kayıtla çalışılan yerde (ekran, form) okunur; **hiçbir dosyaya, kopyaya, sayıya, istatistiğe, grafiğe ve ajanın
    cevabına girmez**.
  - `Open`: gerisi.
  - `readable()`: `Hidden` değilse. `exportable()`: yalnız `Open`.
- **Kayıt sorgusu** `sees(katman, öznitelikler, nesne)`:
  - Veri girişi dışında herkes her kaydı okur.
  - Veri girişi için bir katmanın canlı (yayımlanmış, kaldırılmamış) formlarından biri kaydı kapsıyorsa; kapsamı olmayan form her kaydı, derlenmeyen ve boş
    kalan kapsam hiçbirini kapsar (ADR 0539).
  - **Yayında kullanılan ama bütün formları kaldırılmış katman (`Closed`) hiçbir kaydı vermez.** Bu, ADR 0539'daki "canlı formu olmayan katman kapsamsızdır"
    cümlesinden bilinçli bir sıkılaştırmadır: formun yayından kaldırılması (ADR 0445) kullanımın sonudur; sonu, kayıtları herkese açan bir yan etki olmamalı.
    Hiç yayımlanmamış (yalnız taslak) form katmanı bağlamaz.
- **İşaret** (`redact`) hangi sürümde sayılır: formun **taslağında ve son yayınında**. Eski sürümlerdeki işaret, daha yeni bir sürüm onu bırakınca artık
  sayılmaz: tasarımcı yeni sürümle işareti kaldırabilir. (Eski kural, `python_call::redacted_names`'in "her sürüm"ü, kaldırılamayan işaret demekti; masaüstü de artık
  aynı işlevi çağırır, `kentos_forms::reach::redacted_names`.)
- **Kopya** `copy_attrs`: bütün projenin kopyasında `forms.design` sahibi alanı olduğu gibi alır; ötekiler yalnız `Open` alanları. **Saklandığı gibi duran kopya**
  (dosya projesinin revizyonu, kontrol noktası) kesilemez (bayt ve SHA-256 değişirdi); `whole_copy_ok()` bunu söyler: hiçbir alan işaretli değilse ya da çağıran
  tasarımcıysa `true`.
- **Damga** `stamp()`: kararın metni (kim, hangi alanlar gösteriliyor ve işaretli, hangi katman kısıtlı ya da kapalı). Aynı damga aynı cevap demektir.
- Ortak durumlar `fixtures/reach/v1/cases.json` (16 durum, 3 çağıran sınıfı; hepsi KentOS kodu olmadan `scripts/fixtures/reach_cases.py` ile yazılır; kapsam
  ifadesinin üç biçimi başvuruda yeniden yazılmıştır); Rust tarafı `crates/shared/forms/tests/all/reach_cases.rs`. Birim testler `reach.rs`'te.

### 2. Sunucuda uygulandığı yerler

`projects::Reading` çözücüyü proje satırından (ayarlar, katmanlar, `@` değerleri) kurar; `reading_for` yalnız Veri girişi için, `reading_of` herkes için.

- **Okuma** (`features`, `features_by_id`): kayıt `sees` ile süzülür, öznitelikler `readable` ile kısılır (değişen yalnız Veri girişinin okumasıdır).
- **Sayı ve sınır:** `info` ve `details` Veri girişine yalnız eriştiği kayıtları sayar; sınır (`st_extent`) yalnız onların geometrisidir (`projects::reached_ids`).
- **Olaylar** (`events::after`, HTTP ve WS aynı yoldan): Veri girişi için elde olup erişemediği kaydın değişikliği **silme** olarak, sürümsüz iletilir. İstemci
  elindeki eski kopyayı bırakır (kayıt kapsamdan çıkmış olabilir) ve içeriğini ya da sürümünü bilmez. Silinmiş kaydın olayı olduğu gibi gider: kayıt yok, yargılanamaz.
- **Çakışma:** Veri girişinin eriştiği kayıtta çakışma cevabındaki kopya `readable` ile kısılmış gelir. Erişemediği kayıt sürüm karşılaştırılmadan **404**'tür
  (`Kayıt bulunamadı.`, ADR 0539'daki gibi): ne gerçek sürüm ne kopya söylenir.
- **Formun liste değerleri:** `check_forms` Veri girişi için başka katmanın satırlarını yalnız eriştikleri ve okuyabildiği alanlarıyla kurar. Yalnız erişemediği bir
  kayıtta var olan değer, hiç var olmayan değerle **aynı iletiyle** reddedilir. (Başkalarında kurulum eskisi gibidir: yalnız `properties` okunur, geometri çözülmez.)
- **Geçmiş sayıları:** kontrol noktası ve dosya revizyonu listelerinde `objects` Veri girişine verilmez.
- **Kopyalar:**
  - `GET …/snapshot` ve `project.export` işi kesilmiş kopya verir: `snapshot::take(…, Take::Cut)` işaretli alanları, çağıran `forms.design` sahibi değilse, nesnelerden atar.
    Kopyanın **aynı anının** ayarları (formlar) kopyayı kesen ayarlardır.
  - Kontrol noktası oluşturma (`feature.write` sahibi) kopyayı **olduğu gibi** saklar (`Take::Whole`): yedek, yedek olmalı; ama onu indirmek (kontrol noktası ve dosya
    revizyonu indirme), çoğaltmak, geri yüklemek ve öbür saklama biçimine dönüştürmek `whole_copy_ok()` ister (`projects::require_whole_copy`).
  - Veri girişi kopya almaz: `project.download` izni zaten yoktur; `take` bunu bir de kendisi sorar.
  - Reddin iletisi nedeni ve çıkış yolunu söyler: "Bu projede formların “örneklerde gizle” dediği alanlar var; olduğu gibi saklanan bir kopya … yalnız form tasarlayanlara
    verilir. Güncel hâlini .kcad olarak indirin: gizli alanlar kesilmiş gelir."
- **Yazma:** `form_guard::in_reach` Veri girişinin yazdığı kaydı, hem eski hem yeni hâliyle yargılar (404 / 403); `merged` okumadığı alanları saklı hâlinden korur.

### 3. Yerel çizim ve başsız sunucu: "tüm hak"

- Yerel çizimin sahibi (masaüstünde açılan `.kcad`, başsız sunucunun çizimi, Python `Document`, MCP çizimi) çizimin verisinin **bütün hakkına** sahiptir: orada
  okuma ya da kopyayı kısmak anlamsızdır; dosya zaten onun elindedir. Çözücü yerel yolda `whole` sınıfıyla (alan ekranda okunur) ve **dışarı giden her yolda**
  `exportable` ile çalışır: kayıt örnekleri, Öznitelik tablosunun dosyası ve panosu, İstatistik, Grafik, atlas (ADR 0443, 0534, 0546). Masaüstü ve atlas aynı
  işlevi çağırır (`kentos_forms::reach::redacted_names`); iki yerde ayrı liste tutulmaz.
- Çözücünün satır ve alan kısıtı **sunucuya ve paylaşılan projeye** uygulanır. Bulut projesini açan Veri girişinin masaüstü, okuduğunu sunucudan sayfa sayfa alır; sunucu
  kısmıştır (§2). Python `Connection` ve MCP'nin proje komutları aynı HTTP yolundan geçer: ayrı bir okuma yolu yoktur.
- Sınır: sunucudan bütün kopyayı alan `forms.design` sahibi yerelde alanı görür; bu bilerek böyledir (§1).

### 4. Kesinleşmede yetkinin yeniden sorulması

ADR 0542 iş bitiminde erişimi ve izni yeniden sorar. Bu karar iki şey ekler:

- `project.export` işi, kopyayı kestiği çözücünün **damgasını** taşır (`Outcome::Done.reach`). Bitirirken (aynı işlemde, `access::recheck`'ten sonra) çağıranın çözücüsü
  yeniden kurulur; damga değiştiyse iş `reach_changed` koduyla düşer ve dosya silinir. Bu iki durumu yakalar: iş çalışırken bir formun taslağına ya da yayınına
  `redact` eklenmesi; isteyenin `forms.design` hakkını yitirmesi (tam kopya artık onun değil). Masaüstünün işler görünümü kodun önerisini söyler (`kentos_cloud::server_jobs`).
- Öteki uzun yazmalar zaten kendi kilitleri altında yeniden sorar: dosya revizyonu yazma, içe aktarma, kontrol noktası oluşturma, geri yükleme, dönüştürme ve
  `project.changes` (`changes::lock` → `access::recheck`; `project.changes` formları o anki ayarlarla yargılar). Ek bir kapı gerekmedi.
- Sınır: damgayı karşılaştırma ayar değişikliğiyle aynı işlemde değildir (proje kilidi alınmaz: kopya alan yazarı bekletmez). Karşılaştırmayla yazma arasında
  değişen ayar bir sonraki kopyada görülür. Bu, kopya zaten verilmişken değişen işaretten farksızdır.

### 5. Çevrimdışı, indirilmiş kopya geri alınamaz

Bir kopya kullanıcının cihazına geçtiğinde, sunucu onu geri alamaz. Bu bir yetki sisteminin ayarı değil, fizik: ekranda görülen, indirilen ve cihazda duran veri, erişim
kaldırılsa da orada kalır.

- `redact` bir **erişim denetimi değil**, bir "dışarı çıkan kopyada kesme" kuralıdır: sahibin istemediği alan kopyaya girmez; ekranda gösterilmesi ve daha önce
  verilmiş kopya bununla geri alınmaz.
- Erişimi kaldırmak ya da rolü düşürmek önceden indirilmiş kopyaları geri almaz. Bu cümle kullanıcıya iki yerde söylenir: erişimi kaldırma sorusunda (mevcuttu,
  `share_plan::revoke_question`) ve **Projeyi paylaş penceresinin Kişiler sekmesinde kalıcı not** olarak (yeni, `people::COPIES`): "Erişimi kaldırmak ya da rolü düşürmek,
  kişinin daha önce indirdiği kopyaları ve ekranında gördüklerini geri almaz; formların “örneklerde gizle” dediği alanlar yalnız sunucunun verdiği kopyalara girmez."
- Çoğaltma ve geri yükleme ile aynı kural: başka projeye taşınan veri o projenin sahibinindir.

### 6. Bilinen sınırlar

- Olay günlüğünde kapsam dışı kaydın kimliği ve olay zamanı (silme olarak) hâlâ görünür. Kimlikler rastgele UUID'dir, içerik ya da erişim vermez; etkinliğin zamanlaması ve
  sayısı sızar. Ayrı bir olay günlüğü yazmak sonraya kaldı.
- Sayfa imleci (`next`) bir sayfanın son satırıdır; kapsam dışı satırın kimlik sırası bu yolla seçilebilir, içeriği değil (ADR 0539'daki gibi).
- Kapsamlı katmanın satırları okunup uygulamada yargılanır; çok büyük katmanda kapsam SQL'e çevrilmedikçe okuma maliyeti kapsam dışı satırları da içerir.
- Saklı kopya reddi sert: bir alan işaretliyken Yönetici olmayan biri kontrol noktası ya da dosya revizyonu indiremez. Yolu: güncel hâli `.kcad` olarak indirmek (kesilmiş
  gelir) ya da yöneticiden istemek.

## Sonuç ve kabul

- Veri girişine her okuma yolu aynı cevabı verir: sayı, sınır, olay, çakışma, liste değeri, sayfa; erişemediği kayıt ve okumadığı alan hiçbirinde izlenemez.
- `redact` işaretli alan hiçbir sunucu dosyasına (kopya, dışa aktarma işi) girmez; saklı kopya yalnız onu kesmeden alabilen tarafa verilir.
- Kopya alınırken ya da iş çalışırken yetki ve işaretler değişirse kopya verilmez.

## Doğrulama

- `cargo test -p kentos-forms` (birim: `reach.rs`; ortak durumlar: `tests/all/reach_cases.rs`); `python3 scripts/fixtures/reach_cases.py --check`.
- Gerçek veritabanı (`KENTOS_TEST_DB=required`, geçici PostGIS): `cargo test -p kentos-application --test project_reach`:
  - Veri girişinin sayısı, sınırı, olayı, çakışması ve geçmiş sayısı;
  - liste değeri: yalnız erişilemeyen kayıtta var olan değer, var olmayanla aynı iletiyle reddedilir (ve kapsam süzgeci kaldırılınca düşer, mutasyonla denendi);
  - bütün formları kaldırılmış katman Veri girişine hiçbir şey vermez;
  - görüntüleyenin dosyası işaretli alanı taşımaz, sahibinki taşır; kontrol noktası görüntüleyene verilmez, işaret kalkınca verilir;
  - dışa aktarma işi isteyen için kesilir; iş çalışırken eklenen işaret ve düşürülen rol işi `reach_changed` ile düşürür.
- Eski testler: `project_form_scope`, `project_forms`, `server_jobs`, `project_snapshot`, `project_catalog`, `project_checkpoints`, `project_files`, `project_duplicate`,
  `project_convert`, `changes`; `kentos-api` `jobs_tests`, `ws_tests`, `checkpoints_tests`, `files_tests`; `kentos-plot` atlas; masaüstü `redact` ve `cloud::share`.
- Resimler `cargo test -p kentos-desktop cloud::share_tests::screens -- --ignored --nocapture`: `.run/shots/bulut-paylas-kisiler-{1440x900,1100x650}[-acik].png`.
