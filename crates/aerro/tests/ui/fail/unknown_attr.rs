use aerro;

#[aerro::operation]
pub enum E {
    #[aerro(category = "business", code = "not_found", nonsense = "x")]
    Bad,
}

fn main() {}
