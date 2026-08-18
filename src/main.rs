// Windows 下以 GUI 子系统编译：不弹出控制台黑窗口
#![windows_subsystem = "windows"]

mod state;
mod store;

use egui::{Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use state::{now, MatchState, FULL_SECONDS, HALF_SECONDS};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 大屏布局状态：各元素的拖动偏移与独立缩放（持久化到 JSON）
struct DisplayLayout {
    offsets: HashMap<&'static str, Vec2>,
    /// 每个元素独立的缩放比例
    zooms: HashMap<&'static str, f32>,
    /// 选中的元素集合（Ctrl+点击 多选）
    selected: Vec<&'static str>,
    dragging: Option<&'static str>,
    drag_last: Pos2,
}

impl Default for DisplayLayout {
    fn default() -> Self {
        let mut s = Self {
            offsets: HashMap::new(),
            zooms: HashMap::new(),
            selected: Vec::new(),
            dragging: None,
            drag_last: Pos2::ZERO,
        };
        // 默认布局 = 当前调好的位置（首次打开/双击复位时使用）
        for id in ELEMENT_IDS {
            s.offsets.insert(id, default_offset(id));
            s.zooms.insert(id, default_zoom(id));
        }
        s
    }
}

/// 各元素默认拖动偏移（固化自调好的 display_layout.json）
fn default_offset(id: &str) -> Vec2 {
    match id {
        "event" => Vec2::new(0.023697915, 0.114323825),
        "phase" => Vec2::new(-0.00078124896, 0.6347225),
        "clock" => Vec2::new(0.0010416661, 0.66111153),
        "home" | "away" => Vec2::new(0.0, -0.07638892),
        "score" => Vec2::new(-0.0015625086, -0.048610996),
        _ => Vec2::ZERO,
    }
}

/// 各元素默认缩放（固化自调好的 display_layout.json）
fn default_zoom(id: &str) -> f32 {
    match id {
        "event" => 2.143589,
        "phase" => 1.1,
        "clock" => 0.56447387,
        "home" | "away" => 1.3310001,
        "score" => 1.7715611,
        _ => 1.0,
    }
}

/// 大屏布局持久化文件
const LAYOUT_FILE: &str = "data/display_layout.json";

/// 合法元素 id（用于把磁盘读回的字符串映射回静态引用）
const ELEMENT_IDS: [&str; 6] = ["event", "phase", "clock", "home", "away", "score"];

fn known_element_id(s: &str) -> Option<&'static str> {
    ELEMENT_IDS.iter().find(|&&id| id == s).copied()
}

/// 大屏布局落盘格式
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct LayoutSave {
    offsets: HashMap<String, [f32; 2]>,
    zooms: HashMap<String, f32>,
}

/// 保存大屏元素位置与缩放（拖动结束/缩放变化/双击复位时调用）
fn save_display_layout(lay: &DisplayLayout) {
    let save = LayoutSave {
        offsets: lay.offsets.iter().map(|(k, v)| (k.to_string(), [v.x, v.y])).collect(),
        zooms: lay.zooms.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    };
    std::fs::create_dir_all("data").ok();
    if let Ok(json) = serde_json::to_string_pretty(&save) {
        std::fs::write(LAYOUT_FILE, json).ok();
    }
}

/// 启动时加载已保存的布局（未知元素 id 忽略）
fn load_display_layout(lay: &mut DisplayLayout) {
    let Ok(text) = std::fs::read_to_string(LAYOUT_FILE) else {
        return;
    };
    let Ok(save) = serde_json::from_str::<LayoutSave>(&text) else {
        eprintln!("[warn] 大屏布局文件解析失败，使用默认布局");
        return;
    };
    for (k, v) in save.offsets {
        if let Some(id) = known_element_id(&k) {
            lay.offsets.insert(id, Vec2::new(v[0], v[1]));
        }
    }
    for (k, v) in save.zooms {
        if let Some(id) = known_element_id(&k) {
            lay.zooms.insert(id, v.clamp(0.3, 3.0));
        }
    }
}

// =====================================================================
// 主题
// =====================================================================
mod theme {
    use egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(13, 17, 23);
    pub const CARD: Color32 = Color32::from_rgb(22, 27, 34);
    pub const FIELD: Color32 = Color32::from_rgb(13, 17, 23);
    pub const BORDER: Color32 = Color32::from_rgb(48, 54, 61);
    pub const TEXT: Color32 = Color32::from_rgb(230, 237, 243);
    pub const MUTED: Color32 = Color32::from_rgb(139, 148, 158);
    pub const GREEN: Color32 = Color32::from_rgb(46, 160, 67);
    pub const GREEN_LIGHT: Color32 = Color32::from_rgb(126, 226, 160);
    pub const YELLOW: Color32 = Color32::from_rgb(210, 153, 34);
    pub const YELLOW_LIGHT: Color32 = Color32::from_rgb(242, 201, 76);
    pub const BLUE: Color32 = Color32::from_rgb(47, 129, 247);
    pub const BLUE_LIGHT: Color32 = Color32::from_rgb(142, 198, 255);
    pub const RED: Color32 = Color32::from_rgb(218, 54, 51);
    pub const GRAY_BTN: Color32 = Color32::from_rgb(33, 38, 45);
}
use theme::*;

/// 窗口标题栏图标素材（内嵌于 exe，随程序分发）
const APP_ICON_JPG: &[u8] = include_bytes!("../assets/app_icon.jpg");

/// 窗口标题栏图标：128x128 RGBA，仅首次调用时解码一次
fn window_icon() -> std::sync::Arc<egui::IconData> {
    static ICON: std::sync::OnceLock<std::sync::Arc<egui::IconData>> = std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        let mut rgba = vec![0u8; 128 * 128 * 4];
        if let Ok(img) = image::load_from_memory(APP_ICON_JPG) {
            let img = img.resize_exact(128, 128, image::imageops::FilterType::Lanczos3);
            rgba = img.to_rgba8().into_raw();
        }
        std::sync::Arc::new(egui::IconData { rgba, width: 128, height: 128 })
    })
    .clone()
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("足球比分控制台")
            .with_inner_size([749.0, 749.0])
            .with_icon(window_icon()),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "足球比分控制台",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

/// 全局深色主题
fn setup_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.window_rounding = 12.0.into();
    visuals.window_stroke = Stroke::new(1.0f32, BORDER);
    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.bg_fill = GRAY_BTN;
    visuals.widgets.inactive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.rounding = 8.0.into();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 54, 61);
    visuals.widgets.hovered.rounding = 8.0.into();
    visuals.widgets.active.bg_fill = BLUE;
    visuals.widgets.active.rounding = 8.0.into();
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(47, 129, 247, 90);
    visuals.button_frame = true;
    visuals.extreme_bg_color = FIELD;
    visuals.code_bg_color = FIELD;
    ctx.set_visuals(visuals);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(10.0, 10.0);
        s.spacing.button_padding = Vec2::new(14.0, 8.0);
        s.spacing.window_margin = 0.0.into();
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(15.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(15.0));
        s.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(20.0));
    });
}

struct App {
    data_dir: PathBuf,
    images_dir: PathBuf,
    state: MatchState,
    // 控制台输入缓冲
    home_input: String,
    away_input: String,
    event_input: String,
    score_home_input: String,
    score_away_input: String,
    /// 时间控制：设定比赛时间的分钟输入
    time_min_input: String,
    settings_collapsed: bool,
    // 大屏窗口控制（跨视口共享）
    show_display: Arc<AtomicBool>,
    want_close_display: Arc<AtomicBool>,
    fullscreen: Arc<AtomicBool>,
    fullscreen_applied: Arc<AtomicBool>,
    display_layout: Arc<Mutex<DisplayLayout>>,
    // 素材纹理缓存
    tex_background: Option<egui::TextureHandle>,
    tex_home: Option<egui::TextureHandle>,
    tex_away: Option<egui::TextureHandle>,
    loaded_version: u64,
    // UI 辅助
    status: Option<(String, Instant)>,
    reset_armed_at: Option<Instant>,
    // 系统时间跳变检测
    last_now: i64,            // 上一帧墙钟时间戳
    last_mono: Option<Instant>, // 上一帧单调时钟，用于测「真实流逝」
    clock_anomaly: bool,      // 系统时间异常（被更改）告警状态
    // 调试功能开关：连点左上角足球 5 次解锁，正常启动隐藏手动设置/时间控制
    debug_unlocked: bool,
    egg_clicks: u32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_cjk_fonts(&cc.egui_ctx);
        setup_theme(&cc.egui_ctx);
        // 关闭 egui 内置的 Ctrl+± 整体 UI 缩放：
        // 避免大屏选中元素缩放时其他元素/控制台窗口跟着联动
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);

        let data_dir = PathBuf::from("data");
        let images_dir = data_dir.join("images");
        std::fs::create_dir_all(&images_dir).ok();

        let state = store::load_state(&data_dir).unwrap_or_default();
        let mut app = Self {
            home_input: state.home_team.clone(),
            away_input: state.away_team.clone(),
            event_input: state.event_name.clone(),
            score_home_input: "0".into(),
            score_away_input: "0".into(),
            time_min_input: String::new(),
            settings_collapsed: false,
            data_dir,
            images_dir,
            state,
            show_display: Arc::new(AtomicBool::new(false)),
            want_close_display: Arc::new(AtomicBool::new(false)),
            fullscreen: Arc::new(AtomicBool::new(false)),
            fullscreen_applied: Arc::new(AtomicBool::new(false)),
            display_layout: {
                let mut layout = DisplayLayout::default();
                load_display_layout(&mut layout);
                Arc::new(Mutex::new(layout))
            },
            tex_background: None,
            tex_home: None,
            tex_away: None,
            loaded_version: u64::MAX,
            status: None,
            reset_armed_at: None,
            last_now: 0,
            last_mono: None,
            clock_anomaly: false,
            debug_unlocked: false,
            egg_clicks: 0,
        };
        app.load_textures(&cc.egui_ctx);
        println!("足球比分控制台已启动（数据目录: data/）");
        app
    }

    fn set_status(&mut self, msg: &str) {
        self.status = Some((msg.to_string(), Instant::now()));
    }

    fn persist(&mut self) {
        if let Err(e) = store::save_state(&self.data_dir, &self.state) {
            self.set_status(&format!("状态保存失败: {e}"));
        }
    }

    fn load_textures(&mut self, ctx: &egui::Context) {
        self.tex_background =
            load_texture(ctx, &self.images_dir, self.state.background_image.as_deref(), "background");
        self.tex_home = load_texture(ctx, &self.images_dir, self.state.home_logo.as_deref(), "home");
        self.tex_away = load_texture(ctx, &self.images_dir, self.state.away_logo.as_deref(), "away");
        self.loaded_version = self.state.image_version;
    }

    /// 45/90 分钟自动暂停（每帧检查，计时本身基于时间戳，不依赖定时器累加）。
    /// 标志语义为“阈值已越过”：无论开关是否勾选，越过即置位，
    /// 避免时间一次跨过两个阈值或开关反复切换导致的补暂停/重复暂停
    fn check_auto_pause(&mut self) {
        if !self.state.running {
            return;
        }
        let elapsed = self.state.elapsed_seconds_at(now());
        let mut crossed = false;
        let mut pause_now = false;
        let mut target: Option<i64> = None;
        let mut msg: Option<&str> = None;
        if elapsed >= HALF_SECONDS && !self.state.auto_paused_45 {
            self.state.auto_paused_45 = true;
            crossed = true;
            if self.state.auto_pause_45 {
                pause_now = true;
                target = Some(HALF_SECONDS);
                msg = Some("已到 45 分钟，自动暂停（半场结束）");
            }
        }
        if elapsed >= FULL_SECONDS && !self.state.auto_paused_90 {
            self.state.auto_paused_90 = true;
            crossed = true;
            // 时间已达全场节点 → 必然已在下半场（覆盖程序关闭期间直接跨过 45 的场景）
            self.state.second_half_started = true;
            if self.state.auto_pause_90 {
                pause_now = true;
                target = Some(FULL_SECONDS);
                msg = Some("已到 90 分钟，自动暂停（全场结束）");
            }
        }
        if pause_now {
            // 把时钟钉在阈值整点：程序关闭期间越过阈值时，
            // 将多走的部分补进暂停累计，恢复后从 45:00/90:00 继续
            if let Some(t) = target {
                let over = elapsed - t;
                if over > 0 {
                    self.state.paused_seconds += over;
                }
            }
            self.state.pause(now());
        }
        if crossed {
            self.persist();
            if let Some(m) = msg {
                self.set_status(m);
            }
        }
    }

    fn upload_image(&mut self, kind: &str) {
        let label = match kind {
            "background" => "比赛主背景图",
            "home" => "主队队徽",
            _ => "客队队徽",
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(&format!("选择{label}"))
            .add_filter("图片", &["png", "jpg", "jpeg", "webp", "gif"])
            .pick_file()
        else {
            return;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
            self.set_status("仅支持 png/jpg/jpeg/webp/gif 图片");
            return;
        }
        let new_name = format!("{kind}.{ext}");
        let old = match kind {
            "background" => self.state.background_image.clone(),
            "home" => self.state.home_logo.clone(),
            _ => self.state.away_logo.clone(),
        };
        if let Some(old) = &old {
            if old != &new_name {
                std::fs::remove_file(self.images_dir.join(old)).ok();
            }
        }
        match std::fs::copy(&path, self.images_dir.join(&new_name)) {
            Ok(_) => {
                match kind {
                    "background" => self.state.background_image = Some(new_name),
                    "home" => self.state.home_logo = Some(new_name),
                    _ => self.state.away_logo = Some(new_name),
                }
                self.state.image_version = self.state.image_version.wrapping_add(1);
                self.persist();
                self.set_status(&format!("{label}已更新"));
            }
            Err(e) => self.set_status(&format!("素材保存失败: {e}")),
        }
    }

    fn spawn_display_if_needed(&self, ctx: &egui::Context) {
        if !self.show_display.load(Ordering::SeqCst) {
            return;
        }
        let snap = self.state.clone();
        let bg = tex_info(&self.tex_background);
        let home = tex_info(&self.tex_home);
        let away = tex_info(&self.tex_away);
        let fullscreen = self.fullscreen.clone();
        let fullscreen_applied = self.fullscreen_applied.clone();
        let show_display = self.show_display.clone();
        let want_close = self.want_close_display.clone();
        let disp_layout = self.display_layout.clone();

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("scoreboard-display"),
            egui::ViewportBuilder::default()
                .with_title("足球比分大屏")
                .with_inner_size([1280.0, 720.0])
                .with_icon(window_icon()),
            move |ctx, _class| {
                render_display(
                    ctx,
                    &snap,
                    bg,
                    home,
                    away,
                    &fullscreen,
                    &fullscreen_applied,
                    &show_display,
                    &want_close,
                    &disp_layout,
                );
            },
        );
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 系统时间跳变检测：用单调时钟测「真实流逝」，与「墙钟流逝」比对。
        // 仅当两者明显不符（墙钟相对真实时间回拨/快进）才视为系统时间被改，
        // 避免窗口切走/最小化使 egui 暂停重绘产生的单帧大间隔误报。
        let t = now();
        let mono = Instant::now();
        if self.last_now > 0 {
            let mono_dt = self
                .last_mono
                .map(|m| mono.saturating_duration_since(m).as_secs_f64())
                .unwrap_or(0.0);
            let wall_dt = (t - self.last_now) as f64;
            let back = wall_dt < mono_dt - 1.0; // 墙钟相对真实时间回拨 >1s
            let fwd = wall_dt > mono_dt + 5.0; // 墙钟相对真实时间快进 >5s
            if back || fwd {
                self.clock_anomaly = true;
            }
        }
        self.last_now = t;
        self.last_mono = Some(mono);

        // 时钟走时 + 闪烁效果需要持续重绘；异常时提高帧率以驱动呼吸动画
        ctx.request_repaint_after(if self.clock_anomaly {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(200)
        });

        // eframe 可能按系统浅色主题覆盖 visuals，每帧兜底恢复深色主题
        if ctx.style().visuals.panel_fill != BG {
            setup_theme(ctx);
        }

        if self.loaded_version != self.state.image_version {
            self.load_textures(ctx);
        }
        self.check_auto_pause();
        self.spawn_display_if_needed(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(16.0, 0.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main-scroll")
                    .show(ui, |ui| {
                        ui.add_space(14.0);
                        render_header(ui, self);
                        ui.add_space(6.0);
                        render_live_preview(ui, &self.state);

                        // 单列布局：比赛设置在上，比赛控制在下
                        let upload_kind = render_settings_panel(ui, self);
                        render_match_panel(ui, self);
                        if let Some(kind) = upload_kind {
                            self.upload_image(kind);
                        }

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("数据: data/match.json + data/images/　·　重启后自动恢复比赛状态")
                                    .weak()
                                    .size(12.5),
                            );
                        });
                        ui.add_space(8.0);
                    });
            });
    }
}

// =====================================================================
// 控制台各区块
// =====================================================================

fn render_header(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let football = ui.add(
            egui::Label::new(RichText::new("⚽").size(26.0)).sense(egui::Sense::click()),
        );
        if football.clicked() {
            app.egg_clicks += 1;
            if app.egg_clicks >= 5 && !app.debug_unlocked {
                app.debug_unlocked = true;
                app.set_status("调试功能已解锁：手动设置 / 时间控制");
            } else if !app.debug_unlocked {
                app.set_status(&format!("再点 {} 次解锁调试功能", 5 - app.egg_clicks));
            }
        }
        ui.label(RichText::new("足球比分控制台").size(22.0).strong().color(TEXT));
        ui.add_space(6.0);
        // 系统时间异常告警：显示在状态药丸同一位置，红色呼吸闪烁，点击关闭
        if app.clock_anomaly {
            let tnow = ui.input(|i| i.time);
            let pulse = (tnow * 3.0).sin() * 0.5 + 0.5; // 0..1 呼吸
            let bg = Color32::from_rgba_premultiplied(
                150, 30, 30,
                (110.0 + 145.0 * pulse) as u8, // 背景亮度明显呼吸
            );
            let stroke =
                Stroke::new(1.5f32, RED.gamma_multiply((0.5 + 0.5 * pulse) as f32));
            let resp = egui::Frame::none()
                .fill(bg)
                .stroke(stroke)
                .rounding(16.0)
                .inner_margin(egui::Margin::symmetric(12.0, 3.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("⚠ 系统时间已变更，计时可能不准")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(255, 210, 210)),
                    );
                });
            if resp.response.clicked() {
                app.clock_anomaly = false;
            }
        } else if let Some((msg, t)) = &mut app.status {
            if t.elapsed() < Duration::from_secs(4) {
                chip(ui, msg, GREEN_LIGHT, Color32::from_rgb(18, 38, 30));
            } else {
                app.status = None;
            }
        }
        // 右上角：一键投放副屏 / LED 大屏
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.show_display.load(Ordering::SeqCst) {
                if ghost_button(ui, "关闭大屏").clicked() {
                    app.want_close_display.store(true, Ordering::SeqCst);
                }
                let fs = app.fullscreen.load(Ordering::SeqCst);
                if ghost_button(ui, if fs { "退出全屏" } else { "全屏" }).clicked() {
                    app.fullscreen.store(!fs, Ordering::SeqCst);
                }
            } else if action_button(ui, "投放大屏", GREEN, true, 96.0).clicked() {
                app.show_display.store(true, Ordering::SeqCst);
                app.set_status("大屏窗口已打开，拖到副屏后按 F 全屏");
            }
        });
    });
    ui.add_space(8.0);
    ui.separator();
}

fn render_live_preview(ui: &mut egui::Ui, state: &MatchState) {
    card(ui, "实况预览", |ui| {
        let elapsed = state.elapsed_seconds_at(now());
        let w = ui.available_width();
        // 固定宽度盒 + 对称留白实现精确水平居中（horizontal 子项总是占满可用宽，Align::Center 无法居中）
        const CLOCK_W: f32 = 150.0;
        const PILL_W: f32 = 110.0;
        const SCORE_W: f32 = 140.0;
        let content = CLOCK_W + 26.0 + PILL_W + 26.0 + SCORE_W + 60.0;
        let pad = ((w - content) / 2.0).max(0.0);
        ui.allocate_ui(Vec2::new(w, 96.0), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(pad);
                // 时钟
                let clock_color = if !state.started() {
                    MUTED
                } else if elapsed >= FULL_SECONDS {
                    RED
                } else if state.running {
                    TEXT
                } else {
                    YELLOW_LIGHT
                };
                ui.allocate_ui(Vec2::new(CLOCK_W, 96.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(fmt_clock(elapsed))
                                .monospace()
                                .strong()
                                .size(48.0)
                                .color(clock_color),
                        );
                    });
                });
                ui.add_space(26.0);

                // 阶段 + 状态胶囊
                ui.allocate_ui(Vec2::new(PILL_W, 96.0), |ui| {
                    ui.vertical_centered(|ui| {
                        let (phase, fg, bgc) = if !state.started() {
                            ("未开赛", MUTED, Color32::from_rgb(48, 54, 61))
                        } else if elapsed >= HALF_SECONDS && state.second_half_started {
                            ("下半场", BLUE_LIGHT, Color32::from_rgb(16, 30, 48))
                        } else {
                            ("上半场", GREEN_LIGHT, Color32::from_rgb(18, 38, 30))
                        };
                        pill_label(ui, phase, fg, bgc);
                        ui.add_space(6.0);
                        let (st, fg, bgc) = if !state.started() {
                            ("等待开始", MUTED, Color32::from_rgb(48, 54, 61))
                        } else if state.running {
                            ("● 进行中", GREEN_LIGHT, Color32::from_rgb(18, 38, 30))
                        } else {
                            ("已暂停", YELLOW_LIGHT, Color32::from_rgb(40, 32, 14))
                        };
                        pill_label(ui, st, fg, bgc);
                    });
                });
                ui.add_space(26.0);

                // 比分
                ui.allocate_ui(Vec2::new(SCORE_W, 96.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(format!("{} : {}", state.home_score, state.away_score))
                                .strong()
                                .size(36.0)
                                .color(TEXT),
                        );
                    });
                });
                ui.add_space(pad);
            });
        });
    });
}

fn render_match_panel(ui: &mut egui::Ui, app: &mut App) {
    card(ui, "比赛控制", |ui| {
        let w = ui.available_width();
        // 比分区：队名行 / 比分行（冒号同行）/ +1 按钮行，固定高度并精确居中
        const COL: f32 = 150.0;
        const GAP: f32 = 34.0;
        let pad = ((w - (COL * 2.0 + GAP + 40.0)) / 2.0).max(0.0);
        ui.allocate_ui(Vec2::new(w, 140.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.allocate_ui(Vec2::new(w, 22.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.allocate_ui(Vec2::new(COL, 22.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new(app.state.home_team.clone()).size(14.0).color(MUTED));
                                });
                            });
                            ui.add_space(GAP);
                            ui.allocate_ui(Vec2::new(COL, 22.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new(app.state.away_team.clone()).size(14.0).color(MUTED));
                                });
                            });
                            ui.add_space(pad);
                        });
                    });
                });
                ui.allocate_ui(Vec2::new(w, 52.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.allocate_ui(Vec2::new(COL, 52.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(app.state.home_score.to_string())
                                            .strong()
                                            .size(38.0)
                                            .color(TEXT),
                                    );
                                });
                            });
                            ui.allocate_ui(Vec2::new(GAP, 52.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new(":").size(30.0).strong().color(MUTED));
                                });
                            });
                            ui.allocate_ui(Vec2::new(COL, 52.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(app.state.away_score.to_string())
                                            .strong()
                                            .size(38.0)
                                            .color(TEXT),
                                    );
                                });
                            });
                            ui.add_space(pad);
                        });
                    });
                });
                ui.allocate_ui(Vec2::new(w, 40.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.allocate_ui(Vec2::new(COL, 40.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    if action_button(ui, "+1 进球", GREEN, true, 112.0).clicked() {
                                        app.state.home_score = (app.state.home_score + 1).min(99);
                                        app.persist();
                                    }
                                });
                            });
                            ui.add_space(GAP);
                            ui.allocate_ui(Vec2::new(COL, 40.0), |ui| {
                                ui.vertical_centered(|ui| {
                                    if action_button(ui, "+1 进球", GREEN, true, 112.0).clicked() {
                                        app.state.away_score = (app.state.away_score + 1).min(99);
                                        app.persist();
                                    }
                                });
                            });
                            ui.add_space(pad);
                        });
                    });
                });
            });
        });
        ui.add_space(8.0);
        // 常驻：比分清零（比赛中改分，不打断计时）
        ui.allocate_ui(Vec2::new(w, 36.0), |ui| {
            ui.centered_and_justified(|ui| {
                if ghost_button(ui, "比分清零").clicked() {
                    app.state.home_score = 0;
                    app.state.away_score = 0;
                    app.persist();
                    app.set_status("比分已清零");
                }
            });
        });
        ui.add_space(8.0);

        // 以下为调试功能：正常启动隐藏，连点左上角 ⚽ 5 次解锁
        if app.debug_unlocked {
            ui.allocate_ui(Vec2::new(w, 36.0), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("手动设置").color(MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.score_home_input)
                            .desired_width(52.0)
                            .hint_text("0"),
                    );
                    ui.label(RichText::new(":").color(MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.score_away_input)
                            .desired_width(52.0)
                            .hint_text("0"),
                    );
                    if ghost_button(ui, "应用").clicked() {
                        let h = app.score_home_input.trim().parse::<u32>().unwrap_or(0).min(99);
                        let a = app.score_away_input.trim().parse::<u32>().unwrap_or(0).min(99);
                        app.state.home_score = h;
                        app.state.away_score = a;
                        app.persist();
                        app.set_status("比分已更新");
                    }
                });
            });
            ui.add_space(8.0);
            // 时间控制：直接设定比赛时间（分:秒，如 46:30；纯数字按分钟），用于补时/校准等场景
            ui.allocate_ui(Vec2::new(w, 36.0), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("时间控制").color(MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.time_min_input)
                            .desired_width(70.0)
                            .hint_text("46:30"),
                    );
                    ui.label(RichText::new("分:秒").color(MUTED));
                    if ghost_button(ui, "设定").clicked() {
                        let t = app.time_min_input.trim();
                        let secs = if let Some((m, s)) = t.split_once(':') {
                            m.trim().parse::<i64>().unwrap_or(0) * 60
                                + s.trim().parse::<i64>().unwrap_or(0)
                        } else {
                            t.parse::<i64>().unwrap_or(0) * 60
                        };
                        let secs = secs.clamp(0, 180 * 60);
                        app.state.set_elapsed(secs, now());
                        app.persist();
                        app.set_status(&format!("比赛时间已设定为 {}", fmt_clock(secs)));
                    }
                });
            });
            ui.add_space(8.0);
        }
        ui.separator();
        ui.add_space(8.0);
        // 主控制按钮集合：状态按钮 + 重置按钮并排，适当居中不顶满整行
        let (label, color) = if !app.state.started() {
            ("开始比赛", GREEN)
        } else if app.state.running {
            ("暂停", YELLOW)
        } else {
            ("继续", BLUE)
        };
        // 90 分钟全场结束后，计时器不可再继续
        let finished = app.state.auto_paused_90;
        const BTN_W: f32 = 176.0;
        const RST_W: f32 = 108.0;
        const BTN_GAP: f32 = 16.0;
        let pad = ((w - (BTN_W + RST_W + BTN_GAP + 40.0)) / 2.0).max(0.0);
        ui.allocate_ui(Vec2::new(w, 44.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.allocate_ui(Vec2::new(BTN_W, 44.0), |ui| {
                        ui.vertical_centered(|ui| {
                            if ui
                                .add_enabled(
                                    !finished,
                                    egui::Button::new(
                                        RichText::new(label)
                                            .color(Color32::WHITE)
                                            .size(16.0)
                                            .strong(),
                                    )
                                    .fill(color)
                                    .stroke(Stroke::NONE)
                                    .rounding(10.0)
                                    .min_size(Vec2::new(BTN_W, 42.0)),
                                )
                                .clicked()
                            {
                                if !app.state.started() {
                                    app.state.start(now());
                                    app.set_status("比赛开始");
                                } else if app.state.running {
                                    app.state.pause(now());
                                    app.set_status("比赛已暂停");
                                } else {
                                    app.state.resume(now());
                                    app.set_status("比赛继续");
                                }
                                app.persist();
                            }
                        });
                    });
                    ui.add_space(BTN_GAP);
                    ui.allocate_ui(Vec2::new(RST_W, 44.0), |ui| {
                        ui.vertical_centered(|ui| {
                            let armed = app
                                .reset_armed_at
                                .map(|t| t.elapsed() < Duration::from_secs(3))
                                .unwrap_or(false);
                            let reset_label = if armed { "再点确认" } else { "重置" };
                            if action_button(ui, reset_label, RED, true, RST_W).clicked() {
                                if armed {
                                    app.state.reset();
                                    app.reset_armed_at = None;
                                    app.persist();
                                    app.set_status("比赛已重置（保留球队与素材）");
                                } else {
                                    app.reset_armed_at = Some(Instant::now());
                                    app.set_status("3 秒内再次点击 [重置] 确认");
                                }
                            }
                        });
                    });
                    ui.add_space(pad);
                });
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut app.state.auto_pause_45, "45 分钟自动暂停")
                .changed()
            {
                app.persist();
            }
            ui.add_space(10.0);
            if ui
                .checkbox(&mut app.state.auto_pause_90, "90 分钟自动暂停")
                .changed()
            {
                app.persist();
            }
        });
    });
}

/// 常见球衣颜色调色板
const JERSEY_COLORS: [(&str, [u8; 3]); 10] = [
    ("红", [227, 66, 52]),
    ("橙", [240, 140, 40]),
    ("黄", [242, 201, 76]),
    ("绿", [46, 160, 67]),
    ("青", [40, 180, 190]),
    ("蓝", [47, 129, 247]),
    ("紫", [140, 90, 220]),
    ("粉", [235, 90, 150]),
    ("白", [240, 244, 248]),
    ("黑", [35, 39, 45]),
];

/// 球衣色圆点选择器，返回是否修改
fn jersey_swatches(ui: &mut egui::Ui, current: &mut [u8; 3]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        // 收紧圆点间距，保证整行不超出列宽
        ui.spacing_mut().item_spacing.x = 4.0;
        for (name, c) in JERSEY_COLORS {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
            let col = Color32::from_rgb(c[0], c[1], c[2]);
            let painter = ui.painter();
            painter.circle_filled(rect.center(), 7.0, col);
            if *current == c {
                painter.circle_stroke(rect.center(), 8.5, Stroke::new(1.5f32, TEXT));
            } else if resp.hovered() {
                painter.circle_stroke(rect.center(), 8.5, Stroke::new(1.0f32, MUTED));
            }
            if resp.clicked() {
                *current = c;
                changed = true;
            }
            resp.on_hover_text(name);
        }
    });
    changed
}

/// 比赛设置（球队 + 赛事 + 素材合并，保存后可折叠）
fn render_settings_panel(ui: &mut egui::Ui, app: &mut App) -> Option<&'static str> {
    let mut upload: Option<&'static str> = None;
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0f32, BORDER))
        .rounding(12.0)
        .inner_margin(egui::Margin::symmetric(18.0, 14.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            // 标题行 + 折叠按钮
            ui.horizontal(|ui| {
                ui.label(RichText::new("比赛设置").size(13.0).strong().color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ghost_button(ui, if app.settings_collapsed { "展开" } else { "折叠" }).clicked() {
                        app.settings_collapsed = !app.settings_collapsed;
                    }
                });
            });
            if !app.settings_collapsed {
                ui.add_space(10.0);
                // 赛事名称 + 投放开关
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [70.0, 22.0],
                        egui::Label::new(RichText::new("赛事名称").color(MUTED)),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.event_input)
                            .desired_width(210.0)
                            .hint_text("如：2026 中超联赛·第 20 轮"),
                    );
                    if ui
                        .checkbox(&mut app.state.show_event_name, "投屏显示")
                        .changed()
                    {
                        app.persist();
                    }
                });
                ui.add_space(6.0);
                // 主队名称 + 球衣色
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [70.0, 22.0],
                        egui::Label::new(RichText::new("主队名称").color(MUTED)),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.home_input)
                            .desired_width(126.0)
                            .hint_text("如：上海申花"),
                    );
                    ui.add_space(4.0);
                    if jersey_swatches(ui, &mut app.state.home_color) {
                        app.persist();
                    }
                    if ui
                        .checkbox(&mut app.state.show_home_name, "投屏显示")
                        .changed()
                    {
                        app.persist();
                    }
                });
                ui.add_space(6.0);
                // 客队名称 + 球衣色
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [70.0, 22.0],
                        egui::Label::new(RichText::new("客队名称").color(MUTED)),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.away_input)
                            .desired_width(126.0)
                            .hint_text("如：北京国安"),
                    );
                    ui.add_space(4.0);
                    if jersey_swatches(ui, &mut app.state.away_color) {
                        app.persist();
                    }
                    if ui
                        .checkbox(&mut app.state.show_away_name, "投屏显示")
                        .changed()
                    {
                        app.persist();
                    }
                });
                ui.add_space(8.0);
                ui.allocate_ui(Vec2::new(ui.available_width(), 40.0), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if action_button(ui, "保存设置", BLUE, true, 112.0).clicked() {
                            let h = app.home_input.trim();
                            let a = app.away_input.trim();
                            if !h.is_empty() {
                                app.state.home_team = h.to_string();
                            }
                            if !a.is_empty() {
                                app.state.away_team = a.to_string();
                            }
                            app.state.event_name = app.event_input.trim().to_string();
                            app.persist();
                            app.set_status("比赛设置已保存");
                            app.settings_collapsed = true;
                        }
                    });
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
                // 比赛素材
                if asset_row(ui, "主背景图", &app.tex_background, "上传背景图") {
                    upload = Some("background");
                }
                if asset_row(ui, "主队队徽", &app.tex_home, "上传主队徽") {
                    upload = Some("home");
                }
                if asset_row(ui, "客队队徽", &app.tex_away, "上传客队徽") {
                    upload = Some("away");
                }
            }
        });
    ui.add_space(2.0);
    upload
}

// =====================================================================
// 控制台 UI 组件
// =====================================================================

/// 卡片容器：圆角 + 描边 + 标题
fn card<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let frame = egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0f32, BORDER))
        .rounding(12.0)
        .inner_margin(egui::Margin::symmetric(18.0, 14.0));
    let resp = frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(13.0).strong().color(MUTED));
        });
        ui.add_space(10.0);
        add_contents(ui)
    });
    ui.add_space(2.0);
    resp
}

/// 胶囊标签
fn pill_label(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(20.0)
        .inner_margin(egui::Margin::symmetric(14.0, 4.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(15.0).strong().color(fg));
        });
}

fn chip(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0f32, fg.gamma_multiply(0.4)))
        .rounding(16.0)
        .inner_margin(egui::Margin::symmetric(12.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(13.0).color(fg));
        });
}

/// 主操作按钮（禁用时自动变灰）
fn action_button(
    ui: &mut egui::Ui,
    text: &str,
    fill: Color32,
    enabled: bool,
    width: f32,
) -> egui::Response {
    let fill = if enabled { fill } else { GRAY_BTN };
    let text_color = if enabled {
        Color32::WHITE
    } else {
        Color32::from_rgb(148, 157, 168)
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).color(text_color).size(15.0).strong())
            .fill(fill)
            .stroke(Stroke::NONE)
            .rounding(8.0)
            .min_size(Vec2::new(width, 38.0)),
    )
}

/// 次级按钮（描边幽灵样式）
fn ghost_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(TEXT).size(14.0))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0f32, BORDER))
            .rounding(8.0)
            .min_size(Vec2::new(72.0, 32.0)),
    )
}

/// 素材行：缩略图 + 名称 + 上传按钮
fn asset_row(ui: &mut egui::Ui, label: &str, tex: &Option<egui::TextureHandle>, btn: &str) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.add_sized(
            [86.0, 22.0],
            egui::Label::new(RichText::new(label).color(TEXT)).wrap_mode(egui::TextWrapMode::Extend),
        );
        ui.add_space(4.0);
        let thumb = egui::Frame::none()
            .fill(FIELD)
            .stroke(Stroke::new(1.0f32, BORDER))
            .rounding(8.0)
            .inner_margin(2.0)
            .show(ui, |ui| match tex {
                Some(t) => {
                    ui.add(
                        egui::Image::new((t.id(), t.size_vec2()))
                            .max_size(Vec2::splat(50.0))
                            .rounding(6.0),
                    );
                }
                None => {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(50.0), egui::Sense::hover());
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "无",
                        FontId::proportional(15.0),
                        MUTED,
                    );
                }
            });
        let _ = thumb;
        ui.add_space(8.0);
        clicked = ghost_button(ui, btn).clicked();
        if tex.is_some() {
            ui.label(RichText::new("已上传").color(GREEN_LIGHT).size(13.0));
        }
    });
    clicked
}

// =====================================================================
// 大屏渲染（独立视口）
// =====================================================================

#[allow(clippy::too_many_arguments)]
fn render_display(
    ctx: &egui::Context,
    snap: &MatchState,
    bg: Option<(egui::TextureId, Vec2)>,
    home_logo: Option<(egui::TextureId, Vec2)>,
    away_logo: Option<(egui::TextureId, Vec2)>,
    fullscreen: &Arc<AtomicBool>,
    fullscreen_applied: &Arc<AtomicBool>,
    show_display: &Arc<AtomicBool>,
    want_close: &Arc<AtomicBool>,
    layout: &Arc<Mutex<DisplayLayout>>,
) {
    // 关闭请求：窗口 X / Esc / 控制台按钮
    let close_requested = ctx.input(|i| i.viewport().close_requested());
    if close_requested
        || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        || want_close.swap(false, Ordering::SeqCst)
    {
        show_display.store(false, Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        return;
    }

    // 全屏状态同步（控制台按钮）
    let want_fs = fullscreen.load(Ordering::SeqCst);
    if want_fs != fullscreen_applied.load(Ordering::SeqCst) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(want_fs));
        fullscreen_applied.store(want_fs, Ordering::SeqCst);
    }
    // 大屏内按 F 切换全屏（点击空白全屏移到元素命中判定之后，避免与拖动冲突）
    if ctx.input(|i| i.key_pressed(egui::Key::F)) {
        let v = !fullscreen.load(Ordering::SeqCst);
        fullscreen.store(v, Ordering::SeqCst);
        fullscreen_applied.store(v, Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(v));
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(Color32::BLACK))
        .show(ctx, |ui| {
            let rect = ctx.screen_rect();
            let painter = ui.painter();
            let h = rect.height();
            let w = rect.width();

            let mut lay = layout.lock().unwrap();

            // 缩放仅作用于选中元素：Ctrl + - / =（或 Ctrl+滚轮），Ctrl + 0 恢复
            // 注意：ctx.input 持有写锁，闭包内不能再调 ctx.request_repaint（会死锁）
            let mut zoom_changed = false;
            ctx.input(|i| {
                if i.modifiers.ctrl && !lay.selected.is_empty() {
                    let factor = if i.key_pressed(egui::Key::Minus) {
                        1.0 / 1.1
                    } else if i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus) {
                        1.1
                    } else {
                        i.zoom_delta()
                    };
                    if i.key_pressed(egui::Key::Num0) {
                        for &id in &lay.selected.clone() {
                            lay.zooms.insert(id, 1.0);
                        }
                        zoom_changed = true;
                    } else if (factor - 1.0).abs() > 1e-4 {
                        for &id in &lay.selected.clone() {
                            let z = lay.zooms.get(id).copied().unwrap_or(1.0);
                            lay.zooms.insert(id, (z * factor).clamp(0.3, 3.0));
                        }
                        zoom_changed = true;
                    }
                }
            });
            if zoom_changed {
                save_display_layout(&lay);
                ctx.request_repaint();
            }

            // 背景图（cover 裁剪）
            if let Some((id, size)) = bg {
                let dst = fit_cover(rect, size);
                painter.image(
                    id,
                    dst,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            let elapsed = snap.elapsed_seconds_at(now());

            // 各元素基准位置 + 拖动偏移；缩放只改元素自身尺寸
            // 偏移按窗口尺寸比例存储：全屏/改窗口大小后相对位置保持不变
            let off = |id: &'static str| {
                let n = lay.offsets.get(id).copied().unwrap_or(Vec2::ZERO);
                Vec2::new(n.x * w, n.y * h)
            };
            let z_event = lay.zooms.get("event").copied().unwrap_or(1.0);
            let z_phase = lay.zooms.get("phase").copied().unwrap_or(1.0);
            let z_clock = lay.zooms.get("clock").copied().unwrap_or(1.0);
            let z_home = lay.zooms.get("home").copied().unwrap_or(1.0);
            let z_away = lay.zooms.get("away").copied().unwrap_or(1.0);
            let z_score = lay.zooms.get("score").copied().unwrap_or(1.0);
            let event_visible = snap.show_event_name && !snap.event_name.is_empty();
            let event_pos = rect.center_top() + Vec2::new(0.0, h * 0.038) + off("event");
            let phase_pos = rect.center_top() + Vec2::new(0.0, h * 0.065) + off("phase");
            let clock_pos = rect.center_top() + Vec2::new(0.0, h * 0.15) + off("clock");
            let mid_y = rect.center().y + h * 0.10;
            let home_pos = Pos2::new(rect.center().x - w * 0.28, mid_y) + off("home");
            let away_pos = Pos2::new(rect.center().x + w * 0.28, mid_y) + off("away");
            let score_pos = Pos2::new(rect.center().x, mid_y) + off("score");

            let phase_text = if !snap.started() {
                "未开赛"
            } else if elapsed >= HALF_SECONDS && snap.second_half_started {
                "下半场"
            } else {
                "上半场"
            };

            // 命中区域（按实际文本/卡片尺寸，随各自缩放）
            let mut elements: Vec<(&'static str, Rect)> = Vec::new();
            if event_visible {
                let g = painter.layout_no_wrap(
                    snap.event_name.clone(),
                    FontId::proportional(h * 0.034 * z_event),
                    Color32::WHITE,
                );
                elements.push(("event", Rect::from_center_size(event_pos, g.size()).expand(8.0)));
            }
            {
                let pz = z_phase;
                let g = painter.layout_no_wrap(
                    phase_text.to_string(),
                    FontId::proportional(h * 0.034 * pz),
                    Color32::WHITE,
                );
                let size = Vec2::new(g.size().x + h * 0.06 * pz, g.size().y + h * 0.022 * pz);
                elements.push(("phase", Rect::from_center_size(phase_pos, size).expand(6.0)));
            }
            {
                let g = painter.layout_no_wrap(
                    fmt_clock(elapsed),
                    clock_font(h * 0.155 * z_clock),
                    Color32::WHITE,
                );
                elements.push(("clock", Rect::from_center_size(clock_pos, g.size()).expand(8.0)));
            }
            {
                let ph = h * 0.46 * z_home;
                elements.push((
                    "home",
                    Rect::from_center_size(home_pos - Vec2::new(0.0, ph * 0.06), Vec2::new(w * 0.24, ph)),
                ));
            }
            {
                let ph = h * 0.46 * z_away;
                elements.push((
                    "away",
                    Rect::from_center_size(away_pos - Vec2::new(0.0, ph * 0.06), Vec2::new(w * 0.24, ph)),
                ));
            }
            {
                let g = painter.layout_no_wrap(
                    format!("{} : {}", snap.home_score, snap.away_score),
                    clock_font(h * 0.17 * z_score),
                    Color32::WHITE,
                );
                elements.push(("score", Rect::from_center_size(score_pos, g.size()).expand(8.0)));
            }

            // 指针交互：点选 / Ctrl+点击多选切换 / 拖动选中元素 / 双击复位
            let pointer = ctx.input(|i| i.pointer.clone());
            let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
            if pointer.button_pressed(egui::PointerButton::Primary) {
                let origin = pointer.press_origin();
                let hit = origin
                    .and_then(|p| elements.iter().find(|(_, r)| r.contains(p)).map(|(id, _)| *id));
                match hit {
                    Some(id) => {
                        if ctrl_held {
                            if let Some(idx) = lay.selected.iter().position(|&s| s == id) {
                                lay.selected.remove(idx);
                            } else {
                                lay.selected.push(id);
                            }
                            lay.dragging = None;
                        } else {
                            if !lay.selected.contains(&id) {
                                lay.selected = vec![id];
                            }
                            lay.dragging = Some(id);
                            lay.drag_last = origin.unwrap_or(Pos2::ZERO);
                        }
                    }
                    None => {
                        if !ctrl_held {
                            lay.selected.clear();
                        }
                        lay.dragging = None;
                    }
                }
            }
            if let Some(drag_id) = lay.dragging {
                if pointer.button_down(egui::PointerButton::Primary) {
                    if let Some(pos) = pointer.latest_pos() {
                        let delta = pos - lay.drag_last;
                        if delta != Vec2::ZERO {
                            // 拖动时所有选中元素一起移动（存为比例偏移，分辨率无关）
                            let norm = Vec2::new(delta.x / w, delta.y / h);
                            for &id in &lay.selected.clone() {
                                *lay.offsets.entry(id).or_insert(Vec2::ZERO) += norm;
                            }
                            if !lay.selected.contains(&drag_id) {
                                *lay.offsets.entry(drag_id).or_insert(Vec2::ZERO) += norm;
                            }
                            lay.drag_last = pos;
                            for (_, r) in elements.iter_mut() {
                                *r = r.translate(delta);
                            }
                            ctx.request_repaint();
                        }
                    }
                } else {
                    lay.dragging = None;
                    save_display_layout(&lay);
                }
            }
            if pointer.button_double_clicked(egui::PointerButton::Primary) {
                if !lay.selected.is_empty() {
                    for &id in &lay.selected.clone() {
                        // 双击复位：恢复到固化的默认布局（非全零）
                        lay.offsets.insert(id, default_offset(id));
                        lay.zooms.insert(id, default_zoom(id));
                    }
                    save_display_layout(&lay);
                }
            }
            // 光标样式：悬停可拖 / 拖动中
            let hovering = pointer
                .hover_pos()
                .map(|p| elements.iter().any(|(_, r)| r.contains(p)))
                .unwrap_or(false);
            if lay.dragging.is_some() {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if hovering {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }

            // ---------- 绘制 ----------
            // 赛事名称（可选投放，随自身缩放）
            if event_visible {
                text_shadow(
                    &painter,
                    event_pos,
                    snap.event_name.clone(),
                    FontId::proportional(h * 0.034 * z_event),
                    Color32::from_white_alpha(215),
                );
            }
            // 阶段文字（无背景，白字 + 灰影）
            display_phase_pill(&painter, phase_pos, phase_text, h * z_phase);
            // 比赛时钟（带阴影）：全场结束后红色，暂停时黄色，其余白色
            let clock_color = if elapsed >= FULL_SECONDS {
                RED
            } else if snap.started() && !snap.running {
                YELLOW_LIGHT
            } else {
                Color32::WHITE
            };
            text_soft_shadow(
                &painter,
                clock_pos,
                fmt_clock(elapsed),
                clock_font(h * 0.155 * z_clock),
                clock_color,
                false,
            );

            // 中部：队徽 + 队名 + 比分（无卡片背景，队名/颜色标注统一白色）
            draw_team(
                &painter,
                home_logo,
                &snap.home_team,
                home_pos,
                h * 0.20 * z_home,
                h * z_home,
                Color32::WHITE,
                snap.show_home_name,
                snap.home_color,
            );
            draw_team(
                &painter,
                away_logo,
                &snap.away_team,
                away_pos,
                h * 0.20 * z_away,
                h * z_away,
                Color32::WHITE,
                snap.show_away_name,
                snap.away_color,
            );
            text_soft_shadow(
                &painter,
                score_pos,
                format!("{} : {}", snap.home_score, snap.away_score),
                clock_font(h * 0.17 * z_score),
                Color32::WHITE,
                true,
            );

            // 暂停闪烁徽标（90 分钟全场结束后不显示，时钟以红色标识）
            if snap.started() && !snap.running && elapsed < FULL_SECONDS {
                let ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_millis())
                    .unwrap_or(0);
                if ms % 1200 < 750 {
                    display_pill(
                        &painter,
                        rect.center_top() + Vec2::new(0.0, h * 0.36),
                        "已暂停",
                        h * 0.042,
                        YELLOW_LIGHT,
                        Color32::from_black_alpha(190),
                    );
                }
            }

            // 选中元素的外框提示（多选）
            for &id in &lay.selected {
                if let Some((_, r)) = elements.iter().find(|(i, _)| *i == id) {
                    painter.rect_stroke(
                        r.expand(6.0),
                        10.0,
                        Stroke::new(1.5f32, Color32::from_white_alpha(140)),
                    );
                }
            }

            // 操作提示（非全屏时显示）
            if !fullscreen.load(Ordering::SeqCst) {
                painter.text(
                    rect.right_bottom() - Vec2::new(w * 0.012, h * 0.025),
                    egui::Align2::RIGHT_BOTTOM,
                    "F 全屏 · Esc 关闭 · 点选/Ctrl+点击多选 · 拖动移动 · Ctrl + - / = 缩放选中 · 双击复位",
                    FontId::proportional(h * 0.022),
                    Color32::from_white_alpha(100),
                );
            }
        });
}

/// 大屏阶段文字（无背景）：白色字体 + 灰色投影
fn display_phase_pill(painter: &egui::Painter, center: Pos2, text: &str, h: f32) {
    let font = FontId::proportional(h * 0.034);
    let off = font.size * 0.03;
    let galley = painter.layout_no_wrap(text.to_string(), font, Color32::WHITE);
    let text_pos = Pos2::new(
        center.x - galley.size().x / 2.0,
        center.y - galley.size().y / 2.0,
    );

    // 灰色投影 + 白色本体（与比分风格一致）
    let outer = Color32::from_rgba_unmultiplied(80, 80, 80, 70);
    let inner = Color32::from_rgba_unmultiplied(70, 70, 70, 110);
    painter.galley(text_pos + Vec2::new(off, off), galley.clone(), outer);
    painter.galley(text_pos + Vec2::new(off * 0.5, off * 0.5), galley.clone(), inner);
    painter.galley(text_pos, galley, Color32::WHITE);
}

/// 大屏胶囊标签
fn display_pill(
    painter: &egui::Painter,
    center: Pos2,
    text: &str,
    font_size: f32,
    fg: Color32,
    bg: Color32,
) {
    let font = FontId::proportional(font_size);
    let galley = painter.layout_no_wrap(text.to_string(), font, fg);
    let text_rect = egui::Align2::CENTER_CENTER
        .align_size_within_rect(galley.size(), Rect::from_center_size(center, galley.size()));
    let padded = text_rect.expand2(Vec2::new(font_size * 0.9, font_size * 0.35));
    painter.rect_filled(padded, padded.height() / 2.0, bg);
    painter.galley(text_rect.min, galley, fg);
}

/// 文字 + 柔和投影
fn text_shadow(painter: &egui::Painter, pos: Pos2, text: String, font: FontId, color: Color32) {
    let off = font.size * 0.028;
    painter.text(
        pos + Vec2::new(off, off),
        egui::Align2::CENTER_CENTER,
        text.clone(),
        font.clone(),
        Color32::from_black_alpha(170),
    );
    painter.text(pos, egui::Align2::CENTER_CENTER, text, font, color);
}

/// 大屏数字渲染：白色本体 + 灰色柔和投影（无描边）；bold 时双重叠绘模拟加粗
fn text_soft_shadow(
    painter: &egui::Painter,
    pos: Pos2,
    text: String,
    font: FontId,
    fill: Color32,
    bold: bool,
) {
    let size = font.size;

    // 灰色投影：两层偏移叠加，边缘柔和
    let off = size * 0.03;
    let outer = Color32::from_rgba_unmultiplied(80, 80, 80, 70);
    let inner = Color32::from_rgba_unmultiplied(70, 70, 70, 110);
    painter.text(
        pos + Vec2::new(off, off),
        egui::Align2::CENTER_CENTER,
        text.clone(),
        font.clone(),
        outer,
    );
    painter.text(
        pos + Vec2::new(off * 0.5, off * 0.5),
        egui::Align2::CENTER_CENTER,
        text.clone(),
        font.clone(),
        inner,
    );

    // 本体（bold：向右微量偏移再叠一层，笔画变粗）
    painter.text(pos, egui::Align2::CENTER_CENTER, text.clone(), font.clone(), fill);
    if bold {
        painter.text(
            pos + Vec2::new(size * 0.012, 0.0),
            egui::Align2::CENTER_CENTER,
            text,
            font,
            fill,
        );
    }
}

fn draw_team(
    painter: &egui::Painter,
    logo: Option<(egui::TextureId, Vec2)>,
    name: &str,
    center: Pos2,
    box_size: f32,
    h: f32,
    name_color: Color32,
    show_name: bool,
    jersey: [u8; 3],
) {
    // 投放队名时队徽靠上、队名在下；不投放时队徽在卡片容器内居中
    let logo_center = if show_name {
        center - Vec2::new(0.0, h * 0.115)
    } else {
        center - Vec2::new(0.0, h * 0.028)
    };
    let logo_rect = Rect::from_center_size(logo_center, Vec2::splat(box_size));

    // 队标白色发光：单层半透明圆，半径外扩 10%
    painter.circle_filled(
        logo_rect.center(),
        box_size / 2.0 * 1.10,
        Color32::from_white_alpha(75),
    );

    match logo {
        Some((id, size)) => {
            let dst = fit_contain(logo_rect, size);
            painter.image(
                id,
                dst,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            let ch = name.chars().next().unwrap_or('队').to_string();
            painter.circle_stroke(
                logo_rect.center(),
                box_size / 2.0,
                Stroke::new(h * 0.006, Color32::from_white_alpha(70)),
            );
            painter.text(
                logo_rect.center(),
                egui::Align2::CENTER_CENTER,
                ch,
                FontId::proportional(box_size * 0.45),
                Color32::from_white_alpha(150),
            );
        }
    }
    if !show_name {
        return;
    }
    let name_font = FontId::proportional(h * 0.042);
    let off = h * 0.042 * 0.028;
    let name_pos = center + Vec2::new(0.0, h * 0.085);
    painter.text(
        name_pos + Vec2::new(off, off),
        egui::Align2::CENTER_CENTER,
        name.to_string(),
        name_font.clone(),
        Color32::from_black_alpha(170),
    );
    painter.text(name_pos, egui::Align2::CENTER_CENTER, name, name_font, name_color);

    // 队名下方的球衣颜色标注，如（红）
    let label = format!("（{}）", jersey_color_name(jersey));
    let label_font = FontId::proportional(h * 0.03);
    let loff = h * 0.03 * 0.028;
    let label_pos = center + Vec2::new(0.0, h * 0.134);
    painter.text(
        label_pos + Vec2::new(loff, loff),
        egui::Align2::CENTER_CENTER,
        label.clone(),
        label_font.clone(),
        Color32::from_black_alpha(170),
    );
    painter.text(label_pos, egui::Align2::CENTER_CENTER, label, label_font, name_color);
}

/// RGB 转最接近的球衣颜色名（用于大屏队名下方标注）
fn jersey_color_name(c: [u8; 3]) -> &'static str {
    let mut best = JERSEY_COLORS[0].0;
    let mut best_d = u32::MAX;
    for (name, p) in JERSEY_COLORS {
        let dr = c[0] as i32 - p[0] as i32;
        let dg = c[1] as i32 - p[1] as i32;
        let db = c[2] as i32 - p[2] as i32;
        let d = (dr * dr + dg * dg + db * db) as u32;
        if d < best_d {
            best_d = d;
            best = name;
        }
    }
    best
}

/// cover：填满目标矩形，裁剪超出部分
fn fit_cover(rect: Rect, img: Vec2) -> Rect {
    if img.x <= 0.0 || img.y <= 0.0 || rect.height() <= 0.0 {
        return rect;
    }
    let ia = img.x / img.y;
    let ra = rect.width() / rect.height();
    if ia > ra {
        let hh = rect.height();
        Rect::from_center_size(rect.center(), Vec2::new(hh * ia, hh))
    } else {
        let ww = rect.width();
        Rect::from_center_size(rect.center(), Vec2::new(ww, ww / ia))
    }
}

/// contain：完整显示在目标矩形内
fn fit_contain(rect: Rect, img: Vec2) -> Rect {
    if img.x <= 0.0 || img.y <= 0.0 || rect.height() <= 0.0 {
        return rect;
    }
    let ia = img.x / img.y;
    let ra = rect.width() / rect.height();
    if ia > ra {
        let ww = rect.width();
        Rect::from_center_size(rect.center(), Vec2::new(ww, ww / ia))
    } else {
        let hh = rect.height();
        Rect::from_center_size(rect.center(), Vec2::new(hh * ia, hh))
    }
}

fn fmt_clock(sec: i64) -> String {
    let sec = sec.max(0);
    format!("{:02}:{:02}", sec / 60, sec % 60)
}

fn tex_info(t: &Option<egui::TextureHandle>) -> Option<(egui::TextureId, Vec2)> {
    t.as_ref().map(|t| (t.id(), t.size_vec2()))
}

fn load_texture(
    ctx: &egui::Context,
    dir: &std::path::Path,
    name: Option<&str>,
    label: &str,
) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(dir.join(name?)).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(label, color, egui::TextureOptions::LINEAR))
}

/// 加载中文字体（黑体优先）与大屏专用字体；
/// 注意：ctx.set_fonts 是整体替换，必须合并成一份 FontDefinitions 一次性注册
fn setup_cjk_fonts(ctx: &egui::Context) {
    let fonts_dir = PathBuf::from("C:\\Windows\\Fonts");
    let mut fonts = egui::FontDefinitions::default();

    // 中文字体（黑体优先），否则 egui 默认字体不含汉字
    let mut cjk_loaded = false;
    for name in ["simhei.ttf", "simkai.ttf", "simfang.ttf", "msyh.ttc"] {
        let Ok(bytes) = std::fs::read(fonts_dir.join(name)) else {
            continue;
        };
        fonts
            .font_data
            .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().insert(0, "cjk".to_owned());
        }
        println!("[info] 已加载中文字体: {name}");
        cjk_loaded = true;
        break;
    }
    if !cjk_loaded {
        eprintln!("[warn] 未找到中文字体，界面汉字可能显示为方块");
    }

    // 大屏数字字体（比分与计时共用，减少打包字体）：
    //   DIN Condensed Bold 优先，比分通过模拟加粗区分
    // Windows 不自带 DIN，优先找项目 fonts/ 目录下的用户字体，否则回退系统字体
    let local_fonts = PathBuf::from("fonts");
    let clock_candidates = [
        local_fonts.join("DINCondensed-Bold.ttf"),
        local_fonts.join("din-condensed-bold.ttf"),
        fonts_dir.join("bahnschrift.ttf"),
        fonts_dir.join("arialbd.ttf"),
    ];
    let clock_loaded = load_display_font(&mut fonts, "clock", "clock_font", &clock_candidates);
    CLOCK_FONT_OK.set(clock_loaded).ok();
    if !clock_loaded {
        eprintln!("[warn] 大屏数字字体加载失败，使用默认字体");
    }

    ctx.set_fonts(fonts);
}

/// 按候选路径顺序加载字体并注册为自定义字体族，返回是否成功
fn load_display_font(
    fonts: &mut egui::FontDefinitions,
    family_name: &'static str,
    data_key: &'static str,
    candidates: &[PathBuf],
) -> bool {
    for path in candidates {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts
            .font_data
            .insert(data_key.to_owned(), egui::FontData::from_owned(bytes));
        fonts.families.insert(
            egui::FontFamily::Name(family_name.into()),
            vec![data_key.to_owned()],
        );
        println!("[info] 已加载大屏字体 [{family_name}]: {}", path.display());
        return true;
    }
    false
}

/// 大屏数字字体是否加载成功
static CLOCK_FONT_OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// 大屏数字字体（比分/计时共用 DIN Condensed Bold），未加载成功时回退默认字体
fn clock_font(size: f32) -> FontId {
    if CLOCK_FONT_OK.get().copied().unwrap_or(false) {
        FontId::new(size, egui::FontFamily::Name("clock".into()))
    } else {
        FontId::proportional(size)
    }
}
