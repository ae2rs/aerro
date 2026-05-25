use aerro;

#[derive(Debug, aerro::Aerro)]
pub enum E {
    #[aerro(category = "business", code = "not_found", error = "x not found")]
    NotFound,

    #[aerro(category = "validation", code = "invalid_argument")]
    Bad(String),

    #[aerro(category = "system", code = "internal")]
    Boom,
}

fn main() {
    let _ = E::NotFound;
}
