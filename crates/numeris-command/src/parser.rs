//! # Parser
//!
//! Recursive-descent parser producing the typed [`Command`] AST.

use crate::ast::*;
use crate::lexer::{Tok, Token};
use numeris_core::error::{NumerisError, Result};

pub fn parse(source: &str) -> Result<Command> {
    let tokens = crate::lexer::lex(source)?;
    let mut p = Parser { tokens, i: 0 };
    let cmd = p.command()?;
    p.expect_eof()?;
    Ok(cmd)
}

struct Parser {
    tokens: Vec<Token>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.tokens[self.i].tok
    }

    fn pos(&self) -> usize {
        self.tokens[self.i].pos
    }

    fn advance(&mut self) -> Tok {
        let t = self.tokens[self.i].tok.clone();
        if self.i + 1 < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    fn eat(&mut self, expected: &Tok) -> Result<()> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(self.unexpected(expected))
        }
    }

    fn eat_ident(&mut self) -> Result<String> {
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.unexpected_tok(&other)),
        }
    }

    fn eat_number(&mut self) -> Result<f64> {
        match self.peek().clone() {
            Tok::Number(v) => {
                self.advance();
                Ok(v)
            }
            other => Err(self.unexpected_tok(&other)),
        }
    }

    fn eat_string(&mut self) -> Result<String> {
        match self.peek().clone() {
            Tok::Str(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(self.unexpected_tok(&other)),
        }
    }

    fn at_command_end(&self) -> bool {
        matches!(self.peek(), Tok::Eof)
    }

    fn at_option_boundary(&self) -> bool {
        matches!(self.peek(), Tok::Comma | Tok::Eof | Tok::RParen)
    }

    fn unexpected(&self, expected: &Tok) -> NumerisError {
        NumerisError::syntax(
            format!(
                "Unexpected {} at position {}; expected {}.",
                describe(self.peek()),
                self.pos(),
                describe(expected)
            ),
            "The command does not match the Numeris command grammar at this point.",
            "Check the command syntax with 'help' and try again.",
        )
    }

    fn unexpected_tok(&self, got: &Tok) -> NumerisError {
        NumerisError::syntax(
            format!("Unexpected {} at position {}.", describe(got), self.pos()),
            "The command does not match the Numeris command grammar at this point.",
            "Check the command syntax with 'help' and try again.",
        )
    }

    fn expect_eof(&mut self) -> Result<()> {
        match self.peek() {
            Tok::Eof => Ok(()),
            _ => Err(NumerisError::syntax(
                format!(
                    "Unexpected {} at position {} after the end of the command.",
                    describe(self.peek()),
                    self.pos()
                ),
                "The command appears complete, but extra tokens follow it.",
                "Remove the extra text or split it into separate commands.",
            )),
        }
    }

    fn command(&mut self) -> Result<Command> {
        let head = self.eat_ident()?;
        let lower = head.to_lowercase();
        match lower.as_str() {
            "use" | "import" => {
                let path = if matches!(self.peek(), Tok::Str(_)) {
                    self.eat_string()?
                } else {
                    let mut parts = vec![self.eat_ident()?];
                    while matches!(self.peek(), Tok::Slash) {
                        self.advance();
                        parts.push(self.eat_ident()?);
                    }
                    parts.join("/")
                };
                Ok(Command::Use { path })
            }
            "summarize" | "summarise" | "su" => {
                let variables = self.varlist()?;
                let detail = self.eat_trailing_flag("detail")?;
                Ok(Command::Summarize { variables, detail })
            }
            "describe" | "des" => {
                let variables = self.varlist()?;
                Ok(Command::Describe { variables })
            }
            "correlate" | "corr" => {
                let variables = self.varlist()?;
                let spearman = self.eat_trailing_flag("spearman")?;
                Ok(Command::Correlate {
                    variables,
                    spearman,
                })
            }
            "ttest" => {
                let variable = self.eat_ident()?;
                match self.peek() {
                    Tok::Eq => {
                        self.advance();
                        if matches!(self.peek(), Tok::Number(_)) {
                            let mu = self.eat_number()?;
                            Ok(Command::Ttest {
                                variable,
                                form: TtestForm::OneSample { mu },
                            })
                        } else {
                            // ttest a == b [, paired]
                            let other = self.eat_ident()?;
                            let mut paired = false;
                            if matches!(self.peek(), Tok::Comma) {
                                self.advance();
                                let opt = self.eat_ident()?.to_lowercase();
                                match opt.as_str() {
                                    "paired" => paired = true,
                                    other => {
                                        return Err(NumerisError::syntax(
                                            format!("Unknown t-test option '{other}'."),
                                            "Two-variable t-tests accept the 'paired' option.",
                                            "For example: ttest before == after, paired.",
                                        ));
                                    }
                                }
                            }
                            Ok(Command::Ttest {
                                variable,
                                form: TtestForm::TwoVariables { other, paired },
                            })
                        }
                    }
                    Tok::Comma => {
                        self.advance();
                        let by = self.eat_ident()?;
                        if by.to_lowercase() != "by" {
                            return Err(NumerisError::syntax(
                                "The option after ',' in a t-test must be 'by'.",
                                "Numeris supports: ttest x, by(group) and ttest x, by(group) welch.",
                                "Write the grouping variable inside by(), for example: ttest wage, by(female).",
                            ));
                        }
                        self.eat(&Tok::LParen)?;
                        let group = self.eat_ident()?;
                        self.eat(&Tok::RParen)?;
                        let welch = self.eat_flag("welch")?;
                        Ok(Command::Ttest {
                            variable,
                            form: TtestForm::ByGroup { group, welch },
                        })
                    }
                    _ => Err(NumerisError::syntax(
                        format!("Unexpected {} after the variable in 'ttest'.", describe(self.peek())),
                        "A t-test needs a comparison: 'ttest x == value', 'ttest x, by(group)' or 'ttest a == b'.",
                        "For example: ttest wage == 20 or ttest wage, by(female).",
                    )),
                }
            }
            "anova" => {
                let outcome = self.eat_ident()?;
                let group = self.eat_ident()?;
                Ok(Command::Anova { outcome, group })
            }
            "regress" | "reg" => {
                let spec = self.model_spec()?;
                Ok(Command::Regress { spec })
            }
            "xtreg" => {
                let outcome = self.eat_ident()?;
                let mut predictors = Vec::new();
                while !self.at_option_boundary() {
                    predictors.push(self.eat_ident()?);
                }
                // Options: entity(v) required; fe|be|pooled model; vcov.
                let mut entity = None;
                let mut model = PanelModel::Pooled;
                let mut vcov = VcovOption::Classical;
                if matches!(self.peek(), Tok::Comma) {
                    self.advance();
                    loop {
                        let opt = self.eat_ident()?.to_lowercase();
                        match opt.as_str() {
                            "fe" | "fixed" => model = PanelModel::Fe,
                            "be" | "between" => model = PanelModel::Be,
                            "pooled" => model = PanelModel::Pooled,
                            "entity" | "id" | "i" => {
                                self.eat(&Tok::LParen)?;
                                entity = Some(self.eat_ident()?);
                                self.eat(&Tok::RParen)?;
                            }
                            "robust" => vcov = VcovOption::Robust,
                            "hc2" => vcov = VcovOption::Hc2,
                            "hc3" => vcov = VcovOption::Hc3,
                            "cluster" => {
                                self.eat(&Tok::LParen)?;
                                let v = self.eat_ident()?;
                                self.eat(&Tok::RParen)?;
                                vcov = VcovOption::Cluster { variable: v };
                            }
                            "vce" => {
                                self.eat(&Tok::LParen)?;
                                self.eat_ident()?; // 'hac'
                                let lags = self.eat_number()? as usize;
                                self.eat(&Tok::RParen)?;
                                vcov = VcovOption::Hac { lags };
                            }
                            other => {
                                return Err(NumerisError::syntax(
                                    format!("Unknown xtreg option '{other}'."),
                                    "xtreg accepts: fe, be, pooled, entity(v), robust, hc2, hc3, cluster(v), vce(hac L).",
                                    "Remove the unknown option or check 'help xtreg'.",
                                ));
                            }
                        }
                        if matches!(self.peek(), Tok::Comma) {
                            self.advance();
                            continue;
                        }
                        if matches!(self.peek(), Tok::Ident(_)) {
                            continue; // space-separated option
                        }
                        break;
                    }
                }
                let entity = entity.ok_or_else(|| {
                    NumerisError::syntax(
                        "xtreg requires the entity() option.",
                        "Panel estimators need a variable that identifies entities (firms, persons, regions).",
                        "Add entity(variable) to the command, for example: xtreg y x, fe entity(firm).",
                    )
                })?;
                Ok(Command::Xtreg {
                    outcome,
                    predictors,
                    entity,
                    model,
                    vcov,
                })
            }
            "ivregress" => {
                // ivregress 2sls y (endog = inst1 inst2) exogs, robust
                let estimator = self.eat_ident()?;
                if estimator.to_lowercase() != "2sls" {
                    return Err(NumerisError::invalid_request(
                        format!(
                            "ivregress supports the 2sls estimator; '{estimator}' was requested."
                        ),
                        "V1 implements two-stage least squares.",
                        "Use: ivregress 2sls y (endogenous = instruments) exogenous_controls.",
                    ));
                }
                let outcome = self.eat_ident()?;
                let mut endogenous = Vec::new();
                let mut instruments = Vec::new();
                let mut exogenous = Vec::new();
                if matches!(self.peek(), Tok::LParen) {
                    self.advance();
                    loop {
                        endogenous.push(self.eat_ident()?);
                        if matches!(self.peek(), Tok::Comma) {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                    // '='
                    self.eat(&Tok::Eq)?;
                    loop {
                        instruments.push(self.eat_ident()?);
                        if matches!(self.peek(), Tok::Comma) {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                    self.eat(&Tok::RParen)?;
                } else {
                    return Err(NumerisError::syntax(
                        "ivregress requires instrumented variables in parentheses.",
                        "The endogenous regressor and its instruments must be listed as (endog = inst1 inst2).",
                        "For example: ivregress 2sls wage (education = distance), robust.",
                    ));
                }
                while !self.at_option_boundary() {
                    exogenous.push(self.eat_ident()?);
                }
                let vcov = self.vcov_options()?;
                Ok(Command::Ivregress {
                    estimator,
                    outcome,
                    endogenous,
                    instruments,
                    exogenous,
                    vcov,
                })
            }
            "did" => {
                let outcome = self.eat_ident()?;
                let mut controls = Vec::new();
                while !self.at_option_boundary() {
                    controls.push(self.eat_ident()?);
                }
                let mut treat = None;
                let mut time = None;
                let mut entity = None;
                let mut vcov = VcovOption::Classical;
                if matches!(self.peek(), Tok::Comma) {
                    self.advance();
                    loop {
                        let opt = self.eat_ident()?.to_lowercase();
                        match opt.as_str() {
                            "treat" => {
                                self.eat(&Tok::LParen)?;
                                treat = Some(self.eat_ident()?);
                                self.eat(&Tok::RParen)?;
                            }
                            "time" | "post" => {
                                self.eat(&Tok::LParen)?;
                                time = Some(self.eat_ident()?);
                                self.eat(&Tok::RParen)?;
                            }
                            "entity" => {
                                self.eat(&Tok::LParen)?;
                                entity = Some(self.eat_ident()?);
                                self.eat(&Tok::RParen)?;
                            }
                            "robust" => vcov = VcovOption::Robust,
                            "cluster" => {
                                self.eat(&Tok::LParen)?;
                                let v = self.eat_ident()?;
                                self.eat(&Tok::RParen)?;
                                vcov = VcovOption::Cluster { variable: v };
                            }
                            other => {
                                return Err(NumerisError::syntax(
                                    format!("Unknown did option '{other}'."),
                                    "did accepts: treat(v), time(v), entity(v), robust, cluster(v).",
                                    "Remove the unknown option or check 'help did'.",
                                ));
                            }
                        }
                        if matches!(self.peek(), Tok::Comma) {
                            self.advance();
                            continue;
                        }
                        if matches!(self.peek(), Tok::Ident(_)) {
                            continue; // space-separated option
                        }
                        break;
                    }
                }
                let treat = treat.ok_or_else(|| {
                    NumerisError::syntax(
                        "did requires the treat() option.",
                        "DID needs a treatment indicator variable (1 = treated, 0 = control).",
                        "Add treat(variable) to the command.",
                    )
                })?;
                let time = time.ok_or_else(|| {
                    NumerisError::syntax(
                        "did requires the time() option.",
                        "DID needs a post-period indicator variable (1 = after, 0 = before).",
                        "Add time(variable) to the command.",
                    )
                })?;
                Ok(Command::Did {
                    outcome,
                    controls,
                    treat,
                    time,
                    entity,
                    vcov,
                })
            }
            "logit" | "logistic" => Ok(Command::Logit {
                spec: self.glm_spec()?,
            }),
            "probit" => Ok(Command::Probit {
                spec: self.glm_spec()?,
            }),
            "poisson" => Ok(Command::Poisson {
                spec: self.glm_spec()?,
            }),
            "generate" | "gen" => {
                let name = self.eat_ident()?;
                self.eat(&Tok::Eq)?;
                let expr = self.expression()?;
                Ok(Command::Generate { name, expr })
            }
            "replace" => {
                let name = self.eat_ident()?;
                self.eat(&Tok::Eq)?;
                let expr = self.expression()?;
                let cond = if self.at_ident("if") {
                    self.advance();
                    Some(self.condition()?)
                } else {
                    None
                };
                Ok(Command::Replace { name, expr, cond })
            }
            "drop" => {
                if self.at_ident("if") {
                    self.advance();
                    let cond = self.condition()?;
                    Ok(Command::Drop {
                        targets: DropTarget::If(cond),
                    })
                } else {
                    let vars = self.varlist()?;
                    Ok(Command::Drop {
                        targets: DropTarget::Variables(vars),
                    })
                }
            }
            "keep" => {
                if self.at_ident("if") {
                    self.advance();
                    let cond = self.condition()?;
                    Ok(Command::Keep {
                        targets: KeepTarget::If(cond),
                    })
                } else {
                    let vars = self.varlist()?;
                    Ok(Command::Keep {
                        targets: KeepTarget::Variables(vars),
                    })
                }
            }
            "rename" => {
                let from = self.eat_ident()?;
                let to = self.eat_ident()?;
                Ok(Command::Rename { from, to })
            }
            "label" => {
                let next = self.eat_ident()?;
                if next.to_lowercase() != "variable" {
                    return Err(NumerisError::syntax(
                        "'label' must be followed by 'variable'.",
                        "V1 supports attaching variable labels.",
                        "Use: label variable varname \"label text\".",
                    ));
                }
                let variable = self.eat_ident()?;
                let text = self.eat_string()?;
                Ok(Command::Label { variable, text })
            }
            "sort" => {
                let variable = self.eat_ident()?;
                let descending =
                    self.eat_trailing_flag("desc")? || self.eat_trailing_flag("descending")?;
                Ok(Command::Sort {
                    variable,
                    descending,
                })
            }
            "estimate" => {
                let sub = self.eat_ident()?.to_lowercase();
                match sub.as_str() {
                    "list" => Ok(Command::EstimateList),
                    "compare" => {
                        let mut ids = Vec::new();
                        while !self.at_command_end() {
                            ids.push(self.eat_ident()?);
                        }
                        Ok(Command::EstimateCompare { ids })
                    }
                    other => Err(NumerisError::syntax(
                        format!("Unknown 'estimate {other}' subcommand."),
                        "estimate supports 'list' and 'compare'.",
                        "Use 'estimate list' or 'estimate compare m1 m2'.",
                    )),
                }
            }
            "note" => {
                let text = self.eat_string()?;
                Ok(Command::Note { text })
            }
            "notes" => Ok(Command::Notes),
            "export" => {
                let what = self.eat_ident()?.to_lowercase();
                match what.as_str() {
                    "table" => {
                        // export table "path.md" [ids...]
                        let path = self.eat_string()?;
                        let format = match path
                            .rsplit('.')
                            .next()
                            .unwrap_or("")
                            .to_lowercase()
                            .as_str()
                        {
                            "md" | "markdown" => TableFormat::Markdown,
                            "csv" => TableFormat::Csv,
                            "tex" | "latex" => TableFormat::Latex,
                            "html" | "htm" => TableFormat::Html,
                            other => {
                                return Err(NumerisError::invalid_request(
                                    format!("Unsupported table format '.{other}'."),
                                    "Tables export to Markdown (.md), CSV (.csv), LaTeX (.tex) and HTML (.html).",
                                    "Choose one of the supported file extensions.",
                                ));
                            }
                        };
                        let mut ids = Vec::new();
                        while !self.at_command_end() {
                            ids.push(self.eat_ident()?);
                        }
                        Ok(Command::ExportTable { path, format, ids })
                    }
                    "data" => {
                        let path = self.eat_string()?;
                        Ok(Command::ExportData { path })
                    }
                    other => Err(NumerisError::syntax(
                        format!("Unknown export target '{other}'."),
                        "export supports 'table' and 'data'.",
                        "Use: export table \"file.md\" or export data \"file.csv\".",
                    )),
                }
            }
            "help" => {
                if matches!(self.peek(), Tok::Ident(_)) {
                    Ok(Command::Help {
                        command: Some(self.eat_ident()?),
                    })
                } else {
                    Ok(Command::Help { command: None })
                }
            }
            other => Err(NumerisError::syntax(
                format!("Unknown command '{other}'."),
                "The first word of a command must be a recognized Numeris command.",
                "Run 'help' to see the command list and syntax.",
            )),
        }
    }

    fn at_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s.eq_ignore_ascii_case(name))
    }

    fn varlist(&mut self) -> Result<Vec<String>> {
        let mut vars = Vec::new();
        while matches!(self.peek(), Tok::Ident(_)) {
            vars.push(self.eat_ident()?);
        }
        Ok(vars)
    }

    /// Parse `y x1 x2 ..., options`.
    fn model_spec(&mut self) -> Result<ModelSpec> {
        let outcome = self.eat_ident()?;
        let mut predictors = Vec::new();
        while !self.at_option_boundary() {
            predictors.push(self.eat_ident()?);
        }
        let vcov = self.vcov_options()?;
        let no_constant = false;
        let weights = None;
        Ok(ModelSpec {
            outcome,
            predictors,
            vcov,
            no_constant,
            weights,
        })
    }

    fn glm_spec(&mut self) -> Result<GlmSpec> {
        let outcome = self.eat_ident()?;
        let mut predictors = Vec::new();
        while !self.at_option_boundary() {
            predictors.push(self.eat_ident()?);
        }
        let mut robust = false;
        if matches!(self.peek(), Tok::Comma) {
            self.advance();
            let opt = self.eat_ident()?.to_lowercase();
            if opt == "robust" {
                robust = true;
            } else {
                return Err(NumerisError::syntax(
                    format!("Unknown option '{opt}'."),
                    "GLM commands accept the 'robust' option.",
                    "For example: logit employed education, robust.",
                ));
            }
        }
        Ok(GlmSpec {
            outcome,
            predictors,
            robust,
        })
    }

    fn vcov_options(&mut self) -> Result<VcovOption> {
        let mut vcov = VcovOption::Classical;
        if matches!(self.peek(), Tok::Comma) {
            self.advance();
            loop {
                match self.peek().clone() {
                    Tok::Ident(opt) => {
                        self.advance();
                        match opt.to_lowercase().as_str() {
                            "robust" => vcov = VcovOption::Robust,
                            "hc2" => vcov = VcovOption::Hc2,
                            "hc3" => vcov = VcovOption::Hc3,
                            "cluster" => {
                                self.eat(&Tok::LParen)?;
                                let v = self.eat_ident()?;
                                self.eat(&Tok::RParen)?;
                                vcov = VcovOption::Cluster { variable: v };
                            }
                            "vce" => {
                                self.eat(&Tok::LParen)?;
                                self.eat_ident()?; // 'hac'
                                let lags = self.eat_number()? as usize;
                                self.eat(&Tok::RParen)?;
                                vcov = VcovOption::Hac { lags };
                            }
                            other => {
                                return Err(NumerisError::syntax(
                                    format!("Unknown option '{other}'."),
                                    "Regression commands accept: robust, hc2, hc3, cluster(v), vce(hac L), noconstant.",
                                    "Remove the unknown option or check 'help regress'.",
                                ));
                            }
                        }
                    }
                    other => return Err(self.unexpected_tok(&other)),
                }
                if matches!(self.peek(), Tok::Comma) {
                    self.advance();
                    continue;
                }
                if matches!(self.peek(), Tok::Ident(_)) {
                    continue; // space-separated option
                }
                break;
            }
        }
        Ok(vcov)
    }

    /// Optional trailing flag that appears immediately after the current
    /// position (no comma): used after `by(group)` for `welch`.
    fn eat_flag(&mut self, flag: &str) -> Result<bool> {
        if self.at_ident(flag) {
            self.advance();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Optional flag after a comma: `, detail` / `, spearman` / `, desc`.
    fn eat_trailing_flag(&mut self, flag: &str) -> Result<bool> {
        if !matches!(self.peek(), Tok::Comma) {
            return Ok(false);
        }
        self.advance();
        match self.peek().clone() {
            Tok::Ident(s) if s.eq_ignore_ascii_case(flag) => {
                self.advance();
                Ok(true)
            }
            Tok::Ident(other) => Err(NumerisError::syntax(
                format!("Unknown option '{other}'."),
                "The command accepts a different set of options.",
                format!("Use the '{flag}' option or remove it."),
            )),
            other => Err(self.unexpected_tok(&other)),
        }
    }

    fn condition(&mut self) -> Result<Cond> {
        let variable = self.eat_ident()?;
        let op = match self.peek() {
            Tok::Eq => {
                self.advance();
                CondOp::Eq
            }
            Tok::NotEq => {
                self.advance();
                CondOp::Ne
            }
            Tok::Gt => {
                self.advance();
                CondOp::Gt
            }
            Tok::Ge => {
                self.advance();
                CondOp::Ge
            }
            Tok::Lt => {
                self.advance();
                CondOp::Lt
            }
            Tok::Le => {
                self.advance();
                CondOp::Le
            }
            other => {
                return Err(NumerisError::syntax(
                    format!("Expected a comparison operator, found {}.", describe(other)),
                    "Conditions compare a variable to a number with ==, !=, >, >=, < or <=.",
                    "For example: drop if wage < 0.",
                ));
            }
        };
        let value = self.eat_number()?;
        Ok(Cond {
            variable,
            op,
            value,
        })
    }

    // Expression grammar: term ((+|-) term)*
    // term: factor ((*|/) factor)*
    // factor: unary (^ factor)?
    // unary: ('-')? primary
    // primary: number | ident | ident '(' expr ')' | '(' expr ')'
    fn expression(&mut self) -> Result<Expr> {
        let mut left = self.term()?;
        loop {
            match self.peek() {
                Tok::Plus => {
                    self.advance();
                    let right = self.term()?;
                    left = Expr::BinOp {
                        op: BinOpKind::Add,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                Tok::Minus => {
                    self.advance();
                    let right = self.term()?;
                    left = Expr::BinOp {
                        op: BinOpKind::Sub,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                _ => return Ok(left),
            }
        }
    }

    fn term(&mut self) -> Result<Expr> {
        let mut left = self.factor()?;
        loop {
            match self.peek() {
                Tok::Star => {
                    self.advance();
                    let right = self.factor()?;
                    left = Expr::BinOp {
                        op: BinOpKind::Mul,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                Tok::Slash => {
                    self.advance();
                    let right = self.factor()?;
                    left = Expr::BinOp {
                        op: BinOpKind::Div,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                _ => return Ok(left),
            }
        }
    }

    fn factor(&mut self) -> Result<Expr> {
        let base = self.unary()?;
        if matches!(self.peek(), Tok::Caret) {
            self.advance();
            let exp = self.factor()?;
            return Ok(Expr::BinOp {
                op: BinOpKind::Pow,
                left: Box::new(base),
                right: Box::new(exp),
            });
        }
        Ok(base)
    }

    fn unary(&mut self) -> Result<Expr> {
        if matches!(self.peek(), Tok::Minus) {
            self.advance();
            let inner = self.unary()?;
            return Ok(Expr::Neg(Box::new(inner)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr> {
        match self.peek().clone() {
            Tok::Number(v) => {
                self.advance();
                Ok(Expr::Number(v))
            }
            Tok::LParen => {
                self.advance();
                let inner = self.expression()?;
                self.eat(&Tok::RParen)?;
                Ok(inner)
            }
            Tok::Ident(name) => {
                self.advance();
                let lower = name.to_lowercase();
                let function = match lower.as_str() {
                    "ln" => Some(FnKind::Ln),
                    "log" => Some(FnKind::Log),
                    "exp" => Some(FnKind::Exp),
                    "sqrt" => Some(FnKind::Sqrt),
                    "abs" => Some(FnKind::Abs),
                    "round" => Some(FnKind::Round),
                    "rank" => Some(FnKind::Rank),
                    "standardize" => Some(FnKind::Standardize),
                    _ => None,
                };
                if let Some(f) = function {
                    self.eat(&Tok::LParen)?;
                    let arg = self.expression()?;
                    self.eat(&Tok::RParen)?;
                    return Ok(Expr::Call {
                        function: f,
                        arg: Box::new(arg),
                    });
                }
                Ok(Expr::Var(name))
            }
            other => Err(self.unexpected_tok(&other)),
        }
    }
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Ident(s) => format!("'{s}'"),
        Tok::Number(v) => format!("number {v}"),
        Tok::Str(_) => "a quoted string".to_string(),
        Tok::Comma => "','".to_string(),
        Tok::LParen => "'('".to_string(),
        Tok::RParen => "')'".to_string(),
        Tok::Eq => "'='".to_string(),
        Tok::NotEq => "'!='".to_string(),
        Tok::Gt => "'>'".to_string(),
        Tok::Ge => "'>='".to_string(),
        Tok::Lt => "'<'".to_string(),
        Tok::Le => "'<='".to_string(),
        Tok::Plus => "'+'".to_string(),
        Tok::Minus => "'-'".to_string(),
        Tok::Star => "'*'".to_string(),
        Tok::Slash => "'/'".to_string(),
        Tok::Caret => "'^'".to_string(),
        Tok::Eof => "end of command".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_regress_with_robust() {
        let cmd = parse("regress wage education experience female, robust").unwrap();
        assert_eq!(
            cmd,
            Command::Regress {
                spec: ModelSpec {
                    outcome: "wage".into(),
                    predictors: vec!["education".into(), "experience".into(), "female".into()],
                    vcov: VcovOption::Robust,
                    no_constant: false,
                    weights: None,
                }
            }
        );
    }

    #[test]
    fn parses_cluster_and_hac() {
        let cmd = parse("regress y x1 x2, cluster(firm)").unwrap();
        assert_eq!(
            cmd,
            Command::Regress {
                spec: ModelSpec {
                    outcome: "y".into(),
                    predictors: vec!["x1".into(), "x2".into()],
                    vcov: VcovOption::Cluster {
                        variable: "firm".into()
                    },
                    no_constant: false,
                    weights: None,
                }
            }
        );
        let cmd = parse("regress y x, vce(hac 4)").unwrap();
        match cmd {
            Command::Regress { spec } => assert_eq!(spec.vcov, VcovOption::Hac { lags: 4 }),
            _ => panic!(),
        }
    }

    #[test]
    fn parses_xtreg_fe() {
        let cmd = parse("xtreg wage education experience, fe entity(firm) cluster(firm)").unwrap();
        match cmd {
            Command::Xtreg {
                outcome,
                predictors,
                entity,
                model,
                vcov,
            } => {
                assert_eq!(outcome, "wage");
                assert_eq!(predictors, vec!["education", "experience"]);
                assert_eq!(entity, "firm");
                assert_eq!(model, PanelModel::Fe);
                assert_eq!(
                    vcov,
                    VcovOption::Cluster {
                        variable: "firm".into()
                    }
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parses_ivregress() {
        let cmd = parse("ivregress 2sls wage (education = distance), robust").unwrap();
        match cmd {
            Command::Ivregress {
                estimator,
                outcome,
                endogenous,
                instruments,
                exogenous,
                vcov,
            } => {
                assert_eq!(estimator, "2sls");
                assert_eq!(outcome, "wage");
                assert_eq!(endogenous, vec!["education"]);
                assert_eq!(instruments, vec!["distance"]);
                assert!(exogenous.is_empty());
                assert_eq!(vcov, VcovOption::Robust);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parses_did() {
        let cmd = parse("did wage, treat(treated) time(post) cluster(firm)").unwrap();
        match cmd {
            Command::Did {
                outcome,
                controls,
                treat,
                time,
                entity,
                vcov,
            } => {
                assert_eq!(outcome, "wage");
                assert!(controls.is_empty());
                assert_eq!(treat, "treated");
                assert_eq!(time, "post");
                assert_eq!(entity, None);
                assert_eq!(
                    vcov,
                    VcovOption::Cluster {
                        variable: "firm".into()
                    }
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parses_generate_expressions() {
        let cmd = parse("generate logwage = ln(wage) + 0.5 * experience^2").unwrap();
        match cmd {
            Command::Generate { name, expr } => {
                assert_eq!(name, "logwage");
                assert!(matches!(
                    expr,
                    Expr::BinOp {
                        op: BinOpKind::Add,
                        ..
                    }
                ));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parses_ttest_forms() {
        let c = parse("ttest wage == 20").unwrap();
        assert_eq!(
            c,
            Command::Ttest {
                variable: "wage".into(),
                form: TtestForm::OneSample { mu: 20.0 },
            }
        );
        let c = parse("ttest wage, by(female) welch").unwrap();
        assert_eq!(
            c,
            Command::Ttest {
                variable: "wage".into(),
                form: TtestForm::ByGroup {
                    group: "female".into(),
                    welch: true,
                },
            }
        );
    }

    #[test]
    fn parses_data_management() {
        assert!(matches!(
            parse("drop if wage < 0").unwrap(),
            Command::Drop { targets: DropTarget::If(Cond { variable, op: CondOp::Lt, value: 0.0 }) } if variable == "wage"
        ));
        assert!(matches!(
            parse("keep wage education").unwrap(),
            Command::Keep { targets: KeepTarget::Variables(v) } if v.len() == 2
        ));
        assert!(matches!(
            parse(r#"label variable wage "Hourly wage""#).unwrap(),
            Command::Label { variable, text } if variable == "wage" && text == "Hourly wage"
        ));
    }

    #[test]
    fn unknown_command_is_a_friendly_error() {
        let err = parse("frobnicate y x").unwrap_err();
        assert!(err.what.contains("Unknown command 'frobnicate'"));
        assert!(err.action.contains("help"));
    }

    #[test]
    fn extra_tokens_after_command_rejected() {
        let err = parse("summarize wage extra )").unwrap_err();
        assert!(err.what.contains("after the end of the command"));
    }
}
