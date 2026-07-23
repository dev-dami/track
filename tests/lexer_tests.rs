use track::lexer::{Lexer, Token};

#[test]
fn test_tokenize_keywords() {
    let source = "let mut with fn return if else while true false struct enum union match as const @ @use @macro";
    let tokens = Lexer::tokenize(source).unwrap();
    let token_kinds: Vec<Token> = tokens.into_iter().map(|(t, _)| t).collect();

    assert_eq!(
        token_kinds,
        vec![
            Token::Let,
            Token::Mut,
            Token::With,
            Token::Fn,
            Token::Return,
            Token::If,
            Token::Else,
            Token::While,
            Token::True,
            Token::False,
            Token::Struct,
            Token::Enum,
            Token::Union,
            Token::Match,
            Token::As,
            Token::Const,
            Token::At,
            Token::AtUse,
            Token::AtMacro,
        ]
    );
}

#[test]
fn test_tokenize_types() {
    let source = "i32 u32 i64 u64 bool void ptr";
    let tokens = Lexer::tokenize(source).unwrap();
    let token_kinds: Vec<Token> = tokens.into_iter().map(|(t, _)| t).collect();

    assert_eq!(
        token_kinds,
        vec![
            Token::TyI32,
            Token::TyU32,
            Token::TyI64,
            Token::TyU64,
            Token::TyBool,
            Token::TyVoid,
            Token::TyPtr,
        ]
    );
}

#[test]
fn test_tokenize_literals_and_idents() {
    let source = "12345 \"hello world\" my_var _var2";
    let tokens = Lexer::tokenize(source).unwrap();
    let token_kinds: Vec<Token> = tokens.into_iter().map(|(t, _)| t).collect();

    assert_eq!(
        token_kinds,
        vec![
            Token::Int(12345),
            Token::Str("hello world".to_string()),
            Token::Ident("my_var".to_string()),
            Token::Ident("_var2".to_string()),
        ]
    );
}

#[test]
fn test_tokenize_operators_and_punctuation() {
    let source = "= == != ! { } ( ) [ ] , : :: -> => ; . _ & | * + - / % < > <= >= && || << >>";
    let tokens = Lexer::tokenize(source).unwrap();
    let token_kinds: Vec<Token> = tokens.into_iter().map(|(t, _)| t).collect();

    assert_eq!(
        token_kinds,
        vec![
            Token::Eq,
            Token::EqEq,
            Token::Neq,
            Token::Bang,
            Token::LBrace,
            Token::RBrace,
            Token::LParen,
            Token::RParen,
            Token::LBracket,
            Token::RBracket,
            Token::Comma,
            Token::Colon,
            Token::ColonColon,
            Token::Arrow,
            Token::FatArrow,
            Token::Semicolon,
            Token::Dot,
            Token::Underscore,
            Token::Amp,
            Token::Pipe,
            Token::Star,
            Token::Plus,
            Token::Minus,
            Token::Slash,
            Token::Percent,
            Token::Lt,
            Token::Gt,
            Token::Lte,
            Token::Gte,
            Token::AmpAmp,
            Token::PipePipe,
            Token::Shl,
            Token::Shr,
        ]
    );
}

#[test]
fn test_tokenize_comments_and_whitespace() {
    let source = "// This is a comment\nlet x = 10; // comment after code\n";
    let tokens = Lexer::tokenize(source).unwrap();
    let token_kinds: Vec<Token> = tokens.into_iter().map(|(t, _)| t).collect();

    assert_eq!(
        token_kinds,
        vec![
            Token::Let,
            Token::Ident("x".to_string()),
            Token::Eq,
            Token::Int(10),
            Token::Semicolon,
        ]
    );
}

#[test]
fn test_tokenize_invalid_character() {
    let source = "let x = #;";
    let result = Lexer::tokenize(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unexpected character '#'"));
}
