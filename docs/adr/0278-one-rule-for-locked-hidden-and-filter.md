# ADR 0278: Kilitli, gizli katman ve seçim süzgeci için tek kural

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-14`: “Selection filter, kilitli/gizli katman ve xref
  filtreleri bütün araçlarda aynı kurala uysun; atlananlar sayısıyla raporlansın.” Kapsamı ben belirledim. ADR 0066'nın ve 0260 §5'in
  “biri kilitliyse hiçbiri yazılmaz” kuralını `cad.entities.set` ve `cad.entities.order` için değiştirir.
- **Bağlam belgesi:** ADR 0037 ve 0047 (kilitli katman), 0187 §5 (seçim süzgeci), 0211 (katman süzgeci), 0275 (seçim sorgusu).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Kural yer yer farklıydı: Taşı, Dizi, Sil ve Esnet kilitli katmandakileri sayıyla atlıyordu; Öznitelikler (`cad.entities.set`) ve Öne getir
(`cad.entities.order`) karışık bir seçimde hiçbir şey yazmıyordu. Kutuyla seçen araçların bir kısmı seçim süzgecini uyguluyor, bir kısmı
uygulamıyordu (pencerelerin nesne seçmesi, Özellik kopyala); kenar seçen araçlar (Buda, Uzat, Ötele, Kır …) süzgeci hiç tanımıyordu.
Kilitli nesneye tıklayan, nedenini öğrenemiyordu.

## Karar

1. **Gizli ve süzgeçten geçmeyen nesneler hiçbir seçimde yoktur:** gizli katmanınkiler, katman süzgecinin ve zaman penceresinin dışarıda
   bıraktıkları (ADR 0211, 0210) tıklamada, kutuda, seçim araçlarında, kenar seçmede ve kenette yoktur. Dış başvuru (xref) gelince onun
   içeriği aynı kurala uyar (ADR 0274 §6).
2. **Seçim süzgeci nesne seçen her adımdadır:** seçim aracı, kutu, seçim araçları (ADR 0275'in sorgusu), değiştirme araçlarının nesne
   seçmesi, pencerelerin nesne seçmesi, Özellik kopyala, etiket araçları ve kenar seçen araçlar (`edge::pick`). Dışarıda kalan söylenir:
   kutuda sayısı, tıklamada türü (“Seçim süzgeci tıklanan nesneyi (Çizgi) dışarıda bıraktı.”).
3. **Kilitli katmandakiler seçilir ama değişmez:** bir nesne kümesine aynı işi yapan komutlar (Sil, `cad.entities.transform`, Dizi, Esnet,
   `cad.entities.set`, `cad.entities.order`) kilitlileri atlar ve `layer_locked` uyarısıyla sayar (“1 nesne kilitli katmanda olduğu için
   atlandı. …”); hepsi kilitliyse hiçbir şey yazılmaz ve ret `layer_locked`'tır (ortak kural `checks::skip_locked`). Nesneler arası
   ilişki kuran ya da tek bir nesneyi işleyen düzenlemeler (`cad.entities.edit`, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Ötele)
   ADR 0047'deki gibi bütün ya da hiçtir. Nesneyi kilitli katmana taşımak reddedilir.
4. **Tıklamada neden söylenir:** kenar seçen araçta kilitli nesneye tıklanınca ““Kilitli katman” katmanı kilitli; Çizgi düzenlenemez. …”
   (`edge::pick_said`).
5. Nokta editörünün Katmana taşı'sı komutun taşıdığını sayar.

## Sonuçlar

- Karışık bir seçim Öznitelikler'de ve Sırala'da da işler; kilitliler sayılır.
- `cad.entities.set` ve `.order`'ın ortak durumları ve bağımsız başvuruları (`set_command_cases.py`, `order_command_cases.py`,
  `point_batch_cases.py`) yeni kurala göre.

## Doğrulama

- `cargo test -p kentos-native-application --test all` (ortak durumlar ve yetenekler); `python3 scripts/fixtures/set_command_cases.py
  --check`, `order_command_cases.py --check`, `point_batch_cases.py --check`.
- `KENTOS_TRACES_ONLY=locked-and-filter cargo test -p kentos-desktop every_trace_passes`; bütün izler değişmeden geçer.
