pub mod dashboard;
pub mod convert;
pub mod clean;
pub mod queue;
pub mod settings;
pub mod split;

pub use dashboard::DashboardPage;
pub use convert::ConvertPage;
pub use clean::CleanPage;
pub use queue::QueuePage;
pub use settings::SettingsPage;
#[allow(unused_imports)]
pub use split::SplitPage;
