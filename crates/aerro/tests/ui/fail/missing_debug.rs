#[derive(aerro::Aerro)]
pub enum Foo {
    #[aerro(code = Business::NotFound, error = "not found")]
    NotFound,
}

fn main() {}
