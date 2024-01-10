use std::{collections::HashMap, sync::Arc};

use actix::{fut::wrap_future, Actor, Addr, AsyncContext, Handler, Message, StreamHandler};
use actix_web_actors::ws::{Message as WSMessage, ProtocolError, WebsocketContext};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct Login(String);

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct LoginSuccess;

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct Logout;

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct In {
    to: String,
    payload: String,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct Out {
    from: String,
    status_code: u16,
    payload: String,
}

pub struct WS {
    id: String,
    addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>,
    has_logined: bool,
}

impl WS {
    pub fn new(id: String, addrs: Arc<RwLock<HashMap<String, Addr<Self>>>>) -> Self {
        Self {
            id,
            addrs,
            has_logined: false,
        }
    }
}

impl Actor for WS {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify(Login(self.id.clone()));
    }
}

impl Handler<Login> for WS {
    type Result = ();

    fn handle(&mut self, Login(id): Login, ctx: &mut Self::Context) -> Self::Result {
        let addrs = self.addrs.clone();
        let self_addr = ctx.address();
        ctx.spawn(wrap_future(async move {
            addrs.write().await.insert(id.clone(), self_addr.clone());
            self_addr.do_send(LoginSuccess);
        }));
    }
}

impl Handler<LoginSuccess> for WS {
    type Result = ();
    fn handle(&mut self, _msg: LoginSuccess, _ctx: &mut Self::Context) -> Self::Result {
        self.has_logined = true;
    }
}

impl Handler<Logout> for WS {
    type Result = ();
    fn handle(&mut self, _msg: Logout, ctx: &mut Self::Context) -> Self::Result {
        let addrs = self.addrs.clone();
        let id = self.id.clone();
        ctx.spawn(wrap_future(async move {
            addrs.write().await.remove(&id);
        }));
    }
}

impl Handler<In> for WS {
    type Result = ();
    fn handle(&mut self, msg: In, ctx: &mut Self::Context) -> Self::Result {
        if !self.has_logined {
            ctx.notify(Out {
                from: "server".into(),
                status_code: 401,
                payload: "has not logined, please try again later".into(),
            })
        }
        let id = self.id.clone();
        let addrs = self.addrs.clone();
        let self_addr = ctx.address();
        ctx.spawn(wrap_future(async move {
            if let Some(addr) = addrs.read().await.get(&msg.to) {
                addr.do_send(Out {
                    from: id.clone(),
                    status_code: 200,
                    payload: msg.payload,
                });
                return;
            }
            self_addr.do_send(Out {
                from: "server".into(),
                status_code: 404,
                payload: format!("User {} not found", msg.to),
            });
        }));
    }
}

impl Handler<Out> for WS {
    type Result = ();
    fn handle(&mut self, msg: Out, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(to_string(&msg).unwrap());
    }
}

impl StreamHandler<Result<WSMessage, ProtocolError>> for WS {
    fn handle(&mut self, item: Result<WSMessage, ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(WSMessage::Ping(msg)) => ctx.pong(&msg),
            Ok(WSMessage::Text(text)) => match from_str::<In>(&text) {
                Ok(payload) => {
                    ctx.notify(payload);
                }
                Err(err) => {
                    ctx.notify(Out {
                        from: "server".into(),
                        status_code: 400,
                        payload: err.to_string(),
                    });
                }
            },
            Ok(WSMessage::Close(_)) => {}

            _ => {
                let payload = Out {
                    from: "server".into(),
                    status_code: 400,
                    payload: "Unsupported websocket message type".to_string(),
                };
                ctx.text(to_string(&payload).unwrap());
            }
        }
    }
}
