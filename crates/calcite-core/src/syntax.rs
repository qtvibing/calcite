use crate::{
    CalendarDuration, ClockTime, CompoundUnit, Currency, Date, Number, OffsetDateTime, Radix,
    RadixInteger, Span, UtcOffset, Weekday, WorkdayDuration,
};

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
            Self::ShiftLeft | Self::ShiftRight => (13, 14),
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
            | Self::Off
            | Self::At => (17, 18),
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
        if name.eq_ignore_ascii_case("sales_tax") || name.eq_ignore_ascii_case("tax") {
            Some(Self::SalesTax)
        } else if name.eq_ignore_ascii_case("add_tax") {
            Some(Self::AddTax)
        } else if name.eq_ignore_ascii_case("remove_tax") {
            Some(Self::RemoveTax)
        } else if name.eq_ignore_ascii_case("tax_from_total") {
            Some(Self::TaxFromTotal)
        } else if name.eq_ignore_ascii_case("future_value") {
            Some(Self::FutureValue)
        } else if name.eq_ignore_ascii_case("compound_interest") {
            Some(Self::CompoundInterest)
        } else if name.eq_ignore_ascii_case("present_value") {
            Some(Self::PresentValue)
        } else if name.eq_ignore_ascii_case("cagr") {
            Some(Self::CompoundAnnualGrowthRate)
        } else if name.eq_ignore_ascii_case("roi") {
            Some(Self::ReturnOnInvestment)
        } else if name.eq_ignore_ascii_case("mortgage_payment")
            || name.eq_ignore_ascii_case("loan_payment")
        {
            Some(Self::MortgagePayment)
        } else if name.eq_ignore_ascii_case("total_repayment") {
            Some(Self::TotalRepayment)
        } else if name.eq_ignore_ascii_case("total_interest") {
            Some(Self::TotalInterest)
        } else if name.eq_ignore_ascii_case("purchasing_power") {
            Some(Self::PurchasingPower)
        } else if name.eq_ignore_ascii_case("break_even") {
            Some(Self::BreakEven)
        } else if name.eq_ignore_ascii_case("clamp") {
            Some(Self::Clamp)
        } else if name.eq_ignore_ascii_case("midpoint") {
            Some(Self::Midpoint)
        } else if name.eq_ignore_ascii_case("proportion") {
            Some(Self::ProportionDenominator)
        } else if name.eq_ignore_ascii_case("gcd")
            || name.eq_ignore_ascii_case("greatest_common_divisor")
        {
            Some(Self::GreatestCommonDivisor)
        } else if name.eq_ignore_ascii_case("lcm")
            || name.eq_ignore_ascii_case("least_common_multiple")
        {
            Some(Self::LeastCommonMultiple)
        } else if name.eq_ignore_ascii_case("is_prime")
            || name.eq_ignore_ascii_case("isprime")
            || name.eq_ignore_ascii_case("prime")
        {
            Some(Self::IsPrime)
        } else if name.eq_ignore_ascii_case("factorial") {
            Some(Self::Factorial)
        } else if name.eq_ignore_ascii_case("permutation")
            || name.eq_ignore_ascii_case("permutations")
            || name.eq_ignore_ascii_case("perm")
        {
            Some(Self::Permutation)
        } else if name.eq_ignore_ascii_case("combination")
            || name.eq_ignore_ascii_case("combinations")
            || name.eq_ignore_ascii_case("choose")
        {
            Some(Self::Combination)
        } else if name.eq_ignore_ascii_case("bin") || name.eq_ignore_ascii_case("binary") {
            Some(Self::Binary)
        } else if name.eq_ignore_ascii_case("oct") || name.eq_ignore_ascii_case("octal") {
            Some(Self::Octal)
        } else if name.eq_ignore_ascii_case("hex") || name.eq_ignore_ascii_case("hexadecimal") {
            Some(Self::Hexadecimal)
        } else if name.eq_ignore_ascii_case("int") || name.eq_ignore_ascii_case("integer") {
            Some(Self::Integer)
        } else if name.eq_ignore_ascii_case("sqrt") {
            Some(Self::SquareRoot)
        } else if name.eq_ignore_ascii_case("cbrt") {
            Some(Self::CubicRoot)
        } else if name.eq_ignore_ascii_case("root") {
            Some(Self::NthRoot)
        } else if name.eq_ignore_ascii_case("ln") {
            Some(Self::NaturalLogarithm)
        } else if name.eq_ignore_ascii_case("log")
            || name.eq_ignore_ascii_case("log10")
            || name.eq_ignore_ascii_case("logarithm")
        {
            Some(Self::CommonLogarithm)
        } else if name.eq_ignore_ascii_case("log2") {
            Some(Self::BinaryLogarithm)
        } else if name.eq_ignore_ascii_case("sin") || name.eq_ignore_ascii_case("sine") {
            Some(Self::Sine)
        } else if name.eq_ignore_ascii_case("cos") || name.eq_ignore_ascii_case("cosine") {
            Some(Self::Cosine)
        } else if name.eq_ignore_ascii_case("tan") || name.eq_ignore_ascii_case("tangent") {
            Some(Self::Tangent)
        } else if name.eq_ignore_ascii_case("asin") || name.eq_ignore_ascii_case("arcsine") {
            Some(Self::ArcSine)
        } else if name.eq_ignore_ascii_case("acos") || name.eq_ignore_ascii_case("arccosine") {
            Some(Self::ArcCosine)
        } else if name.eq_ignore_ascii_case("atan") || name.eq_ignore_ascii_case("arctangent") {
            Some(Self::ArcTangent)
        } else if name.eq_ignore_ascii_case("atan2") {
            Some(Self::ArcTangent2)
        } else if name.eq_ignore_ascii_case("exp") || name.eq_ignore_ascii_case("exponential") {
            Some(Self::Exponential)
        } else if name.eq_ignore_ascii_case("sinh") {
            Some(Self::HyperbolicSine)
        } else if name.eq_ignore_ascii_case("cosh") {
            Some(Self::HyperbolicCosine)
        } else if name.eq_ignore_ascii_case("tanh") {
            Some(Self::HyperbolicTangent)
        } else if name.eq_ignore_ascii_case("asinh") {
            Some(Self::InverseHyperbolicSine)
        } else if name.eq_ignore_ascii_case("acosh") {
            Some(Self::InverseHyperbolicCosine)
        } else if name.eq_ignore_ascii_case("atanh") {
            Some(Self::InverseHyperbolicTangent)
        } else if name.eq_ignore_ascii_case("abs") {
            Some(Self::AbsoluteValue)
        } else if name.eq_ignore_ascii_case("sum") {
            Some(Self::Sum)
        } else if name.eq_ignore_ascii_case("average") || name.eq_ignore_ascii_case("avg") {
            Some(Self::Average)
        } else if name.eq_ignore_ascii_case("minimum") || name.eq_ignore_ascii_case("min") {
            Some(Self::Minimum)
        } else if name.eq_ignore_ascii_case("maximum") || name.eq_ignore_ascii_case("max") {
            Some(Self::Maximum)
        } else if name.eq_ignore_ascii_case("median") {
            Some(Self::Median)
        } else if name.eq_ignore_ascii_case("count") {
            Some(Self::Count)
        } else if name.eq_ignore_ascii_case("product") {
            Some(Self::Product)
        } else if name.eq_ignore_ascii_case("weekday")
            || name.eq_ignore_ascii_case("dayofweek")
            || name.eq_ignore_ascii_case("day_of_week")
        {
            Some(Self::Weekday)
        } else if name.eq_ignore_ascii_case("isworkday") || name.eq_ignore_ascii_case("is_workday")
        {
            Some(Self::IsWorkday)
        } else if name.eq_ignore_ascii_case("addworkdays")
            || name.eq_ignore_ascii_case("add_workdays")
            || name.eq_ignore_ascii_case("addbusinessdays")
            || name.eq_ignore_ascii_case("add_business_days")
        {
            Some(Self::AddWorkdays)
        } else if name.eq_ignore_ascii_case("workdaysbetween")
            || name.eq_ignore_ascii_case("workdays_between")
            || name.eq_ignore_ascii_case("businessdaysbetween")
            || name.eq_ignore_ascii_case("business_days_between")
        {
            Some(Self::WorkdaysBetween)
        } else if name.eq_ignore_ascii_case("nextworkday")
            || name.eq_ignore_ascii_case("next_workday")
        {
            Some(Self::NextWorkday)
        } else if name.eq_ignore_ascii_case("previousworkday")
            || name.eq_ignore_ascii_case("previous_workday")
        {
            Some(Self::PreviousWorkday)
        } else if name.eq_ignore_ascii_case("round") {
            Some(Self::Round)
        } else if name.eq_ignore_ascii_case("floor") {
            Some(Self::Floor)
        } else if name.eq_ignore_ascii_case("ceiling") || name.eq_ignore_ascii_case("ceil") {
            Some(Self::Ceiling)
        } else {
            None
        }
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
    Clock {
        value: ClockTime,
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
            | Self::Clock { span, .. }
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
