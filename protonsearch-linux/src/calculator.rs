//! Small, dependency-free calculator provider for launcher queries.

pub fn evaluate(input: &str) -> Option<f64> {
    let input = input.trim();
    if !input.chars().any(|character| character.is_ascii_digit()) {
        return None;
    }
    if let Some((left, right)) = input.split_once("% of") {
        let percentage = left.trim().parse::<f64>().ok()?;
        let base = right.trim().parse::<f64>().ok()?;
        return finite(percentage / 100.0 * base);
    }
    let mut parser = Parser::new(input);
    let value = parser.expression()?;
    parser.skip_space();
    if parser.position == parser.input.len() {
        finite(value)
    } else {
        None
    }
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn expression(&mut self) -> Option<f64> {
        let mut value = self.term()?;
        loop {
            self.skip_space();
            let Some(operator) = self.peek() else {
                return Some(value);
            };
            if operator != '+' && operator != '-' {
                return Some(value);
            }
            self.position += operator.len_utf8();
            let right = self.term()?;
            value = if operator == '+' {
                value + right
            } else {
                value - right
            };
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut value = self.power()?;
        loop {
            self.skip_space();
            let Some(operator) = self.peek() else {
                return Some(value);
            };
            if !matches!(operator, '*' | '/' | '%') {
                return Some(value);
            }
            self.position += operator.len_utf8();
            let right = self.power()?;
            value = match operator {
                '*' => value * right,
                '/' if right != 0.0 => value / right,
                '%' => value % right,
                _ => return None,
            };
        }
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        self.skip_space();
        if self.peek() == Some('^') {
            self.position += 1;
            return Some(base.powf(self.power()?));
        }
        Some(base)
    }

    fn unary(&mut self) -> Option<f64> {
        self.skip_space();
        match self.peek() {
            Some('+') => {
                self.position += 1;
                self.unary()
            }
            Some('-') => {
                self.position += 1;
                Some(-self.unary()?)
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<f64> {
        self.skip_space();
        if self.peek() == Some('(') {
            self.position += 1;
            let value = self.expression()?;
            self.skip_space();
            if self.peek() != Some(')') {
                return None;
            }
            self.position += 1;
            return Some(value);
        }

        if self
            .peek()
            .is_some_and(|character| character.is_ascii_digit() || character == '.')
        {
            return self.number();
        }

        let start = self.position;
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_alphabetic())
        {
            self.position += self.peek()?.len_utf8();
        }
        if start == self.position {
            return None;
        }
        let name = self.input[start..self.position].to_ascii_lowercase();
        self.skip_space();
        if self.peek() != Some('(') {
            return None;
        }
        self.position += 1;
        let value = self.expression()?;
        self.skip_space();
        if self.peek() != Some(')') {
            return None;
        }
        self.position += 1;
        Some(match name.as_str() {
            "sqrt" => value.sqrt(),
            "abs" => value.abs(),
            "round" => value.round(),
            "floor" => value.floor(),
            "ceil" => value.ceil(),
            "sin" => value.sin(),
            "cos" => value.cos(),
            "tan" => value.tan(),
            "log" => value.log10(),
            "ln" => value.ln(),
            _ => return None,
        })
    }

    fn number(&mut self) -> Option<f64> {
        let start = self.position;
        let mut dots = 0;
        while let Some(character) = self.peek() {
            if character == '.' {
                dots += 1;
                if dots > 1 {
                    break;
                }
            } else if !character.is_ascii_digit() {
                break;
            }
            self.position += character.len_utf8();
        }
        self.input[start..self.position].parse().ok()
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += self.peek().map_or(0, char::len_utf8);
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;

    #[test]
    fn evaluates_basic_expression() {
        assert_eq!(evaluate("2 + 3 * 4"), Some(14.0));
    }

    #[test]
    fn evaluates_functions_and_percentage() {
        assert_eq!(evaluate("sqrt(81)"), Some(9.0));
        assert_eq!(evaluate("25% of 200"), Some(50.0));
    }
}
