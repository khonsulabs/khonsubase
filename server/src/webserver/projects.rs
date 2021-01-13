use rocket::{
    http::Status,
    request::Form,
    response::{content::Content, Redirect},
};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use database::schema::issues::{Project, ProjectError};

use crate::webserver::{
    auth::SessionId, localization::UserLanguage, Failure, FullPathAndQuery, RequestData, ResultExt,
};
use database::sqlx::types::chrono::Utc;

// #[derive(Serialize, Deserialize)]
// struct ViewUserContext {
//     request: RequestData,
//     user: User,
// }

#[get("/project/<project_id>")]
pub async fn view_project(
    project_id: i64,
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, Status> {
    todo!()
}

#[derive(Serialize, Deserialize)]
struct EditProjectContext {
    request: RequestData,
    project: Project,
    error_message: Option<String>,
}

#[get("/project/<project_id>/edit")]
pub async fn edit_project(
    project_id: i64,
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        let project = Project::load(project_id).await.map_to_failure()?;

        if session.account.administrator || session.account.id == project.owner_id {
            Ok(Template::render(
                "edit_project",
                EditProjectContext {
                    request,
                    project,
                    error_message: None,
                },
            ))
        } else {
            Err(Failure::Status(Status::Forbidden))
        }
    } else {
        Err(Failure::redirect_to_signin(Some(&request.current_path)))
    }
}

#[get("/projects/new")]
pub async fn new_project(
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        if session.account.administrator {
            let owner_id = session.account.id;
            Ok(Template::render(
                "edit_project",
                EditProjectContext {
                    request,
                    project: Project::new(Default::default(), Default::default(), None, owner_id),
                    error_message: None,
                },
            ))
        } else {
            Err(Failure::Status(Status::Forbidden))
        }
    } else {
        Err(Failure::redirect_to_signin(Some(&request.current_path)))
    }
}

#[derive(FromForm, Clone, Debug)]
pub struct EditProjectForm {
    project_id: i64,
    slug: String,
    name: String,
    description: Option<String>,
}

async fn update_project(
    project_form: &Form<EditProjectForm>,
    current_user_id: i64,
) -> Result<Project, ProjectError> {
    let description = project_form
        .description
        .clone()
        .map(|description| {
            if description.is_empty() {
                None
            } else {
                Some(description)
            }
        })
        .flatten();

    let mut tx = database::pool().begin().await?;
    let mut project = if project_form.project_id == 0 {
        Project::new(
            project_form.slug.clone(),
            project_form.name.clone(),
            description,
            current_user_id,
        )
    } else {
        let mut project = Project::load_for_update(project_form.project_id, &mut tx).await?;
        project.slug = project_form.slug.clone();
        project.name = project_form.name.clone();
        project.description = description;
        project
    };
    project.save(&mut tx).await?;

    tx.commit().await?;

    Ok(project)
}

#[post("/projects/save", data = "<project_form>")]
pub async fn save_project(
    project_form: Form<EditProjectForm>,
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        if session.account.administrator || session.account.id != project_form.project_id {
            let result = update_project(&project_form, session.account.id).await;

            match result {
                Ok(project) => Err(Failure::redirect(format!("/project/{}", project.id))),
                Err(error) => {
                    let error_message = match error {
                        ProjectError::SlugInvalidCharacter(_) => "project-error-invalid-username",
                        ProjectError::SlugConflict => "project-error-slug-conflict",
                        ProjectError::ProjectNotFound => return Err(Failure::not_found()),
                        ProjectError::Sql(sql_error) => {
                            error!("sql error while saving project: {:?}", sql_error);
                            "internal-error-saving"
                        }
                    };

                    Ok(Template::render(
                        "edit_project",
                        EditProjectContext {
                            request,
                            error_message: Some(String::from(error_message)),
                            project: Project {
                                id: project_form.project_id,
                                slug: project_form.slug.clone(),
                                name: project_form.name.clone(),
                                description: project_form.description.clone(),
                                owner_id: 0,
                                created_at: Utc::now(),
                            },
                        },
                    ))
                }
            }
        } else {
            Err(Failure::forbidden())
        }
    } else {
        let origin = if project_form.project_id == 0 {
            "/projects/new".to_string()
        } else {
            format!("/project/{}", project_form.project_id)
        };
        Err(Failure::redirect_to_signin(Some(&origin)))
    }
}
