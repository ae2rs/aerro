use aerro;

#[aerro::operation]
pub enum E {
    #[aerro(category = "business")]
    NoCode,
}

fn main() {}
