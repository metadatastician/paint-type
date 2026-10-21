// SPDX-License-Identifier: AGPL-3.0-or-later
//
// dispatch -- the single function the GUI calls. Pure: Document in,
// Response out, plus bounded PNG / `.ptype` / built-in plugin execution.

use crate::document::{Document, Rect};
use crate::protocol::{Command, DirtyRect, PluginInfo, Response};
use base64::Engine;
use paint_type_plugins::effect::{
    BrightnessContrastEffect, EffectPlugin, GrayscaleEffect, InvertColourEffect,
};

fn builtin_plugins() -> Vec<PluginInfo> {
    vec![
        PluginInfo {
            id: "com.paint-type.effects.brightness-contrast".to_string(),
            name: "Brightness/Contrast".to_string(),
            description: "Adjusts brightness and contrast of the active layer".to_string(),
            plugin_type: "effect".to_string(),
            version: "1.0.0".to_string(),
        },
        PluginInfo {
            id: "com.paint-type.effects.invert".to_string(),
            name: "Invert Colours".to_string(),
            description: "Inverts RGB channels while preserving pixel alpha".to_string(),
            plugin_type: "effect".to_string(),
            version: "1.0.0".to_string(),
        },
        PluginInfo {
            id: "com.paint-type.effects.grayscale".to_string(),
            name: "Grayscale".to_string(),
            description: "Converts RGB channels to Rec.709 luminance grayscale".to_string(),
            plugin_type: "effect".to_string(),
            version: "1.0.0".to_string(),
        },
    ]
}

fn loaded_response(document: &Document, full: Rect) -> Response {
    let pixels = document.render_all();
    let rgba_base64 = base64::engine::general_purpose::STANDARD.encode(&pixels);
    Response::Loaded {
        dirty: DirtyRect {
            x: full.x,
            y: full.y,
            w: document.width(),
            h: document.height(),
            rgba_base64,
        },
    }
}

/// Apply one command to the document, returning the response to hand
/// back to the web UI. `doc` is `None` until the first `NewDoc`.
pub fn dispatch(doc: &mut Option<Document>, cmd: Command) -> Response {
    if let Command::NewDoc { w, h } = cmd {
        if w == 0 || h == 0 || w > crate::codec::MAX_DIM || h > crate::codec::MAX_DIM {
            return Response::Error {
                message: format!(
                    "document dimensions {w}x{h} outside 1..={}",
                    crate::codec::MAX_DIM
                ),
            };
        }
        *doc = Some(Document::new(w, h));
        return Response::Ack;
    }

    let Some(document) = doc.as_mut() else {
        return Response::Error {
            message: "no document; send new_doc first".to_string(),
        };
    };

    match cmd {
        Command::NewDoc { .. } => unreachable!("handled above"),
        Command::SetColour { r, g, b, a } => {
            document.set_colour(r, g, b, a);
            Response::Ack
        }
        Command::SetBrush { diameter, hardness } => {
            document.set_brush(diameter, hardness);
            Response::Ack
        }
        Command::OpenPng { path } => match crate::codec::load_png(&path) {
            Err(e) => Response::Error { message: e },
            Ok((rgba, w, h)) => {
                let full = document.load_png(&rgba, w, h);
                loaded_response(document, full)
            }
        },
        Command::OpenPtype { path } => match ptype_format::decode_from_file(&path) {
            Err(e) => Response::Error {
                message: format!("ptype decode: {e}"),
            },
            Ok(canvas) => {
                let full = document.load_ptype(&canvas);
                loaded_response(document, full)
            }
        },
        Command::PointerDown { x, y } => {
            let rect = document.pointer_down(x, y);
            paint(document, rect)
        }
        Command::PointerMove { x, y } => {
            let rect = document.pointer_move(x, y);
            paint(document, rect)
        }
        Command::PointerUp => {
            document.pointer_up();
            Response::Ack
        }
        Command::SelectTool { kind } => {
            document.set_tool(kind);
            Response::Ack
        }
        Command::FillAt { x, y } => {
            let rect = document.fill_at(x, y);
            paint(document, rect)
        }
        Command::SavePng { path } => {
            let w = document.width();
            let h = document.height();
            let rgba = document.render_all();
            match crate::codec::save_png(&path, &rgba, w, h) {
                Ok(()) => Response::Saved { path },
                Err(e) => Response::Error { message: e },
            }
        }
        Command::SavePtype { path } => {
            if path.is_empty() || path.contains('\0') {
                return Response::Error {
                    message: "invalid empty or NUL-containing file path".to_string(),
                };
            }
            let canvas = document.to_ptype_canvas();
            match ptype_format::encode_to_file(&path, &canvas) {
                Ok(()) => Response::Saved { path },
                Err(e) => Response::Error {
                    message: format!("ptype encode: {e}"),
                },
            }
        }
        Command::Undo => match document.undo() {
            Some(full) => loaded_response(document, full),
            None => Response::Ack,
        },
        Command::Redo => match document.redo() {
            Some(full) => loaded_response(document, full),
            None => Response::Ack,
        },
        Command::AddLayer { name } => {
            let _ = document.add_layer(name);
            let (layers, active_index) = document.list_layers();
            Response::LayerList {
                layers,
                active_index,
            }
        }
        Command::DeleteLayer { index } => match document.delete_layer(index) {
            Ok(rect) => paint(document, rect),
            Err(message) => Response::Error { message },
        },
        Command::SelectLayer { index } => match document.select_layer(index) {
            Ok(()) => {
                let (layers, active_index) = document.list_layers();
                Response::LayerList {
                    layers,
                    active_index,
                }
            }
            Err(message) => Response::Error { message },
        },
        Command::ReorderLayer { from, to } => match document.reorder_layer(from, to) {
            Ok(rect) => paint(document, rect),
            Err(message) => Response::Error { message },
        },
        Command::SetLayerVisibility { index, visible } => {
            match document.set_layer_visibility(index, visible) {
                Ok(rect) => paint(document, rect),
                Err(message) => Response::Error { message },
            }
        }
        Command::SetLayerOpacity { index, opacity } => {
            match document.set_layer_opacity(index, opacity) {
                Ok(rect) => paint(document, rect),
                Err(message) => Response::Error { message },
            }
        }
        Command::SetLayerBlend { index, blend } => match document.set_layer_blend(index, &blend) {
            Ok(rect) => paint(document, rect),
            Err(message) => Response::Error { message },
        },
        Command::ListLayers => {
            let (layers, active_index) = document.list_layers();
            Response::LayerList {
                layers,
                active_index,
            }
        }
        Command::LoadPlugin { path } => {
            if !paint_type_plugins::manifest::is_safe_relative_plugin_path(&path) {
                return Response::Error {
                    message: format!("unsafe plugin path rejected: {path}"),
                };
            }
            let manifest_res = if path.ends_with(".json") {
                paint_type_plugins::manifest::PluginManifest::from_json_file(&path)
            } else {
                paint_type_plugins::manifest::PluginManifest::from_toml_file(&path)
            };
            match manifest_res {
                Ok(m) => Response::PluginLoaded {
                    plugin_id: m.id.as_str().to_string(),
                },
                Err(e) => Response::Error {
                    message: e.to_string(),
                },
            }
        }
        Command::UnloadPlugin { plugin_id } => Response::PluginUnloaded { plugin_id },
        Command::ListPlugins => Response::PluginList {
            plugins: builtin_plugins(),
        },
        Command::InvokePlugin { invocation } => {
            let effect_box: Box<dyn EffectPlugin> = match invocation.plugin_id.as_str() {
                "com.paint-type.effects.invert" | "invert" => Box::new(InvertColourEffect::new()),
                "com.paint-type.effects.grayscale" | "grayscale" => {
                    Box::new(GrayscaleEffect::new())
                }
                "com.paint-type.effects.brightness-contrast" | "brightness_contrast" => {
                    let b = invocation
                        .args
                        .get("brightness")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.0);
                    let c = invocation
                        .args
                        .get("contrast")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(1.0);
                    Box::new(BrightnessContrastEffect::new(b, c))
                }
                other => {
                    return Response::Error {
                        message: format!("unknown plugin id: {other}"),
                    }
                }
            };
            match document.apply_effect(effect_box.as_ref()) {
                Ok(rect) => paint(document, rect),
                Err(message) => Response::Error { message },
            }
        }
    }
}

fn paint(doc: &Document, rect: Rect) -> Response {
    if rect.w == 0 || rect.h == 0 {
        return Response::Ack;
    }
    let rgba = doc.render(rect);
    let rgba_base64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
    Response::Painted {
        dirty: DirtyRect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            rgba_base64,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Command, PluginInvocation, Response, ToolKind};
    use std::collections::HashMap;

    fn decode(dirty_b64: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(dirty_b64)
            .expect("valid base64")
    }

    #[test]
    fn new_doc_then_stroke_paints_non_transparent_pixels() {
        let mut doc: Option<Document> = None;

        assert_eq!(
            dispatch(&mut doc, Command::NewDoc { w: 128, h: 128 }),
            Response::Ack
        );
        assert_eq!(
            dispatch(
                &mut doc,
                Command::SetColour {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0
                }
            ),
            Response::Ack
        );
        assert_eq!(
            dispatch(
                &mut doc,
                Command::SetBrush {
                    diameter: 16,
                    hardness: 0.0
                }
            ),
            Response::Ack
        );

        let down = dispatch(&mut doc, Command::PointerDown { x: 32.0, y: 32.0 });
        let Response::Painted { dirty } = down else {
            panic!("expected Painted, got {down:?}");
        };
        let bytes = decode(&dirty.rgba_base64);
        assert_eq!(bytes.len() as u32, dirty.w * dirty.h * 4);
        let has_opaque = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 240 && p[0] > 200);
        assert!(has_opaque, "expected a near-opaque red pixel in the dab");

        assert_eq!(dispatch(&mut doc, Command::PointerUp), Response::Ack);
    }

    #[test]
    fn undo_and_redo_restore_exact_canvas_state() {
        let mut doc: Option<Document> = None;
        dispatch(&mut doc, Command::NewDoc { w: 64, h: 64 });
        dispatch(
            &mut doc,
            Command::SetColour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        dispatch(
            &mut doc,
            Command::SetBrush {
                diameter: 16,
                hardness: 1.0,
            },
        );

        let blank = doc.as_ref().unwrap().render_all();

        dispatch(&mut doc, Command::PointerDown { x: 32.0, y: 32.0 });
        dispatch(&mut doc, Command::PointerUp);
        let painted = doc.as_ref().unwrap().render_all();
        assert_ne!(blank, painted);

        assert!(matches!(dispatch(&mut doc, Command::Undo), Response::Loaded { .. }));
        assert_eq!(doc.as_ref().unwrap().render_all(), blank);

        assert!(matches!(dispatch(&mut doc, Command::Redo), Response::Loaded { .. }));
        assert_eq!(doc.as_ref().unwrap().render_all(), painted);
    }

    #[test]
    fn multi_layer_and_builtin_effect_commands_work() {
        let mut doc: Option<Document> = None;
        dispatch(&mut doc, Command::NewDoc { w: 64, h: 64 });
        dispatch(
            &mut doc,
            Command::SetColour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        dispatch(&mut doc, Command::FillAt { x: 10.0, y: 10.0 });

        // Add foreground layer and set 50% blue over red.
        let added = dispatch(
            &mut doc,
            Command::AddLayer {
                name: Some("Overlay".to_string()),
            },
        );
        assert!(matches!(added, Response::LayerList { active_index: 1, .. }));
        dispatch(&mut doc, Command::SetLayerOpacity { index: 1, opacity: 0.5 });
        dispatch(
            &mut doc,
            Command::SetColour {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        );
        dispatch(&mut doc, Command::FillAt { x: 10.0, y: 10.0 });

        // Invoke built-in invert plugin on active layer.
        let inv = dispatch(
            &mut doc,
            Command::InvokePlugin {
                invocation: PluginInvocation {
                    plugin_id: "invert".to_string(),
                    function: "apply".to_string(),
                    args: HashMap::new(),
                },
            },
        );
        assert!(matches!(inv, Response::Painted { .. }));
    }

    #[test]
    fn eraser_reduces_alpha() {
        let mut doc: Option<Document> = None;
        dispatch(&mut doc, Command::NewDoc { w: 128, h: 128 });
        dispatch(
            &mut doc,
            Command::SetColour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        dispatch(
            &mut doc,
            Command::SetBrush {
                diameter: 16,
                hardness: 0.0,
            },
        );

        dispatch(&mut doc, Command::PointerDown { x: 32.0, y: 32.0 });
        let after_paint = {
            let document = doc.as_ref().unwrap();
            let rgba = document.render(Rect {
                x: 32,
                y: 32,
                w: 1,
                h: 1,
            });
            rgba[3]
        };
        dispatch(&mut doc, Command::PointerUp);

        dispatch(
            &mut doc,
            Command::SelectTool {
                kind: ToolKind::Eraser,
            },
        );
        dispatch(&mut doc, Command::PointerDown { x: 32.0, y: 32.0 });
        let after_erase = {
            let document = doc.as_ref().unwrap();
            let rgba = document.render(Rect {
                x: 32,
                y: 32,
                w: 1,
                h: 1,
            });
            rgba[3]
        };
        dispatch(&mut doc, Command::PointerUp);

        assert!(
            (after_erase as u16) < (after_paint as u16),
            "expected eraser to reduce alpha: before={after_paint}, after={after_erase}"
        );
    }

    #[test]
    fn fill_paints_region() {
        let mut doc: Option<Document> = None;
        dispatch(&mut doc, Command::NewDoc { w: 64, h: 64 });
        dispatch(
            &mut doc,
            Command::SetColour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );

        let response = dispatch(&mut doc, Command::FillAt { x: 5.0, y: 5.0 });
        let Response::Painted { dirty } = response else {
            panic!("expected Painted, got {response:?}");
        };
        let bytes = decode(&dirty.rgba_base64);
        let has_red = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 200 && p[3] > 200);
        assert!(has_red, "expected red pixels after fill");
    }

    #[test]
    fn commands_before_new_doc_error() {
        let mut doc: Option<Document> = None;
        let r = dispatch(&mut doc, Command::PointerDown { x: 1.0, y: 1.0 });
        assert!(matches!(r, Response::Error { .. }));
    }

    #[test]
    fn open_png_round_trips_pixels() {
        use crate::codec;

        let w: u32 = 4;
        let h: u32 = 4;
        let mut src = vec![0u8; (w * h * 4) as usize];
        let target_offset = ((w + 1) * 4) as usize;
        src[target_offset] = 0;
        src[target_offset + 1] = 200;
        src[target_offset + 2] = 0;
        src[target_offset + 3] = 255;

        let path = std::env::temp_dir().join("pt_dispatch_open_png.png");
        let path_str = path.to_str().unwrap();
        codec::save_png(path_str, &src, w, h).expect("save temp png");

        let mut doc: Option<Document> = None;
        assert_eq!(dispatch(&mut doc, Command::NewDoc { w, h }), Response::Ack);

        let response = dispatch(
            &mut doc,
            Command::OpenPng {
                path: path_str.to_string(),
            },
        );
        let Response::Loaded { dirty } = response else {
            panic!("expected Loaded, got {response:?}");
        };

        assert_eq!(dirty.x, 0);
        assert_eq!(dirty.y, 0);
        assert_eq!(dirty.w, w);
        assert_eq!(dirty.h, h);

        let rendered = decode(&dirty.rgba_base64);
        assert_eq!(rendered.len() as u32, w * h * 4);
        let px = &rendered[target_offset..target_offset + 4];
        assert!(
            px[1] > 190,
            "green channel at (1,1) expected ~200, got {}",
            px[1]
        );
        assert!(px[3] > 250, "alpha at (1,1) expected ~255, got {}", px[3]);
    }

    #[test]
    fn save_and_load_ptype_round_trips() {
        let mut doc: Option<Document> = None;
        assert_eq!(
            dispatch(&mut doc, Command::NewDoc { w: 128, h: 128 }),
            Response::Ack
        );
        dispatch(
            &mut doc,
            Command::SetColour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        dispatch(
            &mut doc,
            Command::SetBrush {
                diameter: 16,
                hardness: 0.0,
            },
        );
        dispatch(&mut doc, Command::PointerDown { x: 32.0, y: 32.0 });
        dispatch(&mut doc, Command::PointerUp);
        dispatch(
            &mut doc,
            Command::SetLayerBlend {
                index: 0,
                blend: "multiply".to_string(),
            },
        );

        let path = std::env::temp_dir().join("pt_dispatch_roundtrip.ptype");
        let path_str = path.to_str().unwrap();
        let save_resp = dispatch(
            &mut doc,
            Command::SavePtype {
                path: path_str.to_string(),
            },
        );
        assert!(
            matches!(save_resp, Response::Saved { .. }),
            "expected Saved, got {save_resp:?}"
        );

        let mut doc2: Option<Document> = None;
        assert_eq!(
            dispatch(&mut doc2, Command::NewDoc { w: 128, h: 128 }),
            Response::Ack
        );
        let load_resp = dispatch(
            &mut doc2,
            Command::OpenPtype {
                path: path_str.to_string(),
            },
        );
        assert!(
            matches!(load_resp, Response::Loaded { .. }),
            "expected Loaded, got {load_resp:?}"
        );

        let path2 = std::env::temp_dir().join("pt_dispatch_roundtrip2.ptype");
        let path2_str = path2.to_str().unwrap();
        let save_resp2 = dispatch(
            &mut doc2,
            Command::SavePtype {
                path: path2_str.to_string(),
            },
        );
        assert!(
            matches!(save_resp2, Response::Saved { .. }),
            "expected Saved, got {save_resp2:?}"
        );

        let bytes1 = std::fs::read(path_str).expect("read first ptype");
        let bytes2 = std::fs::read(path2_str).expect("read second ptype");
        assert_eq!(bytes1, bytes2, "ptype round-trip must be byte-identical");
    }
}
