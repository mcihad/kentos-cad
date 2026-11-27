# ADR 9346: Kayıt akışı: Öznitelikler bekleyen değeri gösterir, formu olmayan katmanın varsayılan formu, üç eylemin adı, yeni kaydın formu

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026, kullanım turu 3
  (`gis-eksik-27-11-2026.md`, T5 P1, T6–T8 P2). Sahibin sözü: “yeni maddeler ekledim onları düzelt komple”.
- **İlgili:**
  - [ADR 9326](9326-one-record-on-every-surface.md) (tablonun satırı Öznitelikler'in kaydıdır);
  - [ADR 9149](9149-form-edit-from-a-table-row.md) (tablonun satırı formla açılır);
  - [ADR 9111](9111-editing-pending-with-the-project.md) §2 (yeni satırın alanları çizimden sonra);
  - [ADR 9123](9123-project-save-and-source-writes-told-apart.md) (bekleyen satırlar ve Kaynağa yaz);
  - [ADR 0403](0403-form-designer-and-entry-extras.md) §4 (çizimden sonra formu aç).

## Bağlam

- **T5:** düzenlenen katmanda tablonun hücresi `A`'dan `Mahalle A`'ya değişti, Kaynağa yaz başarıyla bitti. Tablo
  `Mahalle A`, sağdaki Öznitelikler aynı kayıtta `A` gösterdi. Öznitelikler satırı kaynaktan bir kez okuyup
  saklıyordu; yazmadan sonra yeniden okumuyor, bekleyen değeri de göstermiyordu.
- **T6:** özel formu olmayan katmanda Veri › Formu aç ve tablonun Form düğmesi açılmadı.
  - İki düğme farklı uyarılar verdi.
  - Salt okunur formun penceresi “Formla düzenle” diyordu.
  - Tek alanlı formda pencere büyük ve boştu.
- **T7:** formun Kaydı uygula'sından sonra durum çubuğu “formla yazıldı” dedi; kayıt kaynağa yazılmamıştı.
  - Düzenle şeridi “Kaynağa kaydet”, Veri şeridi ve inceleme penceresi “Kaynağa yaz” diyordu.
  - Düzenlemenin başlangıç iletisi yalnız “Kaydet” diyordu.
- **T8:** form tanımı olmayan bağlı katmanda çizgi çizildi; öznitelik girişi açılmadı. Tablonun yeni satırında Form
  düğmesi genel uyarıya düştü.

## Araştırma

- **QGIS, öznitelik formu:** katman için form tasarlanmamışsa QGIS alanlardan kendiliğinden bir form kurar; yeni bir
  nesne çizilince öznitelik formu açılır, varsayılanları ve zorunlu alanları alanların tanımından gelir
  ([Creating a New Vector Dataset › Forms](https://docs.qgis.org/latest/en/docs/training_manual/create_vector_data/forms.html)).
- **QGIS, düzenleme:** düzenleme oturumunda değişiklikler katmanın düzenleme belleğinde bekler; “Save Layer Edits”
  kaynağa yazar. Proje dosyasının kaydı ayrı bir eylemdir
  ([Editing › Saving Edited Layers](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html#saving-edited-layers)).
- **KentOS kararı:**
  - Bir veri katmanı değişikliğinin üç adımı her yerde aynı adla anılır: **Değişikliği hazırla** (katmanın
    düzenlemesi), **Projeyi kaydet** (proje dosyası), **Kaynağa yaz** (veri kaynağı).
  - Formu olmayan katmanın kaydı okunurken projeye form yazılmaz. Formla düzenlemek katmana alanlarının formunu
    verir ve bunu söyler.

## Karar

### 1. Öznitelikler'de kaynak satırı (T5)

- **Bekleyen değer önce:** tablonun seçili satırı düzenleme oturumuna alınıp değiştiyse Öznitelikler değerlerini
  oturumdaki nesneden gösterir. Altındaki söz: “Bekleyen değişiklik: kaynağa yazılmadı.” Silinmek üzere olan satır
  için: “Silinecek: kaynağa yazılmadı.”
- **Yazmadan ve bırakmadan sonra:** katmanın değişiklikleri kaynağa yazılınca ya da bırakılınca
  (`Document::data_written` değişince) seçili satırlar kaynaktan yeniden okunur.
  - Yanıt gelene kadar eski değerler kaynağınki gibi sunulmaz; söz “Kaynaktan yeniden okunuyor…” olur.
  - Yanıt gelince söz “Kaynaktan okundu; çizime alınmadı.”a döner.
- **Geri al:** oturumdaki değişiklik geri alınınca nesnenin değeri kaynağınkine döner; Öznitelikler kaynağın değerini
  gösterir.

### 2. Formu olmayan katman (T6)

- **Okumak için:** Veri › Formu aç, tablonun Form'u ve haritanın bilgi kartı aynı yoldan gider: tek seçili satır.
  - Katmanın formu varsa o açılır.
  - Yoksa satır, katmanın alanlarından kurulan **varsayılan formla** okunur. Bu form çizimin bir kopyasında kurulur;
    projeye form yazılmaz.
  - Formun söylediği: “Katmanın formu yok: kayıt alanlarından kurulan varsayılan formla gösteriliyor. Düzenle bu formu
    katmana ekler; Veri › Formlar'da tasarlanır.”
- **Düzenle ve düzenlemek için açılan satır:** katmana alanlarının formu verilir (Formlar'ın Yeni form'u gibi,
  `cad.forms.edit` Create). Söz: ““parseller formu” katmanın alanlarından oluşturuldu ve katmana eklendi; Veri ›
  Formlar'da tasarlanır.” Satır o formda düzenlemeye açılır. Formla düzenle, formu olmayan katmandaki tek nesneyi de
  aynı yoldan açar.
- **Pencere:** salt okunur kaydın penceresi “Kayıt formu · <form> · Salt okunur” diye adlandırılır, düzenlenenin
  “Formla düzenle · <form>”. Sekmesiz kısa form alanları kadar yüksektir; uzun form 460 piksellik bölmede kayar.
- **Seçim yoksa ya da birden çoksa:** üç giriş aynı sözü söyler: “Formu açmak için tek bir kayıt seçin: tabloda bir
  satır ya da çizimde bir nesne.” Birden çok satırda: “3 satır seçili: formu açmak için …”.

### 3. Üç eylemin adı (T7)

- **Değişikliği hazırla:** veri katmanının kaydında formun birincil düğmesi “Değişikliği hazırla”dır. Çizim katmanının
  kaydında “Kaydı uygula” kalır.
  - Söz: “Değişiklik hazırlandı: kayıt katmanın düzenlemesinde. Değer “bakim” katmanında bekliyor, kaynağa
    yazılmadı: …”
- **Kaynağa yaz:** `layer.editSave`'in adı “Kaynağa yaz”dır (önce “Kaynağa kaydet”). Düzenle şeridi, Veri şeridi,
  inceleme penceresi, katman menüsü ve sözler bu adı kullanır.
- **Projeyi kaydet:** düzenlemenin başlangıç iletisi: “… Kaynağa yaz'a kadar katmanda bekler; proje dosyası ayrıca
  Projeyi kaydet ile kaydedilir.”
- **Tablonun altı:** kaynağın henüz almadığı satırlar uyarı renginde sayılır (“2 kayıt kaynağa yazılmadı (1 yeni, 1
  değişen satır)”). Yanında Kaynağa yaz düğmesi durur; durum çubuğunun Kaynağa yaz'ıyla aynı eylemdir.
- **Başarı:** “parseller” kaynağa yazıldı: “parseller”: 1 değişen satır GeoPackage'a yazıldı (veritabanında
  “veritabanına”).
- **Taslağın yeri:** açılışta geri gelen taslak ile yalnız bu oturumdaki taslak ayrı sözlerle söylenir (ADR 9123'ün
  sözü, değişmedi).

### 4. Yeni kaydın formu (T8)

- **Çizince:** düzenlenen, alanları olan veri katmanına bir araç nesne yazınca yeni kaydın formu hemen açılır
  (Çizimden sonra formu aç açıkken; öntanımlı açık). Katmanın formu yoksa §2'deki gibi alanlarının formu verilir ve
  söylenir. Yeni kayıt seçilir.
- **Öznitelikleri gir:** tablonun “yeni” satırında form simgeli küçük bir düğme durur (ipucu “Öznitelikleri gir”).
  Basınca satırın nesnesi seçilir ve kaydın formu açılır.
- **Form düğmesi:** tablo yeni bir satırı gösterirken Form aynı formu açar; genel uyarıya düşmez.
- **Tek bekleyen satır:** geometri ve öznitelikler aynı nesnedir; form oturumdaki o nesneyi yazar.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Öznitelikler'de bekleyen değer, yazmadan sonra yeniden okuma | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı gerekçe | var |
| Varsayılan form, pencerenin adı ve boyu | yok: Mimari'de form yoktur | var | var | var |
| Değişikliği hazırla, Kaynağa yaz, tablonun altı | yok: veri katmanı yoktur | var | var | var |
| Yeni kaydın formu, Öznitelikleri gir | yok: aynı gerekçe | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Arayüzün sunumudur. Kanallar aynı işleri `cad.forms.edit` (Create), `cad.forms.submit` ve `cad.layers.session` (save) ile yapar; komutlar değişmedi. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Varsayılan formun geometrisi:** kopyada kurulan form, satırın alanlarını gösterir. Okumak için açılan kayıtta
  ilişkili öğeler yoktur (formu yoktur).
- **Zorunlu alanlı katman:** yeni kaydın formunda zorunlu alanın doğrulaması, formun alanlardan kurulan kuralıyladır;
  inceleme bu örneği ayrıca sınamadı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `source_selection_tests::an_edited_row_shows_its_waiting_value_and_after_the_write_the_sources`: hücre `B01-yeni`
    olunca Öznitelikler onu “Bekleyen değişiklik” ile gösteriyor. Kaynağa yaz'dan sonra “yeniden okunuyor”, yanıtla
    “Kaynaktan okundu” ve kaynağın yeni değeri.
  - `row_target_tests`:
    - formu olmayan katmanda seçim yokken ortak söz;
    - 22. satır varsayılan formla salt okunur açılıyor, projeye form yazılmıyor;
    - Düzenle katmana tek form veriyor ve satırı düzenlemeye açıyor;
    - çizilen çokgen yeni kaydın formunu açıyor, katmana formu veriliyor;
    - Öznitelikleri gir aynı formla yeniden açıyor;
    - tablonun altı “1 kayıt kaynağa yazılmadı (1 yeni satır)”.
  - `data_journal_tests`: yeni satır çizilince formu açılıyor; Çizimden sonra formu aç kapalıyken hiçbir şey açılmıyor.
  - `-- forms:: data_ features:: points module_tests ribbon_tests history::text catalog settings`: 408 geçti.
    - Kırmızı tek sınama bilinen bağımlılık tabanıdır.
    - `points::tests::a_double_click_anywhere_in_a_cell_edits_it` yükte bir kez düştü, tek başına geçti.
- **Kataloglar:** `catalog/v1/additions.json`'da `layer.editSave` “Kaynağa yaz”; `python3 scripts/ui/ribbon.py
  --check` güncel; `python3 scripts/cad/manifest.py --check` 1097 satır tutarlı.
- **Resim** (`.run/gui-evidence/kullanim-9346/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-varsayilan-form`: “Kayıt formu · parseller (alanlardan) · Salt okunur”, varsayılan formun sözü, üç alan,
    Düzenle;
  - `02-bekleyen-tablo`: “yeni” satırında Öznitelikleri gir simgesi; tablonun altında “2 kayıt kaynağa yazılmadı (1
    yeni, 1 değişen satır)” ve Kaynağa yaz; Öznitelikler'de `Ticaret`, “Bekleyen değişiklik”.
