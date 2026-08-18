use crate::state::MatchState;
use std::fs;
use std::path::Path;

/// 状态文件：data/match.json（整体保存，重启后恢复）
fn state_file(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("match.json")
}

pub fn load_state(data_dir: &Path) -> Option<MatchState> {
    let path = state_file(data_dir);
    let content = fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<MatchState>(&content) {
        Ok(s) => {
            println!("[info] 已从 {} 恢复比赛状态", path.display());
            Some(s)
        }
        Err(e) => {
            eprintln!("[warn] 状态文件解析失败，使用默认状态: {e}");
            None
        }
    }
}

pub fn save_state(data_dir: &Path, state: &MatchState) -> std::io::Result<()> {
    fs::create_dir_all(data_dir)?;
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    // 先写临时文件再改名，避免写入中途断电导致文件损坏
    let tmp = state_file(data_dir).with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, state_file(data_dir))?;
    Ok(())
}
