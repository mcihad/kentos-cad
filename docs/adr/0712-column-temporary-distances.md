# ADR 0712: Kolonun geçici ölçüsü ve kaydırılması

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-EDIT-01'in kolon dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-EDIT-01; ADR 0701 (açıklığın geçici ölçüsü), ADR 0710 (duvarın geçici ölçüsü),
  ADR 0343 (aks ve kolon), ADR 0643 (mahalin kolon düşümü), ADR 0707 (aks ağı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Kolon planda en çok “duvardan şu kadar”, “öbür kolondan şu kadar” diye yerleştirilir. KentOS'ta kolonun bir duvar yüzüne ya da
yanındaki kolona net mesafesini görmek için ölçü koymak, değiştirmek için `Taşı` ile yazılan bir yer değiştirme hesaplamak
gerekiyordu; açıklık (ADR 0701) ve duvar (ADR 0710) için olan “seç, sayıya bas, yaz” akışı kolonda yoktu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — About Temporary Dimensions](https://help.autodesk.com/cloudhelp/2023/ENU/RevitLT-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm):
  eleman seçilince en yakın dik/paralel elemanlara geçici ölçüler görünür; sayı değiştirilince seçili eleman taşınır.
- [Revit 2023 — Structural Column Properties](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-StructEng/files/GUID-5EBA5016-4E9C-4131-BD5B-36FC99A92846.htm):
  “Moves With Grids” kolonun yerini aksa bağlar; aks taşınınca kolon aksla gider. Belge bağlı kolonun kendi geçici ölçüsüyle
  taşınmasını anlatmaz.

KentOS kararı: Revit'in akışı kolonda da. Kolonun **dört yanından** dışarıya, aynı kattaki **paralel en yakın yüze** (bir
duvarın bakan yüzü ya da başka bir kolonun yanı) **net** mesafe; değer yazılınca kolon o yana dik kayar, karşıdaki yüz sabit
kalır. Akslara bağlı kolon (ADR 0343, 0707) mesafeyle kaydırılmaz: yeri aksların kesişimidir; neden ve çözüm (Bağı kopar ya da
aksı taşı) sayıya basıldığı anda söylenir, yazı kutusu açılmaz. Alınmayanlar: aksa mesafe (aks yüz değildir), referansın elle
seçilmesi ve kilitlenmesi, kolonun döndürülmesi ya da boyutu, dikdörtgen olmayan (L) kolon.

## Karar

### 1. Ortak hesap: `ops::column_shift`

- **Yanlar:** dikdörtgen kolonun köşeden köşeye dört yanı (`column_corners`'ın sırası: merkezin altı, sağı, üstü, solu;
  kolonun dönüşüyle). Yuvarlak kolonun yanları çevreleyen karesininkidir.
- **Aralıklar:** her yanda, dışarı doğru, aynı kattaki (ya da ikisi de katsız) duvarların düz kenarlarından ve öbür kolonların
  yanlarından o yana 0,001° içinde paralel olup yan boyunca 1e-9 m'den çok örtüşenlerin en yakını. Duvarın yüzleri ADR 0710
  §1'in hizasıyla; yanın 1e-9 m'den çok gerisinde kalan yüz sayılmaz. Mesafe yana diktir, ölçünün yeri örtüşmenin ortası.
  Eşit mesafede duvarlar kolonlardan önce, sonra verilen sıra, sonra küçük kenar.
- **Kaydır:** kolonun merkezi yanın dış normali boyunca `aralık − mesafe` kadar kayar; mesafe 0 kolonu yüze dayar.

### 2. Komutlar

- **`cad.arch.edit`'in yeni işlemi `distance { side, distance }`:** tek kolon (`invalid_column`, `uids`); aks, merdiven ya
  da rampa `not_a_column`, mimari olmayan nesne her işlemdeki gibi `not_arch_element`; sonsuz değer `not_finite`; eksi
  mesafe `invalid_column`; akslara bağlı kolon `column_on_grids` (yeri ve çözümüyle); olmayan yan ve paralel yüzü olmayan
  yan `invalid_column` (`operation.side`); aynı yer `nothing_to_set`; kilitli katman `layer_locked`. Tek adım “Mimari
  nesneyi düzenle”; kolonun izleyenleri (etiketi, onu düşen mahaller, ona bağlı ölçüler) her kolon düzenlemesindeki gibi
  aynı adımda izler.
- **Salt okunur `cad.columns.distances` v1:** kolonların dört yanındaki aralıklar: yan, mesafe, hedef (duvar: kimliği, kenarı,
  yüzü; kolon: kimliği, yanı), ölçünün iki ucu; akslara bağlı kolon `tied` ile söylenir. Python ve MCP aracı katalogdan; ajan
  önce mesafeleri okur, sonra `distance` ile yazar.

### 3. Masaüstü

Tek kolon seçiliyken (komut çalışmazken, kilitli katmanda değilse) dört yandaki net mesafeler, aralığı dik geçen ölçülerle
görünür; karşı yüz işaretlidir. Sayıya basmak sayının yerinde değeri hazır bir yazı kutusu açar; Enter `cad.arch.edit`
`distance` ile yazar, Esc ve okunamayan değer hiçbir şey yazmaz. Akslara bağlı kolonun sayısına basmak nedeni hemen söyler.
Açıklık ve duvarın geçici ölçüleriyle aynı seçim katmanındadır.

Aynı dilimde üç geçici ölçüde iki düzeltme: sayı, üstünde durduğu çizgiyi çizimin zeminiyle örter (çizgi rakamın içinden
geçmez); iki sayı birbirine yakınsa tıklama ilkine değil en yakınına gider (kolonun yanları, dar açıklığın iki yanı).

## Sınırlar

- Aksa, döşeme kenarına ve eğik yüze mesafe yoktur; referans kullanıcı tarafından seçilemez ve kilitlenemez.
- Kolonun boyu, dönüşü ve L/T kolon açıktır; yuvarlak kolon çevreleyen karesiyle ölçülür.
- UCS'e göre ölçü yoktur: mesafe yana diktir, eksenlerden bağımsızdır.

## Doğrulama

- `python3 scripts/fixtures/column_shift_cases.py --check`: `cad.columns.distances.json`'un 22 ve `cad.arch.edit.distance.json`'un
  23 durumu (kare, derin, yuvarlak, dayalı, 30° ve 45° dönük kolon, kat, kilitli katman, sola hizalı duvar, iki kolon, plan,
  doğrula, akslara bağlı kolon ve retler) KentOS kodu olmadan.
  `KENTOS_CASES_ONLY=cad.columns.distances,cad.arch.edit.distance cargo test -p kentos-native-application --test all
  every_command_case`; Python koşucusu; `cargo test -p kentos-mcp`.
- `cargo test -p kentos-geometry-core --test all column_shift` (dört aralık, dönük kolon, yuvarlak kolon ve kat).
- `python -m unittest python.tests.test_column_distances`: SDK'dan okuma, kaydırma, kolonu düşen mahalin aynı adımda izlemesi
  ve tek geri alma; bağlı kolonun reddi.
- `cargo test -p kentos-interaction --test all column_distance` (dört aralık, sayıya basıp yazınca kayma ve tek geri alma, Esc
  ve okunamayan değer, akslara bağlı kolon) ve aynı filtrede açıklık ve duvarın geçici ölçü testleri.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=kolon-mesafe,kolon-mesafe-yazi-kutusu,kolon-mesafe-kaydirildi,kolon-mesafe-yakin,kolon-mesafe-aksa-bagli
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`; `.run/shots/arac-kolon-mesafe-*`.
