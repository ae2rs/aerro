//! One enum, one round-trip across the wire — the simplest possible aerro usage.

use aerro::{Aerro, AerroEncode, ServiceFailure};

#[derive(Debug, aerro::Aerro)]
pub enum CreateUserError {
    #[aerro(
        code = Business::AlreadyExists,
        error = "email already taken: {email}"
    )]
    EmailTaken { email: String },

    #[aerro(code = System::Internal)]
    Boom,
}

fn main() {
    // Server side: a typed failure (explicit options).
    let err = CreateUserError::EmailTaken {
        email: "alice@example.com".into(),
    };
    let status = err.encode();
    println!(
        "server emitted: code={:?} message={:?}",
        status.code(),
        status.message()
    );
    println!("details() length: {} bytes", status.details().len());

    // Server side: same thing with default options.
    let err2 = CreateUserError::Boom;
    let status2 = err2.encode();
    println!(
        "server emitted (default): code={:?} message={:?}",
        status2.code(),
        status2.message()
    );

    // Client side: recover the typed variant.
    let recovered = ServiceFailure::<CreateUserError>::try_from(status).unwrap();
    match recovered.into_inner() {
        CreateUserError::EmailTaken { email } => println!("client recovered: email={email}"),
        CreateUserError::Boom => unreachable!(),
    }

    println!(
        "type_ids known to CreateUserError: {:?}",
        CreateUserError::TYPE_IDS
    );
}
