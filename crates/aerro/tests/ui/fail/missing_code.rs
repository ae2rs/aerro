use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(category = "business")]
    NoCode,
}

fn main() {}
