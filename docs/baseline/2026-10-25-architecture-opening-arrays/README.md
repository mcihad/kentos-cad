# Açıklık dağıtımı — işlevsel kabul dilimi

25 Ekim 2026 · ARCH-OPEN-02 / ADR 0636 · Dal `codex/architecture-2d-workflows`.
Taban `f7ee4c73a`. Ana madde [~]: §10'un tam gerçek boyut/font/tema ve
birleşik zoom/pan/özellik düzenleme matrisi henüz tamamlanmadı.
Açık temada 13 px ile mevcut 1440×828 iç alanda birleşik düzenleme ve gezinme
denendi; bu, kalan boyut/font/tema varyantlarının yerine sayılmaz.

## Ne eklendi

Mevcut `cad.openings.edit` v1'in `array` işlemi; ayrı `tool.openingArray`.
Kaynak kapı/pencere/boşluk yerinde kalır, **Kopya adedi yalnız yeni kopyalardır**.
Net pervaz arası/merkez aralığı, yön veya iki uç payı arasında eşit dağıtım;
isteğe bağlı yeni önek/başlangıç/adım. Aynı biçimde yinelenen numara veya
geometrik çakışma hiçbir kopya yazmaz. Önizleme ve uygulama aynı native
komuttandır. Eski Copy/Renumber ve diğer edit işlemleri değişmez.

Birim ve çizim/pafta ölçeği, mevcut FP64 geometri, import, kamera, kenet ve
render hattı değiştirilmedi. Yeni KCAD nesne/dosya sürümü yoktur. Test çıktıları
`.run/architecture-array/contracts-ts` altına yönlendirildi; yalnız üç yeni
sözleşme tipi ve mevcut OpeningsOperation farkı kaynak olarak alındı.

## Gerçek pencere denemeleri

Kullanıcı çizimleri yerine ayrılmış Ribbon QA uygulaması, ayar/veri dizini ve
iki ölçülü duvar + iki kaynak açıklık + canlı çizelgeden oluşan test çizimi.
`interactive-before.kcad` 5 kaynak nesnedir. Başlangıç m / 1:100.
Görüntüler CUA ile gerçek fare/klavye işleminden alındı ve gözle incelendi;
CUA'nın ham JPEG baytları `.jpg` olarak korundu, maket/offscreen değildir.
İlk pencere 2880×1720 fiziksel (2×; 1440×828 iç alan), açık tema, 13 px.
Bu boyut **1440×900 kabulü diye sayılmadı**.

| Görüntü | Gözlenen sonuç |
| --- | --- |
| 00 | Önceki derlemedeki ayrı mimari regresyon çizimi; özellik #10 genişliği 1 m. Yeni test çiziminin başlangıcı değildir. |
| 01 | Yeni derleme, Mimari şeridinde görünür Açıklık dağıt, 5 kaynak nesne. |
| 02 | Seçili #3 kapı otomatik kaynak, 3 yazmasız kopya; adet/net aralık/yön/numara ayarları. Uzun kaynak özeti kısalıyor; son sunum düzeltmesinin nedeni. |
| 03–04 | Numaralama açılınca K1 tekrarı kırmızı önizleme; sağ tuş Onayla da reddediyor, katmanda hâlâ 5 nesne. |
| 05–06 | İlk numara 2, sonra adım 2 girildi. 05 adım değişmeden önceki ara durum; 06 son geçerli ayarlar ve tek Onayla menüsü. |
| 07 | Bir kez Onayla: 3 yeni kapı, toplam 8; canlı çizelge K2/K4/K6 satırlarını ekledi. Kaydet ve Home. |
| 08–09 | Tek Undo 5 nesne ve eski çizelge; tek Redo 8 nesne ve yeni satırlar. |
| 10 | Aynı kaynakla yeni deneme Escape ile iptal, mevcut 8 nesne korunur. |
| 11 | Uygulama kapat/aç; 8 nesne, kapılar, çizelge ve 1:100 korunur. |
| 12–13 | Yaylı duvarın #4 penceresi: 2 m baş / 1 m son payı, üç adet ve tek adet (ortalı) önizleme. |
| 14 | Uygula ile 3 yaylı pencere; toplam 11. Yeni pencereler numarasız, çizelgeye 3 satır gelir. |

02'nin hücre özeti kısa kaynak kimliği/türüyle değiştirildi. Ara derlemede türün
wire adı görüldü; son kaynakta ortak Türkçe `OpeningForm.label()` kullanılır ve
etkileşim testi `#… · Kapı` değerini doğrular. Son derleme `4230233c24a9d58f76e9ab0104596065eff075f070d3fd2e48e5461bbbcfadf7`
SHA-256 ile ayrıca denendi. 18'de kısa Türkçe kaynak satırı tam okunur.
19'da sıfır adet reddedilip 3 geri gösterilir; 20'de 1 kopya/10 m net aralık
geçerli önizleme verir. 21–23 tek uygulama/undo/redo, 24–25 kapat/aç ve
1 m genişlik/12 m konum; 26–27 aynı kopyanın 1.2 m genişlik düzenlemesi ve
undo ile 1 m'ye dönüşünü doğrular. 28–31 zoom ileri/geri, gerçek Kaydır ve
Home dönüşüdür; kayıttaki nesne/kimlik/ayarlar yine korunur. Bu tek varyantta
kare süresi/titreme ölçümü yapılmadı. 12–13'ün ara derlemedeki İngilizce özeti
son sunum kabulü sayılmaz.

## Bağımsız dosya ve hesap

`tools/kcad/kcad.py` native KentOS kodunu çağırmadan dosyaları okudu:

- `straight-three.kcad`: önceki iki duvar ve iki kaynak açıklık bütün alanlarıyla
  aynı, önceki UUID dizisi aynı, proje ayarları aynı. Yalnız 3 kopya ve çizelge
  satırları eklenir. Merkezler tam 3/5/7 m, enler 1 m; numaralar K2/K4/K6.
- `straight-and-arc.kcad`: önceki bütün kaynaklar ve düz kopyalar aynı; 10 m
  yarıçaplı yarım dairenin uzunluğu 10π. Üç yeni merkez 2.5,
  (1+10π)/2, 10π−1.5 m. Pervaz koordinatları `π+s/10` ve 9.9/10.1 m
  yarıçaplarıyla ayrı trigonometri hesabına 1e−12 m sınırında uyar.
- `independent-*-check.json` sonuçlar; `fixture-prefix-check.json` eski 45 ortak
  edit vakasının JSON olarak birebir korunduğunu ve 40 vaka eklendiğini tutar.

## Ondalıklı tam sığma takibi

Yeni regresyon testi eski dizi ön kontrolünde başarısız oldu: 20−17.8−2 m
aralığında iki 0.1 m açıklık, ikili çıkarma yuvarlamasıyla reddediliyordu.
Ayrı Decimal hesap tam 0.2 m yer ve 0.2 m ihtiyaç olduğunu doğruladı
(`decimal-fit-before.json`). Yeni dizi ön kontrolü ortak mevcut 1e−9 m pervaz
toleransını kullanır; negatif aralığı kabul etmez. Regresyon sonra geçti;
1 µm gerçek eksiklik yine ret verir. Ortak başvuruya iki ondalıklı durum daha
eklendi, toplam 85 edit durumu; eski 45 değişmedi. Genel geometri veya kenet
toleransı değiştirilmedi.

## Çalıştırılmış kontroller

- Ortak sözleşmeler: **799 geçti**; katalog örnekleri ve her sayının birimi dahil.
- Native application: **51 birim + 166 entegrasyon geçti**, 22 entegrasyon atlandı.
  Son ek katman/yetki testiyle hedef dizi kümesi **6 geçti**. Düz/yaylı, üç ankraj,
  iki yön, fit 1/2/3/5 adet, sonluluk, numara/konum retleri ve tek undo/redo.
- Interaction: **63 birim + 656 entegrasyon geçti**; 1 entegrasyon ve 5 resim/ölçüm
  testi atlandı. Son kaynakla hedef dizi kümesi **4 geçti**; görünür ayarlar,
  yazmasız önizleme/iptal, tek onay, yanlış değer retleri ve revision koruması.
- Desktop: **1280 geçti, 221 atlandı**; font testleri seri, ayrılmış XDG dizinleri.
- Python: son kaynakla tam küme **275 geçti**, bunun **4 testi** yeni dizi
  kabulüdür: 20 birim/orijin/düz-yay bileşimi, KCAD tekrar açma, canlı çizelge,
  kaynak/kimlik, idempotency ve ondalıklı tam sığma/gerçek eksiklik.
- MCP: gerçek headless hostta keşif şeması, planın yazmaması, semantic array,
  tek undo/redo ve numara çakışmasında atomik ret: **1 geçti**.
- Son kaynakta application/interaction/desktop/MCP/Python `--all-targets`
  clippy geçti; ortak geometri WASM derlemesi geçti.
- Şerit/SDK/işlem manifesti drift, 79 create + 85 edit bağımsız fixture ve
  bağımlılık yönü (47 crate, 61 hedef) tekrar kontrol edildi.

Tam native/desktop kümeleri son ondalıklı sığma sınır düzeltmesinden önce
çalıştırıldı; bunlar son küçük farkın tam küme kabulü diye sunulmaz. Son
kaynakta 6 application + 4 interaction hedef testi, 85 edit fixture, 275
Python testi, MCP testi, clippy ve WASM çalıştı. Ham günlükler `logs/`,
sonuç ve SHA-256 kayıtları `test-results.json` içindedir. `verify.py` kaydedilmiş
gerçek GUI dosyalarını bağımsız okuyarak tekrar sınar.

Atlanan testler geçti sayılmaz. Tam ekranın 1512×949 iç boyutu ve pencereyi
sürükleyerek boyutlandırma denemeleri hedef matris yerine kullanılamaz.
Ana planın açık kabul kapıları korunur.
