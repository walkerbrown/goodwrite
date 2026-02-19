use goodwrite_core::{CheckContext, Diagnostic, Rule, RuleInput, Severity};

use crate::patterns::keyword_position;

pub struct ClauseOrderRule;

impl Rule for ClauseOrderRule {
    fn id(&self) -> &str {
        "ears/clause-order"
    }

    fn name(&self) -> &str {
        "EARS clause temporal order"
    }

    fn profiles(&self) -> &[&str] {
        &["ears"]
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, input: &RuleInput, _ctx: &CheckContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for sentence in &input.sentences {
            let where_pos = keyword_position(&sentence.text, "where");
            let while_pos = keyword_position(&sentence.text, "while");
            let when_pos = keyword_position(&sentence.text, "when");
            let if_pos = keyword_position(&sentence.text, "if");
            let then_pos = keyword_position(&sentence.text, "then");
            let shall_pos = keyword_position(&sentence.text, "shall");

            let mut last = 0usize;
            let mut valid = true;

            for pos in [where_pos, while_pos, when_pos, if_pos, then_pos, shall_pos]
                .into_iter()
                .flatten()
            {
                if pos < last {
                    valid = false;
                    break;
                }
                last = pos;
            }

            if if_pos.is_some() && then_pos.is_none() {
                valid = false;
            }

            if !valid {
                diagnostics.push(
                    Diagnostic::new(
                        self.id(),
                        self.default_severity(),
                        "EARS clauses are out of order",
                        sentence.range,
                    )
                    .with_help(
                        "use order: Where -> While -> When -> If/then -> the <system> shall <response>",
                    ),
                );
            }
        }

        diagnostics
    }
}
