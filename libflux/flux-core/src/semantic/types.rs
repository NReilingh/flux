//! Semantic representations of types.

use crate::semantic::fresh::{Fresh, Fresher};
use crate::semantic::sub::{Substitutable, Substitution};
use derive_more::Display;
use std::fmt::Write;

use std::{
    cmp,
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
};

/// For use in generics where the specific type of map is not mentioned.
pub type SemanticMap<K, V> = BTreeMap<K, V>;
#[allow(missing_docs)]
pub type SemanticMapIter<'a, K, V> = std::collections::btree_map::Iter<'a, K, V>;

/// A type scheme that quantifies the free variables of a monotype.
#[derive(Debug, Clone)]
pub struct PolyType {
    /// List of the free variables within the monotypes.
    pub vars: Vec<Tvar>,
    /// The list of kind constraints on any of the free variables.
    pub cons: TvarKinds,
    /// The underlying monotype.
    pub expr: MonoType,
}

/// Map of identifier to a polytype that preserves a sorted order when iterating.
pub type PolyTypeMap = SemanticMap<String, PolyType>;
/// Nested map of polytypes that preserves a sorted order when iterating
pub type PolyTypeMapMap = SemanticMap<String, SemanticMap<String, PolyType>>;

/// Alias the maplit literal construction macro so we can specify the type here.
#[macro_export]
macro_rules! semantic_map {
    ( $($x:tt)* ) => ( maplit::btreemap!( $($x)* ) );
}

impl fmt::Display for PolyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.cons.is_empty() {
            self.expr.fmt(f)
        } else {
            write!(
                f,
                "{} where {}",
                self.expr,
                PolyType::display_constraints(&self.cons),
            )
        }
    }
}

impl PartialEq for PolyType {
    fn eq(&self, poly: &Self) -> bool {
        let a: Tvar = self.max_tvar();
        let b: Tvar = poly.max_tvar();

        let max = if a > b { a.0 } else { b.0 };

        let mut f = Fresher::from(max + 1);
        let mut g = Fresher::from(max + 1);

        let mut a = self.clone().fresh(&mut f, &mut TvarMap::new());
        let mut b = poly.clone().fresh(&mut g, &mut TvarMap::new());

        a.vars.sort();
        b.vars.sort();

        for kinds in a.cons.values_mut() {
            kinds.sort();
        }
        for kinds in b.cons.values_mut() {
            kinds.sort();
        }

        a.vars == b.vars && a.cons == b.cons && a.expr == b.expr
    }
}

impl Substitutable for PolyType {
    fn apply(self, sub: &Substitution) -> Self {
        PolyType {
            vars: self.vars,
            cons: self.cons,
            expr: self.expr.apply(sub),
        }
    }
    fn free_vars(&self) -> Vec<Tvar> {
        minus(&self.vars, self.expr.free_vars())
    }
}

impl MaxTvar for Vec<Tvar> {
    fn max_tvar(&self) -> Tvar {
        self.iter()
            .fold(Tvar(0), |max, tv| if *tv > max { *tv } else { max })
    }
}

impl MaxTvar for PolyType {
    fn max_tvar(&self) -> Tvar {
        vec![self.vars.max_tvar(), self.expr.max_tvar()].max_tvar()
    }
}

impl PolyType {
    fn display_constraints(cons: &TvarKinds) -> String {
        cons.iter()
            // A BTree produces a sorted iterator for
            // deterministic display output
            .collect::<BTreeMap<_, _>>()
            .iter()
            .map(|(&&tv, &kinds)| format!("{}: {}", tv, PolyType::display_kinds(kinds)))
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn display_kinds(kinds: &[Kind]) -> String {
        kinds
            .iter()
            // Sort kinds with BTree
            .collect::<BTreeSet<_>>()
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" + ")
    }
    /// Produces a `PolyType` where the type variables have been normalized to start at 0
    /// (i.e. A), instead of whatever type variables are present in the orginal.
    ///
    /// Useful for pretty printing the type in error messages.
    pub fn normal(&self) -> PolyType {
        self.clone()
            .fresh(&mut Fresher::from(0), &mut TvarMap::new())
    }
}

/// Helper function that concatenates two vectors into a single vector while removing duplicates.
pub(crate) fn union<T: PartialEq>(mut vars: Vec<T>, mut with: Vec<T>) -> Vec<T> {
    with.retain(|tv| !vars.contains(tv));
    vars.append(&mut with);
    vars
}

/// Helper function that removes all elements in `vars` from `from`.
pub(crate) fn minus<T: PartialEq>(vars: &[T], mut from: Vec<T>) -> Vec<T> {
    from.retain(|tv| !vars.contains(tv));
    from
}

/// Errors that can be returned during type inference.
/// (Note that these error messages are read by end users.
/// This should be kept in mind when returning one of these errors.)
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
pub enum Error {
    CannotUnify {
        exp: MonoType,
        act: MonoType,
    },
    CannotConstrain {
        exp: Kind,
        act: MonoType,
    },
    OccursCheck(Tvar, MonoType),
    MissingLabel(String),
    ExtraLabel(String),
    CannotUnifyLabel {
        lab: String,
        exp: MonoType,
        act: MonoType,
    },
    MissingArgument(String),
    ExtraArgument(String),
    CannotUnifyArgument(String, Box<Error>),
    CannotUnifyReturn {
        exp: MonoType,
        act: MonoType,
    },
    MissingPipeArgument,
    MultiplePipeArguments {
        exp: String,
        act: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut fresh = Fresher::from(0);
        match self {
            Error::CannotUnify { exp, act } => write!(
                f,
                "expected {} but found {}",
                exp.clone().fresh(&mut fresh, &mut TvarMap::new()),
                act.clone().fresh(&mut fresh, &mut TvarMap::new()),
            ),
            Error::CannotConstrain { exp, act } => write!(
                f,
                "{} is not {}",
                act.clone().fresh(&mut fresh, &mut TvarMap::new()),
                exp,
            ),
            Error::OccursCheck(tv, ty) => {
                write!(f, "recursive types not supported {} != {}", tv, ty)
            }
            Error::MissingLabel(a) => write!(f, "record is missing label {}", a),
            Error::ExtraLabel(a) => write!(f, "found unexpected label {}", a),
            Error::CannotUnifyLabel { lab, exp, act } => write!(
                f,
                "expected {} but found {} for label {}",
                exp.clone().fresh(&mut fresh, &mut TvarMap::new()),
                act.clone().fresh(&mut fresh, &mut TvarMap::new()),
                lab
            ),
            Error::MissingArgument(x) => write!(f, "missing required argument {}", x),
            Error::ExtraArgument(x) => write!(f, "found unexpected argument {}", x),
            Error::CannotUnifyArgument(x, e) => write!(f, "{} (argument {})", e, x),
            Error::CannotUnifyReturn { exp, act } => write!(
                f,
                "expected {} but found {} for return type",
                exp.clone().fresh(&mut fresh, &mut TvarMap::new()),
                act.clone().fresh(&mut fresh, &mut TvarMap::new())
            ),
            Error::MissingPipeArgument => write!(f, "missing pipe argument"),
            Error::MultiplePipeArguments { exp, act } => {
                write!(f, "expected pipe argument {} but found {}", exp, act)
            }
        }
    }
}

/// Represents a constraint on a type variable to a specific kind (*i.e.*, a type class).
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Kind {
    Addable,
    Subtractable,
    Divisible,
    Numeric,
    Comparable,
    Equatable,
    Nullable,
    Record,
    Negatable,
    Timeable,
    Stringable,
}

// Kinds are ordered by name so that polytypes are displayed deterministically
impl cmp::Ord for Kind {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

// Kinds are ordered by name so that polytypes are displayed deterministically
impl cmp::PartialOrd for Kind {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Represents a Flux type. The type may be unknown, represented as a type variable,
/// or may be a known concrete type.
#[derive(Debug, Display, Clone, PartialEq, Serialize)]
#[allow(missing_docs)]
pub enum MonoType {
    #[display(fmt = "bool")]
    Bool,
    #[display(fmt = "int")]
    Int,
    #[display(fmt = "uint")]
    Uint,
    #[display(fmt = "float")]
    Float,
    #[display(fmt = "string")]
    String,
    #[display(fmt = "duration")]
    Duration,
    #[display(fmt = "time")]
    Time,
    #[display(fmt = "regexp")]
    Regexp,
    #[display(fmt = "bytes")]
    Bytes,
    #[display(fmt = "{}", _0)]
    Var(Tvar),
    #[display(fmt = "{}", _0)]
    Arr(Box<Array>),
    #[display(fmt = "{}", _0)]
    Dict(Box<Dictionary>),
    #[display(fmt = "{}", _0)]
    Record(Box<Record>),
    #[display(fmt = "{}", _0)]
    Fun(Box<Function>),
}

/// An ordered map of string identifiers to monotypes.
pub type MonoTypeMap = SemanticMap<String, MonoType>;
#[allow(missing_docs)]
pub type MonoTypeVecMap = SemanticMap<String, Vec<MonoType>>;
#[allow(missing_docs)]
type RefMonoTypeVecMap<'a> = HashMap<&'a String, Vec<&'a MonoType>>;

impl Substitutable for MonoType {
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            MonoType::Bool
            | MonoType::Int
            | MonoType::Uint
            | MonoType::Float
            | MonoType::String
            | MonoType::Duration
            | MonoType::Time
            | MonoType::Regexp
            | MonoType::Bytes => self,
            MonoType::Var(tvr) => sub.apply(tvr),
            MonoType::Arr(arr) => MonoType::Arr(Box::new(arr.apply(sub))),
            MonoType::Dict(dict) => MonoType::Dict(Box::new(dict.apply(sub))),
            MonoType::Record(obj) => MonoType::Record(Box::new(obj.apply(sub))),
            MonoType::Fun(fun) => MonoType::Fun(Box::new(fun.apply(sub))),
        }
    }
    fn free_vars(&self) -> Vec<Tvar> {
        match self {
            MonoType::Bool
            | MonoType::Int
            | MonoType::Uint
            | MonoType::Float
            | MonoType::String
            | MonoType::Duration
            | MonoType::Time
            | MonoType::Regexp
            | MonoType::Bytes => Vec::new(),
            MonoType::Var(tvr) => vec![*tvr],
            MonoType::Arr(arr) => arr.free_vars(),
            MonoType::Dict(dict) => dict.free_vars(),
            MonoType::Record(obj) => obj.free_vars(),
            MonoType::Fun(fun) => fun.free_vars(),
        }
    }
}

impl MaxTvar for MonoType {
    fn max_tvar(&self) -> Tvar {
        match self {
            MonoType::Bool
            | MonoType::Int
            | MonoType::Uint
            | MonoType::Float
            | MonoType::String
            | MonoType::Duration
            | MonoType::Time
            | MonoType::Regexp
            | MonoType::Bytes => Tvar(0),
            MonoType::Var(tvr) => tvr.max_tvar(),
            MonoType::Arr(arr) => arr.max_tvar(),
            MonoType::Dict(dict) => dict.max_tvar(),
            MonoType::Record(obj) => obj.max_tvar(),
            MonoType::Fun(fun) => fun.max_tvar(),
        }
    }
}

impl From<Record> for MonoType {
    fn from(r: Record) -> MonoType {
        MonoType::Record(Box::new(r))
    }
}

impl MonoType {
    /// Performs unification on the type with another type.
    /// If successful, results in a solution to the unification problem,
    /// in the form of a substitution. If there is no solution to the
    /// unification problem then unification fails and an error is reported.
    pub fn unify(
        self, // self represents the expected type
        actual: Self,
        cons: &mut TvarKinds,
        f: &mut Fresher,
    ) -> Result<Substitution, Error> {
        match (self, actual) {
            (MonoType::Bool, MonoType::Bool)
            | (MonoType::Int, MonoType::Int)
            | (MonoType::Uint, MonoType::Uint)
            | (MonoType::Float, MonoType::Float)
            | (MonoType::String, MonoType::String)
            | (MonoType::Duration, MonoType::Duration)
            | (MonoType::Time, MonoType::Time)
            | (MonoType::Regexp, MonoType::Regexp)
            | (MonoType::Bytes, MonoType::Bytes) => Ok(Substitution::empty()),
            (MonoType::Var(tv), t) => tv.unify(t, cons),
            (t, MonoType::Var(tv)) => tv.unify(t, cons),
            (MonoType::Arr(t), MonoType::Arr(s)) => t.unify(*s, cons, f),
            (MonoType::Dict(t), MonoType::Dict(s)) => t.unify(*s, cons, f),
            (MonoType::Record(t), MonoType::Record(s)) => t.unify(*s, cons, f),
            (MonoType::Fun(t), MonoType::Fun(s)) => t.unify(*s, cons, f),
            (exp, act) => Err(Error::CannotUnify { exp, act }),
        }
    }

    /// Validates that the current type meets the constraints of the specified kind.
    pub fn constrain(self, with: Kind, cons: &mut TvarKinds) -> Result<Substitution, Error> {
        match self {
            MonoType::Bool => match with {
                Kind::Equatable | Kind::Nullable | Kind::Stringable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Int => match with {
                Kind::Addable
                | Kind::Subtractable
                | Kind::Divisible
                | Kind::Numeric
                | Kind::Comparable
                | Kind::Equatable
                | Kind::Nullable
                | Kind::Stringable
                | Kind::Negatable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Uint => match with {
                Kind::Addable
                | Kind::Subtractable
                | Kind::Divisible
                | Kind::Numeric
                | Kind::Comparable
                | Kind::Equatable
                | Kind::Nullable
                | Kind::Stringable
                | Kind::Negatable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Float => match with {
                Kind::Addable
                | Kind::Subtractable
                | Kind::Divisible
                | Kind::Numeric
                | Kind::Comparable
                | Kind::Equatable
                | Kind::Nullable
                | Kind::Stringable
                | Kind::Negatable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::String => match with {
                Kind::Addable
                | Kind::Comparable
                | Kind::Equatable
                | Kind::Nullable
                | Kind::Stringable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Duration => match with {
                Kind::Comparable
                | Kind::Equatable
                | Kind::Nullable
                | Kind::Negatable
                | Kind::Stringable
                | Kind::Timeable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Time => match with {
                Kind::Comparable
                | Kind::Equatable
                | Kind::Nullable
                | Kind::Timeable
                | Kind::Stringable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Regexp => Err(Error::CannotConstrain {
                act: self,
                exp: with,
            }),
            MonoType::Bytes => match with {
                Kind::Equatable => Ok(Substitution::empty()),
                _ => Err(Error::CannotConstrain {
                    act: self,
                    exp: with,
                }),
            },
            MonoType::Var(tvr) => {
                tvr.constrain(with, cons);
                Ok(Substitution::empty())
            }
            MonoType::Arr(arr) => arr.constrain(with, cons),
            MonoType::Dict(dict) => dict.constrain(with, cons),
            MonoType::Record(obj) => obj.constrain(with, cons),
            MonoType::Fun(fun) => fun.constrain(with, cons),
        }
    }

    fn contains(&self, tv: Tvar) -> bool {
        match self {
            MonoType::Bool
            | MonoType::Int
            | MonoType::Uint
            | MonoType::Float
            | MonoType::String
            | MonoType::Duration
            | MonoType::Time
            | MonoType::Regexp
            | MonoType::Bytes => false,
            MonoType::Var(tvr) => tv == *tvr,
            MonoType::Arr(arr) => arr.contains(tv),
            MonoType::Dict(dict) => dict.contains(tv),
            MonoType::Record(row) => row.contains(tv),
            MonoType::Fun(fun) => fun.contains(tv),
        }
    }
}

/// `Tvar` stands for *type variable*.
/// A type variable holds an unknown type, before type inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Tvar(pub u64);

/// A map from type variables to their constraining kinds.
pub type TvarKinds = SemanticMap<Tvar, Vec<Kind>>;
#[allow(missing_docs)]
pub type TvarMap = SemanticMap<Tvar, Tvar>;
#[allow(missing_docs)]
pub type SubstitutionMap = SemanticMap<Tvar, MonoType>;

impl fmt::Display for Tvar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0 => write!(f, "A"),
            1 => write!(f, "B"),
            2 => write!(f, "C"),
            3 => write!(f, "D"),
            4 => write!(f, "E"),
            5 => write!(f, "F"),
            6 => write!(f, "G"),
            7 => write!(f, "H"),
            8 => write!(f, "I"),
            9 => write!(f, "J"),
            _ => write!(f, "t{}", self.0),
        }
    }
}

impl MaxTvar for Tvar {
    fn max_tvar(&self) -> Tvar {
        *self
    }
}

impl Tvar {
    fn unify(self, with: MonoType, cons: &mut TvarKinds) -> Result<Substitution, Error> {
        match with {
            MonoType::Var(tv) => {
                if self == tv {
                    // The empty substitution will always
                    // unify a type variable with itself.
                    Ok(Substitution::empty())
                } else {
                    // Unify two distinct type variables.
                    // This will update the kind constraints
                    // associated with these type variables.
                    self.unify_with_tvar(tv, cons)
                }
            }
            _ => {
                if with.contains(self) {
                    // Invalid recursive type
                    Err(Error::OccursCheck(self, with))
                } else {
                    // Unify a type variable with a monotype.
                    // The monotype must satisify any
                    // constraints placed on the type variable.
                    self.unify_with_type(with, cons)
                }
            }
        }
    }

    fn unify_with_tvar(self, tv: Tvar, cons: &mut TvarKinds) -> Result<Substitution, Error> {
        // Kind constraints for both type variables
        let kinds = union(
            cons.remove(&self).unwrap_or_default(),
            cons.remove(&tv).unwrap_or_default(),
        );
        if !kinds.is_empty() {
            cons.insert(tv, kinds);
        }
        Ok(Substitution::from(
            semantic_map! {self => MonoType::Var(tv)},
        ))
    }

    fn unify_with_type(self, t: MonoType, cons: &mut TvarKinds) -> Result<Substitution, Error> {
        let sub = Substitution::from(semantic_map! {self => t.clone()});
        match cons.remove(&self) {
            None => Ok(sub),
            Some(kinds) => Ok(sub.merge(kinds.into_iter().try_fold(
                Substitution::empty(),
                |sub, kind| {
                    // The monotype that is being unified with the
                    // tvar must be constrained with the same kinds
                    // as that of the tvar.
                    Ok(sub.merge(t.clone().constrain(kind, cons)?))
                },
            )?)),
        }
    }

    fn constrain(self, with: Kind, cons: &mut TvarKinds) {
        match cons.get_mut(&self) {
            Some(kinds) => {
                if !kinds.contains(&with) {
                    kinds.push(with);
                }
            }
            None => {
                cons.insert(self, vec![with]);
            }
        }
    }
}

/// A homogeneous list type.
#[derive(Debug, Display, Clone, PartialEq, Serialize)]
#[display(fmt = "[{}]", _0)]
pub struct Array(pub MonoType);

impl Substitutable for Array {
    fn apply(self, sub: &Substitution) -> Self {
        Array(self.0.apply(sub))
    }
    fn free_vars(&self) -> Vec<Tvar> {
        self.0.free_vars()
    }
}

impl MaxTvar for Array {
    fn max_tvar(&self) -> Tvar {
        self.0.max_tvar()
    }
}

impl Array {
    // self represents the expected type.
    fn unify(
        self,
        with: Self,
        cons: &mut TvarKinds,
        f: &mut Fresher,
    ) -> Result<Substitution, Error> {
        self.0.unify(with.0, cons, f)
    }

    fn constrain(self, with: Kind, cons: &mut TvarKinds) -> Result<Substitution, Error> {
        match with {
            Kind::Equatable => self.0.constrain(with, cons),
            _ => Err(Error::CannotConstrain {
                act: MonoType::Arr(Box::new(self)),
                exp: with,
            }),
        }
    }

    fn contains(&self, tv: Tvar) -> bool {
        self.0.contains(tv)
    }
}

/// A key-value data structure.
#[derive(Debug, Display, Clone, PartialEq, Serialize)]
#[display(fmt = "[{}:{}]", key, val)]
pub struct Dictionary {
    /// Type of key.
    pub key: MonoType,
    /// Type of value.
    pub val: MonoType,
}

impl Substitutable for Dictionary {
    fn apply(self, sub: &Substitution) -> Self {
        Dictionary {
            key: self.key.apply(sub),
            val: self.val.apply(sub),
        }
    }
    fn free_vars(&self) -> Vec<Tvar> {
        union(self.key.free_vars(), self.val.free_vars())
    }
}

impl MaxTvar for Dictionary {
    fn max_tvar(&self) -> Tvar {
        vec![self.key.max_tvar(), self.val.max_tvar()].max_tvar()
    }
}

impl Dictionary {
    fn unify(
        self,
        actual: Self,
        cons: &mut TvarKinds,
        f: &mut Fresher,
    ) -> Result<Substitution, Error> {
        let sub = self.key.unify(actual.key, cons, f)?;
        apply_then_unify(self.val, actual.val, sub, cons, f)
    }
    fn constrain(self, with: Kind, _: &mut TvarKinds) -> Result<Substitution, Error> {
        Err(Error::CannotConstrain {
            act: MonoType::Dict(Box::new(self)),
            exp: with,
        })
    }
    fn contains(&self, tv: Tvar) -> bool {
        self.key.contains(tv) || self.val.contains(tv)
    }
}

/// An extensible record type.
///
/// A record is either `Empty`, meaning it has no properties,
/// or it is an extension of a record.
///
/// A record may extend what is referred to as a *record
/// variable*. A record variable is a type variable that
/// represents an unknown record type.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Record {
    /// A record that has no properties.
    Empty,
    /// Extension of a record.
    Extension {
        /// The [`Property`] that extends the record type.
        head: Property,
        /// `tail` is the record variable.
        tail: MonoType,
    },
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("{")?;
        let mut s = String::new();
        let tvar = self.format(&mut s)?;
        if let Some(tv) = tvar {
            write!(f, "{} with ", tv)?;
        }
        if s.len() > 2 {
            // remove trailing ', ' delimiter
            s.truncate(s.len() - 2);
        }
        f.write_str(s.as_str())?;
        f.write_str("}")
    }
}

impl cmp::PartialEq for Record {
    fn eq(mut self: &Self, mut other: &Self) -> bool {
        let mut a = RefMonoTypeVecMap::new();
        let t = loop {
            match self {
                Record::Empty => break None,
                Record::Extension {
                    head,
                    tail: MonoType::Record(o),
                } => {
                    a.entry(&head.k).or_insert_with(Vec::new).push(&head.v);
                    self = o;
                }
                Record::Extension {
                    head,
                    tail: MonoType::Var(t),
                } => {
                    a.entry(&head.k).or_insert_with(Vec::new).push(&head.v);
                    break Some(t);
                }
                _ => return false,
            }
        };
        let mut b = RefMonoTypeVecMap::new();
        let v = loop {
            match other {
                Record::Empty => break None,
                Record::Extension {
                    head,
                    tail: MonoType::Record(o),
                } => {
                    b.entry(&head.k).or_insert_with(Vec::new).push(&head.v);
                    other = o;
                }
                Record::Extension {
                    head,
                    tail: MonoType::Var(t),
                } => {
                    b.entry(&head.k).or_insert_with(Vec::new).push(&head.v);
                    break Some(t);
                }
                _ => return false,
            }
        };
        t == v && a == b
    }
}

impl Substitutable for Record {
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            Record::Empty => Record::Empty,
            Record::Extension { head, tail } => Record::Extension {
                head: head.apply(sub),
                tail: tail.apply(sub),
            },
        }
    }
    fn free_vars(&self) -> Vec<Tvar> {
        match self {
            Record::Empty => Vec::new(),
            Record::Extension { head, tail } => union(tail.free_vars(), head.v.free_vars()),
        }
    }
}

impl MaxTvar for Record {
    fn max_tvar(&self) -> Tvar {
        match self {
            Record::Empty => Tvar(0),
            Record::Extension { head, tail } => vec![head.max_tvar(), tail.max_tvar()].max_tvar(),
        }
    }
}

#[allow(clippy::many_single_char_names)]
impl Record {
    // Below are the rules for record unification. In what follows monotypes
    // are denoted using lowercase letters, and type variables are denoted
    // by a lowercase letter preceded by an apostrophe `'`.
    //
    // `t = u` is read as:
    //
    //     type t unifies with type u
    //
    // `t = u => a = b` is read as:
    //
    //     if t unifies with u, then a must unify with b
    //
    // 1. Two empty records always unify, producing an empty substitution.
    // 2. {a: t | 'r} = {b: u | 'r} => error
    // 3. {a: t | 'r} = {a: u | 'r} => t = u
    // 4. {a: t |  r} = {a: u |  s} => t = u, r = s
    // 5. {a: t |  r} = {b: u |  s} => r = {b: u | 'v}, s = {a: t | 'v}
    //
    // Note rule 2. states that if two records extend the same type variable
    // they must have the same property name otherwise they cannot unify.
    //
    // self represents the expected type.
    //
    fn unify(
        self,
        actual: Self,
        cons: &mut TvarKinds,
        f: &mut Fresher,
    ) -> Result<Substitution, Error> {
        match (self.clone(), actual.clone()) {
            (Record::Empty, Record::Empty) => Ok(Substitution::empty()),
            (
                Record::Extension {
                    head: Property { k: a, v: t },
                    tail: MonoType::Var(l),
                },
                Record::Extension {
                    head: Property { k: b, v: u },
                    tail: MonoType::Var(r),
                },
            ) if a == b && l == r => match t.clone().unify(u.clone(), cons, f) {
                Err(_) => Err(Error::CannotUnifyLabel {
                    lab: a,
                    exp: t,
                    act: u,
                }),
                Ok(sub) => Ok(sub),
            },
            (
                Record::Extension {
                    head: Property { k: a, .. },
                    tail: MonoType::Var(l),
                },
                Record::Extension {
                    head: Property { k: b, .. },
                    tail: MonoType::Var(r),
                },
            ) if a != b && l == r => Err(Error::CannotUnify {
                exp: MonoType::Record(Box::new(self)),
                act: MonoType::Record(Box::new(actual)),
            }),
            (
                Record::Extension {
                    head: Property { k: a, v: t },
                    tail: l,
                },
                Record::Extension {
                    head: Property { k: b, v: u },
                    tail: r,
                },
            ) if a == b => {
                let sub = t.unify(u, cons, f)?;
                apply_then_unify(l, r, sub, cons, f)
            }
            (
                Record::Extension {
                    head: Property { k: a, v: t },
                    tail: l,
                },
                Record::Extension {
                    head: Property { k: b, v: u },
                    tail: r,
                },
            ) if a != b => {
                let var = f.fresh();
                let exp = MonoType::from(Record::Extension {
                    head: Property { k: a, v: t },
                    tail: MonoType::Var(var),
                });
                let act = MonoType::from(Record::Extension {
                    head: Property { k: b, v: u },
                    tail: MonoType::Var(var),
                });
                let sub = l.unify(act, cons, f)?;
                apply_then_unify(exp, r, sub, cons, f)
            }
            // If we are expecting {a: u | r} but find {}, label `a` is missing.
            (
                Record::Extension {
                    head: Property { k: a, .. },
                    ..
                },
                Record::Empty,
            ) => Err(Error::MissingLabel(a)),
            // If we are expecting {} but find {a: u | r}, label `a` is extra.
            (
                Record::Empty,
                Record::Extension {
                    head: Property { k: a, .. },
                    ..
                },
            ) => Err(Error::ExtraLabel(a)),
            _ => Err(Error::CannotUnify {
                exp: MonoType::Record(Box::new(self)),
                act: MonoType::Record(Box::new(actual)),
            }),
        }
    }

    fn constrain(self, with: Kind, cons: &mut TvarKinds) -> Result<Substitution, Error> {
        match with {
            Kind::Record => Ok(Substitution::empty()),
            Kind::Equatable => match self {
                Record::Empty => Ok(Substitution::empty()),
                Record::Extension { head, tail } => {
                    let sub = head.v.constrain(with, cons)?;
                    Ok(sub.merge(tail.constrain(with, cons)?))
                }
            },
            _ => Err(Error::CannotConstrain {
                act: MonoType::Record(Box::new(self)),
                exp: with,
            }),
        }
    }

    fn contains(&self, tv: Tvar) -> bool {
        match self {
            Record::Empty => false,
            Record::Extension { head, tail } => head.v.contains(tv) && tail.contains(tv),
        }
    }

    fn format(&self, f: &mut String) -> Result<Option<Tvar>, fmt::Error> {
        match self {
            Record::Empty => Ok(None),
            Record::Extension { head, tail } => match tail {
                MonoType::Var(tv) => {
                    write!(f, "{}, ", head)?;
                    Ok(Some(*tv))
                }
                MonoType::Record(obj) => {
                    write!(f, "{}, ", head)?;
                    obj.format(f)
                }
                _ => Err(fmt::Error),
            },
        }
    }
}

// Unification requires that the current substitution be applied
// to both sides of a constraint before unifying.
//
// This helper function applies a substitution to a constraint
// before unifying the two types. Note the substitution produced
// from unification is merged with input substitution before it
// is returned.
//
fn apply_then_unify(
    exp: MonoType,
    act: MonoType,
    sub: Substitution,
    cons: &mut TvarKinds,
    f: &mut Fresher,
) -> Result<Substitution, Error> {
    let s = exp.apply(&sub).unify(act.apply(&sub), cons, f)?;
    Ok(sub.merge(s))
}

/// A key-value pair representing a property type in a record.
#[derive(Debug, Display, Clone, PartialEq, Serialize)]
#[display(fmt = "{}:{}", k, v)]
#[allow(missing_docs)]
pub struct Property {
    pub k: String,
    pub v: MonoType,
}

impl Substitutable for Property {
    fn apply(self, sub: &Substitution) -> Self {
        Property {
            k: self.k,
            v: self.v.apply(sub),
        }
    }
    fn free_vars(&self) -> Vec<Tvar> {
        self.v.free_vars()
    }
}

impl MaxTvar for Property {
    fn max_tvar(&self) -> Tvar {
        self.v.max_tvar()
    }
}

/// Represents a function type.
///
/// A function type is defined by a set of required arguments,
/// a set of optional arguments, an optional pipe argument, and
/// a required return type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Function {
    /// Required arguments to a function.
    pub req: MonoTypeMap,
    /// Optional arguments to a function.
    pub opt: MonoTypeMap,
    /// An optional pipe argument.
    pub pipe: Option<Property>,
    /// Required return type.
    pub retn: MonoType,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let required = self
            .req
            .iter()
            // Sort args with BTree
            .collect::<BTreeMap<_, _>>()
            .iter()
            .map(|(&k, &v)| Property {
                k: k.clone(),
                v: v.clone(),
            })
            .collect::<Vec<_>>();

        let optional = self
            .opt
            .iter()
            // Sort args with BTree
            .collect::<BTreeMap<_, _>>()
            .iter()
            .map(|(&k, &v)| Property {
                k: String::from("?") + k,
                v: v.clone(),
            })
            .collect::<Vec<_>>();

        let pipe = match &self.pipe {
            Some(pipe) => {
                if pipe.k == "<-" {
                    vec![pipe.clone()]
                } else {
                    vec![Property {
                        k: String::from("<-") + &pipe.k,
                        v: pipe.v.clone(),
                    }]
                }
            }
            None => vec![],
        };

        write!(
            f,
            "({}) => {}",
            pipe.iter()
                .chain(required.iter().chain(optional.iter()))
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.retn
        )
    }
}

#[allow(clippy::implicit_hasher)]
impl<T: Substitutable> Substitutable for SemanticMap<String, T> {
    fn apply(self, sub: &Substitution) -> Self {
        self.into_iter().map(|(k, v)| (k, v.apply(sub))).collect()
    }
    fn free_vars(&self) -> Vec<Tvar> {
        self.values()
            .fold(Vec::new(), |vars, t| union(vars, t.free_vars()))
    }
}

impl<T: Substitutable> Substitutable for Option<T> {
    fn apply(self, sub: &Substitution) -> Self {
        self.map(|t| t.apply(sub))
    }
    fn free_vars(&self) -> Vec<Tvar> {
        match self {
            Some(t) => t.free_vars(),
            None => Vec::new(),
        }
    }
}

impl Substitutable for Function {
    fn apply(self, sub: &Substitution) -> Self {
        Function {
            req: self.req.apply(sub),
            opt: self.opt.apply(sub),
            pipe: self.pipe.apply(sub),
            retn: self.retn.apply(sub),
        }
    }
    fn free_vars(&self) -> Vec<Tvar> {
        union(
            self.req.free_vars(),
            union(
                self.opt.free_vars(),
                union(self.pipe.free_vars(), self.retn.free_vars()),
            ),
        )
    }
}

impl<U, T: MaxTvar> MaxTvar for SemanticMap<U, T> {
    fn max_tvar(&self) -> Tvar {
        self.iter()
            .map(|(_, t)| t.max_tvar())
            .fold(Tvar(0), |max, tv| if tv > max { tv } else { max })
    }
}

impl<T: MaxTvar> MaxTvar for Option<T> {
    fn max_tvar(&self) -> Tvar {
        match self {
            None => Tvar(0),
            Some(t) => t.max_tvar(),
        }
    }
}

impl MaxTvar for Function {
    fn max_tvar(&self) -> Tvar {
        vec![
            self.req.max_tvar(),
            self.opt.max_tvar(),
            self.pipe.max_tvar(),
            self.retn.max_tvar(),
        ]
        .max_tvar()
    }
}

impl Function {
    /// Given two function types f and g, the process for unifying their arguments is as follows:
    /// 1. If a required arg of f is not present in the arguments of g,
    ///    otherwise unify both argument types.
    /// 2. If an optional arg of f is not present in the arguments of g, continue,
    ///    otherwise unify both argument types (repeat for g).
    /// 3. Lastly unify pipe args. Note that pipe arguments are optional.
    ///    However if a pipe arg was used in a calling context, i.e it's an un-named pipe arg,
    ///    then the other type must specify a pipe arg too, otherwise unification fails.
    ///
    /// For pipe arguments, it becomes quite tricky. Take these statements:
    ///
    /// 1. f = (a=<-, b) => {...}
    /// 2. 0 |> f(b: 1)
    /// 3. f(a: 0, b: 1)
    /// 4. f = (d=<-, b, c=0) => {...}
    ///
    /// 2 and 3 are two equivalent ways of invoking 1, and they should both unify.
    /// `a` is the named pipe argument in 1. In 2, the pipe argument is unnamed.
    ///
    /// Unify 1 and 2: one of the required arguments of 1 will not be in its call,
    /// so, we should check for the pipe argument and succeed. If we do the other way around (unify
    /// 2 with 1), the unnamed pipe argument unifies with the other pipe argument.
    ///
    /// Unify 1 and 3: no problem, required arguments are satisfied. Take care that, if you unify
    /// 3 with 1, you will find `a` in 1's pipe argument.
    ///
    /// Unify 1 and 4: should fail because `d` != `a`.
    ///
    /// Unify 2 and 3: should fail because `a` is not in the arguments of 2.
    ///
    /// Unify 2 and 4: should succeed, the same as 1 and 2.
    ///
    /// Unify 3 and 4: should fail because `a` is not in the arguments of 4.
    ///
    /// self represents the expected type.
    fn unify(
        self,
        actual: Self,
        cons: &mut TvarKinds,
        fresh: &mut Fresher,
    ) -> Result<Substitution, Error> {
        // Some aliasing for coherence with the doc.
        let mut f = self;
        let mut g = actual;
        // Fix pipe arguments:
        // Make them required arguments with the correct name.
        match (f.pipe, g.pipe) {
            // Both functions have pipe arguments.
            (Some(fp), Some(gp)) => {
                if fp.k != "<-" && gp.k != "<-" && fp.k != gp.k {
                    // Both are named and the name differs, fail unification.
                    return Err(Error::MultiplePipeArguments {
                        exp: fp.k,
                        act: gp.k,
                    });
                } else {
                    // At least one is unnamed or they are both named with the same name.
                    // This means they should match. Enforce this condition by inserting
                    // the pipe argument into the required ones with the same key.
                    f.req.insert(fp.k.clone(), fp.v);
                    g.req.insert(fp.k, gp.v);
                }
            }
            // F has a pipe argument and g does not.
            (Some(fp), None) => {
                if fp.k == "<-" {
                    // The pipe argument is unnamed and g does not have one.
                    // Fail unification.
                    return Err(Error::MissingPipeArgument);
                } else {
                    // This is a named argument, simply put it into the required ones.
                    f.req.insert(fp.k, fp.v);
                }
            }
            // G has a pipe argument and f does not.
            (None, Some(gp)) => {
                if gp.k == "<-" {
                    // The pipe argument is unnamed and f does not have one.
                    // Fail unification.
                    return Err(Error::MissingPipeArgument);
                } else {
                    // This is a named argument, simply put it into the required ones.
                    g.req.insert(gp.k, gp.v);
                }
            }
            // Nothing to do.
            (None, None) => (),
        }
        // Now that f has not been consumed yet, check that every required argument in g is in f too.
        for (name, _) in g.req.iter() {
            if !f.req.contains_key(name) && !f.opt.contains_key(name) {
                return Err(Error::ExtraArgument(String::from(name)));
            }
        }
        let mut sub = Substitution::empty();
        // Unify f's required arguments.
        for (name, exp) in f.req.into_iter() {
            if let Some(act) = g.req.remove(&name) {
                // The required argument is in g's required arguments.
                sub = match apply_then_unify(exp.clone(), act.clone(), sub, cons, fresh) {
                    Err(e) => Err(Error::CannotUnifyArgument(name, Box::new(e))),
                    Ok(sub) => Ok(sub),
                }?;
            } else if let Some(act) = g.opt.remove(&name) {
                // The required argument is in g's optional arguments.
                sub = match apply_then_unify(exp.clone(), act.clone(), sub, cons, fresh) {
                    Err(e) => Err(Error::CannotUnifyArgument(name, Box::new(e))),
                    Ok(sub) => Ok(sub),
                }?;
            } else {
                return Err(Error::MissingArgument(name));
            }
        }
        // Unify f's optional arguments.
        for (name, exp) in f.opt.into_iter() {
            if let Some(act) = g.req.remove(&name) {
                sub = match apply_then_unify(exp.clone(), act.clone(), sub, cons, fresh) {
                    Err(e) => Err(Error::CannotUnifyArgument(name, Box::new(e))),
                    Ok(sub) => Ok(sub),
                }?;
            } else if let Some(act) = g.opt.remove(&name) {
                sub = match apply_then_unify(exp.clone(), act.clone(), sub, cons, fresh) {
                    Err(e) => Err(Error::CannotUnifyArgument(name, Box::new(e))),
                    Ok(sub) => Ok(sub),
                }?;
            }
        }
        // Unify return types.
        match apply_then_unify(f.retn.clone(), g.retn.clone(), sub, cons, fresh) {
            Err(_) => Err(Error::CannotUnifyReturn {
                exp: f.retn,
                act: g.retn,
            }),
            Ok(sub) => Ok(sub),
        }
    }

    fn constrain(self, with: Kind, _: &mut TvarKinds) -> Result<Substitution, Error> {
        Err(Error::CannotConstrain {
            act: MonoType::Fun(Box::new(self)),
            exp: with,
        })
    }

    fn contains(&self, tv: Tvar) -> bool {
        if let Some(pipe) = &self.pipe {
            self.req.values().any(|t| t.contains(tv))
                || self.opt.values().any(|t| t.contains(tv))
                || pipe.v.contains(tv)
                || self.retn.contains(tv)
        } else {
            self.req.values().any(|t| t.contains(tv))
                || self.opt.values().any(|t| t.contains(tv))
                || self.retn.contains(tv)
        }
    }
}

/// Trait for returning the maximum type variable of a type.
pub trait MaxTvar {
    /// Return the maximum type variable of a type.
    fn max_tvar(&self) -> Tvar;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::get_err_type_expression;
    use crate::parser;
    use crate::semantic::convert::convert_polytype;
    /// `polytype` is a utility method that returns a `PolyType` from a string.
    pub fn polytype(typ: &str) -> PolyType {
        let mut p = parser::Parser::new(typ);

        let typ_expr = p.parse_type_expression();
        let err = get_err_type_expression(typ_expr.clone());

        if err != "" {
            panic!("TypeExpression parsing failed for {}. {:?}", typ, err);
        }
        convert_polytype(typ_expr, &mut Fresher::default()).unwrap()
    }

    #[test]
    fn display_kind_addable() {
        assert!(Kind::Addable.to_string() == "Addable");
    }
    #[test]
    fn display_kind_subtractable() {
        assert!(Kind::Subtractable.to_string() == "Subtractable");
    }
    #[test]
    fn display_kind_divisible() {
        assert!(Kind::Divisible.to_string() == "Divisible");
    }
    #[test]
    fn display_kind_numeric() {
        assert!(Kind::Numeric.to_string() == "Numeric");
    }
    #[test]
    fn display_kind_comparable() {
        assert!(Kind::Comparable.to_string() == "Comparable");
    }
    #[test]
    fn display_kind_equatable() {
        assert!(Kind::Equatable.to_string() == "Equatable");
    }
    #[test]
    fn display_kind_nullable() {
        assert!(Kind::Nullable.to_string() == "Nullable");
    }
    #[test]
    fn display_kind_row() {
        assert!(Kind::Record.to_string() == "Record");
    }
    #[test]
    fn display_kind_stringable() {
        assert!(Kind::Stringable.to_string() == "Stringable");
    }

    #[test]
    fn display_type_bool() {
        assert_eq!("bool", MonoType::Bool.to_string());
    }
    #[test]
    fn display_type_int() {
        assert_eq!("int", MonoType::Int.to_string());
    }
    #[test]
    fn display_type_uint() {
        assert_eq!("uint", MonoType::Uint.to_string());
    }
    #[test]
    fn display_type_float() {
        assert_eq!("float", MonoType::Float.to_string());
    }
    #[test]
    fn display_type_string() {
        assert_eq!("string", MonoType::String.to_string());
    }
    #[test]
    fn display_type_duration() {
        assert_eq!("duration", MonoType::Duration.to_string());
    }
    #[test]
    fn display_type_time() {
        assert_eq!("time", MonoType::Time.to_string());
    }
    #[test]
    fn display_type_regexp() {
        assert_eq!("regexp", MonoType::Regexp.to_string());
    }
    #[test]
    fn display_type_bytes() {
        assert_eq!("bytes", MonoType::Bytes.to_string());
    }
    #[test]
    fn display_type_tvar() {
        assert_eq!("t10", MonoType::Var(Tvar(10)).to_string());
    }
    #[test]
    fn display_type_array() {
        assert_eq!(
            "[int]",
            MonoType::Arr(Box::new(Array(MonoType::Int))).to_string()
        );
    }
    #[test]
    fn display_type_record() {
        assert_eq!(
            "{A with a:int, b:string}",
            Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Var(Tvar(0)),
                })),
            }
            .to_string()
        );
        assert_eq!(
            "{a:int, b:string}",
            Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Record(Box::new(Record::Empty)),
                })),
            }
            .to_string()
        );
    }
    #[test]
    fn display_type_function() {
        assert_eq!(
            "() => int",
            Function {
                req: MonoTypeMap::new(),
                opt: MonoTypeMap::new(),
                pipe: None,
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(<-:int) => int",
            Function {
                req: MonoTypeMap::new(),
                opt: MonoTypeMap::new(),
                pipe: Some(Property {
                    k: String::from("<-"),
                    v: MonoType::Int,
                }),
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(<-a:int) => int",
            Function {
                req: MonoTypeMap::new(),
                opt: MonoTypeMap::new(),
                pipe: Some(Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                }),
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(<-:int, a:int, b:int) => int",
            Function {
                req: semantic_map! {
                    String::from("a") => MonoType::Int,
                    String::from("b") => MonoType::Int,
                },
                opt: MonoTypeMap::new(),
                pipe: Some(Property {
                    k: String::from("<-"),
                    v: MonoType::Int,
                }),
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(<-:int, ?a:int, ?b:int) => int",
            Function {
                req: MonoTypeMap::new(),
                opt: semantic_map! {
                    String::from("a") => MonoType::Int,
                    String::from("b") => MonoType::Int,
                },
                pipe: Some(Property {
                    k: String::from("<-"),
                    v: MonoType::Int,
                }),
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(<-:int, a:int, b:int, ?c:int, ?d:int) => int",
            Function {
                req: semantic_map! {
                    String::from("a") => MonoType::Int,
                    String::from("b") => MonoType::Int,
                },
                opt: semantic_map! {
                    String::from("c") => MonoType::Int,
                    String::from("d") => MonoType::Int,
                },
                pipe: Some(Property {
                    k: String::from("<-"),
                    v: MonoType::Int,
                }),
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(a:int, ?b:bool) => int",
            Function {
                req: semantic_map! {
                    String::from("a") => MonoType::Int,
                },
                opt: semantic_map! {
                    String::from("b") => MonoType::Bool,
                },
                pipe: None,
                retn: MonoType::Int,
            }
            .to_string()
        );
        assert_eq!(
            "(<-a:int, b:int, c:int, ?d:bool) => int",
            Function {
                req: semantic_map! {
                    String::from("b") => MonoType::Int,
                    String::from("c") => MonoType::Int,
                },
                opt: semantic_map! {
                    String::from("d") => MonoType::Bool,
                },
                pipe: Some(Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                }),
                retn: MonoType::Int,
            }
            .to_string()
        );
    }

    #[test]
    fn display_polytype() {
        assert_eq!(
            "int",
            PolyType {
                vars: Vec::new(),
                cons: TvarKinds::new(),
                expr: MonoType::Int,
            }
            .to_string(),
        );
        assert_eq!(
            "(x:A) => A",
            PolyType {
                vars: vec![Tvar(0)],
                cons: TvarKinds::new(),
                expr: MonoType::Fun(Box::new(Function {
                    req: semantic_map! {
                        String::from("x") => MonoType::Var(Tvar(0)),
                    },
                    opt: MonoTypeMap::new(),
                    pipe: None,
                    retn: MonoType::Var(Tvar(0)),
                })),
            }
            .to_string(),
        );
        assert_eq!(
            "(x:A, y:B) => {x:A, y:B}",
            PolyType {
                vars: vec![Tvar(0), Tvar(1)],
                cons: TvarKinds::new(),
                expr: MonoType::Fun(Box::new(Function {
                    req: semantic_map! {
                        String::from("x") => MonoType::Var(Tvar(0)),
                        String::from("y") => MonoType::Var(Tvar(1)),
                    },
                    opt: MonoTypeMap::new(),
                    pipe: None,
                    retn: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("x"),
                            v: MonoType::Var(Tvar(0)),
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("y"),
                                v: MonoType::Var(Tvar(1)),
                            },
                            tail: MonoType::Record(Box::new(Record::Empty)),
                        })),
                    })),
                })),
            }
            .to_string(),
        );
        assert_eq!(
            "(a:A, b:A) => A where A: Addable",
            PolyType {
                vars: vec![Tvar(0)],
                cons: semantic_map! {Tvar(0) => vec![Kind::Addable]},
                expr: MonoType::Fun(Box::new(Function {
                    req: semantic_map! {
                        String::from("a") => MonoType::Var(Tvar(0)),
                        String::from("b") => MonoType::Var(Tvar(0)),
                    },
                    opt: MonoTypeMap::new(),
                    pipe: None,
                    retn: MonoType::Var(Tvar(0)),
                })),
            }
            .to_string(),
        );
        assert_eq!(
            "(x:A, y:B) => {x:A, y:B} where A: Addable, B: Divisible",
            PolyType {
                vars: vec![Tvar(0), Tvar(1)],
                cons: semantic_map! {
                    Tvar(0) => vec![Kind::Addable],
                    Tvar(1) => vec![Kind::Divisible],
                },
                expr: MonoType::Fun(Box::new(Function {
                    req: semantic_map! {
                        String::from("x") => MonoType::Var(Tvar(0)),
                        String::from("y") => MonoType::Var(Tvar(1)),
                    },
                    opt: MonoTypeMap::new(),
                    pipe: None,
                    retn: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("x"),
                            v: MonoType::Var(Tvar(0)),
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("y"),
                                v: MonoType::Var(Tvar(1)),
                            },
                            tail: MonoType::Record(Box::new(Record::Empty)),
                        })),
                    })),
                })),
            }
            .to_string(),
        );
        assert_eq!(
            "(x:A, y:B) => {x:A, y:B} where A: Comparable + Equatable, B: Addable + Divisible",
            PolyType {
                vars: vec![Tvar(0), Tvar(1)],
                cons: semantic_map! {
                    Tvar(0) => vec![Kind::Comparable, Kind::Equatable],
                    Tvar(1) => vec![Kind::Addable, Kind::Divisible],
                },
                expr: MonoType::Fun(Box::new(Function {
                    req: semantic_map! {
                        String::from("x") => MonoType::Var(Tvar(0)),
                        String::from("y") => MonoType::Var(Tvar(1)),
                    },
                    opt: MonoTypeMap::new(),
                    pipe: None,
                    retn: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("x"),
                            v: MonoType::Var(Tvar(0)),
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("y"),
                                v: MonoType::Var(Tvar(1)),
                            },
                            tail: MonoType::Record(Box::new(Record::Empty)),
                        })),
                    })),
                })),
            }
            .to_string(),
        );
    }

    #[test]
    fn compare_records() {
        assert_eq!(
            // {A with a:int, b:string}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Var(Tvar(0)),
                })),
            })),
            // {A with b:string, a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("b"),
                    v: MonoType::String,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("a"),
                        v: MonoType::Int,
                    },
                    tail: MonoType::Var(Tvar(0)),
                })),
            })),
        );
        assert_eq!(
            // {A with a:int, b:string, b:int, c:float}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("b"),
                            v: MonoType::Int,
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("c"),
                                v: MonoType::Float,
                            },
                            tail: MonoType::Var(Tvar(0)),
                        })),
                    })),
                })),
            })),
            // {A with c:float, b:string, b:int, a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("c"),
                    v: MonoType::Float,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("b"),
                            v: MonoType::Int,
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("a"),
                                v: MonoType::Int,
                            },
                            tail: MonoType::Var(Tvar(0)),
                        })),
                    })),
                })),
            })),
        );
        assert_ne!(
            // {A with a:int, b:string, b:int, c:float}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("b"),
                            v: MonoType::Int,
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("c"),
                                v: MonoType::Float,
                            },
                            tail: MonoType::Var(Tvar(0)),
                        })),
                    })),
                })),
            })),
            // {A with a:int, b:int, b:string, c:float}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::Int,
                    },
                    tail: MonoType::Record(Box::new(Record::Extension {
                        head: Property {
                            k: String::from("b"),
                            v: MonoType::String,
                        },
                        tail: MonoType::Record(Box::new(Record::Extension {
                            head: Property {
                                k: String::from("c"),
                                v: MonoType::Float,
                            },
                            tail: MonoType::Var(Tvar(0)),
                        })),
                    })),
                })),
            })),
        );
        assert_ne!(
            // {a:int, b:string}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("b"),
                        v: MonoType::String,
                    },
                    tail: MonoType::Record(Box::new(Record::Empty)),
                })),
            })),
            // {b:int, a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("b"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Extension {
                    head: Property {
                        k: String::from("a"),
                        v: MonoType::Int,
                    },
                    tail: MonoType::Record(Box::new(Record::Empty)),
                })),
            })),
        );
        assert_ne!(
            // {a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Record(Box::new(Record::Empty)),
            })),
            // {A with a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Var(Tvar(0)),
            })),
        );
        assert_ne!(
            // {A with a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Var(Tvar(0)),
            })),
            // {B with a:int}
            MonoType::Record(Box::new(Record::Extension {
                head: Property {
                    k: String::from("a"),
                    v: MonoType::Int,
                },
                tail: MonoType::Var(Tvar(1)),
            })),
        );
    }

    #[test]
    fn unify_ints() {
        let sub = MonoType::Int
            .unify(
                MonoType::Int,
                &mut TvarKinds::new(),
                &mut Fresher::default(),
            )
            .unwrap();
        assert_eq!(sub, Substitution::empty());
    }
    #[test]
    fn constrain_ints() {
        let allowable_cons = vec![
            Kind::Addable,
            Kind::Subtractable,
            Kind::Divisible,
            Kind::Numeric,
            Kind::Comparable,
            Kind::Equatable,
            Kind::Nullable,
            Kind::Stringable,
        ];
        for c in allowable_cons {
            let sub = MonoType::Int.constrain(c, &mut TvarKinds::new());
            assert_eq!(Ok(Substitution::empty()), sub);
        }

        let sub = MonoType::Int.constrain(Kind::Record, &mut TvarKinds::new());
        assert_eq!(
            Err(Error::CannotConstrain {
                act: MonoType::Int,
                exp: Kind::Record
            }),
            sub
        );
    }
    #[test]
    fn constrain_rows() {
        let sub = Record::Empty.constrain(Kind::Record, &mut TvarKinds::new());
        assert_eq!(Ok(Substitution::empty()), sub);

        let unallowable_cons = vec![
            Kind::Addable,
            Kind::Subtractable,
            Kind::Divisible,
            Kind::Numeric,
            Kind::Comparable,
            Kind::Nullable,
        ];
        for c in unallowable_cons {
            let sub = Record::Empty.constrain(c, &mut TvarKinds::new());
            assert_eq!(
                Err(Error::CannotConstrain {
                    act: MonoType::Record(Box::new(Record::Empty)),
                    exp: c
                }),
                sub
            );
        }
    }
    #[test]
    fn unify_error() {
        let err = MonoType::Int
            .unify(
                MonoType::String,
                &mut TvarKinds::new(),
                &mut Fresher::default(),
            )
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            String::from("expected int but found string"),
        );
    }
    #[test]
    fn unify_tvars() {
        let sub = MonoType::Var(Tvar(0))
            .unify(
                MonoType::Var(Tvar(1)),
                &mut TvarKinds::new(),
                &mut Fresher::default(),
            )
            .unwrap();
        assert_eq!(
            sub,
            Substitution::from(semantic_map! {Tvar(0) => MonoType::Var(Tvar(1))}),
        );
    }
    #[test]
    fn unify_constrained_tvars() {
        let mut cons = semantic_map! {Tvar(0) => vec![Kind::Addable, Kind::Divisible]};
        let sub = MonoType::Var(Tvar(0))
            .unify(MonoType::Var(Tvar(1)), &mut cons, &mut Fresher::default())
            .unwrap();
        assert_eq!(
            sub,
            Substitution::from(semantic_map! {Tvar(0) => MonoType::Var(Tvar(1))})
        );
        assert_eq!(
            cons,
            semantic_map! {Tvar(1) => vec![Kind::Addable, Kind::Divisible]},
        );
    }
    #[test]
    fn cannot_unify_functions() {
        // g-required and g-optional arguments do not contain a f-required argument (and viceversa).
        let f = polytype("(a: A, b: A, ?c: B) => A where A: Addable, B: Divisible ");
        let g = polytype("(d: C, ?e: C) => C where C: Addable ");
        if let (
            PolyType {
                vars: _,
                cons: f_cons,
                expr: MonoType::Fun(f),
            },
            PolyType {
                vars: _,
                cons: g_cons,
                expr: MonoType::Fun(g),
            },
        ) = (f, g)
        {
            // this extends the first map with the second by generating a new one.
            let mut cons = f_cons.into_iter().chain(g_cons).collect();
            let res = f
                .clone()
                .unify(*g.clone(), &mut cons, &mut Fresher::default());
            assert!(res.is_err());
            let res = g
                .clone()
                .unify(*f.clone(), &mut cons, &mut Fresher::default());
            assert!(res.is_err());
        } else {
            panic!("the monotypes under examination are not functions");
        }
        // f has a pipe argument, but g does not (and viceversa).
        let f = polytype("(<-pip:A, a: B) => A where A: Addable, B: Divisible ");
        let g = polytype("(a: C) => C where C: Addable ");
        if let (
            PolyType {
                vars: _,
                cons: f_cons,
                expr: MonoType::Fun(f),
            },
            PolyType {
                vars: _,
                cons: g_cons,
                expr: MonoType::Fun(g),
            },
        ) = (f, g)
        {
            let mut cons = f_cons.into_iter().chain(g_cons).collect();
            let res = f
                .clone()
                .unify(*g.clone(), &mut cons, &mut Fresher::default());
            assert!(res.is_err());
            let res = g
                .clone()
                .unify(*f.clone(), &mut cons, &mut Fresher::default());
            assert!(res.is_err());
        } else {
            panic!("the monotypes under examination are not functions");
        }
    }
    #[test]
    fn unify_function_with_function_call() {
        let fn_type = polytype("(a: A, b: A, ?c: B) => A where A: Addable, B: Divisible ");
        // (a: int, b: int) => int
        let call_type = Function {
            // all arguments are required in a function call.
            req: semantic_map! {
                "a".to_string() => MonoType::Int,
                "b".to_string() => MonoType::Int,
            },
            opt: semantic_map! {},
            pipe: None,
            retn: MonoType::Int,
        };
        if let PolyType {
            vars: _,
            mut cons,
            expr: MonoType::Fun(f),
        } = fn_type
        {
            let sub = f
                .unify(call_type, &mut cons, &mut Fresher::default())
                .unwrap();
            assert_eq!(
                sub,
                Substitution::from(semantic_map! {Tvar(0) => MonoType::Int})
            );
            // the constraint on A gets removed.
            assert_eq!(cons, semantic_map! {Tvar(1) => vec![Kind::Divisible]});
        } else {
            panic!("the monotype under examination is not a function");
        }
    }
    #[test]
    fn unify_higher_order_functions() {
        let f = polytype(
            "(a: A, b: A, ?c: (a: A) => B) => (d:  string) => A where A: Addable, B: Divisible ",
        );
        let g = polytype("(a: int, b: int, c: (a: int) => float) => (d: string) => int");
        if let (
            PolyType {
                vars: _,
                cons: f_cons,
                expr: MonoType::Fun(f),
            },
            PolyType {
                vars: _,
                cons: g_cons,
                expr: MonoType::Fun(g),
            },
        ) = (f, g)
        {
            // this extends the first map with the second by generating a new one.
            let mut cons = f_cons.into_iter().chain(g_cons).collect();
            let sub = f.unify(*g, &mut cons, &mut Fresher::default()).unwrap();
            assert_eq!(
                sub,
                Substitution::from(semantic_map! {
                    Tvar(0) => MonoType::Int,
                    Tvar(1) => MonoType::Float,
                })
            );
            // we know everything about tvars, there is no constraint.
            assert_eq!(cons, semantic_map! {});
        } else {
            panic!("the monotypes under examination are not functions");
        }
    }
}
