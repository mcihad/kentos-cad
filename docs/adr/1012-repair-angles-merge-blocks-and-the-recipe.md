# ADR 1012: Onar'da eğik kenar, yinelenen blok tanımı ve uygulanan tarifin kaydı

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-18` açığı “onarım tarifinin
  sıra/eşik/kapsamla kaydı ve öbür bulgu türleri için onarım” (ADR 1005 §5).
- **Bağlam belgesi:** ADR 1005 (sağlık grubu ve T birleşimi onarımı), `crates/native/application/src/drawing_repair.rs`.

## 1. Kaynak

Bricsys'in [DWGHEALTH](https://help.bricsys.com/en-us/document/command-reference/d/dwghealth-command?version=V27)
ve [Health Advisor ayarları](https://help.bricsys.com/en-us/document/dialog-boxes/h/health-advisor-settings-dialog-box?version=V27)
bulguları ayarlarla (eşik, kapsam) bulur, kullanıcı onayıyla düzeltir; AutoCAD'in yinelenen blok tanımlarını
birleştirmesi için yerleşik bir komutu yoktur, tanımın yeniden tanımlanması (INSERT “redefine”) ve PURGE ayrı
adımlardır (plan [R14]–[R16]). KentOS kararı: bulgu türü başına açık bir düzeltme, tek işlemde ve geri alınabilir;
uygulananın tarifi cevapta.

## 2. Karar

`cad.drawing.repair` v1'e (geriye uyumlu, isteğe bağlı alanlar) eklendi:

- **`angles`** (`{uid, pivot: start|end}`): çizgi pivot ucu çevresinde en yakın eksene (yatay ya da düşey)
  döndürülür, boyu korunur; öbür ucu pivot + boy × eksenin birim vektörüdür (birim vektör tam değerdir, dönen çizgi
  eksende bittir). Sapma `maxAngle`'dan (derece, 0 < x ≤ 10, varsayılan 0,5: denetiminki) büyükse
  `repair_angle_too_large`. Ölçülü/kısıtlı çizgi (`repair_refused_measured`) ve kilitli katman reddedilir.
- **`blocks`** (`{keep, merge: [ad…]}`, 1–100 tanım): yalnız aynı içerikli tanımlar (denetimin anahtarı: kimliksiz
  parçalar, taban, öznitelikler) birleşir, değilse `repair_blocks_differ`. Birleşenlerin çizimdeki ve öbür tanımların
  içindeki yerleştirmeleri tutulan tanıma bağlanır, birleşenler silinir. Dış başvuru ve ilişkili dizinin bloğu
  `repair_unsupported`; olmayan ad `unknown_block`. Bir tanım kurallar yüzünden silinemezse bütün işlem geri alınır.
- **Sıra:** önce T birleşimleri, sonra açılar (bir çizgi hem taşınıp hem dönerse taşındığı hâliyle döner), sonra
  bloklar. Toplam 1–1000 düzeltme.
- **Tarif (`recipe`):** cevap uygulanan adımları bu sırayla (`junction`, `angle`, `blocks`), varsayılanlarla
  çözülmüş eşikleri (`maxGap`, `maxAngle`), değişen nesne sayısını (`objects`: taşınan/dönen çizgi, bağlanan
  yerleştirme ve içinde yerleştirme bağlanan tanım) ve silinen tanım sayısını verir. Günlük girdiyi zaten tutar; tarif
  varsayılanların ne yaptığını açıkça söyler, aynı onarım okunup yeniden çalıştırılabilir. Plan da aynı cevabı verir,
  yazmaz.
- Masaüstü CAD denetimi › Çizim sağlığı: eğik kenarda **Eksene döndür** (başlangıcı çevresinde), yinelenen tanımda
  **Tek tanımda birleştir** (grubun ilk tanımı tutulur; grup çizimden yeniden bulunur:
  `drawing_inspect::duplicate_blocks`). Komut sonrası denetim yenilenir.
- Yeni kodlar `repair_angle_too_large`, `repair_blocks_differ`; `invalid_angle_tolerance` ve `unknown_block`
  bu komutun retlerine katıldı.

## 3. Kanıt

`scripts/fixtures/drawing_repair_cases.py` (KentOS kodu yok) 41 vaka yazar; açı vakaları uçları `atan2`, yuvarlanan
çeyrek ve birim vektörden elle hesaplar, blok vakası yerleştirmeyi kurar, birleştirir, kopya tanımın gittiğini
(tekrarı `unknown_block`) ve yerleştirmenin tutulan tanıma bağlandığını sınar. İlk koşuda geçti. Masaüstü resimleri
`saglik-eksen*`, `saglik-birlestir*`.

## 4. Sınırlar

- Çoklu çizgi kenarı döndürülmez (yalnız çizgi); uzak nesne bulgusu için otomatik onarım yok (silmek kullanıcının
  kararıdır).
- Açılışta otomatik onarım yok (ADR 1009 yalnız denetler); ayrı kurtarma kopyası ve kısmi okuma CAD-ADV-18'de açık.
