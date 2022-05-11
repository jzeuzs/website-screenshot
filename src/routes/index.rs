use actix_web::{get, HttpRequest};

#[get("/")]
pub async fn index(_req: HttpRequest) -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::web::Bytes;
    use actix_web::{test, App};

    use super::*;

    trait BodyTest {
        fn as_str(&self) -> &str;
    }

    impl BodyTest for Bytes {
        fn as_str(&self) -> &str {
            unsafe { std::str::from_utf8_unchecked(self) }
        }
    }

    #[actix_web::test]
    async fn test_index() {
        let app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::default().to_request();
        let res = test::call_service(&app, req).await;

        assert!(res.status().is_success());
        assert_eq!("Hello, world!", to_bytes(res.into_body()).await.unwrap().as_str());
    }
}
