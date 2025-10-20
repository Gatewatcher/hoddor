pub trait LoggerPort: Send + Sync {
    fn log(&self, message: &str);

    fn error(&self, message: &str);

    fn warn(&self, message: &str);

    fn time(&self, label: &str);

    fn time_end(&self, label: &str);
}
