# ADR 0599 — Canlı model görünümü ve pafta başvuruları

Tarih: 22 Ekim 2026. CAD-SHEET-14.

## Kaynak sözleşmesi

`MapItem.source` üç anlamı açıkça ayırır: `drawing` bağımsız kamera, `viewSnapshot` alınmış kamera kopyası, `viewLink` adlı model görünümünün canlı bağıdır. Kopyada ve canlı bağda ilk `NamedView` kaydı saklanır. Canlı bağın hedefi sabit `view_key` kimliğidir; güncel kamerayı yakalanmış adla aramak veya aynı adlı başka görünüme bağlamak yoktur. Kimliksiz eski görünümler mevcut ad tabanlı kimlikle alınır; yeniden adlandırma mevcut görünüm komutunun sabit kimlik politikasını izler.

Ortak `effective_fixed_view` geçerli adlı görünüm kaydından model merkezi/dönüşünü alır ve görünümün dünya genişlik/yüksekliğini iç boşluk/yazı bandından kalan çerçeveye sığdıran gerçek paydayı hesaplar. Kâğıt boyu ve modelin 1:1 geometrisi değişmez. Kaynak yenilendiğinde bu türetilmiş kamera değişir; kayıtlı ilk capture, kâğıt geçmişi ve kaynak nesneler render sırasında değiştirilmez. Kopya kameranın merkezi/ölçeği değişmeden kalır.

`RenderInputs.namedViews` aynı kayıt listesini ortak çekirdeğe verir. Masaüstü, native/headless PDF ve CAD alışverişi aynı kamera hesabını kullanır; `cad.sheets.set` güncel ölçek/köşeleri ve `details=true` ile kaynak seçicisinin güncel `namedViews` listesini verir. Ham `book` ilk kaydı korur. UI, gerçek oranı ve merkezleri salt okunur gösterir; güncel görünümün adını seçicide gösterir. Canlı bağda elle kamera değiştiren alanlar/sığdırma kapalıdır. `setViewport` ve ham özellik yaması bu kuralı aşamaz. Kamera kilidi elle değişikliği engeller; açık canlı bağlantının kaynağı izlemesini engellemez.

Bağı `drawing` yaparken UI ve native/Python aynı `prepare_source` ile güncel etkin kamerayı açık isteğe koyar; böylece ayrılınca eski kameraya sıçramaz, inverse yeniden bağlar. Hedef kayıpsa kullanıcı bilinçli olarak bağı ayırıp tutulmuş kamerayla onarabilir. Diğer saf işlem tüketicileri bu hazırlayıcıya kendi kaynak listesini verir; çıktıdan veya undo'dan tekrar canlı kayıt okunması gerekmez.

## Kopuk bağ, callout ve ölçek

Bulunamayan canlı bağ `missing_named_view` hatasıdır. Eski kameranın çizimi sessizce gösterilmez. Ön denetim yeniden bağlama veya bağı ayırma yolunu söyler; başsız PDF hedef/kamera hatasında dosya yazmadan durur. `.kcad` yeniden açma bağın kimliğini ve ilk kaydını korur. DXF/DWG viewport kamerası aktarım anındaki etkin görünümdür; yabancı yazılımda KentOS canlı bağını yaşattığı iddia edilmez.

Mevcut kesit/görünüş/detay `ViewTarget` aynı kimliklerle adlı görünüme veya `{sheet,item}` çiftine bağlanır. Model görünümü/pafta/çerçeve yeniden adlandırılınca balon yazısı ve başvuru listesi izler; silinen model görünümünün işareti son yazısıyla kopuk olarak kalır. Pafta çerçevesinin güncel zemin kutusu setin etkilenen-pafta hesabına girer.

Etkin kamera paydası annotative temsil seçimine ve kâğıtta boylanan mimari işaretlere gider. Yolda bulunan kusur düzeltildi: haritada farklı ölçek için ayrı spatial görünüm yalnız duvar varsa kuruluyordu; duvarsız çizimde kesit/detay balonu ve yazısı genel çizim ölçeğiyle kalabiliyordu. `has_scale_dependent_geometry` duvar, mekân, aks, kolon, merdiven, rampa, etiket ve görünüm işaretini kapsar. Mevcut `walls_differ_at` çağrıları bu ortak sınıflamayı kullanır; kaynakta nesne çoğaltılmaz. Türetilmiş spatial/annotative önbellek aynı model kimliklerini korur.

## Kabul

- Bağımsız Python referansı: beş payda × dört dönüş, büyük model koordinatları, aynı adı taşıyan başka kimlik; 20 senaryo core/tasarımcı/Python ve mevcut WASM yüzeyinde.
- Python/native/MCP: güncel ölçek, değişmeyen capture, sabit kimlikle yeniden adlandırma, kopuk model hedefi, pafta/öğe başvuruları, bilinçli ayırma ve `.kcad` yeniden açma.
- Gerçek PDF/Poppler: 1:50 ve 1:100'de görünüm işareti ile annotative yazının fiziksel boyları arasındaki fark <0,02 mm; aynı metin iki görünümde aranabilir. DXF yeniden açıldığında etkin 50/100 paydaları korunur.
- Gerçek GPU ekranları: canlı 1:50 ve kopya 1:100 aynı modeli farklı boyda gösterir; iki görünümün işaretleri aynı kâğıt boyundadır. Açık/koyu 1440×900, 18 px/1100×650 ve kaydırılmış kaynak/ölçek denetçisi; PDF görüntüsü de açılarak incelenir. Görsel denetimde sığdırma eylemlerinin canlı bağda açık kaldığı görüldü, kapatıldı; model düzenlemeye giriş ayrı tutuldu.
- CAD-SHEET-15 katman ezmesi/annotative seçimi ve 16 görünür nesnenin seçim/yakalama ayrıntısı bu maddeden ayrı kalır.

Web uygulaması ADR 0228 gereği erteli; TS/katalog/SDK ve ortak WASM devam eder. Web dönüşünde güncel `namedViews` girdisi, salt okunur canlı kamera alanları, ayrılma ve kopuk bağ politikası bağlanmalıdır.

Son kabul: 230 Python testi; source wrapper/undo ve kayıp kaynak/ölçek bütçesi ayrıca sınandı. İlgili core 379+62, UI 46+37+7, native application 42+131, plot 12+22 ve MCP 7+2+23+1 testleri; clippy, schema kapalı commands, katalog/SDK, bağımlılık kapısı, WASM derlemesi ve 20 canlı bağ senaryosu geçti. GPU kaynak/ölçek kontrolleri ve Poppler PDF açılıp incelendi.
