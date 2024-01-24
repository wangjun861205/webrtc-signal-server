use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum MessageType {
    Login,
    LoginResp,
    Logout,
    LogoutResp,
    AcquireUsers,
    AcquireUsersResp,
    UserMessage,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Login {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Message {
    pub typ: MessageType,
    pub payload: Value,
}

// pub(crate) fn deserialize_message<T>(data: &str) -> Result<T, Box<dyn Error>> {
//     let message: Message = serde_json::from_str(data).unwrap();
//     match message.typ {
//         MessageType::Login => {
//             let login: Login =
//                 serde_json::from_str(&message.payload.to_string()).map_err(Box::new)?;
//             Ok(login)
//         }
//         _ => {}
//     }
// }
