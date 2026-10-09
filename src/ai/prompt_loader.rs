use std::fs;

pub fn load(
    name: &str
) -> anyhow::Result<String> {

    Ok(
        fs::read_to_string(
            format!("prompts/{name}")
        )?
    )
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn loads_prompt() {

        let prompt =
            load("inci_review.txt")
                .unwrap();

        assert!(
            !prompt.is_empty()
        );
    }
}