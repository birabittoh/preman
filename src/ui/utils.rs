use ratatui::layout::Rect;
use crate::SortColumn;

pub fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w.min(area.width),
        height: h.min(area.height),
    }
}

/// Given a terminal area and the body rect, return what row index was clicked
/// (0-indexed into filtered_indices, accounting for scroll_offset).
/// Returns None if click is outside the list or on the header.
pub fn hit_test_table_row(click_y: u16, body_area: Rect, scroll_offset: usize) -> Option<usize> {
    let rel_y = click_y.checked_sub(body_area.y)?;
    if rel_y == 0 { return None; } // header row
    let row = rel_y as usize - 1 + scroll_offset;
    Some(row)
}

/// Returns which SortColumn was clicked based on x position within the table area.
pub fn hit_test_table_col(click_x: u16, table_area: Rect, show_installed: bool) -> Option<SortColumn> {
    let rel_x = click_x.checked_sub(table_area.x)? as usize;
    let available = table_area.width as usize;
    if show_installed {
        let name_w = available.saturating_sub(38);
        let boundaries = [name_w, name_w+10, name_w+20, name_w+27, name_w+38];
        if rel_x < boundaries[0]      { Some(SortColumn::Name) }
        else if rel_x < boundaries[1] { Some(SortColumn::AppId) }
        else if rel_x < boundaries[2] { Some(SortColumn::Size) }
        else if rel_x < boundaries[3] { Some(SortColumn::Cloud) }
        else                           { Some(SortColumn::Installed) }
    } else {
        let name_w = available.saturating_sub(27);
        let boundaries = [name_w, name_w+10, name_w+20, name_w+27];
        if rel_x < boundaries[0]      { Some(SortColumn::Name) }
        else if rel_x < boundaries[1] { Some(SortColumn::AppId) }
        else if rel_x < boundaries[2] { Some(SortColumn::Size) }
        else                           { Some(SortColumn::Cloud) }
    }
}
