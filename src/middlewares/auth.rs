use std::env;
use std::future::{ready, Ready};

use actix_web::body::EitherBody;
use actix_web::dev::{self, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::{Error, HttpResponse};
use futures_util::future::LocalBoxFuture;

use crate::error::Error as Errors;

#[derive(Debug)]
pub struct Auth;

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service,
        }))
    }
}

#[derive(Debug)]
pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_token = env::var("AUTH_TOKEN").ok();
        let headers = req.headers().clone();
        let auth_header = headers.get(header::AUTHORIZATION);
        let (req, pl) = req.into_parts();

        if let Some(auth_token) = auth_token {
            if req.path() == "/screenshot" {
                if let Some(auth) = auth_header {
                    let auth = auth.to_str().expect("Failed converting to str").to_owned();

                    if auth_token != auth {
                        let response = HttpResponse::from_error(Errors::Unauthorized)
                            .map_into_right_body::<B>();

                        return Box::pin(async { Ok(ServiceResponse::new(req, response)) });
                    }
                } else {
                    let response = HttpResponse::from_error(Errors::MissingAuthToken)
                        .map_into_right_body::<B>();

                    return Box::pin(async { Ok(ServiceResponse::new(req, response)) });
                }
            }
        }

        let response = self.service.call(ServiceRequest::from_parts(req, pl));

        Box::pin(async move { response.await.map(ServiceResponse::map_into_left_body) })
    }
}
