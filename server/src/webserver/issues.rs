use rocket::{request::Form, response::Redirect};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use database::schema::issues::{
    Issue, IssueOrderingField, IssueQueryBuilder, IssueQueryResults, IssueRevision,
    IssueRevisionChange, IssueRevisionView, IssueView,
};
use database::sqlx;

use super::{auth::SessionId, localization::UserLanguage, FullPathAndQuery, RequestData};
use database::sqlx::types::chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
struct ListIssuesContext {
    request: RequestData,
    response: IssueQueryResults,
}

#[get("/issues")]
pub async fn list_issues(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
) -> Template {
    let request = RequestData::new(language, path, session).await;
    match IssueQueryBuilder::new()
        .open()
        .query(database::pool())
        .await
    {
        Ok(response) => Template::render("list_issues", ListIssuesContext { request, response }),
        Err(sql_error) => todo!("Error executing query: {:?}", sql_error),
    }
}

#[derive(Serialize, Deserialize)]
struct IssueTimeline {
    entries: Vec<IssueRevisionView>,
}

#[derive(Serialize, Deserialize)]
struct ViewIssueContext {
    request: RequestData,
    issue: IssueView,
    timeline: IssueTimeline,
}

async fn render_issue(request: RequestData, issue_id: i64) -> sqlx::Result<Template> {
    let issue = IssueView::load(issue_id).await?;
    let timeline = IssueTimeline {
        entries: IssueRevisionView::list_for(issue_id).await?,
    };
    Ok(Template::render(
        "view_issue",
        ViewIssueContext {
            request,
            issue,
            timeline,
        },
    ))
}

#[get("/issues/<issue_id>")]
pub async fn view_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
) -> Template {
    let request = RequestData::new(language, path, session).await;
    match render_issue(request, issue_id).await {
        Ok(template) => template,
        Err(err) => todo!("db error: {:?}", err),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EditIssueContext {
    pub request: RequestData,
    pub error_message: Option<String>,
    pub issue_id: Option<i64>,
    pub current_revision_id: Option<i64>,
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
            "edit_issue",
            EditIssueContext {
                request,
                issue_id: None,
                current_revision_id: None,
                error_message: None,
                summary,
                description,
            },
        ))
    } else {
        Err(Redirect::to("/signin?origin=/issues/new"))
    }
}

#[get("/issues/edit/<issue_id>")]
pub async fn edit_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
) -> Result<Template, Redirect> {
    let request = RequestData::new(language, path, session).await;
    if request.logged_in() {
        match Issue::load(issue_id).await {
            Ok(issue) => Ok(Template::render(
                "edit_issue",
                EditIssueContext {
                    request,
                    issue_id: Some(issue_id),
                    current_revision_id: issue.current_revision_id,
                    error_message: None,
                    summary: Some(issue.summary),
                    description: issue.description,
                },
            )),
            Err(sqlx::Error::RowNotFound) => todo!(),
            Err(_) => todo!(),
        }
    } else {
        Err(Redirect::to("/signin?origin=/issues/new"))
    }
}

#[derive(FromForm, Clone, Debug)]
pub struct EditIssueForm {
    issue_id: Option<i64>,
    current_revision_id: Option<i64>,
    summary: String,
    description: Option<String>,
}

async fn update_issue(issue_form: &Form<EditIssueForm>, author_id: i64) -> sqlx::Result<i64> {
    let mut tx = database::pool().begin().await?;
    let issue_id = if let Some(issue_id) = issue_form.issue_id {
        let mut issue = Issue::load_for_update(issue_id, &mut tx).await?;
        if issue.current_revision_id != issue_form.current_revision_id {
            todo!("Return a proper error and show it to the user.")
        }

        let issue_revision = IssueRevision::create(issue.id, author_id, &mut tx).await?;
        if issue.summary != issue_form.summary {
            IssueRevisionChange::create(
                issue_revision.id,
                "summary",
                Some(issue.summary),
                Some(issue_form.summary.clone()),
                &mut tx,
            )
            .await?;
            issue.summary = issue_form.summary.clone();
        }

        if issue.description != issue_form.description {
            IssueRevisionChange::create(
                issue_revision.id,
                "description",
                issue.description.clone(),
                issue_form.description.clone(),
                &mut tx,
            )
            .await?;
            issue.description = issue_form.description.clone();
        }
        issue.current_revision_id = Some(issue_revision.id);
        issue.save(&mut tx).await?;

        issue_id
    } else {
        let mut issue = Issue::new(
            author_id,
            issue_form.summary.clone(),
            issue_form.description.clone(),
            None,
        );
        issue.save(&mut tx).await?;
        issue.id
    };

    tx.commit().await?;

    Ok(issue_id)
}

#[post("/issues/save", data = "<issue_form>")]
pub async fn save_issue(
    issue_form: Form<EditIssueForm>,
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
) -> Result<Template, Redirect> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        let result = update_issue(&issue_form, session.account.id).await;

        match result {
            Ok(issue_id) => Err(Redirect::to(format!("/issues/{}", issue_id))),
            Err(sql_error) => {
                error!("error while saving issue: {:?}", sql_error);

                Ok(Template::render(
                    "edit_issue",
                    EditIssueContext {
                        request,
                        error_message: Some(String::from("issue-error-internal-error")),
                        issue_id: issue_form.issue_id,
                        current_revision_id: issue_form.current_revision_id,
                        summary: Some(issue_form.summary.clone()),
                        description: issue_form.description.clone(),
                    },
                ))
            }
        }
    } else {
        Err(Redirect::to("/signin?origin=/issues/new"))
    }
}
