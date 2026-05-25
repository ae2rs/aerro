//! `#[derive(aerro::AerroHandler)]` adapter test.

#![cfg(feature = "macro")]

use aerro::{Handler, ServiceFailure, StatusIntoResultExt};

#[derive(Debug, aerro::Aerro)]
pub enum CreateUser {
    #[aerro(
        category = Business,
        code = AlreadyExists,
        error = "email already taken: {email}"
    )]
    EmailTaken { email: String },
}

#[derive(aerro::AerroHandler)]
#[aerro(service = "create-user", rpc = "create", exposure = Public, max_frames = 8)]
struct CreateUserHandler;

impl Handler for CreateUserHandler {
    type Request = String;
    type Response = String;
    type Error = CreateUser;

    async fn handle(&self, req: String) -> Result<String, CreateUser> {
        if req == "alice@x" {
            Err(CreateUser::EmailTaken { email: req })
        } else {
            Ok(format!("ok:{req}"))
        }
    }
}

#[tokio::test]
async fn handler_ok_passes_through() {
    let v = CreateUserHandler.call_tonic("bob".into()).await.unwrap();
    assert_eq!(v, "ok:bob");
}

#[tokio::test]
async fn handler_error_becomes_status_with_envelope() {
    let st = CreateUserHandler
        .call_tonic("alice@x".into())
        .await
        .unwrap_err();
    assert_eq!(st.code(), tonic::Code::AlreadyExists);
    // At Public exposure, frames are dropped but the typed envelope still ships.
    let sf: ServiceFailure<CreateUser> = st.into_aerro::<CreateUser>().unwrap();
    match sf.into_inner() {
        CreateUser::EmailTaken { email } => assert_eq!(email, "alice@x"),
    }
}
