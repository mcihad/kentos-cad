# ADR 0125: Masaüstünde modeller de arka planda çalışır

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0124 (araçlar arka planda), ADR 0084 (İşlemler), ADR 0116 (modeller), ADR 0020 (belge); docs/PROCESSING.md; TODOS.md PERF-08.

## Bağlam

ADR 0124'ten sonra büyük girdili araçlar arka planda çalışıyordu, modeller çalışmıyordu. Parsel ölçü yazıları modeli 50 000 parselde arayüzü 1,1 sn donduruyordu.

Model üç araç adımını art arda çalıştırır. Adımların hepsi tek geri alma adımıdır: model belgede bir grup açar, adımlar onun içinde yazar (`begin_group`). Web modelin adımlarını ayrı ayrı gönderir; her adım kendi girdisine göre sayfada ya da Worker'da çalışır. Adımlar arasında sayfa kullanılabilir ve grup açık kalır. Kullanıcının bu arada yaptığı düzenleme modelin geri alma adımına karışır.

Masaüstünde aynı yol iki soru doğurur:

- Grup birden çok güncelleme boyunca açık kalır. Başka bir düzenleyicinin bu arada gelen işi ona karışmamalıdır.
- Bir adımın girdisi önceki adımın çıktısıdır: onun eklediği nesnelerin kimlikleri (`Scope::Ids`). Her adım ancak bir önceki çizime yazıldıktan sonra hazırlanabilir.

## Karar

- **Model bütünüyle kopyada çalışır.** Çizimin okuma kopyası (ADR 0124) ayrı iş parçacığına gider. Model orada, her adımı kopyada hazırlanıp hesaplanarak ve kopyaya yazılarak çalışır (`model_runner::record_model`).
  - Her adımın işi (`Job`) ve hesabı kaydedilir, adım başlamadan bittiyse sonucu (`RecordedStep`).
  - Aracın iletileri ve çalıştırıcınınkiler burada çalışan modelin sırasıyla toplanır; sonuçla birlikte komut geçmişine yazılır.
  - İlerleme bütün modelin payıdır: adım adım, adımın kendi payıyla.
- **Cevap çizime yeniden oynatılır** (`replay_model`). Kaydedilen adımlar çizimde, kopyada gittikleri gibi, tek geri alma adımında uygulanır.
  - Model bunun için bir kez daha aynı yoldan geçer (`run_model_with`). Doğrulama, grup, adımların çıktıları, özet ve geçmiş kaydı burada çalışan modelinkidir.
  - Adımlar çalıştırılmaz. Her adımda kaydedilen iş ve hesabı `finish` ile yazılır.
  - Kopya çizimle aynıyken yeni nesneler kopyadakiyle aynı kimlikleri alır. Bu yüzden sonraki adımların kaydettikleri kimlikler çizimde de doğru nesneleri gösterir.
- **Çizim arada değiştiyse** (başka bir düzenleyicinin işi), kaydedilen adımlar ona uymayabilir. Model çizimin şimdiki hâlinde burada yeniden çalışır (`run_model`) ve komut satırı bunu söyler: "Çizim model çalışırken değişti; model çizimin şimdiki hâlinde yeniden çalıştırıldı."
  - Ölçü çizimin kuşağıdır (`Document::generation`). Kendi düzenlemeler de dışarıdan gelenler de onu değiştirir.
  - Pencere açıkken çizim bu kullanıcının tuşlarına ve faresine kapalıdır. Değişiklik ancak dışarıdan gelir; seyrektir.
- **Grup açık kalmaz.** Arka plandaki model çizimde grup açmaz. Grup yalnız yeniden oynatma sırasında, tek bir güncelleme içinde açılıp kapanır. Web'deki karışma masaüstünde olmaz.
- **Durdur** modeli hemen bitirir. Kayıt modelin sözleriyledir: "Model durduruldu; çizim değişmedi." Hesap çökerse: "“…” çalışırken hata: …" (`end_model`). Başka bir çizim açıldıysa hiçbir şey uygulanmaz.
- **Nerede çalışır:** modelin yerleri web'in kuralıyla, adımlarının araçlarının burada gidebildiği yerlerdir (Bu bilgisayarda, Arka planda).
  - Otomatik bütün model için karar verir: girdileri 2 000 ve daha çok nesneyse arka plan.
  - Satır bunu söyler ("şimdi: arka planda"). Web'in "adım adım" notu yerine, çünkü masaüstünde adımlar ayrı ayrı gönderilmez.

## Sonuçlar

Aynı makine (Iris Xe, 15 GB), `perf::frame`'in yeni durumları, 10 000 parsel, bütün parseller seçili. Model 400 000 nesne yazar: 200 000 köşe numarası ve 200 000 kenar yazısı. Arayüz iş parçacığı (MİB), milisaniye:

| Durum | MİB | Uygulama | Görünüm | Çizim |
|---|---|---|---|---|
| Burada çalıştırma | 1 646 | 1 353 | 199 | 93 |
| Arka planda: Çalıştır | 3,4 | 2,4 | 0,3 | 0,2 |
| Arka planda: sonucun uygulanması | 1 370 | 976 | 299 | 94 |

- İş parçacığındaki iş 1 445 ms sürdü. Arayüz o sırada serbestti.
- **Donma kısaldı ama bitmedi:** 1,65 sn yerine 1,37 sn (−%17). Arka plana giden, aracın hesabıdır. Kalan, 400 000 yeni nesnenin çizime, geometri deposuna ve sahneye yazılmasıdır. Bu, çok nesne yazan araçta işin büyük payıdır; sıradaki performans işidir (TODOS.md PERF-08).
- **Bellek:** kopyadaki yeni nesneler ve kaydedilen adımlar sonuç uygulanana kadar bellekte durur. Arka plandaki model, buradakinden bir sonuç kadar fazla bellek ister.
- **50 000 parsel:** model 2 milyon nesne yazar. Ölçüm düzeneği modeli bir burada bir de arka planda çalıştırınca makinenin belleği yetmedi (11 GB'ta durduruldu). Düzenek modeli 10 000 parsele kadar ölçer.
- Arayüz iş parçacığında kalan öbür işler: okuma kopyası (Çalıştır'ın 3,4 ms'si içinde) ve yeniden oynatma.
- Kopya ile çizim arasında düzenleme olursa model iki kez çalışır. İkincisi arayüz iş parçacığındadır; donma bu seyrek durumda kalır.
- Kopyadaki yazma çizimin geri alma geçmişine girmez: kopyanın geçmişi yoktur, iş parçacığıyla gider.

## Doğrulama

- `cargo test -p kentos-processing`: `fixtures/processing/v1`'in bütün durumları iki yoldan oynanır ve aynı sonucu verir:
  - araçlar: hazırlık çizimde, hesap okuma kopyasında, yazma çizimde;
  - modeller: kopyada kayıtla, çizimde yeniden oynatmayla.

  Model, iş ve kaydedilen adım `Send`'dir.
- `KENTOS_PERF_SIZES=10000 cargo test --release -p kentos-desktop perf::frame -- --ignored --nocapture --test-threads=1`: yukarıdaki ölçüm (Parsel ölçü yazıları durumları).
- `cargo test -p kentos-desktop processing::tests`:
  - arka plandaki Parsel ölçü yazıları buradakiyle aynı nesneleri, katmanları, durumu ve iletiyi verir; tek geri alma adımı hepsini geri alır;
  - Durdur modeli hemen bitirir; kayıt iptaldir;
  - arada değişen çizimde model yeniden çalışır ve bunu söyler; sonuç değişen çizimde burada çalışanınkidir.

## Ek (28 Eylül): büyük sonucun yazılması

Sonucun uygulanması, zamanlayıcılarla parçalarına ayrıldı (10 000 parsel, 400 000 nesne). Giderilenler:

- **İki tam kopya.** `Runner::finish`, güncellenen nesnelerin kimliklerini okumak için bütün değişiklik kümesini kopyalıyordu (adım başına 85–90 ms). `apply` de her yeni nesneyi bir kez daha kopyalıyordu. Değişiklik kümesi artık tüketilir: yeni nesneler kopyalanmadan çizime taşınır. Oynatma 779 → 423 ms.
- **Belge deposu.** Her yeni nesne için katmanın kimliğinden yeni bir `String` ayrılıyordu. Artık yalnız listelenmemiş katmanda ayrılır. Toplu eklemede tablolar önce büyütülür.
- **Yineleme geçmişi.** Geri alınmış büyük bir işten sonra yeni düzenleme, yineleme geçmişindeki nesneleri arayüz iş parçacığında serbest bırakıyordu (400 000 nesnede 82 ms). 20 000 ve daha çok işlem tutan adımlar artık ayrı bir iş parçacığında serbest bırakılır; sınırı aşan en eski geri alma adımı da öyle (`free_apart`).
- **Geometri deposu** (web'le ortak): toplu eklemede tablolar önce büyütülür. Art arda aynı katmanın numarası önbellekten gelir. Eşitleme kimlikleri `HashSet` yerine sıralayıp tekilleştirir; yeni kimlikler sırayla verildiği için belge sırası aynıdır.

Aynı makine, iki koşunun ortalaması, arayüz iş parçacığı (MİB), milisaniye:

| Durum | Önce | Sonra |
|---|---|---|
| Arka planda: sonucun uygulanması | 1 259 | 860 (−%32) |
| Oynatma (yazma) | 779 | 335 |
| Burada çalıştırma | 1 600 | 1 260 |
| Arka plandaki iş parçacığı | 1 423 | 1 055 |

Denenip bırakılan: geometri deposunun kayıtlarını çekirdeklere bölmek. Kayıt başına iş küçük olduğundan kazandırmadı (sıralı 30, paralel 35–41 ms). Ayrı iş parçacığında serbest bırakmanın ayırıcıda yarattığı çekişme de ölçüldü: eşitleme 20–25 ms yavaşlıyor, ama toplamda yaklaşık 40 ms kazanç kalıyor (iki koşu: yerinde 901 ve 900, ayrı iş parçacığında 864 ve 854 ms).

**Stilli sahne.** Görünümün yaklaşık 210 ms'sinin 185'i stilli sahneydi: gerçek kurulum yalnız 20 ms (103 parça, çekirdeklerde); gerisi kayıt tutma. Değişen kimlikler tekilleştirilir. Henüz kurulmamış (yeni) katmanların nesneleri için parça aranmaz, o katman zaten baştan kurulur. Parça kaydı katmanın kimliğini nesne başına kopyalamaz (`Arc<str>`), tamsayı karmasıyla ve önceden büyütülmüş tabloyla tutulur (`SlotHasher`). Kayıt 40 → 25 ms; sahne 185 → 150–165 ms. Kirli parçaların bulunması 77–91 ms'de kaldı: yük, büyük tablolarda 410 000 aramanın önbellek ıskaları.

**Kimlik karması.** Belge deposu nesneleri kimlikle (`Slot`, sırayla verilen küçük sayılar) SipHash tablosunda tutuyordu. Eşitleme, sahne, planlar ve yazma yüz binlerce kez arar. `Slot` anahtarlı tablolar artık Fx'in çarpmasıyla karılır (`kentos_domain::SlotMap`); tablolar hiç gezilmez, yalnız aranır, sırası çıktıyı etkilemez. Sırayla verilen 65 536 kimliğin 65 536 ayrı kovaya düştüğü sınanır. Sahnenin parça kaydı da aynı karmayı kullanır.

Son hâl, iki koşu: sonucun uygulanması **784 ve 763 ms** (başta 1 259, −%39); burada çalıştırma 1 154 ve 1 150 ms (başta 1 600); arka plandaki iş 1 014 ve 1 001 ms (başta 1 423).

Kalan pay: iki adımın belgeye yazılması (adım başına yaklaşık 150 ms; kimlik, karma tabloları, sıra ve katman ağaçları), geometri deposunun eşitlenmesi (yaklaşık 180 ms: kimlikler 25, kayıtlar 30, ekleme ve ağacın yeniden kuruluşu 120), sahnenin kurulması (görünüm, yaklaşık 210 ms) ve çizim (yaklaşık 100 ms).
