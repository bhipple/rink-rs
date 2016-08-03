use std::str::Chars;
use std::iter::Peekable;
use std::str::FromStr;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Token {
    Newline,
    Comment,
    Ident(String),
    Number(f64),
    Slash,
    ColonDash,
    DColonDash,
    TriplePipe,
    ColonEq,
    EqBangEq,
    Carot,
    Eof,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Plus,
    Minus,
    Error(String),
}

#[derive(Clone)]
pub struct TokenIterator<'a>(Peekable<Chars<'a>>);

impl<'a> TokenIterator<'a> {
    pub fn new(input: &'a str) -> TokenIterator<'a> {
        TokenIterator(input.chars().peekable())
    }
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.0.peek() == None {
            return Some(Token::Eof)
        }
        let res = match self.0.next().unwrap() {
            ' ' | '\t' => return self.next(),
            '\n' => Token::Newline,
            '[' => Token::LBrack,
            ']' => Token::RBrack,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '/' => match self.0.peek() {
                Some(&'/') => loop {
                    match self.0.next() {
                        Some('\n') => return Some(Token::Comment),
                        _ => ()
                    }
                },
                Some(&'*') => loop {
                    if let Some('*') = self.0.next() {
                        if let Some(&'/') = self.0.peek() {
                            return Some(Token::Comment)
                        }
                    }
                    if self.0.peek() == None {
                        return Some(Token::Error(format!("Expected `*/`, got EOF")))
                    }
                },
                _ => Token::Slash
            },
            x @ '0'...'9' => {
                let mut buf = String::new();
                buf.push(x);
                while let Some(c) = self.0.peek().cloned() {
                    match c {
                        'e' | 'E' => {
                            buf.push(self.0.next().unwrap());
                            loop {
                                match self.0.peek().cloned() {
                                    Some('e') | Some('E') => self.0.next(),
                                    _ => break
                                };
                            }
                        },
                        '0'...'9' | '.' | '-' | '+' => buf.push(self.0.next().unwrap()),
                        _ => break
                    }
                }
                FromStr::from_str(&*buf).map(|x| Token::Number(x))
                    .unwrap_or(Token::Error(format!("Invalid number literal: `{}`", buf)))
            },
            ':' => match self.0.next() {
                Some(':') => match self.0.next() {
                    Some('-') => Token::DColonDash,
                    x => Token::Error(format!("Unexpected {:?}", x)),
                },
                Some('-') => Token::ColonDash,
                Some('=') => Token::ColonEq,
                x => Token::Error(format!("Unexpected {:?}", x))
            },
            '|' => if let Some('|') = self.0.next() {
                if let Some('|') = self.0.next() {
                    Token::TriplePipe
                } else {
                    Token::Error(format!("Unknown symbol"))
                }
            } else {
                Token::Error(format!("Unknown symbol"))
            },
            '=' => if let Some('!') = self.0.next() {
                if let Some('=') = self.0.next() {
                    Token::EqBangEq
                } else {
                    Token::Error(format!("Unknown symbol"))
                }
            } else {
                Token::Error(format!("Unknown symbol"))
            },
            '^' => Token::Carot,
            x => {
                let mut buf = String::new();
                buf.push(x);
                while let Some(c) = self.0.peek().cloned() {
                    if c.is_alphanumeric() || c == '_' {
                        buf.push(self.0.next().unwrap());
                    } else {
                        break;
                    }
                }
                Token::Ident(buf)
            }
        };
        Some(res)
    }
}

pub type Iter<'a> = Peekable<TokenIterator<'a>>;

#[derive(Debug)]
pub enum Expr {
    Unit(String),
    Const(f64),
    Frac(Box<Expr>, Box<Expr>),
    Mul(Vec<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Plus(Box<Expr>),
    Error(String),
}

#[derive(Debug)]
pub enum Def {
    Prefix(Expr),
    SPrefix(Expr),
    Dimension(String),
    Unit(Expr),
    Error(String),
}

#[derive(Debug)]
pub struct Defs {
    pub defs: Vec<(String, Rc<Def>)>,
    pub aliases: Vec<(Expr, String)>,
}

fn parse_term(mut iter: &mut Iter) -> Expr {
    match iter.next().unwrap() {
        Token::Ident(name) => Expr::Unit(name),
        Token::Number(num) => Expr::Const(num),
        Token::Plus => Expr::Plus(Box::new(parse_term(iter))),
        Token::Minus => Expr::Neg(Box::new(parse_term(iter))),
        x => Expr::Error(format!("Expected term, got {:?}", x))
    }
}

fn parse_pow(mut iter: &mut Iter) -> Expr {
    let left = parse_term(iter);
    match *iter.peek().unwrap() {
        Token::Carot => {
            iter.next();
            let right = parse_pow(iter);
            Expr::Pow(Box::new(left), Box::new(right))
        },
        _ => left
    }
}

fn parse_mul(mut iter: &mut Iter) -> Expr {
    let mut terms = vec![parse_pow(iter)];
    loop { match *iter.peek().unwrap() {
        Token::Slash | Token::TriplePipe | Token::Newline | Token::Comment | Token::Eof => break,
        _ => terms.push(parse_pow(iter))
    }}
    if terms.len() == 1 {
        terms.pop().unwrap()
    } else {
        Expr::Mul(terms)
    }
}

pub fn parse_expr(mut iter: &mut Iter) -> Expr {
    let left = parse_mul(iter);
    let mut terms = vec![];
    loop { match *iter.peek().unwrap() {
        Token::Slash => {
            iter.next();
            terms.push(parse_mul(iter));
        },
        Token::TriplePipe | Token::Newline | Token::Comment | Token::Eof | _ => break,
    } }
    terms.into_iter().fold(left, |a, b| Expr::Frac(Box::new(a), Box::new(b)))
}

fn parse_unknown(mut iter: &mut Iter) {
    loop {
        match iter.next().unwrap() {
            Token::Newline | Token::Comment | Token::Eof => break,
            _ => ()
        }
    }
}

fn parse_alias(mut iter: &mut Iter) -> Option<(Expr, String)> {
    let expr = parse_expr(iter);
    match iter.next().unwrap() {
        Token::TriplePipe => (),
        _ => return None
    };
    let name = match iter.next().unwrap() {
        Token::Ident(name) => name,
        _ => return None
    };
    match iter.next().unwrap() {
        Token::Newline | Token::Comment | Token::Eof => (),
        _ => return None
    };
    Some((expr, name))
}

pub fn parse(mut iter: &mut Iter) -> Defs {
    let mut map = vec![];
    let mut aliases = vec![];
    loop {
        let mut copy = iter.clone();
        if let Some(a) = parse_alias(&mut copy) {
            aliases.push(a);
            *iter = copy;
            continue
        }
        match iter.next().unwrap() {
            Token::Newline => continue,
            Token::Comment => continue,
            Token::Eof => break,
            Token::Ident(name) => {
                let def = match iter.next().unwrap() {
                    Token::ColonDash => Def::Prefix(parse_expr(iter)),
                    Token::DColonDash => Def::SPrefix(parse_expr(iter)),
                    Token::EqBangEq => match iter.next().unwrap() {
                        Token::Ident(val) => Def::Dimension(val),
                        _ => Def::Error(format!("Malformed dimensionless unit"))
                    },
                    Token::ColonEq => Def::Unit(parse_expr(iter)),
                    Token::LBrack => {
                        // NYI
                        loop {
                            match iter.next().unwrap() {
                                Token::RBrace => break,
                                _ => ()
                            }
                        }
                        Def::Error(format!("NYI: functions"))
                    }
                    _ => {
                        parse_unknown(iter);
                        Def::Error(format!("Unknown definition"))
                    }
                };
                map.push((name.clone(), Rc::new(def)));
            },
            _ => parse_unknown(iter)
        };
    }
    Defs {
        defs: map,
        aliases: aliases
    }
}

pub fn tokens(mut iter: &mut Iter) -> Vec<Token> {
    let mut out = vec![];
    loop {
        match iter.next().unwrap() {
            Token::Eof => break,
            x => out.push(x)
        }
    }
    out
}
