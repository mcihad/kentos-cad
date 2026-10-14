# ADR 0274: Blokların içine kenet

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-10`: “Blok, xref ve mimari nesnelerin
  içeriklerine snap; görünürlük, kilit ve nested transform kurallarına uysun.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0029 (kenet deposu), 0144 (blok), 0163 (kenet ekleri, katmanın kendi türleri), 0260 §1 (parçanın kendi
  katmanı), 0263 §3 (blok içi parçanın yolu), 0269 (türlerin kapsamı), 0272 (komut olarak kenet).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Depo bir yerleştirmenin tanımını bir kez açar (`Item.expanded`: iç içe yerleştirmeler yerlerinde açılmış, her düzeyin benzerliğiyle
çizime yerleşmiş parçalar) ve kenet bu parçaları kendi nesneleri gibi okuyordu; ama kural yazılı değildi, iç içe, dönük, ölçekli ve
aynalı yerleştirmede bağımsız bir başvuruyla sınanmamıştı, katmanın gizliliği, kilidi ve kendi türleri parçalar için söylenmemişti.
Sınarken bir eksik çıktı: Ağırlık merkezi yalnız çizimin kendi alanında vardı; bloğun kapalı parçası (bir odanın duvarı) onu vermiyordu.

Dış başvuru (xref) ve mimari nesneler (duvar, kapı, pencere, mekân) KentOS'ta henüz yok: DXF'in xref'i açıkça atlanır (plan/cad.md
§10, `CAD-REF-*`), mimari kaynak modeli `CAD-ARCH-01`'in işidir.

## Karar

1. **Parçalar kendi nesneleri gibi kenetlenir**, kimlikleri yerleştirmenin kimliğidir: uç, orta, merkez, çeyrek, düğüm, kesişim,
   dik, teğet, en yakın, yazının ekleme noktası. Yer her düzeyin benzerliğiyle (x' = p + s·R(θ)·M·(x − taban), M x ekseninde aynalama)
   çizime gelir: daire daire kalır, aynalanan yayın uçları yer değiştirir, çeyrekler çizimin doğusu, kuzeyi, batısı ve güneyidir
   (çizimin kendi daireleri ve yayları gibi). Aynı bloğun iki parçası da (bir çizgi ve bir sütun) birbirini keser; parçayla çizimin
   nesnesinin kesişiminin kimliği çizimin sırasında ilk olanınkidir (ADR 0272). Uzantı, Paralel ve nesne izleme parçalardan da alınır.
2. **Bloğun kapalı parçası ağırlık merkezini verir:** Ağırlık merkezi istenince her kapalı parçanınki (alan ya da kapalı çoklu çizgi)
   yerleştirmenin kimliğiyle. `Store::choose`'un yeni satırı budur; başka davranış değişmedi.
3. **Ekleme noktası en dıştaki yerleştirmenindir:** iç içe yerleştirme kendi noktasını değil parçalarını verir (blok yolu bir
   yerleştirmede bitemez, ADR 0263 §3); bloğun içindeki yazının noktası ekleme noktasıdır.
4. **Karar yerleştirmenin katmanınındır:** gizliyse hiçbir parçası kenetlenmez (çizilmez); kilitliyse hepsi kenetlenir (kilit
   düzenlemeyi yasaklar, okumayı değil; çizimin kendi nesneleri gibi); katmanın kendi kenet türleri (ADR 0163 §4) parçalara da uyar.
   Parçanın kendi katmanı görünüşünü değiştirir (ADR 0260 §1), çizilip çizilmeyeceğini değil: kendi katmanı gizli olan parça çizildiği
   için kenetlenir. Çizim parçayı kendi katmanıyla gizlemeye başlarsa (AutoCAD'in blok içi gerçek katmanı, plan/cad.md §8'in notu)
   kenet aynı adımda ona uyar: kenet çizilenedir.
5. **Komut aynı cevabı verir:** `cad.snap.find` parçanın kenedinde yerleştirmenin kalıcı kimliğini, parçanın yolunu (`path`) ve
   üstündeki yeri verir; ölçü onunla bağlanır (ADR 0249, 0263).
6. **Dış başvuru ve mimari nesneler gelince aynı kurala uyar:** dış başvurunun içeriği blok parçası gibi, kaynak belgenin kimliği ve
   başvuru yoluyla (`CAD-REF-13`); mimari nesne kendi kaynağının çizim parçalarıyla (`CAD-ARCH-02`). Bu madde onların kenedini değil
   kuralını koyar.

## Sonuçlar

- Kenet blokların içinde çizildiği gibi ve sınanmış: dönük, ölçekli, aynalı ve iç içe yerleştirmede aynı noktalar.
- Bir odanın, bir parselin bloğundaki kapalı şeklin ağırlık merkezi alınabilir.

## Doğrulama

- `python3 scripts/fixtures/snap_block_cases.py --check` (`fixtures/snap/v1/blocks.json`, 30 durum: KentOS kodu olmadan, tanımlardan
  elle yerleştirilen parçaların adayları); `cargo test -p kentos-geometry-core --test all snap` (durumlar ve parçadan alınan uzantı ve
  doğrultu).
- `python3 scripts/fixtures/snap_find_cases.py --check` (`cad.snap.find`'ın 24 durumu: iç içe ve aynalı bloğun parçasında kimlik, yol ve
  yer); `cargo test -p kentos-native-application --test all fixtures`.
- `cargo test -p kentos-desktop input_scenes` ve resim `kenet-blok-ici` (dönük odadaki aynalı kapının yayının ortası).
