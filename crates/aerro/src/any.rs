//! Error chain rendering utility.

/// Render an error and its `source()` chain into `"msg: src1: src2: ..."` form.
pub fn render_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut out = err.to_string();
    let mut cur = err.source();
    while let Some(s) = cur {
        out.push_str(": ");
        out.push_str(&s.to_string());
        cur = s.source();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct E(&'static str, Option<Box<E>>);

    impl fmt::Display for E {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for E {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.1
                .as_deref()
                .map(|b| b as &(dyn std::error::Error + 'static))
        }
    }

    #[test]
    fn chain_renders_all_levels() {
        let e = E(
            "top",
            Some(Box::new(E("mid", Some(Box::new(E("leaf", None)))))),
        );
        assert_eq!(render_chain(&e), "top: mid: leaf");
    }
}
