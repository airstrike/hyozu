use hyozu::design::Design;
use hyozu::palette::{Palette, Resolved};
use iced::Theme;

/// Comprehensive test ensuring all themes provide acceptable contrast (≥ 2.0)
/// for bar labels against their bar colors.
#[test]
fn test_all_themes_bar_labels_comprehensive() {
    let themes = [
        Theme::CatppuccinLatte,
        Theme::CatppuccinFrappe,
        Theme::CatppuccinMacchiato,
        Theme::CatppuccinMocha,
        Theme::Dark,
        Theme::Dracula,
        Theme::Ferra,
        Theme::GruvboxDark,
        Theme::GruvboxLight,
        Theme::KanagawaDragon,
        Theme::KanagawaLotus,
        Theme::KanagawaWave,
        Theme::Moonfly,
        Theme::Nightfly,
        Theme::Nord,
        Theme::Oxocarbon,
        Theme::SolarizedDark,
        Theme::SolarizedLight,
        Theme::TokyoNight,
        Theme::TokyoNightLight,
        Theme::TokyoNightStorm,
    ];

    let mut failures = Vec::new();

    for theme in &themes {
        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = Design::seed(&theme);
        let palette = Resolved::resolve(&Palette::Categorical, &seed, 5);

        for (i, color_spec) in palette.colors().iter().enumerate().take(5) {
            let bar_color = color_spec.resolve(background, text_pair, &seed, None);
            let label_color = text_pair.resolve(bar_color, Some(background));
            let contrast = bar_color.relative_contrast(label_color);

            if contrast < 2.0 {
                failures.push(format!(
                    "{:?} color {}: contrast {:.2} (bar lum {:.3}, label lum {:.3})",
                    theme,
                    i,
                    contrast,
                    bar_color.relative_luminance(),
                    label_color.relative_luminance()
                ));
            }
        }
    }

    if !failures.is_empty() {
        println!("\n=== CONTRAST FAILURES ===");
        for failure in &failures {
            println!("❌ {}", failure);
        }
        panic!("\n{} theme/color combinations have poor contrast", failures.len());
    }
}

/// Tests that all themes meet WCAG AA standard (4.5:1) for text on background.
#[test]
fn test_all_themes_have_readable_text() {
    let themes = [
        Theme::CatppuccinLatte,
        Theme::CatppuccinFrappe,
        Theme::CatppuccinMacchiato,
        Theme::CatppuccinMocha,
        Theme::Dark,
        Theme::Dracula,
        Theme::Ferra,
        Theme::GruvboxDark,
        Theme::GruvboxLight,
        Theme::KanagawaDragon,
        Theme::KanagawaLotus,
        Theme::KanagawaWave,
        Theme::Moonfly,
        Theme::Nightfly,
        Theme::Nord,
        Theme::Oxocarbon,
        Theme::SolarizedDark,
        Theme::SolarizedLight,
        Theme::TokyoNight,
        Theme::TokyoNightLight,
        Theme::TokyoNightStorm,
    ];

    for theme in &themes {
        let background = theme.background_color();
        let text_pair = theme.text_pair();

        let resolved_text = text_pair.resolve(background, None);
        let contrast = background.relative_contrast(resolved_text);

        assert!(
            contrast >= 4.5,
            "Theme {:?}: text contrast {} is below WCAG AA standard (4.5:1). \
             Background luminance: {}, Text luminance: {}",
            theme,
            contrast,
            background.relative_luminance(),
            resolved_text.relative_luminance()
        );
    }
}

/// Debug test for Dark theme - verifies light text is preferred when contrast is similar.
#[test]
fn test_dark_theme_debug() {
    let theme = Theme::Dark;
    let background = theme.background_color();
    let text_pair = theme.text_pair();
    let seed = Design::seed(&theme);
    let palette = Resolved::resolve(&Palette::Categorical, &seed, 3);

    println!("\n=== Theme::Dark Debug ===");
    println!("Background: lum={:.3}", background.relative_luminance());
    println!("text_pair.on_light: lum={:.3}", text_pair.on_light.relative_luminance());
    println!("text_pair.on_dark: lum={:.3}", text_pair.on_dark.relative_luminance());

    for (i, color_spec) in palette.colors().iter().enumerate().take(3) {
        let bar_color = color_spec.resolve(background, text_pair, &seed, None);
        let label_color = text_pair.resolve(bar_color, Some(background));
        let contrast = bar_color.relative_contrast(label_color);

        let bg_contrast = bar_color.relative_contrast(background);
        let opt1_contrast = bar_color.relative_contrast(text_pair.on_light);
        let opt2_contrast = bar_color.relative_contrast(text_pair.on_dark);

        println!(
            "\nColor {}: bar lum={:.3}, label lum={:.3}, contrast={:.2}",
            i,
            bar_color.relative_luminance(),
            label_color.relative_luminance(),
            contrast
        );
        println!(
            "  (bg={:.2}, opt1={:.2}, opt2={:.2})",
            bg_contrast, opt1_contrast, opt2_contrast
        );

        assert!(contrast >= 2.0);
    }
}

/// Debug test for Solarized Light - edge case with medium-luminance inverted colors.
#[test]
fn test_solarized_light_debug() {
    let theme = Theme::SolarizedLight;
    let background = theme.background_color();
    let text_pair = theme.text_pair();
    let seed = Design::seed(&theme);
    let palette = Resolved::resolve(&Palette::Categorical, &seed, 5);

    println!("\n=== Solarized Light Debug ===");
    println!("Background: lum={:.3}", background.relative_luminance());
    println!("text_pair.on_light: lum={:.3}", text_pair.on_light.relative_luminance());
    println!("text_pair.on_dark: lum={:.3}", text_pair.on_dark.relative_luminance());

    for (i, color_spec) in palette.colors().iter().enumerate().take(5) {
        let bar_color = color_spec.resolve(background, text_pair, &seed, None);
        let label_color = text_pair.resolve(bar_color, Some(background));
        let contrast = bar_color.relative_contrast(label_color);

        let bg_contrast = bar_color.relative_contrast(background);
        let opt1_contrast = bar_color.relative_contrast(text_pair.on_light);
        let opt2_contrast = bar_color.relative_contrast(text_pair.on_dark);

        println!(
            "\nColor {}: bar lum={:.3}, label lum={:.3}, contrast={:.2}",
            i,
            bar_color.relative_luminance(),
            label_color.relative_luminance(),
            contrast
        );
        println!(
            "  (bg={:.2}, opt1={:.2}, opt2={:.2})",
            bg_contrast, opt1_contrast, opt2_contrast
        );

        assert!(contrast >= 2.0, "Color {}: contrast {} too low", i, contrast);
    }
}

/// Debug test for GruvboxLight - verifies background color is preferred when appropriate.
#[test]
fn test_gruvbox_light_debug() {
    let theme = Theme::GruvboxLight;
    let background = theme.background_color();
    let text_pair = theme.text_pair();
    let seed = Design::seed(&theme);
    let palette = Resolved::resolve(&Palette::Categorical, &seed, 5);

    println!("\n=== GruvboxLight Debug ===");
    println!("Background: lum={:.3}", background.relative_luminance());
    println!("text_pair.on_light: lum={:.3}", text_pair.on_light.relative_luminance());
    println!("text_pair.on_dark: lum={:.3}", text_pair.on_dark.relative_luminance());

    for (i, color_spec) in palette.colors().iter().enumerate().take(5) {
        let bar_color = color_spec.resolve(background, text_pair, &seed, None);
        let label_color = text_pair.resolve(bar_color, Some(background));
        let contrast = bar_color.relative_contrast(label_color);

        let bg_contrast = bar_color.relative_contrast(background);
        let opt1_contrast = bar_color.relative_contrast(text_pair.on_light);
        let opt2_contrast = bar_color.relative_contrast(text_pair.on_dark);

        println!(
            "\nColor {}: bar lum={:.3}, label lum={:.3}, contrast={:.2}",
            i,
            bar_color.relative_luminance(),
            label_color.relative_luminance(),
            contrast
        );
        println!(
            "  (bg={:.2}, opt1={:.2}, opt2={:.2})",
            bg_contrast, opt1_contrast, opt2_contrast
        );

        assert!(contrast >= 2.0, "Color {}: contrast {} too low", i, contrast);
    }
}
