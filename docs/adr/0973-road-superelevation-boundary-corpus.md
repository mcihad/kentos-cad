# 0973 — Dever kabul korpusunda sınırlar: çok kısa ters kurp ve km eşitliği

Tarih: 2026-11-05. Durum: kabul (Y68). Kapsam ROAD-DEVER-12.

## Sorun

ROAD-DEVER-12: "Çok kısa ters kurp/transition overlap ve station-equation sınırları bağımsız kabul korpusunda olsun."

Bağımsız korpusta (ADR 0904) geçişleri binen ters kurp vardı ("Ters kurplarda geçişler biniyor"). Eksikler:

- **Sınırın kendisi:** Geçişlerin değmek üzere olduğu ya da az biraz bindiği durumlar korpusta yoktu.
- **Km eşitliği:** Geçişin içine düşen bir km eşitliği yoktu.
- **Pencere:** Dever penceresinin kritik km tablosu "Km (m)" başlığı altında iç km'yi yazıyordu. Eşitlikli aksta
  kullanıcı 211,38 görüyordu; doğru km 1+511,379'dur.

## Araştırma

- **Binme kuralı:** İki kurbun geçişi binerse kesit iki kurala birden uyamaz, çizelge reddedilir (ADR 0904 §2). Kural
  "sonrakinin başı < öncekinin sonu"dur: değmek kabuldür. Rust 10⁻⁹ m'lik yuvarlama payıyla aynı kuralı uygular.
- **Km eşitliği:** İç km'yi değil yalnız km yazısını değiştirir (ADR 0901 §6). Eşitliğin kendi km'si ileri değeri
  alır; bölgenin başı kendisinindir.

## Karar

### Çok kısa ters kurp (`fixtures/road/v1/superelevation.json`)

Tüm geçiş teğetteyse (teğet payı 1) tam dever kurbun ucundadır. Geçiş oradan (e − n)/e·L = 8,5/6·L uzanır (e = %6,
n = −%2,5). Bu yüzden iki geçişin arasındaki açıklık (TS₂ − ST₁) − 2·8,5/6·L'dir. Referans L'yi bu açıklık +1 mm ve
−1 mm olacak biçimde kesirlerle çözer:

| Durum | Geçiş (m) | Sonuç |
|---|---|---|
| "Çok kısa ters kurp: geçişler arasında 1 mm kalır" | 38,274378 | kurulur; kritik km'ler ve beş km'de eğimler |
| "Çok kısa ters kurp: geçişler 1 mm biner" | 38,275084 | `Overlap p1–p2` |

### Km eşitliği (`road.superelevations.query`)

Yeni durum: "Km eşitliği geçişin içinde: kritik km'ler iki yanında". Ana aksa iç km 200'de bir eşitlik konur: geri
1+200, ileri 1+500.

- **Kritik km'ler:**
  - eşitlikten önce başlangıcın tabanıyla: 1+178.046 (NC), 1+194.713 (LC);
  - sonra ileri tabanla: 1+511.379 (RC), 1+534.713 (FS), …
- **Sorulan km'ler:** 199,999 → 1+199.999, **200 → 1+500.000** (eşitliğin kendisi ileri km), 200,001 → 1+500.001.

Referans eşitliği kendi `regions`/`km_of`'uyla çözer. Paylaşılan aks tanımı değişmez; durum kendi çizimini taşır.

### Dever penceresi

Kritik km tablosunun sütunu "Km"dir ve km yazısını gösterir (`critical_rows`): 1+511,379. İç km tabloda yoktur;
diyagram onu kendi ekseninde gösterir.

## Ölçüm

- Rust'ta (`superelevation.rs` korpus testi ve `road.superelevations.query` durumları) ve Python'da geçer.
- `the_critical_table_writes_the_km_across_an_equation`: tablonun ilk dört km'si eşitliğin iki yanında doğru tabanı
  yazar.
- Resimler `.run/gui-evidence/yol-y68/` altındadır.
