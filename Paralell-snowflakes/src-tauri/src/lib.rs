// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Point {
    x: f64,
    y: f64,
}

// Generates the Koch snowflake vertices for Tauri v2.
// Commands in Tauri v2 are async by default.
#[tauri::command]
async fn generate_snowflake(iterations: u32, parallel: bool) -> Result<Vec<Point>, String> {
    let mut points = vec![
        Point { x: 0.0, y: 1.0 },
        Point {
            x: (2.0 * PI / 3.0).cos(),
            y: (2.0 * PI / 3.0).sin(),
        },
        Point {
            x: (4.0 * PI / 3.0).cos(),
            y: (4.0 * PI / 3.0).sin(),
        },
        Point { x: 0.0, y: 1.0 }, // Close the shape
    ];

    if iterations == 0 {
        return Ok(points);
    }

    for _ in 0..iterations {
        let segments: Vec<(Point, Point)> = points.windows(2).map(|p| (p[0].clone(), p[1].clone())).collect();

        let new_points = if parallel {
            // Parallel computation using Rayon
            segments
                .par_iter()
                .flat_map(|(p1, p2)| {
                    let dx = p2.x - p1.x;
                    let dy = p2.y - p1.y;

                    let p_a = Point {
                        x: p1.x + dx / 3.0,
                        y: p1.y + dy / 3.0,
                    };
                    let p_b = Point {
                        x: p1.x + dx / 2.0 - (dy * (3.0f64.sqrt() / 6.0)),
                        y: p1.y + dy / 2.0 + (dx * (3.0f64.sqrt() / 6.0)),
                    };
                    let p_c = Point {
                        x: p1.x + 2.0 * dx / 3.0,
                        y: p1.y + 2.0 * dy / 3.0,
                    };

                    vec![p1.clone(), p_a, p_b, p_c]
                })
                .collect::<Vec<Point>>()
        } else {
            // Sequential computation
            segments
                .iter()
                .flat_map(|(p1, p2)| {
                    let dx = p2.x - p1.x;
                    let dy = p2.y - p1.y;

                    let p_a = Point {
                        x: p1.x + dx / 3.0,
                        y: p1.y + dy / 3.0,
                    };
                    let p_b = Point {
                        x: p1.x + dx / 2.0 - (dy * (3.0f64.sqrt() / 6.0)),
                        y: p1.y + dy / 2.0 + (dx * (3.0f64.sqrt() / 6.0)),
                    };
                    let p_c = Point {
                        x: p1.x + 2.0 * dx / 3.0,
                        y: p1.y + 2.0 * dy / 3.0,
                    };
                    
                    vec![p1.clone(), p_a, p_b, p_c]
                })
                .collect::<Vec<Point>>()
        };

        let mut final_points = new_points;
        if let Some(last_segment) = segments.last() {
            final_points.push(last_segment.1.clone());
        }
        points = final_points;
    }

    Ok(points)
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![generate_snowflake])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

