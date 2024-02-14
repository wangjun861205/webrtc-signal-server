use actix::Message as ActixMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Income {
    Message { to: String, content: String },
    AddFriend { user_id: String },
    // FriendRequests,
    Accept { id: String },
    ChatMessage { to: String, content: String },
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Online;

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Greet {
    pub(crate) user_id: String,
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct AddFriend {
    pub(crate) user_id: String,
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct FriendRequests;

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Accept {
    pub(crate) id: String,
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct AcquireFriends;

#[derive(ActixMessage, Deserialize)]
#[rtype(result = "()")]
pub(crate) struct InMessage {
    pub(crate) user_id: String,
    pub(crate) to: String,
    pub(crate) content: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct OutMessage {
    pub(crate) from: String,
    pub(crate) content: String,
}

#[derive(Debug, Serialize)]
pub(crate) enum OutcomeType {
    Message,
    ChatMessage,
    AddFriend,
    FriendRequests,
    Accept,
    Other,
}

#[derive(Debug, Serialize, ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Outcome<T>
where
    T: Serialize,
{
    pub(crate) typ: OutcomeType,
    pub(crate) status: u16,
    pub(crate) reason: Option<String>,
    pub(crate) data: Option<T>,
}

impl<T> Outcome<T>
where
    T: Serialize,
{
    pub(crate) fn success(typ: OutcomeType, data: T) -> Self {
        Self {
            typ,
            status: 200,
            reason: None,
            data: Some(data),
        }
    }

    pub(crate) fn error(typ: OutcomeType, status: u16, reason: String) -> Self {
        Self {
            typ,
            status,
            reason: Some(reason),
            data: None,
        }
    }
}

#[derive(Debug, Serialize, ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct InChatMessage {
    pub(crate) to: String,
    pub(crate) content: String,
}

#[derive(Debug, Serialize, ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct OutChatMessage {
    pub(crate) id: String,
    pub(crate) from: String,
    pub(crate) content: String,
}
