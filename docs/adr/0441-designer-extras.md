# ADR 0441: Tasarımcı ekleri: paletler, bölücü, grafik, bölüm kitaplığı, yorumlar, profil ve eylem

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AX`; `plan/form.md` FORM-DESIGN-01, -02, -06, -10, -15, -17, -19,
  -21, FORM-STD-10.
- **Bağlam belgesi:** ADR 0403 (tasarımcı), 0425 (ilişkili tablo), 0427 (sürüm karşılaştırması), 0434 (formun şeması ve
  `docs/specs/forms-v1.md`), 0437 (kopyala ve yapıştır), 0440 (ilişkiler, kartlar).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** bölüm kitaplığının bulutta kurumla paylaşılması, yorumlarda rollerle doğrulanan gözden geçiren, mobil giriş
  biçimi denetimi ve mobil yürütücünün yayını engellemesi, çok dilli etiketler ve dar ekran yerleşimi (biçimin sonraki sürümü).

## Bağlam

Tasarımcının bileşenleri tek satırlık bir araç çubuğundaydı, sol sütun yalnız katmanın formda olmayan alanlarını sunuyordu; projenin
ilişkileri ve değer listeleri tasarımcıdan görünmüyordu. Bölümü başka forma taşımanın tek yolu kopyala ve yapıştırdı (ADR 0437):
tekrar kullanılan, sürümlü bir bölüm yoktu. Gözden geçirenin söyleyeceği yer, kapanan pencerenin kaydedilmemiş değişikliğini
koruyan bir kopya yoktu. Formun bir ortamdan ne istediği (an, kullanıcı, başka katman, şekil, iş akışı) bilinmiyordu; ileride web
ve mobil aynı formu açınca eksik yeteneği sessizce atlayabilirdi. Formdan bir işlem aracını açmanın yolu yoktu; olsaydı da alanın
özelliğine adres ya da betik yazılabilirdi.

QGIS'in sürükle bırak tasarımcısında alanlar, ilişkiler ve kapsayıcılar ayrı ağaçtadır; “eylem” katmanın Python ya da kabuk
komutudur ve formdan çağrılır. ArcGIS Field Maps'in form tasarımcısında bölümler ve ifadeler vardır, eylem yoktur.

## Karar

### 1. Paletler ve bölüm kitaplığı

- Sol sütunda dört palet: **Alan** (katmanın formda olmayan alanları), **İlişki** (projenin, formun katmanını üst ya da alt sayan
  ilişkileri: üst ise ilişkili kayıtlar öğesi, alt ise başvuru alanı olarak eklenir), **Liste** (projenin değer listeleri, bu
  katmanda onları kullanan alanlarla; basmak Değer listeleri penceresini açar), **Bölüm** (bölüm kitaplığı).
- Ortadaki araç çubuğunda bileşenler: sekme, grup ve not adlarıyla, bölücü, ilişkili kayıtlar, başvuru, ekler, konum ve eylem
  ikonlarıyla.
- **Bölüm kitaplığı** cihazdadır (`$XDG_DATA_HOME/kentos-cad/formlar/bolumler.json`, biçim `kentos.form-sections` 1; projenin
  verisi değil): seçili öğe (sekme, grup ya da herhangi bir öğe) bir adla konur, aynı adın her kaydı sonraki sürümdür, bağladığı
  katman alanları onunla gider; en çok 200 bölüm, fazlası en eskiden düşer. Okunamayan dosya kenara ayrılır, üzerine yazılmaz.
- Bölüm eklenirken kimlikleri forma göre yenilenir, alanları anahtarla, yoksa adla bağlanır (kopyala ve yapıştırın kuralı);
  palette her sürümün kaç alanının bu katmanda olmadığı görünür, eklendiğinde bağlanamayanlar adlarıyla söylenir ve ön denetim
  onları gösterir.

### 2. Bölücü

`FormItem::Divider { id, label?, visibleWhen? }`: formu bölen bir çizgi, başlığı üstünde; bir kural gizleyebilir. Hiçbir şey yazmaz.

### 3. İlişkili kayıtların grafiği

`RelatedView::Chart`: ilk sütunun (yoksa bağ alanından başka ilk alanın) her değeri için bir çubuk, boyu o değeri tutan alt kayıt
sayısıyla orantılı; çok olandan aza, boş değer sonda. Toplam sayı başlıktadır; Yeni ve Bağla aynen kalır.

### 4. Formun ortamdan istedikleri

`kentos_forms::requirements(body)` ve `cad.forms.validate`'in `requires`'ı: `moment` (`@tarih`, `@simdi`), `user` (`@kullanici`),
`layers` (başka katmandan liste ya da ilişkili kayıtlar), `geometry` (konum ya da mekânsal liste), `attachments`, `workflow`,
`tool:<kimlik>`. Hiçbiri yoksa form saftır. Sağlamayan ortam formu salt okunur açar ve eksiği söyler (`kentos_forms::unsupported`);
biçimin profili `docs/specs/forms-v1.md` §5'tedir. Masaüstü hepsini sağlar; tasarımcı öğe seçilmemişken formun istediklerini
gösterir.

### 5. Yorumlar ve kalan taslak

- `ProjectForm.comments`: `id` (1, 2, …), isteğe bağlı `item`, `author`, `at`, `text`, `resolved`; en çok 200. `cad.forms.edit`'in
  `comment`'i (taslakta olmayan öğe `form_item_unknown`) ve `resolve`'u (olmayan numara `form_comment_unknown`). Yorum proje
  ayarıdır, geri alma adımı değildir (formun öbür düzenlemeleri gibi). Masaüstü yazar ve anı formun kurallarının okuduğu kişi ve
  andan verir.
- Tasarımcının taslağı her adımda cihazda saklanır (`…/formlar/taslak/<proje>-<form>.json`); Kaydet ve Vazgeç onu siler. Pencere
  başka yoldan kaybolunca (program kapandı, çöktü) form yeniden açıldığında, saklanan taslak formunkinden farklıysa üstte sunulur:
  Geri yükle (taslak pencereye gelir, Kaydet bekler) ya da Bırak.

### 6. Eylem

`FormItem::Action { id, label, tool, visibleWhen? }`: formda bir düğme, kayıt seçili olarak İşlemler'in aracını açar. `tool`
yalnız kayıtlı bir aracın kimliğidir: noktalı ASCII harf ve rakam parçaları (`ACTION_TOOL_TEXT`); adres, sorgu ya da betik
sözleşmede (`body_problem`) ve şemada (`pattern`) reddedilir. Tasarımcı aracı yalnız kayıtlı araçlar arasından seçtirir; aracın
bu bilgisayarda olmaması tasarımcının ön denetiminde uyarıdır (`form_action_unknown`, ev sahibinin denetimi: çekirdek ev sahibinin
araçlarını bilmez) ve formda düğme kapalı görünür. Eylem kaydı kendisi yazmaz; aracın penceresi kendi kurallarıyla, kendi geri alma
adımıyla çalışır.

## Denetimler

- `scripts/fixtures/forms_cases.py --check`: `cad.forms.edit.json`'a yorumlar (numara, öğe, çözme ve yeniden açma) ve retleri
  (olmayan öğe ve numara, boş metin, olmayan form), adresli eylemin reddi; `cad.forms.validate.json`'a bölücü, eylem ve konumlu formun
  durumları ve gerektirdikleri. Doğrulama cevaplarının `requires`'ı ikizlerde ADR'den yazılmış `requirements`'tan (`layer_rules`,
  `contingents`, `subtypes` ikizleri de).
- `scripts/fixtures/form_schema_cases.py --check`: altıncı bozuk form, adresli eylem.
- `tools/kcad/kcad.py`: bölücü, eylem, `chart` ve yorumlar okunur ve denetlenir.
- Masaüstü: `a_divider_and_an_action_show_and_the_action_opens_its_tool_on_the_record`,
  `comments_go_through_cad_forms_edit_and_are_settled`, `a_draft_lost_with_the_window_is_offered_back_and_saving_forgets_it`,
  `a_section_goes_into_the_library_and_comes_back_into_another_form_with_its_fields_bound`, `forms::library` testi; resimler
  `form-bolucu-grafik-eylem`, `form-tasarim-iliski-paleti`, `form-tasarim-eylem`, `form-tasarim-bolumler`, `form-tasarim-yorumlar`,
  `form-tasarim-kalan-taslak`.

## Sonuçlar

- Tasarımcı veri kaynağını (alanlar, ilişkiler, listeler) ve tekrar kullanılan bölümleri bir yerden sunar.
- Kapanan pencere kaydedilmemiş tasarımı kaybettirmez; gözden geçirenin notu formun yanında kalır.
- Bir formun hangi ortamda tam açılacağı gövdesinden bilinir; eksik yetenek sessizce atlanmaz.
- Formdan araç açılır ama formun kendisi kod taşımaz.
