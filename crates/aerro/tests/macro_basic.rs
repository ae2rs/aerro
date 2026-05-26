//! End-to-end test of `#[derive(aerro::Aerro)]` — verifies the derive produces
//! a working `Aerro` impl that round-trips via the wire layer.

#![cfg(feature = "macro")]

use aerro::wire::encode::EncodeOptions;
use aerro::{Aerro, AerroEncode, Category, Exposure, ServiceFailure};
use tonic::Code;

#[derive(Debug, aerro::Aerro)]
pub enum CreateUser {
    #[aerro(
        code = Business::AlreadyExists,
        error = "email already taken: {email}"
    )]
    EmailTaken { email: String },

    #[aerro(
        code = Validation::InvalidArgument,
        error = "invalid name: {0}"
    )]
    InvalidName(String),

    #[aerro(code = System::Internal, error = "create_user.boom")]
    Boom,
}

#[test]
fn type_ids_listed() {
    assert_eq!(
        CreateUser::TYPE_IDS,
        &[
            "create_user.email_taken",
            "create_user.invalid_name",
            "create_user.boom",
        ]
    );
}

#[test]
fn type_id_per_variant() {
    assert_eq!(
        Aerro::type_id(&CreateUser::EmailTaken { email: "x".into() }),
        "create_user.email_taken"
    );
    assert_eq!(
        Aerro::type_id(&CreateUser::InvalidName("nope".into())),
        "create_user.invalid_name"
    );
    assert_eq!(Aerro::type_id(&CreateUser::Boom), "create_user.boom");
}

#[test]
fn category_and_code_dispatch() {
    let e = CreateUser::EmailTaken {
        email: "a@b".into(),
    };
    assert_eq!(e.category(), Category::Business);
    assert_eq!(e.code(), Code::AlreadyExists);
    assert_eq!(e.exposure(), Exposure::Public);

    let s = CreateUser::Boom;
    assert_eq!(s.category(), Category::System);
    assert_eq!(s.code(), Code::Internal);
    assert_eq!(s.exposure(), Exposure::Internal);
}

#[test]
fn display_renders_format_string() {
    assert_eq!(
        CreateUser::EmailTaken {
            email: "a@b".into()
        }
        .to_string(),
        "email already taken: a@b"
    );
    assert_eq!(
        CreateUser::InvalidName("@@@".into()).to_string(),
        "invalid name: @@@"
    );
}

#[test]
fn struct_variant_roundtrips_via_wire() {
    let st = CreateUser::EmailTaken {
        email: "alice@x".into(),
    }
    .encode(&EncodeOptions::default());
    let sf: ServiceFailure<CreateUser> = ServiceFailure::try_from(st).unwrap();
    match sf.into_inner() {
        CreateUser::EmailTaken { email } => assert_eq!(email, "alice@x"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn tuple_variant_roundtrips_via_wire() {
    let st =
        CreateUser::InvalidName("bob".into()).encode(&EncodeOptions::default());
    let sf: ServiceFailure<CreateUser> = ServiceFailure::try_from(st).unwrap();
    match sf.into_inner() {
        CreateUser::InvalidName(s) => assert_eq!(s, "bob"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn unit_variant_roundtrips_via_wire() {
    let st = CreateUser::Boom.encode(&EncodeOptions::default());
    let sf: ServiceFailure<CreateUser> = ServiceFailure::try_from(st).unwrap();
    assert!(matches!(sf.inner(), CreateUser::Boom));
}
