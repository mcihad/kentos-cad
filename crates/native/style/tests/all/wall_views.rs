//! A view's own walls (docs/adr/0696's 29 October addendum): a layer built at a detail other than the store's draws
//! what a store at that detail draws, the store keeping its own. An L of two walls of plaster, brick and plaster, the
//! brick hatched, joined at their corner.

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::wall::{Detail, View};
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use kentos_native_style::{ReadingUnit, View as StyleView};
use serde_json::json;

fn wall(id: u32, a: (f64, f64), b: (f64, f64)) -> Entity {
    serde_json::from_value(json!({
        "kind": "wall", "id": id, "layerId": "duvar", "attrs": {},
        "pts": [{ "x": a.0, "y": a.1 }, { "x": b.0, "y": b.1 }], "width": 0.24, "justify": "center",
        "components": [
            { "name": "Sıva", "thickness": 0.02, "detail": "detailed" },
            { "name": "Tuğla", "detail": "standard", "hatch": { "pattern": "ANSI31", "scale": 1.0, "angle": 0.0 } },
            { "name": "Sıva", "thickness": 0.02, "detail": "detailed" }
        ],
        "fill": "#5A5A5A"
    }))
    .expect("a wall")
}

/// A store holding `list`, drawing its walls at `view`.
fn store_at(list: &[&Entity], view: View) -> Store {
    let mut store = Store::new();
    store.put_many(list.iter().map(|e| {
        let b = e.base();
        (f64::from(b.id), b.layer_id.as_str(), false, shape(e))
    }));
    store.set_wall_view(view);
    store
}

/// The layer's batches as the drawing area builds them, the walls at `walls` (none: the store's).
fn built(store: &Store, list: &[&Entity], walls: Option<View>) -> (String, Vec<f32>) {
    let style: LayerStyle = serde_json::from_value(json!({
        "color": "#4E79A7", "lineType": "continuous", "lineWeight": 0.25,
    }))
    .expect("a style");
    let library = StyleLibrary::default();
    let names = |_: &str| "Duvar".to_owned();
    let opts = BuildOptions {
        line_types: None,
        layer_styles: None,
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 100.0,
        screen: false,
        hairlines: false,
        clip: None,
        library: &library,
        layer_name: &names,
        view: StyleView {
            walls,
            ..Default::default()
        },
        frame: None,
        reading_unit: ReadingUnit::METRE,
    };
    let (_, batches) = build_layer(store, &style, list, &opts).expect("the core builds");
    (batches.json, batches.data)
}

#[test]
fn a_layer_at_a_views_own_detail_is_what_a_store_at_it_draws() {
    let walls = [
        wall(1, (0.0, 0.0), (4.0, 0.0)),
        wall(2, (4.0, 0.0), (4.0, 3.0)),
    ];
    let list: Vec<&Entity> = walls.iter().collect();
    let own = View {
        detail: Detail::Schematic,
        scale: 100.0,
    };
    let store = store_at(&list, own);
    let plain = built(&store, &list, None);
    for detail in [Detail::Schematic, Detail::Standard, Detail::Detailed] {
        let view = View {
            detail,
            scale: 100.0,
        };
        assert_eq!(
            built(&store, &list, Some(view)),
            built(&store_at(&list, view), &list, None),
            "{detail:?}"
        );
    }
    // The store's own view asked for is its plain build; detailed draws the layers' lines and the brick's hatch the
    // schematic store does not.
    assert_eq!(built(&store, &list, Some(own)), plain);
    let detailed = built(
        &store,
        &list,
        Some(View {
            detail: Detail::Detailed,
            scale: 100.0,
        }),
    );
    assert!(detailed.1.len() > plain.1.len());
    assert_eq!(store.wall_view(), own);
}
