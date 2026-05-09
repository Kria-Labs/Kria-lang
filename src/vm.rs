use crate::bytecode::Instruction;
use crate::ast::Literal;
use std::sync::Arc;
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    String(Arc<str>),
    Boolean(bool),
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
        }
    }
}

impl Value {
    #[inline(always)]
    fn from_literal(literal: Literal) -> Self {
        match literal {
            Literal::Number(n) => Value::Number(n),
            Literal::String(s) => Value::String(Arc::from(s)),
            Literal::Boolean(b) => Value::Boolean(b),
            Literal::Null => Value::Null,
        }
    }

    #[inline(always)]
    fn expect_boolean(self) -> Result<bool, String> {
        match self {
            Value::Boolean(b) => Ok(b),
            other => Err(format!("Expected boolean condition, found {:?}", other)),
        }
    }
}

pub struct VM {
    stack: SmallVec<[Value; 256]>,
    globals: Vec<Value>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: SmallVec::new(),
            globals: Vec::new(),
        }
    }

    pub fn execute(&mut self, instructions: &[Instruction]) -> Result<(), String> {
        let mut ip = 0;

        while ip < instructions.len() {
            // Hottest instructions first for better CPU branch prediction
            match &instructions[ip] {
                // Most frequent in loops
                Instruction::AddGlobal(index, rhs) => {
                    if *index >= self.globals.len() {
                        return Err(format!("AddGlobal: global slot {} is not initialized", index));
                    }
                    match &self.globals[*index] {
                        Value::Number(n) => self.globals[*index] = Value::Number(n + rhs),
                        other => return Err(format!("AddGlobal expects number, found {:?}", other)),
                    }
                    ip += 1;
                }
                Instruction::LoadGlobal(index) => {
                    let value = self.globals.get(*index).cloned().unwrap_or(Value::Null);
                    self.stack.push(value);
                    ip += 1;
                }
                Instruction::JumpIfFalse(target) => {
                    let condition = self.pop()?;
                    let value = condition.expect_boolean()?;
                    if !value {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }
                Instruction::Jump(target) => {
                    ip = *target;
                }
                Instruction::IncGlobal(index) => {
                    if *index >= self.globals.len() {
                        return Err(format!("IncGlobal: global slot {} is not initialized", index));
                    }
                    match &self.globals[*index] {
                        Value::Number(n) => self.globals[*index] = Value::Number(n + 1),
                        other => return Err(format!("IncGlobal expects number, found {:?}", other)),
                    }
                    ip += 1;
                }
                Instruction::StoreGlobal(index) => {
                    let value = self.pop()?;
                    if *index >= self.globals.len() {
                        self.globals.resize(*index + 1, Value::Null);
                    }
                    self.globals[*index] = value;
                    ip += 1;
                }
                // Less frequent instructions
                Instruction::Constant(literal) => {
                    self.stack.push(Value::from_literal(literal.clone()));
                    ip += 1;
                }
                Instruction::Add => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(self.add_values(left, right)?);
                    ip += 1;
                }
                Instruction::Subtract => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(self.numeric_op(left, right, |l, r| Ok(l - r))?);
                    ip += 1;
                }
                Instruction::Multiply => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(self.numeric_op(left, right, |l, r| Ok(l * r))?);
                    ip += 1;
                }
                Instruction::Divide => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(self.numeric_op(left, right, |l, r| {
                        if r == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(l / r)
                        }
                    })?);
                    ip += 1;
                }
                Instruction::AddInt => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => self.stack.push(Value::Number(l + r)),
                        (l, r) => return Err(format!("AddInt expects numbers, found {:?} and {:?}", l, r)),
                    }
                    ip += 1;
                }
                Instruction::SubtractInt => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => self.stack.push(Value::Number(l - r)),
                        (l, r) => return Err(format!("SubtractInt expects numbers, found {:?} and {:?}", l, r)),
                    }
                    ip += 1;
                }
                Instruction::MultiplyInt => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => self.stack.push(Value::Number(l * r)),
                        (l, r) => return Err(format!("MultiplyInt expects numbers, found {:?} and {:?}", l, r)),
                    }
                    ip += 1;
                }
                Instruction::DivideInt => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            if r == 0 {
                                return Err("Division by zero".to_string());
                            }
                            self.stack.push(Value::Number(l / r));
                        }
                        (l, r) => return Err(format!("DivideInt expects numbers, found {:?} and {:?}", l, r)),
                    }
                    ip += 1;
                }
                Instruction::Equals => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(left == right));
                    ip += 1;
                }
                Instruction::NotEquals => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(left != right));
                    ip += 1;
                }
                Instruction::GreaterThan => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(self.compare_numeric(left, right, |l, r| l > r)?));
                    ip += 1;
                }
                Instruction::LessThan => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(self.compare_numeric(left, right, |l, r| l < r)?));
                    ip += 1;
                }
                Instruction::GreaterOrEqual => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(self.compare_numeric(left, right, |l, r| l >= r)?));
                    ip += 1;
                }
                Instruction::LessOrEqual => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(self.compare_numeric(left, right, |l, r| l <= r)?));
                    ip += 1;
                }
                Instruction::And => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Boolean(l), Value::Boolean(r)) => self.stack.push(Value::Boolean(l && r)),
                        (l, r) => return Err(format!("And operation requires booleans, found {:?} and {:?}", l, r)),
                    }
                    ip += 1;
                }
                Instruction::Or => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Boolean(l), Value::Boolean(r)) => self.stack.push(Value::Boolean(l || r)),
                        (l, r) => return Err(format!("Or operation requires booleans, found {:?} and {:?}", l, r)),
                    }
                    ip += 1;
                }
                Instruction::Not => {
                    let operand = self.pop()?;
                    match operand {
                        Value::Boolean(b) => self.stack.push(Value::Boolean(!b)),
                        other => return Err(format!("not operation requires boolean, found {:?}", other)),
                    }
                    ip += 1;
                }
                Instruction::Print => {
                    let value = self.pop()?;
                    println!("{}", value);
                    ip += 1;
                }
                Instruction::Pop => {
                    self.pop()?;
                    ip += 1;
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    #[inline(always)]
    fn add_values(&self, left: Value, right: Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::String(l), Value::String(r)) => {
                let concatenated = format!("{}{}", l, r);
                Ok(Value::String(Arc::from(concatenated)))
            }
            (l, r) => Err(format!("Add operation requires two numbers or two strings, found {:?} and {:?}", l, r)),
        }
    }

    #[inline(always)]
    fn numeric_op<F>(&self, left: Value, right: Value, op: F) -> Result<Value, String>
    where
        F: FnOnce(i64, i64) -> Result<i64, String>,
    {
        match (left, right) {
            (Value::Number(l), Value::Number(r)) => op(l, r).map(Value::Number),
            (l, r) => Err(format!("Numeric operation requires two numbers, found {:?} and {:?}", l, r)),
        }
    }

    #[inline(always)]
    fn compare_numeric<F>(&self, left: Value, right: Value, comparator: F) -> Result<bool, String>
    where
        F: FnOnce(i64, i64) -> bool,
    {
        match (left, right) {
            (Value::Number(l), Value::Number(r)) => Ok(comparator(l, r)),
            (l, r) => Err(format!("Comparison requires numbers, found {:?} and {:?}", l, r)),
        }
    }
}
