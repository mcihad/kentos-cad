# Büyük belgede mimari önizleme — aday araması

24–25 Ekim 2026 · `codex/architecture-2d-workflows` · ARCH-PERF-01 / ADR 0660.
Aday araması diliminin kaydıdır; ARCH-PERF-01 kapsamındaki gerçek pencere
gecikmesi, GPU timestamp ve tüm stil/evre ölçüm matrisi açık kalır.

## Ölçüm yöntemi ve değişmezler

`architecture_preview.py` aynı SDK/harness ve saklanmış iki release native
modülüyle çalışır. 1/10/100 bin arka plan duvarı veya çizgi; ayrık çevre veya
12 yerel kesişen duvar; sıfır veya (500000,4400000) koordinatı: 24 ayrı süreç.
Her durumda 32 hareketli köşe için eski kapalı Duvar ve yeni Dikdörtgen duvar
planlarının tam geometrisi ve uyarıları eşit olmalıdır. Önce/sonra karşılaştırması
aynı girdi/çıktı SHA-256'larını, makineyi, Python'u ve ölçüm yöntemini zorunlu tutar.
60 sıcak örnek; 10 ısınma; AB/BA dönüşümlü komut sırası. Soğuk plan için
20 ayrı belge kullanılır; KCAD çözme/yok etme süreye dahil değildir. Native modül
süreçte yüklüdür: bu süreç açılış gecikmesi değil, belge başına ilk plandır.

Süre SDK JSON/FFI/plan/yanıtı içerir; DTO kurulumu, çıktı hash'i, dosya açma,
GPU ve pencere olayı içermez. İşlem sonunda KCAD byte'ları ve revision aynı kalır.
RSS/tepe bellek bütün Python sürecidir; SDK/veri kurulumu ve kontroller de dahildir.
Bu bir native tahsis sayacı değildir. Soğuk planın önce/sonra RSS'si ayrıca yazılır.

Iced çalışma zamanı ölçümü aynı 100012 nesneli KCAD, 1440×900, 13 px koyu tema,
serbest/dikdörtgen Duvar, snap açık/kapalı için gerçek Iced fare olaylarını yollar.
Önce 10 ısınma, sonra 60 hareket; olay/uygulama/view/layout/draw ayrı raporlanır.
wgpu ekran dışı çizim ve geri okuma ayrı alandadır. Bu alan GPU geri okumasını da
ölçer; GPU timestamp, gerçek pencere sunumu veya uçtan uca fare gecikmesi değildir.
Yalnız fare önizlemesinden sonra bütün snapshot eşitliği de kontrol edilir.

Donanım bilgisi ayrı `hardware.json` içindedir: eski genel Iced koşucusunun
Linux dışındaki boş CPU/bellek alanları ölçülmüş sıfır sayılmaz.

## Release komut sonucu

Uzak koordinat, 100 bin arka plan nesnesi; süreler milisaniyedir. Bütün 24
durumun girdi ve tam çıktı/uyarı hash'leri eşit (`baseline_identical=true`).

| Arka plan / çevre | Sıcak önce p50 / p95 | Sıcak sonra p50 / p95 | Soğuk önce p50 / p95 | Soğuk sonra p50 / p95 |
|---|---:|---:|---:|---:|
| Duvar / ayrık | 2.263 / 2.522 | 0.015 / 0.016 | 3.041 / 3.613 | 6.322 / 6.772 |
| Duvar / 12 yerel duvar | 2.379 / 2.499 | 0.156 / 0.175 | 3.194 / 3.366 | 6.377 / 6.810 |
| Çizgi / ayrık | 0.545 / 0.782 | 0.015 / 0.021 | 1.181 / 1.310 | 1.212 / 1.443 |
| Çizgi / 12 yerel duvar | 0.721 / 0.770 | 0.161 / 0.175 | 1.315 / 1.436 | 1.367 / 1.442 |

Dizin ilk sorguyu duvarlı büyük belgede medyanda yaklaşık 3,2 ms pahalılaştırır;
sonraki hareketlerde tam tarama kalkar. Duvar yazılması/silinmesi sonrasında ilk
sorgu yeniden kurulum ister. 20 soğuk yüklemenin eklenmesi ilk tek örnekli
denemenin sıcak-cache koşullarını da değiştirdi; tablodaki önce/sonra her ikisi
de **aynı son harness** ile yeniden ölçüldü. Ön denemenin sayılarıyla karıştırılmadı.

| Arka plan / çevre | İlk planın RSS farkı (sonra) | Süreç tepe önce → sonra |
|---|---:|---:|
| Duvar / ayrık | 5.86 MiB | 1563.4 → 1775.7 MiB |
| Duvar / 12 yerel duvar | 6.16 MiB | 1564.0 → 1776.1 MiB |
| Çizgi / ayrık | 1.89 MiB | 1812.3 → 1812.3 MiB |
| Çizgi / 12 yerel duvar | 2.28 MiB | 1813.1 → 1813.2 MiB |

20 ayrı belgeyi aynı süreçte açıp kapatan koşucunun duvarlı durumunda tepe
RSS artışı da gözlendi; yalnız dizinin boyutu gibi sunulmadı. Python/SDK,
KCAD çözme, geçici belgeler ve platform tahsisçisi bu yüksek su izine dahildir.
Tek açık belgede tekrarlı düzenleme denemesi bu yükleme senaryosundan ayrıdır.

Önce native SHA-256: `f9541b92a1f301a8bc6a89829e7ef8f5b0a055212e8a7ba7a78d4485cf84a6c7`.
Sonra native SHA-256: `dbf14dfe32eadef66e78b988dd9f113650607ec259f62cee3e609361345ce661`.

Bu sonuç pencere FPS'i veya GPU kare süresi değildir.

## Ekran dışı Iced çalışma zamanı

| Araç / snap | CPU önce p50 / p95 | CPU sonra p50 / p95 | Uygulama önce → sonra p50 |
|---|---:|---:|---:|
| Duvar / kapalı | 4.190 / 5.155 | 0.236 / 0.262 | 3.784 → 0.071 |
| Dikdörtgen duvar / kapalı | 4.483 / 5.006 | 0.257 / 0.269 | 4.024 → 0.084 |
| Duvar / açık | 4.590 / 4.866 | 0.233 / 0.248 | 4.235 → 0.071 |
| Dikdörtgen duvar / açık | 4.738 / 5.016 | 0.258 / 0.276 | 4.367 → 0.085 |

240 sıcak fare hareketi; her araç iptalinde bütün snapshot eşit. wgpu geri okuma
alanları ham JSON'da ayrıca bulunur. Render kodu değişmedi; o sürelerin farkı
GPU motorunun hızlandığına bağlanmaz. Pencere sunumu ve işletim sistemi olay
gecikmesi bu koşucunun kapsamı dışındadır.

## Tek belgede tekrarlı yazma / geri alma ve bellek

`scripts/perf/architecture_preview_memory.py`: 100 bin arka plan duvarı ve bir
çalışma duvarı; 100 kez genişlik değiştir, planla, geri al, yeniden planla.
Bu koşu soğuk matris gibi yeni belge açmaz. İşlenen duvar bütün alanlarıyla eski
hâline döner; nesne sayısı 100001 kalır. Süre ölçümü yapılmaz.

- Önce RSS: 0. döngü **423.4 MiB**, 100. döngü **424.5 MiB**.
- Sonra RSS: 0. döngü **426.9 MiB**, 100. döngü **428.2 MiB**.
- Ara örnekler [önce](memory-before.json) ve [sonra](memory-after.json) kayıtlarında.
  Bu denemede tekrar sayısıyla sınırsız artış görülmedi; genel bellek sızıntısı
  yokluğu veya bütün yükleme yollarının kabulü iddia edilmez.

## Tekrar üretme

```sh
# Üretim uygulama mantığını değiştirmeden önce iki ikiliyi sakla:
# maturin develop --release; native modülü before/ dizinine kopyala.
.run/py/bin/python scripts/perf/architecture_preview.py \
  --native .run/architecture-preview/before/_native.abi3.so \
  --profile release --output .run/architecture-preview/before.json
# Değişiklikten sonra aynı profile/modülü derle ve sakla:
.run/py/bin/python scripts/perf/architecture_preview.py \
  --native .run/architecture-preview/after/_native.abi3.so \
  --profile release --output .run/architecture-preview/after.json \
  --compare .run/architecture-preview/before.json

# Ayrı bellek döngüsü (before ve after modüllerinde birer kez):
.run/py/bin/python scripts/perf/architecture_preview_memory.py \
  --native .run/architecture-preview/after/_native.abi3.so \
  --output .run/architecture-preview/memory-after.json

# Aynı test ikilisini değişiklik öncesinde ve sonrasında ayrı sakla/çalıştır:
cargo test --release -p kentos-desktop --bin kentos-cad \
  perf::architecture::moving_preview --no-run
# KENTOS_ARCH_PERF_FILE açık bir izole KCAD yolu, OUTPUT ayrı JSON yoludur.
# XDG_CONFIG_HOME ve XDG_DATA_HOME izole test dizinlerine yönlendirilmelidir.
# <saklanan-ikili> perf::architecture::moving_preview \
#   --exact --ignored --nocapture --test-threads=1
```

## Koruma sınırı

Yalnız FP64 aday kutuları dizinlenir; gerçek duvar/birleşim/uyarı hesabı güncel
nesnelerden çalışır. Dizin dosyaya yazılmaz. Gizli/kilitli/evreli duvarların eski
katılımı değişmez. Duvar yazılması/silinmesi, undo/redo, başarısız işlem ve dış
edit dizini geçersiz kılar. Okuma kopyasının dizini diğer kopyayı kirletemez.
Geçersiz kutular için eski kesişme koşulu korunur; bu geçersiz dosya kabulü değildir.
Yay desteği, birimler, yazı/ölçü boyları ve render/zoom hattı bu dilimde değişmez.

## Gerçek pencere etkileşimi

İzole QA uygulaması; sentetik `.run/architecture-preview/interactive.kcad`.
1440×900 mantıksal pencere, 2880×1720 Retina ekranı; açık tema, 18 px yazı;
metre / 1:100; 100 bin uzak duvar ve 12 yerel kesişen duvar. `XDG_CONFIG_HOME`
ve `XDG_DATA_HOME` test dizinidir. Kurtarma kopyası **Sonra** ile korundu.

- Önceki davranış: [ilk açılış](01-before-open.png), [Mimari](02-before-architecture.png),
  [duvar oluşturma](03-before-interaction.png), [tek geri alma](04-before-undo.png),
  [sağ tık menüsü](05-before-preview-menu.png), [salt önizleme](06-before-preview.png).
  İlk sürükleme duvarı yazdı; geri alındı. Sürükleme salt önizleme kanıtı sayılmadı.
  Sonraki gerçek sağ tıkla imleç taşıma + menüyü Escape ile kapatma yalnız önizleme oluşturdu.
- Güncel sürüm: [kurtarma korunumu](07-own-recovery-preserved.png),
  [100012 nesne](08-after-open.png), [aynı 5.068×4.592 m önizleme](09-after-preview.png).
  [İkinci imleç](10-after-second-pointer-edit.png): 4.202×4.072 m, nesne sayısı aynı.
- [Boşluklu birim girdisi reddedildi](11-invalid-spaced-unit-rejected.png);
  eski değer ve belge korundu. `0.300m` ile [kalınlık 0.300 m oldu](12-after-width-preview.png),
  çizim hâlâ 100012 nesne ve temiz. Bu deneyin bulduğu boşluklu birim kullanım kusuru
  ayrı takip işidir; bu performans diliminde parser sessizce değiştirilmedi.
- [Onayla/bitir](13-after-finish-menu.png) bir kez tıklandı:
  [araç çıktı, yarım taslak kalktı, nesne eklenmedi](14-after-one-finish.png).
- İki gerçek sol tıkla [bir duvar yazıldı: 100013](15-after-create.png).
  [Bir undo: 100012](16-after-undo.png); [bir redo: 100013](17-after-redo.png).
  `Cmd+S` sonrası başlık temizlendi. [Yakınlaştır](18-after-zoom-in.png) /
  [uzaklaştır](19-after-zoom-out.png) sonrası birim/çizim ölçeği değişmedi.
  Bu iki resim FPS veya titreme yokluğu sertifikası değildir.

1–6 önceki debug GUI SHA-256 `ce53498b5932f8eaf6af1d2a43ffa96faa76c46a4ab9cf288e78d594c3625583`;
7–19 güncel release GUI `3704d5b5ebce0687a37c7d6a77b33f316c4a3f917078fa5d440caa9aec329820`.
GUI resimleri işlevsel karşılaştırmadır; bu iki farklı profilin pencere hızları
karşılaştırılmadı. Yukarıdaki sayısal karşılaştırmaların her iki tarafı release'dir.
Son clippy düzeltmesi yalnız `join_reach_bounds` üzerinde eski native NaN min/max
semantiğini koruyan, gerekçeli lint istisnasıdır; çalışma zamanı hesabı değişmedi.

Kaydedilen dosya bağımsız `tools/kcad/kcad.py` ile çözüldü: önceki 100012 nesnenin
UID'si ve bütün alanları birebir aynı; bir yeni kapalı, dört köşeli, 0.3 m duvar;
birim metre ve çizim ölçeği 1:100. [Makine kontrolü](saved-file-check.json).

## Otomasyon ve açık kabul

- Masaüstü release: **1274 geçti / 219 isteğe bağlı test atlandı**, 37.02 s.
- Domain: **103 geçti** (10 birim + 66 entegrasyon + 22 belge + 5 pafta geçmişi).
  Yeni 6 test eski tarama oracle'ı, sıra/dokunma/uzak koordinat, yazma/silme/tür
  değiştirme, undo/redo/iptal/ret, dış edit, katman bayrakları, okuma kopyaları
  ve sonlu olmayan savunma kutularını denetler.
- Native uygulama: **51 birim + 158 entegrasyon geçti / 22 atlandı**;
  bütün eski komut fixture'ları ve duvar/açıklık/mahal/bağlı ölçü sonuçları korundu.
- Python SDK: **6 duvar testi geçti**, 0.461 s; birimler, fiziksel alan, açıklık,
  mahal, ilişkili ölçü, undo, KCAD ve atomik retler. 24 karşılaştırmalı performans
  durumu ayrıca aynı SDK/FFI komutunu kullanır.
- Dört ilgili paketin tüm hedefleri `clippy -D warnings` ile geçti, 175 s.
  Bağımlılık yönü: 47 crate / 61 hedef; Python derleme ve diff denetimi geçti.
- Geometry core: **769 geçti / 29 isteğe bağlı test atlandı**; headless **64 / 1**,
  MCP **56** geçti. Komutların ve planların ortak davranışı hostlarda da doğrulandı.
- Geometry WASM derlemesi geçti, 55.03 s; web UI üretilmedi/değiştirilmedi.

ARCH-PERF-01 `[~]` kalır. Gerçek işletim sistemi olayından pencere sunumuna gecikme,
GPU timestamp, tüm stil/evre/birleşim değişikliği hız matrisi ve genel bellek/yükleme
kabulü tamamlandı sayılmaz. Ekran dışı Iced ile gerçek pencere ekranları bu eksik
ölçümlerin yerine geçmez. Diğer mimari plan maddeleri de açık kalır.
