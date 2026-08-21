use candi_theme::{BUILTIN_NAMES, Color, builtin, parse};

fn theme_with(color_line: &str) -> String {
    format!("name: Probe\n{color_line}")
}

#[test]
fn parses_full_document() {
    let src = "\
name: Night
page_bg: \"#101010\"
page_fg: \"#EEEEEE\"
ui_bg: \"#202020\"
panel_bg: \"#2A2A2A\"
ui_fg: \"#DDDDDD\"
accent: \"#FF8800\"
selection: \"#FF880066\"
search: \"#FBBF24\"
";
    let t = parse(src).expect("valid document");
    assert_eq!(t.name, "Night");
    assert_eq!(t.page_bg, Color::from([0x10, 0x10, 0x10, 0xFF]));
    assert_eq!(t.page_fg, Color::from([0xEE, 0xEE, 0xEE, 0xFF]));
    assert_eq!(t.ui_bg, Color::from([0x20, 0x20, 0x20, 0xFF]));
    assert_eq!(t.panel_bg, Color::from([0x2A, 0x2A, 0x2A, 0xFF]));
    assert_eq!(t.ui_fg, Color::from([0xDD, 0xDD, 0xDD, 0xFF]));
    assert_eq!(t.accent, Color::from([0xFF, 0x88, 0x00, 0xFF]));
    assert_eq!(t.selection, Color::from([0xFF, 0x88, 0x00, 0x66]));
    assert_eq!(t.search, Color::from([0xFB, 0xBF, 0x24, 0xFF]));
}

#[test]
fn missing_fields_default_to_light_palette() {
    let t = parse("name: Minimal").expect("name-only document");
    let light = builtin("Light").expect("Light exists");
    assert_eq!(t.page_bg, light.page_bg);
    assert_eq!(t.page_fg, light.page_fg);
    assert_eq!(t.ui_bg, light.ui_bg);
    assert_eq!(t.panel_bg, light.panel_bg);
    assert_eq!(t.ui_fg, light.ui_fg);
    assert_eq!(t.accent, light.accent);
    assert_eq!(t.selection, light.selection);
    assert_eq!(t.search, light.search);
}

#[test]
fn missing_name_is_a_schema_error() {
    let err = parse("page_bg: \"#101010\"").expect_err("name is required");
    assert!(matches!(err, candi_theme::ThemeError::Schema(_)), "{err}");
    assert!(err.to_string().contains("missing field `name`"), "{err}");
}

#[test]
fn unknown_key_is_rejected() {
    let err = parse(&theme_with("font_size: 14")).expect_err("unknown key");
    assert!(matches!(err, candi_theme::ThemeError::Schema(_)), "{err}");
    assert!(err.to_string().contains("unknown field"), "{err}");
}

#[test]
fn malformed_yaml_is_reported_as_such() {
    let err = parse("name: [unclosed").expect_err("broken YAML");
    assert!(matches!(err, candi_theme::ThemeError::Yaml(_)), "{err}");
}

#[test]
fn bad_hex_is_rejected() {
    for bad in ["\"#12345\"", "\"FFFFFF\"", "\"#GGGGGG\"", "\"\"", "5"] {
        let src = theme_with(&format!("page_bg: {bad}"));
        let err = parse(&src).expect_err(bad);
        assert!(matches!(err, candi_theme::ThemeError::Schema(_)), "{err}");
    }
}

#[test]
fn six_digit_hex_gets_full_alpha() {
    let t = parse(&theme_with("page_fg: \"#3B3228\"")).expect("valid");
    assert_eq!(t.page_fg.a(), 255);
}

#[test]
fn display_roundtrips_through_parse() {
    for rgba in [
        [0xFF, 0xFF, 0xFF, 0xFF],
        [0x1A, 0x1A, 0x1A, 0xFF],
        [0x25, 0x63, 0xEB, 0x40],
        [0x00, 0x00, 0x00, 0x00],
    ] {
        let c = Color::from(rgba);
        let t = parse(&theme_with(&format!("selection: \"{c}\""))).expect("roundtrip");
        assert_eq!(t.selection, c, "{c}");
    }
}

#[test]
fn display_omits_opaque_alpha() {
    assert_eq!(Color::from([0x25, 0x63, 0xEB, 0xFF]).to_string(), "#2563EB");
    assert_eq!(
        Color::from([0x25, 0x63, 0xEB, 0x40]).to_string(),
        "#2563EB40"
    );
}

#[test]
fn builtin_names_are_stable() {
    assert_eq!(
        BUILTIN_NAMES,
        ["Light", "Sepia", "Warm Dark", "Dark", "True Dark"]
    );
}

#[test]
fn every_builtin_parses_and_knows_its_name() {
    for name in BUILTIN_NAMES {
        let t = builtin(name).unwrap_or_else(|| panic!("{name} must exist"));
        assert_eq!(t.name, name);
    }
}

#[test]
fn builtin_miss_is_none() {
    assert!(builtin("nope").is_none());
    assert!(builtin("light").is_none());
}

#[test]
fn builtin_page_palettes_match_the_design() {
    assert_eq!(builtin("Sepia").unwrap().page_bg.to_string(), "#F4ECD8");
    assert_eq!(builtin("Warm Dark").unwrap().page_fg.to_string(), "#E8E0D4");
    assert_eq!(builtin("Dark").unwrap().page_bg.to_string(), "#16181D");
    assert_eq!(builtin("True Dark").unwrap().page_bg.to_string(), "#000000");
}
