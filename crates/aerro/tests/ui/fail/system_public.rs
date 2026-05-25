use aerro;

#[derive(aerro::Aerro)]
pub enum E {
    #[aerro(category = System, code = Internal, exposure = Public)]
    Bad,
}

fn main() {}
