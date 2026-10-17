# ADR 0445: Formu yayından kaldırma ve bağ tablosu

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-BB`; `plan/form.md` FORM-PUB-13, FORM-REL-02.
- **Bağlam belgesi:** ADR 0401 (formlar), 0405 ve 0440 (ilişkiler ve silme politikası), 0410 (geometrisiz kayıt), 0438 §7 (açılan
  yayın), 0444 (ilişkili kayıtların sayısı).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** yayından kaldırmanın saha oturumlarına ve çevrimdışı taslaklara etkisi, hedef kitle ve geri alma zamanı (bulut);
  bağ tablosunun sunucuda veritabanı kısıtıyla zorlanması.

## Bağlam

Bir form yayınlandıktan sonra durdurulamıyordu: kurum bir formu kullanımdan çekmek istediğinde ya formu silmesi (taslağı ve
yayınlarıyla) ya da kuralları uygulamayı kapatıp herkesin formu kullanmayı bırakmasını beklemesi gerekiyordu. Çoktan çoğa ilişki
çok değerli başvuruyla kuruluyordu; bağın kendi öznitelikleri (bir malikin parseldeki payı) tutulamıyordu.

## Karar

### 1. Yayından kaldır

`cad.forms.edit`'in `withdraw` işlemi (`on: true` kaldırır, `false` yeniden yayına alır); form `withdrawn`'ı tutar (yalnız `true`
yazılır). Kaldırılmış formla kayıt yazılmaz: `cad.forms.submit`, `.fill`, `.transition` ve `.recompute` `form_withdrawn` ile reddeder
(yol `form`, taslakla da, planda da). Doğrulama, karşılaştırma, dışa aktarma ve tasarım sürer; yeni yayın kaldırılmışlığı
değiştirmez. Yayını olmayan form kaldırılamaz (`form_unpublished`); kuralları katmanın yazma kuralı olan form kaldırılamaz
(`form_enforced`), kaldırılmış formun kuralları uygulanamaz (`form_withdrawn`, yol `edit.on`). Masaüstünde Formlar penceresinde
Yayından kaldır / Yeniden yayına al ve durum satırında “yayından kaldırıldı”; Formla düzenle kaldırılmış formla açılmaz ve nedenini
söyler; kaldırılmadan önce açılmış pencere yazılanı korur, Kaydet reddedilir.

### 2. Bağ tablosu

Bağın kendi kimliği, öznitelikleri ve sürümü olan çoktan çoğa ilişki yeni bir tür değildir: bağ satırları geometrisiz kayıt
katmanıdır (ADR 0410), iki tarafa iki bire çok ilişkiyle bağlanır (ADR 0440). Her bağ satırı bir kayıttır (kendi `uid`'i, öznitelikleri,
formu, geçmişi); iki ilişkinin silme politikaları ayrı seçilir (parsel silinince payları gider, payı olan malik silinmez). Python kabul
testi bu kuruluşu baştan sona sınar.

## Denetimler

- `scripts/fixtures/forms_withdraw_cases.py --check`: `cad.forms.edit.withdraw.json` 20 durum (kaldırma, dört yazma komutunun reddi,
  doğrulama, yeniden yayına alma, yayınsız ve kuralları uygulanan form, kuralların uygulanamaması, yeni yayın, taslak, plan, başka
  form).
- Yetenek testi bir adımın kendi komutunu (`command`) o komutun kodlarına sayar.
- Masaüstü: `a_withdrawn_form_opens_no_record_and_an_open_window_keeps_what_was_typed`; resim `form-yayindan-kaldirildi`.
- Python: `test_forms_acceptance.py`'nin `test_a_link_table_carries_its_own_rows`'u (elle yazılmış beklentiler).

## Sonuçlar

- Kurum bir formu silmeden kullanımdan çeker; açık işler kaybolmaz.
- Bağın öznitelikleri olan çoktan çoğa ilişki var olan parçalarla kurulur ve her girişte aynı kurallarla yazılır.
