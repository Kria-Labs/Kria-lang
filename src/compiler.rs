use std::collections::HashMap;
use crate::ast::{Expression, Statement, Literal, BinaryOperator, UnaryOperator};
use crate::bytecode::*;
use crate::vm::Value;
use std::sync::Arc;

struct UpvalueCapture {
    kind: u8,
    index: u32,
}

struct CompileScope {
    locals: HashMap<String, usize>,
    upvalue_names: Vec<String>,
    captures: Vec<UpvalueCapture>,
}

enum VarResolution {
    Local(usize),
    Upvalue(usize),
    Global(usize),
}

pub struct Compiler {
    bytecode: Bytecode,
    globals: HashMap<String, usize>,
    scope_stack: Vec<CompileScope>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            bytecode: Bytecode::new(),
            globals: HashMap::new(),
            scope_stack: Vec::new(),
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

    fn emit_load_var(&mut self, resolved: VarResolution) -> Result<(), String> {
        match resolved {
            VarResolution::Local(index) => {
                self.emit_opcode(OP_LOAD_LOCAL);
                self.emit_u32(index as u32);
            }
            VarResolution::Upvalue(index) => {
                self.emit_opcode(OP_LOAD_UPVALUE);
                self.emit_u32(index as u32);
            }
            VarResolution::Global(index) => {
                self.emit_opcode(OP_LOAD_GLOBAL);
                self.emit_u32(index as u32);
            }
        }
        Ok(())
    }

    fn emit_store_var(&mut self, resolved: VarResolution) -> Result<(), String> {
        match resolved {
            VarResolution::Local(index) => {
                self.emit_opcode(OP_STORE_LOCAL);
                self.emit_u32(index as u32);
            }
            VarResolution::Upvalue(index) => {
                self.emit_opcode(OP_STORE_UPVALUE);
                self.emit_u32(index as u32);
            }
            VarResolution::Global(index) => {
                self.emit_opcode(OP_STORE_GLOBAL);
                self.emit_u32(index as u32);
            }
        }
        Ok(())
    }

    fn emit_make_closure(&mut self, func_offset: u32, num_params: u32, captures: &[UpvalueCapture]) {
        self.emit_opcode(OP_MAKE_CLOSURE);
        self.emit_u32(func_offset);
        self.emit_u32(num_params);
        self.emit_u32(captures.len() as u32);
        for cap in captures {
            self.bytecode.emit_byte(cap.kind);
            self.emit_u32(cap.index);
        }
    }

    fn compile_function(
        &mut self,
        params: &[String],
        body: &[Statement],
    ) -> Result<(u32, u32, Vec<UpvalueCapture>), String> {
        self.emit_opcode(OP_JUMP);
        let skip_pos = self.emit_u32(0);

        let func_offset = self.bytecode.code.len() as u32;

        self.scope_stack.push(CompileScope {
            locals: HashMap::new(),
            upvalue_names: Vec::new(),
            captures: Vec::new(),
        });

        for (i, param) in params.iter().enumerate() {
            self.scope_stack
                .last_mut()
                .unwrap()
                .locals
                .insert(param.clone(), i);
        }

        self.compile_block(body)?;

        if !matches!(body.last(), Some(Statement::Return(_))) {
            self.emit_opcode(OP_NULL);
            self.emit_opcode(OP_RETURN);
        }

        let scope = self.scope_stack.pop().unwrap();
        let captures = scope.captures;

        let after_func = self.bytecode.code.len();
        self.patch_u32(skip_pos, after_func as u32);

        Ok((func_offset, params.len() as u32, captures))
    }

    fn compile_statement(&mut self, statement: &Statement) -> Result<(), String> {
        match statement {
            Statement::Assignment { name, value } => {
                if self.compile_special_assignment(name, value)?.is_some() {
                    return Ok(());
                }
                let resolved = self.resolve_identifier(name)?;
                self.compile_expression(value)?;
                self.emit_store_var(resolved)?;
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
                if let Some(_) = self.try_compile_combined_while(condition, body)? {
                    return Ok(());
                }

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
                let (func_offset, num_params, captures) = self.compile_function(params, body)?;

                let global_idx = self.resolve_global(name);
                self.emit_make_closure(func_offset, num_params, &captures);
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
        if let Expression::BinaryOp { left, op, right } = condition {
            if *op == BinaryOperator::LessThan {
                if let (Expression::Identifier(cond_var), Expression::Literal(Literal::Number(limit))) =
                    (&**left, &**right)
                {
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

                    let idx = self.resolve_global(cond_var);
                    let loop_start = self.bytecode.code.len();
                    self.emit_opcode(OP_LESS_CONST_JUMP_IF_FALSE);
                    self.emit_u32(idx as u32);
                    self.emit_i64(*limit);
                    let exit_pos = self.emit_u32(0);
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
                let resolved = self.resolve_identifier(name)?;
                self.emit_load_var(resolved)?;
            }
            Expression::Input { types, prompt } => {
                self.compile_expression(prompt)?;

                let mut type_mask: u8 = 0;
                for input_type in types {
                    match input_type {
                        crate::ast::InputType::String => type_mask |= 0x01,
                        crate::ast::InputType::Int => type_mask |= 0x02,
                        crate::ast::InputType::Float => type_mask |= 0x04,
                    }
                }

                self.emit_opcode(OP_INPUT);
                self.bytecode.emit_byte(type_mask);
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
                for arg in args {
                    self.compile_expression(arg)?;
                }

                let resolved = self.resolve_identifier(name)?;
                self.emit_load_var(resolved)?;

                self.emit_opcode(OP_CALL_FUNCTION);
                self.emit_u32(args.len() as u32);
            }
            Expression::FunctionExpr { params, body } => {
                let (func_offset, num_params, captures) = self.compile_function(params, body)?;
                self.emit_make_closure(func_offset, num_params, &captures);
            }
        }
        Ok(())
    }

    fn register_upvalue(&mut self, name: &str, kind: u8, index: u32) {
        let scope = self.scope_stack.last_mut().unwrap();
        scope.upvalue_names.push(name.to_string());
        scope.captures.push(UpvalueCapture { kind, index });
    }

    fn resolve_identifier(&mut self, name: &str) -> Result<VarResolution, String> {
        if self.scope_stack.is_empty() {
            return Ok(VarResolution::Global(self.resolve_global(name)));
        }

        let current = self.scope_stack.len() - 1;

        if let Some(&index) = self.scope_stack[current].locals.get(name) {
            return Ok(VarResolution::Local(index));
        }

        if let Some(index) = self
            .scope_stack[current]
            .upvalue_names
            .iter()
            .position(|n| n == name)
        {
            return Ok(VarResolution::Upvalue(index));
        }

        for up in (0..current).rev() {
            if let Some(&local_idx) = self.scope_stack[up].locals.get(name) {
                self.register_upvalue(name, CAPTURE_LOCAL, local_idx as u32);
                let index = self.scope_stack[current].upvalue_names.len() - 1;
                return Ok(VarResolution::Upvalue(index));
            }
            if let Some(uv_idx) = self
                .scope_stack[up]
                .upvalue_names
                .iter()
                .position(|n| n == name)
            {
                self.register_upvalue(name, CAPTURE_UPVALUE, uv_idx as u32);
                let index = self.scope_stack[current].upvalue_names.len() - 1;
                return Ok(VarResolution::Upvalue(index));
            }
        }

        Ok(VarResolution::Global(self.resolve_global(name)))
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
