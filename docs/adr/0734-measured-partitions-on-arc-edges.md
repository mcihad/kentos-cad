# ADR 0734: Yaylı kaynak ve sınırla ölçülü bölme

- **Durum:** kabul edildi (29 Ekim 2026); `plan/architecture.md` ARCH-DRAW-02'nin “Yaylı kaynak/sınır dilimi” alt maddesi.
- **Bağlam:** ADR 0657 (ölçülü düz bölme, `cad.walls.partition` v1), 0339 (duvar, yay kenarı tek büküm), 0340 (birleşim
  çözücüsü), 0657'nin 27 Ekim eki (bölmenin yanı imleci izler).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0657'nin ilk dilimi yalnız düz kaynak ve düz sınır alıyordu: yaylı kaynak ya da sınır kenarı `invalid_wall` ile, açık
sözle reddediliyordu (“sessiz düzleştirme yok”). Dairesel cephe, yuvarlak salon ve kavisli koridor gibi planlarda kullanıcı
bölmeyi elle çizip uzaklığı ölçmek zorunda kalıyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit API 2026 — Curve.CreateOffset](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-API-MainReference/files/html/450217f3-c0b5-42af-3a05-376ae383d28a.htm):
  yayın ötelemesi yayın düzleminde, **aynı merkezli** başka bir yaydır; yarıçap öteleme kadar büyür ya da küçülür, yön
  başvuru vektörüyle (sağ/sol) seçilir.
- [Revit — Move Elements with the Offset Tool](https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Model/files/GUID-7D416FFB-36A2-4B98-95D1-0A007DCD51CE.htm):
  Offset duvarı boyuna dik, yazılan ya da sürüklenen uzaklıkla kopyalar (Numerical, Copy).
- [Revit LT — Resizing Arc Walls](https://help.autodesk.com/cloudhelp/2014/ENU/Revit-LT/files/GUID-E446DC44-EF99-4FE4-A4BE-CAA8A9E13A51.htm):
  yaylı duvarın yarıçapı Keep Concentric ile merkez korunarak değişir.
- [Archicad 29 — Offset All Edges](https://help.graphisoft.com/AC/29/INT/_AC29_Help/030_Interaction/030_Interaction-75.htm):
  çokgen öğelerin (eğriler dahil) bütün kenarlarını içe ya da dışa öteler, Ctrl ile kopya bırakır; yaylı kenarın ne
  olduğunu söylemez (bu yüzden karar Revit'in belgelenmiş kuralına dayanır).

KentOS kararı: yaylı kaynağın bölmesi Revit'in CreateOffset'i gibi aynı merkezli yaydır; uzaklık radyal ölçülür ve iki
yüz arasında her yarıçapta aynıdır. Düz kaynağın davranışı bit bit aynı kalır.

## Karar

### 1. Geometri (`geom::wall_partition`)

- Kaynak kenarın büküm değeri `tan(θ/4)`'tür (θ saat yönü tersine pozitif). Yaylı kaynakta bölmenin ekseni kaynağın yay
  merkezinde, yarıçapı `r_p = r − sgn(θ)·o` olan çemberdir; `o` kaynağın eksen ötelemesi artı yan × eksen aralığıdır
  (solu pozitif). Saat yönü tersine yayın **solu merkezine doğrudur**, saat yönündekinin solu dışarıdır: `offset_bulge_path`
  ve `wall::line_at`'in “pozitif = yolun solu” kuralıyla aynı.
- Net uzaklık (`clear`) iki çizilmiş yüz arasındaki radyal boşluktur; eksen aralığı (`axis`) iki eksen arasıdır. Yüzler
  aynı merkezli olduğu için boşluk yay boyunca değişmez.
- Bölmenin iç yüzü merkeze ulaşırsa (`r_p − en/2 ≤ 0`) atomik ret: “Bölme yayın merkezine ulaşıyor; daha küçük bir uzaklık
  ya da yayın dış yanını seçin.”
- Uçlar:
  - `source`: kaynağın uçlarının merkezden izdüşümleri (aynı açılar).
  - `length`: `anchor` ve boy bölmenin **kendi ekseni** boyunca, kaynağın başlangıcının izdüşümünden; aralık
    `[0, r_p·|θ|]`'yı aşarsa ADR 0657'nin sözüyle ret.
  - `between`: sınırlar düz ya da yaylı olabilir. Bölmenin ekseni (doğru ya da çember) her sınırın ekseniyle (yaylı sınırda
    yalnız yayın açı aralığındaki kısmı) kesiştirilir; birden çok buluşmadan kaynağın ortasına en yakını alınır (çemberde
    açı kaynağın ortası çevresinde sarılır, halkanın arkası seçilmez). Eşitlikte küçük olan.
  - Retler: paralel düz sınır, aynı merkezli yaylı sınır (“Sınır duvarı bölmeyle eş merkezli; kesişen başka bir kenar
    seçin.”), ulaşılmayan sınır, ekseni yayın merkezini aşan sınır, aynı yerdeki iki sınır.
- Sonuç iki noktalı yoldur ve tek büküm taşır: `β = tan(sgn(θ)·(t1 − t0)/r_p/4)`; belge onu nokta başına saklar
  (`[β, 0]`). Ölçü çizgisi uçların ortasındaki yarıçap üzerinde, kaynak yüzden bölmeye doğru net uzaklık kadardır.

### 2. Sözleşme

- `cad.walls.partition` v1 aynı kalır (yeni girdi yok); yaylı kenar artık reddedilmez. Katalog özeti yayları söyler.
- `WallPartitionPlan.sourceFaceBulge` (isteğe bağlı, birimsiz): yaylı kaynakta kaynak yüzün iki noktası arasındaki yayın
  bükümü; düz kaynakta yok. Önizleme kesikli yüzü bununla çizer.
- KCAD şeması, birim, ölçek ve çizim hattı değişmez: bölme yay kenarlı sıradan bir duvardır (ADR 0339).

### 3. Araç (Ölçülü bölme)

- Kenar, çizildiği yerden seçilir: yaylı kenarda uzaklık yayın kendisine (açı aralığında), dışında en yakın uca; kirişe
  tıklamak kenar seçmez ve “Bir duvarın düz ya da yaylı kenarına tıklayın.” der.
- Seçilen kenarın vurgusu ve kesikli kaynak yüz ortak çekirdeğin `bulge_path`'iyle, 0,5 piksel sapmayla parçalanır.
- Yan imleci izler: yaylı kenarda imleç çemberin içindeyse saat yönü tersine yayın solu, dışındaysa sağı (ADR 0657 eki).
- İstem “referans alınacak düz ya da yaylı duvar kenarını seçin”.

## Sınırlar

Bölme tek kaynak kenarı izler (çok kenarlı ya da kapalı yol boyunca sürmez); uzaklık kalıcı kısıt değildir. Elips ya da
eğri kenarlı duvar yoktur. Farklı kattaki sınırın reddi ve kilitli kaynağın yalnız okunması ADR 0657'deki gibidir.

## Doğrulama

- `uv run --no-project --with mpmath python3 scripts/fixtures/walls_partition_arc_cases.py --check`: 22 bağımsız durum, 50
  basamakta, KentOS kodu olmadan (`fixtures/commands/v1/cad.walls.partition.arcs.json`):
  - 8 yaylı kaynak: saat yönü tersine ve saat yönünde, sol ve sağ, net ve eksen, orta/sol/sağ hizalı kaynak; çizim birimi
    m, mm, cm, in, ft; üçte biri (500000, 4400000) çevresinde; planlar.
  - 3 boy (ileri, geri, iki yana) ve uçları aşan boyun reddi.
  - 4 radyal duvar arası (içte ve dışta, iki dönüş), yaylı kaynakla yaylı sınır, düz kaynakla yaylı sınır (biri büyük
    koordinatta mm çiziminde).
  - Retler: merkezi aşan bölme, ulaşılmayan yaylı sınır, aynı merkezli sınır.
  - Geri alma ve yineleme adımları.
- ADR 0657'nin 90 düz durumu değişmeden geçer (`walls_partition_cases.py --check`); Rust'ta
  `KENTOS_CASES_ONLY=cad.walls.partition cargo test -p kentos-native-application --test all every_command_case` ve Python'un
  tipli katmanında `KENTOS_CASES_ONLY=cad.walls.partition .run/py/bin/python -m unittest python.tests.test_command_cases`
  112 durumu oynatır.
- Çekirdek testi `geom::wall_partition::tests`: dört yönde net boşluk, KentOS'un çizdiği yüzlerin (`wall::line_at`)
  yarıçapları arasında 1,5 m (1e-12).
- Etkileşim testi `partition_arc.rs`: yay çizildiği yerden seçilir, kiriş seçilmez; bölme imlecin yanına, içe ve dışa,
  aynı merkezle yazılır; kesikli kaynak yüz yaydır.
- MCP testleri, `python.tests.test_wall_partition`, clippy (geometry-core, commands, native-application, interaction,
  desktop), `wasm32` denetimi, manifest ve kapsam matrisi.
- Gerçek GUI: `KENTOS_SHOTS_ONLY=mimari-bolme-yay,mimari-bolme-yay-arasi cargo test -p kentos-desktop tools_screens --
  --ignored --nocapture`: yayın içinde 3,000 m net uzaklıkla aynı merkezli önizleme ve iki radyal duvar arasında yazılmış
  yaylı bölme; 1440×900 ve 1100×650, koyu ve açık (`.run/gui-evidence/2026-10-29-partition-arcs/`, Git'e eklenmez).
