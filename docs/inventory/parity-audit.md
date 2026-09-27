# Masaüstü–web eşdeğerlik denetimi

Güncelleme: 28 Eylül 2026. Envanterde (`docs/inventory/web.json`) masaüstünde “var” ya da “kısmen” görünen web ekranları denetlendi. Denetim, main'in sıraladığı ekranlarla başladı:

- Katmanlar ve Öznitelikler panelleri;
- Komut satırı ve alt panel;
- İşlemler paneli ve araç penceresi;
- Stil yöneticisi ve Katman stili;
- dört Hesap penceresi;
- bulutun katalog, proje ve Paylaş pencereleri;
- Uygulama ayarları, Proje ayarları ve Yeni proje;
- Başlangıç ekranı, üzerine gelme kartı ve imleç yanındaki değer alanı.

Sonra uygulama menüsü, Klavye kısayolları, KentOS CAD hakkında, durum çubuğu, komut şeridi, yerinde yazı düzenleyicisi, dosya alışverişi pencereleri ve İfade oluşturucu da sözleriyle tarandı.

Denetim yalnız okur; masaüstünün kodu değişmedi. Masaüstünün resimleri `.run/shots`'tadır.

**Sonradan kapananlar** (güncel liste TODOS.md UX-13'tedir):

- 28 Eylül: 1., 2. madde ve B1–B4 ([ADR 0119](../adr/0119-desktop-file-revisions.md)); reddedilen yüklemeden sonra istek anahtarı.
- 28 Eylül: 3. madde (N1), web'in `newProjectNote.ts` kuralıyla (`apps/desktop/src/project/new.rs`).
- 28 Eylül: 4. maddenin yanlış sözü (A1): web'in Ayar dosyası görünüş tercihlerinin masaüstüne taşınmadığını söyler; masaüstü, dosyadaki web'e özgü değerleri sessizce atlamaz, adlarıyla söyler. Anahtarların birleşmesi TODOS.md UX-13'ün Görünüş maddesindedir.
- 28 Eylül: 9. (A4), 12. (Ç1), 13. (K1, K2), 14. (İ2), 15. (İ1, İ3, İ4) maddeler; S1, Nokta hesabı çipinin ve Koordinat listesi al'daki virgülün ipuçları. Araç kutusunun kategori, model ve araç satırları da web'inki gibi ipucu taşır; İşlemler geçmişi çalıştırmanın saatini yazar.

## Yöntem

- Her ekranın web kodu okundu. Her davranış (düğme, menü öğesi, tuş, soru, ileti, durum) masaüstünün kodunda arandı ve okunarak doğrulandı.
- Ekranların Türkçe sözleri de masaüstünün kaynağında (`apps/desktop/src`, `crates/ui`, `crates/native`, `crates/shared`) bir betikle arandı. Bulunmayan her söz okunarak denetlendi. Görünmeyen `aria-label`'lar ve CSS sınıfları sayılmadı.
- Masaüstü aynı işi başka sözle yapıyorsa fark “görünüş” sayıldı.
- **Önem:**
  - veri kaybı (iş kaybolur);
  - yanlış sonuç (yanlış iş ya da yanıltıcı söz);
  - eksik iş (web'in yaptığı bir iş masaüstünde yok);
  - görünüş (söz, ipucu, düzen).
- **Plan/fixture:** davranışı iki platformda bağlayan ortak dosya var mı. “yok”, masaüstü onu web'in kodundan okuyarak kuracak demektir.
- Satır numaraları 28 Eylül'deki ağaca göredir (web `apps/web/src/` altında, masaüstü `apps/desktop/src/` altında).

## Özet

Ortak bir planı ve fixture'ı olan ekranlarda web'den ayrılan bir davranış bulunmadı. Farklar planı olmayan ekranlarda toplanıyor. Veri kaybettiren bir fark bulunmadı. Yanlış sonuç iki yerde var: Yeni proje penceresinin bulut notu ve iki platformda başka anahtarlarla tutulan görünüş tercihleri.

| Ekran | Denetim | Veri kaybı | Yanlış sonuç | Eksik iş | Görünüş |
|---|---|---|---|---|---|
| Katmanlar paneli | tam | – | – | – | 2 |
| Öznitelikler paneli | tam | – | – | – | – |
| Komut satırı, alt panel | tam | – | – | – | 1 |
| İşlemler paneli, araç penceresi | tam | – | – | – | 4 |
| Stil yöneticisi, Katman stili | örneklem | – | – | – | – |
| Hesap pencereleri (dört) | tam | – | – | – | – |
| Bulut: dosya projesi (kayıt hücresi, çakışma) | tam | – | – | 3 | 1 |
| Bulut: Buluta yükle | tam | – | – | 1 | 3 |
| Bulut: veritabanı çakışması, erişim kalktı | tam | – | – | – | 1 |
| Bulut: katalog, geçmiş, formlar, Paylaş | sözler | – | – | – | – |
| Uygulama ayarları | tam | – | 1 | 2 | 3 |
| Proje ayarları | tam | – | – | 2 | – |
| Yeni proje | tam | – | 1 | – | – |
| Başlangıç ekranı | tam | – | – | – | 1 |
| Üzerine gelme kartı, değer alanı | tam | – | – | – | – |
| Klavye kısayolları | tam | – | – | 1 | 1 |
| Uygulama menüsü, hakkında, durum çubuğu, komut şeridi | sözler | – | – | – | – |

“Örneklem”: fixture'lı kurallar (`classify`, `tally`, `kstil`, `legend`, `designer`) sınandığı için pencereden örnek davranışlar denetlendi. “Sözler”: yalnız sözlerin taraması ve bulunmayanların okunması.

## En önemli 15

1. **Dosya projesinde başkasının yeni revizyonu masaüstünde hiç duyulmuyor.**
   - Masaüstü dosya projesinin olaylarını izlemiyor. Kayıt hücresi hiçbir zaman “Yeni revizyon: rN” demiyor (`cloud/cells.rs:120, 187, 223, 239`: `newer_revision: None`).
   - Çakışma ancak Kaydet'te, dosya yüklendikten sonra ortaya çıkıyor.
   - Web: `app/cloud/fileProject.ts` (`receive`, `askNewest`), `ui/statusbar/cellsPlan.ts:69`.
   - Önem: eksik iş. Plan: [docs/specs/file-revisions.md](../specs/file-revisions.md), `fixtures/cloud/v1/file-revisions.json`.
2. **Dosya çakışması sorusunda “Yerel dosyaya kaydet” yok.**
   - Masaüstünde üç cevap var: Vazgeç, Sunucudaki son revizyonu aç, Ayrı proje olarak kaydet (`cloud/view.rs:280–305`).
   - Web'de dört cevap var; dördüncüsü Farklı kaydet'tir ve çizimi buluttan ayırır (`app/cloud/fileRevisionsPlan.ts:357`, `conflictQuestion`).
   - Başlık ve sözler de ayrı (“Kayıt çakışması”, web'de “Dosya başka biri tarafından kaydedildi”).
   - Önem: eksik iş. Fixture: `file-revisions.json` (`offers`).
3. **Yeni proje, salt okunur veritabanı projesi üstünde yanlış söylüyor.**
   - Masaüstü her veritabanı projesinde “bekleyen değişiklikleri buluta gönderilir” diyor (`project/new.rs:263–267`). Hesap yazamıyorsa bu değişiklikler gönderilemez.
   - Web yazma hakkına bakar: “… değişiklikleriniz buluta kaydedilmiyor (salt okunur); Oluştur'a basınca ne yapılacağı sorulur” (`ui/settings/NewProjectDialog.ts:83–91`, `cloud.autosaves()`).
   - Web'in kendi hatası için aşağıdaki “Web'de düzeltilecekler”e bakın.
   - Önem: yanlış sonuç. Plan: yok.
4. **Görünüş tercihleri iki platformda başka anahtarlarda.**
   - Web: tema yerleşimde (`app/layoutPlan.ts:52`, `kentos.ui.v1`); vurgu `appearance.accent`, yazı tipi `appearance.uiFont`, yazı boyu `appearance.uiScale` (beş adım). Yalnız web'in anahtarlarıdır (`crates/shared/contracts/src/settings/schema.rs:288, 303, 331`).
   - Masaüstü: `appearance.theme`, `appearance.accentColor`, `appearance.typeface`, `appearance.monoTypeface` ve `appearance.textSize` (piksel). Yalnız masaüstünün anahtarlarıdır (`schema.rs:218–273`).
   - Web'in Ayar dosyası “Masaüstü uygulaması da aynı dosyayı okur” der (`ui/settings/settingsFileSection.ts:94`). Oysa bu tercihler dosyayla öbür platforma geçmez; masaüstü bilinmeyen anahtarları sessizce atlar (`settings_view.rs:241–245`).
   - Önem: yanlış sonuç. Fixture: `fixtures/settings/v1` (bu anahtarlar ortak değil). Yazı tipi main'in işinde (`user.uiFont`, TODOS.md UX-13).
5. **Buluta yükle penceresinde proje türü, açıklama ve etiketler yok.**
   - Web yükleme penceresi katalog alanlarını sorar (`ui/cloud/UploadDialog.ts:74, 100`, `catalogFields`).
   - Masaüstünde yalnız çalışma alanı, ad ve saklama biçimi var (`cloud/view.rs:162–232`). Yeni proje varsayılan türle açılır; açıklama ve etiketler sonradan Proje bilgileri'nden yazılır.
   - Önem: eksik iş. Fixture: `forms.json` bu alanların okunuşunu tutar; yükleme penceresi için yok.
6. **Ayar pencerelerinde “Bu bölümü varsayılana döndür” yok.**
   - Web'in iki ayar penceresi her bölümü ayrı ayrı varsayılana döndürür (`ui/settings/SettingsShell.ts:67, 123–126`).
   - Masaüstünde Uygulama ayarları yalnız bütün ayarları döndürür (`settings_view.rs:141–153, 520`). Proje ayarlarında hiç yok (`project/settings.rs`).
   - Önem: eksik iş. Plan: yok.
7. **Ayar pencereleri birbirine yol vermiyor.**
   - Web'de Uygulama ayarları'nın Yeni projeler bölümünde “Açık proje: Bu projenin sistemi … [Proje ayarlarını aç]” satırı var (`ui/settings/AppSettingsDialog.ts:62–63, 78–81`).
   - Web'de Proje ayarları'nda “Yeni projelerin varsayılanı … [Uygulama ayarlarını aç]” satırı var (`ui/settings/ProjectSettingsDialog.ts:80, 91`).
   - Masaüstünde düğmeler yok. Proje ayarlarında yalnız söz var (`project/settings.rs:338–345`).
   - Önem: eksik iş. Plan: yok.
8. **Klavye kısayolları penceresinde arama ve fare yok.**
   - Web'in “Fare ve klavye kısayolları” penceresinde şunlar var (`ui/dialogs.ts:8–26, 45, 48–92`):
     - arama;
     - 17 satırlık fare tablosu (basılı sağ tuşun menüsü, Shift + sağ tık kenet, kenette bekleyerek izleme, tutamaca sağ tık…);
     - kategorilere ayrılmış komutlar, ikonları ve takma adları;
     - Türkçe klavye notu.
   - Masaüstünde düz bir kısayol listesi var (`view.rs:959–977`). Gizli fare hareketleri masaüstünde hiçbir yerde anlatılmıyor.
   - Önem: eksik iş. Plan: yok.
9. **Uygulama ayarları kaydedilince yeni proje varsayılanları söylenmiyor.**
   - Web: “Yeni projeler … ile oluşturulacak. Açık projenin sistemi değişmedi.” ve çalışma modunun satırı (`AppSettingsDialog.ts:126–132`). CLAUDE.md §4.4'ün “varsayılanı değiştirmek açık projeyi değiştirmez” kuralını kullanıcıya söyleyen tek yer burasıdır.
   - Masaüstü yalnız “Uygulama ayarları kaydedildi.” der (`settings_view.rs:302–306`).
   - Önem: görünüş (yanıltmaz ama eksik).
10. **Uygulama ayarları'nın düzeni web'inki değil.**
    - Web'de şunlar var (`AppSettingsDialog.ts:34–103, 140–160, 195–275`; `SettingsShell.ts:57–64`):
      - solda bölüm listesi, her bölümün başlığı ve açıklaması;
      - kapsam kutusu (“Bu tarayıcıda saklanır · Tüm projeler için geçerlidir”);
      - önizlemeli tema kartları;
      - örnek yazılı yazı tipi seçicisi;
      - açıklamalı “Fare yardımcıları” ve kenet türleri.
    - Masaüstünde iki sütunlu tek form var; ek olarak oturum anahtarları ve çizim zemini de orada (`settings_view.rs:318–560`).
    - Önem: görünüş. Klasik/Şerit seçimi masaüstünde anlamsız (sahibin kararı, 27 Eylül).
11. **Buluta yükle penceresinin sözleri ve ilerlemesi ayrı.**
    - Dosya saklamada birincil düğme “Buluta dosya olarak kaydet” olmuyor (web `UploadDialog.ts:114`, masaüstü `cloud/view.rs:226–229`).
    - Saklama başlığında “(sonradan değişmez)” yok (web `:99`).
    - Seçilen biçimin sonucunu söyleyen ayrı not yok (web `:115–118`; masaüstünde genel bir not var, `:204–207`).
    - İlerleme yüzdesiz (web `:190`, masaüstü `:208–210`).
    - Önem: görünüş.
12. **Veritabanı çakışma penceresinin sözleri ayrı.**
    - “Benimkini koru”, web'de “Benimkini kaydet”.
    - “Kimlik başka nesnede”, web'de “sunucuda zaten var”.
    - Nedenler büyük harfle başlıyor.
    - Web `ui/cloud/ConflictDialog.ts:14–19, 42, 56`; masaüstü `cloud/view.rs:270–274`, `cloud/words.rs:196–203`.
    - Önem: görünüş. Plan: yok.
13. **Katmanlar panelinde adın ipucu ve menüdeki tuşlar yok.**
    - Web'de adın üstünde katmanın yolu (grup › katman) görünür (`ui/layers/LayersPanel.ts:268`).
    - Web'in satır menüsü tuşları gösterir: Gizle/Göster (Boşluk), Yeniden adlandır (F2), Sil (Delete) (`:293, 329, 342`).
    - Masaüstünde ikisi de yok (`view.rs:601`, `layering.rs:328–397`). Tuşların kendisi çalışıyor (`layer_tree.rs:175–252`).
    - Önem: görünüş.
14. **İşlemler geçmişinin süresi virgülle yazılıyor.**
    - Masaüstü “1,2 sn” yazar (`processing/panel.rs:240–245`). Web “1.2 sn” yazar (`ui/processing/ProcessingPanel.ts:222`). Uygulamanın kuralı ondalık noktadır (Proje ayarlarının notu).
    - Önem: görünüş.
15. **İşlemler panelinde ipuçları ve bir başlık eksik.**
    - Yeniden aç'ın ipucu (“Aynı değerlerle pencereyi açar”) ve “n nesneyi seç”in ipucu (“Bu çalıştırmanın eklediği nesneleri seçer”) yok (web `ProcessingPanel.ts:211–216`, masaüstü `processing/panel.rs:248–259`).
    - Modeller'in açıklaması yok (web `:25`).
    - Çıktı katmanı seçiminde “Mevcut katmanlar” başlığı yok (web `ui/processing/dialogTexts.ts:57`, masaüstü `processing/fields.rs:365–412`).
    - Önem: görünüş.

## Ekran ekran

### Katmanlar paneli (`ui/layers/LayersPanel.ts` ↔ `layering.rs`, `layer_tree.rs`, `view.rs:543–641`)

Aynı bulunanlar:

- başlıktaki sayı ve iki düğme, kapalı olunca nedeni;
- Katman ara (Türkçe küçük harf, ↓ ağaca girer, Esc);
- satırlar: renk kutusu ve menüsü, göz, kilit, nesne sayısı (grupta toplam), etkin çubuk, gizli grubun altındakiler soluk;
- satır menüsü öğe öğe;
- Sil'in retleri, sorusu ve iletileri;
- yerinde yeniden adlandırma (Enter, Esc, dışarı tıklayınca kaydeder);
- çift tık;
- ağacın tuşları;
- çizimdeki seçimi izleme;
- `project.edit` olmayınca ağacın kilitlenmesi.

Farklar:

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| K1 | Adın ipucu katmanın yoludur (`LayersPanel.ts:268`) | yok (`view.rs:601`) | görünüş | yok |
| K2 | Menü Boşluk, F2 ve Delete'i gösterir (`:293, 329, 342`) | yok (`layering.rs:328–397`) | görünüş | yok |

### Öznitelikler paneli (`ui/properties/PropertiesPanel.ts` ↔ `properties/`)

Fark bulunmadı. Denetlenenler:

- boş seçim ve Çizim bölümü;
- tek nesnede `#id` ve özet;
- çoklu seçimde tür dağılımı, ortak katman, renk ve sembol, Toplamlar;
- Katman ▾ (kilitli katmanlar kapalı), Renk ▾, Sembol ▾ (Kitaplıktan seç…, Katman stili…);
- kilitli katmandaki nesne (“(kilitli)”, düzenleyici yok);
- on üç türün geometri satırları (alanın ikinci birimi dahil) ve düzenlenen alanları (nokta Y/X, ölçü, tarama, yazı);
- Parsel/Ada özniteliği değişince etiketin izlemesi.

### Komut satırı ve alt panel (`ui/bottom/` ↔ `bottom.rs`, `message_log.rs`, `view.rs:643–718`)

Fark bulunmadı. Günlük ve yerleşim ortak planla tutuluyor (`fixtures/shell/v1/log.json`, `layout.json`). Ayrıca denetlenenler:

- öneriler, ↑/↓ ile yazılanları geri getirme, Tab, Boşluk'un Enter olması, Esc;
- istemin seçenek düğmeleri;
- şerit kapalıyken “Nokta hesabı” ve “Sonraki tık: …” çipleri;
- koordinat listesi (noktalar, köşeler, kenar ve semt, alan, çevre, “ilk nesne gösteriliyor”).

Yalnız Nokta hesabı çipinin ipucu yok (web `CommandLine.ts:131`).

### İşlemler paneli ve araç penceresi (`ui/processing/` ↔ `processing/`)

Pencere planla tutuluyor (`fixtures/processing/v1`). Panelin geçmişi web'inki: “Yeniden aç” kaldırılmış araçta kapalı, “n nesneyi seç” seçip yakınlaştırır.

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| İ1 | Geçmiş düğmelerinin ipuçları (`ProcessingPanel.ts:211–216`) | yok (`processing/panel.rs:248–259`) | görünüş | yok |
| İ2 | Süre “1.2 sn” (`:222`) | “1,2 sn” (`panel.rs:240–245`) | görünüş | yok |
| İ3 | Çıktı katmanında “Yeni katman” ve “Mevcut katmanlar” başlıkları (`dialogTexts.ts:56–57`) | başlıksız liste (`processing/fields.rs:365–412`) | görünüş | `fixtures/processing/v1` sözleri tutmuyor |
| İ4 | Kaldırılmış model: “Silinmiş olabilir” (`ToolDialog.ts:63`) | “İşlem aracı bulunamadı: {id}” (`processing/mod.rs:244`) | görünüş | yok |

“Arka planda çalışıyor” ve “Nerede çalışır” web'in işçisine özgüdür; masaüstünde anlamsız.

### Stil yöneticisi ve Katman stili (`ui/style/` ↔ `style/manager/`, `style/layer_style/`)

Örneklemde fark bulunmadı. Sınıflama, sayım, `.kstil`, lejant ve sembol tasarımcısı ortak fixture'larla tutuluyor. Denetlenenler:

- beş işleyici;
- kategorili ve aralıklı sınıflama (yöntem, sınıf sayısı, rampa, Sınıfla);
- ifade alanlarının Alanlar, Değişkenler ve İşlevler menüleri;
- Kurallar: açık/kapalı, değilse, ölçek aralığı, ƒ menüsü, yukarı/aşağı, alt kural, sil;
- yöneticinin kopyala, dışa ve içe aktar, uygula.

Söz taramasında çıkanların çoğu görünmeyen `aria-label`'dı. Tarayıcı panosunun iletileri web'e özgüdür.

### Hesap pencereleri (`ui/calc/` ↔ `calc/`)

Fark bulunmadı. Denetlenenler:

- bilinen nokta alanı (ad ya da Y,X, altında sonucu, Çizimden: pencere kapanır, nokta gösterilir, pencere aynı hâliyle döner, kenete oturunca noktanın adı);
- ölçü tablosu (sabit hücreler, Enter ve ↓ ile alt satır, sona satır ekleme, ↑, elektronik tablodan yapıştırma, okunamayan sayı kırmızı, satır silme, Satır ekle);
- sonuç tabloları, açı ve kapanma birimleri (cc, saniye, mm);
- katman seçimi (kilitli kapalı);
- noktaların tek adımda, Ad, Tür ve Z (m) öznitelikleriyle eklenmesi; kilitli ve gizli katman iletileri;
- raporun panoya kopyalanması.

### Bulut: dosya projesi (kayıt hücresi, çakışma)

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| B1 | Başkasının revizyonu duyulur ve söylenir: hücre, ipucu, günlük; kaydedilmemiş iş üstünde de; yeniden eşitleme çizimi değiştirmez (`app/cloud/fileProject.ts`, `fileRevisionsPlan.ts:94–256`) | dosya projesinin olayları izlenmiyor; `Outdated` hiç hesaplanmıyor (`cloud/cells.rs:176–245`) | eksik iş | `file-revisions.json` (`traces`, `events`, `resync`) |
| B2 | Yeni revizyon biliniyorken Kaydet yüklemeden sorar (`fileRevisionsPlan.ts:164`, `saveStep`) | B1 olmadan bilinemez; çakışma yüklemeden sonra gelir | eksik iş | `file-revisions.json` (`saveSteps`) |
| B3 | Çakışma sorusu: dört cevap, kaydeden ve zaman, çizimin revizyonu (`fileRevisionsPlan.ts:341–365`) | üç cevap, “Yerel dosyaya kaydet” yok; başlık ve sözler ayrı (`cloud/view.rs:280–305`) | eksik iş | `file-revisions.json` (`offers`) |
| B4 | Geçmiş sekmesinde “Açık çizim” işareti (`ui/cloud/catalogHistory.ts`, `revisionMarks`) | yok | görünüş | `file-revisions.json` (`historyMarks`) |

Masaüstünün cihazda tutulan kaydı (ADR 0043) web'de yoktur; web çevrimiçi kalır.

### Bulut: Buluta yükle (`ui/cloud/UploadDialog.ts` ↔ `cloud/upload.rs`, `cloud/view.rs:162–232`)

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| Y1 | Proje türü, Açıklama, Etiketler (`:74, 100`) | yok | eksik iş | `forms.json` (alanların okunuşu) |
| Y2 | Dosya saklamada birincil düğme “Buluta dosya olarak kaydet”; seçilen biçimin sonucu ayrı not (`:114–118`) | hep “Buluta yükle”; genel not | görünüş | yok |
| Y3 | “Saklama biçimi (sonradan değişmez)” (`:99`) | “Saklama” | görünüş | yok |
| Y4 | İlerleme yüzde ve boyutla (`:187–191`) | belirsiz çubuk, “Buluta gönderiliyor: n nesne, x MB” | görünüş | yok |

Masaüstü burada önde:

- yükleme durdurulabiliyor;
- yeniden deneme aynı idempotency anahtarıyla gidiyor;
- reddedilen çizim sunucuda boş proje bırakmıyor (`cloud/upload.rs:1–13`).

Web boş projeyi bırakıp silmeyi soruyor (`UploadDialog.ts:139–181`).

### Bulut: veritabanı çakışması, erişim kalktı

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| Ç1 | “Benimkini kaydet”; nedenler küçük harfle, “sunucuda zaten var” (`ConflictDialog.ts:14–19, 42, 56`) | “Benimkini koru”; “Kimlik başka nesnede” (`cloud/view.rs:270–274`, `cloud/words.rs:196–203`) | görünüş | yok |

Erişim kalktı bildirimi aynı: başlık, kalanlar, “Yerel kopya kaydet…” ve Tamam (`AccessLostNotice.ts` ↔ `cloud/view.rs:98–114`, `cloud/follow.rs:548–600`).

### Bulut: katalog, geçmiş, formlar, Paylaş

Katalog, proje formları, Paylaş ve durum hücreleri ortak planla tutuluyor (`catalog.json`, `forms.json`, `share.json`, `cells.json`). Söz taramasında görünür bir eksik çıkmadı. Tek not: Geçmiş'in “Açık çizim” işareti (B4).

### Uygulama ayarları (`ui/settings/AppSettingsDialog.ts`, `SettingsShell.ts` ↔ `settings_view.rs`)

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| A1 | Görünüş tercihleri web'in anahtarlarında; ayar dosyası “masaüstü de okur” der (`settingsFileSection.ts:94`) | başka anahtarlar; dosyadaki web anahtarları sessizce atlanır (`settings_view.rs:241–245`) | yanlış sonuç | `fixtures/settings/v1`; eşleme yok |
| A2 | Bölüm bölüm varsayılana döndürme (`SettingsShell.ts:67, 123–126`) | yalnız hepsi (`settings_view.rs:141–153, 520`) | eksik iş | yok |
| A3 | Açık projenin sistemi ve “Proje ayarlarını aç” (`AppSettingsDialog.ts:62–63, 78–81`) | yok | eksik iş | yok |
| A4 | Kaydedince yeni proje varsayılanlarının açık projeyi değiştirmediği söylenir (`:126–132`) | söylenmez (`settings_view.rs:302–306`) | görünüş | yok |
| A5 | Bölüm listesi, açıklamalar, kapsam kutusu, tema kartları, örnek yazılı yazı tipi (`:34–103, 140–205`) | tek form (`settings_view.rs:318–560`) | görünüş | yok |
| A6 | Yazı boyu beş adım (Küçük … En büyük, `:217–233`) | piksel sayısı (`settings_view.rs:460–474`) | görünüş | A1'e bağlı |

Çizim motorunun arka ucu (WebGL2 ve WebGPU) ve Klasik/Şerit seçimi masaüstünde anlamsız. Kenet türleri, açı adımı, yakalama yarıçapları, fare yardımcıları ve Başlangıç ekranı her iki yanda var.

### Proje ayarları (`ui/settings/ProjectSettingsDialog.ts` ↔ `project/settings.rs`)

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| P1 | Bölüm bölüm varsayılana döndürme (`SettingsShell.ts:67`) | yok | eksik iş | yok |
| P2 | “Uygulama ayarlarını aç” (`:80, 91`) | yalnız söz (`project/settings.rs:338–345`) | eksik iş | yok |

Bölümler, açıklamaları, önizleme, sistem atama iletisi ve ondalık notu aynı.

### Yeni proje (`ui/settings/NewProjectDialog.ts` ↔ `project/new.rs`)

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| N1 | Yazamayan hesabın değişiklikli veritabanı projesinde “buluta kaydedilmiyor (salt okunur)” (`:83–91`) | her veritabanı projesinde “bekleyen değişiklikleri buluta gönderilir” (`project/new.rs:263–267`) | yanlış sonuç | yok |

Ad, ölçek, çalışma modu, aranan koordinat sistemi, katmanların notu ve boş ad iletisi aynı.

### Başlangıç ekranı (`ui/start/` ↔ `start.rs`)

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| S1 | Son dosyanın × düğmesinin ipucu “Listeden kaldır” (`recentList.ts:35`) | ipucu yok (`start.rs:308–312`) | görünüş | yok |

Eylemler, kısayolları, bulutun alt satırı, Çizime devam et, son dosyalar ve boş listesi, Açılışta göster aynı.

### Üzerine gelme kartı ve değer alanı (`ui/shell/HoverCard.ts`, `CursorInput.ts` ↔ `hover_card.rs`, `preview.rs`, `input.rs`)

Fark bulunmadı. Denetlenenler:

- kartın bekleyişi, içeriği ve kenarda öbür yana geçmesi (ADR 0068, 0082);
- değer alanının açılması, ipucu satırı, Enter ve Boşluk, Esc, Tab'ın alanda kalması;
- anlaşılamayan girdinin iletisi.

### Klavye kısayolları, KentOS CAD hakkında

| # | Web | Masaüstü | Önem | Plan/fixture |
|---|---|---|---|---|
| H1 | Arama, fare tablosu, kategoriler, ikonlar ve takma adlar (`ui/dialogs.ts:8–26, 45–92`) | düz liste, “(web)” işaretli (`view.rs:959–977`) | eksik iş | yok |
| H2 | Başlık “Fare ve klavye kısayolları”; Türkçe klavye notu (`:89–92`) | “Klavye kısayolları”; başka not | görünüş | yok |

Hakkında penceresinin satırları aynı (sürüm, çizim motoru, sistem, sunucu); masaüstü taşınan komut sayısını da yazıyor.

## Web'de düzeltilecekler

Denetim sırasında web'in kendi hataları da bulundu. İkisi de 28 Eylül'de web'de düzeltildi:

- **Yeni proje notu (`ui/settings/NewProjectDialog.ts:83–91`):** kaydedilmemiş değişikliği olan açık bir dosya projesinde “(salt okunur)” diyordu. Nedeni, `cloud.autosaves()`'ın dosya projesinde her zaman false olmasıydı. Dosya projesi yazılabilir; doğru not yerel çizimdeki gibi “kaydedilmemiş değişiklikler var; Oluştur'a basınca önce sorulur”dur.
  - Kural artık `ui/settings/newProjectNote.ts`'te ve sınanıyor.
  - Veritabanı projesi kaydetmiyorsa (salt okunur, arşiv, silinmiş) not “değişiklikleriniz buluta kaydedilmiyor; Oluştur'a basınca ne yapılacağı sorulur” der. Masaüstünün N1'i bu kurala uyacak.
- **Başarısız yükleme (`ui/cloud/UploadDialog.ts:139–181`):** sunucuda boş bir proje bırakıp silmeyi soruyordu. Artık masaüstünün yolundadır:
  - kesin retde boş proje çöp kutusuna gider;
  - yanıt gelmezse proje kalır ve aynı anahtarla yeniden denenir;
  - önceki denemenin içeriği yeniden gönderilmez (ADR 0038'in 28 Eylül notu).

## Denetlenmeyenler

- Model tasarımcısı (ADR 0116) ve SVG düzenleyicisi (ADR 0095) kendi planlarıyla yeni taşındı. Bu denetime girmediler.
- Şerit “kısmen”dir; farkları `docs/specs/ribbon.md` ve ADR 0117'dedir.
- Sembol tasarımcısı ve Lejant kendi fixture'larıyla tutuluyor; ayrıca taranmadı.
- Dosya alışverişi pencereleri ve İfade oluşturucu yalnız sözleriyle tarandı. Çıkanlar ya aynı notun başka sözleriydi ya da tarayıcıya özgüydü:
  - GeoJSON'un RFC 7946 notu ve koordinat sistemi uymayınca kapalı içe aktarma iki yanda var;
  - Koordinat listesi al'da virgül ayırıcıyken “Virgül” ondalığı iki yanda kapalı, ama masaüstünde nedenini söyleyen ipucu yok (web `ui/io/CoordImportDialog.ts:163`, masaüstü `exchange/coord_import.rs:567–572`; görünüş);
  - kaydetme penceresinin izni tarayıcıya özgü.
