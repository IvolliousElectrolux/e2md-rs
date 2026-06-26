#![allow(dead_code)]

/// Color palette for E2MD UI elements.
pub struct Colors;

impl Colors {
    pub const SUCCESS: &'static str = "text-green-500";
    pub const WARN: &'static str = "text-yellow-500";
    pub const ERROR: &'static str = "text-red-500";
    pub const INFO: &'static str = "text-foreground";
    pub const MUTED: &'static str = "text-muted-foreground";
}
