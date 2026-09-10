//! # Canonical command rendering
//!
//! Renders a typed [`Command`] AST back to its canonical command string.
//! The GUI builds an AST from a form, renders it, and shows the command to
//! the user; executing that command parses back to the same AST. This is
//! the mechanism that guarantees GUI ⇄ command equivalence with a single
//! execution path.

use crate::ast::*;

pub fn render(cmd: &Command) -> String {
    match cmd {
        Command::Use { path } => format!("use \"{}\"", path),
        Command::Summarize { variables, detail } => {
            let mut s = list("summarize", variables);
            if *detail {
                s.push_str(", detail");
            }
            s
        }
        Command::Describe { variables } => list("describe", variables),
        Command::Correlate {
            variables,
            spearman,
        } => {
            let mut s = list("correlate", variables);
            if *spearman {
                s.push_str(", spearman");
            }
            s
        }
        Command::Ttest { variable, form } => match form {
            TtestForm::OneSample { mu } => format!("ttest {variable} == {}", fmt_num(*mu)),
            TtestForm::ByGroup { group, welch } => format!(
                "ttest {variable}, by({group}){}",
                if *welch { " welch" } else { "" }
            ),
            TtestForm::TwoVariables { other, paired } => format!(
                "ttest {variable} == {other}{}",
                if *paired { ", paired" } else { "" }
            ),
        },
        Command::Anova { outcome, group } => format!("anova {outcome} {group}"),
        Command::Regress { spec } => format!("regress {}", render_model(spec)),
        Command::Xtreg {
            outcome,
            predictors,
            entity,
            model,
            vcov,
        } => {
            let mut s = format!("xtreg {outcome}");
            for p in predictors {
                s.push(' ');
                s.push_str(p);
            }
            s.push_str(", ");
            s.push_str(match model {
                PanelModel::Fe => "fe",
                PanelModel::Be => "be",
                PanelModel::Pooled => "pooled",
            });
            s.push_str(&format!(", entity({entity})"));
            s.push_str(&render_vcov_suffix(vcov));
            s
        }
        Command::Ivregress {
            estimator,
            outcome,
            endogenous,
            instruments,
            exogenous,
            vcov,
        } => {
            let mut s = format!("ivregress {estimator} {outcome} (");
            s.push_str(&endogenous.join(" "));
            s.push_str(" = ");
            s.push_str(&instruments.join(" "));
            s.push(')');
            for e in exogenous {
                s.push(' ');
                s.push_str(e);
            }
            s.push_str(&render_vcov_suffix(vcov));
            s
        }
        Command::Did {
            outcome,
            controls,
            treat,
            time,
            entity,
            vcov,
        } => {
            let mut s = format!("did {outcome}");
            for c in controls {
                s.push(' ');
                s.push_str(c);
            }
            s.push_str(", ");
            s.push_str(&format!("treat({treat}), time({time})"));
            if let Some(e) = entity {
                s.push_str(&format!(", entity({e})"));
            }
            s.push_str(&render_vcov_suffix(vcov));
            s
        }
        Command::Logit { spec } => format!("logit {}", render_glm(spec)),
        Command::Probit { spec } => format!("probit {}", render_glm(spec)),
        Command::Poisson { spec } => format!("poisson {}", render_glm(spec)),
        Command::Generate { name, expr } => format!("generate {name} = {}", render_expr(expr)),
        Command::Replace { name, expr, cond } => match cond {
            Some(c) => format!(
                "replace {name} = {} if {}",
                render_expr(expr),
                render_cond(c)
            ),
            None => format!("replace {name} = {}", render_expr(expr)),
        },
        Command::Drop { targets } => match targets {
            DropTarget::Variables(vars) => list("drop", vars),
            DropTarget::If(c) => format!("drop if {}", render_cond(c)),
        },
        Command::Keep { targets } => match targets {
            KeepTarget::Variables(vars) => list("keep", vars),
            KeepTarget::If(c) => format!("keep if {}", render_cond(c)),
        },
        Command::Rename { from, to } => format!("rename {from} {to}"),
        Command::Label { variable, text } => format!("label variable {variable} \"{text}\""),
        Command::Sort {
            variable,
            descending,
        } => {
            format!("sort {variable}{}", if *descending { " desc" } else { "" })
        }
        Command::EstimateList => "estimate list".to_string(),
        Command::EstimateCompare { ids } => list("estimate compare", ids),
        Command::Note { text } => format!("note \"{text}\""),
        Command::Notes => "notes".to_string(),
        Command::ExportTable { path, format, ids } => {
            let mut s = format!("export table \"{path}\"");
            for id in ids {
                s.push(' ');
                s.push_str(id);
            }
            let _ = format;
            s
        }
        Command::ExportData { path } => format!("export data \"{path}\""),
        Command::Help { command } => match command {
            Some(c) => format!("help {c}"),
            None => "help".to_string(),
        },
    }
}

fn list(head: &str, variables: &[String]) -> String {
    let mut s = head.to_string();
    for v in variables {
        s.push(' ');
        s.push_str(v);
    }
    s
}

fn render_model(spec: &ModelSpec) -> String {
    let mut s = spec.outcome.clone();
    for p in &spec.predictors {
        s.push(' ');
        s.push_str(p);
    }
    if let Some(w) = &spec.weights {
        s.push_str(&format!(" [aw={w}]"));
    }
    s.push_str(&render_vcov_suffix(&spec.vcov));
    if spec.no_constant {
        s.push_str(", noconstant");
    }
    s
}

fn render_glm(spec: &GlmSpec) -> String {
    let mut s = spec.outcome.clone();
    for p in &spec.predictors {
        s.push(' ');
        s.push_str(p);
    }
    if spec.robust {
        s.push_str(", robust");
    }
    s
}

fn render_vcov_suffix(vcov: &VcovOption) -> String {
    match vcov {
        VcovOption::Classical => String::new(),
        VcovOption::Robust => ", robust".to_string(),
        VcovOption::Hc2 => ", hc2".to_string(),
        VcovOption::Hc3 => ", hc3".to_string(),
        VcovOption::Cluster { variable } => format!(", cluster({variable})"),
        VcovOption::Hac { lags } => format!(", vce(hac {lags})"),
    }
}

fn render_cond(c: &Cond) -> String {
    let op = match c.op {
        CondOp::Eq => "==",
        CondOp::Ne => "!=",
        CondOp::Gt => ">",
        CondOp::Ge => ">=",
        CondOp::Lt => "<",
        CondOp::Le => "<=",
    };
    format!("{} {} {}", c.variable, op, fmt_num(c.value))
}

fn render_expr(e: &Expr) -> String {
    match e {
        Expr::Number(v) => fmt_num(*v),
        Expr::Var(v) => v.clone(),
        Expr::BinOp { op, left, right } => {
            format!(
                "{} {} {}",
                render_expr(left),
                binop_str(*op),
                render_expr(right)
            )
        }
        Expr::Neg(inner) => format!("-{}", render_expr(inner)),
        Expr::Call { function, arg } => format!("{}({})", fn_str(*function), render_expr(arg)),
    }
}

fn binop_str(op: BinOpKind) -> &'static str {
    match op {
        BinOpKind::Add => "+",
        BinOpKind::Sub => "-",
        BinOpKind::Mul => "*",
        BinOpKind::Div => "/",
        BinOpKind::Pow => "^",
    }
}

fn fn_str(f: FnKind) -> &'static str {
    match f {
        FnKind::Ln => "ln",
        FnKind::Log => "log",
        FnKind::Exp => "exp",
        FnKind::Sqrt => "sqrt",
        FnKind::Abs => "abs",
        FnKind::Round => "round",
        FnKind::Rank => "rank",
        FnKind::Standardize => "standardize",
    }
}

fn fmt_num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn render_matches_input_for_canonical_examples() {
        let cases = [
            "regress wage education experience female, robust",
            "regress wage education experience female",
            "regress y x1 x2, cluster(firm)",
            "xtreg wage education, fe, entity(firm)",
            "ivregress 2sls wage (education = distance), robust",
            "did wage, treat(treated), time(post), cluster(firm)",
            "logit employed education experience, robust",
            "summarize wage education",
            "correlate wage education, spearman",
            "ttest wage == 20",
            "ttest wage, by(female)",
            "generate logwage = ln(wage)",
            "rename wage hourly_wage",
            "drop if wage < 0",
            "keep wage education",
        ];
        for case in cases {
            let cmd = parse(case).unwrap_or_else(|e| panic!("parse failed for '{case}': {e}"));
            let rendered = render(&cmd);
            assert_eq!(rendered, case, "render mismatch for '{case}'");
            // Round-trip: rendering must re-parse to the same AST.
            let reparsed = parse(&rendered).unwrap();
            assert_eq!(reparsed, cmd, "round-trip mismatch for '{case}'");
        }
    }

    #[test]
    fn gui_form_spec_renders_to_the_equivalent_command() {
        // This is the exact equivalence required by the specification:
        // GUI: Outcome=wage Predictors=education,experience,female SE=Robust
        let gui_spec = Command::Regress {
            spec: ModelSpec {
                outcome: "wage".into(),
                predictors: vec!["education".into(), "experience".into(), "female".into()],
                vcov: VcovOption::Robust,
                no_constant: false,
                weights: None,
            },
        };
        let rendered = render(&gui_spec);
        assert_eq!(rendered, "regress wage education experience female, robust");
        let from_command = parse("regress wage education experience female, robust").unwrap();
        assert_eq!(gui_spec, from_command);
    }
}
