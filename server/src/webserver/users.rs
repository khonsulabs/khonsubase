use rocket::{
    http::{ContentType, Status},
    response::content::Content,
};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use database::schema::accounts::{AccountError, User};

use crate::webserver::{
    auth::SessionId, localization::UserLanguage, Failure, FullPathAndQuery, RequestData, ResultExt,
};
use rocket::{request::Form, response::Redirect};

#[derive(Serialize, Deserialize)]
struct ViewUserContext {
    request: RequestData,
    user: User,
}

#[get("/user/<user_id>")]
pub async fn view_user(
    user_id: i64,
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, Status> {
    let request = RequestData::new(language, path, session).await;
    let user = User::load(user_id, database::pool())
        .await
        .map_sql_to_http()?;

    Ok(Template::render(
        "view_user",
        ViewUserContext { request, user },
    ))
}

#[get("/user/<user_id>/avatar.jpg?<size>")]
pub async fn user_avatar(user_id: i64, size: Option<usize>) -> Result<Content<Vec<u8>>, Status> {
    let size = size.unwrap_or(64);
    let user = User::load(user_id, database::pool())
        .await
        .map_err(|_| Status::NotFound)?;

    let identicon = identicon_rs::new(user.username).size(8).unwrap();
    let identicon = match size {
        512 => identicon.scale(384).unwrap().border(64), // + 32
        256 => identicon.scale(192).unwrap().border(32), // 224 + 16
        128 => identicon.scale(96).unwrap().border(16),  // 112 + 8
        64 => identicon.scale(48).unwrap().border(8),    // 52 + 6
        32 => identicon.scale(24).unwrap().border(4),    // 24 + 4
        16 => identicon.scale(12).unwrap().border(2),    // 12 + 2
        _ => return Err(Status::new(500, "invalid avatar size requested")),
    };
    let identicon = identicon.export_jpeg_data().unwrap();
    Ok(Content(ContentType::JPEG, identicon))
}

#[derive(Serialize, Deserialize)]
struct EditUserContext {
    request: RequestData,
    user: User,
    error_message: Option<String>,
}

#[get("/user/<user_id>/edit")]
pub async fn edit_user(
    user_id: i64,
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        let user = User::load(user_id, database::pool())
            .await
            .map_to_failure()?;
        // TODO permissions: Add admin ability to edit anyone's profile
        if session.account.id == user_id {
            Ok(Template::render(
                "edit_user",
                EditUserContext {
                    request,
                    user,
                    error_message: None,
                },
            ))
        } else {
            Err(Failure::Status(Status::Forbidden))
        }
    } else {
        Err(Failure::Redirect(Redirect::to(format!(
            "/signin?origin=/user/{}/edit",
            user_id
        ))))
    }
}

#[derive(FromForm, Clone, Debug)]
pub struct EditUserForm {
    user_id: i64,
    username: String,
    displayname: Option<String>,
}

async fn update_user(user_form: &Form<EditUserForm>) -> Result<(), AccountError> {
    let mut tx = database::pool().begin().await?;
    // It's a little wasteful to do a load here when as of writing this comment
    // User.save() is hard-coded to an update function. However, if for some reason
    // User.save was updated to support inserting, we'd want this method to still get a
    // row not found error. This ensures we can't accidentally break that safety check
    let mut user = User::load(user_form.user_id, &mut tx).await?;
    user.username = user_form.username.clone();
    user.display_name = user_form
        .displayname
        .clone()
        .map(|name| if name.is_empty() { None } else { Some(name) })
        .flatten();
    user.save(&mut tx).await?;

    tx.commit().await?;

    Ok(())
}

#[post("/users/save", data = "<user_form>")]
pub async fn save_user(
    user_form: Form<EditUserForm>,
    language: UserLanguage,
    path: FullPathAndQuery,
    session: Option<SessionId>,
) -> Result<Template, Failure> {
    let request = RequestData::new(language, path, session).await;
    if let Some(session) = &request.session {
        // TODO permissions: allow admins to edit user profiles
        if session.account.id != user_form.user_id {}
        let result = update_user(&user_form).await;

        match result {
            Ok(_) => Err(Failure::redirect(format!("/user/{}", user_form.user_id))),
            Err(error) => {
                let error_message = match error {
                    AccountError::UsernameTooShort | AccountError::UsernameInvalidCharacter(_) => {
                        "user-error-invalid-username"
                    }
                    AccountError::UsernameConflict => "user-error-username-conflict",
                    AccountError::Sql(sql_error) => {
                        error!("sql error while saving user: {:?}", sql_error);
                        "internal-error-saving"
                    }
                };

                Ok(Template::render(
                    "edit_user",
                    EditUserContext {
                        request,
                        error_message: Some(String::from(error_message)),
                        user: User {
                            id: user_form.user_id,
                            username: user_form.username.clone(),
                            display_name: user_form.displayname.clone(),
                        },
                    },
                ))
            }
        }
    } else {
        Err(Failure::redirect_to_signin(Some(
            &request.current_path_and_query,
        )))
    }
}
