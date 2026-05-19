use crate::bytecode::*;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    String(Arc<str>),
    Boolean(bool),
    Null,
    Function {
        bytecode_offset: usize,
        num_params: u32,
    },
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Function { bytecode_offset, num_params } => {
                write!(f, "<function at {:x}({} params)>", bytecode_offset, num_params)
            }
        }
    }
}

pub struct CallFrame {
    return_addr: usize,
    locals_start: usize,  // Index where local variables start in stack
    num_locals: u32,
}

pub struct VM {
    stack: Vec<Value>,
    globals: Vec<Value>,
    call_stack: Vec<CallFrame>,
    bytecode: Vec<u8>,  // Keep reference to bytecode for function calls
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::with_capacity(256),
            globals: Vec::new(),
            call_stack: Vec::new(),
            bytecode: Vec::new(),
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    #[inline(always)]
    unsafe fn global_unchecked(&self, idx: usize) -> &Value {
        self.globals.get_unchecked(idx)
    }

    #[inline(always)]
    unsafe fn set_global_unchecked(&mut self, idx: usize, val: Value) {
        *self.globals.get_unchecked_mut(idx) = val;
    }

    pub fn execute(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        let code = &bytecode.code;
        let constants = &bytecode.constants;
        let code_len = code.len();

        // Keep a copy of bytecode for function calls
        self.bytecode = code.clone();

        self.globals.resize(bytecode.num_globals, Value::Null);
        let globals_len = self.globals.len();

        let mut ip: usize = 0;

        while ip < code_len {
            let opcode = unsafe { *code.get_unchecked(ip) };
            ip += 1;

            match opcode {
                OP_RETURN => {
                    if let Some(frame) = self.call_stack.pop() {
                        // Pop return value from stack
                        let return_value = self.pop()?;
                        // Restore previous frame
                        ip = frame.return_addr;
                        // Push return value back onto stack
                        self.stack.push(return_value);
                    } else {
                        // Top-level return, just exit
                        break;
                    }
                }
                OP_LOAD_LOCAL => {
                    let offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    
                    if let Some(frame) = self.call_stack.last() {
                        let idx = frame.locals_start + offset;
                        if idx < self.stack.len() {
                            self.stack.push(self.stack[idx].clone());
                        } else {
                            return Err("Local variable out of bounds".to_string());
                        }
                    } else {
                        return Err("Load local outside function context".to_string());
                    }
                }
                OP_STORE_LOCAL => {
                    let offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let val = self.pop()?;
                    
                    if let Some(frame) = self.call_stack.last() {
                        let idx = frame.locals_start + offset;
                        if idx < self.stack.len() {
                            self.stack[idx] = val;
                        } else {
                            return Err("Local variable out of bounds".to_string());
                        }
                    } else {
                        return Err("Store local outside function context".to_string());
                    }
                }
                OP_DEFINE_FUNCTION => {
                    let global_idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let func_offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let num_params = Bytecode::read_u32(code, ip);
                    ip += 4;
                    
                    let func = Value::Function {
                        bytecode_offset: func_offset,
                        num_params,
                    };
                    debug_assert!(global_idx < globals_len);
                    unsafe { self.set_global_unchecked(global_idx, func); }
                }
                OP_FUNCTION_VALUE => {
                    let func_offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let num_params = Bytecode::read_u32(code, ip);
                    ip += 4;
                    
                    let func = Value::Function {
                        bytecode_offset: func_offset,
                        num_params,
                    };
                    self.stack.push(func);
                }
                OP_CALL_FUNCTION => {
                    let num_args = Bytecode::read_u32(code, ip) as u32;
                    ip += 4;
                    
                    let func = self.pop()?;
                    
                    match func {
                        Value::Function { bytecode_offset, num_params } => {
                            if num_params != num_args {
                                return Err(format!(
                                    "Function expects {} arguments, got {}",
                                    num_params, num_args
                                ));
                            }
                            
                            let locals_start = self.stack.len() - num_args as usize;
                            let frame = CallFrame {
                                return_addr: ip,
                                locals_start,
                                num_locals: num_params,
                            };
                            
                            self.call_stack.push(frame);
                            ip = bytecode_offset;
                        }
                        _ => return Err("Attempted to call non-function value".to_string()),
                    }
                }
                OP_LOOP_INC_LESS => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let limit = Bytecode::read_i64(code, ip);
                    ip += 8;
                    let loop_start = ip - 13;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        if n < limit {
                            unsafe { self.set_global_unchecked(idx, Value::Number(n + 1)); }
                            ip = loop_start;
                        }
                    } else {
                        return Err("LoopIncLess expects number".to_string());
                    }
                }
                OP_LESS_CONST_JUMP_IF_FALSE => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let limit = Bytecode::read_i64(code, ip);
                    ip += 8;
                    let target = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        if n >= limit {
                            ip = target;
                        }
                    } else {
                        return Err("LessConstJumpIfFalse expects number".to_string());
                    }
                }
                OP_ADD_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let rhs = Bytecode::read_i64(code, ip);
                    ip += 8;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        unsafe { self.set_global_unchecked(idx, Value::Number(n + rhs)); }
                    } else {
                        return Err("AddGlobal expects number".to_string());
                    }
                }
                OP_INC_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        unsafe { self.set_global_unchecked(idx, Value::Number(n + 1)); }
                    } else {
                        return Err("IncGlobal expects number".to_string());
                    }
                }
                OP_LOAD_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    debug_assert!(idx < globals_len);
                    self.stack.push(unsafe { self.global_unchecked(idx).clone() });
                }
                OP_STORE_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let val = self.pop()?;
                    debug_assert!(idx < globals_len);
                    unsafe { self.set_global_unchecked(idx, val); }
                }
                OP_JUMP => {
                    ip = Bytecode::read_u32(code, ip) as usize;
                }
                OP_JUMP_IF_FALSE => {
                    let target = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let cond = self.pop()?;
                    match cond {
                        Value::Boolean(b) => {
                            if !b {
                                ip = target;
                            }
                        }
                        _ => return Err("JumpIfFalse expects boolean".to_string()),
                    }
                }
                OP_CONSTANT => {
                    let const_idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    self.stack.push(constants[const_idx].clone());
                }
                OP_NULL => {
                    self.stack.push(Value::Null);
                }
                OP_ADD => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Number(l + r));
                        }
                        (Value::String(l), Value::String(r)) => {
                            let concat: Arc<str> = Arc::from(format!("{}{}", l, r));
                            self.stack.push(Value::String(concat));
                        }
                        (l, r) => return Err(format!(
                            "Add requires two numbers or two strings, found {:?} and {:?}", l, r)),
                    }
                }
                OP_SUBTRACT => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Number(l - r));
                        }
                        (l, r) => return Err(format!(
                            "Subtract requires two numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_MULTIPLY => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Number(l * r));
                        }
                        (l, r) => return Err(format!(
                            "Multiply requires two numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_DIVIDE => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            if r == 0 { return Err("Division by zero".to_string()); }
                            self.stack.push(Value::Number(l / r));
                        }
                        (l, r) => return Err(format!(
                            "Divide requires two numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_EQUALS => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(left == right));
                }
                OP_NOT_EQUALS => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(left != right));
                }
                OP_GREATER => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l > r));
                        }
                        (l, r) => return Err(format!(
                            "Greater requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_LESS => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l < r));
                        }
                        (l, r) => return Err(format!(
                            "Less requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_GREATER_EQUAL => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l >= r));
                        }
                        (l, r) => return Err(format!(
                            "GreaterEqual requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_LESS_EQUAL => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l <= r));
                        }
                        (l, r) => return Err(format!(
                            "LessEqual requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_AND => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Boolean(l), Value::Boolean(r)) => {
                            self.stack.push(Value::Boolean(l && r));
                        }
                        (l, r) => return Err(format!(
                            "And requires booleans, found {:?} and {:?}", l, r)),
                    }
                }
                OP_OR => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Boolean(l), Value::Boolean(r)) => {
                            self.stack.push(Value::Boolean(l || r));
                        }
                        (l, r) => return Err(format!(
                            "Or requires booleans, found {:?} and {:?}", l, r)),
                    }
                }
                OP_NOT => {
                    let operand = self.pop()?;
                    match operand {
                        Value::Boolean(b) => self.stack.push(Value::Boolean(!b)),
                        other => return Err(format!("Not requires boolean, found {:?}", other)),
                    }
                }
                OP_PRINT => {
                    let val = self.pop()?;
                    println!("{}", val);
                }
                OP_POP => {
                    self.pop()?;
                }
                OP_INPUT_STRING => {
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer)
                        .map_err(|e| format!("Input error: {}", e))?;
                    // Remove trailing newline
                    if buffer.ends_with('\n') {
                        buffer.pop();
                        if buffer.ends_with('\r') {
                            buffer.pop();
                        }
                    }
                    self.stack.push(Value::String(Arc::from(buffer)));
                }
                OP_INPUT_INT => {
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer)
                        .map_err(|e| format!("Input error: {}", e))?;
                    let trimmed = buffer.trim();
                    let num = trimmed.parse::<i64>()
                        .map_err(|_| format!("Invalid integer input: '{}'", trimmed))?;
                    self.stack.push(Value::Number(num));
                }
                OP_INPUT_FLOAT => {
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer)
                        .map_err(|e| format!("Input error: {}", e))?;
                    let trimmed = buffer.trim();
                    // Try parsing as float, fallback to int
                    let num = if let Ok(f) = trimmed.parse::<f64>() {
                        f as i64  // Store as i64 (no native float support)
                    } else if let Ok(i) = trimmed.parse::<i64>() {
                        i
                    } else {
                        return Err(format!("Invalid number input: '{}'", trimmed));
                    };
                    self.stack.push(Value::Number(num));
                }
                _ => return Err(format!("Unknown opcode: {}", opcode)),
            }
        }

        Ok(())
    }
}
