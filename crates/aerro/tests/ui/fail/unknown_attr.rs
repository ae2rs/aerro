use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(code = Business::NotFound, nonsense = "x")]
    Bad,
}

fn main() {}
