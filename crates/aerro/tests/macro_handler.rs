//! `#[aerro::handler]` adapter test.

#![cfg(all(feature = "tonic", feature = "macro"))]

use aerro::ServiceFailure;
use aerro::StatusIntoResultExt;

#[aerro::operation]
pub enum CreateUser {
    #[aerro(category = "business", code = "already_exists", error = "email already taken: {email}")]
    EmailTaken { email: String },
}

#[aerro::handler(service = "create-user", rpc = "create", exposure = "public", max_frames = 8)]
async fn create_user(req: String) -> Result<String, CreateUser> {
    if req == "alice@x" {
        Err(CreateUser::EmailTaken { email: req })
    } else {
        Ok(format!("ok:{req}"))
    }
}

#[tokio::test]
async fn handler_ok_passes_through() {
    let v = create_user("bob".into()).await.unwrap();
    assert_eq!(v, "ok:bob");
}

#[tokio::test]
async fn handler_error_becomes_status_with_envelope() {
    let st = create_user("alice@x".into()).await.unwrap_err();
    assert_eq!(st.code(), tonic::Code::AlreadyExists);
    // At Public exposure, frames are dropped but the typed envelope still ships.
    let sf: ServiceFailure<CreateUser> = st.into_aerro::<CreateUser>().unwrap();
    match sf.inner {
        CreateUser::EmailTaken { email } => assert_eq!(email, "alice@x"),
    }
}
