# ADR 0598 — Pafta görünümünde açık model düzenleme kapsamı

Tarih: 22 Ekim 2026. CAD-SHEET-13.

## Karar

Kâğıt ve model verilerinin sahipliği değişmez. Görünümün içine çift tıklamak veya seçili görünümde **Modeli bu görünümde düzenle** eylemi, o görünümü geçici model kapsamı yapar. **Kâğıda dön**, görünüm dışına çift tıklama, başka sekme veya kâğıt öğesi seçimi kapsamı kapatır. Bu durum KCAD verisi değildir.

Ortak `model_scope::Viewport` pafta ve öğe kimliğini birlikte çözer. Yalnız aynı paftanın yerleştirilmiş sabit model haritası kabul edilir; başka paftanın öğesi, atlas, bulunamayan kaynak veya harita olmayan öğe reddedilir. İç boşluk, etiket bandı, çerçeve dönüşü, kamera dönüşü ve çokgen sınırı hesaba katılır. Yeni tıklama yalnız sınır içinde modele geçer; yakalanmış sürüklemenin sınır dışındaki bırakılması kendi model koordinatında tamamlanır.

Masaüstü mevcut model oturumunu, nesne kimliklerini, seçim/yakalama ve çizim araçlarını kullanır. Ayrı düzenlenebilir model veya pafta başına geometri üretilmez. Ekrana hizalı geçici kamera ortak kâğıt dönüşümünden kurulur; model kapsamından çıkınca asıl model kamerası geri gelir. Model klavyesi, komut satırı, sağ tık/Enter davranışı, nesne özellikleri ve yerinde metin editörleri bu kapsama bağlıdır. Kâğıt öğe tutamaçları modelde gösterilmez. Etkin kapsam durum çubuğunda ve özellik panelinde görünür; koordinatlar açıkça model metresidir.

Çizim önizlemesi, seçim çizgileri, seçim penceresi, imleç ve yakalama işareti harita içeriğinden ayrı geçici katmandadır. İçerik önbelleği her fare hareketinde yeniden oluşturulmaz. Ortak çokgen çizgi kırpması içbükey boşlukları böler; araç izleri kâğıdın geri kalanına sızmaz. Bu geçici katman çıktıya dahil değildir. Özel görünümün nihai içerik maskesi ADR 0597'deki ortak yoldadır.

## Kaydırma, yakınlaşma ve otomasyon

`navigateViewport` ortak kâğıt işlemidir. `id`, `delta` (kâğıt µm), pozitif `factor` ve isteğe bağlı mutlak kâğıt `anchor` alır. Önce odakta yakınlaşma, sonra yeni çözünürlükte kaydırma uygulanır. Orta tuş ve tekerlek aynı işlemi kullanır. Kaynak model geometrisi değişmez; pan özgün oran çiftini korur. Yakınlaşma fiziksel payda sınırlarına tabidir. Kamera kilidi gezinmeyi engeller; model nesnelerinin düzenlenmesini engellemez. Kâğıt kamera değişiklikleri kâğıt geçmişindedir, nesne değişiklikleri model geçmişindedir (ADR 0590).

Python/AI için `cad.changeset.apply` isteğe bağlı, tipli `modelScope: {sheet,item}` alır. Bu kapsam doğrulanmadan hiçbir adım çalışmaz. Çocuk model komutlarının girdileri yine kanonik **metredir**, kâğıt pikseli veya mm olarak yorumlanmaz. Kapsam içinde kâğıt, history, görünüm, birim/proje/plot ayarı ve CAD dışı komutlar reddedilir; modeldeki mevcut katman kilidi ve komut doğrulaması korunur. Kapsam bir coğrafi erişim sınırı değildir; görünüm aynı belge modeline açılır. Kapsamsız mevcut model betikleri uyumluluk için sürer. `plan`, `validate`, atomik ret ve tek grup undo mevcut changeset yoludur; ikinci bir yürütücü eklenmez.

## Kabul

- KentOS çağırmayan Python hesaplayıcısı: altı çerçeve dönüşü × dört yakınlaşma; 500 km/4500 km model koordinatları, eğik kamera, kâğıt odak noktası, birlikte pan/zoom ve ileri/ters dönüşüm için 24 senaryo.
- Aynı fixture core, gerçek tasarımcı mesajları, Python ve MCP'de; ters kâğıt işlemi, yanlış pafta/öğe, kilit ve yeniden açma sınanır. WASM mevcut operation yüzeyinden aynı 24 kamerayı üretir.
- Gerçek masaüstü çizgi aracı: görünümden 0–5 m çizgi, nesne sayısı/kimliği, kâğıdın değişmemesi, model undo ve eski kameraya dönüş.
- Açık/koyu ve 18 px/1100×650 GPU ekranları: etkin araç önizlemesi, model koordinatı, Kâğıda dön, gerçek model nesnesinin seçilmesi ve özellikleri. Görseller açılarak incelenir; sorun görülen kâğıt yönergeleri model kapsamına göre düzeltildi.
- Daha ileri katman görünürlüğü/annotative temsil etkileşimi CAD-SHEET-15/16; named view canlı bağı CAD-SHEET-14. Bu madde bunların tamamlandığını iddia etmez.

Web uygulaması ADR 0228 gereği erteli. Ortak ops/katalog/SDK ve WASM derlenir; web yeniden açıldığında kendi olay ve render katmanı aynı kapsamı kullanmalıdır.

Son kabul: 227 Python testi; native application 42+131 test; MCP açık kapsamın 24 senaryosu; ortak sheet 379+61 ve tasarımcı 46+36+7; gerçek masaüstü aracı ve metin girişi, clippy, schema kapalı commands, katalog/SDK ve 24 WASM gezinme durumu geçti. GPU ekranları ve büyük yazılı kaydırılmış özellikler/metin kutusu açılıp incelendi.
