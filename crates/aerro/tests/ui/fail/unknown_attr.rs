use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(category = Business, code = NotFound, nonsense = "x")]
    Bad,
}

fn main() {}
