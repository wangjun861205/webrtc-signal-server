use actix::Message as ActixMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum Income {
    Login { username: String, password: String },
    Signup { username: String, password: String },
    Logout { token: String },
    AcquireFriends { token: String },
    Message { token: String, to: String, content: String },
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Login {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct LoginResp {
    pub(crate) token: String,
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Signup {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct SignupResp;

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct Logout {
    pub(crate) token: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct LogoutResp;

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct AcquireFriends {
    pub(crate) token: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct AcquireFriendsResp {
    pub(crate) friends: Vec<String>,
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub(crate) struct InMessage {
    pub(crate) token: String,
    pub(crate) to: String,
    pub(crate) content: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct OutMessage {
    pub(crate) from: String,
    pub(crate) content: String,
}

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub(crate) struct WSError {
    pub(crate) status: u16,
    pub(crate) reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) enum OutcomeType {
    WSError,
    Login,
    Logout,
    Signup,
    AcquireFriends,
    Message,
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
    pub(crate) fn success(typ: OutcomeType, data: Option<T>) -> Outcome<T>
    where
        T: Serialize,
    {
        Outcome { typ, status: 200, reason: None, data }
    }

    pub(crate) fn error(typ: OutcomeType, status: u16, reason: String) -> Outcome<T> {
        Outcome {
            typ,
            status,
            reason: Some(reason),
            data: None,
        }
    }
}
