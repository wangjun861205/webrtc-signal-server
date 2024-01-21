use crate::handlers::ws::WS;
use actix::{fut::wrap_future, prelude::*};
use auth_service::core::{hasher::Hasher, repository::Repository, token_manager::TokenManager};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

#[derive(Serialize, Deserialize)]
pub enum PayloadType {
    Login,
    Logout,
    Message,
}

#[derive(Serialize, Deserialize)]
pub enum ResponseType {
    Error,
    LoginResponse,
    GreetResponse,
    Message,
}

#[derive(Serialize, Deserialize, Message, MessageResponse)]
#[rtype(result = "()")]
pub struct Out {
    pub from: String,
    pub status_code: u16,
    pub typ: ResponseType,
    pub payload: String,
}

impl Out {
    pub fn new(
        from: impl Into<String>,
        status_code: u16,
        typ: ResponseType,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            status_code,
            typ,
            payload: payload.into(),
        }
    }
}

impl<R, H, T> Handler<Out> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, msg: Out, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(to_string(&msg).unwrap());
    }
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct In {
    pub token: String,
    pub to: String,
    pub payload: String,
}

#[derive(Deserialize)]
pub enum ControlMessageType {
    Greet,
}

#[derive(Deserialize)]
pub struct ControlMessage {
    typ: ControlMessageType,
    data: Option<String>,
}

impl<R, H, T> Handler<In> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, msg: In, ctx: &mut Self::Context) -> Self::Result {
        let auth_service = self.auth_service.clone();
        let self_addr = ctx.address();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            match auth_service.token_manager.verify_token(msg.token).await {
                Err(e) => {
                    self_addr.do_send(Out::new("server", 403, ResponseType::Error, e.to_string()))
                }
                Ok(id) => {
                    if msg.to == "server" {
                        match from_str::<ControlMessage>(&msg.payload) {
                            Err(e) => {
                                self_addr.do_send(Out::new(
                                    "server",
                                    400,
                                    ResponseType::Error,
                                    e.to_string(),
                                ));
                            }
                            Ok(msg) => match msg.typ {
                                ControlMessageType::Greet => {
                                    addrs.write().await.insert(id.clone(), self_addr.clone());
                                    self_addr.do_send(Out::new(
                                        "server",
                                        200,
                                        ResponseType::GreetResponse,
                                        to_string(
                                            &addrs
                                                .read()
                                                .await
                                                .keys()
                                                .map(|k| k.to_owned())
                                                .collect::<Vec<String>>(),
                                        )
                                        .unwrap(),
                                    ));
                                }
                            },
                        }
                        return;
                    }
                    for key in addrs.read().await.keys() {
                        println!("{}", key)
                    }
                    if let Some(addr) = addrs.read().await.get(&msg.to) {
                        addr.do_send(Out::new(id, 200, ResponseType::Message, msg.payload));
                        return;
                    }
                    self_addr.do_send(Out::new(
                        "server",
                        404,
                        ResponseType::Error,
                        "invalid destination",
                    ));
                }
            }
        }));
    }
}
