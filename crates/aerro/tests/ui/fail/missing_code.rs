use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(category = Business)]
    NoCode,
}

fn main() {}
