# ADR 0417: Değiştirilebilir hesap

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-Y`; `plan/form.md` FORM-RULE-08, FORM-RULE-11.
- **Bağlam belgesi:** ADR 0402 (kurallar ve hesaplar), 0401 §8 (Kuralları uygula), 0411 §4 (sunucunun bekçisi).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Formun hesabı (`calculate`) alanı her yazmada yeniden hesaplar ve kilitler. Bina yüksekliği kattan tahmin edilir ama ölçülünce
ölçülen yazılmalıdır; QGIS'te bu “varsayılan değer” ile yapılır: yalnız oluşturmada hesaplanır ya da “güncellemede uygula” ile
her seferinde üzerine yazılır; ikisi arasında, kullanıcı yazana dek hesabı izleyen bir yol yoktur.

## Karar

### 1. Anlam

Alan öğesinin `overridable`'ı (yalnız `calculate` ile; yoksa ön denetimde `form_rule_conflict`, kural `overridable`):

- hesap alanı **boşken** doldurur; alan kilitli değildir, yazılan değer kalır, hesabın girdileri değişince de;
- alan boşaltılınca yeniden hesaplanır;
- Kuralları uygula'da ve sunucunun bekçisinde değiştirilebilir alan elle değiştirilebilir (`form_read_only` yok); formun öbür
  kuralları (alanın türü, denetimler, zorunluluk) aynen işler.

Anlam durumsuzdur: kaydın kendisinden okunur; Python, MCP ve `cad.forms.submit` için alanı izletmek onu boş vermektir.

### 2. Formla düzenle

Pencere açılırken alan boşsa ya da hesabının verdiği değeri tutuyorsa hesabı **izler**: kullanıcı yazmadıkça girdiler değiştikçe
yeniden hesaplanır ve Kaydet onu boş göndererek yeniden hesaplatır. Alana yazılınca izleme biter, alanın yanında ↺ (Hesaba dön)
çıkar; ↺ ya da alanı boşaltmak izlemeyi geri getirir. İpucu “Boşken hesaplanır: …; üzerine yazılabilir”. Tasarımcıda Hesap'ın
altında **Değiştirilebilir**.

## Denetimler

- `scripts/fixtures/forms_cases.py --check`: ön denetimin iki, tamamlamanın iki, kaydın bir, yazmanın üç ve bekçinin bir yeni durumu
  (`fixtures/forms/v1/cases.json`) bağımsız başvurudan; çekirdek (`kentos-forms`) aynı dosyayı geçer.
- Masaüstü: `form_scenes`'in `an_overridable_calculation_follows_until_typed_over`; resim `form-hesap-degistir`.

## Sonuçlar

- Hesap öneri gibi çalışır, ölçülen değer korunur; kullanıcı hangi değerin hesaptan geldiğini ↺ ile görür.
- Açık: hesabın değeri yazılan değerden farklıyken formda ikisini birlikte göstermek.
