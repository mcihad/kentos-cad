//! A layer node's map service and feed, and the project's connections
//! (document schema 32, docs/specs/kcad-v2.md §6.5 and §6.4,
//! docs/adr/0208 §2): each checked whole by the contract's rules, as the
//! readers check it, its keys sorted as RFC 8949 sorts them. A flag is
//! written only when it is true; a list only when it is not empty.

use kentos_contracts::{
    FeatureFeed, LayerNodeType, ServiceConnection, ServiceLayer, ServiceParam, TileGrid,
    TileMatrix, connections_problem,
};

use super::Encoder;
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};

/// One field before the fields are sorted by key.
enum V<'d> {
    Text(&'d str),
    Uint(u64),
    Float(f64),
    True,
    Texts(&'d [String]),
    Grid(&'d TileGrid),
    Params(&'d [ServiceParam]),
    Floats(&'d [f64]),
}

impl<'d> Encoder<'d> {
    /// A map of `fields`, their keys sorted.
    fn sorted(&mut self, mut fields: Vec<(&'static str, V<'d>)>) -> Result<(), KcadError> {
        fields.sort_by(|a, b| key_order(a.0, b.0));
        self.open(fields.len(), true)?;
        for (key, v) in fields {
            self.key(key);
            self.at(Seg::Name(key), |e| e.service_value(v))?;
        }
        self.close();
        Ok(())
    }

    fn service_value(&mut self, v: V<'d>) -> Result<(), KcadError> {
        match v {
            V::Text(t) => self.text(t),
            V::Uint(n) => {
                self.w.uint(n);
                Ok(())
            }
            V::Float(x) => self.float(x),
            V::True => {
                self.w.bool(true);
                Ok(())
            }
            V::Texts(list) => {
                self.open(list.len(), false)?;
                for (i, t) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.text(t))?;
                }
                self.close();
                Ok(())
            }
            V::Grid(g) => self.tile_grid(g),
            V::Params(list) => {
                self.open(list.len(), false)?;
                for (i, p) in list.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.sorted(vec![
                            ("name", V::Text(&p.name)),
                            ("value", V::Text(&p.value)),
                        ])
                    })?;
                }
                self.close();
                Ok(())
            }
            V::Floats(list) => {
                self.open(list.len(), false)?;
                for x in list {
                    self.float(*x)?;
                }
                self.close();
                Ok(())
            }
        }
    }

    fn tile_grid(&mut self, g: &'d TileGrid) -> Result<(), KcadError> {
        self.open(2, true)?;
        self.key("srid");
        self.w.uint(u64::from(g.srid));
        self.key("matrices");
        self.at(Seg::Name("matrices"), |e| {
            e.open(g.matrices.len(), false)?;
            for (i, m) in g.matrices.iter().enumerate() {
                e.at(Seg::Index(i), |e| e.tile_matrix(m))?;
            }
            e.close();
            Ok(())
        })?;
        self.close();
        Ok(())
    }

    fn tile_matrix(&mut self, m: &'d TileMatrix) -> Result<(), KcadError> {
        self.sorted(vec![
            ("id", V::Text(&m.id)),
            ("x0", V::Float(m.x0)),
            ("y0", V::Float(m.y0)),
            ("resolution", V::Float(m.resolution)),
            ("tileWidth", V::Uint(u64::from(m.tile_width))),
            ("tileHeight", V::Uint(u64::from(m.tile_height))),
            ("matrixWidth", V::Uint(m.matrix_width)),
            ("matrixHeight", V::Uint(m.matrix_height)),
        ])
    }

    /// A layer's map service (docs/adr/0208 §2), on a layer only, checked whole.
    pub(super) fn service(
        &mut self,
        kind: LayerNodeType,
        s: &'d ServiceLayer,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun servisi yazılmaz; servis yalnız katmanındır",
            ));
        }
        if let Some(problem) = s.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let mut f = vec![("kind", V::Text(s.kind.name())), ("url", V::Text(&s.url))];
        if !s.layers.is_empty() {
            f.push(("layers", V::Texts(&s.layers)));
        }
        if !s.subdomains.is_empty() {
            f.push(("subdomains", V::Texts(&s.subdomains)));
        }
        if !s.params.is_empty() {
            f.push(("params", V::Params(&s.params)));
        }
        for (key, value) in [
            ("style", &s.style),
            ("format", &s.format),
            ("template", &s.template),
            ("version", &s.version),
            ("attribution", &s.attribution),
            ("connection", &s.connection),
            ("preset", &s.preset),
            ("matrixSet", &s.matrix_set),
        ] {
            if let Some(t) = value {
                f.push((key, V::Text(t)));
            }
        }
        for (key, value) in [
            ("srid", s.srid),
            ("tileSize", s.tile_size),
            ("minZoom", s.min_zoom),
            ("maxZoom", s.max_zoom),
        ] {
            if let Some(n) = value {
                f.push((key, V::Uint(u64::from(n))));
            }
        }
        if let Some(g) = &s.grid {
            f.push(("grid", V::Grid(g)));
        }
        for (key, on) in [
            ("yFlip", s.y_flip),
            ("transparent", s.transparent),
            ("dynamic", s.dynamic),
        ] {
            if on {
                f.push((key, V::True));
            }
        }
        if let Some(o) = s.opacity {
            f.push(("opacity", V::Float(o)));
        }
        if let Some(b) = &s.bbox {
            f.push(("bbox", V::Floats(b)));
        }
        self.sorted(f)
    }

    /// Where a layer's objects came from (docs/adr/0208 §10), on a layer only, checked whole.
    pub(super) fn feed(
        &mut self,
        kind: LayerNodeType,
        d: &'d FeatureFeed,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun veri kaynağı yazılmaz; kaynak yalnız katmanındır",
            ));
        }
        if let Some(problem) = d.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let mut f = vec![("kind", V::Text(d.kind.name())), ("url", V::Text(&d.url))];
        for (key, value) in [
            ("name", &d.name),
            ("filter", &d.filter),
            ("version", &d.version),
            ("key", &d.key),
            ("connection", &d.connection),
            ("fetched", &d.fetched),
        ] {
            if let Some(t) = value {
                f.push((key, V::Text(t)));
            }
        }
        if let Some(srid) = d.srid {
            f.push(("srid", V::Uint(u64::from(srid))));
        }
        if let Some(limit) = d.limit {
            f.push(("limit", V::Uint(limit)));
        }
        if let Some(b) = &d.bbox {
            f.push(("bbox", V::Floats(b)));
        }
        self.sorted(f)
    }

    /// The project's connections (docs/adr/0208 §2), checked whole, without secrets.
    pub(super) fn connections(&mut self, list: &'d [ServiceConnection]) -> Result<(), KcadError> {
        if let Some(problem) = connections_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, c) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let mut f = vec![
                    ("id", V::Text(&c.id)),
                    ("name", V::Text(&c.name)),
                    ("origin", V::Text(&c.origin)),
                    ("auth", V::Text(c.auth.name())),
                ];
                if !c.names.is_empty() {
                    f.push(("names", V::Texts(&c.names)));
                }
                if let Some(u) = &c.token_url {
                    f.push(("tokenUrl", V::Text(u)));
                }
                if let Some(s) = &c.scope {
                    f.push(("scope", V::Text(s)));
                }
                e.sorted(f)
            })?;
        }
        self.close();
        Ok(())
    }
}
