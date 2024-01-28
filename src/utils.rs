use actix_web::{
    error::{ErrorInternalServerError, ErrorUnauthorized},
    FromRequest,
};
use std::future::ready;

#[derive(Debug)]
pub(crate) struct UserID(pub(crate) String);

impl FromRequest for UserID {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    #[inline]
    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(
            req.headers()
                .get("X-User-ID")
                .ok_or(ErrorUnauthorized("no user id header"))
                .map(|s| {
                    s.to_str()
                        .map_err(ErrorInternalServerError)
                        .map(|s| UserID(s.to_owned()))
                })
                .flatten(),
        )
    }
}
