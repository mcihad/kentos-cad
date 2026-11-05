# ADR 1009: Açılışta süre sınırlı, salt okunur sağlık denetimi

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-18` açığı “Açılışta isteğe bağlı salt
  okunur sağlık denetimi (süre/bellek bütçesi, aynı revizyon için önceki sonucun tekrar kullanımı)” (ADR 1005 §5).
- **Bağlam belgesi:** ADR 1005 (sağlık grubu, `cad.drawing.inspect`), `apps/desktop/src/cad_inspection.rs`
  (CAD denetimi sekmesi).

## 1. Kaynak

AutoCAD'in [AUDIT](https://help.autodesk.com/cloudhelp/2016/ENU/AutoCAD-LT/files/GUID-62DDB935-61B1-49DA-8238-3EF1CC45259B.htm)
komutu çizimi kullanıcı istediğinde denetler; “Fix any errors detected?” sorusuna hayır denince yalnız raporlar,
düzeltemediğinde RECOVER önerilir; sonuç `AUDITCTL` açıksa ADT günlüğüne yazılır
([açık çizimi onarma](https://help.autodesk.com/cloudhelp/2016/ENU/AutoCAD-LT/files/GUID-7AFE414B-BCC6-470C-B0E9-8228A252EE7C.htm)). KentOS da sağlık adaylarını kendiliğinden onarmaz (ADR 1005); bu ADR yalnız isteyen kullanıcı için
açılışta denetimi ekler. Varsayılan kapalıdır.

## 2. Karar

- Yeni kullanıcı ayarı `appearance.openHealthCheck` (boolean, varsayılan `false`, yalnız masaüstü; Ayarlar →
  Görünüş → Açılış: “Açılışta sağlık denetimi”).
- Açık olduğunda CAD çizimi açılınca (`Message::Opened`) arayüz iş parçacığı dışında, çizimin okuma kopyasında
  `cad.drawing.inspect` girdisiyle yalnız sağlık grubu (`checks: ["health"]`, görünür çizim, eşik 0,01 m, sınır 200)
  çalışır. Çizim değişmez, geri alma adımı yazılmaz.
- **Süre bütçesi:** `drawing_inspect::inspect_until(doc, input, stopped, deadline)` 2 saniyede durur; o ana kadar
  bulunanı `truncated` ile döndürür (iptal değildir), iş sınırınınki (`WORK_MAX`, 2 milyon adım) gibi
  `inspection_truncated` uyarısıyla. Saat her 1024 adımda bir okunur. Bu sözleşme alanı değildir: Python ve MCP'de
  aynı girdi her zaman aynı cevabı verir. **Bellek bütçesi** ayrı bir sınır olarak eklenmedi: denetimin belleği
  incelenen nesnelerin düz kenarları ve merkezleriyle orantılıdır, bulgular 200'de kesilir.
- **Tekrar kullanım:** cevap (dosya yolu, boyut, değişme zamanı, çizimin revizyonu) anahtarıyla oturum boyunca
  tutulur (en çok 16). Aynı dosya aynı hâliyle yeniden açılınca denetim çalışmaz, önceki cevap gösterilir ve
  söylenir. Dosya diskte değişirse yeniden denetlenir. Oturumlar arası saklama yoktur.
- Cevap CAD denetimi sekmesinin sonucu olur (sekme kendiliğinden açılmaz; sekmenin kendi denetimi sürüyorsa ona
  dokunulmaz) ve Komut geçmişine tek satır yazılır: aday sayısı, incelenen nesne, kısmiyse “Süre ya da iş sınırına
  ulaşıldı: sonuç kısmi; tam denetim için CAD denetimi'nde Denetle'ye basın.”
- Başka bir çizim açılınca süren denetim durdurulur, geç gelen cevabı atılır; çizim arada değiştiyse de atılır.

## 3. Kanıt

`apps/desktop/src/open_health.rs` testleri: T birleşimi kısa kalan çizim açılınca bir aday bulunur, revizyon ve
geri alma değişmez, satır yazılır; aynı dosya yeniden açılınca denetim görevi hiç başlamaz ve önceki sonuç söylenir;
dosya değişince yeniden denetlenir. Ayar kapalıyken (varsayılan) hiçbir şey çalışmaz. Önce açılan çizimin geç
cevabı ikinci çizim açıldıktan sonra atılır. Geçmiş bir süre verilince cevap kısmi, `inspection_truncated`
uyarılı ve satır bunu söyler.

## 4. Sınırlar

- Ayrı bellek sınırı, oturumlar arası önbellek ve açılışta öbür grupların (sınır, tarama, açıklama) denetimi yok.
- Onarım tarifinin kaydı, öbür bulgu türleri için onarım, ayrı kurtarma kopyası ve kısmi okuma CAD-ADV-18'de açık.
- Web dururken (ADR 0228) web'de ayar gösterilmez (ayar yalnız masaüstü konağındadır).
