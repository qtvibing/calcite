use crate::{
    CalendarDuration, ClockTime, CompoundUnit, Currency, Date, Number, OffsetClockTime,
    OffsetDateTime, Radix, RadixInteger, Span, UtcOffset, Weekday, WorkdayDuration, ZonedClockTime,
};

pub(crate) fn identifier_key(name: &str) -> String {
    name.to_lowercase()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Positive,
    Negative,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Remainder,
    Permutation,
    Combination,
    PermutationsOf,
    CombinationsOf,
    Power,
    Of,
    On,
    Off,
    At,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    ShiftLeft,
    ShiftRight,
    And,
    Or,
}

impl BinaryOperator {
    pub(crate) const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Or => (1, 2),
            Self::And => (3, 4),
            Self::BitwiseOr => (5, 6),
            Self::BitwiseXor => (7, 8),
            Self::BitwiseAnd => (9, 10),
            Self::Equal
            | Self::NotEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::Less
            | Self::LessEqual => (11, 12),
            Self::ShiftLeft | Self::ShiftRight | Self::At => (13, 14),
            Self::Add | Self::Subtract => (15, 16),
            Self::Multiply
            | Self::Divide
            | Self::Modulo
            | Self::Remainder
            | Self::Permutation
            | Self::Combination
            | Self::PermutationsOf
            | Self::CombinationsOf
            | Self::Of
            | Self::On
            | Self::Off => (17, 18),
            Self::Power => (19, 19),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Function {
    SquareRoot,
    CubicRoot,
    NthRoot,
    NaturalLogarithm,
    CommonLogarithm,
    BinaryLogarithm,
    Sine,
    Cosine,
    Tangent,
    ArcSine,
    ArcCosine,
    ArcTangent,
    ArcTangent2,
    Exponential,
    HyperbolicSine,
    HyperbolicCosine,
    HyperbolicTangent,
    InverseHyperbolicSine,
    InverseHyperbolicCosine,
    InverseHyperbolicTangent,
    AbsoluteValue,
    Sum,
    Average,
    Minimum,
    Maximum,
    Median,
    Count,
    Product,
    Weekday,
    IsWorkday,
    AddWorkdays,
    WorkdaysBetween,
    NextWorkday,
    PreviousWorkday,
    TimeBetween,
    TimeFrom,
    TimeSince,
    TimeUntil,
    CalendarBetween,
    CalendarFrom,
    CalendarSince,
    CalendarUntil,
    Factorial,
    Permutation,
    Combination,
    Binary,
    Octal,
    Hexadecimal,
    Integer,
    GreatestCommonDivisor,
    LeastCommonMultiple,
    IsPrime,
    Clamp,
    Midpoint,
    ProportionDenominator,
    ProportionNumerator,
    SalesTax,
    AddTax,
    RemoveTax,
    TaxFromTotal,
    FutureValue,
    CompoundInterest,
    PresentValue,
    CompoundAnnualGrowthRate,
    ReturnOnInvestment,
    MortgagePayment,
    TotalRepayment,
    TotalInterest,
    PurchasingPower,
    BreakEven,
    Round,
    Floor,
    Ceiling,
}

impl Function {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "sales_tax" | "tax" => Self::SalesTax,
            "add_tax" => Self::AddTax,
            "remove_tax" => Self::RemoveTax,
            "tax_from_total" => Self::TaxFromTotal,
            "future_value" => Self::FutureValue,
            "compound_interest" => Self::CompoundInterest,
            "present_value" => Self::PresentValue,
            "cagr" => Self::CompoundAnnualGrowthRate,
            "roi" => Self::ReturnOnInvestment,
            "mortgage_payment" | "loan_payment" => Self::MortgagePayment,
            "total_repayment" => Self::TotalRepayment,
            "total_interest" => Self::TotalInterest,
            "purchasing_power" => Self::PurchasingPower,
            "break_even" => Self::BreakEven,
            "clamp" => Self::Clamp,
            "midpoint" => Self::Midpoint,
            "proportion" => Self::ProportionDenominator,
            "gcd" | "greatest_common_divisor" => Self::GreatestCommonDivisor,
            "lcm" | "least_common_multiple" => Self::LeastCommonMultiple,
            "is_prime" | "isprime" | "prime" => Self::IsPrime,
            "factorial" => Self::Factorial,
            "permutation" | "permutations" | "perm" => Self::Permutation,
            "combination" | "combinations" | "choose" => Self::Combination,
            "bin" | "binary" => Self::Binary,
            "oct" | "octal" => Self::Octal,
            "hex" | "hexadecimal" => Self::Hexadecimal,
            "int" | "integer" => Self::Integer,
            "sqrt" => Self::SquareRoot,
            "cbrt" => Self::CubicRoot,
            "root" => Self::NthRoot,
            "ln" => Self::NaturalLogarithm,
            "log" | "log10" | "logarithm" => Self::CommonLogarithm,
            "log2" => Self::BinaryLogarithm,
            "sin" | "sine" => Self::Sine,
            "cos" | "cosine" => Self::Cosine,
            "tan" | "tangent" => Self::Tangent,
            "asin" | "arcsine" => Self::ArcSine,
            "acos" | "arccosine" => Self::ArcCosine,
            "atan" | "arctangent" => Self::ArcTangent,
            "atan2" => Self::ArcTangent2,
            "exp" | "exponential" => Self::Exponential,
            "sinh" => Self::HyperbolicSine,
            "cosh" => Self::HyperbolicCosine,
            "tanh" => Self::HyperbolicTangent,
            "asinh" => Self::InverseHyperbolicSine,
            "acosh" => Self::InverseHyperbolicCosine,
            "atanh" => Self::InverseHyperbolicTangent,
            "abs" => Self::AbsoluteValue,
            "sum" => Self::Sum,
            "average" | "avg" => Self::Average,
            "minimum" | "min" => Self::Minimum,
            "maximum" | "max" => Self::Maximum,
            "median" => Self::Median,
            "count" => Self::Count,
            "product" => Self::Product,
            "weekday" | "dayofweek" | "day_of_week" => Self::Weekday,
            "isworkday" | "is_workday" => Self::IsWorkday,
            "addworkdays" | "add_workdays" | "addbusinessdays" | "add_business_days" => {
                Self::AddWorkdays
            }
            "workdaysbetween"
            | "workdays_between"
            | "businessdaysbetween"
            | "business_days_between" => Self::WorkdaysBetween,
            "nextworkday" | "next_workday" => Self::NextWorkday,
            "previousworkday" | "previous_workday" => Self::PreviousWorkday,
            "timebetween" | "time_between" => Self::TimeBetween,
            "timefrom" | "time_from" => Self::TimeFrom,
            "timesince" | "time_since" => Self::TimeSince,
            "timeuntil" | "time_until" => Self::TimeUntil,
            "calendarbetween" | "calendar_between" => Self::CalendarBetween,
            "calendarfrom" | "calendar_from" => Self::CalendarFrom,
            "calendarsince" | "calendar_since" => Self::CalendarSince,
            "calendaruntil" | "calendar_until" => Self::CalendarUntil,
            "round" => Self::Round,
            "floor" => Self::Floor,
            "ceiling" | "ceil" => Self::Ceiling,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constant {
    Pi,
    Tau,
    E,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalKeyword {
    Today,
    Tomorrow,
    Yesterday,
    Now,
    NextMonth,
    PreviousMonth,
    NextYear,
    PreviousYear,
    StartOfMonth,
    EndOfMonth,
    StartOfYear,
    EndOfYear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeWeekdayDirection {
    Next,
    Previous,
    This,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionTarget {
    Number,
    Percentage,
    Fraction,
    Radix(Radix),
    CompoundUnit(CompoundUnit),
    UtcOffset(UtcOffset),
    TimeZone(String),
    Currency(Currency),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Number {
        value: Number,
        span: Span,
    },
    RadixInteger {
        value: RadixInteger,
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    Constant {
        constant: Constant,
        span: Span,
    },
    Unit {
        value: Number,
        unit: CompoundUnit,
        span: Span,
    },
    Money {
        amount: Number,
        currency: Currency,
        span: Span,
    },
    Date {
        value: Date,
        span: Span,
    },
    PartialDate {
        month: u8,
        day: u8,
        span: Span,
    },
    Clock {
        value: ClockTime,
        span: Span,
    },
    OffsetClock {
        value: OffsetClockTime,
        span: Span,
    },
    ZonedClock {
        value: ZonedClockTime,
        span: Span,
    },
    OffsetDateTime {
        value: OffsetDateTime,
        span: Span,
    },
    CalendarDuration {
        value: CalendarDuration,
        span: Span,
    },
    WorkdayDuration {
        value: WorkdayDuration,
        span: Span,
    },
    TemporalKeyword {
        keyword: TemporalKeyword,
        span: Span,
    },
    RelativeWeekday {
        direction: RelativeWeekdayDirection,
        weekday: Weekday,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    Call {
        function: Function,
        function_span: Span,
        arguments: Vec<Expression>,
        span: Span,
    },
    Unary {
        operator: UnaryOperator,
        operator_span: Span,
        operand: Box<Expression>,
        span: Span,
    },
    Group {
        expression: Box<Expression>,
        span: Span,
    },
    Percentage {
        expression: Box<Expression>,
        operator_span: Span,
        span: Span,
    },
    Conversion {
        expression: Box<Expression>,
        target: ConversionTarget,
        operator_span: Span,
        target_span: Span,
        span: Span,
    },
    Conditional {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
        inverted: bool,
        keyword_span: Span,
        span: Span,
    },
    Binary {
        operator: BinaryOperator,
        operator_span: Span,
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
}

impl Expression {
    pub const fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::RadixInteger { span, .. }
            | Self::Boolean { span, .. }
            | Self::Constant { span, .. }
            | Self::Unit { span, .. }
            | Self::Money { span, .. }
            | Self::Date { span, .. }
            | Self::PartialDate { span, .. }
            | Self::Clock { span, .. }
            | Self::OffsetClock { span, .. }
            | Self::ZonedClock { span, .. }
            | Self::OffsetDateTime { span, .. }
            | Self::CalendarDuration { span, .. }
            | Self::WorkdayDuration { span, .. }
            | Self::TemporalKeyword { span, .. }
            | Self::RelativeWeekday { span, .. }
            | Self::Variable { span, .. }
            | Self::Call { span, .. }
            | Self::Unary { span, .. }
            | Self::Group { span, .. }
            | Self::Percentage { span, .. }
            | Self::Conversion { span, .. }
            | Self::Conditional { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}
