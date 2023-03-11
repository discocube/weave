use std::env;
use std::time::Instant;

pub mod graph;

use graph::check;
use graph::make;
use graph::shrink;
use graph::solve;
use graph::types::*;
use graph::utils;

fn main() {
    // cargo run --release 1373600 10
    let args: Vec<String> = env::args().collect();
    weave_nodes(
        args
            .get(1)
            .unwrap_or(&"79040".to_string())
            .parse()
            .unwrap_or(79040), 
        args
            .get(2)
            .unwrap_or(&"100".to_string())
            .parse()
            .unwrap_or(10)
    )
}

pub fn weave_nodes(order: u32, repeats: u32) {
    let max_xyz = utils::get_max_xyz(order as i32);
    let verts: Verts = make::vertices(max_xyz);
    let vi_map: VIMap = make::vi_map(&verts);
    let adj: Adjacency = make::adjacency_map(&verts, max_xyz, &vi_map);
    let edges: Edges = make::edges_from_adjacency(&adj);
    let edge_adj = make::edges_adjacency_map(&adj, &edges, &verts);
    let (z_adj, z_length) = shrink::adjacency(&verts, &adj);
    let mut solution: Solution = Solution::new();
    let start: Instant = Instant::now();
    for _ in 0..repeats {
        solution = solve::weave(&adj, &vi_map, &edge_adj, &verts, &z_adj, &z_length);
    }
    println!(
        "⭕️ ORDER: {:?} | REPS: {} | DUR: {} | ID: {:?}",
        order, 
        repeats,
        utils::elapsed_ms(start, Instant::now(), repeats, "WEAVE"), 
        check::id_seq(&solution, &adj), 
    );
}
