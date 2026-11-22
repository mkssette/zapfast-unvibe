use egui::text::LayoutJob;
use egui::{Pos2, Rect, TextFormat};

pub use fastframe_emoji::{PLACEHOLDER, only_emoji};

const BUNDLED: &[u8] = include_bytes!("../../assets/fonts/NotoColorEmoji.ttf");

/// The newest emoji version the bundled font draws in full (Noto Color
/// Emoji 2.057 has Emoji 18.0). Raise it with the font: a test checks that
/// the font draws every emoji up to it.
const BUNDLED_VERSION: emojis::EmojiVersion = emojis::EmojiVersion::new(18, 0);

/// Whether ZapFast offers this emoji in its pickers and completions. The
/// bundled font is asked last on every platform, so an emoji it draws has a
/// picture everywhere; one the `emojis` list knows before the font does
/// would show as an empty box, so it is left out.
pub fn offered(emoji: &emojis::Emoji) -> bool {
    emoji.emoji_version() <= BUNDLED_VERSION
}

/// Every emoji ZapFast offers, in the `emojis` list's order.
pub fn catalogue() -> impl Iterator<Item = &'static emojis::Emoji> + Clone {
    emojis::iter().filter(|emoji| offered(emoji))
}

/// The emoji `text` names, when ZapFast offers it: what a saved recent or
/// frequent emoji is checked against before a picker shows it.
pub fn lookup(text: &str) -> Option<&'static emojis::Emoji> {
    emojis::get(text).filter(|emoji| offered(emoji))
}

fn setup() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        fastframe_emoji::EmojiSetup::default()
            .system(true)
            .bundled(BUNDLED)
            .synchronous(cfg!(any(test, feature = "demo")))
            .install();
    });
}

pub fn available() -> bool {
    setup();
    fastframe_emoji::available()
}

pub fn warm_up() {
    setup();
    fastframe_emoji::warm_up();
}

pub fn plugin() -> fastframe_emoji::EmojiPlugin {
    setup();
    fastframe_emoji::EmojiPlugin::default()
}

pub fn prewarm<'a>(ctx: &egui::Context, clusters: impl IntoIterator<Item = &'a str>) {
    setup();
    fastframe_emoji::prewarm(ctx, clusters);
}

pub fn append(
    ui: &egui::Ui,
    job: &mut LayoutJob,
    placements: &mut Vec<String>,
    text: &str,
    format: &TextFormat,
) -> usize {
    setup();
    fastframe_emoji::append(ui, job, placements, text, format)
}

pub fn paint_cluster(ui: &egui::Ui, cluster: &str, rect: Rect) {
    setup();
    fastframe_emoji::paint_cluster(ui, cluster, rect);
}

pub fn editor_job(
    text: &str,
    format: &egui::TextFormat,
) -> (egui::text::LayoutJob, Vec<(usize, usize, String)>) {
    setup();
    fastframe_emoji::editor_job(text, format)
}

pub fn paint(ui: &egui::Ui, galley: &egui::Galley, origin: Pos2, placements: &[String]) {
    setup();
    fastframe_emoji::paint(ui, galley, origin, placements);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled() -> fastframe_emoji::Emoji {
        fastframe_emoji::EmojiSetup::default()
            .system(true)
            .bundled(BUNDLED)
            .synchronous(true)
            .load()
    }

    #[test]
    fn the_bundled_font_renders_colour_emoji() {
        let picture = bundled().render("\u{1F600}", 72).expect("grinning face");
        assert_eq!(picture.size[1], 72);
        assert!(picture.has_colour());
    }

    #[test]
    #[cfg_attr(windows, ignore = "font shaping fallback difference on Windows")]
    fn the_bundled_font_joins_sequences() {
        let emoji = bundled();
        for sequence in ["🇩🇪", "👍🏽", "👨‍👩‍👧", "#️⃣", "🏳️‍🌈"]
        {
            let joined = emoji.render(sequence, 72).expect("a picture");
            let first: String = sequence.chars().take(1).collect();
            assert_ne!(
                Some(joined),
                emoji.render(&first, 72),
                "{sequence} fell back to its first part"
            );
        }
    }

    #[test]
    fn the_bundled_font_draws_every_offered_emoji() {
        let emoji = bundled();
        let mut missing = Vec::new();
        for offered in catalogue() {
            let text = offered.as_str();
            let picture = emoji.render(text, 16);
            let parts = text.chars().filter(|c| *c != '\u{FE0F}').count();
            let first: String = text.chars().take(1).collect();
            if picture.is_none() || (parts > 1 && picture == emoji.render(&first, 16)) {
                missing.push(format!("{text} {}", offered.name()));
            }
        }
        assert!(missing.is_empty(), "the bundled font lacks {missing:?}");
    }

    #[test]
    fn the_newest_emoji_are_offered() {
        let newest = lookup("\u{1FAEB}").expect("an Emoji 18.0 face is offered");
        assert_eq!(newest.emoji_version(), BUNDLED_VERSION);
        assert!(lookup("not an emoji").is_none());
    }

    #[test]
    fn placeholders_line_up_with_placements() {
        let mut job = LayoutJob::default();
        let mut placements = Vec::new();
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            append(
                ui,
                &mut job,
                &mut placements,
                "a 😀 b",
                &TextFormat::default(),
            );
        });
        output.textures_delta.clear();
        assert!(available(), "the bundled font is always there");
        assert_eq!(placements, vec!["😀".to_owned()]);
        assert_eq!(job.text.matches(PLACEHOLDER).count(), 1);
    }

    #[test]
    fn an_editor_job_keeps_the_text_and_places_the_emoji() {
        let format =
            egui::TextFormat::simple(egui::FontId::proportional(14.0), egui::Color32::WHITE);
        let text = "hi 😊 and 👍🏽!";
        let (job, clusters) = editor_job(text, &format);
        assert_eq!(job.text, text, "the galley must mirror the buffer");
        assert_eq!(clusters.len(), 2);
        for (start, length, cluster) in &clusters {
            assert_eq!(
                text.chars().skip(*start).take(*length).collect::<String>(),
                *cluster
            );
        }
    }

    #[test]
    fn emoji_only_messages_are_counted() {
        assert_eq!(only_emoji("😂"), Some(1));
        assert_eq!(only_emoji("😂 🙏"), Some(2));
        assert_eq!(only_emoji("ok 😂"), None);
    }
}
