# ADR 0797: Öznitelikler'den yazılan değerin bağ kartı

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-01'in dördüncü teslimi **[~]**.
- **Bağlam:** ADR 0790 bağların reddini sürüklemede, Duvarı kaydır'da, Boy'da ve açıklığın taşınmasında yerinde işaretledi
  ve reddedilen yazmadan sonra çizimin üstüne “Bağı kaldır” kartını koydu (`Context.refused`, `points::written`). Öznitelikler
  panelinden yazılan değer (duvarın kalınlığı, ötelemesi, açıklığın yeri …) aynı `constraint_conflict`'i alıyor ama kart
  açılmıyordu: panelin yazmaları araç oturumundan geçmez (`kentos_interaction::properties::said`), ret yalnız iletiydi.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'de.
- **Şema:** değişmez; komut değişmez.

## Karar

- `properties::said` bağları olan reddi (`CommandError.bonds` boş değil) saklar, geçen yazma kartı kapatır; masaüstü panelin
  her yazmasından sonra onu alır (`take_bonds_refused`) ve ADR 0790'ın kartını açar ya da kapatır. Kartın düğmesi aynı:
  tek `cad.constraints.edit` `remove`, tek geri alma adımı. Ret iletisi eskisi gibi uyarı satırında da söylenir.
- Kanal iş parçacığına bağlı bir yuvadır (panelin yazmaları arayüz iş parçacığında, ardışık); yazma olmadığında dokunulmaz.

## Doğrulama

- `cargo test -p kentos-desktop --lib bond_drag_scenes`: iki yönden sabit iç duvarın Öznitelikler'den kalınlığı 0,40 yazılınca
  `constraint_conflict` ve kart (yolu olan bağlarla); “Bu iki bağı kaldır” bağları kaldırır, aynı değer yeniden yazılınca
  geçer ve kart açılmaz.

## Sınırlar

- ADR 0790'ın geç çelişkide (kaydırmalar yazılırken, örn. kilitli katman) 48 denemenin en kötü süresi ölçülmedi.
- Gerçek pencerede elle denenmedi.
