use std::str::FromStr;

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};

#[derive(pest_derive::Parser)]
#[grammar = "kbql/kbql.pest"]
struct KBQLParser;

#[derive(PartialEq, Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Term {
    Literal(Literal),
    Lookup(Vec<String>),
    Expression(Box<Expression>),
    Not(Box<Term>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Expression {
    Term(Term),
    Is(Box<Expression>, Box<Expression>),
    IsNot(Box<Expression>, Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("error parsing: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Query {
    pub expression: Expression,
    pub returning: Vec<Term>,
}

impl Query {
    pub fn parse(source: &str) -> Result<Self, Error> {
        let mut results = KBQLParser::parse(Rule::query, source)?;
        let query_tokens = results.next().unwrap();

        let expression = Self::parse_expression(query_tokens);

        let returning = if let Some(returning) = results.next() {
            returning.into_inner().map(Self::parse_term).collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            expression,
            returning,
        })
    }

    fn parse_expression(tokens: Pair<Rule>) -> Expression {
        assert_eq!(tokens.as_rule(), Rule::expression);
        let mut expression_tokens = tokens.into_inner();
        let logical_expression = expression_tokens.next().unwrap().into_inner();
        assert!(expression_tokens.next().is_none());

        Self::parse_logical_expression(logical_expression)
    }

    fn parse_logical_expression(mut logical_expression: Pairs<Rule>) -> Expression {
        let mut lhs = Self::parse_comparison(logical_expression.next().unwrap());
        while let Some(operator) = logical_expression.next() {
            let rhs = Self::parse_comparison(logical_expression.next().unwrap());
            match operator.as_rule() {
                Rule::and => {
                    lhs = Expression::And(Box::new(lhs), Box::new(rhs));
                }
                Rule::or => {
                    lhs = Expression::Or(Box::new(lhs), Box::new(rhs));
                }
                _ => unreachable!("Unexpected rule encountered within the expression grammar"),
            }
        }

        lhs
    }

    fn parse_comparison(comparison: Pair<Rule>) -> Expression {
        assert_eq!(comparison.as_rule(), Rule::comparison_expression);
        let mut comparison = comparison.into_inner();

        let mut lhs = Expression::Term(Self::parse_term(comparison.next().unwrap()));
        while let Some(operator) = comparison.next() {
            let rhs = Expression::Term(Self::parse_term(comparison.next().unwrap()));
            match operator.as_rule() {
                Rule::is => {
                    lhs = Expression::Is(Box::new(lhs), Box::new(rhs));
                }
                Rule::is_not => {
                    lhs = Expression::IsNot(Box::new(lhs), Box::new(rhs));
                }
                _ => unreachable!("Unexpected rule encountered within the expression grammar"),
            }
        }

        lhs
    }

    fn parse_term(tokens: Pair<Rule>) -> Term {
        assert_eq!(tokens.as_rule(), Rule::term);

        let mut tokens = tokens.into_inner();
        let term = tokens.next().unwrap();
        assert!(tokens.next().is_none());
        match term.as_rule() {
            Rule::number => {
                if term.as_str().contains('.') {
                    Term::Literal(Literal::Float(f64::from_str(term.as_str()).unwrap()))
                } else {
                    Term::Literal(Literal::Integer(i64::from_str(term.as_str()).unwrap()))
                }
            }
            Rule::lookup => Term::Lookup(
                term.into_inner()
                    .map(|identifier| identifier.as_str().to_string())
                    .collect(),
            ),
            Rule::quoted_string => {
                let mut quoted_string_tokens = term.into_inner();
                let string = quoted_string_tokens.next().unwrap();
                println!("Inner string: {}", string.as_str());
                todo!()
            }
            Rule::not_term => {
                let mut tokens = term.into_inner();
                let inner_term = tokens.next().unwrap();
                Term::Not(Box::new(Self::parse_term(inner_term)))
            }
            Rule::expression => Term::Expression(Box::new(Self::parse_expression(term))),
            rule => unreachable!(
                "Unexpected rule encountered within the expression grammar: {:?}",
                rule
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use pest::{consumes_to, parses_to};

    use super::*;

    #[test]
    fn simple_query() {
        assert_eq!(
            Query::parse("id is (5 and 1)").unwrap(),
            Query {
                expression: Expression::Is(
                    Box::new(Expression::Term(Term::Lookup(vec!["id".to_owned()]))),
                    Box::new(Expression::Term(Term::Expression(Box::new(
                        Expression::And(
                            Box::new(Expression::Term(Term::Literal(Literal::Integer(5)))),
                            Box::new(Expression::Term(Term::Literal(Literal::Integer(1))))
                        )
                    )))),
                ),
                returning: Default::default()
            }
        )
    }

    #[test]
    fn terms() {
        assert_eq!(
            Query::parse("id").unwrap(),
            Query {
                expression: Expression::Term(Term::Lookup(vec!["id".to_owned()])),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("5").unwrap(),
            Query {
                expression: Expression::Term(Term::Literal(Literal::Integer(5))),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("-5.0").unwrap(),
            Query {
                expression: Expression::Term(Term::Literal(Literal::Float(-5.0))),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("not id").unwrap(),
            Query {
                expression: Expression::Term(Term::Not(Box::new(Term::Lookup(vec![
                    "id".to_owned()
                ])))),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("(id)").unwrap(),
            Query {
                expression: Expression::Term(Term::Expression(Box::new(Expression::Term(
                    Term::Lookup(vec!["id".to_owned()])
                )))),
                returning: Default::default()
            }
        );
    }

    #[test]
    fn expressions() {
        assert_eq!(
            Query::parse("id is 5").unwrap(),
            Query {
                expression: Expression::Is(
                    Box::new(Expression::Term(Term::Lookup(vec!["id".to_owned()]))),
                    Box::new(Expression::Term(Term::Literal(Literal::Integer(5))))
                ),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("id is not 5").unwrap(),
            Query {
                expression: Expression::IsNot(
                    Box::new(Expression::Term(Term::Lookup(vec!["id".to_owned()]))),
                    Box::new(Expression::Term(Term::Literal(Literal::Integer(5))))
                ),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("id and 5").unwrap(),
            Query {
                expression: Expression::And(
                    Box::new(Expression::Term(Term::Lookup(vec!["id".to_owned()]))),
                    Box::new(Expression::Term(Term::Literal(Literal::Integer(5))))
                ),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("id or 5").unwrap(),
            Query {
                expression: Expression::Or(
                    Box::new(Expression::Term(Term::Lookup(vec!["id".to_owned()]))),
                    Box::new(Expression::Term(Term::Literal(Literal::Integer(5))))
                ),
                returning: Default::default()
            }
        );

        assert_eq!(
            Query::parse("a is 1 and b is 2 or c is 3").unwrap(),
            Query {
                expression: Expression::Or(
                    Box::new(Expression::And(
                        Box::new(Expression::Is(
                            Box::new(Expression::Term(Term::Lookup(vec!["a".to_owned()]))),
                            Box::new(Expression::Term(Term::Literal(Literal::Integer(1))))
                        )),
                        Box::new(Expression::Is(
                            Box::new(Expression::Term(Term::Lookup(vec!["b".to_owned()]))),
                            Box::new(Expression::Term(Term::Literal(Literal::Integer(2))))
                        ))
                    )),
                    Box::new(Expression::Is(
                        Box::new(Expression::Term(Term::Lookup(vec!["c".to_owned()]))),
                        Box::new(Expression::Term(Term::Literal(Literal::Integer(3))))
                    ))
                ),
                returning: Default::default()
            }
        );
    }

    #[test]
    fn returning() {
        assert_eq!(
            Query::parse("completed returning id").unwrap(),
            Query {
                expression: Expression::Term(Term::Lookup(vec!["completed".to_owned()])),
                returning: vec![Term::Lookup(vec!["id".to_owned()])],
            }
        );

        assert_eq!(
            Query::parse("completed returning id, project").unwrap(),
            Query {
                expression: Expression::Term(Term::Lookup(vec!["completed".to_owned()])),
                returning: vec![
                    Term::Lookup(vec!["id".to_owned()]),
                    Term::Lookup(vec!["project".to_owned()]),
                ]
            }
        );
    }

    #[test]
    fn string() {
        parses_to! {
            parser: KBQLParser,
            input: r#""abc""#,
            rule: Rule::quoted_string,
            tokens: [
                quoted_string(0, 5, [
                    string(1,4),
                ])
            ]
        }
    }

    #[test]
    fn string_with_escapes() {
        parses_to! {
            parser: KBQLParser,
            input: r#""abc\"def\"ghi""#,
            rule: Rule::quoted_string,
            tokens: [
                quoted_string(0, 15, [
                    string(1,14),
                ])
            ]
        }
    }

    #[test]
    fn raw_terms() {
        parses_to! {
            parser: KBQLParser,
            input: r#""abc""#,
            rule: Rule::term,
            tokens: [
                term(0, 5, [
                    quoted_string(0, 5, [
                        string(1,4),
                    ])
                ])
            ]
        };

        parses_to! {
            parser: KBQLParser,
            input: r#"1234"#,
            rule: Rule::term,
            tokens: [
                term(0, 4, [
                    number(0, 4)
                ])
            ]
        };

        parses_to! {
            parser: KBQLParser,
            input: r#"1234.5678"#,
            rule: Rule::term,
            tokens: [
                term(0, 9, [
                    number(0, 9)
                ])
            ]
        };

        parses_to! {
            parser: KBQLParser,
            input: r#"foo"#,
            rule: Rule::term,
            tokens: [
                term(0, 3, [
                    lookup(0, 3, [
                        identifier(0, 3)
                    ])
                ])
            ]
        };
    }

    #[test]
    fn lookup() {
        parses_to! {
            parser: KBQLParser,
            input: r#"foo.bar.baz"#,
            rule: Rule::term,
            tokens: [
                term(0, 11, [
                    lookup(0, 11, [
                        identifier(0,3),
                        identifier(4,7),
                        identifier(8,11),
                    ])
                ])
            ]
        }
    }

    #[test]
    fn comparison() {
        parses_to! {
            parser: KBQLParser,
            input: r#"foo is "bar""#,
            rule: Rule::comparison_expression,
            tokens: [
                comparison_expression(0, 12, [
                    term(0,3, [lookup(0,3, [identifier(0,3)])]),
                    is(4, 6),
                    term(7, 12, [quoted_string(7, 12, [
                        string(8,11)
                    ])])
                ])
            ]
        }
    }

    #[test]
    fn logic() {
        parses_to! {
            parser: KBQLParser,
            input: r#"foo and bar"#,
            rule: Rule::logical_expression,
            tokens: [
                logical_expression(0, 11, [
                    comparison_expression(0,3, [
                        term(0,3, [lookup(0,3, [identifier(0,3)])]),
                    ]),
                    and(4, 7),
                    comparison_expression(8,11, [
                        term(8,11, [lookup(8,11, [identifier(8,11)])]),
                    ]),
                ])
            ]
        }
    }

    #[test]
    fn chained_expressions() {
        parses_to! {
            parser: KBQLParser,
            input: r#"foo is "bar" and foo is not "baz""#,
            rule: Rule::expression,
            tokens: [
                expression(0, 33, [
                    logical_expression(0, 33, [
                        comparison_expression(0, 12, [
                            term(0,3, [lookup(0,3, [identifier(0,3)])]),
                            is(4, 6),
                            term(7, 12, [quoted_string(7, 12, [
                                string(8,11)
                            ])])
                        ]),
                        and(13,16),
                        comparison_expression(17, 33, [
                            term(17, 20, [
                                lookup(17, 20, [
                                    identifier(17,20)
                                ])
                            ]),
                            is_not(21, 27),
                            term(28, 33, [
                                quoted_string(28, 33, [
                                    string(29,32)
                                ])
                            ])
                        ]),
                    ])
                ])
            ]
        }
    }
}
