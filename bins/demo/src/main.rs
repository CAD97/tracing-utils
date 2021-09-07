use eframe::{egui, epi};
use tracing_subscriber::prelude::*;

#[derive(Debug, Default)]
struct App {
    message: String,
}

impl epi::App for App {
    #[tracing::instrument(skip(ctx, _frame))]
    fn update(&mut self, ctx: &eframe::egui::CtxRef, _frame: &mut eframe::epi::Frame<'_>) {
        egui::Window::new("tracing-egui log")
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.add(tracing_egui::Widget {
                    ..Default::default()
                });
            });

        egui::Window::new("event creator")
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("message:");
                    ui.text_edit_singleline(&mut self.message);
                });
                ui.horizontal(|ui| {
                    if ui.button("ERROR").clicked() {
                        tracing::error!(message = %self.message);
                    }
                    if ui.button("WARN").clicked() {
                        tracing::warn!(message = %self.message);
                    }
                    if ui.button("INFO").clicked() {
                        tracing::info!(message = %self.message);
                    }
                    if ui.button("DEBUG").clicked() {
                        tracing::debug!(message = %self.message);
                    }
                    if ui.button("TRACE").clicked() {
                        tracing::trace!(message = %self.message);
                    }
                });
            });
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        if std::env::var_os("RUST_LOG").is_none() {
            std::env::set_var("RUST_LOG", "info");
        }

        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().pretty())
            .with(tracing_memory::layer())
            .init();

        tracing::warn!("App is starting..");
        log_spam(10);
    }

    fn name(&self) -> &str {
        "tracing-utils-demo"
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let opt = eframe::NativeOptions {
        transparent: true,
        ..Default::default()
    };
    eframe::run_native(Box::new(App::default()), opt);

    Ok(())
}

#[tracing::instrument]
fn log_spam(spam: u64) {
    for instance in 0..spam {
        tracing::info!(instance, "Log spam requested!");
    }
}
