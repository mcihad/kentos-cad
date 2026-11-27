# ADR 9352: Okumak için açılan formda Düzenle alt çubuğun solunda

- **Durum:** kabul edildi (2026-11-27). Kaynak: sahibin sözü, 27 Kasım 2026: “Açılan formlarda düzenle butonu sol altta
  olsun sağ alttaki butonlarla aynı dikey hizada”.
- **İlgili:** [ADR 9346](9346-record-flow-inspector-default-form-and-write-names.md) (varsayılan form, salt okunur
  pencere), [ADR 9149](9149-form-edit-from-a-table-row.md) (tablonun satırı formla), DESIGN.md §7.19 (alt çubuk).

## Bağlam

Okumak için açılan kayıtta (Formu aç, haritanın bilgi kartı, düzenlenmeyen katmanın satırı) Düzenle düğmesi formun
gövdesinde, açıklama kutusunun altında duruyordu. Pencerenin öbür düğmeleri alt çubuktaydı. Alt çubukta yazılamayan
kayıt için kapalı bir “Değişikliği hazırla” / “Kaydı uygula” da duruyordu.

## Karar

- **Düzenle:** salt okunur kaydın alt çubuğunun solundadır, sağdaki düğmelerle aynı satırda ve hizada. Birincil
  düğmedir: kaydın tek eylemidir. Kayıt düzenlenemiyorsa (salt okunur bağlantı, tasarımcının önizlemesi) yoktur.
- **Sağ:** salt okunur kayıtta yalnız Vazgeç. Yazılamayan kayıtta kapalı kaydetme düğmesi gösterilmez.
- **Yerleşik form da aynıdır:** tablonun Form görünümündeki kayıt aynı alt çubuğu kullanır.
- **İzler:** “Düzenle” iz denetiminde düğme adıyla basılır (`Control::Press("Düzenle")`).

### Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Alt çubukta Düzenle | yok: Mimari'de kayıt formu yoktur | var | var | var |

### Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python, MCP, yapay zekâ ajanı, eklenti | yok | Pencerenin düzenidir; kayıt `cad.forms.submit` ile yazılır, komut değişmedi. |

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `-- forms:: form_scenes source_rows row_target` 126 geçti; Düzenle'nin aynı satırı düzenlemeye açtığı
  sınamalar (`row_target_tests`) geçiyor.
- **Takımlar:** `cargo test -p kentos-cad` 37, `-p kentos-desktop-cbs` 63 geçti.
- **Resim:** `.run/gui-evidence/kullanim-9346/01-varsayilan-form` (açık ve koyu tema): Düzenle sol altta, Vazgeç sağ
  altta, aynı satırda.
