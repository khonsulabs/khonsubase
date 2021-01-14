use rocket::{http::Status, request::Form, response::Redirect};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use database::{
    schema::issues::{
        Issue, IssueQueryBuilder, IssueQueryResults, IssueRevision, IssueRevisionChange,
        IssueRevisionView, IssueView, Project,
    },
    sqlx,
    sqlx::types::chrono::Utc,
    DatabaseError, SqlxResultExt,
};

use crate::webserver::{
    auth::SessionId, localization::UserLanguage, Failure, FullPathAndQuery, RequestData, ResultExt,
};

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
        .owned_by(None)
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
    parents: Vec<Issue>,
    timeline: IssueTimeline,
    response: IssueQueryResults,
}

async fn render_issue(request: RequestData, issue_id: i64) -> sqlx::Result<Template> {
    let issue = IssueView::load(issue_id).await?;
    let parents = Issue::all_parents(issue.id).await?;
    let timeline = IssueTimeline {
        entries: IssueRevisionView::list_for(issue_id).await?,
    };
    let response = IssueQueryBuilder::new()
        .owned_by(Some(issue.id))
        .query(database::pool())
        .await?;
    Ok(Template::render(
        "view_issue",
        ViewIssueContext {
            request,
            issue,
            parents,
            timeline,
            response,
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
    parent_id: Option<i64>,
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
                parent_id: None,
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
                parent_id: issue.parent_id,
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
    parent_id: Option<i64>,
    current_revision_id: Option<i64>,
    summary: String,
    description: Option<String>,
    comment: Option<String>,
    completed: bool,
    project_id: Option<i64>,
}

enum IssueUpdateError {
    IssueAlreadyUpdated { current_revision_id: Option<i64> },
    ParentNotFound,
    CantCloseBecauseOfChild,
    InternalError,
}

impl From<sqlx::Error> for IssueUpdateError {
    fn from(sql_error: sqlx::Error) -> Self {
        error!("error while saving issue: {:?}", sql_error);
        Self::InternalError
    }
}

async fn update_issue(
    issue_form: &Form<EditIssueForm>,
    author_id: i64,
) -> Result<Issue, IssueUpdateError> {
    let mut tx = database::pool().begin().await?;
    let issue = if let Some(issue_id) = issue_form.issue_id {
        if let Some(parent_id) = issue_form.parent_id {
            let _ = Issue::load(parent_id)
                .await
                .map_database_error()
                .map_err(|err| {
                    if matches!(err, DatabaseError::RowNotFound) {
                        IssueUpdateError::ParentNotFound
                    } else {
                        IssueUpdateError::InternalError
                    }
                })?;
        }

        let mut issue = Issue::load_for_update(issue_id, &mut tx).await?;
        if issue.current_revision_id != issue_form.current_revision_id {
            return Err(IssueUpdateError::IssueAlreadyUpdated {
                current_revision_id: issue.current_revision_id,
            });
        }

        if issue_form.comment.is_some()
            || issue.summary != issue_form.summary
            || issue.description != issue_form.description
            || issue.completed_at.is_some() != issue_form.completed
            || issue.project_id != issue_form.project_id
            || issue.parent_id != issue_form.parent_id
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
                    let children = IssueQueryBuilder::new()
                        .owned_by(Some(issue.id))
                        .open()
                        .query(&mut tx)
                        .await?;
                    if !children.issues.is_empty() {
                        return Err(IssueUpdateError::CantCloseBecauseOfChild);
                    }
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

            if issue_form.parent_id != issue.parent_id {
                IssueRevisionChange::create(
                    issue_revision.id,
                    "parent_id",
                    issue.parent_id,
                    issue_form.parent_id,
                    &mut tx,
                )
                .await?;
                issue.parent_id = issue_form.parent_id;
            }

            issue.current_revision_id = Some(issue_revision.id);
        }
        issue.save(&mut tx).await?;

        issue
    } else {
        let mut issue = Issue::new(
            author_id,
            issue_form.summary.clone(),
            issue_form.description.clone(),
            issue_form.parent_id,
            issue_form.project_id,
        );
        issue.save(&mut tx).await?;
        issue
    };

    tx.commit().await?;

    Ok(issue)
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
            Ok(issue) => Err(Failure::redirect(format!("/issue/{}", issue.id))),
            Err(error) => {
                let mut current_revision_id = issue_form.current_revision_id;
                let error_messsage = match error {
                    IssueUpdateError::IssueAlreadyUpdated {
                        current_revision_id: updated_revision_id,
                    } => {
                        current_revision_id = updated_revision_id;
                        "issues-error-already-updated"
                    }
                    IssueUpdateError::ParentNotFound => "issues-error-parent-not-found",
                    IssueUpdateError::CantCloseBecauseOfChild => "issues-error-cant-close-child",
                    IssueUpdateError::InternalError => "internal-error-saving",
                }
                .to_string();
                let projects = Project::list().await?;

                Ok(Template::render(
                    "edit_issue",
                    EditIssueContext {
                        request,
                        error_message: Some(error_messsage),
                        issue_id: issue_form.issue_id,
                        current_revision_id,
                        summary: Some(issue_form.summary.clone()),
                        description: issue_form.description.clone(),
                        comment: issue_form.comment.clone(),
                        completed: issue_form.completed,
                        project_id: issue_form.project_id,
                        parent_id: issue_form.parent_id,
                        projects,
                    },
                ))
            }
        }
    } else {
        Err(Failure::redirect("/signin?origin=/issues/new"))
    }
}
