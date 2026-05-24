//! Same typed error encoded at three exposure tiers; observe what each tier
//! ships to its audience.

use aerro::{Exposure, IntoStatus};
use aerro::wire::encode::EncodeOptions;

#[aerro::operation]
pub enum Db {
    #[aerro(category = "system", code = "internal", error = "db.unreachable: {host}")]
    Unreachable { host: String },
}

fn show(label: &str, exposure: Exposure) {
    let err = Db::Unreachable {
        host: "prod-shard-42.internal".into(),
    };
    let st = err.into_status(&EncodeOptions {
        exposure,
        max_frames: 16,
    });
    println!(
        "{label} (Exposure::{:?}): code={:?} message={:?} details_bytes={}",
        exposure,
        st.code(),
        st.message(),
        st.details().len()
    );
}

fn main() {
    show("Internal", Exposure::Internal);
    show("Trusted ", Exposure::Trusted);
    show("Public  ", Exposure::Public);
}
