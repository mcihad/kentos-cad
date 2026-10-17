# ADR 0442: Yayın ve iş akışı ekleri: senaryolar, kırılan kayıtlar, sürüm farkının alanları, alan notları, eylem türleri ve eklerin bilgisi

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AY`; `plan/form.md` FORM-PUB-03, -08, -10, -12, -16, FORM-WORK-05,
  -07, -11, -13, FORM-STD-12.
- **Bağlam belgesi:** ADR 0401 (formlar), 0420 (senaryolar), 0427 (sürüm karşılaştırması), 0428–0429 (iş akışı ve geçmişi), 0438 §7
  (açılan yayın), 0440 §4 (anlık görüntü), 0441 (eylem, gerektirdikleri), 0225 (ekler, EXIF).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** yayının izin ve rol denetimi, yayın onayı ve hedef kitle (bulut), eklerin sahibi ve erişim politikası, sunucuda
  çalışan eylem, alan notlarının rollerle doğrulanan yazarı.

## Bağlam

Yayınla taslağın yalnız ön denetimine bakıyordu: formun kendi senaryoları düşse de yayın çıkıyordu; yeni yayının mevcut kayıtlardan
kaçını geçersiz kıldığı söylenmiyordu. Açık bir form yeniden yayınlanınca Kaydet çakışmayla duruyor, kullanıcıya yalnız Yeniden yükle
ve Üzerine yaz kalıyordu (üzerine yazmak eski sürümün kurallarıyla yazmak olurdu). Sürüm farkı neyin değiştiğini söylüyor, değişikliğin
neye dokunduğunu (yerleşim mi, veri bağı mı, kural mı, eylem mi) söylemiyordu. Geri gönderen inceleyen yalnız tek bir gerekçe yazabiliyordu;
hangi alanın düzeltileceği kaydın üstünde görünmüyordu. ADR 0441'in eylemi yalnız işlem aracı açıyordu; ne yaptığı (yazar mı, yalnız
gösterir mi) ve nerede çalıştığı açık değildi. Ekler adlarıyla duruyordu; türü, boyu, özeti ve çekim anı görünmüyordu.

## Karar

### 1. Yayından önce senaryolar

`cad.forms.edit`'in `publish`'i taslak son yayından farklıysa ve senaryoları varsa hepsini çalıştırır (`cad.forms.test`'in kuralıyla); ilk
düşen senaryo yayını durdurur: `form_tests_failed`, iletisinde senaryonun adı ve ilk farkı. Son yayınla aynı taslak yeniden çalıştırılmaz.
Ön denetim (alanlar, kurallar, iş akışı, eylemin adımı) önce gelir.

### 2. Kırılan kayıtlar ve açık formun yeni sürümü

- Önceki bir yayın varken yeni yayın, katmanın kayıtlarını iki sürümle de yargılar (Kayıtları denetle'nin kuralı: kaydedildiği gibi
  tamamlanır, doğrulanır; tekil değerler dışarıda): öncekine uyup yenisine uymayanlar sayılır. Cevabın `breaking`'i (`records`, alan
  başına sayı, alanların anahtarlarıyla, baytlarının sırasıyla) ve `form_breaking` uyarısı. Yayın yine yapılır (uyarı reddetmez); planı
  aynı sayıları yazmadan verir.
- Tasarımcıda Yayınla önce planı sorar; kırılan kayıt varsa pencerenin üstünde sayısı ve alanlarıyla söylenir: Vazgeç ya da Yine de yayınla.
- Kurallar uygulanıyorsa (`enforce`) katmanın yazma profili kendiliğinden son yayındır; ayrı bir “sunucu profili” güncellemesi yoktur.
- Formla düzenle açıldığı yayına bağlıdır (ADR 0438 §7). Form yeniden yayınlanınca Kaydet'in çakışması **Yeni sürümle aç** sunar:
  pencere yeni yayının gövdesiyle yeniden kurulur, yazılan değerler kalır; yeni sürümde olmayan alanlara yazılanlar bırakılır ve adlarıyla
  söylenir. Kayıt yeni sürümün kurallarıyla yazılır.

### 3. Sürüm farkının alanları

Her değişikliğin `areas`'ı, bu sırayla: `layout` (adlar, yardım, giriş biçimi, sütunlar, görünüm …), `binding` (`field`, `lookup`,
`layer`), `rule` (koşullar, zorunluluk, salt okunurluk, varsayılan, hesap, seçenek süzgeci, denetimler, tekillik, en az ve en çok),
`action` (eylemin türü, aracı, adımı, çalıştığı yer), `workflow`, `tests`. Eklenen ya da kaldırılan öğe yerleşimdir; alan ve ilişkili
kayıtlar bağdır, eylem eylemdir, kural taşıyan öğe kuraldır. İş akışının eklenmesi, kaldırılması ya da değişmesi artık bir değişikliktir.
`cad.forms.compare` ayrıca `requiresAdded` ve `requiresRemoved`'u verir (ADR 0441 §4). Tasarımcının Değişiklikler'i alanları her satırda,
yeni gerektirdiklerini üstte söyler.

### 4. Alanlara not

- İş akışının `notes`'u: serbest metin bir alanın anahtarı (geçmiş ya da gerekçe alanı olamaz). `cad.forms.transition`'ın `notes`'u
  (alanların anahtarlarıyla notlar, yalnız formun alanlarına; boş, uzun ya da denetim karakterli not `form_invalid`) bu alana kanonik
  JSON olarak yazılır (`{"<anahtar>":"<not>"}`, anahtarlar baytlarının sırasıyla); notsuz adım alanı boşaltır. Not alanı değerlerle
  yazılmaz; formda salt okunurdur; anlık görüntüye girmez (ADR 0440 §4'ün durum ve geçmiş alanları gibi).
- Geçmiş satırına “· n alan notu” eklenir; kaydın eski anlık görüntüsü satırda kalır.
- Formla düzenle'de **Alanlara not** açılınca her alanın altında not kutusu çıkar, notlar sıradaki adımla gider; açılan kayıtta son adımın
  notu alanın altında “İnceleyenin notu” olarak görünür. Geri gönderilen kayıt taslak durumdaysa (ADR 0432) eksik hâliyle düzeltilir.

### 5. Eylemin türleri

`FormItem::Action`'ın `kind`'i: `tool` (varsayılan; kayıtlı işlem aracı, `tool` ile), `recompute` (formun hesapları kayıtta pencerede
yeniden çalışır; Kaydet yazar), `step` (iş akışının adımı, `step` ile; formun iş akışında olmayan adım ön denetimde
`form_action_step` hatası), `show` (kayıt çizimde seçilir ve gösterilir; hiçbir şey yazmaz). Her tür yalnız kendi hedefini alır;
sözleşme ve şema öbürünü reddeder. Yazan türler (`tool`, `recompute`, `step`) salt okunur kayıtta (kilitli katman, kilitli durum)
kapalıdır; `show` açıktır. Yazma her zaman komutun kendi kurallarıyladır: araç kendi işlemiyle ve katmanın yazma profiliyle, hesap
Kaydet'in gönderimiyle, adım `cad.forms.transition` ile; eylem formun kurallarını atlatamaz, onaylı kaydı sessizce değiştiremez.
`show` formun gerektirdiklerine `geometry`'yi ekler.

### 6. Eylemin çalıştığı yer

`tool` eyleminin `runs`'ı: `auto` (varsayılan), `here` (bu bilgisayarda), `background` (arka planda, paneli ve Durdur'uyla); İşlemler'in
“Nerede çalışır”ının yerleri. Başka türde `runs` yazılmaz. Aracın gereği formun gerektirdiklerindedir (`tool:<kimlik>`).

### 7. Eklerin bilgisi

Salt okunur `cad.attachments.inspect` (en çok 1 000 nesne; `folder` çizimin klasörü, mutlak yol, göreli dosyalar oradan): her ekin
kaynağı (`asset`, `file`, `url`), ortam türü (PNG, JPEG, GIF, TIFF, PDF ve ZIP baytlarından; ZIP uzantısıyla DOCX, XLSX, ODS ya da KMZ;
ötekiler uzantıdan, yoksa `application/octet-stream`), boyu, SHA-256'sı, JPEG ve TIFF'te EXIF'in çekim anı ve WGS 84 konumu
(`kentos_formats::exif`). Gömülü fotoğraf projenin kitaplığından, adres indirilmez (yalnız uzantısından tür). Bulunamayan `missing`.
256 MB'tan büyük dosyanın yalnız boyu verilir. Retler `invalid_count`, `invalid_uid`, `entity_not_found`, `invalid_folder` (bu sırayla).
Formun ekler kutusunda **Bilgi** her ekin altına türünü, boyunu, çekim anını ve konumunu, özetin başını yazar.

## Denetimler

- `scripts/fixtures/forms_publish_cases.py --check`: `cad.forms.edit.publish.json` 21 durum (geçen ve düşen senaryolar, sıra, aynı taslak,
  kırılan kayıtlar ve alanları, plan, ilk yayın, 51. yayın, güncellenen taslak, Kuralları uygula, olmayan form).
- `scripts/fixtures/forms_cases.py --check`: yayının kırılma sayımı (`breaking_of`), eylem türlerinin doğrulama ve ret durumları, adım eyleminin
  ön denetimi; `forms_transition_cases.py`: alan notlarının yedi durumu; `forms_compare_cases.py`: alanlar, iş akışı ve gerektirdiklerin farkı.
- `scripts/fixtures/attachments_inspect_cases.py --check`: `cad.attachments.inspect.json` 20 durum ve okuduğu dosyalar
  (`fixtures/attachments/v1`), EXIF değerleri EXIF okuyucusunun bağımsız başvurusundan (`fixtures/exif/v1/cases.json`).
- Masaüstü: `a_publish_that_breaks_records_asks_first_and_publishes_when_asked_again`,
  `a_reviewer_s_notes_go_with_the_step_and_show_under_the_fields_after`, `a_new_release_opens_the_record_again_keeping_what_was_typed`,
  `a_step_action_takes_its_step_and_a_show_action_selects_the_record`, `the_attachments_facts_are_read_by_the_command`; resimler
  `form-alan-notlari`, `form-alan-notu-gosterilen`, `form-yeni-surum`, `form-yayin-kirilan`, `form-eylem-adim`,
  `form-degisiklik-alanlari`, `form-ekler-bilgi`.

## Sonuçlar

- Formun kendi senaryoları yayın kapısıdır; yeni yayının mevcut veriye etkisi yayından önce görülür.
- Açık form yeni sürüme yazılanları kaybetmeden geçer; eski sürümün kurallarıyla yazılmaz.
- İnceleyen alan alan geri bildirim verir; düzeltme kaydın üstünde görülür.
- Eylemin ne yaptığı ve nerede çalıştığı formun verisindedir; eklerin türü ve özeti Python'dan, MCP'den ve formdan okunur.
