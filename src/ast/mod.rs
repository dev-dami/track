#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLiteral(i64),
    StringLiteral(String),
    BoolLiteral(bool),
    Variable(String),

    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    ArrayLiteral {
        elements: Vec<Expr>,
    },

    ArrayIndex {
        target: Box<Expr>,
        index: Box<Expr>,
    },

    AddressOf {
        target: Box<Expr>,
    },

    StructInitialization {
        ty_name: String,
        fields: Vec<(String, Expr)>,
    },

    LensBlock {
        target: String,
        lens_name: String,
        body: Vec<Expr>,
    },

    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },

    IfElse {
        condition: Box<Expr>,
        then_body: Vec<Expr>,
        else_body: Vec<Expr>,
    },

    WhileLoop {
        condition: Box<Expr>,
        body: Vec<Expr>,
    },

    Return {
        value: Option<Box<Expr>>,
    },

    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    FnDef {
        name: String,
        params: Vec<(String, TrackType)>,
        return_type: Option<TrackType>,
        body: Vec<Expr>,
    },

    Use {
        path: String,
        imports: Option<Vec<String>>,
        alias: Option<String>,
    },

    ConstDef {
        name: String,
        value: Box<Expr>,
    },

    MacroDef {
        name: String,
        params: Vec<(String, TrackType)>,
        return_type: Option<TrackType>,
        body: Vec<Expr>,
    },

    MacroCall {
        name: String,
        args: Vec<Expr>,
        body: Option<Vec<Expr>>,
    },

    LetDef {
        name: String,
        ty: Option<TrackType>,
        value: Box<Expr>,
    },

    EnumDef {
        name: String,
        underlying_type: Option<TrackType>,
        variants: Vec<(String, Option<Expr>)>,
    },

    UnionDef {
        name: String,
        variants: Vec<(String, Option<TrackType>)>,
    },

    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackType {
    I32,
    U32,
    I64,
    U64,
    Bool,
    Ptr(Box<TrackType>),
    Ref(Box<TrackType>),
    Array(Box<TrackType>, usize),
    Void,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    BitAnd,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
    Deref,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Ident(String),
    Variant {
        enum_or_union: String,
        variant: String,
        binding: Option<String>,
    },
    Wildcard,
}
