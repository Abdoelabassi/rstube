// Rust YouTube Downloader
// GUI: egui/eframe
// Backend: Tokio async
// Features: progress bar, history, format select, folder picker, clean architecture
// Core idea: GUI spawns async download tasks, no blocking threads

use eframe::{egui, App};
use std::{
    process::Stdio,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::{process::Command, runtime::Runtime};

#[derive(PartialEq, Clone)]
enum Format {
    BestVideo,
    AudioOnly,
}

#[derive(Clone)]
struct HistoryItem {
    url: String,
    format: String,
    status: String,
}

struct DownloaderApp {
    url: String,
    format: Format,
    output_dir: Option<String>,

    status: Arc<Mutex<String>>,
    progress: Arc<Mutex<f32>>,
    history: Arc<Mutex<Vec<HistoryItem>>>,

    rt: Runtime,
}

impl Default for DownloaderApp {
    fn default() -> Self {
        Self {
            url: String::new(),
            format: Format::BestVideo,
            output_dir: None,
            status: Arc::new(Mutex::new("Idle".into())),
            progress: Arc::new(Mutex::new(0.0)),
            history: Arc::new(Mutex::new(Vec::new())),
            rt: Runtime::new().expect("Tokio runtime"),
        }
    }
}

impl App for DownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎬 Rust YouTube Downloader — v1.0");
            ui.separator();

            ui.label("YouTube URL");
            ui.text_edit_singleline(&mut self.url);

            ui.horizontal(|ui| {
                ui.label("Format:");
                ui.radio_value(&mut self.format, Format::BestVideo, "Best Video");
                ui.radio_value(&mut self.format, Format::AudioOnly, "MP3 Audio");
            });

            if ui.button("📁 Choose Folder").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = Some(path.display().to_string());
                }
            }

            if let Some(dir) = &self.output_dir {
                ui.label(format!("Saving to: {}", dir));
            }

            if ui.button("⬇ Download").clicked() && !self.url.is_empty() {
                let url = self.url.clone();
                let format = self.format.clone();
                let dir = self.output_dir.clone();
                let status = self.status.clone();
                let progress = self.progress.clone();
                let history = self.history.clone();

                *status.lock().unwrap() = "Starting download…".into();
                *progress.lock().unwrap() = 0.0;

                self.rt.spawn(async move {
                    let mut cmd = Command::new("yt-dlp");
                    cmd.arg("--newline");

                    if let Some(d) = dir {
                        cmd.arg("-P").arg(d);
                    }

                    match format {
                        Format::BestVideo => { cmd.args(["-f","bestvideo+bestaudio/best","--merge-output-format", "mp4"]); }
                        Format::AudioOnly => { cmd.args(["-x", "--audio-format", "mp3"]); }
                    }

                    cmd.arg(&url)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped());

                    let mut child = match cmd.spawn() {
                        Ok(c) => c,
                        Err(_) => {
                            *status.lock().unwrap() = "Failed to start yt-dlp".into();
                            return;
                        }
                    };

                    /*
                    let stdout = child.stdout.take().unwrap();
                    let reader = BufReader::new(stdout);

                    for line in reader.lines().flatten() {
                        if let Some(p) = parse_progress(&line) {
                            *progress.lock().unwrap() = p;
                            *status.lock().unwrap() = format!("Downloading… {:.0}%", p * 100.0);
                        }
                    }
                    */
                    let stdout = child.stdout.take().unwrap();
                    let mut reader = BufReader::new(stdout).lines();

                    while let Ok(Some(line)) = reader.next_line().await {
                        if let Some(p) = parse_progress(&line) {
                            *progress.lock().unwrap() = p;
                            *status.lock().unwrap() = format!("Downloading… {:.0}%", p * 100.0);
                        }
                    }


                    let success = child.wait().await.map(|s| s.success()).unwrap_or(false);

                    history.lock().unwrap().push(HistoryItem {
                        url: url.clone(),
                        format: match format { Format::BestVideo => "Video".into(), Format::AudioOnly => "MP3".into() },
                        status: if success { "Completed".into() } else { "Failed".into() },
                    });

                    *status.lock().unwrap() = if success {
                        "✅ Download completed".into()
                    } else {
                        "❌ Download failed".into()
                    };
                });
            }

            ui.separator();

            ui.add(egui::ProgressBar::new(*self.progress.lock().unwrap()).show_percentage());
            ui.label(format!("Status: {}", self.status.lock().unwrap()));

            ui.separator();
            ui.heading("📜 History");
            for item in self.history.lock().unwrap().iter().rev() {
                ui.label(format!("{} | {} | {}", item.url, item.format, item.status));
            }
        });

        ctx.request_repaint();
    }
}

fn parse_progress(line: &str) -> Option<f32> {
    if let Some(idx) = line.find('%') {
        let start = line[..idx].rfind(' ')? + 1;
        let p: f32 = line[start..idx].trim().parse().ok()?;
        Some(p / 100.0)
    } else {
        None
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rust YouTube Downloader",
        options,
        Box::new(|_| Box::new(DownloaderApp::default())),
    )
}

/*
Cargo.toml

[dependencies]
eframe = "0.27"
egui = "0.27"
rfd = "0.14"
tokio = { version = "1", features = ["process", "rt-multi-thread"] }

System dependency:
yt-dlp
*/

