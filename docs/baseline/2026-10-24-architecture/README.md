# Mimari ilk dilim — gerçek pencere ve hesap kabul kaydı

24 Ekim 2026. Dal: `codex/architecture-2d-workflows`.
Başlangıç kaynak: `bb1f3ffd0`; plan önce ayrı `aec97afa7` commit'inde kaydedildi.
Bu klasör aynı dalın ilk `ARCH-DRAW-01` uygulamasının kanıtıdır. Tüm mimari yol
haritasının, tüm DWG dosyalarının veya render performansının kabulü değildir.

[Plan](../../../plan/architecture.md) ve [ADR 0627](../../adr/0627-rectangular-architectural-walls.md)
kapsamı, veri sözleşmesini ve açık işleri tanımlar. Ticari örnekler üretici
belgelerinden incelendi; bu bilgisayarda çalıştırılmadı.

## Gerçek uygulamada gözlenenler

Güncel kaynaktan derlenen masaüstü, ayrı `KentOS Architecture QA.app` adı ve ayrı
ayar diziniyle açıldı. Kaynak kullanıcı projesi kullanılmadı. Fare/klavye testleri
yerel pencereye uygulandı; aşağıdaki görüntüler otomatik sahne çıktısı değildir.
İlk üç ekran 2880×1726, diğer ekranlar 2880×1720 pikseldir.

| Kanıt | İşlem ve doğrudan gözlenen sonuç |
| --- | --- |
| [Önce](before-free-wall.png), [before.kcad](before.kcad) | Başlangıç derlemesinde Mimari metre/1:100; serbest Duvar ile 4×3 m eksen çevresi, 200 mm kalınlık, fareyle kapı/pencere/mahal. Eksen referansından net alan **10.64 m²**. Mahal geri al/yinele: 3→4 nesne. |
| [Sonra](after-inner-room.png), [after.kcad](after.kcad) | Yeni şerit düğmesi, İç net referansı, B ile 4×3 m ve 200 mm duvar. Önizleme belgeye yazmadı. Tamamlanınca tek kapalı duvar; tek geri al 0, yinele 1. Mevcut kapı/pencere/mahal ile **12.00 m²**, toplam 4 nesne. Ayrı kaydet/aç sonrasında aynı sonuç ve 1:100 görüldü. |
| [Açık tema / 18 px](options-light-18px.png) | Diğer menüsünden referans, stil, kalınlık, boy ve açı seçeneklerine erişim. Mevcut `8"` ile `12',10'` söz dizimi ve 30° döndürme yalnız önizlemede denendi; kalıcı yeni nesne oluşmadı. |
| [Sol üst önizleme](preview-upper-left.png) | Son araç sürümünde 4×3 m sayısal boyla sol üst çeyreğe açılma. Etiket **4.000 × 3.000 m** kaldı; CCW yol normalleştirmesi etiketi 3×4'e çevirmedi. Önceden kaydedilmiş dört nesne değişmedi. |
| [Son derlemede açma](final-reopen-18px.png) | Son test/derleme kontrollerinden sonra kaydedilmiş dosya tekrar açıldı, Tümünü göster ile sığdırıldı. 18 px açık temada dört nesne, kapı/pencere, 12.00 m² ve 1:100 görüldü. |
| [Mevcut özellik paneli](property-width-250mm.png) | Duvar seçilince mevcut Duvar bağlam sekmesi açıldı. Genişlik hücresinde 0.200→0.250 m yazılıp Enter ile uygulandı; duvar dışa kalınlaştı, iki açıklık ve 12.00 m² net alan korundu. Şeritte tek geri al ile 0.200 m ve eski gövde geri geldi; dosyaya kaydedilmedi. |

Ek gerçek etkileşim: birinci köşesi girilmiş yarım taslakta sağ tuş menüsünün
**Onayla/bitir** seçeneği tek tıklamada aracı bitirdi, ek nesne üretmedi. Eski
`before.kcad` yeni derlemede tekrar açıldı; dört nesne, 10.64 m² ve 1:100 korundu.
Statik zoom sonrası duvar/açıklık/mahal görünür kaldı; kare zamanı ölçülmedi.

Dosya bütünlüğü (SHA-256):

- `before.kcad`: `6f74afe76152b638abfb4aab57197e3a9349b0485c878ff404bcfc17c103d34a`
- `after.kcad`: `c929e8f2506afd8a5f70966292ada17cca80bda93ebb8744091f33a0324f762a`

KCAD bağımsız dump denetimi: her iki dosyada `wall`, iki `opening`, bir `space`;
CAD workspace, SRID 0 ve plotScale 100. Son duvar köşeleri metre olarak
(2,2), (6,2), (6,5), (2,5), kalınlık 0.2, hiza `left`. Başlangıç dosyası test
sonrasında aynı hash'i korudu. Gerçek kullanıcı tercihleri değiştirilmedi.

## Otomatik kabul

- Bağımsız Decimal/Taylor başvurusu: **81 ortak komut durumu**. Mevcut sayısal
  bütçeler genişletilmedi; referans, dört çeyrek, dönüş, dört frame türü, stil,
  metadata, beş birim, uzak koordinat, validate/plan/execute/ret ve undo.
- Native entegrasyon: **149 geçti, 22 ignored**; yeni fiziksel alan, hosted kapı,
  snapshot, revision, idempotency ve tek journal/undo testleri dahil. Kaynak boy
  tam kapanma / bir ULP ayrımı ayrıca birim testinden geçti.
- Masaüstü: **1261 geçti, 218 ignored**; ortak etkileşim izleri, katalog başlıkları,
  CAD/CBS şerit ayrımı ve mevcut katman tipi kontrolleri dahil.
- Etkileşim: **637 geçti, 1 ignored**; yeni araçtaki 11 test, mevcut serbest
  Duvar, normal Dikdörtgen, açıklık ve mahal akışları dahil.
- Headless yeni kabulü: **1 geçti**. MCP: **29 geçti**.
- Python: tam kümede **262 geçti**, bunun üçü yeni dikdörtgen duvar kabulü. Beş birim × iki
  başlangıç × üç referans, net alan, yazmayan plan, kaydet/aç, hosted kapı,
  duvar kalınlığı değişimi ve eski revision reddi ayrıca denetlendi. Son native
  paket yeniden derlendikten sonra bu üç test tekrar geçti.
- GPU sahne denetimi: **16 görüntü**, iki tema × iki boyut; üç 13 px sahne ve
  bir 18 px sahne. Tam boy pikseller gözle incelendi. Dar pencerede mevcut
  şerit kompakt davranışı ve Diğer menüsü kullanılıyor; tüm şeridin yeniden
  tasarımı bu kabulün parçası değildir.
- İlgili yedi pakette `clippy --all-targets -- -D warnings` geçti. Yeni Rust
  dosyalarında rustfmt, Python betiklerinde sözdizimi kontrolü geçti.
- Ortak `kentos-geometry-wasm` optimized WASM derlemesi ve güncel masaüstü/Python
  derlemeleri geçti. Katalog, SDK, şerit, 885 satırlı manifest ve 47 crate / 61
  crate×hedef bağımlılık yönü güncel ve tutarlı.
- Repo genelinde `cargo fmt --all --check` geçmedi; başlangıç `bb1f3ffd0`'daki
  `registry.rs` de ayrı rustfmt kontrolünde aynı eski biçim farklarını gösteriyor.
  Yeni dosyalar temiz; var olan kaynaklar topluca yeniden biçimlendirilmedi.

Tekrarlama kaynakları: `scripts/fixtures/walls_rectangle_cases.py --check`,
`python/tests/test_wall_rectangle.py`, native/interaction `walls_rectangle` ve
`wall_rectangle` testleri, `fixtures/interaction/v1/wall-rectangle.json`,
masaüstü `wall_scenes.rs`. Katalog, Python SDK, şerit ve manifest için kendi
üreteçlerinin `--check` yolu kullanılır. Web UI testleri ADR 0228 gereği koşmadı.

## Önizleme süreleri ve sınırlar

[Ham ölçüm](preview-perf.json), aynı fiziksel duvarın yeni `cad.walls.rectangle`
ve mevcut `cad.walls.create` planlarını eşlenik AB/BA sırasıyla karşılaştırır.
Her komut için 10 ısınma + 30 ölçüm; aynı JSON planı ve değişmeyen belge/revision
denetlenir. Yeniden çalıştırma:

```sh
.run/py/bin/python scripts/perf/wall_rectangle.py --large --build-profile debug
```

| İlgisiz arka plan | Yeni komut medyan / p95 (ms) | Mevcut duvar medyan / p95 (ms) |
| --- | --- | --- |
| Boş | 0.046 / 0.047 | 0.048 / 0.049 |
| 1.000 duvar | 0.178 / 0.245 | 0.177 / 0.226 |
| 10.000 duvar | 1.485 / 1.740 | 1.476 / 1.686 |
| 10.000 çizgi | 0.874 / 1.157 | 0.898 / 1.045 |
| 100.000 duvar | 20.269 / 21.099 | 20.192 / 21.636 |
| 100.000 çizgi | 23.127 / 28.081 | 23.116 / 26.630 |

Bu tek debug ölçümü Python DTO/JSON/FFI/native plan yolunundur; eşzamanlı
doğruluk testlerinin yükü izole edilmedi. GPU, olay döngüsü, kalabalık birleşim
ve gerçek UI kare zamanı kapsam dışında. Yeni komutta belirgin ek maliyet
görülmedi; mevcut ortak **O(n) belge taraması** 100 bin nesnede önemlidir.
`ARCH-PERF-01` bu ayrı iyileştirmeyi takip eder; FPS veya titreme giderildi
sonucu çıkarılamaz. Pan sürüklemesi denendi fakat yer değiştirme bağımsız
ölçülmedi; tam pan/zoom performans kabulü açık kalır.

Menüden sonra komut alanına yeniden odaklanma gereği ve `in`/`ft` yerine mevcut
inç/ayak işaretlerinin kullanımı gerçek denemede gözlendi; `ARCH-UX-01`'de
belgelendi. Ortak klavye ayrıştırıcısı bu yeni araç için değiştirilmedi.
