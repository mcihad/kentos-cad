//! The GIS settings as a payload keeps them: the topology rules (docs/adr/0202 §7) and a bound source.

use super::*;

impl<'d> Encoder<'d> {
    /// The topology settings (docs/adr/0202 §7), checked whole as a reader
    /// checks them: `rules` (`id`, `kind`, `layer`, `other`, `value`),
    /// `tolerance`, `exceptions` (`at`, `rule`, `objects`).
    pub(super) fn topology(&mut self, t: &'d TopologySettings) -> Result<(), KcadError> {
        if let Some(problem) = t.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(
            usize::from(!t.rules.is_empty())
                + usize::from(t.profile.is_some())
                + usize::from(t.tolerance.is_some())
                + usize::from(!t.exceptions.is_empty()),
            true,
        )?;
        if !t.rules.is_empty() {
            self.key("rules");
            self.at(Seg::Name("rules"), |e| {
                e.open(t.rules.len(), false)?;
                for (i, r) in t.rules.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(
                            3 + usize::from(r.other.is_some()) + usize::from(r.value.is_some()),
                            true,
                        )?;
                        e.key("id");
                        e.text(&r.id)?;
                        e.key("kind");
                        e.w.text(r.kind.key());
                        e.key("layer");
                        e.text(&r.layer)?;
                        if let Some(o) = &r.other {
                            e.key("other");
                            e.text(o)?;
                        }
                        if let Some(v) = r.value {
                            e.key("value");
                            e.at(Seg::Name("value"), |e| e.float(v))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        // Schema 189 (docs/adr/1260 §2): the quality profile, its keys in the file's order.
        if let Some(p) = &t.profile {
            self.key("profile");
            self.at(Seg::Name("profile"), |e| {
                e.open(3 + usize::from(p.edition.is_some()), true)?;
                e.key("name");
                e.text(&p.name)?;
                if let Some(ed) = &p.edition {
                    e.key("edition");
                    e.text(ed)?;
                }
                e.key("precision");
                e.at(Seg::Name("precision"), |e| e.float(p.precision))?;
                e.key("acceptance");
                e.at(Seg::Name("acceptance"), |e| e.float(p.acceptance))?;
                e.close();
                Ok(())
            })?;
        }
        if let Some(tol) = t.tolerance {
            self.key("tolerance");
            self.at(Seg::Name("tolerance"), |e| e.float(tol))?;
        }
        if !t.exceptions.is_empty() {
            self.key("exceptions");
            self.at(Seg::Name("exceptions"), |e| {
                e.open(t.exceptions.len(), false)?;
                for (i, x) in t.exceptions.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(
                            3 + usize::from(x.digest.is_some())
                                + usize::from(x.decision.is_some())
                                + usize::from(x.reason.is_some())
                                + usize::from(x.rule_digest.is_some())
                                + usize::from(x.scope.is_some()),
                            true,
                        )?;
                        e.key("at");
                        e.at(Seg::Name("at"), |e| e.point(&x.at))?;
                        e.key("rule");
                        e.text(&x.rule)?;
                        // Schema 189 (docs/adr/1260 §1): what it covers, why, what was decided, the rule's version.
                        if let Some(s) = x.scope {
                            e.key("scope");
                            e.w.text(match s {
                                TopologyScope::Finding => "finding",
                                TopologyScope::Objects => "objects",
                            });
                        }
                        if let Some(d) = &x.digest {
                            e.key("digest");
                            e.text(d)?;
                        }
                        if let Some(why) = &x.reason {
                            e.key("reason");
                            e.text(why)?;
                        }
                        e.key("objects");
                        e.at(Seg::Name("objects"), |e| {
                            e.open(x.objects.len(), false)?;
                            for (j, id) in x.objects.iter().enumerate() {
                                e.at(Seg::Index(j), |e| e.id(&id.0))?;
                            }
                            e.close();
                            Ok(())
                        })?;
                        if let Some(d) = x.decision {
                            e.key("decision");
                            e.w.text(match d {
                                TopologyDecision::Exception => "exception",
                                TopologyDecision::Error => "error",
                            });
                        }
                        if let Some(d) = &x.rule_digest {
                            e.key("ruleDigest");
                            e.text(d)?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    pub(super) fn source(&mut self, s: &MigrationSource) -> Result<(), KcadError> {
        let digest = parse_hex32(&s.source_sha256);
        let Some(digest) =
            digest.filter(|_| s.format == DOCUMENT_FORMAT && s.version == DOCUMENT_VERSION)
        else {
            return Err(self.fail(
                Code::BadValue,
                "göç kaynağı kentos.document sürüm 1 ve 64 küçük onaltılık haneli bir SHA-256 olmalı",
            ));
        };
        self.open(3, true)?;
        self.key("format");
        self.w.text(DOCUMENT_FORMAT);
        self.key("version");
        self.w.uint(u64::from(DOCUMENT_VERSION));
        self.key("sourceSha256");
        self.w.bytes(&digest);
        self.close();
        Ok(())
    }
}
