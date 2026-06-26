#![allow(dead_code)]

use parking_lot::Mutex;
use std::sync::Arc;

use crate::models::{AppConfig, JobRecord, JobStatus, RuleDefinition};
use crate::modules::{plugins::PluginsModule, process::ProcessModule};
use crate::work_queue::WorkQueue;

/// A `Send`-able notifier that background threads use to request a UI redraw.
/// Internally this is a channel sender; the UI side drains it periodically.
#[derive(Clone)]
pub struct UiNotifier(Arc<Mutex<Vec<()>>>);

impl UiNotifier {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    /// Called from a background thread to request a redraw.
    pub fn ping(&self) {
        self.0.lock().push(());
    }

    /// Called on the main thread: returns true if there are pending pings, and clears them.
    pub fn drain(&self) -> bool {
        let mut v = self.0.lock();
        if v.is_empty() {
            false
        } else {
            v.clear();
            true
        }
    }
}

/// Global mutable application state shared between all GPUI views.
/// Wrapped in `Arc<Mutex<>>` for interior mutability across the sync GPUI render loop.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<AppStateInner>>,
    /// Ping channel: background threads call `.ping()` to request a UI redraw.
    pub notifier: UiNotifier,
}

pub struct AppStateInner {
    pub config: AppConfig,
    pub process: ProcessModule,
    pub queue: WorkQueue,
    pub rules: Vec<RuleDefinition>,
    pub active_tab: usize,
    pub staged_files: Vec<std::path::PathBuf>,
    pub selected_rule: Option<String>,
    pub auto_clean: bool,
    pub pdf_split_enabled: bool,
    pub selected_jobs_for_clean: Vec<String>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let config = AppConfig::load().unwrap_or_default();
        let process = ProcessModule::init()?;
        let rules = PluginsModule::load_rules();
        let queue = WorkQueue::new(
            config.max_convert_concurrency,
            config.max_clean_concurrency,
        );

        let default_rule = rules.first().map(|r| r.file_name.clone());

        let inner = AppStateInner {
            config,
            process,
            queue,
            rules,
            active_tab: 0,
            staged_files: Vec::new(),
            selected_rule: default_rule,
            auto_clean: false,
            pdf_split_enabled: true,
            selected_jobs_for_clean: Vec::new(),
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            notifier: UiNotifier::new(),
        })
    }

    // ---- Getters ----

    pub fn jobs(&self) -> Vec<JobRecord> {
        self.inner.lock().process.get_jobs()
    }

    pub fn rules(&self) -> Vec<RuleDefinition> {
        self.inner.lock().rules.clone()
    }

    pub fn config(&self) -> AppConfig {
        self.inner.lock().config.clone()
    }

    pub fn log_entries(&self) -> Vec<crate::utils::log::LogEntry> {
        crate::utils::log::history()
    }

    pub fn staged_files(&self) -> Vec<std::path::PathBuf> {
        self.inner.lock().staged_files.clone()
    }

    pub fn selected_rule(&self) -> Option<String> {
        self.inner.lock().selected_rule.clone()
    }

    pub fn auto_clean(&self) -> bool {
        self.inner.lock().auto_clean
    }

    pub fn pdf_split_enabled(&self) -> bool {
        self.inner.lock().pdf_split_enabled
    }

    pub fn selected_jobs_for_clean(&self) -> Vec<String> {
        self.inner.lock().selected_jobs_for_clean.clone()
    }

    pub fn queue(&self) -> WorkQueue {
        self.inner.lock().queue.clone()
    }

    // ---- Setters ----

    pub fn add_staged_file(&self, path: std::path::PathBuf) {
        let mut inner = self.inner.lock();
        if !inner.staged_files.contains(&path) {
            inner.staged_files.push(path);
        }
    }

    pub fn remove_staged_file(&self, idx: usize) {
        let mut inner = self.inner.lock();
        if idx < inner.staged_files.len() {
            inner.staged_files.remove(idx);
        }
    }

    pub fn clear_staged_files(&self) {
        self.inner.lock().staged_files.clear();
    }

    pub fn set_selected_rule(&self, rule: Option<String>) {
        self.inner.lock().selected_rule = rule;
    }

    pub fn set_auto_clean(&self, v: bool) {
        self.inner.lock().auto_clean = v;
    }

    pub fn set_pdf_split_enabled(&self, v: bool) {
        self.inner.lock().pdf_split_enabled = v;
    }

    pub fn toggle_job_for_clean(&self, guid: String) {
        let mut inner = self.inner.lock();
        if let Some(pos) = inner.selected_jobs_for_clean.iter().position(|g| *g == guid) {
            inner.selected_jobs_for_clean.remove(pos);
        } else {
            inner.selected_jobs_for_clean.push(guid);
        }
    }

    pub fn push_log(&self, entry: crate::utils::log::LogEntry) {
        crate::utils::log::emit(entry);
    }

    pub fn update_config<F: FnOnce(&mut AppConfig)>(&self, f: F) {
        let mut inner = self.inner.lock();
        f(&mut inner.config);
        let _ = inner.config.save();
    }

    pub fn reload_rules(&self) {
        let rules = PluginsModule::load_rules();
        self.inner.lock().rules = rules;
    }

    /// Create a job record for each staged file, clear the staged list,
    /// then spawn background threads to run MinerU conversion.
    pub fn submit_staged_files(&self, cx: &mut gpui::App) -> Vec<String> {
        let staged = self.staged_files();
        let mut job_paths: Vec<(String, std::path::PathBuf)> = Vec::new();
        {
            let inner = self.inner.lock();
            for path in &staged {
                match inner.process.create_job(path) {
                    Ok(job) => job_paths.push((job.job_guid, path.clone())),
                    Err(e) => {
                        crate::utils::log::log_error_tagged("convert", format!(
                            "创建任务失败 {}: {}",
                            path.display(),
                            e
                        ));
                    }
                }
            }
        }
        self.clear_staged_files();

        let guids: Vec<String> = job_paths.iter().map(|(g, _)| g.clone()).collect();
        let _ = cx; // cx kept for API symmetry; notify is done via UiNotifier
        for (guid, path) in job_paths {
            self.spawn_convert_task(guid, path);
        }
        guids
    }

    /// Spawn a background thread with a dedicated tokio runtime for the MinerU pipeline.
    /// reqwest requires a tokio reactor; GPUI's executor does not provide one,
    /// so we spin up a per-job single-threaded tokio runtime on a std thread.
    fn spawn_convert_task(&self, guid: String, source_path: std::path::PathBuf) {
        use crate::models::{QueueItem, QueueItemStatus, QueuePoolType};

        let config = self.config();
        let process = self.inner.lock().process.jobs();
        let queue = self.queue();
        let notifier = self.notifier.clone();

        // Determine original file name for queue display
        let file_name = {
            let jobs = process.lock();
            jobs.iter()
                .find(|j| j.job_guid == guid)
                .map(|j| j.original_file_name.clone())
                .unwrap_or_else(|| guid[..8].to_string())
        };

        // Register in the convert queue immediately
        let queue_item = QueueItem::new(&guid, &file_name, QueuePoolType::Convert);
        let queue_item_id = queue_item.id.clone();
        queue.enqueue_convert(queue_item);
        notifier.ping();

        std::thread::spawn(move || {
            use crate::modules::mineru::MinerUModule;
            use crate::utils::log::{log_error_tagged, log_info_tagged};

            if config.mineru_token.is_empty() {
                log_error_tagged("convert", format!(
                    "[{}] MinerU token 未配置, 请先在设置中填写 token",
                    &guid[..8]
                ));
                {
                    let mut jobs = process.lock();
                    if let Some(j) = jobs.iter_mut().find(|j| j.job_guid == guid) {
                        j.status = JobStatus::Failed;
                        j.error_message = Some("MinerU token 未配置".to_string());
                    }
                }
                queue.update_item_status(QueuePoolType::Convert, &queue_item_id, QueueItemStatus::Failed);
                notifier.ping();
                return;
            }

            {
                let mut jobs = process.lock();
                if let Some(j) = jobs.iter_mut().find(|j| j.job_guid == guid) {
                    j.status = JobStatus::Converting;
                }
            }
            queue.update_item_status(QueuePoolType::Convert, &queue_item_id, QueueItemStatus::Running);
            notifier.ping();

            // Build a dedicated tokio runtime — reqwest uses tokio::time internally
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    log_error_tagged("convert", format!("[{}] 无法创建异步运行时: {}", &guid[..8], e));
                    let mut jobs = process.lock();
                    if let Some(j) = jobs.iter_mut().find(|j| j.job_guid == guid) {
                        j.status = JobStatus::Failed;
                        j.error_message = Some(format!("运行时创建失败: {}", e));
                    }
                    notifier.ping();
                    return;
                }
            };

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_default();

            match rt.block_on(MinerUModule::convert(
                &client,
                &config.mineru_base_url,
                &config.mineru_token,
                &guid,
                &source_path,
            )) {
                Ok(_md_path) => {
                    log_info_tagged("convert", format!("[{}] MinerU 转换成功", &guid[..8]));
                    let original_name = {
                        let mut jobs = process.lock();
                        let name = jobs.iter().find(|j| j.job_guid == guid)
                            .map(|j| j.original_file_name.clone())
                            .unwrap_or_default();
                        if let Some(j) = jobs.iter_mut().find(|j| j.job_guid == guid) {
                            j.status = JobStatus::Converted;
                            j.progress = Some(1.0);
                        }
                        name
                    };
                    // Export merged_raw.md + images/ to export/<stem>_raw/
                    // Keep status as Converted so user can still queue for AI cleaning.
                    let raw_dir = crate::utils::path::job_export_dir_raw(&original_name);
                    Self::do_export(&guid, &original_name, "merged_raw.md", raw_dir.clone(), false, &process);
                    log_info_tagged("convert", format!(
                        "[{}] 已导出到: {}",
                        &guid[..8],
                        raw_dir.display()
                    ));
                    queue.update_item_status(QueuePoolType::Convert, &queue_item_id, QueueItemStatus::Done);
                    queue.update_item_progress(QueuePoolType::Convert, &queue_item_id, 1.0);
                }
                Err(e) => {
                    log_error_tagged("convert", format!("[{}] MinerU 转换失败: {}", &guid[..8], e));
                    {
                        let mut jobs = process.lock();
                        if let Some(j) = jobs.iter_mut().find(|j| j.job_guid == guid) {
                            j.status = JobStatus::Failed;
                            j.error_message = Some(e.to_string());
                        }
                    }
                    queue.update_item_status(QueuePoolType::Convert, &queue_item_id, QueueItemStatus::Failed);
                }
            }
            notifier.ping();
        });
    }

    /// Copy job output files into `export_dir`.
    ///
    /// `md_src_name`: filename inside the tmp dir to copy (`merged_raw.md` or `cleaned.md`).
    /// Falls back to `merged_raw.md` if the preferred file does not exist.
    /// Always copies `images/` when present.
    ///
    /// `mark_exported`: if true, advances the job status to `Exported`.
    fn do_export(
        guid: &str,
        original_name: &str,
        md_src_name: &str,
        export_dir: std::path::PathBuf,
        mark_exported: bool,
        process: &parking_lot::Mutex<Vec<crate::models::JobRecord>>,
    ) {
        use crate::utils::{file::{copy_dir, copy_file, ensure_dir}, path::job_tmp_dir};
        use crate::utils::log::log_error_tagged;

        let tmp = job_tmp_dir(guid);

        if let Err(e) = ensure_dir(&export_dir) {
            log_error_tagged("convert", format!("[{}] 创建导出目录失败: {}", &guid[..8], e));
            return;
        }

        // Resolve source: try requested file, fall back to merged_raw.md
        let md_src = {
            let preferred = tmp.join(md_src_name);
            if preferred.exists() { preferred } else { tmp.join("merged_raw.md") }
        };

        // Destination filename: <original_stem>.md
        let stem = std::path::Path::new(original_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let md_dst = export_dir.join(format!("{}.md", stem));

        if md_src.exists() {
            if let Err(e) = copy_file(&md_src, &md_dst) {
                log_error_tagged("convert", format!("[{}] 复制 md 失败: {}", &guid[..8], e));
            }
        }

        // Copy images/ if present
        let images_src = tmp.join("images");
        if images_src.exists() {
            let images_dst = export_dir.join("images");
            if let Err(e) = copy_dir(&images_src, &images_dst) {
                log_error_tagged("convert", format!("[{}] 复制 images 失败: {}", &guid[..8], e));
            }
        }

        // Update export_path on the job record; optionally set Exported status
        let export_str = md_dst.to_string_lossy().to_string();
        let mut jobs = process.lock();
        if let Some(j) = jobs.iter_mut().find(|j| j.job_guid == guid) {
            j.export_path = export_str;
            if mark_exported {
                j.status = crate::models::JobStatus::Exported;
            }
        }
    }

    /// Spawn a background thread with a dedicated tokio runtime for the AI cleaning pipeline.
    pub fn spawn_clean_task(
        &self,
        guids: Vec<String>,
        rule: crate::models::RuleDefinition,
        _cx: &mut gpui::App,
    ) {
        use crate::models::{QueueItem, QueueItemStatus, QueuePoolType};

        let config = self.config();
        let process = self.inner.lock().process.jobs();
        let queue = self.queue();
        let notifier = self.notifier.clone();

        // Enqueue each job in the clean queue and collect (guid → queue_item_id) pairs
        let guid_to_qid: Vec<(String, String)> = guids.iter().map(|guid| {
            let file_name = {
                let jobs = process.lock();
                jobs.iter()
                    .find(|j| &j.job_guid == guid)
                    .map(|j| j.original_file_name.clone())
                    .unwrap_or_else(|| guid[..8].to_string())
            };
            let qi = QueueItem::new(guid, &file_name, QueuePoolType::Clean);
            let qid = qi.id.clone();
            queue.enqueue_clean(qi);
            (guid.clone(), qid)
        }).collect();
        notifier.ping();

        std::thread::spawn(move || {
            use crate::modules::cleaner::CleanerModule;
            use crate::utils::log::{log_error_tagged, log_info_tagged};

            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    log_error_tagged("clean", format!("无法创建清洗运行时: {}", e));
                    return;
                }
            };

            for (guid, qid) in &guid_to_qid {
                {
                    let mut jobs = process.lock();
                    if let Some(j) = jobs.iter_mut().find(|j| &j.job_guid == guid) {
                        j.status = JobStatus::Cleaning;
                    }
                }
                queue.update_item_status(QueuePoolType::Clean, qid, QueueItemStatus::Running);
                notifier.ping();

                match rt.block_on(CleanerModule::clean(guid, &rule, &config)) {
                    Ok(_path) => {
                        log_info_tagged("clean", format!("[{}] AI 清洗完成", &guid[..8]));
                        let original_name = {
                            let mut jobs = process.lock();
                            let name = jobs.iter().find(|j| &j.job_guid == guid)
                                .map(|j| j.original_file_name.clone())
                                .unwrap_or_default();
                            if let Some(j) = jobs.iter_mut().find(|j| &j.job_guid == guid) {
                                j.status = JobStatus::Cleaned;
                            }
                            name
                        };
                        // Export cleaned.md + images/ to export/<stem>_<rule>/, mark Exported.
                        let clean_dir = crate::utils::path::job_export_dir_cleaned(&original_name, &rule.name);
                        Self::do_export(guid, &original_name, "cleaned.md", clean_dir.clone(), true, &process);
                        log_info_tagged("clean", format!(
                            "[{}] 已导出到: {}",
                            &guid[..8],
                            clean_dir.display()
                        ));
                        queue.update_item_status(QueuePoolType::Clean, qid, QueueItemStatus::Done);
                        queue.update_item_progress(QueuePoolType::Clean, qid, 1.0);
                    }
                    Err(e) => {
                        log_error_tagged("clean", format!("[{}] 清洗失败: {}", &guid[..8], e));
                        {
                            let mut jobs = process.lock();
                            if let Some(j) = jobs.iter_mut().find(|j| &j.job_guid == guid) {
                                j.status = JobStatus::Failed;
                                j.error_message = Some(e.to_string());
                            }
                        }
                        queue.update_item_status(QueuePoolType::Clean, qid, QueueItemStatus::Failed);
                    }
                }
                notifier.ping();
            }
        });
    }
}
