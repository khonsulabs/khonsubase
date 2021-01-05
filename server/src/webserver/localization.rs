use fluent_templates::{
    fluent_bundle::{types::FluentNumber, FluentValue},
    Loader,
};
use include_dir::include_dir;
use rocket::{
    request::{FromRequest, Outcome},
    Request,
};
use rocket_contrib::templates::tera;
use std::{collections::HashMap, str::FromStr};
use tera::Value;
use unic_langid::LanguageIdentifier;

use crate::configuration::{ConfigurationManager, SitePrimaryLocale};

// TODO we should refactor this to using a solution that performs the pick_best_language at the time of resolving individual keys
fluent_templates::static_loader! {
    pub static LOCALES = {
        locales: "../strings",
        fallback_language: "en-US",
    };
}

// TODO This is only here because of https://github.com/XAMPPRocky/fluent-templates/issues/2
#[allow(dead_code)]
fn unused() {
    include_dir!("../strings");
}

pub struct Localize;

impl tera::Function for Localize {
    fn call(
        &self,
        args: &std::collections::HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        let key = args.get("key").expect("key parameter required");
        let key = key
            .as_str()
            .ok_or_else(|| tera::Error::msg("key must be a string"))?;

        let lang = args.get("language").expect("language parameter required");
        let lang = unic_langid::LanguageIdentifier::from_str(
            lang.as_str().expect("language not a string."),
        )
        .expect("language code not found");

        let mut fluent_args = HashMap::new();
        for (name, value) in args {
            if name == "language" || name == "key" {
                continue;
            }

            let value = if value.is_number() {
                FluentValue::Number(FluentNumber::new(
                    value.as_f64().unwrap(),
                    Default::default(),
                ))
            } else {
                FluentValue::String(std::borrow::Cow::Owned(value.as_str().unwrap().to_owned()))
            };

            fluent_args.insert(name.clone(), value);
        }

        Ok(Value::String(LOCALES.lookup_with_args(
            &lang,
            &key,
            &fluent_args,
        )))
    }
}

pub struct LanguageCode;

impl tera::Filter for LanguageCode {
    fn filter(
        &self,
        language_identifier: &Value,
        _: &HashMap<String, Value>,
    ) -> tera::Result<Value> {
        let lang = unic_langid::LanguageIdentifier::from_str(
            language_identifier
                .as_str()
                .expect("language not a string."),
        )
        .expect("language code not found");

        Ok(Value::from(lang.language.to_string()))
    }
}

#[derive(Debug)]
pub struct UserLanguage(pub String);

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for UserLanguage {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let default_locale = ConfigurationManager::shared()
            .get::<SitePrimaryLocale>()
            .unwrap();
        let best_language = pick_best_language(
            &default_locale,
            request.headers().get_one("Accept-Language"),
            &LOCALES.locales().cloned().collect::<Vec<_>>(),
        );

        Outcome::Success(UserLanguage(best_language))
    }
}

#[derive(Debug, PartialEq)]
struct AcceptableLanguage {
    code: String,
    weight: f32,
}

fn parse_accept_language_header(header: &str) -> Vec<AcceptableLanguage> {
    let mut languages = Vec::new();
    for possible_language in header.split(',') {
        let possible_language = possible_language.trim();
        let mut parts = possible_language.split(";q=");
        if let Some(code) = parts.next() {
            let qfactor = if let Some(qfactor) = parts.next() {
                qfactor.parse::<f32>().unwrap_or_default()
            } else {
                1f32
            };
            languages.push(AcceptableLanguage {
                code: code.to_string(),
                weight: qfactor,
            })
        }
    }
    languages
}

fn pick_best_language(
    default_code: &str,
    accept_language_header: Option<&str>,
    available_locales: &[LanguageIdentifier],
) -> String {
    let mut best_language = None;
    let mut best_language_weight = 0f32;

    if let Some(accept_language_header) = accept_language_header {
        for language in parse_accept_language_header(accept_language_header) {
            if language.weight > best_language_weight {
                let language_identifier = match LanguageIdentifier::from_str(&language.code) {
                    Ok(identifier) => identifier,
                    Err(_) => continue,
                };
                for supported_locale in available_locales.iter() {
                    if language_identifier.matches(supported_locale, true, false) {
                        best_language_weight = language.weight;
                        best_language = Some(language.code);
                        break;
                    }
                }
            }
        }
    }

    best_language.unwrap_or_else(|| default_code.to_string())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{parse_accept_language_header, pick_best_language, AcceptableLanguage};
    use fluent_templates::LanguageIdentifier;

    #[test]
    fn parse_accept_language_header_tests() {
        assert_eq!(
            parse_accept_language_header("en"),
            vec![AcceptableLanguage {
                code: "en".to_owned(),
                weight: 1.,
            }]
        );

        assert_eq!(
            parse_accept_language_header("en-US"),
            vec![AcceptableLanguage {
                code: "en-US".to_owned(),
                weight: 1.,
            }]
        );

        assert_eq!(
            parse_accept_language_header("en-US;q=0.9"),
            vec![AcceptableLanguage {
                code: "en-US".to_owned(),
                weight: 0.9,
            }]
        );

        assert_eq!(
            parse_accept_language_header("en-US, en;q=0.9,*;q=0.5"),
            vec![
                AcceptableLanguage {
                    code: "en-US".to_owned(),
                    weight: 1.,
                },
                AcceptableLanguage {
                    code: "en".to_owned(),
                    weight: 0.9,
                },
                AcceptableLanguage {
                    code: "*".to_owned(),
                    weight: 0.5,
                }
            ]
        );
    }

    #[test]
    fn pick_best_language_no_header() {
        assert_eq!(pick_best_language("en-US", None, &[]), "en-US");
    }

    #[test]
    fn pick_best_language_no_matching_languages() {
        assert_eq!(
            pick_best_language("en-US", Some("es-MX,es-AR;q=0.9,es;q=0.5"), &[]),
            "en-US"
        );
    }

    #[test]
    fn pick_best_language_best_match_orderless() {
        let es_mx = LanguageIdentifier::from_str("es-MX").unwrap();
        let es = LanguageIdentifier::from_str("es").unwrap();

        assert_eq!(
            pick_best_language(
                "af",
                Some("es-MX,es-AR;q=0.9,es;q=0.5"),
                &[es_mx.clone(), es.clone()]
            ),
            "es-MX"
        );

        assert_eq!(
            pick_best_language("af", Some("es-MX,es-AR;q=0.9,es;q=0.5"), &[es, es_mx]),
            "es-MX"
        );
    }

    #[test]
    fn pick_best_language_second_best_match() {
        let es_ar = LanguageIdentifier::from_str("es-AR").unwrap();
        let es = LanguageIdentifier::from_str("es").unwrap();

        assert_eq!(
            pick_best_language("af", Some("es-MX,es-AR;q=0.9,es;q=0.5"), &[es, es_ar]),
            "es-AR"
        );
    }

    #[test]
    fn pick_best_language_worst_match() {
        let es = LanguageIdentifier::from_str("es").unwrap();
        let en_us = LanguageIdentifier::from_str("en-US").unwrap();
        let en = LanguageIdentifier::from_str("en").unwrap();

        assert_eq!(
            pick_best_language("af", Some("es-MX,es-AR;q=0.9,es;q=0.5"), &[es, en, en_us]),
            "es"
        );
    }
}
