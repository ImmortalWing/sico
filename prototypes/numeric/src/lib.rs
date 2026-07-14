use std::fmt;

use num_bigint::{BigInt, Sign};
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumericLimits {
    pub max_int_bytes: usize,
    pub max_decimal_digits: usize,
    pub max_decimal_scale: u32,
    pub max_wire_bytes: usize,
}

impl NumericLimits {
    pub const fn prototype_default() -> Self {
        Self {
            max_int_bytes: 1 << 20,
            max_decimal_digits: 100_000,
            max_decimal_scale: 100_000,
            max_wire_bytes: (1 << 20) + 16,
        }
    }

    pub const fn test_small() -> Self {
        Self {
            max_int_bytes: 16,
            max_decimal_digits: 32,
            max_decimal_scale: 12,
            max_wire_bytes: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitKind {
    IntBytes,
    DecimalDigits,
    DecimalScale,
    WireBytes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumericError {
    LimitExceeded {
        kind: LimitKind,
        limit: u64,
        actual: u64,
    },
    NonCanonical(&'static str),
    InvalidDecimal(&'static str),
    FixedWidthOverflow(&'static str),
}

impl fmt::Display for NumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded {
                kind,
                limit,
                actual,
            } => write!(f, "{kind:?} limit {limit}, found {actual}"),
            Self::NonCanonical(reason) => write!(f, "non-canonical number: {reason}"),
            Self::InvalidDecimal(reason) => write!(f, "invalid decimal: {reason}"),
            Self::FixedWidthOverflow(target) => write!(f, "value does not fit {target}"),
        }
    }
}

impl std::error::Error for NumericError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerSign {
    Negative,
    Zero,
    Positive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalInt {
    pub sign: IntegerSign,
    pub magnitude_be: Vec<u8>,
}

impl CanonicalInt {
    pub fn from_bigint(value: &BigInt, limits: &NumericLimits) -> Result<Self, NumericError> {
        check_int(value, limits)?;
        let (sign, mut magnitude_be) = value.to_bytes_be();
        let sign = match sign {
            Sign::Minus => IntegerSign::Negative,
            Sign::NoSign => {
                magnitude_be.clear();
                IntegerSign::Zero
            }
            Sign::Plus => IntegerSign::Positive,
        };
        let wire = Self { sign, magnitude_be };
        check_wire_len(wire.encoded_len(), limits)?;
        Ok(wire)
    }

    pub fn to_bigint(&self, limits: &NumericLimits) -> Result<BigInt, NumericError> {
        check_wire_len(self.encoded_len(), limits)?;
        check_limit(
            LimitKind::IntBytes,
            limits.max_int_bytes,
            self.magnitude_be.len(),
        )?;

        if self.magnitude_be.first() == Some(&0) {
            return Err(NumericError::NonCanonical(
                "integer magnitude has a leading zero",
            ));
        }

        let sign = match (self.sign, self.magnitude_be.is_empty()) {
            (IntegerSign::Zero, true) => Sign::NoSign,
            (IntegerSign::Zero, false) => {
                return Err(NumericError::NonCanonical("zero has a magnitude"));
            }
            (IntegerSign::Negative, false) => Sign::Minus,
            (IntegerSign::Positive, false) => Sign::Plus,
            (_, true) => return Err(NumericError::NonCanonical("non-zero sign has no magnitude")),
        };
        Ok(BigInt::from_bytes_be(sign, &self.magnitude_be))
    }

    pub fn encoded_len(&self) -> usize {
        1 + self.magnitude_be.len()
    }

    pub fn deterministic_bytes(&self) -> Vec<u8> {
        let tag = match self.sign {
            IntegerSign::Negative => 0,
            IntegerSign::Zero => 1,
            IntegerSign::Positive => 2,
        };
        let mut bytes = Vec::with_capacity(self.encoded_len());
        bytes.push(tag);
        bytes.extend_from_slice(&self.magnitude_be);
        bytes
    }
}

pub fn checked_int_add(
    left: &BigInt,
    right: &BigInt,
    limits: &NumericLimits,
) -> Result<BigInt, NumericError> {
    check_int(left, limits)?;
    check_int(right, limits)?;
    let value = left + right;
    check_int(&value, limits)?;
    Ok(value)
}

pub fn checked_int_mul(
    left: &BigInt,
    right: &BigInt,
    limits: &NumericLimits,
) -> Result<BigInt, NumericError> {
    check_int(left, limits)?;
    check_int(right, limits)?;
    if !left.is_zero() && !right.is_zero() {
        let minimum_result_bits = left.bits().saturating_add(right.bits()).saturating_sub(1);
        let max_bits = (limits.max_int_bytes as u64).saturating_mul(8);
        if minimum_result_bits > max_bits {
            return Err(NumericError::LimitExceeded {
                kind: LimitKind::IntBytes,
                limit: limits.max_int_bytes as u64,
                actual: minimum_result_bits.div_ceil(8),
            });
        }
    }
    let value = left * right;
    check_int(&value, limits)?;
    Ok(value)
}

pub fn int_to_i64(value: &BigInt) -> Result<i64, NumericError> {
    value
        .to_i64()
        .ok_or(NumericError::FixedWidthOverflow("s64"))
}

fn check_int(value: &BigInt, limits: &NumericLimits) -> Result<(), NumericError> {
    let (_, magnitude) = value.to_bytes_be();
    check_limit(LimitKind::IntBytes, limits.max_int_bytes, magnitude.len())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    TowardZero,
    HalfEven,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactDecimal {
    coefficient: BigInt,
    scale: u32,
}

impl ExactDecimal {
    pub fn new(
        mut coefficient: BigInt,
        mut scale: u32,
        limits: &NumericLimits,
    ) -> Result<Self, NumericError> {
        while scale > 0 && (&coefficient % 10u8).is_zero() {
            coefficient /= 10u8;
            scale -= 1;
        }
        if coefficient.is_zero() {
            scale = 0;
        }
        let value = Self { coefficient, scale };
        value.check(limits)?;
        Ok(value)
    }

    pub fn parse(text: &str, limits: &NumericLimits) -> Result<Self, NumericError> {
        if text.is_empty() {
            return Err(NumericError::InvalidDecimal("empty text"));
        }
        if text.starts_with('+') {
            return Err(NumericError::InvalidDecimal(
                "leading plus is not canonical input",
            ));
        }
        let (negative, unsigned) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let mut parts = unsigned.split('.');
        let integer = parts.next().unwrap_or_default();
        let fraction = parts.next();
        if parts.next().is_some() || integer.is_empty() {
            return Err(NumericError::InvalidDecimal(
                "expected one integer and optional fraction",
            ));
        }
        if !integer.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(NumericError::InvalidDecimal("integer contains a non-digit"));
        }
        if matches!(fraction, Some(value) if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(NumericError::InvalidDecimal(
                "fraction is empty or contains a non-digit",
            ));
        }
        let fraction = fraction.unwrap_or_default();
        let raw_digits = integer.len().saturating_add(fraction.len());
        check_limit(
            LimitKind::DecimalDigits,
            limits.max_decimal_digits,
            raw_digits,
        )?;
        let scale = u32::try_from(fraction.len()).map_err(|_| NumericError::LimitExceeded {
            kind: LimitKind::DecimalScale,
            limit: u64::from(limits.max_decimal_scale),
            actual: u64::MAX,
        })?;
        check_limit_u32(LimitKind::DecimalScale, limits.max_decimal_scale, scale)?;

        let digits = format!("{integer}{fraction}");
        let mut coefficient = BigInt::parse_bytes(digits.as_bytes(), 10)
            .ok_or(NumericError::InvalidDecimal("coefficient is not base 10"))?;
        if negative {
            coefficient = -coefficient;
        }
        Self::new(coefficient, scale, limits)
    }

    pub fn coefficient(&self) -> &BigInt {
        &self.coefficient
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn add(&self, other: &Self, limits: &NumericLimits) -> Result<Self, NumericError> {
        let scale = self.scale.max(other.scale);
        let left = &self.coefficient * pow10(scale - self.scale);
        let right = &other.coefficient * pow10(scale - other.scale);
        Self::new(left + right, scale, limits)
    }

    pub fn mul(&self, other: &Self, limits: &NumericLimits) -> Result<Self, NumericError> {
        let scale = self
            .scale
            .checked_add(other.scale)
            .ok_or(NumericError::LimitExceeded {
                kind: LimitKind::DecimalScale,
                limit: u64::from(limits.max_decimal_scale),
                actual: u64::MAX,
            })?;
        Self::new(&self.coefficient * &other.coefficient, scale, limits)
    }

    pub fn round(
        &self,
        target_scale: u32,
        mode: RoundingMode,
        limits: &NumericLimits,
    ) -> Result<Self, NumericError> {
        check_limit_u32(
            LimitKind::DecimalScale,
            limits.max_decimal_scale,
            target_scale,
        )?;
        if self.scale <= target_scale {
            return Self::new(self.coefficient.clone(), self.scale, limits);
        }
        let factor = pow10(self.scale - target_scale);
        let quotient = &self.coefficient / &factor;
        let remainder = &self.coefficient % &factor;
        let increment = match mode {
            RoundingMode::TowardZero => false,
            RoundingMode::HalfEven => {
                let doubled = remainder.abs() * 2u8;
                doubled > factor || (doubled == factor && (&quotient % 2u8) != BigInt::ZERO)
            }
        };
        let rounded = if increment {
            if self.coefficient.is_negative() {
                quotient - 1u8
            } else {
                quotient + 1u8
            }
        } else {
            quotient
        };
        Self::new(rounded, target_scale, limits)
    }

    pub fn to_canonical_string(&self) -> String {
        let negative = self.coefficient.is_negative();
        let mut digits = self.coefficient.abs().to_str_radix(10);
        if self.scale == 0 {
            return if negative {
                format!("-{digits}")
            } else {
                digits
            };
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let zeros = "0".repeat(scale + 1 - digits.len());
            digits = format!("{zeros}{digits}");
        }
        let split = digits.len() - scale;
        digits.insert(split, '.');
        if negative {
            format!("-{digits}")
        } else {
            digits
        }
    }

    pub fn to_wire(&self, limits: &NumericLimits) -> Result<DecimalWire, NumericError> {
        let wire = DecimalWire {
            coefficient: CanonicalInt::from_bigint(&self.coefficient, limits)?,
            scale: self.scale,
        };
        check_wire_len(wire.encoded_len(), limits)?;
        Ok(wire)
    }

    fn check(&self, limits: &NumericLimits) -> Result<(), NumericError> {
        check_limit_u32(
            LimitKind::DecimalScale,
            limits.max_decimal_scale,
            self.scale,
        )?;
        check_limit(
            LimitKind::DecimalDigits,
            limits.max_decimal_digits,
            decimal_digits(&self.coefficient),
        )?;
        check_int(&self.coefficient, limits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecimalWire {
    pub coefficient: CanonicalInt,
    pub scale: u32,
}

impl DecimalWire {
    pub fn to_decimal(&self, limits: &NumericLimits) -> Result<ExactDecimal, NumericError> {
        check_wire_len(self.encoded_len(), limits)?;
        check_limit_u32(
            LimitKind::DecimalScale,
            limits.max_decimal_scale,
            self.scale,
        )?;
        let coefficient = self.coefficient.to_bigint(limits)?;
        let normalized = ExactDecimal::new(coefficient.clone(), self.scale, limits)?;
        if normalized.coefficient != coefficient || normalized.scale != self.scale {
            return Err(NumericError::NonCanonical(
                "decimal coefficient has a trailing zero or zero has a scale",
            ));
        }
        Ok(normalized)
    }

    pub fn encoded_len(&self) -> usize {
        4 + self.coefficient.encoded_len()
    }

    pub fn deterministic_bytes(&self) -> Vec<u8> {
        let mut bytes = self.coefficient.deterministic_bytes();
        bytes.extend_from_slice(&self.scale.to_be_bytes());
        bytes
    }
}

fn pow10(power: u32) -> BigInt {
    BigInt::from(10u8).pow(power)
}

fn decimal_digits(value: &BigInt) -> usize {
    value.abs().to_str_radix(10).len()
}

fn check_wire_len(actual: usize, limits: &NumericLimits) -> Result<(), NumericError> {
    check_limit(LimitKind::WireBytes, limits.max_wire_bytes, actual)
}

fn check_limit(kind: LimitKind, limit: usize, actual: usize) -> Result<(), NumericError> {
    if actual > limit {
        return Err(NumericError::LimitExceeded {
            kind,
            limit: limit as u64,
            actual: actual as u64,
        });
    }
    Ok(())
}

fn check_limit_u32(kind: LimitKind, limit: u32, actual: u32) -> Result<(), NumericError> {
    if actual > limit {
        return Err(NumericError::LimitExceeded {
            kind,
            limit: u64::from(limit),
            actual: u64::from(actual),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_arithmetic_exceeds_fixed_width_without_overflow() {
        let limits = NumericLimits::prototype_default();
        let left = (BigInt::from(1u8) << 512) - 1u8;
        let result = checked_int_add(&left, &BigInt::from(1u8), &limits).unwrap();
        assert_eq!(result, BigInt::from(1u8) << 512);
        assert!(int_to_i64(&result).is_err());
    }

    #[test]
    fn int_wire_round_trips_unique_sign_and_magnitude() {
        let limits = NumericLimits::test_small();
        for raw in -1_000..=1_000 {
            let value = BigInt::from(raw);
            let wire = CanonicalInt::from_bigint(&value, &limits).unwrap();
            assert_eq!(wire.to_bigint(&limits).unwrap(), value);
        }
    }

    #[test]
    fn int_wire_rejects_negative_zero_and_leading_zero() {
        let limits = NumericLimits::test_small();
        let negative_zero = CanonicalInt {
            sign: IntegerSign::Negative,
            magnitude_be: vec![],
        };
        let leading_zero = CanonicalInt {
            sign: IntegerSign::Positive,
            magnitude_be: vec![0, 1],
        };
        assert!(matches!(
            negative_zero.to_bigint(&limits),
            Err(NumericError::NonCanonical(_))
        ));
        assert!(matches!(
            leading_zero.to_bigint(&limits),
            Err(NumericError::NonCanonical(_))
        ));
    }

    #[test]
    fn int_result_limit_is_checked_after_exact_operation() {
        let limits = NumericLimits {
            max_int_bytes: 1,
            ..NumericLimits::test_small()
        };
        let error =
            checked_int_mul(&BigInt::from(255u16), &BigInt::from(2u8), &limits).unwrap_err();
        assert!(matches!(
            error,
            NumericError::LimitExceeded {
                kind: LimitKind::IntBytes,
                ..
            }
        ));
    }

    #[test]
    fn decimal_normalizes_and_adds_exactly() {
        let limits = NumericLimits::test_small();
        let left = ExactDecimal::parse("1.2300", &limits).unwrap();
        let right = ExactDecimal::parse("0.07", &limits).unwrap();
        assert_eq!(left.to_canonical_string(), "1.23");
        assert_eq!(
            left.add(&right, &limits).unwrap().to_canonical_string(),
            "1.3"
        );
    }

    #[test]
    fn decimal_multiplies_without_binary_float() {
        let limits = NumericLimits::test_small();
        let left = ExactDecimal::parse("0.1", &limits).unwrap();
        let right = ExactDecimal::parse("0.2", &limits).unwrap();
        assert_eq!(
            left.mul(&right, &limits).unwrap().to_canonical_string(),
            "0.02"
        );
    }

    #[test]
    fn decimal_half_even_rounding_is_explicit() {
        let limits = NumericLimits::test_small();
        let even = ExactDecimal::parse("2.345", &limits).unwrap();
        let odd = ExactDecimal::parse("2.355", &limits).unwrap();
        assert_eq!(
            even.round(2, RoundingMode::HalfEven, &limits)
                .unwrap()
                .to_canonical_string(),
            "2.34"
        );
        assert_eq!(
            odd.round(2, RoundingMode::HalfEven, &limits)
                .unwrap()
                .to_canonical_string(),
            "2.36"
        );
    }

    #[test]
    fn decimal_wire_round_trips_and_rejects_trailing_zero() {
        let limits = NumericLimits::test_small();
        let value = ExactDecimal::parse("-12.34", &limits).unwrap();
        let wire = value.to_wire(&limits).unwrap();
        assert_eq!(wire.to_decimal(&limits).unwrap(), value);

        let non_canonical = DecimalWire {
            coefficient: CanonicalInt::from_bigint(&BigInt::from(1230), &limits).unwrap(),
            scale: 3,
        };
        assert!(matches!(
            non_canonical.to_decimal(&limits),
            Err(NumericError::NonCanonical(_))
        ));
    }

    #[test]
    fn decimal_input_and_scale_limits_fail_locally() {
        let limits = NumericLimits {
            max_decimal_digits: 4,
            max_decimal_scale: 2,
            ..NumericLimits::test_small()
        };
        assert!(matches!(
            ExactDecimal::parse("12345", &limits),
            Err(NumericError::LimitExceeded {
                kind: LimitKind::DecimalDigits,
                ..
            })
        ));
        assert!(matches!(
            ExactDecimal::parse("0.001", &limits),
            Err(NumericError::LimitExceeded {
                kind: LimitKind::DecimalScale,
                ..
            })
        ));
    }

    #[test]
    fn decimal_canonical_text_is_idempotent() {
        let limits = NumericLimits::test_small();
        for text in ["0", "-0", "0001.2300", "-0.00100", "999999", "12.00001"] {
            let first = ExactDecimal::parse(text, &limits).unwrap();
            let canonical = first.to_canonical_string();
            let second = ExactDecimal::parse(&canonical, &limits).unwrap();
            assert_eq!(first, second, "{text} -> {canonical}");
        }
    }
}
