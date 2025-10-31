use alphabet_detector::UcdScript;
use strum::IntoEnumIterator;

#[test]
fn count_single_language_scripts() {
    let mut single = 0;
    for script in UcdScript::iter() {
        if alphabet_detector::script_char_to_slangs(script, char::default()).len() == 1 {
            single += 1;
        }
    }
    assert_eq!(
        single, 130,
        "Change single language scripts count in docs to {single}"
    );
}
