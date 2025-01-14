//! Nodes in the semantic graph.

// NOTE(affo): At this stage some nodes are a clone of the AST nodes with some type information added.
//  Nevertheless, new node types allow us to decouple this step of compilation from the parsing.
//  This is of paramount importance if we decide to add responsibilities to the semantic analysis and
//  change it independently from the parsing bits.
//  Uncommented node types are a direct port of the AST ones.
#![allow(clippy::match_single_binding)]

extern crate chrono;
extern crate derivative;

use crate::ast;
use crate::semantic::infer;
use crate::semantic::types;
use crate::semantic::{
    env::Environment,
    fresh::Fresher,
    import::Importer,
    infer::{Constraint, Constraints},
    sub::{Substitutable, Substitution},
    types::{
        Array, Dictionary, Function, Kind, MonoType, MonoTypeMap, PolyType, PolyTypeMap, Tvar,
        TvarKinds,
    },
};

use chrono::prelude::DateTime;
use chrono::FixedOffset;
use derivative::Derivative;
use std::fmt::Debug;
use std::vec::Vec;

use derive_more::Display;

/// Result returned from the various 'infer' methods defined in this
/// module. The result of inferring an expression or statment is an
/// updated type environment and a set of type constraints to be solved.
pub type Result = std::result::Result<(Environment, Constraints), Error>;

#[derive(Debug, Display, PartialEq)]
#[allow(missing_docs)]
pub enum Error {
    #[display(fmt = "{}", _0)]
    Inference(infer::Error),
    #[display(fmt = "error {}: undefined builtin identifier {}", _1, _0)]
    UndefinedBuiltin(String, ast::SourceLocation),
    #[display(fmt = "error {}: undefined identifier {}", _1, _0)]
    UndefinedIdentifier(String, ast::SourceLocation),
    #[display(fmt = "error {}: invalid binary operator {}", _1, _0)]
    InvalidBinOp(ast::Operator, ast::SourceLocation),
    #[display(fmt = "error {}: invalid unary operator {}", _1, _0)]
    InvalidUnaryOp(ast::Operator, ast::SourceLocation),
    #[display(fmt = "error {}: invalid import path {}", _1, _0)]
    InvalidImportPath(String, ast::SourceLocation),
    #[display(fmt = "error {}: return not valid in file block", _0)]
    InvalidReturn(ast::SourceLocation),
}

impl From<infer::Error> for Error {
    fn from(err: infer::Error) -> Error {
        Error::Inference(err)
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum Statement {
    Expr(ExprStmt),
    Variable(Box<VariableAssgn>),
    Option(Box<OptionStmt>),
    Return(ReturnStmt),
    Test(Box<TestStmt>),
    TestCase(Box<TestCaseStmt>),
    Builtin(BuiltinStmt),
}

impl Statement {
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            Statement::Expr(stmt) => Statement::Expr(stmt.apply(sub)),
            Statement::Variable(stmt) => Statement::Variable(Box::new(stmt.apply(sub))),
            Statement::Option(stmt) => Statement::Option(Box::new(stmt.apply(sub))),
            Statement::Return(stmt) => Statement::Return(stmt.apply(sub)),
            Statement::Test(stmt) => Statement::Test(Box::new(stmt.apply(sub))),
            Statement::TestCase(stmt) => Statement::TestCase(Box::new(stmt.apply(sub))),
            Statement::Builtin(stmt) => Statement::Builtin(stmt.apply(sub)),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum Assignment {
    Variable(VariableAssgn),
    Member(MemberAssgn),
}

impl Assignment {
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            Assignment::Variable(assign) => Assignment::Variable(assign.apply(sub)),
            Assignment::Member(assign) => Assignment::Member(assign.apply(sub)),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum Expression {
    Identifier(IdentifierExpr),
    Array(Box<ArrayExpr>),
    Dict(Box<DictExpr>),
    Function(Box<FunctionExpr>),
    Logical(Box<LogicalExpr>),
    Object(Box<ObjectExpr>),
    Member(Box<MemberExpr>),
    Index(Box<IndexExpr>),
    Binary(Box<BinaryExpr>),
    Unary(Box<UnaryExpr>),
    Call(Box<CallExpr>),
    Conditional(Box<ConditionalExpr>),
    StringExpr(Box<StringExpr>),

    Integer(IntegerLit),
    Float(FloatLit),
    StringLit(StringLit),
    Duration(DurationLit),
    Uint(UintLit),
    Boolean(BooleanLit),
    DateTime(DateTimeLit),
    Regexp(RegexpLit),
}

impl Expression {
    #[allow(missing_docs)]
    pub fn type_of(&self) -> MonoType {
        match self {
            Expression::Identifier(e) => e.typ.clone(),
            Expression::Array(e) => e.typ.clone(),
            Expression::Dict(e) => e.typ.clone(),
            Expression::Function(e) => e.typ.clone(),
            Expression::Logical(_) => MonoType::Bool,
            Expression::Object(e) => e.typ.clone(),
            Expression::Member(e) => e.typ.clone(),
            Expression::Index(e) => e.typ.clone(),
            Expression::Binary(e) => e.typ.clone(),
            Expression::Unary(e) => e.typ.clone(),
            Expression::Call(e) => e.typ.clone(),
            Expression::Conditional(e) => e.alternate.type_of(),
            Expression::StringExpr(_) => MonoType::String,
            Expression::Integer(_) => MonoType::Int,
            Expression::Float(_) => MonoType::Float,
            Expression::StringLit(_) => MonoType::String,
            Expression::Duration(_) => MonoType::Duration,
            Expression::Uint(_) => MonoType::Uint,
            Expression::Boolean(_) => MonoType::Bool,
            Expression::DateTime(_) => MonoType::Time,
            Expression::Regexp(_) => MonoType::Regexp,
        }
    }
    #[allow(missing_docs)]
    pub fn loc(&self) -> &ast::SourceLocation {
        match self {
            Expression::Identifier(e) => &e.loc,
            Expression::Array(e) => &e.loc,
            Expression::Dict(e) => &e.loc,
            Expression::Function(e) => &e.loc,
            Expression::Logical(e) => &e.loc,
            Expression::Object(e) => &e.loc,
            Expression::Member(e) => &e.loc,
            Expression::Index(e) => &e.loc,
            Expression::Binary(e) => &e.loc,
            Expression::Unary(e) => &e.loc,
            Expression::Call(e) => &e.loc,
            Expression::Conditional(e) => &e.loc,
            Expression::StringExpr(e) => &e.loc,
            Expression::Integer(lit) => &lit.loc,
            Expression::Float(lit) => &lit.loc,
            Expression::StringLit(lit) => &lit.loc,
            Expression::Duration(lit) => &lit.loc,
            Expression::Uint(lit) => &lit.loc,
            Expression::Boolean(lit) => &lit.loc,
            Expression::DateTime(lit) => &lit.loc,
            Expression::Regexp(lit) => &lit.loc,
        }
    }
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        match self {
            Expression::Identifier(e) => e.infer(env, f),
            Expression::Array(e) => e.infer(env, f),
            Expression::Dict(e) => e.infer(env, f),
            Expression::Function(e) => e.infer(env, f),
            Expression::Logical(e) => e.infer(env, f),
            Expression::Object(e) => e.infer(env, f),
            Expression::Member(e) => e.infer(env, f),
            Expression::Index(e) => e.infer(env, f),
            Expression::Binary(e) => e.infer(env, f),
            Expression::Unary(e) => e.infer(env, f),
            Expression::Call(e) => e.infer(env, f),
            Expression::Conditional(e) => e.infer(env, f),
            Expression::StringExpr(e) => e.infer(env, f),
            Expression::Integer(lit) => lit.infer(env),
            Expression::Float(lit) => lit.infer(env),
            Expression::StringLit(lit) => lit.infer(env),
            Expression::Duration(lit) => lit.infer(env),
            Expression::Uint(lit) => lit.infer(env),
            Expression::Boolean(lit) => lit.infer(env),
            Expression::DateTime(lit) => lit.infer(env),
            Expression::Regexp(lit) => lit.infer(env),
        }
    }
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            Expression::Identifier(e) => Expression::Identifier(e.apply(sub)),
            Expression::Array(e) => Expression::Array(Box::new(e.apply(sub))),
            Expression::Dict(e) => Expression::Dict(Box::new(e.apply(sub))),
            Expression::Function(e) => Expression::Function(Box::new(e.apply(sub))),
            Expression::Logical(e) => Expression::Logical(Box::new(e.apply(sub))),
            Expression::Object(e) => Expression::Object(Box::new(e.apply(sub))),
            Expression::Member(e) => Expression::Member(Box::new(e.apply(sub))),
            Expression::Index(e) => Expression::Index(Box::new(e.apply(sub))),
            Expression::Binary(e) => Expression::Binary(Box::new(e.apply(sub))),
            Expression::Unary(e) => Expression::Unary(Box::new(e.apply(sub))),
            Expression::Call(e) => Expression::Call(Box::new(e.apply(sub))),
            Expression::Conditional(e) => Expression::Conditional(Box::new(e.apply(sub))),
            Expression::StringExpr(e) => Expression::StringExpr(Box::new(e.apply(sub))),
            Expression::Integer(lit) => Expression::Integer(lit.apply(sub)),
            Expression::Float(lit) => Expression::Float(lit.apply(sub)),
            Expression::StringLit(lit) => Expression::StringLit(lit.apply(sub)),
            Expression::Duration(lit) => Expression::Duration(lit.apply(sub)),
            Expression::Uint(lit) => Expression::Uint(lit.apply(sub)),
            Expression::Boolean(lit) => Expression::Boolean(lit.apply(sub)),
            Expression::DateTime(lit) => Expression::DateTime(lit.apply(sub)),
            Expression::Regexp(lit) => Expression::Regexp(lit.apply(sub)),
        }
    }
}

/// Infer the types of a Flux package.
#[allow(missing_docs)]
pub fn infer_pkg_types<T>(
    pkg: &mut Package,
    env: Environment,
    f: &mut Fresher,
    importer: &T,
) -> std::result::Result<(Environment, Substitution), Error>
where
    T: Importer,
{
    let (env, cons) = pkg.infer(env, f, importer)?;
    Ok((env, infer::solve(&cons, &mut TvarKinds::new(), f)?))
}

/// Infer the types of a Flux source code file.
#[allow(missing_docs)]
pub fn infer_file<T>(file: &mut File, env: Environment, f: &mut Fresher, importer: &T) -> Result
where
    T: Importer,
{
    file.infer(env, f, importer)
}

#[allow(missing_docs)]
pub fn inject_pkg_types(pkg: Package, sub: &Substitution) -> Package {
    pkg.apply(sub)
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct Package {
    pub loc: ast::SourceLocation,

    pub package: String,
    pub files: Vec<File>,
}

impl Package {
    fn infer<T>(&mut self, env: Environment, f: &mut Fresher, importer: &T) -> Result
    where
        T: Importer,
    {
        self.files
            .iter_mut()
            .try_fold((env, Constraints::empty()), |(env, rest), file| {
                let (env, cons) = file.infer(env, f, importer)?;
                Ok((env, cons + rest))
            })
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.files = self.files.into_iter().map(|file| file.apply(sub)).collect();
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct File {
    pub loc: ast::SourceLocation,

    pub package: Option<PackageClause>,
    pub imports: Vec<ImportDeclaration>,
    pub body: Vec<Statement>,
}

impl File {
    fn infer<T>(&mut self, mut env: Environment, f: &mut Fresher, importer: &T) -> Result
    where
        T: Importer,
    {
        let mut imports = Vec::with_capacity(self.imports.len());

        for dec in &self.imports {
            let path = &dec.path.value;
            let name = dec.import_name();

            imports.push(name);

            match importer.import(path) {
                Some(poly) => env.add(name.to_owned(), poly),
                None => return Err(Error::InvalidImportPath(path.clone(), dec.loc.clone())),
            };
        }

        let (mut env, constraints) =
            self.body
                .iter_mut()
                .try_fold(
                    (env, Constraints::empty()),
                    |(env, rest), node| match node {
                        Statement::Builtin(stmt) => {
                            let env = stmt.infer(env)?;
                            Ok((env, rest))
                        }
                        Statement::Variable(stmt) => {
                            let (env, cons) = stmt.infer(env, f)?;
                            Ok((env, cons + rest))
                        }
                        Statement::Option(stmt) => {
                            let (env, cons) = stmt.infer(env, f)?;
                            Ok((env, cons + rest))
                        }
                        Statement::Expr(stmt) => {
                            let (env, cons) = stmt.infer(env, f)?;
                            Ok((env, cons + rest))
                        }
                        Statement::Test(stmt) => {
                            let (env, cons) = stmt.infer(env, f)?;
                            Ok((env, cons + rest))
                        }
                        Statement::TestCase(stmt) => {
                            let (env, cons) = stmt.infer(env, f)?;
                            Ok((env, cons + rest))
                        }
                        Statement::Return(stmt) => Err(Error::InvalidReturn(stmt.loc.clone())),
                    },
                )?;

        for name in imports {
            env.remove(name);
        }
        Ok((env, constraints))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.body = self.body.into_iter().map(|stmt| stmt.apply(sub)).collect();
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct PackageClause {
    pub loc: ast::SourceLocation,

    pub name: Identifier,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct ImportDeclaration {
    pub loc: ast::SourceLocation,

    pub alias: Option<Identifier>,
    pub path: StringLit,
}

impl ImportDeclaration {
    #[allow(missing_docs)]
    pub fn import_name(&self) -> &str {
        let path = &self.path.value;
        match &self.alias {
            None => path.rsplitn(2, '/').collect::<Vec<&str>>()[0],
            Some(id) => &id.name[..],
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct OptionStmt {
    pub loc: ast::SourceLocation,

    pub assignment: Assignment,
}

impl OptionStmt {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        match &mut self.assignment {
            Assignment::Member(stmt) => {
                let (env, cons) = stmt.init.infer(env, f)?;
                let (env, rest) = stmt.member.infer(env, f)?;

                Ok((
                    env,
                    cons + rest
                        + vec![Constraint::Equal {
                            exp: stmt.member.typ.clone(),
                            act: stmt.init.type_of(),
                            loc: stmt.init.loc().clone(),
                        }]
                        .into(),
                ))
            }
            Assignment::Variable(stmt) => stmt.infer(env, f),
        }
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.assignment = self.assignment.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct BuiltinStmt {
    pub loc: ast::SourceLocation,
    pub id: Identifier,
    pub typ_expr: PolyType,
}

impl BuiltinStmt {
    fn infer(&self, mut env: Environment) -> std::result::Result<Environment, Error> {
        env.add(self.id.name.clone(), self.typ_expr.clone());
        Ok(env)
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct TestStmt {
    pub loc: ast::SourceLocation,

    pub assignment: VariableAssgn,
}

impl TestStmt {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        self.assignment.infer(env, f)
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.assignment = self.assignment.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct TestCaseStmt {
    pub loc: ast::SourceLocation,
    pub id: Identifier,
    pub block: Block,
}

impl TestCaseStmt {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        self.block.infer(env, f)
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.block = self.block.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct ExprStmt {
    pub loc: ast::SourceLocation,

    pub expression: Expression,
}

impl ExprStmt {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let (env, cons) = self.expression.infer(env, f)?;
        let sub = infer::solve(&cons, &mut TvarKinds::new(), f)?;
        Ok((env.apply(&sub), cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.expression = self.expression.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct ReturnStmt {
    pub loc: ast::SourceLocation,

    pub argument: Expression,
}

impl ReturnStmt {
    #[allow(dead_code)]
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        self.argument.infer(env, f)
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.argument = self.argument.apply(sub);
        self
    }
}

#[derive(Debug, Derivative, Clone)]
#[derivative(PartialEq)]
#[allow(missing_docs)]
pub struct VariableAssgn {
    #[derivative(PartialEq = "ignore")]
    vars: Vec<Tvar>,

    #[derivative(PartialEq = "ignore")]
    cons: TvarKinds,

    pub loc: ast::SourceLocation,

    pub id: Identifier,
    pub init: Expression,
}

impl VariableAssgn {
    #[allow(missing_docs)]
    pub fn new(id: Identifier, init: Expression, loc: ast::SourceLocation) -> VariableAssgn {
        VariableAssgn {
            vars: Vec::new(),
            cons: TvarKinds::new(),
            loc,
            id,
            init,
        }
    }
    #[allow(missing_docs)]
    pub fn poly_type_of(&self) -> PolyType {
        PolyType {
            vars: self.vars.clone(),
            cons: self.cons.clone(),
            expr: self.init.type_of(),
        }
    }
    // Polymorphic generalization, necessary for let-polymorphism, is
    // implemented here.
    //
    // In particular, for every variable assignment we infer the type of
    // its corresponding expression. We then generalize that type by
    // quantifying over all of its free type variables. Finally we bind
    // the variable to its newly generalized type in the type environment
    // before inferring the rest of the program.
    //
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let (env, constraints) = self.init.infer(env, f)?;

        let mut kinds = TvarKinds::new();
        let sub = infer::solve(&constraints, &mut kinds, f)?;

        // Apply substitution to the type environment
        let mut env = env.apply(&sub);

        let t = self.init.type_of().apply(&sub);
        let p = infer::generalize(&env, &kinds, t);

        // Update variable assignment nodes with the free vars
        // and kind constraints obtained from generalization.
        //
        // Note these variables are fixed after generalization
        // and so it is safe to update these nodes in place.
        self.vars = p.vars.clone();
        self.cons = p.cons.clone();

        // Update the type environment
        env.add(String::from(&self.id.name), p);
        Ok((env, constraints))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.init = self.init.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct MemberAssgn {
    pub loc: ast::SourceLocation,

    pub member: MemberExpr,
    pub init: Expression,
}

impl MemberAssgn {
    fn apply(mut self, sub: &Substitution) -> Self {
        self.member = self.member.apply(sub);
        self.init = self.init.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct StringExpr {
    pub loc: ast::SourceLocation,
    pub parts: Vec<StringExprPart>,
}

impl StringExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let mut env = env;
        let mut constraints = Vec::new();
        for p in &mut self.parts {
            if let StringExprPart::Interpolated(ref mut ip) = p {
                let (e, cons) = ip.expression.infer(env, f)?;
                constraints.append(&mut Vec::from(cons));
                constraints.push(Constraint::Kind {
                    exp: Kind::Stringable,
                    act: ip.expression.type_of(),
                    loc: ip.expression.loc().clone(),
                });
                env = e
            }
        }
        Ok((env, Constraints::from(constraints)))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.parts = self.parts.into_iter().map(|part| part.apply(sub)).collect();
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum StringExprPart {
    Text(TextPart),
    Interpolated(InterpolatedPart),
}

impl StringExprPart {
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            StringExprPart::Interpolated(part) => StringExprPart::Interpolated(part.apply(sub)),
            StringExprPart::Text(_) => self,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct TextPart {
    pub loc: ast::SourceLocation,

    pub value: String,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct InterpolatedPart {
    pub loc: ast::SourceLocation,

    pub expression: Expression,
}

impl InterpolatedPart {
    fn apply(mut self, sub: &Substitution) -> Self {
        self.expression = self.expression.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct ArrayExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub elements: Vec<Expression>,
}

impl ArrayExpr {
    fn infer(&mut self, mut env: Environment, f: &mut Fresher) -> Result {
        let mut cons = Vec::new();
        let elt = MonoType::Var(f.fresh());
        for el in &mut self.elements {
            let (e, c) = el.infer(env, f)?;
            cons.append(&mut c.into());
            cons.push(Constraint::Equal {
                exp: elt.clone(),
                act: el.type_of(),
                loc: el.loc().clone(),
            });
            env = e;
        }
        let at = MonoType::Arr(Box::new(Array(elt)));
        cons.push(Constraint::Equal {
            exp: at,
            act: self.typ.clone(),
            loc: self.loc.clone(),
        });
        Ok((env, cons.into()))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.elements = self
            .elements
            .into_iter()
            .map(|element| element.apply(sub))
            .collect();
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct DictExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,
    pub elements: Vec<(Expression, Expression)>,
}

impl DictExpr {
    fn infer(&mut self, mut env: Environment, f: &mut Fresher) -> Result {
        let mut cons = Constraints::empty();

        let key = MonoType::Var(f.fresh());
        let val = MonoType::Var(f.fresh());

        for (k, v) in &mut self.elements {
            let (e, c0) = k.infer(env, f)?;
            let (e, c1) = v.infer(e, f)?;

            let kt = k.type_of();
            let vt = v.type_of();

            let kc = Constraint::Equal {
                exp: key.clone(),
                act: kt,
                loc: k.loc().clone(),
            };
            let vc = Constraint::Equal {
                exp: val.clone(),
                act: vt,
                loc: v.loc().clone(),
            };

            cons = cons + c0 + c1 + vec![kc, vc].into();
            env = e;
        }

        let ty = MonoType::Dict(Box::new(Dictionary {
            key: key.clone(),
            val,
        }));

        let eq = Constraint::Equal {
            exp: ty,
            act: self.typ.clone(),
            loc: self.loc.clone(),
        };
        let tc = Constraint::Kind {
            exp: Kind::Comparable,
            act: key,
            loc: self.loc.clone(),
        };

        Ok((env, cons + vec![eq, tc].into()))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.elements = self
            .elements
            .into_iter()
            .map(|(key, val)| (key.apply(sub), val.apply(sub)))
            .collect();
        self
    }
}

/// Represents the definition of a function.
#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct FunctionExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub params: Vec<FunctionParameter>,
    pub body: Block,
}

impl FunctionExpr {
    fn infer(&mut self, mut env: Environment, f: &mut Fresher) -> Result {
        let mut cons = Constraints::empty();
        let mut pipe = None;
        let mut req = MonoTypeMap::new();
        let mut opt = MonoTypeMap::new();
        // This params will build the nested env when inferring the function body.
        let mut params = PolyTypeMap::new();
        for param in &mut self.params {
            match param.default {
                Some(ref mut e) => {
                    let (nenv, ncons) = e.infer(env, f)?;
                    cons = cons + ncons;
                    let id = param.key.name.clone();
                    // We are here: `f = (a=1) => {...}`.
                    // So, this PolyType is actually a MonoType, whose type
                    // is the one of the default value ("1" in "a=1").
                    let typ = PolyType {
                        vars: Vec::new(),
                        cons: TvarKinds::new(),
                        expr: e.type_of(),
                    };
                    params.insert(id.clone(), typ);
                    opt.insert(id, e.type_of());
                    env = nenv;
                }
                None => {
                    // We are here: `f = (a) => {...}`.
                    // So, we do not know the type of "a". Let's use a fresh TVar.
                    let id = param.key.name.clone();
                    let ftvar = f.fresh();
                    let typ = PolyType {
                        vars: Vec::new(),
                        cons: TvarKinds::new(),
                        expr: MonoType::Var(ftvar),
                    };
                    params.insert(id.clone(), typ.clone());
                    // Piped arguments cannot have a default value.
                    // So check if this is a piped argument.
                    if param.is_pipe {
                        pipe = Some(types::Property {
                            k: id,
                            v: MonoType::Var(ftvar),
                        });
                    } else {
                        req.insert(id, MonoType::Var(ftvar));
                    }
                }
            };
        }
        // Add the parameters to some nested environment.
        let mut nenv = Environment::new(env);
        for (id, param) in params.into_iter() {
            nenv.add(id, param);
        }
        // And use it to infer the body.
        let (nenv, bcons) = self.body.infer(nenv, f)?;
        // Now pop the nested environment, we don't need it anymore.
        let env = nenv.pop();
        let retn = self.body.type_of();
        let func = MonoType::Fun(Box::new(Function {
            req,
            opt,
            pipe,
            retn,
        }));
        cons = cons + bcons;
        cons.add(Constraint::Equal {
            exp: self.typ.clone(),
            act: func,
            loc: self.loc.clone(),
        });
        Ok((env, cons))
    }
    #[allow(missing_docs)]
    pub fn pipe(&self) -> Option<&FunctionParameter> {
        for p in &self.params {
            if p.is_pipe {
                return Some(p);
            }
        }
        None
    }
    #[allow(missing_docs)]
    pub fn defaults(&self) -> Vec<&FunctionParameter> {
        let mut ds = Vec::new();
        for p in &self.params {
            if p.default.is_some() {
                ds.push(p);
            };
        }
        ds
    }
    #[allow(missing_docs)]
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.params = self
            .params
            .into_iter()
            .map(|param| param.apply(sub))
            .collect();
        self.body = self.body.apply(sub);
        self
    }
}

/// Represents a function block and is equivalent to a let-expression
/// in other functional languages.
///
/// Functions must evaluate to a value in Flux. In other words, a function
/// must always have a return value. This means a function block is by
/// definition an expression.
///
/// A function block is an expression that evaluates to the argument of
/// its terminating ReturnStmt.
#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum Block {
    Variable(Box<VariableAssgn>, Box<Block>),
    Expr(ExprStmt, Box<Block>),
    Return(ReturnStmt),
}

impl Block {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        match self {
            Block::Variable(stmt, block) => {
                let (env, cons) = stmt.infer(env, f)?;
                let (env, rest) = block.infer(env, f)?;

                Ok((env, cons + rest))
            }
            Block::Expr(stmt, block) => {
                let (env, cons) = stmt.infer(env, f)?;
                let (env, rest) = block.infer(env, f)?;

                Ok((env, cons + rest))
            }
            Block::Return(e) => e.infer(env, f),
        }
    }
    #[allow(missing_docs)]
    pub fn loc(&self) -> &ast::SourceLocation {
        match self {
            Block::Variable(assign, _) => &assign.loc,
            Block::Expr(es, _) => es.expression.loc(),
            Block::Return(ret) => &ret.loc,
        }
    }
    #[allow(missing_docs)]
    pub fn type_of(&self) -> MonoType {
        let mut n = self;
        loop {
            n = match n {
                Block::Variable(_, b) => b.as_ref(),
                Block::Expr(_, b) => b.as_ref(),
                Block::Return(r) => return r.argument.type_of(),
            }
        }
    }
    fn apply(self, sub: &Substitution) -> Self {
        match self {
            Block::Variable(assign, next) => {
                Block::Variable(Box::new(assign.apply(sub)), Box::new(next.apply(sub)))
            }
            Block::Expr(es, next) => Block::Expr(es.apply(sub), Box::new(next.apply(sub))),
            Block::Return(e) => Block::Return(e.apply(sub)),
        }
    }
}

/// FunctionParameter represents a function parameter.
#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct FunctionParameter {
    pub loc: ast::SourceLocation,

    pub is_pipe: bool,
    pub key: Identifier,
    pub default: Option<Expression>,
}

impl FunctionParameter {
    fn apply(mut self, sub: &Substitution) -> Self {
        match self.default {
            Some(e) => {
                self.default = Some(e.apply(sub));
                self
            }
            None => self,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct BinaryExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub operator: ast::Operator,
    pub left: Expression,
    pub right: Expression,
}

impl BinaryExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        // Compute the left and right constraints.
        // Do this first so that we can return an error if one occurs.
        let (env, lcons) = self.left.infer(env, f)?;
        let (env, rcons) = self.right.infer(env, f)?;

        let cons = match self.operator {
            // The following operators require both sides to be equal.
            ast::Operator::AdditionOperator => Constraints::from(vec![
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.typ.clone(),
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.typ.clone(),
                    exp: Kind::Addable,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::SubtractionOperator => Constraints::from(vec![
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.typ.clone(),
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.typ.clone(),
                    exp: Kind::Subtractable,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::MultiplicationOperator => Constraints::from(vec![
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.typ.clone(),
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.typ.clone(),
                    exp: Kind::Divisible,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::DivisionOperator => Constraints::from(vec![
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.typ.clone(),
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.typ.clone(),
                    exp: Kind::Divisible,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::PowerOperator => Constraints::from(vec![
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.typ.clone(),
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.typ.clone(),
                    exp: Kind::Divisible,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::ModuloOperator => Constraints::from(vec![
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.left.type_of(),
                    act: self.typ.clone(),
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.typ.clone(),
                    exp: Kind::Divisible,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::GreaterThanOperator => Constraints::from(vec![
                // https://github.com/influxdata/flux/issues/2393
                // Constraint::Equal{self.left.type_of(), self.right.type_of()),
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Comparable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Comparable,
                    loc: self.right.loc().clone(),
                },
            ]),
            ast::Operator::LessThanOperator => Constraints::from(vec![
                // https://github.com/influxdata/flux/issues/2393
                // Constraint::Equal{self.left.type_of(), self.right.type_of()),
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Comparable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Comparable,
                    loc: self.right.loc().clone(),
                },
            ]),
            ast::Operator::EqualOperator => Constraints::from(vec![
                // https://github.com/influxdata/flux/issues/2393
                // Constraint::Equal{self.left.type_of(), self.right.type_of()),
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Equatable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Equatable,
                    loc: self.right.loc().clone(),
                },
            ]),
            ast::Operator::NotEqualOperator => Constraints::from(vec![
                // https://github.com/influxdata/flux/issues/2393
                // Constraint::Equal{self.left.type_of(), self.right.type_of()),
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Equatable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Equatable,
                    loc: self.right.loc().clone(),
                },
            ]),
            ast::Operator::GreaterThanEqualOperator => Constraints::from(vec![
                // https://github.com/influxdata/flux/issues/2393
                // Constraint::Equal{self.left.type_of(), self.right.type_of()),
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Equatable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Comparable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Equatable,
                    loc: self.right.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Comparable,
                    loc: self.right.loc().clone(),
                },
            ]),
            ast::Operator::LessThanEqualOperator => Constraints::from(vec![
                // https://github.com/influxdata/flux/issues/2393
                // Constraint::Equal{self.left.type_of(), self.right.type_of()),
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Equatable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.left.type_of(),
                    exp: Kind::Comparable,
                    loc: self.left.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Equatable,
                    loc: self.right.loc().clone(),
                },
                Constraint::Kind {
                    act: self.right.type_of(),
                    exp: Kind::Comparable,
                    loc: self.right.loc().clone(),
                },
            ]),
            // Regular expression operators.
            ast::Operator::RegexpMatchOperator => Constraints::from(vec![
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Equal {
                    act: self.left.type_of(),
                    exp: MonoType::String,
                    loc: self.left.loc().clone(),
                },
                Constraint::Equal {
                    act: self.right.type_of(),
                    exp: MonoType::Regexp,
                    loc: self.right.loc().clone(),
                },
            ]),
            ast::Operator::NotRegexpMatchOperator => Constraints::from(vec![
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
                Constraint::Equal {
                    act: self.left.type_of(),
                    exp: MonoType::String,
                    loc: self.left.loc().clone(),
                },
                Constraint::Equal {
                    act: self.right.type_of(),
                    exp: MonoType::Regexp,
                    loc: self.right.loc().clone(),
                },
            ]),
            _ => return Err(Error::InvalidBinOp(self.operator.clone(), self.loc.clone())),
        };

        // Otherwise, add the constraints together and return them.
        Ok((env, lcons + rcons + cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.left = self.left.apply(sub);
        self.right = self.right.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct CallExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub callee: Expression,
    pub arguments: Vec<Property>,
    pub pipe: Option<Expression>,
}

impl CallExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        // First, recursively infer every type of the children of this call expression,
        // update the environment and the constraints, and use the inferred types to
        // build the fields of the type for this call expression.
        let (mut env, mut cons) = self.callee.infer(env, f)?;
        let mut req = MonoTypeMap::new();
        let mut pipe = None;
        for Property {
            key: ref mut id,
            value: ref mut expr,
            ..
        } in &mut self.arguments
        {
            let (nenv, ncons) = expr.infer(env, f)?;
            cons = cons + ncons;
            env = nenv;
            // Every argument is required in a function call.
            req.insert(id.name.clone(), expr.type_of());
        }
        if let Some(ref mut p) = &mut self.pipe {
            let (nenv, ncons) = p.infer(env, f)?;
            cons = cons + ncons;
            env = nenv;
            pipe = Some(types::Property {
                k: "<-".to_string(),
                v: p.type_of(),
            });
        }
        // Constrain the callee to be a Function.
        cons.add(Constraint::Equal {
            exp: self.callee.type_of(),
            act: MonoType::Fun(Box::new(Function {
                opt: MonoTypeMap::new(),
                req,
                pipe,
                // The return type of a function call is the type of the call itself.
                // Remind that, when two functions are unified, their return types are unified too.
                // As an example take:
                //   f = (a) => a + 1
                //   f(a: 0)
                // The return type of `f` is `int`.
                // The return type of `f(a: 0)` is `t0` (a fresh type variable).
                // Upon unification a substitution "t0 => int" is created, so that the compiler
                // can infer that, for instance, `f(a: 0) + 1` is legal.
                retn: self.typ.clone(),
            })),
            loc: self.loc.clone(),
        });
        Ok((env, cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.callee = self.callee.apply(sub);
        self.arguments = self
            .arguments
            .into_iter()
            .map(|arg| arg.apply(sub))
            .collect();
        match self.pipe {
            Some(e) => {
                self.pipe = Some(e.apply(sub));
                self
            }
            None => self,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct ConditionalExpr {
    pub loc: ast::SourceLocation,
    pub test: Expression,
    pub consequent: Expression,
    pub alternate: Expression,
}

impl ConditionalExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let (env, tcons) = self.test.infer(env, f)?;
        let (env, ccons) = self.consequent.infer(env, f)?;
        let (env, acons) = self.alternate.infer(env, f)?;
        let cons = tcons
            + ccons
            + acons
            + Constraints::from(vec![
                Constraint::Equal {
                    exp: MonoType::Bool,
                    act: self.test.type_of(),
                    loc: self.test.loc().clone(),
                },
                Constraint::Equal {
                    exp: self.consequent.type_of(),
                    act: self.alternate.type_of(),
                    loc: self.alternate.loc().clone(),
                },
            ]);
        Ok((env, cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.test = self.test.apply(sub);
        self.consequent = self.consequent.apply(sub);
        self.alternate = self.alternate.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct LogicalExpr {
    pub loc: ast::SourceLocation,
    pub operator: ast::LogicalOperator,
    pub left: Expression,
    pub right: Expression,
}

impl LogicalExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let (env, lcons) = self.left.infer(env, f)?;
        let (env, rcons) = self.right.infer(env, f)?;
        let cons = lcons
            + rcons
            + Constraints::from(vec![
                Constraint::Equal {
                    exp: MonoType::Bool,
                    act: self.left.type_of(),
                    loc: self.left.loc().clone(),
                },
                Constraint::Equal {
                    exp: MonoType::Bool,
                    act: self.right.type_of(),
                    loc: self.right.loc().clone(),
                },
            ]);
        Ok((env, cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.left = self.left.apply(sub);
        self.right = self.right.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct MemberExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub object: Expression,
    pub property: String,
}

impl MemberExpr {
    // A member expression such as `r.a` produces the constraint:
    //
    //     type_of(r) = {a: type_of(r.a) | 'r}
    //
    // where 'r is a fresh type variable.
    //
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let head = types::Property {
            k: self.property.to_owned(),
            v: self.typ.to_owned(),
        };
        let tail = MonoType::Var(f.fresh());

        let r = MonoType::from(types::Record::Extension { head, tail });
        let t = self.object.type_of();

        let (env, cons) = self.object.infer(env, f)?;
        Ok((
            env,
            cons + vec![Constraint::Equal {
                exp: r,
                act: t,
                loc: self.object.loc().clone(),
            }]
            .into(),
        ))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.object = self.object.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct IndexExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub array: Expression,
    pub index: Expression,
}

impl IndexExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let (env, acons) = self.array.infer(env, f)?;
        let (env, icons) = self.index.infer(env, f)?;
        let cons = acons
            + icons
            + Constraints::from(vec![
                Constraint::Equal {
                    act: self.index.type_of(),
                    exp: MonoType::Int,
                    loc: self.index.loc().clone(),
                },
                Constraint::Equal {
                    act: self.array.type_of(),
                    exp: MonoType::Arr(Box::new(Array(self.typ.clone()))),
                    loc: self.array.loc().clone(),
                },
            ]);
        Ok((env, cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.array = self.array.apply(sub);
        self.index = self.index.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct ObjectExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub with: Option<IdentifierExpr>,
    pub properties: Vec<Property>,
}

impl ObjectExpr {
    fn infer(&mut self, mut env: Environment, f: &mut Fresher) -> Result {
        // If record extension, infer constraints for base
        let (mut r, mut cons) = match &mut self.with {
            Some(expr) => {
                let (e, cons) = expr.infer(env, f)?;
                env = e;
                (expr.typ.to_owned(), cons)
            }
            None => (
                MonoType::Record(Box::new(types::Record::Empty)),
                Constraints::empty(),
            ),
        };
        // Infer constraints for properties
        for prop in self.properties.iter_mut().rev() {
            let (e, rest) = prop.value.infer(env, f)?;
            env = e;
            cons = cons + rest;
            r = MonoType::Record(Box::new(types::Record::Extension {
                head: types::Property {
                    k: prop.key.name.to_owned(),
                    v: prop.value.type_of(),
                },
                tail: r,
            }));
        }
        Ok((
            env,
            cons + vec![Constraint::Equal {
                exp: self.typ.to_owned(),
                act: r,
                loc: self.loc.clone(),
            }]
            .into(),
        ))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        if let Some(e) = self.with {
            self.with = Some(e.apply(sub));
        }
        self.properties = self
            .properties
            .into_iter()
            .map(|prop| prop.apply(sub))
            .collect();
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct UnaryExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub operator: ast::Operator,
    pub argument: Expression,
}

impl UnaryExpr {
    fn infer(&mut self, env: Environment, f: &mut Fresher) -> Result {
        let (env, acons) = self.argument.infer(env, f)?;
        let cons = match self.operator {
            ast::Operator::NotOperator => Constraints::from(vec![
                Constraint::Equal {
                    act: self.argument.type_of(),
                    exp: MonoType::Bool,
                    loc: self.argument.loc().clone(),
                },
                Constraint::Equal {
                    act: self.typ.clone(),
                    exp: MonoType::Bool,
                    loc: self.loc.clone(),
                },
            ]),
            ast::Operator::ExistsOperator => Constraints::from(Constraint::Equal {
                act: self.typ.clone(),
                exp: MonoType::Bool,
                loc: self.loc.clone(),
            }),
            ast::Operator::AdditionOperator | ast::Operator::SubtractionOperator => {
                Constraints::from(vec![
                    Constraint::Equal {
                        act: self.argument.type_of(),
                        exp: self.typ.clone(),
                        loc: self.loc.clone(),
                    },
                    Constraint::Kind {
                        act: self.argument.type_of(),
                        exp: Kind::Negatable,
                        loc: self.argument.loc().clone(),
                    },
                ])
            }
            _ => {
                return Err(Error::InvalidUnaryOp(
                    self.operator.clone(),
                    self.loc.clone(),
                ))
            }
        };
        Ok((env, acons + cons))
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self.argument = self.argument.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct Property {
    pub loc: ast::SourceLocation,

    pub key: Identifier,
    pub value: Expression,
}

impl Property {
    fn apply(mut self, sub: &Substitution) -> Self {
        self.value = self.value.apply(sub);
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct IdentifierExpr {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub typ: MonoType,

    pub name: String,
}

impl IdentifierExpr {
    fn infer(&self, env: Environment, f: &mut Fresher) -> Result {
        match env.lookup(&self.name) {
            Some(poly) => {
                let (t, cons) = infer::instantiate(poly.clone(), f, self.loc.clone());
                Ok((
                    env,
                    cons + Constraints::from(vec![Constraint::Equal {
                        act: t,
                        exp: self.typ.clone(),
                        loc: self.loc.clone(),
                    }]),
                ))
            }
            None => Err(Error::UndefinedIdentifier(
                self.name.to_string(),
                self.loc.clone(),
            )),
        }
    }
    fn apply(mut self, sub: &Substitution) -> Self {
        self.typ = self.typ.apply(sub);
        self
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct Identifier {
    pub loc: ast::SourceLocation,

    pub name: String,
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct BooleanLit {
    pub loc: ast::SourceLocation,
    pub value: bool,
}

impl BooleanLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct IntegerLit {
    pub loc: ast::SourceLocation,
    pub value: i64,
}

impl IntegerLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct FloatLit {
    pub loc: ast::SourceLocation,
    pub value: f64,
}

impl FloatLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct RegexpLit {
    pub loc: ast::SourceLocation,
    pub value: String,
}

impl RegexpLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct StringLit {
    pub loc: ast::SourceLocation,
    pub value: String,
}

impl StringLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct UintLit {
    pub loc: ast::SourceLocation,
    pub value: u64,
}

impl UintLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct DateTimeLit {
    pub loc: ast::SourceLocation,
    pub value: DateTime<FixedOffset>,
}

impl DateTimeLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

/// A struct that keeps track of time in months and nanoseconds.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename = "Duration")]
pub struct Duration {
    /// Must be a positive value.
    pub months: i64,
    /// Must be a positive value.
    pub nanoseconds: i64,
    /// Indicates whether the magnitude of durations converted from the AST have a positive or
    /// negative value. This field is `true` when magnitudes are negative.
    pub negative: bool,
}

/// The atomic unit from which all duration literals are composed.
///
/// A `DurationLit` is a pair consisting of a length of time and the unit of time measured.
#[derive(Derivative)]
#[derivative(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub struct DurationLit {
    pub loc: ast::SourceLocation,
    #[derivative(PartialEq = "ignore")]
    pub value: Duration,
}

impl DurationLit {
    fn infer(&self, env: Environment) -> Result {
        Ok((env, Constraints::empty()))
    }
    fn apply(self, _: &Substitution) -> Self {
        self
    }
}

// The following durations have nanosecond base units
const NANOS: i64 = 1;
const MICROS: i64 = NANOS * 1000;
const MILLIS: i64 = MICROS * 1000;
const SECONDS: i64 = MILLIS * 1000;
const MINUTES: i64 = SECONDS * 60;
const HOURS: i64 = MINUTES * 60;
const DAYS: i64 = HOURS * 24;
const WEEKS: i64 = DAYS * 7;

// The following durations have month base units
const MONTHS: i64 = 1;
const YEARS: i64 = MONTHS * 12;

/// Convert an [`ast::Duration`] node to its semantic counterpart [`Duration`].
///
/// Returns a `Result` type with a possible error message.
pub fn convert_duration(ast_dur: &[ast::Duration]) -> std::result::Result<Duration, String> {
    if ast_dur.is_empty() {
        return Err(String::from(
            "AST duration vector must contain at least one duration value",
        ));
    };

    let negative = ast_dur[0].magnitude.is_negative();

    let (nanoseconds, months) = ast_dur.iter().try_fold((0i64, 0i64), |acc, d| {
        if (d.magnitude.is_negative() && !negative) || (!d.magnitude.is_negative() && negative) {
            return Err("all values in AST duration vector must have the same sign");
        }

        match d.unit.as_str() {
            "y" => Ok((acc.0, acc.1 + d.magnitude * YEARS)),
            "mo" => Ok((acc.0, acc.1 + d.magnitude * MONTHS)),
            "w" => Ok((acc.0 + d.magnitude * WEEKS, acc.1)),
            "d" => Ok((acc.0 + d.magnitude * DAYS, acc.1)),
            "h" => Ok((acc.0 + d.magnitude * HOURS, acc.1)),
            "m" => Ok((acc.0 + d.magnitude * MINUTES, acc.1)),
            "s" => Ok((acc.0 + d.magnitude * SECONDS, acc.1)),
            "ms" => Ok((acc.0 + d.magnitude * MILLIS, acc.1)),
            "us" | "µs" => Ok((acc.0 + d.magnitude * MICROS, acc.1)),
            "ns" => Ok((acc.0 + d.magnitude * NANOS, acc.1)),
            _ => Err("unrecognized magnitude for duration"),
        }
    })?;

    let nanoseconds = nanoseconds.abs();
    let months = months.abs();

    Ok(Duration {
        months,
        nanoseconds,
        negative,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::semantic::types::{MonoType, Tvar};
    use crate::semantic::walk::{walk, Node};
    use std::rc::Rc;

    #[test]
    fn duration_conversion_ok() {
        let t = vec![
            ast::Duration {
                magnitude: 1,
                unit: "y".to_string(),
            },
            ast::Duration {
                magnitude: 2,
                unit: "mo".to_string(),
            },
            ast::Duration {
                magnitude: 3,
                unit: "w".to_string(),
            },
            ast::Duration {
                magnitude: 4,
                unit: "m".to_string(),
            },
            ast::Duration {
                magnitude: 5,
                unit: "ns".to_string(),
            },
        ];
        let expect_nano = 3 * WEEKS + 4 * MINUTES + 5 * NANOS;
        let expect_months = 1 * YEARS + 2 * MONTHS;

        let got = convert_duration(&t).unwrap();
        assert_eq!(expect_nano, got.nanoseconds);
        assert_eq!(expect_months, got.months);
        assert_eq!(false, got.negative);
    }

    #[test]
    fn duration_conversion_same_magnitude_twice() {
        let t = vec![
            ast::Duration {
                magnitude: 1,
                unit: "y".to_string(),
            },
            ast::Duration {
                magnitude: 2,
                unit: "mo".to_string(),
            },
            ast::Duration {
                magnitude: 3,
                unit: "y".to_string(),
            },
        ];
        let expect_nano = 0;
        let expect_months = 4 * YEARS + 2 * MONTHS;

        let got = convert_duration(&t).unwrap();
        assert_eq!(expect_nano, got.nanoseconds);
        assert_eq!(expect_months, got.months);
        assert_eq!(false, got.negative);
    }

    #[test]
    fn duration_conversion_negative_ok() {
        let t = vec![
            ast::Duration {
                magnitude: -1,
                unit: "y".to_string(),
            },
            ast::Duration {
                magnitude: -2,
                unit: "mo".to_string(),
            },
            ast::Duration {
                magnitude: -3,
                unit: "w".to_string(),
            },
        ];
        let expect_months = (-1 * YEARS + (-2 * MONTHS)).abs();
        let expect_nano = (-3 * WEEKS).abs();

        let got = convert_duration(&t).unwrap();
        assert_eq!(expect_nano, got.nanoseconds);
        assert_eq!(expect_months, got.months);
        assert_eq!(true, got.negative);
    }

    #[test]
    fn duration_conversion_unit_error() {
        let t = vec![
            ast::Duration {
                magnitude: -1,
                unit: "y".to_string(),
            },
            ast::Duration {
                magnitude: -2,
                unit: "--idk--".to_string(),
            },
            ast::Duration {
                magnitude: -3,
                unit: "w".to_string(),
            },
        ];
        let exp = "unrecognized magnitude for duration";
        let got = convert_duration(&t).err().expect("should be an error");
        assert_eq!(exp, got.to_string());
    }

    #[test]
    fn duration_conversion_different_signs_error() {
        let t = vec![
            ast::Duration {
                magnitude: -1,
                unit: "y".to_string(),
            },
            ast::Duration {
                magnitude: 2,
                unit: "ns".to_string(),
            },
            ast::Duration {
                magnitude: -3,
                unit: "w".to_string(),
            },
        ];
        let exp = "all values in AST duration vector must have the same sign";
        let got = convert_duration(&t).err().expect("should be an error");
        assert_eq!(exp, got.to_string());
    }

    #[test]
    fn duration_conversion_empty_error() {
        let t = Vec::new();
        let exp = "AST duration vector must contain at least one duration value";
        let got = convert_duration(&t).err().expect("should be an error");
        assert_eq!(exp, got.to_string());
    }

    #[test]
    fn test_inject_types() {
        let b = ast::BaseNode::default();
        let pkg = Package {
            loc: b.location.clone(),
            package: "main".to_string(),
            files: vec![File {
                loc: b.location.clone(),
                package: None,
                imports: Vec::new(),
                body: vec![
                    Statement::Variable(Box::new(VariableAssgn::new(
                        Identifier {
                            loc: b.location.clone(),
                            name: "f".to_string(),
                        },
                        Expression::Function(Box::new(FunctionExpr {
                            loc: b.location.clone(),
                            typ: MonoType::Var(Tvar(0)),
                            params: vec![
                                FunctionParameter {
                                    loc: b.location.clone(),
                                    is_pipe: true,
                                    key: Identifier {
                                        loc: b.location.clone(),
                                        name: "piped".to_string(),
                                    },
                                    default: None,
                                },
                                FunctionParameter {
                                    loc: b.location.clone(),
                                    is_pipe: false,
                                    key: Identifier {
                                        loc: b.location.clone(),
                                        name: "a".to_string(),
                                    },
                                    default: None,
                                },
                            ],
                            body: Block::Return(ReturnStmt {
                                loc: b.location.clone(),
                                argument: Expression::Binary(Box::new(BinaryExpr {
                                    loc: b.location.clone(),
                                    typ: MonoType::Var(Tvar(1)),
                                    operator: ast::Operator::AdditionOperator,
                                    left: Expression::Identifier(IdentifierExpr {
                                        loc: b.location.clone(),
                                        typ: MonoType::Var(Tvar(2)),
                                        name: "a".to_string(),
                                    }),
                                    right: Expression::Identifier(IdentifierExpr {
                                        loc: b.location.clone(),
                                        typ: MonoType::Var(Tvar(3)),
                                        name: "piped".to_string(),
                                    }),
                                })),
                            }),
                        })),
                        b.location.clone(),
                    ))),
                    Statement::Expr(ExprStmt {
                        loc: b.location.clone(),
                        expression: Expression::Call(Box::new(CallExpr {
                            loc: b.location.clone(),
                            typ: MonoType::Var(Tvar(4)),
                            pipe: Some(Expression::Integer(IntegerLit {
                                loc: b.location.clone(),
                                value: 3,
                            })),
                            callee: Expression::Identifier(IdentifierExpr {
                                loc: b.location.clone(),
                                typ: MonoType::Var(Tvar(6)),
                                name: "f".to_string(),
                            }),
                            arguments: vec![Property {
                                loc: b.location.clone(),
                                key: Identifier {
                                    loc: b.location.clone(),
                                    name: "a".to_string(),
                                },
                                value: Expression::Integer(IntegerLit {
                                    loc: b.location.clone(),
                                    value: 2,
                                }),
                            }],
                        })),
                    }),
                ],
            }],
        };
        let sub: Substitution = semantic_map! {
            Tvar(0) => MonoType::Int,
            Tvar(1) => MonoType::Int,
            Tvar(2) => MonoType::Int,
            Tvar(3) => MonoType::Int,
            Tvar(4) => MonoType::Int,
            Tvar(5) => MonoType::Int,
            Tvar(6) => MonoType::Int,
            Tvar(7) => MonoType::Int,
        }
        .into();
        let pkg = inject_pkg_types(pkg, &sub);
        let mut no_types_checked = 0;
        walk(
            &mut |node: Rc<Node>| {
                let typ = node.type_of();
                if let Some(typ) = typ {
                    assert_eq!(typ, MonoType::Int);
                    no_types_checked += 1;
                }
            },
            Rc::new(Node::Package(&pkg)),
        );
        assert_eq!(no_types_checked, 8);
    }
}
