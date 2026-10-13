use crate::{
    CalendarDuration, ClockTime, ComplexNumber, CompoundUnit, Currency, CustomUnitDefinition, Date,
    DiceDistribution, Number, OffsetClockTime, OffsetDateTime, Radix, RadixInteger, RomanInteger,
    Span, UtcOffset, Weekday, WorkdayDuration, ZonedClockTime,
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
    Probability,
    ExpectedValue,
    Roll,
    CoinFlip,
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
    /// Returns whether the function can be called without parentheses or arguments.
    pub const fn zero_argument(self) -> bool {
        matches!(self, Self::CoinFlip)
    }

    /// Every function name and its aliases. The first name in each entry is
    /// the canonical form offered by completion.
    pub(crate) const NAMES: &[(&[&str], Function)] = &[
        (&["sales_tax", "tax"], Function::SalesTax),
        (&["add_tax"], Function::AddTax),
        (&["remove_tax"], Function::RemoveTax),
        (&["tax_from_total"], Function::TaxFromTotal),
        (&["future_value"], Function::FutureValue),
        (&["compound_interest"], Function::CompoundInterest),
        (&["present_value"], Function::PresentValue),
        (&["cagr"], Function::CompoundAnnualGrowthRate),
        (&["roi"], Function::ReturnOnInvestment),
        (&["mortgage_payment", "loan_payment"], Function::MortgagePayment),
        (&["total_repayment"], Function::TotalRepayment),
        (&["total_interest"], Function::TotalInterest),
        (&["purchasing_power"], Function::PurchasingPower),
        (&["break_even"], Function::BreakEven),
        (&["clamp"], Function::Clamp),
        (&["midpoint"], Function::Midpoint),
        (&["proportion"], Function::ProportionDenominator),
        (
            &["gcd", "greatest_common_divisor"],
            Function::GreatestCommonDivisor,
        ),
        (&["lcm", "least_common_multiple"], Function::LeastCommonMultiple),
        (
            &["is_prime", "isprime", "prime"],
            Function::IsPrime,
        ),
        (&["probability"], Function::Probability),
        (
            &["expected_value", "expectedvalue"],
            Function::ExpectedValue,
        ),
        (&["roll"], Function::Roll),
        (&["coinflip", "coin_flip"], Function::CoinFlip),
        (&["factorial"], Function::Factorial),
        (
            &["permutation", "permutations", "perm"],
            Function::Permutation,
        ),
        (
            &["combination", "combinations", "choose"],
            Function::Combination,
        ),
        (&["bin", "binary"], Function::Binary),
        (&["oct", "octal"], Function::Octal),
        (&["hex", "hexadecimal"], Function::Hexadecimal),
        (&["int", "integer"], Function::Integer),
        (&["sqrt"], Function::SquareRoot),
        (&["cbrt"], Function::CubicRoot),
        (&["root"], Function::NthRoot),
        (&["ln"], Function::NaturalLogarithm),
        (
            &["log", "log10", "logarithm"],
            Function::CommonLogarithm,
        ),
        (&["log2"], Function::BinaryLogarithm),
        (&["sin", "sine"], Function::Sine),
        (&["cos", "cosine"], Function::Cosine),
        (&["tan", "tangent"], Function::Tangent),
        (&["asin", "arcsine"], Function::ArcSine),
        (&["acos", "arccosine"], Function::ArcCosine),
        (&["atan", "arctangent"], Function::ArcTangent),
        (&["atan2"], Function::ArcTangent2),
        (&["exp", "exponential"], Function::Exponential),
        (&["sinh"], Function::HyperbolicSine),
        (&["cosh"], Function::HyperbolicCosine),
        (&["tanh"], Function::HyperbolicTangent),
        (
            &["asinh"],
            Function::InverseHyperbolicSine,
        ),
        (
            &["acosh"],
            Function::InverseHyperbolicCosine,
        ),
        (
            &["atanh"],
            Function::InverseHyperbolicTangent,
        ),
        (&["abs"], Function::AbsoluteValue),
        (&["sum"], Function::Sum),
        (&["average", "avg"], Function::Average),
        (&["minimum", "min"], Function::Minimum),
        (&["maximum", "max"], Function::Maximum),
        (&["median"], Function::Median),
        (&["count"], Function::Count),
        (&["product"], Function::Product),
        (
            &["weekday", "dayofweek", "day_of_week"],
            Function::Weekday,
        ),
        (&["isworkday", "is_workday"], Function::IsWorkday),
        (
            &[
                "addworkdays",
                "add_workdays",
                "addbusinessdays",
                "add_business_days",
            ],
            Function::AddWorkdays,
        ),
        (
            &[
                "workdaysbetween",
                "workdays_between",
                "businessdaysbetween",
                "business_days_between",
            ],
            Function::WorkdaysBetween,
        ),
        (&["nextworkday", "next_workday"], Function::NextWorkday),
        (
            &["previousworkday", "previous_workday"],
            Function::PreviousWorkday,
        ),
        (
            &["timebetween", "time_between"],
            Function::TimeBetween,
        ),
        (&["timefrom", "time_from"], Function::TimeFrom),
        (&["timesince", "time_since"], Function::TimeSince),
        (&["timeuntil", "time_until"], Function::TimeUntil),
        (
            &["calendarbetween", "calendar_between"],
            Function::CalendarBetween,
        ),
        (&["calendarfrom", "calendar_from"], Function::CalendarFrom),
        (
            &["calendarsince", "calendar_since"],
            Function::CalendarSince,
        ),
        (
            &["calendaruntil", "calendar_until"],
            Function::CalendarUntil,
        ),
        (&["round"], Function::Round),
        (&["floor"], Function::Floor),
        (&["ceiling", "ceil"], Function::Ceiling),
    ];

    /// Returns the canonical name of every function.
    pub fn canonical_names() -> impl Iterator<Item = &'static str> {
        Self::NAMES.iter().map(|(names, _)| names[0])
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        let lowered = name.to_ascii_lowercase();
        Self::NAMES
            .iter()
            .find(|(names, _)| names.contains(&lowered.as_str()))
            .map(|(_, function)| *function)
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

#[derive(Clone, Debug, PartialEq)]
pub enum ConversionTarget {
    Number,
    Percentage,
    Fraction,
    Radix(Radix),
    Roman,
    CustomUnit(CustomUnitDefinition),
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
    String {
        value: String,
        span: Span,
    },
    RadixInteger {
        value: RadixInteger,
        span: Span,
    },
    RomanInteger {
        value: RomanInteger,
        span: Span,
    },
    Complex {
        value: ComplexNumber,
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
    CustomUnit {
        value: Number,
        definition: CustomUnitDefinition,
        span: Span,
    },
    Money {
        amount: Number,
        currency: Currency,
        span: Span,
    },
    CurrencyAttach {
        expression: Box<Expression>,
        currency: Currency,
        currency_span: Span,
        span: Span,
    },
    Dice {
        value: DiceDistribution,
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
    UserCall {
        name: String,
        name_span: Span,
        arguments: Vec<Expression>,
        span: Span,
    },
    Lambda {
        parameters: Vec<String>,
        body: Box<Expression>,
        span: Span,
    },
    Invoke {
        function: Box<Expression>,
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
            | Self::String { span, .. }
            | Self::RadixInteger { span, .. }
            | Self::RomanInteger { span, .. }
            | Self::Complex { span, .. }
            | Self::Boolean { span, .. }
            | Self::Constant { span, .. }
            | Self::Unit { span, .. }
            | Self::CustomUnit { span, .. }
            | Self::Money { span, .. }
            | Self::CurrencyAttach { span, .. }
            | Self::Dice { span, .. }
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
            | Self::UserCall { span, .. }
            | Self::Lambda { span, .. }
            | Self::Invoke { span, .. }
            | Self::Unary { span, .. }
            | Self::Group { span, .. }
            | Self::Percentage { span, .. }
            | Self::Conversion { span, .. }
            | Self::Conditional { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}
