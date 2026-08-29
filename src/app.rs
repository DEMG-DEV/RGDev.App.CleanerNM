use crate::scanner::{
    format_bytes, start_delete, start_scan, DeleteMessage, ScanItem, ScanMessage, TargetCategory,
};
use crossbeam_channel::Receiver;
use eframe::egui;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq)]
pub enum AppState {
    Selecting,
    Scanning,
    Results,
    ConfirmClean,
    Cleaning,
    Done,
}

pub struct CleanerApp {
    state: AppState,
    selected_path: Option<PathBuf>,
    categories: Vec<TargetCategory>,

    // Scanning state
    scan_rx: Option<Receiver<ScanMessage>>,
    scanned_count: usize,
    current_scan_dir: String,

    // Results
    results: Vec<ScanItem>,
    sort_by_size_desc: bool,
    search_query: String,
    filter_category: Option<String>,
    toast_message: Option<(String, std::time::Instant)>,

    // Cleaning state
    delete_rx: Option<Receiver<DeleteMessage>>,
    delete_current: usize,
    delete_total: usize,
    delete_current_path: String,
    deleted_count: usize,
    freed_bytes: u64,
    delete_errors: Vec<String>,
}

impl Default for CleanerApp {
    fn default() -> Self {
        Self {
            state: AppState::Selecting,
            selected_path: None,
            categories: TargetCategory::default_categories(),
            scan_rx: None,
            scanned_count: 0,
            current_scan_dir: String::new(),
            results: Vec::new(),
            sort_by_size_desc: true,
            search_query: String::new(),
            filter_category: None,
            toast_message: None,
            delete_rx: None,
            delete_current: 0,
            delete_total: 0,
            delete_current_path: String::new(),
            deleted_count: 0,
            freed_bytes: 0,
            delete_errors: Vec::new(),
        }
    }
}

fn open_in_file_manager(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,\"{}\"", path.display()))
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap_or(path);
        let _ = std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn();
    }
}

fn category_colors(cat: &str) -> (egui::Color32, egui::Color32) {
    match cat {
        "Node.js" => (egui::Color32::from_rgb(22, 51, 34), egui::Color32::from_rgb(74, 222, 128)),
        "Flutter / Dart" => (egui::Color32::from_rgb(20, 40, 65), egui::Color32::from_rgb(96, 165, 250)),
        "Rust" => (egui::Color32::from_rgb(55, 30, 20), egui::Color32::from_rgb(251, 146, 60)),
        ".NET" => (egui::Color32::from_rgb(40, 30, 65), egui::Color32::from_rgb(167, 139, 250)),
        "Python" => (egui::Color32::from_rgb(55, 45, 15), egui::Color32::from_rgb(250, 204, 21)),
        "Go" => (egui::Color32::from_rgb(18, 48, 55), egui::Color32::from_rgb(45, 212, 191)),
        _ => (egui::Color32::from_rgb(35, 40, 52), egui::Color32::from_rgb(203, 213, 225)),
    }
}

impl CleanerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(15, 17, 26);
        visuals.window_fill = egui::Color32::from_rgb(22, 25, 38);
        visuals.faint_bg_color = egui::Color32::from_rgb(28, 32, 48);
        visuals.extreme_bg_color = egui::Color32::from_rgb(11, 13, 20);
        cc.egui_ctx.set_visuals(visuals);

        Self::default()
    }

    fn check_background_messages(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.scan_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanMessage::Found(item) => {
                        self.results.push(item);
                        ctx.request_repaint();
                    }
                    ScanMessage::Progress { scanned_dirs, current_path } => {
                        self.scanned_count = scanned_dirs;
                        self.current_scan_dir = current_path;
                        ctx.request_repaint();
                    }
                    ScanMessage::Finished => {
                        self.scan_rx = None;
                        if self.sort_by_size_desc {
                            self.results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
                        }
                        self.state = AppState::Results;
                        ctx.request_repaint();
                        break;
                    }
                }
            }
        }

        if let Some(ref rx) = self.delete_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DeleteMessage::Progress { current, total, path } => {
                        self.delete_current = current;
                        self.delete_total = total;
                        self.delete_current_path = path;
                        ctx.request_repaint();
                    }
                    DeleteMessage::Done { deleted_count, freed_bytes, errors } => {
                        self.delete_rx = None;
                        self.deleted_count = deleted_count;
                        self.freed_bytes = freed_bytes;
                        self.delete_errors = errors;
                        self.state = AppState::Done;
                        ctx.request_repaint();
                        break;
                    }
                }
            }
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 140.0);
                ui.heading(
                    egui::RichText::new("CLEANER")
                        .size(26.0)
                        .strong()
                        .color(egui::Color32::from_rgb(147, 197, 253)),
                );
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 58, 138))
                    .rounding(egui::Rounding::same(4.0_f32))
                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("100% PURE RUST")
                                .size(10.0)
                                .strong()
                                .color(egui::Color32::from_rgb(191, 219, 254)),
                        );
                    });
            });
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Reclaim disk space — Safely inspect and remove heavy build folders and dependencies")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(148, 163, 184)),
            );
            ui.add_space(8.0);
        });

        // Optional Toast
        if let Some((ref msg, time)) = self.toast_message {
            if time.elapsed().as_secs() < 3 {
                ui.vertical_centered(|ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(16, 185, 129))
                        .rounding(egui::Rounding::same(6.0_f32))
                        .inner_margin(egui::Margin::symmetric(14.0, 4.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(msg)
                                    .size(12.0)
                                    .strong()
                                    .color(egui::Color32::BLACK),
                            );
                        });
                });
                ui.add_space(4.0);
            } else {
                self.toast_message = None;
            }
        }

        ui.separator();
        ui.add_space(8.0);
    }
}

impl eframe::App for CleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_background_messages(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui);

            match self.state {
                AppState::Selecting => self.ui_selecting(ui, ctx),
                AppState::Scanning => self.ui_scanning(ui, ctx),
                AppState::Results => self.ui_results(ui, ctx),
                AppState::ConfirmClean => self.ui_confirm_clean(ui),
                AppState::Cleaning => self.ui_cleaning(ui, ctx),
                AppState::Done => self.ui_done(ui),
            }
        });
    }
}

impl CleanerApp {
    fn ui_selecting(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(24, 28, 42))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 52, 75)))
                    .rounding(egui::Rounding::same(10.0_f32))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.set_max_width(680.0);

                        ui.label(
                            egui::RichText::new("Workspace Directory to Scan")
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Select your projects root directory to recursively find cleanable build and dependency folders.",
                            )
                            .size(13.0)
                            .color(egui::Color32::from_rgb(160, 170, 190)),
                        );
                        ui.add_space(14.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Browse System...")
                                            .size(14.0)
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(egui::Color32::from_rgb(59, 130, 246))
                                    .rounding(egui::Rounding::same(6.0_f32))
                                    .min_size(egui::vec2(140.0, 36.0)),
                                )
                                .clicked()
                            {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    self.selected_path = Some(folder);
                                }
                            }

                            if let Some(ref path) = self.selected_path {
                                ui.label(
                                    egui::RichText::new(path.to_string_lossy())
                                        .monospace()
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(147, 197, 253)),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("No folder selected yet")
                                        .italics()
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        });

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(14.0);

                        ui.label(
                            egui::RichText::new("Target Categories to Clean")
                                .size(16.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(10.0);

                        egui::Grid::new("categories_grid")
                            .num_columns(2)
                            .spacing([40.0, 14.0])
                            .show(ui, |ui| {
                                for (i, cat) in self.categories.iter_mut().enumerate() {
                                    let targets_hint = cat.targets.join(", ");
                                    let (bg, fg) = category_colors(cat.label);
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut cat.enabled, "");
                                        egui::Frame::none()
                                            .fill(bg)
                                            .rounding(egui::Rounding::same(4.0_f32))
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(cat.icon)
                                                        .size(11.0)
                                                        .strong()
                                                        .color(fg),
                                                );
                                            });
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} ({})",
                                                cat.label, targets_hint
                                            ))
                                            .size(13.0),
                                        );
                                    });
                                    if i % 2 == 1 {
                                        ui.end_row();
                                    }
                                }
                            });

                        ui.add_space(26.0);

                        let any_enabled = self.categories.iter().any(|c| c.enabled);
                        let can_scan = self.selected_path.is_some() && any_enabled;

                        let scan_btn = egui::Button::new(
                            egui::RichText::new("Start Deep Scan")
                                .size(15.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(if can_scan {
                            egui::Color32::from_rgb(16, 185, 129)
                        } else {
                            egui::Color32::from_rgb(50, 60, 75)
                        })
                        .rounding(egui::Rounding::same(8.0_f32))
                        .min_size(egui::vec2(220.0, 42.0));

                        if ui.add_enabled(can_scan, scan_btn).clicked() {
                            if let Some(path) = self.selected_path.clone() {
                                self.results.clear();
                                self.scanned_count = 0;
                                self.current_scan_dir.clear();
                                self.search_query.clear();
                                self.filter_category = None;
                                self.state = AppState::Scanning;

                                let (tx, rx) = crossbeam_channel::unbounded();
                                self.scan_rx = Some(rx);
                                start_scan(path, self.categories.clone(), tx);
                                ctx.request_repaint();
                            }
                        }

                        if !can_scan {
                            ui.add_space(6.0);
                            let reason = if self.selected_path.is_none() {
                                "Please select a workspace folder first."
                            } else {
                                "Please enable at least one target category."
                            };
                            ui.label(
                                egui::RichText::new(reason)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(248, 113, 113)),
                            );
                        }
                    });
            });
        });
    }

    fn ui_scanning(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.spinner();
            ui.add_space(16.0);
            ui.heading(
                egui::RichText::new("Scanning workspace...")
                    .size(20.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!("Directories traversed: {}", self.scanned_count))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(160, 170, 190)),
            );
            ui.label(
                egui::RichText::new(format!("Targets found so far: {}", self.results.len()))
                    .size(15.0)
                    .strong()
                    .color(egui::Color32::from_rgb(52, 211, 153)),
            );

            let total_size: u64 = self.results.iter().map(|i| i.size_bytes).sum();
            ui.label(
                egui::RichText::new(format!("Identified size: {}", format_bytes(total_size)))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(147, 197, 253)),
            );

            ui.add_space(16.0);
            ui.label(
                egui::RichText::new(&self.current_scan_dir)
                    .size(11.0)
                    .monospace()
                    .color(egui::Color32::from_rgb(120, 130, 150)),
            );

            ui.add_space(24.0);
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Cancel Scan")
                            .size(13.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(239, 68, 68))
                    .rounding(egui::Rounding::same(6.0_f32))
                    .min_size(egui::vec2(120.0, 32.0)),
                )
                .clicked()
            {
                self.scan_rx = None;
                self.state = AppState::Selecting;
                ctx.request_repaint();
            }
        });
    }

    fn ui_results(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let total_count = self.results.len();
        let total_size: u64 = self.results.iter().map(|i| i.size_bytes).sum();
        let selected_count = self.results.iter().filter(|i| i.selected).count();
        let selected_size: u64 = self
            .results
            .iter()
            .filter(|i| i.selected)
            .map(|i| i.size_bytes)
            .sum();

        // 1. TOP SUMMARY CARD
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(22, 26, 40))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 52, 75)))
            .rounding(egui::Rounding::same(8.0_f32))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Folders Found")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 150, 175)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", total_count))
                                .size(20.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                    });

                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(24.0);

                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Total Reclaimable")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 150, 175)),
                        );
                        ui.label(
                            egui::RichText::new(format_bytes(total_size))
                                .size(20.0)
                                .strong()
                                .color(egui::Color32::from_rgb(52, 211, 153)),
                        );
                    });

                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(24.0);

                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Selected for Deletion")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 150, 175)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} ({})",
                                selected_count,
                                format_bytes(selected_size)
                            ))
                            .size(20.0)
                            .strong()
                            .color(if selected_count > 0 {
                                egui::Color32::from_rgb(251, 191, 36)
                            } else {
                                egui::Color32::GRAY
                            }),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_clean = selected_count > 0;
                        let clean_btn = egui::Button::new(
                            egui::RichText::new(format!("Delete Selected ({})", selected_count))
                                .size(14.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(if can_clean {
                            egui::Color32::from_rgb(220, 38, 38)
                        } else {
                            egui::Color32::from_rgb(65, 40, 40)
                        })
                        .rounding(egui::Rounding::same(6.0_f32))
                        .min_size(egui::vec2(170.0, 38.0));

                        if ui.add_enabled(can_clean, clean_btn).clicked() {
                            self.state = AppState::ConfirmClean;
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("New Scan")
                                        .size(13.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(55, 65, 81))
                                .rounding(egui::Rounding::same(6.0_f32))
                                .min_size(egui::vec2(100.0, 38.0)),
                            )
                            .clicked()
                        {
                            self.state = AppState::Selecting;
                        }
                    });
                });
            });

        ui.add_space(10.0);

        // 2. SEARCH AND FILTER TOOLBAR
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Search:").size(13.0).strong());
            let _edit = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Filter by project, folder or path...")
                    .desired_width(280.0),
            );
            if !self.search_query.is_empty() && ui.small_button("Clear").clicked() {
                self.search_query.clear();
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(14.0);

            // Sort button
            let sort_label = if self.sort_by_size_desc {
                "Sort: Largest First (v)"
            } else {
                "Sort: Smallest First (^)"
            };
            if ui.button(sort_label).clicked() {
                self.sort_by_size_desc = !self.sort_by_size_desc;
                if self.sort_by_size_desc {
                    self.results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
                } else {
                    self.results.sort_by(|a, b| a.size_bytes.cmp(&b.size_bytes));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Deselect All").clicked() {
                    for item in &mut self.results {
                        item.selected = false;
                    }
                }
                if ui.button("Select All").clicked() {
                    for item in &mut self.results {
                        item.selected = true;
                    }
                }
            });
        });

        ui.add_space(8.0);

        // Category Filter Pills
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Category:").size(12.0).color(egui::Color32::GRAY));

            let all_active = self.filter_category.is_none();
            let all_btn = ui.selectable_label(all_active, format!("All ({})", self.results.len()));
            if all_btn.clicked() {
                self.filter_category = None;
            }

            let mut categories_count = std::collections::BTreeMap::new();
            for item in &self.results {
                *categories_count.entry(item.category.clone()).or_insert(0) += 1;
            }

            for (cat, count) in categories_count {
                let is_active = self.filter_category.as_deref() == Some(&cat);
                let label = format!("{} ({})", cat, count);
                if ui.selectable_label(is_active, label).clicked() {
                    if is_active {
                        self.filter_category = None;
                    } else {
                        self.filter_category = Some(cat);
                    }
                }
            }
        });

        ui.add_space(8.0);

        // Filter results according to search_query and filter_category
        let query_lower = self.search_query.trim().to_lowercase();
        let active_cat = self.filter_category.clone();

        // 3. RESULTS LIST: CARDS WITH CLEAR VISIBILITY
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(18, 21, 31))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(38, 44, 62)))
            .rounding(egui::Rounding::same(8.0_f32))
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut displayed_count = 0;

                        for item in &mut self.results {
                            if let Some(ref cat) = active_cat {
                                if &item.category != cat {
                                    continue;
                                }
                            }

                            if !query_lower.is_empty() {
                                let match_proj = item.project_name.to_lowercase().contains(&query_lower);
                                let match_folder = item.folder_name.to_lowercase().contains(&query_lower);
                                let match_path = item.path.to_string_lossy().to_lowercase().contains(&query_lower);
                                if !match_proj && !match_folder && !match_path {
                                    continue;
                                }
                            }

                            displayed_count += 1;

                            let (cat_bg, cat_fg) = category_colors(&item.category);

                            // Individual card for this item
                            egui::Frame::none()
                                .fill(if item.selected {
                                    egui::Color32::from_rgb(26, 32, 48)
                                } else {
                                    egui::Color32::from_rgb(16, 18, 27)
                                })
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    if item.selected {
                                        egui::Color32::from_rgb(59, 130, 246)
                                    } else {
                                        egui::Color32::from_rgb(35, 40, 58)
                                    },
                                ))
                                .rounding(egui::Rounding::same(6.0_f32))
                                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                                .show(ui, |ui| {
                                    // Row 1: Checkbox, Category Badge, Folder to be deleted, Project Name, and Size
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut item.selected, "");

                                        // Category Tag
                                        egui::Frame::none()
                                            .fill(cat_bg)
                                            .rounding(egui::Rounding::same(4.0_f32))
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(&item.category)
                                                        .size(11.0)
                                                        .strong()
                                                        .color(cat_fg),
                                                );
                                            });

                                        ui.add_space(4.0);

                                        // Folder to delete badge
                                        egui::Frame::none()
                                            .fill(egui::Color32::from_rgb(60, 25, 30))
                                            .rounding(egui::Rounding::same(4.0_f32))
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("WILL DELETE: {}/", item.folder_name))
                                                        .size(11.0)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(252, 165, 165)),
                                                );
                                            });

                                        ui.add_space(6.0);

                                        // Project name and relative path
                                        ui.label(
                                            egui::RichText::new(format!("Project: {}", item.project_name))
                                                .size(14.0)
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        );

                                        ui.label(
                                            egui::RichText::new(format!("({})", item.relative_path))
                                                .size(11.5)
                                                .color(egui::Color32::from_rgb(148, 163, 184)),
                                        );

                                        // Size on right
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let size_text = format_bytes(item.size_bytes);
                                                let size_color = if item.size_bytes > 1024 * 1024 * 1024 {
                                                    egui::Color32::from_rgb(52, 211, 153) // Green for GB
                                                } else if item.size_bytes > 200 * 1024 * 1024 {
                                                    egui::Color32::from_rgb(251, 191, 36) // Yellow for >200MB
                                                } else {
                                                    egui::Color32::from_rgb(148, 163, 184)
                                                };

                                                ui.label(
                                                    egui::RichText::new(size_text)
                                                        .size(15.0)
                                                        .strong()
                                                        .color(size_color),
                                                );
                                            },
                                        );
                                    });

                                    ui.add_space(4.0);

                                    // Row 2: Full path display (clearly visible) and Action Buttons
                                    ui.horizontal(|ui| {
                                        ui.add_space(26.0); // Indent past checkbox

                                        let full_path = item.path.to_string_lossy().to_string();

                                        ui.label(
                                            egui::RichText::new(&full_path)
                                                .monospace()
                                                .size(11.5)
                                                .color(egui::Color32::from_rgb(148, 163, 184)),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("Reveal in Finder")
                                                    .on_hover_text("Open enclosing folder in file manager")
                                                    .clicked()
                                                {
                                                    open_in_file_manager(&item.path);
                                                }

                                                if ui
                                                    .small_button("Copy Path")
                                                    .on_hover_text("Copy full path to clipboard")
                                                    .clicked()
                                                {
                                                    ctx.copy_text(full_path.clone());
                                                    self.toast_message = Some((
                                                        "Path copied to clipboard!".to_string(),
                                                        std::time::Instant::now(),
                                                    ));
                                                }
                                            },
                                        );
                                    });
                                });

                            ui.add_space(4.0);
                        }

                        if displayed_count == 0 {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("No items match your search or filter.")
                                        .size(15.0)
                                        .color(egui::Color32::GRAY),
                                );
                                ui.add_space(40.0);
                            });
                        }
                    });
            });
    }

    fn ui_confirm_clean(&mut self, ui: &mut egui::Ui) {
        let selected_items: Vec<ScanItem> = self
            .results
            .iter()
            .filter(|i| i.selected)
            .cloned()
            .collect();
        let count = selected_items.len();
        let total_size: u64 = selected_items.iter().map(|i| i.size_bytes).sum();

        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(32, 22, 26))
                .stroke(egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(239, 68, 68)))
                .rounding(egui::Rounding::same(12.0_f32))
                .inner_margin(egui::Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_max_width(680.0);
                    ui.label(
                        egui::RichText::new("CONFIRM PERMANENT DELETION")
                            .size(20.0)
                            .strong()
                            .color(egui::Color32::from_rgb(248, 113, 113)),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "You have selected {} folders for deletion. Total space to be reclaimed: {}.",
                            count,
                            format_bytes(total_size)
                        ))
                        .size(14.0)
                        .color(egui::Color32::WHITE),
                    );
                    ui.label(
                        egui::RichText::new(
                            "The following directories and ALL files inside them will be permanently deleted from your disk:",
                        )
                        .size(12.0)
                        .italics()
                        .color(egui::Color32::from_rgb(252, 165, 165)),
                    );

                    ui.add_space(14.0);

                    // Scrollable list of items to be deleted
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 15, 18))
                        .rounding(egui::Rounding::same(6.0_f32))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    for item in &selected_items {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("• [{}]", item.folder_name))
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(248, 113, 113)),
                                            );
                                            ui.label(
                                                egui::RichText::new(&item.project_name)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.label(
                                                egui::RichText::new(item.path.to_string_lossy())
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(156, 163, 175)),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(format_bytes(item.size_bytes))
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(52, 211, 153)),
                                                    );
                                                },
                                            );
                                        });
                                    }
                                });
                        });

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        ui.add_space(140.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Yes, Delete Permanently")
                                        .size(14.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(220, 38, 38))
                                .rounding(egui::Rounding::same(6.0_f32))
                                .min_size(egui::vec2(190.0, 40.0)),
                            )
                            .clicked()
                        {
                            let (tx, rx) = crossbeam_channel::unbounded();
                            self.delete_rx = Some(rx);
                            self.delete_current = 0;
                            self.delete_total = selected_items.len();
                            self.delete_current_path.clear();
                            self.state = AppState::Cleaning;
                            start_delete(selected_items, tx);
                        }

                        ui.add_space(12.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Cancel - Keep Files")
                                        .size(14.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(75, 85, 99))
                                .rounding(egui::Rounding::same(6.0_f32))
                                .min_size(egui::vec2(140.0, 40.0)),
                            )
                            .clicked()
                        {
                            self.state = AppState::Results;
                        }
                    });
                });
        });
    }

    fn ui_cleaning(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.spinner();
            ui.add_space(16.0);
            ui.heading(
                egui::RichText::new("Deleting selected folders...")
                    .size(20.0)
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!(
                    "Progress: {} / {}",
                    self.delete_current, self.delete_total
                ))
                .size(15.0)
                .color(egui::Color32::from_rgb(160, 170, 190)),
            );

            let frac = if self.delete_total > 0 {
                self.delete_current as f32 / self.delete_total as f32
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(frac)
                    .show_percentage()
                    .desired_width(450.0),
            );

            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(&self.delete_current_path)
                    .size(11.5)
                    .monospace()
                    .color(egui::Color32::from_rgb(140, 150, 170)),
            );
        });
    }

    fn ui_done(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(20, 32, 28))
                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(16, 185, 129)))
                .rounding(egui::Rounding::same(10.0_f32))
                .inner_margin(egui::Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_max_width(580.0);
                    ui.label(
                        egui::RichText::new("CLEANUP COMPLETED SUCCESSFULLY!")
                            .size(22.0)
                            .strong()
                            .color(egui::Color32::from_rgb(52, 211, 153)),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Deleted: {} folders\nFreed disk space: {}",
                            self.deleted_count,
                            format_bytes(self.freed_bytes)
                        ))
                        .size(16.0)
                        .color(egui::Color32::WHITE),
                    );

                    if !self.delete_errors.is_empty() {
                        ui.add_space(14.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} folders failed to delete due to permission or file lock issues:",
                                self.delete_errors.len()
                            ))
                            .color(egui::Color32::from_rgb(248, 113, 113)),
                        );
                        for err in &self.delete_errors {
                            ui.label(
                                egui::RichText::new(err)
                                    .size(11.0)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(252, 165, 165)),
                            );
                        }
                    }

                    ui.add_space(24.0);

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Done / Scan Again")
                                    .size(14.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(16, 185, 129))
                            .rounding(egui::Rounding::same(6.0_f32))
                            .min_size(egui::vec2(160.0, 38.0)),
                        )
                        .clicked()
                    {
                        self.results.clear();
                        self.state = AppState::Selecting;
                    }
                });
        });
    }
}
