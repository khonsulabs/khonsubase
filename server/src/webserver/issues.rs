use rocket::{http::Status, request::Form, response::Redirect};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use database::{
    schema::issues::{
        Issue, IssueQueryBuilder, IssueQueryResults, IssueRevision, IssueRevisionChange,
        IssueRevisionView, IssueView,
    },
    sqlx,
};

use crate::webserver::{
    auth::SessionId, localization::UserLanguage, Failure, FullPathAndQuery, RequestData, ResultExt,
};
use database::{schema::issues::Project, sqlx::types::chrono::Utc};

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
) -> Result<Template, Status> {
    let request = RequestData::new(language, path, session).await;
    let response = IssueQueryBuilder::new()
        .open()
        .query(database::pool())
        .await
        .map_sql_to_http()?;

    Ok(Template::render(
        "list_issues",
        ListIssuesContext { request, response },
    ))
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

#[get("/issue/<issue_id>")]
pub async fn view_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    render_issue(request, issue_id).await.map_to_failure()
}

#[derive(Debug, Serialize, Deserialize)]
struct EditIssueContext {
    request: RequestData,
    error_message: Option<String>,
    issue_id: Option<i64>,
    current_revision_id: Option<i64>,
    summary: Option<String>,
    description: Option<String>,
    comment: Option<String>,
    completed: bool,
    project_id: Option<i64>,

    projects: Vec<Project>,
}

#[get("/issues/new?<summary>&<description>")]
pub async fn new_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    summary: Option<String>,
    description: Option<String>,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    let projects = Project::list().await?;
    if request.logged_in() {
        Ok(Template::render(
            "edit_issue",
            EditIssueContext {
                request,
                summary,
                description,
                issue_id: None,
                current_revision_id: None,
                error_message: None,
                comment: None,
                completed: false,
                project_id: None,
                projects,
            },
        ))
    } else {
        Err(Failure::redirect_to_signin(Some(&request.current_path)))
    }
}

#[get("/issue/<issue_id>/edit")]
pub async fn edit_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if request.logged_in() {
        let issue = Issue::load(issue_id).await.map_to_failure()?;
        let projects = Project::list().await?;
        Ok(Template::render(
            "edit_issue",
            EditIssueContext {
                request,
                issue_id: Some(issue_id),
                current_revision_id: issue.current_revision_id,
                error_message: None,
                summary: Some(issue.summary),
                description: issue.description,
                comment: None,
                completed: issue.completed_at.is_some(),
                project_id: issue.project_id,
                projects,
            },
        ))
    } else {
        Err(Failure::Redirect(Redirect::to(
            "/signin?origin=/issues/new",
        )))
    }
}

#[derive(FromForm, Clone, Debug)]
pub struct EditIssueForm {
    issue_id: Option<i64>,
    current_revision_id: Option<i64>,
    summary: String,
    description: Option<String>,
    comment: Option<String>,
    completed: bool,
    project_id: Option<i64>,
}

async fn update_issue(issue_form: &Form<EditIssueForm>, author_id: i64) -> sqlx::Result<i64> {
    let mut tx = database::pool().begin().await?;
    let issue_id = if let Some(issue_id) = issue_form.issue_id {
        let mut issue = Issue::load_for_update(issue_id, &mut tx).await?;
        if issue.current_revision_id != issue_form.current_revision_id {
            todo!("Return a proper error and show it to the user.")
        }

        if issue_form.comment.is_some()
            || issue.summary != issue_form.summary
            || issue.description != issue_form.description
            || issue.completed_at.is_some() != issue_form.completed
        {
            let issue_revision =
                IssueRevision::create(issue.id, author_id, issue_form.comment.clone(), &mut tx)
                    .await?;
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

            if issue_form.completed != issue.completed_at.is_some() {
                let new_value = if issue_form.completed {
                    Some(Utc::now())
                } else {
                    None
                };
                IssueRevisionChange::create(
                    issue_revision.id,
                    "completed_at",
                    issue.completed_at,
                    new_value,
                    &mut tx,
                )
                .await?;
                issue.completed_at = new_value;
            }

            if issue_form.project_id != issue.project_id {
                IssueRevisionChange::create(
                    issue_revision.id,
                    "project_id",
                    issue.project_id,
                    issue_form.project_id,
                    &mut tx,
                )
                .await?;
                issue.project_id = issue_form.project_id;
            }

            issue.current_revision_id = Some(issue_revision.id);
        }
        issue.save(&mut tx).await?;

        issue_id
    } else {
        let mut issue = Issue::new(
            author_id,
            issue_form.summary.clone(),
            issue_form.description.clone(),
            None,
            issue_form.project_id,
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
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        let result = update_issue(&issue_form, session.account.id).await;

        match result {
            Ok(issue_id) => Err(Failure::redirect(format!("/issue/{}", issue_id))),
            Err(sql_error) => {
                error!("error while saving issue: {:?}", sql_error);
                let projects = Project::list().await?;

                Ok(Template::render(
                    "edit_issue",
                    EditIssueContext {
                        request,
                        error_message: Some(String::from("internal-error-saving")),
                        issue_id: issue_form.issue_id,
                        current_revision_id: issue_form.current_revision_id,
                        summary: Some(issue_form.summary.clone()),
                        description: issue_form.description.clone(),
                        comment: issue_form.comment.clone(),
                        completed: issue_form.completed,
                        project_id: issue_form.project_id,
                        projects,
                    },
                ))
            }
        }
    } else {
        Err(Failure::redirect("/signin?origin=/issues/new"))
    }
}
