use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 半场时长：45 分钟
pub const HALF_SECONDS: i64 = 45 * 60;
/// 全场时长：90 分钟
pub const FULL_SECONDS: i64 = 90 * 60;

/// 当前 Unix 时间戳（秒）
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 比赛核心状态（整体序列化持久化到 JSON 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchState {
    pub home_team: String,
    pub away_team: String,
    pub home_score: u32,
    pub away_score: u32,
    /// 开赛 Unix 时间戳；0 表示尚未开赛
    pub start_timestamp: i64,
    /// 累计暂停秒数
    pub paused_seconds: i64,
    /// 是否正在计时
    pub running: bool,
    /// 本次暂停开始时间戳；0 表示未处于暂停
    pub pause_started_at: i64,
    pub auto_pause_45: bool,
    pub auto_pause_90: bool,
    /// 45/90 分钟自动暂停是否已触发（防止恢复后重复触发）
    #[serde(default)]
    pub auto_paused_45: bool,
    #[serde(default)]
    pub auto_paused_90: bool,
    /// 是否已开始下半场计时（45 分钟自动暂停后点“继续”才置位；
    /// 在此之前大屏仍显示“上半场”）
    #[serde(default)]
    pub second_half_started: bool,
    /// 素材文件名（相对 data/images/）
    pub background_image: Option<String>,
    pub home_logo: Option<String>,
    pub away_logo: Option<String>,
    /// 素材版本号，用于前端缓存刷新
    #[serde(default)]
    pub image_version: u64,
    /// 赛事名称（如：2026 中超联赛·第 20 轮）
    #[serde(default)]
    pub event_name: String,
    /// 赛事名称是否投放到大屏页
    #[serde(default = "default_true")]
    pub show_event_name: bool,
    /// 主/客队名是否投放到大屏页（不投时队徽在容器居中）
    #[serde(default = "default_true")]
    pub show_home_name: bool,
    #[serde(default = "default_true")]
    pub show_away_name: bool,
    /// 主/客队球衣色 RGB（大屏队名与卡片点缀色）
    #[serde(default = "default_home_color")]
    pub home_color: [u8; 3],
    #[serde(default = "default_away_color")]
    pub away_color: [u8; 3],
}

fn default_true() -> bool {
    true
}
fn default_home_color() -> [u8; 3] {
    [227, 66, 52]
}
fn default_away_color() -> [u8; 3] {
    [47, 129, 247]
}

impl Default for MatchState {
    fn default() -> Self {
        Self {
            home_team: "主队".to_string(),
            away_team: "客队".to_string(),
            home_score: 0,
            away_score: 0,
            start_timestamp: 0,
            paused_seconds: 0,
            running: false,
            pause_started_at: 0,
            auto_pause_45: true,
            auto_pause_90: true,
            auto_paused_45: false,
            auto_paused_90: false,
            second_half_started: false,
            background_image: None,
            home_logo: None,
            away_logo: None,
            image_version: 0,
            event_name: String::new(),
            show_event_name: true,
            show_home_name: true,
            show_away_name: true,
            home_color: default_home_color(),
            away_color: default_away_color(),
        }
    }
}

impl MatchState {
    pub fn started(&self) -> bool {
        self.start_timestamp > 0
    }

    /// 比赛时间 = 当前(或暂停时刻) - 开赛时间 - 累计暂停时间
    pub fn elapsed_seconds_at(&self, at: i64) -> i64 {
        if !self.started() {
            return 0;
        }
        let end = if self.running { at } else { self.pause_started_at };
        (end - self.start_timestamp - self.paused_seconds).max(0)
    }

    pub fn start(&mut self, at: i64) {
        if self.started() {
            if !self.running {
                self.resume(at);
            }
            return;
        }
        self.start_timestamp = at;
        self.running = true;
    }

    pub fn pause(&mut self, at: i64) {
        if !self.started() || !self.running {
            return;
        }
        self.running = false;
        self.pause_started_at = at;
    }

    pub fn resume(&mut self, at: i64) {
        if !self.started() || self.running {
            return;
        }
        // 90 分钟全场结束后不可继续计时
        if self.auto_paused_90 {
            return;
        }
        self.paused_seconds += (at - self.pause_started_at).max(0);
        self.pause_started_at = 0;
        self.running = true;
        // 从 45 分钟节点之后恢复计时 → 下半场开始
        if self.elapsed_seconds_at(at) >= HALF_SECONDS {
            self.second_half_started = true;
        }
    }

    /// 直接设定比赛时间（时间控制）：运行中调整开赛基准，暂停中把差值计入暂停累计。
    /// 越过 45/90 阈值时同步置位对应标志，避免后续自动暂停把时间钉回阈值整点。
    pub fn set_elapsed(&mut self, target: i64, at: i64) {
        if !self.started() {
            return;
        }
        let target = target.clamp(0, 2 * FULL_SECONDS);
        if self.running {
            self.start_timestamp = (at - self.paused_seconds - target).max(1);
        } else {
            let base = self.pause_started_at - self.start_timestamp;
            self.paused_seconds = (base - target).max(0);
        }
        if target >= HALF_SECONDS {
            self.auto_paused_45 = true;
            // 运行中设定到 45:00 及以上 → 视为已在下半场（暂停中等待“继续”的不算）
            if self.running {
                self.second_half_started = true;
            }
        }
        if target >= FULL_SECONDS {
            self.auto_paused_90 = true;
        }
    }

    /// 重置比赛（保留球队名称、素材与自动暂停设置）
    pub fn reset(&mut self) {
        self.home_score = 0;
        self.away_score = 0;
        self.start_timestamp = 0;
        self.paused_seconds = 0;
        self.running = false;
        self.pause_started_at = 0;
        self.auto_paused_45 = false;
        self.auto_paused_90 = false;
        self.second_half_started = false;
    }
}
