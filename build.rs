fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_resource::compile("resources/winmoji.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed WinMoji resources");
    }
}
