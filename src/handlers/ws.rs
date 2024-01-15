use crate::core::ws::WS;
use actix::Addr;
use actix_web::{
    error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError, ErrorUnauthorized},
    web::Data,
    web::Payload,
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type AddrMap = Arc<RwLock<HashMap<String, Addr<WS>>>>;

pub async fn index(
    req: HttpRequest,
    stream: Payload,
    map: Data<AddrMap>,
) -> Result<HttpResponse, Error> {
    let uid = req
        .headers()
        .get("X-User-ID")
        .ok_or(ErrorUnauthorized("unlogined"))?
        .to_str()
        .map_err(ErrorBadRequest)?;
    ws::start(WS::new(uid.to_owned(), map.as_ref().clone()), &req, stream)
}
