use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(code = NotFound)]
    Bad,
}

fn main() {}
