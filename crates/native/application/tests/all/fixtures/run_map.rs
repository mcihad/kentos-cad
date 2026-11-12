//! The fixture runner's map commands (docs/adr/8054 §9): `map_cases!` calls `$then!(module, InputType)` for one
//! of them, else gives `$else`; run_op chains the parts.

macro_rules! map_cases {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_PARCELS_CREATE => $then!(parcels, kentos_commands::ParcelsCreate),
            kentos_commands::CAD_SURVEY_POINTS => $then!(survey_points, kentos_commands::SurveyPoints),
            kentos_commands::CAD_SURVEY_ADJUST => $then!(survey_adjust, kentos_commands::SurveyAdjust),
            kentos_commands::CAD_POINTCLOUDS_DESCRIBE => {
                $then!(pointclouds_describe, kentos_commands::PointCloudsDescribe)
            }
            kentos_commands::CAD_POINTCLOUDS_REGION => {
                $then!(pointclouds_region, kentos_commands::PointCloudsRegion)
            }
            kentos_commands::CAD_POINTCLOUDS_PROFILE => {
                $then!(pointclouds_profile, kentos_commands::PointCloudsProfile)
            }
            kentos_commands::CAD_POINTCLOUDS_QUERY => {
                $then!(pointclouds_query, kentos_commands::PointCloudsQuery)
            }
            kentos_commands::MAP_WORK_QUERY => {
                $then!(map_work, kentos_commands::MapWorkQuery)
            }
            kentos_commands::MAP_WORK_EDIT => {
                $then!(map_work_edit, kentos_commands::MapWorkEdit)
            }
            kentos_commands::MAP_WORK_RESTORE => {
                $then!(map_work_restore, kentos_commands::MapWorkRestore)
            }
            kentos_commands::CAD_CRS_FACTORS => {
                $then!(crs_factors, kentos_commands::CrsFactors)
            }
            kentos_commands::MAP_FEATURES_CODE => {
                $then!(map_features, kentos_commands::FeaturesCode)
            }
            kentos_commands::CAD_SURVEY_BASELINES => {
                $then!(survey_baselines, kentos_commands::SurveyBaselines)
            }
            kentos_commands::CAD_PARCELS_CHECK => {
                $then!(parcels_check, kentos_commands::ParcelsCheck)
            }
            kentos_commands::CAD_PARCELS_BOUNDARIES => {
                $then!(parcels_boundaries, kentos_commands::ParcelsBoundaries)
            }
            kentos_commands::CAD_PARCELS_RECORD => {
                $then!(parcel_register, kentos_commands::ParcelsRecord)
            }
            kentos_commands::CAD_PARCELS_RECORDS => {
                use kentos_native_application::parcel_register::list_cmd as parcel_register_list;
                $then!(parcel_register_list, kentos_commands::ParcelsRecords)
            }
            kentos_commands::CAD_PARCELS_APPROVE => {
                use kentos_native_application::parcel_drafts as parcel_approve;
                $then!(parcel_approve, kentos_commands::ParcelsApprove)
            }
            kentos_commands::CAD_ASBUILT_REVIEW => {
                use kentos_native_application::asbuilt_review;
                $then!(asbuilt_review, kentos_commands::AsbuiltReview)
            }
            kentos_commands::CAD_PARCELS_WITHDRAW => {
                use kentos_native_application::parcel_drafts::withdraw as parcel_withdraw;
                $then!(parcel_withdraw, kentos_commands::ParcelsWithdraw)
            }
            kentos_commands::CAD_PIPES_CHECK => {
                $then!(pipes_check, kentos_commands::PipesCheck)
            }
            kentos_commands::CAD_PIPES_PROFILE => {
                $then!(pipes_profile, kentos_commands::PipesProfile)
            }
            kentos_commands::CAD_ASBUILT_CHECK => {
                $then!(asbuilt_check, kentos_commands::AsbuiltCheck)
            }
            kentos_commands::CAD_FIELD_PACKAGE => {
                $then!(field_package, kentos_commands::FieldPackage)
            }
            kentos_commands::CAD_ZONING_CHECK => {
                $then!(zoning_check, kentos_commands::ZoningCheck)
            }
            kentos_commands::CAD_ZONING_SYMBOLIZE => {
                $then!(zoning_symbolize, kentos_commands::ZoningSymbolize)
            }
            kentos_commands::CAD_ZONING_DRAFT => {
                $then!(zoning_draft, kentos_commands::ZoningDraft)
            }
            kentos_commands::CAD_READJUSTMENT_DISTRIBUTE => {
                $then!(readjustment, kentos_commands::ReadjustmentDistribute)
            }
            kentos_commands::CAD_READJUSTMENT_SCOPE => {
                $then!(readjustment_scope, kentos_commands::ReadjustmentScope)
            }
            kentos_commands::CAD_READJUSTMENT_SHEETS => {
                $then!(readjustment_sheets, kentos_commands::ReadjustmentSheets)
            }
            kentos_commands::CAD_ORTHOPHOTO_CHECK => {
                $then!(orthophoto_check, kentos_commands::OrthophotoCheck)
            }
            kentos_commands::CAD_ORTHOPHOTO_COMPARE => {
                $then!(orthophoto_compare, kentos_commands::OrthophotoCompare)
            }
            kentos_commands::CAD_ORTHOPHOTO_SEAMLINES => {
                $then!(orthophoto_seamlines, kentos_commands::OrthophotoSeamlines)
            }
            kentos_commands::CAD_ORTHOPHOTO_NETWORK => {
                use kentos_native_application::orthophoto_network::network as orthophoto_network;
                $then!(orthophoto_network, kentos_commands::OrthophotoNetwork)
            }
            kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT => {
                use kentos_native_application::orthophoto_network::outline as orthophoto_footprint;
                $then!(orthophoto_footprint, kentos_commands::OrthophotoFootprint)
            }
            kentos_commands::CAD_POINTCLOUDS_OVERLAP => {
                $then!(pointcloud_overlap, kentos_commands::PointCloudsOverlap)
            }
            kentos_commands::CAD_POINTCLOUDS_REGISTER => {
                $then!(pointcloud_register, kentos_commands::PointCloudsRegister)
            }
            kentos_commands::CAD_POINTCLOUDS_NETWORK => {
                $then!(pointcloud_network, kentos_commands::PointCloudsNetwork)
            }
            kentos_commands::CAD_POINTCLOUDS_PLACE => {
                $then!(pointcloud_place, kentos_commands::PointCloudsPlace)
            }
            kentos_commands::MAP_SURVEY_QUERY => {
                $then!(map_survey, kentos_commands::MapSurveyQuery)
            }
            kentos_commands::MAP_SURVEY_EDIT => {
                $then!(map_survey_edit, kentos_commands::MapSurveyEdit)
            }
            kentos_commands::MAP_SURVEY_NETWORK => {
                $then!(map_survey_network, kentos_commands::MapSurveyNetwork)
            }
            kentos_commands::MAP_FEATURES_RECONCILE => {
                $then!(map_features_reconcile, kentos_commands::FeaturesReconcile)
            }
            _ => $else,
        }
    };
}
pub(super) use map_cases;
