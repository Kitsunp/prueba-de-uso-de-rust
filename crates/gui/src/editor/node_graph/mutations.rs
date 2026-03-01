use super::*;

impl NodeGraph {
    /// Inserts a new node before the target node, re-routing connections.
    pub fn insert_before(&mut self, target_id: u32, node: StoryNode) {
        let Some((_, _, pos)) = self.nodes.iter().find(|(id, _, _)| *id == target_id) else {
            debug_assert!(
                false,
                "Precondition warning: target_id {} not found in insert_before",
                target_id
            );
            return;
        };

        let new_pos = egui::pos2(pos.x, pos.y - NODE_VERTICAL_SPACING);
        let new_id = self.add_node(node, new_pos);

        for conn in &mut self.connections {
            if conn.to == target_id {
                conn.to = new_id;
            }
        }

        self.connections.push(GraphConnection {
            from: new_id,
            from_port: 0,
            to: target_id,
        });

        self.modified = true;
    }

    /// Inserts a new node after the target node, re-routing connections.
    pub fn insert_after(&mut self, target_id: u32, node: StoryNode) {
        let Some((_, _, pos)) = self.nodes.iter().find(|(id, _, _)| *id == target_id) else {
            return;
        };

        let new_pos = egui::pos2(pos.x, pos.y + NODE_VERTICAL_SPACING);
        let new_id = self.add_node(node, new_pos);

        for conn in &mut self.connections {
            if conn.from == target_id && conn.from_port == 0 {
                conn.from = new_id;
                conn.from_port = 0;
            }
        }

        self.connections.push(GraphConnection {
            from: target_id,
            from_port: 0,
            to: new_id,
        });

        self.modified = true;
    }

    /// Converts a node to a Choice node with default options.
    pub fn convert_to_choice(&mut self, node_id: u32) {
        if let Some((_, node, _)) = self.nodes.iter_mut().find(|(id, _, _)| *id == node_id) {
            *node = StoryNode::Choice {
                prompt: "Choose an option:".to_string(),
                options: vec!["Option 1".to_string(), "Option 2".to_string()],
            };
            self.modified = true;
        }
    }

    /// Creates a branch from a node (adds a Choice with two paths).
    pub fn create_branch(&mut self, node_id: u32) {
        let Some((_, node, pos)) = self.nodes.iter().find(|(id, _, _)| *id == node_id).cloned()
        else {
            return;
        };

        if matches!(node, StoryNode::End) {
            return;
        }

        let choice_pos = egui::pos2(pos.x, pos.y + 120.0);
        let choice_id = self.add_node(
            StoryNode::Choice {
                prompt: "Which path?".to_string(),
                options: vec!["Path A".to_string(), "Path B".to_string()],
            },
            choice_pos,
        );

        let branch_a = self.add_node(
            StoryNode::Dialogue {
                speaker: "Path A".to_string(),
                text: "Content for path A...".to_string(),
            },
            egui::pos2(choice_pos.x - 120.0, choice_pos.y + 140.0),
        );

        let branch_b = self.add_node(
            StoryNode::Dialogue {
                speaker: "Path B".to_string(),
                text: "Content for path B...".to_string(),
            },
            egui::pos2(choice_pos.x + 120.0, choice_pos.y + 140.0),
        );

        self.connect_port(node_id, 0, choice_id);
        self.connect_port(choice_id, 0, branch_a);
        self.connect_port(choice_id, 1, branch_b);
    }

    /// Saves the current Scene node fields into a reusable profile.
    pub fn save_scene_profile(&mut self, profile_id: impl Into<String>, node_id: u32) -> bool {
        let profile_id = profile_id.into().trim().to_string();
        if profile_id.is_empty() {
            return false;
        }

        let Some(StoryNode::Scene {
            background,
            music,
            characters,
            ..
        }) = self.get_node(node_id)
        else {
            return false;
        };

        self.scene_profiles.insert(
            profile_id.clone(),
            SceneProfile {
                background: background.clone(),
                music: music.clone(),
                characters: characters.clone(),
            },
        );

        if let Some(StoryNode::Scene { profile, .. }) = self.get_node_mut(node_id) {
            *profile = Some(profile_id);
        }
        self.modified = true;
        true
    }

    /// Applies a saved Scene profile to an existing Scene node.
    pub fn apply_scene_profile(&mut self, profile_id: &str, node_id: u32) -> bool {
        let Some(scene_profile) = self.scene_profiles.get(profile_id).cloned() else {
            return false;
        };

        let Some(StoryNode::Scene {
            background,
            music,
            characters,
            profile,
        }) = self.get_node_mut(node_id)
        else {
            return false;
        };

        *background = scene_profile.background;
        *music = scene_profile.music;
        *characters = scene_profile.characters;
        *profile = Some(profile_id.to_string());
        self.modified = true;
        true
    }

    /// Returns available scene profile names.
    pub fn scene_profile_names(&self) -> Vec<String> {
        self.scene_profiles.keys().cloned().collect()
    }

    /// Creates or updates a bookmark that points to an existing node.
    pub fn set_bookmark(&mut self, name: impl Into<String>, node_id: u32) -> bool {
        if self.get_node(node_id).is_none() {
            return false;
        }
        let normalized = name.into().trim().to_string();
        if normalized.is_empty() {
            return false;
        }
        self.bookmarks.insert(normalized, node_id);
        self.modified = true;
        true
    }

    /// Removes a bookmark by name.
    pub fn remove_bookmark(&mut self, name: &str) -> bool {
        if self.bookmarks.remove(name).is_some() {
            self.modified = true;
            true
        } else {
            false
        }
    }

    /// Resolves a bookmark name into its node id.
    pub fn bookmarked_node(&self, name: &str) -> Option<u32> {
        self.bookmarks.get(name).copied()
    }

    /// Returns bookmark names and targets in deterministic order.
    pub fn bookmarks(&self) -> impl Iterator<Item = (&String, &u32)> {
        self.bookmarks.iter()
    }
}
