#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Token<'a> {
    Ident(&'a str),
    Oper(&'a str),
}

pub struct Lexer<'a> {
    input: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer { input }
    }
}

impl<'a> Token<'a> {
    fn from_ident_like(input: &'a str) -> Self {
        if input == "as"
            || input == "for"
            || input == "impl"
            || input == "mut"
            || input == "const"
            || input == "fn"
        {
            Token::Oper(input)
        } else {
            Token::Ident(input)
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(input) = self.input.strip_prefix(" ") {
            self.input = input;
        }

        let position = self.input.find(&['<', '>', ':', '[', ']', '(', ')', '{', '}', ' ', ',', '&', '*', '-', ';'] as &[char]);
        if let Some(position) = position {
            if position == 0 {
                if let Some(input) = self.input.strip_prefix("::") {
                    self.input = input;
                    Some(Token::Oper("::"))
                } else if let Some(input) = self.input.strip_prefix("->") {
                    self.input = input;
                    Some(Token::Oper("->"))
                } else {
                    let result = &self.input[..1];
                    self.input = &self.input[1..];
                    Some(Token::Oper(result))
                }
            } else {
                let result = &self.input[..position];
                self.input = &self.input[position..];
                Some(Token::from_ident_like(result))
            }
        } else if !self.input.is_empty() {
            let result = self.input;
            self.input = "";
            Some(Token::from_ident_like(result))
        } else {
            None
        }
    }
}

#[test]
fn test_lexer() {
    assert_eq!(Lexer::new("a::b::c<d>").collect::<Vec<_>>(),
               vec![Token::Ident("a"),
                    Token::Oper("::"),
                    Token::Ident("b"),
                    Token::Oper("::"),
                    Token::Ident("c"),
                    Token::Oper("<"),
                    Token::Ident("d"),
                    Token::Oper(">")]);
}