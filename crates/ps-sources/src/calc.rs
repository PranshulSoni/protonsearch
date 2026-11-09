//! In-process calculator, unit conversion and percentage evaluation.
//! OWNER: sources-content. No eval crates; recursive descent parser.
//!
//! The evaluator is deliberately strict: it only answers queries that are
//! unambiguously arithmetic/conversion, so free-text searches are never hijacked.

#[derive(Debug, Clone, PartialEq)]
pub struct CalcResult {
    pub value: f64,
    pub formatted: String,
    pub label: String,
}

const MAX_QUERY_CHARS: usize = 200;
const MAX_DEPTH: usize = 32;

/// Evaluate an arithmetic expression like "2+2", "15% of 340", "sqrt(9)*4".
/// Returns None when the query is not a complete arithmetic expression.
pub fn evaluate(query: &str) -> Option<CalcResult> {
    let query = clean_query(query)?;
    if let Some(result) = percent_of(query) {
        return Some(result);
    }
    // A trailing "%" means "as a fraction": "15%" -> 0.15.
    if let Some(stripped) = query.strip_suffix('%') {
        if !stripped.trim().is_empty() {
            let value = parse_expression(stripped)?;
            if !value.is_finite() {
                return None;
            }
            return Some(CalcResult {
                value: value / 100.0,
                formatted: format_number(value / 100.0),
                label: format!("{} = {}", query.trim(), format_number(value / 100.0)),
            });
        }
    }
    if !looks_like_math(query) {
        return None;
    }
    let value = parse_expression(query)?;
    if !value.is_finite() {
        return None;
    }
    Some(CalcResult {
        value,
        formatted: format_number(value),
        label: format!("{} = {}", query.trim(), format_number(value)),
    })
}

/// Convert units like "5 km to miles", "100 f to c", "1 gb to mb".
pub fn convert(query: &str) -> Option<CalcResult> {
    let query = clean_query(query)?;
    let (value, from_unit, to_unit) = parse_conversion(query)?;
    let (converted, canonical_from, canonical_to) = convert_units(value, &from_unit, &to_unit)?;
    if !converted.is_finite() {
        return None;
    }
    let formatted = format_number(converted);
    Some(CalcResult {
        value: converted,
        formatted: formatted.clone(),
        label: format!(
            "{} {} = {} {}",
            format_number(value),
            canonical_from,
            formatted,
            canonical_to
        ),
    })
}

/// Try conversion first, then arithmetic (engine convenience entry point).
pub fn evaluate_anything(query: &str) -> Option<CalcResult> {
    convert(query).or_else(|| evaluate(query))
}

fn clean_query(query: &str) -> Option<&str> {
    let trimmed = query.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_QUERY_CHARS {
        return None;
    }
    // Reject control characters outright.
    if trimmed.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(trimmed)
}

/// "15% of 340" / "12.5 percent of (100+50)".
fn percent_of(query: &str) -> Option<CalcResult> {
    let lower = query.to_ascii_lowercase();
    let (index, marker_length) = if let Some(index) = lower.find("% of") {
        (index, 1)
    } else if let Some(index) = lower.find(" percent of") {
        (index, " percent".len())
    } else if let Some(index) = lower.find(" pct of") {
        (index, " pct".len())
    } else {
        return None;
    };
    let percent_part = query[..index].trim();
    let percent: f64 = percent_part.parse().ok()?;
    if !percent.is_finite() {
        return None;
    }
    let after = &query[index + marker_length..];
    let amount_part = after.trim_start_matches(|c: char| c == ' ' || c.is_whitespace());
    let amount_part = amount_part.strip_prefix("of").unwrap_or(amount_part);
    let amount = parse_expression(amount_part.trim())?;
    if !amount.is_finite() {
        return None;
    }
    let value = percent / 100.0 * amount;
    Some(CalcResult {
        value,
        formatted: format_number(value),
        label: format!(
            "{}% of {} = {}",
            format_number(percent),
            format_number(amount),
            format_number(value)
        ),
    })
}

/// Cheap intent check so free text ("10 apples") is not treated as math.
fn looks_like_math(query: &str) -> bool {
    let mut has_digit = false;
    let mut has_operator = false;
    let mut alphabetic = 0usize;
    for ch in query.chars() {
        if ch.is_ascii_digit() {
            has_digit = true;
        } else if matches!(ch, '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')') {
            has_operator = true;
        } else if ch.is_alphabetic() {
            alphabetic += 1;
        }
    }
    if !has_digit {
        return false;
    }
    if has_operator {
        return true;
    }
    // Function/constant-only usage such as "sqrt(9)" still contains digits and
    // parentheses. Allow a small alphabetic budget for names.
    alphabetic <= 16 && query.contains('(')
}

// ---------------------------------------------------------------------------
// Recursive descent parser
// ---------------------------------------------------------------------------

struct Parser<'a> {
    chars: Vec<char>,
    position: usize,
    depth: usize,
    _source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            position: 0,
            depth: 0,
            _source: source,
        }
    }

    fn parse_complete(mut self) -> Option<f64> {
        let value = self.parse_expression()?;
        self.skip_whitespace();
        if self.position == self.chars.len() {
            Some(value)
        } else {
            None
        }
    }

    fn parse_expression(&mut self) -> Option<f64> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('+') => {
                    self.position += 1;
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.position += 1;
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        self.depth -= 1;
        Some(value)
    }

    fn parse_term(&mut self) -> Option<f64> {
        let mut value = self.parse_unary()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('*') => {
                    self.position += 1;
                    value *= self.parse_unary()?;
                }
                Some('/') => {
                    self.position += 1;
                    let divisor = self.parse_unary()?;
                    if divisor == 0.0 {
                        return None;
                    }
                    value /= divisor;
                }
                Some('%') => {
                    self.position += 1;
                    let divisor = self.parse_unary()?;
                    if divisor == 0.0 {
                        return None;
                    }
                    value %= divisor;
                }
                _ => break,
            }
        }
        Some(value)
    }

    fn parse_unary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        match self.peek() {
            Some('-') => {
                self.position += 1;
                Some(-self.parse_unary()?)
            }
            Some('+') => {
                self.position += 1;
                self.parse_unary()
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Option<f64> {
        let base = self.parse_primary()?;
        self.skip_whitespace();
        if self.peek() == Some('^') {
            self.position += 1;
            // Right associative: 2^3^2 == 2^(3^2).
            let exponent = self.parse_unary()?;
            let result = base.powf(exponent);
            if !result.is_finite() {
                return None;
            }
            return Some(result);
        }
        Some(base)
    }

    fn parse_primary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        match self.peek()? {
            '(' => {
                self.position += 1;
                let value = self.parse_expression()?;
                self.skip_whitespace();
                if self.peek() != Some(')') {
                    return None;
                }
                self.position += 1;
                Some(value)
            }
            ch if ch.is_ascii_digit() || ch == '.' => self.parse_number(),
            ch if ch.is_alphabetic() => self.parse_identifier(),
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<f64> {
        let start = self.position;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit() || ch == '.') {
            self.position += 1;
        }
        // Scientific notation: 1e3, 2.5e-2.
        if matches!(self.peek(), Some('e') | Some('E')) {
            let save = self.position;
            self.position += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.position += 1;
            }
            let mut digits = 0;
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.position += 1;
                digits += 1;
            }
            if digits == 0 {
                self.position = save;
            }
        }
        let text: String = self.chars[start..self.position].iter().collect();
        text.parse::<f64>().ok()
    }

    fn parse_identifier(&mut self) -> Option<f64> {
        let start = self.position;
        while matches!(self.peek(), Some(ch) if ch.is_alphabetic() || ch == '_') {
            self.position += 1;
        }
        let name: String = self.chars[start..self.position]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        self.skip_whitespace();
        if self.peek() == Some('(') {
            self.position += 1;
            let mut arguments = Vec::new();
            self.skip_whitespace();
            if self.peek() == Some(')') {
                self.position += 1;
            } else {
                loop {
                    arguments.push(self.parse_expression()?);
                    self.skip_whitespace();
                    match self.peek() {
                        Some(',') => {
                            self.position += 1;
                        }
                        Some(')') => {
                            self.position += 1;
                            break;
                        }
                        _ => return None,
                    }
                }
            }
            return apply_function(&name, &arguments);
        }
        match name.as_str() {
            "pi" => Some(std::f64::consts::PI),
            "e" => Some(std::f64::consts::E),
            "tau" => Some(std::f64::consts::TAU),
            _ => None,
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }
}

fn parse_expression(source: &str) -> Option<f64> {
    Parser::new(source).parse_complete()
}

fn apply_function(name: &str, arguments: &[f64]) -> Option<f64> {
    let unary = |function: fn(f64) -> f64| -> Option<f64> {
        if arguments.len() != 1 {
            return None;
        }
        let value = function(arguments[0]);
        if value.is_finite() {
            Some(value)
        } else {
            None
        }
    };
    match name {
        "sqrt" => unary(f64::sqrt),
        "cbrt" => unary(f64::cbrt),
        "abs" => unary(f64::abs),
        "round" => unary(f64::round),
        "floor" => unary(f64::floor),
        "ceil" => unary(f64::ceil),
        "ln" => unary(f64::ln),
        "log" => unary(f64::log10),
        "log2" => unary(f64::log2),
        "log10" => unary(f64::log10),
        "exp" => unary(f64::exp),
        "sin" => unary(f64::sin),
        "cos" => unary(f64::cos),
        "tan" => unary(f64::tan),
        "asin" => unary(f64::asin),
        "acos" => unary(f64::acos),
        "atan" => unary(f64::atan),
        "sign" => unary(|value| {
            if value > 0.0 {
                1.0
            } else if value < 0.0 {
                -1.0
            } else {
                0.0
            }
        }),
        "pow" => {
            if arguments.len() != 2 {
                return None;
            }
            let value = arguments[0].powf(arguments[1]);
            if value.is_finite() {
                Some(value)
            } else {
                None
            }
        }
        "min" => arguments.iter().copied().reduce(f64::min).filter(|v| v.is_finite()),
        "max" => arguments.iter().copied().reduce(f64::max).filter(|v| v.is_finite()),
        "avg" | "mean" => {
            if arguments.is_empty() {
                return None;
            }
            let sum: f64 = arguments.iter().sum();
            let value = sum / arguments.len() as f64;
            if value.is_finite() {
                Some(value)
            } else {
                None
            }
        }
        "fact" | "factorial" => {
            if arguments.len() != 1 {
                return None;
            }
            let n = arguments[0];
            if n < 0.0 || n > 170.0 || n.fract() != 0.0 {
                return None;
            }
            let mut result = 1.0f64;
            let mut counter = 2.0f64;
            while counter <= n {
                result *= counter;
                counter += 1.0;
            }
            Some(result)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Number formatting
// ---------------------------------------------------------------------------

pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "∞".to_string() } else { "-∞".to_string() };
    }
    let absolute = value.abs();
    if absolute != 0.0 && (absolute >= 1e15 || absolute < 1e-9) {
        let formatted = format!("{value:e}");
        return formatted;
    }
    let rounded = (value * 1e10).round() / 1e10;
    let mut text = format!("{rounded}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

// ---------------------------------------------------------------------------
// Unit conversion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Length,
    Mass,
    Data,
    Time,
    Volume,
    Area,
    Speed,
    Temperature,
}

struct Unit {
    symbol: &'static str,
    aliases: &'static [&'static str],
    category: Category,
    /// Factor to the category base unit (temperature uses a custom path).
    factor: f64,
}

const UNITS: &[Unit] = &[
    // Length (base: metre)
    Unit { symbol: "mm", aliases: &["mm", "millimeter", "millimeters", "millimetre", "millimetres"], category: Category::Length, factor: 0.001 },
    Unit { symbol: "cm", aliases: &["cm", "centimeter", "centimeters", "centimetre", "centimetres"], category: Category::Length, factor: 0.01 },
    Unit { symbol: "m", aliases: &["m", "meter", "meters", "metre", "metres"], category: Category::Length, factor: 1.0 },
    Unit { symbol: "km", aliases: &["km", "kilometer", "kilometers", "kilometre", "kilometres"], category: Category::Length, factor: 1000.0 },
    Unit { symbol: "in", aliases: &["in", "inch", "inches"], category: Category::Length, factor: 0.0254 },
    Unit { symbol: "ft", aliases: &["ft", "foot", "feet"], category: Category::Length, factor: 0.3048 },
    Unit { symbol: "yd", aliases: &["yd", "yard", "yards"], category: Category::Length, factor: 0.9144 },
    Unit { symbol: "mi", aliases: &["mi", "mile", "miles"], category: Category::Length, factor: 1609.344 },
    Unit { symbol: "nmi", aliases: &["nmi", "nauticalmile", "nauticalmiles"], category: Category::Length, factor: 1852.0 },
    // Mass (base: gram)
    Unit { symbol: "mg", aliases: &["mg", "milligram", "milligrams"], category: Category::Mass, factor: 0.001 },
    Unit { symbol: "g", aliases: &["g", "gram", "grams"], category: Category::Mass, factor: 1.0 },
    Unit { symbol: "kg", aliases: &["kg", "kilogram", "kilograms"], category: Category::Mass, factor: 1000.0 },
    Unit { symbol: "t", aliases: &["t", "tonne", "tonnes", "metrict", "metrictons"], category: Category::Mass, factor: 1_000_000.0 },
    Unit { symbol: "oz", aliases: &["oz", "ounce", "ounces"], category: Category::Mass, factor: 28.349523125 },
    Unit { symbol: "lb", aliases: &["lb", "lbs", "pound", "pounds"], category: Category::Mass, factor: 453.59237 },
    Unit { symbol: "st", aliases: &["st", "stone", "stones"], category: Category::Mass, factor: 6350.29318 },
    // Data (base: byte)
    Unit { symbol: "B", aliases: &["b", "byte", "bytes"], category: Category::Data, factor: 1.0 },
    Unit { symbol: "KB", aliases: &["kb", "kilobyte", "kilobytes"], category: Category::Data, factor: 1024.0 },
    Unit { symbol: "MB", aliases: &["mb", "megabyte", "megabytes"], category: Category::Data, factor: 1_048_576.0 },
    Unit { symbol: "GB", aliases: &["gb", "gigabyte", "gigabytes"], category: Category::Data, factor: 1_073_741_824.0 },
    Unit { symbol: "TB", aliases: &["tb", "terabyte", "terabytes"], category: Category::Data, factor: 1_099_511_627_776.0 },
    Unit { symbol: "KiB", aliases: &["kib", "kibibyte", "kibibytes"], category: Category::Data, factor: 1024.0 },
    Unit { symbol: "MiB", aliases: &["mib", "mebibyte", "mebibytes"], category: Category::Data, factor: 1_048_576.0 },
    Unit { symbol: "GiB", aliases: &["gib", "gibibyte", "gibibytes"], category: Category::Data, factor: 1_073_741_824.0 },
    // Time (base: second)
    Unit { symbol: "ms", aliases: &["ms", "millisecond", "milliseconds"], category: Category::Time, factor: 0.001 },
    Unit { symbol: "s", aliases: &["s", "sec", "secs", "second", "seconds"], category: Category::Time, factor: 1.0 },
    Unit { symbol: "min", aliases: &["min", "mins", "minute", "minutes"], category: Category::Time, factor: 60.0 },
    Unit { symbol: "h", aliases: &["h", "hr", "hrs", "hour", "hours"], category: Category::Time, factor: 3600.0 },
    Unit { symbol: "day", aliases: &["d", "day", "days"], category: Category::Time, factor: 86400.0 },
    Unit { symbol: "week", aliases: &["wk", "week", "weeks"], category: Category::Time, factor: 604_800.0 },
    Unit { symbol: "year", aliases: &["yr", "year", "years"], category: Category::Time, factor: 31_557_600.0 },
    // Volume (base: litre)
    Unit { symbol: "ml", aliases: &["ml", "milliliter", "milliliters", "millilitre", "millilitres"], category: Category::Volume, factor: 0.001 },
    Unit { symbol: "l", aliases: &["l", "liter", "liters", "litre", "litres"], category: Category::Volume, factor: 1.0 },
    Unit { symbol: "gal", aliases: &["gal", "gallon", "gallons"], category: Category::Volume, factor: 3.785411784 },
    Unit { symbol: "qt", aliases: &["qt", "quart", "quarts"], category: Category::Volume, factor: 0.946352946 },
    Unit { symbol: "pt", aliases: &["pt", "pint", "pints"], category: Category::Volume, factor: 0.473176473 },
    Unit { symbol: "cup", aliases: &["cup", "cups"], category: Category::Volume, factor: 0.2365882365 },
    Unit { symbol: "floz", aliases: &["floz", "fluidounce", "fluidounces"], category: Category::Volume, factor: 0.0295735295625 },
    // Area (base: square metre)
    Unit { symbol: "m2", aliases: &["m2", "sqm", "squaremeter", "squaremeters"], category: Category::Area, factor: 1.0 },
    Unit { symbol: "km2", aliases: &["km2", "sqkm"], category: Category::Area, factor: 1_000_000.0 },
    Unit { symbol: "cm2", aliases: &["cm2", "sqcm"], category: Category::Area, factor: 0.0001 },
    Unit { symbol: "ft2", aliases: &["ft2", "sqft", "squarefoot", "squarefeet"], category: Category::Area, factor: 0.09290304 },
    Unit { symbol: "in2", aliases: &["in2", "sqin"], category: Category::Area, factor: 0.00064516 },
    Unit { symbol: "acre", aliases: &["acre", "acres"], category: Category::Area, factor: 4046.8564224 },
    Unit { symbol: "ha", aliases: &["ha", "hectare", "hectares"], category: Category::Area, factor: 10_000.0 },
    // Speed (base: metre/second)
    Unit { symbol: "mps", aliases: &["mps", "m/s", "meterspersecond"], category: Category::Speed, factor: 1.0 },
    Unit { symbol: "kmh", aliases: &["kmh", "km/h", "kph", "kilometersperhour"], category: Category::Speed, factor: 1.0 / 3.6 },
    Unit { symbol: "mph", aliases: &["mph", "milesperhour"], category: Category::Speed, factor: 0.44704 },
    Unit { symbol: "kn", aliases: &["kn", "knot", "knots"], category: Category::Speed, factor: 0.514444444 },
    // Temperature (custom formulas; factor unused)
    Unit { symbol: "°C", aliases: &["c", "°c", "celsius", "centigrade"], category: Category::Temperature, factor: 1.0 },
    Unit { symbol: "°F", aliases: &["f", "°f", "fahrenheit"], category: Category::Temperature, factor: 1.0 },
    Unit { symbol: "K", aliases: &["k", "kelvin"], category: Category::Temperature, factor: 1.0 },
];

fn find_unit(name: &str) -> Option<&'static Unit> {
    let normalized: String = name
        .to_lowercase()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_')
        .collect();
    UNITS.iter().find(|unit| {
        unit.aliases
            .iter()
            .any(|alias| alias.replace(' ', "") == normalized)
    })
}

/// Parse "5 km to mi" / "100f to c" / "convert 1 gb to mb".
fn parse_conversion(query: &str) -> Option<(f64, String, String)> {
    let mut text = query.trim();
    let lower = text.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix("convert ") {
        text = &text[text.len() - stripped.len()..];
    }
    // Normalise "->" separators to " to ".
    let text = text.replace("->", " to ");
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < 4 {
        // Support "5km to mi" (no space between value and unit).
        if tokens.len() == 3 {
            let (value, from) = split_value_unit(tokens[0])?;
            let to = tokens[2];
            if !matches!(tokens[1].to_ascii_lowercase().as_str(), "to" | "in" | "into" | "as") {
                return None;
            }
            return Some((value, from, to.to_string()));
        }
        return None;
    }
    let (value, from) = split_value_unit(tokens[0])?;
    let separator = tokens[1].to_ascii_lowercase();
    // tokens[1] may be the unit when value and unit are glued, e.g. "5km to mi".
    let (from, to) = if matches!(separator.as_str(), "to" | "in" | "into" | "as" | "=") {
        (from, tokens[2..].join(" "))
    } else {
        (tokens[1].to_string(), tokens[3..].join(" "))
    };
    Some((value, from, to))
}

fn split_value_unit(token: &str) -> Option<(f64, String)> {
    let mut number_end = 0usize;
    let bytes = token.as_bytes();
    let mut has_digit = false;
    let mut index = 0usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch.is_ascii_digit() {
            has_digit = true;
            index += 1;
        } else if ch == '.' {
            index += 1;
        } else if (ch == 'e' || ch == 'E') && has_digit && index + 1 < bytes.len() {
            // exponent
            index += 1;
            if index < bytes.len() && (bytes[index] as char == '+' || bytes[index] as char == '-') {
                index += 1;
            }
        } else if ch == '-' && !has_digit && index == 0 {
            index += 1;
        } else {
            break;
        }
    }
    number_end = index;
    if !has_digit || number_end == token.len() {
        return None;
    }
    let value: f64 = token[..number_end].parse().ok()?;
    let unit = token[number_end..].trim();
    if unit.is_empty() {
        return None;
    }
    Some((value, unit.to_string()))
}

fn convert_units(value: f64, from: &str, to: &str) -> Option<(f64, &'static str, &'static str)> {
    let from_unit = find_unit(from)?;
    let to_unit = find_unit(to)?;
    if from_unit.category != to_unit.category {
        return None;
    }
    let converted = match from_unit.category {
        Category::Temperature => convert_temperature(value, from_unit.symbol, to_unit.symbol)?,
        _ => value * from_unit.factor / to_unit.factor,
    };
    Some((converted, from_unit.symbol, to_unit.symbol))
}

fn convert_temperature(value: f64, from: &str, to: &str) -> Option<f64> {
    // Normalise to Celsius first.
    let celsius = match from {
        "°C" => value,
        "°F" => (value - 32.0) * 5.0 / 9.0,
        "K" => value - 273.15,
        _ => return None,
    };
    let result = match to {
        "°C" => celsius,
        "°F" => celsius * 9.0 / 5.0 + 32.0,
        "K" => celsius + 273.15,
        _ => return None,
    };
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn evaluates_basic_arithmetic() {
        assert_eq!(evaluate("2+2").unwrap().value, 4.0);
        assert_eq!(evaluate("2 + 2").unwrap().value, 4.0);
        assert_eq!(evaluate("2 * (3 + 4)").unwrap().value, 14.0);
        assert_eq!(evaluate("10 / 4").unwrap().value, 2.5);
        assert_eq!(evaluate("-5 + 3").unwrap().value, -2.0);
        assert_eq!(evaluate("2^10").unwrap().value, 1024.0);
        assert_eq!(evaluate("2^3^2").unwrap().value, 512.0);
        assert_eq!(evaluate("7 % 4").unwrap().value, 3.0);
        assert_eq!(evaluate("sqrt(9)*4").unwrap().value, 12.0);
        assert_eq!(evaluate("abs(-7)").unwrap().value, 7.0);
        assert_eq!(evaluate("pow(2, 8)").unwrap().value, 256.0);
        assert_eq!(evaluate("max(3, 9, 4)").unwrap().value, 9.0);
        assert_eq!(evaluate("fact(5)").unwrap().value, 120.0);
        assert_close(evaluate("sin(0)").unwrap().value, 0.0);
        assert_close(evaluate("cos(0)").unwrap().value, 1.0);
        assert_close(evaluate("2 * pi").unwrap().value, std::f64::consts::TAU);
        assert_eq!(evaluate("1e3 + 1").unwrap().value, 1001.0);
        assert_eq!(evaluate("10 / 0"), None);
        assert_eq!(evaluate("1/0"), None);
    }

    #[test]
    fn evaluates_percentages() {
        assert_eq!(evaluate("15% of 340").unwrap().value, 51.0);
        assert_eq!(evaluate("20% of (100 + 50)").unwrap().value, 30.0);
        assert_eq!(evaluate("12.5 percent of 200").unwrap().value, 25.0);
        assert_eq!(evaluate("50%").unwrap().value, 0.5);
        assert_eq!(evaluate("15%of 340"), None); // malformed spacing is rejected
    }

    #[test]
    fn rejects_free_text() {
        for query in [
            "hello",
            "2 apples",
            "protonsearch",
            "quarterly report 2026",
            "foo(2)",
            "",
            "   ",
            "(2+2",
            "2 2",
            "1/0",
        ] {
            assert!(evaluate(query).is_none(), "must not evaluate {query:?}");
        }
    }

    #[test]
    fn converts_length_and_mass() {
        assert_close(convert("5 km to miles").unwrap().value, 3.106855961);
        assert_close(convert("1 mi to km").unwrap().value, 1.609344);
        assert_close(convert("30 cm to inches").unwrap().value, 11.811023622);
        assert_close(convert("2.5 kg to lb").unwrap().value, 5.511556555);
        assert_close(convert("1 lb to oz").unwrap().value, 16.0);
        assert_close(convert("5km to mi").unwrap().value, 3.106855961);
        assert_close(convert("convert 10 m to ft").unwrap().value, 32.808398950);
    }

    #[test]
    fn converts_temperature() {
        assert_close(convert("100 f to c").unwrap().value, 37.777777777);
        assert_close(convert("10 c to f").unwrap().value, 50.0);
        assert_close(convert("0 c to f").unwrap().value, 32.0);
        assert_close(convert("300 k to c").unwrap().value, 26.85);
        assert_close(convert("0 c to k").unwrap().value, 273.15);
    }

    #[test]
    fn converts_data_time_volume_area_speed() {
        assert_eq!(convert("1 gb to mb").unwrap().value, 1024.0);
        assert_eq!(convert("1024 mb to gb").unwrap().value, 1.0);
        assert_eq!(convert("1 kb to bytes").unwrap().value, 1024.0);
        assert_close(convert("90 min to hours").unwrap().value, 1.5);
        assert_close(convert("1 week to days").unwrap().value, 7.0);
        assert_close(convert("2 l to ml").unwrap().value, 2000.0);
        assert_close(convert("1 acre to m2").unwrap().value, 4046.8564224);
        assert_close(convert("36 kmh to mph").unwrap().value, 22.369362920);
    }

    #[test]
    fn rejects_invalid_conversions() {
        for query in [
            "5 km to kg",
            "hello to world",
            "5 to 10",
            "5 km",
            "km to mi",
            "5 apples to oranges",
        ] {
            assert!(convert(query).is_none(), "must not convert {query:?}");
        }
    }

    #[test]
    fn evaluate_anything_tries_conversion_then_math() {
        assert!(evaluate_anything("5 km to mi").is_some());
        assert!(evaluate_anything("2+2").is_some());
        assert!(evaluate_anything("just words").is_none());
    }

    #[test]
    fn formats_numbers_without_float_artifacts() {
        assert_eq!(format_number(4.0), "4");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
    }
}
