# ADR 0112: Masaüstünde kataloğun proje formları: bilgiler, kopya ve öbür saklama biçimi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** ADR 0028 (proje kataloğu ve yaşam döngüsü), 0039 (öbür saklama biçimine dönüştürme), 0086 (masaüstünün kataloğu), 0087 (Geçmiş sekmesi).
- **Kaynak:** web'in `ui/cloud/ProjectForms.ts`; sözleri ve kuralları `app/cloud/formsPlan.ts`, etiketlerin okunuşu `catalog.ts`'in `parseTags`'i; ortak durumlar `fixtures/cloud/v1/forms.json`.

## Bağlam

Masaüstü kataloğunun seçili proje bölmesinde üç düğme kapalıydı: “Bilgileri düzenle…”, “Kopyasını oluştur…” ve saklama biçimine göre “PostGIS'e aktar…” ya da “Dosya projesine çevir…”. İpuçları “Masaüstüne henüz taşınmadı” diyordu. Bu işler için web'e gitmek gerekiyordu.

## Karar

Üç form, kataloğun üstünde açılır. Geçmiş sekmesinin formları gibi çizilir. Her biri tek bir ürün komutu gönderir. Değerleri sunucu denetler ve düzeltir; ret sunucunun sözleriyle söylenir. Formların sözleri ve kararları `cloud/forms_plan.rs`'tedir; bu, web'in `formsPlan.ts`'inin karşılığıdır. İki platform `fixtures/cloud/v1/forms.json`'u oynatır (`forms_plan_tests.rs`).

Formlar yalnız planın izin verdiği projede açılır. Planın kuralları:

- Bilgileri düzenle `project.edit` ister; arşivlenmiş projede kapalıdır.
- Kopyasını oluştur ve dönüştürme `project.download` ister.

### Proje bilgileri (`project.metadata.update`)

- **Alanlar:** ad, tür (türün bir modül açmadığı notuyla), açıklama ve etiketler.
- **Etiketler:** virgülle ayrılır; her parça kırpılır, içindeki boşluklar teke iner, boşlar düşer.
- **Gönderilen:** yalnız değişenler. Ad kırpılarak, etiketler sırasıyla karşılaştırılır. Kaydet, ad boş değilken ve bir şey değişmişken açıktır.
- **Sürüm:** gönderim, formun gösterdiği katalog sürümüyle (`@catalog`) yapılır. Sunucuda daha yeni bir sürüm varsa hiçbir şey yazılmaz. Pencere web'in sözüyle yenilemeyi önerir.
- **Açık veritabanı projesi:** yeni ad önce projenin kendiliğinden kaydıyla gider, kendisiyle çakışmasın diye (web'in `updateMetadata`'sı). Kalanı, o adın yaptığı katalog sürümüyle ardından gider. Ad kaydedilemezse form durum çubuğundaki kayıt durumunu gösterir.
- **Açık dosya projesi:** adı kataloğundur. Çizim yeni adı sessizce alır.

### Kopyasını oluştur (`project.duplicate`)

- Kopyanın adı önerilir: `<ad> (kopya)`, seçili gelir.
- Çalışma alanları, hesabın proje açabildiği yerlerdir: etkin, koltuklu, `project.create`. Kaynağınki önce gelir. İkiden azsa liste kapalıdır; hiç yoksa form nedenini söyler.
- Kopya kullanıcınındır. “Projelerim”de seçili gösterilir, kendiliğinden açılmaz.

### Öbür saklama biçimi (`project.convert`)

- **Dosya projesi:** “PostGIS'e aktar”. Sınırı aşan bir nesne dosyadaki yeriyle söylenir (`entities[i]` → “dosyanın i+1. nesnesi; hiçbir proje oluşturulmadı”).
- **Veritabanı projesi:** “Dosya projesine çevir”.
- **Formun söyledikleri:** web'in ne olacağına dair maddeleri. Açık projede kaydedilmemiş değişiklik varsa, bunun aktarılmayacağı da yazar.
- **Ad:** isteğe bağlıdır; boşsa sunucu kaynağın adını biçimiyle verir.
- **Açık projenin gönderilmemiş değişiklikleri:** önce gider (web'in `settle`'ı).
- **Sonuç:** yeni proje “Projelerim”de seçilir ve açılır (web'in `openMade`'i).

Geçmiş sekmesinin “Yeni proje olarak geri yükle” formu da çalışma alanlarını aynı planla (`creatable_places`) sunar.

## Sonuçlar

- Kataloğun bütün düğmeleri masaüstünde çalışır.
- Web'in “Yeniden adlandır” penceresinin sözleri de planda ve fixture'dadır. Masaüstünün açık proje penceresi (ADR 0073) bu sözlerle zaten aynıdır.

## Doğrulama

- **`cloud::forms_plan_tests`:** `forms.json`'un bütün bölümleri:
  - sözler, sınırlar, etiketler, kopya adları, sayılar;
  - yama ve düğmeler;
  - çalışma alanları, dönüştürme formu;
  - satırlar ve hatalar.
- **`cloud::catalog_forms_tests`:**
  - yama ve sürüm çakışması;
  - kopyanın “Projelerim”de seçilmesi;
  - PostGIS'e aktarmanın nesne hatası ve açılış;
  - Vazgeç ve Esc'in bekleyen isteği korunması.
- **Görüntüler:** `cloud::catalog_forms_tests::screens` (`.run/shots/bulut-form-*`), web'in `form-*` resimleriyle karşılaştırıldı; koyu ve açık tema, 1440×900 ve 1100×650.
- **Canlı:** `apps/desktop/scripts/cloud-live.sh` (`live_run.rs`, `forms_live`) gerçek `kentosd` ile:
  - veritabanı projesinin etiketleri kaydedilir ve listede görünür;
  - kopyası “Projelerim”de seçilir, açılmaz;
  - dosya projesi PostGIS'e aktarılır ve açılır;
  - yapılanlar koşunun sonunda kalıcı silinir;
  - görüntüler `.run/shots/bulut-34…37-*`.
