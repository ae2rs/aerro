use aerro;

#[aerro::operation]
pub enum E {
    #[aerro(category = "system", code = "internal", exposure = "public")]
    Bad,
}

fn main() {}
