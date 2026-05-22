use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Set,
    If,
    Else,
    ElseIf,
    While,
    Fn,
    Return,
    True,
    False,
    Null,
    Print,
    
    // Identifiers and Literals
    Identifier(String),
    Number(i64),
    String(String),
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    EqualEqual,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    And,
    Or,
    Not,
    Pipe,
    
    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Newline,
    
    // Special
    Eof,
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    current: Option<char>,
    peeked: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars().peekable();
        let current = chars.next();
        let peeked = chars.peek().copied();
        Lexer {
            input: chars,
            current,
            peeked,
        }
    }
    
    fn current_char(&self) -> Option<char> {
        self.current
    }
    
    fn peek_char(&self) -> Option<char> {
        self.peeked
    }
    
    fn advance(&mut self) {
        self.current = self.input.next();
        self.peeked = self.input.peek().copied();
    }
    
    fn skip_spaces(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    fn read_string(&mut self) -> String {
        self.advance(); // Skip opening quote
        let mut result = String::new();
        
        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance();
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        _ => {
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                    self.advance();
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }
        
        result
    }
    
    fn read_number(&mut self) -> i64 {
        let mut result = String::new();
        
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        result.parse::<i64>().unwrap_or(0)
    }
    
    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        
        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        result
    }
    
    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_spaces();

            match self.current_char() {
                None => return Token::Eof,
                Some(ch) => {
                    match ch {
                        '+' => {
                            self.advance();
                            return Token::Plus;
                        }
                        '-' => {
                            self.advance();
                            return Token::Minus;
                        }
                        '*' => {
                            self.advance();
                            return Token::Star;
                        }
                        '/' => {
                            if self.peek_char() == Some('/') {
                                self.advance();
                                self.advance();
                                while let Some(ch) = self.current_char() {
                                    if ch == '\n' {
                                        break;
                                    }
                                    self.advance();
                                }
                                continue;
                            } else {
                                self.advance();
                                return Token::Slash;
                            }
                        }
                        '(' => {
                            self.advance();
                            return Token::LParen;
                        }
                        ')' => {
                            self.advance();
                            return Token::RParen;
                        }
                        '=' => {
                            if self.peek_char() == Some('=') {
                                self.advance();
                                self.advance();
                                return Token::EqualEqual;
                            } else {
                                self.advance();
                                return Token::Equal;
                            }
                        }
                        '!' => {
                            self.advance();
                            if self.current_char() == Some('=') {
                                self.advance();
                                return Token::NotEqual;
                            } else {
                                return Token::Not;
                            }
                        }
                        '>' => {
                            self.advance();
                            if self.current_char() == Some('=') {
                                self.advance();
                                return Token::GreaterEqual;
                            } else {
                                return Token::Greater;
                            }
                        }
                        '<' => {
                            self.advance();
                            if self.current_char() == Some('=') {
                                self.advance();
                                return Token::LessEqual;
                            } else {
                                return Token::Less;
                            }
                        }
                        '{' => {
                            self.advance();
                            return Token::LBrace;
                        }
                        '}' => {
                            self.advance();
                            return Token::RBrace;
                        }
                        ',' => {
                            self.advance();
                            return Token::Comma;
                        }
                        '|' => {
                            self.advance();
                            return Token::Pipe;
                        }
                        '\n' => {
                            self.advance();
                            return Token::Newline;
                        }
                        '"' => return Token::String(self.read_string()),
                        _ if ch.is_ascii_digit() => return Token::Number(self.read_number()),
                        _ if ch.is_alphabetic() || ch == '_' => {
                            let ident = self.read_identifier();
                            return match ident.as_str() {
                                "set" => Token::Set,
                                "if" => Token::If,
                                "else" => Token::Else,
                                "elseif" => Token::ElseIf,
                                "while" => Token::While,
                                "fn" => Token::Fn,
                                "return" => Token::Return,
                                "true" => Token::True,
                                "false" => Token::False,
                                "null" => Token::Null,
                                "print" => Token::Print,
                                "and" => Token::And,
                                "or" => Token::Or,
                                "not" => Token::Not,
                                _ => Token::Identifier(ident),
                            };
                        }
                        _ => {
                            self.advance();
                            continue;
                        }
                    }
                }
            }
        }
    }
    
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token();
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        
        tokens
    }
}