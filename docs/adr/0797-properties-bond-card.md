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

### 2. Geç çelişkide kümelerin aranması (ADR 0790 §4'ün ölçülmeyen en kötü durumu)

Ölçüm (`arch_bond_timing`'in `late` çizimi): en üst katın iki uç duvarı kilitli katmanda (çözüm onları tutmaz, Kaydır en üst
duvarın kaymasında uçlarını sürükler); zincirin dışında dokuz bağ en üst duvarı doğrudan oynatılan duvardan tutar. Tek bağ,
ikili ya da üçlü kalksa da kayma reddedilir: on bağ tek tek, sonra 48 küme denenir, hiçbiri geçirmez. Önce her deneme
bütün bir çözümdü: 200 duvarda **90,8 ms**, 400 duvarda **366,5 ms** (Apple M5, release, ortanca); masaüstünde her imleç
hareketinde bir plan yürüdüğü için bütçe (20 ms) bozuluyordu.

Karar: geç reddin değişkeni ve denklemlerin verdiği kaymalar saklanır (`Solved::refused_at`). Bir küme denenirken önce
yalnız denklemler çözülür (yazma yok; `refused_again`): reddedilen değişkene kadar her kenarın kayması öncekiyle aynıysa
(sıfırlığı aynı, fark `SETTLED` içinde) yazmalar da aynıdır ve Kaydır aynı yerde aynı nedenle reddeder; küme geçirmez,
çelişen bağlar o değişkeni oynatanlardır. Emin olunamayan (kaymalar değişen, denklemleri çelişen) küme eskisi gibi bütün
çözülür; “geçirir” cevabı her zaman bütün çözümle verilir. Sonuç: 200 duvarda **3,6 ms**, 400 duvarda **21,9 ms** (ilk
bütün çözüm ~7 ms ve 58 denklem çözümü). Denklem çelişkisi ve sürükleme ölçüleri değişmedi.

## Doğrulama

- `cargo test --release -p kentos-native-application --test all arch_bond_timing -- --ignored --nocapture`: geç çelişkinin
  satırları (önce 90,75 / 366,46 ms, sonra 3,64 / 21,94 ms), ret kodu `constraint_conflict`, hiçbir bağ ve küme geçirmez.
  `arch_bond_conflict_cases.py` ve bağların ortak durumları aynı cevapları verir.

- `cargo test -p kentos-desktop --lib bond_drag_scenes`: iki yönden sabit iç duvarın Öznitelikler'den kalınlığı 0,40 yazılınca
  `constraint_conflict` ve kart (yolu olan bağlarla); “Bu iki bağı kaldır” bağları kaldırır, aynı değer yeniden yazılınca
  geçer ve kart açılmaz.

## Sınırlar

- 400 duvarda geç çelişkinin en kötüsü 21,9 ms, bütçenin sınırında; kaymaları değişen kümeler yine bütün çözülür.
- Ön süzgeç küme çıkarılınca değişkenlerin yazma sırasının değişmediğini varsayar (bütün kümenin okunuşu); sıra değişip
  yazma başka türlü geçecek olsaydı “geçirmez” der: bir öneri kaçabilir, yanlış öneri verilmez.
- Gerçek pencerede elle denenmedi.
