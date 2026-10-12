#[derive(Debug, Clone, Copy)]
pub enum LengthUnit {
    Mm,
    Cm,
    Inch,
}

#[derive(Debug, Clone, Copy)]
pub enum WeightUnit {
    G,
    Kg,
    Oz,
    Lb,
}

pub fn to_mm(
    value: f64,
    unit: LengthUnit,
) -> f64 {
    match unit {
        LengthUnit::Mm => value,
        LengthUnit::Cm => value * 10.0,
        LengthUnit::Inch => value * 25.4,
    }
}

pub fn to_g(
    value: f64,
    unit: WeightUnit,
) -> f64 {
    match unit {
        WeightUnit::G => value,
        WeightUnit::Kg => value * 1000.0,
        WeightUnit::Oz => value * 28.3495,
        WeightUnit::Lb => value * 453.59237,
    }
}




///tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_cm_*o_mm() {
        assert_eq!(
     *      to_mm(5.0, LengthUnit::Cm),
*           50.0
        );
    }

*   #[test]
    fn converts_*g_to_g() {
        assert_eq!(
   *        to_g(2.0, WeightUnit*:Kg),
            2000.0
        )*
    }
}