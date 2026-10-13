use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency {
    code: &'static str,
    minor_units: Option<u8>,
}

#[allow(non_upper_case_globals)]
impl Currency {
    pub const Usd: Self = Self::new("USD", Some(2));
    pub const Eur: Self = Self::new("EUR", Some(2));
    pub const Gbp: Self = Self::new("GBP", Some(2));
    pub const Jpy: Self = Self::new("JPY", Some(0));
    pub const Cad: Self = Self::new("CAD", Some(2));
    pub const Aud: Self = Self::new("AUD", Some(2));
    pub const Chf: Self = Self::new("CHF", Some(2));
    pub const Cny: Self = Self::new("CNY", Some(2));
    pub const Inr: Self = Self::new("INR", Some(2));
    pub const Krw: Self = Self::new("KRW", Some(0));

    const fn new(code: &'static str, minor_units: Option<u8>) -> Self {
        Self { code, minor_units }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        CURRENCIES
            .iter()
            .copied()
            .find(|currency| currency.code.eq_ignore_ascii_case(code))
    }

    /// Resolves common English currency names, such as `rmb` for CNY.
    ///
    /// Ambiguous names need a leading adjective (`mexican pesos`,
    /// `swedish krona`). Calcite rejects `peso`, `krona`, and `krone` on
    /// their own. `pound` on its own stays a mass unit.
    pub fn from_alias(name: &str) -> Option<Self> {
        const ALIASES: &[(&str, &str)] = &[
            ("rmb", "CNY"),
            ("renminbi", "CNY"),
            ("yuan", "CNY"),
            ("dollar", "USD"),
            ("dollars", "USD"),
            ("us dollar", "USD"),
            ("us dollars", "USD"),
            ("euro", "EUR"),
            ("euros", "EUR"),
            ("sterling", "GBP"),
            ("pound sterling", "GBP"),
            ("british pound", "GBP"),
            ("british pounds", "GBP"),
            ("yen", "JPY"),
            ("won", "KRW"),
            ("rupee", "INR"),
            ("rupees", "INR"),
            ("indian rupee", "INR"),
            ("indian rupees", "INR"),
            ("franc", "CHF"),
            ("francs", "CHF"),
            ("swiss franc", "CHF"),
            ("swiss francs", "CHF"),
            ("ruble", "RUB"),
            ("rouble", "RUB"),
            ("rubles", "RUB"),
            ("russian ruble", "RUB"),
            ("russian rubles", "RUB"),
            ("rand", "ZAR"),
            ("baht", "THB"),
            ("ringgit", "MYR"),
            ("rupiah", "IDR"),
            ("shekel", "ILS"),
            ("shekels", "ILS"),
            ("israeli shekel", "ILS"),
            ("israeli shekels", "ILS"),
            ("lira", "TRY"),
            ("lire", "TRY"),
            ("turkish lira", "TRY"),
            ("zloty", "PLN"),
            ("zlotys", "PLN"),
            ("forint", "HUF"),
            ("koruna", "CZK"),
            ("czech koruna", "CZK"),
            ("dong", "VND"),
            ("mexican peso", "MXN"),
            ("mexican pesos", "MXN"),
            ("argentine peso", "ARS"),
            ("argentine pesos", "ARS"),
            ("philippine peso", "PHP"),
            ("philippine pesos", "PHP"),
            ("colombian peso", "COP"),
            ("colombian pesos", "COP"),
            ("uruguayan peso", "UYU"),
            ("uruguayan pesos", "UYU"),
            ("swedish krona", "SEK"),
            ("swedish kronor", "SEK"),
            ("danish krone", "DKK"),
            ("danish kroner", "DKK"),
            ("norwegian krone", "NOK"),
            ("norwegian kroner", "NOK"),
            ("australian dollar", "AUD"),
            ("australian dollars", "AUD"),
            ("canadian dollar", "CAD"),
            ("canadian dollars", "CAD"),
            ("new zealand dollar", "NZD"),
            ("new zealand dollars", "NZD"),
            ("hong kong dollar", "HKD"),
            ("hong kong dollars", "HKD"),
            ("singapore dollar", "SGD"),
            ("singapore dollars", "SGD"),
        ];
        ALIASES
            .iter()
            .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
            .and_then(|(_, code)| Self::from_code(code))
    }

    pub fn from_symbol(symbol: &str, default_currency: Self) -> Option<Self> {
        match symbol {
            "$" => Some(default_currency),
            "€" => Some(Self::Eur),
            "£" => Some(Self::Gbp),
            "¥" if matches!(default_currency.code(), "JPY" | "CNY") => Some(default_currency),
            "¥" => Some(Self::Jpy),
            "₹" => Some(Self::Inr),
            "₩" if matches!(default_currency.code(), "KRW" | "KPW") => Some(default_currency),
            "₩" => Some(Self::Krw),
            _ => None,
        }
    }

    pub const fn code(self) -> &'static str {
        self.code
    }

    pub const fn minor_units(self) -> Option<u8> {
        self.minor_units
    }

    pub const fn all() -> &'static [Self] {
        CURRENCIES
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

// ISO 4217 List One from SIX, dated 2026-01-01. Minor units apply to display only.
const CURRENCIES: &[Currency] = &[
    Currency::new("AED", Some(2)),
    Currency::new("AFN", Some(2)),
    Currency::new("ALL", Some(2)),
    Currency::new("AMD", Some(2)),
    Currency::new("AOA", Some(2)),
    Currency::new("ARS", Some(2)),
    Currency::new("AUD", Some(2)),
    Currency::new("AWG", Some(2)),
    Currency::new("AZN", Some(2)),
    Currency::new("BAM", Some(2)),
    Currency::new("BBD", Some(2)),
    Currency::new("BDT", Some(2)),
    Currency::new("BHD", Some(3)),
    Currency::new("BIF", Some(0)),
    Currency::new("BMD", Some(2)),
    Currency::new("BND", Some(2)),
    Currency::new("BOB", Some(2)),
    Currency::new("BOV", Some(2)),
    Currency::new("BRL", Some(2)),
    Currency::new("BSD", Some(2)),
    Currency::new("BTN", Some(2)),
    Currency::new("BWP", Some(2)),
    Currency::new("BYN", Some(2)),
    Currency::new("BZD", Some(2)),
    Currency::new("CAD", Some(2)),
    Currency::new("CDF", Some(2)),
    Currency::new("CHE", Some(2)),
    Currency::new("CHF", Some(2)),
    Currency::new("CHW", Some(2)),
    Currency::new("CLF", Some(4)),
    Currency::new("CLP", Some(0)),
    Currency::new("CNY", Some(2)),
    Currency::new("COP", Some(2)),
    Currency::new("COU", Some(2)),
    Currency::new("CRC", Some(2)),
    Currency::new("CUP", Some(2)),
    Currency::new("CVE", Some(2)),
    Currency::new("CZK", Some(2)),
    Currency::new("DJF", Some(0)),
    Currency::new("DKK", Some(2)),
    Currency::new("DOP", Some(2)),
    Currency::new("DZD", Some(2)),
    Currency::new("EGP", Some(2)),
    Currency::new("ERN", Some(2)),
    Currency::new("ETB", Some(2)),
    Currency::new("EUR", Some(2)),
    Currency::new("FJD", Some(2)),
    Currency::new("FKP", Some(2)),
    Currency::new("GBP", Some(2)),
    Currency::new("GEL", Some(2)),
    Currency::new("GHS", Some(2)),
    Currency::new("GIP", Some(2)),
    Currency::new("GMD", Some(2)),
    Currency::new("GNF", Some(0)),
    Currency::new("GTQ", Some(2)),
    Currency::new("GYD", Some(2)),
    Currency::new("HKD", Some(2)),
    Currency::new("HNL", Some(2)),
    Currency::new("HTG", Some(2)),
    Currency::new("HUF", Some(2)),
    Currency::new("IDR", Some(2)),
    Currency::new("ILS", Some(2)),
    Currency::new("INR", Some(2)),
    Currency::new("IQD", Some(3)),
    Currency::new("IRR", Some(2)),
    Currency::new("ISK", Some(0)),
    Currency::new("JMD", Some(2)),
    Currency::new("JOD", Some(3)),
    Currency::new("JPY", Some(0)),
    Currency::new("KES", Some(2)),
    Currency::new("KGS", Some(2)),
    Currency::new("KHR", Some(2)),
    Currency::new("KMF", Some(0)),
    Currency::new("KPW", Some(2)),
    Currency::new("KRW", Some(0)),
    Currency::new("KWD", Some(3)),
    Currency::new("KYD", Some(2)),
    Currency::new("KZT", Some(2)),
    Currency::new("LAK", Some(2)),
    Currency::new("LBP", Some(2)),
    Currency::new("LKR", Some(2)),
    Currency::new("LRD", Some(2)),
    Currency::new("LSL", Some(2)),
    Currency::new("LYD", Some(3)),
    Currency::new("MAD", Some(2)),
    Currency::new("MDL", Some(2)),
    Currency::new("MGA", Some(2)),
    Currency::new("MKD", Some(2)),
    Currency::new("MMK", Some(2)),
    Currency::new("MNT", Some(2)),
    Currency::new("MOP", Some(2)),
    Currency::new("MRU", Some(2)),
    Currency::new("MUR", Some(2)),
    Currency::new("MVR", Some(2)),
    Currency::new("MWK", Some(2)),
    Currency::new("MXN", Some(2)),
    Currency::new("MXV", Some(2)),
    Currency::new("MYR", Some(2)),
    Currency::new("MZN", Some(2)),
    Currency::new("NAD", Some(2)),
    Currency::new("NGN", Some(2)),
    Currency::new("NIO", Some(2)),
    Currency::new("NOK", Some(2)),
    Currency::new("NPR", Some(2)),
    Currency::new("NZD", Some(2)),
    Currency::new("OMR", Some(3)),
    Currency::new("PAB", Some(2)),
    Currency::new("PEN", Some(2)),
    Currency::new("PGK", Some(2)),
    Currency::new("PHP", Some(2)),
    Currency::new("PKR", Some(2)),
    Currency::new("PLN", Some(2)),
    Currency::new("PYG", Some(0)),
    Currency::new("QAR", Some(2)),
    Currency::new("RON", Some(2)),
    Currency::new("RSD", Some(2)),
    Currency::new("RUB", Some(2)),
    Currency::new("RWF", Some(0)),
    Currency::new("SAR", Some(2)),
    Currency::new("SBD", Some(2)),
    Currency::new("SCR", Some(2)),
    Currency::new("SDG", Some(2)),
    Currency::new("SEK", Some(2)),
    Currency::new("SGD", Some(2)),
    Currency::new("SHP", Some(2)),
    Currency::new("SLE", Some(2)),
    Currency::new("SOS", Some(2)),
    Currency::new("SRD", Some(2)),
    Currency::new("SSP", Some(2)),
    Currency::new("STN", Some(2)),
    Currency::new("SVC", Some(2)),
    Currency::new("SYP", Some(2)),
    Currency::new("SZL", Some(2)),
    Currency::new("THB", Some(2)),
    Currency::new("TJS", Some(2)),
    Currency::new("TMT", Some(2)),
    Currency::new("TND", Some(3)),
    Currency::new("TOP", Some(2)),
    Currency::new("TRY", Some(2)),
    Currency::new("TTD", Some(2)),
    Currency::new("TWD", Some(2)),
    Currency::new("TZS", Some(2)),
    Currency::new("UAH", Some(2)),
    Currency::new("UGX", Some(0)),
    Currency::new("USD", Some(2)),
    Currency::new("USN", Some(2)),
    Currency::new("UYI", Some(0)),
    Currency::new("UYU", Some(2)),
    Currency::new("UYW", Some(4)),
    Currency::new("UZS", Some(2)),
    Currency::new("VED", Some(2)),
    Currency::new("VES", Some(2)),
    Currency::new("VND", Some(0)),
    Currency::new("VUV", Some(0)),
    Currency::new("WST", Some(2)),
    Currency::new("XAD", Some(2)),
    Currency::new("XAF", Some(0)),
    Currency::new("XAG", None),
    Currency::new("XAU", None),
    Currency::new("XBA", None),
    Currency::new("XBB", None),
    Currency::new("XBC", None),
    Currency::new("XBD", None),
    Currency::new("XCD", Some(2)),
    Currency::new("XCG", Some(2)),
    Currency::new("XDR", None),
    Currency::new("XOF", Some(0)),
    Currency::new("XPD", None),
    Currency::new("XPF", Some(0)),
    Currency::new("XPT", None),
    Currency::new("XSU", None),
    Currency::new("XTS", None),
    Currency::new("XUA", None),
    Currency::new("XXX", None),
    Currency::new("YER", Some(2)),
    Currency::new("ZAR", Some(2)),
    Currency::new("ZMW", Some(2)),
    Currency::new("ZWG", Some(2)),
];
