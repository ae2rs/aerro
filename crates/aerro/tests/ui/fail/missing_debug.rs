#[derive(aerro::Aerro)]
pub enum Foo {
    #[aerro(category = Business, code = NotFound, error = "not found")]
    NotFound,
}

fn main() {}
