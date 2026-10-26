# Mimari şablonunu farkını görerek uygulama — kabul

24 Ekim 2026 · `codex/architecture-2d-workflows` · ARCH-UX-03 / ADR 0659.
Bu kayıt şablon diliminindir; mimari yol haritasının diğer açık kabullerini kapatmaz.

## Sonuç ve regresyon sınırı

Mimari şablonu önce fark penceresi açar. Varsayılan mevcut birim/çizim ölçeğini
korur; yerel çizimde ofis ayarı cm/1:50 farkını açık gösterir. Koordinat sistemli
CAD'de ofis seçeneği m/1:50'dir; başka birimi isteyen API çağrısı yazmasız reddedilir.
Geometri, mevcut font/tanım ve gizli/kilitli katman durumları korunur. Eksik parçalar
eklenir; kat/stil kimlik-ad eşleşmeleri gösterilir. Eski revizyona veya başka belgeye
onay taşınmaz. Paket ayarlar ve paftalarla tek undo/redo adımıdır.

Eski `cad.templates.apply` v1 çağrısı `settingsMode` göndermiyorsa eski davranışı
korur. Yeni çağrılar açık `keep`/`template` kullanır. Birim değişimi SI model
koordinatlarını ölçeklemez. Açıkça seçilmiş çizim ölçeğinde mevcut varsayılan mimari
gösterimin yazı/kalem boyu kuralı sürer; bu dilimde render/zoom/import hesabı değişmez.
Python ve MCP aynı native komutu kullanır. Web UI ADR 0228 gereği ertelenir.

## Gerçek pencere etkileşimi

Ayrı test uygulaması ve `.run/architecture-ux03/` test kopyaları kullanıldı.
Kurtarma kopyaları Sonra ile korundu; test için geri yüklenmedi/silinmedi.
Açık tema, 18 px yazı; 1440×900 ve 1100×650 pencere içerikleri (Retina ekranları 2×).

- [Varsayılan önizleme](01-default-preview.png): m→m, 1:100→1:100 ve eklenecek
  parça adları. [Yerel ofis seçimi](02-office-preview.png): m→cm / 1:100→1:50.
  [Vazgeç](03-cancel-unchanged.png): aynı üç nesne, 48 m² ve 1:100.
- [Dar önizleme](04-narrow-preview.png) ve [fareyle kaydırılmış liste](05-narrow-scroll-and-actions.png):
  paftalar erişilir, Vazgeç/Uygula sabit görünür. [Bir tık Uygula](06-applied-keep.png):
  aynı 48 m², üç yeni pafta ve 1:100.
- [Tek geri alma](07-one-undo.png) paftaları kaldırdı; [tek yeniden yapma](08-one-redo.png)
  geri getirdi. [İkinci önizleme](09-existing-definitions.png) bütün eklemeleri sıfır
  gösterir. [Korunan tanımlar](10-preserved-list-before-width-fix.png).
  İkinci uygulamadan sonra [bir geri alma](11-repeat-then-one-undo.png) yine bütün
  paketi kaldırdı: boş history adımı eklenmedi. [Kaydet](12-saved-package.png).
- Liste genişliğinin içeriğe göre daraldığı bu denemede görüldü ve düzeltildi.
  [Kaydet/aç](13-reopened-package.png), [tam genişlik](14-final-full-width-preview.png)
  ve [listenin sağ tarafından çalışan kaydırma](15-final-right-side-scroll.png).
- Son projeksiyon düzeltmesi: [EPSG:5256, 48 m² / 1:100](16-projected-start.png);
  [ofis önizlemesi m→m / 1:100→1:50](17-projected-office-preview.png);
  [Uygula: aynı 48 m² / 1:50](18-projected-applied-50.png);
  [bir geri alma: paftalar kalktı, 1:100 geri geldi](19-projected-one-undo-100.png).

İlk 1–12 resimleri SHA-256 `7fd9041f7334f8f8e0fbf02fda3aa645cacc4920388bfb648facfac04ab9a2f4`,
13–15 `ccbf0ae71564251bac25586b4e2f444ae3db3453fe9083745bdfffc5e3359695` derlemesinden.
Son 16–19 ve nihai uygulama: `ce53498b5932f8eaf6af1d2a43ffa96faa76c46a4ab9cf288e78d594c3625583`.

Arayüzden kaydedilen yerel dosya SDK ile yeniden okunup önceki dosyayla karşılaştırıldı:
üç nesnenin kimliği ve bütün geometri verisi birebir eşit; metre/1:100, on ek katman
ve üç pafta. [Makine sonucu](saved-file-check.json). Native test ayrıca bütün KCAD
snapshot'ını kodlayıp açar; bu kontrol yalnız ekrandaki alan metnine dayanmaz.

## Otomasyon

- Tam masaüstü: **1274 geçti / 218 isteğe bağlı test atlandı**, 125.85 s
  (`architecture-ux03-desktop-accepted`). Son hedef kümesi **5 geçti**, 13.54 s:
  yazmasız önizleme/iptal; revision/oturum koruması; paket undo; dar widget tıklaması;
  yerel ve projeksiyonlu ofis politikası.
- Native uygulama: **51 birim + 158 entegrasyon geçti / 22 atlandı**
  (`native-mcp-final`). Beş yeni hedef: bütün snapshot undo/redo, KCAD roundtrip,
  ayrı kimlik/ad çakışması, hata atomikliği ve ilgisiz tercihlerin korunumu.
- Bağımsız referans **34 yeni durum**; beş birim × boş/dolu × iki kip, dört açık
  birim, korunan font/katman/kat/duvar/blok, politika çelişkisi, projeksiyon birimi.
  Eski **23 şablon durumu değiştirilmedi**; standartların ve tüm komutların fixture
  koşusu native entegrasyonda geçti.
- Domain **97**, headless **64** (bir isteğe bağlı test atlandı), MCP **56** test geçti.
  Domain/headless genel koşusu `core-hosts`; MCP'nin düzeltilmiş son koşusu
  `native-mcp-final`. Bunlar farklı katmanların testleridir; yinelenen koşular
  toplama tekrar eklenmedi.
- Sözleşmeler **795 geçti**; katalog, SDK ve fixture güncelliği geçti. TS tip
  testlerinin çıktısı `.run/architecture-ux03/contracts-ts/` dizinine yönlendirildi;
  geçici `bindings/` çıktıları Git'e eklenmedi. Web UI geliştirilmedi.
- Python: **269 geçti**, 247.238 s; dört yeni SDK testi dahil. Gerçek yerel native
  modül yeniden derlendi (`projected-python`); SDK ve Python crate clippy geçti.
- [GPU matrisi](gpu/): **16 görüntü**, 66.63 s. Varsayılan 13 px ve 18 px, 18 px
  ofis/mevcut tanım pencereleri; iki genişlik ve iki tema. Her konfigürasyon gözden
  geçirildi; son projeksiyon düzeltmesinde etkilenen dört ofis görüntüsü tekrar
  incelendi. Otomatik görüntüler yukarıdaki gerçek fare testinden ayrı kanıttır.
- İlgili altı paketin tüm hedefleri `clippy -D warnings` ile geçti (132 s).
  Geometry WASM derlemesi, şerit/887 satırlık manifest, bağımlılık yönü
  (47 crate / 61 hedef) ve `git diff --check` geçti.

## Yakalanan hatalar ve koruma kararı

Genel `set_settings` undo davranışı değiştirilmedi. Yalnız paketin yazdığı birim,
ölçek, stil listeleri, katlar ve mahal yöntemi için dar bir domain işlemi eklendi;
paketin dokunmadığı tercihler veya korunmuş birim/ölçek undo ile geri sarılmaz.
Standart katmanın yazdırma bayrağı ekleme kaydına alındı; redo artık ilk uygulamayla
aynıdır. Açık kipler eksik stilleri ekler; eski standardın mevcut fontları değiştiren
fix yolu şablon üzerinden çalıştırılmaz. Mevcut standardın ve eski şablon çağrısının
sözleşmesi korunur. FPS/zoom titremesi ve tam mimari plan kabulü bu dilimden geçmiş
sayılmaz; ARCH-PERF-01 ve kalan plan maddeleri açıktır.
