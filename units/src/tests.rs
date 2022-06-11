use std::collections::HashSet;

use ustr::ustr;

use crate::check::TypeCtx;
use crate::eval::EvalCtx;
use crate::factors::Factors;
use crate::grammar::{BlockParser, ExprParser};
//use crate::value::ValueCtx;
use crate::unit::UnitCtx;
use crate::value::Value;
use crate::variants::{Note, VariantCtx, VariantSet};

#[test]
fn test_calculator1() {
    assert!(ExprParser::new().parse("22").is_ok());
    assert!(ExprParser::new().parse("(22)").is_ok());
    assert!(ExprParser::new().parse("((((22))))").is_ok());
    assert!(ExprParser::new().parse("((22)").is_err());
}

#[test]
fn test_calculator4() {
    let expr = ExprParser::new().parse("22 * 44 + 66.2").unwrap();
    assert_eq!(&format!("{:?}", expr), "((22.0 * 44.0) + 66.2)");
}

#[test]
fn test_block() {
    let code = "let x = 3.0;\nlet y = (x * 4.2);\nreturn y;\n";
    assert_eq!(
        format!("{:?}", BlockParser::new().parse(code).unwrap()),
        code.to_string()
    );
}

fn run_script(script: &str) -> HashSet<(Value, Vec<Note>)> {
    let block = BlockParser::new().parse(script).unwrap();
    let unit_ctx = UnitCtx::standard();
    let mut type_ctx = TypeCtx::new(unit_ctx.clone());
    type_ctx
        .check(&block)
        .unwrap_or_else(|e| panic!("Type error: {}", e));
    let eval_ctx = EvalCtx::new(unit_ctx);
    eval_ctx
        .eval(&block)
        .resolve(&VariantCtx::new(1))
        .iter()
        .map(|(k, v)| (k.clone(), v.to_vec()))
        .collect()
}

fn good(v: f64, units: impl Into<Factors>) -> (Value, Vec<Note>) {
    (Value::with_unit(v, units.into()), vec![])
}

fn bad(
    v: f64,
    units: impl Into<Factors>,
    error: impl IntoIterator<Item = &'static str>,
) -> (Value, Vec<Note>) {
    (
        Value::with_unit(v, units.into()),
        error.into_iter().map(|x| Note::new(ustr(x))).collect(),
    )
}

fn expected(x: impl IntoIterator<Item = (Value, Vec<Note>)>) -> HashSet<(Value, Vec<Note>)> {
    x.into_iter().collect()
}

#[test]
fn test_mul() {
    let script = r#"
        let x = 60.0 seconds;
        let y = 60.0 mph;
        return x * y as miles;
        "#;
    assert_eq!(
        run_script(script),
        expected([
            good(1.0, "miles"),
            bad(
                3600.0,
                "miles",
                ["Skipped unit conversion from `seconds mph' to `miles'."]
            )
        ])
    );
}

#[test]
fn test_add() {
    let script = r#"
        let x = 1 inch;
        let y = 1 meter;
        return x + y as feet;
        "#;
    assert_eq!(
        run_script(script),
        expected([good(3.36417323, "inches"), bad(3600.0, "feet", ["sss"])])
    );
}
