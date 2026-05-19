use crate::ast::{Expression, Module, Position, Statement};

const RESERVED_KEYWORDS: &[&str] = &[
    "module", "import", "new", "native", "this", "return", "break", "continue", "if", "else",
    "while", "for", "true", "false", "try", "catch", "undef", "throw", "fork", "global",
];

pub struct Parser {
    input: Vec<char>,
    pos: usize,
    line: i32,
    column: i32,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn parse_program(&mut self) -> Result<Module, String> {
        self.skip_whitespace();
        let pos = self.position();
        let name = self.parse_module_declaration()?;
        let imports = self.parse_imports()?;
        let statements = self.parse_statements_until_eof()?;
        Ok(Module {
            name,
            imports,
            statements,
            pos,
        })
    }

    pub fn parse_statements_only(&mut self) -> Result<Vec<Statement>, String> {
        self.skip_whitespace();
        self.parse_statements_until_eof()
    }

    // ---- position helpers ----

    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }

    fn error<T>(&self, msg: &str) -> Result<T, String> {
        Err(format!("{}:{}: {}", self.line, self.column, msg))
    }

    // ---- character-level helpers ----

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn skip_whitespace(&mut self) {
        loop {
            // whitespace
            while let Some(ch) = self.peek() {
                if ch.is_ascii_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            // line comment
            if self.starts_with("//") {
                self.advance();
                self.advance();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }
            // block comment
            if self.starts_with("/*") {
                self.advance();
                self.advance();
                loop {
                    if self.at_end() {
                        break;
                    }
                    if self.starts_with("*/") {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                continue;
            }
            break;
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            match self.input.get(self.pos + i) {
                Some(&c) if c == ch => {}
                _ => return false,
            }
        }
        true
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        self.skip_whitespace();
        match self.peek() {
            Some(ch) if ch == expected => {
                self.advance();
                Ok(())
            }
            Some(ch) => self.error(&format!("expected '{}', found '{}'", expected, ch)),
            None => self.error(&format!("expected '{}', found end of input", expected)),
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), String> {
        self.skip_whitespace();
        let saved_pos = self.pos;
        let saved_line = self.line;
        let saved_col = self.column;
        for ch in kw.chars() {
            match self.advance() {
                Some(c) if c == ch => {}
                _ => {
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.column = saved_col;
                    return self.error(&format!("expected keyword '{}'", kw));
                }
            }
        }
        // must not be followed by letter/digit
        if let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos = saved_pos;
                self.line = saved_line;
                self.column = saved_col;
                return self.error(&format!("expected keyword '{}'", kw));
            }
        }
        Ok(())
    }

    fn try_keyword(&mut self, kw: &str) -> bool {
        self.skip_whitespace();
        let saved_pos = self.pos;
        let saved_line = self.line;
        let saved_col = self.column;
        for ch in kw.chars() {
            match self.advance() {
                Some(c) if c == ch => {}
                _ => {
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.column = saved_col;
                    return false;
                }
            }
        }
        // must not be followed by letter/digit
        if let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos = saved_pos;
                self.line = saved_line;
                self.column = saved_col;
                return false;
            }
        }
        true
    }

    fn try_char(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn try_str(&mut self, s: &str) -> bool {
        self.skip_whitespace();
        let saved_pos = self.pos;
        let saved_line = self.line;
        let saved_col = self.column;
        for ch in s.chars() {
            match self.advance() {
                Some(c) if c == ch => {}
                _ => {
                    self.pos = saved_pos;
                    self.line = saved_line;
                    self.column = saved_col;
                    return false;
                }
            }
        }
        true
    }

    fn save(&self) -> (usize, i32, i32) {
        (self.pos, self.line, self.column)
    }

    fn restore(&mut self, state: (usize, i32, i32)) {
        self.pos = state.0;
        self.line = state.1;
        self.column = state.2;
    }

    // ---- identifier / number / string ----

    fn parse_identifier(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        let mut name = String::new();
        match self.peek() {
            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                name.push(ch);
                self.advance();
            }
            _ => return self.error("expected identifier"),
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if RESERVED_KEYWORDS.contains(&name.as_str()) {
            return self.error(&format!("'{}' is a reserved keyword", name));
        }
        Ok(name)
    }

    fn _try_identifier(&mut self) -> Option<String> {
        let state = self.save();
        match self.parse_identifier() {
            Ok(name) => Some(name),
            Err(_) => {
                self.restore(state);
                None
            }
        }
    }

    fn parse_number(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        let mut s = String::new();
        let negative = self.peek() == Some('-');
        if negative {
            s.push('-');
            self.advance();
        }
        match self.peek() {
            Some(ch) if ch.is_ascii_digit() => {
                s.push(ch);
                self.advance();
            }
            _ => return self.error("expected digit"),
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        let val: i32 = s.parse().map_err(|e| format!("{}:{}: {}", self.line, self.column, e))?;
        Ok(Expression::Constant {
            value: val,
            type_name: "Number".to_string(),
            string: None,
        })
    }

    fn parse_string(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        self.expect_char('"')?;
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return self.error("unterminated string literal"),
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some(ch) => {
                        s.push('\\');
                        s.push(ch);
                    }
                    None => return self.error("unterminated escape in string"),
                },
                Some(ch) => s.push(ch),
            }
        }
        Ok(Expression::Constant {
            value: 0,
            type_name: "String".to_string(),
            string: Some(s),
        })
    }

    // ---- module / imports ----

    fn parse_module_declaration(&mut self) -> Result<String, String> {
        self.expect_keyword("module")?;
        let name = self.parse_identifier()?;
        self.try_char(';');
        Ok(name)
    }

    fn parse_imports(&mut self) -> Result<Vec<String>, String> {
        let mut imports = Vec::new();
        while self.try_keyword("import") {
            let name = self.parse_identifier()?;
            self.try_char(';');
            imports.push(name);
        }
        Ok(imports)
    }

    // ---- statements ----

    fn parse_statements_until_eof(&mut self) -> Result<Vec<Statement>, String> {
        let mut stmts = Vec::new();
        self.skip_whitespace();
        while !self.at_end() {
            stmts.push(self.parse_statement()?);
            self.try_char(';');
            self.skip_whitespace();
        }
        Ok(stmts)
    }

    fn parse_statements_until(&mut self, terminator: char) -> Result<Vec<Statement>, String> {
        let mut stmts = Vec::new();
        self.skip_whitespace();
        while self.peek() != Some(terminator) && !self.at_end() {
            stmts.push(self.parse_statement()?);
            self.try_char(';');
            self.skip_whitespace();
        }
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.skip_whitespace();
        let pos = self.position();

        // scope statement
        if self.peek() == Some('{') {
            return self.parse_scope_statement();
        }

        // return
        if self.try_keyword("return") {
            return self.parse_return_statement(pos);
        }

        // for
        if self.try_keyword("for") {
            return self.parse_for_statement(pos);
        }

        // while
        if self.try_keyword("while") {
            return self.parse_while_statement(pos);
        }

        // if
        if self.try_keyword("if") {
            return self.parse_if_statement(pos);
        }

        // try/catch
        if self.try_keyword("try") {
            return self.parse_try_catch_statement(pos);
        }

        // break
        if self.try_keyword("break") {
            return Ok(Statement::Break(pos));
        }

        // continue
        if self.try_keyword("continue") {
            return Ok(Statement::Continue(pos));
        }

        // throw
        if self.try_keyword("throw") {
            let expr = self.parse_expression()?;
            return Ok(Statement::Throw {
                exception: expr,
                pos,
            });
        }

        // expression statement
        let expr = self.parse_expression()?;
        Ok(Statement::Expression(expr, pos))
    }

    fn parse_scope_statement(&mut self) -> Result<Statement, String> {
        let pos = self.position();
        self.expect_char('{')?;
        let stmts = self.parse_statements_until('}')?;
        self.expect_char('}')?;
        Ok(Statement::Scope {
            statements: stmts,
            pos,
        })
    }

    fn parse_return_statement(&mut self, pos: Position) -> Result<Statement, String> {
        // Check if there's an expression following return.
        // If the next thing is ';', '}', EOF, or a statement keyword at line start, it's bare return.
        self.skip_whitespace();
        let is_bare = self.at_end()
            || self.peek() == Some('}')
            || self.peek() == Some(';');
        if is_bare {
            return Ok(Statement::Return { value: None, pos });
        }
        let expr = self.parse_expression()?;
        Ok(Statement::Return {
            value: Some(expr),
            pos,
        })
    }

    fn parse_for_statement(&mut self, pos: Position) -> Result<Statement, String> {
        self.expect_char('(')?;
        let init = self.parse_expression()?;
        self.expect_char(';')?;
        let cond = self.parse_expression()?;
        self.expect_char(';')?;
        let update = self.parse_expression()?;
        self.expect_char(')')?;
        let body = self.parse_statement()?;
        Ok(Statement::For {
            init,
            condition: cond,
            update,
            body: Box::new(body),
            pos,
        })
    }

    fn parse_while_statement(&mut self, pos: Position) -> Result<Statement, String> {
        self.expect_char('(')?;
        let cond = self.parse_expression()?;
        self.expect_char(')')?;
        let body = self.parse_statement()?;
        Ok(Statement::While {
            condition: cond,
            body: Box::new(body),
            pos,
        })
    }

    fn parse_if_statement(&mut self, pos: Position) -> Result<Statement, String> {
        self.expect_char('(')?;
        let cond = self.parse_expression()?;
        self.expect_char(')')?;
        let then_clause = self.parse_statement()?;
        if self.try_keyword("else") {
            let else_clause = self.parse_statement()?;
            Ok(Statement::If {
                condition: cond,
                then_clause: Box::new(then_clause),
                else_clause: Some(Box::new(else_clause)),
                pos,
            })
        } else {
            Ok(Statement::If {
                condition: cond,
                then_clause: Box::new(then_clause),
                else_clause: None,
                pos,
            })
        }
    }

    fn parse_try_catch_statement(&mut self, pos: Position) -> Result<Statement, String> {
        let try_block = self.parse_statement()?;
        self.expect_keyword("catch")?;
        self.expect_char('(')?;
        let catch_var = self.parse_identifier()?;
        self.expect_char(')')?;
        let catch_block = self.parse_statement()?;
        Ok(Statement::TryCatch {
            try_block: Box::new(try_block),
            catch_var,
            catch_block: Box::new(catch_block),
            pos,
        })
    }

    // ---- expressions ----

    fn parse_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_conditional_expression()?;
        loop {
            self.skip_whitespace();
            if let Some(op) = self.try_assignment_operator() {
                let right = self.parse_conditional_expression()?;
                left = Expression::Assignment {
                    variable: Box::new(left),
                    operator: if op == "=" { None } else { Some(op) },
                    value: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn try_assignment_operator(&mut self) -> Option<String> {
        self.skip_whitespace();
        let state = self.save();
        // Try multi-char operators first
        for op in &["+=", "-=", "*=", "/="] {
            if self.try_str(op) {
                return Some(op.to_string());
            }
        }
        // Try plain '=' but not '==' or '=>'
        if self.peek() == Some('=') {
            if self.peek_ahead(1) != Some('=') && self.peek_ahead(1) != Some('>') {
                self.advance();
                return Some("=".to_string());
            }
        }
        self.restore(state);
        None
    }

    fn parse_conditional_expression(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_or_expression()?;
        loop {
            self.skip_whitespace();
            if self.try_char('?') {
                let positive = self.parse_expression()?;
                self.expect_char(':')?;
                let negative = self.parse_or_expression()?;
                expr = Expression::Conditional {
                    condition: Box::new(expr),
                    positive: Box::new(positive),
                    negative: Box::new(negative),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_or_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and_expression()?;
        loop {
            self.skip_whitespace();
            if self.try_str("||") {
                let right = self.parse_and_expression()?;
                left = Expression::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality_expression()?;
        loop {
            self.skip_whitespace();
            if self.try_str("&&") {
                let right = self.parse_equality_expression()?;
                left = Expression::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_equality_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_relational_expression()?;
        loop {
            self.skip_whitespace();
            let state = self.save();
            if self.try_str("==") {
                let right = self.parse_relational_expression()?;
                left = Expression::Equality(Box::new(left), "==".to_string(), Box::new(right));
            } else if self.try_str("!=") {
                let right = self.parse_relational_expression()?;
                left = Expression::Equality(Box::new(left), "!=".to_string(), Box::new(right));
            } else {
                self.restore(state);
                break;
            }
        }
        Ok(left)
    }

    fn parse_relational_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_additive_expression()?;
        loop {
            self.skip_whitespace();
            let state = self.save();
            // Try two-char operators first
            if self.try_str("<=") {
                let right = self.parse_additive_expression()?;
                left = Expression::Relational(Box::new(left), "<=".to_string(), Box::new(right));
            } else if self.try_str(">=") {
                let right = self.parse_additive_expression()?;
                left = Expression::Relational(Box::new(left), ">=".to_string(), Box::new(right));
            } else if self.peek() == Some('<') {
                self.advance();
                let right = self.parse_additive_expression()?;
                left = Expression::Relational(Box::new(left), "<".to_string(), Box::new(right));
            } else if self.peek() == Some('>') {
                self.advance();
                let right = self.parse_additive_expression()?;
                left = Expression::Relational(Box::new(left), ">".to_string(), Box::new(right));
            } else {
                self.restore(state);
                break;
            }
        }
        Ok(left)
    }

    fn parse_additive_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative_expression()?;
        loop {
            self.skip_whitespace();
            let state = self.save();
            // Make sure + is not ++ and - is not -- or ->
            if self.peek() == Some('+') && self.peek_ahead(1) != Some('+') && self.peek_ahead(1) != Some('=') {
                self.advance();
                let right = self.parse_multiplicative_expression()?;
                left = Expression::Additive(Box::new(left), "+".to_string(), Box::new(right));
            } else if self.peek() == Some('-') && self.peek_ahead(1) != Some('-') && self.peek_ahead(1) != Some('>') && self.peek_ahead(1) != Some('=') {
                self.advance();
                let right = self.parse_multiplicative_expression()?;
                left = Expression::Additive(Box::new(left), "-".to_string(), Box::new(right));
            } else {
                self.restore(state);
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary_expression()?;
        loop {
            self.skip_whitespace();
            let state = self.save();
            if self.peek() == Some('*') && self.peek_ahead(1) != Some('=') {
                self.advance();
                let right = self.parse_unary_expression()?;
                left =
                    Expression::Multiplicative(Box::new(left), "*".to_string(), Box::new(right));
            } else if self.peek() == Some('/') && self.peek_ahead(1) != Some('=') && self.peek_ahead(1) != Some('/') && self.peek_ahead(1) != Some('*') {
                self.advance();
                let right = self.parse_unary_expression()?;
                left =
                    Expression::Multiplicative(Box::new(left), "/".to_string(), Box::new(right));
            } else if self.peek() == Some('%') {
                self.advance();
                let right = self.parse_unary_expression()?;
                left =
                    Expression::Multiplicative(Box::new(left), "%".to_string(), Box::new(right));
            } else {
                self.restore(state);
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        // !expr
        if self.try_char('!') {
            let expr = self.parse_unary_expression()?;
            return Ok(Expression::Not(Box::new(expr)));
        }
        // Try reference with postfix
        let state = self.save();
        if let Ok(reference) = self.parse_reference() {
            // postfix operators ++ / --
            let pstate = self.save();
            if self.try_str("++") {
                return Ok(Expression::PostFixOperator {
                    operator: "++".to_string(),
                    argument: Box::new(reference),
                });
            }
            if self.try_str("--") {
                return Ok(Expression::PostFixOperator {
                    operator: "--".to_string(),
                    argument: Box::new(reference),
                });
            }
            self.restore(pstate);

            // postfix references: [index] or (args)
            let pstate2 = self.save();
            self.skip_whitespace();
            if self.peek() == Some('[') || self.peek() == Some('(') {
                if let Ok(expr) = self.parse_postfix_chain(reference.clone()) {
                    return Ok(expr);
                }
                self.restore(pstate2);
            }

            return Ok(reference);
        }
        self.restore(state);

        self.parse_other_expression()
    }

    fn parse_postfix_chain(&mut self, base: Expression) -> Result<Expression, String> {
        let mut expr = base;
        let mut had_postfix = false;
        loop {
            self.skip_whitespace();
            if self.peek() == Some('[') {
                self.advance();
                let index = self.parse_expression()?;
                self.expect_char(']')?;
                expr = Expression::ArrayReference {
                    reference: Box::new(expr),
                    index: Box::new(index),
                };
                had_postfix = true;
            } else if self.peek() == Some('(') {
                self.advance();
                let args = self.parse_function_arguments()?;
                self.expect_char(')')?;
                expr = Expression::FunctionCall {
                    function: Box::new(expr),
                    parameters: args,
                    field: None,
                };
                had_postfix = true;
            } else if self.peek() == Some('.') {
                let state = self.save();
                self.advance();
                match self.parse_reference_chain() {
                    Ok(mut chain) => {
                        self.set_deepest_field(&mut chain, expr);
                        expr = chain;
                        had_postfix = true;
                    }
                    Err(_) => {
                        self.restore(state);
                        break;
                    }
                }
            } else {
                break;
            }
        }
        if had_postfix {
            Ok(expr)
        } else {
            self.error("expected postfix reference")
        }
    }

    // ---- other expressions (atoms) ----

    fn parse_other_expression(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();

        // new - object/array/map/constructor
        if self.try_keyword("new") {
            return self.parse_new_expression();
        }

        // function definition: (...) -> { ... }
        if self.is_function_definition() {
            return self.parse_function_definition();
        }

        // native(...)
        if self.try_keyword("native") {
            return self.parse_native_function_call();
        }

        // fork()
        if self.try_keyword("fork") {
            self.expect_char('(')?;
            self.expect_char(')')?;
            return Ok(Expression::Fork);
        }

        // boolean
        if self.try_keyword("true") {
            return Ok(Expression::Constant {
                value: 1,
                type_name: "Boolean".to_string(),
                string: None,
            });
        }
        if self.try_keyword("false") {
            return Ok(Expression::Constant {
                value: 0,
                type_name: "Boolean".to_string(),
                string: None,
            });
        }

        // undef
        if self.try_keyword("undef") {
            return Ok(Expression::Constant {
                value: 0,
                type_name: "Undefined".to_string(),
                string: None,
            });
        }

        // number
        if let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                return self.parse_number();
            }
        }

        // string
        if self.peek() == Some('"') {
            return self.parse_string();
        }

        // reference (variable / this)
        self.parse_reference()
    }

    fn parse_new_expression(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        // new { ... } - implicit constructor / object definition
        if self.peek() == Some('{') {
            self.advance();
            let stmts = self.parse_statements_until('}')?;
            self.expect_char('}')?;
            return Ok(Expression::ImplicitConstructor { statements: stmts });
        }
        // new [ ... ] - array or map definition
        if self.peek() == Some('[') {
            self.advance();
            return self.parse_array_or_map_definition();
        }
        // new Variable(...) - constructor call
        let var_expr = self.parse_reference()?;
        let expr_with_postfix = self.parse_postfix_chain(var_expr)?;
        Ok(Expression::ConstructorCall {
            function: Box::new(expr_with_postfix),
        })
    }

    fn parse_array_or_map_definition(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        // empty: new []
        if self.peek() == Some(']') {
            self.advance();
            return Ok(Expression::ArrayDefinition {
                elements: Vec::new(),
            });
        }
        // Parse first expression, then look ahead
        let first = self.parse_expression()?;
        self.skip_whitespace();
        // If we see =>, this is a map
        if self.try_str("=>") {
            let first_val = self.parse_expression()?;
            let mut keys = vec![first];
            let mut values = vec![first_val];
            while self.try_char(',') {
                let k = self.parse_expression()?;
                self.skip_whitespace();
                if !self.try_str("=>") {
                    return self.error("expected '=>' in map literal");
                }
                let v = self.parse_expression()?;
                keys.push(k);
                values.push(v);
            }
            self.expect_char(']')?;
            return Ok(Expression::MapDefinition { keys, values });
        }
        // Otherwise it's an array
        let mut elements = vec![first];
        while self.try_char(',') {
            elements.push(self.parse_expression()?);
        }
        self.expect_char(']')?;
        Ok(Expression::ArrayDefinition { elements })
    }

    fn is_function_definition(&mut self) -> bool {
        self.skip_whitespace();
        if self.peek() != Some('(') {
            return false;
        }
        // Save state and look ahead to see if this is (...) -> { ... }
        let state = self.save();
        self.advance(); // skip (
        let mut depth = 1;
        while depth > 0 {
            match self.advance() {
                Some('(') => depth += 1,
                Some(')') => depth -= 1,
                None => {
                    self.restore(state);
                    return false;
                }
                _ => {}
            }
        }
        self.skip_whitespace();
        let result = self.starts_with("->");
        self.restore(state);
        result
    }

    fn parse_function_definition(&mut self) -> Result<Expression, String> {
        self.expect_char('(')?;
        let mut params = Vec::new();
        self.skip_whitespace();
        if self.peek() != Some(')') {
            params.push(self.parse_identifier()?);
            while self.try_char(',') {
                params.push(self.parse_identifier()?);
            }
        }
        self.expect_char(')')?;
        self.skip_whitespace();
        if !self.try_str("->") {
            return self.error("expected '->' in function definition");
        }
        self.expect_char('{')?;
        let stmts = self.parse_statements_until('}')?;
        self.expect_char('}')?;
        Ok(Expression::FunctionDef {
            parameters: params,
            statements: stmts,
        })
    }

    fn parse_native_function_call(&mut self) -> Result<Expression, String> {
        self.expect_char('(')?;
        let args = self.parse_function_arguments()?;
        self.expect_char(')')?;
        Ok(Expression::NativeFunctionCall { parameters: args })
    }

    fn parse_function_arguments(&mut self) -> Result<Vec<Expression>, String> {
        self.skip_whitespace();
        if self.peek() == Some(')') {
            return Ok(Vec::new());
        }
        let mut args = vec![self.parse_expression()?];
        while self.try_char(',') {
            args.push(self.parse_expression()?);
        }
        Ok(args)
    }

    // ---- references ----

    fn parse_reference(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        self.parse_reference_chain()
    }

    fn parse_reference_chain(&mut self) -> Result<Expression, String> {
        self.skip_whitespace();
        let base = if self.try_keyword("this") {
            Expression::This { field: None }
        } else {
            let name = self.parse_identifier()?;
            Expression::Variable {
                name,
                field: None,
            }
        };
        self.build_dotted_reference(base)
    }

    fn build_dotted_reference(&mut self, base: Expression) -> Result<Expression, String> {
        let state = self.save();
        if self.try_char('.') {
            let mut sub = self.parse_reference_chain()?;
            self.set_deepest_field(&mut sub, base);
            Ok(sub)
        } else {
            self.restore(state);
            Ok(base)
        }
    }

    fn set_deepest_field(&self, expr: &mut Expression, parent: Expression) {
        match expr {
            Expression::Variable { field, .. } => {
                if let Some(inner) = field {
                    self.set_deepest_field(inner, parent);
                } else {
                    *field = Some(Box::new(parent));
                }
            }
            Expression::This { field, .. } => {
                if let Some(inner) = field {
                    self.set_deepest_field(inner, parent);
                } else {
                    *field = Some(Box::new(parent));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_module() {
        let input = "module test;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.name, "test");
        assert!(module.imports.is_empty());
        assert!(module.statements.is_empty());
    }

    #[test]
    fn test_parse_module_with_imports() {
        let input = "module main; import utils; import math;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.name, "main");
        assert_eq!(module.imports, vec!["utils", "math"]);
    }

    #[test]
    fn test_parse_number() {
        let input = "module test; 42;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_string() {
        let input = r#"module test; "hello";"#;
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_variable_assignment() {
        let input = "module test; x = 10;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_if_else() {
        let input = "module test; if (x == 1) { y = 2; } else { y = 3; }";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_while() {
        let input = "module test; while (x < 10) { x = x + 1; }";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_for() {
        let input = "module test; for (i = 0; i < 10; i++) { x = x + i; }";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_function_definition() {
        let input = "module test; add = (a, b) -> { return a + b; };";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_function_call() {
        let input = "module test; add(1, 2);";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_comments() {
        let input = "module test; // line comment\nx = 1; /* block */ y = 2;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 2);
    }

    #[test]
    fn test_parse_dot_access() {
        let input = "module test; this.x = 10;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_try_catch() {
        let input = "module test; try { x = 1; } catch (e) { y = 2; }";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_new_object() {
        let input = "module test; obj = new { x = 1; y = 2; };";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_new_array() {
        let input = "module test; arr = new [1, 2, 3];";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_new_map() {
        let input = r#"module test; m = new ["a" => 1, "b" => 2];"#;
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_ternary() {
        let input = "module test; x = a ? 1 : 0;";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_method_call() {
        let input = "module test; obj.method(1, 2);";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_array_index() {
        let input = "module test; arr[0];";
        let mut parser = Parser::new(input);
        let module = parser.parse_program().unwrap();
        assert_eq!(module.statements.len(), 1);
    }

    #[test]
    fn test_parse_statements_only() {
        let input = "x = 1; y = 2;";
        let mut parser = Parser::new(input);
        let stmts = parser.parse_statements_only().unwrap();
        assert_eq!(stmts.len(), 2);
    }
}
