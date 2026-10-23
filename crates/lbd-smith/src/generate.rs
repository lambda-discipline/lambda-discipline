//! Type-directed generation of well-typed programs.
//!
//! To build an expression of type `t`, the generator picks one of the forms
//! that can have that type (a variable of type `t`, a literal, a builtin
//! whose result is `t`, `let`, `if`, `match`, an application, ...) and
//! generates its parts at the types the form requires. A depth budget makes
//! it terminate; at depth zero it falls back to the smallest value of `t`.
//! The technique follows Pałka et al., "Testing an optimising compiler by
//! generating random lambda terms" (2011).

use arbitrary::{Result, Unstructured};

use crate::model::{DataDecl, Def, Expr, Pat, Program, Ty};

/// Limits on the size of generated programs.
#[derive(Debug, Clone)]
pub struct Config {
    /// How deeply expressions nest.
    pub max_depth: u32,
    /// How deeply types nest.
    pub max_type_depth: u32,
    pub max_data: usize,
    pub max_defs: usize,
    pub max_entries: usize,
    /// Sometimes produce unbound names and ill-typed parts, for tests of
    /// the syntax alone.
    pub syntactic: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_depth: 6,
            max_type_depth: 3,
            max_data: 2,
            max_defs: 4,
            max_entries: 2,
            syntactic: false,
        }
    }
}

/// The type of a name that is in scope but must not be used. No expression
/// is ever generated at a parameter type.
const HIDDEN: Ty = Ty::Param(usize::MAX);

/// Names a syntactic program may use without binding them.
const UNBOUND: &[&str] = &["foo", "bar'", "_x", "a_b", "zz9", "mapp"];

pub(crate) struct Gen<'a, 'b> {
    u: &'a mut Unstructured<'b>,
    config: Config,
    data: Vec<DataDecl>,
    /// Variables in scope, innermost last. Definitions come first.
    env: Vec<(String, Ty)>,
    next_name: usize,
}

impl<'a, 'b> Gen<'a, 'b> {
    pub(crate) fn new(u: &'a mut Unstructured<'b>, config: Config) -> Self {
        Self {
            u,
            config,
            data: Vec::new(),
            env: Vec::new(),
            next_name: 0,
        }
    }

    pub(crate) fn program(mut self) -> Result<Program> {
        let data_count = self.u.int_in_range(0..=self.config.max_data)?;
        for i in 0..data_count {
            let decl = self.data_decl(i)?;
            self.data.push(decl);
        }
        let def_count = self.u.int_in_range(1..=self.config.max_defs.max(1))?;
        let mut defs = Vec::new();
        for i in 0..def_count {
            let ty = self.ty(self.config.max_type_depth)?;
            defs.push((format!("f{i}"), ty));
        }
        self.env = defs.clone();
        let mut out = Vec::new();
        for (name, ty) in defs {
            let body = self.expr(&ty, self.config.max_depth)?;
            out.push(Def {
                name,
                ty,
                ann: None,
                body,
            });
        }
        let mut entries = Vec::new();
        for _ in 0..self.u.int_in_range(0..=self.config.max_entries)? {
            let ty = self.ty(self.config.max_type_depth)?;
            entries.push((self.expr(&ty, self.config.max_depth)?, ty));
        }
        Ok(Program {
            data: self.data,
            defs: out,
            entries,
        })
    }

    // ---------------------------------------------------------------- types

    fn data_decl(&mut self, index: usize) -> Result<DataDecl> {
        let params = self.u.int_in_range(0..=2)?;
        let ctor_count = self.u.int_in_range(1..=3)?;
        let mut ctors = Vec::new();
        for k in 0..ctor_count {
            let mut fields = Vec::new();
            for _ in 0..self.u.int_in_range(0..=2)? {
                fields.push(self.field_ty(index, params, k > 0)?);
            }
            ctors.push((format!("C{index}_{k}"), fields));
        }
        Ok(DataDecl {
            name: format!("T{index}"),
            params,
            ctors,
        })
    }

    /// A field of data type `index`; only `recursive` fields may mention it.
    fn field_ty(&mut self, index: usize, params: usize, recursive: bool) -> Result<Ty> {
        let own = || Ty::Data(index, (0..params).map(Ty::Param).collect());
        Ok(match self.u.int_in_range(0..=5)? {
            0 if params > 0 => Ty::Param(self.u.int_in_range(0..=params - 1)?),
            1 if recursive => own(),
            2 if recursive => Ty::list(own()),
            3 if params > 0 => Ty::fun(Ty::Int, Ty::Param(0)),
            4 if index > 0 => {
                let other = self.u.int_in_range(0..=index - 1)?;
                let args = (0..self.data[other].params)
                    .map(|_| self.ty(0))
                    .collect::<Result<_>>()?;
                Ty::Data(other, args)
            }
            _ => self.ty(1)?,
        })
    }

    fn ty(&mut self, depth: u32) -> Result<Ty> {
        let simple = [Ty::Int, Ty::Float, Ty::String, Ty::Bool, Ty::Unit];
        if depth == 0 || self.u.ratio(1, 3)? {
            return Ok(self.u.choose(&simple)?.clone());
        }
        let d = depth - 1;
        Ok(match self.u.int_in_range(0..=7)? {
            0 => Ty::list(self.ty(d)?),
            1 => Ty::option(self.ty(d)?),
            2 => Ty::result(self.ty(d)?, self.ty(d)?),
            3 => {
                let n = self.u.int_in_range(2..=3)?;
                Ty::Tuple((0..n).map(|_| self.ty(d)).collect::<Result<_>>()?)
            }
            4 => Ty::fun(self.ty(d)?, self.ty(d)?),
            5 => Ty::io(self.ty(d)?),
            6 if !self.data.is_empty() => {
                let index = self.u.int_in_range(0..=self.data.len() - 1)?;
                let args = (0..self.data[index].params)
                    .map(|_| self.ty(d))
                    .collect::<Result<_>>()?;
                Ty::Data(index, args)
            }
            _ => self.u.choose(&simple)?.clone(),
        })
    }

    /// Whether values of `ty` can be compared and shown: no functions or
    /// actions anywhere inside.
    fn comparable(&self, ty: &Ty) -> bool {
        self.comparable_in(ty, &mut Vec::new())
    }

    fn comparable_in(&self, ty: &Ty, seen: &mut Vec<(usize, Vec<Ty>)>) -> bool {
        match ty {
            Ty::Fun(..) | Ty::Io(_) => false,
            Ty::Int | Ty::Float | Ty::String | Ty::Bool | Ty::Unit | Ty::Param(_) => true,
            Ty::List(t) | Ty::Option(t) => self.comparable_in(t, seen),
            Ty::Result(e, a) => self.comparable_in(e, seen) && self.comparable_in(a, seen),
            Ty::Tuple(items) => items.iter().all(|t| self.comparable_in(t, seen)),
            Ty::Data(d, args) => {
                let key = (*d, args.clone());
                if seen.contains(&key) {
                    return true;
                }
                seen.push(key);
                self.data[*d]
                    .ctors
                    .iter()
                    .flat_map(|(_, fields)| fields)
                    .all(|field| self.comparable_in(&field.subst(args), seen))
            }
        }
    }

    // ------------------------------------------------------------ variables

    fn fresh(&mut self) -> String {
        self.next_name += 1;
        format!("x{}", self.next_name)
    }

    /// A name for a new local: usually fresh, sometimes shadowing one.
    fn binder(&mut self) -> Result<String> {
        let locals: Vec<String> = self
            .env
            .iter()
            .filter(|(name, _)| name.starts_with('x'))
            .map(|(name, _)| name.clone())
            .collect();
        if !locals.is_empty() && self.u.ratio(1, 6)? {
            return Ok(self.u.choose(&locals)?.clone());
        }
        Ok(self.fresh())
    }

    /// The variables of type `ty` that no later binding shadows.
    fn visible(&self, ty: &Ty) -> Vec<String> {
        self.env
            .iter()
            .enumerate()
            .filter(|(i, (name, t))| {
                t == ty && !self.env[i + 1..].iter().any(|(later, _)| later == name)
            })
            .map(|(_, (name, _))| name.clone())
            .collect()
    }

    /// The visible variables whose type is a function returning `ty` after
    /// one or two arguments, with the argument types.
    fn visible_functions(&self, ty: &Ty) -> Vec<(String, Vec<Ty>)> {
        let mut out = Vec::new();
        for (i, (name, t)) in self.env.iter().enumerate() {
            if self.env[i + 1..].iter().any(|(later, _)| later == name) {
                continue;
            }
            if let Ty::Fun(a, b) = t {
                if **b == *ty {
                    out.push((name.clone(), vec![(**a).clone()]));
                } else if let Ty::Fun(b1, c) = &**b {
                    if **c == *ty {
                        out.push((name.clone(), vec![(**a).clone(), (**b1).clone()]));
                    }
                }
            }
        }
        out
    }

    /// Runs `f` with `binds` in scope.
    fn scoped<T>(
        &mut self,
        binds: Vec<(String, Ty)>,
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let len = self.env.len();
        self.env.extend(binds);
        let result = f(self);
        self.env.truncate(len);
        result
    }

    // ---------------------------------------------------------- expressions

    fn expr(&mut self, ty: &Ty, depth: u32) -> Result<Expr> {
        if depth == 0 || self.u.is_empty() || self.u.ratio(1, 6)? {
            return self.leaf(ty);
        }
        let d = depth - 1;
        match self.u.int_in_range(0..=11)? {
            0 => {
                // `let x = value in body`, sometimes annotated.
                let value_ty = self.ty(2)?;
                // `let` is recursive: the value sees the new name, which
                // hides any outer variable of that name.
                let name = self.binder()?;
                let hidden = vec![(name.clone(), HIDDEN)];
                let value = self.scoped(hidden, |g| g.expr(&value_ty, d))?;
                let ann = self.u.ratio(1, 3)?.then(|| value_ty.clone());
                let body = self.scoped(vec![(name.clone(), value_ty)], |g| g.expr(ty, d))?;
                Ok(Expr::Let {
                    pat: Pat::Var(name),
                    ann,
                    value: Box::new(value),
                    body: Box::new(body),
                })
            }
            1 => {
                // `let (a, b) = pair in body`.
                let n = self.u.int_in_range(2..=3)?;
                let items: Vec<Ty> = (0..n).map(|_| self.ty(1)).collect::<Result<_>>()?;
                let value = self.expr(&Ty::Tuple(items.clone()), d)?;
                // Fresh names: a `let` is recursive, so reusing a name the
                // value mentions would capture it.
                let (pat, binds) = self.irrefutable_in(&Ty::Tuple(items), false)?;
                let body = self.scoped(binds, |g| g.expr(ty, d))?;
                Ok(Expr::Let {
                    pat,
                    ann: None,
                    value: Box::new(value),
                    body: Box::new(body),
                })
            }
            2 => Ok(Expr::app(
                "if",
                vec![
                    self.expr(&Ty::Bool, d)?,
                    self.expr(ty, d)?,
                    self.expr(ty, d)?,
                ],
            )),
            3 => self.match_expr(ty, d),
            4 => {
                // `((\x. body) arg)`.
                let arg_ty = self.ty(1)?;
                let arg = self.expr(&arg_ty, d)?;
                let lambda = self.lambda(&arg_ty, ty, d)?;
                Ok(Expr::App(Box::new(lambda), vec![arg]))
            }
            5 | 6 => match self.builtin(ty, d)? {
                Some(e) => Ok(e),
                None => self.intro(ty, d),
            },
            7 => {
                let functions = self.visible_functions(ty);
                if functions.is_empty() {
                    return self.intro(ty, d);
                }
                let (name, args) = self.u.choose(&functions)?.clone();
                let args = args
                    .iter()
                    .map(|t| self.expr(t, d))
                    .collect::<Result<_>>()?;
                Ok(Expr::App(Box::new(Expr::Var(name)), args))
            }
            8 if self.config.syntactic => self.junk(d),
            _ => self.intro(ty, d),
        }
    }

    /// Anything grammatical, ignoring scope and types.
    fn junk(&mut self, depth: u32) -> Result<Expr> {
        Ok(match self.u.int_in_range(0..=3)? {
            0 => Expr::Var(self.u.choose(UNBOUND)?.to_string()),
            1 => Expr::Global(format!("Unknown{}", self.u.int_in_range(0..=3)?)),
            2 => {
                let ty = self.ty(1)?;
                let other = self.ty(1)?;
                Expr::App(
                    Box::new(self.expr(&ty, depth)?),
                    vec![self.expr(&other, depth)?],
                )
            }
            _ => {
                let ty = self.ty(2)?;
                self.expr(&ty, depth)?
            }
        })
    }

    /// The smallest expressions of type `ty`.
    fn leaf(&mut self, ty: &Ty) -> Result<Expr> {
        let vars = self.visible(ty);
        if !vars.is_empty() && self.u.ratio(1, 2)? {
            return Ok(Expr::Var(self.u.choose(&vars)?.clone()));
        }
        Ok(match ty {
            Ty::Int => Expr::Int(self.int()?),
            Ty::Float => Expr::Float(self.float()?),
            Ty::String => Expr::String(self.string()?),
            Ty::Bool => self.bool()?,
            Ty::Unit => Expr::Unit,
            Ty::List(_) => Expr::List(Vec::new()),
            Ty::Option(_) => Expr::Global("None".into()),
            Ty::Result(e, a) => {
                if self.u.arbitrary()? {
                    Expr::app("Err", vec![self.leaf(e)?])
                } else {
                    Expr::app("Ok", vec![self.leaf(a)?])
                }
            }
            Ty::Tuple(items) => {
                Expr::Tuple(items.iter().map(|t| self.leaf(t)).collect::<Result<_>>()?)
            }
            Ty::Fun(a, b) => self.lambda(a, b, 0)?,
            Ty::Io(t) => Expr::app("pure", vec![self.leaf(t)?]),
            Ty::Data(d, args) => {
                let (name, fields) = self.data[*d].ctors[0].clone();
                self.construct(&name, &fields, args, 0)?
            }
            Ty::Param(_) => unreachable!("parameters only occur in declarations"),
        })
    }

    /// The forms that build a value of `ty` directly.
    fn intro(&mut self, ty: &Ty, d: u32) -> Result<Expr> {
        Ok(match ty {
            Ty::Int | Ty::Float | Ty::String | Ty::Bool | Ty::Unit => self.leaf(ty)?,
            Ty::List(t) => {
                if self.u.arbitrary()? {
                    let n = self.u.int_in_range(0..=3)?;
                    Expr::List((0..n).map(|_| self.expr(t, d)).collect::<Result<_>>()?)
                } else {
                    Expr::app("cons", vec![self.expr(t, d)?, self.expr(ty, d)?])
                }
            }
            Ty::Option(t) => {
                if self.u.ratio(1, 4)? {
                    Expr::Global("None".into())
                } else {
                    Expr::app("Some", vec![self.expr(t, d)?])
                }
            }
            Ty::Result(e, a) => {
                if self.u.arbitrary()? {
                    Expr::app("Err", vec![self.expr(e, d)?])
                } else {
                    Expr::app("Ok", vec![self.expr(a, d)?])
                }
            }
            Ty::Tuple(items) => Expr::Tuple(
                items
                    .iter()
                    .map(|t| self.expr(t, d))
                    .collect::<Result<_>>()?,
            ),
            Ty::Fun(a, b) => {
                if **a == Ty::Int && **b == Ty::Int && self.u.ratio(1, 4)? {
                    // A partially applied builtin.
                    let op = *self.u.choose(&["add", "sub", "mul"])?;
                    Expr::app(op, vec![self.expr(a, d)?])
                } else {
                    self.lambda(a, b, d)?
                }
            }
            Ty::Io(t) => self.action(t, d)?,
            Ty::Data(index, args) => {
                let ctors = self.data[*index].ctors.clone();
                let (name, fields) = self.u.choose(&ctors)?.clone();
                self.construct(&name, &fields, args, d)?
            }
            Ty::Param(_) => unreachable!("parameters only occur in declarations"),
        })
    }

    fn construct(&mut self, name: &str, fields: &[Ty], args: &[Ty], d: u32) -> Result<Expr> {
        if fields.is_empty() {
            return Ok(Expr::Global(name.to_owned()));
        }
        let values = fields
            .iter()
            .map(|field| {
                let t = field.subst(args);
                if d == 0 {
                    self.leaf(&t)
                } else {
                    self.expr(&t, d)
                }
            })
            .collect::<Result<_>>()?;
        Ok(Expr::app(name, values))
    }

    /// `\param. body` of type `a -> b`.
    fn lambda(&mut self, a: &Ty, b: &Ty, d: u32) -> Result<Expr> {
        let (param, binds) = self.irrefutable(a)?;
        let ann = self.u.ratio(1, 4)?.then(|| a.clone());
        let body = self.scoped(binds, |g| if d == 0 { g.leaf(b) } else { g.expr(b, d) })?;
        Ok(Expr::Lambda {
            param,
            ann,
            body: Box::new(body),
        })
    }

    /// An action producing `t`.
    fn action(&mut self, t: &Ty, d: u32) -> Result<Expr> {
        match self.u.int_in_range(0..=4)? {
            0 if *t == Ty::Unit => {
                let op = *self.u.choose(&["println", "print"])?;
                Ok(Expr::app(op, vec![self.expr(&Ty::String, d)?]))
            }
            1 => {
                let first_ty = self.ty(1)?;
                Ok(Expr::app(
                    "then",
                    vec![
                        self.expr(&Ty::io(first_ty), d)?,
                        self.expr(&Ty::io(t.clone()), d)?,
                    ],
                ))
            }
            2 => {
                // `let x <- action in body`.
                let result_ty = self.ty(1)?;
                let action = self.expr(&Ty::io(result_ty.clone()), d)?;
                let (pat, binds) = if self.u.arbitrary()? {
                    let name = self.binder()?;
                    (Pat::Var(name.clone()), vec![(name, result_ty.clone())])
                } else {
                    self.irrefutable(&result_ty)?
                };
                // Only a name, not a pattern, can be annotated.
                let ann =
                    (matches!(pat, Pat::Var(_)) && self.u.ratio(1, 4)?).then(|| result_ty.clone());
                let body = self.scoped(binds, |g| g.expr(&Ty::io(t.clone()), d))?;
                Ok(Expr::Bind {
                    pat,
                    ann,
                    action: Box::new(action),
                    body: Box::new(body),
                })
            }
            3 if matches!(t, Ty::List(_)) => {
                let Ty::List(item) = t else { unreachable!() };
                let n = self.u.int_in_range(0..=2)?;
                let actions = (0..n)
                    .map(|_| self.expr(&Ty::io((**item).clone()), d))
                    .collect::<Result<_>>()?;
                Ok(Expr::app("sequence", vec![Expr::List(actions)]))
            }
            _ => Ok(Expr::app("pure", vec![self.expr(t, d)?])),
        }
    }

    /// A builtin application whose result has type `ty`, if one fits.
    fn builtin(&mut self, ty: &Ty, d: u32) -> Result<Option<Expr>> {
        let any = self.ty(1)?;
        let e = match (ty, self.u.int_in_range(0..=5)?) {
            (Ty::Int | Ty::Float, 0) => {
                let op = *self.u.choose(&["add", "sub", "mul"])?;
                Expr::app(op, vec![self.expr(ty, d)?, self.expr(ty, d)?])
            }
            (Ty::Int, 1) => Expr::app("length", vec![self.expr(&Ty::list(any), d)?]),
            (Ty::Int, 2) => Expr::app("stringLength", vec![self.expr(&Ty::String, d)?]),
            (Ty::Int, 3) if self.comparable(&any) => {
                Expr::app("compare", vec![self.expr(&any, d)?, self.expr(&any, d)?])
            }
            (Ty::Int, 4) => Expr::app(
                "foldl",
                vec![
                    self.expr(&Ty::fun(Ty::Int, Ty::fun(any.clone(), Ty::Int)), d)?,
                    self.expr(&Ty::Int, d)?,
                    self.expr(&Ty::list(any), d)?,
                ],
            ),
            (Ty::Float, 1) => Expr::app("fdiv", vec![self.expr(ty, d)?, self.expr(ty, d)?]),
            (Ty::Float, 2) => Expr::app("toFloat", vec![self.expr(&Ty::Int, d)?]),
            (Ty::String, 0) => Expr::app(
                "concat",
                vec![self.expr(&Ty::String, d)?, self.expr(&Ty::String, d)?],
            ),
            (Ty::String, 1 | 2) if self.comparable(&any) => {
                Expr::app("show", vec![self.expr(&any, d)?])
            }
            (Ty::String, 3) => Expr::app(
                "join",
                vec![
                    self.expr(&Ty::String, d)?,
                    self.expr(&Ty::list(Ty::String), d)?,
                ],
            ),
            (Ty::Bool, 0 | 1) if self.comparable(&any) => {
                let op = *self.u.choose(&["eq", "neq", "lt", "le", "gt", "ge"])?;
                Expr::app(op, vec![self.expr(&any, d)?, self.expr(&any, d)?])
            }
            (Ty::Bool, 2) => Expr::app("not", vec![self.expr(&Ty::Bool, d)?]),
            (Ty::Bool, 3) => {
                let op = *self.u.choose(&["and", "or"])?;
                Expr::app(op, vec![self.expr(&Ty::Bool, d)?, self.expr(&Ty::Bool, d)?])
            }
            (Ty::Bool, 4) => Expr::app("isEmpty", vec![self.expr(&Ty::list(any), d)?]),
            (Ty::List(t), 0) => Expr::app(
                "map",
                vec![
                    self.expr(&Ty::fun(any.clone(), (**t).clone()), d)?,
                    self.expr(&Ty::list(any), d)?,
                ],
            ),
            (Ty::List(t), 1) => Expr::app(
                "filter",
                vec![
                    self.expr(&Ty::fun((**t).clone(), Ty::Bool), d)?,
                    self.expr(ty, d)?,
                ],
            ),
            (Ty::List(_), 2) => {
                let op = *self.u.choose(&["append", "reverse"])?;
                let args = if op == "append" { 2 } else { 1 };
                let args = (0..args).map(|_| self.expr(ty, d)).collect::<Result<_>>()?;
                Expr::app(op, args)
            }
            (Ty::List(_), 3) => {
                let op = *self.u.choose(&["take", "drop", "remove"])?;
                Expr::app(op, vec![self.expr(&Ty::Int, d)?, self.expr(ty, d)?])
            }
            (Ty::List(t), 4) if self.comparable(t) => Expr::app("sort", vec![self.expr(ty, d)?]),
            (Ty::List(t), 5) if **t == Ty::Int => Expr::app(
                "range",
                vec![self.expr(&Ty::Int, d)?, self.expr(&Ty::Int, d)?],
            ),
            (Ty::Option(t), 0) => Expr::app("head", vec![self.expr(&Ty::list((**t).clone()), d)?]),
            (Ty::Option(t), 1) => Expr::app(
                "get",
                vec![
                    self.expr(&Ty::Int, d)?,
                    self.expr(&Ty::list((**t).clone()), d)?,
                ],
            ),
            (Ty::Option(t), 2) => Expr::app(
                "mapOption",
                vec![
                    self.expr(&Ty::fun(any.clone(), (**t).clone()), d)?,
                    self.expr(&Ty::option(any), d)?,
                ],
            ),
            (Ty::Option(t), 3) => Expr::app(
                "toOption",
                vec![self.expr(&Ty::result(any, (**t).clone()), d)?],
            ),
            (Ty::Result(e, a), 0) => Expr::app(
                "mapResult",
                vec![
                    self.expr(&Ty::fun(any.clone(), (**a).clone()), d)?,
                    self.expr(&Ty::result((**e).clone(), any), d)?,
                ],
            ),
            (_, 0) => Expr::app(
                "fst",
                vec![self.expr(&Ty::Tuple(vec![ty.clone(), any]), d)?],
            ),
            (_, 1) => Expr::app(
                "snd",
                vec![self.expr(&Ty::Tuple(vec![any, ty.clone()]), d)?],
            ),
            (_, 2) => Expr::app(
                "withDefault",
                vec![self.expr(ty, d)?, self.expr(&Ty::option(ty.clone()), d)?],
            ),
            (_, 3) => Expr::app("unwrap", vec![self.expr(&Ty::option(ty.clone()), d)?]),
            _ => return Ok(None),
        };
        Ok(Some(e))
    }

    // ------------------------------------------------------------- patterns

    /// A pattern that matches every value of `ty`, with the variables it binds.
    fn irrefutable(&mut self, ty: &Ty) -> Result<(Pat, Vec<(String, Ty)>)> {
        self.irrefutable_in(ty, true)
    }

    /// `shadow` allows reusing a name in scope, which only the top of a
    /// pattern may do: one pattern cannot bind a name twice.
    fn irrefutable_in(&mut self, ty: &Ty, shadow: bool) -> Result<(Pat, Vec<(String, Ty)>)> {
        match ty {
            Ty::Tuple(items) if self.u.ratio(2, 3)? => {
                let mut pats = Vec::new();
                let mut binds = Vec::new();
                for item in items {
                    let (p, b) = self.irrefutable_in(item, false)?;
                    pats.push(p);
                    binds.extend(b);
                }
                Ok((Pat::Tuple(pats), binds))
            }
            Ty::Unit if self.u.ratio(1, 2)? => Ok((Pat::Unit, Vec::new())),
            _ if self.u.ratio(1, 5)? => Ok((Pat::Wild, Vec::new())),
            _ => {
                let name = if shadow { self.binder()? } else { self.fresh() };
                Ok((Pat::Var(name.clone()), vec![(name, ty.clone())]))
            }
        }
    }

    /// Any pattern of type `ty`, with the variables it binds.
    fn refutable(&mut self, ty: &Ty, depth: u32) -> Result<(Pat, Vec<(String, Ty)>)> {
        if depth == 0 || self.u.ratio(1, 3)? {
            return self.irrefutable_in(ty, false);
        }
        let d = depth - 1;
        let fields = |g: &mut Self, name: &str, tys: Vec<Ty>| -> Result<_> {
            let mut pats = Vec::new();
            let mut binds = Vec::new();
            for t in tys {
                let (p, b) = g.refutable(&t, d)?;
                pats.push(p);
                binds.extend(b);
            }
            Ok((Pat::Con(name.to_owned(), pats), binds))
        };
        Ok(match ty {
            Ty::Int => (Pat::Int(self.int()?), Vec::new()),
            Ty::Float => (Pat::Float(self.float()?), Vec::new()),
            Ty::String => (Pat::String(self.string()?), Vec::new()),
            Ty::Bool => {
                let name = if self.u.arbitrary()? { "True" } else { "False" };
                (Pat::Con(name.into(), Vec::new()), Vec::new())
            }
            Ty::Option(t) => {
                if self.u.arbitrary()? {
                    (Pat::Con("None".into(), Vec::new()), Vec::new())
                } else {
                    fields(self, "Some", vec![(**t).clone()])?
                }
            }
            Ty::Result(e, a) => {
                if self.u.arbitrary()? {
                    fields(self, "Err", vec![(**e).clone()])?
                } else {
                    fields(self, "Ok", vec![(**a).clone()])?
                }
            }
            Ty::Tuple(items) => {
                let (pat, binds) = fields(self, "", items.clone())?;
                let Pat::Con(_, pats) = pat else {
                    unreachable!()
                };
                (Pat::Tuple(pats), binds)
            }
            Ty::Data(index, args) => {
                let ctors = self.data[*index].ctors.clone();
                let (name, tys) = self.u.choose(&ctors)?.clone();
                fields(self, &name, tys.iter().map(|t| t.subst(args)).collect())?
            }
            _ => self.irrefutable_in(ty, false)?,
        })
    }

    /// The constructors of a sum type, with their field types.
    fn constructors(&self, ty: &Ty) -> Option<Vec<(String, Vec<Ty>)>> {
        Some(match ty {
            Ty::Bool => vec![("False".into(), vec![]), ("True".into(), vec![])],
            Ty::Option(t) => vec![
                ("None".into(), vec![]),
                ("Some".into(), vec![(**t).clone()]),
            ],
            Ty::Result(e, a) => vec![
                ("Err".into(), vec![(**e).clone()]),
                ("Ok".into(), vec![(**a).clone()]),
            ],
            Ty::Data(index, args) => self.data[*index]
                .ctors
                .iter()
                .map(|(name, fields)| {
                    (name.clone(), fields.iter().map(|f| f.subst(args)).collect())
                })
                .collect(),
            _ => return None,
        })
    }

    /// An exhaustive `match` producing `ty`.
    fn match_expr(&mut self, ty: &Ty, d: u32) -> Result<Expr> {
        let scrutinee_ty = self.ty(2)?;
        let scrutinee = self.expr(&scrutinee_ty, d)?;
        let mut arms = Vec::new();
        let constructors = self.constructors(&scrutinee_ty);
        if let Some(ctors) = constructors.filter(|_| self.u.ratio(1, 2).unwrap_or(true)) {
            // One arm per constructor, in a random order: exhaustive.
            let mut order: Vec<usize> = (0..ctors.len()).collect();
            for i in (1..order.len()).rev() {
                let j = self.u.int_in_range(0..=i)?;
                order.swap(i, j);
            }
            for i in order {
                let (name, tys) = &ctors[i];
                let mut pats = Vec::new();
                let mut binds = Vec::new();
                for t in tys {
                    let (p, b) = self.irrefutable_in(t, false)?;
                    pats.push(p);
                    binds.extend(b);
                }
                let body = self.scoped(binds, |g| g.expr(ty, d))?;
                arms.push((Pat::Con(name.clone(), pats), body));
            }
        } else {
            for _ in 0..self.u.int_in_range(0..=2)? {
                let (pat, binds) = self.refutable(&scrutinee_ty, 2)?;
                let body = self.scoped(binds, |g| g.expr(ty, d))?;
                arms.push((pat, body));
            }
            // A final arm that matches everything keeps it exhaustive.
            let (pat, binds) = if self.u.arbitrary()? {
                (Pat::Wild, Vec::new())
            } else {
                let name = self.binder()?;
                (Pat::Var(name.clone()), vec![(name, scrutinee_ty.clone())])
            };
            let body = self.scoped(binds, |g| g.expr(ty, d))?;
            arms.push((pat, body));
        }
        Ok(Expr::Match(Box::new(scrutinee), arms))
    }

    // ------------------------------------------------------------- literals

    fn int(&mut self) -> Result<i64> {
        Ok(match self.u.int_in_range(0..=5)? {
            0 => self.u.arbitrary()?,
            1 => *self.u.choose(&[i64::MIN, i64::MAX, -1, 0])?,
            _ => self.u.int_in_range(-3..=20)?,
        })
    }

    fn float(&mut self) -> Result<f64> {
        Ok(match self.u.int_in_range(0..=4)? {
            0 => {
                let x: f64 = self.u.arbitrary()?;
                if x.is_finite() { x } else { 1.5 }
            }
            1 => *self
                .u
                .choose(&[-0.0, 0.0, 1e300, -2.5e-300, f64::MAX, f64::MIN_POSITIVE])?,
            _ => f64::from(self.u.int_in_range(-40..=40)?) / 4.0,
        })
    }

    fn string(&mut self) -> Result<String> {
        Ok(match self.u.int_in_range(0..=3)? {
            0 => self.u.arbitrary()?,
            1 => (*self
                .u
                .choose(&["", "a\"b", "\\", "line\nnext", "tab\t\0", "é😀"])?)
            .to_owned(),
            _ => {
                let n = self.u.int_in_range(0..=4)?;
                (0..n)
                    .map(|_| Ok(char::from(self.u.int_in_range(b'a'..=b'z')?)))
                    .collect::<Result<_>>()?
            }
        })
    }

    fn bool(&mut self) -> Result<Expr> {
        let name = if self.u.arbitrary()? { "True" } else { "False" };
        Ok(Expr::Global(name.into()))
    }
}
