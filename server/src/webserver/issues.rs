use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    str::FromStr,
};

use rocket::{http::Status, request::Form};
use rocket_contrib::templates::{tera, Template};
use serde::{Deserialize, Serialize};

use database::{
    schema::issues::{
        ContextualizedRelationship, Issue, IssueQueryBuilder, IssueQueryResults, IssueRelationship,
        IssueRevision, IssueRevisionChange, IssueRevisionView, IssueView, Project, Relationship,
        Tag, Taxonomy,
    },
    sqlx,
    sqlx::types::chrono::Utc,
    DatabaseError, SqlxResultExt,
};

use crate::{
    webserver::{
        auth::SessionId, localization::UserLanguage, Failure, FullPathAndQuery, RequestData,
        ResultExt,
    },
    Optionable,
};

#[derive(Serialize, Deserialize)]
struct ListIssuesContext {
    request: RequestData,
    response: IssueQueryResults,
    taxonomy: Taxonomy,
}

pub trait AuthoredBy {
    fn author_id(&self) -> i64;
}

impl AuthoredBy for IssueView {
    fn author_id(&self) -> i64 {
        self.author.id
    }
}

impl AuthoredBy for Issue {
    fn author_id(&self) -> i64 {
        self.author_id
    }
}

pub fn can_edit_issue<I: AuthoredBy>(request: &RequestData, _issue: &I) -> bool {
    request.session.is_some()
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
    let taxonomy = Taxonomy::load(database::pool()).await.map_sql_to_http()?;

    Ok(Template::render(
        "list_issues",
        ListIssuesContext {
            request,
            response,
            taxonomy,
        },
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
    relationships: Vec<IssueRelationship>,
    timeline: IssueTimeline,
    response: IssueQueryResults,
    editable: bool,
    projects: HashMap<i64, Project>,
    tags: Vec<i32>,
    taxonomy: Taxonomy,
}

async fn render_issue(request: RequestData, issue_id: i64) -> sqlx::Result<Template> {
    let (issue, parents, relationships, entries, response, projects, tags, taxonomy) = futures::try_join!(
        IssueView::load(issue_id),
        Issue::all_parents(issue_id),
        IssueRelationship::list_for(issue_id, database::pool()),
        IssueRevisionView::list_for(issue_id),
        IssueQueryBuilder::new()
            .owned_by(Some(issue_id))
            .query(database::pool()),
        Project::list_as_map(),
        Tag::list_for_issue(issue_id),
        Taxonomy::load(database::pool())
    )?;
    let timeline = IssueTimeline { entries };
    let editable = can_edit_issue(&request, &issue);
    Ok(Template::render(
        "view_issue",
        ViewIssueContext {
            request,
            issue,
            parents,
            relationships,
            timeline,
            response,
            editable,
            projects,
            taxonomy,
            tags: tags.into_iter().map(|t| t.id).collect(),
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
    started: bool,
    completed: bool,
    project_id: Option<i64>,
    ungrouped_tags: Vec<String>,

    projects: Vec<Project>,
    taxonomy: Taxonomy,
}

#[get("/issues/new?<summary>&<description>&<project_id>&<parent_id>")]
pub async fn new_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    summary: Option<String>,
    description: Option<String>,
    project_id: Option<i64>,
    parent_id: Option<i64>,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    let projects = Project::list().await?;
    let taxonomy = Taxonomy::load(database::pool()).await?;
    if request.logged_in() {
        Ok(Template::render(
            "edit_issue",
            EditIssueContext {
                request,
                summary,
                description,
                project_id,
                parent_id,
                projects,
                taxonomy,

                issue_id: None,
                current_revision_id: None,
                error_message: None,
                comment: None,
                completed: false,
                started: false,
                ungrouped_tags: Default::default(),
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
        if can_edit_issue(&request, &issue) {
            let projects = Project::list().await?;
            let taxonomy = Taxonomy::load(database::pool()).await?;
            let mut unassigned_tags = Vec::default();

            for tag in Tag::list_for_issue(issue.id).await? {
                if tag.tag_group_id.is_none() {
                    unassigned_tags.push(tag.name);
                }
            }

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
                    started: issue.started_at.is_some(),
                    completed: issue.completed_at.is_some(),
                    project_id: issue.project_id,
                    parent_id: issue.parent_id,
                    ungrouped_tags: unassigned_tags,
                    projects,
                    taxonomy,
                },
            ))
        } else {
            Err(Failure::forbidden())
        }
    } else {
        Err(Failure::redirect_to_signin(Some(
            &request.current_path_and_query,
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
    started: bool,
    completed: bool,
    project_id: Option<i64>,
    tags: String,
}

enum IssueUpdateError {
    IssueAlreadyUpdated { current_revision_id: Option<i64> },
    ParentNotFound,
    CantCloseBecauseOfChild,
    CantCloseBecauseBlocked,
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
    taxonomy: &Taxonomy,
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

        let mut changed_issue_status = false;

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

            issue.update_blocked_status(&mut tx).await?;

            if issue_form.started != issue.started_at.is_some() {
                let new_value = if issue_form.started {
                    Some(Utc::now())
                } else {
                    None
                };
                IssueRevisionChange::create(
                    issue_revision.id,
                    "started_at",
                    issue.started_at,
                    new_value,
                    &mut tx,
                )
                .await?;
                issue.started_at = new_value;
            }

            if issue_form.completed != issue.completed_at.is_some() {
                if issue.blocked {
                    return Err(IssueUpdateError::CantCloseBecauseBlocked);
                }

                let new_value = if issue_form.completed {
                    // Make sure no children are open
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
                changed_issue_status = true;
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

            let mut existing_tags = Tag::list_for_issue(issue.id).await?;
            let mut tags_to_remove = existing_tags.iter().map(|t| t.id).collect::<HashSet<_>>();
            let mut tags_to_insert = HashSet::new();
            let mut all_tags = Vec::new();

            for input_tag in parse_tags(&issue_form.tags) {
                if let Some(tag_id) = taxonomy.lower_map.get(&input_tag.to_lowercase()) {
                    all_tags.push(taxonomy.tags[tag_id].name.clone());

                    if existing_tags.iter().any(|t| t.id == *tag_id) {
                        tags_to_remove.remove(tag_id);
                    } else {
                        tags_to_insert.insert(*tag_id);
                    }
                } else {
                    let mut tag = Tag::new(input_tag.to_string());
                    tag.save(&mut tx)
                        .await
                        .map_err(|_| IssueUpdateError::InternalError)?;
                    tags_to_insert.insert(tag.id);
                    all_tags.push(tag.name);
                }
            }

            if !tags_to_remove.is_empty() || !tags_to_insert.is_empty() {
                existing_tags.sort_by_key(|t| t.name.to_lowercase());
                all_tags.sort();

                let old_value = if existing_tags.is_empty() {
                    None
                } else {
                    Some(
                        existing_tags
                            .into_iter()
                            .map(|t| t.name)
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                };

                let new_value = if all_tags.is_empty() {
                    None
                } else {
                    Some(all_tags.join(", "))
                };

                IssueRevisionChange::create(
                    issue_revision.id,
                    "tags",
                    old_value,
                    new_value,
                    &mut tx,
                )
                .await?;

                // Execute the actual changes
                for tag in tags_to_remove {
                    issue.remove_tag(tag, &mut tx).await?;
                }

                for tag in tags_to_insert {
                    issue.add_tag(tag, &mut tx).await?;
                }
            }

            issue.current_revision_id = Some(issue_revision.id);
        }
        issue.save(&mut tx).await?;

        if changed_issue_status {
            tx = Issue::update_blocked_relationships(&[issue.id], tx).await?;
        }

        issue
    } else {
        let mut issue = Issue::new(
            author_id,
            issue_form.summary.clone(),
            issue_form.description.clone(),
            issue_form.parent_id,
            issue_form.project_id,
        );

        if issue_form.started {
            issue.started_at = Some(Utc::now());
        }

        if issue_form.completed {
            issue.completed_at = Some(Utc::now());
        }

        issue.save(&mut tx).await?;
        issue
    };

    tx.commit().await?;

    Ok(issue)
}

fn parse_tags(source: &str) -> Vec<Cow<'_, str>> {
    #[derive(Deserialize)]
    struct TagifyTag<'a> {
        value: Cow<'a, str>,
    }
    let mut tags: Vec<_> = if let Ok(tags) = serde_json::from_str::<Vec<TagifyTag>>(source) {
        tags.into_iter().map(|s| s.value).collect()
    } else {
        source.split(',').map(|s| Cow::from(s.trim())).collect()
    };

    tags.retain(|s| !s.is_empty());

    tags
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
        if issue_form.issue_id.is_none()
            || can_edit_issue(&request, &Issue::load(issue_form.issue_id.unwrap()).await?)
        {
            let taxonomy = Taxonomy::load(database::pool()).await?;
            let result = update_issue(&issue_form, session.account.id, &taxonomy).await;

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
                        IssueUpdateError::CantCloseBecauseOfChild => {
                            "issues-error-cant-close-child"
                        }
                        IssueUpdateError::CantCloseBecauseBlocked => {
                            "issues-error-cant-close-blocked"
                        }
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
                            started: issue_form.started,
                            completed: issue_form.completed,
                            project_id: issue_form.project_id,
                            parent_id: issue_form.parent_id,
                            ungrouped_tags: issue_form
                                .tags
                                .split(',')
                                .map(|s| s.to_string())
                                .collect(),

                            projects,
                            taxonomy,
                        },
                    ))
                }
            }
        } else {
            Err(Failure::forbidden())
        }
    } else if let Some(id) = issue_form.issue_id {
        Err(Failure::redirect_to_signin(Some(&format!(
            "/issue/{}/edit",
            id
        ))))
    } else {
        Err(Failure::redirect_to_signin(Some("/issues/new")))
    }
}

#[derive(Serialize, Deserialize)]
struct LinkIssueContext {
    request: RequestData,
    issue: IssueView,
    existing: bool,
    target: Option<i64>,
    relationship: Option<String>,
    comment: Option<String>,
}

#[get("/issue/<issue_id>/link?<to>")]
pub async fn link_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
    to: Option<i64>,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    let issue = IssueView::load(issue_id).await?;
    let target = to;
    let mut relationship = None;
    let mut comment = None;
    let mut existing = false;

    if let Some(target_id) = target {
        if let Ok(link) = IssueRelationship::find(issue_id, target_id, database::pool()).await {
            relationship = link
                .relationship
                .relationship
                .map(|_| link.relationship.to_string());
            comment = link.comment;
            existing = true;
        }
    }

    if can_edit_issue(&request, &issue) {
        Ok(Template::render(
            "link_issue",
            LinkIssueContext {
                request,
                issue,
                target,
                relationship,
                comment,
                existing,
            },
        ))
    } else {
        Err(Failure::forbidden())
    }
}

#[get("/issue/<issue_id>/unlink/<other_issue_id>")]
pub async fn unlink_issue(
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
    other_issue_id: i64,
) -> Result<(), Failure> {
    let request = RequestData::new(language, path, session).await;
    let issue = IssueView::load(issue_id).await?;

    if can_edit_issue(&request, &issue) {
        let mut tx = database::pool().begin().await?;
        IssueRelationship::unlink(issue_id, other_issue_id, &mut tx).await?;
        tx = Issue::update_blocked_relationships(&[issue_id, other_issue_id], tx).await?;
        tx.commit().await?;

        Err(Failure::redirect(format!("/issue/{}", issue_id)))
    } else {
        Err(Failure::forbidden())
    }
}

#[derive(FromForm, Clone, Debug)]
pub struct LinkIssueForm {
    target: i64,
    relationship: String,
    comment: Option<String>,
}

async fn link_issues(
    issue_a: i64,
    issue_b: i64,
    relationship: Option<Relationship>,
    comment: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = database::pool().begin().await?;
    IssueRelationship::link(issue_a, issue_b, relationship, comment, database::pool()).await?;

    if matches!(relationship, Some(Relationship::Blocks)) {
        tx = Issue::update_blocked_relationships(&[issue_a], tx).await?;
    }

    tx.commit().await?;

    Ok(())
}

#[post("/issue/<issue_id>/link", data = "<form>")]
pub async fn link_issue_post(
    form: Form<LinkIssueForm>,
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
    issue_id: i64,
) -> Result<(), Failure> {
    let request = RequestData::new(language, path, session).await;
    let issue = IssueView::load(issue_id).await?;

    if can_edit_issue(&request, &issue) {
        let link = ContextualizedRelationship::from_str(&form.relationship)?;
        let (issue_a, issue_b) = if link.is_inverse {
            (form.target, issue_id)
        } else {
            (issue_id, form.target)
        };

        link_issues(
            issue_a,
            issue_b,
            link.relationship,
            form.comment
                .as_ref()
                .map(|comment| comment.trim().into_option())
                .flatten(),
        )
        .await?;

        Err(Failure::redirect(format!("/issue/{}", issue_id)))
    } else {
        Err(Failure::forbidden())
    }
}

pub struct RelationshipSummaryKeyFilter;

impl tera::Filter for RelationshipSummaryKeyFilter {
    fn filter(
        &self,
        relationship: &tera::Value,
        _: &HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        let link = if relationship.is_null() {
            ContextualizedRelationship::plain()
        } else {
            serde_json::from_value::<ContextualizedRelationship>(relationship.clone())?
        };
        let key = format!("issue-relationship-{}-summary", link);
        Ok(tera::Value::String(key))
    }
}
