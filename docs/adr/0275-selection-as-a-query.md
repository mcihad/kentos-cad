# ADR 0275: Seçim tek açık sorgu

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-11`: “Selection window/crossing, fence, polygon,
  circle, önceki seçim, benzerini seç ve filtre davranışlarını aynı açık sorgu modeline bağla.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0029 (seçim ve depo), 0141 (Çitle, Daireyle ve İçeren alanı seç), 0187 (seçim ekleri: Sıradakini seç, Çokgenle
  seç, Önceki seçim, Benzerini seç, Seçim süzgeci), 0210 (zaman penceresi), 0211 §1 (katman süzgeci), 0253 (komut protokolü).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Seçimin her biçimi kendi yolundan gidiyordu: pencere ve kesişen kutu, Çitle, Çokgenle ve Daireyle seç deponun sorgularını çağırıp süzgeci
ayrıca uyguluyordu; Tümünü seç ve Ters çevir belgeyi kendileri geziyordu (Ters çevir katman süzgecinin dışarıda bıraktıklarını da
seçiyordu); Önceki seçim yalnız katmanın görünürlüğüne bakıyordu (katman süzgecine ve zaman penceresine değil); Benzerini seç kendi
gezintisini yapıyordu. Python ve yapay zekâ ajanı için seçim yoktu: bir pencerenin nesnelerini almak için geometriyi kendileri
hesaplamaları gerekirdi.

## Karar

1. **Tek sorgu:** `cad.entities.select` v1 (Query, geri alma yok): bir bölge (`window`, `crossing`, `fence`, `polygon` (inside,
   crossing, outside), `circle`, `containing`, `point`, `all`, `objects`, `invert`, `similar` (ölçütler yoksa hepsi açık)), seçim süzgecinin türleri (`kinds`, 17
   tür; NURBS eğrisi Eğri'dir) ve katmanlar (`layers`); cevap nesnelerin kalıcı kimlikleri bölgenin sırasıyla (çizimin; `containing`'de
   küçükten büyüğe; `point`'te en belirliden), dışarıda kalanların sayısı (`leftOut`: türler, katmanlar, verilenlerden gösterilmeyen)
   ve sürüm. Örtük arayüz durumu okumaz (CMD-07): önceki seçim çağıranın tuttuğu listedir (`objects`), süzgeç girdinin `kinds`'idir.
2. **Kurallar deponundur** (ADR 0029, 0141, 0187): pencere nesnenin kutusu bütünüyle içinde; kesişen içinde ya da dokunan (çizgisinin
   bir noktası içinde, kutunun ortası alanının içinde, kenarı kutunun kenarını kesiyor); çit kestikleri ve toleransındaki noktalar;
   çokgenin üç kipi; daire içindekiler ve dokunanlar; içeren alanlar; tıklamanın adayları.
3. **Yalnız gösterilenler:** gizli katmanınkiler, katman süzgecinin ve zaman penceresinin dışarıda bıraktıkları hiçbir bölgede yoktur
   (deponun `shown`'u); kilitli katmanınkiler seçilir (kilit düzenlemeyi yasaklar, seçmeyi değil). Süzgeçler bölgeden sonra: önce
   türler, sonra katmanlar, her biri dışarıda bıraktığını sayar.
4. **Araçlar aynı sorguyla seçer:** pencere ve kesişen kutu, Çitle, Çokgenle, Daireyle ve İçeren alanı seç, Benzerini seç, tıklamanın
   adayları, Tümünü seç, Ters çevir ve Önceki seçim `kentos_native_application::entities_select::select`'i tuttukları depoyla çağırır
   (`kentos_interaction::selectable::query`); süzgecin söylediği sayı cevabın `leftOut.kinds`'idir. Böylece Ters çevir ve Önceki seçim
   katman süzgecine ve zaman penceresine de uyar.
5. Bir bölgenin noktaları ve bir listenin nesneleri en çok 100 000'dir (`too_many_points`).
6. Kod `invalid_ring` eklendi (kendini kesen ya da alansız seçim çokgeni); öbür retler var olan kodlardır.

## Sonuçlar

- Bir pencerenin, bir çokgenin ya da “bu nesneye benzeyenlerin” kimlikleri Python'dan ve MCP'den tek çağrıyla alınır; cevap öbür
  komutların `uids`'ine olduğu gibi verilir.
- Araçlar ile komut aynı sonucu verir; seçimin kuralı tek yerde.

## Doğrulama

- `python3 scripts/fixtures/entities_select_cases.py --check` (`fixtures/commands/v1/cad.entities.select.json`: bölgeler, süzgeçler,
  kilitli ve gizli katman, retler; KentOS kodu olmadan kesirlerle); `cargo test -p kentos-native-application --test all` (ortak durumlar,
  yetenekler; ortak durumun dosyayı büyüteceği `too_many_points` `capabilities.rs`'in `selections_beyond_the_limit_are_refused`'unda).
- `cargo test -p kentos-interaction`, `cargo test -p kentos-desktop` (seçim araçlarının izleri değişmeden geçer); Python `test_select.py`.
