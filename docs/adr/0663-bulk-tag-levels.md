# ADR 0663: Toplu etikette açık mimari kat kapsamı

- Durum: kaynak kat kapsamı dilimi kabul edildi (26 Ekim 2026).
- Taban: `5d217ca4d`, aynı mimari dal; ARCH-SPACE-02.

## Gerekçe ve kaynaklar

Kaynak CAD katmanı ile mimari kat farklıdır. Mevcut toplu etiket aracı
görünür kaynak katmanlarındaki bütün mimari katları okur. Üst üste duran
katlarda yalnız bir katın etiketini üretmek veya yenilemek seçilemiyordu.

[Revit LT 2026 Tag All Not Tagged](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-DocumentPresent/files/GUID-13EA386B-E613-4126-B634-67BEC622177E.htm)
toplu etiketlemenin görünür görünüş ve seçili kaynak kapsamını açıkça seçtirir;
aynı mahalın plan ve tavan planında ayrı etiketleri olabilir.
[Vectorworks 2024 Data Tag](https://app-help.vectorworks.net/2024/eng/VW2024_Guide/Annotation2/Adding_data_tags_and_labels.htm)
uygun kaynakların katman/viewport ve seçim kapsamını ayırır. Buradan çıkan
KentOS kararı kaynak kapsamını görünür yapmaktır. Bu dilim bir görünüş veya
pafta modeli getirmez; mimari kat süzgeci o işlerin yerine geçmez.

## Karar

- Yeni komut veya kalıcı dosya alanı yoktur. `TagAll.levels` isteğe bağlı
  `Array<string | null>`: alan yok/null ise eski tüm-katlar davranışı;
  `['zemin']` yalnız o kat, `[null]` yalnız kat atanmamış kaynaklar,
  `['zemin', null]` ikisinin birleşimi. Boş dizi hiçbir kaynak seçmez;
  tekrarlı kimlik aynı kaynağı iki kez işlemez. Kimlikler adla eşleştirilmez.
- Her dolu kimlik proje mimari katlarında doğrulanır. Yanlış veya silinmiş
  kimlik `level_not_found` ve `all.levels[i]` ile tüm çağrıyı reddeder;
  sessizce tüm katlara veya atanmamışa dönülmez. Sıra: mevcut açık tags,
  all türleri, kaynak katmanları, mimari katlar, alanlar, kalan eski kontroller.
- Duvar/mahal/kolon/aks/merdiven/rampa kendi mimari katını; açıklık yalnız
  bağlı duvarın katını kullanır. Blokta mimari kat alanı bulunmadığından
  kat atanmamış sayılır. Duvarı bulunmayan veya host'u duvar olmayan
  açıklığın katı bilinmez; açık kat ve `[null]` kapsamına girmez. Alan
  yok/null olduğunda eski kapsamı korunur. Yetim öğeler onarılmadan kat
  uydurulmaz. Z koordinatı veya çizim katmanı kata çevrilmez.
- Kat kapsamı mevcut görünürlük, tür, kaynak katmanı ve evre süzgeçleriyle
  kesişir. Koru/Yenile aynı uygun kaynakları kullanır; diğer katlardaki
  mevcut etiketler değişmez. Açık `tags` girdisi bu süzgeçten bağımsızdır.
- Masaüstünde ayrı `Kaynak katı` seçimi: Tüm katlar, Yalnız atanmamış,
  proje katları. Varsayılan Tüm katlar. `Etiket katmanı` hedefi ayrıdır.
  Önizleme/sayı, Python ve MCP aynı `cad.tags.create` planını kullanır.
  Seçim salt okunur; tek onay/undo, ret sonrası araçta kalma ve Escape
  korunur. Kapsam seçimi kaynakların katını veya görünürlüğünü değiştirmez.
- Mevcut ortak TS tipi, katalog ve Python SDK güncellenir; yeni TS/TSX
  uygulama dosyası, renderer, birim veya ölçek değişikliği yapılmaz.

## Kabul kapıları

- Bağımsız referans: eski durumların aynılığı, yok/null/boş/atanmamış/tek
  ve çoklu kat, tekrarlı/yanlış kimlik, host mirası, diğer süzgeçlerle
  kesişim, Koru/Yenile, açık tags, atomik ret ve tek undo/redo.
- Gerçek Python/MCP, tam kayıt modelleri ve aç/kaydet; SDK/katalog/WASM.
- Native GUI: varsayılan/kat/atanmamış seçim, önizleme, tek Enter,
  yenileme, ret, Escape, undo/redo ve kayıt yeniden açma; ham görüntüler
  ve bağımsız KCAD okuyucusuyla tam model karşılaştırması.
- İlgili test takımları, clippy/desktop build; önceki KCAD dosyaları bayt
  düzeyinde aynı. Yalnız doğrulanan alt madde işaretlenir.

## Doğrulanan sonuç ve sınır

[Kabul kaydı](../baseline/2026-10-26-architecture-tag-levels/README.md):
26 yeni bağımsız durum, ilgili beş dosyada 290 durum; önceki 138 create
durumu aynı. 809 commands, 52+221 application, 63+704 interaction, 75 MCP,
340 Python ve bir gerçek desktop widget testi geçti. Clippy, desktop
derleme, ortak WASM, SDK ve bağımlılık yönü kontrolleri geçti; atlananlar
ve ilk başarısız koşumlar kabul kaydında ayrı gösterilir.

43 ham native ekran (tanısal/kesilen ilk görüntüler dahil) ve 27 tam kayıt
bağımsız okuyucuyla doğrulandı. İki dolu kat ve atanmamış mahalde Koru/Yenile,
tek Enter/sağ tuş Onayla, ret, Escape, tek undo/redo ve yeni süreçte kayıt
açma denendi. GUI'nin yakaladığı uzun seçili değer yalnız mevcut bileşenin
“Atanmamış” gösterimine bırakılarak düzeltildi; ilk ve son ikililer ayrı
donduruldu. Son ikilide ayrıca olumlu üst kat yenileme/undo/redo ve açık13/
koyu18 geniş/dar görünüm doğrulandı. Önceki 1781 takipli KCAD baytı aynı.

Yalnız çağrı süresinde bir kat kimliği kümesi oluşturulur; kalıcı arama
indeksi veya cache eklenmedi. 268 komut kimliği aynı, yalnız mevcut
`cad.tags.create` tanımı genişledi. Bir mevcut TS tipi yenilendi, yeni
TS/TSX uygulama dosyası yok. Çoklu görünüş/pafta bağlamı, tam mimari GUI
matrisi ve genel performans kabulü bu dilimin kapsamında değildir.
