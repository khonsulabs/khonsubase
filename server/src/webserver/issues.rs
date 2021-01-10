use rocket::{request::Form, response::Redirect};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use database::schema::issues::Issue;

use super::{auth::SessionId, localization::UserLanguage, FullPathAndQuery, RequestData};

#[get("/issues")]
pub async fn list_issues() -> Template {
    todo!()
}

#[derive(Serialize, Deserialize)]
struct ViewIssueContext {
    request: RequestData,
    issue: Issue,
}

#[get("/issues/<issue_id>")]
pub async fn view_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
) -> Template {
    let request = RequestData::new(language, path, session).await;
    match Issue::load(issue_id).await {
        Ok(issue) => Template::render("view_issue", ViewIssueContext { request, issue }),
        Err(_) => todo!(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NewIssueContext {
    pub request: RequestData,
    pub error_message: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
}

#[get("/issues/new?<summary>&<description>")]
pub async fn new_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    summary: Option<String>,
    description: Option<String>,
) -> Result<Template, Redirect> {
    let request = RequestData::new(language, path, session).await;
    if request.logged_in() {
        Ok(Template::render(
            "new_issue",
            NewIssueContext {
                request,
                error_message: None,
                summary,
                description,
            },
        ))
    } else {
        Err(Redirect::to("/signin?origin=/issues/new"))
    }
}

#[derive(FromForm, Clone, Debug)]
pub struct NewIssueForm {
    summary: String,
    description: Option<String>,
}

#[post("/issues/new", data = "<new_issue>")]
pub async fn create_issue(
    new_issue: Form<NewIssueForm>,
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
) -> Result<Template, Redirect> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        let mut issue = Issue::new(
            session.account.id,
            new_issue.summary.clone(),
            new_issue.description.clone(),
            None,
        );
        match issue.save(database::pool()).await {
            Ok(_) => Err(Redirect::to(format!("/issues/{}", issue.id))),
            Err(sql_error) => {
                error!("error while saving issue: {:?}", sql_error);

                Ok(Template::render(
                    "new_issue",
                    NewIssueContext {
                        request,
                        error_message: Some(String::from("issue-error-internal-error")),
                        summary: Some(new_issue.summary.clone()),
                        description: new_issue.description.clone(),
                    },
                ))
            }
        }
    } else {
        Err(Redirect::to("/signin?origin=/issues/new"))
    }
}
