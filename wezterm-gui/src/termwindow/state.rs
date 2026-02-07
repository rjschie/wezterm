use mux::tab::{PaneNode, SplitDirection};
use mux::window::WindowId as MuxWindowId;
use mux::Mux;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use ::window::{ConnectionOps, Dimensions, ScreenPoint};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SavedGeometry {
    pub x_percent: Option<f64>,
    pub y_percent: Option<f64>,
    pub width: Option<usize>,
    pub height: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SavedState {
    pub version: u32,
    pub windows: Vec<SavedWindow>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SavedWindow {
    pub geometry: SavedGeometry,
    pub workspace: String,
    pub tabs: Vec<SavedTab>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SavedTab {
    pub title: String,
    pub is_active: bool,
    #[serde(default)]
    pub is_zoomed: bool,
    pub pane_tree: SavedPaneNode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum SavedPaneNode {
    #[serde(rename = "split")]
    Split {
        direction: SplitDirection,
        sizes: [f64; 2],
        first: Box<SavedPaneNode>,
        second: Option<Box<SavedPaneNode>>,
    },
    #[serde(rename = "leaf")]
    Leaf {
        working_directory: Option<String>,
        is_active: bool,
    },
}

lazy_static::lazy_static! {
    static ref WINDOW_GEOMETRIES: Mutex<HashMap<MuxWindowId, (Dimensions, Option<ScreenPoint>)>> =
        Mutex::new(HashMap::new());
}

fn state_file_name() -> PathBuf {
    config::DATA_DIR.join("state.json")
}

pub fn load_state() -> Option<SavedState> {
    let f = std::fs::File::open(state_file_name()).ok()?;
    serde_json::from_reader(f).ok()
}

fn save_state(state: &SavedState) {
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(err) = std::fs::write(state_file_name(), json) {
                log::warn!("state: failed to save: {err:#}");
            }
        }
        Err(err) => {
            log::warn!("state: failed to serialize: {err:#}");
        }
    }
}

pub fn pane_node_to_saved(node: &PaneNode) -> Option<SavedPaneNode> {
    match node {
        PaneNode::Empty => None,
        PaneNode::Split { left, right, node } => {
            let (first_cells, second_cells) = match node.direction {
                SplitDirection::Horizontal => {
                    (node.first.cols as f64, node.second.cols as f64)
                }
                SplitDirection::Vertical => {
                    (node.first.rows as f64, node.second.rows as f64)
                }
            };
            let dim = first_cells + 1.0 + second_cells;
            let second_pct = if dim > 0.0 {
                second_cells / dim * 100.0
            } else {
                50.0
            };
            let first_pct = 100.0 - second_pct;

            Some(SavedPaneNode::Split {
                direction: node.direction,
                sizes: [first_pct, second_pct],
                first: Box::new(
                    pane_node_to_saved(left).unwrap_or(SavedPaneNode::Leaf {
                        working_directory: None,
                        is_active: false,
                    }),
                ),
                second: pane_node_to_saved(right).map(Box::new),
            })
        }
        PaneNode::Leaf(entry) => Some(SavedPaneNode::Leaf {
            working_directory: entry
                .working_dir
                .as_ref()
                .and_then(|u| {
                    url::Url::parse(&u.url.to_string())
                        .ok()
                        .and_then(|url| {
                            if url.scheme() == "file" {
                                Some(url.path().to_string())
                            } else {
                                None
                            }
                        })
                }),
            is_active: entry.is_active_pane,
        }),
    }
}

pub fn capture_window_state(
    mux_window_id: MuxWindowId,
    dimensions: &Dimensions,
    position: Option<ScreenPoint>,
) -> SavedWindow {
    let mux = Mux::get();
    let dpi = dimensions.dpi.max(1) as f64;

    let (x_percent, y_percent) = match (position, ::window::Connection::get()) {
        (Some(pos), Some(conn)) => conn
            .screens()
            .ok()
            .map(|screens| {
                let r = screens.virtual_rect;
                let w = r.width() as f64;
                let h = r.height() as f64;
                let x_pct = if w > 0.0 {
                    Some(
                        ((pos.x as f64 - r.min_x() as f64) / w * 100.0 * 1000.0).round()
                            / 1000.0,
                    )
                } else {
                    None
                };
                let y_pct = if h > 0.0 {
                    Some(
                        ((pos.y as f64 - r.min_y() as f64) / h * 100.0 * 1000.0).round()
                            / 1000.0,
                    )
                } else {
                    None
                };
                (x_pct, y_pct)
            })
            .unwrap_or((None, None)),
        _ => (None, None),
    };

    let base_dpi = ::window::DEFAULT_DPI;
    let geometry = SavedGeometry {
        x_percent,
        y_percent,
        width: Some((dimensions.pixel_width as f64 * base_dpi / dpi) as usize),
        height: Some((dimensions.pixel_height as f64 * base_dpi / dpi) as usize),
    };

    let workspace = mux
        .get_window(mux_window_id)
        .map(|w| w.get_workspace().to_string())
        .unwrap_or_else(|| "default".to_string());

    let config = config::configuration();
    let mut tabs = vec![];
    if config.remember_window_state {
        if let Some(window) = mux.get_window(mux_window_id) {
            let active_idx = window.get_active_idx();
            for (idx, tab) in window.iter().enumerate() {
                let pane_tree = tab.codec_pane_tree();
                let saved_tree = pane_node_to_saved(&pane_tree).unwrap_or(SavedPaneNode::Leaf {
                    working_directory: None,
                    is_active: true,
                });
                tabs.push(SavedTab {
                    title: tab.get_title(),
                    is_active: idx == active_idx,
                    is_zoomed: tab.get_zoomed_pane().is_some(),
                    pane_tree: saved_tree,
                });
            }
        }
    }

    SavedWindow {
        geometry,
        workspace,
        tabs,
    }
}

pub fn register_window_geometry(
    id: MuxWindowId,
    dims: Dimensions,
    pos: Option<ScreenPoint>,
) {
    if let Ok(mut map) = WINDOW_GEOMETRIES.lock() {
        map.insert(id, (dims, pos));
    }
}

pub fn unregister_window_geometry(id: MuxWindowId) {
    if let Ok(mut map) = WINDOW_GEOMETRIES.lock() {
        map.remove(&id);
    }
}

pub fn save_all_windows_state() {
    let config = config::configuration();
    if !config.remember_window_position && !config.remember_window_state {
        return;
    }

    let mux = Mux::get();
    let geometries = match WINDOW_GEOMETRIES.lock() {
        Ok(g) => g.clone(),
        Err(_) => return,
    };

    let mut windows = vec![];
    for mux_window_id in mux.iter_windows() {
        if mux.get_window(mux_window_id).is_none() {
            continue;
        }
        if let Some((dims, pos)) = geometries.get(&mux_window_id) {
            windows.push(capture_window_state(mux_window_id, dims, *pos));
        }
    }

    if windows.is_empty() {
        return;
    }
    let state = SavedState {
        version: 2,
        windows,
    };
    save_state(&state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux::tab::{PaneEntry, SplitDirectionAndSize};
    use mux::renderable::StableCursorPosition;
    use wezterm_term::TerminalSize;

    const CELL_W: usize = 10;
    const CELL_H: usize = 20;

    fn make_leaf(rows: usize, cols: usize) -> PaneNode {
        PaneNode::Leaf(PaneEntry {
            window_id: 0,
            tab_id: 0,
            pane_id: 0,
            title: String::new(),
            size: TerminalSize {
                rows,
                cols,
                pixel_width: cols * CELL_W,
                pixel_height: rows * CELL_H,
                dpi: 96,
            },
            working_dir: None,
            is_active_pane: false,
            is_zoomed_pane: false,
            workspace: String::new(),
            cursor_pos: StableCursorPosition::default(),
            physical_top: 0,
            top_row: 0,
            left_col: 0,
            tty_name: None,
        })
    }

    fn make_split(
        dir: SplitDirection,
        first_rows: usize,
        first_cols: usize,
        second_rows: usize,
        second_cols: usize,
        left: PaneNode,
        right: PaneNode,
    ) -> PaneNode {
        PaneNode::Split {
            left: Box::new(left),
            right: Box::new(right),
            node: SplitDirectionAndSize {
                direction: dir,
                first: TerminalSize {
                    rows: first_rows,
                    cols: first_cols,
                    pixel_width: first_cols * CELL_W,
                    pixel_height: first_rows * CELL_H,
                    dpi: 96,
                },
                second: TerminalSize {
                    rows: second_rows,
                    cols: second_cols,
                    pixel_width: second_cols * CELL_W,
                    pixel_height: second_rows * CELL_H,
                    dpi: 96,
                },
            },
        }
    }

    /// Simulate what the mux produces when restoring a SavedPaneNode at given
    /// (rows, cols) dimensions using SplitSize::Cells math (mirrors restore code).
    fn simulate_restore(node: &SavedPaneNode, rows: usize, cols: usize) -> PaneNode {
        match node {
            SavedPaneNode::Leaf { .. } => make_leaf(rows, cols),
            SavedPaneNode::Split {
                direction,
                sizes,
                first,
                second,
            } => {
                let (first_rows, first_cols, second_rows, second_cols) = match direction {
                    SplitDirection::Vertical => {
                        let second_cells =
                            (sizes[1] / 100.0 * rows as f64).round() as usize;
                        let first_cells = rows - second_cells - 1;
                        (first_cells, cols, second_cells, cols)
                    }
                    SplitDirection::Horizontal => {
                        let second_cells =
                            (sizes[1] / 100.0 * cols as f64).round() as usize;
                        let first_cells = cols - second_cells - 1;
                        (rows, first_cells, rows, second_cells)
                    }
                };

                let left = simulate_restore(first, first_rows, first_cols);
                let right = match second {
                    Some(s) => simulate_restore(s, second_rows, second_cols),
                    None => make_leaf(second_rows, second_cols),
                };

                make_split(
                    *direction,
                    first_rows,
                    first_cols,
                    second_rows,
                    second_cols,
                    left,
                    right,
                )
            }
        }
    }

    fn collect_sizes(node: &SavedPaneNode, out: &mut Vec<[f64; 2]>) {
        match node {
            SavedPaneNode::Split {
                sizes,
                first,
                second,
                ..
            } => {
                out.push(*sizes);
                collect_sizes(first, out);
                if let Some(s) = second {
                    collect_sizes(s, out);
                }
            }
            SavedPaneNode::Leaf { .. } => {}
        }
    }

    #[test]
    fn serde_roundtrip() {
        let tree = SavedPaneNode::Split {
            direction: SplitDirection::Vertical,
            sizes: [72.0, 28.0],
            first: Box::new(SavedPaneNode::Leaf {
                working_directory: Some("/home/user".into()),
                is_active: true,
            }),
            second: Some(Box::new(SavedPaneNode::Leaf {
                working_directory: None,
                is_active: false,
            })),
        };

        let json = serde_json::to_string_pretty(&tree).unwrap();
        let deserialized: SavedPaneNode = serde_json::from_str(&json).unwrap();

        let mut orig_sizes = vec![];
        let mut rt_sizes = vec![];
        collect_sizes(&tree, &mut orig_sizes);
        collect_sizes(&deserialized, &mut rt_sizes);
        assert_eq!(orig_sizes, rt_sizes);
    }

    /// Build a complex PaneNode, convert to saved, simulate restore at same
    /// dimensions, re-save, and assert percentages are within tolerance.
    #[test]
    fn save_restore_resave_preserves_proportions() {
        // Layout (matching real-world test case):
        //   Vertical 72/28
        //     first: Horizontal 43/57
        //       first: leaf
        //       second: Vertical 26/74
        //         first: leaf
        //         second: leaf
        //     second: leaf
        let total_rows: usize = 50;
        let total_cols: usize = 100;

        let top_rows = 36; // ~72% of 50
        let bot_rows = 13; // ~28% of 50 (1 for separator)
        let left_cols = 42; // ~43% of 100
        let right_cols = 57; // ~57% of 100 (1 for separator)
        let tr_top_rows = 9; // ~26% of 35
        let tr_bot_rows = 26; // ~74% of 35

        let tree = make_split(
            SplitDirection::Vertical,
            top_rows,
            total_cols,
            bot_rows,
            total_cols,
            make_split(
                SplitDirection::Horizontal,
                top_rows,
                left_cols,
                top_rows,
                right_cols,
                make_leaf(top_rows, left_cols),
                make_split(
                    SplitDirection::Vertical,
                    tr_top_rows,
                    right_cols,
                    tr_bot_rows,
                    right_cols,
                    make_leaf(tr_top_rows, right_cols),
                    make_leaf(tr_bot_rows, right_cols),
                ),
            ),
            make_leaf(bot_rows, total_cols),
        );

        // Step 1: save
        let saved = pane_node_to_saved(&tree).expect("should produce SavedPaneNode");

        // Step 2: simulate restore at same total dimensions
        let restored_pane_node = simulate_restore(&saved, total_rows, total_cols);

        // Step 3: re-save
        let resaved = pane_node_to_saved(&restored_pane_node).expect("should produce SavedPaneNode");

        // Step 4: compare percentages within tolerance
        let mut saved_sizes = vec![];
        let mut resaved_sizes = vec![];
        collect_sizes(&saved, &mut saved_sizes);
        collect_sizes(&resaved, &mut resaved_sizes);

        assert_eq!(
            saved_sizes.len(),
            resaved_sizes.len(),
            "different number of splits"
        );

        // With full f64 precision and Cells-based restore, same-size
        // roundtrips should produce zero drift.
        for (i, (orig, restored)) in saved_sizes.iter().zip(&resaved_sizes).enumerate() {
            assert_eq!(
                orig, restored,
                "split {} drifted: saved {:?} vs resaved {:?}",
                i, orig, restored
            );
        }
    }

    #[test]
    fn save_restore_resave_simple_split() {
        let tree = make_split(
            SplitDirection::Horizontal,
            24,
            39,
            24,
            40,
            make_leaf(24, 39),
            make_leaf(24, 40),
        );

        let saved = pane_node_to_saved(&tree).unwrap();
        let restored = simulate_restore(&saved, 24, 80);
        let resaved = pane_node_to_saved(&restored).unwrap();

        let mut s1 = vec![];
        let mut s2 = vec![];
        collect_sizes(&saved, &mut s1);
        collect_sizes(&resaved, &mut s2);

        assert_eq!(s1.len(), s2.len());
        for (i, (a, b)) in s1.iter().zip(&s2).enumerate() {
            assert_eq!(a, b, "split {} drifted: {:?} vs {:?}", i, a, b);
        }
    }
}
