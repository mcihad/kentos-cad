# ADR 0276: Üst üste nesnelerde dolaşma ve alt nesne seçimi

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-12`: “Üst üste nesnede seçim cycling ve subentity
  selection; UUID/alt nesne kimliğine dönüştürülebilsin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0187 §1 (Sıradakini seç), 0249 ve 0263 (alt nesne yerleri, blok yolu), 0272 (kenetin başvurusu), 0275 (seçim
  sorgusu).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Üst üste nesnelerde dolaşma vardı (Sıradakini seç: tıklamanın adayları en belirliden, “1/3 ▾” çipi, Shift+Boşluk). Eksik olan, tıklamanın
nesnenin hangi parçasına düştüğüydü: bir alanın köşesi mi, kenarı mı, bloğun içindeki hangi parça mı; ve bunun öbür komutların aldığı
kalıcı başvuruya (`uid`, `place`, `path`) çevrilmesi. Kenet bunu yalnız kenetin türüyle veriyordu (ADR 0272).

## Karar

1. **Alt nesne kuralı** (`ops::subref::place_near`, `place_near_in`): tıklamanın toleransındaki en yakın noktası (uç, köşe, nokta
   nesnesinin noktası, ekleme noktası) kenarından önce gelir; nokta yoksa kenarının ya da eğrisinin en yakın noktası `along` yeridir;
   alanın içine ya da yazının gövdesine düşen tıklamanın alt nesnesi yoktur (bütün nesne). Blokta yerleştirmenin kendi noktası ve
   parçaların noktaları önce, sonra parçaların kenarları; parçanın yolu verilir. Nokta nesnesinin noktaları, tıklamanın onu bulduğu gibi
   1,5 toleransla aranır.
2. **Komut:** salt okunur `cad.entities.pick` v1 (tıklama, tolerans, seçim süzgecinin türleri): tıklamanın adayları en belirliden (seçim
   sorgusunun `point` bölgesi), her biri kalıcı kimliği ve alt nesnesiyle (`place`, `path`), dışarıda kalanların sayısı ve sürüm. Cevap
   `cad.dimensions.create`'in `place` noktasına ve öbür komutlara olduğu gibi verilir. Retler `not_finite` ve `invalid_distance`.
3. **Masaüstü:** seçim aracında platformun komut tuşuyla tık (Ctrl; macOS'ta ⌘) alt nesneyi seçer: ilk aday seçilir, alt nesnesinin noktası
   çizimde halkayla işaretlenir, günlük “Alt nesne: Kapalı alan 0283ad87 · 3. köşe” der; birden çok aday varsa çip açılır, Shift+Boşluk ve
   çipin listesi öbür adaya geçer, her adayın kendi alt nesnesiyle. Seçim alt nesneleri oturumundur (`Selection::pick_sub`, `sub`); seçim
   değişince düşer.
4. **Kısayol listesi:** Fare ve klavye kısayolları penceresinde “Alt nesneyi seç” (Ctrl + tık, macOS'ta ⌘ + tık) ve yeni “Çizerken” grubu:
   kenet adayları arasında Tab / Shift+Tab (ADR 0271), Shift'i basılı tutmak Orto'yu o an çevirir (ADR 0268).

## Sonuçlar

- Bir tıklama, Python ve ajan için kimliğe ve yere çevrilir; arayüzde de görünür.
- Alt nesnenin düzenlenmesi (yalnız o köşeyi taşımak, kenarı ötelemek) sonraki maddelerdir (`CAD-INPUT-15`, `CAD-EDIT`).

## Doğrulama

- `python3 scripts/fixtures/subref_cases.py --check` (19 alt nesne ve 5 blok içi durum; `cargo test -p kentos-geometry-core --test all
  subref`); `python3 scripts/fixtures/entities_pick_cases.py --check` (`cad.entities.pick`'in 23 durumu, KentOS kodu olmadan);
  `cargo test -p kentos-native-application --test all`.
- `cargo test -p kentos-desktop input_scenes` (Ctrl tıklaması köşeyi seçer, Sıradakini seç duvara kendi yeriyle geçer); resimler
  `secim-alt-nesne`, `kisayollar`; Python `test_select.py`'nin `PickTests`'i.
