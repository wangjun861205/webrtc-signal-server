use actix::Message as ActixMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct RTCMessage {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ChatMessagePayload {
    pub(crate) mime_type: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct ChatMessage {
    pub(crate) from: String,
    pub(crate) phone: String,
    pub(crate) payload: ChatMessagePayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum SystemMessageType {
    FriendRequest,
    FriendAccept,
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

#[derive(Debug, Clone, ActixMessage)]
#[rtype(result = "()")]
pub(crate) enum SystemMessage {
    FriendRequest(FriendRequest),
    FriendAccept(FriendAccept),
}

#[derive(Debug, Clone, ActixMessage)]
#[rtype(result = "()")]
pub(crate) enum Message {
    RTC(RTCMessage),
    Chat(ChatMessage),
    System(SystemMessage),
}
