//! Round-trip via the `compat-json` envelope. Run with:
//!     cargo run -p aerro --example compat --features compat-json

#[cfg(not(feature = "compat-json"))]
fn main() {
    eprintln!("This example requires the `compat-json` feature.");
    eprintln!("Run: cargo run -p aerro --example compat --features compat-json");
}

#[cfg(feature = "compat-json")]
fn main() {
    use aerro::ServiceFailure;
    use aerro::compat_json::{decode_json, encode_json};
    use aerro::wire::encode::EncodeOptions;

    #[aerro::operation]
    pub enum CreateUser {
        #[aerro(category = "business", code = "already_exists", error = "email already taken: {email}")]
        EmailTaken { email: String },
    }

    let sf: ServiceFailure<CreateUser> = CreateUser::EmailTaken {
        email: "alice@x".into(),
    }
    .into();
    let st = encode_json(&sf, &EncodeOptions::default());
    println!("JSON details: {}", std::str::from_utf8(st.details()).unwrap());
    let r = decode_json(&st).unwrap();
    println!("decoded type_id={} category={:?}", r.type_id, r.category);
}
