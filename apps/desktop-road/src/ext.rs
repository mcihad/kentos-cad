//! The Yol module's `App` methods, every file's extension trait (docs/adr/9199 §3.5): a file of the module, its parts
//! and its tests take them all with `use crate::road_ext::*;`. A file's methods stay its own; this only names them.

pub(crate) use crate::alignment_window::AlignmentWindowApp;
pub(crate) use crate::corridor_window::CorridorWindowApp;
pub(crate) use crate::culvert_window::CulvertWindowApp;
pub(crate) use crate::design_check_window::DesignCheckWindowApp;
pub(crate) use crate::guardrail_window::GuardrailWindowApp;
pub(crate) use crate::haul_fleet_window::HaulFleetWindowApp;
pub(crate) use crate::haul_plan_window::HaulPlanWindowApp;
pub(crate) use crate::haul_window::HaulWindowApp;
pub(crate) use crate::junction_window::JunctionWindowApp;
pub(crate) use crate::pay_window::PayWindowApp;
pub(crate) use crate::platform_water_window::PlatformWaterWindowApp;
pub(crate) use crate::profile_lengths_window::ProfileLengthsWindowApp;
pub(crate) use crate::profile_window::ProfileWindowApp;
pub(crate) use crate::retaining_wall_window::RetainingWallWindowApp;
pub(crate) use crate::road::StateApp;
pub(crate) use crate::road_alignment_fit_window::RoadAlignmentFitWindowApp;
pub(crate) use crate::road_exchange::RoadExchangeApp1;
pub(crate) use crate::road_exchange::RoadExchangeApp2;
pub(crate) use crate::road_exchange::RoadExchangeApp3;
pub(crate) use crate::road_profile_fit_window::RoadProfileFitWindowApp;
pub(crate) use crate::road_sheets_window::RoadSheetsWindowApp;
pub(crate) use crate::road_views_window::RoadViewsWindowApp;
pub(crate) use crate::route_window::RouteWindowApp;
pub(crate) use crate::section_window::SectionWindowApp;
pub(crate) use crate::section_window::editors::EditorsApp;
pub(crate) use crate::sight_window::SightWindowApp;
pub(crate) use crate::stakeout_window::StakeoutWindowApp;
pub(crate) use crate::superelevation_window::SuperelevationWindowApp;
pub(crate) use crate::swept_window::SweptWindowApp;
pub(crate) use crate::transition_window::TransitionWindowApp;
pub(crate) use crate::volume_window::VolumeWindowApp;
pub(crate) use crate::widening_window::WideningWindowApp;
