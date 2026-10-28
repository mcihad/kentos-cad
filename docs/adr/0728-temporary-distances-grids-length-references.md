# ADR 0728: Geçici ölçünün kalanı — aksa mesafe, duvar boyu, referansın seçimi

- **Durum:** kabul edildi (28 Ekim 2026); ARCH-EDIT-01'in kalanı, bütün madde.
- **Bağlam:** `plan/architecture.md` ARCH-EDIT-01; ADR 0701 (açıklığın geçici ölçüsü), ADR 0710 (duvarın geçici ölçüsü ve
  Kaydır), ADR 0712 (kolonun geçici ölçüsü), ADR 0343, 0707 ve 0725 (aks), ADR 0341 §3 (açıklık duvarıyla gider).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Açıklık, duvar ve kolonun geçici ölçüsü (ADR 0701, 0710, 0712) yalnız en yakın yüze ölçüyordu. Planın kabulü ve o
ADR'lerin açık bıraktıkları: aksa mesafe (aks yüz değildir), duvarın boyu, referansın elle seçilmesi ve tutulması (iki
aday), kilitli referans, kaydırılmış UCS, büyük koordinat, Esc ve tek geri alma; aynı anlamsal komut Python ve MCP'de.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — About Temporary Dimensions](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm):
  geçici ölçü en yakın dik elemana bağlanır; referansı tanık çizgisini taşıyarak değişir.
  [Move the Witness Line for Temporary Dimensions](https://help.autodesk.com/view/RVT/2025/ENU/?guid=GUID-DE82A2B9-0129-4D03-BC47-17A52BD4E168):
  tanık çizgisinin denetimi başka bir referansa sürüklenir ya da sağ tıkla Move Witness Line; değişiklik oturum boyunca
  durur (“saved on a per-session basis”).
- [Revit — Specify Temporary Dimension Settings](https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Customize/files/GUID-0C635A25-161D-4756-A3D1-C564079A1965.htm):
  duvarlarda eksen, yüz, çekirdek ekseni ya da çekirdek yüzü; kapı ve pencerede eksen ya da açıklık.
- [Home Designer — Setting Exact Wall Dimension Lengths](https://www.homedesignersoftware.com/support/article/KB-00245/setting-exact-wall-dimension-lengths.html):
  boy yazılırken hangi ucun kayacağı düğmelerle seçilir (bir uç, iki uç, öbür uç).
  [Chief Architect — Moving Walls Using Dimensions](https://www.chiefarchitect.com/support/article/KB-01116/moving-walls-using-dimensions.html):
  seçili duvarda iki geçici ölçü: boyu ve paralel duvara uzaklığı; sayıya basılır, yazılır, Enter.
- Archicad'in düzenlenebilir geçici ölçüsü yok (Tracker ve pet palette); KentOS'un akışı Revit'inkidir.

KentOS kararı: yüz referansı ADR 0701/0710/0712'deki gibi kalır; aks ikinci aday olur ve nesnenin **ekseninden** ölçülür
(Revit'in duvar ekseni ve kapı/pencere ekseni ayarı). Bir yanda iki aday varsa referansın ucundaki kareye basmak öbürüne
geçer; seçim oturum boyunca durur (Revit'teki gibi), çizime yazılmaz. Duvarın boyu Home Designer'daki gibi açık bir
sabit uçla değişir; köşedeki duvar Kaydır'ın kuralıyla izler.

## Karar

### 1. Aks referansı

- Aks: düz `grid` (iki nokta, büküm yok); eğrisel aks referans değildir. Nesnenin katında ya da katsız (katsız aks her
  katta); doğrusu sonsuz sayılır (çizilen boyu ve kesik aksın boşluğu önemsiz).
- **Duvar:** kenara 0,001° içinde paralel aks, kenarın **ekseninden** (iki yüzünün ortası) aksa dik mesafe; aks eksenin
  solundaysa sol, sağındaysa sağ yanda; eksenin 1e-9 m içindeki aks solda sayılır. Ölçünün yeri kenarın ortası.
- **Kolon:** bir yana paralel aks, kutusunun **ortasından** o yanın dışa normali yönünde; ortadan geçen aks iki paralel
  yanın küçük numaralısında (0 ya da 1). Ölçünün yeri yanın ortası hizası.
- **Açıklık:** duvarın ölçü yüzünü (ADR 0701 §1) yüz parçasının içinde kesen aks, açıklığın **ortasından** (iki pervazın
  ortası) yüz boyunca; ortadan önce ya da sonra, tam ortadaki önce.
- Her yanda en yakın aks; eşitlikte çizimdeki sıra. Yazılan değer nesneyi o yana dik (açıklığı duvarı boyunca) kaydırır,
  aks yerinde kalır; 0 ekseni aksın üstüne koyar.

### 2. Referansın seçimi ve tutulması

- Her yanın ölçüsü öntanımlı olarak yüzedir (ADR 0701, 0710, 0712'nin en yakın referansı); yüzü olmayan yanda aks.
  Bir yanda iki aday (yüz ve aks) varsa ölçünün referans ucunda küçük bir kare çizilir; kareye basmak o yanın ölçüsünü
  öbür adaya çevirir.
- Seçim oturumundur: nesnenin kalıcı kimliği ve yanı (duvarda kenarı da) için oturum boyunca durur, nesne yeniden
  seçilince ve sayının yazı alanı açılıp kapanınca da; çizime ve geri alma geçmişine yazılmaz.
- Komutlar iki adayı da verir; düzenleme `to` ile seçer: `face` (öntanımlı, eski girdinin anlamı) ya da `grid`.

### 3. Duvar boyu

- Seçili duvarın imlece yakın düz kenarının en az bir ucu kayabiliyorsa kenar boyunca boy ölçüsü: kenarın taban
  çizgisi uzunluğu. Kayabilen uç yolun ucudur (açık duvarın ilk ya da son kenarı) ve serbesttir, uç uca bir duvarla ya da
  köşededir; başka duvarın gövdesine saplı (T) ya da birden çok duvarın birleştiği uç kaymaz (her zaman reddedilecek bir
  boy gösterilmez). Boy ölçüsü kenarın ölçüsüz yanında; iki yanda da ölçü varsa uzun olanın yanında (kısa ölçünün sayısı
  onunla çakışmaz). Sabit uç ölçü çizgisinin ucunda dolu kare (duvarın kendi uç tutamaçlarından ayrı); iki ucu da
  kayabiliyorsa öbür ucun karesine basmak sabit ucu ona geçirir (oturumun seçimi, §2 gibi).
- `cad.walls.edit`'in yeni işlemi `length { edge, length, anchor }`: `anchor` `start` (öntanımlı: kenarın başı sabit, sonu
  kayar), `end` ya da `middle` (iki uç farkın yarısı kadar). Kayan uç kenarın doğrultusunda gider:
  - ucunda birleşim yoksa (ya da birleşimi `none`) yalnız uç;
  - köşe: orada biten başka bir duvar (aynı doğrultuda değil) ADR 0710'un Kaydır'ıyla kayar, köşe kenarın doğrultusunda
    gider; o duvarın öbür bağları Kaydır'daki gibi izler;
  - aynı doğrultuda uç uca duvar: ortak nokta kayar, öbür duvar o kadar kısalır ya da uzar;
  - başka bir duvarın gövdesine saplı uç (T), birden çok duvarın birleştiği uç ve yolun içindeki köşe kaymaz: ret,
    nedeniyle ve çözümüyle.
- Açıklıklar yerinde kalır (ADR 0341 §3); kilitli katmandaki duvar ve izleyen kilitli duvar reddedilir. Boy sonlu ve
  sıfırdan büyük olmalı; aynı boy `nothing_to_set`.
- `cad.walls.distances` kenarın boyunu, iki ucunu ve hangilerinin kayabildiğini verir.

### 4. Kabul

Kilitli katmandaki duvar, kolon ya da aks referans olur ve yerinde kalır; seçili nesne kilitliyse ret. İki aday ve
seçimin oturum boyunca durması. Döndürülmüş ve kaydırılmış UCS'te aynı mesafe ve aynı kayma (ölçü geometriktir, UCS'ten
bağımsız; yazılan değer uzunluktur). Büyük koordinat (E 487 000, N 4 420 000). Esc hiçbir şey yazmaz; her düzenleme tek
geri alma adımı. Aynı sorgu ve düzenleme Python ve MCP'de.

## Sınırlar

- Eğrisel aks, eğik aks (yana paralel olmayan) ve aksın kesişimine (iki aksa birden) mesafe yok.
- Duvarın boyu yalnız yolun ucundan değişir; yolun içindeki köşe kenarı Kaydır'la oynar. T ve çok duvarlı uç kaymaz.
- Seçim oturumda kalır; dosyaya ve öbür oturumlara geçmez. Kalıcı ölçüye çevirme ve kilitleme (Revit'in kilidi) yok;
  kalıcı bağ ADR 0334'ün kısıtlarıyla kurulur.
- Revit'in tanık çizgisini başka bir nesneye sürükleme yok: adaylar yüz ve aks.

## Doğrulama

- Ortak durumlar, KentOS kodu olmadan ADR'den yazılmış başvurulardan (`scripts/fixtures/wall_shift_cases.py`,
  `column_shift_cases.py`, `opening_distance_cases.py --check`); Rust'ta `fixtures` testi aynı dosyaları oynatır:
  - `cad.walls.distances` 33 (11 yeni): iki yanda akslar, katsız aks her katta, başka kattaki ve yay aks sayılmaz, 30°
    dönük duvar, eksendeki aks solda 0, sola hizalı duvarın ekseni yüzlerinin ortası; boy: L duvarın ikinci kenarında
    yalnız sonu, saplı (T) duvarda ve üç duvarın birleştiği uçta yalnız serbest ucu kayabilir, uç uca ve birleşimi yok
    olan uç kayabilir, iki ucu T bölmede ve kapalı duvarda boy yok.
  - `cad.walls.edit` `shift` 39 (7 yeni): sol ve sağ yanda aksa, 0 ile aksın üstü, sola hizalı ve 30° dönük duvar,
    aksı olmayan yanda ret, aynı mesafe yazılmaz.
  - `cad.walls.edit` `length` 29 (hepsi yeni): üç sabit uç; köşede Kaydır (dışa, içe, ortadan iki köşe birlikte; kuzey
    duvarı uzar, doğu duvarındaki kapı duvarıyla gider, kuzeydeki boşluk yerinde, tek adım); uç uca duvar ve onu yok
    eden boyun reddi; saplanan duvarın serbest ucu; L duvarın iki kenarı; başka kattaki köşe duvarı ve birleşimi yok
    olan uç; T, iki duvarlı uç ve L köşesi ret; kilitli izleyen ve kilitli duvar; kapalı duvar, yay ve olmayan kenar;
    aynı, sıfır ve sonsuz boy; iki duvar, duvar olmayan nesne; plan ve doğrula yazmaz.
  - `cad.columns.distances` 25 (3 yeni) ve `cad.arch.edit` `distance` 30 (7 yeni): katsız ve 1. kattaki kolonun
    aksları, yay aks sayılmaz, 30° dönük kolon, aksa 0, aksı ve yanı olmayan ret, aynı mesafe.
  - `cad.openings.distances` 29 (5 yeni) ve `cad.openings.edit` `distance` 30 (6 yeni): ortası aksın üstündeki kapı,
    pencere, dış yüz, yüzü kesmeyen aks, dönük ve büyük koordinatlı oda, aksı olmayan yan ve bindirme reddi.
  - Toplam 215 durum, 68'i yeni.
- Çekirdek: `ops::grid_ref`'in 3 birim testi (eksenin iki yanında en yakın aks ve üstündeki solda; kutunun yanları ve
  ortasından geçen aks küçük numaralı yanda; parçayı kesen akslar önce ve sonra), `tests/all/wall_length.rs`'in 4 testi
  (serbest uç ve sabit baş; köşe duvarı Kaydır'la, oda uzar; uç uca duvar kısalır, T ve yolun köşesi ret; çok duvarlı
  uç ve geçersiz boy ret).
- Etkileşim: `tests/all/temporary_refs.rs`'in 5 testi (kareyle aksa geçiş ve seçimin yeniden seçimde durması, boy ve
  sabit ucun karesi, Esc hiçbir şey yazmaz, kilitli katmandaki referans ölçer ve yerinde kalır, dönük UCS'te aynı
  mesafe ve kayma, kolon ve açıklığın aksı), `wall_distance.rs`'in güncel 4 testi ve `temporary_refs`'in birim testi;
  her düzenleme tek Geri al adımı (“Duvarı düzenle”).
- Masaüstü: `temporary_scenes`'in sahne testi (bölme aksa 0,60 m, güney duvar 7 m ve doğu duvarın Kaydır'la izlemesi,
  kolon ve kapının aksa mesafesi) ve resimleri (`arac-gecici-aks-adaylar`, `-secildi`, `-yazildi`, `arac-gecici-boy`,
  `-yazildi`, `arac-gecici-kolon-aks`, `arac-gecici-acik-aks`; iki tema, 1440×900 ve 1100×650).
- Gerçek GUI'de bulunup düzeltilenler: sayıya basınca açılan yazı alanı seçim aracını sıfırlıyor, oturumun seçimlerini
  de siliyordu (`Select::reset` artık seçimleri tutar; etkileşim testi düzenlemeden sonra da aksı arar); boy ölçüsünün
  sabit uç işareti ve karesi duvarın uç tutamaçlarının üstündeydi (ölçü çizgisinin uçlarına taşındı); aksa geçen kısa
  yanın sayısı boy ölçüsünün sayısıyla çakışıyordu (boy uzun ölçünün yanına); iki ucu T bölmede hep reddedilecek boy
  gösteriliyordu (yalnız kayabilen uç sayılır, `wall_shift::free_ends`).
- Python SDK'sı katalogdan yeniden üretildi (`DistanceTo`, `WallAnchor`, `GridGap`, `WallLength`,
  `LengthWallsOperation`); MCP araçları katalogdan gelir, `cad.walls.edit` zaten yıkıcı listede.
