pub fn prev_char_boundary(s: &str, idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    let mut prev = 0;
    for (i, _) in s.char_indices() {
        if i >= idx {
            break;
        }
        prev = i;
    }
    prev
}

pub fn next_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    for (i, _) in s.char_indices() {
        if i > idx {
            return i;
        }
    }
    s.len()
}

pub fn select_all(len: usize) -> (usize, usize) {
    (0, len)
}

pub fn move_cursor_left(s: &str, cursor: usize) -> usize {
    prev_char_boundary(s, cursor)
}

pub fn move_cursor_right(s: &str, cursor: usize) -> usize {
    next_char_boundary(s, cursor)
}

pub fn move_cursor_home(_s: &str) -> usize {
    0
}

pub fn move_cursor_end(s: &str) -> usize {
    s.len()
}

pub fn replace_selection(
    value: &mut String,
    cursor: &mut usize,
    anchor: &mut usize,
    insert: &str,
) -> bool {
    let (start, end) = if *cursor <= *anchor {
        (*cursor, *anchor)
    } else {
        (*anchor, *cursor)
    };
    if start > end || end > value.len() {
        return false;
    }
    value.replace_range(start..end, insert);
    let next = start + insert.len();
    *cursor = next;
    *anchor = next;
    true
}

pub fn insert_text(value: &mut String, cursor: &mut usize, anchor: &mut usize, text: &str) -> bool {
    replace_selection(value, cursor, anchor, text)
}

pub fn delete_backward(value: &mut String, cursor: &mut usize, anchor: &mut usize) -> bool {
    if *cursor != *anchor {
        return replace_selection(value, cursor, anchor, "");
    }
    if *cursor == 0 {
        return false;
    }
    let start = prev_char_boundary(value, *cursor);
    value.replace_range(start..*cursor, "");
    *cursor = start;
    *anchor = start;
    true
}

pub fn delete_forward(value: &mut String, cursor: &mut usize, anchor: &mut usize) -> bool {
    if *cursor != *anchor {
        return replace_selection(value, cursor, anchor, "");
    }
    if *cursor >= value.len() {
        return false;
    }
    let end = next_char_boundary(value, *cursor);
    value.replace_range(*cursor..end, "");
    *anchor = *cursor;
    true
}
