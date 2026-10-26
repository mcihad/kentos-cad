# ADR 0688: Seçilmiş yöntemden ayrı net mahal etiketi

- Durum: bu dilim uygulandı ve aşağıdaki kapsamda doğrulandı (26 Ekim 2026).
- Kapsam: ARCH-SPACE-02; mevcut etiket komutlarının `{netArea}` alanı.

## Hesap ve sınır

`{area}` seçilmiş sınır yönteminin alanı olarak aynen kalır. `{netArea}`
mahalın iç yüzler yönteminin, kendi kaynak/kat/kapsam/kolon tercihleriyle
hesaplanan alanıdır. Geçerli elle çizilmiş Net sınır aynı delikli/çok parçalı
alanı verir. Elle çizilmiş brüt/eksen sınırın duvarları bilinmediği için net
alanı boş kalır. Kopuk kaynak/sınır hiçbir yöntemde net sayı uydurmaz.

Seed'li Net/brüt/eksen mahal, mevcut Metraj'ın `SpaceSearch` ve Net yöntemini
kullanır; mevcut mahal sınırını veya yöntemini değiştirmez. Kolon politikası
Net hesabına uygulanır. Alanın birimi ve basamağı mevcut proje ayarıdır;
çizim uzunluk birimi alanı ölçeklemez. Bu alan yasal/kurumsal net alan tanımı
veya tüm BIM yapı elemanlarının düşümü değildir.

Her etiket planı/takip geçişi yalnız immutable belge ödüncü boyunca kaynak
arama ağlarını paylaşır. UI ayrı geometri hesabı yapmaz. Kaynak değişiminde,
brüt mahalın kayıtlı alanı değişmese bile net etiket metni yeniden okunur;
elle etiket yeri/ankraj ve eski şablon davranışı korunur. Ayar/geçmiş/iptal/
dış değişiklik aktarımı da yeni türetilmiş metni güncel kaynaktan yeniler.

## Karşılaştırma

[Graphisoft Zone Tool Settings](https://helpcenter.graphisoft.com/user-guide/89515/)
iç yüzlere kadar ölçülen net alanı, referans çizgisine kadar ölçülen alandan
ve tercihlere bağlı ilave düşümlerden ayırır.
[Autodesk Room Area Boundary](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-0BB62832-36DD-4E06-A9D4-EE98CE0FCF89.htm)
bitiş yüzü, eksen ve çekirdek sınırlarını ayrı seçimler olarak açıklar.
Bu kaynaklar iş akışı karşılaştırmasıdır; KentOS'un daha sınırlı duvar/kolon
modeli bu ürünlerin bütün hesaplarına eşdeğer sayılmaz.

## Kabul

Bağımsız dikdörtgen/delik/çok parça/ters yön/büyük koordinat hesapları;
Net/eksen/brüt, kolon/sanal sınır/kat/kaynak kaybı; validate/plan/tek undo/redo,
Python/MCP/kayıt-aç. **225 ortak durum /30 yeni, 318 Python, 68 domain,
697 interaction, 219 application ve iki gerçek MCP testi** geçti; mevcut
1 interaction/22 application atlaması korunur. Clippy tüm hedefler/derleme/
contracts WASM/1170 bağımsız KCAD/katalog/SDK sapması geçti.

Gerçek GUI şablon/tek Enter, oluşturma ve kaynak değişimi undo/redo,
kolon100cm ve75cm, duvar40cm, sıfır ölçü reddi, kaynak kaybı/geri gelme,
önizleme/Escape ve yeni süreçte kayıt-aç; **21 ham ekran /15 tam belge**
bağımsız okuyucuyla doğrulandı. 05–06 otomasyon odak hatası tanısaldır;
sonraki gerçek alt komut alanıyla kabul edildi. Eski dört nesnenin geometri,
ayar ve UUID'leri ilgili kayıtlarda birebir korunur.

[Kabul kaydı](../baseline/2026-10-26-architecture-tag-net-area/README.md)
ham sonuçları, ilk başarısız denemeleri ve bağımsız hesapları içerir.
200 mahal/etiket,400 kaynak debug komutunda hareket ortancası26,854ms;
20.000 ilgisiz çizgi ve legacy kolon Düşme yolunda348,204ms. Release/FPS
ve genel performans garantisi değildir. Tam mimari görünüm matrisi,
listeler ve Koru/Yenile açık kalır; ARCH-SPACE-02 [~].
