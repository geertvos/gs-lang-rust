/// Position in source code.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: i32,
    pub column: i32,
}

/// AST expression nodes for GScript.
#[derive(Debug, Clone)]
pub enum Expression {
    Additive(Box<Expression>, String, Box<Expression>),
    Multiplicative(Box<Expression>, String, Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    Equality(Box<Expression>, String, Box<Expression>),
    Relational(Box<Expression>, String, Box<Expression>),
    Assignment {
        value: Box<Expression>,
        operator: Option<String>,
        variable: Box<Expression>,
    },
    Constant {
        value: i32,
        type_name: String,
        string: Option<String>,
    },
    Variable {
        name: String,
        field: Option<Box<Expression>>,
    },
    This {
        field: Option<Box<Expression>>,
    },
    FunctionCall {
        function: Box<Expression>,
        parameters: Vec<Expression>,
        field: Option<Box<Expression>>,
    },
    FunctionDef {
        parameters: Vec<String>,
        statements: Vec<Statement>,
    },
    NativeFunctionCall {
        parameters: Vec<Expression>,
    },
    ConstructorCall {
        function: Box<Expression>,
    },
    ImplicitConstructor {
        statements: Vec<Statement>,
    },
    ArrayDefinition {
        elements: Vec<Expression>,
    },
    ArrayReference {
        index: Box<Expression>,
        reference: Box<Expression>,
    },
    MapDefinition {
        keys: Vec<Expression>,
        values: Vec<Expression>,
    },
    Fork,
    PostFixOperator {
        operator: String,
        argument: Box<Expression>,
    },
    Conditional {
        condition: Box<Expression>,
        positive: Box<Expression>,
        negative: Box<Expression>,
    },
}

/// AST statement nodes for GScript.
#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Expression, Position),
    If {
        condition: Expression,
        then_clause: Box<Statement>,
        else_clause: Option<Box<Statement>>,
        pos: Position,
    },
    While {
        condition: Expression,
        body: Box<Statement>,
        pos: Position,
    },
    For {
        init: Expression,
        condition: Expression,
        update: Expression,
        body: Box<Statement>,
        pos: Position,
    },
    Return {
        value: Option<Expression>,
        pos: Position,
    },
    Throw {
        exception: Expression,
        pos: Position,
    },
    Break(Position),
    Continue(Position),
    TryCatch {
        try_block: Box<Statement>,
        catch_var: String,
        catch_block: Box<Statement>,
        pos: Position,
    },
    Scope {
        statements: Vec<Statement>,
        pos: Position,
    },
    Void(Position),
}

/// A GScript module (compilation unit).
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub imports: Vec<String>,
    pub statements: Vec<Statement>,
    pub pos: Position,
}
