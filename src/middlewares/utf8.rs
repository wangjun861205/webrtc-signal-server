use core::task::Context;
use std::str::FromStr;
use std::{future::Future, pin::Pin, task::Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
};

pub struct UTF8Service<S>
where
    S: Service<ServiceRequest>,
{
    service: S,
}

impl<S> Service<ServiceRequest> for UTF8Service<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Future: 'static,
{
    type Error = S::Error;
    type Response = ServiceResponse;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, _ctx: &mut Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);
        Box::pin(async move {
            let mut res = fut.await?;
            if res.status().is_success() {
                res.headers_mut().insert(
                    HeaderName::from_str("Content-Type").unwrap(),
                    HeaderValue::from_str("application/json; charset=utf-8").unwrap(),
                );
            }
            Ok(res)
        })
    }
}

pub struct UTF8Middleware;

impl<S> Transform<S, ServiceRequest> for UTF8Middleware
where
    S: Service<ServiceRequest, Response = ServiceResponse> + 'static,
{
    type InitError = ();
    type Error = S::Error;
    type Response = ServiceResponse;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Transform, Self::InitError>>>>;
    type Transform = UTF8Service<S>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(async move { Ok(UTF8Service { service }) })
    }
}
