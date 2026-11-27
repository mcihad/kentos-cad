# ADR 9347: Düzenleme hedefi ve oturumlar söylenir; Aç önce dosyayı sorar; kaydedilmemiş sekmeyle kapatma soruyla

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turu 3
  (`gis-eksik-27-11-2026.md`, T9 P3 ve T10 P2) ve sahibin sözü: “aynı anda bir kaç sekme açıkken çizimde uygulama
  kapatılamıyor ve hiç bir mesaj da vermiyor”.
- **İlgili:**
  - [ADR 9301](9301-edit-session-on-the-ribbon.md) (Düzenle şeridinde düzenleme oturumu);
  - [ADR 9128](9128-layer-target-and-data-layer-drawing.md) (veri katmanına çizim);
  - [ADR 0351](0351-documents-clipping-and-pdf-underlays.md) (çizim sekmeleri);
  - [ADR 0041](0041-desktop-cloud-interface.md) (çizimden çıkarken).

## Bağlam

- **T9:** `qa_noktalar` düzenlenirken `qa_yollar`'a basıldı.
  - Şerit yol için Düzenlemeyi başlat gösterdi. Noktanın oturumunun açık olduğunu yalnız katman ağacındaki küçük kalem
    söylüyordu.
  - İki oturum açıkken Düzenlemeyi bitir'in hangi katmanı kapattığı yazmıyordu.
- **T10:** iki proje açıkken Cmd+O, dosya seçilmeden önce etkin projeyi kaydetmeyi sordu.
- **Kapatma:** kaydedilmemiş bir çizim başka sekmedeyken pencere kapatılınca yalnız komut geçmişine bir uyarı yazıldı;
  pencere kapanmadı ve görünür bir ileti çıkmadı.

## Araştırma

- **QGIS, düzenleme:** birden çok katman aynı anda düzenlenebilir. “Toggle Editing” etkin katmanınkidir; “Current
  Edits” menüsü seçili ya da bütün katmanların düzenlemelerini kaydeder veya bırakır
  ([Editing › Saving Edited Layers](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html#saving-edited-layers)).
- **Masaüstü yönergesi, kaydedilmemiş belge:** belgeyi kapatan ya da yerine başka belge açan eylem, kaydedilmemiş
  değişiklikleri adıyla sorar; soru, kullanıcının değiştireceği şey belli olunca sorulur
  ([Apple HIG › Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts)).
- **KentOS kararı:**
  - Düzenlemenin hedefi ve öbür oturumlar, eylemin yanında adıyla söylenir.
  - Kaydetme sorusu yalnız çizimin yerini başkası alırken ve o başkası belliyken sorulur.
  - Pencere kapatılırken kaydedilmemiş bütün çizimler tek soruda adlarıyla sorulur.

## Karar

### 1. Düzenleme hedefi (T9)

- **Şeridin ipucu:** Düzenlemeyi başlat/bitir'in ipucu etkin katmanı ve oturumunu söyler.
  - Örnek: “Etkin katman “mahalleler” düzenleniyor; kaynağa yazılmamış kaydı yok. Düzenlemeyi bitir yalnız bu katmanı
    kapatır. Düzenlenen öbür katmanlar açık kalır: “bakim”.”
  - Kapalı katmanda: “Etkin katman “…” düzenlemeye kapalı. Düzenlemeyi başlat onu açar.”
- **Tablonun çubuğu:** katman seçicinin yanında oturumun durumu durur: “Düzenleniyor” ya da “Düzenlemeye kapalı”.
  - Birden çok katman düzenlenirken bu, “Düzenlenen katmanlar ▾” menüsüdür.
  - Menü her düzenlenen katmanı listeler; birine basmak onu etkin katman ve tablonun katmanı yapar.
- **Bitirme sorusu:** bir katmanın düzenlemesi değişikliklerle bitirilirken soru katmanın adını ve bekleyen sayıları
  söyler (önceden de öyleydi). Öbür oturumlar açıksa ekler: “Düzenlenen öbür katmanlar açık kalır: “…”.”
- **Kapalı çizim aracı:** nedenler ayrıdır.
  - Salt okunur kaynak: “… salt okunur bir kaynakta …”.
  - Kapalı oturum: “… düzenlemeye kapalı …”.
  - Geometri türü: “… bu katmana yazılmaz (geometri türü uymuyor).” Bu türü alan bir veri katmanı adıyla önerilir,
    düzenleneni önce: ““qa_yollar” düzenleniyor ve bu türü alır: onu etkin yapın.”

### 2. Aç önce dosyayı sorar (T10)

- **Sıra:** Aç (`file.open`) önce dosya seçiciyi açar. Dosya seçilince ekrandaki çizimin kaydedilmemiş değişikliği
  sorulur: Kaydet ve aç, Kaydetmeden aç, Vazgeç.
- **Vazgeç ve seçiciden vazgeçme:** ikisi de çizimi olduğu gibi bırakır; seçilen dosya sonraki bir Aç'ta açılmaz.
- **Yeni sekmede aç:** ekrandaki çizimin kaydetme durumundan bağımsızdır. Çizim değişiklikleriyle kendi sekmesinde
  kalır; soru sorulmaz (önceden de öyleydi; sınandı).
- **İzler:** iz oynatıcısının dosyası (`Picker::File`) aynı yoldan gider: dosya, soru, açılış.

### 3. Kaydedilmemiş sekmeyle kapatma

- **Soru:** pencere kapatılırken başka sekmede kaydedilmemiş bir çizim varsa “Kaydedilmemiş değişiklikler var” sorusu
  açılır.
  - İleti hepsini sekmelerindeki adlarıyla sayar: “2 çizimde kaydedilmemiş değişiklik var: “Örnek pafta.kcad”,
    “ikinci.kcad”.” Ekrandaki çizim değişmişse o da sayılır.
  - Kaydetmeden çık hepsini bırakır ve programı kapatır. Vazgeç pencereyi açık bırakır.
- **Kayıt sürerken:** kapatma önceki gibi kaydın bitmesini bekler ve bunu söyler.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Şeridin ipucu, tablonun çubuğu, Düzenlenen katmanlar | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı gerekçe | var |
| Kapalı çizim aracının geometri nedeni ve önerisi | yok: aynı gerekçe | var | var | var |
| Aç önce dosyayı sorar, kapatma sorusu | var | var | var | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Arayüzün sunumudur; oturumu `cad.layers.session` açar ve kapatır, komut değişmedi. Dosya açma ve pencere kapatma betiğin işi değildir. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Dosya işleri eklentiye kapalıdır. |

## Açık

- **Hepsini kaydet:** kapatma sorusunda bütün sekmeleri sırayla kaydeden bir “Hepsini kaydet ve çık” yok; her sekme
  kendi Kaydet'iyle kaydedilir.
- **Başka sekmedeki bekleyen kaynak düzenlemesi:** kapatma sorusu sekmedeki çizimin kaydedilmemiş değişikliğini sayar.
  Başka sekmede bekleyen veri katmanı satırları o sekmeye geçince sorulur.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `data_bar_tests::two_sessions_are_named_and_one_press_goes_to_either`: ipucu önce “bakim”'in oturumunu, ikinci
    oturumla öbürünün açık kaldığını söylüyor. Düzenlenen katmanlar'dan “bakim” etkin katman ve tablonun katmanı oluyor.
  - `doc_tabs_tests`:
    - değişmiş A açıkken B yeni sekmede sorusuz açılıyor, A değişikliğiyle sekmesinde;
    - Aç dosya seçilmeden soru sormuyor, C seçilince soruyor, Vazgeç çizimi ve dosyayı bırakıyor;
    - pencere kapatılınca soru iki çizimi sayıyor, onay “Kaydetmeden çık”.
  - `cloud::tests::drafts`: görüntüleyicinin projesinde de soru dosya seçildikten sonra.
  - `-- cloud:: opening saving recovery file_menu doc_tabs project:: traces data_bar layer_types data_edits module_tests
    ribbon_tests history::text catalog settings`: 419 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır.
  - `cargo test -p kentos-cad` (izler): 37 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9347/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-duzenlenen-katmanlar`: tablonun çubuğunda “Düzenleniyor · düzenlenen 2 katman ▾”;
  - `02-kapatma-sorusu`: “2 çizimde kaydedilmemiş değişiklik var: “Örnek pafta.kcad”, “ikinci.kcad”.”
