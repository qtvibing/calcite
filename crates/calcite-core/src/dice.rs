use std::fmt;

use crate::{Number, Percentage};

const MAX_DICE_COUNT: u32 = 100;
const MAX_DIE_SIDES: u32 = 1_000_000;
const MAX_DISTRIBUTION_STATES: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiceDistribution {
    count: u32,
    sides: u32,
    modifier: i64,
}

impl DiceDistribution {
    pub const fn new(count: u32, sides: u32) -> Result<Self, DiceError> {
        if count == 0 || count > MAX_DICE_COUNT {
            return Err(DiceError::InvalidCount);
        }
        if sides < 2 || sides > MAX_DIE_SIDES {
            return Err(DiceError::InvalidSides);
        }
        match count.checked_mul(sides) {
            Some(states) if states as usize <= MAX_DISTRIBUTION_STATES => Ok(Self {
                count,
                sides,
                modifier: 0,
            }),
            _ => Err(DiceError::DistributionTooLarge),
        }
    }

    pub const fn count(self) -> u32 {
        self.count
    }

    pub const fn sides(self) -> u32 {
        self.sides
    }

    pub const fn modifier(self) -> i64 {
        self.modifier
    }

    pub const fn minimum(self) -> i64 {
        self.count as i64 + self.modifier
    }

    pub const fn maximum(self) -> i64 {
        self.count as i64 * self.sides as i64 + self.modifier
    }

    pub fn mean(self) -> Number {
        Number::from_i64(i64::from(self.count))
            .multiply(Number::from_i64(i64::from(self.sides) + 1))
            .divide(Number::from_i64(2))
            .expect("two is nonzero")
            .add(Number::from_i64(self.modifier))
    }

    pub fn with_modifier(self, additional: i64) -> Option<Self> {
        let modifier = self.modifier.checked_add(additional)?;
        if (self.count as i64).checked_add(modifier).is_none()
            || (self.count as i64)
                .checked_mul(self.sides as i64)
                .and_then(|maximum| maximum.checked_add(modifier))
                .is_none()
        {
            return None;
        }
        Some(Self { modifier, ..self })
    }

    pub fn probability(self, total: i64) -> Percentage {
        let target = total.checked_sub(self.modifier);
        let minimum = i64::from(self.count);
        let maximum = minimum * i64::from(self.sides);
        let favorable = match target {
            Some(target) if (minimum..=maximum).contains(&target) => {
                self.outcome_counts()[target as usize].clone()
            }
            _ => Number::from_i64(0),
        };
        let outcomes = Number::from_i64(i64::from(self.sides))
            .pow(Number::from_i64(i64::from(self.count)))
            .expect("bounded positive dice have a representable outcome count");
        Percentage::from_ratio(
            favorable
                .divide(outcomes)
                .expect("dice have at least one outcome"),
        )
    }

    pub fn roll(self, entropy: &mut impl EntropySource) -> Result<i64, DiceError> {
        let mut total = self.modifier;
        for _ in 0..self.count {
            let value = sample_bounded(entropy, u64::from(self.sides))?;
            total = total
                .checked_add(i64::try_from(value + 1).map_err(|_| DiceError::RollOverflow)?)
                .ok_or(DiceError::RollOverflow)?;
        }
        Ok(total)
    }

    fn outcome_counts(self) -> Vec<Number> {
        let maximum = self.count as usize * self.sides as usize;
        let mut counts = vec![Number::from_i64(0); maximum + 1];
        counts[0] = Number::from_i64(1);
        let mut reached = 0usize;
        for _ in 0..self.count {
            let mut next = vec![Number::from_i64(0); maximum + 1];
            for total in 0..=reached {
                for face in 1..=self.sides as usize {
                    next[total + face] = next[total + face].clone().add(counts[total].clone());
                }
            }
            reached += self.sides as usize;
            counts = next;
        }
        counts
    }
}

impl fmt::Display for DiceDistribution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}d{}", self.count, self.sides)?;
        if self.modifier > 0 {
            write!(formatter, "+{}", self.modifier)?;
        } else if self.modifier < 0 {
            write!(formatter, "{}", self.modifier)?;
        }
        Ok(())
    }
}

pub trait EntropySource {
    fn next_u64(&mut self) -> Option<u64>;
}

impl<I: Iterator<Item = u64>> EntropySource for I {
    fn next_u64(&mut self) -> Option<u64> {
        self.next()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoinSide {
    Heads,
    Tails,
}

impl fmt::Display for CoinSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Heads => "heads",
            Self::Tails => "tails",
        })
    }
}

pub fn flip_coin(entropy: &mut impl EntropySource) -> Result<CoinSide, DiceError> {
    Ok(if sample_bounded(entropy, 2)? == 0 {
        CoinSide::Heads
    } else {
        CoinSide::Tails
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiceError {
    InvalidCount,
    InvalidSides,
    DistributionTooLarge,
    EntropyExhausted,
    RollOverflow,
}

impl fmt::Display for DiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCount => "dice count must be between 1 and 100",
            Self::InvalidSides => "die sides must be between 2 and 1000000",
            Self::DistributionTooLarge => "dice distribution is too large",
            Self::EntropyExhausted => "the supplied entropy source was exhausted",
            Self::RollOverflow => "the dice result is too large",
        })
    }
}

impl std::error::Error for DiceError {}

fn sample_bounded(entropy: &mut impl EntropySource, upper: u64) -> Result<u64, DiceError> {
    let threshold = u64::MAX - u64::MAX % upper;
    loop {
        let value = entropy.next_u64().ok_or(DiceError::EntropyExhausted)?;
        if value < threshold {
            return Ok(value % upper);
        }
    }
}
