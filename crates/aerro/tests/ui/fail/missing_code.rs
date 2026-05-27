use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(exposure = Internal)]
    NoCode,
}

fn main() {}
