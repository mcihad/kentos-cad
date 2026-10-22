# 0580 — DWG için OpenCADStudio'nun OpenCADCodec Git bağımlılığı

Tarih: 22 Ekim 2026. Durum: sahibin seçimiyle kabul; temel 2B entegrasyon, üretim DWG uyumluluğu kabulü henüz değil.

## Karar

Sahibin son talebi kaynakları gömmek yerine OpenCADStudio'nun kullandığı kütüphaneyi Git üzerinden izlemektir. `acadrust` 0.6.4, `https://github.com/HakanSeven12/opencadcodec.git` deposunun **ff75fddc322bda4cf817c5f68535c5c2b316b9c4** revizyonuyla workspace bağımlılığıdır. Bu, incelenen OpenCADStudio `25b25aea176c12754b0e7a34c00f983e3262f9c5` kilidindeki revizyondur. `Cargo.lock` kaynağı ve geçişli sürümleri sabitler. Geçici gömülü codec kopyası kaldırılmıştır; `crates/native/dwg` yalnız KentOS adaptörüdür.

Kaynak: [OpenCADStudio Cargo.lock](https://github.com/HakanSeven12/OpenCADStudio/blob/25b25aea176c12754b0e7a34c00f983e3262f9c5/Cargo.lock), [codec Cargo.toml](https://github.com/HakanSeven12/opencadcodec/blob/ff75fddc322bda4cf817c5f68535c5c2b316b9c4/Cargo.toml), [codec lisansı](https://github.com/HakanSeven12/opencadcodec/blob/ff75fddc322bda4cf817c5f68535c5c2b316b9c4/LICENSE).

OpenCADStudio uygulaması GPL-3.0, seçilen ayrı codec MPL-2.0 lisanslıdır. KentOS uygulama kabuğunu almaz. Dağıtımda codec ve geçişli bağımlılıkların lisans/bildirimleri saklanır; değiştirilen MPL dosyalarının kaynak sağlama yükümlülüğü ayrıca izlenir. Bu kayıt bütün ürünün dağıtım koşullarını onaylamaz.

## Güncelleme yöntemi

1. OpenCADStudio'nun kilidindeki codec revizyonunu ve codec değişikliklerini incele.
2. Workspace `acadrust.rev` ve `kentos_dwg::REVISION` değerlerini birlikte değiştir; `cargo update -p acadrust` ile kilidi güncelle.
3. Yedi hedef sürümün testlerini, bağımsız ASCII geometri referansını, komut/undo/atomik yazma denetimlerini çalıştır; güncellenen korpusu ve lisansları incele.
4. Destek tablosunu yalnız kanıt varsa genişlet. Başarısızlıkta önceki revizyon ve kilide dön.

`main` dalı otomatik takip edilmez. Yukarıdaki depoya yeni commit gelmesi kurulu KentOS'u kendiliğinden değiştirmez; yeni KentOS derlemesine test edilmiş revizyon alınır. Kullanıcı OpenCADStudio, Python paketi, dönüştürme servisi veya ayrı codec programı kurmaz. İlk derlemede Cargo kaynağı indirir; önbellekteki kaynaklarla çevrimdışı derleme yapılabilir. Çalışma sırasında dosya ağ üzerinden gönderilmez.

## Sınır ve kapsam

- `kentos-dwg` giriş/çıkışı byte dizileri ve KentOS sözleşmeleridir; codec türleri domain, uygulama sözleşmeleri ve geometri modeline geçmez. `default-features=false`. Adaptör native katmandadır: upstream `web-time` WASM üzerinde JavaScript bağımlılığı getirir; saf shared katmana bu bağımlılık sokulmaz. Mimari denetimi codec için yalnız native hedefi ve mevcut UI/runtime/ağ yasaklarını uygular. Harici C/C++ SDK ve FFI eklenmez.
- DWG bellekte DXF AC1032'ye çevrilir; mevcut KentOS birim, blok, stil ve katman okuyucusu kullanılır. Kaynak dosya değiştirilmez. Yazma mevcut KentOS DXF üretimi üzerinden seçilen DWG biçimine gider.
- Kaynak binary imza ile hedef `DwgVersion` ayrı taşınır. Hedefler AC1014, AC1015, AC1018, AC1021, AC1024, AC1027, AC1032. Bir uygulamanın yıl numarası dosya imzası yerine kullanılmaz.
- R14 INSUNITS ve nesne çizgi kalınlığını taşımaz. İçe alırken eksik birim açıkça varsayılır veya `fileUnit` ile seçilir. Eski okuyucunun eksik kalınlık için verdiği sıfır, adaptörde ByLayer yapılır.
- İçe aktarma `file.import.dwg` validate/plan/execute ve ortak katman yönlendirmesiyle tek undo adımıdır; bozuk dosya sonuç üretmez. Masaüstü ağır okuma/yazmayı iş parçacığında yapar.
- Dışa aktarma `file.export.dwg` açık `uids`, `layers`, `annotations`, `version`, `overwrite` girdileriyle çalışır; plan yazmaz. Dosya önce bellekte üretilip aynı codec ile tekrar okunur, sonra mevcut atomik dosya yazıcısı kullanılır. Aynı codec ile açma bağımsız DWG doğrulaması değildir.
- Okuyucu atlanan kayıtları, codec tanılarını ve KentOS DXF kayıplarını raporlar. Rapordaki satırlar normalize edilmiş ara DXF'ye aittir; özgün DWG bayt konumu oldukları iddia edilmez.

## Sınırlar ve kabul kanıtı

256 MiB giriş/ara çıktı sınırı vardır; bu bir açılmış bellek kotası değildir. Sıkıştırılmış kötü niyetli dosyalara karşı süreç izolasyonu, açılmış bellek ve işlem süresi kotası üretim kabulünün açık işidir. Codec iptal kancası sunmadığından iptal codec öncesi/sonrası ve KentOS DXF aşamasında denetlenir; uzun DWG çözümlemesinin ortasında kesilme garanti edilmez. `catch_unwind` yerel panic'i sınırlar; OOM ve panic=abort için güvence sağlamaz.

Adaptörün test korpusu bağımsız ASCII DXF geometri/birim verilerinden üretilen yedi DWG hedefini ve entity, block, text, style, dimension, hatch, OCS, elevation örneklerini içerir. Beklenen geometri bağımsızdır; **binary DWG örnekleri bu codec ile üretildiği için bağımsız DWG motoru kabulü değildir**. Gerçek müşteri DWG'leri, AutoCAD'de görsel/düzenlenebilirlik, Windows/Linux/ARM64 dağıtımları, performans/bellek ölçümü ve ileri içerik kabulü açık kalır. Test edilen platformları çalıştırılan kontrollerle raporla; upstream desteğini KentOS garantisi sayma.

Dynamic block, constraint, AEC/proxy ve bilinmeyen dictionary/xdata için kayıpsız düzenleme iddia edilmez. Gerçek 2B paper-space/layout/viewport ile xref attach/overlay takip çalışmasında eklendi; kapsam ve güncel kabul kayıtları [ADR 0581](0581-cad-io-layout-xref-and-loss-contract.md) içindedir. Aşağıdaki sayılar ilk dikey dilimin tarihsel test kaydıdır. Web geliştirmesi durduğundan yeni web arayüzü/WASM binding eklenmez (ADR 0228); sözleşme ve katalog üretimi devam eder.

## Bu değişiklikte çalıştırılan kontroller

- Native adaptör: 4 test; yedi hedef sürüm ve sekiz ek ASCII kaynak örneği.
- Native application `--test all`: 125 geçti, 22 mevcut test varsayılan olarak ignored (performans kabulü çalıştırılmadı); yeni DWG komutunun 27 ortak durumu bu koşuya dahil.
- Headless dosya/komut entegrasyonu: 33 test geçti; seçili nesne, plan/yazma hash eşitliği, yedi imza, R14 kayıp uyarısı, eski hedefin korunması ve dosya günlük etkisi dahil.
- Masaüstü katalog: 8 test geçti; gerçek pencere klavye/fare, tema ve yazı ölçeği etkileşim kaydı bu değişiklikte yapılmadı.
- MCP: yeni DWG içe alma aracı üzerinden gerçek komut çağrısı geçti.
- Python: mevcut 205 test ve yeni tipli DWG plan/import/undo/export/idempotency testi geçti. `pnpm py:test` mevcut `.run/py` ortamının silinmiş Homebrew Python 3.14'e işaret etmesi nedeniyle başlayamadı; kullanıcının ortamı değiştirilmedi. Aynı paket mevcut uv Python 3.14.3 ile `/tmp/kentos-dwg-python-env` içinde maturin/uv kullanılarak derlendi ve test edildi; Python crate clippy kontrolü ayrıca geçti.
- Katalog 3 test; TS binding üretimi 855 test; SDK/fixture/manifest drift ve mimari denetimi (47 crate / 61 hedef birleşimi) geçti. WASM'de sözleşme/komut derlemesi geçti; DWG web ürünü eklenmedi.
- Native adaptör, application, headless, desktop ve Python `clippy --all-targets -- -D warnings` temiz. Mevcut ts-rs `deny_unknown_fields` üretim uyarıları ve dış `block` paketinin future-incompatibility bildirimi devam eder.
