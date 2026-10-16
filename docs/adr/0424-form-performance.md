# ADR 0424: Büyük form ve katmanda süreler

- **Durum:** kabul edildi (2026-10-17). Sahibin ilkesi: “yüksek performans ilk önceliğimiz”. TODOS.md §16.6 `FORM-AF`;
  `plan/form.md` FORM-QA-15.
- **Bağlam belgesi:** ADR 0401–0406 (formlar), 0421 (Hesapları yenile), 0422 (alt türler), 0423 (değer bileşimleri).

## Bağlam

Form çekirdeği bir kaydı her tuşta, katmanın bütün kayıtlarını Kayıtları denetle ve Hesapları yenile'de değerlendirir. Ölçülmüş bir
üst sınır yoktu; alt türler ve değer bileşimleri yeni kurallar ekledi.

## Karar

### 1. Ölçüm

`crates/native/application/tests/all/forms_timing.rs` (`forms_timing`, `--ignored`): 100 000 kayıtlı katman (otuz alan, 10 alt tür,
2 000 bileşimli kısıtlayıcı grup) ve 200 öğeli form (24 koşullu alan, 170 şablonlu not, hesap ve denetim). Ölçülenler ve bütçeleri
(ölçülenin yaklaşık iki katı; yavaşlatan değişiklik görünür):

| İş | Önce | Sonra | Bütçe |
|---|---|---|---|
| Formla bir kayıt (`cad.forms.validate`) | 1,35 ms | 1,28 ms | 5 ms |
| Bir tuş (`assess` ve `described`) | 0,163 ms | 0,119 ms | 0,5 ms |
| Kayıtları denetle, 100 000 kayıt | 2 727 ms | 884 ms | 1,8 s |
| Hesapları yenile'nin planı, 100 000 kayıt | 383 ms | 373 ms | 0,8 s |
| 10 000 kayda öznitelik yazma (alt tür ve bileşim kurallarıyla) | 106 ms | 104 ms | 0,4 s |

### 2. Giderilen darboğazlar

- `states` her öğenin kimliğini kopyalıyordu; iç denetimler (`validate`, `complete`'in gizlenenleri silmesi, `locked`,
  `unique_issues`) artık kimliksiz bayraklar (`Prepared::flags`) kullanır. Ekrana giden `states` aynıdır.
- Alanın kendi zorunluluğu dışında denetlenmesi (`loose`) alanı değer listesiyle birlikte her denetimde kopyalıyordu; zorunlu
  olmayan alan artık kopyasız denetlenir (`check_loose`).
- Alt türün zorunluluğu her alan için alanın kopyasıyla soruluyordu; kaydın alt türü bir kez bulunur, kural kopyasız okunur.

## Denetimler

- `cargo test --release -p kentos-native-application --test all forms_timing -- --ignored --nocapture --test-threads=1`.
- Davranış değişmedi: formların, alt türlerin ve bileşimlerin bütün ortak durumları ve masaüstü testleri geçer.

## Sonuçlar

- Formun büyüklüğüne ve katmanın kayıt sayısına göre üst sınırlar yazılı ve sınanıyor.
- Açık: koşulların katman boyunca sütun motoruyla (`kentos_expression`'ın partileri) değerlendirilmesi; web'in ölçümü.
