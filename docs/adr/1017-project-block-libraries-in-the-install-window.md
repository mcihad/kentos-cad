# ADR 1017: Kurulum penceresinde proje blok kitaplığı

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` açığı “masaüstünden proje/bulut
  kitaplığı seçimi” (ADR 1100 §2'nin sınırı: kayıt yolundan çağrılar yalnız yerleşik kitaplıkları okur).
- **Bağlam belgesi:** ADR 1100 (`Libraries`, `cad.standards.install`'ın blok satırları).

## 1. Karar

- Standart kaynaklarını kur penceresi **Kitaplık ekle…** ile bir `.kblok` (`kentos.blocks`) dosyası alır; okunan
  kitaplık KentOS'un kitaplıklarından **önce** aranır (en son eklenen en önce). Önizleme ve kurulum aynı kitaplık
  listesiyle `plan_with` ve `execute_with`'i çağırır; satırın “Kitaplık” sütunu bloğun bulunduğu kitaplığı söyler.
- Kitaplık çizime yazılmaz, pencere açık kaldıkça (ve kurulumdan sonra) seçili kalır; okunamayan dosya söylenir.
- Bulut kitaplığı bu dilimde yok: bulut projesindeki `.kblok` dosyası indirildikten sonra aynı düğmeyle eklenir.

## 2. Kanıt

`standard_install::tests::a_project_library_added_in_the_window_is_searched_for_the_blocks`: yerleşik kitaplıklarda
olmayan yayının satırı `unavailable`; proje kitaplığı eklenince `missing`, kitaplığı “proje”; Kur bloğu çizime ekler.

## 3. Sınırlar

Python ve MCP'nin `cad.standards.install`'ı hâlâ yalnız yerleşik kitaplıkları okur (girdide kitaplık yolu yok);
bulut kitaplığının doğrudan okunması ayrı iştir.
