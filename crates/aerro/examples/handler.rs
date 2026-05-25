//! `#[derive(aerro::AerroHandler)]` — typed RPC handler that converts errors to `tonic::Status`.
//!
//! Define a unit struct, derive `AerroHandler` with service metadata, then implement
//! `Handler` with the actual logic. Calling `call_tonic` wraps the typed error into a
//! `tonic::Status` with an inline frame and the correct exposure redaction applied.

#![cfg(feature = "macro")]

use aerro::{AerroHandler, Handler, ServiceFailure, StatusIntoResultExt};

#[derive(Debug, aerro::Aerro)]
pub enum CreateUser {
    #[aerro(
        category = Business,
        code = AlreadyExists,
        error = "email already taken: {email}"
    )]
    EmailTaken { email: String },

    #[aerro(category = System, code = Internal)]
    Db,
}

#[derive(aerro::AerroHandler)]
#[aerro(service = "users", rpc = "create_user", exposure = Public, max_frames = 4)]
struct CreateUserHandler;

impl Handler for CreateUserHandler {
    type Request = String;
    type Response = String;
    type Error = CreateUser;

    async fn handle(&self, email: String) -> Result<String, CreateUser> {
        if email == "taken@example.com" {
            Err(CreateUser::EmailTaken { email })
        } else {
            Ok(format!("created user with email {email}"))
        }
    }
}

#[tokio::main]
async fn main() {
    println!(
        "handler metadata: service={} rpc={} exposure={:?} max_frames={}",
        CreateUserHandler::SERVICE,
        CreateUserHandler::RPC,
        CreateUserHandler::EXPOSURE,
        CreateUserHandler::MAX_FRAMES,
    );

    // Happy path — call_tonic passes the Ok value through unchanged.
    let ok = CreateUserHandler
        .call_tonic("alice@example.com".into())
        .await
        .unwrap();
    println!("ok: {ok}");

    // Error path — typed error is encoded into tonic::Status with a call frame.
    let status = CreateUserHandler
        .call_tonic("taken@example.com".into())
        .await
        .unwrap_err();
    println!(
        "error status: code={:?} message={:?}",
        status.code(),
        status.message()
    );

    // The client side can recover the typed error back out.
    let sf: ServiceFailure<CreateUser> = status.into_aerro::<CreateUser>().unwrap();
    match sf.into_inner() {
        CreateUser::EmailTaken { email } => println!("recovered: email={email}"),
        CreateUser::Db => unreachable!(),
    }
}
