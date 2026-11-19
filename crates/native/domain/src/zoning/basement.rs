//! 5/8'in bodrum listesi (docs/adr/8148): what Planlı Alanlar İmar Yönetmeliği 5/8 lets out of katlar alanı apart from
//! the 30 % in a building's basements ("binaların bodrum katlarında yapılan; … bu hesaba dâhil edilmeksizin emsal
//! haricidir") — the common areas of a storey wholly under the ground (d), the shelter's least area (b), a common
//! masjid's 150 m² (300 m² where the building holds no dwelling, ç), the common children's play and care rooms' 100 m²
//! (e), the car parks up to twice the mandatory car parks' area (a). The rooms are [`super::Room`]s counted out (22/1),
//! each with its part within the 30 % (its contour less a 22/1 limit's excess).

use kentos_contracts::{ArchLevel, ArchSettings, BuildingProgram, EmsalCount, UnitKind};

use super::Room;

/// A bent of 5/8's basement list (docs/adr/8148).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasementBent {
    /// d: "Bütün cepheleri tamamen gömülü olmak ve ortak alan niteliğinde olmak kaydıyla; otopark alanları ve 22 nci
    /// maddede belirtilen tamamen gömülü ortak alanlar": a common area's room counted out on a storey under the ground,
    /// all of it.
    Buried,
    /// b: the shelter's least area ("Sığınak … ilgili mevzuat, standart ya da bu Yönetmeliğe göre hesap edilen asgari
    /// alanları"): what 22/1-g leaves out of katlar alanı, the building's needed shelter area (Sığınak Yönetmeliği 8/a,
    /// docs/adr/8149), all of it; none when the need is not known. The other rooms of b (stairs, shafts, plant rooms …)
    /// have no least area KentOS computes: they stay within the 30 %.
    Shelter,
    /// ç: "Ortak alan niteliğindeki mescit ve müştemilatın konutlarda 150 m²'si, konut dışı yapılarda 300 m²'si".
    Masjid,
    /// e: "Ticari amaç içermeyen, ortak alan niteliğindeki çocuk oyun alanlarının ve çocuk bakım ünitelerinin toplam
    /// 100 m²'si".
    Children,
    /// a: "Zorunlu otopark alanlarının 2 katı".
    Parking,
}

impl BasementBent {
    /// Its name in the facts.
    pub fn name(self) -> &'static str {
        match self {
            BasementBent::Buried => "buried",
            BasementBent::Shelter => "shelter",
            BasementBent::Masjid => "masjid",
            BasementBent::Children => "children",
            BasementBent::Parking => "parking",
        }
    }
}

/// A bent's rooms (docs/adr/8148): the bent, its rooms by persistent id in the drawing's order, their part within the
/// 30 % before it, what the bent lets apart from the 30 % and what it moved there (m²).
#[derive(Clone, Debug, PartialEq)]
pub struct BasementApart {
    pub bent: BasementBent,
    pub rooms: Vec<String>,
    pub area: f64,
    pub allowed: f64,
    pub apart: f64,
}

/// How many times the mandatory car parks' area the basements' car parks take apart from the 30 % (PAİY 5/8-a, as
/// amended RG 31/12/2022-32060: "Zorunlu otopark alanlarının 2 katı").
pub const PARKING_TIMES: f64 = 2.0;

/// A common masjid's area apart from the 30 % in a building with dwellings, m² (PAİY 5/8-ç: "konutlarda 150 m²'si").
pub const MASJID_DWELLINGS: f64 = 150.0;

/// And in a building of no dwelling (5/8-ç: "konut dışı yapılarda 300 m²'si").
pub const MASJID_OTHERS: f64 = 300.0;

/// The common children's play and care rooms' area apart from the 30 %, m² (PAİY 5/8-e: "toplam 100 m²'si").
pub const CHILDREN_AREA: f64 = 100.0;

/// What the bents read of the building beside its rooms (docs/adr/8148): the mandatory car parks' area (the places
/// Ek-1 asks, docs/adr/8036, times Otopark Yönetmeliği 4/1-ç's 20 m² a car with its manoeuvre,
/// [`crate::provisions::PARKING_UNIT`]: "en az 20 m2", so its least) and whether a unit of it is a dwelling.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Needs {
    pub parking: f64,
    pub dwellings: bool,
}

/// The basements, by their ids (PAİY 4/1-o: "Bodrum kat: Zemin katın altındaki katları"): a storey lower than its
/// building's ground storey (the program's, docs/adr/8072 §4), or, of no building, than the project's (the storey named
/// Zemin, else the one nearest ±0.00).
pub(super) fn basement_levels(arch: &ArchSettings) -> Vec<String> {
    let levels: &[ArchLevel] = &arch.levels;
    let project = levels
        .iter()
        .find(|l| kentos_contracts::is_ground_name(&l.name))
        .or_else(|| levels.iter().min_by(|a, b| a.elevation.abs().total_cmp(&b.elevation.abs())));
    levels
        .iter()
        .filter(|l| {
            let ground = arch
                .program
                .building_of_level(&l.id)
                .and_then(|b| arch.program.ground_of(&b.id, levels))
                .and_then(|g| levels.iter().find(|x| x.id == g))
                .or(project);
            ground.is_some_and(|g| l.elevation < g.elevation - 1e-6)
        })
        .map(|l| l.id.clone())
        .collect()
}

/// 5/8's basement list on the rooms counted out: each room's part moved apart from the 30 %, the bent it is under, and
/// each bent's rooms. `within` is each room's part within the 30 % (0 for a room not counted out); `basement` the
/// basements, `buried` the storeys under the ground; `shelter_known` whether 22/1-g knew the needed shelter area (the
/// shelter's part within the 30 % is then that area); `needs` is asked only when a or ç reads a room.
///
/// A room is under the first bent that reads it, in the order d, b, ç, e, a; a capped bent's part is shared among its
/// rooms by their areas. The car parks d takes apart count against a's twice (docs/adr/8148 K380).
pub(super) fn apply(
    rooms: &[Room],
    within: &[f64],
    uids: &[String],
    program: &BuildingProgram,
    basement: &[String],
    buried: &[String],
    shelter_known: bool,
    needs: impl FnOnce() -> Needs,
) -> (Vec<f64>, Vec<Option<BasementBent>>, Vec<BasementApart>) {
    use BasementBent::{Buried, Children, Masjid, Parking, Shelter};
    let n = rooms.len();
    let mut apart = vec![0.0; n];
    let mut bent_of: Vec<Option<BasementBent>> = vec![None; n];
    let mut out = Vec::new();
    let on = |i: usize, levels: &[String]| rooms[i].level.as_ref().is_some_and(|l| levels.contains(l));
    let common = |i: usize| {
        rooms[i].unit.as_deref().and_then(|u| program.unit(u)).is_some_and(|g| g.kind == UnitKind::Common)
    };
    let open = |i: usize| !rooms[i].void && rooms[i].count == EmsalCount::Exempt && within[i] > 0.0;
    let reads = |bent: BasementBent, i: usize| match bent {
        Buried => on(i, buried) && common(i),
        Shelter => on(i, basement) && rooms[i].shelter,
        Masjid => on(i, basement) && rooms[i].masjid && common(i),
        Children => on(i, basement) && rooms[i].children && common(i),
        Parking => on(i, basement) && rooms[i].parking,
    };
    let asks = [Masjid, Parking].into_iter().any(|b| (0..n).any(|i| open(i) && !reads(Buried, i) && reads(b, i)));
    let needs = if asks { needs() } else { Needs::default() };
    let mut buried_parking = 0.0;
    for bent in [Buried, Shelter, Masjid, Children, Parking] {
        let members: Vec<usize> = (0..n).filter(|&i| bent_of[i].is_none() && open(i) && reads(bent, i)).collect();
        if members.is_empty() {
            continue;
        }
        let area: f64 = members.iter().map(|&i| within[i]).sum();
        let allowed = match bent {
            Buried => area,
            Shelter if shelter_known => area,
            Shelter => 0.0,
            Masjid if needs.dwellings => MASJID_DWELLINGS,
            Masjid => MASJID_OTHERS,
            Children => CHILDREN_AREA,
            Parking => (PARKING_TIMES * needs.parking - buried_parking).max(0.0),
        };
        let moved = area.min(allowed);
        for &i in &members {
            bent_of[i] = Some(bent);
            apart[i] = within[i] * moved / area;
        }
        if bent == Buried {
            buried_parking = members.iter().filter(|&&i| rooms[i].parking).map(|&i| within[i]).sum();
        }
        out.push(BasementApart {
            bent,
            rooms: members.iter().map(|&i| uids[i].clone()).collect(),
            area,
            allowed,
            apart: moved,
        });
    }
    (apart, bent_of, out)
}
