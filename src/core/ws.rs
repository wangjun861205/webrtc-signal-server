use actix::{Actor, Addr, AsyncContext, Handler, Message, StreamHandler};
use actix_web_actors::ws::{Message as WSMessage, ProtocolError, WebsocketContext};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{from_str, to_string};

use super::addr_keeper::AddrKeeper;

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct OuterRequest<'a> {
    to: String,
    #[serde(borrow)]
    payload: &'a RawValue,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct InnerRequest<'a> {
    from: String,
    #[serde(borrow)]
    payload: &'a RawValue,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct OuterResponse<'a> {
    status_code: u16,
    #[serde(borrow)]
    payload: &'a RawValue,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct InnerResponse<'a> {
    to: String,
    status_code: u16,
    #[serde(borrow)]
    payload: &'a RawValue,
}

pub struct WS<K>
where
    K: AddrKeeper,
{
    id: String,
    addr_keeper: K,
}

impl<K> WS<K>
where
    K: AddrKeeper,
{
    pub fn new(id: String, addr_keeper: K) -> Self {
        Self { id, addr_keeper }
    }
}

impl<K> Actor for WS<K>
where
    K: AddrKeeper + Unpin + 'static,
{
    type Context = WebsocketContext<Self>;
}

impl<'a, K> Handler<InnerRequest<'a>> for WS<K>
where
    K: AddrKeeper + Unpin + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: InnerRequest, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.payload.to_string());
    }
}

impl<K> StreamHandler<Result<WSMessage, ProtocolError>> for WS<K>
where
    K: AddrKeeper + Unpin + 'static,
{
    fn handle(&mut self, item: Result<WSMessage, ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(WSMessage::Ping(msg)) => ctx.pong(&msg),
            Ok(WSMessage::Text(text)) => match from_str::<OuterRequest>(&text) {
                Ok(payload) => {
                    ctx.notify(InnerRequest {
                        from: self.id.clone(),
                        payload: payload.payload,
                    });
                }
                Err(err) => {
                    let payload = OuterResponse {
                        status_code: 400,
                        payload: &RawValue::from_string(format!("{}", err)).unwrap(),
                    };
                    let err_str = to_string(&payload).unwrap();
                    ctx.text(err_str);
                }
            },
            Ok(WSMessage::Close(_)) => {
                self.addr_keeper.logout(&self.id);
            }

            _ => {
                let payload = OuterResponse {
                    status_code: 400,
                    payload: &RawValue::from_string(
                        "Unsupported websocket message type".to_string(),
                    )
                    .unwrap(),
                };
                ctx.text(to_string(&payload).unwrap());
            }
        }
    }
}
