# 0674 — Mahal hesabında atanmamış kaynakların açık kapsamı

Tarih: 25 Ekim 2026. **Bu dilim uygulandı ve aşağıdaki sınırlarla doğrulandı.** ARCH-SPACE-01'in kaynak
kapsamı koşulunun ilk dilimidir; seçilmiş kalıcı kaynak listesi ve sanal
ayırıcı tamamlanmış sayılmaz.

## Araştırma ve mevcut kod

- Autodesk [ilişkili mahal üretimi](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-3479D10A-3594-4D0F-9193-9F2851D0AAA7.htm)
  sınır rolünü açık nesne özelliği olarak tutar; görünmez kaynak da rolü varsa
  hesaba katılır. [Sınırları gösterme](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-59FCAE97-D385-4048-9085-522B19889FEB.htm)
  yanlış kaynakları teşhis etmek için ayrıca görünür bir iş akışıdır.
- Revit [oda sınırları](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-ArchDesign/files/GUID-C7338362-6D35-471B-90B0-6A893534FFE2.htm)
  fiziksel sınır ve oda ayırma çizgisini ayrı tanımlar. Bu referanslar KentOS'ta
  rastgele bir çizgiyi kendiliğinden sanal sınıra dönüştürmeyi gerekçelendirmez.
- KentOS `SpaceFields.arch.level=None`, `space_follow::takes_level` ve toplu
  bulmada bütün çizimi kapsar. Bu davranış eski dosya ve API için korunmalı.
  Yalnız ilk sorgudaki bir süzgeç yeterli değildir: aynı seçim mahalda kalmalı,
  follower ve kayıt/aç sonrasında da uygulanmalıdır.

## Karar ve kabul koşulları

- `SpaceScope`: `project` varsayılan/yokluk; mevcut Kat seçiminin anlamı aynen
  korunur. `unassigned` yalnız `arch.level` atanmamış duvar ve kolonları alır.
  Yalnız atanmamış seçeneğinde hedef kat da boş olmalı; iki çelişen istek atomik ret.
- Mahalın `scope` alanı, NewSpace, discover ve edit girdilerinde aynı sözleşme.
  Çizilmiş mahal kaynak aramadığı için yalnız project kullanır; böl/birleştir
  ilişkiyi kaldırırken scope'u da project yapar ve son geometriyi korur.
- Tek/toplu bulma, plan/execute, kolon düşümü, kaynak değişikliği ve yöntem
  değişimi aynı kapsamı kullanır. Kat atama/kaldırma hem eski hem yeni etki
  alanını geçersiz kılar. Kaynak kalmazsa son sınır ve mevcut lostEnclosure
  korunur; kaynak geri dönünce aynı mahal UUID'si iyileşir.
- Mevcut Kat menüsüne **Yalnız atanmamış** eklenir; **Tüm katlar** eski project
  + boş kat isteğidir. Aynı seçenek Öznitelikler'de görünür ve edit komutuna gider.
  Geometri hesabı arayüze kopyalanmaz.
- KCAD 104 yalnız yeni kalıcı kapsam gerektiğinde yazılır; eski dosya aynı
  şema/bayt kalır. İki okuyucu/yazıcı ve schema-altına-saklama retleri birlikte
  sınanır. Katalog/Python/MCP aynı davranışa erişir; web UI ADR 0228 gereği bekler.
- [x] Bağımsız 90 durum: aynı yerde farklı katlar, atanmamış duvar,
  kolon, Net/Eksen/Brüt, beş birim, uzak koordinat, çelişen/drawn girdi.
- [x] Native follower, kalıcılık, atomik ret, tek undo/redo ve grup sonu.
- [x] Python/MCP, ortak fixture, codec ve eski byte korunumu.
- [x] Gerçek GUI: görünür seçenek, tek/toplu önizleme/oluşturma, kaynak katını
  değiştirip geri alma, Öznitelikler, kayıt/aç, ham görüntü ve bağımsız dosya hesabı.
- [x] İlgili regresyon/clippy, ortak WASM, katalog/SDK drift ve manifest.

[Kabul kaydı](../baseline/2026-10-25-architecture-space-scope/README.md):
56 ham gerçek GUI görüntüsü, 29 bağımsız okunmuş kayıt, 292 Python testi;
son binary'de tek sağ menü onayı/undo/redo ve Escape doğrulandı. Kat alanında
**Atanmamış**, menüde **Yalnız atanmamış** yazılır; ilk GUI'de kesilen uzun
değer bu şekilde düzeltildi. Tek/toplu ayarlar açık/koyu × 13/18 px ×
dar/geniş gerçek pencerede kontrol edildi. Dar istemci 1100×650, geniş yaklaşık
1440×825'tir; 1440×900 ve bütün iş akışlarının genel matrisi açık kalır.
Önceki 103 okuyucusu gerçek yeni GUI kaydını açıkça reddetti. Split/merge
scope'u project, columns'u ignore yapar; bağımsız dikdörtgen alanları ve undo
ile özgün ilişkinin dönüşü native testte doğrulandı.

Seçilmiş kaynak UUID listesinin sonraki diliminde kopyala/import/replace
kimlik eşlemesi, silinmiş kaynağın açık durumu ve sonradan eklenen duvarın
listeye kendiliğinden alınmaması ayrıca doğrulanmalıdır. Sanal ayırıcı fiziksel
duvar kalınlığı olmayan ayrı bir sınır rolüdür; bu ilk kapsam seçeneği onu taklit etmez.

## İlk doğrulamada bulunan seed sınırı

Toplu bulunan bütün odanın seed’i sonradan hesaba katılan bölme duvarının
üzerinde kalabiliyor. Eski `BoundaryIndex::found` bu noktayı kasıtlı olarak
duvar malzemesi hücresinde de arar; 1,56 m² duvar gövdesi oda olmamalı.
Yeni `room_at`, aynı analitik malzeme winding bilgisini kullanarak yalnız
boş oda noktasını kabul eder; eski nokta sorgusunun davranışı değişmez.
Atanmamış kapsam follower’ı bu durumda son sınır + lostEnclosure tutar.
Kapsam düzenlemesinde yeni kaynakların oda bulamadığı seed atomik reddedilir;
kullanıcı İç nokta seç ile geçerli nokta seçip sonra kapsamı değiştirir.
Kolon üstündeki seed ise oda kimliği için geçerlidir; fiziksel kolon düşümü
bundan sonra uygulanır. Bu sınıflandırma kolon deliğini oda arama maskesine
dönüştürmez.

## Görünür iç nokta seçimi

Gerçek eski GUI’de “Mahali düzenle” araması komut bulmuyor; Öznitelikler’deki
Duvarlardan yeniden bul eylemi de yalnız mevcut seed’i tekrar kullanıyor.
Bu yüzden yeni güvenli kapsam reddini çözmek için **İç nokta seç** aracı
eklenir. Seçili mahaldan veya mahal içi tıklamadan başlar, yeni oda noktasını
ve sınırını önizler; tıklama veya tek onay aynı `cad.spaces.edit/rebound`
komutunu verir. Metadata/UUID/kapsam korunur; Escape yazmaz. Duvar gövdesi
yerleştirme hedefi değildir; bu seçim sınıflandırması ortak domain sorgusudur,
arayüzde yeni geometri hesabı yoktur. Python/MCP aynı açık seed’i mevcut
Rebound girdisiyle verir; eski genel API’nin duvar içi seed davranışı değişmez.
