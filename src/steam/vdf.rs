use std::path::Path;
use std::collections::HashMap;

// ─── VDF text key-value parser ────────────────────────────────────────────────
// Handles lines of the form:  \t"key"\t\t"value"
pub fn parse_kv_line(line: &str) -> Option<(&str, &str)> {
    let t = line.trim();
    if !t.starts_with('"') { return None; }
    let mut parts = t.splitn(5, '"');
    parts.next()?;                   // empty before first quote
    let key = parts.next()?;
    parts.next()?;                   // whitespace between quotes
    let val = parts.next()?;
    if key.is_empty() { return None; }
    Some((key, val))
}

// ─── appinfo.vdf binary parser ────────────────────────────────────────────────
// Extracts a map of AppID → name from the Steam appcache binary file.
pub fn parse_appinfo_vdf(steam_root: &Path) -> HashMap<u64, String> {
    let path = steam_root.join("appcache/appinfo.vdf");
    let data = match std::fs::read(&path) { Ok(d) => d, Err(_) => return HashMap::new() };

    let mut names = HashMap::new();
    if data.len() < 8 { return names; }

    // Validate magic: last 3 bytes of the 4-byte magic are 0x44 0x56 0x07
    if data[1] != 0x44 || data[2] != 0x56 || data[3] != 0x07 {
        return names;
    }

    // Header: 4 bytes magic + 4 bytes universe = 8 bytes.
    // Format version 0x29+ adds an extra 8 bytes (change number + padding) before app entries.
    let mut pos = if data[0] >= 0x29 { 16usize } else { 8usize };

    loop {
        if pos + 4 > data.len() { break; }
        let app_id = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as u64;
        pos += 4;
        if app_id == 0 { break; }

        if pos + 4 > data.len() { break; }
        let blob_size = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        pos += 4;

        // Skip fixed metadata: state(1) + last_updated(4) + access_token(8) + sha1(20) + change_number(4) + sha1_kv(20) = 57 bytes
        const META_SIZE: usize = 57;
        if pos + META_SIZE > data.len() { break; }
        pos += META_SIZE;

        let kv_start = pos;
        let kv_end = (kv_start + blob_size.saturating_sub(META_SIZE)).min(data.len());

        if let Some(name) = extract_name_from_kv(&data[kv_start..kv_end]) {
            names.insert(app_id, name);
        }

        // Advance past the whole blob (blob_size includes the metadata we already skipped)
        pos = (kv_start + blob_size.saturating_sub(META_SIZE)).min(data.len());
    }

    names
}

/// Extract the game name from a binary KV blob in appinfo.vdf.
fn extract_name_from_kv(data: &[u8]) -> Option<String> {
    // Pattern: STRING type (0x01) followed by key 4 as LE u32
    const NEEDLE: &[u8] = &[0x01, 0x04, 0x00, 0x00, 0x00];
    let pos = data.windows(NEEDLE.len()).position(|w| w == NEEDLE)?;
    let val_start = pos + NEEDLE.len();
    let nul = data[val_start..].iter().position(|&b| b == 0)?;
    let s = std::str::from_utf8(&data[val_start..val_start + nul]).ok()?;
    let name = s.trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}
