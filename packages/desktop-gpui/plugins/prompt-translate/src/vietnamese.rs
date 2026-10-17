//! `hasVietnamese` of `shared/vietnamese.ts`.

/// The letter set of `vietnamese.ts:3`, lowercase. Any Vietnamese diacritic counts; words like
/// "là" share letters with French, but requiring Vietnamese-only letters would miss short
/// prompts such as "cái này là gì".
const LETTERS: &str = "àáảãạăằắẳẵặâầấẩẫậđèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵ";

/// True when `text` has a letter of the set, in either case.
pub(crate) fn has_vietnamese(text: &str) -> bool {
    text.chars()
        .any(|ch| ch.to_lowercase().any(|lower| LETTERS.contains(lower)))
}

#[cfg(test)]
mod tests {
    use super::has_vietnamese;

    #[test]
    fn has_vietnamese_detects_diacritics_including_words_shared_with_other_languages() {
        assert!(has_vietnamese("cái này là gì"));
        assert!(has_vietnamese("ĐỔI TÊN HÀM"));
        assert!(!has_vietnamese("rename the function"));
        assert!(!has_vietnamese("fix src/app.ts:12 and run npm test"));
        assert!(!has_vietnamese("hello"));
        assert!(has_vietnamese("Đ"));
    }
}
