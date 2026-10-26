# Tab sonrası komut satırında Cmd-S — ADR 0654

26 Ekim 2026. **CommandLine/EditCell dilimi doğrulandı.** Genel mimari ve
bütün giriş bileşenlerinin klavye matrisi açık kalır.

Mahal numarala görünür İlk numara alanından Tab ile çıkıp Cmd-S basılması
komut satırına `s` ekliyor. Sahne odaktayken Cmd-S çalışıyor. `01` ham Mac
JPEG'i başarısız durumu gösterir; aynı test penceresi `before-binary.json`
ile hashlenmiştir. Kullanıcı verisi yerine önceki numara kabulünün dört
mahallik test dosyası kullanılmıştır.

Yüklü Iced 0.14.2 text_input, A/C/V/X denetiminden sonra yazdırılabilir
`text` değerini değiştiriciden bağımsız yazar. Klavye olayı kısayol olsa bile
metin taşıyabilir. Ürün yönlendiricisi yalnız yakalanmamış olayı aldığı için
kayıt yoluna erişemez. Düzeltme CommandLine ve EditCell iç metin kutularından önce
komut değiştiricili karakterleri host'a bırakır; platform A/C/X/V ve AltGr
sembolleri kutuda kalır. Kaydet, kayıt biçimi, geometri, Python/MCP/katalog
ve araç onayı değiştirilmez. Diğer kutuların genel kısayol matrisi ayrı iştir.

`command_line_keys.rs`, gerçek odak işlemi, bileşen ağacı, KeyPressed ve
ModifiersChanged olaylarını kullanır. Hashli `red-widget.log`: eski kodda
üç testten ikisi geçer, kaydetme testi `Captured` ve `Input("rooms")` ile
başarısızdır. Beklenti uygulamadan türetilmedi: kaydetme harf eklememeli ve
host'a ulaşmalıdır. Diğer iki test düz/Shift/Türkçe/Option/AltGr metni ile
platform A/C/X/V seçim/pano davranışını denetler.

Desktop ek testi, odaklı satırdan bırakılan Ctrl/Cmd-S'yi gerçek host
subscription → yönlendirici → asenkron dosya Kaydet göreviyle tamamlar;
komut metni, yarım çizgi/nokta sayısı ve model değişmemeli; gerçek KCAD
yeniden okununca tam model aynı olmalıdır. Bu tek başına widget'ın olayı
bıraktığını kanıtlamaz; bileşen testi ve gerçek GUI kabulüyle birlikte okunur.

Test komutları, gerçek çıkış kodu, zaman ve günlük SHA256 `test-runs.json`
içindedir. Ham görüntüler boyanmadan/crop yapılmadan saklanır.

## Son otomatik kontrol kapsamı

Son `ui-final3` komut satırı kümesi **17**, `cells3` mevcut özellik hücresi
**5**, `saving3` desktop Kaydet **6**, `keys3` tuş/şerit tuşları **10**:
toplam **38 başarılı test**. İlk kırmızı ve ara yeşil tekrarlar 38'e eklenmez.
Altı yeni regresyon, gerçek odaklı CommandLine/EditCell olayını, pano ve
tamponun korunmasını; desktop testi gerçek disk yazımını denetler.
Değiştirici bildirimi gelmeden yeni odak verilen satırın host'a Focus(true)
iletmesi de kapsanır. Son `clippy3` (`-D warnings`) ve `build3` geçti.
Mevcut ts-rs/future-compat bildirimleri bu işin sıfırlandığı iddiası değildir.

Düzeltme bir UI olay yönlendirmesidir; ürün komutları/Python/MCP değişmediği
ve önceki numaralandırma diliminde bu yüzeyler geçtiği için Python tam kümesi
bu takipte yeniden çalıştırılmadı. Web/envanter ADR0228 kapsamında duruyor.
Tam ürün ve bütün kutular için evrensel klavye kabulü sayılmaz.

## İlk GUI paketi: yakalanan ikinci giriş yolu

`after-binary.json` ve 02–06 ilk komut satırı düzeltmesidir. 03'te U-099 tek
Enter ile yalnız bellekte değişir; `before-cmd-save.kcad` hâlâ başlangıçtaki
V-099 modelidir. 04'te P-101 görünür ayarından Tab→Cmd-S, U-099'u gerçekten
yazar; `cmd-save-preview.kcad` bağımsız kontrolde beklenen modeldir. Harf
üretilmez ve P-101 uygulanmaz.

05/06 **başarısız/tanısal**: dar pencerede Tab sonrasında özellik hücresi
hâlâ düzenlemededir; Cmd-S P-102 tamponuna `s` ekler. Enter geçersiz değeri
reddeder ve eski P-101'i geri gösterir; araç bitmez. İlk otomatik testler
komut satırı odağını sınamıştı, EditCell içindeki yolu kapsamıyordu.
`red-cell.log`, eski EditCell ile bu açığı gerçek widget olayında da gösterir.
İkinci paket ortak kuralı EditCell'e de uygular; hiçbir tanısal görüntü
başarı diye sayılmaz. Son paket ve test sonuçları aşağıdadır.

`ui-final2`'de yeni pano denemesinin düzeneği kendi kopyalama işlemiyle panoyu
`room` yapmışken `copied` beklediği için başarısız oldu. Test düzeneği dış
pano içeriğini açıkça `copied` olarak değiştirir; ardından yapıştırma,
Cmd-S ve Enter zinciri gerçek hücre tamponunun korunmasını denetler.
Başarısız günlük tutulur, kabul sayılmaz.

## Son paket ve gerçek dosya kabulü

`after2-binary.json` son iki bileşenli düzeltmenin kaynak ve çalıştırılabilir
SHA256 kaydıdır. 07–15 bu değişmez paketle çekildi; eski paketler 01–06'nın
tanısal/ara kanıtı olarak ayrı tutuldu. Ürün kaynakları son derleme ve
paketlemeden sonra değiştirilmedi. Yeni TS/TSX kaynağı yoktur.

- 08: U-099 tek araç onayıyla bellekte, disk hâlâ V-099. 09: doğrudan
  düzenlenen P-101 hücresinden Cmd-S, U-099'u gerçekten yazar; `s` eklemez,
  onaylanmamış alan tamponunu veya çalışan aracı uygulamaz.
- 10: dar pencerede doğrudan hücre Cmd-S ve Tab→Cmd-S. Tab ayarı uyguladığı
  için P-102 önizlemede görünür, model U-099 kalır. Bir sonraki tek araç
  Enter'ı P-102'yi uygular/kapatır; 11'de Cmd-S dosyayı yazar. 12'de tek
  Cmd-Z ve Cmd-S eski U-099'a döner. `final-compact-preview.kcad`, araç Enter'ı
  sonrasında ama Kaydet öncesindeki mevcut disk kopyasıdır; yeni bellek
  modelinin diske yazıldığı kanıtı olarak sunulmaz.
- 13: koyu/13 px, doğrudan odaklı Q-099 tamponundan Cmd-S metni korur;
  eski Z-001 araç ayarı ve U-099 model kalır. 14: alan Enter'ıyla yalnız
  Q-099 ayarı uygulanır; komut satırına açıkça odaklanıp Cmd-S önizlemeyi
  korur. 15: tek Escape vazgeçer, Cmd-S U-099'u korur.

Kaydet onaylanmamış bir özellik tamponunu kendiliğinden modele işlemez.
Mevcut alan Enter/odak kaybı ve ayrı araç onayı sürer; bu iki onay farklı
eylemlerdir. Bu kayıt Tab'ın bütün alanlarda aynı odağa ilerlediği iddiası değildir.

**15 ham JPEG, dokuz disk anlık görüntüsü** (tanısal mevcut-disk kopyaları
dahil). `gui-events.json`, `models.json` ve `screenshot-dimensions.json`
her birini ayırır. Gerçek gözlenen pencere boyları 1440×870, 1100×682 ve
1512×872 pt; açık/15 px ve koyu/13 px denendi. İstenen 1440×900/1100×650
tam matrisinin yerine sayılmaz. 13 px ayarı izole test profilinde kaydedildi;
kullanıcının normal ayarları değiştirilmedi.

`verify.py`, KentOS hesap/komut kodunu çağırmadan bağımsız KCAD okuyucusu ve
Decimal alan hesabıyla dokuz modelde 13 nesnenin UUID/geometri/belge ayarını,
22.04 m² alanı, etiket yer/ankraj/şablonunu, kaynak bağlarını ve tabloyu
karşılaştırdı. Yalnız beklenen numara ve bağlı yazılar/ilk sütun genişliği
değişebilir. Ham resim, paket ve son kaynak SHA'ları da geçti.
`independent-final.log` gerçek sonucu içerir; bütün günlük SHA'ları denetlendi.

Yazı boyu kayıt düzeltmesi: ilk nottaki açık/18 px yanlıştı. İlk paket
profilindeki `appearance.textSize` 15; ikinci paket aynı profili kopyaladı.
`light-profile.json` saklanan kaynak profilidir. Açık denemeler 15 px olarak
düzeltildi; ham JPEG baytları değişmedi. 18 px kabulü iddia edilmez.
