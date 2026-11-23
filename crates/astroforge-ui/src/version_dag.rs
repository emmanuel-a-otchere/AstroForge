//! Version DAG — pure graph logic over the image-version timeline
//! (port of `src/state/version-dag.ts`): nodes, parent edges, and
//! layered layout for the timeline view.

use crate::api::versions::ImageVersionDto;

/// One node in the DAG: a version plus its layout layer (0 = roots).
#[derive(Clone, Debug, PartialEq)]
pub struct DagNode {
    pub version_id: String,
    pub label: String,
    pub sequence: u32,
    pub layer: usize,
}

/// Build the layered DAG. Roots (no visible parent) go to layer 0;
/// every other node sits one past its deepest visible parent. Hidden
/// versions are excluded but their edges pass THROUGH to the child
/// (a hidden parent promotes the child to the hidden parent's layer
/// source, i.e. the child attaches to its nearest visible ancestor).
pub fn build(versions: &[ImageVersionDto]) -> Vec<DagNode> {
    let by_id: std::collections::HashMap<&str, &ImageVersionDto> = versions
        .iter()
        .map(|v| (v.version_id.as_str(), v))
        .collect();

    // Nearest visible ancestor: follow source_version_id until a
    // visible version is found (or none).
    let visible_parent = |v: &ImageVersionDto| -> Option<String> {
        let mut cursor = v.source_version_id.clone();
        let mut hops = 0;
        while let Some(id) = cursor {
            if hops > versions.len() {
                break; // cycle guard
            }
            hops += 1;
            if let Some(parent) = by_id.get(id.as_str()) {
                if !parent.hidden {
                    return Some(parent.version_id.clone());
                }
                cursor = parent.source_version_id.clone();
            } else {
                break;
            }
        }
        None
    };

    // Parents map for layer assignment.
    let mut parents: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    for v in versions {
        parents.insert(v.version_id.clone(), visible_parent(v));
    }

    // Iterative longest-path layering (memoized, cycle-safe).
    fn layer_of(
        id: &str,
        parents: &std::collections::HashMap<String, Option<String>>,
        memo: &mut std::collections::HashMap<String, usize>,
    ) -> usize {
        if let Some(l) = memo.get(id) {
            return *l;
        }
        memo.insert(id.to_string(), 0); // cycle guard
        let layer = match parents.get(id) {
            Some(Some(parent_id)) => layer_of(parent_id, parents, memo) + 1,
            _ => 0,
        };
        memo.insert(id.to_string(), layer);
        layer
    }

    let mut memo = std::collections::HashMap::new();
    let mut nodes: Vec<DagNode> = versions
        .iter()
        .filter(|v| !v.hidden)
        .map(|v| DagNode {
            version_id: v.version_id.clone(),
            label: format!("V{} \u{2014} {}", v.sequence, v.label),
            sequence: v.sequence,
            layer: layer_of(&v.version_id, &parents, &mut memo),
        })
        .collect();
    nodes.sort_by_key(|n| (n.layer, n.sequence));
    nodes
}

/// Group nodes by layer for columnar painting.
pub fn layers(nodes: &[DagNode]) -> Vec<Vec<DagNode>> {
    let mut out: Vec<Vec<DagNode>> = Vec::new();
    for node in nodes {
        while out.len() <= node.layer {
            out.push(Vec::new());
        }
        out[node.layer].push(node.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, source: Option<&str>, hidden: bool) -> ImageVersionDto {
        ImageVersionDto {
            version_id: id.to_string(),
            project_id: "proj_x".into(),
            label: format!("label-{id}"),
            sequence: 0,
            primary_artifact_id: String::new(),
            source_version_id: source.map(|s| s.to_string()),
            created_at: String::new(),
            hidden,
        }
    }

    #[test]
    fn linear_chain_layers_deepen() {
        let versions = vec![
            version("v0", None, false),
            version("v1", Some("v0"), false),
            version("v2", Some("v1"), false),
        ];
        let nodes = build(&versions);
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].layer, 0);
        assert_eq!(nodes[1].layer, 1);
        assert_eq!(nodes[2].layer, 2);
    }

    #[test]
    fn branch_attaches_to_shared_parent_layer() {
        let versions = vec![
            version("v0", None, false),
            version("v1", Some("v0"), false),
            version("b1", Some("v0"), false),
        ];
        let nodes = build(&versions);
        assert_eq!(nodes.len(), 3);
        // v0 at layer 0; v1 and b1 both at layer 1.
        let layers = layers(&nodes);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[1].len(), 2);
    }

    #[test]
    fn hidden_parent_passes_edge_through() {
        let versions = vec![
            version("v0", None, false),
            version("hid", Some("v0"), true),
            version("child", Some("hid"), false),
        ];
        let nodes = build(&versions);
        assert_eq!(nodes.len(), 2); // hidden excluded
        let layers = layers(&nodes);
        assert_eq!(layers.len(), 2); // child hangs off v0 through the hidden hop
        assert_eq!(layers[1][0].version_id, "child");
    }

    #[test]
    fn cycles_terminate() {
        let versions = vec![
            version("a", Some("b"), false),
            version("b", Some("a"), false),
        ];
        // Must not hang; layers clamp at cycle guard.
        let nodes = build(&versions);
        assert_eq!(nodes.len(), 2);
    }
}
