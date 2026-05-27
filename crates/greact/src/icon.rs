/// Built-in icon set (Unicode fallback) with a stable API similar to GPUI's
/// `IconName` usage pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconName {
    Search,
    Settings,
    Check,
    Close,
    Plus,
    Minus,
    ArrowRight,
    ArrowDown,
    Warning,
    Info,
    Folder,
    File,
    Home,
    User,
}

#[derive(Debug, Clone)]
pub enum IconInput {
    Named(IconName),
    Text(String),
}

impl From<IconName> for IconInput {
    fn from(value: IconName) -> Self {
        Self::Named(value)
    }
}

impl From<&str> for IconInput {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for IconInput {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl IconName {
    pub fn as_glyph(self) -> &'static str {
        icon_glyph(self)
    }
}

impl std::str::FromStr for IconName {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.to_ascii_lowercase();
        let icon = match normalized.as_str() {
            "search" => Self::Search,
            "settings" | "gear" => Self::Settings,
            "check" | "ok" => Self::Check,
            "close" | "x" => Self::Close,
            "plus" | "add" => Self::Plus,
            "minus" | "remove" => Self::Minus,
            "arrow_right" | "right" => Self::ArrowRight,
            "arrow_down" | "down" => Self::ArrowDown,
            "warning" | "alert" => Self::Warning,
            "info" => Self::Info,
            "folder" => Self::Folder,
            "file" => Self::File,
            "home" => Self::Home,
            "user" | "person" => Self::User,
            _ => return Err(()),
        };
        Ok(icon)
    }
}

pub fn icon_glyph(name: IconName) -> &'static str {
    match name {
        IconName::Search => "⌕",
        IconName::Settings => "⚙",
        IconName::Check => "✓",
        IconName::Close => "✕",
        IconName::Plus => "+",
        IconName::Minus => "−",
        IconName::ArrowRight => "→",
        IconName::ArrowDown => "↓",
        IconName::Warning => "⚠",
        IconName::Info => "ℹ",
        IconName::Folder => "📁",
        IconName::File => "📄",
        IconName::Home => "⌂",
        IconName::User => "👤",
    }
}

pub fn resolve_icon(input: IconInput) -> IconName {
    match input {
        IconInput::Named(name) => name,
        IconInput::Text(s) => s.parse::<IconName>().unwrap_or(IconName::Info),
    }
}
