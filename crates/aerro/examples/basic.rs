//! One enum, one round-trip across the wire — the simplest possible aerro usage.

use aerro::wire::encode::EncodeOptions;
use aerro::{Aerro, IntoStatus, StatusIntoResultExt};

#[derive(Debug, aerro::Aerro)]
pub enum CreateUser {
    #[aerro(
        category = Business,
        code = AlreadyExists,
        error = "email already taken: {email}"
    )]
    EmailTaken { email: String },

    #[aerro(category = System, code = Internal, error = "create_user.boom")]
    Boom,
}

fn main() {
    // Server side: a typed failure.
    let err = CreateUser::EmailTaken {
        email: "alice@example.com".into(),
    };
    let status = err.into_status(&EncodeOptions::default());
    println!(
        "server emitted: code={:?} message={:?}",
        status.code(),
        status.message()
    );
    println!("details() length: {} bytes", status.details().len());

    // Client side: recover the typed variant.
    let recovered = status.into_aerro::<CreateUser>().unwrap();
    match recovered.into_inner() {
        CreateUser::EmailTaken { email } => println!("client recovered: email={email}"),
        CreateUser::Boom => unreachable!(),
    }

    println!("type_ids known to CreateUser: {:?}", CreateUser::TYPE_IDS);
}
