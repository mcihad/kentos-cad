# 8001 — Esc duvar zincirini korur

Tarih: 2026-11-07. Durum: kabul (M2, ADV-08; plan S1). Kapsam: Duvar aracının zinciri
(`crates/native/interaction/src/path.rs`, `Shape::Wall`).

## Sorun

Duvar aracıyla birkaç köşe çizip Esc'ye basınca çizilen bütün parçalar ileti vermeden kayboluyordu.

- **Kod:** çoklu çizgi aracı (`Path`) `Tool::cancel`'i tanımlamıyordu. Varsayılan `false` oturumu kapatıyor, taslak
  çizime ulaşmadan atılıyordu (`apps/desktop/src/input.rs:472-489`: “its draft is dropped: nothing reaches the
  drawing”).
- **Gerçek pencere** (ADR 8000), `fixtures/interaction/real/duvar-zinciri-esc.txt`:
  - eski ikili: fareyle üç köşe, Esc → 0 nesne, araç kapandı;
  - resim `.run/gui-evidence/mimari-m2/once/`.

## Araştırma (7 Kasım 2026)

- **AutoCAD:**
  - PLINE yardımı (help.autodesk.com, “PLINE (Command)”): komut Enter'la biter; çizilen çoklu çizgi tek nesnedir ve
    kalır.
  - LINE'ın her parçası çizildiği anda ayrı nesnedir; Esc komutu bitirir, parçalar kalır.
- **Revit** (Autodesk'in “Create Walls” başlangıç rehberi ve LinkedIn Learning'in Revit Essential Training dersi): duvar
  zincirinde her duvar ucu tıklanınca yazılır. Bir Esc zinciri bitirir ve araç açık kalır; rehber araçtan çıkmak için de
  Esc'yi söyler.
- **Archicad** (Graphisoft Help Center, zincirle duvar): çift tık bitirir, Backspace yalnız son parçayı geri alır, İptal
  ya da Sil bütün duvarı atar. Graphisoft topluluğunda kullanıcılar Esc'nin bütün zinciri atmasından yakınıyor.
- **Karar dayanağı:** sahibin hedefi “AutoCAD kadar ayrıntılı”; mimari duvar zincirinde en yakın örnek Revit. İkisi de
  çizileni tutar.

## Karar

- **Duvar zincirinde Esc:**
  1. Açık bir soru varsa (genişlik, öteleme, stil adı) önce o bırakılır.
  2. En az bir parça çizildiyse parçalar Enter'ın yazdığı gibi yazılır: tek duvar nesnesi, tek geri alma adımı. Araç
     yeni zincir için açık kalır.
  3. İleti: “Esc zinciri bitirdi: çizilen *n* parça duvar kaldı. Araçtan çıkmak için yeniden Esc, geri almak için
     Ctrl+Z.” (`path::esc_kept`).
  4. Tamamlanmış parça yoksa (tek nokta) araç eskisi gibi kapanır.
- **İkinci Esc** araçtan çıkar. Geri (G) ve Ctrl+Z değişmedi.
- **Kapsam dışı:**
  - Mahal ayırıcı: Enter'la yazılıp kapanır, değişmedi.
  - Genel çoklu çizgi ve kapalı alan araçları: AutoCAD'de PLINE de Esc'de korunur, ama çekirdek CAD'in bütün izlerini
    etkileyen ayrı bir karar; çekirdek CAD planına not edildi.

## Ölçüm

- **Testler:**
  - `crates/native/interaction/tests/all/wall_chain_escape.rs`: 4 yeni durum. Esc iki parçayı tutar, araç kalır, ileti
    doğru; ikinci Esc çıkar; tek geri alma zinciri kaldırır. Tek noktada tutulacak bir şey yok. Soru önce bırakılır.
    Enter eskisi gibi.
  - `kentos-interaction` paketinin bütün testleri: 896 geçti, 2 atlandı.
  - Kod içi izler (wall-tool, wall-edit, wall-partition, wall-rectangle, wall-bond-drag, acceptance-flat, -house,
    -change, -renovation, -curved, -room, polygon-keys, polyline-arc): geçti.
- **Gerçek pencere** (`.run/gui-evidence/mimari-m2/`, `once-sonra.png`):
  - üç köşeli zincir, ilk Esc → L biçimli duvar kaldı (1 nesne), araç açık, ileti durum çubuğunda;
  - ikinci Esc → araç kapandı, duvar duruyor;
  - imleçteki lastik bant parçası tutulmuyor.
- **Etkisi:** kaybolan bir zinciri yeniden çizmek tık başına P + BB ve aracı yeniden başlatmak demekti. Üç köşede KLM'ye
  göre yaklaşık 5 s ve işin kendisi; artık sıfır.
