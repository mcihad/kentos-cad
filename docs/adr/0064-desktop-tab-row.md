# ADR 0064: Masaüstünün şerit satırı web'inki gibi: Yardım, koordinat sistemi, kaydedilmemiş noktası

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** DESIGN.md §7.1, §7.3.1; TODOS.md `UI-11`; ADR 0051 (pencereye sığan şerit), 0058 (tam ekran, komut arama)
- **Web'in bugünkü hâli:**
  - web ajanı Yardım menüsünün başına Komut ara'yı koydu (`324620d`);
  - şerit satırı pencere daralınca adım adım yer açar (`56c6e3a`, `Ribbon.fitBar`).

## Bağlam

Web'in şerit satırında sekmelerden sonra şunlar vardır:
- çizimin adı, kaydedilmemiş değişiklik varsa önünde vurgu renginde nokta;
- komut arama kutusu;
- koordinat sistemi düğmesi;
- Tam ekran;
- Yardım "?";
- şeridi daraltma.

Masaüstünde ad "• kaydedilmedi" yazısıyla gösteriliyordu. Koordinat sistemi düğmesi ve Yardım yoktu; Komut ara, Klavye kısayolları ve KentOS CAD hakkında yalnız kısayolla ya da komut satırından açılıyordu.

## Karar

- **Çizimin adı:**
  - Bulut projesinde çalışma alanı öndedir.
  - Kaydedilmemiş değişiklik varsa önünde 6 px vurgu renginde nokta vardır. İpucu: "Kaydedilmemiş değişiklikler var".
  - "• kaydedilmedi" yazısı kalktı.
- **Koordinat sistemi düğmesi:**
  - Simge ve sistemin adı (ör. "TUREF / TM36"); tıklamak Koordinat sistemi penceresini açar (`crs.set`).
  - İpucu: "{ad}, EPSG:{srid}. Değiştirmek için tıklayın."
- **Yardım "?":** web'in menü çubuğundaki Yardım menüsüdür; envanterden okunur (`layout.menus`, `Catalog::menu`).
  - Bugün: Komut ara (Alt+Q), Klavye kısayolları (F1), çizgi, KentOS CAD hakkında.
  - Masaüstünde çalışmayan bir komut soluk görünür.
  - İpucu: "Yardım" / "Klavye kısayolları ve KentOS CAD hakkında."
- **Dar pencere** (web'in adımlarıyla):
  - önce koordinat sisteminin adı gizlenir, simgesi kalır;
  - sonra çizimin adı kısalır.
  - Düğmeler boyutlarını korur. Genişlik yazının boyundan, pencereye sığıp sığmadığı `responsive` ile karşılaştırılarak kestirilir.

## Web'den ayrılanlar

- **Arama kutusu:** şerit satırında henüz yok. Komut ara, komut satırının listesini açar (ADR 0058). Kutu ayrı bir dilimdir.
- **Adın kısalması:** Iced üç nokta koymaz; dar pencerede ad kırpılır.

## Doğrulama

- `view_commands::tests::the_ribbons_help_menu_is_the_webs`: menü envanterdeki gibidir.
- `pnpm rust:test:desktop` (clippy temiz), `pnpm inventory:check`.
- Görüntüler (`view_commands::screens`, `.run/shots/gorunum-yardim-*`), koyu ve açık, 1440×900 ve 1100×650:
  - kaydedilmemiş noktası;
  - koordinat sistemi (dar pencerede yalnız simgesi);
  - açık Yardım menüsü.
