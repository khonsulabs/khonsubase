use database::schema::accounts::User;
use rocket::http::{ContentType, Status};
use rocket::response::content::Content;

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
