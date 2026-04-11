pub fn build_search_blob<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parts
        .into_iter()
        .flat_map(|part| {
            let normalized = part.as_ref().trim();

            if normalized.is_empty() || normalized == "0" || normalized == "-" {
                return Vec::new();
            }

            normalized
                .split_whitespace()
                .map(|segment| segment.trim().to_lowercase())
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::build_search_blob;

    #[test]
    fn build_search_blob_normalizes_case_spacing_and_placeholder_values() {
        let blob =
            build_search_blob(["  Song Title  ", "Artist Name", "-", "0", "Mad   Professor"]);

        assert_eq!(blob, "song title artist name mad professor");
    }
}
