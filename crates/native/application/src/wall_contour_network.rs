//! Product policy around the shared analytic boundary network.
use super::{invalid, ring};
use crate::checks::Stop;
use kentos_commands::{CommandWarning, Vec2, WallFields, WallJustify};
use kentos_geometry_core::geom::{
    arrangement::TOL,
    wall_network::{self, Problem},
};

pub(crate) fn merge(
    walls: Vec<(usize, String, WallFields)>,
    warnings: &mut Vec<CommandWarning>,
) -> Result<Vec<(usize, String, WallFields)>, Stop> {
    let mut groups = std::collections::BTreeMap::<String, Vec<usize>>::new();
    for (i, (_, layer, _)) in walls.iter().enumerate() {
        groups.entry(layer.clone()).or_default().push(i);
    }
    let mut output = Vec::new();
    for indices in groups.values() {
        let rings: Vec<_> = indices.iter().map(|&i| ring(&walls[i].2)).collect();
        let network=wall_network::build(&rings).map_err(|problem|invalid(match problem {
            Problem::Budget=>"Ortak kontur ağı bu işlemin bellek sınırını aşıyor; kaynakları daha küçük gruplar halinde seçin. Hiçbir duvar yazılmadı.",
            Problem::TinyEdge=>"Ortak kontur kümesinde 0.000001 m veya daha kısa bir kenar var; sessizce atılmadı. Kaynağı düzeltin; hiçbir duvar yazılmadı.",
            Problem::Crossing=>"Oda konturları sınırlarında kesişiyor; ortak kenar yerine çakışan odalar var. Hiçbir duvar yazılmadı.",
            Problem::Contact=>"Konturlar yalnız noktada değiyor veya ortak kenar dışında kesişiyor; bu sınırlar tek duvar olarak eşlenemedi. Hiçbir duvar yazılmadı.",
            Problem::OverlappingRooms=>"Aynı sınırın aynı yanında yinelenen veya üst üste odalar var; ortak duvar iki farklı oda tarafını ayırmalı. Hiçbir duvar yazılmadı.",
            Problem::AmbiguousCurve=>"Konturların analitik yayları eşlenemedi; yaklaşık bir ortak duvar üretilmedi. Hiçbir duvar yazılmadı.",
        },"uids"))?;
        let Some(network) = network else {
            output.extend(indices.iter().map(|&i| walls[i].clone()));
            continue;
        };
        for (s, &affected) in network.affected.iter().enumerate() {
            if !affected {
                continue;
            }
            let w = &walls[indices[s]].2;
            if w.justify != WallJustify::Center || w.offset != 0. {
                return Err(invalid(
                    "Ortak kenarlı odalarda Eksen referansını seçin. Aynı sınır iki odanın birden iç net/dış ölçü yüzü olamaz; hiçbir duvar yazılmadı.",
                    "justify",
                ));
            }
            if w.components.iter().ne(w.components.iter().rev()) {
                return Err(invalid(
                    "Ortak kenarın iki yönündeki katmanlı duvar bileşenleri birbirinin aynası değil. Uyumlu simetrik stil seçin; hiçbir duvar yazılmadı.",
                    "style",
                ));
            }
        }
        for (s, &i) in indices.iter().enumerate() {
            if !network.affected[s] {
                output.push(walls[i].clone());
                continue;
            }
            for run in network.runs.iter().filter(|r| r.source == s) {
                let mut w = walls[i].2.clone();
                w.pts = run.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect();
                w.bulges = run.bulges.clone();
                w.closed = run.closed;
                crate::walls::path_numbers(&w.pts, w.bulges.as_deref(), w.closed, "uids")?;
                if !w.closed
                    && let Some(bulges) = &mut w.bulges
                {
                    bulges.push(0.0);
                }
                crate::walls::valid(&w, "uids")?;
                crate::walls::drawable(&w, "uids")?;
                output.push((walls[i].0, walls[i].1.clone(), w));
            }
        }
        warnings.push(CommandWarning{code:crate::codes::WALL_SHARED_BOUNDARY.into(),message:format!(
            "{} ortak kenar parçası ({:.6} m) birer kez üretildi. Eşleme toleransı {:.6} m; ortak parçanın öznitelik ve görünümü seçim sırasındaki ilk kaynaktan alınır.",network.shared_pieces,network.shared_length,TOL),path:Some("uids".into())});
    }
    if output.len() > kentos_commands::MAX_WALLS {
        return Err(invalid(
            "Kontur ağı en çok 10000 duvar üretebilir; hiçbir duvar yazılmadı.",
            "uids",
        ));
    }
    Ok(output)
}
