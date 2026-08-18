fn main() {
    // 把 assets/app_icon.ico 嵌入 exe 资源，作为文件图标（资源管理器/任务栏显示）
    embed_resource::compile("app.rc", embed_resource::NONE);
}
