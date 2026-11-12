//! What a payload's schema lets an object hold (`Features`): one flag a schema, and the fields each kind
//! may carry under it.

use super::*;

/// What a payload's schema lets an object hold beyond schema 2's fields.
#[derive(Clone, Copy)]
pub(crate) struct Features {
    /// Schema 3 and up: an object's own line weight (`lineWeight`).
    pub(super) weights: bool,
    /// Schema 4 and up: the vertex elevations (`za`, `zb`, `zs`).
    pub(super) elevations: bool,
    /// Schema 5 and up: an area's parts (`parts`).
    pub(super) parts: bool,
    /// Schema 6 and up: block definitions and the `insert` kind.
    pub(crate) blocks: bool,
    /// Schema 7 and up: a text's alignment, width factor and mask, an
    /// attribute definition's alignment and width factor.
    pub(crate) texts: bool,
    /// Schema 8 and up: the `leader` kind.
    pub(super) leaders: bool,
    /// Schema 9: the dimension's new kinds, `mask`, `za` and `zb`.
    pub(super) dimensions: bool,
    /// Schema 10: a layer's own snapping (`snap`).
    pub(crate) layer_snap: bool,
    /// Schema 11: a local project's drawing unit (the settings' `drawingUnit`).
    pub(crate) drawing_unit: bool,
    /// Schema 12: the project's second coordinate system (the settings' `secondSrid`).
    pub(crate) second_srid: bool,
    /// Schema 13: the project's own systems and datum choices (the settings'
    /// `customCrs`, `secondCustomCrs`, `datumTransforms`).
    pub(crate) custom_crs: bool,
    /// Schema 14: the project's survey settings (the settings' `survey`).
    pub(crate) survey: bool,
    /// Schema 15: the survey settings' traverse tolerances.
    pub(crate) traverse_tolerances: bool,
    /// Schema 16: the survey settings' ground height and reduction to the grid.
    pub(crate) ground: bool,
    /// Schema 19: the settings' layer states (docs/adr/0177 §4).
    pub(crate) layer_states: bool,
    /// Schema 17: a polyline's and a point's parts (`parts`, docs/adr/0174).
    pub(crate) line_parts: bool,
    /// Schema 18: a text's link to the object whose label it writes
    /// (`labelOf`, `labelScale`, docs/adr/0175 §4).
    pub(crate) linked_texts: bool,
    /// Schema 20: a multi-line text's `boxWidth`, `lineSpacing` and `runs` (docs/adr/0182).
    pub(crate) paragraphs: bool,
    /// Schema 21: the settings' text and dimension styles, a text's face and a
    /// dimension's look (docs/adr/0183).
    pub(crate) styles: bool,
    /// Schema 22: the `table` kind (docs/adr/0184).
    pub(super) tables: bool,
    /// Schema 23: a hatch's pattern and gradient fields and its tie (docs/adr/0186).
    pub(super) hatches: bool,
    /// Schema 24: the `image` kind (docs/adr/0192).
    pub(super) images: bool,
    /// Schema 25: a text's curve, `path` (docs/adr/0196).
    pub(super) text_paths: bool,
    /// Schema 26: a layer's fields, `fields` (docs/adr/0199 §1).
    pub(crate) layer_fields: bool,
    /// Schema 27: the settings' topology rules (docs/adr/0202 §7).
    pub(crate) topology: bool,
    /// Schema 28: the survey settings' a priori standard deviations (docs/adr/0203 §1).
    pub(crate) survey_sigmas: bool,
    /// Schema 29: the `raster` kind (docs/adr/0204 §2).
    pub(super) rasters: bool,
    /// Schema 31: the `pointcloud` kind and a raster's `url` (docs/adr/0207).
    pub(super) point_clouds: bool,
    /// Schema 32: a layer node's `service` and `feed`, the settings'
    /// `connections` (docs/adr/0208 §2).
    pub(crate) services: bool,
    /// Schema 33: the settings' `networks` (docs/adr/0209 §2).
    pub(crate) networks: bool,
    /// Schema 34: a layer node's `time`, `scenario` and `replaces` (docs/adr/0210 §2).
    pub(crate) temporal: bool,
    /// Schema 35: a layer node's `filter` (docs/adr/0211 §2).
    pub(crate) filters: bool,
    /// Schema 36: the label engine's fields, a layer style's `labels`, an object's `labelPins` (docs/adr/0212 §2).
    pub(crate) labels: bool,
    /// Schema 37: the settings' `variables` (docs/adr/0214 §2.3).
    pub(crate) variables: bool,
    /// Schema 38: a raster's `dataset` and its look's `edges` (docs/adr/0243 §6).
    pub(super) multidim: bool,
    /// Schema 39: the vertex measures (`ma`, `mb`, a polyline's and its parts' `ms`; docs/adr/0224).
    pub(super) measures: bool,
    /// Schema 40: an object's `attachments`, a layer's `actions` and `card` (docs/adr/0225).
    pub(crate) attachments: bool,
    /// Schema 41: a layer's `compose` (docs/adr/0226 §3, §4).
    pub(crate) compose: bool,
    /// Schema 42: the settings' `addressLocators` (docs/adr/0229 §2).
    pub(crate) addresses: bool,
    /// Schema 43: a dimension's `refs` (docs/adr/0249 §4).
    pub(super) dimension_refs: bool,
    /// Schema 44: the document's `sheets` (docs/adr/0251).
    pub(crate) sheets: bool,
    /// Schema 45: the document's `requests` (docs/adr/0253 §6).
    pub(crate) requests: bool,
    /// Schema 46: a polyline's `closed` and `widths` (docs/adr/0258).
    pub(super) polyline_extras: bool,
    /// Schema 47: the NURBS curve (docs/adr/0259).
    pub(super) nurbs: bool,
    /// Schema 75: the wall and the settings' `wallStyles` and `arch` (docs/adr/0339).
    pub(crate) walls: bool,
    pub(crate) wall_joins: bool,
    /// Schema 77: the opening (docs/adr/0341).
    pub(super) openings: bool,
    /// Schema 78: the space (docs/adr/0342).
    pub(crate) spaces: bool,
    pub(super) space_columns: bool,
    pub(super) space_scope: bool,
    pub(super) space_sources: bool,
    pub(super) space_separators: bool,
    pub(crate) space_choices: bool,
    pub(crate) space_voids: bool,
    pub(crate) dimension_sets: bool,
    /// Schema 114: a dimension recipe's edge (docs/adr/0700).
    pub(crate) dimension_set_edges: bool,
    /// Schema 115: a stair's and a ramp's railings (docs/adr/0705).
    pub(super) railings: bool,
    /// Schema 117: a curved stair's arc (docs/adr/0709).
    pub(super) stair_arcs: bool,
    /// Schema 118: a dimension's references' rule `cross` (docs/adr/0711).
    pub(super) dimension_cross: bool,
    /// Schema 119: a niche, a flue, an opening's depth and inset and the break `tooDeep` (docs/adr/0719).
    pub(super) recesses: bool,
    /// Schema 120: the architectural settings' active level and level view (docs/adr/0720).
    pub(crate) level_state: bool,
    /// Schema 121: a Kot's level (`level`, docs/adr/0721 §2).
    pub(super) dimension_levels: bool,
    /// Schema 122: a block schedule's further columns (`columns`, docs/adr/0721 §4).
    pub(super) schedule_columns: bool,
    /// Schema 123: the slab kind (docs/adr/0722).
    pub(super) slabs: bool,
    /// Schema 124: a ramp bound to its level (`baseOffset`, docs/adr/0723 §2).
    pub(super) ramp_levels: bool,
    /// Schema 125: a stair's `heightFromLevel` and `post` (docs/adr/0724).
    pub(super) stair_levels: bool,
    /// Schema 126: a grid's `ends`, a column's `anchor`, an L column (docs/adr/0725).
    pub(super) grid_column_forms: bool,
    /// Schema 127: the kitchen runs (docs/adr/0727).
    pub(crate) casework: bool,
    /// Schema 128: a façade's or a space's dimension recipe (docs/adr/0729).
    pub(crate) dimension_set_spans: bool,
    /// Schema 129: a dimension recipe's sheet scales and its values' places (docs/adr/0730).
    pub(crate) dimension_set_scales: bool,
    /// Schema 130 (docs/adr/0732 §3): Mimari denetim's exceptions.
    pub(crate) audit_exceptions: bool,
    /// Schema 131 (docs/adr/0733): a block insert's level and phase, a space's skirting finish, a schedule's fixtures,
    /// new fields, type filter, headings and subtotals.
    pub(crate) fixtures: bool,
    /// Schema 132 (docs/adr/0735): the data profiles and mappings the project carries.
    pub(crate) profile_files: bool,
    pub(crate) standard_docs: bool,
    /// Schema 180 (docs/adr/1000 §2): review notes.
    pub(crate) review_notes: bool,
    /// Schema 181 (docs/adr/1001 §1): patterns past 12 elements.
    pub(crate) long_patterns: bool,
    /// Schema 184 (docs/adr/1120 §4): a text style's Big Font file.
    pub(crate) shx_fonts: bool,
    /// Schema 185 (docs/adr/1126 §2–§3): the project's table styles and a table's `tableStyle`.
    pub(crate) table_styles: bool,
    /// Schema 186 (docs/adr/1023): a dimension style's text style.
    pub(crate) dimension_text_styles: bool,
    /// Schema 187 (docs/adr/1025): a leader style's text style.
    pub(crate) leader_text_styles: bool,
    /// Schema 188 (docs/adr/1200 §2): a map project's work structure.
    pub(crate) map_work: bool,
    pub(crate) map_work_relative: bool,
    /// Schema 194 (docs/adr/1340 §5): a map project's GNSS archive.
    pub(crate) map_survey: bool,
    /// Schema 195 (docs/adr/1520): a map project's terrain surfaces.
    pub(crate) map_surfaces: bool,
    /// Schema 196 (docs/adr/1560): a terrain surface's edits.
    pub(crate) surface_edits: bool,
    /// Schema 197 (docs/adr/1580): a map project's graded pads.
    pub(crate) map_platforms: bool,
    /// Schema 198 (docs/adr/1720): a map project's drawings kept with their sources' stamps.
    pub(crate) map_drawings: bool,
    /// Schema 199 (docs/adr/1740): a GNSS archive's baselines, known points and network adjustments.
    pub(crate) gnss_network: bool,
    /// Schema 190 (docs/adr/1129): the accepted standards findings.
    pub(crate) standard_exceptions: bool,
    /// Schema 79: the grid, column, stair and ramp (docs/adr/0343).
    pub(super) arch_elements: bool,
    /// Schema 80: the tag and a table's schedule of the drawing's objects (docs/adr/0344).
    pub(super) tags: bool,
    /// Schema 81: the view mark and a named view's id (docs/adr/0345).
    pub(crate) viewmarks: bool,
    /// Schema 82: the architectural settings' phase view (docs/adr/0346).
    pub(crate) phases: bool,
    pub(crate) xrefs: bool,
    pub(crate) xref_paths: bool,
    pub(crate) join_views: bool,
    pub(crate) linear_events: bool,
    pub(crate) lineage: bool,
    /// Schema 98: a layer's source system and the map's system (docs/adr/0583).
    pub(crate) source_crs: bool,
    /// Schema 102: a layer's type and the project's Otomatik seçim (docs/adr/0622 §5).
    pub(crate) layer_geometry: bool,
    /// Schema 99: a topology exception's geometry digest (docs/adr/0589).
    pub(crate) exception_digests: bool,
    /// Schema 189: the topology quality profile and an exception's review fields (docs/adr/1260).
    pub(crate) topology_reviews: bool,
    /// Schema 100: a georeference record's `crs` (docs/adr/0594 §1).
    pub(crate) georef_crs: bool,
    /// Schema 90: a layer time's system time attributes (docs/adr/0537).
    pub(crate) recorded_time: bool,
    pub(crate) source_bases: bool,
    /// Schema 92: a source base's field records (docs/adr/0553 §1).
    pub(crate) source_field_bases: bool,
    /// Schema 85: an insert's clipping and a PDF underlay's page and look (docs/adr/0351).
    pub(crate) clipping: bool,
    /// Schema 86: the settings' heights and coordinate epoch (docs/adr/0519).
    pub(crate) heights: bool,
    /// Schema 87: a network's turns and levels, a speed cost's periods.
    pub(crate) network_turns: bool,
    /// Schema 93: a network's terminals and subnetworks (docs/adr/0557).
    pub(crate) network_terminals: bool,
    pub(crate) network_configs: bool,
    pub(crate) network_profiles: bool,
    /// Schema 96: a closure's weekdays and hours (docs/adr/0568).
    pub(crate) timed_closures: bool,
    /// Schema 88: a raster's georeference record (docs/adr/0518 §1).
    pub(crate) georef: bool,
    /// Schema 103: a raster's GeoPackage tile table (docs/adr/0627 §3).
    pub(crate) raster_tiles: bool,
    /// Schema 104: a layer field's kinds of docs/adr/0629 §2.
    pub(crate) field_kinds: bool,
    pub(crate) withheld_fields: bool,
    /// Schema 133: a layer field's bound column, what the table makes and refers to (docs/adr/0753).
    pub(crate) field_columns: bool,
    pub(crate) table_constraints: bool,
    /// Schema 135 (docs/adr/0801): what a CAD project is drawn for.
    pub(crate) cad_work: bool,
    /// Schema 159 (plan/road.md Y1.1): `cadWork` may be `road`.
    pub(crate) road_work: bool,
    /// Schema 166: a road template's pavement and a volume's rule (docs/adr/0975 §2).
    pub(crate) road_pavement: bool,
    /// Schema 167: a superelevation's axis of rotation (docs/adr/0931 §2).
    pub(crate) road_pivot: bool,
    /// Schema 168: a sample line set's clip, breaks method and curve step (docs/adr/0933 §3).
    pub(crate) section_breaks: bool,
    /// Schema 169: an earthwork volume's curve correction (docs/adr/0934 §2).
    pub(crate) volume_curve: bool,
    /// Schema 170: a corridor's vertical chord height and sampling check (docs/adr/0939).
    pub(crate) corridor_sampling: bool,
    /// Schema 171: a mass haul diagram's unsuitable cut and barriers (docs/adr/0944).
    pub(crate) haul_sorting: bool,
    /// Schema 172: a profile's crossings (docs/adr/0946).
    pub(crate) profile_crossings: bool,
    /// Schema 173: a volume's numbered approvals and their history (docs/adr/0950).
    pub(crate) volume_history: bool,
    /// Schema 174: a mass haul diagram's approved plan and its history (docs/adr/0954).
    pub(crate) haul_approval: bool,
    /// Schema 175: a mass haul diagram's haul road network and its sites off the road (docs/adr/0955).
    pub(crate) haul_network: bool,
    /// Schema 176: a volume factor row's densities and source (docs/adr/0957).
    pub(crate) volume_states: bool,
    /// Schema 177: a template side's cut slope benches (docs/adr/0958).
    pub(crate) cut_benches: bool,
    /// Schema 178: a corridor region's slope transition (docs/adr/0960).
    pub(crate) slope_transitions: bool,
    /// Schema 179: an earthwork volume on a grid and its cell (docs/adr/0974).
    pub(crate) volume_grid: bool,
    /// Schema 191: a road project's grades as rise over run, percent before (docs/adr/0977).
    pub(crate) road_grades: bool,
    /// Schema 192: an alignment PI's ove (docs/adr/0979).
    pub(crate) road_ove: bool,
    /// Schema 200: an alignment's design speed and a PI's own (docs/adr/0985).
    pub(crate) road_speed: bool,
    /// Schema 201: an alignment's places beyond its ends (docs/adr/1028).
    pub(crate) road_places: bool,
    /// Schema 202: a cloud record's vertical system (docs/adr/1029).
    pub(crate) road_cloud_heights: bool,
    /// Schema 203: a cloud record's heights as the call said them (docs/adr/1032).
    pub(crate) road_cloud_conversions: bool,
    /// Schema 204: a volume approval's sources and algorithm (docs/adr/1041).
    pub(crate) road_approval_sources: bool,
    /// Schema 205: two surfaces' cut and fill zones kept as a drawing (docs/adr/1840).
    pub(crate) cutfill_drawings: bool,
    /// Schema 212: a pipe run profile kept as a drawing, a network its source (docs/adr/3560).
    pub(crate) pipe_profile_drawings: bool,
    pub(crate) sheet_contour_drawings: bool,
    pub(crate) work_stamps: bool,
    /// Schema 207: a known point's own velocity (docs/adr/2520).
    pub(crate) known_velocities: bool,
    /// Schema 248: a GNSS archive's baseline sessions, kept limits, a network point's velocity (docs/adr/4920).
    pub(crate) gnss_sessions: bool,
    /// Schema 249: the parcel register's stated lengths, drafts and approvals (docs/adr/5020).
    pub(crate) parcel_drafts: bool,
    /// Schema 251: a point cloud file's place in the drawing (docs/adr/5100).
    pub(crate) cloud_poses: bool,
    /// Schema 252: the adjustments the project keeps (docs/adr/5200).
    pub(crate) kept_adjustments: bool,
    /// Schema 253: the reviews the project keeps of its monitoring jobs (docs/adr/5220).
    pub(crate) monitoring_reviews: bool,
    /// Schema 209: a road drawing's record with its version and revision (docs/adr/2840).
    pub(crate) drawing_provenance: bool,
    /// Schema 210: a map work approval's objects' digests (docs/adr/2880).
    pub(crate) approved_objects: bool,
    /// Schema 211: a map work approval's objects' parts' digests (docs/adr/3280).
    pub(crate) approved_parts: bool,
    /// Schema 213: a map work approval's objects' values (docs/adr/3680).
    pub(crate) approved_values: bool,
    /// Schema 247: a map work item's typed layers' profile (docs/adr/4880).
    pub(crate) work_profiles: bool,
    /// Schema 214: a terrain surface's edit that pastes another surface (docs/adr/3920).
    pub(crate) surface_pastes: bool,
    /// Schema 215: a review note's decision (docs/adr/4140).
    pub(crate) review_decisions: bool,
    /// Schema 216: the parcel register (docs/adr/4220).
    pub(crate) parcel_register: bool,
    /// Schema 250: a zoning plan's kept symbol choices and plan change drafts (docs/adr/5040).
    pub(crate) zoning_drafts: bool,
    /// Schema 208: a constraint naming a boundary edge, a map bond (docs/adr/2580).
    pub(crate) boundary_bonds: bool,
    /// Schema 217 (docs/adr/8012): the project's plan cut height.
    pub(crate) plan_cut: bool,
    /// Schema 218 (docs/adr/8013): how the project's plan lines are weighted.
    pub(crate) line_groups: bool,
    /// Schema 182: a table's parts, cells of several lines and a data extraction table (docs/adr/1030).
    pub(crate) extraction: bool,
    /// Schema 183 (docs/adr/1100 §3): the approved sheet template revisions.
    pub(crate) approved_sheet_templates: bool,
    /// Schema 136 (docs/adr/0758): a layer's indexes.
    pub(crate) layer_indexes: bool,
    /// Schema 137 (docs/adr/0770): constraints naming a wall's line or a jamb.
    pub(crate) arch_bonds: bool,
    pub(crate) arc_edge_bonds: bool,
    pub(crate) block_publications: bool,
    pub(crate) block_parameter_groups: bool,
    pub(crate) block_type_values: bool,
    /// Schema 138 (docs/adr/0771): turning wall components, node decisions, bound slabs.
    pub(crate) building_parts: bool,
    /// Schema 148 (docs/adr/0785): the project's materials, a component's or a slab's binding.
    pub(crate) materials: bool,
    /// Schema 150 (docs/adr/0792): three- and four-leaf sliding doors, their tracks, a schedule of them.
    pub(crate) multi_slide: bool,
    pub(crate) opening_types: bool,
    pub(crate) special_doors: bool,
    pub(crate) ellipse_tags: bool,
    pub(crate) opening_codes: bool,
    pub(crate) paiy_areas: bool,
    pub(crate) zoning: bool,
    /// Schema 223 (docs/adr/8031): kat irtifakı's program fields and schedule columns.
    pub(crate) land_shares: bool,
    /// Schema 224 (docs/adr/8032): the site plan.
    pub(crate) site_plan: bool,
    /// Schema 225 (docs/adr/8034): the municipality's regulation.
    pub(crate) local_rules: bool,
    /// Schema 226 (docs/adr/8035): the building's sprinkler system.
    pub(crate) sprinklers: bool,
    /// Schema 227 (docs/adr/8036): Otopark and Sığınak hesabı.
    pub(crate) provision_tables: bool,
    /// Schema 228 (docs/adr/8038): the room list's finishes and columns.
    pub(crate) room_list: bool,
    /// Schema 229 (docs/adr/8038): a door or window type's frame and leaves and its U value.
    pub(crate) opening_specs: bool,
    /// Schema 230 (docs/adr/8039): a door's or window's own phase.
    pub(crate) opening_phases: bool,
    /// Schema 232 (docs/adr/8040 §2): a slab's layers besides its structure.
    pub(crate) slab_layers: bool,
    /// Schema 233 (docs/adr/8040 §3): a room's floor offset and asma tavan.
    pub(crate) room_heights: bool,
    /// Schema 234 (docs/adr/8041 §1): beams.
    pub(crate) beams: bool,
    pub(crate) roofs: bool,
    pub(crate) roof_parts: bool,
    /// Schema 239 (docs/adr/8044 §8): a roof's gutter and snow guard.
    pub(crate) roof_edges: bool,
    /// Schema 240 (docs/adr/8044 §8): a roof's downpipes.
    pub(crate) roof_downpipes: bool,
    /// Schema 241 (docs/adr/8045 §1): a balanced stair's winders.
    pub(crate) stair_winders: bool,
    /// Schema 242 (docs/adr/8045 §3): free-standing railings.
    pub(crate) railing_objects: bool,
    /// Schema 243 (docs/adr/8049 §2): a constraint naming a grid's line.
    pub(crate) grid_bonds: bool,
    /// Schema 244 (docs/adr/8049 §3): a constraint naming a column's side.
    pub(crate) column_side_bonds: bool,
    /// Schema 245 (docs/adr/8050): a room module's new kinds and its kitchen runs.
    pub(crate) module_scope: bool,
    /// Schema 235 (docs/adr/8041 §2): a column's material.
    pub(crate) column_material: bool,
    /// Schema 236 (docs/adr/8041 §3): a beam's role and base.
    pub(crate) beam_roles: bool,
    /// Schema 231 (docs/adr/8039 §5): the Tadilat projesi view.
    pub(crate) renovation_view: bool,
    /// Schema 140 (docs/adr/0773): the building program, a space's group, a schedule's program fields.
    pub(crate) building_program: bool,
    /// Schema 141 (docs/adr/0774): a named view's recipe.
    pub(crate) view_recipes: bool,
    /// Schema 142 (docs/adr/0775): room modules.
    pub(crate) modules: bool,
    pub(crate) recipe_detail: bool,
    /// Schema 145 (docs/adr/0779): a module instance's group.
    pub(crate) module_units: bool,
    /// Schema 152 (docs/adr/0796): a module member's storey.
    pub(crate) module_storeys: bool,
    /// Schema 153 (docs/adr/0807): an XOX slide, two unequal sliding leaves.
    pub(crate) slide_more: bool,
    /// Schema 154 (docs/adr/0808): combinations of design options, a recipe's combination.
    pub(crate) option_combos: bool,
    pub(crate) instance_storeys: bool,
    /// Schema 146 (docs/adr/0783): view templates and a recipe's pens, black and white and template.
    pub(crate) view_templates: bool,
    /// Schema 147 (docs/adr/0784): the program's shares, levels of buildings and wings, a schedule's share fields.
    pub(crate) program_shares: bool,
    /// Schema 149 (docs/adr/0788): design option sets and a recipe's options.
    pub(crate) options: bool,
    /// Schema 151 (docs/adr/0795): a schedule of the program's units.
    pub(crate) unit_schedules: bool,
    /// Schema 105: a join view's source table and summing columns (docs/adr/0632 §4).
    pub(crate) join_sources: bool,
    /// Schema 106: a layer's dataset description and derivation record (docs/adr/0633 §3).
    pub(crate) datasets: bool,
    /// Schema 51: a reference's `along` place and block `path` (docs/adr/0263).
    pub(super) subref_extras: bool,
    /// Schema 54: an object's `wipeout` (docs/adr/0310).
    pub(super) wipeouts: bool,
    /// Schema 55: a layer's `frozen`, a layer state node's `frozen` and `plot` (docs/adr/0311 §1, §3).
    pub(crate) frozen: bool,
    /// Schema 56: the settings' `lineTypes` and `lineTypeScale`, a layer style's and an object's `pattern`
    /// (docs/adr/0313 §1).
    pub(crate) patterns: bool,
    /// Schema 57: an insert's `yScale` and a block definition's `units` (docs/adr/0315 §3, §4).
    pub(crate) block_extras: bool,
    /// Schema 58: an attribute definition's options, an insert's `attributePlaces` and a block's `card`
    /// (docs/adr/0317 §1, §4).
    pub(crate) block_cards: bool,
    /// Schema 59: a block definition's `dynamic` and an insert's `parameters` (docs/adr/0318).
    pub(crate) dynamic_blocks: bool,
    /// Schema 60: a dimension's references' `rule` (docs/adr/0320 §1).
    pub(super) dimension_rules: bool,
    /// Schema 61: a dimension's `options`, `textShift` and `breaks` (docs/adr/0321).
    pub(crate) dimension_options: bool,
    /// Schema 62: a dimension's kinds `centerMark`, `centerLine`, `elevation` and the rule `pair` (docs/adr/0322).
    pub(super) dimension_marks: bool,
    /// Schema 63: a text's and a dimension's `overrides` (docs/adr/0322 §5).
    pub(super) overrides: bool,
    /// Schema 64: a text's, a dimension's and a leader's `annotative` (docs/adr/0323).
    pub(super) annotative: bool,
    /// Schema 65: a text's `layout` and a run's `stack` script (docs/adr/0324).
    pub(super) paragraph_layout: bool,
    pub(super) text_engine: bool,
    /// Schema 66: a text's `fields` (docs/adr/0325).
    pub(super) text_fields: bool,
    /// Schema 67: a leader's `arms`, `tip`, `blocks` and `leaderStyle` (docs/adr/0326).
    pub(crate) multileader: bool,
    /// Schema 68: a table's `styles`, `formulas`, `fields`, `link` and `schedule` (docs/adr/0327).
    pub(crate) table_cells: bool,
    /// Schema 69: a hatch's `boundary` and its tie's `bounds` (docs/adr/0328).
    pub(crate) hatch_boundaries: bool,
    /// Schema 70: a face's tie's `gap` (docs/adr/0329 §3).
    pub(crate) hatch_gaps: bool,
    pub(crate) hatch_extras: bool,
    /// Schema 72: the settings' `forms` and a layer field's `id` (docs/adr/0401).
    pub(crate) forms: bool,
    /// Schema 73: the document's `constraints` (docs/adr/0334).
    pub(crate) constraints: bool,
    /// Schema 74: the settings' `sources` (docs/adr/0503).
    pub(crate) sources: bool,
    /// Schema 48: an object's `lineType`, `lineTypeScale`, `transparency`,
    /// `order` and a block's object's `byLayer` (docs/adr/0260).
    pub(crate) look: bool,
    /// Schema 49: the settings' `ucs` and `namedUcs` (docs/adr/0261).
    pub(crate) ucs: bool,
    /// Schema 52: the settings' `namedViews` (docs/adr/0285).
    pub(crate) views: bool,
    /// Schema 53: the document's `arrays` (docs/adr/0300).
    pub(crate) arrays: bool,
    /// Schema 50: the drawing units `in` and `ft` and the settings' `lengthStyle` (docs/adr/0262).
    pub(crate) imperial: bool,
    /// Schema 30: the settings' annotation heights, a dimension's and a
    /// dimension style's line fields, a leader's `arrowSize` and the DXF
    /// arrowheads (docs/adr/0205).
    pub(crate) annotation: bool,
    /// Whether an object has its persistent id (`uid`): the drawing's do, a
    /// block definition's do not.
    pub(super) uids: bool,
}

impl Features {
    pub(crate) fn of(schema: u32) -> Self {
        Self {
            weights: schema >= SCHEMA_WITH_LINE_WEIGHTS,
            elevations: schema >= SCHEMA_WITH_ELEVATIONS,
            parts: schema >= SCHEMA_WITH_PARTS,
            blocks: schema >= SCHEMA_WITH_BLOCKS,
            texts: schema >= SCHEMA_WITH_TEXT_EXTRAS,
            leaders: schema >= SCHEMA_WITH_LEADERS,
            dimensions: schema >= SCHEMA_WITH_DIMENSIONS,
            layer_snap: schema >= SCHEMA_WITH_LAYER_SNAP,
            drawing_unit: schema >= SCHEMA_WITH_DRAWING_UNIT,
            second_srid: schema >= SCHEMA_WITH_SECOND_SRID,
            custom_crs: schema >= SCHEMA_WITH_CUSTOM_CRS,
            survey: schema >= SCHEMA_WITH_SURVEY,
            traverse_tolerances: schema >= SCHEMA_WITH_TRAVERSE_TOLERANCES,
            ground: schema >= SCHEMA_WITH_GROUND,
            layer_states: schema >= SCHEMA_WITH_LAYER_STATES,
            line_parts: schema >= SCHEMA_WITH_LINE_PARTS,
            linked_texts: schema >= SCHEMA_WITH_LINKED_TEXTS,
            paragraphs: schema >= SCHEMA_WITH_PARAGRAPHS,
            styles: schema >= SCHEMA_WITH_STYLES,
            tables: schema >= SCHEMA_WITH_TABLES,
            hatches: schema >= SCHEMA_WITH_HATCH_PATTERNS,
            images: schema >= SCHEMA_WITH_IMAGES,
            text_paths: schema >= SCHEMA_WITH_TEXT_PATHS,
            layer_fields: schema >= SCHEMA_WITH_LAYER_FIELDS,
            topology: schema >= SCHEMA_WITH_TOPOLOGY,
            survey_sigmas: schema >= SCHEMA_WITH_SURVEY_SIGMAS,
            rasters: schema >= SCHEMA_WITH_RASTERS,
            point_clouds: schema >= SCHEMA_WITH_POINT_CLOUDS,
            services: schema >= SCHEMA_WITH_SERVICES,
            networks: schema >= SCHEMA_WITH_NETWORKS,
            temporal: schema >= SCHEMA_WITH_TEMPORAL,
            filters: schema >= SCHEMA_WITH_FILTERS,
            multidim: schema >= SCHEMA_WITH_MULTIDIM,
            measures: schema >= SCHEMA_WITH_MEASURES,
            attachments: schema >= SCHEMA_WITH_ATTACHMENTS,
            compose: schema >= SCHEMA_WITH_COMPOSE,
            addresses: schema >= SCHEMA_WITH_ADDRESSES,
            dimension_refs: schema >= SCHEMA_WITH_DIMENSION_REFS,
            sheets: schema >= SCHEMA_WITH_SHEETS,
            requests: schema >= SCHEMA_WITH_REQUESTS,
            polyline_extras: schema >= SCHEMA_WITH_POLYLINE_EXTRAS,
            look: schema >= crate::SCHEMA_WITH_LOOK,
            ucs: schema >= crate::SCHEMA_WITH_UCS,
            views: schema >= crate::SCHEMA_WITH_VIEWS,
            arrays: schema >= crate::SCHEMA_WITH_ARRAYS,
            wipeouts: schema >= crate::SCHEMA_WITH_WIPEOUTS,
            frozen: schema >= crate::SCHEMA_WITH_FROZEN,
            patterns: schema >= crate::SCHEMA_WITH_PATTERNS,
            block_extras: schema >= crate::SCHEMA_WITH_BLOCK_EXTRAS,
            block_cards: schema >= crate::SCHEMA_WITH_BLOCK_CARDS,
            dynamic_blocks: schema >= crate::SCHEMA_WITH_DYNAMIC_BLOCKS,
            dimension_rules: schema >= crate::SCHEMA_WITH_DIMENSION_RULES,
            dimension_options: schema >= crate::SCHEMA_WITH_DIMENSION_OPTIONS,
            dimension_marks: schema >= crate::SCHEMA_WITH_DIMENSION_MARKS,
            overrides: schema >= crate::SCHEMA_WITH_OVERRIDES,
            annotative: schema >= crate::SCHEMA_WITH_ANNOTATIVE,
            paragraph_layout: schema >= crate::SCHEMA_WITH_PARAGRAPH_LAYOUT,
            text_engine: schema >= crate::SCHEMA_WITH_TEXT_ENGINE,
            text_fields: schema >= crate::SCHEMA_WITH_TEXT_FIELDS,
            multileader: schema >= crate::SCHEMA_WITH_MULTILEADER,
            table_cells: schema >= crate::SCHEMA_WITH_TABLE_CELLS,
            hatch_boundaries: schema >= crate::SCHEMA_WITH_HATCH_BOUNDARIES,
            hatch_gaps: schema >= crate::SCHEMA_WITH_HATCH_GAPS,
            hatch_extras: schema >= crate::SCHEMA_WITH_HATCH_EXTRAS,
            forms: schema >= crate::SCHEMA_WITH_FORMS,
            constraints: schema >= crate::SCHEMA_WITH_CONSTRAINTS,
            sources: schema >= crate::SCHEMA_WITH_SOURCES,
            imperial: schema >= crate::SCHEMA_WITH_IMPERIAL,
            subref_extras: schema >= crate::SCHEMA_WITH_SUBREF_EXTRAS,
            nurbs: schema >= SCHEMA_WITH_NURBS,
            walls: schema >= crate::SCHEMA_WITH_WALLS,
            wall_joins: schema >= crate::SCHEMA_WITH_WALL_JOINS,
            openings: schema >= crate::SCHEMA_WITH_OPENINGS,
            spaces: schema >= crate::SCHEMA_WITH_SPACES,
            space_columns: schema >= crate::SCHEMA_WITH_SPACE_COLUMNS,
            space_scope: schema >= crate::SCHEMA_WITH_SPACE_SCOPE,
            space_sources: schema >= crate::SCHEMA_WITH_SPACE_SOURCES,
            space_separators: schema >= crate::SCHEMA_WITH_SPACE_SEPARATORS,
            space_choices: schema >= crate::SCHEMA_WITH_SPACE_CHOICES,
            space_voids: schema >= crate::SCHEMA_WITH_SPACE_VOIDS,
            dimension_sets: schema >= crate::SCHEMA_WITH_DIMENSION_SETS,
            dimension_set_edges: schema >= crate::SCHEMA_WITH_DIMENSION_SET_EDGES,
            railings: schema >= crate::SCHEMA_WITH_RAILINGS,
            stair_arcs: schema >= crate::SCHEMA_WITH_STAIR_ARCS,
            dimension_cross: schema >= crate::SCHEMA_WITH_DIMENSION_CROSS,
            recesses: schema >= crate::SCHEMA_WITH_RECESSES,
            level_state: schema >= crate::SCHEMA_WITH_LEVEL_STATE,
            dimension_levels: schema >= crate::SCHEMA_WITH_DIMENSION_LEVELS,
            schedule_columns: schema >= crate::SCHEMA_WITH_SCHEDULE_COLUMNS,
            slabs: schema >= crate::SCHEMA_WITH_SLABS,
            ramp_levels: schema >= crate::SCHEMA_WITH_RAMP_LEVELS,
            stair_levels: schema >= crate::SCHEMA_WITH_STAIR_LEVELS,
            grid_column_forms: schema >= crate::SCHEMA_WITH_GRID_COLUMN_FORMS,
            casework: schema >= crate::SCHEMA_WITH_CASEWORK,
            dimension_set_spans: schema >= crate::SCHEMA_WITH_DIMENSION_SET_SPANS,
            dimension_set_scales: schema >= crate::SCHEMA_WITH_DIMENSION_SET_SCALES,
            audit_exceptions: schema >= crate::SCHEMA_WITH_AUDIT_EXCEPTIONS,
            fixtures: schema >= crate::SCHEMA_WITH_FIXTURES,
            profile_files: schema >= crate::SCHEMA_WITH_PROFILE_FILES,
            standard_docs: schema >= crate::SCHEMA_WITH_STANDARD_DOCS,
            review_notes: schema >= crate::SCHEMA_WITH_REVIEW_NOTES,
            long_patterns: schema >= crate::SCHEMA_WITH_LONG_PATTERNS,
            shx_fonts: schema >= crate::SCHEMA_WITH_SHX_FONTS,
            table_styles: schema >= crate::SCHEMA_WITH_TABLE_STYLES,
            dimension_text_styles: schema >= crate::SCHEMA_WITH_DIMENSION_TEXT_STYLES,
            leader_text_styles: schema >= crate::SCHEMA_WITH_LEADER_TEXT_STYLES,
            map_work: schema >= crate::SCHEMA_WITH_MAP_WORK,
            map_work_relative: schema >= crate::SCHEMA_WITH_MAP_WORK_RELATIVE,
            map_survey: schema >= crate::SCHEMA_WITH_MAP_SURVEY,
            map_surfaces: schema >= crate::SCHEMA_WITH_MAP_SURFACES,
            surface_edits: schema >= crate::SCHEMA_WITH_SURFACE_EDITS,
            map_platforms: schema >= crate::SCHEMA_WITH_MAP_PLATFORMS,
            map_drawings: schema >= crate::SCHEMA_WITH_MAP_DRAWINGS,
            gnss_network: schema >= crate::SCHEMA_WITH_GNSS_NETWORK,
            standard_exceptions: schema >= crate::SCHEMA_WITH_STANDARD_EXCEPTIONS,
            arch_elements: schema >= crate::SCHEMA_WITH_ARCH_ELEMENTS,
            tags: schema >= crate::SCHEMA_WITH_TAGS,
            viewmarks: schema >= crate::SCHEMA_WITH_VIEWMARKS,
            phases: schema >= crate::SCHEMA_WITH_PHASES,
            xrefs: schema >= crate::SCHEMA_WITH_XREFS,
            xref_paths: schema >= crate::SCHEMA_WITH_XREF_PATHS,
            join_views: schema >= crate::SCHEMA_WITH_JOIN_VIEWS,
            linear_events: schema >= crate::SCHEMA_WITH_LINEAR_EVENTS,
            lineage: schema >= crate::SCHEMA_WITH_LINEAGE,
            source_crs: schema >= crate::SCHEMA_WITH_SOURCE_CRS,
            layer_geometry: schema >= crate::SCHEMA_WITH_LAYER_GEOMETRY,
            exception_digests: schema >= crate::SCHEMA_WITH_EXCEPTION_DIGESTS,
            topology_reviews: schema >= crate::SCHEMA_WITH_TOPOLOGY_REVIEWS,
            georef_crs: schema >= crate::SCHEMA_WITH_GEOREF_CRS,
            recorded_time: schema >= crate::SCHEMA_WITH_RECORDED_TIME,
            source_bases: schema >= crate::SCHEMA_WITH_SOURCE_BASES,
            source_field_bases: schema >= crate::SCHEMA_WITH_SOURCE_FIELD_BASES,
            clipping: schema >= crate::SCHEMA_WITH_CLIPPING,
            heights: schema >= crate::SCHEMA_WITH_HEIGHTS,
            network_turns: schema >= crate::SCHEMA_WITH_NETWORK_TURNS,
            network_terminals: schema >= crate::SCHEMA_WITH_NETWORK_TERMINALS,
            network_configs: schema >= crate::SCHEMA_WITH_NETWORK_CONFIGS,
            network_profiles: schema >= crate::SCHEMA_WITH_NETWORK_PROFILES,
            timed_closures: schema >= crate::SCHEMA_WITH_TIMED_CLOSURES,
            georef: schema >= crate::SCHEMA_WITH_GEOREF,
            raster_tiles: schema >= crate::SCHEMA_WITH_RASTER_TILES,
            field_kinds: schema >= crate::SCHEMA_WITH_FIELD_KINDS,
            withheld_fields: schema >= crate::SCHEMA_WITH_WITHHELD_FIELDS,
            field_columns: schema >= crate::SCHEMA_WITH_FIELD_COLUMNS,
            table_constraints: schema >= crate::SCHEMA_WITH_TABLE_CONSTRAINTS,
            cad_work: schema >= crate::SCHEMA_WITH_CAD_WORK,
            road_work: schema >= crate::SCHEMA_WITH_ROAD_WORK,
            road_pavement: schema >= crate::SCHEMA_WITH_ROAD_PAVEMENT,
            road_pivot: schema >= crate::SCHEMA_WITH_ROAD_PIVOT,
            section_breaks: schema >= crate::SCHEMA_WITH_SECTION_BREAKS,
            volume_curve: schema >= crate::SCHEMA_WITH_VOLUME_CURVE,
            corridor_sampling: schema >= crate::SCHEMA_WITH_CORRIDOR_SAMPLING,
            haul_sorting: schema >= crate::SCHEMA_WITH_HAUL_SORTING,
            profile_crossings: schema >= crate::SCHEMA_WITH_PROFILE_CROSSINGS,
            volume_history: schema >= crate::SCHEMA_WITH_VOLUME_HISTORY,
            haul_approval: schema >= crate::SCHEMA_WITH_HAUL_APPROVAL,
            haul_network: schema >= crate::SCHEMA_WITH_HAUL_NETWORK,
            volume_states: schema >= crate::SCHEMA_WITH_VOLUME_STATES,
            cut_benches: schema >= crate::SCHEMA_WITH_CUT_BENCHES,
            slope_transitions: schema >= crate::SCHEMA_WITH_SLOPE_TRANSITIONS,
            volume_grid: schema >= crate::SCHEMA_WITH_VOLUME_GRID,
            road_grades: schema >= crate::SCHEMA_WITH_ROAD_GRADES,
            road_ove: schema >= crate::SCHEMA_WITH_ROAD_OVE,
            road_speed: schema >= crate::SCHEMA_WITH_ROAD_SPEED,
            road_places: schema >= crate::SCHEMA_WITH_ROAD_PLACES,
            road_cloud_heights: schema >= crate::SCHEMA_WITH_ROAD_CLOUD_HEIGHTS,
            road_cloud_conversions: schema >= crate::SCHEMA_WITH_ROAD_CLOUD_CONVERSIONS,
            road_approval_sources: schema >= crate::SCHEMA_WITH_ROAD_APPROVAL_SOURCES,
            cutfill_drawings: schema >= crate::SCHEMA_WITH_CUTFILL_DRAWINGS,
            pipe_profile_drawings: schema >= crate::SCHEMA_WITH_PIPE_PROFILE_DRAWINGS,
            sheet_contour_drawings: schema >= crate::SCHEMA_WITH_SHEET_CONTOUR_DRAWINGS,
            work_stamps: schema >= crate::SCHEMA_WITH_WORK_STAMPS,
            known_velocities: schema >= crate::SCHEMA_WITH_KNOWN_VELOCITIES,
            gnss_sessions: schema >= crate::SCHEMA_WITH_GNSS_SESSIONS,
            parcel_drafts: schema >= crate::SCHEMA_WITH_PARCEL_DRAFTS,
            cloud_poses: schema >= crate::SCHEMA_WITH_CLOUD_POSES,
            kept_adjustments: schema >= crate::SCHEMA_WITH_KEPT_ADJUSTMENTS,
            monitoring_reviews: schema >= crate::SCHEMA_WITH_MONITORING_REVIEWS,
            drawing_provenance: schema >= crate::SCHEMA_WITH_DRAWING_PROVENANCE,
            approved_objects: schema >= crate::SCHEMA_WITH_APPROVED_OBJECTS,
            approved_parts: schema >= crate::SCHEMA_WITH_APPROVED_PARTS,
            approved_values: schema >= crate::SCHEMA_WITH_APPROVED_VALUES,
            work_profiles: schema >= crate::SCHEMA_WITH_WORK_PROFILES,
            surface_pastes: schema >= crate::SCHEMA_WITH_SURFACE_PASTES,
            review_decisions: schema >= crate::SCHEMA_WITH_REVIEW_DECISIONS,
            parcel_register: schema >= crate::SCHEMA_WITH_PARCEL_REGISTER,
            zoning_drafts: schema >= crate::SCHEMA_WITH_ZONING_DRAFTS,
            boundary_bonds: schema >= crate::SCHEMA_WITH_BOUNDARY_BONDS,
            plan_cut: schema >= crate::SCHEMA_WITH_PLAN_CUT,
            line_groups: schema >= crate::SCHEMA_WITH_LINE_GROUPS,
            extraction: schema >= crate::SCHEMA_WITH_EXTRACTION,
            approved_sheet_templates: schema >= crate::SCHEMA_WITH_APPROVED_SHEET_TEMPLATES,
            layer_indexes: schema >= crate::SCHEMA_WITH_LAYER_INDEXES,
            arch_bonds: schema >= crate::SCHEMA_WITH_ARCH_BONDS,
            arc_edge_bonds: schema >= crate::SCHEMA_WITH_ARC_EDGE_BONDS,
            block_publications: schema >= crate::SCHEMA_WITH_BLOCK_PUBLICATIONS,
            block_parameter_groups: schema >= crate::SCHEMA_WITH_BLOCK_PARAMETER_GROUPS,
            block_type_values: schema >= crate::SCHEMA_WITH_BLOCK_TYPE_VALUES,
            building_parts: schema >= crate::SCHEMA_WITH_BUILDING_PARTS,
            materials: schema >= crate::SCHEMA_WITH_MATERIALS,
            multi_slide: schema >= crate::SCHEMA_WITH_MULTI_SLIDE,
            opening_types: schema >= crate::SCHEMA_WITH_OPENING_TYPES,
            special_doors: schema >= crate::SCHEMA_WITH_SPECIAL_DOORS,
            ellipse_tags: schema >= crate::SCHEMA_WITH_ELLIPSE_TAGS,
            opening_codes: schema >= crate::SCHEMA_WITH_OPENING_CODES,
            paiy_areas: schema >= crate::SCHEMA_WITH_PAIY_AREAS,
            zoning: schema >= crate::SCHEMA_WITH_ZONING,
            land_shares: schema >= crate::SCHEMA_WITH_LAND_SHARES,
            site_plan: schema >= crate::SCHEMA_WITH_SITE_PLAN,
            local_rules: schema >= crate::SCHEMA_WITH_LOCAL_RULES,
            sprinklers: schema >= crate::SCHEMA_WITH_SPRINKLERS,
            provision_tables: schema >= crate::SCHEMA_WITH_PROVISION_TABLES,
            room_list: schema >= crate::SCHEMA_WITH_ROOM_LIST,
            opening_specs: schema >= crate::SCHEMA_WITH_OPENING_SPECS,
            opening_phases: schema >= crate::SCHEMA_WITH_OPENING_PHASES,
            slab_layers: schema >= crate::SCHEMA_WITH_SLAB_LAYERS,
            room_heights: schema >= crate::SCHEMA_WITH_ROOM_HEIGHTS,
            beams: schema >= crate::SCHEMA_WITH_BEAMS,
            roofs: schema >= crate::SCHEMA_WITH_ROOFS,
            roof_parts: schema >= crate::SCHEMA_WITH_ROOF_PARTS,
            roof_edges: schema >= crate::SCHEMA_WITH_ROOF_EDGES,
            roof_downpipes: schema >= crate::SCHEMA_WITH_ROOF_DOWNPIPES,
            stair_winders: schema >= crate::SCHEMA_WITH_STAIR_WINDERS,
            railing_objects: schema >= crate::SCHEMA_WITH_RAILING_OBJECTS,
            grid_bonds: schema >= crate::SCHEMA_WITH_GRID_BONDS,
            column_side_bonds: schema >= crate::SCHEMA_WITH_COLUMN_SIDE_BONDS,
            module_scope: schema >= crate::SCHEMA_WITH_MODULE_SCOPE,
            column_material: schema >= crate::SCHEMA_WITH_COLUMN_MATERIAL,
            beam_roles: schema >= crate::SCHEMA_WITH_BEAM_ROLES,
            renovation_view: schema >= crate::SCHEMA_WITH_RENOVATION_VIEW,
            building_program: schema >= crate::SCHEMA_WITH_BUILDING_PROGRAM,
            view_recipes: schema >= crate::SCHEMA_WITH_VIEW_RECIPES,
            modules: schema >= crate::SCHEMA_WITH_MODULES,
            recipe_detail: schema >= crate::SCHEMA_WITH_RECIPE_DETAIL,
            module_units: schema >= crate::SCHEMA_WITH_MODULE_UNITS,
            module_storeys: schema >= crate::SCHEMA_WITH_MODULE_STOREYS,
            slide_more: schema >= crate::SCHEMA_WITH_SLIDE_MORE,
            option_combos: schema >= crate::SCHEMA_WITH_OPTION_COMBOS,
            instance_storeys: schema >= crate::SCHEMA_WITH_INSTANCE_STOREYS,
            view_templates: schema >= crate::SCHEMA_WITH_VIEW_TEMPLATES,
            program_shares: schema >= crate::SCHEMA_WITH_PROGRAM_SHARES,
            options: schema >= crate::SCHEMA_WITH_OPTIONS,
            unit_schedules: schema >= crate::SCHEMA_WITH_UNIT_SCHEDULES,
            join_sources: schema >= crate::SCHEMA_WITH_JOIN_SOURCES,
            datasets: schema >= crate::SCHEMA_WITH_DATASETS,
            labels: schema >= SCHEMA_WITH_LABELS,
            variables: schema >= SCHEMA_WITH_VARIABLES,
            annotation: schema >= SCHEMA_WITH_ANNOTATION,
            uids: true,
        }
    }

    /// The same for a block definition's objects.
    pub(crate) fn without_uids(self) -> Self {
        Self {
            uids: false,
            ..self
        }
    }
}

/// Whether a kind's map may hold `key` (the fields every kind has, then its
/// own, then what the payload's schema adds).
pub(super) fn allowed(kind: Kind, key: &str, has: Features) -> bool {
    matches!(key, "attrs" | "color" | "label" | "symbol" | "layerId")
        || (has.uids && key == "uid")
        // A label pinned by hand: the drawing's objects only (a definition's have no persistent id).
        || (has.labels && has.uids && key == "labelPins")
        // Photos and documents: the drawing's objects only (a definition's are not picked one by one).
        || (has.attachments && has.uids && key == "attachments")
        || (has.weights && key == "lineWeight")
        // An object's look (docs/adr/0260): its order the drawing's objects', `byLayer` a definition's.
        || (has.look && matches!(key, "lineType" | "lineTypeScale" | "transparency"))
        || (has.look && has.uids && key == "order")
        || (has.look && !has.uids && key == "byLayer")
        || (has.wipeouts && key == "wipeout")
        || (has.patterns && key == "linePattern")
        || match kind {
            Kind::Point => matches!(key, "p" | "z") || (has.line_parts && key == "parts"),
            Kind::Line => {
                matches!(key, "a" | "b")
                    || (has.elevations && matches!(key, "za" | "zb"))
                    || (has.measures && matches!(key, "ma" | "mb"))
            }
            Kind::Polyline => {
                matches!(key, "pts" | "bulges")
                    || (has.space_separators && key == "spaceSeparator")
                    || (has.elevations && key == "zs")
                    || (has.line_parts && key == "parts")
                    || (has.measures && key == "ms")
                    || (has.polyline_extras && matches!(key, "closed" | "widths"))
            }
            Kind::Polygon => {
                matches!(key, "pts" | "bulges" | "holes")
                    || (has.elevations && key == "zs")
                    || (has.parts && key == "parts")
            }
            Kind::Circle => matches!(key, "c" | "r"),
            Kind::Arc => matches!(key, "c" | "r" | "a0" | "a1"),
            Kind::Ellipse => matches!(key, "c" | "major" | "ratio" | "t0" | "t1"),
            Kind::Spline => matches!(key, "pts" | "closed"),
            Kind::Nurbs => matches!(
                key,
                "degree" | "ctrl" | "knots" | "weights" | "closed" | "periodic" | "fit" | "tangents" | "fitTolerance"
            ),
            Kind::Xline | Kind::Ray => matches!(key, "p" | "dir"),
            Kind::Wall => matches!(
                key,
                "pts" | "bulges" | "closed" | "width" | "justify" | "offset" | "style" | "components" | "caps" | "fill"
                    | "height" | "arch"
            ) || (has.wall_joins && matches!(key, "joins" | "cleanup")),
            Kind::Opening => matches!(key, "host" | "at" | "width" | "place" | "opening"),
            Kind::Space => matches!(key, "parts" | "space"),
            Kind::Grid => key == "grid",
            Kind::Column => key == "column",
            Kind::Stair => key == "stair",
            Kind::Ramp => key == "ramp",
            Kind::Tag => key == "tag",
            Kind::ViewMark => key == "viewmark",
            Kind::Slab => key == "slab",
            Kind::Beam => key == "beam",
            Kind::Roof => key == "roof",
            Kind::Railing => key == "railing",
            Kind::Text => {
                matches!(key, "p" | "text" | "height" | "rotation")
                    || (has.texts && matches!(key, "align" | "widthFactor" | "mask"))
                    || (has.linked_texts && has.uids && matches!(key, "labelOf" | "labelScale"))
                    || (has.paragraphs && matches!(key, "boxWidth" | "lineSpacing" | "runs"))
                    || (has.paragraph_layout && key == "layout")
                    || (has.text_fields && key == "fields")
                    || (has.styles
                        && matches!(key, "textStyle" | "font" | "bold" | "italic" | "oblique"))
                    || (has.text_paths && key == "path")
                    || (has.overrides && key == "overrides")
                    || (has.annotative && key == "annotative")
            }
            Kind::Dimension => {
                matches!(
                    key,
                    "a" | "b" | "c" | "text" | "angle" | "style" | "height" | "offset"
                ) || (has.dimensions && matches!(key, "mask" | "za" | "zb"))
                    || (has.styles
                        && matches!(
                            key,
                            "dimStyle"
                                | "arrow"
                                | "arrowSize"
                                | "extOffset"
                                | "extBeyond"
                                | "textGap"
                                | "textPlace"
                                | "decimals"
                                | "unit"
                                | "prefix"
                                | "suffix"
                                | "font"
                        ))
                    || (has.annotation
                        && matches!(
                            key,
                            "dimLineColor"
                                | "dimLineWeight"
                                | "dimLineType"
                                | "extColor"
                                | "extWeight"
                                | "extLineType"
                                | "textColor"
                        ))
                    // What its points follow names objects: only the drawing's dimensions (docs/adr/0249 §4).
                    || (has.dimension_refs && has.uids && key == "refs")
                    || (has.dimension_options && matches!(key, "options" | "textShift" | "breaks"))
                    || (has.overrides && key == "overrides")
                    || (has.annotative && key == "annotative")
                    || (has.dimension_levels && key == "level")
            }
            // A hatch's tie names objects: only the drawing's hatches (docs/adr/0186 §6).
            Kind::Hatch => {
                matches!(key, "ring" | "holes" | "pattern")
                    || (has.hatches && has.uids && key == "assoc")
                    || (has.hatch_boundaries && key == "boundary")
                    || (has.hatch_extras && key == "annotative")
            }
            Kind::Insert => {
                matches!(key, "block" | "p" | "scale" | "rotation" | "mirror")
                    || (has.block_extras && key == "yScale")
                    || (has.block_cards && key == "attributePlaces")
                    || (has.dynamic_blocks && key == "parameters")
                    || (has.clipping && key == "clipping")
                    || (has.fixtures && key == "arch")
            }
            Kind::Leader => {
                matches!(
                    key,
                    "pts" | "text" | "height" | "rotation" | "arrow" | "mask"
                ) || (has.annotation && key == "arrowSize")
                    || (has.annotative && key == "annotative")
                    || (has.multileader && matches!(key, "arms" | "tip" | "blocks" | "leaderStyle"))
            }
            Kind::Table => {
                matches!(
                    key,
                    "p" | "rotation"
                        | "height"
                        | "rows"
                        | "columns"
                        | "cells"
                        | "merges"
                        | "aligns"
                        | "header"
                        | "grid"
                        | "frame"
                        | "source"
                        | "textStyle"
                        | "font"
                        | "bold"
                        | "italic"
                        | "oblique"
                ) || (has.table_cells
                    && matches!(key, "styles" | "formulas" | "fields" | "link" | "schedule"))
                    || (has.extraction && key == "breaks")
                    || (has.table_styles && key == "tableStyle")
            }
            Kind::Image => matches!(
                key,
                "p" | "width"
                    | "height"
                    | "rotation"
                    | "mirror"
                    | "asset"
                    | "file"
                    | "clip"
                    | "opacity"
            ),
            Kind::Raster => {
                matches!(
                    key,
                    "affine"
                        | "width"
                        | "height"
                        | "bands"
                        | "sample"
                        | "asset"
                        | "file"
                        | "srid"
                        | "style"
                        | "opacity"
                ) || (has.point_clouds && key == "url")
                    || (has.multidim && key == "dataset")
                    || (has.georef && key == "georef")
                    || (has.raster_tiles && key == "tiles")
            }
            Kind::PointCloud => matches!(
                key,
                "sources" | "bounds" | "count" | "srid" | "style" | "opacity"
            ),
            // A record has no fields of its own (docs/adr/0410 §2).
            Kind::Record => false,
        }
}
