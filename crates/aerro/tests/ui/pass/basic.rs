use aerro;

#[derive(Debug, aerro::Aerro)]
pub enum E {
    #[aerro(code = Business::NotFound, error = "x not found")]
    NotFound,

    #[aerro(code = Validation::InvalidArgument)]
    Bad(String),

    #[aerro(code = System::Internal)]
    Boom,
}

fn main() {
    let _ = E::NotFound;
}
