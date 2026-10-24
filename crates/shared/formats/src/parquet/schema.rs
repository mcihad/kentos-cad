//! A Parquet file's schema as a tree (docs/adr/0628 §1): the flattened elements of the footer read depth first, each
//! node with its definition and repetition level, each leaf a column chunk of every row group.

use super::meta::{Element, Logical, Physical, Repetition};

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub repetition: Repetition,
    pub physical: Option<Physical>,
    pub type_length: usize,
    pub logical: Option<Logical>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    /// The definition level a value here has when it is there: the optional and repeated nodes from the root to it.
    pub def: u16,
    /// The repeated nodes from the root to it.
    pub rep: u16,
    /// Its column, when it is a leaf.
    pub column: Option<usize>,
}

/// The tree: node 0 is the root (the file's message).
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    pub nodes: Vec<Node>,
    /// The leaf node of each column, in the order of the row groups' column chunks.
    pub columns: Vec<usize>,
}

const DEPTH: u16 = 64;

impl Schema {
    pub fn of(elements: &[Element]) -> Result<Schema, String> {
        let mut s = Schema {
            nodes: Vec::with_capacity(elements.len()),
            columns: Vec::new(),
        };
        let mut at = 0usize;
        s.node(elements, &mut at, None, 0)?;
        if at != elements.len() {
            return Err("Parquet şemasının öğeleri çocuk sayılarıyla tutmuyor".into());
        }
        Ok(s)
    }

    fn node(
        &mut self,
        elements: &[Element],
        at: &mut usize,
        parent: Option<usize>,
        depth: u16,
    ) -> Result<(), String> {
        if depth > DEPTH {
            return Err("Parquet şeması çok derin".into());
        }
        let e = elements.get(*at).ok_or("Parquet şeması yarım")?;
        *at += 1;
        let repetition = if parent.is_none() {
            Repetition::Required
        } else {
            e.repetition.unwrap_or(Repetition::Required)
        };
        let (pdef, prep) = parent.map_or((0, 0), |p| (self.nodes[p].def, self.nodes[p].rep));
        let index = self.nodes.len();
        let children = usize::try_from(e.children.unwrap_or(0))
            .map_err(|_| "Parquet şemasında eksi çocuk sayısı")?;
        let leaf = parent.is_some() && e.children.is_none();
        if leaf && e.physical.is_none() {
            return Err(format!("Parquet sütunu “{}” türsüz", e.name));
        }
        self.nodes.push(Node {
            name: e.name.clone(),
            repetition,
            physical: if leaf { e.physical } else { None },
            type_length: usize::try_from(e.type_length.unwrap_or(0).max(0)).unwrap_or(0),
            logical: e.logical.clone(),
            children: Vec::new(),
            parent,
            def: pdef + u16::from(repetition != Repetition::Required),
            rep: prep + u16::from(repetition == Repetition::Repeated),
            column: None,
        });
        if let Some(p) = parent {
            self.nodes[p].children.push(index);
        }
        if leaf {
            self.nodes[index].column = Some(self.columns.len());
            self.columns.push(index);
            return Ok(());
        }
        if children > elements.len() - *at {
            return Err("Parquet şemasının çocuk sayısı öğelerden çok".into());
        }
        for _ in 0..children {
            self.node(elements, at, Some(index), depth + 1)?;
        }
        Ok(())
    }

    /// A column's names from the root's child down.
    pub fn path(&self, column: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut n = self.columns[column];
        while let Some(p) = self.nodes[n].parent {
            out.push(self.nodes[n].name.clone());
            n = p;
        }
        out.reverse();
        out
    }

    /// The definition level of each repeated node on a column's path, root first.
    pub fn repeated_defs(&self, column: usize) -> Vec<u16> {
        let mut out = Vec::new();
        let mut n = Some(self.columns[column]);
        while let Some(i) = n {
            if self.nodes[i].repetition == Repetition::Repeated {
                out.push(self.nodes[i].def);
            }
            n = self.nodes[i].parent;
        }
        out.reverse();
        out
    }

    /// The top-level node a column belongs to (a child of the root).
    pub fn top(&self, column: usize) -> usize {
        let mut n = self.columns[column];
        while let Some(p) = self.nodes[n].parent {
            if p == 0 {
                return n;
            }
            n = p;
        }
        n
    }

    /// The columns under node `n`, in order.
    pub fn columns_under(&self, n: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![n];
        while let Some(i) = stack.pop() {
            if let Some(c) = self.nodes[i].column {
                out.push(c);
            }
            stack.extend(self.nodes[i].children.iter().rev());
        }
        out
    }
}
