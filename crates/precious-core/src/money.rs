use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Currency {
    #[default]
    USD,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Currency::USD => write!(f, "$"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub currency: Currency,
    pub amount: Decimal,
}

impl Money {
    pub fn usd(amount: Decimal) -> Self {
        Self {
            currency: Currency::USD,
            amount,
        }
    }

    pub fn zero() -> Self {
        Self {
            currency: Currency::USD,
            amount: Decimal::ZERO,
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:.2}", self.currency, self.amount)
    }
}

impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        debug_assert_eq!(self.currency, rhs.currency, "currency mismatch in addition");
        Self {
            currency: self.currency,
            amount: self.amount + rhs.amount,
        }
    }
}

impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        debug_assert_eq!(
            self.currency, rhs.currency,
            "currency mismatch in subtraction"
        );
        Self {
            currency: self.currency,
            amount: self.amount - rhs.amount,
        }
    }
}

impl Mul<Decimal> for Money {
    type Output = Self;
    fn mul(self, rhs: Decimal) -> Self {
        Self {
            currency: self.currency,
            amount: self.amount * rhs,
        }
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Money::zero(), |acc, m| acc + m)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BillingPeriod {
    Hour,
    Month,
    Year,
    GBMonth,
    Request,
    GBSecond,
    IOPS,
}

impl BillingPeriod {
    pub fn hours_per_month() -> Decimal {
        Decimal::from(730)
    }
}

impl fmt::Display for BillingPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BillingPeriod::Hour => write!(f, "hours"),
            BillingPeriod::Month => write!(f, "months"),
            BillingPeriod::Year => write!(f, "years"),
            BillingPeriod::GBMonth => write!(f, "GB/month"),
            BillingPeriod::Request => write!(f, "requests"),
            BillingPeriod::GBSecond => write!(f, "GB-seconds"),
            BillingPeriod::IOPS => write!(f, "IOPS"),
        }
    }
}
