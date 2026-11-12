//! Which document schema a payload needs (docs/specs/kcad-v2.md): the newest of the features it holds, checked
//! from the newest down, with the predicates the reader shares.

use super::*;

/// The oldest schema that holds the drawing: 39 when an object of it or of a
/// block definition has a vertex measure (docs/adr/0224), 35 when a layer has a filter
/// (docs/adr/0211), 34 when a layer has a time
/// setting, a group is a scenario or a layer stands for a base layer
/// (docs/adr/0210), 33 when the project has a
/// network (docs/adr/0209), 32 when a layer is drawn from a
/// service or has a feed, or the project has connections (docs/adr/0208), 31
/// when it has a point cloud or a raster read from an address
/// (docs/adr/0207), 30 when the project has
/// annotation heights or a dimension style lines, or a dimension of it or of
/// a block definition has lines or a leader an arrowhead size or one of
/// the DXF arrowheads (docs/adr/0205), 29 when it has a raster
/// (docs/adr/0204 §2), 28 when its survey settings
/// name an a priori standard deviation (docs/adr/0203 §1), 27 when the
/// project has topology settings (docs/adr/0202 §7), 26 when a layer has fields
/// (docs/adr/0199 §1), 25 when a text of it or of a block definition has a
/// curve (docs/adr/0196), 24 when it has a picture (docs/adr/0192), 23 when a
/// hatch has a pattern, a gradient or a tie (docs/adr/0186), 22 when it has a table
/// (docs/adr/0184), 21 when the project has a text
/// or a dimension style or a text or a dimension of it or of a block
/// definition a face or a look (docs/adr/0183), 20 when a text of it or of a
/// block definition has a box, a line spacing or letter formats
/// (docs/adr/0182 §1), 19 when the project has layer
/// states (docs/adr/0177 §4), 18 when a text of it writes an
/// object's label (`labelOf`, `labelScale`), 17 when a polyline or a point,
/// of it or of a block definition, has parts, 16 when its survey settings
/// name the ground, 15 when they name a traverse tolerance, 14 when the project has survey settings, 13 when it has its own systems or datum choices, 12 when it has
/// a second system, 11 when it names a drawing unit, 10 when a layer has its own
/// snapping, 9 when a dimension of it or of
/// a block definition has one of schema 9's kinds or fields, 8 when it or a
/// block definition has a leader, 7 when a text or an attribute definition has an
/// alignment, a width factor or a mask, 6 when it has block definitions, 5
/// when an area has parts, 4 when an object has a vertex elevation, 3 when
/// one has its own line weight, else 2. A drawing without any stays as it
/// always was, byte for byte. (An insert needs a definition: one without is
/// refused before the schema is written.)
pub(super) fn schema_of(doc: &DocumentSnapshotV2, objs: Objs<'_>) -> u32 {
    // Schema 253 (docs/adr/5220): the monitoring reviews the project keeps.
    if !doc.settings.monitoring_reviews.is_empty() {
        return crate::SCHEMA_WITH_MONITORING_REVIEWS;
    }
    // Schema 252 (docs/adr/5200): the adjustments the project keeps.
    if !doc.settings.survey_adjustments.is_empty() {
        return crate::SCHEMA_WITH_KEPT_ADJUSTMENTS;
    }
    // Schema 251 (docs/adr/5100): a point cloud file's place in the drawing.
    if objs.iter().any(|e| matches!(e, Entity::PointCloud(c) if c.cloud.placed())) {
        return crate::SCHEMA_WITH_CLOUD_POSES;
    }
    // Schema 250 (docs/adr/5040): a zoning plan's kept symbol choices and plan change drafts.
    if doc.settings.zoning_plan.is_some() {
        return crate::SCHEMA_WITH_ZONING_DRAFTS;
    }
    // Schema 249 (docs/adr/5020): the parcel register's stated lengths, drafts or an approval.
    if doc.settings.parcel_register.as_ref().is_some_and(kentos_contracts::ParcelRegister::has_drafts) {
        return crate::SCHEMA_WITH_PARCEL_DRAFTS;
    }
    // Schema 248 (docs/adr/4920): a GNSS archive's baseline sessions, kept limits, a network point's velocity.
    if doc.settings.map_survey.as_ref().is_some_and(kentos_contracts::MapSurvey::has_sessions) {
        return crate::SCHEMA_WITH_GNSS_SESSIONS;
    }
    // Schema 247 (docs/adr/4880): a map work item's typed layers' profile.
    if doc.settings.map_work.as_ref().is_some_and(kentos_contracts::MapWork::has_profiles) {
        return crate::SCHEMA_WITH_WORK_PROFILES;
    }
    // Schema 246 (docs/adr/4740): a sheet's contour lines kept as a drawing, a layer as a drawing's source, drawing
    // again whole.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_sheet_contour_drawings) {
        return crate::SCHEMA_WITH_SHEET_CONTOUR_DRAWINGS;
    }
    // Schema 245 (docs/adr/8050): a room module's columns, stairs, ramps, slabs, room separators and kitchen runs.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(|a| kentos_contracts::modules_have_scope(&a.modules))
    {
        return crate::SCHEMA_WITH_MODULE_SCOPE;
    }
    // Schema 244 (docs/adr/8049 §3): a constraint naming a column's side.
    if doc
        .constraints
        .constraints
        .iter()
        .any(|c| kentos_contracts::rule_names_column_side(&c.rule))
    {
        return crate::SCHEMA_WITH_COLUMN_SIDE_BONDS;
    }
    // Schema 243 (docs/adr/8049 §2): a constraint naming a grid's line.
    if doc
        .constraints
        .constraints
        .iter()
        .any(|c| kentos_contracts::rule_names_grid(&c.rule))
    {
        return crate::SCHEMA_WITH_GRID_BONDS;
    }
    // Schema 242 (docs/adr/8045 §3): a railing.
    if objs.iter().any(|e| matches!(e, Entity::Railing(_))) {
        return crate::SCHEMA_WITH_RAILING_OBJECTS;
    }
    // Schema 241 (docs/adr/8045 §1): a balanced stair's winders.
    if objs.iter().any(kentos_contracts::needs_stair_winders) {
        return crate::SCHEMA_WITH_STAIR_WINDERS;
    }
    // Schema 240 (docs/adr/8044 §8): a roof's downpipes.
    if objs.iter().any(kentos_contracts::needs_roof_downpipes) {
        return crate::SCHEMA_WITH_ROOF_DOWNPIPES;
    }
    // Schema 239 (docs/adr/8044 §8): a roof's gutter and snow guard.
    if objs.iter().any(kentos_contracts::needs_roof_edges) {
        return crate::SCHEMA_WITH_ROOF_EDGES;
    }
    // Schema 238 (docs/adr/8044 §6): a roof's chimneys, vents and windows.
    if objs.iter().any(kentos_contracts::needs_roof_parts) {
        return crate::SCHEMA_WITH_ROOF_PARTS;
    }
    // Schema 237 (docs/adr/8044 §1): a roof.
    if objs.iter().any(|e| matches!(e, Entity::Roof(_))) {
        return crate::SCHEMA_WITH_ROOFS;
    }
    // Schema 236 (docs/adr/8041 §3): a beam's role or its base over its storey's floor.
    if objs.iter().any(kentos_contracts::needs_beam_roles) {
        return crate::SCHEMA_WITH_BEAM_ROLES;
    }
    // Schema 235 (docs/adr/8041 §2): a column's material.
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Column(c) if c.column.material.is_some()))
    {
        return crate::SCHEMA_WITH_COLUMN_MATERIAL;
    }
    // Schema 234 (docs/adr/8041 §1): a beam.
    if objs.iter().any(|e| matches!(e, Entity::Beam(_))) {
        return crate::SCHEMA_WITH_BEAMS;
    }
    // Schema 233 (docs/adr/8040 §3): a room's floor offset or asma tavan.
    if objs.iter().any(kentos_contracts::needs_room_heights) {
        return crate::SCHEMA_WITH_ROOM_HEIGHTS;
    }
    // Schema 232 (docs/adr/8040 §2): a slab's layers besides its structure.
    let slab_layers = |e: &Entity| matches!(e, Entity::Slab(s) if !s.slab.layers.is_empty());
    if objs.iter().any(slab_layers)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(slab_layers))
    {
        return crate::SCHEMA_WITH_SLAB_LAYERS;
    }
    // Schema 231 (docs/adr/8039 §5): the Tadilat projesi view, the project's or a named view's recipe's.
    let renovation = Some(kentos_contracts::PhaseView::Renovation);
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(|a| a.phase_view == renovation)
        || doc
            .settings
            .named_views
            .iter()
            .any(|v| v.recipe.as_ref().is_some_and(|r| r.phase == renovation))
    {
        return crate::SCHEMA_WITH_RENOVATION_VIEW;
    }
    // Schema 230 (docs/adr/8039): a door's or window's own phase.
    let opening_phase = |e: &Entity| matches!(e, Entity::Opening(o) if o.opening.phase.is_some());
    if objs.iter().any(opening_phase)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(opening_phase))
    {
        return crate::SCHEMA_WITH_OPENING_PHASES;
    }
    // Schema 229 (docs/adr/8038): a door or window type's frame and leaves and its U value.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_opening_specs)
    {
        return crate::SCHEMA_WITH_OPENING_SPECS;
    }
    // Schema 228 (docs/adr/8038): the room list's finishes and columns.
    if objs.iter().any(kentos_contracts::needs_room_list)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(kentos_contracts::needs_room_list))
    {
        return crate::SCHEMA_WITH_ROOM_LIST;
    }
    // Schema 227 (docs/adr/8036): Otopark and Sığınak hesabı.
    let provision_table = |e: &Entity| {
        matches!(e, Entity::Table(t)
            if matches!(t.source, Some(kentos_contracts::TableSource::Parking {} | kentos_contracts::TableSource::Shelter {})))
    };
    if objs.iter().any(provision_table) || doc.blocks.iter().any(|b| b.entities.iter().any(provision_table)) {
        return crate::SCHEMA_WITH_PROVISION_TABLES;
    }
    // Schema 226 (docs/adr/8035): the building's sprinkler system.
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_sprinklers) {
        return crate::SCHEMA_WITH_SPRINKLERS;
    }
    // Schema 225 (docs/adr/8034): the municipality's regulation.
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_local_rules) {
        return crate::SCHEMA_WITH_LOCAL_RULES;
    }
    // Schema 224 (docs/adr/8032): the site plan.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_site)
    {
        return crate::SCHEMA_WITH_SITE_PLAN;
    }
    // Schema 223 (docs/adr/8031): kat irtifakı's program fields and schedule columns.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(|a| a.program.has_land_shares())
        || objs.iter().any(kentos_contracts::needs_land_shares)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(kentos_contracts::needs_land_shares))
    {
        return crate::SCHEMA_WITH_LAND_SHARES;
    }
    // Schema 222 (docs/adr/8030): the parcel's zoning, a brief's floor area ratio count, Emsal ve TAKS tablosu.
    let zoning_table = |e: &Entity| matches!(e, Entity::Table(t) if matches!(t.source, Some(kentos_contracts::TableSource::Zoning {})));
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(|a| a.has_zoning() || a.program.has_emsal())
        || objs.iter().any(zoning_table)
        || doc
            .blocks
            .iter()
            .flat_map(|b| &b.entities)
            .any(zoning_table)
    {
        return crate::SCHEMA_WITH_ZONING;
    }
    // Schema 221 (docs/adr/8029): a units schedule's regulation areas.
    if objs.iter().any(kentos_contracts::needs_paiy_areas)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(kentos_contracts::needs_paiy_areas))
    {
        return crate::SCHEMA_WITH_PAIY_AREAS;
    }
    // Schema 220 (docs/adr/8028): a type's joinery code, a tag naming it or an opening's size.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_opening_codes)
        || objs.iter().any(kentos_contracts::needs_opening_codes)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(kentos_contracts::needs_opening_codes))
    {
        return crate::SCHEMA_WITH_OPENING_CODES;
    }
    // Schema 219 (docs/adr/8027): a tag framed by an ellipse (a room's number, Mimarlar Odası's).
    if objs.iter().any(kentos_contracts::needs_ellipse_tags)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(kentos_contracts::needs_ellipse_tags))
    {
        return crate::SCHEMA_WITH_ELLIPSE_TAGS;
    }
    // Schema 218 (docs/adr/8013): how the project's plan lines are weighted.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_line_groups)
    {
        return crate::SCHEMA_WITH_LINE_GROUPS;
    }
    // Schema 217 (docs/adr/8012): the project's plan cut height.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_plan_cut)
    {
        return crate::SCHEMA_WITH_PLAN_CUT;
    }
    // Schema 216 (docs/adr/4220): the parcel register.
    if doc.settings.parcel_register.is_some() {
        return crate::SCHEMA_WITH_PARCEL_REGISTER;
    }
    // Schema 215 (docs/adr/4140): a review note recording a decision.
    if kentos_contracts::review_notes_have_decisions(&doc.settings.review_notes) {
        return crate::SCHEMA_WITH_REVIEW_DECISIONS;
    }
    // Schema 214 (docs/adr/3920): a terrain surface's edit that pastes another surface.
    if doc.settings.road.as_ref().is_some_and(road_has_pastes) {
        return crate::SCHEMA_WITH_SURFACE_PASTES;
    }
    // Schema 213 (docs/adr/3680): a map work item's approval with its objects' values.
    if doc.settings.map_work.as_ref().is_some_and(kentos_contracts::MapWork::has_approved_values) {
        return crate::SCHEMA_WITH_APPROVED_VALUES;
    }
    // Schema 212 (docs/adr/3560): a pipe run profile kept as a drawing, a network its source.
    if doc.settings.road.as_ref().is_some_and(|r| {
        r.drawings.iter().any(|d| d.command == "cad.pipes.profile" || d.sources.iter().any(|s| s.kind == kentos_contracts::RoadSourceKind::Network))
    }) {
        return crate::SCHEMA_WITH_PIPE_PROFILE_DRAWINGS;
    }
    // Schema 211 (docs/adr/3280): a map work item's approval with its objects' parts' digests.
    if doc.settings.map_work.as_ref().is_some_and(kentos_contracts::MapWork::has_approved_parts) {
        return crate::SCHEMA_WITH_APPROVED_PARTS;
    }
    // Schema 210 (docs/adr/2880): a map work item's approval with its objects' digests.
    if doc.settings.map_work.as_ref().is_some_and(kentos_contracts::MapWork::has_approved_objects) {
        return crate::SCHEMA_WITH_APPROVED_OBJECTS;
    }
    // Schema 209 (docs/adr/2840): a road drawing's record with the version that drew it or the revision its write made.
    if doc.settings.road.as_ref().is_some_and(|r| r.drawings.iter().any(|d| d.version.is_some() || d.revision.is_some())) {
        return crate::SCHEMA_WITH_DRAWING_PROVENANCE;
    }
    // Schema 208 (docs/adr/2580): a constraint naming a boundary edge, a map bond.
    if doc
        .constraints
        .constraints
        .iter()
        .any(|c| kentos_contracts::rule_is_map_bond(&c.rule))
    {
        return crate::SCHEMA_WITH_BOUNDARY_BONDS;
    }
    // Schema 207 (docs/adr/2520): a known point's own velocity.
    if doc.settings.map_survey.as_ref().is_some_and(kentos_contracts::MapSurvey::has_velocities) {
        return crate::SCHEMA_WITH_KNOWN_VELOCITIES;
    }
    // Schema 206 (docs/adr/2080): a map work item's approval stamp, a derivation from an item without a file.
    if doc.settings.map_work.as_ref().is_some_and(kentos_contracts::MapWork::has_stamps) {
        return crate::SCHEMA_WITH_WORK_STAMPS;
    }
    // Schema 205 (docs/adr/1840): two surfaces' cut and fill zones kept as a drawing.
    if doc.settings.road.as_ref().is_some_and(|r| r.drawings.iter().any(|d| d.command == "road.terrains.cutfill")) {
        return crate::SCHEMA_WITH_CUTFILL_DRAWINGS;
    }
    // Schema 204 (docs/adr/1041): a volume approval's sources and algorithm.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_approval_sources) {
        return crate::SCHEMA_WITH_ROAD_APPROVAL_SOURCES;
    }
    // Schema 203 (docs/adr/1032): a cloud record's heights as the call said them.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_cloud_conversions) {
        return crate::SCHEMA_WITH_ROAD_CLOUD_CONVERSIONS;
    }
    // Schema 202 (docs/adr/1029): a cloud record's vertical system.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_cloud_heights) {
        return crate::SCHEMA_WITH_ROAD_CLOUD_HEIGHTS;
    }
    // Schema 201 (docs/adr/1028): an alignment's places beyond its ends.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_places) {
        return crate::SCHEMA_WITH_ROAD_PLACES;
    }
    // Schema 200 (docs/adr/0985): an alignment's design speed or a PI's own.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_speed) {
        return crate::SCHEMA_WITH_ROAD_SPEED;
    }
    // Schema 199 (docs/adr/1740): a GNSS archive's baselines, known points, network adjustments and network points.
    if doc.settings.map_survey.as_ref().is_some_and(kentos_contracts::MapSurvey::has_network) {
        return crate::SCHEMA_WITH_GNSS_NETWORK;
    }
    // Schema 198 (docs/adr/1720): a map project's drawings, and its drawing them again by itself; a zero line's drawing in
    // any project.
    if doc.settings.road.as_ref().is_some_and(|r| r.drawings.iter().any(|d| d.command == "road.terrains.zeroline"))
        || doc.settings.cad_work == Some(kentos_contracts::CadWork::Map)
            && doc.settings.road.as_ref().is_some_and(|r| !r.drawings.is_empty() || r.auto_rebuild)
    {
        return crate::SCHEMA_WITH_MAP_DRAWINGS;
    }
    // Schema 197 (docs/adr/1580): a map project's graded pads.
    if doc.settings.cad_work == Some(kentos_contracts::CadWork::Map)
        && doc.settings.road.as_ref().is_some_and(|r| !r.platforms.is_empty())
    {
        return crate::SCHEMA_WITH_MAP_PLATFORMS;
    }
    // Schema 196 (docs/adr/1560): a terrain surface's edits.
    if doc.settings.road.as_ref().is_some_and(|r| r.surfaces.iter().any(|s| !s.edits.is_empty())) {
        return crate::SCHEMA_WITH_SURFACE_EDITS;
    }
    // Schema 195 (docs/adr/1520): a map project's terrain surfaces.
    if doc.settings.road.is_some() && doc.settings.cad_work == Some(kentos_contracts::CadWork::Map) {
        return crate::SCHEMA_WITH_MAP_SURFACES;
    }
    // Schema 194 (docs/adr/1340 §5): a map project's GNSS archive.
    if doc.settings.map_survey.is_some() {
        return crate::SCHEMA_WITH_MAP_SURVEY;
    }
    // Schema 193 (docs/adr/1201 §1): a map work item's file with its relative path.
    if doc.settings.map_work.as_ref().is_some_and(kentos_contracts::MapWork::has_relative) {
        return crate::SCHEMA_WITH_MAP_WORK_RELATIVE;
    }
    // Schema 192 (docs/adr/0979): an alignment PI's ove.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_ove) {
        return crate::SCHEMA_WITH_ROAD_OVE;
    }
    // Schema 191 (docs/adr/0977): a road project's grades as rise over run (a reader of 190 would take them for percent).
    if doc.settings.road.as_ref().is_some_and(kentos_contracts::RoadSettings::has_grades) {
        return crate::SCHEMA_WITH_ROAD_GRADES;
    }
    // Schema 190 (docs/adr/1129): standards findings the project accepts.
    if !doc.settings.standard_exceptions.is_empty() {
        return crate::SCHEMA_WITH_STANDARD_EXCEPTIONS;
    }
    // Schema 189 (docs/adr/1260): the topology quality profile, an exception's review fields.
    if doc.settings.topology.as_ref().is_some_and(|t| {
        t.profile.is_some()
            || t.exceptions.iter().any(|x| {
                x.decision.is_some()
                    || x.reason.is_some()
                    || x.rule_digest.is_some()
                    || x.scope.is_some()
            })
    }) {
        return crate::SCHEMA_WITH_TOPOLOGY_REVIEWS;
    }
    // Schema 188 (docs/adr/1200 §2): a map project's work structure.
    if doc.settings.map_work.is_some() {
        return crate::SCHEMA_WITH_MAP_WORK;
    }
    // Schema 187 (docs/adr/1025): a leader style that names the text style of its note.
    if doc.settings.leader_styles.iter().any(|s| s.text_style.is_some()) {
        return crate::SCHEMA_WITH_LEADER_TEXT_STYLES;
    }
    // Schema 186 (docs/adr/1023): a dimension style that names the text style of its value.
    if doc.settings.dimension_styles.iter().any(|s| s.text_style.is_some()) {
        return crate::SCHEMA_WITH_DIMENSION_TEXT_STYLES;
    }
    // Schema 185 (docs/adr/1126 §2–§3): the project's table styles, a table that follows one.
    if !doc.settings.table_styles.is_empty()
        || objs.iter().any(|e| matches!(e, Entity::Table(t) if t.table_style.is_some()))
    {
        return crate::SCHEMA_WITH_TABLE_STYLES;
    }
    // Schema 184 (docs/adr/1120 §4): a Big Font beside a text style's font, an SHX unifont or Big Font resource.
    if doc.settings.text_styles.iter().any(|s| s.big_font_file.is_some())
        || doc.styles.items.iter().any(kentos_contracts::shape_resource::is_font_form)
    {
        return crate::SCHEMA_WITH_SHX_FONTS;
    }
    // Schema 183 (docs/adr/1100 §3): the approved sheet template revisions.
    if !doc.settings.approved_sheet_templates.is_empty() {
        return crate::SCHEMA_WITH_APPROVED_SHEET_TEMPLATES;
    }
    // Schema 182 (docs/adr/1030): a table's parts, a cell of several lines or a data extraction table.
    let extracted = |e: &Entity| {
        matches!(e, Entity::Table(t)
            if t.breaks.is_some()
                || matches!(t.source, Some(kentos_contracts::TableSource::Extraction(_)))
                || t.cells.iter().flatten().any(|w| w.contains('\n')))
    };
    if objs.iter().any(extracted) || doc.blocks.iter().flat_map(|b| &b.entities).any(extracted) {
        return crate::SCHEMA_WITH_EXTRACTION;
    }
    // Schema 181 (docs/adr/1001 §1): a pattern past a LIN file's 12 elements.
    if kentos_contracts::has_long_patterns(&doc.settings.line_types) {
        return crate::SCHEMA_WITH_LONG_PATTERNS;
    }
    // Schema 180 (docs/adr/1000 §2): review notes.
    if !doc.settings.review_notes.is_empty() {
        return crate::SCHEMA_WITH_REVIEW_NOTES;
    }
    // Schema 179 (docs/adr/0974): an earthwork volume on a grid and its cell.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_volume_grid) {
        return crate::SCHEMA_WITH_VOLUME_GRID;
    }
    // Schema 178 (docs/adr/0960): a corridor region's slope transition.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_slope_transitions) {
        return crate::SCHEMA_WITH_SLOPE_TRANSITIONS;
    }
    // Schema 177 (docs/adr/0958): a template side's cut and fill slope benches.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_cut_benches) {
        return crate::SCHEMA_WITH_CUT_BENCHES;
    }
    // Schema 176 (docs/adr/0957): a volume factor row's densities and source.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_volume_states) {
        return crate::SCHEMA_WITH_VOLUME_STATES;
    }
    // Schema 175 (docs/adr/0955): a mass haul diagram's haul road network and its sites off the road.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_haul_network) {
        return crate::SCHEMA_WITH_HAUL_NETWORK;
    }
    // Schema 174 (docs/adr/0954): a mass haul diagram's approved plan and its history.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_haul_approval) {
        return crate::SCHEMA_WITH_HAUL_APPROVAL;
    }
    // Schema 173 (docs/adr/0950): a volume's numbered approvals and their history.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_volume_history) {
        return crate::SCHEMA_WITH_VOLUME_HISTORY;
    }
    // Schema 172 (docs/adr/0946): a profile's crossings.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_profile_crossings) {
        return crate::SCHEMA_WITH_PROFILE_CROSSINGS;
    }
    // Schema 171 (docs/adr/0944): a mass haul diagram's unsuitable cut or barriers.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_haul_sorting) {
        return crate::SCHEMA_WITH_HAUL_SORTING;
    }
    // Schema 170 (docs/adr/0939): a corridor's vertical chord height or sampling check.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_corridor_sampling) {
        return crate::SCHEMA_WITH_CORRIDOR_SAMPLING;
    }
    // Schema 169 (docs/adr/0934 §2): an earthwork volume's curve correction.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_volume_curve) {
        return crate::SCHEMA_WITH_VOLUME_CURVE;
    }
    // Schema 168 (docs/adr/0933 §3): a sample line set's clip, breaks method or curve step.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_section_breaks) {
        return crate::SCHEMA_WITH_SECTION_BREAKS;
    }
    // Schema 167 (docs/adr/0931 §2): a superelevation's axis of rotation.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_pivot) {
        return crate::SCHEMA_WITH_ROAD_PIVOT;
    }
    // Schema 166 (docs/adr/0975 §2): a road template's pavement or a volume's rule.
    if doc.settings.road.as_ref().is_some_and(crate::decode::road_has_pavement) {
        return crate::SCHEMA_WITH_ROAD_PAVEMENT;
    }
    if !doc.settings.standard_docs.is_empty() {
        return crate::SCHEMA_WITH_STANDARD_DOCS;
    }
    if doc
        .styles
        .items
        .iter()
        .any(kentos_contracts::shape_resource::is_resource)
    {
        return crate::SCHEMA_WITH_SHAPE_RESOURCES;
    }
    let new_text = |e: &Entity| matches!(e, Entity::Text(t)
        if t.paragraph.layout.as_ref().is_some_and(|l| l.engine.is_some()));
    if objs.iter().any(new_text) || doc.blocks.iter().any(|b| b.entities.iter().any(new_text)) {
        return crate::SCHEMA_WITH_TEXT_ENGINE;
    }
    // Schema 162 (plan/road.md Y1.1): a road project's work.
    if doc.settings.cad_work == Some(kentos_contracts::CadWork::Road) {
        return crate::SCHEMA_WITH_ROAD_WORK;
    }
    if doc.arrays.iter().any(|a|matches!(a.layout,kentos_contracts::ArrayLayout::Lattice {..})) {
        return crate::SCHEMA_WITH_LATTICE_ARRAYS;
    }
    if doc.blocks.iter().any(|b| b.dynamic.as_ref().is_some_and(|d| !d.type_parameters.is_empty() || !d.type_values.is_empty())) {
        return crate::SCHEMA_WITH_BLOCK_TYPE_VALUES;
    }
    if doc.blocks.iter().any(|b| b.dynamic.as_ref().is_some_and(|d| !d.parameter_groups.is_empty())) {
        return crate::SCHEMA_WITH_BLOCK_PARAMETER_GROUPS;
    }
    if doc.blocks.iter().any(|b| b.card.as_ref().is_some_and(|c| c.publication.is_some())) {
        return crate::SCHEMA_WITH_BLOCK_PUBLICATIONS;
    }
    if doc.constraints.constraints.iter().any(|c| {
        c.rule.elements().iter().any(|r| {
            matches!(r.element, kentos_contracts::CadElement::ArcEdge { .. })
        })
    }) {
        return crate::SCHEMA_WITH_ARC_EDGE_BONDS;
    }
    // Schema 155 (docs/adr/0811): a module instance holding one storey only.
    if doc.settings.arch.as_ref().is_some_and(|a| kentos_contracts::modules_have_instance_storeys(&a.modules)) {
        return crate::SCHEMA_WITH_INSTANCE_STOREYS;
    }
    // Schema 154 (docs/adr/0808): combinations of design options, a view recipe's combination.
    if doc.settings.arch.as_ref().is_some_and(|a| !a.option_combos.is_empty())
        || doc.settings.named_views.iter().any(|v| v.recipe.as_ref().is_some_and(|r| r.combo.is_some()))
    {
        return crate::SCHEMA_WITH_OPTION_COMBOS;
    }
    // Schema 153 (docs/adr/0807): an XOX slide, two unequal sliding leaves.
    let more = |list: Objs<'_>| list.iter().any(kentos_contracts::needs_slide_more);
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_slide_more)
        || more(objs)
        || doc.blocks.iter().any(|b| more(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_SLIDE_MORE;
    }
    // Schema 152 (docs/adr/0796): a module member on a storey above its instance's base level.
    if doc.settings.arch.as_ref().is_some_and(|a| kentos_contracts::modules_have_storeys(&a.modules)) {
        return crate::SCHEMA_WITH_MODULE_STOREYS;
    }
    // Schema 151 (docs/adr/0795): a schedule of the program's units, its fields, filter or rows' units.
    if objs.iter().any(unit_schedules) || doc.blocks.iter().any(|b| Objs::Plain(&b.entities).iter().any(unit_schedules)) {
        return crate::SCHEMA_WITH_UNIT_SCHEDULES;
    }
    // Schema 150 (docs/adr/0792): a three- or four-leaf sliding door or window, its tracks, a schedule of them.
    let multi = |list: Objs<'_>| list.iter().any(kentos_contracts::needs_multi_slide);
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_multi_slide)
        || multi(objs)
        || doc.blocks.iter().any(|b| multi(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_MULTI_SLIDE;
    }
    // Schema 149 (docs/adr/0788): design option sets, a view recipe's options.
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_options)
        || doc
            .settings
            .named_views
            .iter()
            .any(|v| v.recipe.as_ref().is_some_and(|r| !r.options.is_empty()))
    {
        return crate::SCHEMA_WITH_OPTIONS;
    }
    // Schema 148 (docs/adr/0785): the project's materials, a component or a slab bound to one.
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_materials)
        || objs.iter().any(materials)
        || doc.blocks.iter().any(|b| Objs::Plain(&b.entities).iter().any(materials))
        || doc.settings.wall_styles.iter().any(|s| s.components.iter().any(|c| c.material_id.is_some()))
    {
        return crate::SCHEMA_WITH_MATERIALS;
    }
    // Schema 147 (docs/adr/0784): the program's shares, a building's or a wing's levels, a schedule's share fields.
    if doc.settings.arch.as_ref().is_some_and(|a| a.program.has_shares())
        || objs.iter().any(program_shares)
        || doc.blocks.iter().any(|b| Objs::Plain(&b.entities).iter().any(program_shares))
    {
        return crate::SCHEMA_WITH_PROGRAM_SHARES;
    }
    // Schema 146 (docs/adr/0783): view templates, a recipe's pens, black and white or template.
    if !doc.settings.view_templates.is_empty()
        || doc.settings.named_views.iter().any(|v| {
            v.recipe.as_ref().is_some_and(|r| !r.pens.is_empty() || r.mono || r.template.is_some())
        })
    {
        return crate::SCHEMA_WITH_VIEW_TEMPLATES;
    }
    // Schema 145 (docs/adr/0779): a module instance's building-program group.
    if doc.settings.arch.as_ref().is_some_and(|a| kentos_contracts::modules_have_units(&a.modules)) {
        return crate::SCHEMA_WITH_MODULE_UNITS;
    }
    // Schema 144 (docs/adr/0777): a revolving, pocket or folding door, a sliding door's track, a schedule of them.
    let special = |list: Objs<'_>| list.iter().any(kentos_contracts::needs_special_doors);
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_special_doors)
        || special(objs)
        || doc.blocks.iter().any(|b| special(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_SPECIAL_DOORS;
    }
    // Schema 143 (docs/adr/0781): a view recipe's walls' detail or crop.
    if doc.settings.named_views.iter().any(|v| v.recipe.as_ref().is_some_and(|r| r.detail.is_some() || r.crop)) {
        return crate::SCHEMA_WITH_RECIPE_DETAIL;
    }
    // Schema 142 (docs/adr/0775): room modules and their instances.
    if doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_modules) {
        return crate::SCHEMA_WITH_MODULES;
    }
    // Schema 141 (docs/adr/0774): a named view's recipe.
    if doc.settings.named_views.iter().any(|v| v.recipe.is_some()) {
        return crate::SCHEMA_WITH_VIEW_RECIPES;
    }
    // Schema 140 (docs/adr/0773): the building program, a space's group, a schedule's program field.
    if doc.settings.arch.as_ref().is_some_and(|a| !a.program.is_empty())
        || objs.iter().any(building_program)
        || doc.blocks.iter().any(|b| Objs::Plain(&b.entities).iter().any(building_program))
    {
        return crate::SCHEMA_WITH_BUILDING_PROGRAM;
    }
    // Schema 139 (docs/adr/0772): door and window types, an opening following one or built, a schedule of widths.
    let opening_typed = |list: Objs<'_>| list.iter().any(kentos_contracts::needs_opening_types);
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_opening_types)
        || opening_typed(objs)
        || doc.blocks.iter().any(|b| opening_typed(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_OPENING_TYPES;
    }
    // Schema 138 (docs/adr/0771): a turning wall component, a node decision, a bound slab.
    if objs.iter().any(building_parts)
        || doc.blocks.iter().any(|b| Objs::Plain(&b.entities).iter().any(building_parts))
        || doc.settings.wall_styles.iter().any(|s| s.components.iter().any(|c| c.wrap))
    {
        return crate::SCHEMA_WITH_BUILDING_PARTS;
    }
    // Schema 137 (docs/adr/0770): a constraint naming a wall's line or an opening's jamb.
    if doc
        .constraints
        .constraints
        .iter()
        .any(|c| kentos_contracts::rule_is_architectural(&c.rule))
    {
        return crate::SCHEMA_WITH_ARCH_BONDS;
    }
    // Schema 136 (docs/adr/0758): a layer's indexes.
    fn indexed(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| !n.indexes.is_empty() || indexed(&n.children))
    }
    if indexed(&doc.layers) {
        return crate::SCHEMA_WITH_LAYER_INDEXES;
    }
    // Schema 135 (docs/adr/0801): what a CAD project is drawn for.
    if doc.settings.cad_work.is_some() {
        return crate::SCHEMA_WITH_CAD_WORK;
    }
    // Schema 134 (docs/adr/0755): a bound table's constraints in its link, a field's list from one of them.
    fn checked(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| {
            n.fields
                .iter()
                .any(|f| f.column.as_ref().is_some_and(|c| c.check.is_some()))
                || checked(&n.children)
        })
    }
    if checked(&doc.layers)
        || doc
            .settings
            .sources
            .iter()
            .any(|s| !s.checks.is_empty() || !s.uniques.is_empty())
    {
        return crate::SCHEMA_WITH_TABLE_CONSTRAINTS;
    }
    // Schema 133 (docs/adr/0753): a layer field's bound column, what the table makes and refers to.
    fn columns(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.fields.iter().any(|f| f.column.is_some()) || columns(&n.children))
    }
    if columns(&doc.layers) {
        return crate::SCHEMA_WITH_FIELD_COLUMNS;
    }
    // Schema 132 (docs/adr/0735): the data profiles and mappings the project carries.
    if !doc.settings.profile_files.is_empty() {
        return crate::SCHEMA_WITH_PROFILE_FILES;
    }
    // Schema 131 (docs/adr/0733): a block insert's level and phase, a space's skirting finish, a schedule's fixtures,
    // new fields, type filter, headings or subtotals.
    let fixtures = |es: Objs<'_>| {
        es.iter().any(|e| match e {
            Entity::Insert(i) => !i.arch.is_empty(),
            Entity::Space(s) => s.space.finishes.base.is_some(),
            Entity::Table(t) => matches!(
                &t.source,
                Some(kentos_contracts::TableSource::Schedule { of, columns, filter, sort, .. })
                    if kentos_contracts::schedule_extras(*of, columns, filter.as_ref(), sort)
            ),
            _ => false,
        })
    };
    if fixtures(objs)
        || doc
            .blocks
            .iter()
            .any(|b| fixtures(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_FIXTURES;
    }
    // Schema 130 (docs/adr/0732 §3): Mimari denetim's exceptions.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_audit_exceptions)
    {
        return crate::SCHEMA_WITH_AUDIT_EXCEPTIONS;
    }
    // Schema 129 (docs/adr/0730): a dimension recipe's sheet scales, its values shifted, a member's value placed.
    if doc.dimension_sets.iter().any(|s| {
        !s.scales.is_empty() || !s.tight.is_keep() || s.members.iter().any(|m| m.shift.is_some())
    }) {
        return crate::SCHEMA_WITH_DIMENSION_SET_SCALES;
    }
    // Schema 128 (docs/adr/0729): a façade's or a space's dimension recipe.
    if doc.dimension_sets.iter().any(|s| s.span.is_some()) {
        return crate::SCHEMA_WITH_DIMENSION_SET_SPANS;
    }
    // Schema 127 (docs/adr/0727): the kitchen runs.
    if !doc.casework.is_empty() {
        return crate::SCHEMA_WITH_CASEWORK;
    }
    // Schema 126 (docs/adr/0725): a gapped grid, an anchored or an L column.
    if objs.iter().any(|e| match e {
        Entity::Grid(g) => g.grid.ends.is_some(),
        Entity::Column(c) => {
            c.column.anchor != kentos_contracts::ColumnAnchor::Center
                || c.column.legs.is_some()
                || c.column.shape == kentos_contracts::ColumnShape::LShape
        }
        _ => false,
    }) {
        return crate::SCHEMA_WITH_GRID_COLUMN_FORMS;
    }
    // Schema 125 (docs/adr/0724): a stair whose rise is its level's height, or with a centre post.
    if objs.iter().any(
        |e| matches!(e, Entity::Stair(s) if s.stair.height_from_level || s.stair.post.is_some()),
    ) {
        return crate::SCHEMA_WITH_STAIR_LEVELS;
    }
    // Schema 124 (docs/adr/0723 §2): a ramp bound to its level.
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Ramp(r) if r.ramp.base_offset.is_some()))
    {
        return crate::SCHEMA_WITH_RAMP_LEVELS;
    }
    // Schema 123 (docs/adr/0722): a slab.
    if objs.iter().any(|e| matches!(e, Entity::Slab(_))) {
        return crate::SCHEMA_WITH_SLABS;
    }
    // Schema 122 (docs/adr/0721 §4): a block schedule with further columns (of the drawing or a block).
    let columned = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Table(t)
                if matches!(&t.source, Some(kentos_contracts::TableSource::Blocks { columns, .. }) if !columns.is_empty()))
        })
    };
    if columned(objs)
        || doc
            .blocks
            .iter()
            .any(|b| columned(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_SCHEDULE_COLUMNS;
    }
    // Schema 121 (docs/adr/0721): a Kot bound to a level (of the drawing or a block).
    let levelled = |list: Objs<'_>| {
        list.iter()
            .any(|e| matches!(e, Entity::Dimension(d) if d.level.is_some()))
    };
    if levelled(objs)
        || doc
            .blocks
            .iter()
            .any(|b| levelled(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_DIMENSION_LEVELS;
    }
    // Schema 120 (docs/adr/0720): the level and view the drawing opens with.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_level_state)
    {
        return crate::SCHEMA_WITH_LEVEL_STATE;
    }
    // Schema 119 (docs/adr/0719): a niche or flue, an opening too deep for its wall or a schedule of niches (of the
    // drawing or a block).
    let recessed = |list: Objs<'_>| list.iter().any(kentos_contracts::needs_recesses);
    if recessed(objs)
        || doc
            .blocks
            .iter()
            .any(|b| recessed(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_RECESSES;
    }
    // Schema 118 (docs/adr/0711): a dimension (of the drawing or of a block) whose references follow a cut.
    let crossed = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d)
                if d.refs.as_ref().is_some_and(|r| r.rule == Some(kentos_contracts::DimensionRule::Cross)))
        })
    };
    if crossed(objs) || doc.blocks.iter().any(|b| crossed(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_DIMENSION_CROSS;
    }
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Stair(s) if s.stair.bulges.is_some()))
    {
        return crate::SCHEMA_WITH_STAIR_ARCS;
    }
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Space(s) if s.space.void))
    {
        return crate::SCHEMA_WITH_SPACE_VOIDS;
    }
    if objs.iter().any(|e| match e {
        Entity::Stair(s) => s.stair.railings.is_some(),
        Entity::Ramp(r) => r.ramp.railings.is_some(),
        _ => false,
    }) {
        return crate::SCHEMA_WITH_RAILINGS;
    }
    if doc.dimension_sets.iter().any(|s| s.edge.is_some()) {
        return crate::SCHEMA_WITH_DIMENSION_SET_EDGES;
    }
    if !doc.dimension_sets.is_empty() {
        return crate::SCHEMA_WITH_DIMENSION_SETS;
    }
    fn snaps(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| n.snap.is_some() || snaps(&n.children))
    }
    fn services(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.service.is_some() || n.feed.is_some() || services(&n.children))
    }
    fn temporal(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| {
            n.time.is_some()
                || n.scenario.is_some()
                || n.replaces.is_some()
                || temporal(&n.children)
        })
    }
    fn filters(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.filter.is_some() || filters(&n.children))
    }
    fn labelled(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| {
            n.style.labels.is_some()
                || n.style
                    .label
                    .as_ref()
                    .is_some_and(labels::has_engine_fields)
                || labelled(&n.children)
        })
    }
    fn informed(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| !n.actions.is_empty() || n.card.is_some() || informed(&n.children))
    }
    fn composed(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.compose.is_some() || composed(&n.children))
    }
    // Schema 90 (docs/adr/0537): a layer time's system time attributes.
    fn recorded(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| {
            n.time
                .as_ref()
                .is_some_and(kentos_contracts::LayerTime::recorded)
                || recorded(&n.children)
        })
    }
    // Schema 112 (docs/adr/0690): the project's room names, usages and level prefixes.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(kentos_contracts::ArchSettings::has_space_choices)
    {
        return crate::SCHEMA_WITH_SPACE_CHOICES;
    }
    // Schema 111 (docs/adr/0679): a polyline's virtual room separator role.
    let separator = |e: &Entity| matches!(e, Entity::Polyline(p) | Entity::Polygon(p) if p.space_separator.is_some());
    if objs.iter().any(separator) || doc.blocks.iter().any(|b| b.entities.iter().any(separator)) {
        return crate::SCHEMA_WITH_SPACE_SEPARATORS;
    }
    // Schema 110 (docs/adr/0678): a space's chosen sources.
    if objs.iter().any(|e| {
        matches!(e, Entity::Space(s) if s.space.sources.is_some()
            || s.space.broken == Some(kentos_contracts::SpaceBreak::SourceUnavailable))
    }) {
        return crate::SCHEMA_WITH_SPACE_SOURCES;
    }
    // Schema 109 (docs/adr/0674): a space's source scope.
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Space(s) if !s.space.scope.is_project()))
    {
        return crate::SCHEMA_WITH_SPACE_SCOPE;
    }
    // Schema 108 (docs/adr/0672): a space's column deduction and its new break reasons.
    if objs.iter().any(|e| {
        matches!(e, Entity::Space(s) if !s.space.columns.is_ignore()
            || s.space.broken.is_some_and(|b| b != kentos_contracts::SpaceBreak::LostEnclosure))
    }) {
        return crate::SCHEMA_WITH_SPACE_COLUMNS;
    }
    // Schema 104 (docs/adr/0629 §5): a layer field of a kind of its own.
    fn wide(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.fields.iter().any(|f| f.kind.is_wide()) || wide(&n.children))
    }
    // Schema 107 (docs/adr/0638 §5): a layer field whose values do not leave the drawing.
    fn withheld(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.fields.iter().any(|f| f.withheld) || withheld(&n.children))
    }
    if withheld(&doc.layers) {
        return crate::SCHEMA_WITH_WITHHELD_FIELDS;
    }
    // Schema 106 (docs/adr/0633 §3): a layer's dataset description or derivation record.
    fn described(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.dataset.is_some() || n.derivation.is_some() || described(&n.children))
    }
    if described(&doc.layers) {
        return crate::SCHEMA_WITH_DATASETS;
    }
    // Schema 105 (docs/adr/0632 §4): a join view reading a source's table or summing its joined records.
    if doc
        .settings
        .join_views
        .iter()
        .any(|v| v.source.is_some() || !v.aggregates.is_empty())
    {
        return crate::SCHEMA_WITH_JOIN_SOURCES;
    }
    if wide(&doc.layers) {
        return crate::SCHEMA_WITH_FIELD_KINDS;
    }
    // Schema 103 (docs/adr/0627 §3): a raster showing a GeoPackage tile table.
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Raster(r) if r.raster.tiles.is_some()))
    {
        return crate::SCHEMA_WITH_RASTER_TILES;
    }
    // Schema 102 (docs/adr/0622 §5): a layer's type, the project's Otomatik seçim.
    fn typed(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.geometry.is_some() || typed(&n.children))
    }
    if doc.settings.auto_select.is_some() || typed(&doc.layers) {
        return crate::SCHEMA_WITH_LAYER_GEOMETRY;
    }
    // Schema 101 (docs/adr/0598 §1): what the split and merged objects came from.
    if !doc.settings.lineage.is_empty() {
        return crate::SCHEMA_WITH_LINEAGE;
    }
    // Schema 100 (docs/adr/0594 §1): a georeference record's targets' system as a definition.
    if objs.iter().any(
        |e| matches!(e, Entity::Raster(r) if r.raster.georef.as_ref().is_some_and(|g| g.crs.is_some())),
    ) {
        return crate::SCHEMA_WITH_GEOREF_CRS;
    }
    // Schema 99 (docs/adr/0589): a topology exception's geometry digest.
    if doc
        .settings
        .topology
        .as_ref()
        .is_some_and(|t| t.exceptions.iter().any(|x| x.digest.is_some()))
    {
        return crate::SCHEMA_WITH_EXCEPTION_DIGESTS;
    }
    // Schema 98 (docs/adr/0583): a layer's source system, the map's system.
    fn source_crs(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.source_crs.is_some() || source_crs(&n.children))
    }
    if doc.settings.map_srid.is_some() || source_crs(&doc.layers) {
        return crate::SCHEMA_WITH_SOURCE_CRS;
    }
    // Schema 97 (docs/adr/0569): the project's links of event layers to their routes.
    if !doc.settings.linear_events.is_empty() {
        return crate::SCHEMA_WITH_LINEAR_EVENTS;
    }
    // Schema 96 (docs/adr/0568): a closure's weekdays or hours.
    if doc
        .settings
        .networks
        .iter()
        .filter_map(|n| n.closures.as_ref())
        .any(|c| c.days.is_some() || c.hours.is_some())
    {
        return crate::SCHEMA_WITH_TIMED_CLOSURES;
    }
    // Schema 95 (docs/adr/0567): a network's vehicle profiles or dated closures.
    if doc
        .settings
        .networks
        .iter()
        .any(|n| !n.profiles.is_empty() || n.closures.is_some())
    {
        return crate::SCHEMA_WITH_NETWORK_PROFILES;
    }
    // Schema 94 (docs/adr/0562): a device's configurations, the subnetworks' keepCurrent.
    if doc.settings.networks.iter().any(|n| {
        n.terminals.as_ref().is_some_and(|t| {
            t.devices
                .iter()
                .any(|d| d.by.is_some() || !d.configs.is_empty())
        }) || n.subnetworks.as_ref().is_some_and(|s| s.keep_current)
    }) {
        return crate::SCHEMA_WITH_NETWORK_CONFIGS;
    }
    // Schema 93 (docs/adr/0557): a network's terminals or subnetworks.
    if doc
        .settings
        .networks
        .iter()
        .any(|n| n.terminals.is_some() || n.subnetworks.is_some())
    {
        return crate::SCHEMA_WITH_NETWORK_TERMINALS;
    }
    // Schema 91 (docs/adr/0538 §1): the bound layers' bases.
    if doc.settings.source_bases.iter().any(|b| b.fields.is_some()) {
        return crate::SCHEMA_WITH_SOURCE_FIELD_BASES;
    }
    if !doc.settings.source_bases.is_empty() {
        return crate::SCHEMA_WITH_SOURCE_BASES;
    }
    if recorded(&doc.layers) {
        return crate::SCHEMA_WITH_RECORDED_TIME;
    }
    // Schema 89 (docs/adr/0527): the project's join views.
    if !doc.settings.join_views.is_empty() {
        return crate::SCHEMA_WITH_JOIN_VIEWS;
    }
    // Schema 88 (docs/adr/0518 §1): a raster's georeference record.
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Raster(r) if r.raster.georef.is_some()))
    {
        return crate::SCHEMA_WITH_GEOREF;
    }
    // Schema 87 (docs/adr/0521): a network's turns or levels, a speed cost's time profile.
    if doc.settings.networks.iter().any(|n| {
        n.turns.is_some() || n.levels.is_some() || n.costs.iter().any(|c| !c.periods.is_empty())
    }) {
        return crate::SCHEMA_WITH_NETWORK_TURNS;
    }
    // Schema 86 (docs/adr/0519 §1): what the elevations are, the coordinates' epoch.
    if doc.settings.heights.is_some() || doc.settings.coordinate_epoch.is_some() {
        return crate::SCHEMA_WITH_HEIGHTS;
    }
    // Schema 85 (docs/adr/0351): an insert's clipping, a PDF underlay.
    let clipped = |es: Objs<'_>| {
        es.iter()
            .any(|e| matches!(e, Entity::Insert(i) if i.clipping.is_some()))
    };
    if clipped(objs)
        || doc.blocks.iter().any(|b| {
            clipped(Objs::Plain(&b.entities))
                || b.xref
                    .as_ref()
                    .is_some_and(|x| x.page.is_some() || x.underlay.is_some())
        })
    {
        return crate::SCHEMA_WITH_CLIPPING;
    }
    // Schema 84 (docs/adr/0349 §1): the external references' search paths.
    if !doc.settings.xref_paths.is_empty() {
        return crate::SCHEMA_WITH_XREF_PATHS;
    }
    // Schema 83 (docs/adr/0348 §1): an external reference.
    if doc.blocks.iter().any(|b| b.xref.is_some()) {
        return crate::SCHEMA_WITH_XREFS;
    }
    // Schema 82 (docs/adr/0346 §2): the project's phase view.
    if doc
        .settings
        .arch
        .as_ref()
        .is_some_and(|a| a.phase_view.is_some())
    {
        return crate::SCHEMA_WITH_PHASES;
    }
    // Schema 81 (docs/adr/0345): a view mark, or a named view with its own id.
    if objs.iter().any(|e| matches!(e, Entity::ViewMark(_)))
        || doc.settings.named_views.iter().any(|v| !v.id.is_empty())
    {
        return crate::SCHEMA_WITH_VIEWMARKS;
    }
    // Schema 80 (docs/adr/0344): a tag, or a table's schedule of the drawing's objects.
    let scheduled = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Tag(_))
                || matches!(e, Entity::Table(t) if matches!(t.source, Some(kentos_contracts::TableSource::Schedule { .. })))
        })
    };
    if scheduled(objs) {
        return crate::SCHEMA_WITH_TAGS;
    }
    // Schema 79 (docs/adr/0343): a grid, a column, a stair or a ramp.
    if objs.iter().any(|e| {
        matches!(
            e,
            Entity::Grid(_) | Entity::Column(_) | Entity::Stair(_) | Entity::Ramp(_)
        )
    }) {
        return crate::SCHEMA_WITH_ARCH_ELEMENTS;
    }
    // Schema 78 (docs/adr/0342): a space, or the project's space method.
    if objs.iter().any(|e| matches!(e, Entity::Space(_)))
        || doc
            .settings
            .arch
            .as_ref()
            .is_some_and(|a| a.space_method.is_some())
    {
        return crate::SCHEMA_WITH_SPACES;
    }
    // Schema 77 (docs/adr/0341): an opening.
    if objs.iter().any(|e| matches!(e, Entity::Opening(_))) {
        return crate::SCHEMA_WITH_OPENINGS;
    }
    // Schema 76 (docs/adr/0340): a wall's joins or cleanup, a component's priority.
    let ranked = |cs: &[kentos_contracts::WallComponent]| cs.iter().any(|c| c.priority.is_some());
    if objs.iter().any(|e| {
        matches!(e, Entity::Wall(w) if !w.wall.joins.is_auto() || !w.wall.cleanup || ranked(&w.wall.components))
    }) || doc.settings.wall_styles.iter().any(|s| ranked(&s.components))
    {
        return crate::SCHEMA_WITH_WALL_JOINS;
    }
    // Schema 75 (docs/adr/0339): a wall, a wall style or the architectural settings.
    if objs.iter().any(|e| matches!(e, Entity::Wall(_)))
        || !doc.settings.wall_styles.is_empty()
        || doc.settings.arch.is_some()
    {
        return crate::SCHEMA_WITH_WALLS;
    }
    // Schema 74 (docs/adr/0503): a layer bound to its source.
    if !doc.settings.sources.is_empty() {
        return crate::SCHEMA_WITH_SOURCES;
    }
    // Schema 72 (docs/adr/0401, 0405, 0408, 0410, 0412, 0413, 0418, 0430): a form, a value list or a relation of the
    // project, a layer field's stable id, its several values, its link to a list, a retired value or its date-and-time
    // or time kind, a layer's table rules, or a record without a geometry.
    fn keyed(nodes: &[LayerNode]) -> bool {
        let later = |f: &kentos_contracts::LayerField| {
            f.id.is_some()
                || f.multiple
                || f.domain.is_some()
                || f.values
                    .as_ref()
                    .is_some_and(|v| v.iter().any(|c| c.retired))
                || f.split.is_some()
                || f.merge.is_some()
                || f.unique
                || f.min_exclusive
                || f.max_exclusive
                || f.unit.is_some()
                || f.values.as_ref().is_some_and(|v| {
                    v.iter()
                        .any(|c| c.description.is_some() || c.from.is_some() || c.until.is_some())
                })
                || matches!(
                    f.kind,
                    kentos_contracts::LayerFieldKind::DateTime
                        | kentos_contracts::LayerFieldKind::Time
                )
        };
        nodes.iter().any(|n| {
            n.fields.iter().any(later)
                || !n.table_styles.is_empty()
                || n.subtypes.is_some()
                || !n.contingents.is_empty()
                || !n.rules.is_empty()
                || keyed(&n.children)
        })
    }
    // Schema 73 (docs/adr/0334): constraints or parameters.
    if !doc.constraints.is_empty() {
        return crate::SCHEMA_WITH_CONSTRAINTS;
    }
    // The project's value lists and relations and a field's link to a list are schema 72's too (docs/adr/0430, 0405).
    if !doc.settings.forms.is_empty()
        || !doc.settings.domains.is_empty()
        || !doc.settings.relations.is_empty()
        || keyed(&doc.layers)
        || objs.iter().any(|e| matches!(e, Entity::Record(_)))
    {
        return crate::SCHEMA_WITH_FORMS;
    }
    // Schema 71 (docs/adr/0331): the project's hatch patterns, a pattern's own origin, an annotative hatch.
    if !doc.settings.hatch_patterns.is_empty()
        || objs.iter().any(|e| {
            matches!(e, Entity::Hatch(h) if h.pattern.origin.is_some() || h.annotative.is_some())
        })
        || doc.blocks.iter().any(|b| {
            b.entities.iter().any(|e| {
                matches!(e, Entity::Hatch(h) if h.pattern.origin.is_some() || h.annotative.is_some())
            })
        })
    {
        return crate::SCHEMA_WITH_HATCH_EXTRAS;
    }
    // Schema 70 (docs/adr/0329 §3): a face's tie closed across gaps.
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Hatch(h) if h.assoc.as_ref().is_some_and(|a| a.gap.is_some())))
    {
        return crate::SCHEMA_WITH_HATCH_GAPS;
    }
    // Schema 69 (docs/adr/0328): a hatch's exact boundary or a tie to a face's line work.
    let bounded = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Hatch(h)
                if h.boundary.is_some() || h.assoc.as_ref().is_some_and(|a| !a.bounds.is_empty()))
        })
    };
    if bounded(objs) || doc.blocks.iter().any(|b| bounded(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_HATCH_BOUNDARIES;
    }
    // Schema 68 (docs/adr/0327): a table's styles, formulas, fields, linked file or block schedule.
    let tabled = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Table(t)
                if !t.styles.is_empty()
                    || !t.formulas.is_empty()
                    || !t.fields.is_empty()
                    || matches!(&t.source, Some(kentos_contracts::TableSource::Blocks { .. }))
                    || matches!(&t.source, Some(kentos_contracts::TableSource::File { path, digest, copy, overrides, key_column, .. })
                        if path.is_some() || digest.is_some() || copy.is_some() || !overrides.is_keep() || key_column.is_some()))
        })
    };
    if tabled(objs) {
        return crate::SCHEMA_WITH_TABLE_CELLS;
    }
    // Schema 67 (docs/adr/0326): a leader's arms, tip, blocks or style, or the project's leader styles.
    let multi = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Leader(l)
                if !l.arms.is_empty() || l.tip.is_some() || !l.blocks.is_empty() || l.leader_style.is_some())
        })
    };
    if multi(objs)
        || doc.blocks.iter().any(|b| multi(Objs::Plain(&b.entities)))
        || !doc.settings.leader_styles.is_empty()
    {
        return crate::SCHEMA_WITH_MULTILEADER;
    }
    // Schema 66 (docs/adr/0325): a text's fields (of the drawing or of a block).
    let fielded = |list: Objs<'_>| {
        list.iter()
            .any(|e| matches!(e, Entity::Text(t) if !t.paragraph.fields.is_empty()))
    };
    if fielded(objs) || doc.blocks.iter().any(|b| fielded(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_TEXT_FIELDS;
    }
    // Schema 65 (docs/adr/0324): a text's layout or a stacked fraction (of the drawing or of a block).
    let laid = |list: Objs<'_>| {
        list.iter().any(|e| match e {
            Entity::Text(t) => {
                t.paragraph.layout.is_some()
                    || t.paragraph
                        .runs
                        .iter()
                        .any(|r| r.script == Some(kentos_contracts::TextScript::Stack))
            }
            _ => false,
        })
    };
    if laid(objs) || doc.blocks.iter().any(|b| laid(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_PARAGRAPH_LAYOUT;
    }
    // Schema 64 (docs/adr/0323): an annotative text, dimension or leader (of the drawing or of a block).
    let scaled = |list: Objs<'_>| {
        list.iter()
            .any(|e| kentos_contracts::annotative_of(e).is_some())
    };
    if scaled(objs) || doc.blocks.iter().any(|b| scaled(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_ANNOTATIVE;
    }
    // Schema 63 (docs/adr/0322 §5): a text's or a dimension's overrides (of the drawing or of a block).
    let overridden = |list: Objs<'_>| {
        list.iter().any(|e| match e {
            Entity::Text(t) => !t.face.overrides.is_empty(),
            Entity::Dimension(d) => !d.look.overrides.is_empty(),
            _ => false,
        })
    };
    if overridden(objs)
        || doc
            .blocks
            .iter()
            .any(|b| overridden(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_OVERRIDES;
    }
    // Schema 62 (docs/adr/0322): a centre mark, an axis or a level (of the drawing or of a block), or a pair rule.
    let marked = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d)
                if d.style.is_some_and(DimensionStyle::is_schema_62)
                    || d.refs.as_ref().is_some_and(|r| r.rule == Some(kentos_contracts::DimensionRule::Pair)))
        })
    };
    if marked(objs) || doc.blocks.iter().any(|b| marked(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_DIMENSION_MARKS;
    }
    // Schema 61 (docs/adr/0321): a dimension's options, moved value or breaks (of the drawing or of a block), or a
    // dimension style's options.
    let optioned = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d)
                if d.look.options.is_some() || d.text_shift.is_some() || !d.breaks.is_empty())
        })
    };
    if optioned(objs)
        || doc
            .blocks
            .iter()
            .any(|b| optioned(Objs::Plain(&b.entities)))
        || doc
            .settings
            .dimension_styles
            .iter()
            .any(|s| s.options.is_some())
    {
        return crate::SCHEMA_WITH_DIMENSION_OPTIONS;
    }
    // Schema 60 (docs/adr/0320 §1): a dimension's references' rule (in the drawing only: a block's dimensions keep none).
    if objs.iter().any(
        |e| matches!(e, Entity::Dimension(d) if d.refs.as_ref().is_some_and(|r| r.rule.is_some())),
    ) {
        return crate::SCHEMA_WITH_DIMENSION_RULES;
    }
    // Schema 59 (docs/adr/0318): a block's dynamic part or an insert's parameter values, of the drawing or of a block.
    let valued = |list: Objs<'_>| {
        list.iter()
            .any(|e| matches!(e, kentos_contracts::Entity::Insert(i) if !i.parameters.is_empty()))
    };
    if valued(objs)
        || doc
            .blocks
            .iter()
            .any(|b| b.dynamic.is_some() || valued(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_DYNAMIC_BLOCKS;
    }
    // Schema 58 (docs/adr/0317 §1, §4): an attribute definition's option, an insert's moved attribute (of the drawing
    // or of a block definition) or a block's card.
    let placed = |list: Objs<'_>| {
        list.iter().any(
            |e| matches!(e, kentos_contracts::Entity::Insert(i) if !i.attribute_places.is_empty()),
        )
    };
    if placed(objs)
        || doc.blocks.iter().any(|b| {
            b.card.is_some()
                || placed(Objs::Plain(&b.entities))
                || b.attributes
                    .iter()
                    .any(|a| a.options != kentos_contracts::AttributeOptions::default())
        })
    {
        return crate::SCHEMA_WITH_BLOCK_CARDS;
    }
    // Schema 57 (docs/adr/0315 §3, §4): an insert's own y factor, of the drawing or of a block definition, or a
    // block's unit.
    let stretched = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, kentos_contracts::Entity::Insert(i) if i.y_scale.is_some_and(|y| y != i.scale))
        })
    };
    if stretched(objs)
        || doc
            .blocks
            .iter()
            .any(|b| b.units.is_some() || stretched(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_BLOCK_EXTRAS;
    }
    // Schema 56 (docs/adr/0313): a line type with a pattern, a line type scale, a pattern named by a layer, a layer
    // state's style or an object of the drawing or of a block definition.
    fn patterned(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.style.line_pattern.is_some() || patterned(&n.children))
    }
    let named = |list: Objs<'_>| list.iter().any(|e| e.base().look.line_pattern.is_some());
    if !doc.settings.line_types.is_empty()
        || doc.settings.line_type_scale.is_some()
        || patterned(&doc.layers)
        || doc
            .settings
            .layer_states
            .iter()
            .flat_map(|s| &s.nodes)
            .any(|n| n.style.as_ref().is_some_and(|s| s.line_pattern.is_some()))
        || named(objs)
        || doc.blocks.iter().any(|b| named(Objs::Plain(&b.entities)))
    {
        return crate::SCHEMA_WITH_PATTERNS;
    }
    // Schema 55 (docs/adr/0311): a frozen node, or a layer state keeping freezes and printing.
    fn frozen(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.frozen == Some(true) || frozen(&n.children))
    }
    if frozen(&doc.layers)
        || doc
            .settings
            .layer_states
            .iter()
            .flat_map(|s| &s.nodes)
            .any(|n| n.frozen.is_some() || n.plot.is_some())
    {
        return crate::SCHEMA_WITH_FROZEN;
    }
    // Schema 54 (docs/adr/0310): an area's mask, a block definition's too.
    let masks = |list: Objs<'_>| list.iter().any(|e| e.base().look.wipeout.is_some());
    if masks(objs) || doc.blocks.iter().any(|b| masks(Objs::Plain(&b.entities))) {
        return crate::SCHEMA_WITH_WIPEOUTS;
    }
    // Schema 53 (docs/adr/0300): an associative array.
    if !doc.arrays.is_empty() {
        return SCHEMA_WITH_ARRAYS;
    }
    // Schema 52 (docs/adr/0285): a named view.
    if !doc.settings.named_views.is_empty() {
        return SCHEMA_WITH_VIEWS;
    }
    // Schema 51 (docs/adr/0263): a dimension's reference along an edge or inside a block.
    let extended = |r: &kentos_contracts::SubRef| {
        matches!(r.place, kentos_contracts::SubPlace::Along { .. }) || !r.path.is_empty()
    };
    if objs.iter().any(|e| {
        matches!(e, kentos_contracts::Entity::Dimension(d)
            if d.refs.as_ref().is_some_and(|r| r.each().any(|(_, s)| extended(s))))
    }) {
        return SCHEMA_WITH_SUBREF_EXTRAS;
    }
    // Schema 50 (docs/adr/0262): an inch or foot drawing, or a length style.
    if doc.settings.drawing_unit.is_some_and(|u| u.imperial())
        || doc.settings.length_style.is_some()
    {
        return SCHEMA_WITH_IMPERIAL;
    }
    // Schema 49 (docs/adr/0261): a user coordinate system, current or named.
    if doc.settings.ucs.is_some() || !doc.settings.named_ucs.is_empty() {
        return SCHEMA_WITH_UCS;
    }
    // Schema 48 (docs/adr/0260): an object's look, a block definition's too, or a layer not printed.
    fn unprinted(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.plot == Some(false) || unprinted(&n.children))
    }
    let looks = |list: Objs<'_>| list.iter().any(|e| !e.base().look.is_empty());
    if looks(objs)
        || doc.blocks.iter().any(|b| looks(Objs::Plain(&b.entities)))
        || unprinted(&doc.layers)
    {
        return SCHEMA_WITH_LOOK;
    }
    // Schema 47 (docs/adr/0259): a NURBS curve, a block definition's too.
    let curves = |list: Objs<'_>| list.iter().any(|e| matches!(e, Entity::Nurbs(_)));
    if curves(objs) || doc.blocks.iter().any(|b| curves(Objs::Plain(&b.entities))) {
        return SCHEMA_WITH_NURBS;
    }
    // Schema 46 (docs/adr/0258): a closed or a wide polyline, a block definition's too.
    let extras = |list: Objs<'_>| {
        list.iter().any(|e| match e {
            Entity::Polyline(p) => p.closed || p.widths.is_some(),
            _ => false,
        })
    };
    if extras(objs) || doc.blocks.iter().any(|b| extras(Objs::Plain(&b.entities))) {
        return SCHEMA_WITH_POLYLINE_EXTRAS;
    }
    // Schema 45 (docs/adr/0253 §6): the requests the drawing executed.
    if !doc.requests.is_empty() {
        return SCHEMA_WITH_REQUESTS;
    }
    // Schema 44 (docs/adr/0251): the project's sheet book.
    if doc.sheets.is_some() {
        return SCHEMA_WITH_SHEETS;
    }
    // Schema 43 (docs/adr/0249 §4): a dimension's references (the drawing's only; a block
    // definition's are refused).
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Dimension(d) if d.refs.is_some()))
    {
        return SCHEMA_WITH_DIMENSION_REFS;
    }
    // Schema 42 (docs/adr/0229 §2): the project's address data.
    if !doc.settings.address_locators.is_empty() {
        return SCHEMA_WITH_ADDRESSES;
    }
    // Schema 41 (docs/adr/0226 §3, §4): a layer's composition.
    if composed(&doc.layers) {
        return SCHEMA_WITH_COMPOSE;
    }
    // Schema 40 (docs/adr/0225): an object's attachment, a layer's action or card.
    if objs.iter().any(|e| !e.base().attachments.is_empty()) || informed(&doc.layers) {
        return SCHEMA_WITH_ATTACHMENTS;
    }
    // Schema 39 (docs/adr/0224): a vertex measure of an object of the drawing or of a block definition.
    if objs.iter().any(has_measure)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(has_measure))
    {
        return SCHEMA_WITH_MEASURES;
    }
    // Schema 38 (docs/adr/0243): a raster showing a NetCDF variable, or a mesh's lines.
    if objs.iter().any(|e| match e {
        Entity::Raster(r) => r.raster.dataset.is_some() || r.raster.style.edges.is_some(),
        _ => false,
    }) {
        return SCHEMA_WITH_MULTIDIM;
    }
    // Schema 37 (docs/adr/0214): the project's variables.
    if !doc.settings.variables.is_empty() {
        return SCHEMA_WITH_VARIABLES;
    }
    // Schema 36 (docs/adr/0212): the label engine's fields, a layer's labelling, an object's pins (a layer
    // state's style counts too).
    if labelled(&doc.layers)
        || objs.iter().any(|e| !e.base().label_pins.is_empty())
        || doc
            .settings
            .layer_states
            .iter()
            .flat_map(|s| &s.nodes)
            .any(|n| {
                n.style.as_ref().is_some_and(|st| {
                    st.labels.is_some() || st.label.as_ref().is_some_and(labels::has_engine_fields)
                })
            })
    {
        return SCHEMA_WITH_LABELS;
    }
    // Schema 35 (docs/adr/0211): a layer's filter.
    if filters(&doc.layers) {
        return SCHEMA_WITH_FILTERS;
    }
    // Schema 34 (docs/adr/0210): a layer's time, a scenario, a scenario layer's base layer.
    if temporal(&doc.layers) {
        return SCHEMA_WITH_TEMPORAL;
    }
    // Schema 33 (docs/adr/0209): the project's networks.
    if !doc.settings.networks.is_empty() {
        return SCHEMA_WITH_NETWORKS;
    }
    // Schema 32 (docs/adr/0208): a layer drawn from a service, a layer's
    // feed, or the project's connections.
    if services(&doc.layers) || !doc.settings.connections.is_empty() {
        return SCHEMA_WITH_SERVICES;
    }
    // Schema 31 (docs/adr/0207): a point cloud, or a raster read from an address; the drawing's only.
    if objs.iter().any(|e| match e {
        Entity::PointCloud(_) => true,
        Entity::Raster(r) => r.raster.url.is_some(),
        _ => false,
    }) {
        return SCHEMA_WITH_POINT_CLOUDS;
    }
    // Schema 30 (docs/adr/0205): annotation heights, a dimension's lines, a
    // leader's arrowhead size or one of the DXF arrowheads.
    let annotated = |list: Objs<'_>| {
        list.iter().any(|e| match e {
            Entity::Dimension(d) => d.look.has_lines(),
            Entity::Leader(l) => l.arrow_size.is_some() || l.arrow.is_some_and(|a| a.is_added()),
            _ => false,
        })
    };
    if has_heights(&doc.settings)
        || doc
            .settings
            .dimension_styles
            .iter()
            .any(DimensionStyleDef::has_lines)
        || annotated(objs)
        || doc
            .blocks
            .iter()
            .any(|b| annotated(Objs::Plain(&b.entities)))
    {
        return SCHEMA_WITH_ANNOTATION;
    }
    fn schemas(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| !n.fields.is_empty() || schemas(&n.children))
    }
    // A raster is the drawing's only (docs/adr/0204 §2).
    if objs.iter().any(|e| matches!(e, Entity::Raster(_))) {
        return SCHEMA_WITH_RASTERS;
    }
    if doc
        .settings
        .survey
        .as_ref()
        .is_some_and(SurveySettings::has_sigmas)
    {
        return SCHEMA_WITH_SURVEY_SIGMAS;
    }
    if doc.settings.topology.is_some() {
        return SCHEMA_WITH_TOPOLOGY;
    }
    if schemas(&doc.layers) {
        return SCHEMA_WITH_LAYER_FIELDS;
    }
    let line_parts = |list: Objs<'_>| {
        list.iter().any(|e| match e {
            Entity::Polyline(p) => p.parts.is_some(),
            Entity::Point(p) => p.parts.is_some(),
            _ => false,
        })
    };
    let paragraphs = |list: Objs<'_>| {
        list.iter()
            .any(|e| matches!(e, Entity::Text(t) if !t.paragraph.is_plain()))
    };
    let styled = |list: Objs<'_>| {
        list.iter().any(|e| match e {
            Entity::Text(t) => !t.face.is_plain(),
            Entity::Dimension(d) => !d.look.is_plain(),
            _ => false,
        })
    };
    let patterned = |list: Objs<'_>| {
        list.iter().any(
            |e| matches!(e, Entity::Hatch(h) if h.pattern.has_definition() || h.assoc.is_some()),
        )
    };
    let curved = |list: Objs<'_>| {
        list.iter()
            .any(|e| matches!(e, Entity::Text(t) if t.path.is_some()))
    };
    if curved(objs) || doc.blocks.iter().any(|b| curved(Objs::Plain(&b.entities))) {
        return SCHEMA_WITH_TEXT_PATHS;
    }
    // A picture is the drawing's only (docs/adr/0192 §1).
    if objs.iter().any(|e| matches!(e, Entity::Image(_))) {
        return SCHEMA_WITH_IMAGES;
    }
    if patterned(objs)
        || doc
            .blocks
            .iter()
            .any(|b| patterned(Objs::Plain(&b.entities)))
    {
        return SCHEMA_WITH_HATCH_PATTERNS;
    }
    if objs.iter().any(|e| matches!(e, Entity::Table(_))) {
        return SCHEMA_WITH_TABLES;
    }
    if !doc.settings.text_styles.is_empty()
        || !doc.settings.dimension_styles.is_empty()
        || styled(objs)
        || doc.blocks.iter().any(|b| styled(Objs::Plain(&b.entities)))
    {
        return SCHEMA_WITH_STYLES;
    }
    if paragraphs(objs)
        || doc
            .blocks
            .iter()
            .any(|b| paragraphs(Objs::Plain(&b.entities)))
    {
        return SCHEMA_WITH_PARAGRAPHS;
    }
    if !doc.settings.layer_states.is_empty() {
        return SCHEMA_WITH_LAYER_STATES;
    }
    // A block definition's texts have no link: only the drawing's (docs/adr/0175 §4).
    if objs
        .iter()
        .any(|e| matches!(e, Entity::Text(t) if t.label_of.is_some() || t.label_scale.is_some()))
    {
        return SCHEMA_WITH_LINKED_TEXTS;
    }
    if line_parts(objs)
        || doc
            .blocks
            .iter()
            .any(|b| line_parts(Objs::Plain(&b.entities)))
    {
        return SCHEMA_WITH_LINE_PARTS;
    }
    let s = &doc.settings;
    if s.survey.as_ref().is_some_and(SurveySettings::has_ground) {
        return SCHEMA_WITH_GROUND;
    }
    if s.survey.as_ref().is_some_and(SurveySettings::has_traverse) {
        return SCHEMA_WITH_TRAVERSE_TOLERANCES;
    }
    if s.survey.is_some() {
        return SCHEMA_WITH_SURVEY;
    }
    if s.custom_crs.is_some() || s.second_custom_crs.is_some() || !s.datum_transforms.is_empty() {
        return SCHEMA_WITH_CUSTOM_CRS;
    }
    if doc.settings.second_srid.is_some() {
        return SCHEMA_WITH_SECOND_SRID;
    }
    if doc.settings.drawing_unit.is_some() {
        return SCHEMA_WITH_DRAWING_UNIT;
    }
    if snaps(&doc.layers) {
        return SCHEMA_WITH_LAYER_SNAP;
    }
    let dimensions = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d) if d.mask
                || d.za.is_some()
                || d.zb.is_some()
                || d.style.is_some_and(DimensionStyle::is_schema_9))
        })
    };
    if dimensions(objs)
        || doc
            .blocks
            .iter()
            .any(|b| dimensions(Objs::Plain(&b.entities)))
    {
        return SCHEMA_WITH_DIMENSIONS;
    }
    let leaders = |list: Objs<'_>| list.iter().any(|e| matches!(e, Entity::Leader(_)));
    if leaders(objs) || doc.blocks.iter().any(|b| leaders(Objs::Plain(&b.entities))) {
        return SCHEMA_WITH_LEADERS;
    }
    let extras = |list: Objs<'_>| {
        list.iter().any(|e| {
            matches!(e, Entity::Text(t) if t.align.is_some() || t.width_factor.is_some() || t.mask)
        })
    };
    if extras(objs)
        || doc.blocks.iter().any(|b| {
            extras(Objs::Plain(&b.entities))
                || b.attributes
                    .iter()
                    .any(|a| a.align.is_some() || a.width_factor.is_some())
        })
    {
        return SCHEMA_WITH_TEXT_EXTRAS;
    }
    if !doc.blocks.is_empty() {
        return SCHEMA_WITH_BLOCKS;
    }
    let mut schema = DOCUMENT_VERSION_2;
    for e in objs.iter() {
        if matches!(e, Entity::Polygon(p) if p.parts.is_some()) {
            // The newest: nothing later in the drawing can change it.
            return SCHEMA_WITH_PARTS;
        }
        if has_elevation(e) {
            schema = SCHEMA_WITH_ELEVATIONS;
        } else if e.base().line_weight.is_some() {
            schema = schema.max(SCHEMA_WITH_LINE_WEIGHTS);
        }
    }
    schema
}

/// Whether an object has a vertex measure (a line's end, a polyline's or a
/// part's `ms`, even one whose every vertex is without): schema 39's fields.
pub(super) fn has_measure(e: &Entity) -> bool {
    match e {
        Entity::Line(l) => l.ma.is_some() || l.mb.is_some(),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            p.ms.is_some()
                || p.parts
                    .as_ref()
                    .is_some_and(|parts| parts.iter().any(|q| q.ms.is_some()))
        }
        _ => false,
    }
}

/// Whether an object has a vertex elevation (a line's end, a path's or a
/// hole's `zs`, even one whose every vertex is without): schema 4's fields.
pub(super) fn has_elevation(e: &Entity) -> bool {
    match e {
        Entity::Line(l) => l.za.is_some() || l.zb.is_some(),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            p.zs.is_some()
                || p.holes
                    .as_ref()
                    .is_some_and(|holes| holes.iter().any(|h| h.zs.is_some()))
        }
        _ => false,
    }
}

/// Whether an object needs schema 138 (docs/adr/0771): a wall with a turning component or a node decision, a bound
/// slab.
pub(crate) fn building_parts(e: &Entity) -> bool {
    match e {
        Entity::Wall(w) => {
            w.wall.components.iter().any(|c| c.wrap)
                || w.wall.joins.start_pass.is_some()
                || w.wall.joins.end_pass.is_some()
        }
        Entity::Slab(s) => s.slab.bound.is_some(),
        _ => false,
    }
}

/// Whether an object needs schema 147 (docs/adr/0784): a schedule with a share field.
pub(crate) fn program_shares(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Table(t) if matches!(
            &t.source,
            Some(kentos_contracts::TableSource::Schedule { columns, .. }) if columns.iter().any(|c| c.field.is_share())
        )
    )
}

/// Whether an object needs schema 151 (docs/adr/0795): a schedule of the program's units, a units field or filter, its
/// rows' units.
pub(crate) fn unit_schedules(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Table(t) if matches!(
            &t.source,
            Some(kentos_contracts::TableSource::Schedule { of, columns, filter, units, .. })
                if kentos_contracts::schedule_units(*of, columns, filter.as_ref(), units)
        )
    )
}

/// Whether an object needs schema 148 (docs/adr/0785): a wall component or a slab bound to a project material.
pub(crate) fn materials(e: &Entity) -> bool {
    match e {
        Entity::Wall(w) => w.wall.components.iter().any(|c| c.material_id.is_some()),
        Entity::Slab(s) => s.slab.material_id.is_some(),
        _ => false,
    }
}

/// Whether an object needs schema 140 (docs/adr/0773): a space in a group, a schedule with a program field.
pub(crate) fn building_program(e: &Entity) -> bool {
    match e {
        Entity::Space(s) => s.space.unit.is_some(),
        Entity::Table(t) => matches!(
            &t.source,
            Some(kentos_contracts::TableSource::Schedule { columns, .. }) if columns.iter().any(|c| c.field.is_program())
        ),
        _ => false,
    }
}
