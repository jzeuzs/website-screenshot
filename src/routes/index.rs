use actix_web::{get, HttpRequest};

#[get("/")]
pub async fn index(_req: HttpRequest) -> &'static str {
    "Hello World!"
}
