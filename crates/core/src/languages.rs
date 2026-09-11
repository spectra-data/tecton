use tantivy::tokenizer::Language;
use whichlang::Lang;

const SUPPORTED_LANGUAGES: &[&'static str] = &[
    "ar", "da", "nl", "en", "fi", "fr", "de", "el", "hu", "it", "no", "pt", "ro", "ru", "es", "sv",
    "ta", "tr", // not supported by Language "zh", "ja", "ko",
];

/// return true if the langiuage is supported  base 2 char lang
pub fn is_language_supported(lang: &str) -> bool {
    SUPPORTED_LANGUAGES.iter().any(|&l| l == lang)
}

/// get Tantivy Language from 2 char lang code
pub fn get_language_for_lang(lang: &str) -> Language {
    match lang {
        "ar" => Language::Arabic,
        "da" => Language::Danish,
        "nl" => Language::Dutch,
        "en" => Language::English,
        "fi" => Language::Finnish,
        "fr" => Language::French,
        "de" => Language::German,
        "el" => Language::Greek,
        "hu" => Language::Hungarian,
        "it" => Language::Italian,
        "no" => Language::Norwegian,
        "pt" => Language::Portuguese,
        "ro" => Language::Romanian,
        "ru" => Language::Russian,
        "es" => Language::Spanish,
        "sv" => Language::Swedish,
        "ta" => Language::Tamil,
        "tr" => Language::Turkish,
        _ => Language::English,
    }
}

/// get the 2 char lang from text via `whichlang `
pub fn whichlang_lang(text: &str) -> &'static str {
    lang_two_letter_code(whichlang::detect_language(text))
}

/// Get 2 letter code from the `whichlang::Lang` value
/// If no match "en" will be returned
pub fn lang_two_letter_code(lang: Lang) -> &'static str {
    match lang {
        Lang::Ara => "ar",

        //Lang::Cmn => "cmn",
        Lang::Deu => "de",
        Lang::Eng => "en",
        Lang::Fra => "fr",
        //Lang::Hin => "hin",
        Lang::Ita => "it",
        //Lang::Jpn => "jpn",
        //Lang::Kor => "kor",
        Lang::Nld => "nl",
        Lang::Por => "pt",
        Lang::Rus => "ru",
        Lang::Spa => "es",
        Lang::Swe => "sv",
        Lang::Tur => "tr",
        //Lang::Vie => "vie",
        _ => "en",
    }
}
