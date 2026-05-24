//! 3-hop trace accumulation: backend (B) → middleware (M) → gateway (G).
//! Each hop appends a frame; the final `RemoteError` shows the full chain.

use aerro::{Category, Frame, IntoStatus, ServiceFailure, StatusIntoResultExt};
use aerro::wire::encode::EncodeOptions;
use tonic::Code;

#[aerro::operation]
pub enum Pipeline {
    #[aerro(category = "system", code = "internal", error = "backend.unreachable")]
    Unreachable,
}

fn backend() -> Result<(), Pipeline> {
    Err(Pipeline::Unreachable)
}

fn middleware() -> Result<(), tonic::Status> {
    match backend() {
        Ok(v) => Ok(v),
        Err(e) => {
            let mut sf: ServiceFailure<Pipeline> = e.into();
            sf.frames.push(Frame::local(
                "backend",
                "ping",
                Code::Internal,
                "backend.unreachable",
                Category::System,
            ));
            Err(sf.into_status(&EncodeOptions::default()))
        }
    }
}

fn gateway() -> Result<(), tonic::Status> {
    match middleware() {
        Ok(v) => Ok(v),
        Err(st) => {
            let mut sf = st.into_aerro::<Pipeline>().expect("typed");
            sf.frames.push(Frame::local(
                "middleware",
                "forward",
                Code::Internal,
                "wrapped",
                Category::System,
            ));
            Err(sf.into_status(&EncodeOptions::default()))
        }
    }
}

fn main() {
    let st = gateway().unwrap_err();
    let sf = st.into_aerro::<Pipeline>().expect("typed at gateway");
    let mut sf = sf;
    sf.frames.push(Frame::local(
        "gateway",
        "handle",
        Code::Internal,
        "client-side",
        Category::System,
    ));
    println!("frames on final hop:");
    for (i, f) in sf.frames.iter().enumerate() {
        println!(
            "  {i}: service={} rpc={} code={:?} msg={}",
            f.service, f.rpc, f.code, f.message
        );
    }
    println!("trace_id={:02x?}", sf.trace.trace_id);
}
