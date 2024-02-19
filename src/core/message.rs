use actix::Message as ActixMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ChatPayload {
    pub(crate) id: String,
    pub(crate) mime_type: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FriendRequest {
    pub(crate) id: String,
    pub(crate) phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FriendAccept {
    pub(crate) id: String,
}

#[derive(Debug, Clone, ActixMessage, Serialize)]
#[serde(tag = "typ")]
#[rtype(result = "()")]
pub(crate) enum SystemMessage {
    FriendRequest {
        id: String,
        phone: String,
        avatar: Option<String>,
    },
    FriendAccept {
        id: String,
    },
}

#[derive(Debug, Clone, ActixMessage, Serialize)]
#[serde(tag = "typ")]
#[rtype(result = "()")]
pub(crate) enum Message {
    RTC {
        from: String,
        phone: String,
        payload: String,
    },
    Chat {
        from: String,
        phone: String,
        payload: ChatPayload,
    },
    System(SystemMessage),
}
