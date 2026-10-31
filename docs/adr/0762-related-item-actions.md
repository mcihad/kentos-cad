# ADR 0762: İlişkili kayıtlar öğesinin eylemleri ve izinleri

- **Durum:** kabul edildi (31 Ekim 2026).
- **İş:** plan/form.md `FORM-REL-11`: “Child create/update/delete/link/unlink/view/select-on-map action'ları ayrı yetki
  ve capability ile gösterilsin.”
- **Dal:** `formlar-surer` (sahibin isteği: “Form ile ilgili işlere devam et ama ayrı bir dal açarak devam et”).
- **Web:** web dururken (ADR 0228) yalnız masaüstü ve bulut; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez (§5).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

İlişkili kayıtlar öğesi (ADR 0404, 0405, 0409, 0425, 0440) her alt kaydın yanında Git, Bağı kopar ve Aç'ı, altında
Yeni ile Bağla'yı her zaman gösteriyordu. Bunların ikisi ayrı soruyu karıştırıyordu:

- **Formun sunduğu (capability):** bir tasarımcı malik formunda yalnız var olanları bağlatmak, yeni malik yazdırmamak
  isteyebilir. Ya da sayaç listesini yalnız okunur göstermek ister. Formun bunu söyleyecek yeri yoktu. Alt kaydı
  silmek ise hiç yoktu.
- **Kullanıcının yapabildiği (permission):** alt katman kilitliyse, bulut projesinde rol nesne yazamıyorsa ya da Veri
  girişi rolü yalnız var olan kayıtların özniteliklerini değiştirebiliyorsa (ADR 0411 §5), düğmeler yine basılabilir
  görünüyordu. Basınca da komutun reddiyle karşılaşılıyordu.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **QGIS, ilişki düzenleyici bileşeni:** düğmeler tek tek açılıp kapanan bayraklardır:
  `Link | Unlink | SaveChildEdits | AddChildFeature | DuplicateChildFeature | DeleteChildFeature | ZoomToChildFeature`
  (`QgsRelationEditorWidget::Button`, `setVisibleButtons`). Belgedeki gerekçe: “as in some projects we only want to
  allow linking/unlinking existing features”. Öntanımlı hâlde hepsi görünür. Yazılabilirlik ayrıca katmanın düzenleme
  durumundan gelir. <https://api.qgis.org/api/classQgsRelationEditorWidget.html>
- **ArcGIS Survey123, ilişkili tablonun (repeat) parametreleri:** `allowAdds` (öntanımlı true) ve `allowUpdates`
  (öntanımlı false) ilişkili kayıt eklemeyi ve var olanları düzenlemeyi ayrı ayrı açar. Silme sahada desteklenmez.
  <https://doc.arcgis.com/en/survey123/create-surveys/prepareforediting.htm>

İkisinde de eylemler tek tek açılıp kapanır. Yetki ise katmanın ve hesabın durumundan gelir. KentOS bu ayrımı aynen
yapar: formun sunduğunu öğe söyler, yapılıp yapılamayacağını çizim ve hesap söyler.

## Karar

### 1. Öğenin eylemleri

`FormItem::Related`'ın isteğe bağlı `actions`'ı, `RelatedAction` listesidir: `create` (Yeni), `update` (Düzenle),
`delete` (Sil), `link` (Bağla), `unlink` (Bağı kopar), `view` (Aç) ve `show` (Git). Her biri bir kez yazılır;
yinelenen ret alır: “öğesinin eylemleri birer kez yazılır”.

- **Yazılmamışsa:** öğe, öğelerin bugüne dek yaptığını sunar. Bu silme dışındaki altı eylemdir
  (`RelatedAction::DEFAULT`). Var olan formların davranışı değişmez.
- **Boş liste:** alt kayıtlar yalnız listelenir.
- **Silme:** yalnız açıkça yazılınca sunulur. Silmek geri dönüşsüz görünen tek eylemdir; QGIS ve Survey123 de onu ayrı
  tutar.
- **`view` ve `update`:** `view` alt kaydın formunu okumak için açar. `update` açılan formu düzenlenir kılar ve
  gönderimde alt kayda değer yazdırır. `update` olup `view` yoksa Aç yine görünür ve düzenlenir açar. `view` olup
  `update` yoksa alt kaydın formu salt okunur açılır, Kaydet kapalıdır.

### 2. İzin: ayrı soru, ayrı gerekçe

Çekirdek (`kentos_forms::related_offer`), öğenin sunduğu her eylem için izni ayrı yargılar (`ChildPermits`). Sırası
şudur:

1. **Kayıt henüz yazılmamış:** önizlemede ya da kimliksiz yeni kayıtta yazan eylemler kapalıdır. Gerekçesi “Önce kaydı
   kaydedin.”
2. **Formun katmanı kilitli ya da hesap katman ağacını değiştiremiyor:** kaydın kendisi yazılamaz, bağ ve alt kayıt da
   onunla gider (ADR 0440 §2).
3. **Alt katman kilitli:** oluşturma, düzenleme, silme, bağlama ve bağ koparma kapalıdır. Gerekçe katmanın adıyla
   söylenir. Açmak ve Git açıktır.
4. **Hesap nesne yazamıyor:** bulut projesinde rol `feature.write` ya da `forms.use` taşımıyorsa, yazan eylemlerin
   hepsi kapalıdır.
5. **Veri girişi (ADR 0411 §5):** veritabanı projesinde yalnız var olan kayıtların özniteliklerini değiştirir.
   Oluşturma ve silme kapalıdır. Bağlama ve bağ koparma alt kaydın bir özniteliğidir ve açıktır; sunucu her değişikliği
   yine yargılar.

Okuyan eylemler (`view`, `show`) yalnız kayıt varsa sunulur. Pencere iki soruyu ayrı gösterir. Formun sunmadığı eylem
hiç görünmez. Sunulup izni olmayan eylem soluk görünür, gerekçesi ipucundadır. Bölümün altında da tek satır yazar,
örneğin “Sayac katmanı kilitli; alt kayıt yazılamaz.” Kullanıcı neyin neden kapalı olduğunu basmadan görür.

### 3. Komut da formun sunduğuna uyar

`cad.forms.submit`'in `children`'ı (ADR 0440 §2) artık formun sunduğuyla sınanır. Böylece Python, MCP ve bulut
istemcisi de pencerenin yapamadığını yapamaz. Her alt kaydın istediği eylem şöyle bulunur
(`kentos_forms::child_actions`):

| Alt kayıt | Eylem |
|---|---|
| Çizimde yok (yeni kimlik) | `create` |
| Var, kayda bağlı değil, `unlink` yok | `link` |
| Var, kayda bağlı, `unlink` | `unlink` |
| Var, verdiği değerlerden biri tuttuğundan farklı | `update` (bağlamayla birlikte de) |

Bağlı olmayanın bağını koparmak ve tuttuğu değeri yeniden vermek eylem değildir.

Formda ilişkinin alt katmanını ve alanını gösteren öğeler sunulanı birleşik verir (`kentos_forms::relation_offer`).
Öğenin görünürlük kuralı bunu değiştirmez; gizli öğenin sınırları sayılmaz (ADR 0444 §1), ama eylemleri formun
tasarımıdır. Formda ilişkiyi gösteren öğe yoksa form o ilişki hakkında bir şey söylemez ve gönderim eskisi gibi
yazılır.

Sunulmayan eylem `form_related_action` reddidir:
“‹Öğe› alt kayıtlarında ‹eylem› sunmuyor; formun tasarımında öğenin eylemlerine ekleyin.” Yolu `children[i]`,
nesnesi alt kayıttır. Gönderim bütünüyle reddedilir, hiçbir şey yazılmaz (ADR 0440'ın hep ya da hiç kuralı).

### 4. Silme

Sil, satırın yanında açılır. Basınca satırda “Silinsin mi?” ile Sil ve Vazgeç belirir, iç içe pencere açılmaz. Onayla
`cad.entities.delete` alt kaydı kendi geri alma adımında siler. Projenin ilişkisinin silme kuralları (ADR 0405 §3:
bileşik, kısıtla) olduğu gibi işler. Kısıtlayan ilişkinin reddi pencerede söylenir. Silmek kaydın taslağını
bekletmez. Yeni ile alt kayıt yazmak da öyleydi (ADR 0409 §3).

### 5. Tasarımcı ve biçim

Tasarımcının ilişkili kayıtlar öğesinde “Eylemler” satırı yedi düğmeyle eylemleri açar ve kapar. Öntanımlıya dönen
liste yazılmaz.

`actions` formun gövdesindedir. Formlar `.kcad` şema 72'de bütün durur ve şema değişmez. Bağımsız Python okuyucusu
(`tools/kcad/kcad.py`) yeni anahtarı ve yineleme kuralını bilir. Formun JSON Schema'sı yeniden üretilir.

### 6. Açık kalanlar

- **Sunucu:** sunucunun form bekçisi (`form_guard`) Veri girişinin gönderdiği alt kayıt değişikliğini öğenin eylemleriyle
  sınamaz. Bu, üst kaydı düzenleme izninin alt kaydı düzenleme izni vermemesiyle birlikte `FORM-REL-13`'tür.
- **Satır düzeyinde yetki:** satır ve alan düzeyinde yetki (alt kaydın kapsamı, ADR 0539) pencerede ayrı gösterilmez.
  Kapsam dışı satır listede zaten yoktur.
- **Web:** TODOS.md §0.2.

## Sonuçlar

- Tasarımcı “yalnız bağla”, “salt okunur liste” ya da “silinebilir alt kayıtlar” gibi formları kurar. Komut da aynı
  kurala uyar.
- Kullanıcı kapalı düğmenin nedenini basmadan görür. Kilitli katman ve rol farkı yanlış düğmeye basıp ret almakla
  öğrenilmez.
- Var olan formlar değişmez. Silme yeni bir eylemdir ve açıkça istenir.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/related_actions_cases.py --check`, KentOS kodu olmadan yazıldı:
  - `cad.forms.submit.actions.json`, eylemlerin gönderim durumları;
  - `forms_cases.py`'nin form gövdesi durumları (yineleme reddi).
- Çekirdek: `cargo test -p kentos-forms` (`related_offer`, `child_actions`).
- Masaüstü: `cargo test -p kentos-desktop --lib forms::`. Resimler `form-eylemler-*`:
  - yalnız bağlanan malik listesi;
  - kilitli alt katmanın soluk düğmeleri ve gerekçesi;
  - satırda Sil onayı;
  - tasarımcının Eylemler satırı.
