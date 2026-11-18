# ADR 8122: 3B pencereler her zaman ayrı işletim sistemi penceresinde

- **Durum:** uygulandı (18 Kasım 2026). Sahibin kararı: 3B görünüm pencereleri her zaman işletim sisteminin ayrı
  penceresinde açılsın; yoksa iş karışabilir.
- **Bağlam:** ADR [8060](8060-secondary-os-windows.md) §9–§10 (ayrı pencereler; 3B görünüm kendiliğinden ayrı açılıyor,
  “Ana pencereye al” onu çizimin üstüne getiriyor ve seçim saklanıyordu), [8119](8119-building-3d-in-the-model-viewer.md),
  [8121](8121-viewer-measuring-and-folding-panes.md); DESIGN.md §7.20–7.21.
- **Web, `.kcad`, komut:** değişmedi. `ayri-pencereler.json`'daki eski seçim silinmez; bu iki pencere için izlenmez.

## §1. Sorun

3B görünüm ve IFC modeli kendiliğinden ayrı pencerede açılıyordu. Ancak şeridin sonundaki “Ana pencereye al” onları
çizimin üstüne alabiliyordu ve bu seçim sonraki açılışlarda da izleniyordu. Çizimin üstündeki 3B pencere planı örtüyor,
planla model aynı yerde üst üste biniyordu. Ana pencereye alınmış bir görünüm sonraki gün de orada açılıyordu ve neyin
nerede açılacağı karışıyordu.

## §2. Karar

- **K266 — Hep ayrı:** pencere sözleşmesine `always_apart` eklendi (`OwnWindow`, varsayılan hayır). 3B görünüm (`View3d`)
  ve IFC modeli (`IfcView`) için evet. Bu pencereler her açılışta işletim sisteminin ayrı penceresinde, planın yanında
  açılır.
- **K267 — Geri alma yok:** bu iki pencerenin şeridinde “Ana pencereye al” gösterilmez. İleti gelse de (`Apart(…,
  false)`, eski bir iz) yerine getirilmez, seçim de saklanmaz. Daha önce saklanmış “ana pencerede” seçimi izlenmez. Bir
  iz “Ana pencereye al”a basmak isterse bu pencerelerde böyle bir denetim yoktur.
- **K268 — Öbür pencereler aynı kalır:** işlemler aracı (`ProcessingTool`) ayrı ya da ana pencerede açılmayı sürdürür;
  ondaki seçim yine saklanır.

## §3. Uygulama

- `shell/windows/surface.rs`: `OwnWindow::always_apart`; `App::is_apart` hep ayrıyı önce sorar; `set_apart` hep ayrı
  pencerede bir şey yapmaz; izin “Ana pencereye al”ı bu pencereleri atlar.
- `shell/scene3d_view.rs`, `shell/ifc_viewer.rs`: `always_apart`; şeridin sonundaki geri alma düğmesi kalktı.

## §4. Sınamalar

- `scene3d_window_tests::the_window_always_opens_beside_the_plan_never_over_the_drawing`: önceden “ana pencerede” seçimi
  saklanmış olsa da pencere ayrı açılır. İz “Ana pencereye al” bulamaz. `Apart(…, false)` pencereyi yerinde bırakır,
  çizimin üstünde pencere yoktur. Esc ile kapanır ve yeniden ayrı açılır.
- Resim sınamalarından “çizimin üstünde” resimleri kalktı (yol 7–8, bina 2c, IFC modeli 5).
