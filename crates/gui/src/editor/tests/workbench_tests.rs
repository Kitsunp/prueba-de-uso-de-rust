use super::*;

#[test]
fn test_workbench_initialization() {
    let config = VnConfig::default();
    let mut workbench = EditorWorkbench::new(config);

    // Assert default state
    assert_eq!(workbench.mode, EditorMode::Editor);
    assert!(workbench.node_graph.nodes.is_empty());
    assert!(!workbench.is_playing);

    // Add dummy track
    let mut track = visual_novel_engine::Track::new(
        visual_novel_engine::EntityId::new(1),
        visual_novel_engine::PropertyType::PositionX,
    );
    track
        .add_keyframe(visual_novel_engine::Keyframe::new(
            100,
            0,
            visual_novel_engine::Easing::Linear,
        ))
        .unwrap();
    workbench.timeline.add_track(track).unwrap();

    // Test simple update
    workbench.is_playing = true;
    workbench.update(1);
    assert!(
        workbench.current_time > 0.0,
        "Time should advance when playing"
    );
}
