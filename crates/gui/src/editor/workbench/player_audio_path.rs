pub(super) fn resolve_player_audio_asset_path(
    project_root: Option<&std::path::Path>,
    raw_path: &str,
) -> Option<String> {
    let project_root = project_root?;
    let normalized = raw_path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return None;
    }

    let mut candidates = Vec::new();
    push_unique_candidate(&mut candidates, normalized.clone());
    if !normalized.starts_with("assets/") {
        push_unique_candidate(&mut candidates, format!("assets/{normalized}"));
    }

    if std::path::Path::new(&normalized).extension().is_none() {
        const AUDIO_EXTS: [&str; 5] = ["ogg", "opus", "mp3", "wav", "flac"];
        let base_candidates = candidates.clone();
        for entry in base_candidates {
            for ext in AUDIO_EXTS {
                push_unique_candidate(&mut candidates, format!("{entry}.{ext}"));
            }
        }
    }

    candidates
        .into_iter()
        .find(|candidate| project_root.join(candidate).exists())
}

fn push_unique_candidate(candidates: &mut Vec<String>, value: String) {
    if candidates.iter().any(|existing| existing == &value) {
        return;
    }
    candidates.push(value);
}
