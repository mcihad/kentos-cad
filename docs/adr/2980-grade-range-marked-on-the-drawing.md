# 2980 — Eğimle kot ver'de aralığın çizimde işaretlenmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-06, beşinci tur. Durum: taslak. ADR 2180'in “Sonra”sındaki “seçilen köşelerin
çizimde işaretlenmesi (önizleme katmanı) ve aralığın kenarını vurgulamak”.

İlgili:
- [ADR 1290](1290-grade-between-vertices.md): Eğimle kot ver (`cad.entities.grade`);
- [ADR 2180](2180-grade-range-picked-on-the-drawing.md): aralığın sahneden seçilmesi;
- [ADR 0172](0172-vertex-table.md) §3: Köşe tablosunun seçili satırlarının çizimde halkalanması.

## Dayanak

- **Autodesk Civil 3D, To Specify a Gradient or Slope Between Points on a Feature Line**
  ([belge](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4A3A774F-06E4-4ED6-8EBE-3B2F8A3E6E7E.htm),
  9 Kasım 2026'da okundu): çizgi seçilir, sonra düzenlenecek parçanın başlangıç ve bitiş noktası, sonra eğim girilir.
  Aralık iki noktayla çizim üstünde tanımlanır.
- **KentOS bugün** (ADR 2180):
  - aralık iki tıkla ya da numarayla seçiliyordu;
  - hangi köşelerin seçildiğini yalnız pencerenin yardım satırı (Y, X) söylüyordu;
  - çizimde bir işaret yoktu. Yaylı kenarı olan bir yolda aralığın hangi kenarları kapsadığı görülmüyordu;
  - Köşe tablosunun seçili köşeleri zaten çizimde halkalanıyordu (ADR 0172 §3). Aynı işaret kullanılır.

## Karar

1. **İşaretler** (masaüstü, yalnız görüntü; komut ve sözleşme değişmez):
   - Eğimle kot ver açıkken başlangıç ve bitiş köşeleri Köşe tablosununki gibi vurgu renkli halkayla işaretlenir;
   - aradaki yol vurgu renginde, ince bir haleyle üstten çizilir. Yaylı kenar yay boyunca noktalarla çizilir
     (çekirdeğin `bulge_path`'ı);
   - aralık kurala uymuyorsa (başlangıç bitişten önce değil) işaret çizilmez; pencere nedenini zaten söyler.
2. **Sahneden seç sırasında:**
   - başlangıç beklenirken işaret yoktur;
   - bitiş beklenirken yalnız seçilen başlangıç halkalanır. Kullanıcı ikinci tıklamada neyin seçildiğini görür.
3. Pencere kapanınca işaretler kalkar. İşaretler pencerenin tuttuğu nesnenin kimliğinden çizilir; seçim değişse de
   pencere açık kaldıkça kalır.
4. **Kapsam işareti:** ADR 1290'daki gibi **CAD (Harita + Yol)**.

## Kanıt

- **Masaüstü** (`grade_window_tests.rs`, `the_range_is_marked_on_the_drawing`):
  - açılışta bütün yol, ilk ve son köşe; yayın noktaları yolun köşe sayısından çok;
  - 2–4 aralığında uçlar 2. ve 4. köşe, yol 2. köşeden 4. köşeye;
  - işaretler katmanı iki köşeyi halkalar;
  - başlangıç beklenirken hiçbir şey, bitiş beklenirken yalnız başlangıç; kapanınca hiçbir şey.

  ADR 2180'in testleri, köşe tablosu ve çapraz denetimler geçer. Resimler `.run/gui-evidence/aralik-2980/`: pencere
  açıkken 2–4 aralığı, yayı dahil.
- **MCP ve Python:** komut değişmedi.

## Sonra

- Yüksek/alçak nokta ekleme (yapıldı, [ADR 3380](3380-grade-high-low-point.md)), ~~eğimle yükselt/alçalt~~ ([ADR 3780](3780-raise-lower-by-reference.md)).
- Aralığın köşelerine numara etiketi.
- Çok parçalı nesne; dönen ve yaylı referansın bağı (MAP-06'nın kalanı).
- Web (TODOS §0.2).
