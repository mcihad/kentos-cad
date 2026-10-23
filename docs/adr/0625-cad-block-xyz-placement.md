# 0625 — CAD bloklarında tam XYZ yerleştirme

23 Ekim 2026. `codex/cad-inspector-units-open` dalında, ana çalışma klasöründe.
[ADR 0588](0588-polyline-integrity.md) / CAD-IO-15 için kısmi ilerleme; madde açık kalır.

## Bulunan hata ve nedeni

DXF okuyucu blok dönüşümünü bir XY matrisi ve bağımsız Z çarpanı/ötelemesi olarak taşıyordu.
Eğik bir OCS'de Z, X/Y'ye; X/Y de Z'ye katkı verir. İç blokta önce XY'ye düşürüp sonra
üst bloğu uygulamak bu katkıları geri getiremez. Bu, gösterim birimi veya pafta ölçeği sorunu değildir.

Bağımsız metre kaynaklı `tilted-yz-line` örneğinde çizginin ikinci ucu Y=9,2 / Z=55,6 olmalıdır;
önceki hesap Y=−2,8 / Z=40 üretiyordu. İlk ucun Z=36 yerine 24 olması da aynı hatadandır.
SOLID/TRACE/3DFACE ayrıca köşe kotlarını tamamen atıyordu; yalnız XY'ye göre tekrarlı köşe
elemek, düşey bir kenarı da silebiliyordu.

## Karar ve uygulama

- Okuyucunun özel `Map3` dönüşümü üç baz vektörü ve orijin taşır. Çizim modeli veya kalıcı
  sözleşmeye yeni alan eklenmez. Mevcut `za/zb`, `z`, `zs` alanları kullanılır.
- INSERT sırası: eksi XYZ blok tabanı, XYZ ölçek, ölçeklenmeyen dizi aralığı, yerel Z dönüşü,
  OCS yerleştirme noktası, OCS bazları, üst blok dönüşümü. XY izdüşümü bütün üst dönüşümlerden sonra alınır.
- LINE, POINT, düz 2D/3D POLYLINE ve LWPOLYLINE'ın plan koordinatları ile kotları aynı
  bileşik dönüşümden hesaplanır. 2D POLYLINE'da köşe Z'si yerine başlığın düzlem kotu kullanılır.
- SOLID ve TRACE kendi OCS'sinde 1–2–4–3 sırasıyla; 3DFACE dünya koordinatında 1–2–3–4
  sırasıyla alınır. Köşe tekilleştirmesi XYZ'ye göre yapılır; alanın mevcut `zs` alanı korunur.
  Yerel temsilleri kapalı alandır; özgün yüz entity türünün birebir korunması iddia edilmez.
- Desteklenen düz XYZ nesnelerin eğik bloktan alınması, kullanıcının **Blokları patlat** seçmesiyle
  çalışır; artık bunlar için gerçekte olmayan bir geometri kaybını kabul etmesi gerekmez.
- Patlatmadan eğik blok hiyerarşisini koruma desteği eklenmez. Eğik/afin yaylar, genişlikler ve kaynak
  düzlemini tutamayan diğer nesneler için `polyline_geometry_loss` varsayılan ret olarak kalır.
  Açık yaklaşık kabulde elips/spline izdüşümünün konum ve yön vektörleri de tam XYZ dönüşümü kullanır.
- Genel düzlem/afin kaynak modeli, analitik eğri/genişlik düzenlemesi, bu modelin KCAD/Python/AI ve
  standart DXF'e taşınması ADR 0588'deki açık kabul kapsamıdır. Bu düzeltme bunları tamamlamaz.

## Bağımsız doğrulama

- `scripts/fixtures/cad_spatial_cases.py`: KentOS kullanmadan **50 basamak Decimal** ile 64 DXF ve
  ortak komut senaryosu üretir. Sekiz yerleştirme × sekiz nesne türü; negatif normal, eğik/kenar
  görünüşlü düzlem, aynalı/eşit olmayan ölçek, iç içe blok, 3×2 MINSERT ve milyarlık kaynak
  koordinatında küçük ayrıntı. Metre/mm başlıkları birlikte sınanır. Toplam 104 nesne, 312 XYZ köşe.
- Beklentiler uygulamanın çıktısından üretilmez. Çeyrek dönüş ve 3–4–5 normalleri bağımsız
  rasyonel hesap verir. Sayı kontrolü koordinat başına `32 × epsilon × |beklenen| + 10⁻¹² m`;
  ortak komut koşucusunun tek mutlak sınırı için aynı formül senaryonun en büyük koordinatına uygulanır.
  Bu yalnız kayan nokta aritmetiği sınırıdır; kullanıcı kenet toleransı veya kaynak ölçü belirsizliği değildir.
- `scripts/fixtures/cad_spatial_reference.py`: ezdxf'nin kendi OCS, INSERT `matrix44`, `multi_insert`
  ve WCS köşe yordamları aynı Decimal kaynağı ayrıca sınar. `--exports` KentOS'un standart DXF
  çıktısını sınar; KentOS XDATA'sı çıkarılır. ezdxf yalnız test ortamındadır, ürün bağımlılığı değildir.
- Ortak `file.import.spatial.json`: gerçek ürün komutu, tek undo/redo; native ve tipli Python koşucuları.
  Python ek kabulü taşıma, kotların değişmemesi, undo/redo, UID koruyan KCAD kayıt/yeniden açılış ve
  özgün kaynak baytlarının değişmemesini sınar. MCP aynı 64 kaynağı gerçek araçlarla içe alır ve geri alır.
- `cad-spatial-import` masaüstü izi gerçek içe alma penceresi, açık patlatma, kabul kutusuna gerek
  duymayan düz XYZ geometri, seçim, undo/redo ve KCAD yeniden açılışında koordinat/kot doğrular.
  Ortak `saveAndReopen`, içe alma kaynağının seçicisi yerine geçici KCAD hedefini kullanır;
  açılan dosyanın hedef KCAD olduğu ve belge oturumunun gerçekten değiştiği ayrıca doğrulanır.
  `acceptance-room` mevcut mimari oda senaryosu da çizim/düzenleme/ilişkili ölçü regresyonunu sınar.

Birincil standart başvuruları: [Autodesk OCS](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-D99F1509-E4E4-47A3-8691-92EA07DC88F5.htm),
[arbitrary axis algoritması](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-DXF/files/GUID-E19E5B42-0CC7-4EBA-B29F-5E1D595149EE.htm),
[ezdxf LWPOLYLINE ve WCS/OCS](https://ezdxf.readthedocs.io/en/stable/dxfentities/lwpolyline.html).

Web arayüzü ADR 0228 uyarınca ertelidir; ortak okuyucu/WASM ve komut sözleşmesi sürer.
AutoCAD'de görsel/düzenleme kabulü yapılmış sayılmaz (CAD-IO-20 kullanıcının kararıyla atlandı).

## Bu çalıştırmada doğrulananlar

- Formats paketinin 185 birim, 113 okuma/analiz, 50 DXF yazma ve 6 diğer format testi geçti;
  mevcut 7 performans testi ignored olarak kaldı ve geçmiş sayılmaz.
- Bağımsız ezdxf 1.4.5 hem 64 kaynakta hem 64 standart DXF çıktısında 104 nesne / 312 XYZ köşeyi doğruladı.
- Native ortak komut koşucusunda seçilen 287 senaryo geçti: yeni 64 kaynak, önceki DXF/DWG ve çoklu çizgi
  oluşturma/kayıp politikası örnekleri. Kaynak yay/genişlik için önceki varsayılan ret kuralları korunuyor.
- Tipli Python'ın yeni kabulü 64 kaynakta içe alma, taşıma, undo/redo, KCAD kayıt/yeniden açılış,
  UID ve kaynak baytlarının korunmasını doğruladı (macOS geliştirme paketinde 0,36 s).
- Güncel native paketle Python SDK'nın 259 testi geçti; ortak komut korpusu da bu çalıştırmaya dahil.
- MCP'nin 28 testi geçti; yeni 64 kaynak gerçek `file.import.dxf`, `drawing.entities`, undo/redo ve kapatma
  araçlarıyla kontrol edildi. Gerçek stdio sunucusu testi de geçti.
- Downloads'taki `2-1-ev-kat-plani.dwg` (282 nesne), `toki-3+1-sc3-plan.dxf` (201, inç) ve
  `Tek-katli-verandali-koy-evi-dwg.dwg` (9.730) güncel Python native paketiyle açıldı, KCAD kaydedilip
  yeniden açıldı. Bütün entity verileri ve UID'ler eşit; özgün kaynakların SHA-256'sı değişmedi.
- `cad-spatial-import`, önceki `cad-io-polyline-integrity` ve mimari `acceptance-room` izleri
  US / Türkçe Q / 2× ekran varyantlarında geçti (9 akış). Mevcut **Blokları patlat** kutusunun
  iz sürücüsündeki eksik bağlantısı da eklendi; kutunun gerçek `Event::Explode` olayı kullanılır.
- Son kayıt/yeniden açılış sürücüsüyle masaüstünün 172 etkileşim izi üç varyantta yeniden çalıştırıldı
  ve **516 akış geçti** (pending iz yok); kaydedilen KCAD yolu ve yeni belge oturumu ayrıca denetlenir.
- Güncel masaüstüyle açık/koyu tema ve 18 px yazıda dört GPU görüntüsü `.run/shots/cad-xyz-*` altında
  alındı ve açılıp incelendi. Özelliklerde başlangıç kotu 36000 mm, bitiş kotu 55600 mm;
  yatay uzunluk 13994,285 mm ve 3B uzunluk 24083,189 mm ayrı gösteriliyor. Önizlemede açık patlatma
  sonrası gerçek bir eğri/genişlik kaybı yokken kayıp kabulü istenmiyor; İçe aktar etkin.
- Değişen Rust paketleri ve otomasyon host'larında `clippy --all-targets -- -D warnings` geçti.
- Ortak `kentos-formats` ve `kentos-dxf-wasm` paketlerinin `wasm32-unknown-unknown` derleme denetimi geçti.
  Bu, tarayıcı arayüzü testi veya WASM çalışma zamanında sayısal eşitlik testi olarak sunulmaz.
