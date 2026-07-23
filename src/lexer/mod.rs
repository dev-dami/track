use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    // Keywords
    #[token("import")]
    Import,
    #[token("let")]
    Let,

    #[token("mut")]
    Mut,
    #[token("with")]
    With,
    #[token("fn")]
    Fn,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("union")]
    Union,
    #[token("match")]
    Match,
    #[token("as")]
    As,
    #[token("@macro")]
    AtMacro,

    #[token("const")]
    Const,
    #[token("@")]
    At,

    // Type keywords
    #[token("i32")]
    TyI32,
    #[token("u32")]
    TyU32,
    #[token("i64")]
    TyI64,
    #[token("u64")]
    TyU64,
    #[token("bool")]
    TyBool,
    #[token("void")]
    TyVoid,
    #[token("ptr")]
    TyPtr,

    // Literals
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    Str(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| Some(lex.slice().to_string()))]
    Ident(String),

    // Punctuation
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Neq,
    #[token("!")]
    Bang,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token("_", priority = 3)]
    Underscore,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("*")]
    Star,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Lte,
    #[token(">=")]
    Gte,
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
}

pub struct Lexer;

impl Lexer {
    pub fn tokenize(input: &str) -> Result<Vec<(Token, std::ops::Range<usize>)>, String> {
        let mut tokens = Vec::new();
        let mut lex = Token::lexer(input);

        loop {
            match lex.next() {
                Some(Ok(token)) => {
                    let span = lex.span();
                    tokens.push((token, span));
                }
                Some(Err(())) => {
                    let span = lex.span();
                    let ch = input[span.clone()].chars().next().unwrap_or('?');
                    return Err(format!(
                        "Lexer error at {:?}: unexpected character '{}'",
                        span, ch
                    ));
                }
                None => break,
            }
        }
        Ok(tokens)
    }
}
