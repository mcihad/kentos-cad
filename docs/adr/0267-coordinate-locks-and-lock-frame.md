# ADR 0267: Koordinat kilitleri ve kilidin çerçevesi

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-03`: dinamik girişte mesafe, açı, **X ve Y** alanları
  ayrı kilitlenebilsin; kilidin hangi eksene ya da UCS'e göre çalıştığı **görünsün**. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0166 (sayısallaştırma kilitleri: uzunluk, açı, sapma, paralel, dik, kalıcı, referans), 0261 (UCS), 0266
  (birimli ve toplamlı yazım), 0165 §4 (projenin eksen adları: CAD'de X doğu, CBS'de Y doğu).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0166'nın kilitleri bir sonraki noktanın **referanstan** uzunluğunu ve doğrultusunu tutar. AutoCAD'in nokta süzgeçleri (.X, .Y) ve
dinamik girişin X/Y alanları gibi, noktanın **bir koordinatını** tutmak eksikti: “bu duvarın ucu X = 12,40'ta olsun, Y imleçten” ya da
“referanstan 5 m, X = 3'te”. Ayrıca kilit çipleri (“Açı 30°”) açının hangi eksenden ölçüldüğünü söylemiyordu: UCS dönükken açı UCS'in
x ekseninden ölçülür (ADR 0261 §4), çip bunu göstermiyordu. Değer alanında Enter, kilitli noktayı UCS'i bilmeden yerleştiriyordu (önizleme
UCS'e göre, Enter dünyaya göre: bulunan delik).

## Karar

### 1. X ve Y kilitleri

- Kilit durumuna iki yeni değer: **X** ve **Y**, kilidin çerçevesinde (UCS varsa UCS'in, yoksa dünyanın) koordinat; metrede saklanır,
  çizim biriminde, kendi birimiyle ya da toplamla yazılır (ADR 0266). Adları projenin eksen adlarıdır (CAD'de X doğu, Y kuzey; CBS'de Y
  doğu, X kuzey): kilit **doğu** ve **kuzey** eksenini tutar, çipte projenin adıyla görünür.
- Nokta kuralı (çerçevede; o referans, c imleç):
  - doğu ve kuzey kilitli: (doğu, kuzey); imleç önemsiz;
  - yalnız biri kilitli: o eksen sabit, öbürü imleçten;
  - biri ve **uzunluk** kilitli: o merkezli L yarıçaplı çemberin o eksen doğrusuyla kesişimlerinden imlece yakın olanı (eşitse artı
    yöndeki); çember doğruya ulaşmıyorsa nokta yoktur (imleç referansta kalır, Enter yerleştirmez ve söyler).
- Doğrultu kilidi (açı, sapma, paralel, dik) ile koordinat kilidi birlikte tutulmaz: birini kilitlemek öbürünü bırakır (fazla belirli
  nokta olmasın diye). Orto ve kutupsal izleme koordinat kilidi varken uygulanmaz (doğrultu kilidi gibi).
- Tek seferlik ve Kalıcı kuralı ADR 0166 §1'deki gibidir: nokta konunca tek seferlik kilitler kalkar.
- Komutlar: `draft.lock.east` ve `draft.lock.north` (Kilit ▸'te projenin eksen adlarıyla: CAD'de “X kilidi…” ve “Y kilidi…”, CBS'de
  “Y kilidi…” ve “X kilidi…”), değer alanını o kilidin adıyla açar; koordinat alanında Tab öbür koordinata geçer, uzunluk ↔ doğrultu
  sırası değişmez. Arayüz kaydı `catalog/v1/additions.json`'da.

### 2. Kilidin çerçevesi görünür

- UCS dünyadan başkayken açı, X ve Y kilitlerinin çipleri ve imlecin yanındaki etiket **“(UCS)”** ekiyle söylenir: “Açı 30.0000° (UCS)”,
  “X 12.400 m (UCS)”. Uzunluk, sapma, paralel ve dik çerçeveden bağımsızdır, eki yoktur.
- Değer alanının Enter'ı kilitli noktayı önizlemeyle **aynı** kuralla (kilidin çerçevesinde) yerleştirir: tek kural `LockState::point`.
- Kilitli koordinatın doğrusu çizimde kesikli kılavuzdur (doğrultu kilidinin ışını gibi): doğununki çerçevenin y ekseni, kuzeyinki x ekseni
  boyunca (`LockState::coordinate_lines`).

## Sonuçlar

- Bir noktanın bir ya da iki koordinatı kilitlenebilir, uzunlukla birlikte de; UCS'te kilitler UCS'in eksenlerindedir ve öyle görünür.
- Değer alanının Enter'ı ile önizleme UCS'te ayrışmaz.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/lock_coordinate_cases.py --check` (`fixtures/locks/v1/coordinates.json`, 15 durum; kesirler ve 60
  basamaklı kök); çekirdekte `tests/all/locks.rs`.
- Oturumda `kentos-interaction`'ın kilit testleri (X ve Y'nin önizlemesi, UCS'te, doğrultu kilidinin bırakılması, çipler).
- Etkileşim izi `fixtures/interaction/v1/lock-coordinates.json` masaüstünde üç çeşitte
  (`KENTOS_TRACES_ONLY=lock-coordinates cargo test -p kentos-desktop every_trace_passes`).
- Resim: `KENTOS_SHOTS_ONLY=ucs-koordinat-kilidi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-ucs-koordinat-kilidi-*`: UCS'te X kilidi, çip, kesikli kılavuz, açık Y alanı).
