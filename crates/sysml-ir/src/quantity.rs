//! Typed physical quantity values with explicit unit annotations.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// A scalar physical measurement carrying its SysML v2 unit symbol (e.g. `"kg"`, `"m/s"`, `"kN"`).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Quantity<T = f64> {
    pub value: T,
    pub unit: &'static str,
}

impl<T> Quantity<T> {
    pub const fn new(value: T, unit: &'static str) -> Self {
        Self { value, unit }
    }
}

impl Quantity<f64> {
    pub const fn kg(value: f64) -> Self {
        Self::new(value, "kg")
    }

    pub const fn m(value: f64) -> Self {
        Self::new(value, "m")
    }

    pub const fn m_per_s(value: f64) -> Self {
        Self::new(value, "m/s")
    }

    pub const fn kn(value: f64) -> Self {
        Self::new(value, "kN")
    }

    pub const fn watts(value: f64) -> Self {
        Self::new(value, "W")
    }

    pub const fn seconds(value: f64) -> Self {
        Self::new(value, "s")
    }

    pub const fn percent(value: f64) -> Self {
        Self::new(value, "%")
    }

    pub const fn per_hour(value: f64) -> Self {
        Self::new(value, "1/h")
    }

    pub fn value(self) -> f64 {
        self.value
    }
}

impl Default for Quantity<f64> {
    fn default() -> Self {
        Self {
            value: 0.0,
            unit: "",
        }
    }
}

impl fmt::Display for Quantity<f64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unit.is_empty() {
            write!(f, "{}", self.value)
        } else {
            write!(f, "{} [{}]", self.value, self.unit)
        }
    }
}

impl Add for Quantity<f64> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        debug_assert!(
            self.unit.is_empty() || rhs.unit.is_empty() || self.unit == rhs.unit,
            "unit mismatch: {} + {}",
            self.unit,
            rhs.unit
        );
        Self {
            value: self.value + rhs.value,
            unit: if self.unit.is_empty() {
                rhs.unit
            } else {
                self.unit
            },
        }
    }
}

impl Sub for Quantity<f64> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        debug_assert!(
            self.unit.is_empty() || rhs.unit.is_empty() || self.unit == rhs.unit,
            "unit mismatch: {} - {}",
            self.unit,
            rhs.unit
        );
        Self {
            value: self.value - rhs.value,
            unit: if self.unit.is_empty() {
                rhs.unit
            } else {
                self.unit
            },
        }
    }
}

impl Mul<f64> for Quantity<f64> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            value: self.value * rhs,
            unit: self.unit,
        }
    }
}

impl Div<f64> for Quantity<f64> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            value: self.value / rhs,
            unit: self.unit,
        }
    }
}
