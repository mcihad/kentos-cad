//! The layer tree as a payload keeps it: layers with their fields, snapping and styles, the layer states, and the
//! project's line types and their pattern elements.

use super::*;

impl<'d> Encoder<'d> {
    /// The project's line types (docs/adr/0313 §1), checked whole as a reader checks them; keys in the canonical
    /// order.
    pub(super) fn line_types(
        &mut self,
        types: &'d [kentos_contracts::LineTypeDef],
    ) -> Result<(), KcadError> {
        if let Some(problem) = kentos_contracts::line_types_problem(types) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(types.len(), false)?;
        for (i, t) in types.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let n =
                    3 + usize::from(t.source.is_some()) + usize::from(!t.description.is_empty());
                e.open(n, true)?;
                // name unit (4), source (6), elements (8), description (11).
                e.key("name");
                e.text(&t.name)?;
                e.key("unit");
                e.w.text(match t.unit {
                    kentos_contracts::PatternUnit::Paper => "paper",
                    kentos_contracts::PatternUnit::Drawing => "drawing",
                });
                if let Some(source) = &t.source {
                    e.key("source");
                    e.text(source)?;
                }
                e.key("elements");
                e.at(Seg::Name("elements"), |e| {
                    e.open(t.elements.len(), false)?;
                    for (k, el) in t.elements.iter().enumerate() {
                        e.at(Seg::Index(k), |e| e.pattern_element(el))?;
                    }
                    e.close();
                    Ok(())
                })?;
                if !t.description.is_empty() {
                    e.key("description");
                    e.text(&t.description)?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    /// A pattern's element: text (4), shape (5), length (6).
    pub(super) fn pattern_element(
        &mut self,
        el: &'d kentos_contracts::PatternElement,
    ) -> Result<(), KcadError> {
        self.open(
            1 + usize::from(el.text.is_some()) + usize::from(el.shape.is_some()),
            true,
        )?;
        if let Some(t) = &el.text {
            self.key("text");
            self.at(Seg::Name("text"), |e| {
                let place = &t.place;
                e.open(2 + place_count(place), true)?;
                e.place_xy(place)?;
                // text (4), scale style (5), upright (7), absolute rotation (8).
                e.key("text");
                e.text(&t.text)?;
                e.place_scale(place)?;
                e.key("style");
                e.text(&t.style)?;
                e.place_turn(place)?;
                e.close();
                Ok(())
            })?;
        }
        if let Some(s) = &el.shape {
            self.key("shape");
            self.at(Seg::Name("shape"), |e| {
                let place = &s.place;
                e.open(2 + place_count(place), true)?;
                e.place_xy(place)?;
                // file name (4), scale (5), upright (7), absolute rotation (8).
                e.key("file");
                e.text(&s.file)?;
                e.key("name");
                e.text(&s.name)?;
                e.place_scale(place)?;
                e.place_turn(place)?;
                e.close();
                Ok(())
            })?;
        }
        self.key("length");
        self.at(Seg::Name("length"), |e| e.float(el.length))?;
        self.close();
        Ok(())
    }

    pub(super) fn place_xy(&mut self, p: &kentos_contracts::PatternPlace) -> Result<(), KcadError> {
        if p.x != 0.0 {
            self.key("x");
            self.at(Seg::Name("x"), |e| e.float(p.x))?;
        }
        if p.y != 0.0 {
            self.key("y");
            self.at(Seg::Name("y"), |e| e.float(p.y))?;
        }
        Ok(())
    }

    pub(super) fn place_scale(
        &mut self,
        p: &kentos_contracts::PatternPlace,
    ) -> Result<(), KcadError> {
        if p.scale != 1.0 {
            self.key("scale");
            self.at(Seg::Name("scale"), |e| e.float(p.scale))?;
        }
        Ok(())
    }

    pub(super) fn place_turn(
        &mut self,
        p: &kentos_contracts::PatternPlace,
    ) -> Result<(), KcadError> {
        if p.upright {
            self.key("upright");
            self.w.bool(true);
        }
        if p.absolute {
            self.key("absolute");
            self.w.bool(true);
        }
        if p.rotation != 0.0 {
            self.key("rotation");
            self.at(Seg::Name("rotation"), |e| e.float(p.rotation))?;
        }
        Ok(())
    }

    /// The layer states (docs/adr/0177 §4), checked whole as a reader checks
    /// them: `id`, `name`, `nodes`; a node's `node`, `style`, `locked`, `visible`.
    pub(super) fn layer_states(&mut self, states: &'d [LayerState]) -> Result<(), KcadError> {
        if let Some(problem) = layer_states_problem(states) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(states.len(), false)?;
        for (i, state) in states.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                e.open(3, true)?;
                e.key("id");
                e.text(&state.id)?;
                e.key("name");
                e.text(&state.name)?;
                e.key("nodes");
                e.at(Seg::Name("nodes"), |e| {
                    e.open(state.nodes.len(), false)?;
                    for (j, node) in state.nodes.iter().enumerate() {
                        e.at(Seg::Index(j), |e| e.layer_state_node(node))?;
                    }
                    e.close();
                    Ok(())
                })?;
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    pub(super) fn layer_state_node(&mut self, n: &'d LayerStateNode) -> Result<(), KcadError> {
        self.open(
            2 + usize::from(n.style.is_some())
                + usize::from(n.locked.is_some())
                + usize::from(n.frozen.is_some())
                + usize::from(n.plot.is_some()),
            true,
        )?;
        self.key("node");
        self.text(&n.node)?;
        // Schema 55 (docs/adr/0311 §3): its freeze and printing.
        if let Some(plot) = n.plot {
            self.key("plot");
            self.w.bool(plot);
        }
        if let Some(style) = &n.style {
            self.key("style");
            self.at(Seg::Name("style"), |e| e.layer_style(style))?;
        }
        if let Some(frozen) = n.frozen {
            self.key("frozen");
            self.w.bool(frozen);
        }
        if let Some(locked) = n.locked {
            self.key("locked");
            self.w.bool(locked);
        }
        self.key("visible");
        self.w.bool(n.visible);
        self.close();
        Ok(())
    }

    // ── The layer tree ──────────────────────────────────────────────────

    pub(super) fn layers(&mut self, nodes: &'d [LayerNode]) -> Result<(), KcadError> {
        self.open(nodes.len(), false)?;
        for (i, n) in nodes.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.layer(n))?;
        }
        self.close();
        Ok(())
    }

    pub(super) fn layer(&mut self, n: &'d LayerNode) -> Result<(), KcadError> {
        let fields = !n.fields.is_empty();
        let compose = n.compose.as_ref();
        self.open(
            8 + usize::from(n.snap.is_some())
                + usize::from(fields)
                + usize::from(n.service.is_some())
                + usize::from(n.feed.is_some())
                + usize::from(n.time.is_some())
                + usize::from(n.scenario.is_some())
                + usize::from(n.replaces.is_some())
                + usize::from(n.filter.is_some())
                + usize::from(!n.actions.is_empty())
                + usize::from(n.card.is_some())
                + usize::from(compose.is_some())
                + usize::from(n.plot == Some(false))
                + usize::from(n.frozen == Some(true))
                + usize::from(!n.table_styles.is_empty())
                + usize::from(n.subtypes.is_some())
                + usize::from(!n.contingents.is_empty())
                + usize::from(!n.rules.is_empty())
                + usize::from(!n.indexes.is_empty())
                + usize::from(n.source_crs.is_some())
                + usize::from(n.geometry.is_some())
                + usize::from(n.dataset.is_some())
                + usize::from(n.derivation.is_some()),
            true,
        )?;
        // id (2), card feed name plot snap time type (4), rules style (5), fields filter frozen locked (6),
        // actions compose dataset indexes service visible (7), children expanded geometry replaces scenario
        // subtypes (8), sourceCrs (9), derivation (10), contingents tableStyles (11).
        if n.service.is_some() && n.feed.is_some() {
            return Err(self.fail(
                Code::BadValue,
                "katman hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz",
            ));
        }
        self.key("id");
        self.at(Seg::Name("id"), |e| e.text(&n.id))?;
        if let Some(card) = &n.card {
            self.key("card");
            self.at(Seg::Name("card"), |e| e.info_card(n.kind, card))?;
        }
        if let Some(feed) = &n.feed {
            self.key("feed");
            self.at(Seg::Name("feed"), |e| e.feed(n.kind, feed))?;
        }
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&n.name))?;
        // Yazdır (docs/adr/0260 §4): only `false` is written.
        match n.plot {
            Some(false) => {
                self.key("plot");
                self.w.bool(false);
            }
            Some(true) => {
                return Err(self.fail(
                    Code::BadValue,
                    "plot yalnız false yazılır; yazdırılan katmanda alan yoktur",
                ));
            }
            None => {}
        }
        if let Some(snap) = &n.snap {
            self.key("snap");
            self.at(Seg::Name("snap"), |e| e.layer_snap(n.kind, snap))?;
        }
        if let Some(time) = &n.time {
            self.key("time");
            self.at(Seg::Name("time"), |e| e.layer_time(n.kind, time))?;
        }
        self.key("type");
        self.w.text(match n.kind {
            LayerNodeType::Group => "group",
            LayerNodeType::Layer => "layer",
        });
        // Schema 72 (docs/adr/0438 §1): the record rules, as their JSON, fitting the fields.
        if !n.rules.is_empty() {
            self.key("rules");
            self.at(Seg::Name("rules"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun kayıt kuralları olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some((_, problem)) =
                    kentos_contracts::layer_rules_problem(&n.fields, &n.rules)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.rules)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        self.key("style");
        self.at(Seg::Name("style"), |e| e.layer_style(&n.style))?;
        if fields {
            self.key("fields");
            self.at(Seg::Name("fields"), |e| e.layer_fields(n.kind, &n.fields))?;
        }
        if let Some(filter) = &n.filter {
            self.key("filter");
            self.at(Seg::Name("filter"), |e| e.layer_filter(n.kind, filter))?;
        }
        // Dondur (docs/adr/0311 §1): only `true` is written.
        match n.frozen {
            Some(true) => {
                self.key("frozen");
                self.w.bool(true);
            }
            Some(false) => {
                return Err(self.fail(
                    Code::BadValue,
                    "frozen yalnız true yazılır; çözülmüş katmanda alan yoktur",
                ));
            }
            None => {}
        }
        self.key("locked");
        self.w.bool(n.locked);
        if !n.actions.is_empty() {
            self.key("actions");
            self.at(Seg::Name("actions"), |e| {
                e.layer_actions(n.kind, &n.actions)
            })?;
        }
        if let Some(c) = compose {
            self.key("compose");
            self.at(Seg::Name("compose"), |e| e.layer_compose(n.kind, c))?;
        }
        // Schema 106 (docs/adr/0633 §1): the layer as a dataset, as its JSON.
        if let Some(d) = &n.dataset {
            self.key("dataset");
            self.at(Seg::Name("dataset"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun veri kümesi bilgisi olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some(problem) = kentos_contracts::dataset_problem(d) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(d)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 136 (docs/adr/0758): the indexes, as their JSON, fitting the fields.
        if !n.indexes.is_empty() {
            self.key("indexes");
            self.at(Seg::Name("indexes"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun dizinleri olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some((_, problem)) =
                    kentos_contracts::layer_indexes_problem(&n.fields, &n.indexes)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.indexes)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        if let Some(service) = &n.service {
            self.key("service");
            self.at(Seg::Name("service"), |e| e.service(n.kind, service))?;
        }
        self.key("visible");
        self.w.bool(n.visible);
        self.key("children");
        self.at(Seg::Name("children"), |e| e.layers(&n.children))?;
        self.key("expanded");
        self.w.bool(n.expanded);
        // Schema 102 (docs/adr/0622 §5): what the layer holds; a group holds no type.
        if let Some(g) = n.geometry {
            if n.kind == LayerNodeType::Group {
                return Err(self.fail(
                    Code::BadValue,
                    "grubun türü olmaz; nokta, çizgi, alan ya da açıklama yalnız katmanın türüdür",
                ));
            }
            self.key("geometry");
            self.at(Seg::Name("geometry"), |e| {
                e.text(match g {
                    kentos_contracts::LayerGeometry::Point => "point",
                    kentos_contracts::LayerGeometry::Line => "line",
                    kentos_contracts::LayerGeometry::Area => "area",
                    kentos_contracts::LayerGeometry::Annotation => "annotation",
                })
            })?;
        }
        if let Some(base) = &n.replaces {
            self.key("replaces");
            self.at(Seg::Name("replaces"), |e| e.replaces(n.kind, base))?;
        }
        if let Some(scenario) = &n.scenario {
            self.key("scenario");
            self.at(Seg::Name("scenario"), |e| e.scenario(n.kind, scenario))?;
        }
        // Schema 72 (docs/adr/0422 §4): the subtypes, as their JSON, on a layer whose fields they fit.
        if let Some(subtypes) = &n.subtypes {
            self.key("subtypes");
            self.at(Seg::Name("subtypes"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun alt türleri olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some((_, problem)) = kentos_contracts::subtypes_problem(&n.fields, subtypes)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(subtypes)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 98 (docs/adr/0583 §1): the system its objects came from, as its JSON (the only 9-letter key).
        if let Some(c) = &n.source_crs {
            self.key("sourceCrs");
            self.at(Seg::Name("sourceCrs"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun kaynak sistemi olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some(problem) = kentos_contracts::layer_crs_problem(c) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(c)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 106 (docs/adr/0633 §2): what a processing tool made the layer from, as its JSON.
        if let Some(d) = &n.derivation {
            self.key("derivation");
            self.at(Seg::Name("derivation"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun türetim kaydı olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some(problem) = kentos_contracts::derivation_problem(d) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(d)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 72 (docs/adr/0423 §4): the value combinations, as their JSON, fitting the fields.
        if !n.contingents.is_empty() {
            self.key("contingents");
            self.at(Seg::Name("contingents"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun değer bileşimleri olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some((_, problem)) = kentos_contracts::contingents_problem(
                    &n.fields,
                    n.subtypes.as_ref(),
                    &n.contingents,
                ) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.contingents)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 72 (docs/adr/0413 §3): the table's rules, as their JSON, on a layer.
        if !n.table_styles.is_empty() {
            self.key("tableStyles");
            self.at(Seg::Name("tableStyles"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun koşullu biçimi olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some((_, problem)) = kentos_contracts::table_styles_problem(&n.table_styles)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.table_styles)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        self.close();
        Ok(())
    }

    /// A layer's own snapping (docs/adr/0163 §4): `off` or `kinds`, on a layer only.
    pub(super) fn layer_snap(
        &mut self,
        kind: LayerNodeType,
        snap: &'d LayerSnap,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun keneti yazılmaz; kenet yalnız katmanındır",
            ));
        }
        if let Some(problem) = snap.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(1, true)?;
        match &snap.kinds {
            None => {
                self.key("off");
                self.w.bool(true);
            }
            Some(kinds) => {
                self.key("kinds");
                self.open(kinds.len(), false)?;
                for (i, k) in kinds.iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.text(k))?;
                }
                self.close();
            }
        }
        self.close();
        Ok(())
    }

    /// A layer's fields (docs/adr/0199 §1), on a layer only, checked whole.
    pub(super) fn layer_fields(
        &mut self,
        kind: LayerNodeType,
        fields: &'d [LayerField],
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun alanları yazılmaz; alanlar yalnız katmanındır",
            ));
        }
        if let Some(problem) = layer_fields_problem(fields) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(fields.len(), false)?;
        for (i, f) in fields.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.layer_field(f))?;
        }
        self.close();
        Ok(())
    }

    /// A field: max, min (3), kind, name (4), alias, scale (5), length,
    /// values (6), default (7), multiple, required (8); only what it has.
    pub(super) fn layer_field(&mut self, f: &'d LayerField) -> Result<(), KcadError> {
        let n = 2
            + usize::from(f.id.is_some())
            + usize::from(f.max.is_some())
            + usize::from(f.min.is_some())
            + usize::from(f.alias.is_some())
            + usize::from(f.scale.is_some())
            + usize::from(f.length.is_some())
            + usize::from(f.values.is_some())
            + usize::from(f.default.is_some())
            + usize::from(f.multiple)
            + usize::from(f.required)
            + usize::from(f.split.is_some())
            + usize::from(f.merge.is_some())
            + usize::from(f.domain.is_some())
            + usize::from(f.unique)
            + usize::from(f.min_exclusive)
            + usize::from(f.max_exclusive)
            + usize::from(f.unit.is_some())
            + usize::from(f.withheld)
            + usize::from(f.column.is_some());
        self.open(n, true)?;
        if let Some(id) = &f.id {
            self.key("id");
            self.at(Seg::Name("id"), |e| e.text(id))?;
        }
        if let Some(max) = &f.max {
            self.key("max");
            self.at(Seg::Name("max"), |e| e.text(max))?;
        }
        if let Some(min) = &f.min {
            self.key("min");
            self.at(Seg::Name("min"), |e| e.text(min))?;
        }
        self.key("kind");
        self.w.text(f.kind.name());
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&f.name))?;
        // Schema 72 (docs/adr/0438 §4): the unit, in the keys' canonical order.
        if let Some(unit) = &f.unit {
            self.key("unit");
            self.at(Seg::Name("unit"), |e| e.text(unit))?;
        }
        if let Some(alias) = &f.alias {
            self.key("alias");
            self.at(Seg::Name("alias"), |e| e.text(alias))?;
        }
        // Schema 72 (docs/adr/0419): the merge and split policies, in the keys' canonical order.
        if let Some(merge) = f.merge {
            self.key("merge");
            self.w.text(merge_name(merge));
        }
        if let Some(scale) = f.scale {
            self.key("scale");
            self.w.uint(u64::from(scale));
        }
        if let Some(split) = f.split {
            self.key("split");
            self.w.text(split_name(split));
        }
        // Schema 133 (docs/adr/0753): the bound table's column, what the table makes and what it refers to.
        if let Some(c) = &f.column {
            self.key("column");
            self.at(Seg::Name("column"), |e| e.field_column(c))?;
        }
        // Schema 72 (docs/adr/0430): the value list it links.
        if let Some(domain) = &f.domain {
            self.key("domain");
            self.at(Seg::Name("domain"), |e| e.text(domain))?;
        }
        if let Some(length) = f.length {
            self.key("length");
            self.w.uint(u64::from(length));
        }
        // Schema 72 (docs/adr/0438 §3).
        if f.unique {
            self.key("unique");
            self.w.bool(true);
        }
        if let Some(values) = &f.values {
            self.key("values");
            self.at(Seg::Name("values"), |e| {
                e.open(values.len(), false)?;
                for (i, c) in values.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(
                            2 + usize::from(c.retired)
                                + usize::from(c.description.is_some())
                                + usize::from(c.from.is_some())
                                + usize::from(c.until.is_some()),
                            true,
                        )?;
                        // code from (4), label until (5), retired (7), description (11).
                        e.key("code");
                        e.at(Seg::Name("code"), |e| e.text(&c.code))?;
                        if let Some(from) = &c.from {
                            e.key("from");
                            e.at(Seg::Name("from"), |e| e.text(from))?;
                        }
                        e.key("label");
                        e.at(Seg::Name("label"), |e| e.text(&c.label))?;
                        if let Some(until) = &c.until {
                            e.key("until");
                            e.at(Seg::Name("until"), |e| e.text(until))?;
                        }
                        if c.retired {
                            e.key("retired");
                            e.w.bool(true);
                        }
                        if let Some(d) = &c.description {
                            e.key("description");
                            e.at(Seg::Name("description"), |e| e.text(d))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        if let Some(default) = &f.default {
            self.key("default");
            self.at(Seg::Name("default"), |e| e.text(default))?;
        }
        if f.multiple {
            self.key("multiple");
            self.w.bool(true);
        }
        if f.required {
            self.key("required");
            self.w.bool(true);
        }
        // Schema 107 (docs/adr/0638 §5): its values do not leave the drawing.
        if f.withheld {
            self.key("withheld");
            self.w.bool(true);
        }
        if f.max_exclusive {
            self.key("maxExclusive");
            self.w.bool(true);
        }
        if f.min_exclusive {
            self.key("minExclusive");
            self.w.bool(true);
        }
        self.close();
        Ok(())
    }

    pub(super) fn layer_style(&mut self, s: &'d LayerStyle) -> Result<(), KcadError> {
        if matches!(s.renderer, Some(Value::Null)) {
            return Err(self.fail(
                Code::BadValue,
                "çizici null olamaz; çizici yoksa alan boş bırakılır",
            ));
        }
        let n = 3
            + usize::from(s.line_pattern.is_some())
            + usize::from(s.fill.is_some())
            + usize::from(s.label.is_some())
            + usize::from(s.labels.is_some())
            + usize::from(s.point.is_some())
            + usize::from(s.renderer.is_some())
            + usize::from(s.pick_interior.is_some());
        self.open(n, true)?;
        // fill (4), color label point (5), labels (6), lineType renderer (8), lineWeight (10), linePattern (11),
        // pickInterior (12).
        if let Some(fill) = &s.fill {
            self.key("fill");
            self.at(Seg::Name("fill"), |e| e.text(fill))?;
        }
        self.key("color");
        self.at(Seg::Name("color"), |e| e.text(&s.color))?;
        if let Some(label) = &s.label {
            self.key("label");
            self.at(Seg::Name("label"), |e| e.label_style(label))?;
        }
        if let Some(p) = &s.point {
            self.key("point");
            self.at(Seg::Name("point"), |e| {
                e.open(2, true)?;
                e.key("size");
                e.at(Seg::Name("size"), |e| e.float(p.size))?;
                e.key("symbol");
                e.w.text(point_symbol(p.symbol));
                e.close();
                Ok(())
            })?;
        }
        if let Some(labels) = &s.labels {
            self.key("labels");
            self.at(Seg::Name("labels"), |e| e.layer_labels(labels))?;
        }
        self.key("lineType");
        self.w.text(line_type(s.line_type));
        if let Some(r) = &s.renderer {
            self.key("renderer");
            self.at(Seg::Name("renderer"), |e| e.opaque(r))?;
        }
        self.key("lineWeight");
        self.at(Seg::Name("lineWeight"), |e| e.float(s.line_weight))?;
        if let Some(pattern) = &s.line_pattern {
            self.key("linePattern");
            self.at(Seg::Name("linePattern"), |e| e.text(pattern))?;
        }
        if let Some(pick) = s.pick_interior {
            self.key("pickInterior");
            self.w.bool(pick);
        }
        self.close();
        Ok(())
    }
}
