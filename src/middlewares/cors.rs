use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpResponse};
use core::task::Context;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

pub struct CORSService<S>
where
    S: Service<ServiceRequest>,
{
    service: S,
}

impl<S> Service<ServiceRequest> for CORSService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
{
    type Error = Error;
    type Response = ServiceResponse;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, _ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);
        Box::pin(async move {
            let mut res = fut.await?;
            res.headers_mut().insert(
                "Access-Control-Allow-Origin".parse().unwrap(),
                "*".parse().unwrap(),
            );
            res.headers_mut().insert(
                "Access-Control-Allow-Credentials".parse().unwrap(),
                "true".parse().unwrap(),
            );
            Ok(res)
        })
    }
}

#[derive(Default)]
pub struct CORSMiddleware;

impl<S> Transform<S, ServiceRequest> for CORSMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
{
    type Error = Error;
    type Response = ServiceResponse;
    type Transform = CORSService<S>;
    type InitError = ();
    type Future = Pin<Box<dyn Future<Output = Result<Self::Transform, Self::InitError>>>>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(async move { Ok(CORSService { service }) })
    }
}
