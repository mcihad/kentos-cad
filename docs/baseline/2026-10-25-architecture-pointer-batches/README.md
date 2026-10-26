# Native fare olay grubu — kabul kaydı

25 Ekim 2026 · ADR 0664 · Bu düzeltmenin belirtilen kabulü tamamlandı.
Dal: `codex/architecture-2d-workflows`; taban `269f1a82a`.
Yeni GUI SHA-256: `9d839a3e98e72303d634e338137f5ada4164fe4966cb5227d915df1002344181`.
Gerçek pencere 2880×1720 fiziksel / 1440×828 iç alan, açık tema, 13 px.
Kullanıcının uygulaması yerine ayrılmış Ribbon QA kopyası ve test dizinleri kullanıldı.

## Önce / sonra

| Görüntü | Kanıt |
|---|---|
| 01–02 | İzleme olmayan önceki derlemede Kaydır sürüklemesi kamerayı değiştirmedi. |
| 03–04 | Yalnız olay kaydı eklenmiş eski kod: ileri sürükleme hareket etti, geri sürükleme etmedi. `native-before.txt` ters basışa son cursor'un verildiğini gösterir. |
| 05–11 | İzleme kodu olmayan yeni derlemede üç ileri/geri çifti. Her ileri hareket +360/+150 fiziksel px, her geri dönüş başlangıçla aynı. |
| 12 | Görünür Genişlik alanına `1 m` yazılıp fareyle bir kapı yerleştirildi, toplam 10 nesne. Tekrarlı araç aynı noktada sonraki kapının çakışma önizlemesini gösterir. |
| 13–14 | Mevcut ayar sağ tuş menüsü açar; menüdeki **Onayla / bitir** bir kez seçilince araç biter. Sağ tuşun tek başına bitirdiği iddia edilmez. |
| 15–16 | Tek Undo yalnız yeni kapıyı kaldırır (9); tek Redo geri getirir (10). |
| 17–19 | Teker yakınlaştırma ve geri dönüş, ardından yeni kapının seçimi: en 1,000 m. Kare zamanı/titreme ölçümü değildir. |
| 20–21 | Aynı fare hedefinde kenet açıkken konum 4,772 m, kapalıyken 5,000 m. Takip testi bu adayı #1 duvar yüzündeki iki açıklık arasının orta noktası olarak doğruladı; aşağıdaki bağımsız hesapla aynı. |
| 22 | Kaydet, uygulamayı kapat, yeniden aç, seç: #10 en 1,000 m / konum 5,000 m. |

PNG'ler gerçek pencereden alındı ve gözle incelendi. 05–11'de yay duvarının
ilk parlak pikseli ayrıca bağımsız görüntü okumasıyla karşılaştırıldı;
`pixel-check.json` başlangıç (1125,450) ve ileri (1485,600) değerlerini tutar.
Bu sabit senaryo genel kare hızı veya titremesiz çalışma sertifikası değildir.

## Veri korunumu

`tools/kcad/kcad.py` ile native kod çağırmadan `before`, `created`, `undo`,
`after.kcad` okundu. Önceki dokuz nesne bütün alanlarıyla aynı; yalnız #10
kapısı eklendi. Başlangıç/proje ayarları aynı. Tek Undo'da dokuz nesne,
UUID sırası ve ayarlar birebir geri geldi. Son kayıt yeniden açıldıktan sonra
aynı nesne/UUID/ayar karşılaştırması tekrar geçti. `saved-file-check.json`
nesne ayrıntılarını ve dosya hash'lerini tutar.

## Testler

- Eski kodda yeni basış sırası testi **başarısız**; beklenen (50,20), gelen (250,120).
- Düzeltme sonrası ilgili viewport kümesi **14 geçti**.
- Native runtime gibi tek `Snapshot::update` çağrısıyla verilen toplu olay GPU
  testi **1 geçti**; iki yönde tam (80,40) mantıksal px, belge değişmez.
- Tam desktop **1279 geçti, 221 atlandı**; atlananlar başarıya sayılmaz.
- Desktop clippy bütün hedeflerde `-D warnings` ile geçti; GUI derlendi.
- Sekiz boyut/font/tema durumundaki mevcut GPU matrisi ayrıca son kaynakta çalıştırıldı;
  **1 geçti**, 18 görüntünün 11’i ADR 0663 kaydıyla bayt bayt aynı, farklı 7 görüntü
  yeniden incelendi ve `gpu/` altına eklendi. Günlük sonucu `test-results.json` içindedir.
  Bu offscreen test gerçek pencere matrisi değildir.

Örnek tekrar: `cargo test -p kentos-desktop a_batched_pan_moves_out_and_back_without_editing_the_drawing -- --ignored --test-threads=1`.
Testte geçici TS çıktısı ve ayrılmış XDG ayar/veri dizinleri kullanılmalıdır.

## Kenet farkının kapanan takibi

`created.kcad` içinde #8 açıklığının bitiş pervazı 4,05 m, #10 açıklığının
başlangıç pervazı 5,4947046843177185 m. Aradaki duvar yüzünün orta noktası
**4,77235234215885925 m**, yüz y=0,1 m. İmleçten 9,133 mantıksal px
uzaktadır; mevcut 11 px kenet açıklığı içinde geçerli Midpoint/#1 adayıdır.
`snap-candidate-check.json` yalnız bağımsız KCAD okuyucusu ve 60 basamaklı
decimal aritmetiğiyle bu hesabı tutar. App regresyonu gerçek adayın kimliğini,
bu koordinatı, önizlemede yazmamayı, tutamaçla kaydı, undo'yu ve kenet
kapalıyken raw 5 m noktasını doğrular. **1 ek test geçti**; üretim kodu,
kenet sırası veya tolerans değiştirilmedi. GUI derlemesi aynı SHA ile kalır.

## Açık takip

ARCH-BASE-01'in bütün araç/ölçü/mahal ve farklı belge tabanı, ARCH-PERF-01'in
kare zamanı, ARCH-OPEN-01'in tam gerçek pencere matrisi açıktır. Kamera matematiği, ölçek/hassasiyet ve model değişmedi.
