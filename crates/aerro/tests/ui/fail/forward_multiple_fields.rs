use aerro;

#[derive(Debug, aerro::Aerro)]
pub enum Bad {
    #[aerro(category = System, code = Internal)]
    Broken(
        #[aerro(forward)] String,
        String,
    ),
}

fn main() {}
