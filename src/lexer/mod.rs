//! Lexer module for tokenizing Fortran source code

pub mod token;

pub use token::{Token, TokenType, SourceLocation};

use std::fmt;

/// Lexer error types
#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    UnexpectedCharacter(char, SourceLocation),
    InvalidNumber(String, SourceLocation),
    UnterminatedString(SourceLocation),
    InvalidEscape(char, SourceLocation),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexerError::UnexpectedCharacter(ch, loc) => {
                write!(f, "Unexpected character '{}' at {}", ch, loc)
            }
            LexerError::InvalidNumber(s, loc) => {
                write!(f, "Invalid number '{}' at {}", s, loc)
            }
            LexerError::UnterminatedString(loc) => {
                write!(f, "Unterminated string at {}", loc)
            }
            LexerError::InvalidEscape(ch, loc) => {
                write!(f, "Invalid escape sequence '\\{}' at {}", ch, loc)
            }
        }
    }
}

impl std::error::Error for LexerError {}

pub type LexerResult<T> = Result<T, LexerError>;

/// The Lexer tokenizes Fortran source code
pub struct Lexer {
    source: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    /// Create a new lexer for the given source code
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Tokenize the entire source code
    pub fn tokenize(&mut self) -> LexerResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.token_type == TokenType::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    /// Get the next token from the source
    pub fn next_token(&mut self) -> LexerResult<Token> {
        self.skip_whitespace_and_comments();

        let location = self.current_location();

        if self.is_at_end() {
            return Ok(Token::new(TokenType::Eof, location));
        }

        let ch = self.current_char();

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return self.read_identifier();
        }

        // Numbers
        if ch.is_ascii_digit() {
            return self.read_number();
        }

        // String literals
        if ch == '\'' || ch == '"' {
            return self.read_string();
        }

        // Operators and punctuation
        self.read_operator_or_punctuation()
    }

    // Helper methods

    fn current_char(&self) -> char {
        self.source[self.position]
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.source.get(self.position + offset).copied()
    }

    fn advance(&mut self) -> char {
        let ch = self.current_char();
        self.position += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.source.len()
    }

    fn current_location(&self) -> SourceLocation {
        SourceLocation {
            line: self.line,
            column: self.column,
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            let ch = self.current_char();

            // Whitespace (but not newlines in Fortran - they're significant)
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
                continue;
            }

            // Newlines
            if ch == '\n' {
                self.advance();
                continue;
            }

            // Comments (! to end of line)
            if ch == '!' {
                // Skip until newline
                while !self.is_at_end() && self.current_char() != '\n' {
                    self.advance();
                }
                continue;
            }

            // Line continuation (&)
            if ch == '&' {
                self.advance();
                self.skip_whitespace_and_comments();
                continue;
            }

            break;
        }
    }

    fn read_identifier(&mut self) -> LexerResult<Token> {
        let location = self.current_location();
        let mut ident = String::new();

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.advance());
            } else {
                break;
            }
        }

        // Check for logical operators (.TRUE., .FALSE., .AND., etc.)
        if ident.starts_with('.') && ident.ends_with('.') {
            let token_type = Self::keyword_or_operator(&ident.to_uppercase());
            return Ok(Token::new(token_type, location));
        }

        // Check if it's a keyword (case-insensitive)
        let token_type = Self::keyword_or_identifier(&ident.to_uppercase());

        Ok(Token::new(token_type, location))
    }

    fn keyword_or_identifier(ident: &str) -> TokenType {
        match ident {
            "PROGRAM" => TokenType::Program,
            "END" => TokenType::End,
            "SUBROUTINE" => TokenType::Subroutine,
            "FUNCTION" => TokenType::Function,
            "RETURN" => TokenType::Return,
            "IF" => TokenType::If,
            "THEN" => TokenType::Then,
            "ELSE" => TokenType::Else,
            "ELSEIF" => TokenType::ElseIf,
            "DO" => TokenType::Do,
            "WHILE" => TokenType::While,
            "CYCLE" => TokenType::Cycle,
            "EXIT" => TokenType::Exit,
            "CONTINUE" => TokenType::Continue,
            "SELECT" => TokenType::Select,
            "CASE" => TokenType::Case,
            "DEFAULT" => TokenType::Default,
            "MODULE" => TokenType::Module,
            "USE" => TokenType::Use,
            "ONLY" => TokenType::Only,
            "IMPLICIT" => TokenType::Implicit,
            "NONE" => TokenType::None,
            "INTEGER" => TokenType::Integer,
            "REAL" => TokenType::Real,
            "DOUBLE" => TokenType::Double,
            "PRECISION" => TokenType::Precision,
            "COMPLEX" => TokenType::Complex,
            "LOGICAL" => TokenType::Logical,
            "CHARACTER" => TokenType::Character,
            "DIMENSION" => TokenType::Dimension,
            "PARAMETER" => TokenType::Parameter,
            "ALLOCATABLE" => TokenType::Allocatable,
            "ALLOCATE" => TokenType::Allocate,
            "DEALLOCATE" => TokenType::Deallocate,
            "POINTER" => TokenType::Pointer,
            "TARGET" => TokenType::Target,
            "INTENT" => TokenType::Intent,
            "IN" => TokenType::In,
            "OUT" => TokenType::Out,
            "INOUT" => TokenType::InOut,
            "CALL" => TokenType::Call,
            "PRINT" => TokenType::Print,
            "READ" => TokenType::Read,
            "WRITE" => TokenType::Write,
            "OPEN" => TokenType::Open,
            "CLOSE" => TokenType::Close,
            "FORMAT" => TokenType::Format,
            "STOP" => TokenType::Stop,
            "TYPE" => TokenType::Type,
            "CLASS" => TokenType::Class,
            "EXTENDS" => TokenType::Extends,
            "CONTAINS" => TokenType::Contains,
            "PROCEDURE" => TokenType::Procedure,
            "INTERFACE" => TokenType::Interface,
            "OPERATOR" => TokenType::Operator,
            "ASSIGNMENT" => TokenType::Assignment,
            "PUBLIC" => TokenType::Public,
            "PRIVATE" => TokenType::Private,
            "SAVE" => TokenType::Save,
            "RESULT" => TokenType::Result,
            "RECURSIVE" => TokenType::Recursive,
            "CONCURRENT" => TokenType::Concurrent,
            "SYNC" => TokenType::Sync,
            "ALL" => TokenType::All,
            "IMAGES" => TokenType::Images,
            "CRITICAL" => TokenType::Critical,
            "WHERE" => TokenType::Where,
            "ELSEWHERE" => TokenType::Elsewhere,
            "FORALL" => TokenType::Forall,
            "LEN" => TokenType::Len,
            "KIND" => TokenType::Kind,
            "STAT" => TokenType::Stat,
            "OPTIONAL" => TokenType::Optional,
            "UNIT" => TokenType::Unit,
            "FILE" => TokenType::File,
            "STATUS" => TokenType::Status,
            "ACTION" => TokenType::Action,
            "IOSTAT" => TokenType::Iostat,
            "IOMSG" => TokenType::Iomsg,
            _ => TokenType::Identifier(ident.to_string()),
        }
    }

    fn keyword_or_operator(ident: &str) -> TokenType {
        match ident {
            ".TRUE." => TokenType::True,
            ".FALSE." => TokenType::False,
            ".AND." => TokenType::And,
            ".OR." => TokenType::Or,
            ".NOT." => TokenType::Not,
            ".EQ." => TokenType::Eq,
            ".NE." => TokenType::Ne,
            ".LT." => TokenType::Lt,
            ".LE." => TokenType::Le,
            ".GT." => TokenType::Gt,
            ".GE." => TokenType::Ge,
            ".EQV." => TokenType::Eqv,
            ".NEQV." => TokenType::Neqv,
            _ => TokenType::Identifier(ident.to_string()),
        }
    }

    fn read_number(&mut self) -> LexerResult<Token> {
        let location = self.current_location();
        let mut number = String::new();

        // Read integer part
        while !self.is_at_end() && self.current_char().is_ascii_digit() {
            number.push(self.advance());
        }

        // Check for real number (decimal point or exponent)
        let mut is_real = false;

        // Decimal point
        if !self.is_at_end() && self.current_char() == '.' {
            // Check if it's really a decimal (not .. operator or .and., etc.)
            if let Some(next) = self.peek_char(1) {
                if next.is_ascii_digit() {
                    is_real = true;
                    number.push(self.advance()); // consume '.'
                    while !self.is_at_end() && self.current_char().is_ascii_digit() {
                        number.push(self.advance());
                    }
                }
            }
        }

        // Exponent (E or D for double precision)
        if !self.is_at_end() {
            let ch = self.current_char();
            if ch == 'E' || ch == 'e' || ch == 'D' || ch == 'd' {
                is_real = true;
                number.push(self.advance());

                // Optional sign
                if !self.is_at_end() {
                    let sign = self.current_char();
                    if sign == '+' || sign == '-' {
                        number.push(self.advance());
                    }
                }

                // Exponent digits
                while !self.is_at_end() && self.current_char().is_ascii_digit() {
                    number.push(self.advance());
                }
            }
        }

        // Kind specifier (_kind)
        if !self.is_at_end() && self.current_char() == '_' {
            number.push(self.advance());
            while !self.is_at_end() && self.current_char().is_alphanumeric() {
                number.push(self.advance());
            }
        }

        let token_type = if is_real {
            TokenType::RealLiteral(number.clone())
        } else {
            TokenType::IntegerLiteral(number.clone())
        };

        Ok(Token::new(token_type, location))
    }

    fn read_string(&mut self) -> LexerResult<Token> {
        let location = self.current_location();
        let quote = self.advance(); // consume opening quote
        let mut string = String::new();

        while !self.is_at_end() {
            let ch = self.current_char();

            if ch == quote {
                // Check for doubled quote (Fortran escape)
                if let Some(next) = self.peek_char(1) {
                    if next == quote {
                        string.push(quote);
                        self.advance();
                        self.advance();
                        continue;
                    }
                }
                // End of string
                self.advance();
                return Ok(Token::new(TokenType::StringLiteral(string), location));
            }

            if ch == '\n' {
                return Err(LexerError::UnterminatedString(location));
            }

            string.push(self.advance());
        }

        Err(LexerError::UnterminatedString(location))
    }

    fn read_operator_or_punctuation(&mut self) -> LexerResult<Token> {
        let location = self.current_location();
        let ch = self.advance();

        let token_type = match ch {
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            ',' => TokenType::Comma,
            ';' => TokenType::Semicolon,
            ':' => {
                if !self.is_at_end() && self.current_char() == ':' {
                    self.advance();
                    TokenType::DoubleColon
                } else {
                    TokenType::Colon
                }
            }
            '+' => TokenType::Plus,
            '-' => TokenType::Minus,
            '*' => {
                if !self.is_at_end() && self.current_char() == '*' {
                    self.advance();
                    TokenType::Power
                } else {
                    TokenType::Star
                }
            }
            '/' => {
                if !self.is_at_end() && self.current_char() == '=' {
                    self.advance();
                    TokenType::NotEqual
                } else {
                    TokenType::Slash
                }
            }
            '=' => {
                if !self.is_at_end() && self.current_char() == '=' {
                    self.advance();
                    TokenType::EqualEqual
                } else if !self.is_at_end() && self.current_char() == '>' {
                    self.advance();
                    TokenType::Arrow
                } else {
                    TokenType::Equal
                }
            }
            '<' => {
                if !self.is_at_end() && self.current_char() == '=' {
                    self.advance();
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                }
            }
            '>' => {
                if !self.is_at_end() && self.current_char() == '=' {
                    self.advance();
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                }
            }
            '%' => TokenType::Percent,
            '.' => {
                // Check for logical operators
                let mut operator = String::from(".");
                let start_pos = self.position;

                while !self.is_at_end() {
                    let c = self.current_char();
                    if c.is_alphabetic() {
                        operator.push(self.advance());
                    } else if c == '.' {
                        operator.push(self.advance());
                        break;
                    } else {
                        break;
                    }
                }

                if operator.starts_with('.') && operator.ends_with('.') {
                    Self::keyword_or_operator(&operator.to_uppercase())
                } else {
                    // Not a complete operator, reset
                    self.position = start_pos;
                    self.column -= operator.len() - 1;
                    TokenType::Dot
                }
            }
            _ => {
                return Err(LexerError::UnexpectedCharacter(ch, location));
            }
        };

        Ok(Token::new(token_type, location))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_source() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_simple_keywords() {
        let mut lexer = Lexer::new("PROGRAM END IF THEN");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Program);
        assert_eq!(tokens[1].token_type, TokenType::End);
        assert_eq!(tokens[2].token_type, TokenType::If);
        assert_eq!(tokens[3].token_type, TokenType::Then);
    }

    #[test]
    fn test_case_insensitive() {
        let mut lexer = Lexer::new("program Program PROGRAM");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Program);
        assert_eq!(tokens[1].token_type, TokenType::Program);
        assert_eq!(tokens[2].token_type, TokenType::Program);
    }

    #[test]
    fn test_identifier() {
        let mut lexer = Lexer::new("x my_var var123");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].token_type, TokenType::Identifier(_)));
        assert!(matches!(tokens[1].token_type, TokenType::Identifier(_)));
        assert!(matches!(tokens[2].token_type, TokenType::Identifier(_)));
    }

    #[test]
    fn test_integer_literals() {
        let mut lexer = Lexer::new("0 123 456789");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].token_type, TokenType::IntegerLiteral(_)));
        assert!(matches!(tokens[1].token_type, TokenType::IntegerLiteral(_)));
        assert!(matches!(tokens[2].token_type, TokenType::IntegerLiteral(_)));
    }

    #[test]
    fn test_real_literals() {
        let mut lexer = Lexer::new("1.0 3.14 2.5E10 1.0D-5");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].token_type, TokenType::RealLiteral(_)));
        assert!(matches!(tokens[1].token_type, TokenType::RealLiteral(_)));
        assert!(matches!(tokens[2].token_type, TokenType::RealLiteral(_)));
        assert!(matches!(tokens[3].token_type, TokenType::RealLiteral(_)));
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new(r#"'hello' "world""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(
            &tokens[0].token_type,
            TokenType::StringLiteral(s) if s == "hello"
        ));
        assert!(matches!(
            &tokens[1].token_type,
            TokenType::StringLiteral(s) if s == "world"
        ));
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / ** == /= < <= > >=");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Plus);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].token_type, TokenType::Star);
        assert_eq!(tokens[3].token_type, TokenType::Slash);
        assert_eq!(tokens[4].token_type, TokenType::Power);
        assert_eq!(tokens[5].token_type, TokenType::EqualEqual);
        assert_eq!(tokens[6].token_type, TokenType::NotEqual);
        assert_eq!(tokens[7].token_type, TokenType::Less);
        assert_eq!(tokens[8].token_type, TokenType::LessEqual);
        assert_eq!(tokens[9].token_type, TokenType::Greater);
        assert_eq!(tokens[10].token_type, TokenType::GreaterEqual);
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = Lexer::new(".TRUE. .FALSE. .AND. .OR. .NOT.");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::True);
        assert_eq!(tokens[1].token_type, TokenType::False);
        assert_eq!(tokens[2].token_type, TokenType::And);
        assert_eq!(tokens[3].token_type, TokenType::Or);
        assert_eq!(tokens[4].token_type, TokenType::Not);
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("x ! this is a comment\ny");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].token_type, TokenType::Identifier(_)));
        assert!(matches!(tokens[1].token_type, TokenType::Identifier(_)));
        assert_eq!(tokens.len(), 3); // x, y, EOF
    }
}
