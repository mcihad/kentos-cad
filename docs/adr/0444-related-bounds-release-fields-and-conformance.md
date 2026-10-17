# ADR 0444: İlişkili kayıtların sayısı, yayının alanları, formun nasıl açıldığı ve pencere ile komutun eşdeğerliği

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-BA`; `plan/form.md` FORM-REL-01, FORM-PUB-01, -05, -17, FORM-STD-09.
- **Bağlam belgesi:** ADR 0404 (ilişkili kayıtlar), 0407 (eklerin sayısı), 0438 §7 (açılan yayın), 0440 (gönderimin alt kayıtları),
  0442 §2 (kırılan kayıtlar), 0443 (erişim ve kabul).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** ilişkinin sayısının sunucuda veritabanı kısıtıyla zorlanması, yayının sahibi, kurumu ve hedef kitlesi, izinlerin
  kaynağının gösterilmesi (bulut), web ve sunucu çizicileriyle eşdeğerlik.

## Bağlam

İlişkili kayıtlar öğesi bir kaydın alt kayıtlarını gösteriyordu ama kaç tane olabileceğini söyleyemiyordu: bire bir ilişki ya da
zorunlu alt kayıt formda ifade edilemiyordu. Yayın taslağın gövdesini ve özetini tutuyordu; katmanın alanları yayından sonra
değişince (aralık daralınca, değer listesi değişince) form eski alanlara göre yayınlanmış kalıyor ve kimse bunu görmüyordu. Formla
düzenle formun hangi yayınla açıldığını, kurallarının katmanın yazma kuralı olup olmadığını ve neden salt okunur olduğunu yalnız
dolaylı söylüyordu. Masaüstü penceresi ile `cad.forms.validate` aynı çekirdeği kullanıyordu ama bunu sınayan bir test yoktu.

## Karar

### 1. İlişkili kayıtların sayısı

`FormItem::Related`'ın `min` (0–1000) ve `max` (1–1000)'ı: öğe görünürken kayda başvuran nesnelerin sayısı bu aralıkta olmalıdır;
`max: 1` bire bir, `min: 1` zorunlu alt kayıttır. Sayılan: öğenin katmanında, alanı kaydın kimliğini tutan nesneler (çoktan çoğa
alanda kimliklerden biri), ve gönderimin alt kayıtları: henüz bağlı olmayan bağlanan +1, bağlı olan koparılan −1, öbürleri 0.
Az `form_related_few`, çok `form_related_many` (hata, öğenin `min` ya da `max` kuralı; ileti öğenin adını ve sayıyı söyler).
Kayıt bir komutla bütün olarak yargılanırken sayılır (formla yazma, doğrulama, Formla düzenle; belgenin form izlemesi saymaz,
eklerin sayısı gibi). Gönderimde ana kayıt alt kayıtlarından önce yargılanır; ilk sorun bütün gönderimi reddeder
(`form_invalid`, yol `values`). Formla düzenle'de pencerede bağlanan ya da koparılan da hemen sayılır; sorun öğenin başlığının
altında, uzun listeden önce görünür. Tasarımcıda ilişkili kayıtlar öğesinde En az ve En çok.

### 2. Yayının alanları

Yayın `fields`'ı tutar: gövdenin bağladığı katman alanlarının, katmandaki sıralarıyla, kanonik JSON'unun SHA-256'sının ilk 32 hanesi
(`bound_fields_digest`). Yayınla taslak son yayınla aynı olsa da bağlı alanlar değiştiyse yeniden yayınlar; özeti olmayan eski yayın
bu yüzden yeniden yayınlanmaz. `cad.forms.validate` son yayının alanları değiştiyse `fieldsChanged: true` der (taslakta yok).
Tasarımcı ve Formla düzenle aynı durumu uyarıyla söyler: formu gözden geçirip yeniden yayınlayın. Bağlanmayan alanın eklenmesi
yayını değiştirmez.

### 3. Formun nasıl açıldığı

Formla düzenle'nin üst satırındaki ⓘ: hangi yayınla (ya da taslakla) açıldığı, kurallarının katmanın yazma kuralı olup olmadığı,
formun gerektirdikleri ve bu bilgisayarın sağladığı, yazılabilir mi salt okunur mu ve nedeni (katman kilitli, iş akışı durumu),
alanların yayından sonra değişip değişmediği.

### 4. Pencere ile komutun eşdeğerliği

Masaüstü testi aynı değerleri Formla düzenle'ye yazar ve `cad.forms.validate`'e verir; kaydın sonucu, sorunlar (kod, ağırlık, öğe,
alan) ve öğelerin görünür, zorunlu, salt okunur durumları aynı olmalıdır. Bağlanan ve koparılan alt kayıtlar penceredeki sayıyla
gönderimin sayısının aynı olduğu testle sınanır.

## Denetimler

- `scripts/fixtures/related_count_cases.py --check`: `cad.forms.submit.related.json` 25 durum (en az, en çok, bire bir, bağla, kopar,
  yeni alt kayıt, gizli öğe, adlı öğe, plan, başka binanın sayaçları, çoktan çoğa, iki öğe, alt kaydın kendi formu).
- `scripts/fixtures/forms_publish_cases.py --check`: yayının alanları (değişen alan yeniden yayınlar ve doğrulama söyler,
  bağlanmayan alan değiştirmez, özetsiz eski yayın); `forms_cases.py`: yayınların `fields`'ı, ilişkili öğenin sayısı.
- Masaüstü: `the_window_and_the_command_judge_a_record_alike`, `a_related_item_bounded_one_to_one_refuses_a_record_holding_more`,
  `a_link_taken_off_in_the_window_counts_before_it_is_written`, `a_release_whose_fields_changed_is_said_in_the_window`; resimler
  `form-iliskili-sayi`, `form-yayin-alanlari`.

## Sonuçlar

- Bire bir ve zorunlu alt kayıt formun verisindedir ve her girişte aynı sayılır.
- Yayın bağlandığı alanları bilir; alanlar değişince yayının eskidiği görülür.
- Formu açan, formun nasıl açıldığını ve neden salt okunur olduğunu görür.
- Masaüstü penceresi komutla aynı yargıyı verir.
