use crate::remangle::lexer::{Lexer, Token};
use lookahead::Lookahead;
use lookahead::lookahead;
use std::fmt::{Debug, Formatter};
use std::fmt;
use crate::remangle::path::{PathSegment, Path, PathBraces, PathArg};

pub struct Parser<'a> {
    pub lexer: Lookahead<Lexer<'a>>,
}


#[derive(Debug)]
pub struct ParseError(&'static str);

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser { lexer: lookahead(Lexer::new(input)) }
    }
    fn can_read(&mut self, tokens: &[&str]) -> bool {
        for (i, token) in tokens.iter().enumerate() {
            if self.lexer.lookahead(i) != Some(&Token::Oper(token)) {
                return false;
            }
        }
        true
    }
    fn read(&mut self, tokens: &[&str]) -> bool {
        if !self.can_read(tokens) {
            return false;
        }
        for _ in tokens.iter() {
            self.lexer.next();
        }
        true
    }

    fn read_ident<'b>(&'b mut self) -> Option<&'a str> where 'a: 'b {
        match self.lexer.lookahead(0)? {
            Token::Ident(ident) => {
                let ident = *ident;
                self.lexer.next();
                Some(ident)
            }
            Token::Oper(_) => None,
        }
    }
    fn parse_braces<'b>(&'b mut self) -> ParseResult<Option<PathBraces<'a>>> where 'a: 'b {
        if self.read(&["{"]) {
            if self.read(&["{"]) {
                let ident = self.read_ident().ok_or(ParseError("expected shim or closure, found oper"))?;
                let result;
                if ident == "vtable.shim" {
                    result = Some(PathBraces::UnknownVTable);
                } else if ident == "closure" {
                    result = Some(PathBraces::UnknownClosure);
                } else {
                    return Err(ParseError("expected 'vtable.shim' or 'closure'"));
                }
                if !self.read(&["}"]) {
                    return Err(ParseError("expected '}'"));
                }
                if !self.read(&["}"]) {
                    return Err(ParseError("expected '}'"));
                }
                return Ok(result);
            } else if let Some(prefix) = self.read_ident() {
                let result;
                if prefix == "shim.vtable" {
                    result = Some(PathBraces::VTable { vtable: "" });
                } else if prefix == "shim" {
                    if !self.read(&[":"]) {
                        return Err(ParseError("expected ':'"));
                    }
                    let vtable = self.read_ident().ok_or(ParseError("expected vtable"))?;
                    let vtable = vtable.strip_prefix("vtable#").ok_or(ParseError("expected vtable..."))?;
                    result = Some(PathBraces::VTable { vtable });
                } else if let Some(closure) = prefix.strip_prefix("closure#") {
                    result = Some(PathBraces::Closure { closure });
                } else {
                    return Err(ParseError("expected shim or closure"))?;
                }
                if !self.read(&["}"]) {
                    return Err(ParseError("expected '}'"));
                }
                return Ok(result);
            } else {
                return Err(ParseError("expected shim or closure"));
            }
        } else {
            Ok(None)
        }
    }
    fn parse_segment<'b>(&'b mut self) -> ParseResult<Option<PathSegment<'a>>> where 'a: 'b {
        if self.can_read(&["as"])
            || self.can_read(&[">"])
            || self.can_read(&["]"])
            || self.can_read(&[")"])
            || self.can_read(&["}"])
            || self.can_read(&[","])
            || self.can_read(&["for"])
            || self.can_read(&[";"])
            || self.can_read(&["+"])
            || self.can_read(&["="])
        {
            return Ok(None);
        }
        if self.read(&["["]) {
            let ty = self.parse_path()?;
            let mut length = None;
            let mut length_ty = None;
            if self.read(&[";"]) {
                length = Some(self.read_ident().ok_or(ParseError("expected length"))?);
                if self.read(&[":"]) {
                    length_ty = Some(self.parse_path()?);
                }
            }
            if !self.read(&["]"]) {
                return Err(ParseError("Expected ']' after '['"));
            }
            return Ok(Some(PathSegment::Array { ty, length, length_ty }));
        } else if self.read(&["("]) {
            let mut tys = vec![];
            if !self.read(&[")"]) {
                loop {
                    tys.push(self.parse_path()?);
                    if self.read(&[")"]) || self.read(&[",", ")"]) {
                        break;
                    } else if self.read(&[","]) {
                        continue;
                    } else {
                        return Err(ParseError("expected ')' or ','"));
                    }
                }
            }
            return Ok(Some(PathSegment::Tuple { tys }));
        } else if self.read(&["fn", "("]) {
            let mut tys = vec![];
            loop {
                tys.push(self.parse_path()?);
                if self.read(&[")"]) {
                    break;
                } else if self.read(&[","]) {
                    continue;
                } else {
                    return Err(ParseError("expected ')' or ',' in 'fn' section"));
                }
            }
            let mut output = None;
            if self.read(&["->"]) {
                output = Some(self.parse_path()?);
            }
            return Ok(Some(PathSegment::FnPtr { tys, output }));
        } else if let Some(braces) = self.parse_braces()? {
            return Ok(Some(PathSegment::Ident {
                name: "",
                version: None,
                braces: Some(braces),
                turbofish: false,
                tys: vec![],
            }));
        } else if self.read(&["&", "mut"]) {
            return Ok(Some(PathSegment::Pointy { raw: false, mutable: true, ty: self.parse_path()? }));
        } else if self.read(&["&"]) {
            return Ok(Some(PathSegment::Pointy { raw: false, mutable: false, ty: self.parse_path()? }));
        } else if self.read(&["*", "mut"]) {
            return Ok(Some(PathSegment::Pointy { raw: true, mutable: true, ty: self.parse_path()? }));
        } else if self.read(&["*", "const"]) {
            return Ok(Some(PathSegment::Pointy { raw: true, mutable: false, ty: self.parse_path()? }));
        } else if self.read(&["<", "impl"]) {
            let trait_for = self.parse_path()?;
            if !self.read(&["for"]) {
                return Err(ParseError("missing for"));
            }
            let for_ty = self.parse_path()?;
            if !self.read(&[">"]) {
                return Err(ParseError("impl bad ending"));
            }
            return Ok(Some(PathSegment::ImplFor { trait_for, for_ty }));
        } else if self.read(&["<"]) {
            let ty = self.parse_path()?;
            let mut as_trait = None;
            if self.read(&["as"]) {
                as_trait = Some(self.parse_path()?);
            }
            if !self.read(&[">"]) {
                return Err(ParseError("inherent bad ending"));
            }
            if let Some(as_trait) = as_trait {
                return Ok(Some(PathSegment::As { ty, as_trait }));
            } else {
                return Ok(Some(PathSegment::Ty { ty }));
            }
        } else if self.read(&["dyn"]) {
            let mut tys = vec![];
            loop {
                tys.push(self.parse_path()?);
                if self.read(&["+"]) {
                    continue;
                } else {
                    break;
                }
            }
            return Ok(Some(PathSegment::Dyn { tys }));
        } else if let Some(name) = self.read_ident() {
            let mut tys = vec![];
            let mut version = None;
            if self.read(&["["]) {
                version = Some(self.read_ident().ok_or(ParseError("expected version in brackets"))?);
                if !self.read(&["]"]) { return Err(ParseError("expected ']' after version")); };
            }
            let braces = self.parse_braces()?;
            let mut turbofish = false;
            let mut tardyfish = false;
            if !self.can_read(&["::", "<", "impl"]) {
                if self.read(&["::", "<"]) {
                    turbofish = true;
                } else if self.read(&["<"]) {
                    tardyfish = true
                }
                if turbofish || tardyfish {
                    loop {
                        let p1 = self.parse_path()?;
                        if self.read(&["="]) {
                            let p2 = self.parse_path()?;
                            tys.push(PathArg { name: Some(p1), value: p2 });
                        } else {
                            tys.push(PathArg { name: None, value: p1 });
                        }
                        if self.read(&[","]) {
                            continue;
                        } else if self.read(&[">"]) {
                            break;
                        } else {
                            return Err(ParseError("expecting ',' or '>'"));
                        }
                    }
                }
            }
            return Ok(Some(PathSegment::Ident { name, version, braces, turbofish, tys }));
        } else {
            return Err(ParseError("No valid segment prefix"));
        }
    }
    pub fn parse_path<'b>(&'b mut self) -> ParseResult<Path<'a>> where 'a: 'b {
        let mut result = Path { segments: vec![] };
        if let Some(segment) = self.parse_segment()? {
            result.segments.push(segment);
            while self.read(&["::"]) {
                result.segments.push(self.parse_segment()?.ok_or(ParseError("missing ::"))?);
            }
        }
        Ok(result)
    }
    pub fn finish(&mut self) -> ParseResult<()> {
        if self.lexer.lookahead(0).is_none() {
            Ok(())
        } else {
            Err(ParseError("Unused tokens"))
        }
    }
}

impl<'a> Path<'a> {}

#[test]
fn test_parser() {
    use crate::remangle::path::EXAMPLES;
    for example in EXAMPLES {
        println!("{:?}", Path::parse(example).unwrap());
    }
}
