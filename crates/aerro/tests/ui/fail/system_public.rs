use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(category = "system", code = "internal", exposure = "public")]
    Bad,
}

fn main() {}
