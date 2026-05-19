/// Abstract Syntax Tree definitions for Kria language

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment {
        name: String,
        value: Expression,
    },
    Print(Expression),
    If {
        branches: Vec<(Expression, Vec<Statement>)>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Return(Option<Expression>),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    FunctionExpr {
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Input(InputType),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputType {
    String,
    Int,
    Float,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    Not,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(i64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    And,
    Or,
}
