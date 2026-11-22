use super::*;

pub(super) fn install_font_fallback(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/LanguageSupport/PingFang.ttc",
        "/system/fonts/NotoSansCJK-Regular.ttc",
        "/system/fonts/NotoSansCJK-VF.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/local/share/fonts/noto/NotoSansCJK-Regular.ttc",
        "/usr/local/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "C:\\Windows\\Fonts\\msyh.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "system-cjk".into(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("system-cjk".into());
            ctx.set_fonts(fonts);
            return;
        }
    }
}

pub(super) struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub navigation: Color32,
    pub edge: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub hero: Color32,
}

pub(super) fn palette(ui: &egui::Ui) -> Palette {
    if ui.visuals().dark_mode {
        Palette {
            background: Color32::from_rgb(16, 23, 36),
            surface: Color32::from_rgb(25, 35, 52),
            navigation: Color32::from_rgb(20, 29, 44),
            edge: Color32::from_rgb(49, 64, 87),
            ink: Color32::from_rgb(235, 241, 255),
            muted: Color32::from_rgb(166, 181, 206),
            accent: Color32::from_rgb(157, 174, 255),
            hero: Color32::from_rgb(34, 49, 83),
        }
    } else {
        Palette {
            background: Color32::from_rgb(244, 247, 252),
            surface: Color32::WHITE,
            navigation: Color32::from_rgb(233, 239, 248),
            edge: Color32::from_rgb(220, 228, 241),
            ink: Color32::from_rgb(25, 38, 65),
            muted: Color32::from_rgb(81, 99, 126),
            accent: Color32::from_rgb(67, 84, 195),
            hero: Color32::from_rgb(25, 38, 65),
        }
    }
}

pub(super) fn label(value: &str) -> &'static str {
    match value {
        "light" => "Light",
        "dark" => "Dark",
        _ => "Follow system",
    }
}

pub(super) fn apply(ctx: &egui::Context, value: &str) {
    ctx.set_theme(match value {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    });
}

pub(super) fn configure(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
        style.spacing.item_spacing = egui::vec2(9.0, 8.0);
        style.spacing.button_padding = egui::vec2(11.0, 6.0);
        let dark = style.visuals.dark_mode;
        style.visuals.selection.bg_fill = if dark {
            Color32::from_rgb(42, 54, 89)
        } else {
            Color32::from_rgb(217, 227, 251)
        };
        style.visuals.selection.stroke.color = if dark {
            Color32::from_rgb(220, 229, 255)
        } else {
            Color32::from_rgb(38, 56, 143)
        };
    });
}
