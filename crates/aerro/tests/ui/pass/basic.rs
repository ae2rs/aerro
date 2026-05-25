use aerro;

#[derive(Debug, aerro::Aerro)]
pub enum E {
    #[aerro(category = Business, code = NotFound, error = "x not found")]
    NotFound,

    #[aerro(category = Validation, code = InvalidArgument)]
    Bad(String),

    #[aerro(category = System, code = Internal)]
    Boom,
}

fn main() {
    let _ = E::NotFound;
}
