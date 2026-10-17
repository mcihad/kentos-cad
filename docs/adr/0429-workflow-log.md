# ADR 0429: İş akışının geçmişi

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AL`; `plan/form.md` FORM-WORK-03 ve 06'nın yerel kısmı.
- **Bağlam belgesi:** ADR 0428 (kayıt iş akışı).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir kaydın akıştaki yolculuğu (kim, ne zaman gönderdi, neden geri gönderildi) çizimin komut kaydında vardı ama kaydın kendisinde
görünmüyordu.

## Karar

- İş akışının `log`'u: serbest metin bir alan (gerekçe alanlarından biri olamaz). Her adım ona bir satır ekler: an ve kişi (komuta
  verildiyse), adım ve durumları, gerekçe: `2026-10-17T14:30:00 · Ayşe Yılmaz · Geri gönder (Gönderildi → İade edildi): Kat eksik.`
  Alanın uzunluğu dolunca en eski satırlar gider (son satır uzunluğu aşarsa kısalır). Komut anı ve kişiyi kendisi okumaz: Formla
  düzenle cihazın anını ve kullanıcısını verir.
- Geçmiş alanı formda salt okunurdur ve adımın değerleriyle yazılamaz (`form_invalid`). Salt okunur çok satırlı alan bütün satırlarını
  gösterir.
- Tasarımcının İş akışı bölümünde **Geçmiş alanı**.

## Denetimler

- `forms_transition_cases.py --check`: geçmişin beş durumu (an ve kişiyle, onlarsız, gerekçeli, değerle yazılamayan, dolunca en eski
  satırların gitmesi), toplam 37.
- Masaüstü: `a_step_of_the_workflow_writes_the_state_and_closes_the_editor`'ın geçmiş satırı; resim `form-is-akisi-gecmis`.

## Sonuçlar

- Kaydın akıştaki yolu kaydın üstünde okunur; aynı satırlar Python ve ajanın adımlarında da yazılır.
- Açık: adımın anlık görüntüsü ve gözden geçirenin onayladığı sürüm (FORM-WORK-06), rollerle doğrulanan kişi (bulut).
