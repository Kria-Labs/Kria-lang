use crate::vm::Value;

pub const OP_CONSTANT: u8 = 0;
pub const OP_LOAD_GLOBAL: u8 = 1;
pub const OP_STORE_GLOBAL: u8 = 2;
pub const OP_ADD: u8 = 3;
pub const OP_SUBTRACT: u8 = 4;
pub const OP_MULTIPLY: u8 = 5;
pub const OP_DIVIDE: u8 = 6;
pub const OP_EQUALS: u8 = 7;
pub const OP_NOT_EQUALS: u8 = 8;
pub const OP_GREATER: u8 = 9;
pub const OP_LESS: u8 = 10;
pub const OP_GREATER_EQUAL: u8 = 11;
pub const OP_LESS_EQUAL: u8 = 12;
pub const OP_AND: u8 = 13;
pub const OP_OR: u8 = 14;
pub const OP_NOT: u8 = 15;
pub const OP_PRINT: u8 = 16;
pub const OP_POP: u8 = 17;
pub const OP_JUMP: u8 = 18;
pub const OP_JUMP_IF_FALSE: u8 = 19;
pub const OP_INC_GLOBAL: u8 = 20;
pub const OP_ADD_GLOBAL: u8 = 21;
// Combined instructions for loop hot path
pub const OP_LESS_CONST_JUMP_IF_FALSE: u8 = 22; // LoadGlobal + Constant + LessThan + JumpIfFalse
pub const OP_LOOP_INC_LESS: u8 = 23;            // IncGlobal + Jump (full loop tick)
pub const OP_NULL: u8 = 24;
pub const OP_DEFINE_FUNCTION: u8 = 25;          // Define function: global_idx, bytecode_offset, num_params
pub const OP_FUNCTION_VALUE: u8 = 26;           // Create function value: bytecode_offset, num_params
pub const OP_CALL_FUNCTION: u8 = 27;            // Call function: num_args
pub const OP_RETURN: u8 = 28;                    // Return from function
pub const OP_LOAD_LOCAL: u8 = 29;               // Load local variable: frame_offset
pub const OP_STORE_LOCAL: u8 = 30;              // Store local variable: frame_offset
pub const OP_INPUT_STRING: u8 = 31;             // Read string from stdin
pub const OP_INPUT_INT: u8 = 32;                // Read integer from stdin
pub const OP_INPUT_FLOAT: u8 = 33;              // Read float from stdin

pub struct Bytecode {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub num_globals: usize,
}

impl Bytecode {
    pub fn new() -> Self {
        Bytecode {
            code: Vec::new(),
            constants: Vec::new(),
            num_globals: 0,
        }
    }

    pub fn emit_byte(&mut self, byte: u8) -> usize {
        let pos = self.code.len();
        self.code.push(byte);
        pos
    }

    pub fn emit_u32(&mut self, value: u32) -> usize {
        let pos = self.code.len();
        self.code.extend_from_slice(&value.to_le_bytes());
        pos
    }

    pub fn emit_i64(&mut self, value: i64) -> usize {
        let pos = self.code.len();
        self.code.extend_from_slice(&value.to_le_bytes());
        pos
    }

    #[inline(always)]
    pub fn read_u32(code: &[u8], ip: usize) -> u32 {
        u32::from_le_bytes([code[ip], code[ip+1], code[ip+2], code[ip+3]])
    }

    #[inline(always)]
    pub fn read_i64(code: &[u8], ip: usize) -> i64 {
        i64::from_le_bytes([
            code[ip], code[ip+1], code[ip+2], code[ip+3],
            code[ip+4], code[ip+5], code[ip+6], code[ip+7],
        ])
    }

    pub fn patch_u32(&mut self, pos: usize, value: u32) {
        let bytes = value.to_le_bytes();
        self.code[pos] = bytes[0];
        self.code[pos+1] = bytes[1];
        self.code[pos+2] = bytes[2];
        self.code[pos+3] = bytes[3];
    }

    pub fn add_constant(&mut self, value: Value) -> u32 {
        for (i, c) in self.constants.iter().enumerate() {
            if *c == value { return i as u32; }
        }
        let idx = self.constants.len() as u32;
        self.constants.push(value);
        idx
    }
}
