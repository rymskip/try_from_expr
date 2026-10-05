//! Registry of the bare names each `TryFromExpr` type accepts, published as
//! associated consts so a wrapper enum can route a bare name to the one child
//! that accepts it, and reject overlaps at compile time with `all_unique`.

use const_panic::concat_panic;
use ordered_float::OrderedFloat;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

/// The syntactic form a bare name takes. Names only collide within a shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaShape {
    /// `name`, including a bare unit path like `Required`.
    Path,
    /// `name = value`
    Assign,
    /// `name(..)`
    Call,
}

const SHAPES: [MetaShape; 3] = [MetaShape::Path, MetaShape::Assign, MetaShape::Call];

/// The names a type accepts without being qualified by a path.
pub struct MetaNameSet {
    /// Type name used in diagnostics.
    pub type_name: &'static str,
    /// Name that selects this type inside a wrapper, as in `string_validator(..)`.
    /// It takes the call shape.
    pub dispatch_name: Option<&'static str>,
    pub path_names: &'static [&'static str],
    pub assign_names: &'static [&'static str],
    pub call_names: &'static [&'static str],
    /// Types whose bare names this type also accepts.
    pub children: &'static [&'static MetaNameSet],
}

/// Implemented by every type a wrapper enum holds. A hand-written
/// `TryFrom<&syn::Expr>` type lists the bare names it accepts, or uses
/// [`MetaNameSet::empty`] when it accepts none.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is held by a wrapper enum but does not publish its meta names",
    label = "a wrapper enum's child must implement `MetaNames`",
    note = "derive `TryFromExpr` for it, or implement `MetaNames` listing the bare names its `TryFrom<&syn::Expr>` accepts"
)]
pub trait MetaNames {
    const META_NAMES: MetaNameSet;
}

/// Where a wrapper sends a bare name.
#[derive(Debug, PartialEq, Eq)]
pub enum MetaRoute {
    /// One of the wrapper's own variants, or its own `type_name(..)` form.
    Own,
    /// The child at this index of [`MetaNameSet::children`].
    Child(usize),
}

/// A bare name that more than one branch of a wrapper accepts.
#[derive(Debug)]
pub struct AmbiguousMetaName {
    pub name: String,
    pub owner: &'static str,
    pub accepted_by: Vec<&'static str>,
    pub qualifiers: Vec<&'static str>,
}

impl fmt::Display for AmbiguousMetaName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Ambiguous meta name `{}` for enum '{}': accepted by {}. ",
            self.name,
            self.owner,
            self.accepted_by.join(", ")
        )?;
        if self.qualifiers.is_empty() {
            write!(formatter, "Use the path form to choose one")
        } else {
            let qualifiers: Vec<_> = self
                .qualifiers
                .iter()
                .map(|qualifier| format!("`{qualifier}(..)`"))
                .collect();
            write!(formatter, "Qualify it with {}", qualifiers.join(" or "))
        }
    }
}

impl std::error::Error for AmbiguousMetaName {}

#[derive(Clone, Copy)]
struct Branch<'a> {
    set: &'a MetaNameSet,
    /// A child branch reaches its own children; a wrapper's own branch does not.
    with_children: bool,
}

impl MetaNameSet {
    /// The set for a type that accepts no bare names, such as a literal.
    pub const fn empty(type_name: &'static str) -> Self {
        Self {
            type_name,
            dispatch_name: None,
            path_names: &[],
            assign_names: &[],
            call_names: &[],
            children: &[],
        }
    }

    const fn names(&self, shape: MetaShape) -> &'static [&'static str] {
        match shape {
            MetaShape::Path => self.path_names,
            MetaShape::Assign => self.assign_names,
            MetaShape::Call => self.call_names,
        }
    }

    /// Routes `name` within this wrapper to the single branch that accepts it.
    pub fn route(
        &self,
        shape: MetaShape,
        name: &str,
    ) -> Result<Option<MetaRoute>, AmbiguousMetaName> {
        let mut routes = Vec::new();
        if branch_accepts(self.own_branch(), shape, name) {
            routes.push(MetaRoute::Own);
        }
        for (index, child) in self.children.iter().enumerate() {
            if branch_accepts(child_branch(child), shape, name) {
                routes.push(MetaRoute::Child(index));
            }
        }
        if routes.len() <= 1 {
            return Ok(routes.pop());
        }

        let mut accepted_by = Vec::new();
        let mut qualifiers = Vec::new();
        for route in &routes {
            let set = match route {
                MetaRoute::Own => self,
                MetaRoute::Child(index) => self.children[*index],
            };
            accepted_by.push(set.type_name);
            // The wrapper's own variants are chosen with the path form instead
            if *route != MetaRoute::Own
                && let Some(dispatch_name) = set.dispatch_name
            {
                qualifiers.push(dispatch_name);
            }
        }
        Err(AmbiguousMetaName {
            name: name.to_string(),
            owner: self.type_name,
            accepted_by,
            qualifiers,
        })
    }

    /// Fails compilation when two branches of this wrapper, or of any wrapper
    /// it holds, accept the same bare name.
    #[track_caller]
    pub const fn assert_all_unique(&self) {
        self.assert_unique_within(self.type_name);
    }

    #[track_caller]
    const fn assert_unique_within(&self, marked: &str) {
        let mut left_index = 0;
        while left_index <= self.children.len() {
            let mut right_index = left_index + 1;
            while right_index <= self.children.len() {
                let left = self.branch(left_index);
                let right = self.branch(right_index);
                if let Some(name) = first_shared(left, right) {
                    concat_panic!(
                        "Meta name `",
                        display: name,
                        "` is accepted by both `",
                        display: left.set.type_name,
                        "` and `",
                        display: right.set.type_name,
                        "` in `",
                        display: self.type_name,
                        "`, and `",
                        display: marked,
                        "` is marked `all_unique`"
                    );
                }
                right_index += 1;
            }
            left_index += 1;
        }

        let mut index = 0;
        while index < self.children.len() {
            self.children[index].assert_unique_within(marked);
            index += 1;
        }
    }

    /// Branch 0 is this wrapper's own; branch `n` is child `n - 1`.
    const fn branch(&self, index: usize) -> Branch<'_> {
        if index == 0 {
            self.own_branch()
        } else {
            child_branch(self.children[index - 1])
        }
    }

    const fn own_branch(&self) -> Branch<'_> {
        Branch {
            set: self,
            with_children: false,
        }
    }
}

const fn child_branch(set: &MetaNameSet) -> Branch<'_> {
    Branch {
        set,
        with_children: true,
    }
}

const fn branch_accepts(branch: Branch<'_>, shape: MetaShape, name: &str) -> bool {
    if let MetaShape::Call = shape
        && let Some(dispatch_name) = branch.set.dispatch_name
        && str_eq(dispatch_name, name)
    {
        return true;
    }
    let names = branch.set.names(shape);
    let mut index = 0;
    while index < names.len() {
        if str_eq(names[index], name) {
            return true;
        }
        index += 1;
    }
    if branch.with_children {
        let mut index = 0;
        while index < branch.set.children.len() {
            if branch_accepts(child_branch(branch.set.children[index]), shape, name) {
                return true;
            }
            index += 1;
        }
    }
    false
}

/// The first name `left` accepts that `right` also accepts in the same shape.
const fn first_shared(left: Branch<'_>, right: Branch<'_>) -> Option<&'static str> {
    if let Some(dispatch_name) = left.set.dispatch_name
        && branch_accepts(right, MetaShape::Call, dispatch_name)
    {
        return Some(dispatch_name);
    }
    let mut shape_index = 0;
    while shape_index < SHAPES.len() {
        let shape = SHAPES[shape_index];
        let names = left.set.names(shape);
        let mut index = 0;
        while index < names.len() {
            if branch_accepts(right, shape, names[index]) {
                return Some(names[index]);
            }
            index += 1;
        }
        shape_index += 1;
    }
    if left.with_children {
        let mut index = 0;
        while index < left.set.children.len() {
            if let Some(name) = first_shared(child_branch(left.set.children[index]), right) {
                return Some(name);
            }
            index += 1;
        }
    }
    None
}

const fn str_eq(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

macro_rules! impl_no_meta_names {
    ($($literal:ty),* $(,)?) => {
        $(
            impl MetaNames for $literal {
                const META_NAMES: MetaNameSet = MetaNameSet::empty(stringify!($literal));
            }
        )*
    };
}

impl_no_meta_names!(
    bool, char, String, f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize,
);

impl<Element> MetaNames for Vec<Element> {
    const META_NAMES: MetaNameSet = MetaNameSet::empty("Vec");
}

impl<Key, Value> MetaNames for HashMap<Key, Value> {
    const META_NAMES: MetaNameSet = MetaNameSet::empty("HashMap");
}

impl<Key, Value> MetaNames for BTreeMap<Key, Value> {
    const META_NAMES: MetaNameSet = MetaNameSet::empty("BTreeMap");
}

impl<Float> MetaNames for OrderedFloat<Float> {
    const META_NAMES: MetaNameSet = MetaNameSet::empty("OrderedFloat");
}

/// A bare value parses as an implicit `Some`, so `Option<T>` accepts `T`'s names.
impl<Inner: MetaNames> MetaNames for Option<Inner> {
    const META_NAMES: MetaNameSet = MetaNameSet {
        children: &[&Inner::META_NAMES],
        ..MetaNameSet::empty("Option")
    };
}
