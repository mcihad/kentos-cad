# ADR 0714: Açıklığın numarası yerleştirirken

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-OPEN-01'in numara dilimi.
- **Bağlam:** ADR 0341 (açıklık, Numarala), ADR 0344 (etiket ve çizelge; 27 Ekim eki: boş etiket), ADR 0636 (açıklık
  dağıtımının numaraları).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Kapı ve pencere numarasız konuyordu: Hepsini etiketle boş altıgenler yazıyor, Kapı ve pencere çizelgesi numarasız satırlar
veriyordu; numara için ayrıca Açıklıkları numarala çalıştırmak gerekiyordu. Daire yürüyüşünde (ADR 0344'ün 27 Ekim eki)
etiketlerin altısı boştu. Ayrıca duvarın üstünde duran kapı etiketinin yazısını duvarın çizgileri kesiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit — Create Sequential Tags for Doors or Windows](https://help.autodesk.com/cloudhelp/2018/ENU/Revit-DocumentPresent/files/GUID-F9B6BB41-CC51-49C8-A17D-DE0F16340E17.htm):
  kapının Mark değeri her yerleştirmede kendiliğinden artar; bir kapıya değer yazılınca sonrakiler ondan sırayla gider.
- [Revit — Mark (instance property)](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-08C6541F-4974-489E-B941-F365628B2884.htm):
  kullanılan bir değer yazılınca uyarır, ama izin verir.

KentOS kararı: Revit'in Mark akışı, Türkiye'deki K/P alışkanlığıyla. Kapı K1, K2…, pencere P1, P2…, boşluk B1… Numara
komutun girdisindedir (`cad.openings.create`'in `number`'ı, CMD-07): Python ve MCP değişmez, onlar numarayı kendileri verir.

## Karar

### 1. Araçlar: Numara (N)

- **Otomatik** (varsayılan): aynı biçimdeki (kapı, pencere, boşluk) en son numaralanmış açıklığın (en büyük yuvası)
  numarasının sondaki sayısı bir artar, genişliği korunur (K3 → K4, K-07 → K-08, K9 → K10, K-99 → K-100); hiç yoksa
  biçimin ilki (K1, P1, B1). Sonunda sayı olmayan numaraya 1 eklenir.
- **Yazılan numara** sıradaki açıklığınkidir; sonrakiler ondan Otomatik'le gider.
- **Yok** numarasız koyar; boş ya da “otomatik” yazmak Otomatik'e döner. En çok 128 harf.
- Numara aynı biçimdeki bir açıklıkta varsa sonraki boşa geçilir (Revit uyarıp yinelerdi; KentOS yinelemez, yinelenen
  numara denetimi ADR 0344 §2'de kalır).
- Ayarlarda “K4 · otomatik” yazar; yazılınca “Kapı K4 eklendi.” denir. Örnekten al ve iki pervaz yöntemleri de aynı
  numarayı alır.

### 2. Etiketin yazısı zeminli

Etiketin satırları çizimin zemini rengindeki maskenin üstünde çizilir (`geom::tag`; yazının Zemin'i gibi, ADR 0145): duvarın
eksenine düşen kapı etiketinin yazısını duvar çizgileri kesmez. Çerçeve ve kol aynıdır.

## Sınırlar

- Numara tür kodu değildir (Revit'in Type Mark'ı): aynı tipteki kapılar ayrı numara alır; tür kodu çizelgenin Tür sütunudur.
- Odaya göre numara (101-A gibi, Revit'in To Room/From Room'u) yoktur; Açıklıkları numarala (önek, başlangıç) sonradan
  yeniden numaralar.
- Etiketin yeri değişmedi: düşey duvardaki kapının etiketi duvarın üstünde durur, yazısı artık okunur.

## Doğrulama

- `cargo test -p kentos-interaction --test all opening_numbers`: kapı K1, K2, pencere P1; yazılan K-07'den K-08; kullanılan
  K-09 atlanır, K9'dan K10; Yok numarasız, boş Otomatik'e döner, 129 harf reddedilir; numara `cad.openings.create`'in
  girdisinde. Tam etkileşim (763) ve masaüstü (1343) takımları geçti; çekirdeğin etiket testleri ve `kentos-plot` geçti.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=daire-kapi-numara,daire-etiketler cargo test -p kentos-desktop --bin kentos-cad
  tools_screens::screens -- --ignored --nocapture`: Kapı aracında “Numara: K4 · otomatik”, etiketlerde K1–K3 ve P1–P3,
  duvar üstündeki K1 ve P3 okunur.
