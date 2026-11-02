# 0836 — Kısıt etiketinin sahne içinde ve doğru birimde kalması

2026-11-02. Durum: uygulandı; otomatik ve gerçek Mac arayüz kabulü geçti. CAD-ADV-05 açık.

## Bulgu ve araştırma

ADR 0835'in gerçek fare kabulünde kiriş etiketi sağdaki katman/öznitelik
panelinin altında kesiliyordu. `constraint_view::boxes`, her noktaya
sağa doğru sabit yerleşim yapıyor, görüntü alanına sığdırmıyordu. Aynı
noktadaki işaretleri ayrı ayrı kenara sıkıştırmak da birbirine bindirirdi.
`constraint_marks::value_words` ise üç ondalığa yuvarlıyor, uzunluk birimini
yazmıyor, grad projesindeki açının sonuna derece işareti koyuyordu.

[AutoCAD kısıtları görsel doğrulama](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-56EA46E3-5425-4368-99A1-AD159B28DE4D.htm)
işaret ile ilgili nesnelerin vurgusunu birlikte tanımlar; işaret gizleme ve
konumunu düzenleme de okuma yükünü azaltır.
[Geometrik kısıt gösterimi ayarları](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-AA5BEC92-FEC7-4FD7-9C3D-0F4A91F80B2B.htm)
tür ve seçimle geçici gösterime örnektir. KentOS'un bu dar işi panel
kenarında okunabilirliktir; bu kaynaklardaki tüm filtre/taşıma seçenekleri
uygulanmış sayılmaz.

## Karar ve kod kanıtı

- `constraint_view.rs`: aynı dört piksel hücresindeki ortak nokta grubu
  birlikte ölçülür. Grup sahnenin dört kenarından içeride kalır; satır
  genişliği yetmiyorsa alt satıra geçer. Panel alanı yerleşim boyutuna
  dahil edilmez. Çizilen kutu ile hover hedefi aynı dikdörtgendir.
- Etiket genişliği mevcut UI yazı tipiyle ölçülür; mevcut tipografi
  önbelleği tekrar dizimi önler. Aşırı uzun ad dar sahnede üç noktayla
  kısalır; kaynak adı/değeri değiştirilmez. Hover açıklaması de sahne
  sınırında tutulur. Bu, farklı noktalardan gelen bütün işaretler için
  genel çakışmasız yerleşim algoritması değildir. Görüntü yüksekliğinden
  çok ortak satır varsa kalan bağlar Parametreler penceresinde korunur.
- `interaction/constraint_marks.rs`: parametre penceresinin mevcut
  12 anlamlı basamaklı gösterimi ortak `parameter_value_words` yoluna
  taşındı. Parametre zaten kendi proje biriminde değerlendirildiğinden
  ikinci dönüşüm yapılmaz. Uzunlukta `settings.unit().mark()`, açıda
  `°` veya `gon`, sayıda sayı açıklaması kullanılır. Çok küçük sıfır
  olmayan değer bilimsel yazılır; saklanan değer/tolerans değişmez.
- Aynı incelemede işaret önbelleğinin yalnız revision/generation tuttuğu
  bulundu. İki yeni açılan çizim bu sayıları paylaşabilir. Anahtara çizim
  oturumu eklendi; başka çizimin işareti taşınmaz. Geometri/komut/Python/
  MCP veya dosya şeması değişmedi. Genel göster/gizle ve hover-vurgu korunur.

## Kabul

- Dört görüntü kenarında ortak noktadaki üç işaretin tamamen içeride ve
  birbirinden ayrı olması, dar alanda satır geçişi/uzun adın kaynakta
  korunması ve hover açıklamasının sığması test edildi.
- Aynı revision/generation ile ikinci çizimi açan uygulama testi, eski
  işaret nesnesinin kullanılmadığını ve yeni adı gösterdiğini doğruladı.
- mm, gon, derece, küçük sıfır olmayan sayı ve negatif sıfır gösterimi
  gerçek query'den üretilen işaretlerle test edildi.
- Masaüstü 1582 ve etkileşim 947 test geçti. Mevcut 256 masaüstü ve
  2 etkileşim ignored ile 5 performans testi çalıştırılmış sayılmaz.
  İki pakette all-targets clippy `-D warnings` geçti.

- Gerçek Mac uygulamasında ADR 0835'in panel kenarında kesilen aynı
  kiriş etiketi artık tam `d1 = 4.472135955 m` okunuyor; hover kutusu ve
  iki bağlı uç vurgusu görünür. Aynı oturumda dosya seçiciden ikinci
  çizim açıldı: eski elips/kısıtlar taşınmadan `boy = 4000 mm` ve
  `aci = 100 gon` gösterildi. Parametreler penceresinde aynı değerler
  ve iki yeşil bağ doğrulandı. Bu salt okunur işlemlerde çizim kirlenmedi.
- Son masaüstü derlemesi geçti. Kanıtlar repo dışında
  `/tmp/kentos-cad-adv04-2026-11-01/` altında:
  `105-constraint-label-inside-panel-edge.png`,
  `106-constraint-label-hover-inside.png`, `107-scene-mm-and-gon.png`,
  `108-parameter-window-mm-and-gon.png`, `constraint-label-*.log`.
  Ekran görüntüleri repoya eklenmedi.
