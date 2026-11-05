# 0962 — Boykesit değişikliğinin etkisi

Tarih: 2026-11-05. Durum: kabul (Y56). Kapsam ROAD-PROFILE-09.

## Sorun

Boykesitin bir PVI'sını oynatmak kırmızı kotu bir km aralığında değiştirir. O aralıkta duran her şey eskir:

- koridor bölgeleri;
- kesit takımlarının km'leri;
- kübajlar ve onların taşıma planları.

Onaylı bir kübajın ya da taşıma planının onayı ancak kaydedildikten sonra, hesap yeniden okunduğunda eskiyordu (ADR
0950, 0954). PVI yazılırken bunu söyleyen bir şey yoktu: 1 m'lik bir düzeltmenin onaylı kübajı değiştireceği,
kaydedip hesabı açınca anlaşılıyordu.

ROAD-PROFILE-09: "Aks profilini değiştirirken yeniden üretilecek corridor/quantity/section aralıklarını göster;
stale derived kaynaklar işaretlensin."

## Araştırma

- **Civil 3D:**
  - Profil değişince koridor "out of date" işaretlenir; Rebuild Automatic açıksa hemen yeniden kurulur.
  - Hangi km aralığının değiştiği söylenmez. Koridorun tamamı, ya da Rebuild ile tek bölge yeniden kurulur.
  - Kaynaklar: Autodesk Civil 3D yardımı "Corridor rebuild", "Out of date corridors".
- **OpenRoads:** Düzenleme sırasında koridor "dinamik" güncellenir. Etkilenen kesitler ve miktarlar için ayrı bir rapor
  yoktur.
- **Ortak eksik:** Değişikliğin **sınırı** (hangi km'den hangi km'ye) ve **onaylı hesaplara** etkisi, kaydetmeden önce
  sayıyla verilmiyor.

## Karar

### Matematik: iki kırmızı kotun farkı

Eski ve yeni çizginin kırıkları (PVI'lar, kurp uçları) birleştirilir. İki kırık arasında iki çizgi de tek bir düzgün
parçadır: doğru, parabol ya da daire. Farkları Δz(s), türevi de eğim farkı Δg(s)'dir. Her parçada:

1. Δz'nin ve Δg'nin sıfırları bulunur: 64 örnekte işaret değişimi ikiye bölmeyle (bisection) daraltılır. Örnek
   noktasında tam sıfır olan Δg de sayılır.
2. **Bu noktalar arasında |Δz| tekdüzedir.** Toleransı (öntanımlı 1 mm) kestiği yer ardışık iki nokta arasındaki
   işaret değişiminden ikiye bölmeyle bulunur. Tolerans altına inen dar bir çukur böylece kaçmaz.
   - İlk sürüm yalnız 64 örneğe bakıyordu ve bir çukuru kaçırdı. v3'ün km'si kayınca km 519,447 – 519,939 arasında
     0,49 m'lik bir yerde fark 1 mm'nin altına iniyor; 40 m'lik parçada örnek aralığı 0,625 m'ydi.
3. **Bir aralıktaki en büyük |Δz|** ya bir uçta ya da Δg = 0 olan yerdedir. Δg'nin kökü basit bir köktür, iyi
   koşulludur.
   - İlk sürüm |Δz|'nin düz tepesini altın oran aramasıyla arıyordu. Yeri ancak √ε'a, yaklaşık 10⁻⁵ m'ye kadar
     buluyordu ve Python ile Rust 170,0 ile 169,99999 diye ayrıştı.
   - Eşit en büyükler varsa en erkeni alınır (1e-9 m içinde).
4. **Uçlar:** Boykesit uzar ya da kısalırsa yalnız bir çizginin olduğu aralık `added` ya da `removed` olarak verilir.

### `road.profiles.impact` (salt okur)

- **Girdi:** boykesit, olacak PVI'lar ve isteğe bağlı tolerans (0–1 m).
- **Doğrulama:** PVI'lar `road.profiles.edit` gibi doğrulanır ve kurulur; aynı iletilerle reddedilir.
- **Çıktı:**
  - değişen aralıklar: km'leri, en büyük farkı ve yeri;
  - bu boykesitteki koridorların bu aralıklara düşen bölgeleri;
  - kesit takımlarının aralığa düşen km sayısı;
  - o koridorlardaki kübajlar ve o kübajlardaki taşıma planları, onaylı olup olmadıkları.
- **Uyarı:** Onaylı bir hesap etkileniyorsa `profile_impact_approved` verilir: yazılınca onayı eskir.

### Pencere

Boykesit penceresi, seçili boykesitin yazılan PVI'ları kaydedilenden farklıysa etkiyi okur:

- **Çizimde:** değişen aralıklar turuncuyla taranır.
- **Altında:** bir cümle: "Kaydedilirse kırmızı kot değişir: km 1+000.170 – 1+388.619 (en çok 0,882 m). Eskiyecekler:
  Ana koridor (1., 2. bölge), Enkesitler (8/8 kesit), Ana kübaj (onaylı), …".
- **Onaylı hesap etkileniyorsa:** uyarı satırı da gösterilir.

Yazılan değer kaydedilene dönerse ya da kaydedilince satırlar kalkar.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_profile_impact_command_cases.py` KentOS kodu kullanmaz:

- kırmızı kot `road_vertical_cases.py`'nindir (kesirler ve mpmath);
- sıfırlar 400 örnekte mpmath'in anderson köküyle bulunur;
- çizim taşıma onayı referansınındır, ana kübaj ve ana Brükner onaylıdır.

26 durumun hepsi Rust'ta ve Python'da 1e-6 m içinde geçer.

| Değişiklik | Değişen aralık | En büyük fark |
|---|---|---|
| v1 1 m yükselir (L = 80 parabol) | km 0,170 – 388,619 | 0,882 m, v1'de. 1 m değil: kurbun ortası A·L/8 kadar içerde kalır |
| v1 parabolü 80 → 120 m | km 112,020 – 227,980 | 0,294 m |
| v2 dairesi R 1200 → 1800 | km 266,731 – 413,354 | 0,525 m |
| v3 asimetrik kolları 50/35 → 35/50 | km 471,802 – 519,924 ve 520,076 – 568,198 | 0,136 m |
| v1 5 mm yükselir, 1 cm tolerans | yok | — |
| Bitiş 600'e çekilir | km 475,124 – 600 değişir, 600 – 669 kalkar | 0,742 m |

Asimetrik kurbun kolları yer değiştirince fark PVI'da sıfırdır. Kurbun PVI'daki sapması l₁·l₂·Δg/(2L)'dir ve kollar
yer değiştirince aynı kalır; bu yüzden aralık PVI'da ikiye bölünür.

### Pencere testi

v1 102,5 yazılınca okunan etki paylaşılan durumla aynıdır:

- bir aralık;
- Ana koridor;
- onaylı kübaj ve plan;
- `profile_impact_approved` uyarısı.

101,5'e dönülünce etki kalkar. Kaydedilince de kalkar.

## Açık

- **Hizalamanın kendisi (ROAD-ALIGN-10):** Aksın değişmesi km eşlemesi ister. Bu komut yalnız kırmızı kotu karşılaştırır.
- **Yalnız gösterir:** Koridor ve kesitler zaten her okunuşta yeniden kurulur. Etkilenen aralığı yeniden kurmak
  (ROAD-COR-15) ve yüzeyi o aralıkta yamamak ayrı bir iştir.
- **Çizimdeki boykesit:** Çizime yazılmış boykesit (ADR 0902 §5) etki listesinde yok. O, yazıldığı anın kopyasıdır.
