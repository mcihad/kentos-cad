//! The command registry's part of mapping, cadastre and survey commands (docs/adr/8054 §9): `map_commands!` calls `crate::map_dispatch::map_arm!($command, $then, module, InputType)`
//! for one of them, else gives `$else`. The registry (core/registry.rs) chains the parts; the handlers' names are
//! resolved where the registry calls it.

macro_rules! map_commands {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::MAP_WORK_QUERY => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_work,
                    kentos_commands::MapWorkQuery
                )
            }
            kentos_commands::MAP_WORK_EDIT => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_work_edit,
                    kentos_commands::MapWorkEdit
                )
            }
            kentos_commands::MAP_WORK_RESTORE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_work_restore,
                    kentos_commands::MapWorkRestore
                )
            }
            kentos_commands::CAD_CRS_FACTORS => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    crs_factors,
                    kentos_commands::CrsFactors
                )
            }
            kentos_commands::MAP_FEATURES_CODE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_features,
                    kentos_commands::FeaturesCode
                )
            }
            kentos_commands::CAD_SURVEY_BASELINES => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    survey_baselines,
                    kentos_commands::SurveyBaselines
                )
            }
            kentos_commands::CAD_PARCELS_CHECK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    parcels_check,
                    kentos_commands::ParcelsCheck
                )
            }
            kentos_commands::CAD_PARCELS_BOUNDARIES => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    parcels_boundaries,
                    kentos_commands::ParcelsBoundaries
                )
            }
            kentos_commands::CAD_PARCELS_RECORD => crate::map_dispatch::map_arm!(
                $command,
                $then,
                parcel_register,
                kentos_commands::ParcelsRecord
            ),
            kentos_commands::CAD_PARCELS_RECORDS => crate::map_dispatch::map_arm!(
                $command,
                $then,
                parcel_register_list,
                kentos_commands::ParcelsRecords
            ),
            kentos_commands::CAD_PARCELS_APPROVE => crate::map_dispatch::map_arm!(
                $command,
                $then,
                parcel_approve,
                kentos_commands::ParcelsApprove
            ),
            kentos_commands::CAD_ASBUILT_REVIEW => crate::map_dispatch::map_arm!(
                $command,
                $then,
                asbuilt_review,
                kentos_commands::AsbuiltReview
            ),
            kentos_commands::CAD_PARCELS_WITHDRAW => crate::map_dispatch::map_arm!(
                $command,
                $then,
                parcel_withdraw,
                kentos_commands::ParcelsWithdraw
            ),
            kentos_commands::CAD_PIPES_CHECK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pipes_check,
                    kentos_commands::PipesCheck
                )
            }
            kentos_commands::CAD_PIPES_PROFILE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pipes_profile,
                    kentos_commands::PipesProfile
                )
            }
            kentos_commands::CAD_ASBUILT_CHECK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    asbuilt_check,
                    kentos_commands::AsbuiltCheck
                )
            }
            kentos_commands::CAD_FIELD_PACKAGE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    field_package,
                    kentos_commands::FieldPackage
                )
            }
            kentos_commands::CAD_ZONING_CHECK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    zoning_check,
                    kentos_commands::ZoningCheck
                )
            }
            kentos_commands::CAD_ZONING_SYMBOLIZE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    zoning_symbolize,
                    kentos_commands::ZoningSymbolize
                )
            }
            kentos_commands::CAD_ZONING_DRAFT => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    zoning_draft,
                    kentos_commands::ZoningDraft
                )
            }
            kentos_commands::CAD_READJUSTMENT_DISTRIBUTE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    readjustment,
                    kentos_commands::ReadjustmentDistribute
                )
            }
            kentos_commands::CAD_READJUSTMENT_SCOPE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    readjustment_scope,
                    kentos_commands::ReadjustmentScope
                )
            }
            kentos_commands::CAD_READJUSTMENT_SHEETS => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    readjustment_sheets,
                    kentos_commands::ReadjustmentSheets
                )
            }
            kentos_commands::CAD_ORTHOPHOTO_CHECK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    orthophoto_check,
                    kentos_commands::OrthophotoCheck
                )
            }
            kentos_commands::CAD_ORTHOPHOTO_COMPARE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    orthophoto_compare,
                    kentos_commands::OrthophotoCompare
                )
            }
            kentos_commands::CAD_ORTHOPHOTO_SEAMLINES => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    orthophoto_seamlines,
                    kentos_commands::OrthophotoSeamlines
                )
            }
            kentos_commands::CAD_ORTHOPHOTO_NETWORK => crate::map_dispatch::map_arm!(
                $command,
                $then,
                orthophoto_network,
                kentos_commands::OrthophotoNetwork
            ),
            kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT => crate::map_dispatch::map_arm!(
                $command,
                $then,
                orthophoto_footprint,
                kentos_commands::OrthophotoFootprint
            ),
            kentos_commands::CAD_POINTCLOUDS_OVERLAP => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointcloud_overlap,
                    kentos_commands::PointCloudsOverlap
                )
            }
            kentos_commands::CAD_POINTCLOUDS_REGISTER => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointcloud_register,
                    kentos_commands::PointCloudsRegister
                )
            }
            kentos_commands::CAD_POINTCLOUDS_NETWORK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointcloud_network,
                    kentos_commands::PointCloudsNetwork
                )
            }
            kentos_commands::CAD_POINTCLOUDS_PLACE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointcloud_place,
                    kentos_commands::PointCloudsPlace
                )
            }
            kentos_commands::MAP_SURVEY_QUERY => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_survey,
                    kentos_commands::MapSurveyQuery
                )
            }
            kentos_commands::MAP_SURVEY_EDIT => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_survey_edit,
                    kentos_commands::MapSurveyEdit
                )
            }
            kentos_commands::MAP_SURVEY_NETWORK => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_survey_network,
                    kentos_commands::MapSurveyNetwork
                )
            }
            kentos_commands::MAP_FEATURES_RECONCILE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    map_features_reconcile,
                    kentos_commands::FeaturesReconcile
                )
            }
            kentos_commands::CAD_PARCELS_CREATE => crate::map_dispatch::map_arm!(
                $command,
                $then,
                parcels,
                kentos_commands::ParcelsCreate
            ),
            kentos_commands::CAD_SURVEY_POINTS => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    survey_points,
                    kentos_commands::SurveyPoints
                )
            }
            kentos_commands::CAD_SURVEY_ADJUST => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    survey_adjust,
                    kentos_commands::SurveyAdjust
                )
            }
            kentos_commands::CAD_POINTCLOUDS_DESCRIBE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointclouds_describe,
                    PointCloudsDescribe
                )
            }
            kentos_commands::CAD_POINTCLOUDS_QUERY => {
                crate::map_dispatch::map_arm!($command, $then, pointclouds_query, PointCloudsQuery)
            }
            kentos_commands::CAD_POINTCLOUDS_REGION => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointclouds_region,
                    PointCloudsRegion
                )
            }
            kentos_commands::CAD_POINTCLOUDS_PROFILE => {
                crate::map_dispatch::map_arm!(
                    $command,
                    $then,
                    pointclouds_profile,
                    PointCloudsProfile
                )
            }
            _ => $else,
        }
    };
}
pub(crate) use map_commands;

/// One arm: the handler with `mod-map` (docs/adr/8054 §11), else the answer that this build has no harita commands.
/// The handler's module is never named in a build without it.
#[cfg(feature = "mod-map")]
macro_rules! map_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        $then!($module, $input)
    };
}
#[cfg(not(feature = "mod-map"))]
macro_rules! map_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        Err(crate::registry::not_built($command, "harita"))
    };
}
pub(crate) use map_arm;
