//use crate::value::Unit;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::iter;

use itertools::Itertools;
use ustr::{ustr, Ustr};

use crate::ast::{UnitPart, UnitSet};
use crate::factors::Factors;
use crate::value::Value;

#[derive(Clone, Debug)]
pub struct ConversionRatio {
    ratio: f64,
}

#[derive(Clone, Debug)]
pub struct ConversionLine {
    ratio: f64,
    intercept: f64,
}

#[derive(Clone)]
pub enum Conversion {
    Ratio(ConversionRatio),
    Line(ConversionLine),
}

#[derive(Clone)]
pub struct UnitData {
    pub dimension: Factors,
    pub conversion: Conversion,
}

#[derive(Clone)]
pub struct UnitCtx {
    units: im::HashMap<Ustr, UnitData>,
}

#[derive(Clone, Debug)]
pub enum UnitError {
    UnknownUnit(Ustr),
    BadCombination,
}

impl From<f64> for Conversion {
    fn from(ratio: f64) -> Self { Conversion::Ratio(ConversionRatio { ratio }) }
}

impl From<ConversionRatio> for Conversion {
    fn from(x: ConversionRatio) -> Self { Conversion::Ratio(x) }
}

impl From<ConversionLine> for Conversion {
    fn from(x: ConversionLine) -> Self { Conversion::Line(x) }
}

impl UnitCtx {
    pub fn new() -> Self {
        UnitCtx {
            units: im::HashMap::new(),
        }
    }
    pub fn push(
        &mut self,
        unit: impl IntoIterator<Item = &'static str>,
        dimension: impl Into<Factors>,
        conversion: impl Into<Conversion>,
    ) {
        let dimension = dimension.into();
        let conversion = conversion.into();
        let unit_data = UnitData {
            dimension,
            conversion,
        };
        for unit in unit {
            self.units.insert(unit.into(), unit_data.clone());
        }
    }
    pub fn push_composite(
        &mut self,
        unit: impl IntoIterator<Item = &'static str>,
        count: f64,
        expr: impl IntoIterator<Item = (&'static str, i32)>,
    ) {
        let mut unit_data = self
            .unit_data_for_factors(&expr.into_iter().map(|(k, v)| (ustr(k), v)).collect())
            .unwrap();
        match &mut unit_data.conversion {
            Conversion::Ratio(ratio) => ratio.ratio *= count,
            Conversion::Line(_) => panic!(),
        }
        for unit in unit {
            self.units.insert(unit.into(), unit_data.clone());
        }
    }
    // pub fn unit_data_for_set(&self, unit: &UnitSet) -> Result<UnitData, UnitError> {
    //     let mut total = HashMap::<Ustr, i32>::new();
    //     match unit {
    //         UnitSet::Simple(n) => {
    //             for (k, v) in n.0.iter() {
    //                 *total.entry(*k).or_default() += v;
    //             }
    //         }
    //         UnitSet::Fraction(n, d) => {
    //             for (k, v) in n.0.iter() {
    //                 *total.entry(*k).or_default() += v;
    //             }
    //             for (k, v) in d.0.iter() {
    //                 *total.entry(*k).or_default() -= v;
    //             }
    //         }
    //     };
    //     self.unit_data_for_map(&total)
    // }
    pub fn unit_data_for_factors(&self, unit: &Factors) -> Result<UnitData, UnitError> {
        if let Ok((unit, 1)) = unit.iter().exactly_one() {
            if let Some(
                data @ UnitData {
                    conversion: Conversion::Line(r),
                    ..
                },
            ) = self.units.get(unit)
            {
                return Ok(data.clone());
            }
        }
        let mut ratio = 1.0;
        let mut dimension = Factors::new();
        for (unit, power) in unit.iter() {
            let data = self.units.get(unit).ok_or(UnitError::UnknownUnit(*unit))?;
            if let Conversion::Ratio(r) = &data.conversion {
                ratio *= r.ratio.powi(*power);
                dimension = &dimension * &data.dimension.powi(*power);
            } else {
                return Err(UnitError::BadCombination);
            }
        }
        Ok(UnitData {
            dimension,
            conversion: Conversion::Ratio(ConversionRatio { ratio }),
        })
    }
    pub fn factors_for_unit_set(&self, unit_set: &UnitSet) -> Factors {
        match unit_set {
            UnitSet::None => Factors::new(),
            UnitSet::Simple(x) => self.unit_part_to_factors(x),
            UnitSet::Fraction(n, d) => {
                &self.unit_part_to_factors(n) / &self.unit_part_to_factors(d)
            }
        }
    }
    fn unit_part_to_factors(&self, unit_part: &UnitPart) -> Factors {
        unit_part.0.iter().cloned().collect()
    }

    pub fn standard() -> Self {
        let mut ctx = UnitCtx::new();

        ctx.push(["s", "second", "seconds"], "time", 1.0);
        ctx.push(["min", "minute", "minutes"], "time", 60.0);
        ctx.push(["h", "hour", "hours"], "time", 3600.0);
        ctx.push(["day", "days"], "time", 3600.0 * 24.0);

        ctx.push(["m", "meter", "meters"], "length", 1.0);
        ctx.push(["ft", "foot", "feet"], "length", 0.3048);
        ctx.push(["mi", "mile", "miles"], "length", 1609.344);
        ctx.push(["km", "kilometer", "kilometers"], "length", 1000.0);

        ctx.push(["g", "gram", "grams"], "mass", 1.0);
        ctx.push(["kg", "kilogram", "kilograms"], "mass", 1000.0);

        ctx.push(["K", "kelvin"], "temperature", 1.0);
        ctx.push(
            ["C", "celsius"],
            "temperature",
            ConversionLine {
                ratio: 1.0,
                intercept: 273.15,
            },
        );
        ctx.push(
            ["F", "fahrenheit"],
            "temperature",
            ConversionLine {
                ratio: 5.0 / 9.0,
                intercept: 45967.0 / 180.0,
            },
        );

        ctx.push_composite(["mph"], 1.0, [("mile", 1), ("hour", -1)]);
        ctx.push_composite(["acre", "acres"], 560.0, [("feet", 2)]);
        ctx.push_composite(["in", "inch", "inches"], 1.0 / 12.0, [("feet", 1)]);

        ctx
    }
}

impl Conversion {
    pub fn to_canonical(&self, input: f64) -> f64 {
        match self {
            Conversion::Ratio(ConversionRatio { ratio }) => input * ratio,
            Conversion::Line(_) => todo!(),
        }
    }
    pub fn from_canonical(&self, input: f64) -> f64 {
        match self {
            Conversion::Ratio(ConversionRatio { ratio }) => input / ratio,
            Conversion::Line(_) => todo!(),
        }
    }
}

impl UnitData {
    pub fn new() -> Self {
        UnitData {
            dimension: Factors::new(),
            conversion: Conversion::Ratio(ConversionRatio { ratio: 1.0 }),
        }
    }
}

impl Debug for UnitCtx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "UnitCtx {{")?;
        for (unit, unit_data) in self.units.iter() {
            writeln!(f, "{}: {:?}", unit, unit_data)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl Debug for UnitData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.dimension, self.conversion)
    }
}

impl Debug for Conversion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Conversion::Ratio(r) => write!(f, "{:?}", r.ratio),
            Conversion::Line(l) => write!(f, "{:?} * x + {:?}", l.ratio, l.intercept),
        }
    }
}

impl Display for UnitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitError::UnknownUnit(u) => write!(f, "Unknown unit `{}'", u),
            UnitError::BadCombination => write!(f, "Bad combination"),
        }
    }
}

#[test]
fn test_standard() {
    let standard = UnitCtx::standard();
    println!("{:?}", standard);
}
