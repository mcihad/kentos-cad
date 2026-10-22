# 0587 — CAD değişimi: gerçek paftalar, dış başvurular ve kayıp sözleşmesi

22 Ekim 2026. CAD-IO kabul kaydı; sağlayıcı ve Git güncelleme yolu [ADR 0586](0586-dwg-opencadcodec-git.md). Karar/özellik/lisans/platform matrisi [exchange-support.md](../cad/exchange-support.md).

## Uygulama

- Shared DXF okuyucusu önce dosya boyutu, SECTION/ENDSEC/BLOCK/ENDBLK/TABLE/ENDTAB/EOF ve sürüm/kodlama denetler. Yapısı yarım dosyadan kısmi sonuç üretilmez. ASCII sürümleri ve desteklenmeyen binary/UTF-16/codepage için sabit hata kodları vardır. UTF-8 olarak bildirilen geçersiz baytlar reddedilir. Native dosya okuması büyüyen dosyada da 256 MiB sınırını uygular.
- `ReportItem` sayı, önem ve en çok beş gerçek kaynak örneği taşır: entity tipi, özellik, handle ve satır. Satır gerektirmeyen dosya geneli notları örnek uydurmaz. DWG'de satır normalize edilen DXF'nin satırıdır; DWG bayt ofseti değildir. Kaynak handle dosya kapsamındadır.
- Foreign SPLINE degree/knot/weights/control verisi analitik NURBS olur. Rasyonel çeyrek daire örneği KentOS metadata'sı olmadan sınanır. SORTENTSTABLE sıralaması (0 anahtarı sonda, eşitlerde sabit sıra) entity üretiminden önce uygulanır. Dışa aktarımın katman seçicisi belge sırasını değiştirmez; mevcut nesne order değerleri yazıcının sırasına uygulanır.
- BLOCK dış referansları grup 1 yol, 70 attach/overlay bitleri ve INSERT'in ayrı X/Y ölçek/dönüşünü korur. İçe alma dış dosya açmaz: referans unloaded, digest 64 sıfırla “henüz yüklenmedi” durumundadır. Yeniden yükleme dosyayı açık komutla okur ve gerçek SHA-256 yazar. Göreli yol çizimin klasörüne göre çözülür; CWD referansı değildir.
- `cad.xrefs.edit` DXF ve DWG kaynaklarını attach/reload ile okuyabilir. Kaynak handle'dan türetilen UUID, çevreleyen xref bloğu içinde kapsamlanır; aynı kaynağın tekrar okunması kimliği korur. Handle olmayan kaynakta indeks kullanılır: dosyanın öğe sırası değişirse bu yedek kimliğin kararlılığı garanti edilmez. İç içe dosya kendiliğinden yüklenmez. Xref source ve blok kimlikleri başka dosyalara ham pointer olarak kopyalanmaz.
- LAYOUT ve paper-space entity'leri `CadLayout`, 2B ortografik VIEWPORT'lar `CadViewport` sözleşmesinden gerçek `kentos.sheet/1` kitabına geçer. Sahte model katmanı yaratılmaz. Pafta kâğıdı mm, geometri metre, kitap çerçevesi µm'dir; Y-up/Y-down dönüşümü açı ve merkezle birlikte yapılır. VIEWPORT model hedefi, DCS merkezi ve twist birlikte çözülür. İnaktif *Paper_SpaceN blokları ve group 330 owner eşlemesi kullanılır.
- Pafta çizgi/çokgen/yazı/daire/tam elips ve sabit harita görünümü iki yönlüdür. Eğriler paftanın yerel modeline 0,001 mm kiriş toleransıyla çevrilir; model eğrileri analitik kalır. Kitabın ölçeği tam sayı, açı mdeg ve konumu µm çözünürlüğündedir. Perspektif/eğik 3B görünüm, özel clipping/frozen layer, atlas, ileri pafta öğeleri ve desteklenmeyen dolgular açık kayıp raporundadır. Boş varsayılan Layout1 kullanıcı paftası diye çoğaltılmaz; özel adlı boş paftalar korunur.
- Yazıcı LAYOUT dictionary, BLOCK_RECORD sahipliği, kâğıt alanı işaretleri ve VIEWPORT kayıtlarını üretir. Her paftanın 1 numaralı genel görünümü ayrı kayıttır; DWG ID'leri kalıcı tutmadığından bu kayıt gerçek model görünümünün tekrar açılınca editör sanılmasını önler. Pafta kabulü ASCII AC1021 ve DWG AC1032 üzerinde doğrulanmıştır; diğer DWG hedeflerinin temel model testi ileri pafta kabulü yerine geçmez.
- İçe alınan stiller/çizgi tipleri ve kitap da aynı undo grubundadır. `ImportMetadata` yalnız bu alanları geri alır; arada değişen ilgisiz proje tercihlerini ezmez. Kitap doğrulaması belgeye yazmadan yapılır. Büyük içe almanın progressive finish yolu da aynı metadata adımını kullanır.
- Atomik dosya yazma benzersiz create_new geçici dosya, sync ve içerik doğrulaması kullanır. `overwrite=false` hedefi atomik hard-link ile ayırır; iki eşzamanlı yazandan yalnız biri kazanır. Ön kontrol ile final yazma arasında oluşturulmuş hedef ezilmez. Overwrite açıkken rename kullanılır. Bu, elektrik kesintisinde dizin girdisinin kalıcılığı veya dosya sistemi dışı çoklu işlem garantisi değildir.
- UI, Python ve MCP mevcut komut kataloğunu kullanır. Capability'de model/annotation/xref/layout/dynamic/constraint/proxy/opaque satırları ayrı, offline ve upload davranışı açıkça bildirilir. Codec tipi domain/shared geometriye geçmez. Formats sözleşmesi 95'tir; KCAD şeması değişmedi.

## Kanıt ve açık sınırlar

Bağımsız küçük DXF akışları `scripts/fixtures/cad_io_cases.py` ve `tests/all/cad_io.rs` içindedir. Sayısal oracle: rasyonel çeyrek daire, iki pafta, döndürülmüş DCS/WCS merkezi, ayrı X/Y ölçeği, attach/overlay, kaynak handle ve SORTENTS. Headless testleri gerçek kitap, DXF/DWG yazma/okuma, bozuk dosyada değişmez belge, metadata undo ve source UUID kararlılığını sınar. Kendi codec'iyle tekrar okuma bağımsız DWG motoru testi sayılmaz.

`cad-io-dwg-import/export` etkileşim izleri gerçek masaüstü uygulama mesajlarını, Iced klavye/fare olaylarını ve dosya seçicinin test yanıtını kullanır. Açık/koyu tema, 1440×900 ve 1100×650 görüntüleri ve 18 px arayüz yazısı kontrol edilir. Bu, AutoCAD kontrolü veya OS dosya seçicisinin erişilebilirlik testi değildir.

Mac ARM64 üzerinde native çalışma; codec adaptörü için Linux x64, Windows GNU x64 ve Linux ARM64 derleme kontrolü yapılır. Çapraz derleme hedefte çalıştırılmış kabul edilmez. Ortak format/komut/sözleşme WASM'de derlenir; DWG web ürünü ADR 0228 nedeniyle ertelidir.

**CAD-IO-15 tamamlanmadı:** standart düzlemde width/bulge/closed/elevation ve blok nonuniform scale varsayılan korunmuş INSERT yolunda sınanır. Kotlu düz ve geniş çizginin sabit elevation'ı artık LWPOLYLINE'da kalır; JSON girişindeki width/closed/object override düşmesi giderildi. Fakat eğik OCS'nin geniş/yaylı yolunu WCS'ye izdüşürme, farklı köşe kotu ile genişlik/yay birleşimi ve kullanıcının nonuniform bloğu patlatması hâlâ kayıp/örnekleme doğurabilir. Bunlar tam kapatılmadan madde işaretlenmez; toleransı büyütmek çözüm değildir. Sonraki gereksinim, kaynak düzlem/3B dönüşümü düzenlenebilir modelde temsil etmek ve her affine dönüşümde analitik geniş konturu korumaktır.

**Devam kaydı:** [ADR 0588](0588-polyline-integrity.md), ilave genişlik/ölçek/kapanış düzeltmelerini ve varsayılan ret/açık kabul kuralını içerir; CAD-IO-15 hâlâ açıktır.

**CAD-IO-20 sahibin açık kararıyla atlandı.** AutoCAD çalıştırılmış sayılmaz. Bağımsız ezdxf planar kabulü sonraki ADR 0588’dedir; bağımsız DWG motoru kabulü değildir. Upstream'in decode ortasında iptal ve açılmış bellek kotası sınırları ADR 0586'de sürer.

## Çalıştırılan son doğrulamalar

- Shared formats: 184 birim + 103 okuma/analiz + 50 DXF yazma + 6 diğer format testi geçti; 7 mevcut performans testi çalıştırılmadı. Yeni CAD-IO korpusu 9 test içerir; LWPOLYLINE/POLYLINE ve flattened JSON override denetimleri dahil.
- Native application: 40 birim testi ve 8.045 ortak durumu oynatan 125 entegrasyon testi geçti; 22 mevcut performans testi çalıştırılmadı.
- Domain: 66 birim ve 22 entegrasyon testi geçti. Headless genel 36 test ve son CAD-IO odaklı 5 test geçti; yalnız paftası olan dosyanın import/export/undo yolu da var. Adaptörün 4 testi yedi DWG hedefini sınar.
- Masaüstü katalog/geçmiş başlıkları: 44 geçti, 5 mevcut test ignored. İki etkileşim izinin dört tema/boyut birleşimi geçti; fareyle nesne seçimi, Esc, Ctrl+Z, sürüm seçimi ve dosya yazma çalıştırıldı. 18 px görüntü ayrıca incelendi. UI görüntüleri bu çalıştırmada `/tmp/kentos-cadio-ui-final` altında üretildi.
- Python: izole uv Python 3.14 ortamında yeniden derlenen paketle 207 test geçti; tipli DWG seçimi/idempotency ve iki paftanın DXF→kitap→DWG→kitap yolu dahil. Kullanıcının bozuk `.run/py` interpreter bağlantısı değiştirilmedi.
- MCP: gerçek `file.import.dwg` çağrısı geçti. Komut kataloğu 3, TS üretimi 861 test geçti; SDK/manifest ve bağımsız fixture üreticilerinin `--check` denetimleri temiz.
- Adapter Linux x64 / Windows GNU x64 / Linux ARM64 ve ortak format/komut/sözleşme WASM derlemeleri geçti. Hedeflerde runtime veya masaüstü dağıtımı çalıştırılmadı.
- Codec/formats/domain/application/headless/desktop/Python için `clippy --all-targets -- -D warnings` temiz. Mimari bağımlılık denetimi 47 crate / 61 hedef birleşiminde geçti. Üretilen TS'nin üretici kaynaklı satır sonu boşlukları ve DXF'nin zorunlu CRLF baytları dışında değişen kaynaklarda whitespace kontrolü temiz; binary/golden veriler elle normalize edilmedi.
