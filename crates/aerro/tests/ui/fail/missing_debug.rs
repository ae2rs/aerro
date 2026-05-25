#[derive(aerro::Aerro)]
pub enum Foo {
    #[aerro(category = "business", code = "not_found", error = "not found")]
    NotFound,
}

fn main() {}
