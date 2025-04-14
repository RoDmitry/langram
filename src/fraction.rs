use ::std::fmt::{self, Debug, Display};
use fraction::GenericFraction;
use serde::{
    de::{Error, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

type Size = usize;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fraction {
    numerator: Size,
    denominator: Size,
}

impl Fraction {
    #[inline]
    pub(crate) fn new(numerator: Size, denominator: Size) -> Self {
        let gf = GenericFraction::<Size>::new(numerator, denominator);
        Self::from(gf)
    }

    #[inline]
    pub(crate) fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

impl From<GenericFraction<Size>> for Fraction {
    #[inline]
    fn from(gf: GenericFraction<Size>) -> Self {
        let numerator = *gf.numer().unwrap();
        let denominator = *gf.denom().unwrap();
        Self {
            numerator,
            denominator,
        }
    }
}

impl Debug for Fraction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fraction({}, {})", self.numerator, self.denominator)
    }
}

impl Display for Fraction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl Serialize for Fraction {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}/{}", self.numerator, self.denominator))
    }
}

struct FractionVisitor;

impl<'de> Visitor<'de> for FractionVisitor {
    type Value = Fraction;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a rational number of the format 'numerator/denominator'")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        let bytes = v.as_bytes();
        let (parsed_numerator, len) = atoi_simd::parse_any(bytes)
            .map_err(|e| Error::invalid_value(Unexpected::Str(v), &e.to_string().as_str()))?;

        if bytes[len] != b'/' {
            return Err(Error::invalid_value(Unexpected::Str(v), &"'/'"));
        }

        let parsed_denominator = atoi_simd::parse(&bytes[(len + 1)..]).map_err(|e| {
            serde::de::Error::invalid_value(Unexpected::Str(v), &e.to_string().as_str())
        })?;

        Ok(Fraction::new(parsed_numerator, parsed_denominator))
    }
}

impl<'de> Deserialize<'de> for Fraction {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(FractionVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_reduction() {
        assert_eq!(Fraction::new(12, 144), Fraction::new(1, 12));
    }

    #[test]
    fn test_fraction_serializer() {
        let fraction = Fraction::new(3, 5);
        let serialized = serde_encom::to_string(&fraction).unwrap();
        assert_eq!(serialized, "3=3/5");
    }

    #[test]
    fn test_fraction_deserializer() {
        let fraction = serde_encom::from_str::<Fraction>("3=3/5").unwrap();
        assert_eq!(fraction, Fraction::new(3, 5));
    }
}
