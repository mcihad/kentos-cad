//! 3B nesne tarayıcısı örneğinin kitaplığı: kentin yapıları, kent
//! mobilyası, bitkiler, altyapı ve afet donatısı. Ağlar KentOS UI'ın yapı
//! taşlarından kurulur (gerçek kitaplıkta buluttan gelecek modellerin yerini
//! tutar); ölçüler metredir.

use std::rc::Rc;

use iced::Color;

use kentos_ui::widget::class_scale::Class;
use kentos_ui::widget::floor_picker::Floor;
use kentos_ui::widget::mesh::Mesh;
use kentos_ui::widget::object_browser::{Lod, Object3d, Source};

fn rgb(hex: u32) -> Color {
    Color::from_rgb8(
        (hex >> 16) as u8,
        (hex >> 8 & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

/// Örnek nesneler.
pub fn sample_objects() -> Vec<Object3d> {
    let plaster = rgb(0xd9cbb3);
    let tile = rgb(0xb5532f);
    let concrete = rgb(0xb8b6ae);
    let glass = rgb(0x7fa6b8);
    let stone = rgb(0xd8d2c4);
    let lead = rgb(0x7d868f);
    let metal = rgb(0x6f767d);
    let wood = rgb(0x8b5a2b);
    let leaf = rgb(0x5d8f3a);
    let deep = rgb(0x3f6e33);
    let red = rgb(0xc8372d);
    let green = rgb(0x2e8b57);

    let item = |id: &str, name: &str, category: &str, mesh: Mesh| {
        Object3d::new(id, name, category, Some(Rc::new(mesh)))
    };

    vec![
        item("ev-mustakil", "Müstakil ev", "Yapı", house(plaster, tile))
            .lod(Lod::Lod2)
            .format("CityGML")
            .source(Source::Library)
            .tags(["konut", "beşik çatı"]),
        item(
            "blok-konut",
            "Konut bloğu",
            "Yapı",
            block(concrete, plaster),
        )
        .lod(Lod::Lod1)
        .format("CityGML")
        .source(Source::Project)
        .tags(["konut", "5 kat", "deprem envanteri"]),
        item("kule-ofis", "Ofis kulesi", "Yapı", tower(concrete, glass))
            .lod(Lod::Lod2)
            .format("IFC")
            .triangles(48_210)
            .source(Source::Cloud)
            .tags(["ticari", "yüksek yapı"]),
        item("okul", "Okul", "Yapı", school(plaster, tile))
            .lod(Lod::Lod2)
            .format("CityGML")
            .source(Source::Cloud)
            .tags(["eğitim", "toplanma alanı"]),
        item("cami", "Cami", "Yapı", mosque(stone, lead))
            .lod(Lod::Lod3)
            .format("glTF")
            .triangles(26_480)
            .source(Source::Cloud)
            .tags(["ibadet", "kubbe", "minare"]),
        item("lamba", "Sokak lambası", "Kent mobilyası", lamp(metal))
            .lod(Lod::Lod3)
            .source(Source::Library)
            .tags(["aydınlatma"]),
        item("bank", "Park bankı", "Kent mobilyası", bench(wood, metal))
            .lod(Lod::Lod3)
            .source(Source::Library)
            .tags(["oturma", "park"]),
        item(
            "durak",
            "Otobüs durağı",
            "Kent mobilyası",
            shelter(metal, glass),
        )
        .lod(Lod::Lod2)
        .source(Source::Cloud)
        .tags(["ulaşım"]),
        item("cinar", "Çınar", "Bitki", plane_tree(wood, leaf))
            .lod(Lod::Lod2)
            .source(Source::Library)
            .tags(["ağaç", "yaprak döken"]),
        item("selvi", "Selvi", "Bitki", cypress(wood, deep))
            .lod(Lod::Lod2)
            .source(Source::Library)
            .tags(["ağaç", "her dem yeşil"]),
        item("cali", "Çalı", "Bitki", bush(leaf))
            .lod(Lod::Lod1)
            .source(Source::Project)
            .tags(["peyzaj"]),
        item("hidrant", "Yangın musluğu", "Altyapı", hydrant(red))
            .lod(Lod::Lod3)
            .source(Source::Library)
            .tags(["itfaiye", "su"]),
        item("trafik", "Trafik lambası", "Altyapı", traffic_light(metal))
            .lod(Lod::Lod3)
            .source(Source::Cloud)
            .tags(["kavşak", "sinyalizasyon"]),
        item(
            "tabela",
            "Toplanma alanı tabelası",
            "Afet",
            sign(metal, green),
        )
        .lod(Lod::Lod2)
        .source(Source::Library)
        .tags(["AFAD", "toplanma alanı", "yönlendirme"]),
        item(
            "cadir",
            "Afet çadırı",
            "Afet",
            tent(rgb(0xe8ecef), rgb(0x2f6fb5)),
        )
        .lod(Lod::Lod2)
        .source(Source::Cloud)
        .tags(["barınma", "geçici"]),
        item(
            "konteyner",
            "Konteyner kent birimi",
            "Afet",
            container(rgb(0xece8dc)),
        )
        .lod(Lod::Lod1)
        .source(Source::Cloud)
        .tags(["barınma", "geçici", "konteyner kent"]),
    ]
}

fn house(wall: Color, roof: Color) -> Mesh {
    Mesh::gable([8.0, 6.0], 3.0, 5.2, wall, roof)
        .merge(Mesh::cuboid(
            [5.6, 3.6, 3.8],
            [6.3, 4.3, 5.9],
            rgb(0x8a5a44),
        ))
        .merge(Mesh::cuboid(
            [3.4, -0.06, 0.0],
            [4.4, 0.0, 2.1],
            rgb(0x6b4a32),
        ))
}

fn block(wall: Color, top: Color) -> Mesh {
    let mut mesh = Mesh::prism(
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 12.0], [0.0, 12.0]],
        0.0,
        15.0,
        wall,
        top,
    );

    // Kat döşemelerinin çizgisi: ince, koyu bantlar.
    for floor in 1..5 {
        let z = floor as f32 * 3.0;
        mesh = mesh.merge(Mesh::cuboid(
            [-0.05, -0.05, z - 0.12],
            [18.05, 12.05, z + 0.12],
            rgb(0x8e8b84),
        ));
    }

    mesh.merge(Mesh::cuboid([7.0, 4.0, 15.0], [11.0, 8.0, 17.4], top))
}

fn tower(base: Color, glass: Color) -> Mesh {
    Mesh::cuboid([0.0, 0.0, 0.0], [24.0, 24.0, 6.0], base)
        .merge(Mesh::cuboid([4.0, 4.0, 6.0], [20.0, 20.0, 46.0], glass))
        .merge(Mesh::cuboid([6.0, 6.0, 46.0], [18.0, 18.0, 49.0], base))
}

fn school(wall: Color, roof: Color) -> Mesh {
    Mesh::gable([30.0, 10.0], 8.0, 10.5, wall, roof)
        .merge(Mesh::gable([10.0, 14.0], 8.0, 10.5, wall, roof).translate([0.0, 10.0, 0.0]))
}

fn mosque(stone: Color, lead: Color) -> Mesh {
    let mut mesh = Mesh::cuboid([0.0, 0.0, 0.0], [16.0, 16.0, 10.0], stone)
        // Kasnak ve kubbe.
        .merge(Mesh::cylinder([8.0, 8.0], 6.2, 10.0, 12.0, 16, stone))
        .merge(Mesh::sphere([8.0, 8.0, 12.0], 6.2, 24, true, lead))
        .merge(Mesh::cone([8.0, 8.0], 0.25, 18.2, 19.6, 6, rgb(0xc9a24a)));

    // Köşelerde yarım kubbeler.
    for [x, y] in [[2.0, 2.0], [14.0, 2.0], [2.0, 14.0], [14.0, 14.0]] {
        mesh = mesh.merge(Mesh::sphere([x, y, 10.0], 1.6, 12, true, lead));
    }

    // Minare: gövde, şerefe, külah.
    mesh.merge(Mesh::cylinder([19.5, 1.5], 1.0, 0.0, 26.0, 12, stone))
        .merge(Mesh::frustum([19.5, 1.5], 1.0, 1.5, 19.0, 19.8, 12, stone))
        .merge(Mesh::cone([19.5, 1.5], 1.0, 26.0, 31.0, 12, lead))
}

fn lamp(metal: Color) -> Mesh {
    Mesh::cylinder([0.0, 0.0], 0.09, 0.0, 6.0, 8, metal)
        .merge(Mesh::cuboid([-0.05, -0.05, 5.8], [1.6, 0.05, 5.9], metal))
        .merge(Mesh::cuboid(
            [1.2, -0.18, 5.62],
            [1.9, 0.18, 5.8],
            rgb(0xe9e1b8),
        ))
        .merge(Mesh::cylinder([0.0, 0.0], 0.18, 0.0, 0.4, 8, metal))
}

fn bench(wood: Color, metal: Color) -> Mesh {
    let mut mesh = Mesh::new();

    // Oturma ve sırt latları.
    for slat in 0..3 {
        let y = slat as f32 * 0.16;
        mesh = mesh.merge(Mesh::cuboid([0.0, y, 0.42], [1.8, y + 0.12, 0.46], wood));
    }

    for slat in 0..2 {
        let z = 0.62 + slat as f32 * 0.16;
        mesh = mesh.merge(Mesh::cuboid([0.0, 0.46, z], [1.8, 0.5, z + 0.12], wood));
    }

    for x in [0.1, 1.62] {
        mesh = mesh
            .merge(Mesh::cuboid([x, 0.0, 0.0], [x + 0.08, 0.5, 0.42], metal))
            .merge(Mesh::cuboid([x, 0.44, 0.42], [x + 0.08, 0.5, 0.95], metal));
    }

    mesh
}

fn shelter(metal: Color, glass: Color) -> Mesh {
    let mut mesh = Mesh::cuboid([-0.2, -0.3, 2.5], [4.2, 1.6, 2.62], metal).merge(Mesh::cuboid(
        [0.0, 1.3, 0.1],
        [4.0, 1.36, 2.4],
        glass,
    ));

    for x in [0.0, 3.9] {
        mesh = mesh.merge(Mesh::cuboid([x, 1.2, 0.0], [x + 0.1, 1.4, 2.5], metal));
    }

    mesh.merge(Mesh::cuboid([0.6, 0.9, 0.42], [3.4, 1.25, 0.48], metal))
}

fn plane_tree(trunk: Color, leaf: Color) -> Mesh {
    Mesh::cylinder([0.0, 0.0], 0.35, 0.0, 4.6, 10, trunk)
        .merge(Mesh::sphere([0.0, 0.0, 7.2], 3.6, 16, false, leaf))
        .merge(Mesh::sphere(
            [1.6, 0.8, 5.8],
            2.2,
            12,
            false,
            scale(leaf, 0.9),
        ))
        .merge(Mesh::sphere(
            [-1.4, -0.9, 6.2],
            2.0,
            12,
            false,
            scale(leaf, 1.08),
        ))
}

fn cypress(trunk: Color, leaf: Color) -> Mesh {
    Mesh::cylinder([0.0, 0.0], 0.18, 0.0, 1.2, 8, trunk)
        .merge(Mesh::frustum([0.0, 0.0], 0.8, 0.95, 1.0, 3.0, 12, leaf))
        .merge(Mesh::cone([0.0, 0.0], 0.95, 3.0, 10.5, 12, leaf))
}

fn bush(leaf: Color) -> Mesh {
    Mesh::sphere([0.0, 0.0, 0.7], 0.9, 12, false, leaf).merge(Mesh::sphere(
        [0.8, 0.3, 0.55],
        0.65,
        10,
        false,
        scale(leaf, 0.9),
    ))
}

fn hydrant(red: Color) -> Mesh {
    Mesh::cylinder([0.0, 0.0], 0.22, 0.0, 0.12, 12, scale(red, 0.8))
        .merge(Mesh::cylinder([0.0, 0.0], 0.15, 0.12, 0.78, 12, red))
        .merge(Mesh::sphere([0.0, 0.0, 0.78], 0.15, 12, true, red))
        .merge(Mesh::cuboid(
            [0.12, -0.06, 0.45],
            [0.3, 0.06, 0.57],
            scale(red, 0.85),
        ))
        .merge(Mesh::cuboid(
            [-0.3, -0.06, 0.45],
            [-0.12, 0.06, 0.57],
            scale(red, 0.85),
        ))
}

fn traffic_light(metal: Color) -> Mesh {
    let mut mesh = Mesh::cylinder([0.0, 0.0], 0.08, 0.0, 3.4, 8, metal).merge(Mesh::cuboid(
        [-0.18, -0.16, 2.3],
        [0.18, 0.12, 3.4],
        rgb(0x2b2e33),
    ));

    for (step, color) in [rgb(0xe04a3f), rgb(0xe9b33b), rgb(0x46b05c)]
        .into_iter()
        .enumerate()
    {
        let z = 3.1 - step as f32 * 0.32;
        mesh = mesh.merge(Mesh::cuboid([-0.1, -0.2, z], [0.1, -0.16, z + 0.2], color));
    }

    mesh
}

fn sign(metal: Color, green: Color) -> Mesh {
    Mesh::cuboid([0.0, 0.0, 0.0], [0.08, 0.08, 2.6], metal)
        .merge(Mesh::cuboid([1.42, 0.0, 0.0], [1.5, 0.08, 2.6], metal))
        .merge(Mesh::cuboid([-0.1, -0.04, 1.6], [1.6, 0.04, 2.6], green))
        .merge(Mesh::cuboid(
            [0.2, -0.06, 1.85],
            [1.3, -0.04, 2.35],
            rgb(0xf4f4f0),
        ))
}

fn tent(fabric: Color, roof: Color) -> Mesh {
    Mesh::gable([4.0, 3.0], 1.3, 2.5, fabric, roof)
}

fn container(shell: Color) -> Mesh {
    Mesh::cuboid([0.0, 0.0, 0.0], [6.0, 2.4, 2.6], shell)
        .merge(Mesh::cuboid(
            [2.6, -0.04, 0.0],
            [3.5, 0.0, 2.1],
            rgb(0x6b7580),
        ))
        .merge(Mesh::cuboid(
            [0.6, -0.04, 1.2],
            [1.8, 0.0, 2.0],
            rgb(0x7fa6b8),
        ))
        .merge(Mesh::cuboid(
            [4.3, -0.04, 1.2],
            [5.5, 0.0, 2.0],
            rgb(0x7fa6b8),
        ))
}

fn scale(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r * amount).min(1.0),
        g: (color.g * amount).min(1.0),
        b: (color.b * amount).min(1.0),
        a: color.a,
    }
}

/// Şehir ve afet sayfasının yapısı: iki bodrum, zemin ve beş kat; her kat
/// ayrı bir dilim. Dizinin ilki bütün katlar, sonrakiler sırasıyla katın
/// vurgulandığı hâller (önizlemenin önbelleği seçime göre değişsin diye
/// önceden kurulur).
pub fn floor_meshes() -> Vec<Rc<Mesh>> {
    let shell = rgb(0xcfc8b8);
    let accent = rgb(0x4c7fe0);
    let floors = 8;
    let height = 3.2;
    let build = |highlight: Option<usize>| {
        let mut mesh = Mesh::new();

        for floor in 0..floors {
            // İlk iki dilim zeminin altındadır.
            let z0 = (floor as f32 - 2.0) * height;
            let color = if highlight == Some(floor) {
                accent
            } else if floor < 2 {
                rgb(0x9a9488)
            } else {
                shell
            };

            mesh = mesh
                .merge(Mesh::cuboid(
                    [0.0, 0.0, z0],
                    [16.0, 11.0, z0 + height - 0.25],
                    color,
                ))
                .merge(Mesh::cuboid(
                    [-0.15, -0.15, z0 + height - 0.25],
                    [16.15, 11.15, z0 + height],
                    rgb(0x8e8b84),
                ));
        }

        Rc::new(mesh)
    };

    std::iter::once(build(None))
        .chain((0..floors).map(|floor| build(Some(floor))))
        .collect()
}

/// Hasar sınıflarının renkleri: yeşilden koyu kırmızıya (veridir; sınıfın
/// adı ve sayısı her zaman yazar).
pub fn damage_classes() -> Vec<Class> {
    vec![
        Class::new("Hasarsız", rgb(0x4fa86b)).count(612),
        Class::new("Az hasarlı", rgb(0xc9c24f)).count(318),
        Class::new("Orta hasarlı", rgb(0xe3923c)).count(201),
        Class::new("Ağır hasarlı", rgb(0xd24b3e)).count(112),
        Class::new("Göçmüş", rgb(0x7d1f24)).count(41),
    ]
}

/// Sel senaryosunun su derinliği sınıfları: aralıklarıyla, eşit bantlı.
pub fn depth_classes() -> Vec<Class> {
    vec![
        Class::new("Islak zemin", rgb(0xbfddee))
            .note("0–0,1 m")
            .count(2_140),
        Class::new("Sığ", rgb(0x86bfe0))
            .note("0,1–0,5 m")
            .count(860),
        Class::new("Orta", rgb(0x4a95cf)).note("0,5–1 m").count(310),
        Class::new("Derin", rgb(0x2263a8)).note("1–2 m").count(96),
        Class::new("Çok derin", rgb(0x133a6e))
            .note("2 m üstü")
            .count(18),
    ]
}

/// Yapının katları, alttan üste: iki bodrum, zemin ve beş kat; katın rengi
/// senaryodaki hasar sınıfı.
pub fn building_floors() -> Vec<Floor> {
    let classes = damage_classes();
    let tone = |class: usize| classes[class].color;
    let floor = |name: &str, index: i32| Floor::new(name, index as f32 * 3.2);

    vec![
        floor("−2", -2).note("Otopark"),
        floor("−1", -1).note("Sığınak"),
        floor("Z", 0).note("Ağır hasar").tone(tone(3)),
        floor("1", 1).note("Orta hasar").tone(tone(2)),
        floor("2", 2).note("Orta hasar").tone(tone(2)),
        floor("3", 3).note("Az hasar").tone(tone(1)),
        floor("4", 4).note("Az hasar").tone(tone(1)),
        floor("5", 5).note("Hasarsız").tone(tone(0)),
    ]
}
