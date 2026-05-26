use aerro;

#[derive(Debug, aerro::Aerro)]
pub enum Bad {
    #[aerro(code = System::Internal)]
    Broken(
        #[aerro(forward)] String,
        String,
    ),
}

fn main() {}
