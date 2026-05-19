use std::collections::HashMap;
use crate::ast::{Expression, Statement, Literal, BinaryOperator, UnaryOperator};
use crate::bytecode::*;
use crate::vm::Value;
use std::sync::Arc;

pub struct Compiler {
    bytecode: Bytecode,
    globals: HashMap<String, usize>,
    local_scope: Vec<HashMap<String, usize>>,  // Stack of local scopes
    function_definitions: HashMap<String, (usize, u32)>, // function_name -> (bytecode_offset, num_params)
    return_stack: Vec<usize>,  // Stack of jump positions for return statements
    in_function: bool,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            bytecode: Bytecode::new(),
            globals: HashMap::new(),
            local_scope: Vec::new(),
            function_definitions: HashMap::new(),
            return_stack: Vec::new(),
            in_function: false,
        }
    }

    pub fn compile(mut self, statements: &[Statement]) -> Result<Bytecode, String> {
        for statement in statements {
            self.compile_statement(statement)?;
        }
        self.bytecode.num_globals = self.globals.len();
        Ok(self.bytecode)
    }

    fn emit_opcode(&mut self, op: u8) -> usize {
        self.bytecode.emit_byte(op)
    }

    fn emit_u32(&mut self, value: u32) -> usize {
        self.bytecode.emit_u32(value)
    }

    fn emit_i64(&mut self, value: i64) -> usize {
        self.bytecode.emit_i64(value)
    }

    fn patch_u32(&mut self, pos: usize, value: u32) {
        self.bytecode.patch_u32(pos, value);
    }

    fn emit_constant(&mut self, value: Value) {
        let idx = self.bytecode.add_constant(value);
        self.emit_opcode(OP_CONSTANT);
        self.emit_u32(idx);
    }

    fn compile_statement(&mut self, statement: &Statement) -> Result<(), String> {
        match statement {
            Statement::Assignment { name, value } => {
                if self.compile_special_assignment(name, value)?.is_some() {
                    return Ok(());
                }
                let (is_local, index) = self.resolve_identifier(name)?;
                self.compile_expression(value)?;
                if is_local {
                    self.emit_opcode(OP_STORE_LOCAL);
                } else {
                    self.emit_opcode(OP_STORE_GLOBAL);
                }
                self.emit_u32(index as u32);
            }
            Statement::Print(expr) => {
                self.compile_expression(expr)?;
                self.emit_opcode(OP_PRINT);
            }
            Statement::If { branches, else_branch } => {
                let mut jump_to_end_positions = Vec::new();

                for (i, (condition, branch)) in branches.iter().enumerate() {
                    self.compile_expression(condition)?;
                    self.emit_opcode(OP_JUMP_IF_FALSE);
                    let jump_to_next_pos = self.emit_u32(0);
                    self.compile_block(branch)?;
                    self.emit_opcode(OP_JUMP);
                    let jump_over_rest_pos = self.emit_u32(0);
                    jump_to_end_positions.push(jump_over_rest_pos);

                    let next_location = self.bytecode.code.len();
                    self.patch_u32(jump_to_next_pos, next_location as u32);

                    if i == branches.len() - 1 {
                        break;
                    }
                }

                if let Some(else_branch) = else_branch {
                    self.compile_block(else_branch)?;
                }

                let end_location = self.bytecode.code.len();
                for pos in jump_to_end_positions {
                    self.patch_u32(pos, end_location as u32);
                }
            }
            Statement::While { condition, body } => {
                // Try to emit combined loop instruction
                if let Some(_) = self.try_compile_combined_while(condition, body)? {
                    return Ok(());
                }

                // Fallback: standard while loop
                let loop_start = self.bytecode.code.len();
                self.compile_expression(condition)?;
                self.emit_opcode(OP_JUMP_IF_FALSE);
                let exit_jump_pos = self.emit_u32(0);
                self.compile_block(body)?;
                self.emit_opcode(OP_JUMP);
                self.emit_u32(loop_start as u32);
                let loop_end = self.bytecode.code.len();
                self.patch_u32(exit_jump_pos, loop_end as u32);
            }
            Statement::FunctionDef { name, params, body } => {
                // Emit JUMP to skip function body
                self.emit_opcode(OP_JUMP);
                let skip_pos = self.emit_u32(0);  // placeholder for jump target
                
                // Record function offset (after the jump instruction)
                let func_offset = self.bytecode.code.len();

                // Compile function body
                self.local_scope.push(HashMap::new());
                self.in_function = true;

                for (i, param) in params.iter().enumerate() {
                    self.local_scope.last_mut().unwrap().insert(
                        param.clone(),
                        i,
                    );
                }

                self.compile_block(body)?;

                // Ensure function returns something
                if !matches!(body.last(), Some(Statement::Return(_))) {
                    self.emit_opcode(OP_NULL);
                    self.emit_opcode(OP_RETURN);
                }

                self.local_scope.pop();
                self.in_function = false;

                // Patch jump to skip over function body
                let after_func = self.bytecode.code.len();
                self.patch_u32(skip_pos, after_func as u32);
                
                // Now create and store function value in global
                let global_idx = self.resolve_global(name);
                self.emit_opcode(OP_FUNCTION_VALUE);
                self.emit_u32(func_offset as u32);
                self.emit_u32(params.len() as u32);
                self.emit_opcode(OP_STORE_GLOBAL);
                self.emit_u32(global_idx as u32);
            }
            Statement::Return(expr) => {
                match expr {
                    Some(e) => self.compile_expression(e)?,
                    None => {
                        self.emit_opcode(OP_NULL);
                    }
                }
                self.emit_opcode(OP_RETURN);
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                self.emit_opcode(OP_POP);
            }
        }

        Ok(())
    }

    fn try_compile_combined_while(
        &mut self,
        condition: &Expression,
        body: &[Statement],
    ) -> Result<Option<()>, String> {
        // Pattern: while (var < const) { var = var + 1 } → OP_LOOP_INC_LESS
        if let Expression::BinaryOp { left, op, right } = condition {
            if *op == BinaryOperator::LessThan {
                if let (Expression::Identifier(cond_var), Expression::Literal(Literal::Number(limit))) =
                    (&**left, &**right)
                {
                    // Check if body is just: set var = var + 1
                    if body.len() == 1 {
                        if let Statement::Assignment { name, value } = &body[0] {
                            if name == cond_var {
                                if let Expression::BinaryOp {
                                    left: body_left,
                                    op: body_op,
                                    right: body_right,
                                } = value
                                {
                                    if *body_op == BinaryOperator::Add {
                                        match (&**body_left, &**body_right) {
                                            (Expression::Identifier(src), Expression::Literal(Literal::Number(n)))
                                            | (Expression::Literal(Literal::Number(n)), Expression::Identifier(src))
                                                if src == cond_var =>
                                            {
                                                if *n == 1 {
                                                    let idx = self.resolve_global(cond_var);
                                                    self.emit_opcode(OP_LOOP_INC_LESS);
                                                    self.emit_u32(idx as u32);
                                                    self.emit_i64(*limit);
                                                    return Ok(Some(()));
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Pattern: while (var < const) { set var = var + N } → OP_LESS_CONST_JUMP_IF_FALSE + OP_ADD_GLOBAL + OP_JUMP
                    // Or any body: while (var < const) { ... } → OP_LESS_CONST_JUMP_IF_FALSE + body + OP_JUMP
                    let idx = self.resolve_global(cond_var);
                    let loop_start = self.bytecode.code.len();
                    self.emit_opcode(OP_LESS_CONST_JUMP_IF_FALSE);
                    self.emit_u32(idx as u32);
                    self.emit_i64(*limit);
                    let exit_pos = self.emit_u32(0); // placeholder for jump target
                    self.compile_block(body)?;
                    self.emit_opcode(OP_JUMP);
                    self.emit_u32(loop_start as u32);
                    let loop_end = self.bytecode.code.len();
                    self.patch_u32(exit_pos, loop_end as u32);
                    return Ok(Some(()));
                }
            }
        }
        Ok(None)
    }

    fn compile_block(&mut self, block: &[Statement]) -> Result<(), String> {
        for statement in block {
            self.compile_statement(statement)?;
        }
        Ok(())
    }

    fn compile_special_assignment(&mut self, name: &str, value: &Expression) -> Result<Option<()>, String> {
        match value {
            Expression::BinaryOp { left, op, right } => {
                match (&**left, &**right) {
                    (Expression::Identifier(src), Expression::Literal(Literal::Number(n)))
                    | (Expression::Literal(Literal::Number(n)), Expression::Identifier(src))
                        if src == name =>
                    {
                        let index = self.resolve_global(name);
                        return match op {
                            BinaryOperator::Add => {
                                if *n == 1 {
                                    self.emit_opcode(OP_INC_GLOBAL);
                                    self.emit_u32(index as u32);
                                } else {
                                    self.emit_opcode(OP_ADD_GLOBAL);
                                    self.emit_u32(index as u32);
                                    self.emit_i64(*n);
                                }
                                Ok(Some(()))
                            }
                            BinaryOperator::Subtract => {
                                if let Expression::Identifier(_) = &**left {
                                    self.emit_opcode(OP_ADD_GLOBAL);
                                    self.emit_u32(index as u32);
                                    self.emit_i64(-*n);
                                    Ok(Some(()))
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
                let val = match literal {
                    Literal::Number(n) => Value::Number(*n),
                    Literal::String(s) => Value::String(Arc::from(s.as_str())),
                    Literal::Boolean(b) => Value::Boolean(*b),
                    Literal::Null => Value::Null,
                };
                self.emit_constant(val);
            }
            Expression::Identifier(name) => {
                // Handle special identifiers
                match name.as_str() {
                    "input" => {
                        // "input" alone is like input()
                        self.emit_opcode(OP_INPUT_STRING);
                    }
                    _ => {
                        let (is_local, index) = self.resolve_identifier(name)?;
                        if is_local {
                            self.emit_opcode(OP_LOAD_LOCAL);
                        } else {
                            self.emit_opcode(OP_LOAD_GLOBAL);
                        }
                        self.emit_u32(index as u32);
                    }
                }
            }
            Expression::Input(input_type) => {
                match input_type {
                    crate::ast::InputType::String => { self.emit_opcode(OP_INPUT_STRING); },
                    crate::ast::InputType::Int => { self.emit_opcode(OP_INPUT_INT); },
                    crate::ast::InputType::Float => { self.emit_opcode(OP_INPUT_FLOAT); },
                };
            }
            Expression::UnaryOp { op, expr } => {
                self.compile_expression(expr)?;
                match op {
                    UnaryOperator::Not => self.emit_opcode(OP_NOT),
                };
            }
            Expression::BinaryOp { left, op, right } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                let opcode = match op {
                    BinaryOperator::Add => OP_ADD,
                    BinaryOperator::Subtract => OP_SUBTRACT,
                    BinaryOperator::Multiply => OP_MULTIPLY,
                    BinaryOperator::Divide => OP_DIVIDE,
                    BinaryOperator::Equals => OP_EQUALS,
                    BinaryOperator::NotEquals => OP_NOT_EQUALS,
                    BinaryOperator::GreaterThan => OP_GREATER,
                    BinaryOperator::LessThan => OP_LESS,
                    BinaryOperator::GreaterOrEqual => OP_GREATER_EQUAL,
                    BinaryOperator::LessOrEqual => OP_LESS_EQUAL,
                    BinaryOperator::And => OP_AND,
                    BinaryOperator::Or => OP_OR,
                };
                self.emit_opcode(opcode);
            }
            Expression::FunctionCall { name, args } => {
                // Handle built-in input functions
                match name.as_str() {
                    "input" => {
                        if !args.is_empty() {
                            return Err("input() takes no arguments".to_string());
                        }
                        self.emit_opcode(OP_INPUT_STRING);
                    }
                    "int" => {
                        if !args.is_empty() {
                            return Err("int() takes no arguments".to_string());
                        }
                        self.emit_opcode(OP_INPUT_INT);
                    }
                    "float" => {
                        if !args.is_empty() {
                            return Err("float() takes no arguments".to_string());
                        }
                        self.emit_opcode(OP_INPUT_FLOAT);
                    }
                    _ => {
                        // Regular function call
                        // Compile arguments onto stack
                        for arg in args {
                            self.compile_expression(arg)?;
                        }
                        
                        // Load function
                        let (is_local, idx) = self.resolve_identifier(name)?;
                        if is_local {
                            self.emit_opcode(OP_LOAD_LOCAL);
                        } else {
                            self.emit_opcode(OP_LOAD_GLOBAL);
                        }
                        self.emit_u32(idx as u32);
                        
                        // Call function
                        self.emit_opcode(OP_CALL_FUNCTION);
                        self.emit_u32(args.len() as u32);
                    }
                }
            }
            Expression::FunctionExpr { params, body } => {
                // Emit JUMP to skip function body
                self.emit_opcode(OP_JUMP);
                let skip_pos = self.emit_u32(0);  // placeholder
                
                let func_offset = self.bytecode.code.len();
                
                // Compile function body
                self.local_scope.push(HashMap::new());
                self.in_function = true;
                
                for (i, param) in params.iter().enumerate() {
                    self.local_scope.last_mut().unwrap().insert(
                        param.clone(),
                        i,
                    );
                }
                
                self.compile_block(body)?;
                
                // Ensure function returns something
                if !matches!(body.last(), Some(Statement::Return(_))) {
                    self.emit_opcode(OP_NULL);
                    self.emit_opcode(OP_RETURN);
                }
                
                self.local_scope.pop();
                self.in_function = false;
                
                // Patch jump to skip function body
                let after_func = self.bytecode.code.len();
                self.patch_u32(skip_pos, after_func as u32);
                
                // Create function value
                self.emit_opcode(OP_FUNCTION_VALUE);
                self.emit_u32(func_offset as u32);
                self.emit_u32(params.len() as u32);
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

    fn resolve_identifier(&mut self, name: &str) -> Result<(bool, usize), String> {
        // Check if identifier is in local scope
        if let Some(locals) = self.local_scope.last() {
            if let Some(&index) = locals.get(name) {
                return Ok((true, index));  // (is_local, index)
            }
        }
        
        // Otherwise, resolve as global
        let index = self.resolve_global(name);
        Ok((false, index))  // (is_local, index)
    }
}
