//! Syntax highlighting for Fortran code in the REPL

use crate::lexer::{Lexer, TokenType};

/// ANSI color codes
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    
    // Keywords - bold blue
    pub const KEYWORD: &str = "\x1b[1;34m";
    
    // Types - bold cyan
    pub const TYPE: &str = "\x1b[1;36m";
    
    // Control flow - bold magenta
    pub const CONTROL: &str = "\x1b[1;35m";
    
    // Operators - yellow
    pub const OPERATOR: &str = "\x1b[33m";
    
    // Numbers - green
    pub const NUMBER: &str = "\x1b[32m";
    
    // Strings - bright green
    pub const STRING: &str = "\x1b[92m";
    
    // Booleans - cyan
    pub const BOOLEAN: &str = "\x1b[36m";
    
    // Comments - gray/dim
    pub const COMMENT: &str = "\x1b[90m";
    
    // Punctuation - dim white
    pub const PUNCT: &str = "\x1b[37m";
    
    // Identifiers - default
    pub const IDENT: &str = "\x1b[0m";
    
    // Procedures (function/subroutine names) - bold yellow
    pub const PROCEDURE: &str = "\x1b[1;33m";
}

/// Get the color for a token type
fn token_color(token: &TokenType) -> &'static str {
    use TokenType::*;
    match token {
        // Control flow keywords - magenta
        Program | End | If | Then | Else | ElseIf | Do | While | Cycle | Exit |
        Continue | Select | Case | Default | Return | Call | Contains | Where |
        Elsewhere | Forall | Associate | Block | Stop => colors::CONTROL,
        
        // Procedure keywords - blue
        Subroutine | Function | Result | Recursive | Pure | Elemental |
        Module | Use | Only | Interface | Operator | Assignment | Procedure => colors::KEYWORD,
        
        // Type keywords - cyan
        Integer | Real | Double | Precision | Complex | Logical | Character |
        Type | Class | Dimension | Allocatable | Allocate | Deallocate |
        Pointer | Target | Implicit | None | Parameter | Save | Len | Kind => colors::TYPE,
        
        // Intent/visibility keywords - cyan
        Intent | In | Out | InOut | Public | Private | Optional | Extends |
        Present | Stat => colors::TYPE,
        
        // I/O keywords - blue
        Print | Read | Write | Open | Close | Format | Unit | File | Status |
        Action | Iostat | Iomsg => colors::KEYWORD,
        
        // Parallel keywords - blue
        Concurrent | Sync | All | Images | Critical => colors::KEYWORD,
        
        // Literals
        IntegerLiteral(_) | RealLiteral(_) => colors::NUMBER,
        StringLiteral(_) => colors::STRING,
        True | False => colors::BOOLEAN,
        
        // Operators
        Plus | Minus | Star | Slash | Power |
        Equal | EqualEqual | NotEqual | Less | LessEqual | Greater | GreaterEqual |
        And | Or | Not | Eq | Ne | Lt | Le | Gt | Ge | Eqv | Neqv => colors::OPERATOR,
        
        // Delimiters
        LeftParen | RightParen | LeftBracket | RightBracket |
        Comma | Colon | DoubleColon | Semicolon | Dot | Percent | Arrow => colors::PUNCT,
        
        // Identifiers - default color
        Identifier(_) => colors::IDENT,
        
        // EOF - no color
        Eof => colors::RESET,
    }
}

/// Highlight Fortran source code with ANSI colors
pub fn highlight(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return source.to_string(), // Return unhighlighted on error
    };
    
    if tokens.is_empty() {
        return source.to_string();
    }
    
    let mut result = String::with_capacity(source.len() * 2);
    let chars: Vec<char> = source.chars().collect();
    let mut pos = 0;
    
    for token in &tokens {
        let token_start = token.location.column.saturating_sub(1);
        
        // Add any characters before this token (whitespace, comments)
        while pos < token_start && pos < chars.len() {
            let ch = chars[pos];
            // Check for comment start
            if ch == '!' {
                result.push_str(colors::COMMENT);
                // Add rest of line as comment
                while pos < chars.len() && chars[pos] != '\n' {
                    result.push(chars[pos]);
                    pos += 1;
                }
                result.push_str(colors::RESET);
                continue;
            }
            result.push(ch);
            pos += 1;
        }
        
        // Add the token with color
        let color = token_color(&token.token_type);
        result.push_str(color);
        
        let token_str = token_to_string(&token.token_type, source, pos);
        let token_len = token_str.len();
        result.push_str(&token_str);
        result.push_str(colors::RESET);
        
        pos += token_len;
    }
    
    // Add any remaining characters
    while pos < chars.len() {
        let ch = chars[pos];
        if ch == '!' {
            result.push_str(colors::COMMENT);
            while pos < chars.len() && chars[pos] != '\n' {
                result.push(chars[pos]);
                pos += 1;
            }
            result.push_str(colors::RESET);
            continue;
        }
        result.push(ch);
        pos += 1;
    }
    
    result
}

/// Convert a token type to its string representation
fn token_to_string(token: &TokenType, source: &str, start_pos: usize) -> String {
    use TokenType::*;
    match token {
        // Literals with values
        IntegerLiteral(s) | RealLiteral(s) | StringLiteral(s) | Identifier(s) => s.clone(),
        
        // Keywords - extract from source to preserve case
        Program => extract_word(source, start_pos, 7),
        End => extract_word(source, start_pos, 3),
        If => extract_word(source, start_pos, 2),
        Then => extract_word(source, start_pos, 4),
        Else => extract_word(source, start_pos, 4),
        ElseIf => extract_word(source, start_pos, 6), // Could be "ELSEIF" or "ELSE IF"
        Do => extract_word(source, start_pos, 2),
        While => extract_word(source, start_pos, 5),
        Cycle => extract_word(source, start_pos, 5),
        Exit => extract_word(source, start_pos, 4),
        Continue => extract_word(source, start_pos, 8),
        Select => extract_word(source, start_pos, 6),
        Case => extract_word(source, start_pos, 4),
        Default => extract_word(source, start_pos, 7),
        Subroutine => extract_word(source, start_pos, 10),
        Function => extract_word(source, start_pos, 8),
        Return => extract_word(source, start_pos, 6),
        Call => extract_word(source, start_pos, 4),
        Contains => extract_word(source, start_pos, 8),
        Result => extract_word(source, start_pos, 6),
        Recursive => extract_word(source, start_pos, 9),
        Module => extract_word(source, start_pos, 6),
        Use => extract_word(source, start_pos, 3),
        Only => extract_word(source, start_pos, 4),
        Public => extract_word(source, start_pos, 6),
        Private => extract_word(source, start_pos, 7),
        Implicit => extract_word(source, start_pos, 8),
        None => extract_word(source, start_pos, 4),
        Integer => extract_word(source, start_pos, 7),
        Real => extract_word(source, start_pos, 4),
        Double => extract_word(source, start_pos, 6),
        Precision => extract_word(source, start_pos, 9),
        Complex => extract_word(source, start_pos, 7),
        Logical => extract_word(source, start_pos, 7),
        Character => extract_word(source, start_pos, 9),
        Type => extract_word(source, start_pos, 4),
        Class => extract_word(source, start_pos, 5),
        Dimension => extract_word(source, start_pos, 9),
        Parameter => extract_word(source, start_pos, 9),
        Allocatable => extract_word(source, start_pos, 11),
        Allocate => extract_word(source, start_pos, 8),
        Deallocate => extract_word(source, start_pos, 10),
        Pointer => extract_word(source, start_pos, 7),
        Target => extract_word(source, start_pos, 6),
        Intent => extract_word(source, start_pos, 6),
        In => extract_word(source, start_pos, 2),
        Out => extract_word(source, start_pos, 3),
        InOut => extract_word(source, start_pos, 5),
        Save => extract_word(source, start_pos, 4),
        Len => extract_word(source, start_pos, 3),
        Kind => extract_word(source, start_pos, 4),
        Stat => extract_word(source, start_pos, 4),
        Optional => extract_word(source, start_pos, 8),
        Print => extract_word(source, start_pos, 5),
        Read => extract_word(source, start_pos, 4),
        Write => extract_word(source, start_pos, 5),
        Open => extract_word(source, start_pos, 4),
        Close => extract_word(source, start_pos, 5),
        Format => extract_word(source, start_pos, 6),
        Stop => extract_word(source, start_pos, 4),
        Unit => extract_word(source, start_pos, 4),
        File => extract_word(source, start_pos, 4),
        Status => extract_word(source, start_pos, 6),
        Action => extract_word(source, start_pos, 6),
        Iostat => extract_word(source, start_pos, 6),
        Iomsg => extract_word(source, start_pos, 5),
        Extends => extract_word(source, start_pos, 7),
        Procedure => extract_word(source, start_pos, 9),
        Interface => extract_word(source, start_pos, 9),
        Operator => extract_word(source, start_pos, 8),
        Assignment => extract_word(source, start_pos, 10),
        Concurrent => extract_word(source, start_pos, 10),
        Sync => extract_word(source, start_pos, 4),
        All => extract_word(source, start_pos, 3),
        Images => extract_word(source, start_pos, 6),
        Critical => extract_word(source, start_pos, 8),
        Where => extract_word(source, start_pos, 5),
        Elsewhere => extract_word(source, start_pos, 9),
        Forall => extract_word(source, start_pos, 6),
        Associate => extract_word(source, start_pos, 9),
        Block => extract_word(source, start_pos, 5),
        Present => extract_word(source, start_pos, 7),
        Pure => extract_word(source, start_pos, 4),
        Elemental => extract_word(source, start_pos, 9),
        True => extract_word(source, start_pos, 5),  // .TRUE.
        False => extract_word(source, start_pos, 6), // .FALSE.
        
        // Operators
        Plus => "+".to_string(),
        Minus => "-".to_string(),
        Star => "*".to_string(),
        Slash => "/".to_string(),
        Power => "**".to_string(),
        Equal => "=".to_string(),
        EqualEqual => "==".to_string(),
        NotEqual => "/=".to_string(),
        Less => "<".to_string(),
        LessEqual => "<=".to_string(),
        Greater => ">".to_string(),
        GreaterEqual => ">=".to_string(),
        And => extract_word(source, start_pos, 5),  // .AND.
        Or => extract_word(source, start_pos, 4),   // .OR.
        Not => extract_word(source, start_pos, 5),  // .NOT.
        Eq => extract_word(source, start_pos, 4),   // .EQ.
        Ne => extract_word(source, start_pos, 4),   // .NE.
        Lt => extract_word(source, start_pos, 4),   // .LT.
        Le => extract_word(source, start_pos, 4),   // .LE.
        Gt => extract_word(source, start_pos, 4),   // .GT.
        Ge => extract_word(source, start_pos, 4),   // .GE.
        Eqv => extract_word(source, start_pos, 5),  // .EQV.
        Neqv => extract_word(source, start_pos, 6), // .NEQV.
        
        // Delimiters
        LeftParen => "(".to_string(),
        RightParen => ")".to_string(),
        LeftBracket => "[".to_string(),
        RightBracket => "]".to_string(),
        Comma => ",".to_string(),
        Colon => ":".to_string(),
        DoubleColon => "::".to_string(),
        Semicolon => ";".to_string(),
        Dot => ".".to_string(),
        Percent => "%".to_string(),
        Arrow => "=>".to_string(),
        
        Eof => String::new(),
    }
}

/// Extract a word from source at position with expected length
fn extract_word(source: &str, start: usize, expected_len: usize) -> String {
    let chars: Vec<char> = source.chars().collect();
    if start >= chars.len() {
        return String::new();
    }
    
    let end = (start + expected_len).min(chars.len());
    chars[start..end].iter().collect()
}

/// Highlight a single line for REPL display (simpler, more robust)
pub fn highlight_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len() * 2);
    let upper = line.to_uppercase();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let ch = chars[i];
        
        // Comment - rest of line is gray
        if ch == '!' {
            result.push_str(colors::COMMENT);
            while i < chars.len() {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str(colors::RESET);
            continue;
        }
        
        // String literal
        if ch == '\'' || ch == '"' {
            let quote = ch;
            result.push_str(colors::STRING);
            result.push(ch);
            i += 1;
            while i < chars.len() && chars[i] != quote {
                result.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str(colors::RESET);
            continue;
        }
        
        // Number
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            result.push_str(colors::NUMBER);
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || 
                   chars[i] == 'e' || chars[i] == 'E' || chars[i] == 'd' || chars[i] == 'D' ||
                   chars[i] == '+' || chars[i] == '-' || chars[i] == '_') {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str(colors::RESET);
            continue;
        }
        
        // Word (keyword or identifier)
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let word_upper: String = upper.chars().skip(start).take(i - start).collect();
            
            let color = get_word_color(&word_upper);
            result.push_str(color);
            result.push_str(&word);
            result.push_str(colors::RESET);
            continue;
        }
        
        // Dot operators (.AND., .OR., .TRUE., etc.)
        if ch == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let word_upper: String = upper.chars().skip(start).take(i - start).collect();
            
            let color = get_dot_operator_color(&word_upper);
            result.push_str(color);
            result.push_str(&word);
            result.push_str(colors::RESET);
            continue;
        }
        
        // Operators and punctuation
        if "+-*/=<>".contains(ch) {
            result.push_str(colors::OPERATOR);
            result.push(ch);
            // Check for multi-char operators
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if (ch == '*' && next == '*') || (ch == '=' && next == '=') ||
                   (ch == '/' && next == '=') || (ch == '<' && next == '=') ||
                   (ch == '>' && next == '=') || (ch == '=' && next == '>') {
                    i += 1;
                    result.push(next);
                }
            }
            result.push_str(colors::RESET);
            i += 1;
            continue;
        }
        
        // Punctuation
        if "()[],:;%".contains(ch) {
            result.push_str(colors::PUNCT);
            result.push(ch);
            // Check for ::
            if ch == ':' && i + 1 < chars.len() && chars[i + 1] == ':' {
                i += 1;
                result.push(':');
            }
            result.push_str(colors::RESET);
            i += 1;
            continue;
        }
        
        // Default: just push the character
        result.push(ch);
        i += 1;
    }
    
    result
}

/// Get color for a word (keyword or identifier)
fn get_word_color(word: &str) -> &'static str {
    match word {
        // Control flow
        "PROGRAM" | "END" | "IF" | "THEN" | "ELSE" | "ELSEIF" | "DO" | "WHILE" |
        "CYCLE" | "EXIT" | "CONTINUE" | "SELECT" | "CASE" | "DEFAULT" | "RETURN" |
        "CALL" | "CONTAINS" | "WHERE" | "ELSEWHERE" | "FORALL" | "ASSOCIATE" |
        "BLOCK" | "STOP" => colors::CONTROL,
        
        // Procedure keywords
        "SUBROUTINE" | "FUNCTION" | "RESULT" | "RECURSIVE" | "PURE" | "ELEMENTAL" |
        "MODULE" | "USE" | "ONLY" | "INTERFACE" | "OPERATOR" | "ASSIGNMENT" |
        "PROCEDURE" => colors::KEYWORD,
        
        // Types
        "INTEGER" | "REAL" | "DOUBLE" | "PRECISION" | "COMPLEX" | "LOGICAL" |
        "CHARACTER" | "TYPE" | "CLASS" | "DIMENSION" | "ALLOCATABLE" | "ALLOCATE" |
        "DEALLOCATE" | "POINTER" | "TARGET" | "IMPLICIT" | "NONE" | "PARAMETER" |
        "SAVE" | "LEN" | "KIND" | "INTENT" | "IN" | "OUT" | "INOUT" | "PUBLIC" |
        "PRIVATE" | "OPTIONAL" | "EXTENDS" | "PRESENT" | "STAT" => colors::TYPE,
        
        // I/O
        "PRINT" | "READ" | "WRITE" | "OPEN" | "CLOSE" | "FORMAT" | "UNIT" |
        "FILE" | "STATUS" | "ACTION" | "IOSTAT" | "IOMSG" => colors::KEYWORD,
        
        // Parallel
        "CONCURRENT" | "SYNC" | "ALL" | "IMAGES" | "CRITICAL" => colors::KEYWORD,
        
        // Default: identifier
        _ => colors::IDENT,
    }
}

/// Get color for dot operators
fn get_dot_operator_color(op: &str) -> &'static str {
    match op {
        ".TRUE." | ".FALSE." => colors::BOOLEAN,
        ".AND." | ".OR." | ".NOT." | ".EQ." | ".NE." | ".LT." | ".LE." |
        ".GT." | ".GE." | ".EQV." | ".NEQV." => colors::OPERATOR,
        _ => colors::IDENT,
    }
}
