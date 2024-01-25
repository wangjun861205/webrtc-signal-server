use crate::messages::{AcquireFriends, AcquireFriendsResp, InMessage, Login, LoginResp, Logout, LogoutResp, OutMessage, Outcome, OutcomeType, Signup, SignupResp, WSError};
use crate::ws::WS;
use actix::{fut::wrap_future, AsyncContext, Handler};
use auth_service::core::{hasher::Hasher, repository::Repository, token_manager::TokenManager};

impl<R, H, T> Handler<WSError> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, WSError { status, reason }: WSError, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&Outcome::<String>::error(OutcomeType::WSError, status, reason)).unwrap());
    }
}

impl<R, H, T> Handler<Login> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, Login { username, password }: Login, ctx: &mut Self::Context) -> Self::Result {
        let self_addr = ctx.address();
        let auth_service = self.auth_service.clone();
        let addrs = self.addrs.clone();
        let token_manager = self.auth_service.token_manager.clone();
        ctx.spawn(wrap_future(async move {
            match auth_service.login_by_password(&username, &password).await {
                Ok(token) => {
                    let id = token_manager.verify_token(&token).await.unwrap();
                    addrs.write().await.insert(id, self_addr.clone());
                    self_addr.do_send(LoginResp { token })
                }
                Err(err) => self_addr.do_send(WSError { status: 403, reason: err.to_string() }),
            }
        }));
    }
}

impl<R, H, T> Handler<LoginResp> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, resp: LoginResp, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&Outcome::success(OutcomeType::Login, Some(resp))).unwrap());
    }
}

impl<R, H, T> Handler<Logout> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, Logout { token }: Logout, ctx: &mut Self::Context) -> Self::Result {
        let self_addr = ctx.address();
        let auth_service = self.auth_service.clone();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            match auth_service.token_manager.verify_token(token.clone()).await {
                Ok(id) => {
                    addrs.write().await.remove(&id);
                    self_addr.do_send(LogoutResp);
                }
                Err(err) => self_addr.do_send(WSError { status: 403, reason: err.to_string() }),
            }
        }));
    }
}

impl<R, H, T> Handler<LogoutResp> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, resp: LogoutResp, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&Outcome::success(OutcomeType::Logout, Some(resp))).unwrap());
    }
}

impl<R, H, T> Handler<Signup> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, Signup { username, password }: Signup, ctx: &mut Self::Context) -> Self::Result {
        let self_addr = ctx.address();
        let auth_service = self.auth_service.clone();
        ctx.spawn(wrap_future(async move {
            match auth_service.signup(&username, &password).await {
                Ok(_) => self_addr.do_send(SignupResp),
                Err(err) => self_addr.do_send(WSError { status: 500, reason: err.to_string() }),
            }
        }));
    }
}

impl<R, H, T> Handler<SignupResp> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, _: SignupResp, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&Outcome::<String>::success(OutcomeType::Signup, None)).unwrap());
    }
}

impl<R, H, T> Handler<AcquireFriends> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, AcquireFriends { token }: AcquireFriends, ctx: &mut Self::Context) -> Self::Result {
        let self_addr = ctx.address();
        let auth_service = self.auth_service.clone();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            match auth_service.verify_token(&token).await {
                Ok(_) => {
                    self_addr.do_send(AcquireFriendsResp {
                        friends: addrs.read().await.keys().map(|k| k.to_owned()).collect(),
                    });
                }
                Err(err) => self_addr.do_send(WSError { status: 403, reason: err.to_string() }),
            }
        }));
    }
}

impl<R, H, T> Handler<AcquireFriendsResp> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, resp: AcquireFriendsResp, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&Outcome::success(OutcomeType::AcquireFriends, Some(resp))).unwrap());
    }
}

impl<R, H, T> Handler<InMessage> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, InMessage { token, to, content }: InMessage, ctx: &mut Self::Context) -> Self::Result {
        let self_addr = ctx.address();
        let auth_service = self.auth_service.clone();
        let addrs = self.addrs.clone();
        ctx.spawn(wrap_future(async move {
            match auth_service.verify_token(&token).await {
                Ok(id) => {
                    if let Some(addr) = addrs.read().await.get(&to) {
                        addr.do_send(OutMessage { from: id, content });
                        return;
                    }
                    self_addr.do_send(WSError {
                        status: 404,
                        reason: "User not found".to_string(),
                    });
                }
                Err(err) => self_addr.do_send(WSError { status: 403, reason: err.to_string() }),
            }
        }));
    }
}

impl<R, H, T> Handler<OutMessage> for WS<R, H, T>
where
    R: Repository + Clone + 'static,
    H: Hasher + Clone + 'static,
    T: TokenManager + Clone + 'static,
{
    type Result = ();
    fn handle(&mut self, resp: OutMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&Outcome::success(OutcomeType::Message, Some(resp))).unwrap());
    }
}
