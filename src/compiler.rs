use std::collections::HashMap;
use crate::ast::{Expression, Statement};
use crate::bytecode::{Instruction, Bytecode};

pub struct Compiler {
    instructions: Bytecode,
    globals: HashMap<String, usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            globals: HashMap::new(),
        }
    }

    pub fn compile(mut self, statements: &[Statement]) -> Result<Bytecode, String> {
        for statement in statements {
            self.compile_statement(statement)?;
        }
        // Apply constant folding optimization
        self.instructions = Self::fold_constants(self.instructions);
        Ok(self.instructions)
    }

    /// Fold constant expressions at compile time
    fn fold_constants(mut instructions: Bytecode) -> Bytecode {
        let mut i = 0;
        while i < instructions.len().saturating_sub(2) {
            // Pattern: Constant(a), Constant(b), BinaryOp
            if let (
                Instruction::Constant(crate::ast::Literal::Number(a)),
                Instruction::Constant(crate::ast::Literal::Number(b)),
                binary_op,
            ) = (&instructions[i], &instructions[i + 1], &instructions[i + 2])
            {
                let folded = match binary_op {
                    Instruction::Add => Some(a + b),
                    Instruction::Subtract => Some(a - b),
                    Instruction::Multiply => Some(a * b),
                    Instruction::Divide if *b != 0 => Some(a / b),
                    _ => None,
                };

                if let Some(result) = folded {
                    instructions[i] = Instruction::Constant(crate::ast::Literal::Number(result));
                    instructions.remove(i + 1);
                    instructions.remove(i + 1);
                    continue;
                }
            }

            i += 1;
        }
        instructions
    }

    fn emit(&mut self, instruction: Instruction) -> usize {
        let index = self.instructions.len();
        self.instructions.push(instruction);
        index
    }

    fn patch_jump(&mut self, index: usize, target: usize) {
        let previous = std::mem::replace(&mut self.instructions[index], Instruction::Jump(target));
        self.instructions[index] = match previous {
            Instruction::Jump(_) => Instruction::Jump(target),
            Instruction::JumpIfFalse(_) => Instruction::JumpIfFalse(target),
            other => panic!("Attempted to patch a non-jump instruction: {:?}", other),
        };
    }

    fn compile_statement(&mut self, statement: &Statement) -> Result<(), String> {
        match statement {
            Statement::Assignment { name, value } => {
                if let Some(instruction) = self.compile_special_assignment(name, value)? {
                    self.emit(instruction);
                } else {
                    let index = self.resolve_global(name);
                    self.compile_expression(value)?;
                    self.emit(Instruction::StoreGlobal(index));
                }
            }
            Statement::Print(expr) => {
                self.compile_expression(expr)?;
                self.emit(Instruction::Print);
            }
            Statement::If { branches, else_branch } => {
                let mut jump_to_end_locations = Vec::new();

                for (index, (condition, branch)) in branches.iter().enumerate() {
                    self.compile_expression(condition)?;
                    let jump_to_next = self.emit(Instruction::JumpIfFalse(0));
                    self.compile_block(branch)?;
                    let jump_over_rest = self.emit(Instruction::Jump(0));
                    jump_to_end_locations.push(jump_over_rest);

                    let next_location = self.instructions.len();
                    self.patch_jump(jump_to_next, next_location);

                    if index == branches.len() - 1 {
                        break;
                    }
                }

                if let Some(else_branch) = else_branch {
                    self.compile_block(else_branch)?;
                }

                let end_location = self.instructions.len();
                for location in jump_to_end_locations {
                    self.patch_jump(location, end_location);
                }
            }
            Statement::While { condition, body } => {
                let loop_start = self.instructions.len();
                self.compile_expression(condition)?;
                let exit_jump = self.emit(Instruction::JumpIfFalse(0));
                self.compile_block(body)?;
                self.emit(Instruction::Jump(loop_start));
                let loop_end = self.instructions.len();
                self.patch_jump(exit_jump, loop_end);
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                self.emit(Instruction::Pop);
            }
        }

        Ok(())
    }

    fn compile_block(&mut self, block: &[Statement]) -> Result<(), String> {
        for statement in block {
            self.compile_statement(statement)?;
        }
        Ok(())
    }

    fn compile_special_assignment(&mut self, name: &String, value: &Expression) -> Result<Option<Instruction>, String> {
        match value {
            Expression::BinaryOp { left, op, right } => {
                match (&**left, &**right) {
                    (Expression::Identifier(src), Expression::Literal(crate::ast::Literal::Number(n)))
                    | (Expression::Literal(crate::ast::Literal::Number(n)), Expression::Identifier(src))
                        if src == name =>
                    {
                        let index = self.resolve_global(name);
                        return match op {
                            crate::ast::BinaryOperator::Add => {
                                if *n == 1 {
                                    Ok(Some(Instruction::IncGlobal(index)))
                                } else {
                                    Ok(Some(Instruction::AddGlobal(index, *n)))
                                }
                            }
                            crate::ast::BinaryOperator::Subtract => {
                                if let Expression::Identifier(_) = &**left {
                                    Ok(Some(Instruction::AddGlobal(index, -*n)))
                                } else {
                                    Ok(None)
                                }
                            }
                            _ => Ok(None),
                        };
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    fn compile_expression(&mut self, expression: &Expression) -> Result<(), String> {
        match expression {
            Expression::Literal(literal) => {
                self.emit(Instruction::Constant(literal.clone()));
            }
            Expression::Identifier(name) => {
                let index = self.resolve_global(name);
                self.emit(Instruction::LoadGlobal(index));
            }
            Expression::UnaryOp { op, expr } => {
                self.compile_expression(expr)?;
                match op {
                    crate::ast::UnaryOperator::Not => self.emit(Instruction::Not),
                };
            }
            Expression::BinaryOp { left, op, right } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                self.emit(match op {
                    crate::ast::BinaryOperator::Add => Instruction::Add,
                    crate::ast::BinaryOperator::Subtract => Instruction::Subtract,
                    crate::ast::BinaryOperator::Multiply => Instruction::Multiply,
                    crate::ast::BinaryOperator::Divide => Instruction::Divide,
                    crate::ast::BinaryOperator::Equals => Instruction::Equals,
                    crate::ast::BinaryOperator::NotEquals => Instruction::NotEquals,
                    crate::ast::BinaryOperator::GreaterThan => Instruction::GreaterThan,
                    crate::ast::BinaryOperator::LessThan => Instruction::LessThan,
                    crate::ast::BinaryOperator::GreaterOrEqual => Instruction::GreaterOrEqual,
                    crate::ast::BinaryOperator::LessOrEqual => Instruction::LessOrEqual,
                    crate::ast::BinaryOperator::And => Instruction::And,
                    crate::ast::BinaryOperator::Or => Instruction::Or,
                });
            }
            Expression::FunctionCall { name, .. } => {
                return Err(format!("Function calls are not supported in bytecode yet: {}", name));
            }
        }
        Ok(())
    }

    fn resolve_global(&mut self, name: &str) -> usize {
        if let Some(&index) = self.globals.get(name) {
            index
        } else {
            let index = self.globals.len();
            self.globals.insert(name.to_string(), index);
            index
        }
    }
}
