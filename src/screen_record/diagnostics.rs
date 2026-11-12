use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct Trace {
    id: u128,
    started: std::time::Instant,
    write_error: Arc<Mutex<Option<String>>>,
}
impl Trace {
    pub fn new() -> Self {
        Self {
            id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            started: std::time::Instant::now(),
            write_error: Arc::new(Mutex::new(None)),
        }
    }
    pub fn note(&self, phase: &str, message: impl std::fmt::Display) {
        let module = format!(
            "screen_record/{} {phase} +{:.3}s",
            self.id,
            self.started.elapsed().as_secs_f64()
        );
        let level = if phase == "FAILED" || phase.ends_with(".failed") {
            "ERROR"
        } else {
            "INFO"
        };
        if let Err(error) = crate::error_log::write(level, &module, &message.to_string()) {
            *self.write_error.lock().unwrap_or_else(|p| p.into_inner()) = Some(error.to_string());
        }
    }
    pub fn finish(
        &self,
        result: Result<std::path::PathBuf, String>,
    ) -> Result<std::path::PathBuf, String> {
        match result {
            Ok(path) => {
                self.note("complete", path.display());
                Ok(path)
            }
            Err(error) => {
                self.note("FAILED", &error);
                if let Some(log_error) = self
                    .write_error
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .as_ref()
                {
                    Err(format!(
                        "{error}\n{}: {log_error}",
                        crate::localization::tr("写入错误日志失败")
                    ))
                } else if let Some(path) = crate::error_log::path() {
                    Err(format!("{error}\nerror.log: {}", path.display()))
                } else {
                    Err(error)
                }
            }
        }
    }
}
