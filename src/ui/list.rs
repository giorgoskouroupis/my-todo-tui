use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::ListItem;

use crate::app::NoteDisplay;
use crate::data::{category_badge, Note, Priority, TodoItem};
use crate::date;

use super::theme::Theme;

fn priority_color(p: Priority, theme: &Theme) -> ratatui::style::Color {
    match p {
        Priority::Low => theme.text_muted,
        Priority::Normal => theme.accent,
        Priority::High => theme.warning,
        Priority::Urgent => theme.error,
    }
}

fn due_date_color(due_date: &str, theme: &Theme) -> ratatui::style::Color {
    match date::days_until(due_date) {
        Some(days) if days <= 3 => theme.error,
        Some(days) if days <= 10 => theme.warning,
        Some(_) => theme.text_muted,
        None => theme.text_muted,
    }
}

fn highlight_matches<'a>(
    text: &str,
    filter: &str,
    base_style: Style,
    match_style: Style,
) -> Vec<Span<'a>> {
    if filter.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let lower = text.to_lowercase();
    let q = filter.to_lowercase();
    let mut spans = Vec::new();
    let mut start = 0;
    while let Some(pos) = lower[start..].find(&q) {
        let abs = start + pos;
        if abs > start {
            spans.push(Span::styled(text[start..abs].to_string(), base_style));
        }
        spans.push(Span::styled(
            text[abs..abs + q.len()].to_string(),
            match_style,
        ));
        start = abs + q.len();
    }
    if start < text.len() {
        spans.push(Span::styled(text[start..].to_string(), base_style));
    }
    spans
}

/// Inline note gutter: two spaces of item indent, then "│ ", then the date column.
const NOTE_GUTTER: &str = "\u{2502} ";
const NOTE_DATE_CELL: usize = 12;

fn note_text_offset() -> usize {
    2 + NOTE_GUTTER.chars().count() + NOTE_DATE_CELL
}

/// Width left for note text once the gutter and date column are accounted for.
/// The item's own text spans columns 2..2+text_width, so notes end at the same
/// right edge.
fn note_text_width(text_width: usize) -> usize {
    (text_width + 2).saturating_sub(note_text_offset()).max(8)
}

/// An item's notes in display order. They are stored oldest-first (append-only)
/// but always shown newest-first, so a task's current state reads nearest its
/// title. The popup uses this too, so the two listings cannot disagree.
pub fn notes_newest_first<'a>(item: &'a TodoItem) -> impl Iterator<Item = &'a Note> {
    item.notes.iter().rev()
}

/// How many inline note rows `item` contributes under the current display mode.
/// Kept next to [`note_lines`] so row-height consumers (the calendar anchor)
/// cannot drift from what is actually drawn.
pub fn note_row_count(item: &TodoItem, text_width: usize, note_display: NoteDisplay) -> usize {
    match note_display {
        NoteDisplay::Hidden => 0,
        NoteDisplay::Latest => usize::from(!item.notes.is_empty()),
        NoteDisplay::All => {
            let available = note_text_width(text_width);
            item.notes
                .iter()
                .map(|note| super::wrap_text(&note.text, available).len())
                .sum()
        }
    }
}

/// Builds the inline lines for one note. The first line carries the date, later
/// wrap lines align under the text column.
fn note_lines(
    note: &Note,
    text_width: usize,
    wrap: bool,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let available = note_text_width(text_width);
    let gutter_style = Style::default().fg(theme.text_muted);
    let date_style = Style::default().fg(theme.text_muted);
    let body_style = Style::default().fg(theme.text_secondary);

    let mut segments = super::wrap_text(&note.text, available);
    if !wrap && segments.len() > 1 {
        // Latest-only mode must stay one row: re-wrap one column narrower so the
        // ellipsis fits inside the same right edge, and keep the first segment.
        let mut first = super::wrap_text(&note.text, available.saturating_sub(1))
            .into_iter()
            .next()
            .unwrap_or_default();
        first.push('\u{2026}');
        segments = vec![first];
    }

    segments
        .into_iter()
        .enumerate()
        .map(|(i, segment)| {
            if i == 0 {
                let date: String = note.created.chars().take(NOTE_DATE_CELL - 2).collect();
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(NOTE_GUTTER.to_string(), gutter_style),
                    Span::styled(
                        format!("{date:<width$}", width = NOTE_DATE_CELL),
                        date_style,
                    ),
                    Span::styled(segment, body_style),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(NOTE_GUTTER.to_string(), gutter_style),
                    Span::raw(" ".repeat(NOTE_DATE_CELL)),
                    Span::styled(segment, body_style),
                ])
            }
        })
        .collect()
}

pub fn render_item(
    item: &TodoItem,
    selected: bool,
    multi_selected: bool,
    theme: &Theme,
    text_width: usize,
    filter: &str,
    category_filter: Option<&str>,
    note_display: NoteDisplay,
) -> ListItem<'static> {
    let base_bg = if selected {
        theme.bg_tertiary
    } else if multi_selected {
        theme.accent_selection
    } else {
        theme.bg_primary
    };

    let is_overdue_item = !item.done && item.due_date.as_deref().is_some_and(date::is_overdue);

    let title_style = if item.done {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::CROSSED_OUT)
    } else if is_overdue_item {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_primary)
    };

    let match_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::UNDERLINED);

    let diamond = "\u{25c6}";
    let diamond_color = priority_color(item.priority, theme);

    let wrapped = super::wrap_text(&item.text, text_width.max(10));
    let mut lines = Vec::new();

    // Line 1: status marker, priority, pin, due date
    let mut line1 = Vec::new();
    line1.push(Span::styled(
        if multi_selected { "x" } else { " " },
        Style::default().fg(theme.accent),
    ));
    if item.done {
        line1.push(Span::styled("\u{2713}", Style::default().fg(theme.success)));
    } else if item.doing {
        line1.push(Span::styled(
            "\u{25e6}",
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        line1.push(Span::styled("-", Style::default().fg(theme.text_primary)));
    }
    line1.push(Span::raw(" "));
    line1.push(Span::styled(diamond, Style::default().fg(diamond_color)));
    if item.pinned {
        line1.push(Span::raw(" \u{1F4CC}"));
    }
    if let Some(ref date) = item.due_date {
        let date_color = due_date_color(date, theme);
        line1.push(Span::raw(" \u{1F4C5} "));
        line1.push(Span::styled(date.clone(), Style::default().fg(date_color)));
    }
    if !item.notes.is_empty() {
        line1.push(Span::raw(" \u{1F4DD} "));
        line1.push(Span::styled(
            item.notes.len().to_string(),
            Style::default().fg(theme.text_muted),
        ));
    }
    let badge_span = item.category.as_ref().and_then(|cat| {
        category_badge(cat, category_filter).map(|badge| {
            Span::styled(
                format!("[{}]", badge),
                Style::default().fg(theme.text_muted),
            )
        })
    });
    match badge_span {
        Some(badge) => {
            let left_width: usize = line1.iter().map(|s| s.width()).sum();
            let badge_width = badge.width();
            let target_width = text_width + 2;
            let gap = target_width.saturating_sub(left_width + badge_width);
            if gap >= 2 {
                line1.push(Span::raw(" ".repeat(gap)));
            } else {
                line1.push(Span::raw(" "));
            }
            line1.push(badge);
        }
        None => line1.push(Span::raw(" ")),
    }
    lines.push(Line::from(line1));

    // Line 2+: text at char 2, justified (last wrap line stays left-aligned)
    let last_idx = wrapped.len().saturating_sub(1);
    let content_width = text_width.max(10);
    for (i, seg) in wrapped.iter().enumerate() {
        let mut spans = vec![Span::raw("  ")];
        let justified;
        let render_text: &str = if i < last_idx {
            justified = justify_line(seg, content_width);
            &justified
        } else {
            seg
        };
        spans.extend(highlight_matches(render_text, filter, title_style, match_style));
        lines.push(Line::from(spans));
    }

    // Note log, under the item text it belongs to.
    match note_display {
        NoteDisplay::Hidden => {}
        NoteDisplay::Latest => {
            if let Some(note) = item.notes.last() {
                lines.extend(note_lines(note, text_width, false, theme));
            }
        }
        NoteDisplay::All => {
            for note in notes_newest_first(item) {
                lines.extend(note_lines(note, text_width, true, theme));
            }
        }
    }

    ListItem::new(Text::from(lines)).style(Style::default().bg(base_bg))
}

fn justify_line(text: &str, target_width: usize) -> String {
    let current_width = text.chars().count();
    if current_width >= target_width {
        return text.to_string();
    }
    let slack = target_width - current_width;
    let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
    if words.len() <= 1 {
        return text.to_string();
    }
    let num_gaps = words.len() - 1;
    let extra_per_gap = slack / num_gaps;
    let remainder = slack % num_gaps;

    let mut result = String::with_capacity(text.len() + slack);
    for (i, word) in words.iter().enumerate() {
        result.push_str(word);
        if i < num_gaps {
            let extra = extra_per_gap + usize::from(i < remainder);
            for _ in 0..(1 + extra) {
                result.push(' ');
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        justify_line, note_lines, note_row_count, note_text_offset, note_text_width,
        notes_newest_first, NoteDisplay,
    };
    use crate::data::TodoData;
    use crate::ui::theme::Theme;

    const TEXT_WIDTH: usize = 40;
    const LONG_NOTE: &str =
        "no answer this morning, called back at two and the line was busy again";

    #[test]
    fn display_order_is_newest_first_while_storage_stays_chronological() {
        let mut data = TodoData::new();
        let id = data.add("call for a rdv");
        data.add_note(id, "oldest");
        data.add_note(id, "middle");
        data.add_note(id, "newest");
        let item = data.get(id).unwrap();

        let shown: Vec<&str> = notes_newest_first(item)
            .map(|note| note.text.as_str())
            .collect();
        assert_eq!(shown, vec!["newest", "middle", "oldest"]);

        let stored: Vec<&str> = item.notes.iter().map(|note| note.text.as_str()).collect();
        assert_eq!(stored, vec!["oldest", "middle", "newest"]);
    }

    #[test]
    fn note_row_count_follows_the_display_mode() {
        let mut data = TodoData::new();
        let id = data.add("call for a rdv");
        data.add_note(id, "short one");
        data.add_note(id, LONG_NOTE);
        let item = data.get(id).unwrap();

        let wrapped = crate::ui::wrap_text(LONG_NOTE, note_text_width(TEXT_WIDTH)).len();
        assert!(wrapped > 1, "long note should wrap at this width");

        assert_eq!(note_row_count(item, TEXT_WIDTH, NoteDisplay::Hidden), 0);
        assert_eq!(note_row_count(item, TEXT_WIDTH, NoteDisplay::Latest), 1);
        assert_eq!(
            note_row_count(item, TEXT_WIDTH, NoteDisplay::All),
            1 + wrapped
        );
    }

    #[test]
    fn note_row_count_is_zero_without_notes() {
        let mut data = TodoData::new();
        let id = data.add("no notes here");
        let item = data.get(id).unwrap();

        for mode in [NoteDisplay::Hidden, NoteDisplay::Latest, NoteDisplay::All] {
            assert_eq!(note_row_count(item, TEXT_WIDTH, mode), 0);
        }
    }

    #[test]
    fn latest_note_stays_on_one_row_within_the_text_column() {
        let mut data = TodoData::new();
        let id = data.add("call for a rdv");
        data.add_note(id, LONG_NOTE);
        let item = data.get(id).unwrap();
        let theme = Theme::one_dark();

        let lines = note_lines(&item.notes[0], TEXT_WIDTH, false, &theme);
        assert_eq!(lines.len(), 1);
        // The ellipsis must fit inside the same right edge as the item text.
        assert!(
            lines[0].width() <= 2 + TEXT_WIDTH,
            "truncated note overflows the text column: {} > {}",
            lines[0].width(),
            2 + TEXT_WIDTH
        );
        assert_eq!(note_row_count(item, TEXT_WIDTH, NoteDisplay::Latest), 1);
    }

    #[test]
    fn wrapped_note_lines_align_under_the_text_column() {
        let mut data = TodoData::new();
        let id = data.add("call for a rdv");
        data.add_note(id, LONG_NOTE);
        let item = data.get(id).unwrap();
        let theme = Theme::one_dark();

        let lines = note_lines(&item.notes[0], TEXT_WIDTH, true, &theme);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.width() <= 2 + TEXT_WIDTH);
            // Every row reserves the same gutter + date columns.
            let prefix: usize = line
                .spans
                .iter()
                .take(3)
                .map(ratatui::text::Span::width)
                .sum();
            assert_eq!(prefix, note_text_offset());
        }
    }

    #[test]
    fn justify_line_fills_to_target_width() {
        let out = justify_line("hello world foo", 20);
        assert_eq!(out.chars().count(), 20);
    }

    #[test]
    fn justify_line_distributes_remainder_to_earlier_gaps() {
        let out = justify_line("a b c", 8);
        assert_eq!(out, "a   b  c");
        assert_eq!(out.chars().count(), 8);
    }

    #[test]
    fn justify_line_leaves_single_word_alone() {
        assert_eq!(justify_line("hello", 20), "hello");
    }

    #[test]
    fn justify_line_returns_original_when_already_at_or_over_width() {
        assert_eq!(justify_line("hello world", 5), "hello world");
        assert_eq!(justify_line("hello world", 11), "hello world");
    }
}
